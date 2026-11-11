"""Independent CSPICE-written DAS/DLA/type-2 DSK structure and geometry fixtures."""
from pathlib import Path
import csv
import numpy as np
import spiceypy as s
root=Path(__file__).resolve().parents[1]
out=root/'tests/golden/dsk';out.mkdir(exist_ok=True)
vertices=np.array([[3.,0.,0.],[-2.,0.,0.],[0.,4.,0.],[0.,-3.,0.],[0.,0.,5.],[0.,0.,-2.]])
plates=np.array([[1,3,5],[3,2,5],[2,4,5],[4,1,5],[3,1,6],[2,3,6],[4,2,6],[1,4,6]],dtype=np.int32)
rows=[];rays=[]
for optimize in [False,True]:
 name='optimized.bds' if optimize else 'unoptimized.bds'
 path=out/name
 if path.exists():path.unlink()
 h=s.dskopn(str(path),'STKCraft independent native DSK fixture',4096)
 s.dasac(h,['Native CSPICE N0067 fixture with multiple DLA segments.']*35)
 for index,(scale,offset) in enumerate([(1.,[0.,0.,0.]),(1.23456789,[.125,-.25,.375])]):
  v=vertices*scale+offset
  d,i=s.dskmi2(v,plates,5.,4,100000,100000,100000,True,400000)
  lower,upper=s.dskrb2(v,plates,1,np.zeros(10))
  s.dskw02(h,301,100+index,2,'IAU_MOON',1,np.zeros(10),-np.pi,np.pi,-np.pi/2,np.pi/2,lower,upper,-1e9+index,1e9-index,v,plates,d,i)
 s.dskcls(h,optimize)
 h=s.dasopr(str(path));seg=s.dlabfs(h);index=0
 while True:
  desc=s.dskgd(h,seg)
  raw=[seg.bwdptr,seg.fwdptr,seg.ibase,seg.isize,seg.dbase,seg.dsize,seg.cbase,seg.csize]
  rows.append([name,index,'dla',';'.join(map(str,raw))])
  ds=[desc.surfce,desc.center,desc.dclass,desc.dtype,desc.frmcde,desc.corsys,*desc.corpar,desc.co1min,desc.co1max,desc.co2min,desc.co2max,desc.co3min,desc.co3max,desc.start,desc.stop]
  rows.append([name,index,'descriptor',';'.join(map(repr,map(float,ds)))])
  for label,item in [('bounds',16),('origin',17),('voxel_size',18)]:
   a=s.dskd02(h,seg,item,0,100)
   rows.append([name,index,label,';'.join(map(repr,map(float,a)))])
  pars=s.dasrdi(h,seg.ibase+1,seg.ibase+10)
  rows.append([name,index,'parameters',';'.join(map(str,pars))])
  for j,v in enumerate(s.dskv02(h,seg,1,int(pars[0]))):rows.append([name,index,'vertex'+str(j+1),';'.join(map(repr,map(float,v)))])
  for j,p in enumerate(s.dskp02(h,seg,1,int(pars[1]))):
   rows.append([name,index,'plate'+str(j+1),';'.join(map(str,p))])
   rows.append([name,index,'normal'+str(j+1),';'.join(map(repr,map(float,s.dskn02(h,seg,j+1))))])
  rng=np.random.default_rng(72613)
  vertices_local=s.dskv02(h,seg,1,int(pars[0]))
  inputs=[]
  for distance in [0.,.5,1.,10.,1e6,1e12]:
   for _ in range(70):
    vertex=rng.normal(size=3)*distance
    direction=-vertex+rng.normal(size=3)*3
    inputs.append((vertex,direction))
  for vertex in [[20.,0.,0.],[-20.,0.,0.],[0.,20.,0.],[0.,0.,20.],[.125,-.25,.375]]:
   for aim in vertices_local:
    for perturb in [0.,1e-12,-1e-12]:inputs.append((np.array(vertex),aim-np.array(vertex)+[0.,perturb,0.]))
  for vertex,direction in inputs:
   try:
    plate,point,found=s.dskx02(h,seg,vertex,direction)
    expected=[plate,*point,*s.dskn02(h,seg,int(plate))] if found else []
   except s.utils.exceptions.NotFoundError:expected=[]
   rays.append([name,index,*map(float,vertex),*map(float,direction),';'.join(map(repr,map(float,expected)))])
  try:seg=s.dlafns(h,seg)
  except s.utils.exceptions.NotFoundError:break
  index+=1
 s.dascls(h)
with (out/'structure.csv').open('w',newline='') as f:
 w=csv.writer(f);w.writerow(['file','segment','kind','expected']);w.writerows(rows)
print(len(rows),'exact native structure/normal rows')



with (out/'rays.csv').open('w',newline='') as f:
 w=csv.writer(f);w.writerow(['file','segment','vx','vy','vz','dx','dy','dz','expected']);w.writerows(rays)
print(len(rays),'native ray/plate/normal cases')
