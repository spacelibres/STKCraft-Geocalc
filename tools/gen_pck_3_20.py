"""Native PCKW20 and generic-segment PCK3 orientation fixtures."""
from pathlib import Path
import csv,math,random,ctypes as ct
import spiceypy as s
from spiceypy.utils.libspicehelper import libspice as lib
ROOT=Path(__file__).resolve().parents[1];OUT=ROOT/'tests/golden';rng=random.Random(320);path=OUT/'binary_pck_3_20.bpc'
if path.exists():path.unlink()
h=s.pckopn(str(path),'native PCK3 and20 source',0);hc=ct.c_int(h);queries=[];frames=[]
def double(x):return ct.c_double(x)
def integer(x):return ct.c_int(x)
def check():
    if lib.failed_():
        message=s.getmsg('LONG',4000);s.reset();raise RuntimeError(message)
def pck20(cid,frame,degree,ascale,tscale):
    jd=2451544.;fraction=.123456789;days=.7;n=3;start=(jd-2451545.+fraction)*86400.;end=start+n*days*86400.;data=[]
    for record in range(n):
        for axis in range(3):
            data.extend([rng.uniform(-1e-6,1e-6)*tscale/ascale*10**(-j) for j in range(degree+1)]);data.append([1.,.4,9e6][axis]/ascale+record*.1)
    frame=s.frmnam(frame).encode();name=f'PCK20 {cid}'.encode();first=double(start);last=double(end);interval=double(days);nc=integer(n);dg=integer(degree);cs=(ct.c_double*len(data))(*data);as_=double(ascale);ts=double(tscale);jd_=double(jd);fr=double(fraction);id_=integer(cid)
    lib.pckw20_(ct.byref(hc),ct.byref(id_),frame,ct.byref(first),ct.byref(last),name,ct.byref(interval),ct.byref(nc),ct.byref(dg),cs,ct.byref(as_),ct.byref(ts),ct.byref(jd_),ct.byref(fr),ct.c_int(len(frame)),ct.c_int(len(name)));check()
    epochs=[start,start+.123,start+days*86400.-1e-6,start+days*86400.,start+days*86400.+1e-6,(start+end)/2,end-.123,end]
    return epochs
for degree in [0,1,2,7,19]:
    for scale_index,(ascale,tscale) in enumerate([(1.,1.),(math.pi/180.,86400.),(3.7,13.)]):
        for frame in range(1,22):
            cid=920000+degree*1000+scale_index*100+frame;epochs=pck20(cid,frame,degree,ascale,tscale);frames.append((cid,20));queries += [(cid,et) for et in epochs]
for index_type in range(5):
    for variable in [False,True]:
        for ncoeff in [1,2,5,21]:
            for reference in [1,13,17]:
                cid=930000+index_type*10000+int(variable)*1000+ncoeff*20+reference;n=205 if ncoeff==1 and reference==1 else 3;step=10000.;base=-30000.25
                refs=[base+i*step+(i%7)*.125 for i in range(n)] if index_type>=2 else [base+i*step for i in range(n)]
                nearest=index_type in [1,4];first=refs[0]-step/2 if nearest else refs[0]+(1e-7 if index_type==2 else 0.);last=refs[-1]+step/2 if nearest else refs[-1]+step
                descriptor=s.dafps(2,5,[first,last],[cid,reference,3,0,0]);desc=(ct.c_double*5)(*descriptor[:5]);const=(ct.c_double*1)(float(ncoeff));nc=integer(1);index=integer(index_type);size=integer(2+6*ncoeff);name=f'PCK3 {cid}'.encode()
                if variable:lib.sgbwvs_(ct.byref(hc),desc,name,ct.byref(nc),const,ct.byref(index),ct.c_int(len(name)))
                else:lib.sgbwfs_(ct.byref(hc),desc,name,ct.byref(nc),const,ct.byref(size),ct.byref(index),ct.c_int(len(name)))
                check();data=[]
                for j,ref in enumerate(refs):
                    data += [ref if nearest else ref+step/2,step/2]
                    for component in range(6):data += [rng.uniform(-1.,1.)*(100. if component<3 else .001)*10.**(-k) for k in range(ncoeff)]
                packets=(ct.c_double*len(data))(*data);npk=integer(n);refdata=[base,step] if index_type<2 else refs;rd=(ct.c_double*len(refdata))(*refdata);nr=integer(len(refdata))
                if variable:
                    sizes=(ct.c_int*n)(*[size.value]*n);lib.sgwvpk_(ct.byref(hc),ct.byref(npk),sizes,packets,ct.byref(nr),rd)
                else:lib.sgwfpk_(ct.byref(hc),ct.byref(npk),packets,ct.byref(nr),rd)
                check();lib.sgwes_(ct.byref(hc));check();frames.append((cid,3))
                epochs=[first,first+1e-7,refs[1]-1e-7,refs[1],refs[1]+1e-7,(refs[1]+refs[2])/2,last-1e-7,last]
                if n>100:
                    for i in [98,99,100,198,199,200]:epochs += [refs[i]-1e-7,refs[i],refs[i]+1e-7]

                if index_type>=2:queries += [(cid,et) for et in epochs]
s.pckcls(h)
fk='KPL/FK\n\\begindata\n'
for cid,kind in frames:fk+=f"FRAME_PCK_MODEL_{cid}={cid+1000000}\nFRAME_{cid+1000000}_NAME='PCK_MODEL_{cid}'\nFRAME_{cid+1000000}_CLASS=2\nFRAME_{cid+1000000}_CLASS_ID={cid}\nFRAME_{cid+1000000}_CENTER=301\n"
(OUT/'binary_pck_3_20.tf').write_text(fk);s.furnsh(str(path));s.furnsh(str(OUT/'binary_pck_3_20.tf'))
with (OUT/'binary_pck_3_20.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['classid','et','outframe',*[f'r{i}' for i in range(9)],*[f'dr{i}' for i in range(9)]])
    for cid,et in queries:
        for out in ['J2000','ECLIPJ2000']:

            try:xf=s.sxform(out,f'PCK_MODEL_{cid}',et)
            except Exception:
                print('FAILED',cid,repr(et),out);raise
            w.writerow([cid,repr(et),out,*[repr(float(x)) for x in xf[:3,:3].flat],*[repr(float(x)) for x in xf[3:,:3].flat]])
s.kclear();print(f'{len(queries)*2} exact native PCK3/20 frame cases')
