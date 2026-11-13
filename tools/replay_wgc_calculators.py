"""Replay retained NASA calculator observations against local geometry.

This checks the observed WGC decimal display, not native binary64 results.
The server's kernel-set bytes are not available for independent hashing.
"""
import argparse
import hashlib
import json
from pathlib import Path

import wgc2_geometry_compare as comparison

ROOT = Path(__file__).resolve().parents[1]
REFERENCE = ROOT / 'docs/reference/wgc-calculators'


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def replay_size_grid(wgc, spk, pck):
    times = [float(index * 86400 * 5) for index in range(-30, 31)]
    payload = {'calculationType':'ANGULAR_SIZE', 'kernels':[{'type':'KERNEL_SET','id':1}],
               'target':'EARTH', 'observer':'SUN', 'targetFrame':'IAU_EARTH',
               'aberrationCorrection':'NONE', 'computationMethod':'LIMB_EXTENTS',
               **comparison.time_block(times)}
    reference = comparison.reference_result(wgc, payload)
    ids = [column['outputID'] for column in reference['columns']]
    queries = [f'10,399,{et!r},NONE,LIMB_EXTENTS' for et in times]
    actual = comparison.crate_queries('size', queries, spk, pck)
    checked = 0
    for query, row, local in comparison.verified_pairs(queries, reference['rows'], actual, 'size'):
        comparison.compare_mapped_row(ids, row, local,
                                      {'MIN_ANGULAR_SIZE':5,'MAX_ANGULAR_SIZE':6}, query)
        checked += 1
    return checked


def replay_size_matrix(wgc, spk, pck):
    times = [-500000000.0, 0.0, 500000000.0]
    checked = 0
    for target, tid, observer, oid, frame in [('EARTH',399,'MOON',301,'IAU_EARTH'),
                                              ('MOON',301,'EARTH',399,'IAU_MOON'),
                                              ('VENUS',299,'EARTH',399,'IAU_VENUS')]:
        for corr in ['NONE','LT','LT+S','CN+S','XLT+S']:
            payload = {'calculationType':'ANGULAR_SIZE', 'kernels':[{'type':'KERNEL_SET','id':1}],
                       'target':target, 'observer':observer, 'targetFrame':frame,
                       'aberrationCorrection':corr, 'computationMethod':'LIMB_EXTENTS',
                       **comparison.time_block(times)}
            reference = comparison.reference_result(wgc, payload)
            ids = [column['outputID'] for column in reference['columns']]
            queries = [f'{oid},{tid},{et!r},{corr},LIMB_EXTENTS' for et in times]
            actual = comparison.crate_queries('size', queries, spk, pck)
            for query, row, local in comparison.verified_pairs(queries, reference['rows'], actual, 'size'):
                comparison.compare_mapped_row(ids, row, local,
                                              {'MIN_ANGULAR_SIZE':5,'MAX_ANGULAR_SIZE':6}, query)
                checked += 1
    return checked


def replay_subobserver_lst_grid(wgc, spk, pck):
    times = [float(index * 60) for index in range(60)]
    payload = {'calculationType':'SUB_OBSERVER_POINT', 'kernels':[{'type':'KERNEL_SET','id':1}],
               'target':'MOON', 'observer':'EARTH', 'targetFrame':'IAU_MOON',
               'aberrationCorrection':'CN+S', 'subPointType':'Near point: ellipsoid',
               'stateRepresentation':'RECTANGULAR', **comparison.time_block(times)}
    reference = comparison.reference_result(wgc, payload)
    index = next(i for i, column in enumerate(reference['columns']) if column['outputID'] == 'LTST')
    queries = [f'observer,301,399,{et!r},CN+S,NEAR_POINT_ELLIPSOID,IAU_MOON' for et in times]
    actual = comparison.crate_queries('subpoint', queries, spk, pck)
    checked = 0
    for query, row, local in comparison.verified_pairs(queries, reference['rows'], actual, 'subpoint'):
        if row[index] != local[18]:
            comparison.STRICT_FAILURES.append(f'{query}: LTST WGC={row[index]}, local={local[18]}')
        checked += 1
    return checked


