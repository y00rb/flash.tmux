import io
import json
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
TARGETS = ['aarch64-apple-darwin', 'x86_64-apple-darwin', 'aarch64-unknown-linux-musl', 'x86_64-unknown-linux-musl']


class Publish(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.tag = 'v0.1.0'
        self.script = ROOT / 'scripts/publish.sh'
        self.dist = self.root / 'dist'
        self.dist.mkdir()
        for target in TARGETS:
            with tarfile.open(self.dist / ('flash-tmux-v0.1.0-' + target + '.tar.gz'), 'w:gz') as archive:
                data = b'binary fixture'
                info = tarfile.TarInfo('flash-tmux')
                info.size = len(data)
                archive.addfile(info, io.BytesIO(data))
        self.state = self.root / 'state.json'
        self.state.write_text(json.dumps({'draft': None, 'calls': []}))
        gh = self.root / 'gh'
        gh.write_text('''#!/usr/bin/env python3
import json,os,sys
from pathlib import Path
p=Path(os.environ['FAKE_GH_STATE']); s=json.loads(p.read_text()); a=sys.argv[1:]
s['calls'].append(a)
if a[0]=='api':
    if os.environ.get('FAIL_API'):
        p.write_text(json.dumps(s)); print('HTTP 403',file=sys.stderr); sys.exit(1)
    if s['draft'] is None:
        p.write_text(json.dumps(s)); print('HTTP 404',file=sys.stderr); sys.exit(1)
    print(str(s['draft']).lower())
elif a[:2]==['release','create']: s['draft']=True
elif a[:2]==['release','upload'] and os.environ.get('FAIL_UPLOAD'):
    p.write_text(json.dumps(s)); sys.exit(1)
elif a[:2]==['release','edit']: s['draft']=False
p.write_text(json.dumps(s))
''')
        gh.chmod(0o755)
        self.env = dict(os.environ, PATH=str(self.root) + ':' + os.environ['PATH'], GH_REPO='y00rb/flash.tmux', FAKE_GH_STATE=str(self.state))

    def tearDown(self):
        self.temp.cleanup()

    def publish(self):
        return subprocess.run(['sh', str(self.script), self.tag, str(self.dist)], env=self.env, capture_output=True, text=True)

    def test_complete_assets_publish_from_draft(self):
        result = self.publish()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse(json.loads(self.state.read_text())['draft'])
        self.assertEqual(len((self.dist / 'SHA256SUMS').read_text().splitlines()), 4)

    def test_missing_target_never_creates_release(self):
        next(self.dist.glob('*.tar.gz')).unlink()
        self.assertNotEqual(self.publish().returncode, 0)
        self.assertEqual(json.loads(self.state.read_text())['calls'], [])

    def test_public_release_is_immutable(self):
        self.state.write_text(json.dumps({'draft': False, 'calls': []}))
        result = self.publish()
        self.assertNotEqual(result.returncode, 0)
        calls = json.loads(self.state.read_text())['calls']
        self.assertFalse(any(call[0] == 'release' for call in calls))

    def test_failed_upload_can_resume_draft(self):
        self.env['FAIL_UPLOAD'] = '1'
        self.assertNotEqual(self.publish().returncode, 0)
        self.assertTrue(json.loads(self.state.read_text())['draft'])
        del self.env['FAIL_UPLOAD']
        self.assertEqual(self.publish().returncode, 0)
        self.assertFalse(json.loads(self.state.read_text())['draft'])

    def test_prerelease_does_not_become_latest(self):
        import shutil
        project = self.root / 'prerelease'
        (project / 'scripts').mkdir(parents=True)
        for filename in ['publish.sh', 'check-release.py']:
            shutil.copy(ROOT / 'scripts' / filename, project / 'scripts' / filename)
        (project / 'VERSION').write_text('0.1.0-rc.1' + chr(10))
        (project / 'Cargo.toml').write_text('[package]' + chr(10) + 'version="0.1.0-rc.1"' + chr(10))
        self.tag = 'v0.1.0-rc.1'
        self.script = project / 'scripts/publish.sh'
        for archive in list(self.dist.glob('*.tar.gz')):
            archive.rename(archive.with_name(archive.name.replace('v0.1.0-', self.tag + '-')))
        result = self.publish()
        self.assertEqual(result.returncode, 0, result.stderr)
        edit = [call for call in json.loads(self.state.read_text())['calls'] if call[:2] == ['release', 'edit']][0]
        self.assertIn('--prerelease', edit)
        self.assertIn('--latest=false', edit)

    def test_auth_errors_do_not_create_release(self):
        self.env['FAIL_API'] = '1'
        self.assertNotEqual(self.publish().returncode, 0)
        self.assertFalse(any(call[0] == 'release' for call in json.loads(self.state.read_text())['calls']))


if __name__ == '__main__':
    unittest.main()
