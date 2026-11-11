"""Independent native SINCPT/SUBPNT over DSK, including source iteration metadata."""
from pathlib import Path
import csv
import numpy as np
import spiceypy as s
root=Path(__file__).resolve().parents[1];out=root/'tests/golden/dsk';rows=[]
for file in ['optimized.bds','dense_lat.bds']:
 s.kclear()
 for k in ['de440s.bsp','pck00011.tpc']:s.furnsh(str(root/'target/kernels'/k))
 s.furnsh(str(out/'frames.tf'));s.furnsh(str(out/file));s.pdpool('BODY301_RADII',[3.2,4.1,4.9])
 for frame in ['IAU_MOON','DSK_LOCAL']:
  for et in [0.,1e8,5e8]:
   for observer in ['EARTH','SUN']:
    for corr in ['NONE','LT','LT+S','CN','CN+S','XLT','XLT+S','XCN','XCN+S']:
     for surfaces in [[],[100]]:
      suffix='' if not surfaces else '/SURFACES='+','.join(map(str,surfaces))
      method='DSK/UNPRIORITIZED'+suffix
      position=s.spkpos('MOON',et,'J2000',corr,observer)[0]
      for offset in [[0.,0.,0.],[.3,-.2,.1],[3.,3.,3.]]:
       direction=position+s.mxv(s.pxform(frame,'J2000',et),offset)
       try:
        point,epoch,vector=s.sincpt(method,'MOON',et,frame,corr,observer,'J2000',direction)
        expected=[*point,epoch,*vector,s.vnorm(vector)]
       except s.utils.exceptions.NotFoundError:expected=[]
       except s.utils.exceptions.SpiceDSKDATANOTFOUND:expected='ERROR'
       encoded=expected if expected=='ERROR' else ';'.join(map(repr,map(float,expected)))
       rows.append([file,'intercept',frame,et,observer,corr,';'.join(map(str,surfaces)),*map(float,direction),encoded])
      for submethod in ['NADIR','INTERCEPT']:
       try:
        point,epoch,vector=s.subpnt(submethod+'/'+method,'MOON',et,frame,corr,observer)
        expected=[*point,epoch,*vector,s.vnorm(vector)]
       except (s.utils.exceptions.SpiceDSKDATANOTFOUND,s.utils.exceptions.SpiceSUBPOINTNOTFOUND):expected='ERROR'
       encoded=expected if expected=='ERROR' else ';'.join(map(repr,map(float,expected)))
       rows.append([file,submethod,frame,et,observer,corr,';'.join(map(str,surfaces)),0.,0.,0.,encoded])
s.kclear()
with (out/'surface.csv').open('w',newline='') as file:
 w=csv.writer(file);w.writerow(['kernel','kind','frame','epoch','observer','correction','surfaces','dx','dy','dz','expected']);w.writerows(rows)
print(len(rows),'native DSK SINCPT/SUBPNT results')
