"""Capture independent C math bits into a new directory; never overwrite fixtures.

Requires the pinned Windows SDK 10.0.26100.0 static UCRT and official N0067
archive. The C executable receives and returns binary doubles, so no decimal
formatting or Rust calculation participates in the oracle.
"""
from __future__ import annotations

import argparse
import csv
from datetime import datetime, timezone
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import random
import struct
import subprocess
import sys
import zipfile

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / 'tools/reference/runtime_math_oracle.c'
STATIC_UCRT_SHA256 = 'ea2ffa5d9b1ebc3fa76f34c364471750db8951b341dbccbc1bef628ef1a20e39'
CSPICE_LIB_SHA256 = '114dc34aea8533bcdd452997b9096219c3697234a00dfe59a4ba1d87731a9650'
SEED = 16702467
OPERATIONS = ['asin', 'acos', 'atan2', 'cos', 'cosh', 'exp', 'log', 'pow', 'sin', 'sinh', 'sqrt', 'tan', 'sin_cos']


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def bits(value: float) -> int:
    return struct.unpack('<Q', struct.pack('<d', value))[0]


def inputs(operation: str, rng: random.Random) -> list[tuple[float, float]]:
    pairs = []
    for _ in range(1000):
        if operation in ['asin', 'acos']:
            pair = (rng.uniform(-1, 1), 0.)
        elif operation in ['sin', 'cos', 'tan', 'sin_cos']:
            pair = (rng.uniform(-1e12, 1e12), 0.)
        elif operation in ['cosh', 'sinh', 'exp']:
            pair = (rng.uniform(-700, 700), 0.)
        elif operation in ['log', 'sqrt']:
            pair = (math.ldexp(rng.uniform(1., 2.), rng.randrange(-1000, 1000)), 0.)
        elif operation == 'pow':
            pair = (math.ldexp(rng.uniform(1., 2.), rng.randrange(-400, 400)), rng.uniform(-2, 2))
        else:
            pair = (rng.uniform(-1, 1), rng.uniform(-1, 1))
        pairs.append(pair)
    minimum = float.fromhex('0x0.0000000000001p-1022')
    if operation in ['asin', 'acos', 'sin', 'cos', 'tan', 'sin_cos', 'sqrt']:
        pairs.extend((x, 0.) for x in [-0., 0., minimum, 1e-300, 1.])
    if operation in ['asin', 'acos']:
        pairs.extend([(math.nextafter(-1., 0.), 0.), (-1., 0.), (math.nextafter(1., 0.), 0.)])
        with (ROOT / 'tests/golden/orientation_exact.csv').open(encoding='utf-8', newline='') as source:
            rows = csv.reader(source)
            next(rows)
            for row in rows:
                for cell in row[5:14]:
                    value = float(cell)
                    if -1 <= value <= 1:
                        pairs.append((value, 0.))
    if operation == 'atan2':
        pairs.extend((x, y) for x in [0., -0., 1., -1., minimum, 1e300]
                     for y in [0., -0., 1., -1., minimum, -1e300])
    if operation == 'pow':
        pairs.extend([(0., 0.), (0., 1.), (-0., 1.), (1., 0.), (-1., 2.), (-1., 3.), (1e-300, .5), (1e300, .5)])
    if operation == 'log':
        pairs.extend((x, 0.) for x in [minimum, 1e-300, 1., math.nextafter(1., 0.), math.nextafter(1., 2.)])
    if operation == 'exp':
        pairs.extend((x, 0.) for x in [-745., -744., -0., 0., 1.])
    return list({(bits(x), bits(y)): (x, y) for x, y in pairs}.values())


