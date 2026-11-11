"""Native CKW06/CKGP oracle: piecewise quaternion packets, gaps, ties/history."""
from pathlib import Path
import csv,math,ctypes as c
import spiceypy as s
from spiceypy.utils.libspicehelper import libspice
root=Path(__file__).resolve().parents[1];out=root/'tests/golden';path=out/'ck6.bc'
if path.exists():path.unlink()
h=s.ckopn(str(path),'CK6 native source',0);queries=[];model=0
def ints(v):return (c.c_int*len(v))(*v)
def doubles(v):return (c.c_double*len(v))(*v)
writer=libspice.ckw06_;writer.argtypes=[c.c_void_p]*16+[c.c_int,c.c_int]
for choose_last in [False,True]:
    for avflag in [False,True]:
        for intervals in [1,3,205,10205]:
            inst=-760000-model;frame=s.frmnam(model%21+1).encode();model+=1
            bounds=[100.+i*100. for i in range(intervals+1)];counts=[];subtypes=[];degrees=[];rates=[];epochs=[];packets=[]
            for i in range(intervals):
                subtype=i%4;count=205 if intervals==3 and i==1 else 17 if intervals<1000 else 2
                counts.append(count);subtypes.append(subtype);degrees.append(11 if subtype%2==0 else 15);rates.append(1.25 if i%2==0 else 13.7)
                # Every other interval ends20ticks before its nominal boundary.
                times=[bounds[i]+(80. if i%2==0 else 100.)*j/(count-1) for j in range(count)];epochs+=times
                for j,t in enumerate(times):
                    a=i*.0001+j*.001;sign=-1. if subtype%2==1 and j%3==1 else 1.
                    q=[sign*math.cos(a),sign*math.sin(a),sign*.01*math.sin(a*.7),sign*.02*math.sin(a*1.3)]
                    dq=[-.0001*math.sin(a),.0001*math.cos(a),.0000007*math.cos(a*.7),.0000026*math.cos(a*1.3)]
                    av=[.01,-.02,.003];da=[1e-8,-1e-8,2e-8]
                    packets += q+(dq if subtype in [0,2] else [])+(av+da if subtype==2 else av if subtype==3 else [])
            hh=c.c_int(h);ii=c.c_int(inst);flag=c.c_int(int(avflag));first=c.c_double(bounds[0]);last=c.c_double(bounds[-1]);nn=c.c_int(intervals);sel=c.c_int(int(choose_last))
            writer(c.byref(hh),c.byref(ii),frame,c.byref(flag),c.byref(first),c.byref(last),b'CK6 native',c.byref(nn),ints(counts),ints(subtypes),ints(degrees),doubles(packets),doubles(rates),doubles(epochs),doubles(bounds),c.byref(sel),len(frame),10)
            if s.failed():raise RuntimeError(s.getmsg('LONG',2048))
            ticks=[bounds[0]-2,bounds[0],bounds[0]+1e-8,bounds[0]+3.125,bounds[0]+80.,bounds[0]+80.00000001,bounds[0]+90.,bounds[0]+99.99999999,bounds[-1]-20.,bounds[-1]-1e-8,bounds[-1],bounds[-1]+2.]
            if intervals>1:ticks += [bounds[1],bounds[1]+1e-8,bounds[1]+3.,bounds[2]-1e-8,bounds[2],bounds[2]+1e-8]
            if intervals>100:ticks += [bounds[99],bounds[100]-10.,bounds[100],bounds[101],bounds[-2],bounds[-2]+90.]
            # A successful gap→next interval map changes the native saved
            # rate through its NINT branch; following calls exercise reuse.
            if intervals>1:ticks += [bounds[0]+99.,bounds[1]+1.,bounds[1]+2.,bounds[1]+1.]
            for tick in ticks+list(reversed(ticks)):
                for tol in [0.,.1,5.,1000.]:
                    for need in [False,True]:queries.append((inst,tick,tol,need,1))
            for ref in range(1,22):
                for need in [False,True]:queries.append((inst,bounds[0]+3.125,0.,need,ref))
s.ckcls(h);s.furnsh(str(path))
with (out/'ck6.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['inst','ticks','tol','av','frame','status','clock',*[f'm{i}' for i in range(9)],'x','y','z'])
    for inst,t,tol,need,ref in queries:
        try:
            with s.no_found_check():
                if need:m,av,clock,found=s.ckgpav(inst,t,tol,s.frmnam(ref))
                else:m,clock,found=s.ckgp(inst,t,tol,s.frmnam(ref));av=[]
            status='FOUND' if found else 'NONE';values=[repr(clock),*[repr(float(v)) for v in m.flat],*([repr(float(v)) for v in av] if need else ['']*3)] if found else ['']*13
        except s.utils.exceptions.SpiceyError as error:status=error.short;values=['']*13
        w.writerow([inst,repr(t),repr(tol),int(need),ref,status,*values])
s.kclear();print(len(queries),'native CK6 cases')
