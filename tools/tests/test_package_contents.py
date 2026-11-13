"""Check the release archive boundary for the web documentation gallery."""
import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

class PackageContentsTests(unittest.TestCase):
    def test_gallery_is_packaged_without_web_build_dependencies(self):
        result = subprocess.run(
            ['cargo', 'package', '--locked', '--all-features', '--allow-dirty', '--list'],
            cwd=ROOT, capture_output=True, text=True, encoding='utf-8', check=True,
        )
        files = {line.replace('\\', '/') for line in result.stdout.splitlines()}
        for path in ['README.md', 'web/README.md', 'web/public/albatross.svg',
                     'docs/screenshots/01-workspace.jpg', 'docs/screenshots/05-api-reference.jpg']:
            self.assertIn(path, files)
        self.assertFalse(any('/node_modules/' in f or '/.next/' in f or '/target/' in f for f in files))

if __name__ == '__main__':
    unittest.main()
