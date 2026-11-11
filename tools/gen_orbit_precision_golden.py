"""Independent CSPICE OSCELT/OSCLTX reference vectors."""
from pathlib import Path
import csv,random,spiceypy as s
root=Path(__file__).resolve().parents[1]
rng=random.Random(6702)
rows=[]
for e in [0.,.2,.99,1.,1.+5e-11,1.2,3.]:
 for inc in [0.,.4,3.141592653589793]:
  for m in [-1.,0.,1.5]:
   st=s.conics([7000.,e,inc,.7,.9,m,0.,398600.],0.)
   rows.append((list(st),398600.))
for _ in range(100):
 rows.append(([rng.uniform(-2e5,2e5) for _ in range(3)]+[rng.uniform(-10,10) for _ in range(3)],398600.))
with (root/'tests/golden/orbit_precision.csv').open('w',newline='') as f:
 w=csv.writer(f)
 for state,gm in rows:
  el=s.oscltx(state,0.,gm)
  w.writerow([*state,gm,*el[:6],el[8],el[9],el[10],s.vnorm(state[:3]),s.vnorm(state[3:]),gm])
s.furnsh(str(root/'target/kernels/de440s.bsp'))
s.furnsh(str(root/'target/kernels/gm_de440.tpc'))
with (root/'tests/golden/orbit_observations_precision.csv').open('w',newline='') as f:
 w=csv.writer(f)
 for orbiter,center in [(301,399),(399,10),(199,10)]:
  for et in [0.,5e8]:
   for corr in ['NONE','LT','LT+S','CN','CN+S','XLT','XLT+S','XCN','XCN+S']:
    for frameid in s.bltfrm(1):
     frame=s.frmnam(frameid)
     state,_=s.spkezr(str(orbiter),et,frame,corr,str(center))
     cg=s.bodvcd(center,'GM',1)[1][0];og=s.bodvcd(orbiter,'GM',1)[1][0]
     for mode,gm in [('CENTER',cg),('CENTER_ORBITER',cg+og),('CUSTOM',cg*1.02)]:
      el=s.oscltx(state,et,gm)
      w.writerow([orbiter,center,et,corr,frame,mode,gm,*el[:6],el[8],el[9],el[10],s.vnorm(state[:3]),s.vnorm(state[3:]),gm,*state])
