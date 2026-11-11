"""Native CK5 interpolation, quaternion signs and clipped gap-window oracle."""
from pathlib import Path
import csv,math
import numpy as np
import spiceypy as s
root=Path(__file__).resolve().parents[1];out=root/'tests/golden';path=out/'ck5.bc'
if path.exists():path.unlink()
h=s.ckopn(str(path),'CK5 native interpolation',0);queries=[];index=0
for subtype in range(4):
    for degree in ([3,11] if subtype in [0,2] else [1,15]):
        for avflag in [False,True]:
            for n in [2,17,205,605]:
                inst=-750000-index;frame=index%21+1;rate=[.01,13.7][index%2];index+=1
                times=np.array([100.+i*10.+(i%7)*.03125 for i in range(n)]);packets=[]
                for i,t in enumerate(times):
                    angle=.001*i;sign=-1. if subtype in [1,3] and i%3==1 else 1.
                    q=[sign*math.cos(angle),sign*math.sin(angle),sign*.01*math.sin(angle*.7),sign*.02*math.sin(angle*1.3)]
                    dq=[-.0001*math.sin(angle),.0001*math.cos(angle),.0000007*math.cos(angle*.7),.0000026*math.cos(angle*1.3)]
                    av=[.01+i*1e-7,-.02,.003];da=[1e-8,-1e-8,2e-8]
                    packets.append(q+(dq if subtype in [0,2] else [])+(av+da if subtype==2 else av if subtype==3 else []))
                starts=times[::3].copy()
                s.ckw05(h,subtype,degree,float(times[0]-1),float(times[-1]+1),inst,s.frmnam(frame),avflag,str(inst),times,packets,rate,len(starts),starts)
                ticks=[times[0]-2,times[0]-.1,times[0],times[0]+.1,times[-1],times[-1]+.1,times[-1]+2]
                for i in sorted({min(n-2,j) for j in [0,1,2,3,98,99,100,298,299,300,n-2]}):
                    ticks += [times[i]-1e-8,times[i],times[i]+1e-8,(times[i]+times[i+1])/2]
                for t in ticks:
                    for tol in [0.,.1,5.,1000.]:
                        for need in [False,True]:queries.append((inst,float(t),tol,need,1))
                for ref in range(1,22):
                    for need in [False,True]:queries.append((inst,float((times[0]+times[1])/2),0.,need,ref))
s.ckcls(h);s.furnsh(str(path))
with (out/'ck5.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['inst','ticks','tol','av','frame','status','clock',*[f'm{i}' for i in range(9)],'x','y','z'])
    for inst,t,tol,need,ref in queries:
        try:
            with s.no_found_check():
                if need:m,av,clock,found=s.ckgpav(inst,t,tol,s.frmnam(ref))
                else:m,clock,found=s.ckgp(inst,t,tol,s.frmnam(ref));av=[]
            status='FOUND' if found else 'NONE'
            values=[repr(clock),*[repr(float(v)) for v in m.flat],*([repr(float(v)) for v in av] if need else ['']*3)] if found else ['']*13
        except s.utils.exceptions.SpiceyError as error:status=error.short;values=['']*13
        w.writerow([inst,repr(t),repr(tol),int(need),ref,status,*values])
s.kclear();print(len(queries),'native CK5 cases')
