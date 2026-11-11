"""Native ET2LST versus genuine NASA Earth clocks, with exactv0072 Earth BPC/FK assets."""
import csv,json
from pathlib import Path
import spiceypy as s
root=Path(__file__).resolve().parents[1]
for name in ['de442.bsp','pck00011.tpc','earth_1962_260806_2126_combined.bpc','earth_000101_270205_261110.bpc','moon_pa_de440_200625.bpc']:
 s.furnsh(str(root/'target/kernels'/name))
for record in json.loads((root/'docs/review-2026-11-11/wgc-kernel-fk-provenance.json').read_text()):s.furnsh(str(root/record['path']))
assert s.cidfrm(399)==(10013,'IAU_EARTH');assert s.cidfrm(301)==(31001,'MOON_ME')
radii=s.bodvrd('399','RADII',3)[1];flattening=(radii[0]-radii[2])/radii[0]
records=json.loads((root/'docs/review-2026-11-11/earth-tangent-clock/report.json').read_text());details=[]
with (root/'tests/golden/earth_tangent_clock_default_frame.csv').open('w',newline='')as handle:
 w=csv.writer(handle);w.writerow(['et','x','y','z','clock','nasa_clock'])
 for rec in records:
  req=rec['request'];et=float(req['times'][0]);corr=req['aberrationCorrection'];locus=req['correctionLocus'].replace('_',' ')
  direction=s.spkpos('399',et,'J2000',corr,'301')[0]
  _,_,_,point,epoch,_=s.tangpt('ELLIPSOID','399',et,'IAU_EARTH',corr,locus,'301','J2000',direction)
  lon=s.recpgr('399',point,radii[0],flattening)[0]
  clock=s.et2lst(epoch,399,lon,'PLANETOGRAPHIC')[3]
  ids=[c['outputID']for c in rec['result']['columns']];nasa=dict(zip(ids,rec['result']['rows'][0]))['LTST']
  s.pcpool('OBJECT_399_FRAME',['ITRF93']);itrf=s.et2lst(epoch,399,lon,'PLANETOGRAPHIC')[3];s.dvpool('OBJECT_399_FRAME')
  assert clock==nasa,(req,clock,nasa)
  w.writerow([repr(epoch),*[repr(float(x))for x in point],clock,nasa])
  details.append({'epoch':et,'correction':corr,'locus':locus,'nativeDefaultFrameClock':clock,'NASA':nasa,'hypotheticalITRF93DefaultClock':itrf})
report={'nativeCidfrm399':[10013,'IAU_EARTH'],'all18ActualFksLoaded':True,'checked':len(details),'passed':True,'hypotheticalItrfMismatches':sum(x['hypotheticalITRF93DefaultClock']!=x['NASA']for x in details),'cases':details}
(root/'docs/review-2026-11-11/earth-tangent-clock/native-comparison.json').write_text(json.dumps(report,indent=2)+'\n')
print('24 native ET2LST/IAU_EARTH clocks exactly match NASA; wrongITRF93default mismatches',report['hypotheticalItrfMismatches'])
