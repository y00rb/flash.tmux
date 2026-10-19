import hashlib
import io
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


class Installer(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='flash install with spaces ')
        self.root = Path(self.temp.name)
        self.plugin = self.root / 'plugin'
        (self.plugin / 'scripts').mkdir(parents=True)
        shutil.copy(ROOT / 'scripts/install.sh', self.plugin / 'scripts/install.sh')
        if (ROOT / 'scripts/common.sh').exists():
            shutil.copy(ROOT / 'scripts/common.sh', self.plugin / 'scripts/common.sh')
        self.mock = self.root / 'mock'
        self.mock.mkdir()
        self.assets = self.root / 'assets'
        self.assets.mkdir()
        self.archive = 'flash-tmux-v0.1.0-aarch64-apple-darwin.tar.gz'
        data = b'#!/bin/sh\nprintf "flash-tmux 0.1.0\\n"\n'
        with tarfile.open(self.assets / self.archive, 'w:gz') as archive:
            info = tarfile.TarInfo('flash-tmux')
            info.size = len(data)
            info.mode = 0o755
            archive.addfile(info, io.BytesIO(data))
        digest = hashlib.sha256((self.assets / self.archive).read_bytes()).hexdigest()
        (self.assets / 'SHA256SUMS').write_text(digest + '  ' + self.archive + '\n')
        for name, script in {
            'uname': '#!/bin/sh\ncase "$1" in -s) echo "${TEST_OS:-Darwin}";; -m) echo "${TEST_ARCH:-arm64}";; esac\n',
            'curl': '#!/bin/sh\nwhile [ "$#" -gt 0 ]; do case "$1" in -o) out=$2; shift 2;; https:*) url=$1; shift;; *) shift;; esac; done\n[ "${FAIL_DOWNLOAD:-}" != 1 ] || exit 1\ncp "$TEST_ASSETS/${url##*/}" "$out"\n',
        }.items():
            path = self.mock / name
            path.write_text(script)
            path.chmod(0o755)
        self.env = dict(os.environ, PATH=str(self.mock) + ':' + os.environ['PATH'], TEST_ASSETS=str(self.assets))

    def tearDown(self):
        self.temp.cleanup()

    def run_install(self):
        return subprocess.run(['sh', str(self.plugin / 'scripts/install.sh'), '--version', 'v0.1.0', '--repo', 'y00rb/flash.tmux'], env=self.env, capture_output=True, text=True)

    def test_verified_download_and_executable(self):
        result = self.run_install()
        self.assertEqual(result.returncode, 0, result.stderr)
        binary = self.plugin / 'bin/flash-tmux'
        self.assertTrue(os.access(binary, os.X_OK))
        self.assertEqual(subprocess.check_output([str(binary)]).decode().strip(), 'flash-tmux 0.1.0')

    def test_checksum_failure_preserves_existing_install(self):
        (self.plugin / 'bin').mkdir()
        (self.plugin / 'bin/flash-tmux').write_text('existing')
        (self.assets / 'SHA256SUMS').write_text('0' * 64 + '  ' + self.archive + '\n')
        result = self.run_install()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual((self.plugin / 'bin/flash-tmux').read_text(), 'existing')

    def test_each_supported_platform_selects_its_asset(self):
        for system, arch, target in [
            ('Darwin', 'arm64', 'aarch64-apple-darwin'),
            ('Darwin', 'x86_64', 'x86_64-apple-darwin'),
            ('Linux', 'aarch64', 'aarch64-unknown-linux-musl'),
            ('Linux', 'x86_64', 'x86_64-unknown-linux-musl'),
        ]:
            with self.subTest(target=target):
                name = 'flash-tmux-v0.1.0-' + target + '.tar.gz'
                if name != self.archive:
                    shutil.copy(self.assets / self.archive, self.assets / name)
                digest = hashlib.sha256((self.assets / name).read_bytes()).hexdigest()
                (self.assets / 'SHA256SUMS').write_text(digest + '  ' + name + chr(10))
                self.env.update(TEST_OS=system, TEST_ARCH=arch)
                result = self.run_install()
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual((self.plugin / 'bin/install-info').read_text().strip(), '0.1.0 ' + target)

    def test_install_records_version_and_reuses_it_offline(self):
        result = self.run_install()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.plugin / 'bin/install-info').read_text().strip(), '0.1.0 aarch64-apple-darwin')
        self.env['FAIL_DOWNLOAD'] = '1'
        result = subprocess.run(['sh', str(self.plugin / 'scripts/install.sh'), '--version', 'v0.1.0', '--repo', 'y00rb/flash.tmux', '--if-needed'], env=self.env, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_wrong_binary_version_preserves_existing_install(self):
        (self.plugin / 'bin').mkdir()
        (self.plugin / 'bin/flash-tmux').write_text('existing')
        data = bytes([10]).join([b'#!/bin/sh', b'echo "flash-tmux 0.0.9"', b''])
        with tarfile.open(self.assets / self.archive, 'w:gz') as archive:
            info = tarfile.TarInfo('flash-tmux')
            info.size = len(data)
            archive.addfile(info, io.BytesIO(data))
        digest = hashlib.sha256((self.assets / self.archive).read_bytes()).hexdigest()
        (self.assets / 'SHA256SUMS').write_text(digest + '  ' + self.archive + chr(10))
        result = self.run_install()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual((self.plugin / 'bin/flash-tmux').read_text(), 'existing')

    def test_offline_and_unsupported_platform(self):
        self.env['FAIL_DOWNLOAD'] = '1'
        self.assertNotEqual(self.run_install().returncode, 0)
        self.env['TEST_OS'] = 'Plan9'
        self.assertIn('unsupported', self.run_install().stderr.lower())


if __name__ == '__main__':
    unittest.main()
