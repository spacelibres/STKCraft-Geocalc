"""Generate independent CSPICE VECTOR pointing fixtures, including S/XS and anti vectors."""
import argparse
from pathlib import Path
import spiceypy as s
from gen_pointing_sources_exact import frame_epoch
CORRECTIONS=['NONE','LT','CN','XLT','XCN','S','XS']
def main():
 p=argparse.ArgumentParser();p.add_argument('--spk',default='target/kernels/de442s.bsp');p.add_argument('--pck',default='target/kernels/pck00011.tpc');p.add_argument('--out',default='tests/golden/pointing_vectors_exact.csv');a=p.parse_args()
 s.furnsh(a.spk);s.furnsh(a.pck)
 out=Path(a.out);out.parent.mkdir(parents=True,exist_ok=True)
 with out.open('w',newline='\n') as h:
  h.write('# Independent CSPICE pxform,spkezp,stelab/stlabx,vhat; anti applied after aberration per live WGC\n')
  h.write('et,correction,input_frame,output_frame,anti,magnitude,x,y,z,frame_epoch,result_x,result_y,result_z\n');count=0
  for et in [-5e8,0.,5e8]:
   for correction in CORRECTIONS:
    for inf,outf in [('J2000','J2000'),('J2000','ECLIPJ2000'),('IAU_MOON','J2000'),('IAU_MOON','IAU_EARTH')]:
     epoch=frame_epoch(inf,et,'NONE' if correction in ['S','XS'] else correction,399)
     vector=[1.2,-3.4,5.6]
     if correction in ['S','XS']:
      j=s.mxv(s.pxform(inf,'J2000',epoch),vector)
      obs,_=s.spkezr('399',et,'J2000','NONE','0')
      j=s.stlabx(j,obs[3:]) if correction=='XS' else s.stelab(j,obs[3:])
      result=s.mxv(s.pxform('J2000',outf,epoch),j)
     else:result=s.mxv(s.pxform(inf,outf,epoch),vector)
     for anti in [False,True]:
      r=-result if anti else result
      for mag in ['PRESERVE_ORIGINAL','UNIT']:
       v=s.vhat(r) if mag=='UNIT' else r
       h.write(','.join(map(str,[et,correction,inf,outf,str(anti).lower(),mag,*vector,epoch,*v]))+'\n');count+=1
 s.kclear();print(f'wrote {out}: {count} rows')
if __name__=='__main__':main()
