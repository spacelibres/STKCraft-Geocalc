"""Download/verify the actual Cassini2012 subset used by both live FOV searches."""
from pathlib import Path
import argparse,subprocess,concurrent.futures,hashlib,json
BASE='https://naif.jpl.nasa.gov/pub/naif/pds/data/co-s_j_e_v-spice-6-v1.0/cosp_1000/data/'
FILES=['fk/cas_v44.tf','ik/cas_iss_v10.ti','sclk/cas00172.tsc','spk/200128RU_SCPSE_12280_12328.bsp','ck/12289_12294ra.bc','pck/pck00010.tpc','pck/enceladus_ssd_250728_v2.tpc']
def main():
 parser=argparse.ArgumentParser();parser.add_argument('--verify-only',action='store_true');parser.add_argument('--proxy',default='http://127.0.0.1:52345');args=parser.parse_args()
 manifest=Path('docs/review-2026-11-11/cassini-fov-kernel-provenance.json');old=json.loads(manifest.read_text(encoding='utf-8'))if manifest.exists()else{};old=old if isinstance(old,dict)else{'downloadedCoveringSubset':old};known={r['sourcePath']:r['sha256']for r in old.get('downloadedCoveringSubset',[])}
 def get(rel):
  path=Path('target/kernels/cassini')/Path(rel).name;path.parent.mkdir(parents=True,exist_ok=True)
  if args.verify_only:raw=path.read_bytes()
  else:
   temporary=path.with_suffix(path.suffix+'.download');subprocess.run(['curl.exe','--silent','--show-error','--fail','--max-time','180','--proxy',args.proxy,BASE+rel,'-o',str(temporary)],check=True);raw=temporary.read_bytes()
  digest=hashlib.sha256(raw).hexdigest()
  if rel in known and digest!=known[rel]:raise RuntimeError(f'Pinned kernel hash mismatch: {rel}')
  if not args.verify_only:temporary.replace(path)
  return {'sourcePath':rel,'url':BASE+rel,'path':path.as_posix(),'bytes':len(raw),'sha256':digest}
 with concurrent.futures.ThreadPoolExecutor(max_workers=4)as pool:records=list(pool.map(get,FILES))
 if not args.verify_only:old['downloadedCoveringSubset']=records;manifest.write_text(json.dumps(old,indent=2)+'\n',encoding='utf-8')
 for record in records:print(record['sourcePath'],record['bytes'],record['sha256'])
if __name__=='__main__':main()
