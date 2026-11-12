"""Capture native PCK queries after typed replacement and failed updates."""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import json
from pathlib import Path
import subprocess

from gen_kernel_pool import ROOT, compile_native, digest, kernel


def cases() -> list[tuple[str, str]]:
    return [
        ('replace_radii_character', "BODY499_RADII=('6' '5' '4')"),
        ('replace_gm_character', "BODY499_GM='456'"),
        ('replace_pm_character', "BODY499_PM=('19' '23' '29')"),
        ('replace_ra_character', "BODY499_POLE_RA=('19' '23' '29')"),
        ('replace_dec_character', "BODY499_POLE_DEC=('19' '23' '29')"),
        ('failed_radii_append', "BODY499_GM=456\nBODY499_RADII+='6'\nBODY499_PM=(19 23 29)"),
        ('failed_gm_append', "BODY499_RADII=(6 5 4)\nBODY499_GM+='456'\nBODY499_PM=(19 23 29)"),
        ('failed_pm_append', "BODY499_RADII=(6 5 4)\nBODY499_PM+='19'\nBODY499_GM=456"),
    ]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output-dir', type=Path, required=True)
    dest = parser.parse_args().output_dir.resolve()
    dest.mkdir(parents=True, exist_ok=False)
    (dest / 'kernels').mkdir()
    provenance = compile_native(dest)
    initial = kernel('''BODY499_RADII=(3 2 1)
BODY499_GM=123.456
BODY499_POLE_RA=(11.3 .001 0)
BODY499_POLE_DEC=(-7.2 .002 0)
BODY499_PM=(19.1 -.003 0)''', 'PCK')
    queries = ['B|499|RADII', 'B|499|GM', 'B|499|POLE_RA', 'B|499|POLE_DEC', 'B|499|PM',
               'R|IAU_MARS|J2000|405edd2f1a9fbe77', 'S|J2000|IAU_MARS|405edd2f1a9fbe77']
    commands, rows, hashes = [], [], {}
    for name, replacement in cases():
        commands.append('C'); rows.append((name, 'C'))
        for index, text in enumerate([initial, kernel(replacement, 'PCK')]):
            path = dest / 'kernels' / f'{name}-{index}.tpc'
            path.write_bytes(text.encode('ascii')); hashes[path.name] = digest(path)
            commands.append('L|' + str(path)); rows.append((name, 'L|' + path.name))
            commands.extend(queries); rows.extend((name, q) for q in queries)
    data = ('\n'.join(commands) + '\n').encode('ascii')
    result = subprocess.run([str(dest / 'kernel-pool-oracle.exe')], input=data, capture_output=True)
    for name, raw in [('stdin', data), ('stdout', result.stdout), ('stderr', result.stderr)]:
        (dest / ('native-' + name + '.bin')).write_bytes(raw)
    if result.returncode or not result.stderr.startswith(b'CRT_FMA:1\r\n'):
        raise RuntimeError('The independent native PCK update capture failed')
    replies = result.stdout.decode('ascii').splitlines()
    if len(replies) != len(rows):
        raise ValueError('Native response count mismatch')
    output = 'case\tcommand\tnative\n' + ''.join(a+'\t'+b+'\t'+c+'\n' for (a,b),c in zip(rows,replies))
    (dest / 'pck_pool_updates.tsv').write_bytes(output.encode('ascii'))
    provenance.update(captured_utc=datetime.now(timezone.utc).isoformat(),
        checkout_commit=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
        wrapper_sha256=digest(Path(__file__)), cases=len(cases()), operations=len(rows),
        fixture_sha256=digest(dest/'pck_pool_updates.tsv'), kernel_sha256=hashes,
        native_stdout_sha256=digest(dest/'native-stdout.bin'), native_stderr_sha256=digest(dest/'native-stderr.bin'))
    (dest/'capture.json').write_text(json.dumps(provenance,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({'cases':len(cases()),'operations':len(rows),'fixture_sha256':provenance['fixture_sha256']}))


if __name__ == '__main__':
    main()
