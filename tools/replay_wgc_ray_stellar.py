"""Replay WGC GFRFOV stellar corrections and native endpoint bits."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
from wgc_gf_compare import c, compare_gf_rows, parse_gf_output

ROOT = Path(__file__).resolve().parents[1]
REFERENCE = ROOT/'docs/reference/wgc-finders'
TRACE = REFERENCE/'finder-matrix-ray-corrections.jsonl'
CAPTURE = REFERENCE/'finder-matrix-ray-native.json'


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--report', type=Path, required=True)
    parser.add_argument('--glibc-rotation', action='store_true')
    parser.add_argument('--target-fov', action='store_true', help='replay target-in-FOV correction cases')
    args = parser.parse_args()
    binary, output = args.binary.resolve(strict=True), args.report.resolve()
    reference_dir = (ROOT/'docs/reference').resolve()
    if output == reference_dir or reference_dir in output.parents:
        parser.error('write the report outside the reference directory')
    capture_path = REFERENCE/'finder-matrix-target-fov-native.json' if args.target_fov else CAPTURE
    trace_path = REFERENCE/'finder-matrix-target-fov.jsonl' if args.target_fov else TRACE
    capture = json.loads(capture_path.read_text(encoding='utf8'))
    cases = capture['cases']
    expected_corrections = ['NONE','LT','CN+S'] if args.target_fov else ['S','XS']
    if [case['payload']['aberrationCorrection'] for case in cases] != expected_corrections:
        raise RuntimeError('expected correction cases are incomplete or out of order')
    resource = cases[0]['resources']
    if any(case['resources'] != resource for case in cases):
        raise RuntimeError('native cases use different resources')
    paths = resource['spk'].split(',')+resource['pck'].split(',')+resource['fk'].split(',')
    paths += [resource['lsk'],resource['ik'],resource['ck'],resource['sclk'].split('|',1)[1]]
    kernels = [{'name':Path(path).name,'sha256':digest((ROOT/path).resolve(strict=True))} for path in paths]
    if kernels != capture['kernels_in_load_order']:
        raise RuntimeError('native capture kernel hashes or load order differ')
    env = os.environ.copy()
    env.pop('STKCRAFT_WGC_GLIBC_ROT3', None)
    if args.glibc_rotation:
        env['STKCRAFT_WGC_GLIBC_ROT3'] = '1'
    profile = subprocess.run([str(binary),'--rotation-profile'],cwd=ROOT,env=env,
                             text=True,capture_output=True,check=True).stdout.strip()
    expected = 'glibc-fma-r3' if args.glibc_rotation else 'native'
    if profile != expected:
        raise RuntimeError(f'backend profile mismatch: expected {expected}, got {profile}')
    env.update(STKCRAFT_SPK=resource['spk'],STKCRAFT_LSK=resource['lsk'],
               STKCRAFT_CK=resource['ck'],STKCRAFT_SCLK=resource['sclk'])
    replay = c.Wgc(False,120,replay=trace_path)
    records = []
    for case in cases:
        process = subprocess.run([str(binary),resource['pck'],resource['fk'],resource['ik'],str(resource['instrument'])],
                                 cwd=ROOT,env=env,input=case['local']+'\n',text=True,
                                 capture_output=True,check=True,timeout=360)
        item = parse_gf_output(process.stdout,[case['id']])[case['id']]
        if 'error' in item:
            raise RuntimeError(item['error'])
        rows = item['rows']
        c.checked_count(case['id']+' native',len(case['native']),len(rows))
        native_errors = []
        for i,(row,expected_row) in enumerate(zip(rows,case['native'])):
            if len(row) != 3 or len(expected_row) != 3:
                raise RuntimeError('invalid native window width')
            for j,(value,expected_value) in enumerate(zip(row,expected_row)):
                try:c.compare_cspice_f64(float(value),expected_value,f"{case['id']}/{i}/{j}")
                except RuntimeError as error:native_errors.append(str(error))
        result = compare_gf_rows(replay.run(case['payload']),rows,case['id'],output_exact=True)
        records.append({'id':case['id'],'native_errors':native_errors,'wgc_errors':result['failures']})
    posts = [json.loads(line) for line in trace_path.read_text(encoding='utf8').splitlines()]
    invalid = [] if args.target_fov else [event for event in posts if event['method']=='POST' and event['request']['aberrationCorrection'] in ('LT','CN+S')]
    if not args.target_fov and ([event['request']['aberrationCorrection'] for event in invalid] != ['LT','CN+S'] or any('SPICE(INVALIDOPTION)' not in json.loads(event['raw_response'])['error']['shortDescription'] for event in invalid)):
        raise RuntimeError('expected WGC invalid ray correction observations')
    invalid_options = [] if args.target_fov else ['LT','CN+S']
    for correction in invalid_options:
        fields = cases[0]['local'].split(',');fields[0] = 'invalid_'+correction;fields[15] = correction
        process = subprocess.run([str(binary),resource['pck'],resource['fk'],resource['ik'],str(resource['instrument'])],
                                 cwd=ROOT,env=env,input=','.join(fields)+'\n',text=True,
                                 capture_output=True,check=True,timeout=360)
        expected_error = f'ERROR,{fields[0]},SPICE(INVALIDOPTION)'
        if process.stdout.strip() != expected_error:
            records.append({'id':fields[0],'native_errors':[process.stdout.strip()],'wgc_errors':[]})
    report = {'scope':'WGC displayed values, native endpoint bits, and invalid-option symbols',
              'server_kernel_bytes_verified':False,'verified_profile':profile,
              'binary_sha256':digest(binary),'trace_sha256':digest(trace_path),'capture_sha256':digest(capture_path),
              'cases':records,'invalid_options_checked':invalid_options}
    output.parent.mkdir(parents=True,exist_ok=True)
    output.write_text(json.dumps(report,indent=2)+'\n',encoding='utf8')
    print(f"{len(cases)} FOV cases; {len(invalid)} invalid options; {sum(len(row['native_errors'])+len(row['wgc_errors']) for row in records)} differences")
    return int(any(row['native_errors'] or row['wgc_errors'] for row in records))


if __name__=='__main__':
    raise SystemExit(main())
