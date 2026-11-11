"""Native FOVTOOWIDE boundaries, including visibility initialization on empty windows."""
from pathlib import Path
import csv,math
import spiceypy as s
root=Path(__file__).resolve().parents[1]
limit=math.pi/2-1e-6
angles=[0.,1.,math.nextafter(limit,-math.inf),limit,math.nextafter(limit,math.inf),math.pi/2,math.pi/2+1e-6]
text=['KPL/IK','\\begindata']
for i,angle in enumerate(angles):
    inst=949000+i
    text += [f"INS{inst}_FOV_FRAME = 'J2000'",f"INS{inst}_FOV_SHAPE = 'CIRCLE'",f'INS{inst}_BORESIGHT = ( 0,0,1 )',
             f'INS{inst}_FOV_BOUNDARY = ( {math.sin(angle)!r},0,{math.cos(angle)!r} )']
path=root/'tests/golden/fov_width_limits.ti';path.write_text('\n'.join(text)+'\n\\begintext\n')
s.furnsh(str(path))
with (root/'tests/golden/fov_width_limits.csv').open('w',newline='') as file:
    out=csv.writer(file);out.writerow(['instrument','point_success','empty_search_success'])
    for i in range(len(angles)):
        inst=949000+i
        try:s.fovray(str(inst),[0.,0.,1.],'J2000','NONE','EARTH',0.);valid=1
        except s.utils.exceptions.SpiceyError:valid=0
        try:s.gfrfov(str(inst),[0.,0.,1.],'J2000','NONE','EARTH',1.,s.cell_double(2),s.cell_double(20));empty=1
        except s.utils.exceptions.SpiceyError:empty=0
        out.writerow([inst,valid,empty])
s.kclear()
