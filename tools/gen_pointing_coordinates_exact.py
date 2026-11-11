"""Independent CSPICE double-precision pointing coordinate fixtures (no tolerances)."""
import argparse
from pathlib import Path
import spiceypy as s

REPRESENTATIONS=['RECTANGULAR','RA_DEC','LATITUDINAL','CYLINDRICAL','SPHERICAL','AZ_EL_TT','AZ_EL_TF','AZ_EL_FT','AZ_EL_FF']
VECTORS=[[1.,2.,3.],[-4.,5.,-6.],[1e200,-2e200,3e200],[1e-200,2e-200,-3e-200],
         [0.,0.,1.],[0.,0.,-1.],[-1.,0.,0.],[0.,-1.,0.],[0.,0.,0.],[0.1,-0.3,0.7]]
def coordinates(v,rep):
    if rep=='RECTANGULAR':return v
    if rep=='RA_DEC':
        r,a,b=s.recrad(v);return [a,b,r]
    if rep=='LATITUDINAL':
        r,a,b=s.reclat(v);return [a,b,r]
    if rep=='CYLINDRICAL':
        r,a,b=s.reccyl(v);return [a,b,r]
    if rep=='SPHERICAL':
        r,b,a=s.recsph(v);return [a,b,r]
    flags=rep.split('_')[-1];r,a,b=s.recazl(v,flags[0]=='T',flags[1]=='T');return [a,b,r]

def main():
    parser=argparse.ArgumentParser();parser.add_argument('--out',default='tests/golden/pointing_coordinates_exact.csv');args=parser.parse_args()
    out=Path(args.out);out.parent.mkdir(parents=True,exist_ok=True)
    with out.open('w',newline='\n') as h:
        h.write('# CSPICE N0067 recrad/reclat/reccyl/recsph/recazl; angles radians; exact doubles\n')
        h.write('representation,magnitude,x,y,z,output_x,output_y,output_z,c1,c2,c3\n')
        count=0
        for v in VECTORS:
            for magnitude in ['UNIT','PRESERVE_ORIGINAL']:
                output=list(s.vhat(v)) if magnitude=='UNIT' else v
                for rep in REPRESENTATIONS:
                    h.write(','.join(map(str,[rep,magnitude,*v,*output,*coordinates(output,rep)]))+'\n');count+=1
    print(f'wrote {out}: {count} independent CSPICE rows')
if __name__=='__main__':main()
