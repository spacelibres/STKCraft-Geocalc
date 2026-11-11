"""Native ZZINVELT with excluded coordinates and zero/greedy margins."""
from pathlib import Path
import csv,ctypes as c
import spiceypy as s
from spiceypy.utils.libspicehelper import libspice
root=Path(__file__).resolve().parents[1];rows=[];arr=lambda v:(c.c_double*len(v))(*v)
with (root/'tests/golden/dsk/contains.csv').open() as file:
 for index,row in enumerate(list(csv.reader(file))[1:]):
  if index%8:continue
  values=list(map(float,row));system=c.c_int(int(values[0]));p=values[17:20];pars=values[1:11];bounds=values[11:17]
  for margin in [0.,1e-8,2e-8]:
   for excluded in range(4):
    found=c.c_int();m=c.c_double(margin);ex=c.c_int(excluded)
    libspice.zzinvelt_(arr(p),c.byref(system),arr(pars),arr(bounds),c.byref(m),c.byref(ex),c.byref(found))
    if s.failed():raise Exception(s.getmsg('LONG'))
    rows.append([*values[:20],margin,excluded,found.value])
with (root/'tests/golden/dsk/contains_excluding.csv').open('w',newline='') as file:
 w=csv.writer(file);w.writerow(['corsys',*[f'param{i}' for i in range(10)],*[f'bound{i}' for i in range(6)],'x','y','z','margin','excluded','inside']);w.writerows(rows)
print(len(rows),'native excluded-coordinate volume predicates')
