import subprocess,sys,json
cases={'none':[[],[],[]],'names_only':[['A'],[],[]],'missing_body':[['A'],[1],[]],'wrong_lengths':[['A','B'],[1],[301]],'float_codes':[['A'],[1.5],[301]],'numeric_name':[['100'],[42],[301]],'dup_name':[['A','A'],[1,2],[301,301]],'dup_code':[['A','B'],[1,1],[301,301]]}
child=r'''import sys,json,spiceypy as s
names,codes,bodies=json.loads(sys.argv[1]);s.kclear()
if names:s.pcpool('NAIF_SURFACE_NAME',names)
if codes:s.pdpool('NAIF_SURFACE_CODE',codes)
if bodies:s.pipool('NAIF_SURFACE_BODY',bodies)
with s.no_found_check():
 for q in ['A','B','100','1','2']:
  try:print(q,s.srfscc(q,301),flush=True)
  except Exception as e:print(q,getattr(e,'short',type(e).__name__),flush=True)
'''
records=[]
for name,case in cases.items():
 p=subprocess.run([sys.executable,'-c',child,json.dumps(case)],capture_output=True,text=True,timeout=10)
 records.append({'case':name,'inputs':case,'exit_code':p.returncode,'stdout':p.stdout,'stderr':p.stderr})
print(json.dumps({'method':'separate native subprocess per kernel-pool configuration','records':records},indent=2))
