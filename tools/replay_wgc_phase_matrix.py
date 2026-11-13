"""Replay additional PHASE_ANGLE cases against retained WGC display values.

The SPK argument is an ordered, comma-separated list. WGC server kernel bytes
are not independently verified. Exact displayed equality is not raw equality.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess

import wgc2_geometry_compare as comparison

ROOT = Path(__file__).resolve().parents[1]
TRACE = ROOT / 'docs/reference/wgc-calculators/phase-angle-expanded.jsonl'
CASES = [('MOON', 301, 'SUN', 10, 'EARTH', 399),
         ('EARTH', 399, 'SUN', 10, 'MOON', 301),
         ('MARS', 499, 'SUN', 10, 'EARTH', 399)]
TIMES = [-100000000.0, 0.0, 100000000.0]
CORRECTIONS = ['LT', 'CN', 'CN+S']


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--spk', required=True)
    parser.add_argument('--report', type=Path, required=True)
    parser.add_argument('--glibc-rotation', action='store_true')
    parser.add_argument('--native-capture', type=Path,
                        help='also compare every raw angle against this native capture')
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    kernels = [Path(part).resolve(strict=True) for part in args.spk.split(',')]
    output = args.report.resolve()
    reference_dir = (ROOT / 'docs/reference').resolve()
    if output == reference_dir or reference_dir in output.parents:
        parser.error('write the report outside the reference directory')
    env = os.environ.copy()
    env.pop('STKCRAFT_LSK', None)
    env.pop('STKCRAFT_WGC_GLIBC_ROT3', None)
    if args.glibc_rotation:
        env['STKCRAFT_WGC_GLIBC_ROT3'] = '1'
    expected_profile = 'glibc-fma-r3' if args.glibc_rotation else 'native'
    profile = subprocess.run([str(binary), '--rotation-profile'], cwd=ROOT,
                             env=env, capture_output=True, text=True, check=True).stdout.strip()
    if profile != expected_profile:
        raise RuntimeError(f'backend profile mismatch: expected {expected_profile}, got {profile}')
    env['STKCRAFT_SPK'] = ','.join(map(str, kernels))
    comparison.WGC_OUTPUT_EXACT = True
    comparison.STRICT = True
    comparison.STRICT_FAILURES = []
    wgc = comparison.Wgc(False, 120, replay=TRACE)
    queries, reference = [], []
    for target, tid, illuminator, iid, observer, oid in CASES:
        for correction in CORRECTIONS:
            payload = {'calculationType': 'PHASE_ANGLE',
                       'kernels': [{'type': 'KERNEL_SET', 'id': 1}],
                       'target': target, 'illuminator': illuminator,
                       'observer': observer, 'aberrationCorrection': correction,
                       **comparison.time_block(TIMES)}
            result = comparison.reference_result(wgc, payload)
            for et, row in zip(TIMES, result['rows']):
                queries.append(f'{tid},{iid},{oid},{et!r},{correction}')
                reference.append(row)
    process = subprocess.run([str(binary), 'phase'], cwd=ROOT, env=env,
                             input='\n'.join(queries)+'\n', text=True,
                             capture_output=True, check=True)
    values = [None if line.startswith('ERROR') else line.split(',')
              for line in process.stdout.splitlines() if line.strip()]
    native_errors = []
    native = None
    if args.native_capture:
        native = json.loads(args.native_capture.read_text(encoding='utf8'))
        local_kernels = [{'name': p.name, 'sha256': digest(p)} for p in kernels]
        if native['kernels_in_load_order'] != local_kernels or len(native['rows']) != len(queries):
            raise RuntimeError('native capture kernel order, hashes, or row count do not match')
    for index, (query, row, actual) in enumerate(comparison.verified_pairs(queries, reference, values, 'phase')):
        comparison.strict_difference(actual[6], row[1], query)
        if native is not None:
            captured = native['rows'][index]
            if [str(value) for value in captured['query']] != query.split(','):
                raise RuntimeError(f'native capture query differs: {query}')
            actual_bits = struct.pack('>d', float(actual[5]))
            native_bits = struct.pack('>d', float.fromhex(captured['native_rad']))
            if actual_bits != native_bits:
                native_errors.append(f'{query}: native angle bits differ')
    report = {'scope': 'Exact WGC displayed values, not WGC raw binary64',
              'server_kernel_bytes_verified': False,
              'binary_sha256': digest(binary), 'verified_profile': profile,
              'kernels_in_load_order': [{'name': p.name, 'sha256': digest(p)} for p in kernels],
              'trace_sha256': digest(TRACE), 'rows': len(queries),
              'errors': comparison.STRICT_FAILURES, 'native_errors': native_errors,
              'native_capture_sha256': digest(args.native_capture) if args.native_capture else None}
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2)+'\n', encoding='utf8')
    print(f"{len(queries)} rows; {len(comparison.STRICT_FAILURES)} WGC display and {len(native_errors)} native bit differences")
    return int(bool(comparison.STRICT_FAILURES or native_errors))


if __name__ == '__main__':
    raise SystemExit(main())
