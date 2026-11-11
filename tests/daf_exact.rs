use stkcraft_geocalc::daf::DafFile;
#[test]
fn daf_names_and_summaries_equal_cspice() {
    for line in include_str!("golden/daf_exact.txt").lines() {
        let f: Vec<_> = line.split('|').collect();
        let daf = DafFile::open(f[0]).unwrap();
        let index: usize = f[1].parse().unwrap();
        let segment = &daf.segments()[index];
        assert_eq!(segment.name, f[2], "{} array{index}", f[0]);
        assert_eq!(
            segment.dc,
            f[3..5]
                .iter()
                .map(|x| x.parse::<f64>().unwrap())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            segment.ic,
            f[5..]
                .iter()
                .map(|x| x.parse::<i32>().unwrap())
                .collect::<Vec<_>>()
        );
    }
}
