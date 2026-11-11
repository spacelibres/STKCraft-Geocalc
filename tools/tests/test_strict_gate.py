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
    def test_native_diagnostics_can_interleave_before_success(self):
        name=gate.REQUIRED_TESTS[0]
        output=self.valid().replace(f'{name} ... ok',f'{name} ... EDLIMB 0 differences, max0\nok')
        self.assertEqual(gate.audit_rust_output(output)[1],[])
    def test_interleaved_failure_cannot_be_accepted(self):
        name=gate.REQUIRED_TESTS[0]
        output=self.valid().replace(f'{name} ... ok',f'{name} ... native diagnostics\nFAILED')
        self.assertTrue(gate.audit_rust_output(output)[1])
    def test_next_test_success_cannot_complete_a_missing_status(self):
        name=gate.REQUIRED_TESTS[0]
        output=self.valid().replace(f'{name} ... ok',f'{name} ... native diagnostics')
        self.assertTrue(gate.audit_rust_output(output)[1])
    def test_diagnostic_success_cannot_hide_a_failed_status(self):
        name=gate.REQUIRED_TESTS[0]
        output=self.valid().replace(f'{name} ... ok',f'{name} ... native diagnostics\nok\nFAILED')
        self.assertTrue(gate.audit_rust_output(output)[1])
    def test_similarly_suffixed_name_cannot_replace_a_required_test(self):
        name=gate.REQUIRED_TESTS[0]
        self.assertTrue(gate.audit_rust_output(self.valid().replace(name,'unrelated_'+name))[1])
    def test_later_success_cannot_hide_an_earlier_required_failure(self):
        name=gate.REQUIRED_TESTS[0]
        output=f'test unit::{name} ... FAILED\n'+self.valid()
        self.assertTrue(gate.audit_rust_output(output)[1])

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
