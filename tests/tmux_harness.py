"""Disposable real-client tmux harness. Never uses the caller's tmux socket."""
import fcntl
import os
import pty
import select
import shlex
import signal
import struct
import subprocess
import termios
import tempfile
import time
import uuid


class Tmux:
    def __init__(self, width=80, height=24):
        self.socket = 'flash-test-' + uuid.uuid4().hex
        self.base = ['tmux', '-L', self.socket]
        self.width, self.height = width, height
        self.client_process = None
        self.master = None
        self.output = b''
        self.cache = tempfile.TemporaryDirectory(prefix='flash-test-cache-')

    def command(self, *args):
        return subprocess.check_output(self.base + list(args), stderr=subprocess.PIPE).decode()

    def __enter__(self):
        env = dict(os.environ, TERM='xterm-256color', LC_ALL='en_US.UTF-8', XDG_CACHE_HOME=self.cache.name)
        env.pop('TMUX', None)
        subprocess.run(self.base + ['-f', '/dev/null', 'new-session', '-d', '-s', 'probe',
                                   '-x', str(self.width), '-y', str(self.height), 'sleep 600'],
                       env=env, check=True, capture_output=True)
        self.command('set', '-g', 'mode-keys', 'vi')
        self.command('set', '-g', 'status-position', 'bottom')
        self.command('set', '-g', 'status-left', '')
        self.command('set', '-g', 'status-right', '')
        self.command('set', '-g', 'status-format[0]', 'STATUS_SENTINEL')
        self.master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', self.height, self.width, 0, 0))
        self.client_process = subprocess.Popen(self.base + ['attach', '-t', 'probe'],
                                               stdin=slave, stdout=slave, stderr=slave,
                                               env=env, start_new_session=True)
        os.close(slave)
        self.wait(lambda: bool(self.command('list-clients', '-F', '#{client_name}').strip()))
        self.client = self.command('list-clients', '-F', '#{client_name}').strip()
        self.pane = self.command('display', '-p', '#{pane_id}').strip()
        self.drain()
        return self

    def wait(self, condition, timeout=3):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            self.drain()
            if condition():
                return
            time.sleep(0.02)
        raise AssertionError('condition timed out')

    def drain(self, timeout=0.03):
        if self.master is None:
            return b''
        chunks = []
        while select.select([self.master], [], [], timeout)[0]:
            try:
                data = os.read(self.master, 65536)
            except OSError:
                break
            if not data:
                break
            chunks.append(data)
            timeout = 0
        data = b''.join(chunks)
        self.output += data
        return data

    def fixture(self, rows):
        program = "import sys,time;sys.stdout.write('\\x1b[2J\\x1b[H'+" + repr('\r\n'.join(rows)) + ");sys.stdout.flush();time.sleep(600)"
        self.command('respawn-pane', '-k', '-t', self.pane, 'python3 -c ' + shlex.quote(program))
        self.wait(lambda: rows[-1] in self.command('capture-pane', '-p', '-t', self.pane))

    def move(self, action, count=None):
        args = ['send-keys', '-t', self.pane, '-X']
        if count is not None:
            args += ['-N', str(count)]
        self.command(*args, action)

    def resize(self, width, height):
        fcntl.ioctl(self.master, termios.TIOCSWINSZ,
                    struct.pack('HHHH', height, width, 0, 0))
        os.kill(self.client_process.pid, signal.SIGWINCH)
        # SIGWINCH delivery is asynchronous; wait for tmux to observe it.
        self.wait(lambda: self.command('display', '-p', '-c', self.client,
                                       '#{client_width};#{client_height}').strip()
                  == f'{width};{height}')

    def cursor(self):
        return tuple(map(int, self.command('display', '-p', '-t', self.pane,
                                          '#{copy_cursor_x} #{copy_cursor_y}').split()))

    def __exit__(self, *exc):
        subprocess.run(self.base + ['kill-server'], capture_output=True)
        if self.client_process:
            deadline = time.monotonic() + 3
            while self.client_process.poll() is None and time.monotonic() < deadline:
                self.drain()
                time.sleep(0.01)
            if self.client_process.poll() is None:
                self.client_process.terminate()
            self.client_process.wait(timeout=3)
        if self.master is not None:
            os.close(self.master)
        self.cache.cleanup()
