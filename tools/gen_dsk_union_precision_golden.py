"""Independent native DSKXSI union/frame/surface/time-selection oracle."""
from pathlib import Path
import csv
import numpy as np
import spiceypy as s
root=Path(__file__).resolve().parents[1];out=root/'tests/golden/dsk'
fk=out/'frames.tf';fk.write_text('''KPL/FK
\\begindata
FRAME_DSK_LOCAL=1399301
FRAME_1399301_NAME='DSK_LOCAL'
FRAME_1399301_CLASS=4
FRAME_1399301_CLASS_ID=1399301
FRAME_1399301_CENTER=301
TKFRAME_1399301_RELATIVE='IAU_MOON'
TKFRAME_1399301_SPEC='ANGLES'
TKFRAME_1399301_ANGLES=(17.25 -31.5 11.1)
TKFRAME_1399301_AXES=(3 2 1)
TKFRAME_1399301_UNITS='DEGREES'
\\begintext
''')
s.furnsh(str(root/'target/kernels/pck00011.tpc'));s.furnsh(str(fk))
v=np.array([[3.,0.,0.],[-2.,0.,0.],[0.,4.,0.],[0.,-3.,0.],[0.,0.,5.],[0.,0.,-2.]])
p=np.array([[1,3,5],[3,2,5],[2,4,5],[4,1,5],[3,1,6],[2,3,6],[4,2,6],[1,4,6]],dtype=np.int32)
for name in ['mixed_frames','duplicate']:
 path=out/(name+'.bds')
 if path.exists():path.unlink()
 h=s.dskopn(str(path),name,0)
 for index in range(3):
  verts=v*(1. if name=='duplicate' else [1.,1.2,1.1][index]);d,i=s.dskmi2(verts,p,5.,4,100000,100000,100000,True,400000)
  lower,upper=s.dskrb2(verts,p,1,np.zeros(10));frame='DSK_LOCAL' if index==1 and name=='mixed_frames' else 'IAU_MOON'
  s.dskw02(h,301,100+index,2,frame,1,np.zeros(10),-np.pi,np.pi,-np.pi/2,np.pi/2,lower,upper,-500.-index*1000,500.+index*1000,verts,p,d,i)
 s.dskcls(h,True)
configs=[['optimized.bds'],['mixed_frames.bds'],['duplicate.bds'],['optimized.bds','duplicate.bds'],['duplicate.bds','optimized.bds'],['dense_lat.bds','dense_partial.bds','dense_rect.bds','dense_pdt.bds','dense_prolate.bds']]
rows=[];rng=np.random.default_rng(9541)
for files in configs:
 s.kclear();s.furnsh(str(root/'target/kernels/pck00011.tpc'));s.furnsh(str(fk))
 for file in files:s.furnsh(str(out/file))
 handles={s.kdata(j,'DSK')[3]:j for j in range(s.ktotal('DSK'))}
 for frame in ['IAU_MOON','DSK_LOCAL']:
  for et in [-2000.,0.,2000.]:
   for surfaces in [[],[100],[101,102]]:
    for j in range(80):
     vertex=rng.normal(size=3)*([.5,3.,10.,1e6][j%4]);direction=-vertex+rng.normal(size=3)*2.
     try:
      point,handle,dla,desc,dc,ic=s.dskxsi(False,'MOON',surfaces,et,frame,vertex,direction)
      expected=[handles[handle],dla.ibase,int(ic[0]),*point]
     except s.utils.exceptions.NotFoundError:expected=[]
     except s.utils.exceptions.SpiceDSKDATANOTFOUND:expected='ERROR'
     encoded=expected if expected=='ERROR' else ';'.join(map(repr,map(float,expected)))
     rows.append([';'.join(files),frame,et,';'.join(map(str,surfaces)),*map(float,vertex),*map(float,direction),encoded])
s.kclear()
with (out/'union.csv').open('w',newline='') as file:
 w=csv.writer(file);w.writerow(['kernels','frame','epoch','surfaces','vx','vy','vz','dx','dy','dz','expected']);w.writerows(rows)
print(len(rows),'native DSKXSI cases')
