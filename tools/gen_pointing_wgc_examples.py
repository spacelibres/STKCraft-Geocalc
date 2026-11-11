"""Capture WGC presentation evidence for pointing frame epochs and anti-vector order.

NASA API is only a remote reference transport. No local HTTP service is started.
These examples resolve semantics; they do not establish high-precision bit parity.
"""
import argparse
import json
import subprocess
import time
from pathlib import Path
BASE='https://wgc2.jpl.nasa.gov:8443/webgeocalc/api'
def main():
 p=argparse.ArgumentParser();p.add_argument('--proxy');p.add_argument('--out',default='tests/golden/pointing_wgc_examples.json');a=p.parse_args()
 def request(method,path,payload=None):
  command=['curl.exe','--silent','--show-error','--fail','--max-time','30','-X',method,'-H','Content-Type: application/json',BASE+path]
  if a.proxy:command+=['--proxy',a.proxy]
  if payload is not None:command+=['--data-binary','@-']
  return json.loads(subprocess.run(command,input=json.dumps(payload) if payload is not None else None,text=True,capture_output=True,check=True).stdout)
 examples=[
  ('velocity_both_frames_at_input_frame_epoch',{'directionType':'VELOCITY','target':'SUN','observer':'EARTH','referenceFrame':'IAU_MOON','aberrationCorrection':'CN+S','antiVectorFlag':False},'IAU_EARTH'),
  ('position_uses_output_frame_center_epoch',{'directionType':'POSITION','target':'SUN','observer':'EARTH','aberrationCorrection':'CN+S','antiVectorFlag':False},'IAU_MOON'),
  ('anti_vector_applied_after_stellar_aberration',{'directionType':'VECTOR','observer':'EARTH','directionVectorType':'REFERENCE_FRAME_AXIS','directionFrame':'J2000','directionFrameAxis':'X','aberrationCorrection':'S','antiVectorFlag':True},'J2000')]
 records=[]
 for label,direction,frame in examples:
  payload={'calculationType':'POINTING_DIRECTION','kernels':[{'type':'KERNEL_SET','id':1}],'timeSystem':'TDB','timeFormat':'SECONDS_PAST_J2000','times':['500000000'],'outputTimeSystem':'TDB','outputTimeFormat':'SECONDS_PAST_J2000','direction':direction,'referenceFrame':frame,'coordinateRepresentation':'RECTANGULAR','vectorMagnitude':'PRESERVE_ORIGINAL'}
  created=request('POST','/calculation/new',payload);cid=created['calculationId'];deadline=time.monotonic()+60
  while created['result']['phase']!='COMPLETE':
   if created['result']['phase']=='FAILED':raise RuntimeError(created)
   if time.monotonic()>=deadline:raise TimeoutError(cid)
   time.sleep(.2);created=request('GET','/calculation/'+cid)
  result=request('GET','/calculation/'+cid+'/results')
  if result.get('status')!='OK' or len(result.get('rows',[]))!=1:raise RuntimeError(result)
  records.append({'label':label,'request':payload,'response':result})
  print(label,result['rows'])
 out=Path(a.out);out.parent.mkdir(parents=True,exist_ok=True);out.write_text(json.dumps({'reference':BASE,'scope':'WGC column presentation only; independent CSPICE doubles are tested separately','examples':records},indent=2)+'\n')
if __name__=='__main__':main()