def compile_oracle(dest: Path) -> tuple[Path, dict]:
    kits = Path(os.environ['ProgramFiles(x86)']) / 'Windows Kits/10/Lib/10.0.26100.0'
    ucrt = kits / 'ucrt/x64/libucrt.lib'
    if digest(ucrt) != STATIC_UCRT_SHA256:
        raise ValueError('Static UCRT bytes differ from the independent capture profile')
    manifest = json.loads((ROOT / 'docs/reference/CSPICE_N0067.json').read_text(encoding='utf-8'))
    archive = ROOT / 'target/reference/cspice-n0067.zip'
    if digest(archive) != manifest['archive_sha256']:
        raise ValueError('Official CSPICE archive checksum differs')
    library = ROOT / 'target/reference/n0067/cspice/lib/cspice.lib'
    with zipfile.ZipFile(archive) as original:
        if library.read_bytes() != original.read('cspice/lib/cspice.lib'):
            raise ValueError('CSPICE library differs from the original archive')
    if digest(library) != CSPICE_LIB_SHA256:
        raise ValueError('Unexpected CSPICE library bytes')
    vswhere = Path(os.environ['ProgramFiles(x86)']) / 'Microsoft Visual Studio/Installer/vswhere.exe'
    installation = Path(subprocess.check_output([str(vswhere), '-latest', '-products', '*', '-requires',
                        'Microsoft.VisualStudio.Component.VC.Tools.x86.x64', '-property', 'installationPath'], text=True).strip())
    environment = installation / 'VC/Auxiliary/Build/vcvarsall.bat'
    if not environment.is_file():
        raise ValueError('MSVC x64 environment is unavailable')
    executable = dest / 'runtime-math-oracle.exe'
    include = ROOT / 'target/reference/n0067/cspice/include'
    paths = [environment, SOURCE, library, executable, include, dest, ucrt.parent]
    if any(any(char in str(path) for char in ['"', '%', '\n', '\r', '&', '|', '<', '>', '^', '!']) for path in paths):
        raise ValueError('Build paths contain unsupported command characters')
    script = dest / 'build.cmd'
    script.write_text('@echo off\ncall "' + str(environment) + '" x64 >nul\nif errorlevel 1 exit /b 1\nset VSLANG=1033\n'
        + 'where cl > "' + str(dest / 'compiler-paths.txt') + '"\nif errorlevel 1 exit /b 1\n'
        + 'cl /nologo /Bv /Od /fp:strict /MT /I"' + str(include) + '" "' + str(SOURCE) + '" "' + str(library)
        + '" "' + str(ucrt) + '" /Fe:"' + str(executable) + '" /Fo:"' + str(dest / 'oracle.obj') + '" /link /Brepro /LIBPATH:"'
        + str(ucrt.parent) + '" /LINKREPROFULLPATHRSP:"' + str(dest / 'actual-link.rsp') + '"\n', encoding='utf-8')
    result = subprocess.run(['cmd', '/d', '/c', str(script)], capture_output=True, check=True)
    (dest / 'compiler-stdout.bin').write_bytes(result.stdout)
    (dest / 'compiler-stderr.bin').write_bytes(result.stderr)
    compiler_output = (result.stdout + result.stderr).decode('utf-8')
    (dest / 'build.log').write_text(compiler_output, encoding='utf-8')
    compiler = Path((dest / 'compiler-paths.txt').read_text(encoding='utf-8').splitlines()[0])
    response = (dest / 'actual-link.rsp').read_bytes()
    response = response.decode('utf-16' if response.startswith(b'\xff\xfe') else 'utf-8-sig')
    linked_ucrt = []
    for line in response.splitlines():
        value = line.strip().strip('"')
        if value.lower().startswith('/defaultlib:'):
            value = value[len('/defaultlib:'):]
        if value.lower().endswith('libucrt.lib'):
            linked_ucrt.append(Path(value))
    if len(linked_ucrt) != 1 or digest(linked_ucrt[0]) != STATIC_UCRT_SHA256:
        raise ValueError('The actual linked static UCRT differs from the capture profile')
    # /MT must remove the system UCRT dependency rather than merely add a library.
    sys.path.insert(0, str(ROOT / 'tools'))
    from diagnose_runtime_math import pe_imports
    imports = pe_imports(executable)
    if any('ucrt' in name.lower() or 'crt-math' in name.lower() for name in imports):
        raise ValueError('Independent oracle unexpectedly imports dynamic UCRT math')
    for arguments, payload, expected in [([], b'', 5), (['unknown'], b'', 5), (['asin'], b'\0', 2)]:
        result = subprocess.run([str(executable), *arguments], input=payload, capture_output=True)
        if result.returncode != expected:
            raise ValueError('Independent oracle rejected malformed input incorrectly')
    return executable, dict(static_ucrt_sha256=digest(ucrt), static_ucrt_sdk='10.0.26100.0',
                            cspice_lib_sha256=digest(library), archive_sha256=digest(archive), imports=imports,
                            compiler_path=str(compiler), compiler_sha256=digest(compiler),
                            compiler_output=compiler_output,
                            compiler_stdout_sha256=hashlib.sha256(result.stdout).hexdigest(),
                            compiler_stderr_sha256=hashlib.sha256(result.stderr).hexdigest())


