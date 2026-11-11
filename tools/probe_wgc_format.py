from wgc2_geometry_compare import Wgc,time_block
from pathlib import Path
import json
w=Wgc(False,40,curl=True,proxy='http://127.0.0.1:52345',trace='docs/review-2026-11-11/wgc-format-probes.jsonl')
vectors=[[12345678901.2345,12345678850.,12345678950.],[.000099999999994,.0001,.000100000000001],[9999999999.999,1e10,-12345678850.],[.001953125,.005859375,-.001953125],[1.234567894567e-5,9.876543215e-7,-.0000999999999999]]
vectors += [[1.234567885,1.234567895,-1.234567885],[.0000987654321567,.000123456789567,.0001]]
records=[]
for vector in vectors:
 p={'calculationType':'POINTING_DIRECTION','kernels':[{'type':'KERNEL_SET','id':1}],'direction':{'directionType':'VECTOR','directionVectorType':'VECTOR_IN_REFERENCE_FRAME','directionFrame':'J2000','directionVectorX':vector[0],'directionVectorY':vector[1],'directionVectorZ':vector[2],'aberrationCorrection':'NONE','antiVectorFlag':False},'referenceFrame':'J2000','coordinateRepresentation':'RECTANGULAR','vectorMagnitude':'PRESERVE_ORIGINAL',**time_block([0.])}
 r=w.run(p);records.append({'request':p,'result':r});print(vector,r.get('rows'),flush=True)
Path('docs/review-2026-11-11/wgc-format-probes.json').write_text(json.dumps(records,indent=2,default=str)+'\n')
