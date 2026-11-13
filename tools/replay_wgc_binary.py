"""Replay retained WGC calculators with an explicitly supplied local binary."""
import argparse
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

import replay_wgc_calculators as calculators
import replay_wgc_earth_frames as earth_frames
import replay_wgc_mars as mars
import wgc2_geometry_compare as comparison

ROOT=Path(__file__).resolve().parents[1]


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream,'sha256').hexdigest()


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',type=Path,required=True)
    parser.add_argument('--suite',choices=['calculators','earth-frames','mars'],default='calculators')
    parser.add_argument('--glibc-rotation',action='store_true',help='activate the explicit glibc R3 profile')
    parser.add_argument('--report',type=Path,required=True)
    args,replay_args=parser.parse_known_args()
    replay_args += ['--report',str(args.report)]
    binary=args.binary.resolve(strict=True)
    modes={}

    environment=os.environ.copy()
    for key in ('STKCRAFT_SPK','STKCRAFT_CK','STKCRAFT_SCLK','STKCRAFT_LSK','STKCRAFT_WGC_GLIBC_ROT3'):
        environment.pop(key,None)
    if args.glibc_rotation:
        environment['STKCRAFT_WGC_GLIBC_ROT3']='1'
    probe=subprocess.run([str(binary),'--rotation-profile'],cwd=ROOT,text=True,encoding='utf8',
                         errors='replace',capture_output=True,env=environment)
    expected='glibc-fma-r3' if args.glibc_rotation else 'native'
    if probe.returncode or probe.stdout.strip()!=expected:
        raise RuntimeError(f'backend profile mismatch: expected {expected}, got {probe.stdout.strip()!r}; {probe.stderr.strip()}')

    def run(mode,lines,spk,parameters):
        child_environment={**environment,'STKCRAFT_SPK':spk}
        process=subprocess.run([str(binary),mode,*parameters],cwd=ROOT,input='\n'.join(lines)+'\n',
                               text=True,encoding='utf8',errors='replace',capture_output=True,
                               env=child_environment)
        if process.returncode:
            raise RuntimeError(f'{mode}: backend failed: {process.stderr}\n{process.stdout}')
        modes[mode]=modes.get(mode,0)+1
        return process.stdout

    def queries(mode,lines,spk,parameter):
        params=[]
        if mode in ('frame','size','elements','subpoint','longitude','illum','intercept','tangent',
                    'pointing','pointing_full','state_rep','separation'):
            params.append(parameter)
        if mode in ('tangent','subpoint','illum','intercept','pointing_full','size'):
            params.append(comparison.CLOCK_FK)
        return [None if row.startswith('ERROR') else row.split(',')
                for row in run(mode,lines,spk,params).splitlines() if row.strip()]

    def structured(raw):
        result=[]
        for line in raw.splitlines():
            if not line.strip():
                continue
            if line.startswith('ERROR'):
                result.append(None)
                continue
            fields=line.split(',')
            if fields[0]=='COLS':
                result.append({'ids':fields[1:],'rows':[]})
            elif fields[0]=='ROW' and result and result[-1] is not None:
                result[-1]['rows'].append(fields[1:])
            else:
                raise RuntimeError(f'invalid structured backend output: {line}')
        return result

    comparison.crate_queries=queries
    comparison.crate_frame_rep=lambda lines,fk,pck:structured(run('frame_rep',lines,comparison.ACTIVE_SPK,[fk,pck]))
    comparison.crate_location_queries=lambda lines,spk,pck:structured(run('location',lines,spk,[pck]))

    class NoUntrackedBackend:
        @staticmethod
        def run(*_args,**_kwargs):
            raise RuntimeError('untracked subprocess in binary comparison')

    comparison.subprocess=NoUntrackedBackend
    sys.argv=['replay',*replay_args]
    suite={'calculators':calculators.main,'earth-frames':earth_frames.main,'mars':mars.main}[args.suite]
    status=suite()
    report=args.report.resolve(strict=True)
    result=json.loads(report.read_text(encoding='utf8'))
    result['backend']={'binary_sha256':digest(binary),'modes':modes,'glibc_rotation':args.glibc_rotation,'verified_profile':expected}
    report.write_text(json.dumps(result,indent=2)+'\n',encoding='utf8')
    return status


if __name__=='__main__':
    raise SystemExit(main())
