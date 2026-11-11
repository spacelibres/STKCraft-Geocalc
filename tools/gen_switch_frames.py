"""CSPICE class-6 switch priority, intervals and CK fallback oracle."""
from pathlib import Path
import csv
import spiceypy as s
ROOT=Path(__file__).resolve().parents[1];lines=['KPL/FK','\\begindata'];models=[]
configs=[(['J2000','ECLIPJ2000'],None),(['J2000','CK_T1_NOAV'],None),(['ECLIPJ2000','CK_T3_AV'],None),(['J2000','CK_T1_AV','BASE_FIXED','DYN_EULER_13'],[(-1000.,1000.),(1000.,1005.),(1005.,1010.),(1010.,2000.)]),(['IAU_MARS','CK_T2','ECLIPJ2000'],[(-1000.,2000.),(1000.,1010.),(1003.,1007.)]),(['J2000','CK_T3_AV','BASE_FIXED'],[(-1000.,0.),(1000.,1005.),(1010.,2000.)]),(['SWITCH_0','CK_T1_AV'],None),([1,17,10014,891013],[(-1000.,1000.),(999.,1005.),(1005.,1010.),(1010.,2000.)]),(['CK_T1_AV','CK_T3_AV'],None)]
for n,(bases,bounds) in enumerate(configs):
 fid=894000+n;name=f'SWITCH_{n}';models.append(name);lines += [f"FRAME_{name}={fid}",f"FRAME_{fid}_NAME='{name}'",f"FRAME_{fid}_CLASS=6",f"FRAME_{fid}_CLASS_ID={fid}",f"FRAME_{fid}_CENTER=499",f"FRAME_{fid}_ALIGNED_WITH=("+' '.join(repr(x) if isinstance(x,str) else str(x) for x in bases)+')']
 if bounds:
  lines += [f"FRAME_{fid}_START=("+' '.join(str(x[0]) for x in bounds)+')',f"FRAME_{fid}_STOP=("+' '.join(str(x[1]) for x in bounds)+')']
lines+=['\\begintext'];(ROOT/'tests/golden/switch_frames.tf').write_text('\n'.join(lines)+'\n',encoding='utf-8')
for file in ['target/kernels/naif0012.tls','target/kernels/pck00011.tpc','tests/golden/ck_test.tsc','tests/golden/ck_test.tf','tests/golden/ck_types_a.bc','tests/golden/ck_types_b.bc','tests/golden/dynamic_euler.tf','tests/golden/switch_frames.tf']:s.furnsh(str(ROOT/file))
times=[-1001.,-1000.,0.,999.9,1000.,1000.1,1001.,1003.,1005.,1007.,1010.,1011.,2000.,2001.]+[s.sct2e(-100,t) for t in [0.,100.25,200.5,500.25,900.5]]+[s.sct2e(-300,t) for t in [0.,100.25,200.5,500.25,900.5]]
with (ROOT/'tests/golden/switch_frames.csv').open('w',newline='',encoding='utf-8') as f:
 w=csv.writer(f);w.writerow(['kind','from','to','et','status',*[f'r{i}' for i in range(9)],*[f'd{i}' for i in range(9)]]);count=0
 for name in models:
  for et in times:
   for reference in ['J2000','ECLIPJ2000']:
    for a,b in [(name,reference),(reference,name)]:
     for kind in ['rotation','state']:
      status='OK';values=[]
      try:
       if kind=='rotation':values=s.pxform(a,b,et).flatten().tolist()
       else:x=s.sxform(a,b,et);values=[*x[:3,:3].flatten().tolist(),*x[3:,:3].flatten().tolist()]
      except s.utils.exceptions.SpiceyError as e:status=e.short
      w.writerow([kind,a,b,et,status,*values]);count+=1
print(f'{len(models)} switch frames, {count} native transforms')
