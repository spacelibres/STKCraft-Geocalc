"""Source FOVRAY/GFRFOV/FOVTRG/GFTFOV across frames and corrections."""
from pathlib import Path
import csv
import spiceypy as s
root = Path(__file__).resolve().parents[1]
gold = root/'tests/golden'
lines = ['KPL/IK', '\\begindata']
for ident, shape, frame in [(919001,'CIRCLE','IAU_EARTH'),(919002,'ELLIPSE','ECLIPJ2000'),
                             (919003,'RECTANGLE','IAU_MOON'),(919004,'POLYGON','J2000')]:
    lines += [f"INS{ident}_FOV_SHAPE = '{shape}'", f"INS{ident}_FOV_FRAME = '{frame}'",
              f"INS{ident}_BORESIGHT = ( 1.0, 0.2, 0.3 )"]
    if shape == 'POLYGON':
        lines += [f'INS{ident}_FOV_BOUNDARY = ( 1.0, 0.9, 0.5, 1.0, -0.3, 0.8, 1.0, -0.5, -0.2, 1.0, 0.7, -0.4 )']
    else:
        lines += [f"INS{ident}_FOV_CLASS_SPEC = 'ANGLES'", f"INS{ident}_FOV_ANGLE_UNITS = 'DEGREES'",
                  f'INS{ident}_FOV_REF_VECTOR = ( 0.1, 1.0, 0.2 )', f'INS{ident}_FOV_REF_ANGLE = 40.0']
        if shape != 'CIRCLE': lines += [f'INS{ident}_FOV_CROSS_ANGLE = 25.0']
(gold/'fov_precision.ti').write_text('\n'.join(lines)+'\n\\begintext\n')
s.furnsh(str(root/'target/kernels/de440s.bsp'))
s.furnsh(str(root/'target/kernels/pck00011.tpc'))
s.furnsh(str(gold/'fov_precision.ti'))
corrections = ['NONE','LT','LT+S','CN','CN+S','XLT','XLT+S','XCN','XCN+S']
with (gold/'fov_precision.csv').open('w',newline='') as file:
    out=csv.writer(file);out.writerow(['kind','inst','correction','frame','x','y','z','et','visible'])
    for inst in range(919001,919005):
        for epoch in [-5e8,0.,5e8]:
            for offset in range(9):
                et=epoch+offset*3671.125
                for correction in corrections:
                    visible=s.fovtrg(str(inst),'MOON','POINT','IAU_MOON',correction,'EARTH',et)
                    out.writerow(['target',inst,correction,'J2000',0,0,0,repr(et),int(visible)])
            for correction in ['NONE','S','XS']:
                for frame in ['J2000','ECLIPJ2000','IAU_EARTH','IAU_MOON']:
                    for vector in [[1.,0.,0.],[0.,1.,0.],[1.,.2,.3],[-1.,-.2,-.3],[.5,-.9,.25]]:
                        visible=s.fovray(str(inst),vector,frame,correction,'EARTH',epoch)
                        out.writerow(['ray',inst,correction,frame,*map(repr,vector),repr(epoch),int(visible)])
with (gold/'fov_windows_exact.csv').open('w',newline='') as file:
    out=csv.writer(file);out.writerow(['kind','inst','correction','frame','step','x','y','z','interval','start','end'])
    cnfine=s.cell_double(2);s.wninsd(0.,259200.,cnfine)
    s.gfstol(1e-6)
    for inst in range(919001,919005):
        cases=[('target',c,'J2000',[0.,0.,0.]) for c in corrections]
        cases += [('ray',c,fr,[1.,.2,.3]) for c in ['NONE','S','XS'] for fr in ['J2000','IAU_MOON','IAU_EARTH']]
        for kind,c,fr,vector in cases:
            for step in [601.75,3671.125]:
                result=s.cell_double(2000)
                if kind=='target':s.gftfov(str(inst),'MOON','POINT','IAU_MOON',c,'EARTH',step,cnfine,result)
                else:s.gfrfov(str(inst),vector,fr,c,'EARTH',step,cnfine,result)
                count=s.wncard(result)
                if not count:out.writerow([kind,inst,c,fr,repr(step),*map(repr,vector),-1,0,0])
                for n in range(count):out.writerow([kind,inst,c,fr,repr(step),*map(repr,vector),n,*map(repr,s.wnfetd(result,n))])
s.kclear()
