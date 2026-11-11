import importlib.util
import pathlib
import unittest
from unittest.mock import patch

PATH = pathlib.Path(__file__).resolve().parents[1] / 'wgc2_geometry_compare.py'
spec = importlib.util.spec_from_file_location('geometry_compare', PATH)
c = importlib.util.module_from_spec(spec)
spec.loader.exec_module(c)

class StateReference:
    def run(self, payload):
        ids = ['DATE', 'DISTANCE', 'SPEED', 'X', 'Y', 'Z', 'D_X_DT', 'D_Y_DT', 'D_Z_DT', 'TIME_AT_TARGET', 'LIGHT_TIME']
        return {'columns': [{'outputID': i} for i in ids], 'rows': [[et, 0., 0., 0., 0., 0., 0., 0., 0., et, 0.] for et in c.EPOCHS]}

class SyntheticReference:
    """Valid transport shapes to exercise rejection of missing local output."""
    def run(self, payload):
        calc=payload['calculationType']
        rows_per_epoch=1
        if calc=='OSCULATING_ELEMENTS':
            ids=['DATE','PERIFOCAL_DISTANCE','ECCENTRICITY','INCLINATION','ASCENDING_NODE_LONGITUDE',
                 'ARGUMENT_OF_PERIAPSE','MEAN_ANOMALY_AT_EPOCH','ORBITING_BODY_RANGE','ORBITING_BODY_SPEED',
                 'PERIOD','CENTER_BODY_GM','TRUE_ANOMALY','SEMIMAJOR_AXIS']
        elif calc=='FRAME_TRANSFORMATION':
            ids=['DATE','X']
            if payload['orientationRepresentation'] in ('MATRIX_ROW_BY_ROW','MATRIX_FLAGGED'):
                rows_per_epoch=6 if payload['angularVelocityRepresentation']=='MATRIX' else 3
        else:
            rep=payload['stateRepresentation']
            coordinates={
                'RA_DEC':['RIGHT_ASCENSION','DECLINATION','RANGE'],
                'LATITUDINAL':['LONGITUDE','LATITUDE','RADIUS'],
                'CYLINDRICAL':['LONGITUDE','Z','RADIUS'],
                'SPHERICAL':['LONGITUDE','COLATITUDE','RADIUS'],
                'AZ_EL':['AZIMUTH','ELEVATION','RANGE'],
                'PLANETODETIC':['LONGITUDE','LATITUDE','ALTITUDE'],
                'PLANETOGRAPHIC':['LONGITUDE','LATITUDE','ALTITUDE'],
            }
            if rep=='RECTANGULAR':
                ids=['DATE','DISTANCE','SPEED','X','Y','Z','D_X_DT','D_Y_DT','D_Z_DT','TIME_AT_TARGET','LIGHT_TIME']
            else:
                xyz=coordinates[rep]
                ids=['DATE']+xyz+['D_'+i+'_DT' for i in xyz]+['SPEED','TIME_AT_TARGET','LIGHT_TIME']
        rows=[[float(et)]+[0.]*(len(ids)-1) for et in payload['times'] for _ in range(rows_per_epoch)]
        return {'columns':[{'outputID':i} for i in ids],'rows':rows}

