"""Capture SINCPT with invalid ellipsoid radii using native kernels."""
from pathlib import Path
import spiceypy as spice

root = Path(__file__).resolve().parents[3]
spice.kclear()
for name in ['target/kernels/de440s.bsp', 'target/kernels/pck00011.tpc']:
    spice.furnsh(str(root / name))
direction = spice.spkpos('MOON', 0., 'J2000', 'NONE', 'EARTH')[0]
print('radii|result')
for radii in [[0., 1., 1.], [-1., 1., 1.], [1., 0., 1.], [1., 1., 0.], [1., 1., -1.]]:
    spice.pdpool('BODY301_RADII', radii)
    try:
        spice.sincpt('ELLIPSOID', 'MOON', 0., 'IAU_MOON', 'NONE', 'EARTH', 'J2000', direction)
        result = 'SUCCESS'
    except spice.utils.exceptions.SpiceyError as error:
        result = error.short
    print(';'.join(map(str, radii)) + '|' + result)
spice.kclear()
