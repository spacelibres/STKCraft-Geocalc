"""Generate source GETFOV values for every committed instrument."""
from pathlib import Path
import csv,re
import spiceypy as spice
root=Path(__file__).resolve().parents[1]
with (root/'tests/golden/fov_boundaries_exact.csv').open('w',newline='') as file:
    writer=csv.writer(file);writer.writerow(['kernel','instrument','boundary','bx','by','bz','x','y','z'])
    for name in ['fov.tf','fov_ellipsoid.tf']:
        spice.kclear()
        path=root/'tests/golden'/name; spice.furnsh(str(path))
        instruments=sorted(set(map(int,re.findall(r'INS(-?\d+)_FOV_SHAPE',path.read_text()))))
        for instrument in instruments:
            shape,frame,boresight,count,bounds=spice.getfov(instrument,100)
            for index,point in enumerate(bounds):
                writer.writerow([name,instrument,index,*[repr(float(x)) for x in boresight],*[repr(float(x)) for x in point]])
spice.kclear()
