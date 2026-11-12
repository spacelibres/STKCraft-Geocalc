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
 def test_exact_snapshot_stages_without_network(self):
  snapshot=self.root/'snapshot.bpc';snapshot.write_bytes(b'original native kernel bytes')
  sha=hashlib.sha256(snapshot.read_bytes()).hexdigest();dest=self.root/'target/kernel.bpc'
  with patch('urllib.request.urlopen',side_effect=AssertionError('network must not run')):
   bootstrap.ensure_download(dest,'https://naif.jpl.nasa.gov/removed.bpc',sha,snapshot_path=snapshot)
  self.assertEqual(dest.read_bytes(),snapshot.read_bytes())
 def test_changed_snapshot_fails_without_replacing_existing_file(self):
  snapshot=self.root/'snapshot.bpc';snapshot.write_bytes(b'changed');dest=self.root/'kernel';dest.write_bytes(b'prior')
  with patch('urllib.request.urlopen',side_effect=AssertionError('network must not run')):
   with self.assertRaisesRegex(RuntimeError,'snapshot'):
    bootstrap.ensure_download(dest,'https://naif.jpl.nasa.gov/removed.bpc','a'*64,snapshot_path=snapshot)
  self.assertEqual(dest.read_bytes(),b'prior')
 def test_missing_snapshot_fails_explicitly(self):
  with self.assertRaisesRegex(RuntimeError,'snapshot'):
   bootstrap.ensure_download(self.root/'dest','https://naif.jpl.nasa.gov/removed.bpc','a'*64,snapshot_path=self.root/'missing')
 def test_verify_only_does_not_stage_snapshot(self):
  snapshot=self.root/'snapshot';snapshot.write_bytes(b'good');dest=self.root/'dest'
  with self.assertRaisesRegex(RuntimeError,'Missing or changed pinned file'):
   bootstrap.ensure_download(dest,'https://naif.jpl.nasa.gov/removed.bpc',hashlib.sha256(b'good').hexdigest(),verify_only=True,snapshot_path=snapshot)
  self.assertFalse(dest.exists())
 def test_captured_earth_snapshot_matches_canonical_pin(self):
  name='earth_000101_270205_261110.bpc';root=bootstrap.ROOT
  manifest=json.loads((root/'docs/reference/CSPICE_N0067.json').read_text(encoding='utf-8'))
  capture=json.loads((root/'docs/reference/kernel-snapshots/capture.json').read_text(encoding='utf-8'))
  snapshot=root/bootstrap.SNAPSHOTS[name]
  self.assertEqual(bootstrap.digest(snapshot),manifest['kernels_sha256'][name])
  self.assertEqual(bootstrap.digest(snapshot),capture['sha256'])
  self.assertEqual(snapshot.stat().st_size,capture['bytes'])
  original=json.loads((root/capture['original_capture']).read_text(encoding='utf-8'))
  original_row=next(row for row in original['files'] if row['filename']==name)
  self.assertEqual(bootstrap.digest(snapshot),original_row['sha256'])
 def test_earth_snapshot_inventory_retains_official_source(self):
  root=bootstrap.ROOT;manifest=json.loads((root/'docs/reference/CSPICE_N0067.json').read_text(encoding='utf-8'))
  row=next(row for row in bootstrap.pinned_kernel_records(manifest) if row['name']=='earth_000101_270205_261110.bpc')
  self.assertEqual(row['snapshot'],bootstrap.SNAPSHOTS[row['name']])
  self.assertTrue(row['url'].startswith(bootstrap.NAIF))
  self.assertEqual(row['sha256'],manifest['kernels_sha256'][row['name']])
