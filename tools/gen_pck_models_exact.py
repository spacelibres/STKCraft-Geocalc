"""TISBOD reference for valid zero-valued PCK models."""
from pathlib import Path
import csv
import spiceypy as s
root=Path(__file__).resolve().parents[1]
models={999001:([0.,0.,0.],[0.,0.,0.],[0.,0.,0.]),
        999002:([0.,0.,0.],[0.,0.,0.],[1.,2.,3.]),
        999003:([90.,0.,0.],[0.,0.,0.],[0.,0.,0.])}
lines=['KPL/PCK','\\begindata']
for body,values in models.items():
    for key,value in zip(['POLE_RA','POLE_DEC','PM'],values):
        lines.append(f'BODY{body}_{key} = ( '+', '.join(map(repr,value))+' )')
path=root/'tests/golden/pck_models_exact.tpc';path.write_text('\n'.join(lines)+'\n\\begintext\n')
s.furnsh(str(path))
with (root/'tests/golden/pck_models_exact.csv').open('w',newline='') as file:
    out=csv.writer(file);out.writerow(['body','et','rotation_and_derivative'])
    for body in models:
        for et in [-5e8,0.,5e8]:
            x=s.tisbod('J2000',body,et)
            out.writerow([body,repr(et),*[repr(float(v)) for v in x[:3,:3].flat],*[repr(float(v)) for v in x[3:,:3].flat]])
s.kclear()
