"""Independent CSPICE probes for options absent from the live comparator."""
import json
import math
import os
import subprocess
from pathlib import Path
import spiceypy as spice

spice.furnsh('target/kernels/de440s.bsp')
spice.furnsh('target/kernels/pck00011.tpc')
env = {**os.environ, 'STKCRAFT_SPK': 'target/kernels/de440s.bsp'}
def local(mode, lines):
    p = subprocess.run(['target/debug/wgc_geometry.exe', mode, 'target/kernels/pck00011.tpc'], input='\n'.join(lines)+'\n', text=True, capture_output=True, env=env, check=True)
    return [row.split(',') for row in p.stdout.splitlines()]

records = []
for et in [-5e8, 0.0, 5e8]:
    for correction in ['NONE', 'LT+S', 'CN+S']:
        point = spice.georec(math.radians(10), math.radians(20), 0, 6378.1366, (6378.1366-6356.7519)/6378.1366)
        _, _, phase, incidence, emission = spice.ilumin('ELLIPSOID', 'EARTH', et, 'IAU_EARTH', correction, 'MOON', point)
        ours = local('illum', [f'399,301,10,20,0,{et},{correction}'])[0]
        actual = list(map(float, ours[7:10]))
        expected = [math.degrees(x) for x in [incidence, emission, phase]]
        records.append({'type': 'illumination', 'et': et, 'correction': correction, 'ours_degrees': actual, 'cspice_degrees': expected, 'max_error_degrees': max(abs(a-b) for a,b in zip(actual, expected))})
for observer in [301, 10]:
    et = 5e8
    point, epoch, vector = spice.subslr('NEAR POINT/ELLIPSOID', 'EARTH', et, 'IAU_EARTH', 'CN+S', str(observer))
    ours = local('subpoint', [f'solar,399,{observer},{et},CN+S'])[0]
    records.append({'type': 'subsolar_observer', 'observer': observer, 'et': et, 'raw_local': ours, 'cspice_point': list(map(float,point)), 'cspice_range': float(spice.vnorm(vector)), 'cspice_epoch': epoch})
Path('docs/review-2026-11-11/numeric-defects.json').write_text(json.dumps(records, indent=2))
for record in records:
    print(record)
