"""Reproduce corrected illumination/sub-solar differences against live WGC."""
import json
import os
import subprocess
import time
from pathlib import Path

OUT = Path('docs/review-2026-11-11')
BASE = 'https://wgc2.jpl.nasa.gov:8443/webgeocalc/api'
records = []
def request(method, path, payload=None):
    command = ['curl.exe', '--silent', '--show-error', '--fail', '--max-time', '30',
               '--proxy', 'http://127.0.0.1:52345', '-X', method, '-H', 'Content-Type: application/json', BASE + path]
    if payload is not None:
        command += ['--data-binary', '@-']
    result = subprocess.run(command, input=None if payload is None else json.dumps(payload), text=True, capture_output=True, check=True)
    response = json.loads(result.stdout)
    records.append({'method': method, 'path': path, 'payload': payload, 'response': response})
    (OUT / 'targeted-live.json').write_text(json.dumps(records, indent=2))
    return response

def run(payload):
    created = request('POST', '/calculation/new', payload)
    assert created['status'] == 'OK', created
    cid = created['calculationId']
    for _ in range(30):
        if created['result']['phase'] == 'COMPLETE':
            break
        assert created['result']['phase'] != 'FAILED', created
        time.sleep(0.5)
        created = request('GET', f'/calculation/{cid}')
    response = request('GET', f'/calculation/{cid}/results')
    assert response['status'] == 'OK', response
    return response

common = {'kernels': [{'type': 'KERNEL_SET', 'id': 1}], 'timeSystem': 'TDB',
          'timeFormat': 'SECONDS_PAST_J2000', 'times': ['500000000'],
          'outputTimeSystem': 'TDB', 'outputTimeFormat': 'SECONDS_PAST_J2000',
          'target': 'EARTH', 'observer': 'MOON', 'targetFrame': 'IAU_EARTH',
          'aberrationCorrection': 'CN+S'}
for mode in ['illum', 'subpoint']:
    if mode == 'illum':
        payload = {**common, 'calculationType': 'ILLUMINATION_ANGLES', 'shape1': 'ELLIPSOID',
                   'coordinateRepresentation': 'PLANETODETIC', 'longitude': 10, 'latitude': 20, 'altitude': 0}
        line = '399,301,10,20,0,500000000,CN+S\n'
    else:
        payload = {**common, 'calculationType': 'SUB_SOLAR_POINT', 'subPointType': 'Near point: ellipsoid',
                   'stateRepresentation': 'RECTANGULAR'}
        line = 'solar,399,301,500000000,CN+S\n'
    result = run(payload)
    local = subprocess.run(['target/debug/wgc_geometry.exe', mode, 'target/kernels/pck00011.tpc'],
                           input=line, capture_output=True, text=True, check=True,
                           env={**os.environ, 'STKCRAFT_SPK': 'target/kernels/de442s.bsp'})
    records.append({'mode': mode, 'local_input': line, 'local_output': local.stdout,
                    'remote_columns': [c['outputID'] for c in result['columns']], 'remote_rows': result['rows']})
    (OUT / 'targeted-live.json').write_text(json.dumps(records, indent=2))
    print(json.dumps(records[-1]), flush=True)
