"""Exact GF endpoint/duration verification, separate from WGC serialization.
Official GF tables intentionally repeat DATE for the start/stop columns.
No tolerance affects acceptance; raw comparison is the default.
"""
from pathlib import Path
import importlib.util
spec=importlib.util.spec_from_file_location('gf_geometry_compare',Path(__file__).with_name('wgc2_geometry_compare.py'))
c=importlib.util.module_from_spec(spec);spec.loader.exec_module(c)

def compare_gf_rows(reference,actual,label,*,output_exact=False):
 columns=reference.get('columns',[])
 if [x.get('outputID')for x in columns]!=['DATE','DATE','DURATION'] or [x.get('type')for x in columns]!=['DATE','DATE','NUMBER']:
  raise RuntimeError(f'{label}: GF column schema mismatch')
 if not columns[0].get('name','').startswith('Start Time') or not columns[1].get('name','').startswith('Stop Time') or not columns[2].get('name','').startswith('Duration'):
  raise RuntimeError(f'{label}: GF column schema order mismatch')
 expected=reference.get('rows',[])
 c.checked_count(label+' GF',len(expected),len(actual))
 failures=[];differences=[]
 for dataset,rows in [('reference',expected),('local',actual)]:
  previous=None
  for index,row in enumerate(rows):
   if len(row)!=3:raise RuntimeError(f'{label}: {dataset} column count mismatch')
   values=[c.finite_decimal(v,f'{label}/{dataset}/{index}')for v in row]
   if values[0]>values[1] or values[2]<0 or (previous is not None and previous>values[0]):raise RuntimeError(f'{label}: {dataset} interval order invalid')
   previous=values[1]
 for i,(ours,theirs)in enumerate(zip(actual,expected)):
  for j,(value,ref)in enumerate(zip(ours,theirs)):
   raw=c.finite_decimal(value,label);ref=c.finite_decimal(ref,label);difference=abs(raw-ref)
   compared=c.wgc_output_value(raw,'DATE'if j<2 else 'DURATION')if output_exact else raw
   if compared!=ref:failures.append(f'{label}/row{i}/col{j}: actual={raw} reference={ref} raw_difference={difference} compared={compared}')
   differences.append(str(difference))
 return {'passed':not failures,'windows':len(expected),'compared_cells':3*len(expected),'raw_differences':differences,'failures':failures,'output_serialization_equality':output_exact}

def parse_gf_output(text,expected_ids):
 if not expected_ids or len(set(expected_ids))!=len(expected_ids):raise RuntimeError('nonzero unique GF query IDs required')
 output={};active=None;declared=0;rows=[];next_query=0
 for line in text.splitlines():
  if not line.strip():continue
  f=line.strip().split(',')
  if f[0]in ('BEGIN','ERROR'):
   if active is not None or next_query>=len(expected_ids)or len(f)<3 or f[1]!=expected_ids[next_query]:raise RuntimeError('GF query identity/order mismatch')
   if f[0]=='ERROR':output[f[1]]={'error':','.join(f[2:])};next_query+=1;continue
   if len(f)!=3:raise RuntimeError('GF BEGIN schema invalid')
   declared=int(f[2]);active=f[1];rows=[]
   if declared<0:raise RuntimeError('GF negative row count')
  elif f[0]=='ROW':
   if active is None or len(f)!=6 or f[1]!=active or int(f[2])!=len(rows):raise RuntimeError('GF ROW identity/index/schema mismatch')
   rows.append(f[3:])
  elif f[0]=='END':
   if active is None or len(f)!=2 or f[1]!=active:raise RuntimeError('GF END identity mismatch')
   if len(rows)!=declared:raise RuntimeError('GF row count mismatch')
   output[active]={'rows':rows};active=None;next_query+=1
  else:raise RuntimeError(f'unknown GF output record {line!r}')
 if active is not None or next_query!=len(expected_ids):raise RuntimeError('GF query output truncated/missing')
 return output
