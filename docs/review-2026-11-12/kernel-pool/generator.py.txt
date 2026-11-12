"""Capture N0067 kernel-pool behavior into a new directory before repairs."""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess
import zipfile

from capture_runtime_math import CSPICE_LIB_SHA256, STATIC_UCRT_SHA256, digest
from runtime_profile import linked_libraries, pe_imports

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / 'tools/reference/kernel_pool_oracle.c'


def compile_native(dest: Path) -> dict:
    manifest = json.loads((ROOT / 'docs/reference/CSPICE_N0067.json').read_text())
    archive = ROOT / 'target/reference/cspice-n0067.zip'
    library = ROOT / 'target/reference/n0067/cspice/lib/cspice.lib'
    ucrt = Path(os.environ['ProgramFiles(x86)']) / 'Windows Kits/10/Lib/10.0.26100.0/ucrt/x64/libucrt.lib'
    if digest(archive) != manifest['archive_sha256'] or digest(library) != CSPICE_LIB_SHA256:
        raise ValueError('The pinned official CSPICE bytes are unavailable')
    if digest(ucrt) != STATIC_UCRT_SHA256:
        raise ValueError('The independent static CRT capture profile differs')
    with zipfile.ZipFile(archive) as package:
        if library.read_bytes() != package.read('cspice/lib/cspice.lib'):
            raise ValueError('The native library differs from the official archive')
        sources = {}
        for name in ['pool.c', 'zzrvar.c', 'zzcln.c', 'nparsd.c', 'tparse.c',
                     'tkfram.c', 'framex.c', 'zznamfrm.c', 'getfov_c.c', 'bodvrd_c.c']:
            path = ROOT / 'target/reference/n0067/cspice/src/cspice' / name
            if path.read_bytes() != package.read('cspice/src/cspice/' + name):
                raise ValueError('Native source differs: ' + name)
            sources[name] = digest(path)
    vswhere = Path(os.environ['ProgramFiles(x86)']) / 'Microsoft Visual Studio/Installer/vswhere.exe'
    install = Path(subprocess.check_output([str(vswhere), '-latest', '-products', '*', '-requires',
                   'Microsoft.VisualStudio.Component.VC.Tools.x86.x64', '-property', 'installationPath'], text=True).strip())
    environment = install / 'VC/Auxiliary/Build/vcvarsall.bat'
    executable = dest / 'kernel-pool-oracle.exe'
    include = ROOT / 'target/reference/n0067/cspice/include'
    for path in [environment, SOURCE, library, executable, include, dest, ucrt]:
        if any(c in str(path) for c in ['"', '%', '\n', '\r', '&', '|', '<', '>', '^', '!']):
            raise ValueError('Unsupported command character in capture path')
    script = dest / 'build.cmd'
    script.write_text('@echo off\ncall "' + str(environment) + '" x64 >nul\nif errorlevel 1 exit /b 1\n'
        + 'where cl > "' + str(dest / 'compiler.txt') + '"\nif errorlevel 1 exit /b 1\n'
        + 'cl /nologo /Bv /Od /fp:strict /MT /I"' + str(include) + '" "' + str(SOURCE)
        + '" "' + str(library) + '" "' + str(ucrt) + '" /Fe:"' + str(executable)
        + '" /Fo:"' + str(dest / 'oracle.obj') + '" /link /Brepro /LIBPATH:"' + str(ucrt.parent)
        + '" /LINKREPROFULLPATHRSP:"' + str(dest / 'actual-link.rsp') + '"\n', encoding='utf-8')
    build = subprocess.run(['cmd', '/d', '/c', str(script)], capture_output=True)
    (dest / 'compiler-stdout.bin').write_bytes(build.stdout)
    (dest / 'compiler-stderr.bin').write_bytes(build.stderr)
    if build.returncode:
        raise RuntimeError('Native compilation failed; original compiler diagnostics are preserved')
    libraries = linked_libraries(dest / 'actual-link.rsp')
    selected = [row for row in libraries if Path(row['path']).name.lower() == 'libucrt.lib']
    if len(selected) != 1 or selected[0]['sha256'] != STATIC_UCRT_SHA256:
        raise ValueError('The actual linked CRT differs from the capture profile')
    imports = pe_imports(executable)
    if any('ucrt' in name.lower() or 'vcruntime' in name.lower() or name.lower() == 'msvcrt.dll' for name in imports):
        raise ValueError('The C oracle imports dynamic CRT math')
    compiler = Path((dest / 'compiler.txt').read_text().splitlines()[0])
    return dict(archive_sha256=digest(archive), cspice_lib_sha256=digest(library),
                static_ucrt_sha256=digest(ucrt), source_sha256=digest(SOURCE),
                generator_sha256=digest(Path(__file__)), compiler_sha256=digest(compiler),
                linked_libraries=libraries, imports=imports, native_source_sha256=sources,
                executable_sha256=digest(executable))


