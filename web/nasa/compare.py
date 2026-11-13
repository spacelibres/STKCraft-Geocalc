"""Live NASA WGC comparisons. Never infer parity from unavailable or unmapped output."""
import json
import os
import sys
import urllib.error
from decimal import Decimal
from pathlib import Path
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
import wgc2_geometry_compare as wgc

DEFAULT = [{'name': 'NASA NAIF WGC2', 'url': 'https://wgc2.jpl.nasa.gov:8443/webgeocalc/api', 'kernel_sets': [1]}]

def validate_url(url):
    parsed = urlsplit(url)
    host = (parsed.hostname or '').lower()
    if parsed.scheme != 'https' or not (host == 'nasa.gov' or host.endswith('.nasa.gov')) or parsed.username or parsed.password or parsed.query or parsed.fragment or not parsed.path.endswith('/api'):
        raise ValueError('NASA endpoints must be HTTPS nasa.gov API URLs without credentials, query, or fragment.')
    return url.rstrip('/')

def endpoints():
    entries = json.loads(os.environ.get('STKCRAFT_WEB_WGC_ENDPOINTS', json.dumps(DEFAULT)))
    if not isinstance(entries, list) or not 1 <= len(entries) <= 3: raise ValueError('Configure one to three NASA endpoints.')
    for entry in entries:
        validate_url(entry['url'])
        if not isinstance(entry.get('name'), str) or not entry['name'].strip(): raise ValueError('NASA endpoint requires a name.')
        if not isinstance(entry.get('kernel_sets'), list) or not entry['kernel_sets'] or any(type(x) is not int or x < 0 for x in entry['kernel_sets']): raise ValueError('NASA kernel_sets must be a nonempty list of IDs.')
    return entries

