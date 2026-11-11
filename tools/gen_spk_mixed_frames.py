"""Independent N0067 SPKGEO/SPKEZR/SPKPOS mixed-frame and 20-node oracle."""
from pathlib import Path
import csv, random
import spiceypy as spice
OUT=Path(__file__).resolve().parents[1]/'tests/golden'
rng=random.Random(20261111)
path=OUT/'spk_mixed_frames.bsp'
if path.exists():path.unlink()
h=spice.spkopn(str(path),'SPKGEO frame chain source oracle',0)
def segment(body,center,frame,scale=1.):
    # Type3 independent position and velocity polynomials, all evaluations native.
    coeff=[]
    for i in range(6):
        m=1e8*scale if i<3 else .1*scale
        coeff += [rng.uniform(-m,m),rng.uniform(-m,m)*.03,rng.uniform(-m,m)*.002]
    spice.spkw03(h,body,center,spice.frmnam(frame),-1e7,1e7,str(body),2e7,1,2,coeff,-1e7)
for frame in range(1,22):
    segment(20000+frame,0,frame)
    segment(21000+frame,20000+frame,22-frame,.01)
    segment(22000+frame,20000+frame,frame,.02)
# Long chain overwrites target array slot20 and may lose first common center.
for n in range(35):
    segment(30000+n,30001+n if n<34 else 0,(n*13)%21+1,1e-2 if n%3 else 1e3)
segment(31001,30025,19,.01)
segment(31002,30018,17,.03)
segment(90001,0,10020) # no implicit inertial substitution for body-fixed segment.
spice.spkcls(h);spice.furnsh(str(path))
queries=[]
for n in range(1,22):queries += [(21000+n,22000+n),(21000+n,20000+n),(21000+n,0),(0,21000+n),(21000+n,21000+(n%21)+1)]
queries += [(30000,0),(30000,31001),(31001,30000),(30000,31002),(30000,30025),(30000,30018),(30014,30030),(31001,31002)]
with (OUT/'spk_mixed_geometric.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['target','observer','et','frame','x','y','z','vx','vy','vz'])
    for t,o in queries:
        for frame in range(1,22):
            for et in [-12345.67,0.,23456.78]:
                state,_=spice.spkgeo(t,et,spice.frmnam(frame),o)
                w.writerow([t,o,repr(et),frame,*map(lambda x:repr(float(x)),state)])
with (OUT/'spk_mixed_corrected.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['target','observer','et','frame','correction','x','y','z','vx','vy','vz','lt','px','py','pz','plt'])
    for t,o in [(21002,22018),(21017,22013),(30000,31001),(31001,30000)]:
        for frame in range(1,22):
            for correction in ['NONE','LT','LT+S','CN','CN+S','XLT','XLT+S','XCN','XCN+S']:
                for et in [-12345.67,23456.78]:
                    state,lt=spice.spkezr(str(t),et,spice.frmnam(frame),correction,str(o));position,plt=spice.spkpos(str(t),et,spice.frmnam(frame),correction,str(o))
                    w.writerow([t,o,repr(et),frame,correction,*map(lambda x:repr(float(x)),state),repr(lt),*map(lambda x:repr(float(x)),position),repr(plt)])
fk=OUT/'spk_static_output_frames.tf'
fk.write_text("\\begindata\n"+"\n".join(f"FRAME_SOURCE_STATIC_{frame}={1500000+frame}\nFRAME_{1500000+frame}_NAME='SOURCE_STATIC_{frame}'\nFRAME_{1500000+frame}_CLASS=4\nFRAME_{1500000+frame}_CLASS_ID={1500000+frame}\nFRAME_{1500000+frame}_CENTER=399\nTKFRAME_{1500000+frame}_RELATIVE='{spice.frmnam(frame)}'\nTKFRAME_{1500000+frame}_SPEC='ANGLES'\nTKFRAME_{1500000+frame}_ANGLES=(0 0 0)\nTKFRAME_{1500000+frame}_AXES=(1 2 3)" for frame in [2,13,17])+"\n")
spice.furnsh(str(fk));spice.furnsh(str(OUT.parents[1]/'target/kernels/de440s.bsp'))
with (OUT/'spk_static_output_frames.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['target','observer','et','inertial_matrix','correction','x','y','z','vx','vy','vz','lt'])
    for t,o in [(301,399),(299,399),(199,301)]:
        for frame in [2,13,17]:
            for correction in ['NONE','LT','LT+S','CN','CN+S','XLT','XLT+S','XCN','XCN+S']:
                for et in [-2e9,0.,2e9]:
                    state,lt=spice.spkezr(str(t),et,f'SOURCE_STATIC_{frame}',correction,str(o))
                    w.writerow([t,o,repr(et),frame,correction,*map(lambda x:repr(float(x)),state),repr(lt)])
spice.kclear()
print(f'Generated {len(queries)*21*3}geometric and{4*21*9*2}corrected mixedframe source cases')
