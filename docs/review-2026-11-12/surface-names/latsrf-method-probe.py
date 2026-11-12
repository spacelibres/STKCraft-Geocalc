"""Capture LATSRF method validation and frame-error precedence."""
from pathlib import Path
import struct
import random
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
           'DSK', 'DSK/UNPRIORITIZED/INTERCEPT', 'DSK/UNPRIORITIZED/NADIR',
           'DSK/UNPRIORITIZED/TANGENT', 'ELLIPSOID', 'ELLIPSOID/INTERCEPT',
           'ELLIPSOID/NEAR', 'ELLIPSOID/TANGENT', 'BOGUS']
rng = random.Random(20261114)
clauses = ['DSK', 'ELLIPSOID', 'UNPRIORITIZED', 'NADIR', 'INTERCEPT',
           'NEAR', 'TANGENT', 'UMBRAL', 'SURFACES=100', 'SURFACES=UNKNOWN', '']
for _ in range(200):
    methods.append('/'.join(rng.choice(clauses) for _ in range(rng.randrange(1, 5))))
frames = ['DSK_LOCAL', 'IAU_EARTH', 'NONEXISTENT_FRAME', '', ' ']
print('method|frame|result')
for method in methods:
    for frame in frames:
        try:
            point = spice.latsrf(method, 'MOON', 0., frame, [[0.5, 0.2]])[0]
            result = 'N|' + '|'.join(struct.pack('>d', value).hex() for value in point)
        except spice.utils.exceptions.SpiceyError as error:
            result = 'E|' + error.short
        print(f'{method}|{frame}|{result}')
spice.kclear()