def payload_for(operation, p, kernel_sets):
    common = {'kernels': [{'type': 'KERNEL_SET', 'id': i} for i in kernel_sets]}
    corr = p.get('correction', 'NONE')
    if operation in ('utc_to_et', 'wgc_gps_seconds_to_et'):
        if operation == 'utc_to_et':
            if int(p['year']) < 1583 or str(p.get('calendar', '')).upper() not in ('', 'GREGORIAN', 'MIXED'):
                raise NotImplementedError('Historical or non-Gregorian calendar mapping is not verified for NASA.')
            second = str(p.get('second', 0)).zfill(2)
            times = [f"{int(p['year']):04d}-{int(p['month']):02d}-{int(p['day']):02d}T{int(p.get('hour', 0)):02d}:{int(p.get('minute', 0)):02d}:{second}"]
            system, format_name = 'UTC', 'CALENDAR'
        else:
            times = [str(p['gps_seconds'])]
            system, format_name = 'GPS', 'SECONDS_PAST_GPS0'
        return {**common, 'calculationType':'TIME_CONVERSION', 'timeSystem':system, 'timeFormat':format_name, 'times':times, 'timeStep':1, 'timeStepUnits':'SECONDS', 'outputTimeSystem':'TDB', 'outputTimeFormat':'SECONDS_PAST_J2000'}
    if operation in ('state_vector', 'angular_separation', 'phase_angle', 'angular_size', 'osculating_elements'):
        result = {**common, **wgc.time_block([p['epoch_et']]), 'calculationType': operation.upper(), 'aberrationCorrection': corr}
        if operation != 'osculating_elements': result['observer'] = str(p['observer'])
        if operation == 'state_vector': result.update(target=str(p['target']), referenceFrame='J2000', stateRepresentation='RECTANGULAR')
        elif operation == 'angular_size':
            result.update(target=str(p['target']), computationMethod=p.get('method','MAX_RADIUS'))
            if result['computationMethod'] == 'LIMB_EXTENTS': result['targetFrame'] = p.get('target_frame', 'IAU_' + str(p['target']))
        elif operation == 'osculating_elements':
            if p.get('gm_source', 'CENTER') != 'CENTER': raise NotImplementedError('Non-default GM source is not mapped to NASA yet.')
            result.update(orbitingBody=str(p['target']), centerBody=str(p['center']), referenceFrame=p.get('reference_frame','J2000'))
        elif operation == 'phase_angle': result.update(target=str(p['target']), illuminator=str(p.get('illuminator', 10)))
        else: result.update(specType='TWO_TARGETS', target1=str(p['target1']), target2=str(p['target2']), shape1=p.get('shape1', 'POINT'), shape2=p.get('shape2', 'POINT'), frame1='J2000', frame2='J2000')
        return result
    if operation in ('gf_distance_search', 'gf_range_rate_search', 'gf_phase_angle_search', 'gf_coordinate_search', 'gf_angular_separation_search'):
        condition = {'relationalCondition': p.get('relation', '>'), 'referenceValue': p.get('reference', 0)}
        if condition['relationalCondition'] == 'RANGE': condition['upperLimit'] = p['upper']
        result = {**common, 'calculationType': operation.upper(), 'timeSystem': 'TDB', 'timeFormat': 'SECONDS_PAST_J2000', 'intervals': [{'startTime': str(v['start']), 'endTime': str(v['end'])} for v in p['confinement']], 'timeStep': p['step_seconds'], 'timeStepUnits': 'SECONDS', 'outputTimeSystem': 'TDB', 'outputTimeFormat': 'SECONDS_PAST_J2000', 'outputDurationUnits': 'SECONDS', 'shouldComplementWindow': False, 'intervalAdjustment': 'NO_ADJUSTMENT', 'intervalFiltering': 'NO_FILTERING', 'target': str(p['target']), 'observer': str(p['observer']), 'aberrationCorrection': corr, 'condition': condition}
        # Additional post-processing is not silently discarded.
        if any(p.get(k) not in (None, False, '', 0, 'NONE', 'NO_ADJUSTMENT', 'NO_FILTERING', 'SECONDS') for k in ('complement', 'adjustment', 'interval_adjustment', 'interval_amount', 'filter', 'filter_threshold', 'duration_units')): raise NotImplementedError('This finder post-processing configuration is not mapped to NASA yet.')
        if operation == 'gf_phase_angle_search': result['illuminator'] = str(p.get('illuminator', 10))
        if operation == 'gf_coordinate_search':
            result['referenceFrame'] = p.get('reference_frame', 'J2000')
            condition.update(coordinateSystem=p.get('coordinate_system', 'RECTANGULAR'), coordinate=p.get('coordinate', 'X'))
        if operation == 'gf_angular_separation_search':
            result['target1'] = result.pop('target')
            result['target2'] = str(p.get('second',10))
            result['shape1'], result['shape2'] = p.get('shape1','POINT'), p.get('shape2','POINT')
        return result
    raise NotImplementedError('A verified NASA request mapping is not yet available for this operation.')

MAPS = {'angular_size': {'DATE':'epoch_et','ANGULAR_SIZE':'angular_size_deg','MIN_ANGULAR_SIZE':'min_angular_size_deg','MAX_ANGULAR_SIZE':'max_angular_size_deg'}, 'osculating_elements': {'DATE':'epoch_et','PERIFOCAL_DISTANCE':'perifocal_distance_km','ECCENTRICITY':'eccentricity','INCLINATION':'inclination_deg','ASCENDING_NODE_LONGITUDE':'ascending_node_deg','ARGUMENT_OF_PERIAPSE':'argument_of_periapsis_deg','MEAN_ANOMALY_AT_EPOCH':'mean_anomaly_deg','ORBITING_BODY_RANGE':'range_km','ORBITING_BODY_SPEED':'speed_km_s','PERIOD':'period_s','CENTER_BODY_GM':'gm_km3_s2','TRUE_ANOMALY':'true_anomaly_deg','SEMIMAJOR_AXIS':'semi_major_axis_km'}, 'utc_to_et': {'DATE2':'et'}, 'wgc_gps_seconds_to_et': {'DATE2':'et'}, 'state_vector': {'DATE':'epoch_et','DISTANCE':'range_km','SPEED':'speed_km_s','X':'x_km','Y':'y_km','Z':'z_km','D_X_DT':'vx_km_s','D_Y_DT':'vy_km_s','D_Z_DT':'vz_km_s','TIME_AT_TARGET':'time_at_target_et','LIGHT_TIME':'light_time_s'}, 'angular_separation': {'DATE':'epoch_et','ANGULAR_SEPARATION':'separation_deg'}, 'phase_angle': {'DATE':'epoch_et','PHASE_ANGLE':'phase_deg'}}

