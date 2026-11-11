"""Native DSKX02 on a concave, densely tessellated radial surface and clipped domains."""
from pathlib import Path
import csv
import numpy as np
import spiceypy as s
root=Path(__file__).resolve().parents[1];out=root/'tests/golden/dsk'
v=[np.array(a,dtype=float) for a in [[1,0,0],[-1,0,0],[0,1,0],[0,-1,0],[0,0,1],[0,0,-1]]]
p=[[0,2,4],[2,1,4],[1,3,4],[3,0,4],[2,0,5],[1,2,5],[3,1,5],[0,3,5]]
for _ in range(3):
 edges={};nextplates=[]
 def middle(a,b):
  key=tuple(sorted((a,b)))
  if key not in edges:
   x=s.vhat(v[a]+v[b]);edges[key]=len(v);v.append(x)
  return edges[key]
 for a,b,c in p:
  ab=middle(a,b);bc=middle(b,c);ca=middle(c,a)
  nextplates.extend([[a,ab,ca],[ab,b,bc],[ca,bc,c],[ab,bc,ca]])
 p=nextplates
v=np.array([x*(3.+.8*np.sin(4.*x[0])*np.cos(6.*x[1])*np.sin(3.*x[2])) for x in v]);p=np.array(p,dtype=np.int32)+1
spxd,spxi=s.dskmi2(v,p,1.5,4,100000,100000,100000,True,400000)
rows=[];rng=np.random.default_rng(837342)
for name,system,pars,bounds in [('dense_lat',1,np.zeros(10),[-np.pi,np.pi,-np.pi/2,np.pi/2,0.,5.]),('dense_partial',1,np.zeros(10),[2.,-2.,-.4,.7,1.,5.]),('dense_rect',3,np.zeros(10),[-2.,2.,-2.,2.,-2.,2.]),('dense_pdt',4,np.array([1.,.2]+[0.]*8),[-np.pi,.5,-.4,1.2,.1,5.]),('dense_prolate',4,np.array([1.,-.2]+[0.]*8),[-np.pi,np.pi,-np.pi/2,np.pi/2,.1,5.])]:
 path=out/(name+'.bds')
 if path.exists():path.unlink()
 h=s.dskopn(str(path),name,0)
 s.dskw02(h,301,1,2,'IAU_MOON',system,pars,*bounds,-1e9,1e9,v,p,spxd,spxi);s.dskcls(h,True)
 h=s.dasopr(str(path));seg=s.dlabfs(h);inputs=[]
 for distance in [.5,3.,10.,1e6,1e12]:
  for _ in range(350):
   vertex=rng.normal(size=3)*distance;direction=-vertex+rng.normal(size=3)*3.;inputs.append((vertex,direction))
 for vertex in [[20.,0.,0.],[-20.,0.,0.],[0.,20.,0.],[0.,0.,20.],[0.,0.,0.]]:
  for target in v[::13]:
   for perturb in [0.,1e-12,-1e-12]:inputs.append((np.array(vertex),target-np.array(vertex)+[perturb,0.,0.]))
 for vertex,direction in inputs:
  plate,point,found=s.dskx02(h,seg,vertex,direction);expected=[plate,*point,*s.dskn02(h,seg,int(plate))] if found else []
  rows.append([name+'.bds',0,*map(float,vertex),*map(float,direction),';'.join(map(repr,map(float,expected)))])
 s.dascls(h)
with (out/'dense_rays.csv').open('w',newline='') as f:
 w=csv.writer(f);w.writerow(['file','segment','vx','vy','vz','dx','dy','dz','expected']);w.writerows(rows)
print(len(v),'vertices',len(p),'plates;',len(rows),'native concave/clipped DSKX02 rays')
