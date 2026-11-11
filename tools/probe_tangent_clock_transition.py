"""Probe WGC tangent LTST epoch rules; save raw reference transport and candidates."""
import concurrent.futures
import json
from pathlib import Path
import spiceypy as s
from wgc2_geometry_compare import Wgc,time_block
OUT=Path('docs/review-2026-11-11/tangent-clock-transition');OUT.mkdir(parents=True,exist_ok=True)
def case(item):
 index,(et,corr,locus)=item
 payload={'calculationType':'TANGENT_POINT','kernels':[{'type':'KERNEL_SET','id':1}],'target':'MOON','observer':'EARTH','targetFrame':'IAU_MOON','aberrationCorrection':corr,'computationMethod':'ELLIPSOID','correctionLocus':locus,'directionVectorType':'DIRECTION_TO_OBJECT','directionObject':'MOON','coordinateRepresentation':'PLANETOGRAPHIC',**time_block([et])}
 try:
  result=Wgc(False,45,curl=True,proxy='http://127.0.0.1:52345',trace=OUT/f'{index:02}.jsonl').run(payload)
  return {'request':payload,'result':result}
 except Exception as e:return {'request':payload,'error':str(e)}
items=[(et,'NONE','TANGENT_POINT') for et in [.5,1.,2.,3.,5.,10.]]
with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:records=list(pool.map(case,enumerate(items)))
s.furnsh('target/kernels/de442s.bsp');s.furnsh('target/kernels/pck00011.tpc')
for record in records:
 if 'result' not in record:continue
 ids=[c['outputID'] for c in record['result'].get('columns',[])];rows=record['result'].get('rows',[])
 if not rows:continue
 values=dict(zip(ids,rows[0]));et=float(record['request']['times'][0]);lt=float(values['LIGHT_TIME']);lon=float(values['SURFACE_POINT_LONGITUDE'])*s.rpd();epoch=float(values['TIME_AT_POINT']);observed=values['LTST']
 candidates={name:s.et2lst(t,301,lon,'PLANETOGRAPHIC')[3] for name,t in [('observer',et),('point_epoch',epoch),('minus_lt',et-lt),('minus_2lt',et-2*lt),('plus_lt',et+lt)]}
 record['clock_candidates']=candidates;record['observed']=observed;record['matches']=[name for name,value in candidates.items() if value==observed]
 print(et,record['request']['aberrationCorrection'],record['request']['correctionLocus'],observed,candidates,flush=True)
s.kclear();(OUT/'report.json').write_text(json.dumps(records,indent=2,default=str)+'\n')
