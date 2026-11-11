"""Record native fixture drift and C/Rust UCRT dispatch without editing expectations.

This is a diagnostic, not an acceptance gate. A completed report does not certify
numerical conformance. FMA changes are confined to disposable probe processes.
"""
from __future__ import annotations

import argparse
import csv
import ctypes
import hashlib
import json
import os
from pathlib import Path
import platform
import struct
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools'))
from bootstrap_reference import resolve_reference_binary

FUNCTIONS = ['asin', 'acos', 'sin', 'cos', 'atan2', 'log', 'exp', 'sinh', 'cosh', 'pow']


def bits(value: float) -> int:
    return struct.unpack('>Q', struct.pack('>d', value))[0]


def rows(path: Path):
    with path.open(encoding='utf-8', newline='') as source:
        yield from csv.reader(source)


def pe_imports(path: Path) -> list[str]:
    """Read import DLL names from the trusted diagnostic PE64 executable."""
    data = path.read_bytes()
    header = struct.unpack_from('<I', data, 60)[0]
    assert data[header:header + 4] == b'PE\0\0'
    count = struct.unpack_from('<H', data, header + 6)[0]
    optional_size = struct.unpack_from('<H', data, header + 20)[0]
    optional = header + 24
    assert struct.unpack_from('<H', data, optional)[0] == 0x20b
    sections = []
    for index in range(count):
        entry = optional + optional_size + index * 40
        virtual_size, address, raw_size, raw_offset = struct.unpack_from('<IIII', data, entry + 8)
        sections.append((address, max(virtual_size, raw_size), raw_offset))

    def offset(rva: int) -> int:
        for address, length, raw_offset in sections:
            if address <= rva < address + length:
                return raw_offset + rva - address
        raise ValueError('Diagnostic import RVA is outside PE sections.')

    import_rva = struct.unpack_from('<I', data, optional + 120)[0]
    result = []
    if import_rva:
        entry = offset(import_rva)
        while any(data[entry:entry + 20]):
            name = offset(struct.unpack_from('<I', data, entry + 12)[0])
            result.append(data[name:data.index(b'\0', name)].decode('ascii'))
            entry += 20
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=ROOT / 'target/runtime-math/report.json')
    args = parser.parse_args()
    if sys.platform != 'win32' or ctypes.sizeof(ctypes.c_void_p) != 8:
        raise SystemExit('The UCRT diagnostic requires x86_64 Windows.')
    manifest = json.loads((ROOT / 'docs/reference/CSPICE_N0067.json').read_text(encoding='utf-8'))
    native_path = resolve_reference_binary(manifest)
    native_sha256 = hashlib.sha256(native_path.read_bytes()).hexdigest()
    if native_sha256 != manifest['reference_binary']['sha256']:
        raise RuntimeError('Native diagnostic DLL differs from the pinned reference.')
    native = ctypes.CDLL(str(native_path))
    crt_path = Path(os.environ['SystemRoot']) / 'System32/ucrtbase.dll'
    crt = ctypes.CDLL(str(crt_path))
    crt._get_FMA3_enable.restype = ctypes.c_int
    crt._set_FMA3_enable.argtypes = [ctypes.c_int]
    crt._set_FMA3_enable.restype = ctypes.c_int
    original = crt._get_FMA3_enable()
    pointer = ctypes.POINTER(ctypes.c_double)
    native.vsep_c.argtypes = [pointer, pointer]
    native.vsep_c.restype = ctypes.c_double
    native.prop2b_c.argtypes = [ctypes.c_double, pointer, ctypes.c_double, pointer]
    native.failed_c.restype = ctypes.c_int
    native.reset_c.argtypes = []
    for name in FUNCTIONS:
        function = getattr(crt, name)
        function.argtypes = [ctypes.c_double] * (2 if name in ['atan2', 'pow'] else 1)
        function.restype = ctypes.c_double
    report = {
        'diagnostic_only': True,
        'platform': platform.platform(),
        'python': sys.version,
        'native_path': str(native_path),
        'native_sha256': native_sha256,
        'native_imports': pe_imports(native_path),
        'ucrt_path': str(crt_path),
        'ucrt_sha256': hashlib.sha256(crt_path.read_bytes()).hexdigest(),
        'python_native_default_fma': original,
        'functions': FUNCTIONS,
        'inputs': [0.4348237736593652, 0.6666666666666666],
        'variants': [],
    }
    try:
        for requested in [original, 0, 1]:
            enabled = crt._set_FMA3_enable(requested)
            variant = {'requested': requested, 'enabled': enabled, 'fixtures': {}}
            x, y = report['inputs']
            variant['math_bits'] = [bits(getattr(crt, name)(x, y) if name in ['atan2', 'pow'] else getattr(crt, name)(x)) for name in FUNCTIONS]
            for family, filename in [('vsep', 'two_directions_vsep_precision.csv'), ('prop2b', 'prop2b.csv')]:
                count, mismatches, errors, examples = 0, 0, 0, []
                data = rows(ROOT / 'tests/golden' / filename)
                if family == 'prop2b':
                    next(data)
                for row in data:
                    values = list(map(float, row))
                    if family == 'vsep':
                        actual = [native.vsep_c((ctypes.c_double * 3)(*values[:3]), (ctypes.c_double * 3)(*values[3:6]))]
                        expected = values[6:]
                    else:
                        output = (ctypes.c_double * 6)()
                        native.prop2b_c(values[0], (ctypes.c_double * 6)(*values[1:7]), values[7], output)
                        actual, expected = list(output), values[8:]
                    failed = native.failed_c() != 0
                    if failed:
                        errors += 1
                        native.reset_c()
                    different = failed or list(map(bits, actual)) != list(map(bits, expected))
                    if different:
                        mismatches += 1
                        if len(examples) < 8:
                            examples.append({'row': count, 'input': values[:6] if family == 'vsep' else values[:8], 'actual_bits': list(map(bits, actual)), 'expected_bits': list(map(bits, expected)), 'native_error': failed})
                    count += 1
                variant['fixtures'][family] = {'rows': count, 'mismatches': mismatches, 'errors': errors, 'examples': examples}
            report['variants'].append(variant)
    finally:
        crt._set_FMA3_enable(original)
        assert crt._get_FMA3_enable() == original
    dest = args.output.resolve()
    dest.parent.mkdir(parents=True, exist_ok=True)
    for linkage in ['dynamic', 'static']:
        probe = dest.parent / ('runtime-math-probe-' + linkage + '.exe')
        command = ['rustc', '--edition=2021', '--print=link-args', str(ROOT / 'tools/reference/runtime_math_probe.rs'), '-o', str(probe)]
        if linkage == 'static':
            command += ['-C', 'target-feature=+crt-static']
        link_args = subprocess.check_output(command, text=True)
        output = subprocess.check_output([str(probe), *map(repr, report['inputs'])], text=True)
        report['rust_' + linkage] = {'events': [json.loads(line) for line in output.splitlines()], 'link_args': link_args, 'imports': pe_imports(probe)}
    sdk_root = Path(os.environ.get('ProgramFiles(x86)', 'C:/Program Files (x86)')) / 'Windows Kits/10/Lib'
    report['static_crt_candidates'] = [{'path': str(path), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()} for path in sorted(sdk_root.glob('*/ucrt/x64/libucrt.lib'))]
    report['rustc'] = subprocess.check_output(['rustc', '-Vv'], text=True)
    dest.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({'report': str(dest), 'diagnostic_only': True, 'ucrt_sha256': report['ucrt_sha256'], 'native_default_fma': original, 'rust_runtimes': {kind: report['rust_' + kind]['events'][0] for kind in ['dynamic', 'static']}, 'static_crt_candidates': report['static_crt_candidates'], 'native_fixture_counts': [{'requested': v['requested'], 'enabled': v['enabled'], 'families': {k: {'rows': f['rows'], 'mismatches': f['mismatches'], 'errors': f['errors']} for k, f in v['fixtures'].items()}} for v in report['variants']]}, indent=2))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
