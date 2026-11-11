"""Native N0067 ZZPTPL02 and PLTNP point-membership references."""
from pathlib import Path
import csv, ctypes as c
import numpy as np
import spiceypy as s
from spiceypy.utils.libspicehelper import libspice
ROOT=Path(__file__).resolve().parents[1];OUT=ROOT/'tests/golden/dsk'
def da(v):return(c.c_double*len(v))(*v)
def ia(v):return(c.c_int*len(v))(*v)
def main():
 rng=np.random.default_rng(271828)
 with (OUT/'point_membership.csv').open('w',newline='')as f:
  w=csv.writer(f);w.writerow(['file','segment','x','y','z','found','plate'])
  for name in ['unoptimized.bds','optimized.bds','dense_lat.bds','dense_partial.bds','dense_rect.bds','dense_pdt.bds','dense_prolate.bds']:
   h=s.dasopr(str(OUT/name));d=s.dlabfs(h);index=0
   while True:
    desc=s.dskgd(h,d);raw=[desc.surfce,desc.center,desc.dclass,desc.dtype,desc.frmcde,desc.corsys,*desc.corpar,desc.co1min,desc.co1max,desc.co2min,desc.co2max,desc.co3min,desc.co3max,desc.start,desc.stop]
    draw=[d.bwdptr,d.fwdptr,d.ibase,d.isize,d.dbase,d.dsize,d.cbase,d.csize]
    nv,np_=s.dskz02(h,d);vertices=s.dskv02(h,d,1,nv);plates=s.dskp02(h,d,1,np_)
    points=[]
    for plate in plates[::max(1,np_//80)]:
     v=vertices[plate-1];normal=s.vhat(s.pltnrm(*v))
     for base in [v[0],(v[0]+v[1])/2.,(v[0]+v[1]+v[2])/3.]:
      for delta in [0.,1e-10,-1e-10,1e-7,-1e-7,1e-6,-1e-6]:points.append(base+delta*normal)
    points.extend(rng.normal(size=(200,3))*4.)
    for point in points:
     ph=c.c_int(h);plid=c.c_int();plate=ia([0]*3);verts=da([0.]*9);found=c.c_int()
     libspice.zzptpl02_(c.byref(ph),ia(draw),da(raw),da(point),c.byref(plid),plate,verts,c.byref(found))
     if s.failed():msg=s.getmsg('LONG');s.reset();raise RuntimeError(msg)
     w.writerow([name,index,*map(repr,map(float,point)),found.value,plid.value if found.value else 0])
    try:d=s.dlafns(h,d)
    except s.utils.exceptions.NotFoundError:break
    index+=1
   s.dascls(h)
 with (OUT/'plate_nearest.csv').open('w',newline='')as f:
  w=csv.writer(f);w.writerow([*['v'+str(i)for i in range(12)],'x','y','z','distance'])
  for i in range(2400):
   verts=rng.normal(size=(3,3))*10.;point=rng.normal(size=3)*20.
   if i%11==0:verts[2]=verts[1]
   if i%17==0:verts[1]=verts[0];verts[2]=verts[0]
   if i%23==0:verts[2]=verts[0]+.5*(verts[1]-verts[0])
   near,dist=s.pltnp(point,*verts)
   w.writerow([*map(repr,map(float,[*verts.flatten(),*point])),*map(repr,map(float,near)),repr(float(dist))])
if __name__=='__main__':main()
