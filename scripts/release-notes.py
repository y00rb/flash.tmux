#!/usr/bin/env python3
"""Extract one version's Markdown release notes from a changelog."""
import argparse
from pathlib import Path
import re
import sys


def extract_notes(changelog, version):
    lines = changelog.splitlines(keepends=True)
    heading = re.compile(r'^## \[' + re.escape(version) + r'\](?:\s.*)?$')
    for index, line in enumerate(lines):
        if heading.fullmatch(line.rstrip('\r\n')):
            end = index + 1
            while end < len(lines) and not lines[end].startswith('## '):
                end += 1
            notes = ''.join(lines[index + 1:end]).strip()
            if not notes:
                raise ValueError(f'empty changelog section for {version}')
            return notes + '\n'
    raise ValueError(f'missing changelog section for {version}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('tag')
    parser.add_argument('changelog', type=Path)
    args = parser.parse_args()
    print(extract_notes(args.changelog.read_text(encoding='utf-8'),
                        args.tag.removeprefix('v')), end='')


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError) as error:
        print(f'release notes: {error}', file=sys.stderr)
        sys.exit(1)
