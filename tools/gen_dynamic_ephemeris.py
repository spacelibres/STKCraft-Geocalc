"""Native ephemeris TWO-VECTOR and constant-vector aberration frames."""
from pathlib import Path
import csv,itertools
import spiceypy as s
ROOT=Path(__file__).resolve().parents[1];lines=['KPL/FK','\\begindata',"NAIF_BODY_NAME=('MY_EARTH')","NAIF_BODY_CODE=(399)"];models=[]
corrections=['NONE','LT','LT+S','CN','CN+S','XLT','XLT+S','XCN','XCN+S']
for definition,correction,vector_frame,mode in itertools.product(['OBSERVER_TARGET_POSITION','OBSERVER_TARGET_VELOCITY','TARGET_NEAR_POINT'],corrections,['J2000','ECLIPJ2000','IAU_MOON','DYN_EULER_0'],['ROTATING','INERTIAL','FROZEN']):
 n=len(models);fid=896000+n;name=f'EPHEM_TWO_{n}';models.append(name);base=['J2000','ECLIPJ2000','IAU_MOON','DYN_EULER_13'][n%4];target,observer=(('Earth','SUN') if definition=='OBSERVER_TARGET_VELOCITY' else ('MOON','MY_EARTH'))
 lines += [f"FRAME_{name}={fid}",f"FRAME_{fid}_NAME='{name}'",f"FRAME_{fid}_CLASS=5",f"FRAME_{fid}_CLASS_ID={fid}",f"FRAME_{fid}_CENTER=399",f"FRAME_{fid}_RELATIVE='{base}'",f"FRAME_{fid}_DEF_STYLE='PARAMETERIZED'",f"FRAME_{fid}_FAMILY='TWO-VECTOR'",f"FRAME_{fid}_PRI_AXIS='X'",f"FRAME_{fid}_PRI_VECTOR_DEF='{definition}'",f"FRAME_{fid}_PRI_TARGET='{target}'",f"FRAME_{fid}_PRI_OBSERVER='{observer}'",f"FRAME_{fid}_PRI_ABCORR='{correction}'",f"FRAME_{fid}_PRI_FRAME='{vector_frame}'",f"FRAME_{fid}_SEC_AXIS='-Z'",f"FRAME_{fid}_SEC_VECTOR_DEF='CONSTANT'",f"FRAME_{fid}_SEC_FRAME='ECLIPJ2000'",f"FRAME_{fid}_SEC_SPEC='RECTANGULAR'",f"FRAME_{fid}_SEC_VECTOR=(1.1 0.7 0.3)"]
 if mode=='FROZEN':lines.append(f'FRAME_{fid}_FREEZE_EPOCH=12345.25')
 else:lines.append(f"FRAME_{fid}_ROTATION_STATE='{mode}'")
for correction,frame in itertools.product(['S','XS','LT','CN','XLT','XCN','LT+S','CN+S','XCN+S'],['ECLIPJ2000','IAU_MOON']):
 n=len(models);fid=896000+n;name=f'EPHEM_TWO_{n}';models.append(name)
 lines += [f"FRAME_{name}={fid}",f"FRAME_{fid}_NAME='{name}'",f"FRAME_{fid}_CLASS=5",f"FRAME_{fid}_CLASS_ID={fid}",f"FRAME_{fid}_CENTER=399",f"FRAME_{fid}_RELATIVE='J2000'",f"FRAME_{fid}_DEF_STYLE='PARAMETERIZED'",f"FRAME_{fid}_FAMILY='TWO-VECTOR'",f"FRAME_{fid}_PRI_AXIS='X'",f"FRAME_{fid}_PRI_VECTOR_DEF='CONSTANT'",f"FRAME_{fid}_PRI_OBSERVER='MY_EARTH'",f"FRAME_{fid}_PRI_ABCORR='{correction}'",f"FRAME_{fid}_PRI_FRAME='{frame}'",f"FRAME_{fid}_PRI_SPEC='RA/DEC'",f"FRAME_{fid}_PRI_RA=23.7",f"FRAME_{fid}_PRI_DEC=-19.8",f"FRAME_{fid}_PRI_UNITS='DEGREES'",f"FRAME_{fid}_SEC_AXIS='Z'",f"FRAME_{fid}_SEC_VECTOR_DEF='CONSTANT'",f"FRAME_{fid}_SEC_FRAME='ECLIPJ2000'",f"FRAME_{fid}_SEC_SPEC='RECTANGULAR'",f"FRAME_{fid}_SEC_VECTOR=(1.1 0.7 0.3)"]
lines+=['\\begintext'];(ROOT/'tests/golden/dynamic_ephemeris.tf').write_text('\n'.join(lines)+'\n',encoding='utf-8')
for file in ['target/kernels/de440s.bsp','target/kernels/pck00011.tpc','tests/golden/dynamic_euler.tf','tests/golden/spk_dynamic_sources.bsp','tests/golden/dynamic_ephemeris.tf']:s.furnsh(str(ROOT/file))
with (ROOT/'tests/golden/dynamic_ephemeris.csv').open('w',newline='',encoding='utf-8') as f:
 w=csv.writer(f);w.writerow(['kind','from','to','et','status',*[f'r{i}' for i in range(9)],*[f'd{i}' for i in range(9)]]);count=0
 for name in models:
  for et in [-3000000000.,-12345.25,0.,12345.25,3000000000.]:
   for reference in ['J2000','ECLIPJ2000']:
    for a,b in [(name,reference),(reference,name)]:
     for kind in ['rotation','state']:
      status='OK';values=[]
      try:
       if kind=='rotation':values=s.pxform(a,b,et).flatten().tolist()
       else:x=s.sxform(a,b,et);values=[*x[:3,:3].flatten().tolist(),*x[3:,:3].flatten().tolist()]
      except s.utils.exceptions.SpiceyError as e:status=e.short
      w.writerow([kind,a,b,et,status,*values]);count+=1
print(f'{len(models)} ephemeris frames, {count} native transforms')
