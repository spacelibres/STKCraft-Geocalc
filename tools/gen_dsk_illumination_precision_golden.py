"""Native ILLUMF DSK angles, self-occultation flags, and normal-selection union."""
from pathlib import Path
import csv
import numpy as np
import spiceypy as s
root=Path(__file__).resolve().parents[1];out=root/'tests/golden/dsk';rows=[]
for files in [['dense_lat.bds'],['optimized.bds'],['mixed_frames.bds'],['dense_lat.bds','dense_partial.bds']]:
 s.kclear()
 for file in ['target/kernels/de440s.bsp','target/kernels/pck00011.tpc','tests/golden/spk_dynamic_sources.bsp','tests/golden/dsk/frames.tf','tests/golden/dsk/dynamic_frames.tf']:s.furnsh(str(root/file))
 for file in files:s.furnsh(str(out/file))
 for frame in ['IAU_MOON','DSK_LOCAL','DYN_EULER_0']:
  for et in [-10.,0.,10.]:
   # Native LATSRF generates valid surface points, including concave mesh.
   lonlat=np.array([[lon,lat] for lon,lat in [(0.1,.2),(-2.,.4),(2.5,-.3),(1.2,-1.)]])
   points=s.latsrf('DSK/UNPRIORITIZED','MOON',et,frame,lonlat)
   for point in points:
    for observer in ['399','91002']:
     for illuminator in ['10','399']:
      for corr in ['NONE','LT','LT+S','CN','CN+S','XLT','XLT+S','XCN','XCN+S']:
       for surfaces in [[],[100]]:
        method='DSK/UNPRIORITIZED'+('/SURFACES=100' if surfaces else '')
        try:
         epoch,vec,phase,solar,emission,visible,lit=s.illumf(method,'MOON',illuminator,et,frame,corr,observer,point)
         expected=';'.join(map(repr,map(float,[phase,solar,emission,epoch,*vec,visible,lit])))
        except s.utils.exceptions.SpiceyError as e:expected='ERROR:'+e.short
        rows.append([';'.join(files),frame,et,observer,illuminator,corr,';'.join(map(str,surfaces)),*map(float,point),expected])
s.kclear()
with (out/'illumination.csv').open('w',newline='') as f:
 w=csv.writer(f);w.writerow(['kernels','frame','epoch','observer','illuminator','correction','surfaces','x','y','z','expected']);w.writerows(rows)
print(len(rows),'native DSK ILLUMF outputs')
