import importlib.util
from pathlib import Path
import unittest

spec=importlib.util.spec_from_file_location('release_policy',Path(__file__).resolve().parents[1]/'check_release.py')
release=importlib.util.module_from_spec(spec);spec.loader.exec_module(release)

class VersionedCommits(unittest.TestCase):
    def test_patch_increment(self):
        self.assertEqual(release.validate_increment('0.1.5','0.1.6'),[])
    def test_initial_repository(self):
        self.assertEqual(release.validate_increment(None,'0.1.6'),[])
    def test_same_or_skipped_version_rejected(self):
        for value in ['0.1.5','0.1.7','0.1.4']:
            self.assertTrue(release.validate_increment('0.1.5',value))
    def test_unreviewed_minor_rejected(self):
        self.assertTrue(release.validate_increment('0.1.5','0.2.0'))
    def test_invalid_semver_rejected(self):
        for value in ['01.1.6','0.1','0.1.6.0','0.1.6-alpha']:
            self.assertTrue(release.validate_increment(None,value))
