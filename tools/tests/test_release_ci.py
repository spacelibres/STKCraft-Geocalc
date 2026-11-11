import copy
import importlib.util
import hashlib
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('release_ci', Path(__file__).resolve().parents[1] / 'verify_release_ci.py')
guard = importlib.util.module_from_spec(spec)
spec.loader.exec_module(guard)
import runtime_profile


class ReleaseCI(unittest.TestCase):
    def setUp(self):
        self.sha = 'a' * 40
        self.repo = 'spacelibres/STKCraft-Geocalc'
        self.run = dict(head_sha=self.sha, head_branch='main', event='push',
                        path='.github/workflows/ci.yml', status='completed', conclusion='success',
                        repository={'full_name': self.repo}, head_repository={'full_name': self.repo})
        self.jobs = [dict(name=name, status='completed', conclusion='success') for name in guard.REQUIRED_JOBS]
        self.inventory = {'src/lib.rs': '0' * 64, 'dependency:stkcraft-time@0.1.18/src/lib.rs': '1' * 64}
        rust = '\n'.join('test exact::' + name + ' ... ok' for name in guard.verify_strict.REQUIRED_TESTS)
        rust += '\ntest result: ok. 123 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;\n'
        self.logs = {'format': '', 'python-verifier': 'Ran 70 tests in 0.1s\n\nOK\n', 'rust-debug': rust, 'rust-release': rust}
        checks = [dict(name=name, exit_code=0) for name in self.logs]
        for row in checks:
            if row['name'].startswith('rust-'):
                row.update(passed=123, failed=0, ignored=0)
        self.report = dict(local_gate_passed=True, failures=[], source_sha256=self.inventory,
                           verified_source_sha256=self.inventory, checks=checks,
                           release_binary_sha256={name: 'a' * 64 for name in guard.BINARY_NAMES})

    def test_exact_main_run_and_complete_evidence(self):
        guard.validate_run(self.run, self.jobs, self.repo, self.sha)
        guard.validate_evidence(self.report, self.logs, self.inventory)

    def test_wrong_commit_branch_event_workflow_or_repository(self):
        for field, value in [('head_sha', 'b' * 40), ('head_branch', 'feature'), ('event', 'pull_request'),
                             ('path', '.github/workflows/other.yml'), ('status', 'in_progress'),
                             ('conclusion', 'failure'), ('repository', {'full_name': 'fork/project'}),
                             ('head_repository', {'full_name': 'fork/project'})]:
            with self.subTest(field=field), self.assertRaises(ValueError):
                guard.validate_run({**self.run, field: value}, self.jobs, self.repo, self.sha)

    def test_failed_skipped_missing_duplicate_and_unexpected_jobs(self):
        variants = [self.jobs[:-1], self.jobs + self.jobs[:1], self.jobs + [dict(name='unexpected')]]
        for conclusion in ['failure', 'skipped', 'cancelled', None]:
            jobs = copy.deepcopy(self.jobs)
            jobs[0]['conclusion'] = conclusion
            variants.append(jobs)
        for jobs in variants:
            with self.subTest(jobs=jobs), self.assertRaises(ValueError):
                guard.validate_run(self.run, jobs, self.repo, self.sha)

    def test_failed_report_and_changed_sources(self):
        for change in [dict(local_gate_passed=False), dict(failures=['mismatch']),
                       dict(source_sha256={}), dict(verified_source_sha256={})]:
            with self.subTest(change=change), self.assertRaises(ValueError):
                guard.validate_evidence({**self.report, **change}, self.logs, self.inventory)

    def test_windows_path_separator_preserves_source_hash_validation(self):
        report = copy.deepcopy(self.report)
        for key in ['source_sha256', 'verified_source_sha256']:
            report[key] = {name.replace('/', '\\'): value for name, value in self.inventory.items()}
        guard.validate_evidence(report, self.logs, self.inventory)
        report['source_sha256']['src\\lib.rs'] = 'f' * 64
        with self.assertRaises(ValueError):
            guard.validate_evidence(report, self.logs, self.inventory)

    def test_missing_failed_or_duplicate_gate_commands(self):
        for checks in [self.report['checks'][:-1], self.report['checks'] + self.report['checks'][:1],
                       [{**row, 'exit_code': 1} for row in self.report['checks']]]:
            with self.assertRaises(ValueError):
                guard.validate_evidence({**self.report, 'checks': checks}, self.logs, self.inventory)

    def test_missing_raw_logs_and_required_native_cases(self):
        for name in self.logs:
            logs = dict(self.logs)
            del logs[name]
            with self.assertRaises(ValueError):
                guard.validate_evidence(self.report, logs, self.inventory)
        logs = dict(self.logs)
        logs['rust-debug'] = logs['rust-debug'].replace(guard.verify_strict.REQUIRED_TESTS[0], 'different')
        with self.assertRaises(ValueError):
            guard.validate_evidence(self.report, logs, self.inventory)

    def test_raw_failed_or_skipped_test_logs(self):
        variants = [('rust-debug', '0 ignored', '1 ignored'), ('rust-release', '0 failed', '1 failed'),
                    ('python-verifier', 'OK', 'OK (skipped=1)'), ('python-verifier', 'OK', 'FAILED (errors=1)')]
        for name, old, new in variants:
            logs = {**self.logs, name: self.logs[name].replace(old, new)}
            with self.subTest(name=name, new=new), self.assertRaises(ValueError):
                guard.validate_evidence(self.report, logs, self.inventory)

    def test_report_counts_must_match_the_raw_log(self):
        report = copy.deepcopy(self.report)
        report['checks'][-1]['passed'] = 124
        with self.assertRaises(ValueError):
            guard.validate_evidence(report, self.logs, self.inventory)

    def test_tested_binary_inventory_is_required(self):
        for inventory in [{}, {'wgc_geometry.exe': 'a' * 64},
                          {name: 'invalid' for name in guard.BINARY_NAMES}]:
            with self.subTest(inventory=inventory), self.assertRaises(ValueError):
                guard.validate_evidence({**self.report, 'release_binary_sha256': inventory}, self.logs, self.inventory)

    def test_tested_binary_bytes_and_runtime_are_required(self):
        with tempfile.TemporaryDirectory() as temp:
            directory = Path(temp)
            inventory = {}
            for name in guard.BINARY_NAMES:
                blob = (name.encode() + b'\0') * 2000
                (directory / name).write_bytes(blob)
                inventory[name] = hashlib.sha256(blob).hexdigest()
            report = {**self.report, 'release_binary_sha256': inventory}
            with patch.object(runtime_profile, 'pe_imports', return_value=['KERNEL32.dll']):
                guard.validate_binaries(directory, report)
                file = directory / 'wgc_gf.exe'
                original = file.read_bytes()
                file.write_bytes(original + b'changed')
                with self.assertRaises(ValueError):
                    guard.validate_binaries(directory, report)
                file.write_bytes(original)
                for imports in [['ucrtbase.dll'], ['api-ms-win-crt-math-l1-1-0.dll'], ['VCRUNTIME140.dll']]:
                    with patch.object(runtime_profile, 'pe_imports', return_value=imports), self.assertRaises(ValueError):
                        guard.validate_binaries(directory, report)
                file.unlink()
                with self.assertRaises(ValueError):
                    guard.validate_binaries(directory, report)


if __name__ == '__main__':
    unittest.main()
