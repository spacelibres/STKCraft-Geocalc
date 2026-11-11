"""Independent RECPGR missing-PM/override behavior from native N0067."""
from pathlib import Path
import csv
import spiceypy as s
OUT=Path(__file__).resolve().parents[1]/'tests/golden'
rows=[]
for body in [10,301,399,299]:
    for coefficients in [[],[0.],[0.,0.],[0.,1.],[0.,-1.],[0.,2.,0.]]:
        for override in ['NONE','EAST','WEST']:
            s.kclear()
            if coefficients:s.pdpool(f'BODY{body}_PM',coefficients)
            if override!='NONE':s.pcpool(f'BODY{body}_PGR_POSITIVE_LON',[override])
            try:result=s.recpgr(str(body),[3.,4.,5.],1.,0.);expected=';'.join(repr(float(x)) for x in result)
            except Exception as error:expected=error.short
            rows.append([body,';'.join(map(repr,coefficients)),override,expected])
with (OUT/'planetographic_metadata.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['body','pm','override','expected']);w.writerows(rows)
s.kclear();print(f'{len(rows)} native PM and override fixtures')

# Orientation polynomials independently allow short, zero-padded arrays.
with (OUT/'pck_polynomial_lengths.csv').open('w',newline='') as f:
    w=csv.writer(f);w.writerow(['ra_count','dec_count','pm_count','et',*[f'r{i}' for i in range(9)],*[f'dr{i}' for i in range(9)]])
    for nr in [1,2,3]:
        for nd in [1,2,3]:
            for np in [1,2,3]:
                s.kclear();s.pdpool('BODY299_POLE_RA',[30.,-.1,.001][:nr]);s.pdpool('BODY299_POLE_DEC',[70.,.2,-.003][:nd]);s.pdpool('BODY299_PM',[10.,100.,.0002][:np])
                for et in [0.,1e7,-2e9]:
                    xf=s.tisbod('J2000',299,et);w.writerow([nr,nd,np,repr(et),*[repr(float(x)) for x in xf[:3,:3].flat],*[repr(float(x)) for x in xf[3:,:3].flat]])
s.kclear()
