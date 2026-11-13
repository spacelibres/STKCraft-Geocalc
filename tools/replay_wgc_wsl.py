"""Replay the calculator corpus using a Linux binary through WSL.

All paths must be inside this checkout. Supply the remaining arguments from
replay_wgc_calculators.py, including the report and local kernel paths.
"""
import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path

import replay_wgc_calculators as replay
import replay_wgc_earth_frames as earth_frames
import replay_wgc_mars as mars
import wgc2_geometry_compare as comparison

ROOT = Path(__file__).resolve().parents[1]


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--suite', choices=['calculators','earth-frames','mars'], default='calculators')
    parser.add_argument('--wsl-distribution', required=True)
    parser.add_argument('--linux-workspace', required=True)
    parser.add_argument('--linux-binary', type=Path, required=True)
    parser.add_argument('--linux-loader', type=Path, required=True)
    parser.add_argument('--linux-lib-dir', type=Path, required=True)
    parser.add_argument('--report', type=Path, required=True)
    args, replay_args = parser.parse_known_args()
    replay_args += ['--report',str(args.report)]
    root_linux = args.linux_workspace.rstrip('/')

    def linux_path(value):
        if value == '-':
            return value
        path = Path(value).resolve(strict=True)
        try:
            relative = path.relative_to(ROOT)
        except ValueError as error:
            try:
                relative = Path('target/kernels') / path.relative_to((ROOT / 'target/kernels').resolve(strict=True))
            except ValueError:
                raise ValueError(f'path is outside the checkout or its kernel directory: {path}') from error
        return root_linux + '/' + relative.as_posix()

    def linux_list(value):
        return ','.join(linux_path(part) for part in value.split(','))

    binary, loader, library = (item.resolve(strict=True) for item in
                                (args.linux_binary, args.linux_loader, args.linux_lib_dir))
    command_prefix = ['wsl', '-d', args.wsl_distribution, '--cd', root_linux,
                      '--', 'env', '-u', 'STKCRAFT_WGC_GLIBC_ROT3']
    probe=subprocess.run(command_prefix+[linux_path(loader),'--library-path',linux_path(library),
                                         linux_path(binary),'--rotation-profile'],
                         cwd=ROOT,text=True,encoding='utf8',errors='replace',capture_output=True)
    if probe.returncode or probe.stdout.strip()!='native':
        raise RuntimeError(f'Linux backend profile mismatch: {probe.stdout.strip()!r}; {probe.stderr.strip()}')
    backend_modes = {}

    def run_linux(mode, lines, spk, parameters):
        command = command_prefix + ['STKCRAFT_SPK=' + linux_list(spk), linux_path(loader),
                                    '--library-path', linux_path(library), linux_path(binary), mode]
        command.extend(parameters)
        process = subprocess.run(command, cwd=ROOT, input='\n'.join(lines)+'\n', text=True,
                                 encoding='utf8', errors='replace', capture_output=True)
        if process.returncode:
            raise RuntimeError(f'WSL geometry exited {process.returncode}:\n{process.stderr}\n{process.stdout}')
        backend_modes[mode] = backend_modes.get(mode, 0) + 1
        return process.stdout

    def crate_queries(mode, lines, spk, parameter):
        parameters = []
        if mode in ('frame','size','elements','subpoint','longitude','illum','intercept',
                    'tangent','pointing','pointing_full','state_rep','separation'):
            parameters.append(linux_list(parameter))
        if mode in ('tangent','subpoint','illum','intercept','pointing_full','size'):
            parameters.append(linux_list(comparison.CLOCK_FK))
        return [None if row.startswith('ERROR') else row.split(',')
                for row in run_linux(mode, lines, spk, parameters).splitlines() if row.strip()]

    def structured_rows(raw):
        output = []
        for line in raw.splitlines():
            if not line.strip():
                continue
            if line.startswith('ERROR'):
                output.append(None)
                continue
            fields = line.split(',')
            if fields[0] == 'COLS':
                output.append({'ids':fields[1:],'rows':[]})
            elif fields[0] == 'ROW' and output and output[-1] is not None:
                output[-1]['rows'].append(fields[1:])
            else:
                raise RuntimeError(f'invalid structured geometry output: {line}')
        return output

    def crate_frame_rep(lines, fk, pck):
        return structured_rows(run_linux('frame_rep', lines, comparison.ACTIVE_SPK,
                                         [linux_list(fk),linux_list(pck)]))

    def crate_location_queries(lines, spk, pck):
        return structured_rows(run_linux('location',lines,spk,[linux_list(pck)]))

    comparison.crate_queries = crate_queries
    comparison.crate_frame_rep = crate_frame_rep
    comparison.crate_location_queries = crate_location_queries

    class NoUntrackedBackend:
        @staticmethod
        def run(*_args, **_kwargs):
            raise RuntimeError('untracked subprocess in Linux comparison')

    comparison.subprocess = NoUntrackedBackend
    sys.argv = ['replay_wgc_calculators.py', *replay_args]
    suite = {'calculators':replay.main,'earth-frames':earth_frames.main,'mars':mars.main}[args.suite]
    status = suite()
    report_path = args.report.resolve(strict=True)
    report = json.loads(report_path.read_text(encoding='utf8'))
    report['linux_profile'] = {'binary_sha256': digest(binary), 'loader_sha256': digest(loader),
                               'libm_sha256': digest(library / 'libm.so.6'),
                               'libc_sha256': digest(library / 'libc.so.6'),
                               'verified_profile': 'native',
                               'backend_modes': backend_modes,
                               'wsl_distribution': args.wsl_distribution}
    report_path.write_text(json.dumps(report, indent=2)+'\n', encoding='utf8')
    return status


if __name__ == '__main__':
    raise SystemExit(main())
