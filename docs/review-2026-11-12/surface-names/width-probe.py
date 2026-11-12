"""Capture fixed-width SRFSCC name translation from CSPICE N0067."""
import hashlib
import json
from pathlib import Path
import spiceypy as spice
import spiceypy.utils.libspicehelper as helper

names=['A'*36+'Z','A'*35+' B','A'*34+'  B']
queries=[*names,'A'*36,'A'*35+' ','A'*34,'A'*34+' B','A'*35+' B    X']
spice.kclear()
spice.pcpool('NAIF_SURFACE_NAME',names)
spice.pipool('NAIF_SURFACE_CODE',[1,2,3])
spice.pipool('NAIF_SURFACE_BODY',[301,301,301])
results=[]
with spice.no_found_check():
    for query in queries:
        code,found=spice.srfscc(query,301)
        results.append({'query':query,'code':code,'found':found})
report={'toolkit':spice.tkvrsn('TOOLKIT'),
        'native_library_sha256':hashlib.sha256(Path(helper.libspice._name).read_bytes()).hexdigest(),
        'names':names,'codes':[1,2,3],'bodies':[301,301,301],'results':results}
spice.kclear()
print(json.dumps(report,indent=2))
