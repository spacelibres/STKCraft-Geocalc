"""Native SPKW10/SPKGEO/SPKEZR integration; no local geometry in oracle."""
from pathlib import Path
import csv,math,ctypes as c
import spiceypy as s
from spiceypy.utils.libspicehelper import libspice
root=Path(__file__).resolve().parents[1];out=root/'tests/golden';path=out/'spk10.bsp'
if path.exists():path.unlink()
s.furnsh(str(root/'target/kernels/naif0012.tls'))
h=s.spkopn(str(path),'SPK10 native integration',0);queries=[];time_inputs=[];index=0
g=[.001082616,-2.53881e-6,-1.65597e-6,.0743669161,120.,78.,6378.135,1.]
for period in [90.,225.01,720.,1436.1,10000.]:
    for n in [1,3,205]:
        body=710000+index;frame=index%21+1;index+=1
        epochs=[123456789.+i*86400. for i in range(n)];elems=[]
        for i,epoch in enumerate(epochs):
            elems += [0.,0.,1e-6,.7+i*.0001,.2+i*.001,.02,.3+i*.002,.4+i*.003,2*math.pi/period,epoch]
            tv=(c.c_double*8)(epoch,*[0.]*7);libspice.ttrans_(b'TDB',b'JDUTC',tv,c.c_int(3),c.c_int(5));assert not s.failed();time_inputs.append((repr(epoch),repr(tv[0])))
        s.spkw10(h,body,399,s.frmnam(frame),epochs[0]-20000.,epochs[-1]+20000.,str(body),g,n,elems,epochs)
        times=[epochs[0]-20000.,epochs[0],epochs[0]+1e-7,epochs[0]+1234.5,epochs[-1],epochs[-1]+20000.]
        if n>1:times += [epochs[1]-1e-7,epochs[1],epochs[1]+1e-7,(epochs[0]+epochs[1])/2.]
        if n>100:times += [epochs[99],epochs[100],epochs[101]]
        queries.extend((body,t) for t in times)
s.spkcls(h);s.furnsh(str(path));s.furnsh(str(root/'target/kernels/de440s.bsp'));s.furnsh(str(root/'target/kernels/pck00011.tpc'))
with (out/'spk10.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['body','et','frame','x','y','z','vx','vy','vz'])
    for body,et in queries:
        for frame in range(1,22):
            state,_=s.spkgeo(body,et,s.frmnam(frame),399);w.writerow([body,repr(et),frame,*[repr(float(x)) for x in state]])
with (out/'spk10_time_inputs.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['et','native_jdutc']);w.writerows(sorted(set(time_inputs)))
with (out/'spk10_corrected.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['body','observer','et','frame','correction','x','y','z','vx','vy','vz','lt','px','py','pz','plt'])
    for body in [710001,710007,710013]:
        for observer in [399,301]:
            for frame in ['J2000','B1950','GALACTIC','IAU_EARTH','IAU_MOON']:
                for correction in ['NONE','LT','LT+S','CN','CN+S','XLT','XLT+S','XCN','XCN+S']:
                    et=123458023.5;state,lt=s.spkezr(str(body),et,frame,correction,str(observer));p,plt=s.spkpos(str(body),et,frame,correction,str(observer))
                    w.writerow([body,observer,repr(et),frame,correction,*map(lambda x:repr(float(x)),state),repr(lt),*map(lambda x:repr(float(x)),p),repr(plt)])
s.kclear()
