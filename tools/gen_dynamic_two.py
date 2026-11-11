"""Native CSPICE constant-vector parameterized frame oracle."""
from pathlib import Path
import csv,itertools
import spiceypy as s
ROOT=Path(__file__).resolve().parents[1]
lines=['KPL/FK','\\begindata'];models=[]
for pri in ['X','-X','Y','-Y','Z','-Z']:
 for sec in ['X','-X','Y','-Y','Z','-Z']:
  if pri.strip('-')==sec.strip('-'):continue
  for spec in ['RECTANGULAR','LATITUDINAL','RA/DEC']:
   for mode in ['ROTATING','INERTIAL','FROZEN','DEFAULT']:
    n=len(models);fid=892000+n;name=f'DYN_TWO_{n}';models.append(name);base=['J2000','ECLIPJ2000','IAU_MARS','BASE_FIXED'][n%4]
    lines += [f"FRAME_{name}={fid}",f"FRAME_{fid}_NAME='{name}'",f"FRAME_{fid}_CLASS=5",f"FRAME_{fid}_CLASS_ID={fid}",f"FRAME_{fid}_CENTER=499",f"FRAME_{fid}_RELATIVE='{base}'",f"FRAME_{fid}_DEF_STYLE='PARAMETERIZED'",f"FRAME_{fid}_FAMILY='TWO-VECTOR'"]
    for i,prefix in enumerate(['PRI','SEC']):
     fr=['J2000','ECLIPJ2000','IAU_MARS','BASE_FIXED','DYN_EULER_13'][(n+i)%5]
     lines += [f"FRAME_{fid}_{prefix}_AXIS='{[pri,sec][i]}'",f"FRAME_{fid}_{prefix}_VECTOR_DEF='CONSTANT'",f"FRAME_{fid}_{prefix}_FRAME='{fr}'",f"FRAME_{fid}_{prefix}_SPEC='{spec}'"]
     if spec=='RECTANGULAR':
      mag=[1e-150,1.,1e150][(n+i)%3];v=[x*mag for x in ([1.1,-2.2,3.3] if i==0 else [-3.1,.7,1.9])];lines.append(f"FRAME_{fid}_{prefix}_VECTOR=("+' '.join(repr(x) for x in v)+')')
     else:
      lines.append(f"FRAME_{fid}_{prefix}_UNITS='DEGREES'")
      for attr,value in zip(['LONGITUDE','LATITUDE'] if spec=='LATITUDINAL' else ['RA','DEC'],[23.7+i*117.2,-19.8+i*33.4]):lines.append(f'FRAME_{fid}_{prefix}_{attr}={value!r}')
    if mode=='FROZEN':lines.append(f'FRAME_{fid}_FREEZE_EPOCH=345.678')
    elif mode!='DEFAULT':lines.append(f"FRAME_{fid}_ROTATION_STATE='{mode}'")
lines+=['\\begintext'];(ROOT/'tests/golden/dynamic_two.tf').write_text('\n'.join(lines)+'\n',encoding='utf-8')
s.furnsh(str(ROOT/'target/kernels/pck00011.tpc'));s.furnsh(str(ROOT/'tests/golden/dynamic_euler.tf'));s.furnsh(str(ROOT/'tests/golden/dynamic_two.tf'))
with (ROOT/'tests/golden/dynamic_two.csv').open('w',newline='',encoding='utf-8') as f:
 w=csv.writer(f);w.writerow(['kind','from','to','et','status',*[f'r{i}' for i in range(9)],*[f'd{i}' for i in range(9)]]);count=0
 for name in models:
  for et in [-12345.25,0.,12345.25]:
   for reference in ['J2000','ECLIPJ2000']:
    for a,b in [(name,reference),(reference,name)]:
     for kind in ['rotation','state']:
      status='OK';values=[]
      try:
       if kind=='rotation':values=s.pxform(a,b,et).flatten().tolist()
       else:x=s.sxform(a,b,et);values=[*x[:3,:3].flatten().tolist(),*x[3:,:3].flatten().tolist()]
      except s.utils.exceptions.SpiceyError as e:status=e.short
      w.writerow([kind,a,b,et,status,*values]);count+=1
print(f'{len(models)} constant two-vector frames, {count} native transforms')
