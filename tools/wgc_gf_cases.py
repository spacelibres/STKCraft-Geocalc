"""Representative official GF requests and independent N0067 endpoint oracles."""
import json,math
from pathlib import Path
import spiceypy as s
ROOT=Path(__file__).resolve().parents[1]
PCK='target/kernels/pck00011.tpc,target/kernels/mars_iau2000_v1.tpc,target/kernels/gm_de431.tpc,target/kernels/moon_pa_de440_200625.bpc'
FK=','.join(x['path']for x in json.loads((ROOT/'docs/review-2026-11-11/wgc-kernel-fk-provenance.json').read_text()))
def load():
 s.kclear()
 for path in ['target/kernels/de442.bsp','target/kernels/naif0012.tls',*PCK.split(','),*FK.split(',')]:s.furnsh(str(ROOT/path))
 s.gfstol(1e-6)
def cell(windows):
 c=s.cell_double(4000)
 for a,b in windows:s.wninsd(a,b,c)
 return c
def pairs(c):return [list(map(float,s.wnfetd(c,i)))for i in range(s.wncard(c))]
def payload(kind,windows,step,**extra):
 return {'calculationType':kind,'kernels':[{'type':'KERNEL_SET','id':1}],'timeSystem':'TDB','timeFormat':'SECONDS_PAST_J2000',
         'intervals':[{'startTime':repr(a),'endTime':repr(b)}for a,b in windows],'timeStep':step,'timeStepUnits':'SECONDS',
         'outputTimeSystem':'TDB','outputTimeFormat':'SECONDS_PAST_J2000','outputDurationUnits':'SECONDS','shouldComplementWindow':False,
         'intervalAdjustment':'NO_ADJUSTMENT','intervalFiltering':'NO_FILTERING',**extra}
def cases():
 load();window=[(0.,2592000.)];corr='NONE';common={'target':'MOON','observer':'EARTH','aberrationCorrection':corr};out=[]
 def add(label,kind,windows=window,step=3600.,**extra):out.append({'id':label,'payload':payload(kind,windows,step,**extra)})
 add('distance','GF_DISTANCE_SEARCH',**common,condition={'relationalCondition':'<','referenceValue':400000.})
 add('range_rate','GF_RANGE_RATE_SEARCH',**common,condition={'relationalCondition':'>','referenceValue':0.})
 add('coordinate','GF_COORDINATE_SEARCH',windows=[(0.,86400.)],step=60.,**common,referenceFrame='IAU_EARTH',condition={'coordinateSystem':'LATITUDINAL','coordinate':'LONGITUDE','relationalCondition':'>','referenceValue':0.})
 add('phase','GF_PHASE_ANGLE_SEARCH',**common,illuminator='SUN',condition={'relationalCondition':'<','referenceValue':90.})
 add('separation','GF_ANGULAR_SEPARATION_SEARCH',target1='MOON',target2='SUN',shape1='POINT',shape2='POINT',observer='EARTH',aberrationCorrection=corr,condition={'relationalCondition':'<','referenceValue':90.})
 add('subpoint','GF_SUB_POINT_SEARCH',**common,targetFrame='MOON_ME',subPointType='Near point: ellipsoid',condition={'coordinateSystem':'LATITUDINAL','coordinate':'LATITUDE','relationalCondition':'>','referenceValue':0.})
 add('illumination','GF_ILLUMINATION_ANGLES_SEARCH',**common,illuminator='SUN',targetFrame='MOON_ME',shape1='ELLIPSOID',offSurfaceFlag=False,coordinateRepresentation='PLANETODETIC',longitude=10.,latitude=20.,altitude=0.,condition={'angleType':'INCIDENCE','relationalCondition':'<','referenceValue':90.})
 direction=s.spkpos('MOON',0.,'J2000',corr,'EARTH')[0]
 add('intercept','GF_SURFACE_INTERCEPT_POINT_SEARCH',windows=[(-10000.,10000.)],step=60.,**common,targetFrame='MOON_ME',directionVectorType='VECTOR_IN_REFERENCE_FRAME',directionFrame='J2000',directionVectorX=float(direction[0]),directionVectorY=float(direction[1]),directionVectorZ=float(direction[2]),condition={'coordinateSystem':'LATITUDINAL','coordinate':'LONGITUDE','relationalCondition':'<','referenceValue':0.})
 add('occultation','GF_OCCULTATION_SEARCH',windows=[(float(s.str2et('2017-08-21T15:00:00')),float(s.str2et('2017-08-21T23:00:00')))],step=60.,target1='MOON',target2='EARTH',shape1='ELLIPSOID',shape2='ELLIPSOID',frame1='IAU_MOON',frame2='IAU_EARTH',observer='SUN',aberrationCorrection=corr,occultationType='ANY')
 add('range_postprocessing','GF_DISTANCE_SEARCH',windows=[(0.,1300000.),(2000000.,4000000.)],**common,condition={'relationalCondition':'RANGE','referenceValue':380000.,'upperLimit':400000.})
 out[-1]['payload'].update({'shouldComplementWindow':True,'intervalAdjustment':'EXPAND_INTERVALS','intervalAdjustmentAmount':.25,'intervalAdjustmentUnits':'SECONDS','intervalFiltering':'FILTER_INTERVALS','intervalFilteringThreshold':1.,'intervalFilteringThresholdUnits':'MINUTES','outputDurationUnits':'HOURS'})
 return out

