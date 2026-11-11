"""Native CSPICE N0067 CK1/2/3 encoded-tick CKG P/CKGPAV fixtures."""
from pathlib import Path
import csv,random,math
import numpy as np
import spiceypy as s
ROOT=Path(__file__).resolve().parents[1];OUT=ROOT/'tests/golden';rng=random.Random(123123);paths=[];queries=[]
def quat(i):
    if i%17==0:return [1.,0.,0.,0.]
    if i%17==1:return [0.,1.,0.,0.]
    return s.m2q(s.eul2m(rng.uniform(-3.,3.),rng.uniform(-1.,1.),rng.uniform(-3.,3.),3,1,3))
def write(h,kind,inst,frame,avflag,n=205,shift=0.):
    epochs=np.array([shift+100.25+i*10+(i%7)*.03125 for i in range(n)])
    qs=np.array([quat(i) for i in range(n)]);av=np.array([[rng.uniform(-.01,.01) for j in range(3)] for i in range(n)])
    if kind==1:s.ckw01(h,float(epochs[0]-1),float(epochs[-1]+1),inst,s.frmnam(frame),avflag,str(inst),n,epochs,qs,av)
    elif kind==2:
        stops=epochs+np.array([4. if i%5 else 10. for i in range(n)]);stops=np.minimum(stops,np.r_[epochs[1:],epochs[-1]+5.]);rates=np.array([.1+(i%3)*.02 for i in range(n)])
        s.ckw02(h,float(epochs[0]-1),float(stops[-1]+1),inst,s.frmnam(frame),str(inst),n,epochs,stops,qs,av,rates)
    else:
        starts=epochs[::3].copy();s.ckw03(h,float(epochs[0]-1),float(epochs[-1]+1),inst,s.frmnam(frame),avflag,str(inst),n,epochs,qs,av,len(starts),starts)
    ticks=[epochs[0]-2,epochs[0]-1,epochs[0]-.1,epochs[0],epochs[0]+.1,epochs[-1],epochs[-1]+.1,epochs[-1]+1,epochs[-1]+2]
    for i in sorted({min(j,n-2) for j in [1,2,3,5,98,99,100,*([297,298,299,300] if n>300 else []),n-2]}):
        ticks.extend([epochs[i]-1e-8,epochs[i],epochs[i]+1e-8,(epochs[i]+epochs[i+1])/2])
        if kind==2:ticks += [stops[i],(stops[i]+epochs[i+1])/2]
    for tick in ticks:
        for tol in [0.,.1,5.,1000.]:
            for needav in [False,True]:queries.append(('ab',inst,float(tick),tol,needav,1))
    for reference in range(1,22):
        for needav in [False,True]:queries.append(('ab',inst,float((epochs[min(6,n-2)]+epochs[min(7,n-1)])/2),5.,needav,reference))
    return epochs
path=OUT/'ck_types_a.bc'
if path.exists():path.unlink()
h=s.ckopn(str(path),'native ticks type123',0)
for kind in [1,2,3]:
    for frame in range(1,22):
        for avflag in ([True] if kind==2 else [False,True]):write(h,kind,-(kind*100000+frame*10+int(avflag)),frame,avflag,(605 if kind==3 else 205) if frame==1 else 12)