class FalsePassRegressions(unittest.TestCase):
    def test_zero_local_rows_cannot_pass_135_reference_rows(self):
        with patch.object(c, 'crate_queries', return_value=[]):
            with self.assertRaisesRegex(RuntimeError, 'row count'):
                c.run_state(StateReference(), '-',)

    def test_extra_local_rows_fail(self):
        with patch.object(c, 'crate_queries', return_value=[['0']]*136):
            with self.assertRaisesRegex(RuntimeError, 'row count'):
                c.run_state(StateReference(), '-')

    def test_nan_local_output_fails(self):
        def output(mode, lines, *args):
            return [q.split(',') + ['nan']*10 for q in lines]
        with patch.object(c, 'crate_queries', side_effect=output):
            with self.assertRaisesRegex(RuntimeError, 'finite'):
                c.run_state(StateReference(), '-')

    def test_wrong_local_identity_fails(self):
        def output(mode, lines, *args):
            return [['999', *q.split(',')[1:]] + ['0']*10 for q in lines]
        with patch.object(c, 'crate_queries', side_effect=output):
            with self.assertRaisesRegex(RuntimeError, 'identity'):
                c.run_state(StateReference(), '-')

    def test_short_reference_rows_fail_before_zip(self):
        class Short(StateReference):
            def run(self, payload):
                r = super().run(payload); r['rows'].pop(); return r
        with patch.object(c, 'crate_queries', return_value=[]):
            with self.assertRaisesRegex(RuntimeError, 'row count'):
                c.run_state(Short(), '-')

    def test_every_comparison_group_rejects_empty_local_output(self):
        # Malformed transport results must never become zero-error successes.
        for name, args, transport in [
            ('run_state_reps', ('-', '-'), 'crate_queries'),
            ('run_elements', ('-', '-'), 'crate_queries'),
            ('run_frames', ('-', '-'), 'crate_frame_rep'),
            ('run_locations', ('-', '-'), 'crate_location_queries')]:
            with self.subTest(group=name), patch.object(c, transport, return_value=[]):
                with self.assertRaisesRegex(RuntimeError, "row count"):
                    getattr(c, name)(SyntheticReference(), *args)

class StrictComparison(unittest.TestCase):
    def test_sub_tolerance_difference_is_failure(self):
        with self.assertRaisesRegex(RuntimeError, 'raw difference'):
            c.strict_difference('1.000000001', '1.000000000', 'X')

    def test_gm_difference_is_failure(self):
        with self.assertRaisesRegex(RuntimeError, 'GM'):
            c.strict_difference('132712440041.94', '132712440000.0', 'GM')

    def test_identical_decimal_values_pass(self):
        self.assertEqual(c.strict_difference('1.0000', 1., 'X'), 0.)

    def test_nonfinite_rejected(self):
        for value in ['nan', 'inf', '-inf']:
            with self.subTest(value=value), self.assertRaisesRegex(RuntimeError, 'finite'):
                c.strict_difference(value, '0', 'X')

    def test_precision_must_be_declared_explicitly(self):
        self.assertGreater(c.strict_difference('1.000000001', '1.00000000', 'X', decimal_places=8), 0.)

    def test_unknown_type_fails(self):
        with self.assertRaises(RuntimeError):
            c.strict_difference(True, 1., 'X')

    def test_table_ids_and_order_required(self):
        ref = {'columns': [{'outputID':'DATE'}, {'outputID':'X'}, {'outputID':'Y'}], 'rows': [[0., 1., 2.]]}
        ours = {'ids':['Y', 'X'], 'rows': [['2', '1']]}
        with self.assertRaisesRegex(RuntimeError, 'IDs'):
            c.compare_table(ref, ours, 'frame')

    def test_table_does_not_skip_numeric_strings_or_blanks(self):
        for value in ['1.000000001', None, '']:
            ref = {'columns':[{'outputID':'DATE'}, {'outputID':'X'}], 'rows':[[0., value]]}
            with self.subTest(value=value), self.assertRaises(RuntimeError):
                c.compare_table(ref, {'ids':['X'], 'rows':[['1']]}, 'location')

class CspiceStrictComparison(unittest.TestCase):
    def test_one_ulp_difference_is_not_cspice_parity(self):
        import math
        with self.assertRaisesRegex(RuntimeError, 'CSPICE'):
            c.compare_cspice_f64(math.nextafter(1., 2.), 1., 'X')

    def test_signed_zero_is_distinguished_for_bit_parity(self):
        with self.assertRaisesRegex(RuntimeError, 'bits'):
            c.compare_cspice_f64(-0., 0., 'X')

    def test_equal_finite_f64_passes(self):
        self.assertEqual(c.compare_cspice_f64(1., 1., 'X'), 0.)

    def test_all_failures_are_collected_in_audit_mode(self):
        original=c.STRICT_FAILURES
        try:
            c.STRICT_FAILURES=[]
            c.strict_difference('1', '2', 'X')
            c.strict_difference('3', '4', 'Y')
            self.assertEqual(len(c.STRICT_FAILURES),2)
        finally: c.STRICT_FAILURES=original

