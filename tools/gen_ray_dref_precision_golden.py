"""Native SINCPT/TANGPT with DREF, alongside independently resolved rays."""
from pathlib import Path
import csv,spiceypy as s
root=Path(__file__).resolve().parents[1]
s.furnsh(str(root/'target/kernels/de442s.bsp'));s.furnsh(str(root/'target/kernels/pck00011.tpc'));s.furnsh(str(root/'tests/golden/gf_intercept_dref.tf'))
rows=[]
for target,observer,fixref in [(301,399,'IAU_MOON'),(399,301,'IAU_EARTH')]:
 for et in [-5e8,0.,5e8]:
  for corr in ['NONE','LT','LT+S','CN','CN+S','XLT','XLT+S','XCN','XCN+S']:
   for frame in ['J2000','ECLIPJ2000','IAU_EARTH','IAU_MOON','GF_DREF_MOON']:
    ray=s.spkpos(str(target),et,frame,corr,str(observer))[0]
    for anti in [False,True]:
     vec=s.vminus(ray) if anti else ray
     center,cl,_=s.frinfo(s.namfrm(frame));epoch=et
     if corr!='NONE' and cl!=1 and center!=observer:
      lt=s.spkpos(str(center),et,'J2000',corr,str(observer))[1];epoch=et+(lt if corr.startswith('X') else -lt)
     resolved=s.mxv(s.pxform(frame,'J2000',epoch),vec)
     try:
      point,te,sv=s.sincpt('ELLIPSOID',str(target),et,fixref,corr,str(observer),frame,vec);surf=[1,*point,s.vnorm(sv)]
     except s.utils.exceptions.NotFoundError:surf=[0,0.,0.,0.,0.]
     point,alt,ran,srf,te,sv=s.tangpt('ELLIPSOID',str(target),et,fixref,corr,'TANGENT POINT',str(observer),frame,vec)
     rows.append([target,observer,et,corr,frame,*ray,int(anti),*resolved,*surf,*point,alt,ran,*srf,te,*sv])
with open(root/'tests/golden/ray_dref_precision.csv','w',newline='') as f:
 w=csv.writer(f);w.writerow(['target','observer','epoch','correction','frame','dx','dy','dz','anti','rx','ry','rz','found','sx','sy','sz','surface_range','tx','ty','tz','altitude','range','px','py','pz','target_epoch','svx','svy','svz']);w.writerows(rows)
print(len(rows),'native typed DREF ray cases')
