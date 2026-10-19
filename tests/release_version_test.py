import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


class Versions(unittest.TestCase):
    def test_repository_version_agrees(self):
        result = subprocess.run(['python3', str(ROOT / 'scripts/check-release.py'), '--root', str(ROOT)], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('0.1.0', result.stdout)

    def test_generated_files_ignored_but_docs_remain_unignored(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / '.gitignore').write_text((ROOT / '.gitignore').read_text())
            subprocess.run(['git', 'init', '-q', tmp], check=True)
            for name, ignored in [('target/output', True), ('bin/flash-tmux', True), ('dist/asset', True), ('docs/spec.md', False)]:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text('fixture')
                result = subprocess.run(['git', '-C', tmp, 'check-ignore', '-q', name])
                self.assertEqual(result.returncode, 0 if ignored else 1, name)

    def test_wrong_tag_or_binary_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / 'VERSION').write_text('0.1.0\n')
            (root / 'Cargo.toml').write_text('[package]\nname="flash-tmux"\nversion="0.1.0"\n')
            binary = root / 'binary'
            binary.write_text('#!/bin/sh\necho "flash-tmux 0.0.9"\n')
            binary.chmod(0o755)
            command = ['python3', str(ROOT / 'scripts/check-release.py'), '--root', tmp]
            bad_tag = subprocess.run(command + ['--tag', 'v0.2.0'], capture_output=True, text=True)
            self.assertNotEqual(bad_tag.returncode, 0)
            self.assertIn('version', bad_tag.stderr.lower())
            bad_binary = subprocess.run(command + ['--tag', 'v0.1.0', '--binary', str(binary)], capture_output=True, text=True)
            self.assertNotEqual(bad_binary.returncode, 0)
            self.assertIn('binary', bad_binary.stderr.lower())


if __name__ == '__main__':
    unittest.main()