def replay_subobserver_lst_matrix(wgc, spk, pck):
    times = [0.0, 300.0, 500000000.0]
    checked = 0
    for target, tid, observer, oid, frame in [('MOON',301,'EARTH',399,'IAU_MOON'),
                                              ('MOON',301,'EARTH',399,'MOON_ME'),
                                              ('EARTH',399,'MOON',301,'IAU_EARTH')]:
        for method, cli in [('Near point: ellipsoid','NEAR_POINT_ELLIPSOID'),
                            ('Intercept: ellipsoid','INTERCEPT_ELLIPSOID')]:
            for corr in ['NONE','LT','LT+S','CN+S','XLT+S']:
                payload = {'calculationType':'SUB_OBSERVER_POINT', 'kernels':[{'type':'KERNEL_SET','id':1}],
                           'target':target, 'observer':observer, 'targetFrame':frame,
                           'aberrationCorrection':corr, 'subPointType':method,
                           'stateRepresentation':'RECTANGULAR', **comparison.time_block(times)}
                reference = comparison.reference_result(wgc, payload)
                index = next(i for i, column in enumerate(reference['columns']) if column['outputID'] == 'LTST')
                queries = [f'observer,{tid},{oid},{et!r},{corr},{cli},{frame}' for et in times]
                actual = comparison.crate_queries('subpoint', queries, spk, pck)
                for query, row, local in comparison.verified_pairs(queries, reference['rows'], actual, 'subpoint'):
                    if row[index] != local[18]:
                        comparison.STRICT_FAILURES.append(f'{query}: LTST WGC={row[index]}, local={local[18]}')
                    checked += 1
    return checked


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--spk', type=Path, required=True)
    parser.add_argument('--pck', type=Path, required=True)
    parser.add_argument('--frame-pck', required=True,
                        help='comma-separated PCK paths in load order')
    parser.add_argument('--report', type=Path, required=True)
    args = parser.parse_args()
    spk, pck = args.spk.resolve(strict=True), args.pck.resolve(strict=True)
    frame_pcks = [Path(path).resolve(strict=True) for path in args.frame_pck.split(',')]
    reference = (ROOT / 'docs/reference').resolve()
    output = args.report.resolve()
    if output == reference or reference in output.parents:
        parser.error('write the report outside the committed reference directory')
    comparison.WGC_OUTPUT_EXACT = True
    comparison.STRICT = True
    comparison.ACTIVE_SPK = str(spk)
    cases = [
        ('STATE_VECTOR object rows', comparison.run_state, (str(spk),), 'state-object.jsonl', 135),
        ('STATE_VECTOR representation cells', comparison.run_state_reps, (str(spk), str(pck)), 'state-representations.jsonl', 2184),
        ('STATE_VECTOR point rows', comparison.run_locations, (str(spk), str(pck)), 'state-locations.jsonl', 29),
        ('ANGULAR_SEPARATION rows', comparison.run_separation, (str(spk), str(pck)), 'separation-objects.jsonl', 42),
        ('ANGULAR_SIZE rows', comparison.run_size, (str(spk), str(pck)), 'angular-size.jsonl', 54),
        ('ANGULAR_SIZE Earth grid rows', replay_size_grid, (str(spk), str(pck)), 'angular-size-earth-grid.jsonl', 61),
        ('ANGULAR_SIZE limb matrix rows', replay_size_matrix, (str(spk), ','.join(map(str, frame_pcks))), 'angular-size-limb-matrix.jsonl', 45),
        ('FRAME_TRANSFORMATION rows', comparison.run_frames, ('-', ','.join(map(str, frame_pcks))), 'frame-transformation.jsonl', 87),
        ('ILLUMINATION_ANGLES rows', comparison.run_illumination, (str(spk), ','.join(map(str, frame_pcks))), 'illumination.jsonl', 9),
        ('PHASE_ANGLE rows', comparison.run_phase, (str(spk),), 'phase-angle.jsonl', 18),
        ('POINTING_DIRECTION rows', comparison.run_pointing, (str(spk), ','.join(map(str, frame_pcks))), 'pointing.jsonl', 7),
        ('SUB_SOLAR_POINT rows', comparison.run_sub_solar, (str(spk), ','.join(map(str, frame_pcks))), 'sub-solar.jsonl', 12),
        ('SUB_OBSERVER_POINT rows', comparison.run_sub_observer, (str(spk), ','.join(map(str, frame_pcks))), 'sub-observer.jsonl', 12),
        ('SUB_OBSERVER_POINT local time grid', replay_subobserver_lst_grid, (str(spk), ','.join(map(str, frame_pcks))), 'sub-observer-lst.jsonl', 60),
        ('SUB_OBSERVER_POINT local time matrix', replay_subobserver_lst_matrix, (str(spk), ','.join(map(str, frame_pcks))), 'sub-observer-lst-matrix.jsonl', 90),
        ('SURFACE_INTERCEPT_POINT rows', comparison.run_surface_intercept, (str(spk), ','.join(map(str, frame_pcks))), 'surface-intercept.jsonl', 6),
        ('TANGENT_POINT rows', comparison.run_tangent, (str(spk), ','.join(map(str, frame_pcks))), 'tangent.jsonl', 9),
        ('OSCULATING_ELEMENTS rows', comparison.run_elements, (str(spk), ','.join(map(str, frame_pcks))), 'orbital-elements.jsonl', 6),
        ('PLANETOCENTRIC_SOLAR_LONGITUDE rows', comparison.run_solar_longitude, (str(spk), ','.join(map(str, frame_pcks))), 'solar-longitude.jsonl', 6),
    ]
    records = []
    for label, function, arguments, name, expected in cases:
        comparison.STRICT_FAILURES = []
        trace = REFERENCE / name
        wgc = comparison.Wgc(False, 120, replay=trace)
        result = function(wgc, *arguments)
        count = result if function in (replay_size_grid, replay_size_matrix, replay_subobserver_lst_grid, replay_subobserver_lst_matrix) else result[1] if function in (comparison.run_locations, comparison.run_separation, comparison.run_size, comparison.run_frames, comparison.run_illumination, comparison.run_phase, comparison.run_pointing, comparison.run_sub_solar, comparison.run_sub_observer, comparison.run_surface_intercept, comparison.run_tangent, comparison.run_elements, comparison.run_solar_longitude) else result[-1]
        errors = list(comparison.STRICT_FAILURES)
        if function in (comparison.run_locations, comparison.run_frames):
            errors.extend(result[2])
        if count != expected:
            errors.append(f'Expected {expected} comparisons, got {count}')
        records.append({'case': label, 'count': count, 'trace_sha256': sha256(trace), 'errors': errors})
        print(f'{label}: {count} comparisons; {len(errors)} errors', flush=True)
    report = {'scope': 'Exact WGC formatted output; not native binary64 equivalence',
              'kernel_set': 1, 'server_kernel_bytes_verified': False,
              'local_spk_sha256': sha256(spk), 'local_pck_sha256': sha256(pck),
              'frame_pcks': [{'path': str(path), 'sha256': sha256(path)} for path in frame_pcks],
              'frame_kernels': [{'path': str((ROOT / path).resolve(strict=True)),
                                 'sha256': sha256((ROOT / path).resolve(strict=True))}
                                for path in comparison.CLOCK_FK.split(',') if path and path != '-'],
              'cases': records}
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2) + '\n', encoding='utf8')
    return int(any(row['errors'] for row in records))


if __name__ == '__main__':
    raise SystemExit(main())
