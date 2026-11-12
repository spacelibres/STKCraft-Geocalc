"""Capture native fixed/dynamic frame error timing and kernel-date handoff."""
from __future__ import annotations
import argparse
from datetime import datetime, timezone
import json
from pathlib import Path
import subprocess

from gen_kernel_pool import ROOT, compile_native, digest, kernel


def models() -> list[tuple[str, str, str]]:
    fixed = {
        'FRAME_CHECK_FRAME': '963101', 'FRAME_963101_NAME': "'CHECK_FRAME'",
        'FRAME_963101_CLASS': '4', 'FRAME_963101_CLASS_ID': '963101',
        'FRAME_963101_CENTER': '399', 'TKFRAME_963101_RELATIVE': "'J2000'",
        'TKFRAME_963101_SPEC': "'ANGLES'", 'TKFRAME_963101_ANGLES': '(11.3 -7.2 19.1)',
        'TKFRAME_963101_AXES': '(3 2 1)', 'TKFRAME_963101_UNITS': "'DEGREES'",
    }
    dynamic = {
        'FRAME_CHECK_FRAME': '963101', 'FRAME_963101_NAME': "'CHECK_FRAME'",
        'FRAME_963101_CLASS': '5', 'FRAME_963101_CLASS_ID': '963101',
        'FRAME_963101_CENTER': '399', 'FRAME_963101_RELATIVE': "'J2000'",
        'FRAME_963101_DEF_STYLE': "'PARAMETERIZED'", 'FRAME_963101_FAMILY': "'EULER'",
        'FRAME_963101_EPOCH': '123.456', 'FRAME_963101_AXES': '(3 2 1)',
        'FRAME_963101_UNITS': "'DEGREES'", 'FRAME_963101_ROTATION_STATE': "'ROTATING'",
        'FRAME_963101_ANGLE_1_COEFFS': '(11.3 .001)',
        'FRAME_963101_ANGLE_2_COEFFS': '(-7.2 .002)',
        'FRAME_963101_ANGLE_3_COEFFS': '(19.1 -.003)',
    }
    records = []
    for family, base in [('fixed', fixed), ('dynamic', dynamic)]:
        for field, original in base.items():
            if field == 'FRAME_CHECK_FRAME':
                continue
            variants = [('missing', None), ('number', '7'), ('character', "'7'"),
                        ('number_array', '(7 8)'), ('character_array', "('7' '8')")]
            for label, value in variants:
                if value == original:
                    continue
                values = dict(base)
                if value is None:
                    values.pop(field)
                else:
                    values[field] = value
                name = family + '_' + field.replace('963101_', '') + '_' + label
                text = kernel('\n'.join(key+'='+item for key, item in values.items()))
                records.append((name, text, 'frame'))
        values = dict(base)
        field = 'TKFRAME_963101_SPEC' if family == 'fixed' else 'FRAME_963101_FAMILY'
        values[field] = "'MYSTERY'"
        records.append((family+'_unsupported', kernel('\n'.join(k+'='+v for k,v in values.items())), 'frame'))
    for name, literal in [
        ('j2000_noon', '@2000-JAN-01-12:00:00'), ('j2000_midnight', '@2000-JAN-01'),
        ('calibration_1', '@31-JAN-1987'), ('calibration_2', '@feb/4/1987'),
        ('calibration_3', '@March-7-1987-3:10:39.221'), ('leap_day', '@2000-FEB-29-23:59:59.999'),
        ('ordinal_day', '@2000-060'), ('historical', '@1965-JUL-01'),
        ('invalid_day', '@2001-FEB-29'), ('empty', '@'), ('invalid_text', '@NOT_A_DATE')
    ]:
        records.append(('date_'+name, kernel('EPOCH='+literal), 'date'))
    records.append(('legacy_class2_tk_spec', """
\begindata
FRAME_X                 = 1400201
FRAME_1400201_NAME      = 'X'
FRAME_1400201_CLASS     = 2
FRAME_1400201_CLASS_ID  = 1400201
FRAME_1400201_CENTER    = 399
TKFRAME_1400201_RELATIVE = 'J2000'
TKFRAME_1400201_SPEC    = 'MYSTERY'
\begintext
""".replace('\x08', '\\b'), 'legacy'))
    records.append(('builtin_id_override', kernel("""FRAME_CHECK_FRAME=1
FRAME_1_NAME='CHECK_FRAME'
FRAME_1_CLASS=4
FRAME_1_CLASS_ID=1
FRAME_1_CENTER=399
TKFRAME_1_RELATIVE='ECLIPJ2000'
TKFRAME_1_SPEC='ANGLES'
TKFRAME_1_ANGLES=(13 17 19)
TKFRAME_1_AXES=(3 2 1)
TKFRAME_1_UNITS='DEGREES'"""), 'builtin'))
    values=dict(fixed);values['FRAME_J2000']='963101'
    records.append(('builtin_name_override',kernel('\n'.join(k+'='+v for k,v in values.items())),'builtin'))
    return records


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output-dir', type=Path, required=True)
    args = parser.parse_args()
    dest = args.output_dir.resolve(); dest.mkdir(parents=True, exist_ok=False)
    (dest/'kernels').mkdir()
    provenance = compile_native(dest)
    commands, rows, hashes = [], [], {}
    for name, text, kind in models():
        path=dest/'kernels'/(name+'.tf'); path.write_bytes(text.encode('ascii'))
        hashes[path.name]=digest(path)
        commands+=['C','L|'+str(path)]
        rows += [(name,'C'),(name,'L|'+path.name)]
        queries = ['D|EPOCH'] if kind=='date' else [
            'R|CHECK_FRAME|J2000|0000000000000000',
            'S|J2000|CHECK_FRAME|0000000000000000',
            'R|CHECK_FRAME|CHECK_FRAME|0000000000000000',
            'S|CHECK_FRAME|CHECK_FRAME|0000000000000000',
        ]
        if kind=='legacy':
            queries=[q.replace('CHECK_FRAME','X') for q in queries]
        if kind=='builtin':
            queries=['R|J2000|ECLIPJ2000|0000000000000000',
                     'S|J2000|ECLIPJ2000|0000000000000000',
                     'R|J2000|J2000|0000000000000000',
                     'S|J2000|J2000|0000000000000000']
        commands+=queries; rows += [(name,q) for q in queries]
    data=('\n'.join(commands)+'\n').encode('ascii')
    result=subprocess.run([str(dest/'kernel-pool-oracle.exe')],input=data,capture_output=True)
    for name, raw in [('stdin',data),('stdout',result.stdout),('stderr',result.stderr)]:
        (dest/('native-'+name+'.bin')).write_bytes(raw)
    if result.returncode or not result.stderr.startswith(b'CRT_FMA:1\r\n'):
        raise RuntimeError('The pinned independent native capture failed')
    replies=result.stdout.decode('ascii').splitlines()
    if len(replies)!=len(rows):raise ValueError('Native response count mismatch')
    output='case\tcommand\tnative\n'+''.join(a+'\t'+b+'\t'+c+'\n' for (a,b),c in zip(rows,replies))
    (dest/'frame_kernel_errors.tsv').write_bytes(output.encode('ascii'))
    provenance.update(captured_utc=datetime.now(timezone.utc).isoformat(),
        checkout_commit=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
        wrapper_sha256=digest(Path(__file__)),cases=len(models()),operations=len(rows),
        fixture_sha256=digest(dest/'frame_kernel_errors.tsv'),kernel_sha256=hashes,
        native_stdout_sha256=digest(dest/'native-stdout.bin'),native_stderr_sha256=digest(dest/'native-stderr.bin'))
    (dest/'capture.json').write_text(json.dumps(provenance,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({'cases':len(models()),'operations':len(rows),'fixture_sha256':provenance['fixture_sha256']}))


if __name__=='__main__':main()
