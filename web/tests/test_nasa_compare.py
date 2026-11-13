import importlib.util
import unittest
from decimal import Decimal
from pathlib import Path
from unittest.mock import patch
import urllib.error

spec = importlib.util.spec_from_file_location('nasa_compare', Path(__file__).parents[1] / 'nasa' / 'compare.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

class ComparisonTests(unittest.TestCase):
    def test_state_payload_preserves_request(self):
        payload = module.payload_for('state_vector', {'target': 301, 'observer': 399, 'epoch_et': 0, 'correction': 'LT+S'}, [1])
        self.assertEqual(payload['target'], '301')
        self.assertEqual(payload['aberrationCorrection'], 'LT+S')
        self.assertEqual(payload['times'], ['0.0'])
        self.assertEqual(payload['kernels'], [{'type': 'KERNEL_SET', 'id': 1}])

    def test_orbital_elements_payload_does_not_require_observer(self):
        payload = module.payload_for('osculating_elements', {'target':301, 'center':399, 'epoch_et':0}, [1])
        self.assertEqual(payload['orbitingBody'], '301')
        self.assertEqual(payload['centerBody'], '399')
        self.assertNotIn('observer', payload)

    def test_finder_mapping_preserves_condition_and_windows(self):
        params = {'target':301,'observer':399,'confinement':[{'start':0,'end':10}], 'step_seconds':1,'relation':'>','reference':0,'coordinate_system':'LATITUDINAL','coordinate':'LONGITUDE'}
        payload = module.payload_for('gf_coordinate_search',params,[1])
        self.assertEqual(payload['intervals'],[{'startTime':'0','endTime':'10'}])
        self.assertEqual(payload['condition']['coordinateSystem'],'LATITUDINAL')
        params.pop('relation')
        self.assertEqual(module.payload_for('gf_coordinate_search',params,[1])['condition']['relationalCondition'], '>')
        params['filter'] = True
        with self.assertRaises(NotImplementedError): module.payload_for('gf_coordinate_search',params,[1])

    def test_exact_serialized_comparison_and_unmapped(self):
        local = {'columns': ['epoch_et', 'x_km'], 'rows': [[0, -291608.3847334454]]}
        remote = {'columns': [{'outputID':'DATE'}, {'outputID':'X'}, {'outputID':'UNKNOWN'}], 'rows': [[0, Decimal('-291608.38473345'), 9]]}
        result = module.compare('state_vector', local, remote)
        self.assertEqual(result['status'], 'partial')
        self.assertTrue(all(cell['equal'] for cell in result['cells']))
        self.assertEqual(result['unmapped'], ['UNKNOWN'])
        remote['rows'][0][1] = Decimal('-291608.38473344')
        self.assertEqual(module.compare('state_vector', local, remote)['status'], 'difference')

    def test_endpoint_validation_rejects_untrusted_or_insecure_hosts(self):
        for url in ['http://wgc2.jpl.nasa.gov/webgeocalc/api', 'https://nasa.gov.evil.test/api', 'https://evil.test/api', 'https://u:p@wgc2.jpl.nasa.gov/api']:
            with self.assertRaises(ValueError): module.validate_url(url)
        self.assertEqual(module.validate_url('https://wgc2.jpl.nasa.gov:8443/webgeocalc/api'), 'https://wgc2.jpl.nasa.gov:8443/webgeocalc/api')

    def test_transport_failure_uses_next_configured_nasa_endpoint(self):
        configured = [{'name':'first','url':'https://wgc2.jpl.nasa.gov:8443/webgeocalc/api','kernel_sets':[1]}, {'name':'second','url':'https://wgc.jpl.nasa.gov:8443/webgeocalc/api','kernel_sets':[2]}]
        response = {'status':'OK', 'columns':[{'outputID':'DATE'}], 'rows':[[0]]}
        request = {'operation':'state_vector','parameters':{'target':301,'observer':399,'epoch_et':0},'local':{'columns':['epoch_et'],'rows':[[0]]}}
        with patch.object(module, 'endpoints', return_value=configured), patch.object(module.wgc.Wgc, 'run', side_effect=[urllib.error.URLError('offline'),response]):
            result = module.main(request)
        self.assertEqual(result['endpoint'],'second')
        self.assertEqual(result['kernel_sets'],[2])
        self.assertEqual(len(result['attempts']),1)

    def test_application_rejection_does_not_retry_on_another_endpoint(self):
        configured = [{'name':'first','url':'https://wgc2.jpl.nasa.gov:8443/webgeocalc/api','kernel_sets':[1]}, {'name':'second','url':'https://wgc.jpl.nasa.gov:8443/webgeocalc/api','kernel_sets':[2]}]
        request = {'operation':'state_vector','parameters':{'target':301,'observer':399,'epoch_et':0},'local':{}}
        with patch.object(module, 'endpoints', return_value=configured), patch.object(module.wgc.Wgc, 'run', side_effect=RuntimeError('invalid request')) as run:
            result = module.main(request)
        self.assertEqual(result['status'],'error')
        self.assertEqual(run.call_count,1)

    def test_unsupported_is_not_misreported_as_match(self):
        with self.assertRaises(NotImplementedError): module.payload_for('unknown', {}, [1])

if __name__ == '__main__': unittest.main()
