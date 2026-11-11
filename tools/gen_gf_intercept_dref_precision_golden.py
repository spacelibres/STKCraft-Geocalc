"""Independent GFSNTC windows and Fortran ZZGFSSIN states for named DREF."""
from pathlib import Path
import csv, ctypes as ct
import spiceypy as s
from spiceypy.utils.libspicehelper import libspice
root=Path(__file__).resolve().parents[1]
fk='''KPL/FK
\\begindata
FRAME_1480001_NAME='GF_DREF_MOON'
FRAME_GF_DREF_MOON=1480001
FRAME_1480001_CLASS=4
FRAME_1480001_CLASS_ID=1480001
FRAME_1480001_CENTER=301
TKFRAME_1480001_RELATIVE='IAU_MOON'
TKFRAME_1480001_SPEC='ANGLES'
TKFRAME_1480001_UNITS='DEGREES'
TKFRAME_1480001_AXES=(3,2,1)
TKFRAME_1480001_ANGLES=(37,11,-24)
\\begintext
'''
(root/'tests/golden/gf_intercept_dref.tf').write_text(fk)
s.furnsh(str(root/'target/kernels/de442s.bsp'));s.furnsh(str(root/'target/kernels/pck00011.tpc'));s.furnsh(str(root/'tests/golden/gf_intercept_dref.tf'))
D=ct.c_double;I=ct.c_int

def state(target,observer,fixref,radii,et,frame,corr,dvec):
 t=I(target);o=I(observer);dc=I(s.frinfo(s.namfrm(frame))[0]);epoch=D(et);vec=(D*3)(*dvec);r=(D*3)(*radii);out=(D*6)();found=I()
 libspice.zzgfssin_(b'ELLIPSOID',ct.byref(t),ct.byref(epoch),fixref.encode(),corr.encode(),ct.byref(o),frame.encode(),ct.byref(dc),vec,r,out,ct.byref(found),I(9),I(len(fixref)),I(len(corr)),I(len(frame)))
 if s.failed():raise RuntimeError(s.getmsg('LONG'))
 return found.value,list(out)

def cell(a,b):
 c=s.cell_double(1000);s.wninsd(a,b,c);return c

def packed(w):return ';'.join(repr(a)+':'+repr(b) for a,b in [s.wnfetd(w,i) for i in range(s.wncard(w))])
rows=[];states=[]
for target,observer,fixref in [(301,399,'IAU_MOON'),(399,301,'IAU_EARTH')]:
 radii=s.bodvcd(target,'RADII',3)[1]
 for epoch in [0.,5e8,-5e8]:
  for corr in ['NONE','LT','LT+S','CN','CN+S','XLT','XLT+S','XCN','XCN+S']:
   for frame in ['J2000','ECLIPJ2000','IAU_EARTH','GF_DREF_MOON']:
    dvec=s.spkpos(str(target),epoch,frame,corr,str(observer))[0]
    for offset in [-120.,0.,30.,120.,300.]:
     found,st=state(target,observer,fixref,radii,epoch+offset,frame,corr,dvec)
     states.append([epoch+offset,corr,frame,*dvec,found,*st,target,observer])
    point=s.sincpt('ELLIPSOID',str(target),epoch,fixref,corr,str(observer),frame,dvec)[0]
    for system,coordinate,ref in [('RECTANGULAR','X',point[0]),('LATITUDINAL','LONGITUDE',s.reclat(point)[1]),('PLANETOGRAPHIC','LATITUDE',s.recpgr(str(target),point,radii[0],(radii[0]-radii[2])/radii[0])[1])]:
     for op in ['<','>','=','ABSMAX','ABSMIN','LOCMAX','LOCMIN']:
      w=s.gfsntc(str(target),fixref,'ELLIPSOID',corr,str(observer),frame,dvec,system,coordinate,op,float(ref),0.,10.,1000,cell(epoch,epoch+300.))
      rows.append([epoch,corr,frame,*dvec,system,coordinate,op,ref,packed(w),target,observer])
with open(root/'tests/golden/gf_intercept_dref_precision.csv','w',newline='') as f:
 w=csv.writer(f);w.writerow(['epoch','correction','frame','dx','dy','dz','system','coordinate','relation','reference','expected','target','observer']);w.writerows(rows)
with open(root/'tests/golden/gf_intercept_dref_states_precision.csv','w',newline='') as f:
 w=csv.writer(f);w.writerow(['epoch','correction','frame','dx','dy','dz','found','x','y','z','vx','vy','vz','target','observer']);w.writerows(states)
print(len(rows),'GFSNTC windows,',len(states),'direct ZZGFSSIN states')
