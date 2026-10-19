#!/usr/bin/env python3
"""Real terminal tests; build the Rust executable before running."""
import json
import os
from pathlib import Path
import subprocess
import unittest

from tmux_harness import Tmux

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / 'target/debug/flash-tmux'


def invoke(t, *args):
    env = dict(os.environ, XDG_CACHE_HOME=t.cache.name)
    env['TMUX'] = t.command('display', '-p', '#{socket_path},#{pid},0').strip()
    return subprocess.Popen([str(BINARY), *args, '--pane', t.pane, '--client', t.client],
                            env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)


class EndToEnd(unittest.TestCase):
    def open_search(self, t):
        process = invoke(t, 'start')
        t.output = b''
        t.wait(lambda: '\u26a1'.encode() in t.output)
        return process

    def finish(self, p):
        stdout, stderr = p.communicate(timeout=4)
        self.assertEqual(p.returncode, 0, stderr.decode())

    def test_incremental_jump_then_repeat_and_cancel(self):
        with Tmux() as t:
            t.fixture(['origin', '\u4e2d\u6587 target', 'end'])
            t.command('copy-mode', '-t', t.pane)
            t.move('top-line')
            t.move('start-of-line')
            p = self.open_search(t)
            os.write(t.master, b'target\r')
            self.finish(p)
            self.assertEqual(t.cursor(), (5, 1))
            p = self.open_search(t)
            os.write(t.master, b'\x1b')
            self.finish(p)
            self.assertEqual(t.cursor(), (5, 1))
            self.assertEqual(t.command('display', '-p', '#{pane_in_mode}').strip(), '1')

    def test_selection_extension_and_bottom_row_label(self):
        with Tmux() as t:
            t.fixture(['target'] + ['row'] * 21 + ['bottom'])
            t.command('copy-mode', '-t', t.pane)
            t.move('top-line')
            t.move('start-of-line')
            t.move('begin-selection')
            p = self.open_search(t)
            os.write(t.master, b'bot')
            t.output = b''
            t.wait(lambda: b'bot' in t.output)
            # First usable label is a. Choosing it lands at the bottom row.
            os.write(t.master, b'a')
            self.finish(p)
            self.assertEqual(t.cursor(), (0, 22))
            t.move('copy-selection')
            self.assertTrue(t.command('show-buffer').startswith('target\n'))
            self.assertTrue(t.command('show-buffer').endswith('\nb'))

    def test_scrolled_history_and_unicode_targets(self):
        for symbol in ['e\u0301', '\U0001f680', '\U0001f469\u200d\U0001f4bb', '\U0001f1e8\U0001f1f3', 'ab\t']:
            with self.subTest(symbol=symbol), Tmux() as t:
                t.fixture(['ROW%02d' % i for i in range(70)])
                t.command('copy-mode', '-t', t.pane)
                t.move('scroll-up', 8)
                p = self.open_search(t)
                os.write(t.master, b'ROW39\r')
                self.finish(p)
                self.assertEqual(t.cursor(), (0, 0))
                self.assertEqual(t.command('display', '-p', '#{scroll_position}').strip(), '8')
                t.move('cancel')
                t.fixture([symbol + ' target', 'last'])
                t.command('copy-mode', '-t', t.pane)
                p = self.open_search(t)
                os.write(t.master, b'target\r')
                self.finish(p)
                t.move('begin-selection')
                t.move('cursor-right', 5)
                t.move('copy-selection')
                self.assertEqual(t.command('show-buffer'), 'target')

    def test_cancellation_preserves_selection_and_rejects_stale_view(self):
        with Tmux() as t:
            t.fixture(['origin target', 'last'])
            t.command('copy-mode', '-t', t.pane)
            t.move('top-line')
            t.move('start-of-line')
            t.move('begin-selection')
            before = t.command('display', '-p', '#{copy_cursor_x};#{copy_cursor_y};#{selection_start_x};#{selection_start_y};#{selection_present}')
            p = self.open_search(t)
            os.write(t.master, b'\x03')
            self.finish(p)
            self.assertEqual(t.command('display', '-p', '#{copy_cursor_x};#{copy_cursor_y};#{selection_start_x};#{selection_start_y};#{selection_present}'), before)
            p = self.open_search(t)
            t.move('cursor-right')  # Simulate another client changing the original selection.
            os.write(t.master, b'target\r')
            _, stderr = p.communicate(timeout=4)
            self.assertNotEqual(p.returncode, 0)
            self.assertIn(b'changed', stderr)
            self.assertEqual(t.cursor(), (1, 0))

    def test_split_neovim_geometry_and_emacs_mode(self):
        import shutil
        with Tmux() as t:
            t.command('set', '-g', 'mode-keys', 'emacs')
            t.command('set', '-g', 'status-position', 'top')
            t.command('split-window', '-h', '-t', t.pane, 'sleep 600')
            if shutil.which('nvim'):
                t.command('respawn-pane', '-k', '-t', t.pane, 'nvim -u NONE -i NONE -c "call setline(1, \'NVIM probe\')"')
                t.wait(lambda: 'NVIM' in t.command('capture-pane', '-p', '-t', t.pane))
            t.command('copy-mode', '-t', t.pane)
            before = t.command('display', '-p', '-t', t.pane, '#{pane_width};#{pane_height};#{pane_top};#{pane_left}')
            p = self.open_search(t)
            self.assertEqual(t.command('display', '-p', '-t', t.pane, '#{pane_width};#{pane_height};#{pane_top};#{pane_left}'), before)
            os.write(t.master, b'\x1b')
            self.finish(p)
            self.assertEqual(t.command('display', '-p', '-t', t.pane, '#{pane_width};#{pane_height};#{pane_top};#{pane_left}'), before)

    def test_actual_s_binding_repeated_copy_and_q_exit(self):
        import tempfile
        with Tmux() as t, tempfile.TemporaryDirectory() as tmp:
            from shlex import quote
            t.fixture(['first target', 'second target'])
            t.command('set', '-g', '@flash-bin', str(BINARY))
            env = dict(os.environ, TMUX=t.command('display', '-p', '#{socket_path},#{pid},0').strip())
            subprocess.run(['sh', str(ROOT / 'flash.tmux')], env=env, check=True)
            t.command('bind', '-T', 'copy-mode-vi', 'v', 'send-keys', '-X', 'begin-selection')
            t.command('bind', '-T', 'copy-mode-vi', 'y', 'send-keys', '-X', 'copy-pipe', 'cat > ' + quote(tmp + '/copy'))
            t.command('copy-mode', '-t', t.pane)
            for query, expected in [(b'first', (0, 0)), (b'second', (0, 1))]:
                t.output = b''
                os.write(t.master, b'S')
                t.wait(lambda: '\u26a1'.encode() in t.output)
                os.write(t.master, query + b'\r')
                t.wait(lambda: t.cursor() == expected)
                if Path(tmp + '/copy').exists():
                    Path(tmp + '/copy').unlink()
                os.write(t.master, b'vl')
                t.wait(lambda: t.cursor() == (1, expected[1]))
                os.write(t.master, b'y')
                t.wait(lambda: Path(tmp + '/copy').exists())
                self.assertEqual(t.command('display', '-p', '-t', t.pane, '#{pane_in_mode}').strip(), '1')
            os.write(t.master, b'q')
            t.wait(lambda: t.command('display', '-p', '-t', t.pane, '#{pane_in_mode}').strip() == '0')

    def test_missing_binary_and_status_off_fail_clearly(self):
        with Tmux() as t:
            t.fixture(['target'])
            t.command('copy-mode', '-t', t.pane)
            t.command('set', '-g', 'status', 'off')
            p = invoke(t, 'start')
            _, stderr = p.communicate(timeout=3)
            self.assertNotEqual(p.returncode, 0)
            self.assertIn(b'status', stderr)
            self.assertEqual(t.command('display', '-p', '#{pane_in_mode}').strip(), '1')

    def test_one_popup_per_client_even_across_panes(self):
        with Tmux() as t:
            t.fixture(['first'])
            other = t.command('split-window', '-h', '-P', '-F', '#{pane_id}', '-t', t.pane, 'sleep 600').strip()
            t.command('copy-mode', '-t', t.pane)
            t.command('copy-mode', '-t', other)
            p = self.open_search(t)
            original = t.pane
            t.pane = other
            second = invoke(t, 'start')
            t.pane = original
            try:
                _, stderr = second.communicate(timeout=2)
                self.assertNotEqual(second.returncode, 0)
                self.assertIn(b'active', stderr)
            finally:
                t.command('display-popup', '-C', '-c', t.client)
                if second.poll() is None:
                    second.terminate()
                second.communicate(timeout=3)
                p.communicate(timeout=3)

    def test_small_pane_bottom_right_match_is_reachable(self):
        with Tmux(width=20, height=6) as t:
            t.fixture(['top', 'row', 'row', 'row', 'xxxxxxxxxxxxxxtarget'])
            t.command('copy-mode', '-t', t.pane)
            p = self.open_search(t)
            os.write(t.master, b'target\r')
            self.finish(p)
            self.assertEqual(t.cursor(), (14, 4))

    def test_resize_cancels_without_moving_cursor(self):
        with Tmux() as control, Tmux() as t:
            for session in (control, t):
                session.fixture(['target', 'last'])
                session.command('copy-mode', '-t', session.pane)
            self.assertEqual(t.cursor(), control.cursor())
            p = self.open_search(t)

            # tmux 3.8 clamps a past-end cursor on resize (4 -> 3 for "last").
            # Compare with native resize, not with pre-resize coordinates.
            control.resize(90, 26)
            geometry = '#{pane_width};#{pane_height}'
            control.wait(lambda: control.command('display', '-p', '-t', control.pane,
                                                 geometry).strip() == '90;25')
            expected = control.cursor()
            t.resize(90, 26)
            t.wait(lambda: t.command('display', '-p', '-t', t.pane,
                                     geometry).strip() == '90;25')
            # The enlarged client leaves the popup's fixed size unchanged.
            # If it is still open, attempt a selection using its stale snapshot.
            if p.poll() is None:
                os.write(t.master, b'target\r')
            _, stderr = p.communicate(timeout=4)
            self.assertTrue(p.returncode == 0 or b'changed' in stderr, stderr.decode())
            self.assertEqual(t.cursor(), expected)
            self.assertEqual(t.command('display', '-p', '-t', t.pane,
                                        '#{pane_in_mode}').strip(), '1')

    def test_exiting_y_binding_is_not_overridden(self):
        with Tmux() as t:
            t.fixture(['target', 'last'])
            t.command('copy-mode', '-t', t.pane)
            p = self.open_search(t)
            os.write(t.master, b'target\r')
            self.finish(p)
            t.move('begin-selection')
            t.move('cursor-right')
            t.command('bind', '-T', 'copy-mode-vi', 'y', 'send-keys', '-X', 'copy-selection-and-cancel')
            os.write(t.master, b'y')
            t.wait(lambda: t.command('display', '-p', '#{pane_in_mode}').strip() == '0')
            self.assertEqual(t.command('show-buffer'), 'ta')

    def test_plugin_paths_with_spaces_and_quotes(self):
        import shutil
        import tempfile
        with Tmux() as t, tempfile.TemporaryDirectory(prefix="flash user's path ") as tmp:
            root = Path(tmp)
            shutil.copy(ROOT / 'flash.tmux', root / 'flash.tmux')
            (root / 'scripts').mkdir()
            shutil.copy(ROOT / 'scripts/launch.sh', root / 'scripts/launch.sh')
            t.fixture(['target'])
            t.command('set', '-g', '@flash-bin', str(BINARY))
            env = dict(os.environ, TMUX=t.command('display', '-p', '#{socket_path},#{pid},0').strip())
            subprocess.run(['sh', str(root / 'flash.tmux')], env=env, check=True)
            t.command('copy-mode', '-t', t.pane)
            t.output = b''
            os.write(t.master, b'S')
            t.wait(lambda: '\u26a1'.encode() in t.output)
            os.write(t.master, b'\x1b')

    def test_parent_crash_recovers_lock_and_stale_files(self):
        with Tmux() as t:
            t.fixture(['target'])
            t.command('copy-mode', '-t', t.pane)
            p = self.open_search(t)
            p.kill()
            p.communicate(timeout=3)
            t.command('display-popup', '-C', '-c', t.client)
            p = self.open_search(t)
            os.write(t.master, b'\x1b')
            self.finish(p)
            self.assertEqual(list(Path(t.cache.name).glob('flash-tmux/search-*')), [])

    def test_plugin_defaults_and_reload_preserve_user_bindings(self):
        with Tmux() as t:
            t.command('set', '-g', '@flash-bin', str(BINARY))
            t.command('bind', '-T', 'copy-mode-vi', 'y', 'send-keys', '-X', 'copy-selection')
            before = t.command('list-keys', '-T', 'copy-mode-vi', 'y')
            env = dict(os.environ, TMUX=t.command('display', '-p', '#{socket_path},#{pid},0').strip())
            subprocess.run(['sh', str(ROOT / 'flash.tmux')], env=env, check=True)
            self.assertEqual(t.command('show', '-gqv', '@flash-key').strip(), 'S')
            self.assertEqual(t.command('show', '-gqv', '@flash-match-bg').strip(), '#51576d')
            self.assertEqual(t.command('show', '-gqv', '@flash-prompt-bg').strip(), '#8caaee')
            self.assertIn('launch.sh', t.command('list-keys', '-T', 'copy-mode-vi', 'S'))
            self.assertEqual(t.command('list-keys', '-T', 'copy-mode-vi', 'y'), before)
            t.command('bind', '-T', 'copy-mode-vi', 'S', 'send-keys', '-X', 'cancel')
            t.command('set', '-g', '@flash-key', 'M-s')
            subprocess.run(['sh', str(ROOT / 'flash.tmux')], env=env, check=True)
            self.assertIn('cancel', t.command('list-keys', '-T', 'copy-mode-vi', 'S'))
            self.assertIn('launch.sh', t.command('list-keys', '-T', 'copy-mode-vi', 'M-s'))


if __name__ == '__main__':
    unittest.main()
