from pathlib import Path
exec(Path('tools/probe_gf_constraints_domains_wgc.py').read_text().split('for system,coord')[0])
out=Path('docs/review-2026-11-11/gf-constraints-turns-live.json')
for a,b in [(540,10),(-540,10),(180,10),(-180,10)]:calc(str(a)+'_'+str(b),{**base,'condition':{'coordinateSystem':'LATITUDINAL','coordinate':'LONGITUDE','relationalCondition':'RANGE','referenceValue':a,'upperLimit':b}})
