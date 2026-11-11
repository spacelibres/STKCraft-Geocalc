"""Native GFSUBC/GFILUM and ZZGFSSOB named-target-frame source fixtures."""
from pathlib import Path
import csv,ctypes as ct,spiceypy as s
from spiceypy.utils.libspicehelper import libspice
root=Path(__file__).resolve().parents[1]
for name in ['de440s.bsp','pck00011.tpc','moon_pa_de440_200625.bpc','moon_de440_250416.tf','moon_assoc_me.tf']:s.furnsh(str(root/'target/kernels'/name))
s.furnsh(str(root/'tests/golden/surface_target_frames.tf'))
D=ct.c_double;I=ct.c_int;end=1728000.;step=7200.
def cf():
 c=s.cell_double(1000);s.wninsd(0.,end,c);return c
def packed(w):return ';'.join(repr(a)+':'+repr(b) for a,b in [s.wnfetd(w,i) for i in range(s.wncard(w))])
def run(fn,*args):
 try:return packed(fn(*args))
 except s.utils.exceptions.SpiceyError as e:return 'ERROR:'+type(e).__name__.removeprefix('Spice')
def substate(method,frame,corr,et,radii):
 target=I(301);observer=I(399);time=D(et);r=(D*3)(*radii);state=(D*6)()
 libspice.zzgfssob_(method.encode(),ct.byref(target),ct.byref(time),frame.encode(),corr.encode(),ct.byref(observer),r,state,I(len(method)),I(len(frame)),I(len(corr)))
 if s.failed():raise RuntimeError(s.getmsg('LONG'))
 return list(state)
subrows=[];illrows=[];states=[]
for scenario in ['native','triaxial']:
 if scenario=='triaxial':s.pdpool('BODY301_RADII',[1738.1,1736.7,1734.2])
 radii=s.bodvrd('MOON','RADII',3)[1]
 point=s.surfpt([10000.,-5000.,7000.],[-10000.,5000.,-7000.],*radii)
 for frame in ['MOON_ME','MOON_PA','TEST_MOON_TK']:
  for corr in ['NONE','LT','LT+S','CN','CN+S','XLT','XLT+S','XCN','XCN+S']:
   for method in ['NEAR POINT: ELLIPSOID','INTERCEPT: ELLIPSOID']:
    for et in [0.,200000.,800000.]:states.append([scenario,frame,corr,method,et,*substate(method,frame,corr,et,radii)])
    mid=s.subpnt(method,'MOON',end*.5,frame,corr,'EARTH')[0]
    for system,coordinate,ref in [('RECTANGULAR','X',mid[0]),('LATITUDINAL','LONGITUDE',s.reclat(mid)[1]),('PLANETOGRAPHIC','LATITUDE',s.recpgr('MOON',mid,radii[0],(radii[0]-radii[2])/radii[0])[1])]:
     for op in ['>','=','ABSMAX']:
      expected=run(s.gfsubc,'MOON',frame,method,corr,'EARTH',system,coordinate,op,float(ref),0.,step,1000,cf())
      subrows.append([scenario,frame,corr,method,system,coordinate,op,ref,expected])
   for source in [10,299]:
    for angle in ['PHASE','INCIDENCE','EMISSION']:
     # Native GFILUM rejects transmission corrections at initialization.
     ref=s.illumg('ELLIPSOID','MOON',str(source),end*.5,frame,'NONE','EARTH',point)[2+['PHASE','INCIDENCE','EMISSION'].index(angle)]
     for op in ['>','=','ABSMAX']:
      expected=run(s.gfilum,'ELLIPSOID',angle,'MOON',str(source),frame,corr,'EARTH',point,op,float(ref),0.,step,1000,cf())
      illrows.append([scenario,frame,corr,source,angle,*point,op,ref,expected])
for name,header,rows in [('gf_subpoint_target_frames_precision.csv',['scenario','target_frame','correction','method','system','coordinate','relation','reference','expected'],subrows),('gf_illumination_target_frames_precision.csv',['scenario','target_frame','correction','illuminator','angle','x','y','z','relation','reference','expected'],illrows),('gf_subpoint_target_states_precision.csv',['scenario','target_frame','correction','method','epoch','x','y','z','vx','vy','vz'],states)]:
 with open(root/'tests/golden'/name,'w',newline='') as f:
  w=csv.writer(f);w.writerow(header);w.writerows(rows)
print(len(subrows),'GFSUBC,',len(illrows),'GFILUM,',len(states),'ZZGFSSOB cases')
