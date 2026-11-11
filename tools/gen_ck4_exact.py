"""Native CKW04/CKGP/CKGPAV generic-segment Chebyshev oracle."""
from pathlib import Path
import csv,ctypes as c,random
import spiceypy as s
from spiceypy.utils.libspicehelper import libspice
root=Path(__file__).resolve().parents[1];out=root/'tests/golden';path=out/'ck4.bc'
if path.exists():path.unlink()
r=random.Random(4067);h=s.ckopn(str(path),'CK4 native oracle',0);queries=[];index=0
for frame in [1,13]:
    for av in [False,True]:
        for scale in [1e-150,1.,1e150]:
            inst=-740000-index;index+=1;n=205 if index==1 else 3
            starts=[float(j*20) for j in range(n)];packets=[];sizes=[]
            for j,start in enumerate(starts):
                counts=[1+j%3,2,4,3,*([2,4,1] if av else [0,0,0])];values=[]
                for k,count in enumerate(counts):
                    values += [(1. if k==0 and i==0 else r.uniform(-.1,.1))*(scale if k<4 else .01) for i in range(count)]
                packet=[start+5.,5.,*map(float,counts),*values];sizes.append(len(packet));packets+=packet
            hh=c.c_int(h);beg=c.c_double(0.);ii=c.c_int(inst);flag=c.c_int(int(av));name=s.frmnam(frame).encode()
            libspice.ckw04b_(c.byref(hh),c.byref(beg),c.byref(ii),name,c.byref(flag),b'CK4 native',c.c_int(len(name)),c.c_int(10))
            # CKW04A's in-place batch compression overwrites later packet
            # midpoint/radius headers in N0067. One packet per invocation
            # preserves the valid input packets (and exercises SG appends).
            offset=0
            for length,start in zip(sizes,starts):
                number=c.c_int(1);size=(c.c_int*1)(length)
                data=(c.c_double*(length+6))(*packets[offset:offset+length],*[0.]*6);epochs=(c.c_double*1)(start)
                libspice.ckw04a_(c.byref(hh),c.byref(number),size,data,epochs);offset+=length
            end=c.c_double(starts[-1]+10.);libspice.ckw04e_(c.byref(hh),c.byref(end))
            if s.failed():raise RuntimeError(s.getmsg('LONG',2048))
            ticks=[-2.,-1e-8,0.,1e-8,3.14159,10.,10.00000001,15.,20.,25.,30.,35.,40.,starts[-1]+10.,starts[-1]+10.00000001,starts[-1]+12.]
            if n>100:ticks += [starts[99],starts[99]+10.,starts[100]+5.,starts[101]]
            for t in ticks:
                for tol in [0.,.1,5.,1000.]:
                    for need in [False,True]:queries.append((inst,t,tol,need,1))
            for ref in range(1,22):
                for need in [False,True]:queries.append((inst,3.14159,0.,need,ref))
s.ckcls(h);s.furnsh(str(path))
with (out/'ck4.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['inst','ticks','tol','av','frame','found','clock',*[f'm{i}' for i in range(9)],'x','y','z'])
    for inst,t,tol,need,ref in queries:
        with s.no_found_check():
            if need:m,av,clock,found=s.ckgpav(inst,t,tol,s.frmnam(ref))
            else:m,clock,found=s.ckgp(inst,t,tol,s.frmnam(ref));av=[]
        w.writerow([inst,repr(t),repr(tol),int(need),ref,int(found),repr(clock) if found else '',*([repr(float(v)) for v in m.flat] if found else ['']*9),*([repr(float(v)) for v in av] if found and need else ['']*3)])
s.kclear();print(len(queries),'native CK4 cases')
