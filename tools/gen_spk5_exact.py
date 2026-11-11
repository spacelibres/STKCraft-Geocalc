"""Independent native PROP2B/STMP03/SPK5 conic propagation oracle."""
from pathlib import Path
import csv,random,math
import spiceypy as s
root=Path(__file__).resolve().parents[1];out=root/'tests/golden';path=out/'spk5.bsp'
if path.exists():path.unlink()
r=random.Random(5067);h=s.spkopn(str(path),'SPK5 two-body source',0);queries=[];prop=[];index=0
for e in [0.,.2,.9,.999999999,1.,1.000000001,1.5,3.]:
    for n in [1,2,100,205]:
        body=705000+index;frame=index%21+1;index+=1;gm=398600.435436
        epochs=[-1000.+i*100. for i in range(n)]
        states=[s.conics([7000.+i*.1,e,.1+i*.0001,.2,.3,.4,0.,gm],t) for i,t in enumerate(epochs)]
        s.spkw05(h,body,0,s.frmnam(frame),epochs[0]-200.,epochs[-1]+200.,str(body),gm,n,states,epochs)
        times=[epochs[0]-200.,epochs[0]-1e-8,epochs[0],epochs[0]+1e-8,epochs[0]+12.345,epochs[-1],epochs[-1]+200.]
        if n>1:times += [epochs[1],epochs[1]-1e-8,epochs[n//2]+.125]
        if n>100:times += [epochs[99],epochs[99]+1e-8,epochs[100]]
        queries.extend((body,t) for t in times)
        state=states[0]
        for dt in [0.,1e-15,-1e-12,1.,-1.,86400.,-86400.,1e8,-1e8]:
            actual=s.prop2b(gm,state,dt);prop.append([repr(gm),*[repr(float(x)) for x in state],repr(dt),*[repr(float(x)) for x in actual]])
s.spkcls(h);s.furnsh(str(path))
with (out/'spk5.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['body','et','frame','x','y','z','vx','vy','vz'])
    for body,et in queries:
        for frame in range(1,22):
            state,_=s.spkgeo(body,et,s.frmnam(frame),0);w.writerow([body,repr(et),frame,*[repr(float(x)) for x in state]])
with (out/'prop2b.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['gm','px','py','pz','vx','vy','vz','dt','x','y','z','dx','dy','dz']);w.writerows(prop)
s.kclear()