def compare(operation, local, remote):
    columns, rows = remote.get('columns', []), remote.get('rows', [])
    own_columns, own_rows = local.get('columns', []), local.get('rows', [])
    cells, unmapped = [], []
    if len(rows) != len(own_rows): return {'status':'difference', 'message':f'Row count differs: local {len(own_rows)}, NASA {len(rows)}.', 'cells':[], 'unmapped':[]}
    for row_index, (own, reference) in enumerate(zip(own_rows, rows)):
        if len(reference) != len(columns): return {'status':'error','message':'NASA returned an invalid row shape.','cells':[], 'unmapped':[]}
        for i, col in enumerate(columns):
            output_id = col.get('outputID', '')
            local_name = (['start_et', 'end_et', 'duration'][i] if operation.startswith('gf_') and i < 3 else MAPS.get(operation, {}).get(output_id))
            if not local_name or local_name not in own_columns:
                if output_id not in unmapped: unmapped.append(output_id or f'column {i + 1}')
                continue
            local_value = own[own_columns.index(local_name)]
            try:
                local_formatted = wgc.wgc_output_value(local_value, output_id)
                nasa_value = wgc.finite_decimal(reference[i], 'NASA output')
                equal = local_formatted == nasa_value
                delta = str(local_formatted - nasa_value)
            except (ValueError, TypeError, RuntimeError):
                if output_id not in unmapped: unmapped.append(output_id or f'column {i + 1}')
                continue
            cells.append({'row': row_index + 1, 'field': col.get('name', output_id), 'local': str(local_value), 'formatted': str(local_formatted), 'nasa': str(reference[i]), 'difference': delta, 'equal': equal})
    status = 'difference' if any(not cell['equal'] for cell in cells) else 'partial' if unmapped or not cells else 'display_match'
    return {'status':status, 'cells':cells, 'unmapped':unmapped, 'message':'Equality compares NASA displayed values after verified WGC serialization; it does not establish binary equality or identical kernels.'}

def main(request):
    attempts = []
    for entry in endpoints():
        try:
            payload = payload_for(request['operation'], request['parameters'], entry['kernel_sets'])
        except NotImplementedError as error:
            return {'status':'unsupported','message':str(error),'attempts':attempts}
        wgc.API_BASE = validate_url(entry['url'])
        try:
            remote = wgc.Wgc(False, 25).run(payload)
            if remote.get('status') != 'OK': return {'status':'error','message':str(remote.get('message','NASA calculation failed')), 'endpoint':entry['name'], 'url':entry['url'], 'kernel_sets':entry['kernel_sets'], 'attempts':attempts}
            comparison = compare(request['operation'], request['local'], remote)
            return {**comparison, 'endpoint':entry['name'], 'url':entry['url'], 'kernel_sets':entry['kernel_sets'], 'calculation_id':remote.get('calculationId'), 'attempts':attempts}
        except urllib.error.HTTPError as error:
            if error.code < 500: return {'status':'error','message':f'NASA HTTP {error.code}', 'endpoint':entry['name'], 'url':entry['url'], 'kernel_sets':entry['kernel_sets'], 'attempts':attempts}
            attempts.append({'endpoint':entry['name'], 'error':f'HTTP {error.code}'})
        except (urllib.error.URLError, TimeoutError, OSError) as error:
            attempts.append({'endpoint':entry['name'], 'error':str(error)})
        except RuntimeError as error:
            return {'status':'error','message':str(error), 'endpoint':entry['name'], 'url':entry['url'], 'kernel_sets':entry['kernel_sets'], 'attempts':attempts}
    return {'status':'unavailable','message':'All configured NASA endpoints failed to respond.', 'attempts':attempts}

if __name__ == '__main__':
    try: print(json.dumps(main(json.load(sys.stdin)), default=str, allow_nan=False))
    except Exception as error: print(json.dumps({'status':'error','message':str(error)}))
