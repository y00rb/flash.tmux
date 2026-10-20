#!/usr/bin/env python3
"""Syntax workflows on disposable servers only; never the user's session."""
import os
import subprocess
import unittest
from e2e import invoke, ROOT, BINARY
from tmux_harness import Tmux


class SyntaxWorkflows(unittest.TestCase):
    def open(self, t, mode):
        t.command('set-option', '-p', '-t', t.pane,
                  '@flash-treesitter-language', 'rust')
        t.output = b''
        process = invoke(t, 'start', '--mode', mode)
        t.wait(lambda: b'TS' in t.output)
        return process

    def finish(self, process):
        _, stderr = process.communicate(timeout=5)
        self.assertEqual(process.returncode, 0, stderr.decode())

    def test_s_confirm_smallest_and_expand(self):
        with Tmux() as t:
            text = 'fn main() { let value = outer(inner(1)); }'
            t.fixture([text, 'END'])
            t.command('copy-mode', '-t', t.pane)
            t.move('top-line')
            t.move('start-of-line')
            t.move('cursor-right', text.index('1'))
            process = self.open(t, 'treesitter')
            os.write(t.master, b'\r')
            self.finish(process)
            t.move('copy-selection')
            self.assertEqual(t.command('show-buffer'), '1')

    def test_r_search_selects_identifier(self):
        with Tmux() as t:
            t.fixture(['fn main() { let value = 42; }', 'END'])
            t.command('copy-mode', '-t', t.pane)
            process = self.open(t, 'treesitter-search')
            os.write(t.master, b'value\r')
            self.finish(process)
            t.move('copy-selection')
            self.assertEqual(t.command('show-buffer'), 'value')

    def test_actual_s_and_r_bindings(self):
        for key, query, expected in [(b'S', b'\r', '1'), (b'R', b'value\r', 'value')]:
            with self.subTest(key=key), Tmux() as t:
                text = 'fn main() { let value = inner(1); }'
                t.fixture([text, 'END'])
                t.command('set', '-g', '@flash-bin', str(BINARY))
                t.command('set', '-p', '-t', t.pane, '@flash-treesitter-language', 'rust')
                env = dict(os.environ, TMUX=t.command('display', '-p', '#{socket_path},#{pid},0').strip())
                subprocess.run(['sh', str(ROOT / 'flash.tmux')], env=env, check=True)
                for table in ['copy-mode', 'copy-mode-vi']:
                    self.assertIn('treesitter', t.command('list-keys', '-T', table, key.decode()))
                t.command('copy-mode', '-t', t.pane)
                t.move('top-line')
                t.move('start-of-line')
                t.move('cursor-right', text.index('1'))
                t.output = b''
                os.write(t.master, key)
                t.wait(lambda: b'TS rust' in t.output)
                os.write(t.master, query)
                start = text.index('1') if key == b'S' else text.index('value')
                t.wait(lambda: t.command('display', '-p', '#{selection_start_x}').strip() == str(start))
                t.move('copy-selection')
                self.assertEqual(t.command('show-buffer'), expected)

    def test_multiline_range_and_emacs_selection(self):
        for keys in ['vi', 'emacs']:
            with self.subTest(keys=keys), Tmux() as t:
                t.command('set-option', '-w', 'mode-keys', keys)
                rows = ['fn main() {', '    let value = 42;', '}']
                t.fixture(rows + ['END'])
                t.command('copy-mode', '-t', t.pane)
                process = self.open(t, 'treesitter-search')
                os.write(t.master, b'fn\r')
                self.finish(process)
                t.move('copy-selection')
                self.assertEqual(t.command('show-buffer'), '\n'.join(rows))

    def test_auto_detection_failure_reports_cause_without_moving_cursor(self):
        with Tmux() as t:
            t.fixture(['ordinary prose without a language declaration', 'END'])
            t.command('set-option', '-p', '-t', t.pane, '@flash-treesitter-language', 'auto')
            t.command('copy-mode', '-t', t.pane)
            before = t.cursor()
            process = invoke(t, 'start', '--mode', 'treesitter')
            _, stderr = process.communicate(timeout=5)
            self.assertNotEqual(process.returncode, 0)
            self.assertIn('ambiguous syntax language', stderr.decode())
            self.assertNotIn('tmux display-popup:', stderr.decode())
            self.assertEqual(t.cursor(), before)
            t.command('set', '-g', '@flash-bin', str(BINARY))
            env = dict(os.environ, TMUX=t.command('display', '-p', '#{socket_path},#{pid},0').strip())
            result = subprocess.run(['sh', str(ROOT / 'scripts/launch.sh'), t.pane, t.client, 'treesitter'], env=env, capture_output=True, timeout=5)
            self.assertEqual(result.returncode, 0, result.stderr.decode())
            self.assertIn('ambiguous syntax language', t.command('show-messages'))
            self.assertEqual(t.cursor(), before)

    def test_cancel_preserves_existing_selection(self):
        with Tmux() as t:
            t.fixture(['fn main() { let value = 42; }', 'END'])
            t.command('copy-mode', '-t', t.pane)
            t.move('top-line')
            t.move('start-of-line')
            t.move('begin-selection')
            t.move('cursor-right', 4)
            fmt = '#{copy_cursor_x};#{copy_cursor_y};#{selection_present};#{selection_start_x};#{selection_start_y}'
            before = t.command('display', '-p', '-t', t.pane, fmt)
            process = self.open(t, 'treesitter')
            os.write(t.master, b'\x03')
            self.finish(process)
            self.assertEqual(t.command('display', '-p', '-t', t.pane, fmt), before)


if __name__ == '__main__':
    unittest.main()
