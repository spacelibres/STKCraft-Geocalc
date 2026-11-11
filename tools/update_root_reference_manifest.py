"""Record hashes for source routines and data actually used in root regressions."""
from pathlib import Path
import json,hashlib
root=Path(__file__).resolve().parents[1];path=root/'docs/reference/CSPICE_N0067.json'
manifest=json.loads(path.read_text());source=root/'target/reference/n0067/cspice/src/cspice'
names=['spkpvn.c','prop2b.c','stmp03.c','chbigr.c','eqncpv.c','kepleq.c','kpsolv.c','vrotv.c',
       'zzwahr.c','zzmobliq.c','zzeprc76.c','zzenut80.c','zzteme.c','zztwovxf.c','ducrss.c','dvcrss.c','dvhat.c','invstm.c','xposbl.c',
       'spkgps.c','ckgp.c','ckgpav.c','ckpfs.c','ckbsr.c','lstltd.c','qdq2av.c','q2m.c','lgrind.c','hrmint.c']
for kind in [1,5,8,9,10,12,13,14,15,17,18,19,20,21]:
    names.extend([f'spkr{kind:02}.c',f'spke{kind:02}.c'])
for kind in [4,5,6]:
    names.extend([f'ckr{kind:02}.c',f'cke{kind:02}.c'])
for name in names:
    manifest['reference_source_sha256'][name]=hashlib.sha256((source/name).read_bytes()).hexdigest()
for separate in ['CK_PCK_SOURCE_HASHES.json','SGP4_SOURCE_HASHES.json','DYNAMIC_FRAME_SOURCE_HASHES.json','DSK_SOURCE_HASHES.json']:
    extra=json.loads((path.parent/separate).read_text())
    # These manifests also retain algorithm-specific notes independently.
    for key,value in extra.items():
        if isinstance(value,dict):
            for name,digest in value.items():
                if name.endswith('.c') and isinstance(digest,str) and len(digest)==64:
                    manifest['reference_source_sha256'][name]=digest
manifest['kernels_sha256'].pop('earth_latest_high_prec.bpc',None)
for name in ['de442.bsp','solar_system_v0072.tm','earth_000101_270205_261110.bpc',
             'earth_1962_260806_2126_combined.bpc','EARTHFIXEDITRF93.TF',
             'cassini/pck00010.tpc','cassini/enceladus_ssd_250728_v2.tpc']:
    manifest['kernels_sha256'][name]=hashlib.sha256((root/'target/kernels'/name).read_bytes()).hexdigest()
manifest['rolling_kernel_dates']={'earth_latest_high_prec.bpc':{'last_modified_utc':'2026-11-11T01:07:47Z','bytes':5160960,
    'immutable_equivalent':'earth_000101_270205_261110.bpc','sha256':manifest['kernels_sha256']['earth_000101_270205_261110.bpc']}}
path.write_text(json.dumps(manifest,indent=2)+'\n')
