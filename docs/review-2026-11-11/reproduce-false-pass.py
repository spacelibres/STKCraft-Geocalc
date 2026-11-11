import importlib.util
import json
from pathlib import Path

spec = importlib.util.spec_from_file_location('compare', 'tools/wgc2_geometry_compare.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

class FakeWgc:
    def run(self, payload):
        return {'rows': [[0.0] * 11 for _ in module.EPOCHS]}

module.crate_queries = lambda *args: []
result = module.run_state(FakeWgc(), '-',)
print('run_state with 135 NASA rows and ZERO local rows:', result)
print('Existing PASS predicate:', result[0] <= module.TOLERANCE_POSITION_KM and result[1] <= module.TOLERANCE_VELOCITY_KMS and result[2] <= module.TOLERANCE_LIGHT_TIME_S)
Path('docs/review-2026-11-11/false-pass-repro.json').write_text(json.dumps({'scenario': '135 remote rows; zero local output rows', 'run_state_result': result, 'existing_pass_predicate': result[0] <= module.TOLERANCE_POSITION_KM and result[1] <= module.TOLERANCE_VELOCITY_KMS and result[2] <= module.TOLERANCE_LIGHT_TIME_S}, indent=2))
