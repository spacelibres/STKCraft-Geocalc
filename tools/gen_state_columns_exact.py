"""Generate direct CSPICE XFMSTA/DAZLDR state-column references; no finite differences."""
from pathlib import Path
import spiceypy as s
s.furnsh('target/kernels/pck00011.tpc')
states=[[1234.5,-2345.6,3456.7,.7,-1.1,.3],[1e8,2e8,-3e8,-2.,3.,4.],[-1000.,3000.,-4000.,1.,2.,3.],[1.,2.,3.,.1,.2,.3]]
reps=['RECTANGULAR','RA_DEC','LATITUDINAL','CYLINDRICAL','SPHERICAL','AZ_EL_TT','AZ_EL_TF','AZ_EL_FT','AZ_EL_FF','PLANETODETIC','PLANETOGRAPHIC']
out=Path('tests/golden/state_columns_exact.csv')
with out.open('w',newline='\n') as h:
 h.write('# CSPICE XFMSTA,REC RAD/REC AZL,DAZLDR,MXV,VNORM; exact doubles\n')
 h.write('representation,body,x,y,z,vx,vy,vz,values...\n')
 for state in states+[[0.,0.,1000.,0.,0.,2.],[0.,0.,-1000.,0.,0.,-2.]]:
  distance=s.vnorm(state[:3]);speed=s.vnorm(state[3:])
  for rep in reps:
   if state[0]==0. and state[1]==0. and rep.startswith('AZ_EL'):continue
   if rep=='RECTANGULAR':values=[distance,speed,*state]
   elif rep.startswith('AZ_EL'):
    flags=rep[-2:];r,az,el=s.recazl(state[:3],flags[0]=='T',flags[1]=='T');rates=s.mxv(s.dazldr(*state[:3],flags[0]=='T',flags[1]=='T'),state[3:]);values=[az*s.dpr(),el*s.dpr(),r,rates[1]*s.dpr(),rates[2]*s.dpr(),rates[0],speed]
   else:
    name={'RA_DEC':'LATITUDINAL','PLANETODETIC':'GEODETIC'}.get(rep,rep)
    v=s.xfmsta(state,'RECTANGULAR',name,'EARTH')
    if rep in ['RA_DEC','LATITUDINAL']:
     r,a,b=v[:3]
     if rep=='RA_DEC':r,a,b=s.recrad(state[:3])
     values=[a*s.dpr(),b*s.dpr(),r,v[4]*s.dpr(),v[5]*s.dpr(),v[3],speed]
    elif rep=='CYLINDRICAL':values=[v[1]*s.dpr(),v[2],v[0],v[4]*s.dpr(),v[5],v[3],speed]
    elif rep=='SPHERICAL':values=[v[2]*s.dpr(),v[1]*s.dpr(),v[0],v[5]*s.dpr(),v[4]*s.dpr(),v[3],speed]
    else:values=[v[0]*s.dpr(),v[1]*s.dpr(),v[2],v[3]*s.dpr(),v[4]*s.dpr(),v[5],speed]
   values += [12.3,4.5]
   h.write(','.join(map(str,[rep,399,*state,*values]))+'\n')
s.kclear();print('wrote',out)
