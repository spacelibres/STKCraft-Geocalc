"""GFSEP event epochs/values and native ZZGFSPDC crossing directions."""
from pathlib import Path
import csv,ctypes as ct,spiceypy as s
from spiceypy.utils.libspicehelper import libspice
root=Path(__file__).resolve().parents[1]
s.furnsh(str(root/'target/kernels/de440s.bsp'));s.furnsh(str(root/'target/kernels/pck00011.tpc'))
D=ct.c_double;I=ct.c_int
libspice.zzgfspdc_.argtypes=[ct.c_void_p,ct.POINTER(D),ct.POINTER(I)]
def cf():
 c=s.cell_double(1000);s.wninsd(0.,2592000.,c);return c
rows=[]
for obs,t1,t2 in [(399,10,301),(301,399,10)]:
 for corr in ['NONE','LT','LT+S','CN','CN+S','XLT','XLT+S','XCN','XCN+S']:
  for a,b in [('POINT','POINT'),('POINT','SPHERE'),('SPHERE','POINT'),('SPHERE','SPHERE')]:
   for threshold in [None,s.halfpi()]:
    events=[]
    for op,kind in [('LOCMIN','MIN'),('LOCMAX','MAX')]+([('=','CROSS')] if threshold is not None else []):
     w=s.gfsep(str(t1),a,'IAU_'+str(t1),str(t2),b,'IAU_'+str(t2),corr,str(obs),op,threshold or 0.,0.,3600.,1000,cf())
     for j in range(s.wncard(w)):
      et,end=s.wnfetd(w,j)
      assert et==end
      epoch=D(et);value=D();libspice.zzgfspgq_(ct.byref(epoch),ct.byref(value))
      if kind=='CROSS':
       dec=I();libspice.zzgfspdc_(None,ct.byref(epoch),ct.byref(dec));k='DOWN' if dec.value else 'UP'
      else:k=kind
      if s.failed():raise RuntimeError(s.getmsg('LONG'))
      events.append((et,k,value.value))
    events.sort(key=lambda e:e[0])
    rows.append([obs,t1,t2,corr,a,b,'' if threshold is None else threshold,';'.join(repr(et)+':'+kind+':'+repr(v) for et,kind,v in events)])
with open(root/'tests/golden/separation_native_events_precision.csv','w',newline='') as f:
 w=csv.writer(f);w.writerow(['observer','target1','target2','correction','shape1','shape2','threshold','events']);w.writerows(rows)
print(len(rows),'native separation event cases')
