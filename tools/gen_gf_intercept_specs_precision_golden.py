"""Native four fixed-ray GFSNTC specification oracles."""
from pathlib import Path
import csv,spiceypy as s
root=Path(__file__).resolve().parents[1]
s.furnsh(str(root/'target/kernels/de442s.bsp'));s.furnsh(str(root/'target/kernels/pck00011.tpc'));s.furnsh(str(root/'tests/golden/gf_intercept_dref.tf'))
boresight=s.spkpos('MOON',0.,'GF_DREF_MOON','NONE','EARTH')[0]
x,y,z=s.frame(boresight)
axis_matrix=[*x,*y,*z]
fk='''KPL/IK
\\begindata
INS1481001_FOV_CLASS_SPEC='ANGLES'
INS1481001_FOV_SHAPE='CIRCLE'
INS1481001_FOV_FRAME='GF_DREF_MOON'
INS1481001_BORESIGHT=(%s)
INS1481001_FOV_REF_VECTOR=(0,0,1)
INS1481001_FOV_REF_ANGLE=1
INS1481001_FOV_ANGLE_UNITS='DEGREES'
FRAME_1481002_NAME='GF_SPEC_AXIS'
FRAME_GF_SPEC_AXIS=1481002
FRAME_1481002_CLASS=4
FRAME_1481002_CLASS_ID=1481002
FRAME_1481002_CENTER=301
TKFRAME_1481002_RELATIVE='GF_DREF_MOON'
TKFRAME_1481002_SPEC='MATRIX'
TKFRAME_1481002_MATRIX=(%s)
\\begintext
'''%(','.join(repr(float(v)) for v in boresight),',\n'.join(','.join(repr(float(v)) for v in axis_matrix[i:i+3]) for i in range(0,9,3)))
(root/'tests/golden/gf_intercept_specs.ti').write_text(fk)
s.furnsh(str(root/'tests/golden/gf_intercept_specs.ti'))
_,fovframe,bore,_,_=s.getfov(1481001,10)
r=s.bodvrd('MOON','RADII',3)[1]
def cf():
 c=s.cell_double(1000);s.wninsd(0.,1000.,c);return c
def packed(w):return ';'.join(repr(a)+':'+repr(b) for a,b in [s.wnfetd(w,i) for i in range(s.wncard(w))])
rows=[]
for corr in ['NONE','LT','LT+S','CN','CN+S','XLT','XLT+S','XCN','XCN+S']:
 for kind,frame,dvec in [('BORESIGHT',fovframe,bore),('AXIS','GF_SPEC_AXIS',[1.,0.,0.]),('VECTOR_FOV',fovframe,s.spkpos('MOON',0.,fovframe,corr,'EARTH')[0]),('VECTOR_FRAME','ECLIPJ2000',s.spkpos('MOON',0.,'ECLIPJ2000',corr,'EARTH')[0])]:
  point=s.sincpt('ELLIPSOID','MOON',0.,'IAU_MOON',corr,'EARTH',frame,dvec)[0]
  for system,coordinate,ref in [('RECTANGULAR','X',point[0]),('LATITUDINAL','LONGITUDE',s.reclat(point)[1]),('PLANETOGRAPHIC','LATITUDE',s.recpgr('MOON',point,r[0],0.)[1])]:
   for op in ['<','>','=','ABSMAX','ABSMIN','LOCMAX','LOCMIN']:
    w=s.gfsntc('MOON','IAU_MOON','ELLIPSOID',corr,'EARTH',frame,dvec,system,coordinate,op,float(ref),0.,10.,1000,cf())
    rows.append([kind,corr,frame,*dvec,system,coordinate,op,ref,packed(w)])
with open(root/'tests/golden/gf_intercept_specs_precision.csv','w',newline='') as f:
 w=csv.writer(f);w.writerow(['kind','correction','frame','dx','dy','dz','system','coordinate','relation','reference','expected']);w.writerows(rows)
print(len(rows),'four-spec GFSNTC source cases')
