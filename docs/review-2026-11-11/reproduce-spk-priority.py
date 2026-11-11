"""Run from the project root after cargo build --bin wgc_geometry."""
import json
import os
import subprocess
from pathlib import Path
import spiceypy as spice

path = Path('target/audit-priority-reproduce.bsp')
if path.exists():
    path.unlink()
handle = spice.spkopn(str(path), 'priority audit', 0)
for value, name in [(1.0, 'older'), (2.0, 'newer')]:
    spice.spkw02(handle, 10001, 0, 'J2000', 0., 10., name, 10., 1, 1,
                 [value, 0., 0., 0., 0., 0.], 0.)
spice.spkcls(handle)
spice.furnsh(str(path))
expected = spice.spkgeo(10001, 5., 'J2000', 0)[0].tolist()
local = subprocess.run(['target/debug/wgc_geometry.exe', 'state'],
                       input='10001,0,5,NONE\n', capture_output=True, text=True,
                       check=True, env={**os.environ, 'STKCRAFT_SPK': str(path)})
print(json.dumps({'cspice': expected, 'local': local.stdout}, indent=2))
