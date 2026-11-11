"""Native TANGPT with dynamic target-fixed axes and dynamic SPK sources."""
from pathlib import Path
import csv
import spiceypy as s
ROOT=Path(__file__).resolve().parents[1]
for file in ['target/kernels/de440s.bsp','target/kernels/pck00011.tpc','tests/golden/spk_dynamic_sources.bsp','tests/golden/dynamic_euler.tf']:
    s.furnsh(str(ROOT/file))
with (ROOT/'tests/golden/tangent_dynamic.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['target','observer','et','frame','corr','locus','dx','dy','dz',*[f'x{i}' for i in range(12)]])
    for observer in [399,91002]:
        for frame in ['DYN_EULER_0','DYN_EULER_12','DYN_EULER_13','DYN_EULER_14']:
            for et in [-12345.67,0.,23456.78]:
                for correction in ['NONE','LT','LT+S','CN','CN+S','XLT','XLT+S','XCN','XCN+S']:
                    center=s.spkpos('499',et,'J2000',correction,str(observer))[0]
                    offset=s.ucrss(center,[0.,0.,1.])
                    for distance in [0.,5000.]:
                        direction=center+offset*distance
                        for locus in ['TANGENT POINT','SURFACE POINT']:
                            p,alt,rng,sp,epoch,sv=s.tangpt('ELLIPSOID','499',et,frame,correction,locus,str(observer),'J2000',direction)
                            w.writerow([499,observer,repr(et),frame,correction,locus,*map(repr,map(float,direction)),*map(repr,map(float,[*p,alt,rng,*sp,epoch,*sv]))])
s.kclear()
