from pathlib import Path
import spiceypy as s,csv
r=Path(__file__).resolve().parents[1]
s.furnsh(str(r/'target/kernels/de440s.bsp'));s.furnsh(str(r/'target/kernels/pck00011.tpc'))
with (r/'tests/golden/surface_ray_precision.csv').open('w',newline='') as f:
 w=csv.writer(f); w.writerow(['target','observer','et','abcorr','dx','dy','dz','px','py','pz','epoch','vx','vy','vz'])
 for target,observer,frame in [(399,301,'IAU_EARTH'),(301,399,'IAU_MOON')]:
  for et in [0.,5e8,-1e8]:
   for corr in ['NONE','LT','LT+S','CN','CN+S','XLT','XLT+S','XCN','XCN+S']:
    pos,lt=s.spkpos(str(target),et,'J2000',corr,str(observer))
    for d in [pos,pos+s.vhat(s.vcrss(pos,[0,0,1]))*300]:
     p,t,v=s.sincpt('ELLIPSOID',str(target),et,frame,corr,str(observer),'J2000',d)
     w.writerow([target,observer,et,corr,*d,*p,t,*v])

# At this grazing epoch the initial NONE geometry misses the ellipsoid;
# ZZSFXCOR still performs its absolute-state recovery, which finds it.
et=1707908.0129460432
rotation=s.pxform('J2000','IAU_EARTH',et)
position,_=s.spkpos('EARTH',et,'IAU_EARTH','NONE','MOON')
direction,_=s.spkpos('SUN',et,'J2000','NONE','MOON')
point,epoch,vector=s.sincpt('ELLIPSOID','EARTH',et,'IAU_EARTH','NONE','MOON','J2000',direction)
with (r/'tests/golden/surface_limb_precision.csv').open('w',newline='') as f:
 csv.writer(f).writerow([et,*direction,*position,*rotation.flatten(),*point,epoch,*vector])
