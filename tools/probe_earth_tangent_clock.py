"""Capture genuine NASA Earth tangent clocks for the ITRF93 default frame."""
import concurrent.futures,json
from pathlib import Path
import sys
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'tools'))
from wgc2_geometry_compare import Wgc,time_block
OUT=Path('docs/review-2026-11-11/earth-tangent-clock');OUT.mkdir(parents=True,exist_ok=True)
items=[(et,corr,locus)for et in [-5e8,0.,5e8]for corr in ['NONE','LT','XLT']for locus in ['TANGENT_POINT','SURFACE_POINT']]
items += [(0.,corr,locus)for corr in ['LT+S','CN+S','XCN+S']for locus in ['TANGENT_POINT','SURFACE_POINT']]
def case(item):
 i,(et,corr,locus)=item
 p={'calculationType':'TANGENT_POINT','kernels':[{'type':'KERNEL_SET','id':1}],'target':'EARTH','observer':'MOON','targetFrame':'IAU_EARTH','aberrationCorrection':corr,'computationMethod':'ELLIPSOID','correctionLocus':locus,'directionVectorType':'DIRECTION_TO_OBJECT','directionObject':'EARTH','coordinateRepresentation':'PLANETOGRAPHIC',**time_block([et])}
 try:return {'request':p,'result':Wgc(False,60,curl=True,proxy='http://127.0.0.1:52345',trace=OUT/f'{i:02}.jsonl').run(p)}
 except Exception as e:return {'request':p,'error':str(e)}
with concurrent.futures.ThreadPoolExecutor(max_workers=2)as pool:records=list(pool.map(case,enumerate(items)))
(OUT/'report.json').write_text(json.dumps(records,indent=2,default=str)+'\n')
print(len(records),'actual NASA Earth clock probes; errors',sum('error'in r for r in records))
