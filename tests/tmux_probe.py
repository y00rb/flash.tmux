#!/usr/bin/env python3
"""Contract probes run only against a unique, disposable tmux server."""
import os
import subprocess
import unittest

from tmux_harness import Tmux


class Contracts(unittest.TestCase):
    def test_copy_cursor_coordinates_and_selection(self):
        with Tmux() as t:
            # Literal escapes keep this test source portable while exercising real Unicode.
            rows = ['ASCII target', '\u4e2d\u6587 target', 'e\u0301 target', '\U0001f680 target', 'BOTTOM']
            t.fixture(rows)
            t.command('copy-mode', '-t', t.pane)
            t.move('top-line')
            t.move('start-of-line')
            self.assertEqual(t.cursor(), (0, 0))
            t.move('cursor-down')
            t.move('start-of-line')
            t.move('cursor-right', 3)
            # Two wide glyphs plus a space: cursor motions skip continuation cells.
            self.assertEqual(t.cursor(), (5, 1))
            t.move('begin-selection')
            t.move('cursor-right', 5)
            t.move('copy-selection')
            self.assertEqual(t.command('show-buffer'), 'target')
            t.move('clear-selection')
            t.move('top-line')
            t.move('cursor-down', 2)
            t.move('start-of-line')
            t.move('cursor-right', 2)
            self.assertEqual(t.cursor(), (2, 2))
            t.move('begin-selection')
            t.move('cursor-right', 5)
            t.move('copy-selection')
            self.assertEqual(t.command('show-buffer'), 'target')
            print('coordinate contract: cells differ from motion counts; combining marks consume no motion')

    def test_popup_prompt_without_obscuring_viewport(self):
        with Tmux() as t:
            rows = ['ROW%02d' % i for i in range(23)]
            t.fixture(rows)
            t.command('copy-mode', '-t', t.pane)
            before = t.command('display', '-p', '-t', t.pane, '#{pane_width} #{pane_height}')
            popup = subprocess.Popen(t.base + ['display-popup', '-B', '-E', '-c', t.client,
                                                '-x', '0', '-y', '0', '-w', '80', '-h', '23',
                                                'printf POPUP_SENTINEL; sleep 10'],
                                     stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            try:
                t.wait(lambda: b'POPUP_SENTINEL' in t.output)
                t.output = b''
                t.command('display-message', '-c', t.client, '-d', '0', '\u26a1 QUERY_SENTINEL')
                t.wait(lambda: b'QUERY_SENTINEL' in t.output)
                self.assertEqual(t.command('display', '-p', '-t', t.pane,
                                            '#{pane_width} #{pane_height}'), before)
                self.assertIsNone(popup.poll(), 'prompt must not dismiss the popup')
                print('popup contract: client message prompt visible while pane-sized popup stays open')
            finally:
                t.command('display-popup', '-C', '-c', t.client)
                popup.communicate(timeout=3)


if __name__ == '__main__':
    unittest.main()
