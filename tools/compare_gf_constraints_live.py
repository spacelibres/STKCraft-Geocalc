"""Keep raw NASA transport differences separate from exact source regressions."""
from pathlib import Path
from decimal import Decimal
import csv,json,math
import spiceypy as s
root=Path(__file__).resolve().parents[1]
fixtures=list(csv.DictReader(open(root/'tests/golden/gf_constraints_precision.csv')))
s.furnsh(str(root/'target/kernels/de442s.bsp'))
def pairs(encoded):return [tuple(map(float,p.split(':'))) for p in encoded.split(';') if p]
def cell(p):
 c=s.cell_double(4000)
 for a,b in p:s.wninsd(a,b,c)
 return c

def expected(payload):
 condition=payload['condition'];lo=condition['referenceValue'];hi=condition.get('upperLimit');kind=payload['calculationType'];sys=condition.get('coordinateSystem','')
 if kind=='GF_COORDINATE_SEARCH':
  candidates=[r for r in fixtures if r['family']=='COORDINATE' and r['correction']=='NONE' and r['system']==sys and float(r['lower'])==lo and float(r['upper'])==hi]
  if not candidates:return None
  return [[a,b,s.convrt(b-a,'SECONDS',payload['outputDurationUnits'])] for a,b in pairs(candidates[0]['expected'])]
 cf=cell([(float(w['startTime']),float(w['endTime'])) for w in payload['intervals']])
 if condition['relationalCondition']=='RANGE':
  candidates=[r for r in fixtures if r['family']=='DISTANCE' and r['correction']=='NONE' and r['relation']=='RANGE' and float(r['lower'])==lo and float(r['upper'])==hi and r['confinement']=='0.0:4000000.0']
  if not candidates:return None
  w=cell(pairs(candidates[0]['expected']))
 else:w=s.gfdist('MOON','NONE','EARTH',condition['relationalCondition'],lo,0.,payload['timeStep'],1000,cf)
 if payload.get('shouldComplementWindow'):w=s.wndifd(cf,w)
 mode=payload.get('intervalAdjustment');amount=s.convrt(payload.get('intervalAdjustmentAmount',0),payload.get('intervalAdjustmentUnits','SECONDS'),'SECONDS')
 if mode=='EXPAND_INTERVALS':s.wnexpd(amount,amount,w)
 if mode=='CONTRACT_INTERVALS':s.wncond(amount,amount,w)
 if payload.get('intervalFiltering')=='FILTER_INTERVALS':s.wnfltd(s.convrt(payload['intervalFilteringThreshold'],payload['intervalFilteringThresholdUnits'],'SECONDS'),w)
 return [[a,b,s.convrt(b-a,'SECONDS',payload['outputDurationUnits'])] for a,b in [s.wnfetd(w,i) for i in range(s.wncard(w))]]
rows=[]
for file in ['gf-constraints-live.json','gf-constraints-edge-live.json','gf-constraints-domains-live.json','gf-constraints-postprocess-edge-live.json','gf-constraints-turns-live.json']:
 records=json.loads((root/'docs/review-2026-11-11'/file).read_text(),parse_float=Decimal)
 for r in records:
  if 'label' not in r:continue
  if r['final']['status']!='OK':
   rows.append({'label':r['label'],'status':'NASA_REJECTED','error':r['final'].get('error')});continue
  # Convert only query parameters to floats for the independent numeric oracle.
  payload=json.loads(json.dumps(r['payload'],default=float))
  ref=expected(payload)
  if ref is None:raise RuntimeError(r['label'])
  nasa=r['final']['rows'];raw=[[Decimal(str(v)) for v in row] for row in ref]
  observed=[[Decimal(str(v)) for v in row] for row in nasa]
  rendered=[[Decimal(f'{a:.6f}'),Decimal(f'{b:.6f}'),Decimal(f'{duration:.8f}')] for a,b,duration in ref]
  rows.append({'label':r['label'],'status':'EXACT' if raw==observed else 'STRICT_TRANSPORT_MISMATCH','raw_source':[[str(v) for v in row] for row in raw],'nasa_transport':[[str(v) for v in row] for row in observed],'transport_format_diagnostic_matches':rendered==observed})
report={'acceptance':'raw exact decimal equality; rendering diagnostic does not change strict acceptance','complete_webgeocalc_parity':False,'cases':len(rows),'strict_transport_mismatches':sum(r['status']=='STRICT_TRANSPORT_MISMATCH' for r in rows),'rendering_diagnostic_mismatches':sum(not r.get('transport_format_diagnostic_matches',True) for r in rows),'rows':rows}
(root/'docs/review-2026-11-11/gf-constraints-live-comparison.json').write_text(json.dumps(report,indent=2,default=str))
print({k:v for k,v in report.items() if k!='rows'})
