"""Independent CSPICE TWO_DIRECTIONS vectors, shape radii, and VSEP angles."""
from pathlib import Path
import csv,math,spiceypy as s
root=Path(__file__).resolve().parents[1]
s.furnsh(str(root/'target/kernels/de442s.bsp'));s.furnsh(str(root/'target/kernels/pck00011.tpc'));s.furnsh(str(root/'tests/golden/fov_ellipsoid.tf'))
CORR=['NONE','LT','LT+S','CN','CN+S','XLT','XLT+S','XCN','XCN+S'];VCORR=['NONE','LT','CN','XLT','XCN','S','XS']
def spec(kind='POSITION',target=301,observer=399,corr='NONE',frame='J2000',anti=False,shape='POINT',instrument=1420001,vector=(1.2,-3.4,5.6)):
 return [kind,target,observer,corr,frame,int(anti),shape,instrument,*vector]
def frame_epoch(frame,observer,et,corr,state=False):
 center,cl,_=s.frinfo(s.namfrm(frame))
 if cl==1 or corr in ['NONE','S','XS'] or center==observer:return et
 lt=s.spkezr(str(center),et,'J2000',corr,str(observer))[1] if state else s.spkpos(str(center),et,'J2000',corr,str(observer))[1]
 return et+(lt if corr.startswith('X') else -lt)
def resolve(sp,et):
 kind,target,observer,corr,frame,anti,shape,instrument,x,y,z=sp
 if kind=='POSITION':vec,lt=s.spkpos(str(target),et,'J2000',corr,str(observer));epoch=et
 elif kind=='VELOCITY':
  state,_=s.spkezr(str(target),et,frame,corr,str(observer));epoch=frame_epoch(frame,observer,et,corr,True)
  vec=state[3:] if frame=='J2000' else s.mxv(s.pxform(frame,'J2000',epoch),state[3:])
 else:
  if kind in ['BORESIGHT','VECTOR_FOV']:
   _,frame,boresight,_,_=s.getfov(instrument,100)
   raw=boresight if kind=='BORESIGHT' else [x,y,z]
  elif kind=='AXIS':raw=[0.,0.,1.]
  else:raw=[x,y,z]
  epoch=frame_epoch(frame,observer,et,corr)
  vec=s.mxv(s.pxform(frame,'J2000',epoch),raw)
  if corr in ['S','XS']:
   obs=s.spkezr(str(observer),et,'J2000','NONE','0')[0]
   vec=s.stlabx(vec,obs[3:]) if corr=='XS' else s.stelab(vec,obs[3:])
  # POINTING applies J2000 -> J2000 in its stellar path.
  if corr in ['S','XS']:vec=s.mxv(s.pxform('J2000','J2000',epoch),vec)
 if anti:vec=-vec
 radius=0.
 if shape=='SPHERE':radius=math.asin(max(s.bodvcd(target,'RADII',3)[1])/s.vnorm(vec))
 return vec,epoch,radius
pairs=[]
for et in [-5e8,0.,5e8]:
 for i,corr in enumerate(CORR):
  for anti in [False,True]:
   for frame in ['J2000','ECLIPJ2000','IAU_MOON','IAU_EARTH']:
    for first in [spec('POSITION',corr=corr,anti=anti),spec('VELOCITY',target=10,corr=corr,frame=frame,anti=anti)]:
     for second in [spec('POSITION',target=399,observer=301,corr=CORR[(i+3)%9],anti=not anti),spec('VELOCITY',target=399,observer=301,corr=CORR[(i+5)%9],frame=frame,anti=not anti),spec('VECTOR',corr=VCORR[i%7],frame=frame,anti=anti)]:pairs.append((et,first,second))
 for corr in VCORR:
  for frame in ['J2000','ECLIPJ2000','IAU_EARTH','IAU_MOON']:
   for anti in [False,True]:
    for kind in ['AXIS','VECTOR','BORESIGHT','VECTOR_FOV']:
     pairs.append((et,spec(kind,corr=corr,frame=frame,anti=anti,instrument=1420001),spec('BORESIGHT',corr='NONE',instrument=1420025)))
 for sh1,sh2 in [('POINT','POINT'),('SPHERE','POINT'),('POINT','SPHERE'),('SPHERE','SPHERE')]:
  pairs.append((et,spec(shape=sh1,corr='CN+S'),spec(target=10,shape=sh2,corr='LT+S')))
with (root/'tests/golden/two_directions_precision.csv').open('w',newline='') as f:
 w=csv.writer(f)
 for et,first,second in pairs:
  a,ae,ar=resolve(first,et);b,be,br=resolve(second,et)
  w.writerow([et,*first,*second,s.vsep(a,b)-ar-br,*a,*b,ae,be])
print('wrote',len(pairs),'independent TWO_DIRECTIONS cases')

# VSEP edge cases independent of ephemerides, frames, and instrument state.
import random
rng=random.Random(6706)
with (root/'tests/golden/two_directions_vsep_precision.csv').open('w',newline='') as f:
 w=csv.writer(f)
 vectors=[([0.,0.,0.],[1.,2.,3.]),([1.,0.,0.],[1.,1e-14,0.]),([1.,0.,0.],[-1.,1e-14,0.]),([1e200,-2e200,3e200],[2e-200,1e-200,-3e-200])]
 vectors += [([rng.uniform(-1,1) for _ in range(3)],[rng.uniform(-1,1) for _ in range(3)]) for _ in range(200)]
 for a,b in vectors:w.writerow([*a,*b,s.vsep(a,b)])

live=[
 ('orthogonal',spec('VECTOR',vector=(1.,0.,0.)),spec('VECTOR',vector=(0.,1.,0.))),
 ('velocity',spec(corr='CN+S'),spec('VELOCITY',target=399,observer=301,corr='XCN+S',frame='IAU_MOON',anti=True)),
 ('sphere',spec(corr='CN+S',shape='SPHERE'),spec('VECTOR',corr='XS',anti=True,vector=(1.,0.,0.))),
 ('vector_s_anti',spec('VECTOR',corr='S',anti=True,vector=(1.,.3,-.2)),spec('VECTOR',vector=(0.,1.,0.))),
]
with (root/'tests/golden/two_directions_live_cspice.csv').open('w',newline='') as f:
 w=csv.writer(f)
 for label,first,second in live:
  et=5e8;a,ae,ar=resolve(first,et);b,be,br=resolve(second,et)
  w.writerow([label,et,*first,*second,s.vsep(a,b)-ar-br,*a,*b,ae,be])
