"""Independent SPKEZP/PXFORM/STELAB ray specification oracle."""
import csv
from pathlib import Path
import spiceypy as spice
root=Path(__file__).resolve().parents[1]
spice.furnsh(str(root/'target/kernels/de440s.bsp'))
spice.furnsh(str(root/'target/kernels/pck00011.tpc'))
with (root/'tests/golden/ray_resolver_exact.csv').open('w',newline='') as file:
    writer=csv.writer(file)
    writer.writerow(['kind','source','axis','vx','vy','vz','stellar','anti','et','abcorr','x','y','z'])
    for et in [-5e8,0.,5e8]:
        for body in [10,301]:
            for correction in ['NONE','LT','LT+S','CN','CN+S','XLT','XLT+S','XCN','XCN+S']:
                vector,_=spice.spkezp(body,et,'J2000',correction,399)
                writer.writerow(['OBJECT',body,'-',0,0,0,0,0,et,correction,*[repr(float(v)) for v in vector]])
        for frame in ['J2000','ECLIPJ2000','IAU_EARTH','IAU_MOON']:
            rotation=spice.pxform(frame,'J2000',et)
            for axis,vector in [('X',[1.,0.,0.]),('Y',[0.,1.,0.]),('Z',[0.,0.,1.]),('-', [1.2,-2.3,4.5])]:
                for stellar in ([False] if axis!='-' else [False,True]):
                    resolved=spice.mxv(rotation,vector)
                    if stellar: resolved=spice.stelab(resolved,spice.spkssb(399,et,'J2000')[3:])
                    writer.writerow(['AXIS' if axis!='-' else 'VECTOR',frame,axis,*vector,int(stellar),0,et,'NONE',*[repr(float(v)) for v in resolved]])
spice.kclear()
