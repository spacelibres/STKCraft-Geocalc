import hashlib,json,spiceypy as s,spiceypy.utils.libspicehelper as lib
from pathlib import Path
s.kclear(); records=[]
with s.no_found_check():
 for name in ['100',' 100 ','+100','-3','01','abc','']:
  try:code,found=s.srfscc(name,301);result={'code':code,'found':found}
  except Exception as e:result={'error':getattr(e,'short',type(e).__name__)}
  records.append({'phase':'empty','name':name,'body':301,**result})
 s.pcpool('NAIF_SURFACE_NAME',['Alpha  Surface','ALPHA SURFACE','Other'])
 s.pipool('NAIF_SURFACE_CODE',[100,101,102]);s.pipool('NAIF_SURFACE_BODY',[301,301,399])
 for name in ['alpha surface','  Alpha   Surface  ','ALPHA  SURFACE','other','100']:
  for body in [301,399]:
   try:code,found=s.srfscc(name,body);result={'code':code,'found':found}
   except Exception as e:result={'error':getattr(e,'short',type(e).__name__)}
   records.append({'phase':'assigned','name':name,'body':body,**result})
 report={'toolkit':s.tkvrsn('TOOLKIT'),'native_library_sha256':hashlib.sha256(Path(lib.libspice._name).read_bytes()).hexdigest(),'assignments':{'NAIF_SURFACE_NAME':['Alpha  Surface','ALPHA SURFACE','Other'],'NAIF_SURFACE_CODE':[100,101,102],'NAIF_SURFACE_BODY':[301,301,399]},'records':records}
s.kclear();print(json.dumps(report,indent=2))
