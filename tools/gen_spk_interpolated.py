"""CSPICE N0067 state-interpolation SPK types 8/9/12/13 exact fixtures."""
from pathlib import Path
import csv,math,random
import spiceypy as s
ROOT=Path(__file__).resolve().parents[1];OUT=ROOT/'tests/golden';path=OUT/'spk_interpolated.bsp'
if path.exists():path.unlink()
h=s.spkopn(str(path),'state interpolation source fixtures',0);rng=random.Random(891213);cases=[];corrected=[]
for kind in [8,9,12,13]:
    for di,degree in enumerate([1,2,3,4,27] if kind in [8,9] else [1,3,5,7,27]):
        for frame in range(1,22):
            n=205 if frame==1 else 40
            start=-1e7;step=2e7/(n-1)
            epochs=[start+i*step for i in range(n)]
            if kind in [9,13]:
                for i in range(1,n-1):epochs[i]+=rng.uniform(-.2,.2)*step
            states=[]
            for i,et in enumerate(epochs):
                phase=et/1e7
                states.append([1e7+10000*math.sin(phase+.3*j)+rng.uniform(-1,1) for j in range(3)]+[.01*math.cos(phase+.2*j)+rng.uniform(-1e-5,1e-5) for j in range(3)])
            body=kind*100000+di*100+frame
            if kind in [8,12]:getattr(s,f'spkw{kind:02}')(h,body,0,s.frmnam(frame),epochs[0],epochs[-1],str(body),degree,n,states,start,step)
            else:getattr(s,f'spkw{kind:02}')(h,body,0,s.frmnam(frame),epochs[0],epochs[-1],str(body),degree,n,states,epochs)
            ets=[epochs[0],epochs[-1],epochs[1],epochs[-2],epochs[n//2],(epochs[2]+epochs[3])/2]
            ets += [((epochs[2]+epochs[3])/2)+v for v in [-1e-7,1e-7]]
            if frame==1:
                for i in [98,99,100,101,198,199,200]:ets += [epochs[i]-1e-7,epochs[i],epochs[i]+1e-7]
            cases += [(body,et,1) for et in ets]
            cases += [(body,12345.6789,out) for out in range(1,22)]
            if frame==17 and di in [1,4]:corrected.append(body)
# One irregular directory requires multiple 101-entry buffer reads.
n=10205;epochs=[float(i*1000+i%7) for i in range(n)];states=[[1e7+i*.1,2e6+math.sin(i),-3e6+i*.01,.01,.02,.03] for i in range(n)]
s.spkw09(h,900001,0,'J2000',epochs[0],epochs[-1],'large epoch directory',4,n,states,epochs)
for i in [9998,9999,10000,10098,10099,10100,10203]:cases += [(900001,epochs[i]+v,1) for v in [-1e-7,0.,1e-7]]
# A moving observer in a third inertial source frame exercises nonzero
# stellar aberration and its derivative, in addition to the SSB observer.
s.spkw03(h,999999,0,'GALACTIC',-1e7,1e7,'moving source-frame observer',2e7,1,1,[1e6,1.2e8,-2e6,-7e7,3e6,4e7,12.,0.,-7.,0.,4.,0.],-1e7)
# An unsupported newer type must mask an older supported segment.
s.spkw02(h,990001,0,'J2000',-10.,10.,'old supported',20.,1,0,[1.,2.,3.],-10.)
s.spkw05(h,990001,0,'J2000',-10.,10.,'new unknown type999',398600.4,2,[[7000.,0.,0.,0.,7.5,0.],[6999.,75.,0.,-.08,7.5,0.]],[-10.,10.])
s.spkcls(h)
# Type5 is now implemented. Mark only the deliberately unknown test
# segment using the native DAF summary updater; it must still mask type2.
h=s.dafopw(str(path));s.dafbfs(h)
while s.daffna():
    dc,ic=s.dafus(s.dafgs(),2,6)
    if ic[0]==990001 and ic[3]==5:
        ic[3]=999;s.dafrs(s.dafps(2,6,dc,ic))
s.dafcls(h);s.furnsh(str(path))
try:s.spkgeo(990001,0.,'J2000',0)
except s.utils.exceptions.SpiceyError as error:
    assert 'SPK' in str(error)
else:raise AssertionError('native reference silently fell back past unknown segment')
with (OUT/'spk_interpolated_geometric.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['target','et','frame','x','y','z','vx','vy','vz'])
    for body,et,frame in cases:
        state,_=s.spkgeo(body,et,s.frmnam(frame),0);w.writerow([body,repr(et),frame,*[repr(float(x)) for x in state]])
with (OUT/'spk_interpolated_corrected.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['target','observer','et','frame','correction','x','y','z','vx','vy','vz','lt','px','py','pz','plt'])
    for body in corrected:
        for observer in [0,999999]:
            for frame in range(1,22):
                for correction in ['NONE','LT','LT+S','CN','CN+S','XLT','XLT+S','XCN','XCN+S']:
                    for et in [-12345.67,23456.78]:
                        state,lt=s.spkezr(str(body),et,s.frmnam(frame),correction,str(observer));pos,plt=s.spkpos(str(body),et,s.frmnam(frame),correction,str(observer));w.writerow([body,observer,repr(et),frame,correction,*[repr(float(x)) for x in state],repr(lt),*[repr(float(x)) for x in pos],repr(plt)])
s.kclear();print(f'{len(cases)} geometric, {len(corrected)*2*21*9*2} corrected source cases')
