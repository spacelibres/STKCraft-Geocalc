"""Capture SINCPT with inertial target-centered frames and missing radii."""
from pathlib import Path
import struct
import spiceypy as spice

root = Path(__file__).resolve().parents[3]
spice.kclear()
spice.furnsh(str(root / 'target/kernels/de440s.bsp'))
print('target|frame|method|radii|dx|dy|dz|result')
with spice.no_found_check():
    for target, frame, method, radii in [(0, 'J2000', 'DSK/UNPRIORITIZED', False),
                                         (0, 'J2000', 'ELLIPSOID', False),
                                         (0, 'J2000', 'ELLIPSOID', True),
                                         (301, 'IAU_MOON', 'ELLIPSOID', False)]:
        if target == 301:
            spice.dvpool('BODY0_RADII')
        if radii:
            spice.pdpool('BODY0_RADII', [1., 1., 1.])
        direction = spice.spkpos(str(target), 0., 'J2000', 'NONE', 'EARTH')[0]
        try:
            point, epoch, vector, found = spice.sincpt(method, str(target), 0., frame, 'NONE', 'EARTH', 'J2000', direction)
            result = ('N|' + '|'.join(struct.pack('>d', v).hex() for v in [*point, epoch, *vector])) if found else 'MISS'
        except spice.utils.exceptions.SpiceyError as error:
            result = 'E|' + error.short
        encoded = '|'.join(struct.pack('>d', v).hex() for v in direction)
        print(f'{target}|{frame}|{method}|{int(radii)}|{encoded}|{result}')
spice.kclear()
