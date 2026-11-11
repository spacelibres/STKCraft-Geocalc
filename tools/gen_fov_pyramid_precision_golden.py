"""Independent Fortran ZZELVUPY / INELPL predicates and intersections."""
from pathlib import Path
import csv,ctypes as ct,random,spiceypy as s,numpy as np
from spiceypy.utils.libspicehelper import libspice
root=Path(__file__).resolve().parents[1];rng=random.Random(6705);D=ct.c_double
with (root/'tests/golden/fov_pyramid_precision.csv').open('w',newline='') as f,(root/'tests/golden/fov_inelpl_precision.csv').open('w',newline='') as g:
 w=csv.writer(f);wp=csv.writer(g)
 for i in range(1000):
  center=[rng.uniform(-2,2),rng.uniform(-2,2),rng.uniform(4,7)]
  m=s.eul2m(*[rng.uniform(-3,3) for _ in range(3)],3,2,1)
  major=m[0]*rng.uniform(.1,1.2);minor=m[1]*rng.uniform(.05,.5)
  el=s.cgv2el(center,major,minor);center,major,minor=s.el2cgv(el);ellipse=[*center,*major,*minor]
  apex=[rng.uniform(-.5,.5) for _ in range(3)];axis=[0.,0.,1.]
  bounds=[[.15,-.2,1.],[.15,.2,1.],[-.15,.2,1.],[-.15,-.2,1.]]
  ec=(D*9)(*ellipse);vc=(D*3)(*apex);ac=(D*3)(*axis);bc=(D*12)(*np.array(bounds).flatten());n=ct.c_int(4);found=ct.c_int()
  libspice.zzelvupy_(ec,vc,ac,ct.byref(n),bc,ct.byref(found))
  if s.failed():s.reset();continue
  w.writerow([*ellipse,*apex,*axis,*np.array(bounds).flatten(),found.value])
  normal=[rng.uniform(-1,1) for _ in range(3)];plane=s.nvp2pl(normal,np.array(center)+np.array(normal)*rng.uniform(-.5,.5));pn,pc=s.pl2nvc(plane)
  nx=ct.c_int();p1=(D*3)();p2=(D*3)();pl=(D*4)(*pn,pc)
  libspice.inelpl_(ec,pl,ct.byref(nx),p1,p2)
  wp.writerow([*ellipse,*pn,pc,nx.value,*p1,*p2])
