"""Independent CSPICE N0067 surface regression vectors; no network required."""
from pathlib import Path
import csv
import spiceypy as s
root = Path(__file__).resolve().parents[1]
s.furnsh(str(root/'target/kernels/de440s.bsp'))
s.furnsh(str(root/'target/kernels/pck00011.tpc'))
with (root/'tests/golden/surface_precision.csv').open('w',newline='') as f:
 w=csv.writer(f); w.writerow(['kind','target','observer','et','abcorr','method_or_source','px','py','pz','epoch','vx','vy','vz','phase','solar','emission'])
 for target,observer in [(399,301),(301,399),(399,10),(499,399)]:
  # de440s has no Mars center segment.
  if target==499: continue
  frame='IAU_EARTH' if target==399 else 'IAU_MOON'
  for et in [0.,5e8,-1e8]:
   for corr in ['NONE','LT','LT+S','CN','CN+S','XLT','XLT+S','XCN','XCN+S']:
    for method in ['NEAR POINT/ELLIPSOID','INTERCEPT/ELLIPSOID']:
     for kind,fn in [('observer',s.subpnt),('solar',s.subslr)]:
      if kind=='solar' and corr.startswith('X'): continue
      p,t,v=fn(method,str(target),et,frame,corr,str(observer))
      w.writerow([kind,target,observer,et,corr,method,*p,t,*v,0,0,0])
    if corr.startswith('X'): continue
    for src in [10,301 if target==399 else 399]:
     p=s.georec(.7,.4,0.,6378.1366,1-6356.7519/6378.1366) if target==399 else s.latrec(1737.4,.7,.4)
     t,v,phase,solar,emission=s.illumg('ELLIPSOID',str(target),str(src),et,frame,corr,str(observer),p)
     w.writerow(['illum',target,observer,et,corr,src,*p,t,*v,phase,solar,emission])

# NEARPT is independently checked without ephemerides or frame rotations.
import random
rng=random.Random(672023)
with (root/'tests/golden/nearpt_precision.csv').open('w',newline='') as f:
 w=csv.writer(f)
 cases=[([10000.,3000.,4000.],[6378.1366,6378.1366,6356.7519]),([300000.,-200000.,100000.],[1000.,2000.,3000.]),([1e8,-5e7,4e7],[1737.4]*3),([5000.,0.,0.],[1000.,2000.,3000.]),([0.,5000.,0.],[1000.,2000.,3000.])]
 for _ in range(200):
  axes=[10**rng.uniform(1,4) for _ in range(3)]
  p=[rng.uniform(-1,1)*max(axes)*10**rng.uniform(1,5) for _ in range(3)]
  cases.append((p,axes))
 for p,axes in cases:
  q,h=s.nearpt(p,*axes);w.writerow([*p,*axes,*q,h])
with (root/'tests/golden/solar_longitude_precision.csv').open('w',newline='') as f:
 w=csv.writer(f)
 for body in [399,301,199,299]:
  for et in [0.,5e8,-1e8]:
   for corr in ['NONE','LT','LT+S','CN','CN+S']:
    w.writerow([body,et,corr,s.lspcn(str(body),et,corr)])
with (root/'tests/golden/georec_precision.csv').open('w',newline='') as f:
 w=csv.writer(f)
 for lon,lat,alt in [(0.7,0.4,0.),(-2.,1.2,10.),(1.,-.3,-1.)]:
  for axes in [[6378.1366,6378.1366,6356.7519],[1737.4]*3]:
   p=s.georec(lon,lat,alt,axes[0],1-axes[2]/axes[0]);w.writerow([lon,lat,alt,*axes,*p])
with (root/'tests/golden/nearpt_precision.csv').open('a',newline='') as f:
 w=csv.writer(f)
 for p,axes in [([0.,0.,0.],[1000.,2000.,3000.]),([0.,10.,0.],[1000.,2000.,3000.]),([0.,0.,10.],[1000.,1000.,3000.])]+[([rng.uniform(-100,100) for _ in range(3)],[1000.,2000.,3000.]) for _ in range(100)]:
  q,h=s.nearpt(p,*axes);w.writerow([*p,*axes,*q,h])
