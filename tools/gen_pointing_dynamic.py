"""Independent native PXFORM direction rotations in dynamic frames."""
from pathlib import Path
import csv
import spiceypy as s
ROOT=Path(__file__).resolve().parents[1]
for file in ['target/kernels/de440s.bsp','target/kernels/pck00011.tpc','tests/golden/spk_dynamic_sources.bsp','tests/golden/dynamic_euler.tf','tests/golden/dynamic_two.tf','tests/golden/dynamic_of_date.tf']:
    s.furnsh(str(ROOT/file))
with (ROOT/'tests/golden/pointing_dynamic.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['kind','input','output','et','x','y','z','observer','correction'])
    for kind in ['VECTOR','VELOCITY']:
        for source in ['J2000','DYN_EULER_0','DYN_EULER_12','DYN_EULER_13','DYN_TWO_0','OF_DATE_0']:
            for output in ['J2000','DYN_EULER_0','DYN_EULER_12','DYN_EULER_13','DYN_TWO_0','OF_DATE_0']:
                for et in [-12345.67,0.,23456.78]:
                    vector=[123.456789,-987.654321,17.013] if kind=='VECTOR' else s.spkezr('91002',et,source,'NONE','399')[0][3:]
                    result=s.mxv(s.pxform(source,output,et),vector)
                    w.writerow([kind,source,output,repr(et),*[repr(float(v)) for v in result],399,'NONE'])
    for observer in [399,91002]:
        for correction in ['S','XS']:
            for source in ['J2000','DYN_EULER_0','DYN_EULER_13','OF_DATE_0']:
                for output in ['J2000','DYN_EULER_0','DYN_EULER_13']:
                    for et in [-12345.67,0.,23456.78]:
                        vector=s.mxv(s.pxform(source,'J2000',et),[123.456789,-987.654321,17.013])
                        observer_state=s.spkezr(str(observer),et,'J2000','NONE','0')[0]
                        vector=s.stelab(vector,observer_state[3:]) if correction=='S' else s.stlabx(vector,observer_state[3:])
                        result=s.mxv(s.pxform('J2000',output,et),vector)
                        w.writerow(['VECTOR',source,output,repr(et),*[repr(float(v)) for v in result],observer,correction])
s.kclear()
