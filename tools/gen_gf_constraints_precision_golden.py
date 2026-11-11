"""Independent CSPICE native GF composition and WN postprocessing fixtures."""
from pathlib import Path
import csv, math, random
import spiceypy as s
root=Path(__file__).resolve().parents[1]
s.furnsh(str(root/'target/kernels/de442s.bsp'));s.furnsh(str(root/'target/kernels/pck00011.tpc'))
CORR=['NONE','LT','LT+S','CN','CN+S','XLT','XLT+S','XCN','XCN+S']
def cell(v):
 c=s.cell_double(4000)
 for a,b in v:s.wninsd(a,b,c)
 return c
def pairs(c):return [s.wnfetd(c,i) for i in range(s.wncard(c))]
def packed(c):return ';'.join(repr(a)+':'+repr(b) for a,b in pairs(c))
def serialize(v):return packed(cell(v))
def comp(w,cf):return s.wndifd(cf,w)
def normalize(v,origin,period):
 if origin<=v<origin+period or (origin<0 and v==origin+period):return v
 result=origin+(v-origin)%period
 return origin+period if origin<0 and result==origin and v>origin else result

def run_range(native,cf,a,b,origin=None,period=None):
 if a==b:return native('=',a,cf)
 if origin is None:
  a,b=min(a,b),max(a,b);return native('<',b,native('>',a,cf))
 aa,bb=normalize(a,origin,period),normalize(b,origin,period)
 wrap=aa>bb
 if wrap:a,b,aa,bb=b,a,bb,aa
 w=cf if aa==origin else native('>',a,cf)
 w=w if bb==origin+period else native('<',b,w)
 return comp(w,cf) if wrap else w

rows=[]
for corr in CORR:
 for cfv in [[(0.,4e6)],[(0.,1.3e6),(2e6,4e6)]]:
  cf=cell(cfv);native=lambda op,ref,w:s.gfdist('MOON',corr,'EARTH',op,ref,0.,3600.,1000,w)
  for a,b in [(380000.,400000.),(400000.,380000.),(400000.,400000.),(0.,1e9)]:
   rows.append(['DISTANCE',corr,'','RANGE',a,b,'','',serialize(cfv),3600.,packed(run_range(native,cf,a,b))])
  for op,r in [('=',400000.),('LOCMIN',0.),('ABSMAX',0.)]:rows.append(['DISTANCE',corr,'',op,r,'','','',serialize(cfv),3600.,packed(native(op,r,cf))])
 for system,coord,origin in [('LATITUDINAL','LONGITUDE',-180.),('CYLINDRICAL','LONGITUDE',0.),('RA/DEC','RIGHT ASCENSION',0.)]:
  # Keep the WGC degree inputs through range scheduling; convert only at the
  # CSPICE call. 10 and 370 degrees have identical phases, whereas conversion
  # to radians first can reverse their normalized order by one rounding bit.
  cf=cell([(0.,86400.)]);native=lambda op,ref,w:s.gfposc('MOON','IAU_EARTH',corr,'EARTH',system,coord,op,ref*s.rpd(),0.,60.,1000,w)
  vals=[(170,-170),(-170,170),(350,10),(10,10),(-180,180),(180,-180),(0,360),(10,370),(-720,720),(300,60),(540,10),(-540,10),(180,10),(-180,10)]
  for a,b in vals:
   rows.append(['COORDINATE',corr,system,'RANGE',a,b,origin,360.,serialize([(0.,86400.)]),60.,packed(run_range(native,cf,a,b,origin,360.))])
with open(root/'tests/golden/gf_constraints_precision.csv','w',newline='') as f:
 w=csv.writer(f);w.writerow(['family','correction','system','relation','lower','upper','origin','period','confinement','step','expected']);w.writerows(rows)

rng=random.Random(610346)
rows=[]
for i in range(240):
 cf=cell([(5.,5.),(20.,20.),(30.,40.)] if i%13==0 else [(0.,100.),(200.,300.)])
 raw=[] if i%12==0 else [(rng.uniform(-100,400),rng.uniform(-100,400)) for _ in range(5)]
 raw=[(min(a,b),max(a,b)) for a,b in raw]
 result=cell(raw);original_result=packed(result)
 complement=i%2;mode=['NONE','EXPAND','CONTRACT'][i%3];adjust=rng.choice([-100.,-10.,0.,0.125,1.,10.,50.,100.]);au=['SECONDS','MINUTES','HOURS','DAYS'][i%4]
 filt=i%4==0;threshold=rng.choice([-100.,-1.,0.,0.25,1.,10.,100.]);fu=['SECONDS','MINUTES','HOURS','DAYS'][(i//4)%4];outunit=['SECONDS','MINUTES','HOURS','DAYS'][(i//16)%4]
 if complement:result=s.wndifd(cf,result)
 seconds=s.convrt(adjust,au,'SECONDS')
 if mode=='EXPAND':s.wnexpd(seconds,seconds,result)
 if mode=='CONTRACT':s.wncond(seconds,seconds,result)
 if filt:s.wnfltd(s.convrt(threshold,fu,'SECONDS'),result)
 durations=';'.join(repr(s.convrt(b-a,'SECONDS',outunit)) for a,b in pairs(result))
 rows.append([original_result,packed(cf),complement,mode,adjust,au,int(filt),threshold,fu,outunit,packed(result),durations])
with open(root/'tests/golden/gf_window_processing_precision.csv','w',newline='') as f:
 w=csv.writer(f);w.writerow(['input','confinement','complement','adjustment','amount','amount_units','filter','threshold','threshold_units','duration_units','expected','durations']);w.writerows(rows)
print('generated',504,'native GF combinations and',len(rows),'source WN combinations')
