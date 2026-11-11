"""Native N0067 SPK17/EQNCPV equinoctial orbit oracle."""
from pathlib import Path
import csv,random,math
import spiceypy as s
root=Path(__file__).resolve().parents[1];out=root/'tests/golden';path=out/'spk17.bsp'
if path.exists():path.unlink()
r=random.Random(17067);h=s.spkopn(str(path),'SPK17 source oracle',0)
for index in range(24):
    e=[0.,1e-14,.1,.5,.89,.899999999][index%6];angle=r.uniform(-math.pi,math.pi)
    eq=[r.uniform(1e3,1e7),e*math.sin(angle),e*math.cos(angle),r.uniform(-8,8),r.uniform(-4,4),r.uniform(-4,4),r.uniform(-1e-9,1e-9),r.uniform(.00001,.001),r.uniform(-1e-9,1e-9)]
    s.spkw17(h,717000+index,0,s.frmnam(index%21+1),-1e9,1e9,str(index),12345.6789,eq,r.uniform(-math.pi,math.pi),r.uniform(-math.pi/2,math.pi/2))
s.spkcls(h);s.furnsh(str(path))
with (out/'spk17.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['body','et','frame','x','y','z','vx','vy','vz'])
    for i in range(24):
        for et in [-1e9,-12345.67,0.,12345.6789,12345.67890001,1e9]:
            for frame in range(1,22):
                state,_=s.spkgeo(717000+i,et,s.frmnam(frame),0);w.writerow([717000+i,repr(et),frame,*[repr(float(x)) for x in state]])
s.kclear()
