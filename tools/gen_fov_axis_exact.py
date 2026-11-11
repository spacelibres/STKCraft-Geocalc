"""ZZFOVAXI reference including the ZZHULLAX fallback."""
from pathlib import Path
import csv,ctypes as c,random,math
import spiceypy as s
from spiceypy.utils.libspicehelper import libspice
root=Path(__file__).resolve().parents[1];random.seed(693291)
fallback=0
with (root/'tests/golden/fov_axis_exact.csv').open('w',newline='') as file:
    out=csv.writer(file)
    for case in range(300):
        count=random.randint(3,25)
        rays=[]
        for index in range(count):
            vector=[random.uniform(.001,.05),random.uniform(.2,1.),random.uniform(-.1,.1)]
            if index==0:vector=[random.uniform(.001,.05),-1.,random.uniform(-.1,.1)]
            if case%3==0:vector=[random.uniform(.01,2.),random.uniform(-1.,1.),random.uniform(-1.,1.)]
            factor=10.**random.randint(-10,10);rays.append([x*factor for x in vector])
        data=(c.c_double*(count*3))(*[x for v in rays for x in v]);axis=(c.c_double*3)();n=c.c_int(count)
        libspice.zzfovaxi_(b'TEST',c.byref(n),data,axis,c.c_int(4))
        found=not s.failed()
        if not found:s.reset()
        average=s.vhat(sum((s.vhat(v) for v in rays),start=__import__('numpy').zeros(3))/count)
        fallback+=int(found and any(s.vsep(v,average)>math.pi/2-1e-12 for v in rays))
        out.writerow([case,count,int(found),*[repr(float(x)) for x in axis],*[repr(x) for v in rays for x in v]])
print('successful fallback cases',fallback)
