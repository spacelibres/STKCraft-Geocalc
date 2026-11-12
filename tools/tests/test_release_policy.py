import importlib.util
from pathlib import Path
import unittest
import tempfile
from unittest.mock import patch

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
    def test_breaking_minor_requires_migration_evidence(self):
        self.assertEqual(release.validate_increment('0.1.5','0.2.0',True),[])
        for value in ['0.2.1','0.3.0','1.0.0']:
            self.assertTrue(release.validate_increment('0.1.5',value,True))
        self.assertFalse(release.breaking_release_review('### Changed\n- **Breaking:** change', '# Release\n'))
        self.assertFalse(release.breaking_release_review('### Changed\n- change', '## Migration\nupgrade'))
        self.assertFalse(release.breaking_release_review('### Changed\n- **Breaking:** change', '## Migration\n\n## Other\ntext'))
        self.assertTrue(release.breaking_release_review('### Changed\n- **Breaking:** change', '## Migration\nupgrade'))
    def test_invalid_semver_rejected(self):
        for value in ['01.1.6','0.1','0.1.6.0','0.1.6-alpha']:
            self.assertTrue(release.validate_increment(None,value))

    def test_staged_migration_notes_must_match_reviewed_worktree(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            (root/'docs/releases').mkdir(parents=True)
            (root/'.agents').mkdir()
            manifest='[package]\nname="stkcraft-geocalc"\nversion="0.2.0"\ndescription="x"\nlicense="MIT"\nrepository="x"\nhomepage="x"\ndocumentation="x"\nreadme="README.md"\n'
            files={'Cargo.toml':manifest,'Cargo.lock':'[[package]]\nname="stkcraft-geocalc"\nversion="0.2.0"\n',
                   'CHANGELOG.md':'## [0.2.0] - 2026-11-13\n### Changed\n- **Breaking:** change\n',
                   'CITATION.cff':'version: "0.2.0"\n',
                   'docs/releases/v0.2.0.md':'# Release\n## Migration\nUpgrade.\n'}
            for name,body in files.items():(root/name).write_text(body,encoding='utf8')
            def fake_git(*args,**kwargs):
                if args==('show','HEAD:Cargo.toml'):return '[package]\nversion="0.1.15"\n'
                if args==('diff','--cached','--name-only'):return '\n'.join(files)
                if args[0]=='show' and args[1].startswith(':'):
                    name=args[1][1:]
                    return '# Release\n' if name.startswith('docs/releases/') else files[name]
                raise AssertionError(args)
            with patch.object(release,'ROOT',root),patch.object(release,'git',side_effect=fake_git):
                self.assertTrue(any('Staged' in failure and 'release' in failure for failure in release.check(staged=True)))
