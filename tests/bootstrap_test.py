import os
from pathlib import Path
import shutil
import subprocess
import unittest

import install_test

ROOT = Path(__file__).resolve().parents[1]


class Bootstrap(unittest.TestCase):
    setUp = install_test.Installer.setUp
    tearDown = install_test.Installer.tearDown

    def prepare(self, remote='https://github.com/y00rb/flash.tmux.git'):
        for name in ['ensure.sh', 'common.sh']:
            shutil.copy(ROOT / 'scripts' / name, self.plugin / 'scripts' / name)
        (self.plugin / 'VERSION').write_text('0.1.0\n')
        subprocess.run(['git', 'init', '-q', str(self.plugin)], check=True)
        subprocess.run(['git', '-C', str(self.plugin), 'remote', 'add', 'origin', remote], check=True)

    def ensure(self, *options):
        return subprocess.run(['sh', str(self.plugin / 'scripts/ensure.sh'), *options], env=self.env, capture_output=True, text=True)

    def test_github_origin_install_and_offline_reload(self):
        self.prepare()
        result = self.ensure()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.env['FAIL_DOWNLOAD'] = '1'
        self.assertEqual(self.ensure().returncode, 0)
        self.assertEqual(self.ensure('--check').returncode, 0)

    def test_ssh_origins_are_supported(self):
        for remote in ['git@github.com:y00rb/flash.tmux.git', 'ssh://git@github.com/y00rb/flash.tmux.git']:
            self.prepare(remote) if not (self.plugin / '.git').exists() else subprocess.run(['git', '-C', str(self.plugin), 'remote', 'set-url', 'origin', remote], check=True)
            result = self.ensure()
            self.assertEqual(result.returncode, 0, result.stderr)
            shutil.rmtree(self.plugin / 'bin')

    def test_bad_remote_does_not_leak_credentials(self):
        self.prepare('https://SECRET_TOKEN@github.com/y00rb/flash.tmux.git')
        result = self.ensure()
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn('SECRET_TOKEN', result.stderr)

    def test_check_never_downloads_and_mismatched_version_fails(self):
        self.prepare()
        result = self.ensure('--check')
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((self.plugin / 'bin/flash-tmux').exists())
        self.assertEqual(self.ensure().returncode, 0)
        (self.plugin / 'VERSION').write_text('0.1.1\n')
        self.env['FAIL_DOWNLOAD'] = '1'
        self.assertNotEqual(self.ensure('--check').returncode, 0)
        self.assertNotEqual(self.ensure().returncode, 0)
        self.assertIn('0.1.0', subprocess.check_output([str(self.plugin / 'bin/flash-tmux')], text=True))

    def test_tpm_entrypoint_installs_and_registers_without_cargo(self):
        from tmux_harness import Tmux
        self.prepare('https://git::@github.com/y00rb/flash.tmux.git')
        shutil.copy(ROOT / 'flash.tmux', self.plugin / 'flash.tmux')
        shutil.copy(ROOT / 'scripts/launch.sh', self.plugin / 'scripts/launch.sh')
        with Tmux() as tmux:
            self.env['TMUX'] = tmux.command('display', '-p', '#{socket_path},#{pid},0').strip()
            result = subprocess.run(['sh', str(self.plugin / 'flash.tmux')], env=self.env, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn('launch.sh', tmux.command('list-keys', '-T', 'copy-mode-vi', 'S'))
            self.assertTrue((self.plugin / 'bin/flash-tmux').exists())
            self.env['FAIL_DOWNLOAD'] = '1'
            result = subprocess.run(['sh', str(self.plugin / 'flash.tmux')], env=self.env, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)

    def test_concurrent_install_is_serialized(self):
        self.prepare()
        command = ['sh', str(self.plugin / 'scripts/ensure.sh')]
        first = subprocess.Popen(command, env=self.env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        second = subprocess.Popen(command, env=self.env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        for process in [first, second]:
            _, error = process.communicate(timeout=15)
            self.assertEqual(process.returncode, 0, error.decode())
        self.assertEqual(self.ensure('--check').returncode, 0)
        self.assertFalse((self.plugin / 'bin/.install-lock').exists())


if __name__ == '__main__':
    unittest.main()
