from pathlib import Path
exec(Path('tools/probe_gf_constraints_wgc.py').read_text().split('common=')[0])
out=Path('docs/review-2026-11-11/gf-constraints-domains-live.json')
base={'calculationType':'GF_COORDINATE_SEARCH','kernels':[{'type':'KERNEL_SET','id':1}],'timeSystem':'TDB','timeFormat':'SECONDS_PAST_J2000','intervals':[{'startTime':'0','endTime':'86400'}],'timeStep':60,'timeStepUnits':'SECONDS','outputTimeSystem':'TDB','outputTimeFormat':'SECONDS_PAST_J2000','outputDurationUnits':'SECONDS','target':'MOON','observer':'EARTH','referenceFrame':'IAU_EARTH','aberrationCorrection':'NONE','shouldComplementWindow':False,'intervalAdjustment':'NO_ADJUSTMENT','intervalFiltering':'NO_FILTERING'}
for system,coord in [('CYLINDRICAL','LONGITUDE'),('RA/DEC','RIGHT ASCENSION')]:
 for label,lo,hi in [('pi_wrap',170,-170),('zero_wrap',350,10),('full',0,360),('minus_full',-180,180),('offset_equal',10,370)]:calc(system+'_'+label,{**base,'condition':{'coordinateSystem':system,'coordinate':coord,'relationalCondition':'RANGE','referenceValue':lo,'upperLimit':hi}})
