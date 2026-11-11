"""N0067 body aliases from ZZIDMAP, each independently checked by BODS2C."""
from pathlib import Path
import re,json,csv
import spiceypy as s
ROOT=Path(__file__).resolve().parents[1]
source=(ROOT/'target/reference/cached-n0067/cspice/src/cspice/zzidmap.c').read_text()
codes={int(i):int(v) for i,v in re.findall(r'bltcod\[(\d+)\]\s*=\s*(-?\d+)',source)}
names={int(off or 0)//36:''.join(re.findall(r'"(.*?)"',value)) for off,value in re.findall(r's_copy\(bltnam(?: \+ (\d+))?,\s*((?:"[^"]*"\s*)+),\s*\(ftnlen\)36',source)}
assert codes.keys()==names.keys();assert len(names)==692
for i,name in names.items():assert s.bods2c(name)==codes[i],(i,name)
(ROOT/'src/frames/frame_body_names.rs').write_text('// Generated from N0067 ZZIDMAP, independently checked by native BODS2C.\npub(super) const NAMES:&[(&str,i32)]=&[\n'+''.join(f'    ({json.dumps(names[i])},{codes[i]}),\n' for i in sorted(names))+'];\n',encoding='utf-8')
text="\\begindata\nNAIF_BODY_NAME=('MY_EARTH')\nNAIF_BODY_CODE=(399)\nNAIF_BODY_NAME += 'MY_MOON'\nNAIF_BODY_CODE += 301\nNAIF_BODY_NAME+=('MY_EARTH' 'A   moon')\nNAIF_BODY_CODE+=(499 301)\nNAIF_BODY_NAME+='MY_LUNA'\nNAIF_BODY_CODE+=301\nOBJECT_MY_LUNA_FRAME='ECLIPJ2000'\n"
(ROOT/'tests/golden/frame_body_aliases.tf').write_text(text,encoding='utf-8');s.furnsh(str(ROOT/'tests/golden/frame_body_aliases.tf'))
with (ROOT/'tests/golden/frame_body_aliases.csv').open('w',newline='',encoding='utf-8') as f:
 w=csv.writer(f);w.writerow(['name','code'])
 for name in ['MY_EARTH','MY_MOON','A moon','  a     moon ','Moon','CAS','+00399','Rstar','SOLAR SYSTEM BARYCENTER']:w.writerow([name,s.bods2c(name)])
assert s.cidfrm(301)==(17,'ECLIPJ2000')
print('692 native body aliases,9 append/priority lookups,CIDFRM preferred alias checked')

import subprocess
subprocess.run(["rustfmt","--edition","2021",str(ROOT/"src/frames/frame_body_names.rs")],check=True)
