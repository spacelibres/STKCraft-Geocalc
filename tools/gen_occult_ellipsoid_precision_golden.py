"""Fortran ZZOCCED oracle used by the OCCULT_C ellipsoid path."""
import csv,ctypes as ct,random,numpy as np,spiceypy as s
from pathlib import Path
from spiceypy.utils.libspicehelper import libspice
root=Path(__file__).resolve().parents[1];rng=random.Random(6704)
D=ct.c_double
libspice.zzocced_.restype=ct.c_int
with (root/'tests/golden/occult_ellipsoid_precision.csv').open('w',newline='') as f:
 w=csv.writer(f)
 for i in range(1000):
  d1=rng.uniform(20,100);d2=rng.uniform(120,300)
  c1=np.array([rng.uniform(-5,5),rng.uniform(-5,5),d1]);c2=np.array([rng.uniform(-15,15),rng.uniform(-15,15),d2])
  if i%4==0:c1,c2=c2,c1
  a1=np.array([10**rng.uniform(-1,1) for _ in range(3)]);a2=np.array([10**rng.uniform(-.5,1.3) for _ in range(3)])
  m1=s.eul2m(*[rng.uniform(-3,3) for _ in range(3)],3,2,1);m2=s.eul2m(*[rng.uniform(-3,3) for _ in range(3)],3,2,1)
  # Each Fortran semi-axis matrix column is the original rotation row
  # scaled by that body's axis length, matching OCCULT -> ZZOCCED.
  sem1=(D*9)(*(m1*a1[:,None]).flatten());sem2=(D*9)(*(m2*a2[:,None]).flatten())
  c1c=(D*3)(*c1);c2c=(D*3)(*c2);view=(D*3)(0,0,0)
  code=libspice.zzocced_(view,c2c,sem2,c1c,sem1)
  if s.failed():s.reset();continue
  w.writerow([*c1,*a1,*m1.flatten(),*c2,*a2,*m2.flatten(),code])

with (root/'tests/golden/occult_limb_precision.csv').open('w',newline='') as f, (root/'tests/golden/occult_angle_precision.csv').open('w',newline='') as g:
 wl=csv.writer(f);wa=csv.writer(g)
 for _ in range(100):
  axes=[rng.uniform(.2,4) for _ in range(3)];view=[rng.uniform(10,30),rng.uniform(-5,5),rng.uniform(-5,5)]
  ac=[D(x) for x in axes];vc=(D*3)(*view);el=(D*9)()
  libspice.edlimb_(*(ct.byref(x) for x in ac),vc,el);wl.writerow([*axes,*view,*el])
  for extrem in ['MIN','MAX']:
   direction=[-view[0],-view[1]+rng.uniform(-5,5),-view[2]+rng.uniform(-5,5)]
   dc=(D*3)(*direction);angle=D();point=(D*3)()
   libspice.zzasryel_(extrem.encode(),el,vc,dc,ct.byref(angle),point,ct.c_int(3))
   if s.failed():s.reset();continue
   wa.writerow([extrem,*el,*view,*direction,angle.value,*point])
