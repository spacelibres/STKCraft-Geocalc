"""Replay 11 retained NASA finder examples and independent native windows."""
import argparse
import hashlib
import json
import os
import subprocess
from pathlib import Path

from wgc_gf_compare import c, compare_gf_rows, parse_gf_output
from wgc_gf_cases import FK, PCK

ROOT = Path(__file__).resolve().parents[1]
REFERENCE = ROOT / 'docs/reference/wgc-finders'
CASES = ROOT / 'docs/review-2026-11-11/gf-all-families/cases.json'
ORDER = ['coordinate','separation','distance','subpoint','occultation','intercept',
         'target_fov','ray_fov','range_rate','phase','illumination','range_postprocessing']


def digest(path):
    result = hashlib.sha256()
    with path.open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            result.update(block)
    return result.hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True, help='prebuilt wgc_gf binary')
    parser.add_argument('--report', type=Path, required=True)
    parser.add_argument('--glibc-rotation', action='store_true', help='activate the explicit glibc R3 profile')
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    output = args.report.resolve()
    reference = (ROOT / 'docs/reference').resolve()
    if output == reference or reference in output.parents:
        parser.error('write the report outside the committed reference directory')
    probe_environment=os.environ.copy()
    probe_environment.pop('STKCRAFT_WGC_GLIBC_ROT3',None)
    if args.glibc_rotation:
        probe_environment['STKCRAFT_WGC_GLIBC_ROT3']='1'
    probe=subprocess.run([str(binary),'--rotation-profile'],cwd=ROOT,text=True,encoding='utf8',
                         errors='replace',capture_output=True,env=probe_environment)
    expected_profile='glibc-fma-r3' if args.glibc_rotation else 'native'
    if probe.returncode or probe.stdout.strip()!=expected_profile:
        raise RuntimeError(f'backend profile mismatch: expected {expected_profile}, got {probe.stdout.strip()!r}; {probe.stderr.strip()}')
    cases = json.loads(CASES.read_text(encoding='utf8'))
    if len({case['id'] for case in cases}) != len(cases):
        raise RuntimeError('duplicate case IDs')
    records, files = [], {binary, CASES}
    for case_id in ORDER:
        case = next(case for case in cases if case['id'] == case_id)
        trace = REFERENCE / ('finder-' + case_id + '.jsonl')
        files.add(trace)
        record = {'id':case_id, 'family':case['payload']['calculationType'], 'failures':[]}
        try:
            reference_table = c.Wgc(False, 120, replay=trace).run(case['payload'])
            resources = case.get('resources', {'spk':'target/kernels/de442.bsp','pck':PCK,'fk':FK})
            command = [str(binary), resources['pck'], resources['fk']]
            if 'ik' in resources:
                command += [resources['ik'], str(resources['instrument'])]
            environment = os.environ.copy()
            environment['STKCRAFT_SPK'] = resources['spk']
            environment.pop('STKCRAFT_WGC_GLIBC_ROT3',None)
            if args.glibc_rotation:
                environment['STKCRAFT_WGC_GLIBC_ROT3']='1'
            for key, variable in [('ck','STKCRAFT_CK'),('sclk','STKCRAFT_SCLK'),('lsk','STKCRAFT_LSK')]:
                environment.pop(variable, None)
                if key in resources:
                    environment[variable] = resources[key]
            paths = [*resources['spk'].split(','), *resources['pck'].split(','), *resources['fk'].split(',')]
            paths += [resources[key] for key in ('ik','ck','lsk') if key in resources]
            if 'sclk' in resources:
                paths += [part.split('|',1)[1] for part in resources['sclk'].split(',')]
            files.update((ROOT / path).resolve(strict=True) for path in paths)
            process = subprocess.run(command, cwd=ROOT, input=case['local']+'\n',
                                     text=True, encoding='utf8', capture_output=True,
                                     env=environment, timeout=360)
            if process.returncode:
                raise RuntimeError('local process failed: '+process.stderr)
            item = parse_gf_output(process.stdout, [case_id])[case_id]
            if 'error' in item:
                raise RuntimeError(item['error'])
            rows = item['rows']
            c.checked_count(case_id+' native', len(case['native']), len(rows))
            for index, (row, expected) in enumerate(zip(rows, case['native'])):
                if len(row) != 3 or len(expected) != 3:
                    raise RuntimeError('native column count mismatch')
                for column, (value, native) in enumerate(zip(row, expected)):
                    try:
                        c.compare_cspice_f64(float(value), native, f'{case_id}/{index}/{column}')
                    except RuntimeError as error:
                        record['failures'].append(str(error))
            result = compare_gf_rows(reference_table, rows, case_id, output_exact=True)
            record.update(windows=result['windows'], compared_cells=result['compared_cells'])
            record['failures'].extend(result['failures'])
        except Exception as error:
            record['failures'].append(f'{type(error).__name__}: {error}')
        records.append(record)
        print(f"{case_id}: {record.get('windows', 0)} windows; {len(record['failures'])} errors", flush=True)
    report = {'scope':'Native binary64 equality and WGC formatted output equality for retained cases',
              'server_kernel_bytes_verified':False, 'glibc_rotation':args.glibc_rotation,
              'verified_profile':expected_profile, 'cases':records,
              'files':{str(path):digest(path) for path in sorted(files)}}
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2)+'\n', encoding='utf8')
    return int(any(row['failures'] for row in records))


if __name__ == '__main__':
    raise SystemExit(main())
