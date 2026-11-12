"""Capture CSPICE ILLUMF missing-DSK short errors using local reference kernels."""
import hashlib
import json
import struct
from pathlib import Path

import spiceypy as spice
import spiceypy.utils.libspicehelper as helper

root = Path(__file__).resolve().parents[3]
spk = root / 'target/kernels/de440s.bsp'
pck = root / 'target/kernels/pck00011.tpc'
dsk = root / 'tests/golden/dsk/dense_lat.bds'

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

records = []
for target, observer, frame, load_dsk in [('MOON', 'EARTH', 'IAU_MOON', False), ('EARTH', 'MOON', 'IAU_EARTH', True), ('MOON', 'MOON', 'IAU_MOON', False), ('MOON', 'MOON', 'J2000', False), ('MOON', 'EARTH', 'J2000', False)]:
    spice.kclear()
    for path in [spk, pck] + ([dsk] if load_dsk else []):
        spice.furnsh(str(path))
    try:
        spice.illumf('DSK/UNPRIORITIZED', target, 'SUN', 0.0,
                     frame, 'NONE', observer, [1.0, 0.0, 0.0])
    except spice.utils.exceptions.SpiceyError as error:
        records.append({'target': target, 'observer': observer,
                        'frame': frame, 'dsk_loaded': load_dsk, 'short': error.short})
    else:
        raise RuntimeError('Expected native error was not raised')
spice.kclear()
for path in [spk, pck]:
    spice.furnsh(str(path))
same_source = []
for method, frame in [('ELLIPSOID', 'J2000'), ('DSK/UNPRIORITIZED', 'J2000'),
                      ('ELLIPSOID', 'IAU_MOON'), ('DSK/UNPRIORITIZED', 'IAU_MOON')]:
    record = {'method': method, 'target': 'MOON', 'illuminator': 'MOON',
              'observer': 'EARTH', 'frame': frame}
    try:
        result = spice.illumf(method, 'MOON', 'MOON', 0.0, frame, 'NONE',
                              'EARTH', [1.0, 0.0, 0.0])
    except spice.utils.exceptions.SpiceyError as error:
        record['short'] = error.short
    else:
        values = [result[0], *result[1], *result[2:5]]
        record['ieee754_bits'] = [format(struct.unpack('>Q', struct.pack('>d', float(value)))[0], '016x') for value in values]
        record['visible'], record['illuminated'] = bool(result[5]), bool(result[6])
    same_source.append(record)
spice.kclear()
report = {'toolkit': spice.tkvrsn('TOOLKIT'), 'spiceypy': spice.__version__,
          'native_library_sha256': sha(Path(helper.libspice._name)),
          'spk_sha256': sha(spk), 'pck_sha256': sha(pck), 'dsk_sha256': sha(dsk),
          'epoch_et': 0.0, 'point_km': [1.0, 0.0, 0.0], 'records': records,
          'same_source': same_source}
print(json.dumps(report, indent=2))
