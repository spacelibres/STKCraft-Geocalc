"""Fetch and verify pinned CSPICE N0067 and the complete local-gate kernel subset.

This tool never rewrites provenance manifests. The Windows DLL is supplied by
SpiceyPy 8.3.0; its bytes, toolkit version, archive and required kernels must all
match the committed records. Linux/macOS build checks are not exactness claims.
"""
from __future__ import annotations
import argparse,concurrent.futures,hashlib,json,os,platform,re,shutil,sys,time,urllib.request,zipfile
from pathlib import Path,PurePosixPath
ROOT=Path(__file__).resolve().parents[1]
NAIF='https://naif.jpl.nasa.gov/pub/naif/'
SNAPSHOTS={
 'earth_000101_270205_261110.bpc':'docs/reference/kernel-snapshots/earth_000101_270205_261110.bpc',
}
GENERIC={
 'de440s.bsp':'spk/planets/de440s.bsp','de442s.bsp':'spk/planets/de442s.bsp',
 'de442.bsp':'spk/planets/de442.bsp','gm_de431.tpc':'pck/gm_de431.tpc',
 'gm_de440.tpc':'pck/gm_de440.tpc','mars_iau2000_v1.tpc':'pck/mars_iau2000_v1.tpc',
 'naif0012.tls':'lsk/naif0012.tls','pck00011.tpc':'pck/pck00011.tpc',
 'moon_080317.tf':'fk/satellites/moon_080317.tf',
 'moon_de440_250416.tf':'fk/satellites/moon_de440_250416.tf',
 'moon_assoc_me.tf':'fk/satellites/moon_assoc_me.tf',
 'moon_pa_de440_200625.bpc':'pck/moon_pa_de440_200625.bpc',
 'earth_000101_270205_261110.bpc':'pck/earth_000101_270205_261110.bpc',
}
def digest(path):
 h=hashlib.sha256()
 with Path(path).open('rb') as f:
  for block in iter(lambda:f.read(1024*1024),b''):h.update(block)
 return h.hexdigest()
def pinned_kernel_records(manifest,root=ROOT):
 root=Path(root);records={}
 def add(name,url,sha):
  name=name.replace('\\','/');parts=PurePosixPath(name)
  if parts.is_absolute() or '..' in parts.parts or ':' in name:raise ValueError(f'Unsafe kernel destination: {name}')
  if not re.fullmatch('[a-fA-F0-9]{64}',sha or ''):raise ValueError(f'Missing pinned SHA256: {name}')
  if not url.startswith(NAIF):raise ValueError(f'Kernel URL is not the official NAIF source: {url}')
  if name in records and records[name]['sha256']!=sha:raise ValueError(f'Conflicting pinned kernel hashes: {name}')
  records[name]={'name':name,'url':url,'sha256':sha}
  if name in SNAPSHOTS:records[name]['snapshot']=SNAPSHOTS[name]
 provenance=root/'docs/review-2026-11-11'
 wgc=json.loads((provenance/'wgc-kernel-provenance.json').read_text(encoding='utf-8'))
 for row in wgc['files']:
  # The mutable alias is represented by its captured dated equivalent.
  # A dated filename does not guarantee permanent availability at NAIF.
  if row['filename']=='earth_latest_high_prec.bpc':continue
  add(row['filename'],row['url'],row['sha256'])
 for name,sha in manifest.get('kernels_sha256',{}).items():
  if name in records:
   if records[name]['sha256']!=sha:raise ValueError(f'Conflicting pinned kernel hashes: {name}')
  elif name in GENERIC:add(name,NAIF+'generic_kernels/'+GENERIC[name],sha)
  elif not name.startswith('cassini/'):raise ValueError(f'No pinned official download URL for required kernel: {name}')
 fk=json.loads((provenance/'wgc-kernel-fk-provenance.json').read_text(encoding='utf-8'))
 for row in fk:add(Path(row['relativePath']).name,NAIF+'pds/wgc/kernels/'+row['relativePath'],row['sha256'])
 cassini=json.loads((provenance/'cassini-fov-kernel-provenance.json').read_text(encoding='utf-8'))
 for row in cassini['downloadedCoveringSubset']:add('cassini/'+Path(row['sourcePath']).name,row['url'],row['sha256'])
 if 'metakernel' in cassini:
  row=cassini['metakernel'];add('cassini/'+PurePosixPath(row['path']).name,row['url'],row['sha256'])
 for name,sha in manifest.get('kernels_sha256',{}).items():
  if name not in records:raise ValueError(f'Missing required pinned kernel: {name}')
  if records[name]['sha256']!=sha:raise ValueError(f'Conflicting pinned kernel hashes: {name}')
 if not records:raise ValueError('Pinned kernel inventory is empty')
 return [records[name] for name in sorted(records)]
