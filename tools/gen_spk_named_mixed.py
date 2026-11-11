"""N0067 SPKEZR/SPKPOS oracle: mixed segment frames, rotating output/source."""
from pathlib import Path
import csv
import spiceypy as s
root=Path(__file__).resolve().parents[1]
out=root/'tests/golden'
fk=out/'spk_named_mixed.tf'
fk.write_text("""\\begindata
FRAME_MIXED_OUTPUT=1590001
FRAME_1590001_NAME='MIXED_OUTPUT'
FRAME_1590001_CLASS=4
FRAME_1590001_CLASS_ID=1590001
FRAME_1590001_CENTER=399
TKFRAME_1590001_RELATIVE='B1950'
TKFRAME_1590001_SPEC='ANGLES'
TKFRAME_1590001_ANGLES=(13 27 -8)
TKFRAME_1590001_AXES=(3 1 2)
TKFRAME_1590001_UNITS='DEGREES'
""")
for path in [root/'target/kernels/de440s.bsp',out/'spk_mixed_frames.bsp',root/'target/kernels/pck00011.tpc',fk]:s.furnsh(str(path))
with (out/'spk_named_mixed.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['target','observer','et','frame','correction','x','y','z','vx','vy','vz','lt','px','py','pz','plt'])
    for t,o in [(21002,22018),(21017,22013),(30000,31001),(90001,399)]:
        for et in [-12345.67,0.,23456.78]:
            for frame in ['MIXED_OUTPUT','IAU_MOON','IAU_EARTH',*[s.frmnam(i) for i in range(1,22)]]:
                for corr in ['NONE','LT','LT+S','CN','CN+S','XLT','XLT+S','XCN','XCN+S']:
                    state,lt=s.spkezr(str(t),et,frame,corr,str(o));p,plt=s.spkpos(str(t),et,frame,corr,str(o))
                    w.writerow([t,o,repr(et),frame,corr,*map(lambda x:repr(float(x)),state),repr(lt),*map(lambda x:repr(float(x)),p),repr(plt)])
s.kclear()
