"""Independent native M2Q/RAXISA/XF2RAV/XF2EUL oracles; no differencing."""
from pathlib import Path
import csv,itertools,math,random
import numpy as np
import spiceypy as s
root=Path(__file__).resolve().parents[1]
s.furnsh(str(root/'target/kernels/pck00011.tpc'))
transforms=[s.sxform(a,b,t) for a,b,t in [('IAU_SATURN','IAU_ENCELADUS',0.),
 ('IAU_MOON','IAU_EARTH',0.),('IAU_SATURN','IAU_ENCELADUS',5e8),('J2000','IAU_MOON',0.),('IAU_EARTH','J2000',-5e8)]]
random.seed(34829)
axes=[x for x in itertools.product([1,2,3],repeat=3) if x[0]!=x[1] and x[1]!=x[2]]
for _ in range(40):
    a=axes[random.randrange(12)]
    values=[random.uniform(-math.pi,math.pi) for _ in range(3)]+[random.uniform(-.01,.01) for _ in range(3)]
    transforms.append(s.eul2xf(values,*a))
for a in axes:
    for angle in [0.,math.pi/2,math.pi,1e-15]:
        transforms.append(s.eul2xf([0.,angle,0.,.001,-.002,.003],*a))
with (root/'tests/golden/orientation_exact.csv').open('w',newline='') as file:
    out=csv.writer(file);out.writerow(['case','kind','axis1','axis2','axis3',*[f'm{i}' for i in range(18)],'values'])
    for case,xform in enumerate(transforms):
        r=xform[:3,:3].copy(); d=xform[3:,:3].copy(); prefix=[*[repr(float(x)) for x in r.flat],*[repr(float(x)) for x in d.flat]]
        def row(kind,a,values):out.writerow([case,kind,*a,*prefix,*[repr(float(x)) for x in values]])
        row('QUAT',[0,0,0],s.m2q(r))
        axis,angle=s.raxisa(r);row('AA',[0,0,0],[angle,*axis])
        _,av=s.xf2rav(xform);row('AV1',[0,0,0],av);row('AV2',[0,0,0],s.mxv(r,av))
        row('UNIT_AV1',[0,0,0],s.vhat(av));row('UNIT_AV2',[0,0,0],s.vhat(s.mxv(r,av)))
        for a1,a2,a3 in axes:
            values,unique=s.xf2eul(xform,a3,a2,a1)
            row('EULER',[a1,a2,a3],values)
            row('EULER_DEG_RAD',[a1,a2,a3],[*[s.convrt(float(x),'RADIANS','DEGREES') for x in values[:3]],*values[3:]])
            row('EULER_RAD_DEG',[a1,a2,a3],[*values[:3],*[s.convrt(float(x),'RADIANS','DEGREES') for x in values[3:]]])
s.kclear()
