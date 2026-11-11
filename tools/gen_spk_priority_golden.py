"""Generate exact CSPICE segment-priority and polynomial regression data."""
from pathlib import Path
import csv
import spiceypy as spice

root = Path(__file__).resolve().parents[1]
output = root / 'tests/golden/spk_priority.bsp'
if output.exists():
    output.unlink()
handle = spice.spkopn(str(output), 'STKCraft priority reference', 0)

def segment(body, center, value, start=0., end=30.):
    spice.spkw02(handle, body, center, 'J2000', start, end,
                 f'{body}:{center}:{value}', end-start, 1, 1,
                 [value, 0., 0., 0., 0., 0.], start)

segment(10001, 0, 1.)
segment(10001, 0, 2.)
segment(10003, 0, 100.)
segment(10004, 0, 200.)
segment(10002, 10003, 10.)
segment(10002, 10004, 20.)
segment(10005, 0, 100.)
segment(10005, 0, 200., 10., 20.)
# Independent nontrivial type 2/type 3 Chebyshev coefficients.
coefficients = [((i*17+3) % 23-11) * 10.**(-i/3) for i in range(13)]
spice.spkw02(handle, 10006, 0, 'J2000', -16., 16., 'CHBINT', 32., 1, 12,
             coefficients + [c*3.7 for c in coefficients] + [c*-1.9 for c in coefficients], -16.)
spice.spkw03(handle, 10007, 0, 'J2000', -16., 16., 'CHBVAL', 32., 1, 12,
             [c*factor for factor in [1.,3.7,-1.9,.007,.003,-.001] for c in coefficients], -16.)
# A common-center case with cancellation-sensitive accumulation.
segment(10010, 0, 1e16)
segment(10011, 10010, 3.)
segment(10012, 10011, -1e16)
segment(10013, 10010, 5.)
spice.spkcls(handle)
spice.furnsh(str(output))
queries = [(10001,0,5.),(10002,0,5.),(0,10002,5.),(10002,10003,5.)]
queries += [(10005,0,t) for t in [0.,9.999,10.,15.,20.,20.001,30.]]
queries += [(body,0,t) for body in [10006,10007] for t in [-16.,-15.99,-8.,-1.,0.,.1,3.14,8.,15.99,16.]]
queries += [(10012,10013,5.),(10013,10012,5.),(10012,0,5.)]
with (output.parent/'spk_priority.csv').open('w',newline='',encoding='utf-8') as file:
    writer=csv.writer(file)
    writer.writerow(['target','observer','et','x','y','z','vx','vy','vz'])
    for target,observer,et in queries:
        state,_=spice.spkgeo(target,et,'J2000',observer)
        writer.writerow([target,observer,repr(et),*[repr(float(x)) for x in state]])
spice.kclear()
print(f'Wrote {output} and {len(queries)} CSPICE reference states')

layers=[]
for name, value, narrow in [('a',1.,False),('b',2.,True)]:
    path=output.parent/f'spk_layer_{name}.bsp'
    if path.exists(): path.unlink()
    handle=spice.spkopn(str(path),'ordered kernel layer',0)
    segment(10001,0,value)
    segment(10002,0 if narrow else 10001,20. if narrow else 10.,10. if narrow else 0.,20. if narrow else 30.)
    spice.spkcls(handle)
    layers.append(path)
with (output.parent/'spk_layers.csv').open('w',newline='') as file:
    writer=csv.writer(file);writer.writerow(['order','target','observer','et','x','y','z','vx','vy','vz'])
    for order, paths in [('ab',layers),('ba',layers[::-1])]:
        spice.kclear()
        for path in paths: spice.furnsh(str(path))
        for target,observer in [(10001,0),(10002,0),(10002,10001),(0,10002)]:
            for et in [0.,9.999,10.,15.,20.,20.001,30.]:
                state,_=spice.spkgeo(target,et,'J2000',observer)
                writer.writerow([order,target,observer,repr(et),*[repr(float(v)) for v in state]])
spice.kclear()
