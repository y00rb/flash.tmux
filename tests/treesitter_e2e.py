#!/usr/bin/env python3
"""Syntax workflows on disposable servers only; never the user's session."""
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch
from e2e import invoke, ROOT, BINARY
from tmux_harness import Tmux


# Tall panes require per-row tmux capture before the first prompt. Hosted macOS
# runners can exceed the harness's 3s default; this is not a latency assertion.
SYNTAX_START_TIMEOUT = 15


class SyntaxWorkflows(unittest.TestCase):
    def wait_for_prompt(self, t, process, language='rust'):
        marker = ('TS ' + language).encode()
        try:
            t.wait(lambda: marker in t.output or process.poll() is not None,
                   timeout=SYNTAX_START_TIMEOUT)
        except AssertionError:
            was_running = process.poll() is None
            if was_running:
                process.kill()
            _, stderr = process.communicate(timeout=5)
            self.fail(f'syntax prompt missing after {SYNTAX_START_TIMEOUT}s; '
                      f'process_running={was_running}; stderr={stderr.decode()!r}; '
                      f'terminal_tail={t.output[-500:]!r}')
        if process.poll() is not None:
            _, stderr = process.communicate(timeout=5)
            self.fail(f'syntax process exited before prompt (status {process.returncode}): '
                      + stderr.decode())

    def open(self, t, mode):
        t.command('set-option', '-p', '-t', t.pane,
                  '@flash-treesitter-language', 'rust')
        t.output = b''
        process = invoke(t, 'start', '--mode', mode)
        self.wait_for_prompt(t, process)
        return process

    def finish(self, process):
        _, stderr = process.communicate(timeout=5)
        self.assertEqual(process.returncode, 0, stderr.decode())

    def test_slow_capture_still_reaches_prompt_and_selects_exact_range(self):
        real_tmux = shutil.which('tmux')
        with Tmux() as t, tempfile.TemporaryDirectory() as tmp:
            t.fixture(['fn main() { let value = 42; }', 'END'])
            t.command('copy-mode', '-t', t.pane)
            marker = Path(tmp) / 'capture-delayed'
            wrapper = Path(tmp) / 'tmux'
            wrapper.write_text('#!/bin/sh\n'
                               'if [ "$1" = capture-pane ] && [ ! -e "$FLASH_SLOW_CAPTURE_MARKER" ]; then\n'
                               '    : > "$FLASH_SLOW_CAPTURE_MARKER"\n'
                               '    sleep 4\n'
                               'fi\nexec ' + shlex.quote(real_tmux) + ' "$@"\n')
            wrapper.chmod(0o755)
            with patch.dict(os.environ, PATH=tmp + os.pathsep + os.environ['PATH'],
                            FLASH_SLOW_CAPTURE_MARKER=str(marker)):
                process = self.open(t, 'treesitter-search')
            self.assertTrue(marker.exists())
            os.write(t.master, b'value\r')
            self.finish(process)
            t.move('copy-selection')
            self.assertEqual(t.command('show-buffer'), 'value')

    def test_prompt_wait_reports_early_process_failure(self):
        with Tmux() as t:
            t.output = b''
            process = invoke(t, 'start', '--mode', 'invalid-mode')
            with self.assertRaisesRegex(AssertionError, 'mode must be search'):
                self.wait_for_prompt(t, process)

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
                t.wait(lambda: b'TS rust' in t.output, timeout=SYNTAX_START_TIMEOUT)
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

    def test_auto_detects_cat_rust_with_shell_output(self):
        with Tmux(width=141, height=65) as t:
            rows = ['INFO previous terminal output %02d' % i for i in range(40)]
            rows += ['paul@host ~/checkout $ cat tests/snippets/rust.rs',
                     'fn main() {', '    let value = 42;',
                     '    if value > 0 {', '        println!("{}", value);',
                     '    }', '}', 'paul@host ~/checkout $']
            t.fixture(rows)
            t.command('set-option', '-p', '-t', t.pane, '@flash-treesitter-language', 'auto')
            t.command('copy-mode', '-t', t.pane)
            t.move('top-line')
            t.move('cursor-down', 42)
            t.move('start-of-line')
            t.move('cursor-right', 8)
            t.output = b''
            process = invoke(t, 'start', '--mode', 'treesitter')
            self.wait_for_prompt(t, process)
            os.write(t.master, b'\r')
            self.finish(process)
            t.move('copy-selection')
            self.assertEqual(t.command('show-buffer'), 'value')

    def test_auto_detects_go_with_old_rust_output(self):
        with Tmux(width=141, height=65) as t:
            rows = ['user@host $ cat tests/snippets/rust.rs',
                    'fn old() { let value = 1; }'] * 20
            rows += ['package sample', '', 'func describe(value int) int {',
                     '    if value > 0 {', '        return value + 1',
                     '    }', '    return 0', '}', 'user@host $']
            t.fixture(rows)
            t.command('set-option', '-p', '-t', t.pane, '@flash-treesitter-language', 'auto')
            t.command('copy-mode', '-t', t.pane)
            t.move('top-line')
            t.move('cursor-down', 44)
            t.move('start-of-line')
            t.move('cursor-right', 15)
            t.output = b''
            process = invoke(t, 'start', '--mode', 'treesitter')
            self.wait_for_prompt(t, process, 'go')
            os.write(t.master, b'\r')
            self.finish(process)
            t.move('copy-selection')
            self.assertEqual(t.command('show-buffer'), 'value')

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

    def test_rectangle_is_cleared_for_confirmed_syntax_range(self):
        with Tmux() as t:
            text = 'fn main() { let value = 42; }'
            t.fixture([text, 'END'])
            t.command('copy-mode', '-t', t.pane)
            t.move('top-line')
            t.move('start-of-line')
            t.move('rectangle-on')
            t.move('begin-selection')
            process = self.open(t, 'treesitter-search')
            os.write(t.master, b'42\r')
            self.finish(process)
            self.assertEqual(t.command('display', '-p', '#{rectangle_toggle}').strip(), '0')
            t.move('copy-selection')
            self.assertEqual(t.command('show-buffer'), '42')

    def test_stale_state_is_rejected_without_overwriting_new_cursor(self):
        with Tmux() as t:
            t.fixture(['fn main() { let value = 42; }', 'END'])
            t.command('copy-mode', '-t', t.pane)
            t.move('top-line')
            t.move('start-of-line')
            process = self.open(t, 'treesitter-search')
            t.move('cursor-right', 3)
            changed = t.cursor()
            os.write(t.master, b'42\r')
            _, stderr = process.communicate(timeout=5)
            self.assertNotEqual(process.returncode, 0)
            self.assertIn('changed', stderr.decode())
            self.assertEqual(t.cursor(), changed)

    def test_resize_cancels_syntax_preview(self):
        with Tmux() as t:
            t.fixture(['fn main() { let value = 42; }', 'END'])
            t.command('copy-mode', '-t', t.pane)
            process = self.open(t, 'treesitter-search')
            t.resize(90, 26)
            self.finish(process)
            self.assertEqual(t.command('display', '-p', '#{pane_in_mode}').strip(), '1')

    def test_s_on_blank_line_selects_enclosing_block(self):
        with Tmux() as t:
            rows = ['fn main() {', '', '    let value = 42;', '}']
            t.fixture(rows)
            t.command('copy-mode', '-t', t.pane)
            t.move('top-line')
            t.move('cursor-down')
            t.move('start-of-line')
            process = self.open(t, 'treesitter')
            os.write(t.master, b'\r')
            self.finish(process)
            t.move('copy-selection')
            self.assertEqual(t.command('show-buffer'), '{\n\n    let value = 42;\n}')

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
