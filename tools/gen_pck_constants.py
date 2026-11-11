"""Native text-PCK constants reference epoch/frame and array metadata."""
from pathlib import Path
import csv,itertools
import spiceypy as s
ROOT=Path(__file__).resolve().parents[1]
with (ROOT/'tests/golden/pck_constants.csv').open('w',newline='',encoding='utf-8') as f:
 w=csv.writer(f);w.writerow(['body','reference','epoch','frame','spelling','et','status',*[f'r{i}' for i in range(9)],*[f'd{i}' for i in range(9)]])
 count=0
 for body,reference in [(999,9),(10020,1),(2000001,2000001)]:
  for epoch,frame,spelling,et in itertools.product([2451545.,2461545.25,2455607.694660],[1,13,17,21],['CONSTANTS','CONSTS'],[-987654321.,0.,123456789.]):
   s.kclear()
   for key,v in [('POLE_RA',[30.,-.1,.001]),('POLE_DEC',[70.,.2,-.003]),('PM',[10.,100.,.0002])]:s.pdpool(f'BODY{body}_{key}',v)
   s.pdpool(f'BODY{reference}_{spelling}_JED_EPOCH',[epoch]);s.pdpool(f'BODY{reference}_{spelling}_REF_FRAME',[float(frame)])
   x=s.tisbod('J2000',body,et);w.writerow([body,reference,epoch,frame,spelling,et,'OK',*x[:3,:3].flatten().tolist(),*x[3:,:3].flatten().tolist()]);count+=1
 print(f'{count} native PCK constants cases')
