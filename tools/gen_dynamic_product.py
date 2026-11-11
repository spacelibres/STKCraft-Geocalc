"""Native CSPICE PRODUCT dynamic frames, inversion order and recursion errors."""
from pathlib import Path
import csv
import spiceypy as s
ROOT=Path(__file__).resolve().parents[1];lines=['KPL/FK','\\begindata'];models=[]
factors=[('J2000','ECLIPJ2000'),('B1950','IAU_MARS'),('BASE_FIXED','J2000'),('DYN_EULER_13','ECLIPJ2000'),('J2000','DYN_TWO_0')]
for nf in [1,2,5,10]:
 for mode in ['ROTATING','INERTIAL','FROZEN','DEFAULT']:
  for variant in range(3):
   n=len(models);fid=893000+n;name=f'DYN_PRODUCT_{n}';models.append(name);base=['J2000','ECLIPJ2000','IAU_MARS'][variant]
   pairs=[factors[(i+variant)%len(factors)] for i in range(nf)]
   lines += [f"FRAME_{name}={fid}",f"FRAME_{fid}_NAME='{name}'",f"FRAME_{fid}_CLASS=5",f"FRAME_{fid}_CLASS_ID={fid}",f"FRAME_{fid}_CENTER=499",f"FRAME_{fid}_RELATIVE='{base}'",f"FRAME_{fid}_DEF_STYLE='PARAMETERIZED'",f"FRAME_{fid}_FAMILY='PRODUCT'",f"FRAME_{fid}_FROM_FRAMES=("+' '.join("'"+a+"'" for a,b in pairs)+')',f"FRAME_{fid}_TO_FRAMES=("+' '.join("'"+b+"'" for a,b in pairs)+')']
   if mode=='FROZEN':lines.append(f'FRAME_{fid}_FREEZE_EPOCH=345.678')
   elif mode!='DEFAULT':lines.append(f"FRAME_{fid}_ROTATION_STATE='{mode}'")
fid=893100;name='DYN_PRODUCT_DEEP';models.append(name)
lines += [f"FRAME_{name}={fid}",f"FRAME_{fid}_NAME='{name}'",f"FRAME_{fid}_CLASS=5",f"FRAME_{fid}_CLASS_ID={fid}",f"FRAME_{fid}_CENTER=499",f"FRAME_{fid}_RELATIVE='J2000'",f"FRAME_{fid}_DEF_STYLE='PARAMETERIZED'",f"FRAME_{fid}_FAMILY='PRODUCT'",f"FRAME_{fid}_FROM_FRAMES=('J2000')",f"FRAME_{fid}_TO_FRAMES=('DYN_TWO_3')"]
lines+=['\\begintext'];# Keep physical lines below native kernel132char limit.
text='\n'.join(lines).replace("' '","'\n '");(ROOT/'tests/golden/dynamic_product.tf').write_text(text+'\n',encoding='utf-8')
s.furnsh(str(ROOT/'target/kernels/pck00011.tpc'))
for file in ['dynamic_euler.tf','dynamic_two.tf','dynamic_product.tf']:s.furnsh(str(ROOT/'tests/golden'/file))
with (ROOT/'tests/golden/dynamic_product.csv').open('w',newline='',encoding='utf-8') as f:
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
print(f'{len(models)} product frames, {count} native transforms')
