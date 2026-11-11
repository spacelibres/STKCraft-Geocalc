import csv,spiceypy as s
from pathlib import Path
for n in ['naif0012.tls','cassini/cas_v44.tf','cassini/cas_iss_v10.ti','cassini/cas00172.tsc','cassini/12289_12294ra.bc']:s.furnsh('target/kernels/'+n)
with Path('tests/golden/cassini_fov_frames.csv').open('w',newline='')as h:
 w=csv.writer(h);w.writerow(['et','ticks','rotation...'])
 for et in [403902000.,403902000.123,403902500.,403903000.,403903500.,403904000.,403904500.,403904651.19958746,403905000.,403905462.16845226,403905723.95332265,403906000.]:w.writerow([repr(et),repr(s.sce2c(-82,et)),*map(lambda x:repr(float(x)),s.pxform('CASSINI_ISS_NAC','J2000',et).flat)])
