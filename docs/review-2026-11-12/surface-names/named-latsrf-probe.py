"""Capture LATSRF results using surface-name method strings."""
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
groups = [[name] for name in ['Example Shape', 'example   shape', '100', 'Missing Surface', 'No Such Name']]
groups += [[], ['Example Shape', '100'], ['No Such Name', 'Missing Surface'],
           ['Missing Surface', 'No Such Name'], ['Missing Surface', '100']]
for names in groups:
    method = 'DSK/UNPRIORITIZED'
    if names:
        method += '/SURFACES = ' + ','.join('"' + name + '"' for name in names)
    for longitude, latitude in [(0., 0.), (0.5, 0.2), (-1., -0.4)]:
        try:
            point = spice.latsrf(method, 'MOON', 0., 'DSK_LOCAL', [[longitude, latitude]])[0]
            output = 'N|' + '|'.join(struct.pack('>d', value).hex() for value in point)
        except spice.utils.exceptions.SpiceyError as error:
            output = 'E|' + error.short
        print(f'{";".join(names)}|{longitude}|{latitude}|{output}')
spice.kclear()
