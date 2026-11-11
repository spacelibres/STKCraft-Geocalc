"""Independent native Moon SPKPOS/SPKEZR and GFPOSC dynamic-frame CLI cases."""
from pathlib import Path
import csv
import spiceypy as s
ROOT=Path(__file__).resolve().parents[1];OUT=ROOT/'tests/golden'
for name in ['target/kernels/de442.bsp','target/kernels/pck00011.tpc','tests/golden/dynamic_ephemeris.tf']:s.furnsh(str(ROOT/name))
with (OUT/'cli_ephemeris_context.csv').open('w',newline='')as f:
 w=csv.writer(f);w.writerow(['kind','frame','et','correction','x','y','z'])
 for kind in ['POSITION','VELOCITY']:
  for frame in ['EPHEM_TWO_0','EPHEM_TWO_1','EPHEM_TWO_2']:
   for et in [-12345.67,23456.78]:
    for correction in ['NONE','LT','LT+S','CN','CN+S','XLT','XLT+S','XCN','XCN+S']:
     v=s.spkpos('301',et,frame,correction,'399')[0]if kind=='POSITION'else s.spkezr('301',et,frame,correction,'399')[0][3:]
     w.writerow([kind,frame,repr(et),correction,*map(repr,map(float,v))])
window=s.utils.support_types.SPICEDOUBLE_CELL(4);s.wninsd(0.,2000000.,window)
result=s.gfposc('301','EPHEM_TWO_0','NONE','399','RECTANGULAR','X','>',400000.,0.,10800.,1000,window)
with (OUT/'cli_ephemeris_context_gf.csv').open('w',newline='')as f:
 w=csv.writer(f);w.writerow(['start','stop','duration'])
 for i in range(s.wncard(result)):
  a,b=s.wnfetd(result,i);w.writerow([repr(a),repr(b),repr(b-a)])
