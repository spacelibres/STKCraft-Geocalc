"""Capture named-surface SINCPT outputs and validation precedence."""
from pathlib import Path
import struct
import spiceypy as spice

root = Path(__file__).resolve().parents[3]
spice.kclear()
for path in ['target/kernels/de440s.bsp', 'target/kernels/pck00011.tpc',
             'tests/golden/dsk/frames.tf', 'tests/golden/dsk/optimized.bds']:
    spice.furnsh(str(root / path))
spice.pcpool('NAIF_SURFACE_NAME', ['Example Shape', 'Missing Surface'])
spice.pipool('NAIF_SURFACE_CODE', [100, 999])
spice.pipool('NAIF_SURFACE_BODY', [301, 301])
methods = ['DSK/UNPRIORITIZED', 'DSK/UNPRIORITIZED/SURFACES="Example Shape"',
           'DSK/UNPRIORITIZED/SURFACES="Missing Surface"', 'DSK/UNPRIORITIZED/SURFACES=UNKNOWN',
           'DSK', 'DSK/UNPRIORITIZED/NADIR', 'ELLIPSOID', 'ELLIPSOID/INTERCEPT', '']
position = spice.spkpos('MOON', 0., 'J2000', 'NONE', 'EARTH')[0]
directions = [position, [0., 0., 0.], -position]
frames = ['DSK_LOCAL', 'IAU_EARTH', 'NONEXISTENT_FRAME', '']
print('method|frame|correction|direction_x|direction_y|direction_z|result')
with spice.no_found_check():
    for method in methods:
        for frame in frames:
            for correction in ['NONE', 'LT', 'LT+S', 'CN', 'CN+S', 'XLT', 'XCN']:
                for direction in directions:
                    try:
                        point, epoch, vector, found = spice.sincpt(method, 'MOON', 0., frame, correction, 'EARTH', 'J2000', direction)
                        result = ('N|' + '|'.join(struct.pack('>d', v).hex() for v in [*point, epoch, *vector])) if found else 'MISS'
                    except spice.utils.exceptions.SpiceyError as error:
                        result = 'E|' + error.short
                    vector = '|'.join(struct.pack('>d', v).hex() for v in direction)
                    print(f'{method}|{frame}|{correction}|{vector}|{result}')
spice.kclear()