def audit_original_fixtures(oracle: Path) -> dict:
    counts = {}
    for operation, filename, width, expected_rows in [
        ('vsep', 'two_directions_vsep_precision.csv', 6, 204),
        ('prop2b', 'prop2b.csv', 8, 288),
    ]:
        with (ROOT / 'tests/golden' / filename).open(encoding='utf-8', newline='') as source:
            rows = list(csv.reader(source))
        if operation == 'prop2b':
            rows = rows[1:]
        values = [list(map(float, row)) for row in rows]
        payload = b''.join(struct.pack('<' + 'd' * width, *row[:width]) for row in values)
        result = subprocess.run([str(oracle), operation], input=payload, capture_output=True, check=True)
        expected = b''.join(struct.pack('<' + 'd' * (len(row) - width), *row[width:]) for row in values)
        if len(rows) != expected_rows or result.stdout != expected or result.stderr != b'CRT_FMA:1\r\n':
            raise ValueError('Independent static CSPICE differs from original fixture bits: ' + operation)
        counts[operation] = dict(rows=len(rows), mismatches=0)
    return counts


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output-dir', type=Path, required=True)
    args = parser.parse_args()
    if sys.platform != 'win32' or struct.calcsize('P') != 8:
        raise SystemExit('The independent capture profile requires x86_64 Windows')
    dest = args.output_dir.resolve()
    if dest.exists() or dest == ROOT or dest.is_relative_to(ROOT / 'tests') or dest.is_relative_to(ROOT / 'docs'):
        raise ValueError('Capture requires a new directory outside maintained fixtures/evidence')
    dest.mkdir(parents=True)
    oracle, provenance = compile_oracle(dest)
    original_fixtures = audit_original_fixtures(oracle)
    rng = random.Random(SEED)
    records, counts, states = [], {}, {}
    for operation in OPERATIONS:
        pairs = inputs(operation, rng)
        payload = b''.join(struct.pack('<dd', x, y) for x, y in pairs)
        result = subprocess.run([str(oracle), operation], input=payload, capture_output=True, check=True)
        width = 2 if operation == 'sin_cos' else 1
        if len(result.stdout) != len(pairs) * width * 8 or result.stderr != b'CRT_FMA:1\r\n':
            raise ValueError('Unexpected independent output width or native dispatch: ' + operation)
        states[operation] = result.stderr.decode().strip()
        for index, (x, y) in enumerate(pairs):
            raw = result.stdout[index * width * 8:(index + 1) * width * 8]
            if not all(math.isfinite(value) for value in struct.unpack('<' + 'd' * width, raw)):
                raise ValueError('Nonfinite native output cannot establish exactness')
            expected = [f'{int.from_bytes(raw[cell * 8:(cell + 1) * 8], "little"):016x}' for cell in range(width)]
            records.append([operation, f'{bits(x):016x}', f'{bits(y):016x}', expected[0], expected[1] if width == 2 else ''])
        counts[operation] = len(pairs)
    path = dest / 'runtime_math.csv'
    with path.open('w', newline='', encoding='utf-8') as output:
        writer = csv.writer(output, lineterminator='\n')
        writer.writerow(['operation', 'x_bits', 'y_bits', 'expected_1_bits', 'expected_2_bits'])
        writer.writerows(records)
    report = dict(captured_utc=datetime.now(timezone.utc).isoformat(),
                  checkout_commit=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
                  profile='Windows x86_64 static UCRT /MT /fp:strict, default FMA3 enabled',
                  seed=SEED, rows=len(records), output_cells=sum(2 if row[0] == 'sin_cos' else 1 for row in records),
                  counts=counts, fixture_sha256=digest(path), source_sha256=digest(SOURCE),
                  oracle_sha256=digest(oracle), platform=platform.platform(), native_states=states,
                  generator_sha256=digest(Path(__file__)), python=sys.version,
                  original_cspice_fixtures=original_fixtures,
                  input_source_sha256={'tests/golden/' + name: digest(ROOT / 'tests/golden' / name)
                                       for name in ['orientation_exact.csv', 'two_directions_vsep_precision.csv', 'prop2b.csv']},
                  **provenance)
    (dest / 'report.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(report, indent=2))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
