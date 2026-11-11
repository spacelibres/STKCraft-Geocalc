"""Use actual Cassini2012 kernels to capture both missing NASA GF families."""
import json
from pathlib import Path
import spiceypy as s
from wgc_gf_cases import load,payload,local_parameters,native_windows,ROOT,FK,PCK
from wgc_gf_compare import c
OUT=ROOT/'docs/review-2026-11-11/gf-all-families';records=json.loads((OUT/'cases.json').read_text())
s.kclear()
for name in ['pck00010.tpc','enceladus_ssd_250728_v2.tpc','cas_v44.tf','cas_iss_v10.ti','cas00172.tsc','200128RU_SCPSE_12280_12328.bsp','12289_12294ra.bc']:s.furnsh(str(ROOT/'target/kernels/cassini'/name))
s.furnsh(str(ROOT/'target/kernels/naif0012.tls'));s.furnsh(str(ROOT/'target/kernels/de442.bsp'));s.gfstol(1e-6)
selectors=[{'type':'KERNEL_SET','id':5},{'type':'KERNEL','path':'pds/wgc/kernels/spk/de442.bsp'}]
resources={'spk':'target/kernels/cassini/200128RU_SCPSE_12280_12328.bsp,target/kernels/de442.bsp','pck':'target/kernels/cassini/pck00010.tpc,target/kernels/cassini/enceladus_ssd_250728_v2.tpc','fk':FK+',target/kernels/cassini/cas_v44.tf','ik':'target/kernels/cassini/cas_iss_v10.ti','instrument':-82360,'ck':'target/kernels/cassini/12289_12294ra.bc','sclk':'-82|target/kernels/cassini/cas00172.tsc','lsk':'target/kernels/naif0012.tls'}
ray=s.radrec(1.,208.*s.rpd(),-10.*s.rpd())
new=[{'id':'target_fov','payload':payload('GF_TARGET_IN_INSTRUMENT_FOV_SEARCH',[(403902000.,403906000.)],60.,instrument='CASSINI_ISS_NAC',target='ENCELADUS',shape='ELLIPSOID',targetFrame='IAU_ENCELADUS',observer='CASSINI',aberrationCorrection='NONE')},
     {'id':'ray_fov','payload':payload('GF_RAY_IN_FOV_SEARCH',[(403902000.,403906000.)],60.,instrument='CASSINI_ISS_NAC',observer='CASSINI',directionVectorType='VECTOR_IN_REFERENCE_FRAME',directionFrame='J2000',directionVectorX=float(ray[0]),directionVectorY=float(ray[1]),directionVectorZ=float(ray[2]),aberrationCorrection='NONE')}]
results=[]
for case in new:
 case['payload']['kernels']=selectors;case['resources']=resources;case['local']=local_parameters(case);case['native']=native_windows(case)
 print(case['id'],'CSPICE',case['native'],flush=True)
 try:
  result=c.Wgc(False,90,curl=True,proxy='http://127.0.0.1:52345',trace=OUT/(case['id']+'.jsonl')).run(case['payload']);results.append({'id':case['id'],'result':result});print(case['id'],'NASA',result['rows'],flush=True)
 except Exception as error:results.append({'id':case['id'],'error':str(error)});print(case['id'],'NASA ERROR',str(error),flush=True)
records=[r for r in records if r['id']not in ['target_fov','ray_fov']]+new
(OUT/'cases.json').write_text(json.dumps(records,indent=2)+'\n')
(OUT/'cassini-nasa-results.json').write_text(json.dumps(results,indent=2,default=str)+'\n')
(OUT/'cassini-queries.csv').write_text('\n'.join(c['local']for c in new)+'\n')
