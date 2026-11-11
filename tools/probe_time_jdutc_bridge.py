"""Read-only handoff evidence for the public STKCraft-time SGP4 epoch bridge."""
from pathlib import Path
import csv,ctypes as c,random,math
import spiceypy as s
from spiceypy.utils.libspicehelper import libspice
root=Path(__file__).resolve().parents[1];s.furnsh(str(root/'target/kernels/naif0012.tls'));r=random.Random(1067)
times=[-315576000.,0.,123456789.,536500800.,846763200.,1577880000.]
for _ in range(500):
    et=r.uniform(-8e8,1.6e9)
    jd=s.unitim(et,'ET','JED');half=math.ulp(jd)*86400./2.
    for delta in [0.,half,-half]:
        t=et+delta;times += [t,math.nextafter(t,-math.inf),math.nextafter(t,math.inf)]
out=root/'target/time-jdutc-bridge.csv'
with out.open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['et','native_jdutc'])
    for et in times:
        tv=(c.c_double*8)(et,*[0.]*7);libspice.ttrans_(b'TDB',b'JDUTC',tv,c.c_int(3),c.c_int(5));assert not s.failed()
        w.writerow([repr(et),repr(tv[0])])
s.kclear();print(out)
