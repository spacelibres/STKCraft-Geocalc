"""Fetch every configured GF request as genuine raw NASA transport."""
import concurrent.futures,json
from pathlib import Path
from wgc_gf_compare import c
OUT=Path('docs/review-2026-11-11/gf-all-families')
records=json.loads((OUT/'cases.json').read_text(encoding='utf-8'))
def fetch(case):
 try:
  result=c.Wgc(False,90,curl=True,proxy='http://127.0.0.1:52345',trace=OUT/(case['id']+'.jsonl')).run(case['payload'])
  return {'id':case['id'],'result':result}
 except Exception as error:return {'id':case['id'],'error':str(error)}
with concurrent.futures.ThreadPoolExecutor(max_workers=2)as pool:results=list(pool.map(fetch,records))
(OUT/'nasa-results.json').write_text(json.dumps(results,indent=2,default=str)+'\n',encoding='utf-8')
for result in results:print(result['id'],result.get('error')or str(len(result['result']['rows']))+' genuine NASA intervals',flush=True)
