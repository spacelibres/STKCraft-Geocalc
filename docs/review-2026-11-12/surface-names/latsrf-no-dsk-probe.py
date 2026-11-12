"""Capture LATSRF method and frame errors with no loaded DSK files."""
from pathlib import Path
import spiceypy as spice

root = Path(__file__).resolve().parents[3]
spice.kclear()
for path in ['target/kernels/de440s.bsp', 'target/kernels/pck00011.tpc',
             'tests/golden/dsk/frames.tf']:
    spice.furnsh(str(root / path))
methods = ['DSK/UNPRIORITIZED', 'DSK/UNPRIORITIZED/SURFACES=100',
           'DSK/UNPRIORITIZED/SURFACES=UNKNOWN', 'DSK',
           'DSK/UNPRIORITIZED/NADIR', '']
frames = ['DSK_LOCAL', 'IAU_EARTH', 'NONEXISTENT_FRAME', '']
print('method|frame|result')
for method in methods:
    for frame in frames:
        try:
            spice.latsrf(method, 'MOON', 0., frame, [[0.5, 0.2]])
            result = 'SUCCESS'
        except spice.utils.exceptions.SpiceyError as error:
            result = error.short
        print(f'{method}|{frame}|{result}')
spice.kclear()
