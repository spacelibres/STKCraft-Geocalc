"""CSPICE state/light-time and two-epoch frame transformation oracle."""
from pathlib import Path
import csv
import spiceypy as s
root=Path(__file__).resolve().parents[1]
s.furnsh(str(root/'target/kernels/de442s.bsp'));s.furnsh(str(root/'target/kernels/pck00011.tpc'))
pairs=[('IAU_EARTH','IAU_MOON'),('IAU_MOON','IAU_SUN'),('IAU_EARTH','ECLIPJ2000'),
       ('B1950','IAU_MOON'),('IAU_EARTH','IAU_EARTH'),('J2000','ECLIPJ2000')]
with (root/'tests/golden/frame_epochs_exact.csv').open('w',newline='') as file:
    out=csv.writer(file);out.writerow(['frame1','frame2','et','correction','locus','et1','et2','lt','state_matrix'])
    for one,two in pairs:
        center1=s.frinfo(s.namfrm(one))[0];center2=s.frinfo(s.namfrm(two))[0]
        for et in [-5e8,0.,5e8]:
            for correction in ['NONE','LT','CN','XLT','XCN']:
                for locus in ['FRAME1','FRAME2']:
                    lt=0.;factor=1.;t1=et;t2=et
                    if correction!='NONE' and center1!=center2:
                        _,lt,dlt=s.spkacs(center2 if locus=='FRAME1' else center1,et,'J2000',correction,center1 if locus=='FRAME1' else center2)
                        sign=1. if correction.startswith('X') else -1.
                        factor=1.+sign*dlt
                        if locus=='FRAME1':t2=et+sign*lt
                        else:t1=et+sign*lt
                    if correction=='NONE' or center1==center2:
                        matrix=s.sxform(one,two,et)
                    else:
                        a=s.sxform(one,'J2000',t1);b=s.sxform('J2000',two,t2)
                        if locus=='FRAME1':b[3:,:3]*=factor
                        else:a[3:,:3]*=factor
                        matrix=s.mxmg(b,a)
                    out.writerow([one,two,repr(et),correction,locus,repr(t1),repr(t2),repr(lt),*[repr(float(x)) for x in matrix.flat]])
s.kclear()
