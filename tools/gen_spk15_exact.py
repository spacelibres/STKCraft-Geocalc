"""Native N0067 SPK15 conics/J2 nodal/apsidal precession oracle."""
from pathlib import Path
import csv,random
import spiceypy as s
root=Path(__file__).resolve().parents[1];out=root/'tests/golden';path=out/'spk15.bsp'
if path.exists():path.unlink()
r=random.Random(15067);h=s.spkopn(str(path),'SPK15 source oracle',0);index=0
for eccentricity in [0.,.2,.9,1.,1.5]:
    for flag in [0.,1.,2.,3.]:
        for below in [False,True]:
            pole=s.vhat([r.random(),r.random(),r.random()]);peri=s.vhat(s.vperp([r.random(),r.random(),r.random()],pole))
            p=7000.*(eccentricity+1.) if not below else 3000.*(eccentricity+1.)
            s.spkw15(h,715000+index,0,s.frmnam(index%21+1),-1e8,1e8,str(index),123.456,pole,peri,p,eccentricity,flag,[.01,.03,1.],398600.435436,.0010826265,6378.1366)
            index+=1
s.spkcls(h);s.furnsh(str(path))
with (out/'spk15.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['body','et','frame','x','y','z','vx','vy','vz'])
    for body in range(715000,715000+index):
        for et in [-1e8,-123.456,0.,123.456,123.456000001,5000.,1e8]:
            for frame in range(1,22):
                state,_=s.spkgeo(body,et,s.frmnam(frame),0);w.writerow([body,repr(et),frame,*[repr(float(x)) for x in state]])
s.kclear()
