import hashlib,json,spiceypy as s,spiceypy.utils.libspicehelper as lib
from pathlib import Path
s.kclear();records=[]
with s.no_found_check():
 def query(phase):
  for name in ['A','B','100']:
   try:code,found=s.srfscc(name,301);out={'code':code,'found':found}
   except Exception as e:out={'error':getattr(e,'short',type(e).__name__)}
   records.append({'phase':phase,'name':name,**out})
 query('empty')
 s.pcpool('NAIF_SURFACE_NAME',['A']);s.pipool('NAIF_SURFACE_CODE',[100]);s.pipool('NAIF_SURFACE_BODY',[301]);query('initial')
 s.pcpool('NAIF_SURFACE_NAME',['B']);query('rename')
 s.pipool('NAIF_SURFACE_CODE',[101]);query('recode')
 s.dvpool('NAIF_SURFACE_BODY');query('deleted_body')
 s.pipool('NAIF_SURFACE_BODY',[301]);query('restored_body')
 report={'toolkit':s.tkvrsn('TOOLKIT'),'native_library_sha256':hashlib.sha256(Path(lib.libspice._name).read_bytes()).hexdigest(),'records':records}
s.kclear();print(json.dumps(report,indent=2))
