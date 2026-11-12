"""Verify independent native evidence and checked fixtures stay identical."""
from pathlib import Path
import hashlib
import json
import unittest

ROOT = Path(__file__).resolve().parents[2]


class NativeSurfaceModelEvidence(unittest.TestCase):
    def check_capture(self, directory, fixture, source, snapshot_source, snapshot_generator, count):
        path = ROOT / 'docs/review-2026-11-12' / directory
        record = json.loads((path / 'capture.json').read_text(encoding='utf-8'))
        digest = lambda value: hashlib.sha256(value).hexdigest()
        for stream in ['stdout', 'stderr']:
            self.assertEqual(digest((path / f'native-{stream}.bin').read_bytes()), record[f'{stream}_sha256'])
        raw = (path / 'native-stdout.bin').read_bytes()
        self.assertEqual(len(raw.splitlines()), count)
        self.assertEqual(raw, (ROOT / 'tests/golden' / fixture).read_bytes())
        self.assertEqual(digest((path / snapshot_source).read_bytes()), record['source_sha256'])
        self.assertEqual((path / snapshot_source).read_bytes(), (ROOT / 'tools/reference' / source).read_bytes())
        # The snapshot records the exact generator at capture time. Later
        # validation changes must not rewrite that historical evidence.
        self.assertEqual(digest((path / snapshot_generator).read_bytes()), record['wrapper_sha256'])
        manifest = json.loads((ROOT / 'docs/reference/CSPICE_N0067.json').read_text(encoding='utf-8'))
        self.assertTrue(record['source_sha256_additional'])
        for name, expected in record['source_sha256_additional'].items():
            self.assertEqual(expected, manifest['reference_source_sha256'][name])
        if 'generated_spk_sha256' in record:
            self.assertEqual(digest((ROOT / 'tests/golden/dsk/occultation-points.bsp').read_bytes()), record['generated_spk_sha256'])

    def test_surface_coordinate_capture_integrity(self):
        self.check_capture('surface-coordinates', 'surface_coordinates.txt',
                           'surface_coordinates_oracle.c', 'oracle.c.txt', 'generator.py.txt', 2352)

    def test_surface_intercept_capture_integrity(self):
        self.check_capture('surface-intercepts', 'surface_intercepts.txt',
                           'surface_intercepts_oracle.c', 'oracle.c.txt', 'generator.py.txt', 1296)

    def test_dsk_occultation_capture_integrity(self):
        self.check_capture('dsk-occultation', 'dsk_occultation.txt',
                           'dsk_occultation_oracle.c', 'dsk_occultation_oracle.c', 'gen_dsk_occultation.py', 864)
