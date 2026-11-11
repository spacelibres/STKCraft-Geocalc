"""N0067 native SPKW19 writer and SPKGEO subtype/interval-boundary oracle."""
from pathlib import Path
import csv,random,ctypes as c
import spiceypy as s
from spiceypy.utils.libspicehelper import libspice
root=Path(__file__).resolve().parents[1];out=root/'tests/golden';path=out/'spk19.bsp'
if path.exists():path.unlink()
r=random.Random(19067);h=s.spkopn(str(path),'SPK19 native source oracle',0);queries=[];index=0
writer=libspice.spkw19_;writer.argtypes=[c.c_void_p]*15+[c.c_int,c.c_int]
def ints(values):return (c.c_int*len(values))(*values)
def doubles(values):return (c.c_double*len(values))(*values)
for last in [0,1]:
    for intervals in [1,3,205,10205]:
        body=719000+index;frame=s.frmnam(index%21+1).encode();index+=1
        boundaries=[float(i*100) for i in range(intervals+1)]
        counts=[];subtypes=[];degrees=[];packets=[];epochs=[]
        for i in range(intervals):
            subtype=i%3;n=205 if intervals==3 and i==1 else 17 if intervals<1000 else 2
            counts.append(n);subtypes.append(subtype);degrees.append([15,15,15][subtype])
            ep=[boundaries[i]+100.*j/(n-1) for j in range(n)];epochs.extend(ep)
            packets.extend(r.uniform(-100,100) for _ in range(n*(12 if subtype==0 else 6)))
        values=[c.c_int(h),c.c_int(body),c.c_int(0),c.c_double(boundaries[0]),c.c_double(boundaries[-1]),c.c_int(intervals),c.c_int(last)]
        hh,bb,cc,first,stop,ni,choose=values
        writer(c.byref(hh),c.byref(bb),c.byref(cc),frame,c.byref(first),c.byref(stop),b'19 native',c.byref(ni),ints(counts),ints(subtypes),ints(degrees),doubles(packets),doubles(epochs),doubles(boundaries),c.byref(choose),len(frame),9)
        if s.failed():raise RuntimeError(s.getmsg('LONG',2048))
        times=[0.,1e-8,12.345,99.99999999,100.,boundaries[-1]-1e-8,boundaries[-1]]
        if intervals>1:times += [boundaries[1]-1e-8,boundaries[1],boundaries[1]+1e-8,boundaries[2],boundaries[2]+1e-8]
        if intervals>100:times += [boundaries[99],boundaries[100],boundaries[-2],boundaries[-100]]
        queries.extend((body,t) for t in times if t<=boundaries[-1])
s.spkcls(h);s.furnsh(str(path))
with (out/'spk19.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['body','et','frame','x','y','z','vx','vy','vz'])
    for body,et in queries+list(reversed(queries)):
        for frame in range(1,22):
            state,_=s.spkgeo(body,et,s.frmnam(frame),0);w.writerow([body,repr(et),frame,*[repr(float(x)) for x in state]])
s.kclear()
