"""Independent native GF degenerate-confinement regressions (CSPICE N0067)."""
from pathlib import Path
import csv,spiceypy as s
root=Path(__file__).resolve().parents[1]
s.furnsh(str(root/'target/kernels/de442s.bsp'));s.furnsh(str(root/'target/kernels/pck00011.tpc'))
def packed(c):return ';'.join(repr(a)+':'+repr(b) for a,b in [s.wnfetd(c,i) for i in range(s.wncard(c))])
def cell(pairs):
 c=s.cell_double(500)
 for a,b in pairs:s.wninsd(a,b,c)
 return c
rows=[]
for corr in ['NONE','LT','LT+S','CN','CN+S','XLT','XLT+S','XCN','XCN+S']:
 for cfp in [[(5.,5.)],[(5.,5.),(1e6,1e6)]]:
  cf=cell(cfp)
  for op in ['<','>','=']:
   for ref in [0.,380000.,400000.,1e9]:
    w=s.gfdist('MOON',corr,'EARTH',op,ref,0.,60.,100,cf)
    rows.append(['DISTANCE',corr,'',op,ref,packed(cf),packed(w)])
   for system,coordinate in [('LATITUDINAL','LONGITUDE'),('CYLINDRICAL','LONGITUDE'),('RA/DEC','RIGHT ASCENSION')]:
    for ref in [-180.,-170.,0.,170.,180.,360.]:
     w=s.gfposc('MOON','IAU_EARTH',corr,'EARTH',system,coordinate,op,ref*s.rpd(),0.,60.,100,cf)
     rows.append(['COORDINATE',corr,system,op,ref,packed(cf),packed(w)])
with open(root/'tests/golden/gf_singleton_windows_precision.csv','w',newline='') as f:
 w=csv.writer(f);w.writerow(['family','correction','system','relation','reference','confinement','expected']);w.writerows(rows)
print(len(rows),'CSPICE singleton-confinement cases')