def ensure_download(path,url,sha,verify_only=False,snapshot_path=None):
 path=Path(path)
 if path.is_file() and digest(path)==sha:return
 if verify_only:raise RuntimeError(f'Missing or changed pinned file: {path}')
 if snapshot_path is not None:
  snapshot=Path(snapshot_path)
  if not snapshot.is_file() or digest(snapshot)!=sha:
   raise RuntimeError(f'Missing or changed pinned snapshot: {snapshot}')
  path.parent.mkdir(parents=True,exist_ok=True);temporary=path.with_name(path.name+'.download')
  try:
   shutil.copyfile(snapshot,temporary)
   if digest(temporary)!=sha:raise RuntimeError(f'Copied pinned snapshot differs: {snapshot}')
   temporary.replace(path)
  finally:temporary.unlink(missing_ok=True)
  return
 path.parent.mkdir(parents=True,exist_ok=True);temporary=path.with_name(path.name+'.download')
 request=urllib.request.Request(url,headers={'User-Agent':'stkcraft-geocalc-reference-bootstrap/1.0'})
 for attempt in range(3):
  try:
   with urllib.request.urlopen(request,timeout=180) as response,temporary.open('wb') as output:
    for block in iter(lambda:response.read(1024*1024),b''):output.write(block)
   actual=digest(temporary)
   if actual!=sha:raise RuntimeError(f'SHA256 mismatch for {url}: expected {sha}, received {actual}')
   temporary.replace(path);return
  except RuntimeError:
   temporary.unlink(missing_ok=True);raise
  except (OSError,TimeoutError):
   temporary.unlink(missing_ok=True)
   if attempt==2:raise
   time.sleep(attempt+1)
def resolve_reference_binary(manifest):
 override=os.environ.get('STKCRAFT_REFERENCE_DLL')
 if override:return Path(override)
 recorded=Path(manifest.get('reference_binary',{}).get('path',''))
 if recorded.is_file():return recorded
 try:
  from spiceypy.utils.libspicehelper import libspice
 except (ImportError,OSError) as error:raise RuntimeError('Install the pinned Windows wheel: python -m pip install spiceypy==8.3.0') from error
 return Path(libspice._name)
def bootstrap(root=ROOT,verify_only=False,kernels_only=False,workers=4):
 root=Path(root);manifest_path=root/'docs/reference/CSPICE_N0067.json';before=manifest_path.read_bytes();manifest=json.loads(before)
 records=pinned_kernel_records(manifest,root)
 def kernel(row):
  path=root/'target/kernels'/row['name']
  snapshot=root/row['snapshot'] if 'snapshot' in row else None
  ensure_download(path,row['url'],row['sha256'],verify_only,snapshot_path=snapshot)
  print('Verified kernel '+row['name'],flush=True)
 with concurrent.futures.ThreadPoolExecutor(max_workers=workers) as pool:list(pool.map(kernel,records))
 report={'kernels':records,'platform':platform.platform(),'complete_webgeocalc_parity':False}
 if not kernels_only:
  if sys.platform!='win32':raise RuntimeError('The pinned native exactness reference is Windows x86_64; use --kernels-only for data-only setup')
  archive=root/'target/reference/cspice-n0067.zip';ensure_download(archive,manifest['official_url'],manifest['archive_sha256'],verify_only)
  source=root/'target/reference/n0067/cspice/src/cspice'
  valid=len(list(source.glob('*.c')))==manifest['C_source_count'] and all((source/name).is_file() and digest(source/name)==sha for name,sha in manifest['reference_source_sha256'].items())
  if not valid:
   if verify_only:raise RuntimeError('Independent reference source tree is missing or changed')
   destination=root/'target/reference/n0067'
   with zipfile.ZipFile(archive) as package:
    for member in package.infolist():
     path=PurePosixPath(member.filename.replace('\\','/'))
     if path.is_absolute() or '..' in path.parts or ':' in member.filename:raise RuntimeError('Unsafe path in reference archive')
    package.extractall(destination)
  if len(list(source.glob('*.c')))!=manifest['C_source_count']:raise RuntimeError('Independent C source inventory differs')
  for name,sha in manifest['reference_source_sha256'].items():
   if digest(source/name)!=sha:raise RuntimeError('Changed independent C reference: '+name)
  import spiceypy
  if spiceypy.__version__!=manifest['reference_spiceypy']:raise RuntimeError('SpiceyPy version differs from the pinned reference')
  if spiceypy.tkvrsn('TOOLKIT')!=manifest['reference_binary']['toolkit_version']:raise RuntimeError('Native CSPICE toolkit version differs')
  binary=resolve_reference_binary(manifest)
  if not binary.is_file() or digest(binary)!=manifest['reference_binary']['sha256']:raise RuntimeError('Native reference DLL does not match the pinned SHA256')
  report['reference_binary']={'runtime_path':str(binary),'sha256':digest(binary),'toolkit_version':spiceypy.tkvrsn('TOOLKIT')}
 if before!=manifest_path.read_bytes():raise RuntimeError('Canonical reference manifest changed during bootstrap')
 out=root/'target/bootstrap-report.json';out.parent.mkdir(parents=True,exist_ok=True);out.write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
 return report
def main():
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--verify-only',action='store_true');parser.add_argument('--kernels-only',action='store_true');parser.add_argument('--workers',type=int,default=4);args=parser.parse_args()
 if args.workers<1:parser.error('--workers must be positive')
 try:report=bootstrap(verify_only=args.verify_only,kernels_only=args.kernels_only,workers=args.workers)
 except (OSError,ValueError,RuntimeError,zipfile.BadZipFile) as error:print('Reference bootstrap failed: '+str(error),file=sys.stderr);return 1
 print(f"Verified {len(report['kernels'])} pinned kernel files; canonical manifest unchanged.");return 0
if __name__=='__main__':raise SystemExit(main())
