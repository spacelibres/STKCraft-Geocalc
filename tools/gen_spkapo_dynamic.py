"""Independent SPKEZP with dynamic input/output frames and mixed sources."""
from pathlib import Path
import csv,numpy as np
import spiceypy as s
ROOT=Path(__file__).resolve().parents[1]
for file in ['target/kernels/de440s.bsp','target/kernels/pck00011.tpc','tests/golden/spk_mixed_frames.bsp','tests/golden/dynamic_euler.tf','tests/golden/dynamic_two.tf','tests/golden/dynamic_of_date.tf']:s.furnsh(str(ROOT/file))
p=ROOT/'tests/golden/spk_dynamic_sources.bsp';p.unlink(missing_ok=True);handle=s.spkopn(str(p),'DYNAMIC SOURCE FRAME TEST',0)
for target,center,frame in [(91001,399,'DYN_EULER_13'),(91002,91001,'DYN_EULER_0')]:
 coeff=np.array([1e6,123.4,.9, .003,-2e6,-89.,-.7,-.002,3e6,67.,.5,.001]*2,dtype=float)
 s.spkw02(handle,target,center,frame,-2000000.,2000000.,'DYNAMIC REFCHG',2000000.,2,3,coeff,-2000000.)
s.spkw02(handle,499,4,'J2000',-4000000000.,4000000000.,'MARS CENTER FIXTURE',4000000000.,2,3,np.array([5.,0.,0.,0.,6.,0.,0.,0.,7.,0.,0.,0.]*2),-4000000000.)
s.spkcls(handle);s.furnsh(str(p))
with (ROOT/'tests/golden/spkapo_dynamic.csv').open('w',newline='',encoding='utf-8') as f:
 w=csv.writer(f);w.writerow(['target','observer','et','frame','correction','x','y','z','lt']);count=0
 for target,observer in [(301,399),(90001,399),(91002,399),(91002,91001),(21002,22018),(30000,31001),(31001,30000)]:
  for et in [-12345.67,0.,23456.78]:
   for frame in ['J2000','ECLIPJ2000','DYN_EULER_0','DYN_EULER_12','DYN_TWO_0','OF_DATE_0']:
    for correction in ['NONE','LT','LT+S','CN','CN+S','XLT','XLT+S','XCN','XCN+S']:
     pos,lt=s.spkpos(str(target),et,frame,correction,str(observer));w.writerow([target,observer,et,frame,correction,*pos.tolist(),lt]);count+=1
print(count,'native SPKEZP dynamic frame cases')