write(h,3,-990000,1,True,12)
write(h,1,-990000,17,False,5,40.) # AV search must fall back to older AV segment.
s.ckcls(h);paths.append(path)
path=OUT/'ck_types_b.bc'
if path.exists():path.unlink()
h=s.ckopn(str(path),'later ticks overlay',0);write(h,3,-990000,2,True,5,50.);s.ckcls(h);paths.append(path)
queries += [('ba',inst,tick,tol,av,frame) for order,inst,tick,tol,av,frame in list(queries) if inst==-990000]
with (OUT/'ck_ticks.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['order','instrument','ticks','tolerance','need_av','reference','found','clock',*[f'm{i}' for i in range(9)],'avx','avy','avz'])
    for order in ['ab','ba']:
        s.kclear()
        for p in (paths if order=='ab' else paths[::-1]):s.furnsh(str(p))
        for qorder,inst,ticks,tol,av,ref in queries:
            if qorder!=order:continue
            with s.no_found_check():
                if av:matrix,velocity,clock,found=s.ckgpav(inst,ticks,tol,s.frmnam(ref))
                else:matrix,clock,found=s.ckgp(inst,ticks,tol,s.frmnam(ref));velocity=[]
            w.writerow([order,inst,repr(ticks),repr(tol),int(av),ref,int(found),repr(clock) if found else '',*([repr(float(x)) for x in matrix.flat] if found else ['']*9),*([repr(float(x)) for x in velocity] if found and av else ['']*3)])
s.kclear();print(f'{len(queries)} native encoded-tick CK cases')

# Class-3 frame integration uses real SCE2C conversion, including TDT clocks.
clocktext='KPL/SCLK\n\\begindata\n'
for clock in [100,200,300]:
    clocktext+=f'SCLK_DATA_TYPE_{clock} = 1\nSCLK01_TIME_SYSTEM_{clock} = {2 if clock==300 else 1}\nSCLK01_N_FIELDS_{clock} = 2\nSCLK01_MODULI_{clock} = ( 1000000000 256 )\nSCLK01_OFFSETS_{clock} = ( 0 0 )\nSCLK01_OUTPUT_DELIM_{clock} = 1\nSCLK_PARTITION_START_{clock} = ( 0 )\nSCLK_PARTITION_END_{clock} = ( 1000000000 )\nSCLK01_COEFFICIENTS_{clock} = ( 0 1000 2.5 1000 1009.765625 2.6 )\n'
(OUT/'ck_test_scalar.tsc').write_text(clocktext)
import re
clocktext=re.sub(r'(SCLK(?:_DATA_TYPE|01_TIME_SYSTEM|01_N_FIELDS|01_OUTPUT_DELIM)_\d+) = ([^\n(]+)\n',r'\1 = ( \2 )\n',clocktext)
(OUT/'ck_test.tsc').write_text(clocktext)
names=[('CK_T1_NOAV',-100010),('CK_T1_AV',-100011),('CK_T2',-200011),('CK_T3_NOAV',-300010),('CK_T3_AV',-300011),('CK_T3_ECLIP',-300171)]
fk='KPL/FK\n\\begindata\n'
for i,(name,inst) in enumerate(names):
    identifier=-(80000+i);fk+=f"FRAME_{name}={identifier}\nFRAME_{identifier}_NAME='{name}'\nFRAME_{identifier}_CLASS=3\nFRAME_{identifier}_CLASS_ID={inst}\nFRAME_{identifier}_CENTER=-100\n"
fk+="FRAME_CK_CHILD=-81000\nFRAME_-81000_NAME='CK_CHILD'\nFRAME_-81000_CLASS=4\nFRAME_-81000_CLASS_ID=-81000\nFRAME_-81000_CENTER=-100\nTKFRAME_-81000_RELATIVE='CK_T3_AV'\nTKFRAME_-81000_SPEC='ANGLES'\nTKFRAME_-81000_ANGLES=(13 27 -11)\nTKFRAME_-81000_AXES=(3 1 2)\nTKFRAME_-81000_UNITS='DEGREES'\n"
(OUT/'ck_test.tf').write_text(fk)
s.kclear();s.furnsh(str(OUT/'ck_test.tf'));s.furnsh(str(OUT/'ck_test.tsc'));s.furnsh(str(ROOT/'target/kernels/naif0012.tls'))
for p in paths:s.furnsh(str(p))
with (OUT/'ck_sclk_ticks.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['clock','et','ticks'])
    for clock in [-100,-200,-300]:
        for tick in [98.,100.25,105.,110.28125,115.,120.3125,125.,130.34375,140.375,200.,999.9,1000.,1000.1,1100.]:
            et=s.sct2e(clock,tick);w.writerow([clock,repr(et),repr(s.sce2c(clock,et))])
with (OUT/'ck_frames.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['kind','from','to','et','found',*[f'm{i}' for i in range(9)],*[f'd{i}' for i in range(9)]])
    pairs=[('J2000',name) for name,_ in names]+[(name,'B1950') for name,_ in names]+[('CK_T2','CK_T3_AV'),('CK_CHILD','ECLIPJ2000'),('CK_T3_AV','CK_CHILD')]
    for a,b in pairs:
        clock=-200 if 'T2' in a+b else -100 if 'T1' in a+b else -300
        for tick in [98.,100.25,105.,110.28125,115.,120.3125,125.,130.34375,140.375,200.,999.9,1000.,1000.1,1100.]:
            et=s.sct2e(clock,tick)
            for kind in ['rotation','state']:
                try:
                    if kind=='state':xf=s.sxform(a,b,et);values=[repr(float(x)) for x in xf[:3,:3].flat]+[repr(float(x)) for x in xf[3:,:3].flat]
                    else:values=[repr(float(x)) for x in s.pxform(a,b,et).flat]+['']*9
                    found=True
                except Exception:values=['']*18;found=False
                w.writerow([kind,a,b,repr(et),int(found),*values])
s.kclear()
