"""Replay retained WGC and native finder cases with a Linux binary via WSL."""
import argparse
import hashlib
import json
import shlex
import subprocess
from pathlib import Path

from wgc_gf_compare import c, compare_gf_rows, parse_gf_output
from wgc_gf_cases import FK, PCK
from replay_wgc_finder_matrix import validate_matrix

ROOT = Path(__file__).resolve().parents[1]
CASES = ROOT / 'docs/review-2026-11-11/gf-all-families/cases.json'
REFERENCE = ROOT / 'docs/reference/wgc-finders'


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--wsl-distribution', required=True)
    parser.add_argument('--linux-workspace', required=True)
    parser.add_argument('--linux-binary', type=Path, required=True)
    parser.add_argument('--linux-loader', type=Path, required=True)
    parser.add_argument('--linux-lib-dir', type=Path, required=True)
    parser.add_argument('--report', type=Path, required=True)
    parser.add_argument('--matrix', action='store_true', help='replay the 72-case finder matrix')
    args = parser.parse_args()
    root_linux = args.linux_workspace.rstrip('/')

    def linux_path(value):
        path = Path(value).resolve(strict=True)
        try:
            relative = path.relative_to(ROOT)
        except ValueError as error:
            try:
                relative = Path('target/kernels') / path.relative_to((ROOT/'target/kernels').resolve(strict=True))
            except ValueError:
                raise ValueError(f'path is outside the checkout or kernel directory: {path}') from error
        return root_linux + '/' + relative.as_posix()

    binary, loader, library = (p.resolve(strict=True) for p in
                               (args.linux_binary, args.linux_loader, args.linux_lib_dir))
    output = args.report.resolve()
    if output == (ROOT/'docs/reference').resolve() or (ROOT/'docs/reference').resolve() in output.parents:
        parser.error('write the report outside the reference directory')
    probe_command=['env','-u','STKCRAFT_WGC_GLIBC_ROT3',linux_path(loader),'--library-path',
                   linux_path(library),linux_path(binary),'--rotation-profile']
    probe=subprocess.run(['wsl','-d',args.wsl_distribution,'--cd',root_linux,'--','sh','-c',
                          ' '.join(shlex.quote(part) for part in probe_command)],cwd=ROOT,
                         text=True,encoding='utf8',errors='replace',capture_output=True)
    if probe.returncode or probe.stdout.strip()!='native':
        raise RuntimeError(f'Linux finder backend profile mismatch: {probe.stdout.strip()!r}; {probe.stderr.strip()}')
    case_file = REFERENCE/'finder-matrix-cases.json' if args.matrix else CASES
    document = json.loads(case_file.read_text(encoding='utf8'))
    cases = document['cases'] if args.matrix else document
    if args.matrix:
        validate_matrix(cases)
    elif len(cases) != 12 or len({case['id'] for case in cases}) != 12:
        raise RuntimeError('expected 12 unique finder cases')
    records, kernel_hashes = [], {}
    for case in cases:
        identifier = case['id']
        resources = case.get('resources', {'spk':'target/kernels/de442.bsp','pck':PCK,'fk':FK})
        paths = [*resources['spk'].split(','), *resources['pck'].split(','), *resources['fk'].split(',')]
        paths += [resources[key] for key in ('ik','ck','lsk') if key in resources]
        if 'sclk' in resources:
            paths += [part.split('|',1)[1] for part in resources['sclk'].split(',')]
        for value in paths:
            name = value.replace('\\','/')
            if name not in kernel_hashes:
                kernel_hashes[name] = digest((ROOT / name).resolve(strict=True))
        command = ['env','-u','STKCRAFT_WGC_GLIBC_ROT3','-u','STKCRAFT_LSK',
                   '-u','STKCRAFT_SCLK','-u','STKCRAFT_CK',
                   'STKCRAFT_SPK='+resources['spk'].replace('\\','/')]
        for key, variable in [('ck','STKCRAFT_CK'),('sclk','STKCRAFT_SCLK'),('lsk','STKCRAFT_LSK')]:
            if key in resources:
                command.append(variable+'='+resources[key].replace('\\','/'))
        command += [linux_path(loader),'--library-path',linux_path(library),linux_path(binary),
                    resources['pck'].replace('\\','/'),resources['fk'].replace('\\','/')]
        if 'ik' in resources:
            command += [resources['ik'].replace('\\','/'),str(resources['instrument'])]
        shell_command = ' '.join(shlex.quote(part) for part in command)
        process = subprocess.run(['wsl','-d',args.wsl_distribution,'--cd',root_linux,'--',
                                  'sh','-c',shell_command], cwd=ROOT, input=case['local']+'\n',
                                 text=True, encoding='utf8', errors='replace', capture_output=True,
                                 timeout=360)
        if process.returncode:
            raise RuntimeError(f'{identifier}: Linux process failed: {process.stderr}')
        item = parse_gf_output(process.stdout, [identifier])[identifier]
        if 'error' in item:
            raise RuntimeError(f"{identifier}: {item['error']}")
        rows = item['rows']
        c.checked_count(identifier+' native', len(case['native']), len(rows))
        native_failures = []
        for index, (row, expected) in enumerate(zip(rows, case['native'])):
            if len(row) != 3 or len(expected) != 3:
                raise RuntimeError('native column count mismatch')
            for column, (value, native) in enumerate(zip(row, expected)):
                try:
                    c.compare_cspice_f64(float(value), native, f'{identifier}/{index}/{column}')
                except RuntimeError as error:
                    native_failures.append(str(error))
        trace = REFERENCE / (case['trace'] if args.matrix else 'finder-'+identifier+'.jsonl')
        reference = c.Wgc(False,120,replay=trace).run(case['payload'])
        wgc = compare_gf_rows(reference, rows, identifier, output_exact=True)
        records.append({'id':identifier,'windows':wgc['windows'],
                        'compared_cells':wgc['compared_cells'],
                        'wgc_failures':wgc['failures'],'native_failures':native_failures,
                        'trace_sha256':digest(trace)})
        print(f"{identifier}: {wgc['windows']} windows; {len(wgc['failures'])} WGC and {len(native_failures)} native errors", flush=True)
    result = {'scope':'WGC displayed equality and recorded Windows CSPICE binary64 equality',
              'server_kernel_bytes_verified':False, 'cases':records, 'cases_sha256':digest(case_file),
              'local_kernel_sha256':kernel_hashes,
              'linux_profile':{'binary_sha256':digest(binary),'loader_sha256':digest(loader),
                               'verified_profile':'native',
                               'libm_sha256':digest(library/'libm.so.6'),
                               'libc_sha256':digest(library/'libc.so.6')}}
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(result,indent=2)+'\n',encoding='utf8')
    return int(any(row['wgc_failures'] or row['native_failures'] for row in records))


if __name__ == '__main__':
    raise SystemExit(main())
