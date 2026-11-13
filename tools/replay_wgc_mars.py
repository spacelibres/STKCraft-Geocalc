"""Replay WGC limb-extents observations for Mars and its triaxial moons."""
import argparse
import hashlib
import json
from pathlib import Path

import wgc2_geometry_compare as m

ROOT = Path(__file__).resolve().parents[1]
TRACE = ROOT / 'docs/reference/wgc-calculators/angular-size-triaxial-mars.jsonl'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--spk', required=True, help='comma-separated DE442 and Mars satellite SPKs')
    parser.add_argument('--pck', required=True, help='comma-separated PCK paths')
    parser.add_argument('--report', type=Path, required=True)
    args = parser.parse_args()
    paths = [Path(p).resolve(strict=True) for p in (args.spk + ',' + args.pck).split(',')]
    reference = (ROOT / 'docs/reference').resolve()
    report = args.report.resolve()
    if report == reference or reference in report.parents:
        parser.error('write the report outside the reference directory')
    m.WGC_OUTPUT_EXACT = True
    m.STRICT = True
    m.STRICT_FAILURES = []
    wgc = m.Wgc(False, 120, replay=TRACE)
    times = [-100000000.0, 0.0, 100000000.0]
    count = 0
    for target, tid, observer, oid, frame in [('PHOBOS',401,'MARS',499,'IAU_PHOBOS'),
                                              ('DEIMOS',402,'MARS',499,'IAU_DEIMOS'),
                                              ('MARS',499,'EARTH',399,'IAU_MARS')]:
        for correction in ['NONE','LT','LT+S','CN+S','XLT+S']:
            payload = {'calculationType':'ANGULAR_SIZE', 'kernels':[{'type':'KERNEL_SET','id':1}],
                       'target':target, 'observer':observer, 'targetFrame':frame,
                       'aberrationCorrection':correction, 'computationMethod':'LIMB_EXTENTS',
                       **m.time_block(times)}
            reference = m.reference_result(wgc, payload)
            ids = [column['outputID'] for column in reference['columns']]
            queries = [f'{oid},{tid},{et!r},{correction},LIMB_EXTENTS' for et in times]
            actual = m.crate_queries('size', queries, args.spk, args.pck)
            for query, row, local in m.verified_pairs(queries, reference['rows'], actual, 'size'):
                m.compare_mapped_row(ids, row, local, {'MIN_ANGULAR_SIZE':5,'MAX_ANGULAR_SIZE':6}, query)
                count += 1
    if count != 45:
        raise RuntimeError(f'expected 45 rows; got {count}')
    result = {'scope':'Exact WGC displayed output; not raw WGC binary64 equality',
              'server_kernel_bytes_verified':False, 'rows':count, 'errors':m.STRICT_FAILURES,
              'trace_sha256':hashlib.sha256(TRACE.read_bytes()).hexdigest(),
              'local_kernels':[{'path':str(p),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in paths]}
    report.parent.mkdir(parents=True, exist_ok=True)
    report.write_text(json.dumps(result,indent=2)+'\n',encoding='utf8')
    print(f'{count} rows; {len(m.STRICT_FAILURES)} differences')
    return int(bool(m.STRICT_FAILURES))


if __name__ == '__main__':
    raise SystemExit(main())
