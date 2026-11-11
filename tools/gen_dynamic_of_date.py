"""CSPICE dynamic mean/true equator and mean ecliptic of date oracle."""
from pathlib import Path
import csv,itertools
import spiceypy as s
ROOT=Path(__file__).resolve().parents[1];lines=['KPL/FK','\\begindata'];models=[]
for family,mode,base in itertools.product(['MEAN_EQUATOR_AND_EQUINOX_OF_DATE','TRUE_EQUATOR_AND_EQUINOX_OF_DATE','MEAN_ECLIPTIC_AND_EQUINOX_OF_DATE'],['ROTATING','INERTIAL','FROZEN'],['J2000','ECLIPJ2000','IAU_MARS','BASE_FIXED']):
 n=len(models);fid=895000+n;name=f'OF_DATE_{n}';models.append(name);lines += [f"FRAME_{name}={fid}",f"FRAME_{fid}_NAME='{name}'",f"FRAME_{fid}_CLASS=5",f"FRAME_{fid}_CLASS_ID={fid}",f"FRAME_{fid}_CENTER=399",f"FRAME_{fid}_RELATIVE='{base}'",f"FRAME_{fid}_DEF_STYLE='PARAMETERIZED'",f"FRAME_{fid}_FAMILY='{family}'",f"FRAME_{fid}_PREC_MODEL='EARTH_IAU_1976'"]
 if family.startswith('TRUE_'):lines.append(f"FRAME_{fid}_NUT_MODEL='EARTH_IAU_1980'")
 if family.startswith('MEAN_ECLIPTIC'):lines.append(f"FRAME_{fid}_OBLIQ_MODEL='EARTH_IAU_1980'")
 if mode=='FROZEN':lines.append(f'FRAME_{fid}_FREEZE_EPOCH=123456789.25')
 else:lines.append(f"FRAME_{fid}_ROTATION_STATE='{mode}'")
lines+=['\\begintext'];(ROOT/'tests/golden/dynamic_of_date.tf').write_text('\n'.join(lines)+'\n',encoding='utf-8')
for file in ['target/kernels/pck00011.tpc','tests/golden/dynamic_euler.tf','tests/golden/dynamic_of_date.tf']:s.furnsh(str(ROOT/file))
with (ROOT/'tests/golden/dynamic_of_date.csv').open('w',newline='',encoding='utf-8') as f:
 w=csv.writer(f);w.writerow(['kind','from','to','et',*[f'r{i}' for i in range(9)],*[f'd{i}' for i in range(9)]]);count=0
 for name in models:
  for et in [-1e10,-86400.25,0.,1e-6,12345.25,1e8,1e9,3.15576e9,1e10]:
   for reference in ['J2000','ECLIPJ2000']:
    for a,b in [(name,reference),(reference,name)]:
     for kind in ['rotation','state']:
      if kind=='rotation':values=s.pxform(a,b,et).flatten().tolist()
      else:x=s.sxform(a,b,et);values=[*x[:3,:3].flatten().tolist(),*x[3:,:3].flatten().tolist()]
      w.writerow([kind,a,b,et,*values]);count+=1
print(f'{len(models)} of-date frames, {count} native transforms')
