"""CSPICE N0067 TANGPT regression for both correction loci."""
import csv
from pathlib import Path
import spiceypy as s
r=Path(__file__).resolve().parents[1]
s.furnsh(str(r/'target/kernels/de440s.bsp'));s.furnsh(str(r/'target/kernels/pck00011.tpc'))
with (r/'tests/golden/tangent_precision.csv').open('w',newline='') as f:
 w=csv.writer(f);w.writerow(['target','observer','et','corr','locus','dx','dy','dz','px','py','pz','alt','range','sx','sy','sz','epoch','vx','vy','vz'])
 for target,observer,frame in [(301,399,'IAU_MOON'),(399,301,'IAU_EARTH')]:
  for et in [0.,5e7,-5e7]:
   for corr in ['NONE','LT','LT+S','CN','CN+S','XLT','XLT+S','XCN','XCN+S']:
    pos,_=s.spkpos(str(target),et,'J2000',corr,str(observer))
    radial=s.vhat(pos);other=s.ucrss(radial,[0,0,1]);radius=1737.4 if target==301 else 6378.1366
    for offset in [0.,radius*1.5,radius*2.,None]:
     direction=pos+other*offset if offset is not None else -pos
     for locus in ['TANGENT POINT','SURFACE POINT']:
      p,alt,rng,srf,t,v=s.tangpt('ELLIPSOID',str(target),et,frame,corr,locus,str(observer),'J2000',direction)
      w.writerow([target,observer,et,corr,locus,*direction,*p,alt,rng,*srf,t,*v])

import random
rng=random.Random(6701)
with (r/'tests/golden/npedln_precision.csv').open('w',newline='') as f:
 w=csv.writer(f)
 for _ in range(200):
  axes=[rng.uniform(100,5000) for _ in range(3)]
  p=[rng.uniform(-20000,20000) for _ in range(3)]
  d=[rng.uniform(-1,1) for _ in range(3)]
  # TANGPT_C delegates to Fortran TANGPT/NPEDLN, whose floating
  # operation order differs from the separately implemented NPEDLN_C.
  import ctypes as ct
  from spiceypy.utils.libspicehelper import libspice
  values=[ct.c_double(x) for x in axes]; pp=(ct.c_double*3)(*p);dd=(ct.c_double*3)(*d);q=(ct.c_double*3)();h=ct.c_double()
  libspice.npedln_(*(ct.byref(x) for x in values),pp,dd,q,ct.byref(h))
  w.writerow([*axes,*p,*d,*q,h.value])
with (r/'tests/golden/ellipse_axes_precision.csv').open('w',newline='') as f:
 w=csv.writer(f)
 for _ in range(100):
  a=[rng.uniform(-1,1) for _ in range(3)];b=[rng.uniform(-1,1) for _ in range(3)]
  major,minor=s.saelgv(a,b);w.writerow([*a,*b,*major,*minor])
