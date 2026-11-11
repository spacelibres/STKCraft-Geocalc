"""Native named-FIXREF GFSNTC/ZZGFSSIN with binary PCK and triaxial radii."""
from pathlib import Path
import csv,ctypes as ct,spiceypy as s
from spiceypy.utils.libspicehelper import libspice
root=Path(__file__).resolve().parents[1]
for name in ['de440s.bsp','pck00011.tpc','moon_pa_de440_200625.bpc','moon_de440_250416.tf','moon_assoc_me.tf']:s.furnsh(str(root/'target/kernels'/name))
s.furnsh(str(root/'tests/golden/surface_target_frames.tf'))
D=ct.c_double;I=ct.c_int

def state(et,fixref,corr,dref,dvec,radii):
 t=I(301);o=I(399);center=I(s.frinfo(s.namfrm(dref))[0]);epoch=D(et);vec=(D*3)(*dvec);r=(D*3)(*radii);st=(D*6)();found=I()
 libspice.zzgfssin_(b'ELLIPSOID',ct.byref(t),ct.byref(epoch),fixref.encode(),corr.encode(),ct.byref(o),dref.encode(),ct.byref(center),vec,r,st,ct.byref(found),I(9),I(len(fixref)),I(len(corr)),I(len(dref)))
 if s.failed():raise RuntimeError(s.getmsg('LONG'))
 return found.value,list(st)

def cf(et):
 c=s.cell_double(1000);s.wninsd(et,et+300.,c);return c

def packed(w):return ';'.join(repr(a)+':'+repr(b) for a,b in [s.wnfetd(w,i) for i in range(s.wncard(w))])
rows=[];states=[]
for scenario in ['native','triaxial']:
 if scenario=='triaxial':s.pdpool('BODY301_RADII',[1738.1,1736.7,1734.2])
 radii=s.bodvrd('MOON','RADII',3)[1]
 for fixref in ['MOON_ME','MOON_PA','TEST_MOON_TK']:
  for et in [0.,5e7]:
   for corr in ['NONE','LT','LT+S','CN','CN+S','XLT','XLT+S','XCN','XCN+S']:
    for dref in ['J2000','TEST_MOON_TK']:
     dvec=s.spkpos('MOON',et,dref,corr,'EARTH')[0]
     for offset in [0.,30.,120.]:
      found,st=state(et+offset,fixref,corr,dref,dvec,radii);states.append([scenario,fixref,et+offset,corr,dref,*dvec,found,*st])
     point=s.sincpt('ELLIPSOID','MOON',et,fixref,corr,'EARTH',dref,dvec)[0]
     for system,coordinate,ref in [('RECTANGULAR','X',point[0]),('LATITUDINAL','LONGITUDE',s.reclat(point)[1]),('PLANETOGRAPHIC','LATITUDE',s.recpgr('MOON',point,radii[0],(radii[0]-radii[2])/radii[0])[1])]:
      for op in ['<','>','=','ABSMAX','ABSMIN','LOCMAX','LOCMIN']:
       try:
        w=s.gfsntc('MOON',fixref,'ELLIPSOID',corr,'EARTH',dref,dvec,system,coordinate,op,float(ref),0.,10.,1000,cf(et));expected=packed(w)
       except s.utils.exceptions.SpiceNOTSUPPORTED:expected='ERROR:NOTSUPPORTED'
       rows.append([scenario,fixref,et,corr,dref,*dvec,system,coordinate,op,ref,expected])
with open(root/'tests/golden/gf_intercept_target_frames_precision.csv','w',newline='') as f:
 w=csv.writer(f);w.writerow(['scenario','target_frame','epoch','correction','direction_frame','dx','dy','dz','system','coordinate','relation','reference','expected']);w.writerows(rows)
with open(root/'tests/golden/gf_intercept_target_states_precision.csv','w',newline='') as f:
 w=csv.writer(f);w.writerow(['scenario','target_frame','epoch','correction','direction_frame','dx','dy','dz','found','x','y','z','vx','vy','vz']);w.writerows(states)
print(len(rows),'named-target-frame GFSNTC cases;',len(states),'ZZGFSSIN states')
