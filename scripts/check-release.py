#!/usr/bin/env python3
"""Validate local and tagged release versions without network access."""
import argparse
from pathlib import Path
import re
import subprocess
import sys
import tomllib


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument('--tag')
    parser.add_argument('--binary', type=Path)
    args = parser.parse_args()
    version = (args.root / 'VERSION').read_text().strip()
    if not re.fullmatch(r'(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-[0-9A-Za-z]+(?:[.-][0-9A-Za-z]+)*)?', version):
        raise ValueError('VERSION must contain a supported semantic version')
    cargo = tomllib.loads((args.root / 'Cargo.toml').read_text())['package']['version']
    if version != cargo:
        raise ValueError('VERSION and Cargo package version differ')
    if args.tag is not None and args.tag != 'v' + version:
        raise ValueError('tag and package version differ')
    if args.binary is not None:
        output = subprocess.check_output([str(args.binary.resolve()), '--version'], text=True).strip()
        if output != 'flash-tmux ' + version:
            raise ValueError('binary version differs from package version')
    print('Release version verified: ' + version)


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError) as error:
        print('release check: ' + str(error), file=sys.stderr)
        sys.exit(1)
