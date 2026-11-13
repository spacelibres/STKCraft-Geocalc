"""Replay WGC angular-size values in IAU_EARTH and ITRF93 frames."""
import argparse
import hashlib
import json
from pathlib import Path
import wgc2_geometry_compare as m

ROOT = Path(__file__).resolve().parents[1]
TRACE = ROOT / 'docs/reference/wgc-calculators/angular-size-earth-frames.jsonl'


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--spk', required=True)
    parser.add_argument('--pck', required=True, help='text PCK and binary Earth PCK, comma-separated')
    parser.add_argument('--report', type=Path, required=True)
    args = parser.parse_args()
    paths = [Path(p).resolve(strict=True) for p in (args.spk+','+args.pck).split(',')]
    output = args.report.resolve()
    reference = (ROOT/'docs/reference').resolve()
    if output == reference or reference in output.parents:
        parser.error('write the report outside the reference directory')
    m.WGC_OUTPUT_EXACT = True
    m.STRICT = True
    m.STRICT_FAILURES = []
    wgc = m.Wgc(False,120,replay=TRACE)
    times = [0.0,100000000.0,500000000.0]
    count = 0
    for frame in ['IAU_EARTH','ITRF93']:
        for correction in ['NONE','LT+S','CN+S']:
            payload = {'calculationType':'ANGULAR_SIZE','kernels':[{'type':'KERNEL_SET','id':1},{'type':'KERNEL_SET','id':4}],
                       'target':'EARTH','observer':'MOON','targetFrame':frame,
                       'aberrationCorrection':correction,'computationMethod':'LIMB_EXTENTS',**m.time_block(times)}
            result = m.reference_result(wgc,payload)
            ids = [column['outputID'] for column in result['columns']]
            queries = [f'301,399,{et!r},{correction},LIMB_EXTENTS,{frame}' for et in times]
            actual = m.crate_queries('size',queries,args.spk,args.pck)
            for query,row,local in m.verified_pairs(queries,result['rows'],actual,'size'):
                m.compare_mapped_row(ids,row,local,{'MIN_ANGULAR_SIZE':5,'MAX_ANGULAR_SIZE':6},query)
                count += 1
    if count != 18:
        raise RuntimeError(f'expected 18 rows; got {count}')
    report={'scope':'Exact WGC displayed output; not raw WGC binary64 equality',
            'server_kernel_bytes_verified':False,'rows':count,'errors':m.STRICT_FAILURES,
            'trace_sha256':digest(TRACE),'local_kernels':[{'name':p.name,'sha256':digest(p)} for p in paths]}
    output.parent.mkdir(parents=True,exist_ok=True)
    output.write_text(json.dumps(report,indent=2)+'\n',encoding='utf8')
    print(f'{count} rows; {len(m.STRICT_FAILURES)} differences')
    return int(bool(m.STRICT_FAILURES))


if __name__ == '__main__':
    raise SystemExit(main())
