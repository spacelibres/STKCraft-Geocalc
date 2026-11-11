import importlib.util,hashlib,json,tempfile,unittest
from pathlib import Path
from unittest.mock import patch
spec=importlib.util.spec_from_file_location('bootstrap',Path(__file__).resolve().parents[1]/'bootstrap_reference.py')
bootstrap=importlib.util.module_from_spec(spec);spec.loader.exec_module(bootstrap)
class BootstrapInputs(unittest.TestCase):
 def setUp(self):
  self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup);self.root=Path(self.temp.name);self.provenance=self.root/'docs/review-2026-11-11';self.provenance.mkdir(parents=True)
  for name,data in [('wgc-kernel-provenance.json',{'files':[]}),('wgc-kernel-fk-provenance.json',[]),('cassini-fov-kernel-provenance.json',{'downloadedCoveringSubset':[]})]:(self.provenance/name).write_text(json.dumps(data))
 def test_main_pinned_inventory_is_not_empty(self):
  records=bootstrap.pinned_kernel_records({'kernels_sha256':{'de440s.bsp':'a'*64}},self.root);self.assertEqual(len(records),1);self.assertEqual(records[0]['sha256'],'a'*64);self.assertTrue(records[0]['url'].endswith('/spk/planets/de440s.bsp'))
 def test_unknown_url_fails_instead_of_skipping(self):
  with self.assertRaises(ValueError):bootstrap.pinned_kernel_records({'kernels_sha256':{'unknown.bsp':'a'*64}},self.root)
 def test_conflicting_provenance_fails(self):
  (self.provenance/'wgc-kernel-provenance.json').write_text(json.dumps({'files':[{'filename':'de440s.bsp','url':'https://naif.jpl.nasa.gov/pub/naif/example','sha256':'b'*64}]}))
  with self.assertRaises(ValueError):bootstrap.pinned_kernel_records({'kernels_sha256':{'de440s.bsp':'a'*64}},self.root)
 def test_cassini_and_actual_fk_are_required(self):
  (self.provenance/'wgc-kernel-fk-provenance.json').write_text(json.dumps([{'relativePath':'fk/actual.tf','path':'target\\kernels\\actual.tf','sha256':'b'*64}]))
  (self.provenance/'cassini-fov-kernel-provenance.json').write_text(json.dumps({'downloadedCoveringSubset':[{'sourcePath':'fk/cas.tf','url':'https://naif.jpl.nasa.gov/pub/naif/cas.tf','path':'target\\kernels\\cassini\\cas.tf','sha256':'c'*64}]}))
  records=bootstrap.pinned_kernel_records({'kernels_sha256':{'de440s.bsp':'a'*64}},self.root);self.assertEqual({x['name'] for x in records},{'de440s.bsp','actual.tf','cassini/cas.tf'})
 def test_bad_existing_hash_fails_verify_only(self):
  p=self.root/'kernel';p.write_bytes(b'bad')
  with self.assertRaises(RuntimeError):bootstrap.ensure_download(p,'https://naif.jpl.nasa.gov/test','a'*64,verify_only=True)
 def test_good_existing_file_never_uses_network(self):
  p=self.root/'kernel';p.write_bytes(b'good');sha=hashlib.sha256(b'good').hexdigest()
  with patch('urllib.request.urlopen',side_effect=AssertionError('network must not run')):bootstrap.ensure_download(p,'https://naif.jpl.nasa.gov/test',sha)
