"""Capture one CSPICE SPKCVT state for the CLI moving-point regression."""
import hashlib
import json
import struct
from pathlib import Path
import spiceypy as spice
import spiceypy.utils.libspicehelper as helper

root = Path(__file__).resolve().parents[3]
spk = root / 'target/kernels/de440s.bsp'
pck = root / 'target/kernels/pck00011.tpc'
for path in [spk, pck]:
    spice.furnsh(str(path))
state, light_time = spice.spkcvt([1, 0, 0, 1, 0, 0], 10.0, 'MOON',
                                 'J2000', 100.0, 'J2000', 'CENTER',
                                 'NONE', 'EARTH')
observer_state, observer_light_time = spice.spkcvo('MOON', 100.0, 'J2000', 'CENTER', 'NONE',
                                                   [1, 0, 0, 1, 0, 0], 10.0, 'EARTH', 'J2000')
def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()
def bits(value):
    return format(struct.unpack('>Q', struct.pack('>d', float(value)))[0], '016x')
report = {'toolkit': spice.tkvrsn('TOOLKIT'), 'spiceypy': spice.__version__,
          'native_library_sha256': sha(Path(helper.libspice._name)),
          'spk_sha256': sha(spk), 'pck_sha256': sha(pck),
          'target_center': 'MOON', 'observer': 'EARTH', 'observer_point_center': 'EARTH', 'frame': 'J2000',
          'state_at_epoch': [1, 0, 0, 1, 0, 0], 'reference_epoch_et': 10.0,
          'query_epoch_et': 100.0, 'locus': 'CENTER', 'aberration': 'NONE',
          'target_point_state_and_light_time_bits': [bits(value) for value in [*state, light_time]],
          'observer_point_state_and_light_time_bits': [bits(value) for value in [*observer_state, observer_light_time]]}
spice.kclear()
print(json.dumps(report, indent=2))