def kernel(lines: str, kind: str = 'FK') -> str:
    return 'KPL/' + kind + '\n\\begindata\n' + lines.strip() + '\n\\begintext\n'


def cases() -> list[tuple[str, list[tuple[str, str]]]]:
    records = []
    def add(name, text, queries, second=None):
        steps = [('L', kernel(text)), *[(q, '') for q in queries]]
        if second is not None:
            steps += [('L', kernel(second)), *[(q, '') for q in queries]]
        records.append((name, steps))
    add('numeric_append', 'A = 1.1\nA += ( -2D0, -0.0 )\nA += PI', ['D|A'])
    add('append_new', 'A += (1 2)\nB += 3', ['D|A', 'D|B'])
    add('replacement_order', 'A = (1 2)\nA += 3\nA = 4\nA += 5', ['D|A'])
    add('across_loads', 'A=(1 2)\nB=4', ['D|A', 'D|B'], 'A += (3 4)\nB = 5')
    add('character_append', "A='123'\nA += ('1D3' 'a=b,(x)')\nA += 'can''t'", ['D|A'])
    add('case_sensitive', 'A=1\na=2\nA+=3', ['D|A', 'D|a'])
    add('type_replace', "A=1\nA='123'\nA+='456'", ['D|A'], 'A=(7 8)')
    add('mixed_numeric', "KEPT=17\nA=(1 '2')\nAFTER=19", ['D|KEPT', 'D|A', 'D|AFTER'])
    add('mixed_character', "KEPT=17\nA=('1' 2)\nAFTER=19", ['D|KEPT', 'D|A', 'D|AFTER'])
    add('append_type_change', 'A=(1 2)', ['D|A'], "KEPT=17\nA+='3'\nAFTER=19")
    add('append_numeric_to_character', "A='1'", ['D|A'], 'A+=2')
    for name, value in [('empty_string', "''"), ('blank_string', "' '"),
                        ('empty_vector', '()'), ('unclosed_vector', '(1 2'),
                        ('unclosed_quote', "'abc"), ('double_quote', '"abc"'),
                        ('bad_number', '1E-'), ('bare_word', 'abc'), ('nested', '((1 2))')]:
        add(name, f'KEPT=17\nA={value}\nAFTER=19', ['D|KEPT', 'D|A', 'D|AFTER'])
    for name, operator in [('separate_plus', ' +='), ('separate_equals', '+ ='), ('spaced_both', ' + =')]:
        add(name, 'A=1\nA'+operator+'2', ['D|A', 'D|A+'])
    add('tabs', 'A\t=\t(1\t2\t3)', ['D|A'])
    add('same_line', 'A=1 B=2 A+=3', ['D|A', 'D|B'])
    base = """FRAME_POOL_FRAME=963001
FRAME_963001_NAME='POOL_FRAME'
FRAME_963001_CLASS=4
FRAME_963001_CLASS_ID=963001
FRAME_963001_CENTER=399
TKFRAME_963001_RELATIVE='J2000'
TKFRAME_963001_SPEC='ANGLES'
TKFRAME_963001_UNITS='DEGREES'
"""
    angles = 'TKFRAME_963001_ANGLES=11.3\nTKFRAME_963001_ANGLES+=(-7.2 19.1)\nTKFRAME_963001_AXES=3\nTKFRAME_963001_AXES+=(2 1)'
    queries = ['D|TKFRAME_963001_ANGLES', 'N|POOL_FRAME', 'I|963001', 'R|POOL_FRAME|J2000|0000000000000000', 'S|J2000|POOL_FRAME|0000000000000000']
    add('frame_append', base+angles, queries)
    add('frame_partial_update', base+angles, queries, 'TKFRAME_963001_ANGLES=(3 7 13)')
    add('frame_cross_file_append', base+'TKFRAME_963001_ANGLES=11.3\nTKFRAME_963001_AXES=3', [], 'TKFRAME_963001_ANGLES+=(-7.2 19.1)\nTKFRAME_963001_AXES+=(2 1)')
    records[-1][1].extend((q, '') for q in queries)
    add('frame_integer_notation', (base+angles).replace('CLASS=4', 'CLASS=4D0').replace('CLASS_ID=963001', 'CLASS_ID=963001.0'), queries)
    add('frame_lower_alias', (base+angles).replace('FRAME_POOL_FRAME=', 'FRAME_pool_frame='), queries)
    add('frame_quoted_axes', (base+angles).replace('AXES=3', "AXES='3'").replace('AXES+=(2 1)', "AXES+=('2' '1')"), queries)
    add('pck_append', 'BODY9999_RADII=3\nBODY9999_RADII+=(2 1)\nBODY9999_GM=123.456', ['D|BODY9999_RADII', 'B|9999|RADII', 'B|9999|GM'], 'BODY9999_RADII=(6 5 4)')
    ik = "INS-96301_FOV_SHAPE='RECTANGLE'\nINS-96301_FOV_FRAME='J2000'\nINS-96301_BORESIGHT=0\nINS-96301_BORESIGHT+=(0 1)\nINS-96301_FOV_BOUNDARY_CORNERS=(1 1 10)\nINS-96301_FOV_BOUNDARY_CORNERS+=(-1 1 10)\nINS-96301_FOV_BOUNDARY_CORNERS+=(-1 -1 10)\nINS-96301_FOV_BOUNDARY_CORNERS+=(1 -1 10)"
    add('fov_append', ik, ['D|INS-96301_BORESIGHT', 'F|-96301'])
    add('numeric_character_metadata', "A=('0001' '1.234567890123456789' '-0.0')", ['D|A'])
    return records


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output-dir', type=Path, required=True)
    args = parser.parse_args()
    dest = args.output_dir.resolve()
    dest.mkdir(parents=True, exist_ok=False)
    (dest / 'kernels').mkdir()
    provenance = compile_native(dest)
    commands, rows, kernel_hashes = [], [], {}
    for case, steps in cases():
        commands.append('C'); rows.append((case, 'C'))
        loaded = 0
        for command, text in steps:
            if command == 'L':
                name = case + '-' + str(loaded) + '.tf'; loaded += 1
                path = dest / 'kernels' / name
                path.write_bytes(text.encode('ascii'))
                kernel_hashes[name] = digest(path)
                commands.append('L|' + str(path))
                rows.append((case, 'L|' + name))
            else:
                commands.append(command); rows.append((case, command))
    raw_input = ('\n'.join(commands) + '\n').encode('ascii')
    native = subprocess.run([str(dest / 'kernel-pool-oracle.exe')], input=raw_input, capture_output=True)
    (dest / 'native-stdin.bin').write_bytes(raw_input)
    (dest / 'native-stdout.bin').write_bytes(native.stdout)
    (dest / 'native-stderr.bin').write_bytes(native.stderr)
    if native.returncode or not native.stderr.startswith(b'CRT_FMA:1\r\n'):
        raise RuntimeError('The independent native capture failed')
    replies = native.stdout.decode('ascii').splitlines()
    if len(rows) != len(replies):
        raise ValueError('The native response count differs from the inputs')
    output = 'case\tcommand\tnative\n' + ''.join(case+'\t'+command+'\t'+reply+'\n'
              for (case, command), reply in zip(rows, replies))
    (dest / 'kernel_pool.tsv').write_bytes(output.encode('ascii'))
    provenance.update(captured_utc=datetime.now(timezone.utc).isoformat(),
        checkout_commit=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
        cases=len(cases()), operations=len(rows), fixture_sha256=digest(dest / 'kernel_pool.tsv'),
        kernel_sha256=kernel_hashes, native_stdout_sha256=digest(dest / 'native-stdout.bin'),
        native_stderr_sha256=digest(dest / 'native-stderr.bin'))
    (dest / 'capture.json').write_text(json.dumps(provenance, indent=2)+'\n', encoding='utf-8')
    print(json.dumps({'cases':len(cases()), 'operations':len(rows), 'fixture_sha256':provenance['fixture_sha256']}))


if __name__ == '__main__':
    main()
