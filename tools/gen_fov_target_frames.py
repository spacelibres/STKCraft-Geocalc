"""FOVTRG reference with a selectable, rotated target ellipsoid frame."""
from pathlib import Path
import csv
import spiceypy as s
root=Path(__file__).resolve().parents[1]
s.furnsh(str(root/'target/kernels/de440s.bsp'));s.furnsh(str(root/'target/kernels/pck00011.tpc'))
lines=['KPL/FK','\\begindata','BODY399_RADII = ( 6378.1366,4200.0,3000.0 )',
 "FRAME_ROTATED_EARTH = 939000","FRAME_939000_NAME = 'ROTATED_EARTH'",
 'FRAME_939000_CLASS = 4','FRAME_939000_CLASS_ID = 939000','FRAME_939000_CENTER = 399',
 "TKFRAME_939000_RELATIVE = 'IAU_EARTH'","TKFRAME_939000_SPEC = 'ANGLES'",
 'TKFRAME_939000_ANGLES = ( 25,35,47 )','TKFRAME_939000_AXES = ( 3,2,1 )',"TKFRAME_939000_UNITS = 'DEGREES'"]
cases=[]
for epoch in [-5e8,0.,5e8]:
    position,_=s.spkezp(399,epoch,'J2000','NONE',301)
    base=s.twovec(position,3,[0.,0.,1.],1)
    for shape in ['CIRCLE','ELLIPSE','RECTANGLE','POLYGON']:
        for offset in [.0,.01,.018,.03]:
            ident=939001+len(cases);frame=f'FOV_FRAME_{ident}'
            matrix=s.mxm(s.rotate(offset,2),base)
            lines += [f"FRAME_{frame} = {ident}",f"FRAME_{ident}_NAME = '{frame}'",f'FRAME_{ident}_CLASS = 4',
                      f'FRAME_{ident}_CLASS_ID = {ident}',f'FRAME_{ident}_CENTER = 301',
                      f"TKFRAME_{ident}_RELATIVE = 'J2000'",f"TKFRAME_{ident}_SPEC = 'MATRIX'",
                      f'TKFRAME_{ident}_MATRIX = ( '+',\n '.join(repr(float(x)) for x in matrix.flat)+' )',
                      f"INS{ident}_FOV_FRAME = '{frame}'",f"INS{ident}_FOV_SHAPE = '{shape}'",f'INS{ident}_BORESIGHT = ( 0,0,1 )']
            if shape=='POLYGON':
                lines += [f'INS{ident}_FOV_BOUNDARY = ( .004,.003,1, -.004,.003,1, -.004,-.003,1, .004,-.003,1 )']
            else:
                lines += [f"INS{ident}_FOV_CLASS_SPEC = 'ANGLES'",f"INS{ident}_FOV_ANGLE_UNITS = 'RADIANS'",
                          f'INS{ident}_FOV_REF_VECTOR = ( 1,0,0 )',f'INS{ident}_FOV_REF_ANGLE = .004',f'INS{ident}_FOV_CROSS_ANGLE = .003']
            cases.append((ident,epoch))
path=root/'tests/golden/fov_target_frames.tf';path.write_text('\n'.join(lines)+'\n\\begintext\n')
s.furnsh(str(path))
with (root/'tests/golden/fov_target_frames.csv').open('w',newline='') as file:
    out=csv.writer(file);out.writerow(['instrument','et','correction','target_frame','visible'])
    for ident,epoch in cases:
        for correction in ['NONE','LT','LT+S','CN','CN+S','XLT','XLT+S','XCN','XCN+S']:
            for frame in ['IAU_EARTH','ROTATED_EARTH']:
                value=s.fovtrg(str(ident),'EARTH','ELLIPSOID',frame,correction,'MOON',epoch)
                out.writerow([ident,repr(epoch),correction,frame,int(value)])
s.kclear()
