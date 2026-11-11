import importlib.util
from pathlib import Path
import unittest
import tempfile
from unittest.mock import patch

spec=importlib.util.spec_from_file_location('strict_gate',Path(__file__).resolve().parents[1]/'verify_strict.py')
gate=importlib.util.module_from_spec(spec);spec.loader.exec_module(gate)

class StrictGate(unittest.TestCase):
    def valid(self):
        return '\n'.join(f'test unit::{name} ... ok' for name in gate.REQUIRED_TESTS)+'\ntest result: ok. 123 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;\n'
    def test_complete_executed_battery(self):
        counts,failures=gate.audit_rust_output(self.valid())
        self.assertEqual(counts,dict(passed=123,failed=0,ignored=0));self.assertEqual(failures,[])
    def test_empty_battery_fails(self):
        self.assertTrue(gate.audit_rust_output('')[1])
    def test_skipped_tests_fail(self):
        self.assertTrue(gate.audit_rust_output(self.valid()+'\nskipping missing kernel')[1])
    def test_ignored_tests_fail(self):
        self.assertTrue(gate.audit_rust_output(self.valid().replace('0 ignored','1 ignored'))[1])
    def test_missing_required_exact_case_fails(self):
        self.assertTrue(gate.audit_rust_output(self.valid().replace(gate.REQUIRED_TESTS[0],'different_test'))[1])
    def test_failed_result_fails(self):
        self.assertTrue(gate.audit_rust_output(self.valid().replace('0 failed','1 failed'))[1])

class ReferenceProvenance(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup)
        self.root=Path(self.temp.name);source=self.root/'target/reference/n0067/cspice/src/cspice'
        source.mkdir(parents=True);(source/'spkgeo.c').write_bytes(b'independent reference')
        archive=self.root/'target/reference/cspice-n0067.zip';archive.write_bytes(b'archive')
        binary=self.root/'cspice.dll';binary.write_bytes(b'native binary')
        self.manifest={'toolkit':'CSPICE N0067','C_source_count':1,
            'archive_sha256':gate.digest(archive),
            'reference_source_sha256':{'spkgeo.c':gate.digest(source/'spkgeo.c')},
            'reference_binary':{'path':str(binary),'sha256':gate.digest(binary),'toolkit_version':'CSPICE_N0067'}}
    def test_pinned_reference_files_pass(self):
        self.assertEqual(gate.audit_reference_files(self.manifest,self.root),[])
    def test_modified_native_binary_fails(self):
        Path(self.manifest['reference_binary']['path']).write_bytes(b'other binary')
        self.assertTrue(gate.audit_reference_files(self.manifest,self.root))
    def test_missing_or_modified_source_fails(self):
        (self.root/'target/reference/n0067/cspice/src/cspice/spkgeo.c').unlink()
        self.assertTrue(gate.audit_reference_files(self.manifest,self.root))
    def test_empty_source_provenance_fails(self):
        self.manifest['reference_source_sha256']={}
        self.assertTrue(gate.audit_reference_files(self.manifest,self.root))

    def test_runtime_binary_override_is_still_hash_checked(self):
        alternate=self.root/'alternate.dll';alternate.write_bytes(b'changed binary')
        with patch.dict('os.environ',{'STKCRAFT_REFERENCE_DLL':str(alternate)}):
            self.assertTrue(gate.audit_reference_files(self.manifest,self.root))
