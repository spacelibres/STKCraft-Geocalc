"""Native CSPICE parameterized Euler frame oracle, including frozen/inertial."""
from pathlib import Path
import csv,itertools
import spiceypy as s
ROOT=Path(__file__).resolve().parents[1]
lines=['KPL/FK','\\begindata',"FRAME_BASE_FIXED=890000","FRAME_890000_NAME='BASE_FIXED'","FRAME_890000_CLASS=4","FRAME_890000_CLASS_ID=890000","FRAME_890000_CENTER=499","TKFRAME_890000_RELATIVE='IAU_MARS'","TKFRAME_890000_SPEC='ANGLES'","TKFRAME_890000_ANGLES=(11.3 -7.2 19.1)","TKFRAME_890000_AXES=(3 2 1)","TKFRAME_890000_UNITS='DEGREES'"]
models=[];units=['RADIANS','DEGREES','ARCSECONDS','ARCMINUTES','HOURANGLE','MINUTEANGLE','SECONDANGLE']
for axes in itertools.product([1,2,3],repeat=3):
 for degree in [0,3,19]:
  for mode in ['ROTATING','INERTIAL','FROZEN','DEFAULT']:
   n=len(models);fid=891000+n;name=f'DYN_EULER_{n}';base=['J2000','ECLIPJ2000','IAU_MARS','BASE_FIXED'][n%4];models.append(name)
   lines += [f"FRAME_{name}={fid}",f"FRAME_{fid}_NAME='{name}'",f"FRAME_{fid}_CLASS=5",f"FRAME_{fid}_CLASS_ID={fid}",f"FRAME_{fid}_CENTER=499",f"FRAME_{fid}_RELATIVE='{base}'",f"FRAME_{fid}_DEF_STYLE='PARAMETERIZED'",f"FRAME_{fid}_FAMILY='EULER'",f"FRAME_{fid}_EPOCH=123.456",f"FRAME_{fid}_AXES=({axes[0]} {axes[1]} {axes[2]})",f"FRAME_{fid}_UNITS='{units[n%len(units)]}'"]
   for axis in range(3):
    cp=[(-1)**(i+axis)*(axis+1)*.31*(1e-4**i)/(i+1) for i in range(degree+1)]
    lines.append(f"FRAME_{fid}_ANGLE_{axis+1}_COEFFS=("+'\n '.join(' '.join(repr(x) for x in cp[i:i+4]) for i in range(0,len(cp),4))+')')
   if mode=='FROZEN':lines.append(f'FRAME_{fid}_FREEZE_EPOCH=345.678')
   elif mode!='DEFAULT':lines.append(f"FRAME_{fid}_ROTATION_STATE='{mode}'")
lines+=['\\begintext']
(ROOT/'tests/golden/dynamic_euler.tf').write_text('\n'.join(lines)+'\n',encoding='utf-8')
s.furnsh(str(ROOT/'target/kernels/pck00011.tpc'));s.furnsh(str(ROOT/'tests/golden/dynamic_euler.tf'))
with (ROOT/'tests/golden/dynamic_euler.csv').open('w',newline='',encoding='utf-8') as f:
 w=csv.writer(f);w.writerow(['kind','from','to','et',*[f'r{i}' for i in range(9)],*[f'd{i}' for i in range(9)]]);count=0
 for name in models:
  for et in [-12345.25,0.,123.456,345.678,12345.25]:
   for reference in ['J2000','ECLIPJ2000']:
    for a,b in [(name,reference),(reference,name)]:
     for kind in ['rotation','state']:
      if kind=='rotation':r=s.pxform(a,b,et);d=[]
      else:x=s.sxform(a,b,et);r=x[:3,:3];d=x[3:,:3].flatten().tolist()
      w.writerow([kind,a,b,et,*r.flatten().tolist(),*d]);count+=1
print(f'{len(models)} Euler frames, {count} native transforms')
