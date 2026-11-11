"""CSPICE surface oracle with a non-inertial SPK source segment."""
import csv
from pathlib import Path
import spiceypy as s
root=Path(__file__).resolve().parents[1]
text='''KPL/FK
\\begindata
BODY90001_RADII=(1000 800 700)
FRAME_SURFACE_MIXED_TARGET=1590002
FRAME_1590002_NAME='SURFACE_MIXED_TARGET'
FRAME_1590002_CLASS=4
FRAME_1590002_CLASS_ID=1590002
FRAME_1590002_CENTER=90001
TKFRAME_1590002_RELATIVE='IAU_MOON'
TKFRAME_1590002_SPEC='ANGLES'
TKFRAME_1590002_UNITS='DEGREES'
TKFRAME_1590002_ANGLES=(7 -9 11)
TKFRAME_1590002_AXES=(3 2 1)
\\begintext
'''
fk=root/'tests/golden/surface_rotating_spk.tf';fk.write_text(text)
for file in [root/'target/kernels/de440s.bsp',root/'target/kernels/pck00011.tpc',root/'tests/golden/spk_mixed_frames.bsp',fk]:s.furnsh(str(file))
corrections=['NONE','LT','LT+S','CN','CN+S','XLT','XLT+S','XCN','XCN+S'];frame='SURFACE_MIXED_TARGET';count=0
with (root/'tests/golden/surface_rotating_spk.csv').open('w',newline='')as h:
 w=csv.writer(h);w.writerow(['scenario','kind','frame','et','corr','method','in0','in1','in2','expected...'])
 def emit(kind,et,corr,method,inputs,outputs):
  global count
  w.writerow(['native',kind,frame,repr(et),corr,method,*[repr(float(x))for x in inputs],*[str(x).lower()if isinstance(x,bool)else repr(float(x))for x in outputs]]);count+=1
 for et in [-12345.67,0.,23456.78]:
  for corr in corrections:
   direction=s.spkpos('90001',et,'J2000',corr,'399')[0]
   for method in ['NEAR POINT/ELLIPSOID','INTERCEPT/ELLIPSOID']:
    p,epoch,v=s.subpnt(method,'90001',et,frame,corr,'399');emit('SUBPNT',et,corr,method,[0,0,0],[*p,epoch,*v])
    if not corr.startswith('X'):
     p,epoch,v=s.subslr(method,'90001',et,frame,corr,'399');emit('SUBSLR',et,corr,method,[0,0,0],[*p,epoch,*v])
   p,epoch,v=s.sincpt('ELLIPSOID','90001',et,frame,corr,'399','J2000',direction);emit('SINCPT',et,corr,'-',direction,[*p,epoch,*v])
   point=s.subpnt('NEAR POINT/ELLIPSOID','90001',et,frame,'NONE','399')[0]
   epoch,v,phase,inc,emi,visible,lit=s.illumf('ELLIPSOID','90001','10',et,frame,corr,'399',point);emit('ILLUMF',et,corr,'-',point,[epoch,*v,phase,inc,emi,bool(visible),bool(lit)])
   other=s.ucrss(direction,[0,0,1])
   for offset in [0.,2500.]:
    ray=direction+other*offset
    for locus in ['TANGENT POINT','SURFACE POINT']:
     p,alt,rng,srf,epoch,v=s.tangpt('ELLIPSOID','90001',et,frame,corr,locus,'399','J2000',ray);emit('TANGPT',et,corr,locus,ray,[*p,alt,rng,*srf,epoch,*v])
print(count,'native rotating-source surface rows')
