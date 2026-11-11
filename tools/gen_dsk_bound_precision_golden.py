"""Source Fortran ZZINVELT boundary predicates, including prolate/oblate PDT."""
from pathlib import Path
import csv,ctypes as c
import numpy as np
import spiceypy as s
from spiceypy.utils.libspicehelper import libspice
root=Path(__file__).resolve().parents[1]
arr=lambda v:(c.c_double*len(v))(*v)
rows=[];rng=np.random.default_rng(532513)
for corsys in [1,3,4]:
 for bounds in ([[-np.pi,np.pi,-np.pi/2,np.pi/2,.4,7.],[2.,-2.,-.6,.9,.4,7.],[-1.,.5,.3,1.2,1.5,5.]] if corsys!=3 else [[-3.,3.,-4.,4.,-2.,5.],[-1.,.2,.3,1.2,1.5,5.]]):
  for f in ([0.,.2,-.2] if corsys==4 else [0.]):
   pars=[2.,f]+[0.]*8
   if corsys==4:bounds=[*bounds[:4],.4,7.]
   points=[rng.normal(size=3)*rng.uniform(0.,12.) for _ in range(300)]
   for p in [[0.,0.,0.],[0.,0.,1.],[0.,0.,-1.],[1e-14,0.,7.]]:points.append(p)
   for lon in [bounds[0],bounds[1],0.,np.pi]:
    for lat in [bounds[2],bounds[3],-np.pi/2,np.pi/2]:
     for r in [bounds[4],bounds[5]]:
      for d in [0.,-1e-12,1e-12]:points.append(s.latrec(r,lon+d,lat+d))
   for p in points:
    margin=1e-8;inside=c.c_int();system=c.c_int(corsys);excluded=c.c_int(0);m=c.c_double(margin)
    libspice.zzinvelt_(arr(p),c.byref(system),arr(pars),arr(bounds),c.byref(m),c.byref(excluded),c.byref(inside))
    if s.failed():raise Exception(s.getmsg('LONG'))
    rows.append([corsys,*pars,*bounds,*map(float,p),margin,inside.value])
with (root/'tests/golden/dsk/contains.csv').open('w',newline='') as file:
 w=csv.writer(file);w.writerow(['corsys',*[f'param{i}' for i in range(10)],*[f'bound{i}' for i in range(6)],'x','y','z','margin','inside']);w.writerows(rows)
print(len(rows),'native volume predicates')