class MatrixReferenceLayout(unittest.TestCase):
    def test_three_matrix_rows_for_one_epoch_are_valid(self):
        class Matrix:
            def run(self,payload):
                return {'columns':[{'outputID':i} for i in ['DATE','ROW_NUMBER','M11','M12','M13']],
                        'rows':[[0.,float(n),1.,0.,0.] for n in (1,2,3)]}
        payload={'calculationType':'FRAME_TRANSFORMATION','orientationRepresentation':'MATRIX_ROW_BY_ROW',
                 'angularVelocityRepresentation':'NOT_INCLUDED',**c.time_block([0.])}
        self.assertEqual(len(c.reference_result(Matrix(),payload)['rows']),3)

    def test_row_numbers_must_be_ordered(self):
        class Matrix:
            def run(self,payload):
                return {'columns':[{'outputID':i} for i in ['DATE','ROW_NUMBER','M11','M12','M13']],
                        'rows':[[0.,float(n),1.,0.,0.] for n in (3,2,1)]}
        payload={'calculationType':'FRAME_TRANSFORMATION','orientationRepresentation':'MATRIX_ROW_BY_ROW',
                 'angularVelocityRepresentation':'NOT_INCLUDED',**c.time_block([0.])}
        with self.assertRaisesRegex(RuntimeError,'order'):
            c.reference_result(Matrix(),payload)

class AuditReporting(unittest.TestCase):
    def test_main_collects_every_group_failure_and_writes_report(self):
        import contextlib
        import io
        import json
        import tempfile
        from contextlib import ExitStack
        names=['run_state','run_state_reps','run_separation','run_phase','run_size','run_elements',
               'run_solar_longitude','run_illumination','run_surface_intercept','run_tangent','run_frames','run_locations','run_sub_observer','run_sub_solar','run_pointing']
        previous=(c.STRICT,c.STRICT_FAILURES)
        try:
            with tempfile.TemporaryDirectory() as directory, ExitStack() as stack:
                report=pathlib.Path(directory)/'report.json'
                for name in names:
                    stack.enter_context(patch.object(c,name,side_effect=RuntimeError('synthetic transport failure')))
                stack.enter_context(patch.object(c.sys,'argv',['compare','--report',str(report)]))
                with contextlib.redirect_stdout(io.StringIO()):
                    self.assertEqual(c.main(),1)
                result=json.loads(report.read_text())
                self.assertEqual(len(result['groups']),15)
                self.assertFalse(result['passed'])
                self.assertTrue(all(not group['passed'] and group['failures'] for group in result['groups']))
        finally:c.STRICT,c.STRICT_FAILURES=previous

class ReferenceValidation(unittest.TestCase):
    def test_swapped_reference_epochs_fail(self):
        class Swapped(StateReference):
            def run(self, payload):
                r=super().run(payload);r['rows'].reverse();return r
        with self.assertRaisesRegex(RuntimeError, 'order/epoch'):
            c.run_state(Swapped(), '-')

    def test_duplicate_column_ids_fail(self):
        class Duplicate(StateReference):
            def run(self, payload):
                r=super().run(payload);r['columns'][2]=r['columns'][1];return r
        with self.assertRaisesRegex(RuntimeError, 'IDs'):
            c.run_state(Duplicate(), '-')

    def test_state_reordered_columns_fail(self):
        class Reordered(StateReference):
            def run(self, payload):
                r=super().run(payload);r['columns'][3],r['columns'][4]=r['columns'][4],r['columns'][3];return r
        with self.assertRaisesRegex(RuntimeError, 'IDs'):
            c.run_state(Reordered(), '-')

    def test_missing_computational_columns_cannot_claim_parity(self):
        with self.assertRaisesRegex(RuntimeError, 'coverage'):
            c.compare_mapped_row(['DATE','INCIDENCE_ANGLE','LTST'],[0.,1.,'12:00:00'],['1'],{'INCIDENCE_ANGLE':0},'illum')

    def test_request_uses_configured_timeout(self):
        wgc=c.Wgc(False, 0.25)
        from unittest.mock import MagicMock
        response=MagicMock();response.__enter__.return_value.read.return_value=b'{}'
        with patch.object(c.urllib.request, 'urlopen', return_value=response) as transport:
            wgc._request('GET','/x')
        self.assertLessEqual(transport.call_args.kwargs['timeout'],0.25)

if __name__ == '__main__': unittest.main()
