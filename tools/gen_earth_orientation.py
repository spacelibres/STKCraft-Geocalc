"""Independent native EUL2XF/ZZWAHR/IAU1976/1980/TEME state oracles."""
from pathlib import Path
import csv,itertools,random,ctypes as c
import spiceypy as s
from spiceypy.utils.libspicehelper import libspice
root=Path(__file__).resolve().parents[1];out=root/'tests/golden';r=random.Random(19761980)
with (out/'eul2xf_exact.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['a','b','c','alpha','beta','gamma','da','db','dc',*[f'x{i}' for i in range(36)]])
    for axes in itertools.product([1,2,3],repeat=3):
        for _ in range(20):
            e=[r.uniform(-10,10) for _ in range(3)]+[r.uniform(-.01,.01) for _ in range(3)]
            xf=s.eul2xf(e,*axes);w.writerow([*axes,*map(repr,e),*[repr(float(x)) for x in xf.flat]])
times=[-1e12,-1e10,-2e9,-1.,0.,1.,2e9,1e10,1e12]+[r.uniform(-1e10,1e10) for _ in range(100)]
with (out/'earth_orientation.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['kind','et','values'])
    for et in times:
        t=c.c_double(et)
        for name in ['zzwahr_','zzeprc76_','zzenut80_','zzteme_']:
            output=(c.c_double*(4 if name=='zzwahr_' else 36))();inverse=(c.c_double*36)()
            fn=getattr(libspice,name)
            if name=='zzteme_':fn(c.byref(t),output,inverse)
            else:fn(c.byref(t),output)
            if name=='zzwahr_':values=list(output)
            else:values=[output[col*6+row] for row in range(6) for col in range(6)]
            w.writerow([name,repr(et),*map(repr,values)])
            if name=='zzteme_':w.writerow(['teme_inverse',repr(et),*[repr(inverse[col*6+row]) for row in range(6) for col in range(6)]])
