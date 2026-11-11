//! Native DAF generic-segment metadata, reference selection and packets.
//! Implements SGMETA/SGFRVI/SGFCON/SGFPKT without a sampled surrogate.
use crate::{daf::DafFile, error::GeoError};
pub(crate) struct GenericSegment<'a> {
    file: &'a DafFile,
    meta: [i64; 17],
}
impl<'a> GenericSegment<'a> {
    pub(crate) fn new(file: &'a DafFile, begin: usize, end: usize) -> Result<Self, GeoError> {
        let declared = file.read_doubles(end, 1)?[0].round() as usize;
        if declared < 15 {
            return Err(GeoError::BadDaf(
                "generic segment has fewer than15 metadata items",
            ));
        }
        let actual = if declared == 15 { 16 } else { declared };
        let known = actual.min(17);
        if end < actual {
            return Err(GeoError::BadDaf("generic metadata address out of range"));
        }
        let values = file.read_doubles(end - actual + 1, known)?;
        let mut meta = [0i64; 17];
        for (i, x) in values.iter().enumerate() {
            meta[i] = x.round() as i64;
        }
        meta[16] = known as i64;
        meta[known.saturating_sub(1)..16].fill(0);
        for i in [0, 2, 5, 7, 10, 12] {
            meta[i] += begin as i64 - 1;
        }
        if meta[11] <= 0 {
            return Err(GeoError::BadDaf("generic segment has no packets"));
        }
        Ok(Self { file, meta })
    }
    fn read(&self, address: i64, count: usize) -> Result<Vec<f64>, GeoError> {
        let address = usize::try_from(address)
            .map_err(|_| GeoError::BadDaf("negative generic data address"))?;
        self.file.read_doubles(address, count)
    }
    pub(crate) fn constant(&self, index: usize) -> Result<f64, GeoError> {
        if index == 0 || index as i64 > self.meta[1] {
            return Err(GeoError::BadDaf("generic constant index out of range"));
        }
        Ok(self.read(self.meta[0] + index as i64, 1)?[0])
    }
    pub(crate) fn packet_at(&self, et: f64) -> Result<Option<Vec<f64>>, GeoError> {
        let Some(index) = self.reference_index(et)? else {
            return Ok(None);
        };
        Ok(Some(self.packet(index)?))
    }
    fn reference_index(&self, x: f64) -> Result<Option<usize>, GeoError> {
        let n = self.meta[11] as usize;
        let kind = self.meta[4];
        if kind == 0 || kind == 1 {
            let refs = self.read(self.meta[5] + 1, 2)?;
            let last = refs[0] + (n - 1) as f64 * refs[1];
            if x < refs[0] {
                return Ok(if kind == 0 { None } else { Some(1) });
            }
            if x > last {
                return Ok(Some(n));
            }
            if n == 1 {
                return Ok(Some(1));
            }
            if refs[1] <= 0.0 {
                return Err(GeoError::BadDaf("invalid generic implicit reference step"));
            }
            let raw = (x - refs[0]) / refs[1] + if kind == 0 { 1.0 } else { 1.5 };
            if raw > i32::MAX as f64 + if kind == 0 { 0.0 } else { 0.5 } {
                return Err(GeoError::BadDaf(
                    "generic reference index exceeds native integer range",
                ));
            }
            return Ok(Some((raw as usize).min(n)));
        }
        if !(2..=4).contains(&kind) {
            return Err(GeoError::Unsupported(format!(
                "generic reference directory type {kind}"
            )));
        }
        let count = self.meta[6] as usize;
        let refs = self.read(self.meta[5] + 1, count)?;
        let index = refs.partition_point(|r| if kind == 2 { *r < x } else { *r <= x });
        if kind != 4 {
            return Ok(if index == 0 { None } else { Some(index) });
        }
        if index == 0 {
            return Ok(Some(1));
        }
        if index == refs.len() {
            return Ok(Some(index));
        }
        Ok(Some(if refs[index] - x <= x - refs[index - 1] {
            index + 1
        } else {
            index
        }))
    }
    pub(crate) fn reference_at(&self, et: f64) -> Result<Option<(usize, f64)>, GeoError> {
        let Some(index) = self.reference_index(et)? else {
            return Ok(None);
        };
        let value = if self.meta[4] <= 1 {
            let refs = self.read(self.meta[5] + 1, 2)?;
            refs[0] + (index - 1) as f64 * refs[1]
        } else {
            self.read(self.meta[5] + index as i64, 1)?[0]
        };
        Ok(Some((index, value)))
    }
    pub(crate) fn reference_count(&self) -> usize {
        self.meta[6] as usize
    }
    pub(crate) fn packet_count(&self) -> usize {
        self.meta[11] as usize
    }
    pub(crate) fn packet(&self, index: usize) -> Result<Vec<f64>, GeoError> {
        if index == 0 || index as i64 > self.meta[11] {
            return Err(GeoError::BadDaf("generic packet index out of range"));
        }
        let size = self.meta[14];
        let offset = self.meta[15];
        let base = self.meta[10];
        match self.meta[9] {
            0 => {
                if offset == 0 {
                    self.read(base + (index as i64 - 1) * size + 1, size as usize)
                } else {
                    let start = base + (index as i64 - 1) * (size + offset) + 1;
                    self.read(start + offset, (size - offset + 1) as usize)
                }
            }
            1 => {
                let bounds = self.read(self.meta[7] + index as i64, 2)?;
                let begin = bounds[0] as i64;
                let end = bounds[1] as i64;
                let size = end - begin - offset;
                if size <= 0 {
                    return Err(GeoError::BadDaf("invalid generic variable packet size"));
                }
                self.read(base + begin, size as usize)
            }
            other => Err(GeoError::Unsupported(format!(
                "generic packet directory type {other}"
            ))),
        }
    }
}
