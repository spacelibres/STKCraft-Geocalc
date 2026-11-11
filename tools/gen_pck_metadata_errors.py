"""Native text-PCK malformed metadata errors and scalar spellings."""
from pathlib import Path
import csv
import spiceypy as s
ROOT=Path(__file__).resolve().parents[1];base={'POLE_RA':[30.,-.1,.001],'POLE_DEC':[70.,.2,-.003],'PM':[10.,100.,.0002]};cases=[]
def add(name,changes):
 d={f'BODY2000001_{k}':list(v) for k,v in base.items()}
 for k,v in changes.items():
  if v is None:d.pop(f'BODY2000001_{k}',None)
  else:d[f'BODY2000001_{k}']=v
 cases.append((name,d))
for key in base:
 add('missing_'+key,{key:None});add('overfull_'+key,{key:[1.,2.,3.,4.]});add('type_'+key,{key:'bad'})
add('scalars',{k:float(v[0]) for k,v in base.items()})
for kind in ['JED_EPOCH','REF_FRAME']:add('competing_'+kind,{'CONSTANTS_'+kind:[2451545.] if kind=='JED_EPOCH' else [1.],'CONSTS_'+kind:[2451545.] if kind=='JED_EPOCH' else [1.]})
for frame in [0.,22.]:add('bad_frame_'+str(frame),{'CONSTANTS_REF_FRAME':[frame]})
for key in ['RA','DEC','PM']:add('nut_type_'+key,{'NUT_PREC_ANGLES':[10.,20.],'NUT_PREC_'+key:'bad'})
add('phase_overfull',{'NUT_PREC_ANGLES':[float(i) for i in range(801)]})
add('nut_overfull',{'NUT_PREC_ANGLES':[10.,20.],'NUT_PREC_RA':[.1]*201})
add('degree_rounding',{'NUT_PREC_ANGLES':[10.,20.,30.],'MAX_PHASE_DEGREE':[1.8],'NUT_PREC_RA':[.1]})
add('phase_odd_size',{'NUT_PREC_ANGLES':[10.,20.,30.],'NUT_PREC_RA':[.1]})
with (ROOT/'tests/golden/pck_metadata_errors.csv').open('w',newline='',encoding='utf-8') as f:
 w=csv.writer(f);w.writerow(['case','status','kernel',*[f'r{i}' for i in range(9)],*[f'd{i}' for i in range(9)]])
 for name,pool in cases:
  s.kclear();lines=['\\begindata']
  for key,value in pool.items():
   if isinstance(value,str):s.pcpool(key,[value]);lines.append(key+"='"+value+"'")
   elif isinstance(value,float):s.pdpool(key,[value]);lines.append(key+'='+repr(value))
   else:
    s.pdpool(key,value);lines.append(key+'=( '+'\n '.join(' '.join(repr(x) for x in value[i:i+5]) for i in range(0,len(value),5))+' )')
  status='OK';values=[]
  try:x=s.tisbod('J2000',2000001,123456789.);values=[*x[:3,:3].flatten().tolist(),*x[3:,:3].flatten().tolist()]
  except s.utils.exceptions.SpiceyError as e:status=e.short
  w.writerow([name,status,'\\n'.join(lines).replace('\n','\\n'),*values])
print(f'{len(cases)} native PCK metadata spellings/errors')
