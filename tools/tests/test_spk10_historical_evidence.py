"""Preserve original historical SPK10 oracle output and generated kernel bytes."""
from pathlib import Path
import hashlib,json,unittest
ROOT=Path(__file__).resolve().parents[2]
def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
class HistoricalSpk10Evidence(unittest.TestCase):
    def test_native_capture_and_original_fixture_bytes(self):
        path=ROOT/'docs/review-2026-11-12/spk10-historical'
        record=json.loads((path/'capture.json').read_text(encoding='utf-8'))
        for stream in ['stdout','stderr']:
            self.assertEqual(digest(path/f'native-{stream}.bin'),record[f'{stream}_sha256'])
        self.assertEqual((path/'native-stdout.bin').read_bytes(),(ROOT/'tests/golden/spk10_historical.txt').read_bytes())
        self.assertEqual(digest(ROOT/'tests/golden/spk10_historical.bsp'),record['generated_spk_sha256'])
        self.assertEqual((record['segments'],record['queries'],record['numeric'],record['errors']),(52,33696,33696,0))
        self.assertEqual(digest(path/'spk10_historical_oracle.c'),record['source_sha256'])
        self.assertEqual((path/'spk10_historical_oracle.c').read_bytes(),(ROOT/'tools/reference/spk10_historical_oracle.c').read_bytes())
        self.assertEqual(digest(path/'gen_spk10_historical.py'),record['generator_sha256'])
        manifest=json.loads((ROOT/'docs/reference/CSPICE_N0067.json').read_text(encoding='utf-8'))
        for name,expected in record['source_sha256_additional'].items():
            self.assertEqual(manifest['reference_source_sha256'][name],expected)
