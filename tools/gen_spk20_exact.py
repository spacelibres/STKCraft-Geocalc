"""Native N0067 type20 record/scale/integral/source-frame oracle."""
from pathlib import Path
import csv,random
import numpy as np
import spiceypy as s
root=Path(__file__).resolve().parents[1];out=root/'tests/golden';path=out/'spk20.bsp'
if path.exists():path.unlink()
r=random.Random(56017);h=s.spkopn(str(path),'SPK20 exact source',0);queries=[]
for degree in [0,1,2,5,27]:
    for index,(ds,ts,fr) in enumerate([(1.,1.,0.),(13.7,86400.,.123456789),(.001,864.,.75)]):
        frame=(degree*3+index*7)%21+1;body=720000+degree*3+index
        intlen=.125;start=fr*86400.;end=start+3*intlen*86400.
        coefficients=np.array([r.uniform(-20,20) for _ in range(3*3*(degree+2))])
        s.spkw20(h,body,0,s.frmnam(frame),start,end,str(body),intlen,3,degree,coefficients,ds,ts,2451545.,fr)
        queries.extend((body,frame,t) for t in [start,start+1e-8,start+123.456,start+10800.,start+16200.,end-1e-8,end])
s.spkcls(h);s.furnsh(str(path))
with (out/'spk20.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['body','et','frame','x','y','z','vx','vy','vz'])
    for body,_,et in queries:
        for frame in range(1,22):
            state,_=s.spkgeo(body,et,s.frmnam(frame),0);w.writerow([body,repr(et),frame,*[repr(float(x)) for x in state]])
s.kclear()
