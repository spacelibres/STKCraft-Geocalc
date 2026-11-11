"""Read-only NASA WebGeocalc TWO_DIRECTIONS contract probes."""
import json,subprocess,time
from pathlib import Path
BASE='https://wgc2.jpl.nasa.gov:8443/webgeocalc/api';out=Path('docs/review-2026-11-11/two-directions-live.json');records=[]
def req(method,path,payload=None):
 args=['curl.exe','--silent','--show-error','--max-time','30','--proxy','http://127.0.0.1:52345','-X',method,'-H','Content-Type: application/json',BASE+path]
 if payload is not None:args+=['--data-binary','@-']
 r=subprocess.run(args,input=json.dumps(payload) if payload is not None else None,text=True,capture_output=True,check=True)
 response=json.loads(r.stdout);records.append({'method':method,'path':path,'payload':payload,'response':response});out.write_text(json.dumps(records,indent=2));return response
def calc(label,a,b):
 payload={'calculationType':'ANGULAR_SEPARATION','specType':'TWO_DIRECTIONS','kernels':[{'type':'KERNEL_SET','id':1}],'timeSystem':'TDB','timeFormat':'SECONDS_PAST_J2000','times':['500000000'],'outputTimeSystem':'TDB','outputTimeFormat':'SECONDS_PAST_J2000','direction1':a,'direction2':b}
 r=req('POST','/calculation/new',payload)
 if r.get('status')!='OK':print(label,r,flush=True);return
 cid=r['calculationId']
 for _ in range(50):
  if r['result']['phase'] in ['COMPLETE','FAILED']:break
  time.sleep(.25);r=req('GET',f'/calculation/{cid}')
 if r['result']['phase']=='COMPLETE':r=req('GET',f'/calculation/{cid}/results')
 records.append({'label':label,'final':r});out.write_text(json.dumps(records,indent=2));print(label,json.dumps(r),flush=True)
v=lambda x,y,z,corr='NONE',anti=False:{'directionType':'VECTOR','directionVectorType':'VECTOR_IN_REFERENCE_FRAME','directionFrame':'J2000','directionVectorX':x,'directionVectorY':y,'directionVectorZ':z,'observer':'EARTH','aberrationCorrection':corr,'antiVectorFlag':anti}
p=lambda shape='POINT',anti=False:{'directionType':'POSITION','target':'MOON','shape':shape,'observer':'EARTH','aberrationCorrection':'CN+S','antiVectorFlag':anti}
calc('orthogonal',v(1,0,0),v(0,1,0))
calc('velocity',p(),{'directionType':'VELOCITY','target':'EARTH','observer':'MOON','referenceFrame':'IAU_MOON','aberrationCorrection':'XCN+S','antiVectorFlag':True})
calc('sphere',p('SPHERE'),v(1,0,0,'XS',True))
calc('sphere_anti',p('SPHERE',True),v(1,0,0))
calc('boundary_forbidden',{'directionType':'VECTOR','directionVectorType':'INSTRUMENT_FOV_BOUNDARY_VECTORS','directionInstrument':'CASSINI_ISS_NAC','aberrationCorrection':'NONE','antiVectorFlag':False},v(1,0,0))
calc('vector_s_anti',v(1,.3,-.2,'S',True),v(0,1,0))
