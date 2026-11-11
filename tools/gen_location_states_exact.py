"""Independent strict SPKCPT/SPKCPO/SPKCVT/SPKCVO location fixtures."""
import argparse
from pathlib import Path
import spiceypy as s
p=argparse.ArgumentParser();p.add_argument('--out',default='tests/golden/location_states_exact.csv');a=p.parse_args()
s.furnsh('target/kernels/de442s.bsp');s.furnsh('target/kernels/pck00011.tpc')
r=s.bodvrd('EARTH','RADII',3)[1];station=list(s.georec(-118*s.rpd(),34*s.rpd(),.3,r[0],1-r[2]/r[0]))
corrections=['NONE','LT','CN','LT+S','CN+S','XLT','XCN','XLT+S','XCN+S']
cases=[('FIXED_TARGET',301,'IAU_MOON',[1000.,2000.,1500.],[0.,0.,0.],0.,0,399),
       ('FIXED_OBSERVER',399,'IAU_EARTH',station,[0.,0.,0.],0.,301,0),
       ('CV_TARGET',399,'J2000',[7000.,100.,200.],[.01,7.4,.2],0.,0,301),
       ('CV_OBSERVER',399,'IAU_EARTH',station,[.001,-.002,.003],0.,301,0)]
out=Path(a.out);count=0
with out.open('w',newline='\n') as h:
 h.write('# Independent CSPICE N0067 SPKCPT/SPKCPO/SPKCVT/SPKCVO, DE442s/PCK00011; exact doubles\n')
 h.write('kind,et,output_frame,locus,correction,center,input_frame,x,y,z,vx,vy,vz,point_epoch,target,observer,result_x,result_y,result_z,result_vx,result_vy,result_vz,light_time\n')
 for kind,center,source,pos,vel,epoch,target,observer in cases:
  for et in [-1e8,0.,1e8]:
   for frame in ['J2000','ECLIPJ2000','IAU_EARTH','IAU_MOON']:
    for locus in ['CENTER','OBSERVER','TARGET']:
     for correction in corrections:
      if kind=='FIXED_TARGET':state,lt=s.spkcpt(pos,str(center),source,et,frame,locus,correction,str(observer))
      elif kind=='FIXED_OBSERVER':state,lt=s.spkcpo(str(target),et,frame,locus,correction,pos,str(center),source)
      elif kind=='CV_TARGET':state,lt=s.spkcvt(pos+vel,epoch,str(center),source,et,frame,locus,correction,str(observer))
      else:state,lt=s.spkcvo(str(target),et,frame,locus,correction,pos+vel,epoch,str(center),source)
      h.write(','.join(map(str,[kind,et,frame,locus,correction,center,source,*pos,*vel,epoch,target,observer,*state,lt]))+'\n');count+=1
s.kclear();print('wrote',out,count,'rows')
