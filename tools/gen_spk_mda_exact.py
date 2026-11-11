"""Native N0067 SPKW01/SPKW21 modified difference-line oracle."""
from pathlib import Path
import csv,random,ctypes as c
import spiceypy as s
from spiceypy.utils.libspicehelper import libspice
root=Path(__file__).resolve().parents[1];out=root/'tests/golden';path=out/'spk_mda.bsp'
if path.exists():path.unlink()
r=random.Random(21001);h=s.spkopn(str(path),'SPK difference lines',0);queries=[];index=0
for kind,dimension in [(1,15),(21,15),(21,20),(21,25)]:
    for n in [1,17,100,205]+([10205] if dimension==25 else []):
        body=721000+index;frame=s.frmnam(index%21+1).encode();index+=1
        epochs=[-900.+i*100. for i in range(n)];records=[]
        for epoch in epochs:
            maximum=r.randint(3,dimension if kind==1 else dimension+1)
            orders=[r.randint(1,maximum-1) for _ in range(3)]
            reference=[r.uniform(-1e7,1e7),r.uniform(-10,10),r.uniform(-1e7,1e7),r.uniform(-10,10),r.uniform(-1e7,1e7),r.uniform(-10,10)]
            records.extend([epoch,*[100.*(j+1) for j in range(dimension)],*reference,*[r.uniform(-1e-8,1e-8) for _ in range(dimension*3)],float(maximum),*map(float,orders)])
        hh=c.c_int(h);bb=c.c_int(body);cc=c.c_int(0);first=c.c_double(-1000.);last=c.c_double(epochs[-1]);nn=c.c_int(n);size=c.c_int(4*dimension+11)
        data=(c.c_double*len(records))(*records);ep=(c.c_double*n)(*epochs)
        writer=getattr(libspice,f'spkw{kind:02d}_')
        if kind==1:
            writer.argtypes=[c.c_void_p]*10+[c.c_int,c.c_int]
            writer(c.byref(hh),c.byref(bb),c.byref(cc),frame,c.byref(first),c.byref(last),b'MDA native',c.byref(nn),data,ep,len(frame),10)
        else:
            writer.argtypes=[c.c_void_p]*11+[c.c_int,c.c_int]
            writer(c.byref(hh),c.byref(bb),c.byref(cc),frame,c.byref(first),c.byref(last),b'MDA native',c.byref(nn),c.byref(size),data,ep,len(frame),10)
        if s.failed():raise RuntimeError(s.getmsg('LONG',2048))
        times=[-1000.,-999.99999999,epochs[0]-1e-8,epochs[0],epochs[-1]-1e-8,epochs[-1]]
        if n>1:times += [epochs[1],epochs[n//2],epochs[n//2]-.125]
        if n>100:times += [epochs[99],epochs[99]+1e-8,epochs[100]]
        queries.extend((body,t) for t in times)
s.spkcls(h);s.furnsh(str(path))
with (out/'spk_mda.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['body','et','frame','x','y','z','vx','vy','vz'])
    for body,et in queries:
        for frame in range(1,22):
            state,_=s.spkgeo(body,et,s.frmnam(frame),0);w.writerow([body,repr(et),frame,*[repr(float(x)) for x in state]])
s.kclear()
