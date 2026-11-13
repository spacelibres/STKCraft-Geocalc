"""Check retained WGC two-direction samples and independent CSPICE bits."""
import argparse
import csv
import hashlib
import json
import subprocess
from decimal import Decimal
from pathlib import Path
import wgc2_geometry_compare as m

ROOT=Path(__file__).resolve().parents[1]
TRACE=ROOT/'docs/review-2026-11-11/two-directions-live.json'
LOCAL=ROOT/'target/reference/two-directions-local.csv'
FIXTURE=ROOT/'tests/golden/two_directions_live_cspice.csv'


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream,'sha256').hexdigest()


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--report',type=Path,required=True)
    args=parser.parse_args()
    output=args.report.resolve()
    if (ROOT/'docs/reference').resolve() in output.parents:
        parser.error('write the report outside the reference directory')
    # This native-bit test regenerates the local values and fails on any
    # mismatch with the independently captured CSPICE fixture.
    run=subprocess.run(['cargo','test','--locked','--lib','exact_cspice_for_live_nasa_probe_inputs'],
                       cwd=ROOT,capture_output=True,text=True,encoding='utf8',errors='replace')
    if run.returncode:
        raise RuntimeError('native-bit regression failed:\n'+run.stdout+'\n'+run.stderr)
    if 'running 1 test' not in run.stdout or '1 passed; 0 failed' not in run.stdout:
        raise RuntimeError('native-bit regression did not execute exactly one passing test')
    traces=json.loads(TRACE.read_text(encoding='utf8'),parse_float=Decimal)
    references={entry['label']:entry['final'] for entry in traces if 'label' in entry}
    rows=list(csv.DictReader(LOCAL.open(encoding='utf8',newline='')))
    if set(references)!={'orthogonal','velocity','sphere','vector_s_anti'} or len(rows)!=4:
        raise RuntimeError('unexpected retained directions or local row count')
    failures=[]
    seen=set()
    for row in rows:
        label=row['label']
        if label in seen or label not in references:
            raise RuntimeError('duplicate or unknown local direction result')
        seen.add(label)
        reference=references[label]
        if [column['outputID'] for column in reference['columns']]!=['DATE','ANGULAR_SEPARATION'] or len(reference['rows'])!=1 or reference['rows'][0][0]!=Decimal('500000000'):
            raise RuntimeError(f'{label}: unexpected WGC schema or epoch')
        actual=m.wgc_output_value(Decimal(row['angle_degrees']),'ANGULAR_SEPARATION')
        expected=Decimal(str(reference['rows'][0][1]))
        if actual!=expected:
            failures.append(f'{label}: WGC {expected}, local formatted {actual}')
    report={'scope':'Exact WGC displayed values and independent native binary64 regression',
            'server_kernel_bytes_verified':False,'rows':len(rows),'errors':failures,
            'trace_sha256':digest(TRACE),'native_fixture_sha256':digest(FIXTURE),
            'local_kernels':{name:digest(ROOT/'target/kernels'/name) for name in ['de442s.bsp','pck00011.tpc']}}
    output.parent.mkdir(parents=True,exist_ok=True)
    output.write_text(json.dumps(report,indent=2)+'\n',encoding='utf8')
    print(f'{len(rows)} two-direction rows; {len(failures)} display differences; native bit regression passed')
    return int(bool(failures))


if __name__=='__main__':
    raise SystemExit(main())
