"""Native surface APIs with dynamic FIXREF and rotating SPK source frames."""
from pathlib import Path
import csv
import numpy as np
import spiceypy as s
root=Path(__file__).resolve().parents[1];out=root/'tests/golden/dsk'
fk=(root/'tests/golden/dynamic_euler.tf').read_text().replace('_CENTER=499','_CENTER=301').replace("'IAU_MARS'","'IAU_MOON'");(out/'dynamic_frames.tf').write_text(fk)
rows=[]
for shape in ['ellipsoid','dsk']:
 s.kclear()
 for file in ['target/kernels/de440s.bsp','target/kernels/pck00011.tpc','tests/golden/spk_dynamic_sources.bsp','tests/golden/dsk/dynamic_frames.tf']:s.furnsh(str(root/file))
 if shape=='dsk':s.furnsh(str(out/'dense_lat.bds'));s.pdpool('BODY301_RADII',[3.2,4.1,4.9])
 for observer in ['399','91002']:
  for frame in ['DYN_EULER_0','DYN_EULER_12','DYN_EULER_13','DYN_EULER_14']:
   for et in [-12345.67,0.,23456.78]:
    for corr in ['NONE','LT','LT+S','CN','CN+S','XLT','XLT+S','XCN','XCN+S']:
     center=s.spkpos('301',et,'J2000',corr,observer)[0]
     method='ELLIPSOID' if shape=='ellipsoid' else 'DSK/UNPRIORITIZED'
     for kind in ['intercept','subNear','subIntercept','solarNear','solarIntercept']:
      try:
       if kind=='intercept':point,epoch,vector=s.sincpt(method,'301',et,frame,corr,observer,'J2000',center)
       else:
        submethod=('NEAR POINT' if shape=='ellipsoid' else 'NADIR') if kind.endswith('Near') else 'INTERCEPT'
        point,epoch,vector=(s.subslr if kind.startswith('solar') else s.subpnt)(submethod+'/'+method,'301',et,frame,corr,observer)
       expected=';'.join(map(repr,map(float,[*point,epoch,*vector,s.vnorm(vector)])))
      except s.utils.exceptions.NotFoundError:expected=''
      except s.utils.exceptions.SpiceyError as e:expected='ERROR:'+e.short
      rows.append([shape,kind,frame,et,observer,corr,*map(float,center),expected])
s.kclear()
with (out/'dynamic_surface.csv').open('w',newline='') as f:
 w=csv.writer(f);w.writerow(['shape','kind','frame','epoch','observer','correction','dx','dy','dz','expected']);w.writerows(rows)
print(len(rows),'native dynamic FIXREF surface results')
