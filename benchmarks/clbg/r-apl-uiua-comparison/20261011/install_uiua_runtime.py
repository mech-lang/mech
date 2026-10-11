#!/usr/bin/env python3
"""Reconstruct ignored official Uiua runtime and its generic GMP FFI support.

The interpreter is downloaded, never compiled locally. Only the small generic
GMP adapter is compiled. This script does not run a benchmark or timing job.
"""
from pathlib import Path
import argparse
import hashlib
import json
import platform
import re
import shutil
import subprocess
import tarfile
import urllib.request
import zipfile

G = Path(__file__).resolve().parent
H = G / '.build/toolchains/uiua-0.19.1'
PINS = json.loads((G / 'uiua-install-pins.json').read_text())


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def download(name, destination):
    pin = PINS[name]
    if not destination.exists() or sha(destination) != pin['sha256']:
        with urllib.request.urlopen(pin['url']) as source, destination.open('wb') as output:
            shutil.copyfileobj(source, output)
    if sha(destination) != pin['sha256']:
        raise RuntimeError(f'SHA256 mismatch for {destination}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--skip-support-build', action='store_true')
    options = parser.parse_args()
    if platform.system() != 'Darwin' or platform.machine() != 'arm64':
        raise RuntimeError('This pinned official distribution requires ARM64 macOS.')
    H.mkdir(parents=True, exist_ok=True)
    download('runtime', H / 'runtime.zip')
    with zipfile.ZipFile(H / 'runtime.zip') as archive:
        (H / 'uiua').write_bytes(archive.read('uiua'))
    (H / 'uiua').chmod(0o755)
    if sha(H / 'uiua') != PINS['runtime']['executable_sha256']:
        raise RuntimeError('Official Uiua executable hash differs from the pin.')
    download('runtime_source', H / 'source.tar.gz')
    download('ecow_source', H / 'ecow-0.2.4.crate')
    for path in [H / 'source.tar.gz', H / 'ecow-0.2.4.crate']:
        with tarfile.open(path) as archive:
            archive.extractall(H, filter='data')
    # The immutable snapshot avoids mutable GitHub download counters affecting
    # recorded metadata hashes during reconstruction.
    shutil.copyfile(G / 'support/uiua/release-0.19.1.json', H / 'release.json')
    if not options.skip_support_build:
        gmp = Path(PINS['gmp']['installed_library'])
        if not gmp.exists() or sha(gmp) != PINS['gmp']['installed_library_sha256']:
            raise RuntimeError('Install the recorded Homebrew GMP 6.3.0 library before building support; its installed dylib must match uiua-install-pins.json.')
        output = G / '.build/uiua-support/libgmp-uiua.dylib'
        output.parent.mkdir(parents=True, exist_ok=True)
        subprocess.run(['cc', '-O3', '-fPIC', '-dynamiclib',
                        str(G / 'support/uiua/gmp.c'),
                        '-I/opt/homebrew/opt/gmp/include',
                        '-L/opt/homebrew/opt/gmp/lib', '-lgmp',
                        '-o', str(output)], check=True)
        # Only relocate the generic library path when rebuilding elsewhere.
        for name in ['compact', 'fast']:
            source = G / 'sources/uiua/pidigits' / (name + '.ua')
            text = source.read_text()
            relocated = re.sub(r'(?m)^(Lib\s*←\s*&ffi\s*⊂□)"[^"]*"',
                               lambda match: match.group(1) + json.dumps(str(output)), text)
            if relocated != text:
                source.write_text(relocated)
                print(f'Relocated library path in {source}; repeat correctness validation before repinning.')
    print('Official Uiua installed. Revalidate the suite and run prepare_uiua_manifest.py to pin the rebuilt support binary.')


if __name__ == '__main__':
    main()
