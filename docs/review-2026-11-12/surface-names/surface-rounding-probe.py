import hashlib,json,spiceypy as s,spiceypy.utils.libspicehelper as lib
from pathlib import Path
records=[]
with s.no_found_check():
 for value in [-2.5,-1.5,-0.5,0.5,1.5,2.5,2147483647.4,2147483648.0]:
  s.kclear();s.pcpool('NAIF_SURFACE_NAME',['A']);s.pdpool('NAIF_SURFACE_CODE',[value]);s.pipool('NAIF_SURFACE_BODY',[301])
  try:code,found=s.srfscc('A',301);out={'code':code,'found':found}
  except Exception as e:out={'error':getattr(e,'short',type(e).__name__)}
  records.append({'surface_code_pool_value':value,**out})
 report={'toolkit':s.tkvrsn('TOOLKIT'),'native_library_sha256':hashlib.sha256(Path(lib.libspice._name).read_bytes()).hexdigest(),'records':records}
s.kclear();print(json.dumps(report,indent=2))
