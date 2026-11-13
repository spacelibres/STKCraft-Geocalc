"""Validate hashes and key outcomes in the retained WGC captures."""
import hashlib
import json
import struct
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


class CalculatorEvidence(unittest.TestCase):
    def test_subpoint_cross_platform_native_evidence(self):
        directory=ROOT/'docs/reference/native-cross-platform'
        meta=json.loads((directory/'metadata.json').read_text(encoding='utf8'))
        for name, expected in meta['captures'].items():
            self.assertEqual(digest(directory/name),expected)
        windows=json.loads((directory/'sub-tangent-windows.json').read_text())
        linux=json.loads((directory/'sub-tangent-linux.json').read_text())
        self.assertEqual(len(windows),7)
        self.assertEqual(len(linux),7)
        for index in (0,1):
            self.assertNotEqual(windows[index]['emission_rad_bits'],linux[index]['emission_rad_bits'])
            self.assertEqual(windows[index]['et2lst'],linux[index]['et2lst'])
        for index in (2,3):
            self.assertNotEqual(windows[index]['alt_bits'],linux[index]['alt_bits'])
        self.assertEqual(windows[5]['frame_et'],linux[5]['frame_et'])
        self.assertEqual(sum(a!=b for a,b in zip(windows[5]['matrix_bits'],linux[5]['matrix_bits'])),3)
        windows_angles=[json.loads(line) for line in (directory/'orientation-windows.jsonl').read_text(encoding='utf8').splitlines()]
        linux_angles=[json.loads(line) for line in (directory/'orientation-linux.jsonl').read_text(encoding='utf8').splitlines()]
        self.assertEqual(windows_angles[1]['angles'], linux_angles[1]['angles'])
        self.assertNotEqual(windows_angles[1]['cos'][2], linux_angles[1]['cos'][2])
        intervention=[json.loads(line) for line in (directory/'sub-tangent-linux-windows-cos.jsonl').read_text(encoding='utf8').splitlines()]
        self.assertEqual(intervention, windows)
        self.assertEqual([i for i,(a,b) in enumerate(zip(intervention,linux)) if a!=b], [0,1,2,3,5])
        official = (directory/'naif-official-surface.txt').read_text(encoding='utf8').splitlines()
        actual = [line.rsplit(': ',1)[1] for line in official]
        expected = [windows[0]['emission_rad_bits'], windows[1]['emission_rad_bits'],
                    windows[2]['alt_bits'], windows[3]['alt_bits']]
        bits = [f"{struct.unpack('>Q', struct.pack('>d', float.fromhex(value)))[0]:016x}" for value in expected]
        self.assertEqual(actual, bits)
        official_linux=(directory/'naif-official-linux-surface.txt').read_text(encoding='utf8').splitlines()
        linux_actual=[line.rsplit(': ',1)[1] for line in official_linux]
        linux_expected=[linux[0]['emission_rad_bits'],linux[1]['emission_rad_bits'],linux[2]['alt_bits'],linux[3]['alt_bits']]
        linux_bits=[f"{struct.unpack('>Q', struct.pack('>d', float.fromhex(value)))[0]:016x}" for value in linux_expected]
        self.assertEqual(linux_actual, linux_bits)
        matrices=(directory/'naif-official-matrix.txt').read_text(encoding='utf8').splitlines()
        for line, capture in zip(matrices, windows[4:]):
            actual = line.split()[1].split(',')
            expected = [f"{struct.unpack('>Q', struct.pack('>d', float.fromhex(value)))[0]:016x}" for value in capture['matrix_bits']]
            self.assertEqual(actual, expected)
        self.assertEqual(len(matrices), 3)
        linux_matrices=(directory/'naif-official-linux-matrix.txt').read_text(encoding='utf8').splitlines()
        self.assertEqual(len(linux_matrices), 3)
        for line, capture in zip(linux_matrices, linux[4:]):
            actual = line.split()[1].split(',')
            expected = [f"{struct.unpack('>Q', struct.pack('>d', float.fromhex(value)))[0]:016x}" for value in capture['matrix_bits']]
            self.assertEqual(actual, expected)
        inputs=(directory/'cos-inputs.txt').read_text(encoding='utf8').splitlines()
        candidate=(directory/'cos-libm-results.txt').read_text(encoding='utf8').splitlines()
        glibc=(directory/'cos-glibc-results.txt').read_text(encoding='utf8').splitlines()
        self.assertEqual(len(inputs), 20001)
        self.assertEqual(len(candidate), len(inputs))
        self.assertEqual(len(glibc), len(inputs))
        self.assertEqual(sum(a != b for a,b in zip(candidate,glibc)), 632)
        self.assertEqual(inputs[-1], '3fe701f621f7403f')
        self.assertEqual(candidate[-1], glibc[-1])
        report=json.loads((directory/'wgc-linux-calculators.json').read_text(encoding='utf8'))
        self.assertEqual(len(report['cases']), 19)
        self.assertTrue(all(not case['errors'] for case in report['cases']))
        self.assertFalse(report['server_kernel_bytes_verified'])
        self.assertEqual(report['linux_profile']['verified_profile'], 'native')
        self.assertGreater(report['linux_profile']['backend_modes'].get('location',0),0)
        self.assertGreater(report['linux_profile']['backend_modes'].get('frame_rep',0),0)
        self.assertEqual(len(report['frame_kernels']),2)
        finders=json.loads((directory/'wgc-linux-finders.json').read_text(encoding='utf8'))
        self.assertEqual(len(finders['cases']), 12)
        self.assertTrue(all(not case['wgc_failures'] and not case['native_failures'] for case in finders['cases']))
        self.assertFalse(finders['server_kernel_bytes_verified'])
        self.assertEqual(finders['linux_profile']['verified_profile'], 'native')
        matrix = json.loads((directory/'wgc-linux-finder-matrix.json').read_text(encoding='utf8'))
        self.assertEqual(len(matrix['cases']), 72)
        self.assertEqual(matrix['linux_profile']['verified_profile'], 'native')
        self.assertTrue(all(not case['wgc_failures'] and not case['native_failures'] for case in matrix['cases']))
        self.assertFalse(matrix['server_kernel_bytes_verified'])
        for name,count in [('wgc-linux-earth-frames.json',18),('wgc-linux-mars.json',45)]:
            extra=json.loads((directory/name).read_text(encoding='utf8'))
            self.assertEqual(extra['rows'],count)
            self.assertEqual(extra['errors'],[])
            self.assertEqual(extra['linux_profile']['verified_profile'], 'native')
            self.assertGreater(extra['linux_profile']['backend_modes'].get('size',0),0)
            self.assertFalse(extra['server_kernel_bytes_verified'])
        trig=json.loads((directory/'glibc-trig-corpus-metadata.json').read_text(encoding='utf8'))
        self.assertEqual(trig['count'],500000)
        self.assertEqual(trig['mismatching_pairs'],0)
        self.assertEqual(trig['source_glibc_sha256'],trig['source_port_sha256'])
        self.assertEqual(len((ROOT/'tests/golden/glibc_trig_exact.csv').read_text().splitlines())-2,trig['fixture_rows'])
        self.assertEqual(digest(ROOT/'tests/golden/glibc_trig_exact.csv'),trig['fixture_sha256'])
        windows_report=json.loads((directory/'wgc-windows-glibc-calculators.json').read_text(encoding='utf8'))
        self.assertEqual(len(windows_report['cases']),19)
        self.assertTrue(all(not case['errors'] for case in windows_report['cases']))
        self.assertTrue(windows_report['backend']['glibc_rotation'])
        self.assertEqual(windows_report['backend']['verified_profile'],'glibc-fma-r3')
        self.assertEqual(windows_report['build_profile'],'release')
        self.assertGreater(windows_report['backend']['modes'].get('frame_rep',0),0)
        self.assertGreater(windows_report['backend']['modes'].get('location',0),0)
        for name,count in [('wgc-windows-glibc-earth-frames.json',18),('wgc-windows-glibc-mars.json',45)]:
            extra=json.loads((directory/name).read_text(encoding='utf8'))
            self.assertEqual(extra['rows'],count)
            self.assertEqual(extra['errors'],[])
            self.assertTrue(extra['backend']['glibc_rotation'])
            self.assertEqual(extra['backend']['verified_profile'],'glibc-fma-r3')
        windows_finders=json.loads((directory/'wgc-windows-glibc-finders.json').read_text(encoding='utf8'))
        self.assertEqual(len(windows_finders['cases']),12)
        self.assertTrue(windows_finders['glibc_rotation'])
        self.assertEqual(windows_finders['verified_profile'],'glibc-fma-r3')
        self.assertTrue(all(not case['failures'] for case in windows_finders['cases']))

    def test_finder_traces_match_case_requests(self):
        directory = ROOT / 'docs/reference/wgc-finders'
        rows = [line.split('  ', 1) for line in (directory / 'SHA256SUMS.txt').read_text().splitlines()]
        self.assertEqual({name for _, name in rows}, {p.name for p in directory.iterdir() if p.suffix in ('.jsonl','.json')})
        cases = json.loads((ROOT / 'docs/review-2026-11-11/gf-all-families/cases.json').read_text(encoding='utf8'))
        self.assertEqual(len(rows), 34)
        for expected, name in rows:
            self.assertEqual(digest(directory / name), expected, name)
            if name.startswith('finder-matrix-'):
                continue
            case_id = name.removeprefix('finder-').removesuffix('.jsonl')
            case = next(case for case in cases if case['id'] == case_id)
            events = [json.loads(line) for line in (directory / name).read_text(encoding='utf8').splitlines()]
            requests = [event['request'] for event in events if event['method'] == 'POST']
            results = [event for event in events if event['path'].endswith('/results')]
            self.assertEqual(requests, [case['payload']])
            self.assertEqual(len(results), 1)
            self.assertEqual(json.loads(results[0]['raw_response'])['status'], 'OK')

    def test_finder_matrix_retains_before_and_after_results(self):
        directory = ROOT / 'docs/reference/wgc-finders'
        data = json.loads((directory/'finder-matrix-cases.json').read_text(encoding='utf8'))
        cases = data['cases']
        sys.path.insert(0, str(ROOT/'tools'))
        from replay_wgc_finder_matrix import validate_matrix
        validate_matrix(cases)
        for invalid in ([], cases[:-1], cases[:-1] + [cases[0]]):
            with self.assertRaises(RuntimeError):
                validate_matrix(invalid)
        tampered = [dict(case) for case in cases]
        tampered[0]['local'] = tampered[1]['local']
        with self.assertRaises(RuntimeError):
            validate_matrix(tampered)
        self.assertEqual(len(cases), 72)
        self.assertFalse(data['server_kernel_bytes_verified'])
        self.assertEqual(data['cspice'], 'CSPICE_N0067')
        failing = [case for case in cases if case['before_fix_errors']]
        self.assertEqual([case['id'] for case in failing], ['intercept_LT_3', 'intercept_LT_4', 'intercept_LT_5'])
        self.assertEqual(sum(len(case['before_fix_errors']) for case in failing), 8)
        after = json.loads((directory/'finder-matrix-after.json').read_text(encoding='utf8'))
        self.assertEqual(after['capture_sha256'], digest(directory/'finder-matrix-cases.json'))
        self.assertEqual(after['verified_profile'], 'glibc-fma-r3')
        self.assertEqual(len(after['cases']), 72)
        self.assertTrue(all(not row['native_errors'] and not row['wgc_errors'] for row in after['cases']))
        native_after = json.loads((directory/'finder-matrix-windows-native-after.json').read_text(encoding='utf8'))
        self.assertEqual(native_after['capture_sha256'], digest(directory/'finder-matrix-cases.json'))
        self.assertEqual(native_after['verified_profile'], 'native')
        self.assertEqual(len(native_after['cases']), 72)
        self.assertTrue(all(not row['native_errors'] and not row['wgc_errors'] for row in native_after['cases']))
        for name in {case['trace'] for case in cases}:
            events = [json.loads(line) for line in (directory/name).read_text(encoding='utf8').splitlines()]
            posts = [event['request'] for event in events if event['method'] == 'POST']
            expected = [case['payload'] for case in cases if case['trace'] == name]
            self.assertEqual(posts, expected)
            self.assertEqual(sum(event['path'].endswith('/results') for event in events), len(expected))

    def test_ray_stellar_reference_and_invalid_options(self):
        directory = ROOT / 'docs/reference/wgc-finders'
        events = [json.loads(line) for line in (directory/'finder-matrix-ray-corrections.jsonl').read_text(encoding='utf8').splitlines()]
        posts = [event for event in events if event['method'] == 'POST']
        self.assertEqual([event['request']['aberrationCorrection'] for event in posts], ['S', 'XS', 'LT', 'CN+S'])
        responses = [json.loads(event['raw_response']) for event in posts]
        self.assertEqual([row['status'] for row in responses], ['OK', 'OK', 'ERROR', 'ERROR'])
        self.assertTrue(all('SPICE(INVALIDOPTION)' in row['error']['shortDescription'] for row in responses[2:]))
        native = json.loads((directory/'finder-matrix-ray-native.json').read_text(encoding='utf8'))
        self.assertEqual(native['cspice'], 'CSPICE_N0067')
        self.assertEqual([case['payload']['aberrationCorrection'] for case in native['cases']], ['S', 'XS'])
        after = json.loads((directory/'finder-matrix-ray-after.json').read_text(encoding='utf8'))
        self.assertEqual(after['trace_sha256'], digest(directory/'finder-matrix-ray-corrections.jsonl'))
        self.assertEqual(after['capture_sha256'], digest(directory/'finder-matrix-ray-native.json'))
        self.assertEqual(after['invalid_options_checked'], ['LT', 'CN+S'])
        self.assertEqual(len(after['cases']), 2)
        self.assertTrue(all(not row['wgc_errors'] and not row['native_errors'] for row in after['cases']))
        empty = json.loads((directory/'finder-matrix-ray-empty-native.json').read_text(encoding='utf8'))
        self.assertEqual([row['short'] for row in empty['rows']], ['SPICE(EMPTYSTRING)', 'SPICE(INVALIDOPTION)', 'SPICE(INVALIDOPTION)', 'SPICE(INVALIDOPTION)'])
        empty_events = [json.loads(line) for line in (directory/'finder-matrix-ray-empty.jsonl').read_text(encoding='utf8').splitlines()]
        self.assertEqual([event['request']['aberrationCorrection'] for event in empty_events], ['', ' '])
        self.assertTrue(all('java.lang.IllegalArgumentException' in json.loads(event['raw_response'])['error']['shortDescription'] for event in empty_events))

    def test_occultation_matrix_includes_empty_windows(self):
        directory = ROOT / 'docs/reference/wgc-finders'
        data = json.loads((directory/'finder-matrix-occultation-cases.json').read_text(encoding='utf8'))
        sys.path.insert(0, str(ROOT/'tools'))
        from replay_wgc_finder_matrix import validate_occultation_matrix
        validate_occultation_matrix(data['cases'])
        with self.assertRaises(RuntimeError):
            validate_occultation_matrix(data['cases'][:-1])
        self.assertEqual(sum(not case['native'] for case in data['cases']), 3)
        self.assertTrue(all(not case['errors'] for case in data['cases']))
        after = json.loads((directory/'finder-matrix-occultation-after.json').read_text(encoding='utf8'))
        self.assertEqual(after['capture_sha256'], digest(directory/'finder-matrix-occultation-cases.json'))
        self.assertEqual(len(after['cases']), 12)
        self.assertTrue(all(not row['wgc_errors'] and not row['native_errors'] for row in after['cases']))
        events = [json.loads(line) for line in (directory/'finder-matrix-occultation.jsonl').read_text(encoding='utf8').splitlines()]
        self.assertEqual([event['request'] for event in events if event['method'] == 'POST'],
                         [case['payload'] for case in data['cases']])

    def test_target_fov_correction_references(self):
        directory = ROOT / 'docs/reference/wgc-finders'
        native = json.loads((directory/'finder-matrix-target-fov-native.json').read_text(encoding='utf8'))
        self.assertEqual([case['payload']['aberrationCorrection'] for case in native['cases']], ['NONE', 'LT', 'CN+S'])
        events = [json.loads(line) for line in (directory/'finder-matrix-target-fov.jsonl').read_text(encoding='utf8').splitlines()]
        self.assertEqual([event['request'] for event in events if event['method'] == 'POST'],
                         [case['payload'] for case in native['cases']])
        after = json.loads((directory/'finder-matrix-target-fov-after.json').read_text(encoding='utf8'))
        self.assertEqual(after['capture_sha256'], digest(directory/'finder-matrix-target-fov-native.json'))
        self.assertEqual(after['trace_sha256'], digest(directory/'finder-matrix-target-fov.jsonl'))
        self.assertEqual(len(after['cases']), 3)
        self.assertTrue(all(not row['wgc_errors'] and not row['native_errors'] for row in after['cases']))

    def test_live_calculator_trace_hashes_and_request_counts(self):
        directory = ROOT / 'docs/reference/wgc-calculators'
        rows = [line.split('  ', 1) for line in (directory / 'SHA256SUMS.txt').read_text().splitlines()]
        self.assertEqual({name for _, name in rows}, {p.name for p in directory.iterdir() if p.suffix in ('.jsonl','.json')})
        for expected, name in rows:
            self.assertEqual(digest(directory / name), expected, name)
        expected = {'state-object.jsonl':45, 'state-representations.jsonl':80,
                    'state-locations.jsonl':29, 'separation-objects.jsonl':14,
                    'angular-size.jsonl':18, 'angular-size-kernelsets.jsonl':2,
                    'angular-size-earth-grid.jsonl':1,
                    'angular-size-limb-matrix.jsonl':15,
                    'angular-size-triaxial-mars.jsonl':15,
                    'angular-size-earth-frames.jsonl':6,
                    'frame-transformation.jsonl':87, 'illumination.jsonl':3,
                    'phase-angle.jsonl':6, 'phase-angle-expanded.jsonl':9, 'pointing.jsonl':7,
                    'sub-solar.jsonl':12, 'sub-observer.jsonl':12,
                    'sub-observer-kernelsets.jsonl':2, 'sub-observer-lst.jsonl':1,
                    'sub-observer-lst-matrix.jsonl':30,
                    'surface-intercept.jsonl':2, 'tangent.jsonl':9,
                    'orbital-elements.jsonl':2, 'solar-longitude.jsonl':2}
        for name, count in expected.items():
            events = [json.loads(line) for line in (directory / name).read_text(encoding='utf8').splitlines()]
            self.assertEqual(sum(event['method'] == 'POST' for event in events), count, name)
            self.assertEqual(sum(event['path'].endswith('/results') for event in events), count, name)
        phase = json.loads((directory/'phase-angle-expanded-native.json').read_text(encoding='utf8'))
        reversed_phase = json.loads((directory/'phase-angle-expanded-native-reversed.json').read_text(encoding='utf8'))
        self.assertEqual([row['name'] for row in phase['kernels_in_load_order']], ['mar099s.bsp', 'de442.bsp'])
        self.assertEqual([row['name'] for row in reversed_phase['kernels_in_load_order']], ['de442.bsp', 'mar099s.bsp'])
        self.assertEqual(len(phase['rows']), 27)
        self.assertEqual(len(reversed_phase['rows']), 27)
        self.assertTrue(all(row['bits_equal'] for row in phase['rows']))
        self.assertTrue(all(row['bits_equal'] for row in reversed_phase['rows']))
        self.assertTrue(any(a['native_rad'] != b['native_rad'] for a,b in zip(phase['rows'], reversed_phase['rows'])))
        fixture = (ROOT/'tests/golden/phase_expanded_exact.csv').read_text(encoding='utf8').splitlines()[2:]
        earth_rows = [row for row in phase['rows'] if row['query'][0] != 499]
        self.assertEqual(len(fixture), 18)
        for line, row in zip(fixture, earth_rows):
            fields = line.split(',')
            self.assertEqual(fields[:5], [str(x) for x in row['query']])
            bits = struct.unpack('>Q', struct.pack('>d', float.fromhex(row['native_rad'])))[0]
            self.assertEqual(fields[5], f'{bits:016x}')
        windows=json.loads((directory/'angular-size-native-windows.json').read_text())
        linux=json.loads((directory/'angular-size-native-linux.json').read_text())
        frames=json.loads((directory/'angular-size-earth-frames-native.json').read_text(encoding='utf8'))
        edlimb_errors=json.loads((directory/'edlimb-errors-native.json').read_text(encoding='utf8'))
        self.assertEqual([row['short'] for row in edlimb_errors['rows']], ['SPICE(DEGENERATECASE)']*4)
        fixture=(ROOT/'tests/golden/angular_size_frames_exact.csv').read_text(encoding='utf8').splitlines()[2:]
        self.assertEqual(len(fixture),18)
        self.assertEqual(len(frames['rows']),18)
        for line,row in zip(fixture,frames['rows']):
            f=line.split(',')
            self.assertEqual(f[:3],[row['frame'],row['correction'],str(row['et'])])
            bits=[f"{struct.unpack('>Q',struct.pack('>d',float.fromhex(x)))[0]:016x}" for x in row['width_rad_hex']]
            self.assertEqual(f[3:],bits)
        self.assertEqual(windows['cspice'],'CSPICE_N0067')
        self.assertEqual(linux['cspice'],'CSPICE_N0067')
        self.assertEqual(windows['rows'],linux['rows'])
        self.assertEqual(windows['kernels'],linux['kernels'])
        self.assertEqual(len(windows['rows']),3)
        errors=[json.loads(line) for line in (directory/'angular-size-error-path.jsonl').read_text(encoding='utf8').splitlines()]
        self.assertEqual(len(errors),3)
        descriptions=[json.loads(event['raw_response'])['error']['shortDescription'] for event in errors]
        self.assertIn('CSPICE.edlimb',descriptions[0])
        self.assertIn('SPICE(DEGENERATECASE)',descriptions[0])

    def test_coincident_calculator_and_finder_observations(self):
        directory = ROOT / 'docs/reference/angular-separation'
        metadata = json.loads((directory / 'capture.json').read_text(encoding='utf8'))
        for name, field in [('wgc-coincident.jsonl','wgc_trace_sha256'),
                            ('wgc-finder-coincident.jsonl','wgc_finder_trace_sha256')]:
            self.assertEqual(digest(directory / name), metadata[field])
        events = [json.loads(line) for line in (directory / 'wgc-coincident.jsonl').read_text(encoding='utf8').splitlines()]
        results = [json.loads(event['raw_response']) for event in events if event['path'].endswith('/results')]
        self.assertEqual([row['rows'][0][1] for row in results], [0.0, -0.90808012, -0.90808012, 0.0, 0.0])
        failures = [json.loads(event['raw_response']) for event in events if event['method'] == 'POST' and json.loads(event['raw_response'])['status'] == 'ERROR']
        self.assertEqual(len(failures), 1)
        self.assertIn('inside target 1 body', failures[0]['error']['shortDescription'])
        finder = json.loads((directory / 'wgc-finder-coincident.jsonl').read_text(encoding='utf8'))
        self.assertIn('SPICE(BODIESNOTDISTINCT)', json.loads(finder['raw_response'])['error']['shortDescription'])


if __name__ == '__main__':
    unittest.main()
