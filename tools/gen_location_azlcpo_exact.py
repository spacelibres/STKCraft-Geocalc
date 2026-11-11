"""Independent AZLCPO references: normal and polar observers, all signs/corrections."""
from pathlib import Path
import spiceypy as s
s.furnsh('target/kernels/de442s.bsp');s.furnsh('target/kernels/pck00011.tpc')
r=s.bodvrd('EARTH','RADII',3)[1]
positions=[list(s.georec(-118*s.rpd(),34*s.rpd(),.3,r[0],1-r[2]/r[0])),[0.,0.,r[2]+.3],[0.,0.,-r[2]-.3]]
corrections=['NONE','LT','CN','LT+S','CN+S','XLT','XCN','XLT+S','XCN+S'];out=Path('tests/golden/location_azlcpo_exact.csv');count=0
with out.open('w',newline='\n') as h:
 h.write('# Independent CSPICE N0067 AZLCPO, DE442s/PCK00011; exact doubles\n')
 h.write('et,x,y,z,azccw,elplsz,correction,range,az,el,range_rate,az_rate,el_rate,light_time\n')
 for pos in positions:
  for et in [-1e8,0.,1e8]:
   for correction in corrections:
    for azccw in [False,True]:
     for elplsz in [False,True]:
      state,lt=s.azlcpo('ELLIPSOID','MOON',et,correction,azccw,elplsz,pos,'EARTH','IAU_EARTH')
      h.write(','.join(map(str,[et,*pos,str(azccw).lower(),str(elplsz).lower(),correction,*state,lt]))+'\n');count+=1
s.kclear();print('wrote',out,count,'rows')
