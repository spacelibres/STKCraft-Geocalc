"""NASA GF RANGE and window-postprocessing contract probes."""
import json,subprocess,time
from pathlib import Path
BASE='https://wgc2.jpl.nasa.gov:8443/webgeocalc/api';out=Path('docs/review-2026-11-11/gf-constraints-live.json');records=[]
def request(method,path,payload=None):
 cmd=['curl.exe','--silent','--show-error','--max-time','30','--proxy','http://127.0.0.1:52345','-X',method,'-H','Content-Type: application/json',BASE+path]
 if payload is not None:cmd+=['--data-binary','@-']
 r=subprocess.run(cmd,input=json.dumps(payload) if payload is not None else None,text=True,capture_output=True,check=True)
 response=json.loads(r.stdout);records.append({'method':method,'path':path,'payload':payload,'response':response});out.write_text(json.dumps(records,indent=2));return response
def calc(label,payload):
 r=request('POST','/calculation/new',payload)
 if r.get('status')=='OK':
  cid=r['calculationId']
  for _ in range(60):
   if r['result']['phase'] in ['COMPLETE','FAILED']:break
   time.sleep(.25);r=request('GET',f'/calculation/{cid}')
  if r['result']['phase']=='COMPLETE':r=request('GET',f'/calculation/{cid}/results')
 records.append({'label':label,'payload':payload,'final':r});out.write_text(json.dumps(records,indent=2));print(label,json.dumps(r),flush=True)
common={'calculationType':'GF_DISTANCE_SEARCH','kernels':[{'type':'KERNEL_SET','id':1}],'timeSystem':'TDB','timeFormat':'SECONDS_PAST_J2000','intervals':[{'startTime':'0','endTime':'4000000'}],'timeStep':3600,'timeStepUnits':'SECONDS','outputTimeSystem':'TDB','outputTimeFormat':'SECONDS_PAST_J2000','outputDurationUnits':'SECONDS','target':'MOON','observer':'EARTH','aberrationCorrection':'NONE','shouldComplementWindow':False,'intervalAdjustment':'NO_ADJUSTMENT','intervalFiltering':'NO_FILTERING'}
for label,condition in [('distance_range',{'relationalCondition':'RANGE','referenceValue':380000.,'upperLimit':400000.}),('distance_reverse',{'relationalCondition':'RANGE','referenceValue':400000.,'upperLimit':380000.}),('distance_equal_range',{'relationalCondition':'RANGE','referenceValue':400000.,'upperLimit':400000.}),('distance_equal',{'relationalCondition':'=','referenceValue':400000.})]:calc(label,{**common,'condition':condition})
for label,lo,hi in [('longitude_wrap',170.,-170.),('longitude_normal',-170.,170.),('longitude_350_10',350.,10.)]:calc(label,{**common,'calculationType':'GF_COORDINATE_SEARCH','referenceFrame':'IAU_EARTH','intervals':[{'startTime':'0','endTime':'86400'}],'timeStep':60,'condition':{'coordinateSystem':'LATITUDINAL','coordinate':'LONGITUDE','relationalCondition':'RANGE','referenceValue':lo,'upperLimit':hi}})
cf=[{'startTime':'0','endTime':'100'},{'startTime':'200','endTime':'300'}]
empty={**common,'condition':{'relationalCondition':'<','referenceValue':1000.},'intervals':cf,'timeStep':10,'shouldComplementWindow':True}
for label,extra in [('complement',{}),('complement_expand',{'intervalAdjustment':'EXPAND_INTERVALS','intervalAdjustmentAmount':10,'intervalAdjustmentUnits':'SECONDS'}),('complement_expand_filter_equal',{'intervalAdjustment':'EXPAND_INTERVALS','intervalAdjustmentAmount':10,'intervalAdjustmentUnits':'SECONDS','intervalFiltering':'FILTER_INTERVALS','intervalFilteringThreshold':2,'intervalFilteringThresholdUnits':'MINUTES'}),('complement_filter_equal',{'intervalFiltering':'FILTER_INTERVALS','intervalFilteringThreshold':100,'intervalFilteringThresholdUnits':'SECONDS'}),('complement_contract',{'intervalAdjustment':'CONTRACT_INTERVALS','intervalAdjustmentAmount':1,'intervalAdjustmentUnits':'MINUTES'}),('duration_hours',{'outputDurationUnits':'HOURS'})]:calc(label,{**empty,**extra})
