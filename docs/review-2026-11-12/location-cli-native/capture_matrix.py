"""Capture independent SPKCVT and SPKCVO correction-matrix bit patterns."""
import struct
from pathlib import Path
import spiceypy as spice

root = Path(__file__).resolve().parents[3]
spice.furnsh(str(root / 'target/kernels/de440s.bsp'))
spice.furnsh(str(root / 'target/kernels/pck00011.tpc'))
print('endpoint,correction,x,y,z,vx,vy,vz,light_time')
for correction in ['NONE', 'LT', 'LT+S', 'CN', 'CN+S', 'XLT', 'XLT+S', 'XCN', 'XCN+S']:
    for endpoint in ['target', 'observer']:
        if endpoint == 'target':
            state, light = spice.spkcvt([1, 0, 0, 1, 0, 0], 10.0, 'MOON', 'J2000',
                                        100.0, 'J2000', 'CENTER', correction, 'EARTH')
        else:
            state, light = spice.spkcvo('MOON', 100.0, 'J2000', 'CENTER', correction,
                                        [1, 0, 0, 1, 0, 0], 10.0, 'EARTH', 'J2000')
        bits = [format(struct.unpack('>Q', struct.pack('>d', float(x)))[0], '016x') for x in [*state, light]]
        print(','.join([endpoint, correction, *bits]))
spice.kclear()
