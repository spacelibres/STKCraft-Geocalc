"""Audit all11 official GF families against independent CSPICE bits and raw NASA.
Raw transport equality is the default. --output-exact is a separate, explicit
WGC display-serialization comparison; source endpoint bit parity is always required.
"""
import argparse,hashlib,json,os,subprocess
from pathlib import Path
from wgc_gf_compare import c,compare_gf_rows,parse_gf_output
from wgc_gf_cases import ROOT,PCK,FK
OFFICIAL={'GF_COORDINATE_SEARCH','GF_ANGULAR_SEPARATION_SEARCH','GF_DISTANCE_SEARCH','GF_SUB_POINT_SEARCH','GF_OCCULTATION_SEARCH','GF_SURFACE_INTERCEPT_POINT_SEARCH','GF_TARGET_IN_INSTRUMENT_FOV_SEARCH','GF_RAY_IN_FOV_SEARCH','GF_RANGE_RATE_SEARCH','GF_PHASE_ANGLE_SEARCH','GF_ILLUMINATION_ANGLES_SEARCH'}

def main():
 p=argparse.ArgumentParser();p.add_argument('--directory',type=Path,default=ROOT/'docs/review-2026-11-11/gf-all-families');p.add_argument('--report',type=Path,required=True);p.add_argument('--output-exact',action='store_true');a=p.parse_args()
 cases=json.loads((a.directory/'cases.json').read_text(encoding='utf-8'));ids=[x['id']for x in cases]
 if not ids or len(set(ids))!=len(ids):raise RuntimeError('nonzero unique GF cases required')
 build=subprocess.run(['cargo','build','--quiet','--bin','wgc_gf'],cwd=ROOT,capture_output=True,text=True,encoding='utf-8')
 if build.returncode:raise RuntimeError(build.stderr)
 base={'spk':'target/kernels/de442.bsp','pck':PCK,'fk':FK};groups={}
 for case in cases:
  resources=case.get('resources',base);key=json.dumps(resources,sort_keys=True);groups.setdefault(key,[]).append(case)
 actual={};resources_used=[]
 for index,(encoded,group)in enumerate(groups.items()):
  resources=json.loads(encoded);command=[str(ROOT/'target/debug/wgc_gf.exe'),resources['pck'],resources['fk']]
  if 'ik'in resources:command += [resources['ik'],str(resources['instrument'])]
  env={**os.environ,'STKCRAFT_SPK':resources['spk']}
  for key,variable in [('ck','STKCRAFT_CK'),('sclk','STKCRAFT_SCLK'),('lsk','STKCRAFT_LSK')]:
   env.pop(variable,None)
   if key in resources:env[variable]=resources[key]
  query='\n'.join(x['local']for x in group)+'\n';process=subprocess.run(command,cwd=ROOT,input=query,text=True,encoding='utf-8',capture_output=True,env=env,timeout=240)
  (a.directory/f'local-queries-{index}.csv').write_text(query,encoding='utf-8');(a.directory/f'local-output-{index}.txt').write_text(process.stdout,encoding='utf-8')
  if process.returncode:
   for case in group:actual[case['id']]={'error':process.stderr}
  else:
   try:actual.update(parse_gf_output(process.stdout,[x['id']for x in group]))
   except Exception as error:
    for case in group:actual[case['id']]={'error':str(error)}
  resources_used.append(resources)
 rows=[]
 for case in cases:
  record={'id':case['id'],'family':case['payload']['calculationType'],'failures':[]};local=actual[case['id']]
  try:
   if 'error'in local:raise RuntimeError(local['error'])
   c.checked_count(case['id']+' independent native',len(case['native']),len(local['rows']))
   native_failures=[];native_checked=0
   for i,(ours,theirs)in enumerate(zip(local['rows'],case['native'])):
    if len(ours)!=3 or len(theirs)!=3:raise RuntimeError('native column count mismatch')
    for j,(value,expected)in enumerate(zip(ours,theirs)):
     try:c.compare_cspice_f64(float(value),expected,f'{case["id"]}/{i}/{j}')
     except RuntimeError as error:native_failures.append(str(error))
     native_checked+=1
   record.update({'native_bit_exact':not native_failures,'native_compared_components':native_checked,'native_failures':native_failures,'local_native_rows':local['rows']})
   reference=c.Wgc(False,30,replay=a.directory/(case['id']+'.jsonl')).run(case['payload'])
   record['raw_transport']=compare_gf_rows(reference,local['rows'],case['id'])
   record['output_serialization']=compare_gf_rows(reference,local['rows'],case['id'],output_exact=True)
   chosen=record['output_serialization']if a.output_exact else record['raw_transport']
   record['failures'].extend(native_failures);record['failures'].extend(chosen['failures'])
  except Exception as error:record['failures'].append(str(error))
  record['passed']=not record['failures'];rows.append(record);print(case['id'],'PASS'if record['passed']else'FAIL',record['failures'][:2],flush=True)
 hashes={}
 for resources in resources_used:
  paths=[*resources['spk'].split(','),*resources['pck'].split(','),*resources['fk'].split(',')]
  paths += [resources[x]for x in ['ik','ck','lsk']if x in resources]
  if 'sclk'in resources:paths += [part.split('|',1)[1]for part in resources['sclk'].split(',')]
  for path in paths:
   if path not in hashes:
    raw=(ROOT/path).read_bytes();hashes[path]={'bytes':len(raw),'sha256':hashlib.sha256(raw).hexdigest()}
 covered={r['family']for r in rows if r['passed']};missing=sorted(OFFICIAL-covered)
 report={'mode':'WGC_OUTPUT_SERIALIZATION_EXACT'if a.output_exact else'RAW_TRANSPORT_EXACT','native_endpoint_acceptance':'finite IEEE754 binary64 equality; no comparison tolerance','NASA_endpoint_serialization':'explicit DATE six decimals and numeric eight fixed/mantissa decimals, independently probed','official_family_count':11,'covered_families':sorted(covered),'missing_or_failing_families':missing,'cases':rows,'resources':resources_used,'kernel_hashes':hashes,'passed':not missing and all(r['passed']for r in rows)}
 a.report.write_text(json.dumps(report,indent=2,allow_nan=False)+'\n',encoding='utf-8');print('OVERALL',report['passed'],'families',len(covered),'/11');return 0 if report['passed']else 1
if __name__=='__main__':raise SystemExit(main())
