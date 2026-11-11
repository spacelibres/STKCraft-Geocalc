"""Strict full-column NASA audit of the new explicit targetFrame CLI paths."""
import argparse,json
from pathlib import Path
import wgc2_geometry_compare as c

def main():
 p=argparse.ArgumentParser();p.add_argument('--proxy');p.add_argument('--replay',type=Path);p.add_argument('--trace',type=Path);p.add_argument('--report',type=Path,required=True);a=p.parse_args()
 c.QUICK=True;c.EPOCHS=[0.];c.STRICT=True;c.WGC_OUTPUT_EXACT=True
 c.ILLUMINATION_CASES=[('MOON',301,'EARTH',399,'MOON_ME',(10.,20.,0.))];c.INTERCEPT_CASES=[('MOON',301,'EARTH',399,'MOON_ME')]
 w=c.Wgc(False,60,curl=True,proxy=a.proxy,replay=a.replay,trace=a.trace)
 spk='target/kernels/de442.bsp';pck='target/kernels/pck00011.tpc,target/kernels/mars_iau2000_v1.tpc,target/kernels/gm_de431.tpc,target/kernels/moon_pa_de440_200625.bpc'
 records=[]
 for name,runner in [('Illumination',lambda:c.run_illumination(w,spk,pck)),('Intercept',lambda:c.run_surface_intercept(w,spk,pck)),('Tangent',lambda:c.run_tangent(w,spk,pck,'MOON_ME'))]:
  c.STRICT_FAILURES=[];record={'family':name,'failures':c.STRICT_FAILURES}
  try:record['result']=runner()
  except Exception as e:c.STRICT_FAILURES.append(str(e))
  record['passed']=not c.STRICT_FAILURES;records.append(record);print(name,record,flush=True)
 c.STRICT_FAILURES=None
 result={'precision_claim':'exact WGC output serialization equality; independent native bit parity tested separately','target_frame':'MOON_ME','spk':spk,'groups':records,'passed':all(r['passed']for r in records)}
 a.report.write_text(json.dumps(result,indent=2,allow_nan=False)+'\n');return 0 if result['passed']else 1
if __name__=='__main__':raise SystemExit(main())
