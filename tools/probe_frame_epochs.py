"""Capture WGC frame epoch and angular-velocity-unit behavior."""
from pathlib import Path
import importlib.util,json
import spiceypy as s
root=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('wgc',root/'tools/wgc2_geometry_compare.py')
w=importlib.util.module_from_spec(spec);spec.loader.exec_module(w)
dest=root/'docs/review-2026-11-11/frame-epoch-probes';dest.mkdir(exist_ok=True)
client=w.Wgc(True,60,curl=True,proxy='http://127.0.0.1:52345',trace=dest/'requests.jsonl')
s.furnsh(str(root/'target/kernels/de442s.bsp'));s.furnsh(str(root/'target/kernels/pck00011.tpc'))
base={'kernels':[{'type':'KERNEL_SET','id':1}], 'calculationType':'FRAME_TRANSFORMATION',
      'timeSystem':'TDB','timeFormat':'SECONDS_PAST_J2000','times':['0.0','500000000.0'],
      'outputTimeSystem':'TDB','outputTimeFormat':'SECONDS_PAST_J2000',
      'frame1':'IAU_EARTH','frame2':'IAU_MOON','orientationRepresentation':'MATRIX_ALL_ONE_ROW',
      'angularVelocityRepresentation':'MATRIX','angularUnits':'rad','angularVelocityUnits':'rad/s'}
records=[]
for correction in ['NONE','LT','CN','XLT','XCN']:
    for locus in ['FRAME1','FRAME2']:
        payload={**base,'aberrationCorrection':correction,'timeLocation':locus}
        result=client.run(payload)
        record={'payload':payload,'result':result,'oracle':[]}
        for et in [0.,5e8]:
            if correction=='NONE':lt=0.
            else:_,lt=s.spkezp(301 if locus=='FRAME1' else 399,et,'J2000',correction,399 if locus=='FRAME1' else 301)
            shifted=et+(1. if correction.startswith('X') else -1.)*lt
            t1,t2=(et,shifted) if locus=='FRAME1' else (shifted,et)
            a=s.sxform('IAU_EARTH','J2000',t1);b=s.sxform('J2000','IAU_MOON',t2)
            matrix=s.mxmg(b,a)
            record['oracle'].append({'et':et,'time1':t1,'time2':t2,'lt':lt,'xform_unscaled':matrix.tolist(),
                                     'pxfrm2':s.pxfrm2('IAU_EARTH','IAU_MOON',t1,t2).tolist()})
        records.append(record)
        (dest/'results.json').write_text(json.dumps(records,indent=2,default=str))
        print(correction,locus,'captured',flush=True)
for units in ['rad/s','deg/s','RPM','unitless','Unitary']:
    payload={**base,'aberrationCorrection':'NONE','timeLocation':'FRAME1','orientationRepresentation':'ANGLE_AND_AXIS',
             'angularUnits':'rad','angularVelocityRepresentation':'VECTOR_IN_FRAME1','angularVelocityUnits':units}
    records.append({'payload':payload,'result':client.run(payload)})
    print(units,'captured',flush=True)
(dest/'results.json').write_text(json.dumps(records,indent=2,default=str))
s.kclear()
