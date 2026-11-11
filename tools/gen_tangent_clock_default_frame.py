"""Independent CSPICE ET2LST with actual lunar CIDFRM association.
Point coordinates remain in WGC requested IAU_MOON frame, matching its clock call.
"""
import csv,json
from pathlib import Path
import spiceypy as s
root=Path(__file__).resolve().parents[1]
for name in ['de442s.bsp','pck00011.tpc','moon_pa_de440_200625.bpc','moon_de440_250416.tf','moon_assoc_me.tf']:
 s.furnsh(str(root/'target/kernels'/name))
records=[]
for directory in ['tangent-clock','tangent-clock-transition']:
 records+=json.loads((root/'docs/review-2026-11-11'/directory/'report.json').read_text())
with (root/'tests/golden/tangent_clock_default_frame.csv').open('w',newline='') as handle:
 w=csv.writer(handle);w.writerow(['et','x','y','z','clock','nasa_clock'])
 for rec in records:
  req=rec['request'];et=float(req['times'][0]);corr=req['aberrationCorrection'];locus=req['correctionLocus'].replace('_',' ')
  direction=s.spkpos('301',et,'J2000',corr,'399')[0]
  _,_,_,point,epoch,_=s.tangpt('ELLIPSOID','301',et,'IAU_MOON',corr,locus,'399','J2000',direction)
  clock=s.et2lst(epoch,301,s.reclat(point)[1],'PLANETOGRAPHIC')[3]
  ids=[c['outputID'] for c in rec['result']['columns']];nasa=dict(zip(ids,rec['result']['rows'][0]))['LTST']
  assert clock==nasa,(req,clock,nasa)
  w.writerow([repr(epoch),*[repr(float(x)) for x in point],clock,nasa])
print('24 native ET2LST clocks exactly match NASA')
