"""Independent CSPICE N0067 explicit target-frame surface oracle.
Uses actual binary lunar orientation plus a target-centered TKFRAME.
Synthetic triaxial radii prove the selected frame defines the ellipsoid.
"""
import csv
from pathlib import Path
import spiceypy as s
root=Path(__file__).resolve().parents[1]
fk='''KPL/FK
\\begindata
FRAME_TEST_MOON_TK = 1499301
FRAME_1499301_NAME = 'TEST_MOON_TK'
FRAME_1499301_CLASS = 4
FRAME_1499301_CLASS_ID = 1499301
FRAME_1499301_CENTER = 301
TKFRAME_1499301_RELATIVE = 'MOON_ME'
TKFRAME_1499301_SPEC = 'ANGLES'
TKFRAME_1499301_UNITS = 'DEGREES'
TKFRAME_1499301_AXES = (3 2 1)
TKFRAME_1499301_ANGLES = (12.3 -4.5 6.7)
\\begintext
'''
(root/'tests/golden/surface_target_frames.tf').write_text(fk)
for name in ['de440s.bsp','pck00011.tpc','moon_pa_de440_200625.bpc','moon_de440_250416.tf','moon_assoc_me.tf']:
 s.furnsh(str(root/'target/kernels'/name))
s.furnsh(str(root/'tests/golden/surface_target_frames.tf'))
corrections=['NONE','LT','LT+S','CN','CN+S','XLT','XLT+S','XCN','XCN+S']
with (root/'tests/golden/surface_target_frames.csv').open('w',newline='') as handle:
 w=csv.writer(handle);w.writerow(['scenario','kind','frame','et','corr','method','in0','in1','in2','expected...'])
 count=0
 def emit(kind,frame,et,corr,method,inputs,outputs):
  global count
  w.writerow([scenario,kind,frame,repr(et),corr,method,*[repr(float(x)) for x in inputs],*[repr(float(x)) if not isinstance(x,bool) else str(x).lower() for x in outputs]]);count+=1
 for scenario in ['native','triaxial']:
  if scenario=='triaxial':s.pdpool('BODY301_RADII',[1738.1,1736.7,1734.2])
  for frame in ['MOON_ME','MOON_PA','TEST_MOON_TK']:
   for et in [-5e7,0.,5e7]:
    for corr in corrections:
     direction=s.spkpos('301',et,'J2000',corr,'399')[0]
     for method in ['NEAR POINT/ELLIPSOID','INTERCEPT/ELLIPSOID']:
      p,epoch,v=s.subpnt(method,'301',et,frame,corr,'399');emit('SUBPNT',frame,et,corr,method,[0,0,0],[*p,epoch,*v])
      if not corr.startswith('X'):
       p,epoch,v=s.subslr(method,'301',et,frame,corr,'399');emit('SUBSLR',frame,et,corr,method,[0,0,0],[*p,epoch,*v])
     p,epoch,v=s.sincpt('ELLIPSOID','301',et,frame,corr,'399','J2000',direction);emit('SINCPT',frame,et,corr,'-',direction,[*p,epoch,*v])
     point=s.subpnt('NEAR POINT/ELLIPSOID','301',et,frame,'NONE','399')[0]
     epoch,v,phase,inc,emi,visible,lit=s.illumf('ELLIPSOID','301','10',et,frame,corr,'399',point);emit('ILLUMF',frame,et,corr,'-',point,[epoch,*v,phase,inc,emi,bool(visible),bool(lit)])
     perpendicular=s.ucrss(direction,[0,0,1])
     for offset in [0.,2500.,None]:
      ray=direction+perpendicular*offset if offset is not None else -direction
      for locus in ['TANGENT POINT','SURFACE POINT']:
       p,alt,rng,srf,epoch,v=s.tangpt('ELLIPSOID','301',et,frame,corr,locus,'399','J2000',ray);emit('TANGPT',frame,et,corr,locus,ray,[*p,alt,rng,*srf,epoch,*v])
print(count,'independent named-frame surface rows')
