"""Run the existing final two comparison groups with curl transport.

Enforces exact output cardinality before the existing comparison loops.
The existing comparator's geometry and tolerance logic are unchanged.
"""
import importlib.util
import json
import subprocess
from pathlib import Path

spec = importlib.util.spec_from_file_location('comparison', Path(__file__).with_name('wgc2_geometry_compare.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
out = Path('docs/review-2026-11-11')
records = []
def request(self, method, path, payload=None):
    command = ['curl.exe', '--silent', '--show-error', '--fail', '--max-time', '30',
               '--proxy', 'http://127.0.0.1:52345', '-X', method,
               '-H', 'Content-Type: application/json', module.API_BASE + path]
    if payload is not None:
        command += ['--data-binary', '@-']
    process = subprocess.run(command, input=None if payload is None else json.dumps(payload),
                             text=True, capture_output=True, check=True)
    result = json.loads(process.stdout)
    records.append(dict(method=method, path=path, request=payload, response=result))
    (out / 'frames-locations-live.json').write_text(json.dumps(records, indent=2))
    assert result.get('status') == 'OK', result
    return result

module.Wgc._request = request
for name in ['crate_frame_rep', 'crate_location_queries']:
    original = getattr(module, name)
    def checked(lines, *args, original=original):
        result = original(lines, *args)
        assert len(result) == len(lines), (len(lines), len(result))
        return result
    setattr(module, name, checked)

summary = {}
wgc = module.Wgc(False, 120)
pck = 'target/kernels/pck00011.tpc,target/kernels/mars_iau2000_v1.tpc,target/kernels/gm_de431.tpc'
for name, function, args, expected_count in [
    ('frames', module.run_frames, ('-', pck), 87),
    ('locations', module.run_locations, ('target/kernels/de442s.bsp', pck), 29),
]:
    print('Starting', name, flush=True)
    result = function(wgc, *args)
    worst, count, failures = result
    summary[name] = dict(worst=worst, count=count, failures=failures,
                         pass_by_existing_tolerance=(worst <= module.TOLERANCE_FRAME_REP and not failures and count == expected_count))
    (out / 'frames-locations-summary.json').write_text(json.dumps(summary, indent=2))
    print(name, json.dumps(summary[name]), flush=True)
assert all(item['pass_by_existing_tolerance'] for item in summary.values()), summary