def local_parameters(case):
 p=case['payload'];condition=p.get('condition',{});family=p['calculationType'];target=p.get('target',p.get('target1','MOON'));observer=p.get('observer','EARTH');second=p.get('target2','SUN');illuminator=p.get('illuminator','SUN');ids={'MOON':301,'EARTH':399,'SUN':10,'ENCELADUS':602,'CASSINI':-82};num=lambda v:ids[v]if v in ids else int(v)
 point=[0.,0.,0.]
 if family=='GF_ILLUMINATION_ANGLES_SEARCH':
  radii=s.bodvrd(str(num(target)),'RADII',3)[1];point=list(map(float,s.georec(p['longitude']*s.rpd(),p['latitude']*s.rpd(),p.get('altitude',0.),radii[0],(radii[0]-radii[2])/radii[0])))
 elif family in ['GF_SURFACE_INTERCEPT_POINT_SEARCH','GF_RAY_IN_FOV_SEARCH']:point=[p['directionVectorX'],p['directionVectorY'],p['directionVectorZ']]
 values=[case['id'],family,num(target),num(observer),num(second),num(illuminator),p.get('referenceFrame',p.get('directionFrame','J2000')),p.get('targetFrame','IAU_MOON'),p.get('subPointType','Near point: ellipsoid'),condition.get('coordinateSystem','RECTANGULAR'),condition.get('coordinate','X'),condition.get('angleType','INCIDENCE'),condition.get('relationalCondition','>'),condition.get('referenceValue',0.),condition.get('upperLimit',0.),p['aberrationCorrection'],';'.join(str(w['startTime'])+':'+str(w['endTime'])for w in p['intervals']),p['timeStep'],condition.get('adjustmentValue',0.),str(p.get('shouldComplementWindow',False)).lower(),{'NO_ADJUSTMENT':'NONE','EXPAND_INTERVALS':'EXPAND','CONTRACT_INTERVALS':'CONTRACT'}[p.get('intervalAdjustment','NO_ADJUSTMENT')],p.get('intervalAdjustmentAmount',0.),p.get('intervalAdjustmentUnits','SECONDS'),str(p.get('intervalFiltering','NO_FILTERING')=='FILTER_INTERVALS').lower(),p.get('intervalFilteringThreshold',0.),p.get('intervalFilteringThresholdUnits','SECONDS'),p['outputDurationUnits'],*point,p.get('shape1',p.get('shape','POINT')),p.get('shape2','POINT'),p.get('occultationType','ANY')]
 assert len(values)==33
 return ','.join(map(str,values))

