"""Capture CSPICE SPKCVT/SPKCVO bit patterns in rotating frames."""
import struct
from pathlib import Path
import spiceypy as spice

root = Path(__file__).resolve().parents[3]
spice.furnsh(str(root / 'target/kernels/de440s.bsp'))
spice.furnsh(str(root / 'target/kernels/pck00011.tpc'))
print('frame,locus,endpoint,correction,x,y,z,vx,vy,vz,light_time')
for frame in ['IAU_EARTH', 'IAU_MOON']:
    for locus in ['CENTER', 'OBSERVER', 'TARGET']:
        for correction in ['NONE', 'LT', 'LT+S', 'CN', 'CN+S', 'XLT', 'XLT+S', 'XCN', 'XCN+S']:
            for endpoint in ['target', 'observer']:
                if endpoint == 'target':
                    state, light = spice.spkcvt([1, 0, 0, 1, 0, 0], 10.0, 'MOON', 'J2000',
                                                100.0, frame, locus, correction, 'EARTH')
                else:
                    state, light = spice.spkcvo('MOON', 100.0, frame, locus, correction,
                                                [1, 0, 0, 1, 0, 0], 10.0, 'EARTH', 'J2000')
                bits = [format(struct.unpack('>Q', struct.pack('>d', float(x)))[0], '016x') for x in [*state, light]]
                print(','.join([frame, locus, endpoint, correction, *bits]))
spice.kclear()
