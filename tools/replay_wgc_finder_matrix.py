"""Replay the retained WGC finder matrix and native endpoint bits.

A nonzero exit reports any difference; no tolerances are used for acceptance.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

from wgc_gf_compare import c, compare_gf_rows, parse_gf_output
from wgc_gf_cases import FK, PCK

ROOT = Path(__file__).resolve().parents[1]
REFERENCE = ROOT / 'docs/reference/wgc-finders'
CAPTURE = REFERENCE / 'finder-matrix-cases.json'
OCCULTATION_CAPTURE = REFERENCE / 'finder-matrix-occultation-cases.json'
MATRIX = {
    'distance': ('GF_DISTANCE_SEARCH', [('<', 380000.0), ('>', 400000.0), ('=', 390000.0)]),
    'phase': ('GF_PHASE_ANGLE_SEARCH', [('<', 60.0), ('>', 100.0), ('=', 90.0)]),
    'range_rate': ('GF_RANGE_RATE_SEARCH', [('<', -0.05), ('>', 0.05), ('=', 0.0)]),
    'separation': ('GF_ANGULAR_SEPARATION_SEARCH', [('<', 60.0), ('>', 120.0), ('=', 90.0)]),
    'subpoint': ('GF_SUB_POINT_SEARCH', [('<', -5.0), ('>', 5.0), ('=', 0.0)]),
    'illumination': ('GF_ILLUMINATION_ANGLES_SEARCH', [('<', 60.0), ('>', 120.0), ('=', 90.0)]),
    'coordinate': ('GF_COORDINATE_SEARCH', [('<', -90.0), ('>', 90.0), ('=', 0.0)]),
    'intercept': ('GF_SURFACE_INTERCEPT_POINT_SEARCH', [('<', -10.0), ('>', 10.0), ('=', 0.0)]),
}
CORRECTIONS = [('NONE', 'NONE'), ('LT', 'LT'), ('CN_S', 'CN+S')]


def validate_matrix(cases):
    expected = {}
    for family, (kind, conditions) in MATRIX.items():
        for offset, (label, correction) in enumerate(CORRECTIONS):
            for index, (relation, value) in enumerate(conditions):
                case_id = f'{family}_{label}_{offset * 3 + index}'
                expected[case_id] = (family, kind, correction, relation, value)
    ids = [case['id'] for case in cases]
    if len(ids) != len(expected) or len(set(ids)) != len(ids) or set(ids) != set(expected):
        raise RuntimeError('finder matrix case IDs are incomplete or duplicated')
    for case in cases:
        family, kind, correction, relation, value = expected[case['id']]
        payload = case['payload']
        condition = payload['condition']
        fields = case['local'].split(',')
        if (payload['calculationType'] != kind or payload['aberrationCorrection'] != correction
                or condition['relationalCondition'] != relation or condition['referenceValue'] != value
                or case['trace'] != f'finder-matrix-{family}.jsonl' or not case['native']
                or len(fields) != 33 or fields[0] != case['id'] or fields[1] != kind
                or fields[12] != relation or float(fields[13]) != value or fields[15] != correction):
            raise RuntimeError(f"finder matrix case does not match expected inputs: {case['id']}")


def validate_occultation_matrix(cases):
    if len(cases) != 12:
        raise RuntimeError('expected 12 occultation cases')
    for index, case in enumerate(cases):
        correction = ['NONE', 'LT', 'CN+S'][index // 4]
        kind = ['ANY', 'FULL', 'PARTIAL', 'ANNULAR'][index % 4]
        payload, fields = case['payload'], case['local'].split(',')
        if (case['id'] != f'occultation_{index}' or len(fields) != 33
                or fields[0] != case['id'] or fields[1] != 'GF_OCCULTATION_SEARCH'
                or fields[15] != correction or fields[32] != kind
                or payload['calculationType'] != 'GF_OCCULTATION_SEARCH'
                or payload['aberrationCorrection'] != correction or payload['occultationType'] != kind):
            raise RuntimeError(f"occultation matrix case does not match expected inputs: {case['id']}")


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--report', type=Path, required=True)
    parser.add_argument('--glibc-rotation', action='store_true')
    parser.add_argument('--occultation', action='store_true', help='replay the 12-case occultation matrix')
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    output = args.report.resolve()
    reference = (ROOT / 'docs/reference').resolve()
    if output == reference or reference in output.parents:
        parser.error('write the report outside the reference directory')
    env = os.environ.copy()
    for name in ('STKCRAFT_LSK', 'STKCRAFT_SCLK', 'STKCRAFT_CK', 'STKCRAFT_WGC_GLIBC_ROT3'):
        env.pop(name, None)
    if args.glibc_rotation:
        env['STKCRAFT_WGC_GLIBC_ROT3'] = '1'
    profile = subprocess.run([str(binary), '--rotation-profile'], env=env, cwd=ROOT,
                             text=True, capture_output=True, check=True).stdout.strip()
    expected_profile = 'glibc-fma-r3' if args.glibc_rotation else 'native'
    if profile != expected_profile:
        raise RuntimeError(f'backend profile mismatch: expected {expected_profile}, got {profile}')
    env['STKCRAFT_SPK'] = 'target/kernels/de442.bsp'
    capture_file = OCCULTATION_CAPTURE if args.occultation else CAPTURE
    capture = json.loads(capture_file.read_text(encoding='utf8'))
    if args.occultation:
        validate_occultation_matrix(capture['cases'])
    else:
        validate_matrix(capture['cases'])
    paths = [ROOT/'target/kernels/de442.bsp', ROOT/'target/kernels/naif0012.tls',
             *(ROOT/path for path in PCK.split(',')), *(ROOT/path for path in FK.split(','))]
    kernels = [{'name': path.name, 'sha256': digest(path)} for path in paths]
    if kernels != capture['native_kernels_in_load_order']:
        raise RuntimeError('native capture kernel hashes or load order do not match')
    records = []
    for case in capture['cases']:
        trace = REFERENCE/(case.get('trace') or 'finder-matrix-occultation.jsonl')
        reference_table = c.Wgc(False, 120, replay=trace).run(case['payload'])
        result = subprocess.run([str(binary), PCK, FK], cwd=ROOT, env=env,
                                input=case['local']+'\n', text=True, encoding='utf8',
                                capture_output=True, check=True, timeout=360)
        item = parse_gf_output(result.stdout, [case['id']])[case['id']]
        if 'error' in item:
            raise RuntimeError(f"{case['id']}: {item['error']}")
        rows = item['rows']
        if len(case['native']) != len(rows):
            raise RuntimeError(f"{case['id']}: native window count differs")
        native_errors = []
        for i, (actual, expected) in enumerate(zip(rows, case['native'])):
            if len(actual) != 3 or len(expected) != 3:
                raise RuntimeError('invalid native window width')
            for j, (value, native) in enumerate(zip(actual, expected)):
                try:
                    c.compare_cspice_f64(float(value), native, f"{case['id']}/{i}/{j}")
                except RuntimeError as error:
                    native_errors.append(str(error))
        comparison = compare_gf_rows(reference_table, rows, case['id'], output_exact=True,
                                     allow_empty=args.occultation)
        records.append({'id': case['id'], 'windows': len(rows),
                        'native_errors': native_errors, 'wgc_errors': comparison['failures']})
        print(f"{case['id']}: {len(rows)} windows; {len(native_errors)} native and {len(comparison['failures'])} WGC differences", flush=True)
    report = {'scope': 'WGC displayed values and local CSPICE endpoint bits',
              'server_kernel_bytes_verified': False, 'verified_profile': profile,
              'binary_sha256': digest(binary), 'capture_sha256': digest(capture_file),
              'native_reference_kernels_in_load_order': kernels,
              'backend_inputs': {'spk': ['de442.bsp'],
                                 'pck': [Path(path).name for path in PCK.split(',')],
                                 'fk': [Path(path).name for path in FK.split(',')],
                                 'lsk': None},
              'cases': records}
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2)+'\n', encoding='utf8')
    return int(any(row['native_errors'] or row['wgc_errors'] for row in records))


if __name__ == '__main__':
    raise SystemExit(main())
