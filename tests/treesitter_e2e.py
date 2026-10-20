#!/usr/bin/env python3
"""Syntax workflows on disposable servers only; never the user's session."""
import os
import unittest
from e2e import invoke
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
