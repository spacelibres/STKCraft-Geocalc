import concurrent.futures,re,subprocess,json,hashlib
from pathlib import Path
files=[x for x in re.findall(r"'\$DATA/([^']+)'",Path('target/kernels/solar_system_v0072.tm').read_text())if x.startswith('fk/')]
def get(rel):
 path=Path('target/kernels')/Path(rel).name
 if not path.exists():subprocess.run(['curl.exe','--silent','--show-error','--fail','--max-time','30','--proxy','http://127.0.0.1:52345','https://naif.jpl.nasa.gov/pub/naif/pds/wgc/kernels/'+rel,'-o',str(path)],check=True)
 return {'relativePath':rel,'path':str(path),'bytes':path.stat().st_size,'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}
with concurrent.futures.ThreadPoolExecutor(max_workers=4)as pool:records=list(pool.map(get,files))
Path('docs/review-2026-11-11/wgc-kernel-fk-provenance.json').write_text(json.dumps(records,indent=2)+'\n')
import spiceypy as s
for record in records:s.furnsh(record['path'])
print('Loaded all',len(records),'actualv0072 FK files; CIDFRM399=',s.cidfrm(399),'CIDFRM301=',s.cidfrm(301))
for record in records:
 text=Path(record['path']).read_text(errors='replace')
 if 'OBJECT_EARTH_FRAME' in text or 'OBJECT_399_FRAME' in text:print('EARTH override',record['path'])