def native_windows(case):
 p=case['payload'];k=p['calculationType'];c=p.get('condition',{});target=p.get('target','MOON');obs=p['observer'];corr=p['aberrationCorrection'];step=p['timeStep'];windows=[(float(x['startTime']),float(x['endTime']))for x in p['intervals']];cf=cell(windows)
 angular=k in ['GF_PHASE_ANGLE_SEARCH','GF_ANGULAR_SEPARATION_SEARCH','GF_ILLUMINATION_ANGLES_SEARCH']or c.get('coordinate')in ['LONGITUDE','LATITUDE','RIGHT ASCENSION','DECLINATION','COLATITUDE']
 factor=s.rpd()if angular else 1.;adjust=float(c.get('adjustmentValue',0.))*factor
 def search(rel,ref,w):
  v=ref*factor
  if k=='GF_DISTANCE_SEARCH':return s.gfdist(target,corr,obs,rel,v,adjust,step,1000,w)
  if k=='GF_RANGE_RATE_SEARCH':return s.gfrr(target,corr,obs,rel,v,adjust,step,1000,w)
  if k=='GF_PHASE_ANGLE_SEARCH':return s.gfpa(target,p['illuminator'],corr,obs,rel,v,adjust,step,1000,w)
  if k=='GF_ANGULAR_SEPARATION_SEARCH':return s.gfsep(p['target1'],p['shape1'],'J2000',p['target2'],p['shape2'],'J2000',corr,obs,rel,v,adjust,step,1000,w)
  if k=='GF_COORDINATE_SEARCH':return s.gfposc(target,p['referenceFrame'],corr,obs,c['coordinateSystem'],c['coordinate'],rel,v,adjust,step,1000,w)
  if k=='GF_SUB_POINT_SEARCH':return s.gfsubc(target,p['targetFrame'],p['subPointType'],corr,obs,c['coordinateSystem'],c['coordinate'],rel,v,adjust,step,1000,w)
  if k=='GF_ILLUMINATION_ANGLES_SEARCH':
   f=local_parameters(case).split(',');point=list(map(float,f[27:30]));return s.gfilum('ELLIPSOID',c['angleType'],target,p['illuminator'],p['targetFrame'],corr,obs,point,rel,v,adjust,step,1000,w)
  if k=='GF_SURFACE_INTERCEPT_POINT_SEARCH':return s.gfsntc(target,p['targetFrame'],'ELLIPSOID',corr,obs,p['directionFrame'],[p['directionVectorX'],p['directionVectorY'],p['directionVectorZ']],c['coordinateSystem'],c['coordinate'],rel,v,adjust,step,1000,w)
  raise RuntimeError(k)
 if k=='GF_RAY_IN_FOV_SEARCH':result=s.gfrfov(p['instrument'],[p['directionVectorX'],p['directionVectorY'],p['directionVectorZ']],p['directionFrame'],corr,obs,step,cf)
 elif k=='GF_TARGET_IN_INSTRUMENT_FOV_SEARCH':result=s.gftfov(p['instrument'],target,p['shape'],p['targetFrame'],corr,obs,step,cf)
 elif k=='GF_OCCULTATION_SEARCH':result=s.gfoclt(p['occultationType'],p['target1'],p['shape1'],p['frame1'],p['target2'],p['shape2'],p['frame2'],corr,obs,step,cf)
 elif c['relationalCondition']=='RANGE':
  lo,hi=sorted([float(c['referenceValue']),float(c['upperLimit'])]);result=search('<',hi,search('>',lo,cf))
 else:result=search(c['relationalCondition'],float(c.get('referenceValue',0.)),cf)
 if p.get('shouldComplementWindow'):result=s.wndifd(cf,result)
 amount=s.convrt(p.get('intervalAdjustmentAmount',0.),p.get('intervalAdjustmentUnits','SECONDS'),'SECONDS')
 if p.get('intervalAdjustment')=='EXPAND_INTERVALS':s.wnexpd(amount,amount,result)
 if p.get('intervalAdjustment')=='CONTRACT_INTERVALS':s.wncond(amount,amount,result)
 if p.get('intervalFiltering')=='FILTER_INTERVALS':s.wnfltd(s.convrt(p['intervalFilteringThreshold'],p['intervalFilteringThresholdUnits'],'SECONDS'),result)
 return [[a,b,float(s.convrt(b-a,'SECONDS',p['outputDurationUnits']))]for a,b in pairs(result)]
if __name__=='__main__':
 records=cases()
 for case in records:
  case['local']=local_parameters(case);case['native']=native_windows(case);print(case['id'],len(case['native']),case['native'][:1],flush=True)
 out=ROOT/'docs/review-2026-11-11/gf-all-families';out.mkdir(parents=True,exist_ok=True);(out/'cases.json').write_text(json.dumps(records,indent=2)+'\n');(out/'queries.csv').write_text('\n'.join(c['local']for c in records)+'\n')
