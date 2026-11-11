"""Native TISBOD phase cardinality, degree and system-key semantics."""
from pathlib import Path
import csv,itertools
import spiceypy as s
ROOT=Path(__file__).resolve().parents[1]
models=[]
for body,ref in [(602,6),(10020,1),(2000010,2000010)]:
 for degree in [0,1,2,3,4]:
  for parent_count in [0,1,2,3]:
   for coeff_count in [0,1,2,4]:
    models.append((body,ref,degree,parent_count,coeff_count))
with (ROOT/'tests/golden/pck_phase_metadata.csv').open('w',newline='',encoding='utf-8') as f:
 w=csv.writer(f);w.writerow(['body','ref','degree','phase_count','coeff_count','status',*[f'r{i}' for i in range(9)],*[f'd{i}' for i in range(9)]])
 for body,ref,degree,pc,nc in models:
  s.kclear();pool={f'BODY{body}_POLE_RA':[30.,-.1,.001],f'BODY{body}_POLE_DEC':[70.,.2,-.003],f'BODY{body}_PM':[10.,100.,.0002]}
  if pc:pool[f'BODY{ref}_NUT_PREC_ANGLES']=[17.+i*3.1 for i in range(pc*(degree+1))]
  if ref!=body:pool[f'BODY{body}_NUT_PREC_ANGLES']=[80.,2.,160.,3.,240.,4.,320.,5.]
  pool[f'BODY{ref}_MAX_PHASE_DEGREE']=[float(degree)]
  for i,key in enumerate(['RA','DEC','PM']):
   if nc:pool[f'BODY{body}_NUT_PREC_{key}']=[(-1)**(j+i)*(i+1)*.01*(j+1) for j in range(nc)]
  for key,value in pool.items():s.pdpool(key,value)
  status='OK';values=[]
  try:x=s.tisbod('J2000',body,123456789.);values=[*x[:3,:3].flatten().tolist(),*x[3:,:3].flatten().tolist()]
  except s.utils.exceptions.SpiceyError as e:status=e.short
  w.writerow([body,ref,degree,pc,nc,status,*values])
print(f'{len(models)} native PCK metadata models')
