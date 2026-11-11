"""Generate strict CSPICE pointing source fixtures; velocity is a rotated direction."""
import argparse
from pathlib import Path
import spiceypy as s

CORRECTIONS=['NONE','LT','LT+S','CN','CN+S','XLT','XLT+S','XCN','XCN+S']
def frame_epoch(frame,et,correction,observer):
    fid=s.namfrm(frame);center,kind,_=s.frinfo(fid)
    if kind==1 or correction=='NONE' or center==observer:return et
    _,lt=s.spkezr(str(center),et,'J2000',correction,str(observer))
    return et+(lt if correction.startswith('X') else -lt)

def main():
    p=argparse.ArgumentParser();p.add_argument('--spk',default='target/kernels/de442s.bsp');p.add_argument('--pck',default='target/kernels/pck00011.tpc');p.add_argument('--out',default='tests/golden/pointing_sources_exact.csv');a=p.parse_args()
    s.furnsh(a.spk);s.furnsh(a.pck)
    out=Path(a.out);out.parent.mkdir(parents=True,exist_ok=True)
    with out.open('w',newline='\n') as h:
        h.write('# CSPICE N0067 spkezp for POSITION; spkezr then pxform at source-frame epoch for VELOCITY\n')
        h.write('kind,target,observer,et,correction,input_frame,output_frame,magnitude,frame_epoch,x,y,z\n')
        count=0
        for et in [-5e8,0.,5e8]:
          for kind in ['POSITION','VELOCITY']:
           for correction in CORRECTIONS:
            pairs = [('J2000','J2000'),('J2000','ECLIPJ2000'),('J2000','IAU_MOON'),('J2000','IAU_EARTH')] if kind=='POSITION' else [('J2000','J2000'),('J2000','ECLIPJ2000'),('ECLIPJ2000','J2000'),('ECLIPJ2000','IAU_EARTH'),('IAU_MOON','J2000'),('IAU_MOON','IAU_EARTH')]
            for input_frame,output_frame in pairs:
             if kind=='POSITION':
                v,_=s.spkezp(10,et,output_frame,correction,399)
                epoch=frame_epoch(output_frame,et,correction,399)
             else:
                state,_=s.spkezr('10',et,input_frame,correction,'399')
                epoch=frame_epoch(input_frame,et,correction,399)
                v=s.mxv(s.pxform(input_frame,output_frame,epoch),state[3:]) if input_frame!=output_frame else state[3:]
             for magnitude in ['PRESERVE_ORIGINAL','UNIT']:
                vector=s.vhat(v) if magnitude=='UNIT' else v
                h.write(','.join(map(str,[kind,10,399,et,correction,input_frame,output_frame,magnitude,epoch,*vector]))+'\n');count+=1
    s.kclear();print(f'wrote {out}: {count} independent CSPICE rows')
if __name__=='__main__':main()
