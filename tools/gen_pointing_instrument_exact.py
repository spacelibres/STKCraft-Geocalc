"""Generate exact instrument pointing fixtures from CSPICE GETFOV on identical IK text."""
from pathlib import Path
import spiceypy as s
IK=r'''\begindata
INS-999_FOV_CLASS_SPEC = 'CORNERS'
INS-999_FOV_SHAPE = 'RECTANGLE'
INS-999_FOV_FRAME = 'J2000'
INS-999_BORESIGHT = (0.1,0.2,7.3)
INS-999_FOV_BOUNDARY_CORNERS = (1,1,4,-1,1,4,-1,-1,4,1,-1,4)
\begintext'''
s.lmpool([line for line in IK.splitlines() if not line.startswith('\\')])
shape,frame,boresight,n,bounds=s.getfov(-999,10)
out=Path('tests/golden/pointing_instrument_exact.csv')
with out.open('w',newline='\n') as h:
 h.write('# CSPICE GETFOV, PXFORM, VHAT; IK text in gen_pointing_instrument_exact.py\n')
 h.write('kind,index,output_frame,magnitude,raw_x,raw_y,raw_z,result_x,result_y,result_z\n')
 for kind,index,raw in [('BORESIGHT',0,boresight)]+[('BOUNDARY',i+1,bounds[i]) for i in range(n)]:
  for frame in ['J2000','ECLIPJ2000']:
   for mag in ['UNIT','PRESERVE_ORIGINAL']:
    vector=s.mxv(s.pxform('J2000',frame,0.),raw)
    if mag=='UNIT':vector=s.vhat(vector)
    h.write(','.join(map(str,[kind,index,frame,mag,*raw,*vector]))+'\n')
s.kclear();print('wrote',out)
