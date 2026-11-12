"""Capture deterministic long-name and whitespace lookups using CSPICE."""
import random
import string
from pathlib import Path
import spiceypy as spice

rng = random.Random(260112)
folder = Path(__file__).parent
names = []
for _ in range(100):
    names.append(''.join(rng.choice(string.ascii_letters + '   ') for _ in range(rng.randrange(5, 65))).strip())
names += ['Alpha Surface', 'Alpha   Surface', 'A' * 36 + 'X', 'A' * 36 + 'Y']
codes = list(range(1000, 1000 + len(names)))
kernel = 'KPL/FK\n\\begindata\nNAIF_SURFACE_NAME = (\n' + '\n'.join("'" + n + "'" for n in names)
kernel += '\n)\nNAIF_SURFACE_CODE = (\n' + '\n'.join(map(str, codes))
kernel += '\n)\nNAIF_SURFACE_BODY = (\n' + '\n'.join(['301'] * len(names)) + '\n)\n\\begintext\n'
path = folder / 'mapping-fuzz.tf'
path.write_text(kernel, encoding='ascii', newline='\n')
queries = []
for name in names:
    queries += [name, name.upper(), '  ' + name + '  ', name.replace(' ', '   ')]
queries += ['A' * 36, 'A' * 36 + 'Z', 'not present']
spice.kclear()
spice.furnsh(str(path))
print('name\texpected')
with spice.no_found_check():
    for query in queries:
        value, found = spice.srfscc(query, 301)
        print(f'{query}\t{f"I:{value}" if found else "N"}')
spice.kclear()
