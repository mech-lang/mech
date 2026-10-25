#!/usr/bin/env python3
"""Test the pinned maintained runner without adding archives to Mech dependencies."""
import argparse
from pathlib import Path
import subprocess
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--fetch', action='store_true')
    parser.add_argument('--checkout', type=Path)
    args = parser.parse_args()
    pin = tomllib.loads((ROOT / 'tools/benchmark-runner.toml').read_text())
    with tempfile.TemporaryDirectory(prefix='mech-benchmark-runner-') as temporary:
        checkout = args.checkout or Path(temporary)
        if args.checkout is None:
            if not args.fetch:
                parser.error('provide --checkout or --fetch')
            subprocess.run(['git', 'init', str(checkout)], check=True, capture_output=True)
            subprocess.run(['git', '-C', str(checkout), 'remote', 'add', 'origin', pin['repository']], check=True)
            subprocess.run(['git', '-C', str(checkout), 'sparse-checkout', 'set', '--no-cone', '/tools/canonical-hosted-runner/'], check=True)
            subprocess.run(['git', '-C', str(checkout), 'fetch', '--filter=blob:none', '--depth=1', 'origin', pin['revision']], check=True)
            subprocess.run(['git', '-C', str(checkout), 'checkout', '--detach', pin['revision']], check=True)
        revision = subprocess.check_output(['git', '-C', str(checkout), 'rev-parse', 'HEAD'], text=True).strip()
        if revision != pin['revision']:
            raise SystemExit('benchmark checkout differs from the immutable runner revision')
        dirty = subprocess.check_output(['git', '-C', str(checkout), 'status', '--porcelain', '--', 'tools/canonical-hosted-runner'], text=True)
        if dirty:
            raise SystemExit('maintained runner source is modified')
        manifest = checkout / pin['manifest']
        print(f'Runner {revision}: {manifest}', flush=True)
        if args.fetch:
            subprocess.run(['cargo', 'fetch', '--locked', '--manifest-path', str(manifest)], check=True)
        subprocess.run(['cargo', 'test', '--locked', '--offline', '--manifest-path', str(manifest)], check=True)


if __name__ == '__main__':
    main()
