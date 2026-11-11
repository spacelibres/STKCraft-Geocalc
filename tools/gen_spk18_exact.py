"""Native N0067 SPK18 subtype0/1 oracle, clipped windows and directories."""
from pathlib import Path
import csv,random
import spiceypy as s
root=Path(__file__).resolve().parents[1];out=root/'tests/golden';path=out/'spk18.bsp'
if path.exists():path.unlink()
r=random.Random(18067);h=s.spkopn(str(path),'SPK18 source oracle',0);queries=[];index=0
for subtype in [0,1]:
    for degree in ([3,7,11,15] if subtype==0 else [1,3,7,15]):
        for n in [2,17,205,10205]:
            body=718000+index;frame=index%21+1;index+=1
            epochs=[-1000.];
            for _ in range(n-1):epochs.append(epochs[-1]+r.uniform(1,30))
            packets=[[r.uniform(-100,100) for _ in range(12 if subtype==0 else 6)] for _ in epochs]
            s.spkw18(h,subtype,body,0,s.frmnam(frame),epochs[0],epochs[-1],str(body),degree,packets,epochs)
            times=[epochs[0],epochs[0]+1e-8,epochs[1],(epochs[0]+epochs[1])*.5,epochs[n//2],epochs[n//2]+.125,epochs[-1]-1e-8,epochs[-1]]
            if n>100:times += [epochs[99],epochs[99]+1e-8,epochs[100],epochs[-101]]
            queries.extend((body,min(epochs[-1],t)) for t in times)
s.spkcls(h);s.furnsh(str(path))
with (out/'spk18.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['body','et','frame','x','y','z','vx','vy','vz'])
    for body,et in queries:
        for frame in range(1,22):
            state,_=s.spkgeo(body,et,s.frmnam(frame),0);w.writerow([body,repr(et),frame,*[repr(float(x)) for x in state]])
s.kclear()
