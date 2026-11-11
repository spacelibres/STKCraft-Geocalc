"""Native legal eccentricity boundary for SPKW17/EQNCPV (ecc >0.9 errors)."""
from pathlib import Path
import csv,math
import spiceypy as s
root=Path(__file__).resolve().parents[1];path=root/'tests/golden/spk17_boundary.bsp'
if path.exists():path.unlink()
h=s.spkopn(str(path),'Exact0.9equinoctialboundary',0)
for i,(a,b)in enumerate([(0.9,0.),(0.,0.9),(math.nextafter(.9,0.),0.)]):
 eq=[1e6,a,b,.2,.1,-.2,1e-7,.0003,-2e-7]
 s.spkw17(h,717100+i,0,'J2000',-1000.,1000.,str(i),0.,eq,.3,.4)
s.spkcls(h);s.furnsh(str(path))
with (root/'tests/golden/spk17_boundary.csv').open('w',newline='')as f:
 w=csv.writer(f);w.writerow(['body','et','frame','x','y','z','vx','vy','vz'])
 for i in range(3):
  for et in [-900.9,-855.4,-800.8]:
   for frame in [1,2,13,21]:
    state,_=s.spkgeo(717100+i,et,s.frmnam(frame),0);w.writerow([717100+i,repr(et),frame,*[repr(float(x))for x in state]])
