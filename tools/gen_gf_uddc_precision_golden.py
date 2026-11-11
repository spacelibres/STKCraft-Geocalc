"""Unmodified native GFUDS with UDDC/UDDF/QDERIV callbacks."""
from pathlib import Path
import csv,spiceypy as s
from spiceypy.utils.callbacks import SpiceUDFUNS,SpiceUDFUNB
root=Path(__file__).resolve().parents[1]
functions=[('linear',lambda t:t),('quadratic',lambda t:t*t),('negative_quadratic',lambda t:-t*t),('cubic',lambda t:t*(t-2.)*(t+2.)),('quartic',lambda t:(t-1.)*(t-1.)*(t+2.)*(t+2.)),('constant',lambda t:2.),('small_linear',lambda t:1e-12*t),('small_quadratic',lambda t:1e-12*(t-.37)*(t-.37))]
rows=[];primitives=[]
def cell(windows):
 c=s.cell_double(1000)
 for a,b in windows:s.wninsd(a,b,c)
 return c
def packed(w):return ';'.join(repr(a)+':'+repr(b) for a,b in [s.wnfetd(w,i) for i in range(s.wncard(w))])
for name,fn in functions:
 udf=SpiceUDFUNS(fn)
 for dx in [-1.,.01,.125,1.,2.,4.]:
  decreasing=SpiceUDFUNB(lambda u,t:s.uddc(u,t,dx))
  for t in [-4.,-2.,-.37,0.,.37,1.,2.,4.,1e10]:primitives.append([name,t,dx,s.uddf(udf,t,dx),int(s.uddc(udf,t,dx))])
  for windows in [[(-4.,4.)],[(-4.,-1.),(0.,.125),(1.,4.)]]:
   for op,ref,adjust in [('>',0.,0.),('<',0.,0.),('=',0.,0.),('LOCMIN',0.,0.),('LOCMAX',0.,0.),('ABSMIN',0.,0.),('ABSMAX',0.,0.),('ABSMIN',0.,.25),('ABSMAX',0.,.25)]:
    s.gfstol(1e-6);out=s.cell_double(1000)
    s.gfuds(udf,decreasing,op,ref,adjust,.7,1000,cell(windows),out)
    rows.append([name,dx,op,ref,adjust,packed(cell(windows)),packed(out)])
with open(root/'tests/golden/gf_uddc_precision.csv','w',newline='') as f:
 w=csv.writer(f);w.writerow(['function','derivative_step','relation','reference','adjustment','confinement','expected']);w.writerows(rows)
with open(root/'tests/golden/uddf_precision.csv','w',newline='') as f:
 w=csv.writer(f);w.writerow(['function','epoch','derivative_step','derivative','decreasing']);w.writerows(primitives)
print(len(rows),'GFUDS/UDDC cases;',len(primitives),'UDDF/UDDC primitives')

# Original regression controls, retained as native outputs rather than
# mathematical roots. GFUDS's interval refinement need not return exact zero.
for name in ['de440s.bsp','pck00011.tpc']:s.furnsh(str(root/'target/kernels'/name))
regressions=[]
solar=lambda et:float(s.subslr('NEAR POINT/ELLIPSOID','MOON',et,'IAU_MOON','CN','EARTH')[0][0])
for name,fn,intervals,step,reference in [
 ('small_linear',lambda t:1e-9*t,[(-10.,10.)],1.,0.),
 ('closed_cubic',lambda t:t*(t-2.)*(t-4.),[(0.,4.)],1.,0.),
 ('solar_sublr',solar,[(0.,0.)],3600.,solar(0.))]:
 udf=SpiceUDFUNS(fn);dec=SpiceUDFUNB(lambda u,t:s.uddc(u,t,1.));out=s.cell_double(1000);s.gfstol(1e-6)
 s.gfuds(udf,dec,'=',reference,0.,step,1000,cell(intervals),out)
 regressions.append([name,reference,step,packed(cell(intervals)),packed(out)])
with open(root/'tests/golden/gf_uddc_regressions.csv','w',newline='') as f:
 w=csv.writer(f);w.writerow(['function','reference','step','confinement','expected']);w.writerows(regressions)
