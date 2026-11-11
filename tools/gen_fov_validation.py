"""GETFOV input validation and competing boundary-keyword priority."""
from pathlib import Path
import csv
import spiceypy as s
root=Path(__file__).resolve().parents[1]
cases=[]
for shape,count in [('CIRCLE',1),('CIRCLE',2),('ELLIPSE',2),('ELLIPSE',3),('RECTANGLE',4),('RECTANGLE',3),('POLYGON',3),('POLYGON',2)]:
    cases.append((shape,{'FOV_BOUNDARY':f'( '+', '.join(map(str,[.2,.1,1.,-.2,.1,1.,-.2,-.1,1.,.2,-.1,1.][:count*3]))+' )'}))
cases += [('CIRCLE',{'FOV_CLASS_SPEC':"'BOGUS'",'FOV_BOUNDARY':'( 0.2,0,1 )'}),
          ('CIRCLE',{'FOV_CLASS_SPEC':"'ANGLES'",'FOV_REF_VECTOR':'( 1,0,0 )','FOV_REF_ANGLE':'0.1'}),
          ('CIRCLE',{'BORESIGHT':'( 0,0,1,2 )','FOV_BOUNDARY':'( 0.2,0,1 )'}),
          ('CIRCLE',{'FOV_CLASS_SPEC':"'ANGLES'",'FOV_REF_VECTOR':'( 1,0,0,2 )','FOV_REF_ANGLE':'0.1','FOV_ANGLE_UNITS':"'RADIANS'"}),
          ('CIRCLE',{'FOV_BOUNDARY':'( 0.2,0,1 )','FOV_BOUNDARY_CORNERS':'( 0.3,0,1 )'})]
text=['KPL/IK','\\begindata']
for offset,(shape,fields) in enumerate(cases):
    ident=929000+offset
    pool={'FOV_SHAPE':repr(shape),'FOV_FRAME':"'J2000'",'BORESIGHT':'( 0,0,1 )',**fields}
    text.extend(f'INS{ident}_{key} = {value}' for key,value in pool.items())
path=root/'tests/golden/fov_validation.ti';path.write_text('\n'.join(text)+'\n\\begintext\n')
s.furnsh(str(path))
with (root/'tests/golden/fov_validation.csv').open('w',newline='') as file:
    out=csv.writer(file);out.writerow(['instrument','valid','first_boundary_x'])
    for offset in range(len(cases)):
        ident=929000+offset
        try:
            _,_,_,_,bounds=s.getfov(ident,20)
            out.writerow([ident,1,repr(float(bounds[0][0]))])
        except s.utils.exceptions.SpiceyError:
            out.writerow([ident,0,0])
s.kclear()
