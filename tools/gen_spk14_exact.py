"""Native SPK14 variable-interval Chebyshev and generic-segment oracle."""
from pathlib import Path
import csv,random
import spiceypy as s
root=Path(__file__).resolve().parents[1];out=root/'tests/golden';path=out/'spk14.bsp'
if path.exists():path.unlink()
r=random.Random(14067);h=s.spkopn(str(path),'SPK14 source oracle',0);queries=[];index=0
for degree in [0,1,7,27]:
    for n in [1,17,205]:
        body=714000+index;frame=index%21+1;index+=1
        bounds=[-1000.]
        for _ in range(n):bounds.append(bounds[-1]+r.uniform(20,200))
        records=[]
        for start,end in zip(bounds,bounds[1:]):
            records.extend([(start+end)/2.,(end-start)/2.,*[r.uniform(-1e6,1e6) for _ in range(6*(degree+1))]])
        s.spk14b(h,str(body),body,0,s.frmnam(frame),bounds[0],bounds[-1],degree)
        s.spk14a(h,n,records,bounds[:-1]);s.spk14e(h)
        times=[bounds[0],bounds[0]+1e-8,bounds[-1]-1e-8,bounds[-1],bounds[0]+3.125]
        if n>1:times += [bounds[1]-1e-8,bounds[1],bounds[1]+1e-8,bounds[n//2]]
        if n>100:times += [bounds[99],bounds[100],bounds[101]]
        queries.extend((body,t) for t in times)
s.spkcls(h);s.furnsh(str(path))
with (out/'spk14.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['body','et','frame','x','y','z','vx','vy','vz'])
    for body,et in queries:
        for frame in range(1,22):
            state,_=s.spkgeo(body,et,s.frmnam(frame),0);w.writerow([body,repr(et),frame,*[repr(float(x)) for x in state]])
s.kclear()
