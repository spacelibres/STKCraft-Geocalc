"""Independent N0067 ZZSEGBOX/ZZRYTELT references; no local geometry inputs."""
from pathlib import Path
import csv, ctypes as c, math, random
import spiceypy as s
from spiceypy.utils.libspicehelper import libspice

ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'tests/golden/dsk'; OUT.mkdir(exist_ok=True)
D=c.c_double
def arr(v): return (D*len(v))(*v)
def check():
    if s.failed():
        msg=s.getmsg('LONG');s.reset();raise RuntimeError(msg)

def main():
    rng=random.Random(10967)
    domains=[]
    for i in range(180):
        system=[1,3,4][i%3]
        if system==3:
            bounds=[]
            for j in range(3):
                lo=rng.uniform(-100,100);bounds += [lo,lo+rng.uniform(.01,200)]
        else:
            lo=rng.uniform(-math.pi,math.pi)
            width=rng.choice([2*math.pi,.2,1.9,4.2]);hi=lo+width
            if width==2*math.pi:lo,hi=-math.pi,math.pi
            if hi>math.pi:hi-=2*math.pi
            lat=sorted([rng.uniform(-math.pi/2,math.pi/2) for _ in range(2)])
            bounds=[lo,hi,*lat,1.,5.] if system==1 else [lo,hi,*lat,-.5,2.]
        params=[3.2, rng.choice([0.,.2,-.25])] + [0.]*8
        raw=[1.,399.,2.,2.,10013.,float(system),*params,*bounds,-1e6,1e6]
        domains.append(raw)
    with (OUT/'volume_bounds.csv').open('w',newline='') as f:
        w=csv.writer(f);w.writerow([*['d'+str(i) for i in range(24)],'cx','cy','cz','radius'])
        for raw in domains:
            center=arr([0.]*3);radius=D()
            libspice.zzsegbox_(arr(raw),center,c.byref(radius));check()
            w.writerow([*map(repr,raw),*map(repr,center),repr(radius.value)])
    with (OUT/'volume_rays.csv').open('w',newline='') as f:
        w=csv.writer(f);w.writerow([*['d'+str(i) for i in range(24)],'vx','vy','vz','dx','dy','dz','margin','found','x','y','z'])
        for raw in domains:
            for j in range(24):
                vertex=[rng.uniform(-15,15) for _ in range(3)]
                direction=[-v+rng.uniform(-4,4) for v in vertex]
                margin=D([0.,1e-8,2e-8][j%3]);count=c.c_int();point=arr([0.]*3)
                libspice.zzrytelt_(arr(vertex),arr(direction),arr(raw),c.byref(margin),c.byref(count),point);check()
                w.writerow([*map(repr,raw),*map(repr,vertex),*map(repr,direction),repr(margin.value),count.value,*map(repr,point)])
    with (OUT/'volume_edges.csv').open('w',newline='') as f:
        w=csv.writer(f);w.writerow([*['d'+str(i) for i in range(24)],'vx','vy','vz','dx','dy','dz','margin','found','x','y','z'])
        for system,flattenings in [(1,[0.]),(4,[0.,.8,-2.])]:
            for flattening in flattenings:
                for lonbounds in [(-math.pi,math.pi),(2.9,-2.9),(-.01,.01),(0.,math.pi)]:
                    for latbounds in [(-math.pi/2,math.pi/2),(0.,0.),(-.01,.01),(-math.pi/2,0.),(0.,math.pi/2),(.01,.4),(-.4,-.01)]:
                        bounds=[*lonbounds,*latbounds,*([0.,5.]if system==1 else [-.01,2.])]
                        params=[3.2,flattening]+[0.]*8
                        raw=[1.,399.,2.,2.,10013.,float(system),*params,*bounds,-1e6,1e6]
                        pairs=[([0.,0.,0.],[1.,0.,0.]),([10.,0.,0.],[-1.,0.,0.]),([0.,0.,10.],[0.,0.,-1.]),([-10.,0.,0.],[1.,0.,0.]),([3.2,0.,0.],[0.,0.,1.]),([1e12,-2e12,3e12],[-1.,2.,-3.])]
                        pairs += [([3.,4.,1e-15],[0.,1.,0.]),([0.,0.,-1.],[1e-15,0.,1.]),([0.,0.,1.],[1e-15,0.,-1.])]
                        for vertex,direction in pairs:
                            for margin_value in [0.,1e-8]:
                                margin=D(margin_value);count=c.c_int();point=arr([0.]*3)
                                libspice.zzrytelt_(arr(vertex),arr(direction),arr(raw),c.byref(margin),c.byref(count),point);check()
                                w.writerow([*map(repr,raw),*map(repr,vertex),*map(repr,direction),repr(margin.value),count.value,*map(repr,point)])
    with (OUT/'volume_cones.csv').open('w',newline='') as f:
        w=csv.writer(f);w.writerow(['ax','ay','az','angle','p1x','p1y','p1z','p2x','p2y','p2z','count','x1','y1','z1','x2','y2','z2'])
        for i in range(4200):
            apex=[0.,0.,rng.uniform(-2,2)]
            angle=rng.choice([0.,1e-15,1e-10,.3,1.2,math.pi/2-1e-15,math.pi/2,math.pi/2+1e-15,2.,math.pi-1e-10,math.pi])
            p1=[rng.uniform(-10,10) for _ in range(3)]
            p2=[rng.uniform(-10,10) for _ in range(3)]
            if i%5==0:
                p1=[10.,0.,rng.uniform(-5,5)];p2=[-10.,0.,p1[2]]
            count=c.c_int();x1=arr([0.]*3);x2=arr([0.]*3);a=D(angle)
            libspice.incnsg_(arr(apex),arr([0.,0.,1.]),c.byref(a),arr(p1),arr(p2),c.byref(count),x1,x2);check()
            w.writerow([*map(repr,apex),repr(angle),*map(repr,p1),*map(repr,p2),count.value,*map(repr,x1),*map(repr,x2)])
    with (OUT/'volume_quadratic.csv').open('w',newline='') as f:
        w=csv.writer(f);w.writerow(['a','b','c','upper','count','r1','r2'])
        coefficients=[(1.,-3.,2.),(0.,2.,1.),(0.,0.,0.),(0.,0.,1.),(1.,2.,1.),(1.,2.,3.)]
        coefficients += [(rng.choice([0.,1.,-1.])*10**rng.uniform(-20,20),rng.uniform(-10,10),rng.uniform(-10,10)) for _ in range(3200)]
        for aa,bb,cc in coefficients:
            for ub in [.1,100.]:
                a,b,co,u=map(D,[aa,bb,cc,ub]);n=c.c_int();r1=D();r2=D()
                libspice.zzcnquad_(c.byref(a),c.byref(b),c.byref(co),c.byref(u),c.byref(n),c.byref(r1),c.byref(r2));check()
                w.writerow([repr(aa),repr(bb),repr(cc),repr(ub),n.value,repr(r1.value),repr(r2.value)])
if __name__=='__main__':main()
