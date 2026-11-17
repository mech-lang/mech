"""Prepare and compile the isolated b72b7419 spill experiment."""
from hashlib import file_digest
import json
import os
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
REPO = Path('/private/tmp/mech-shared-cold-epilogues.2L6obP')
QUAL = REPO / '.qualification'
PUBLIC = QUAL / 'source-frame-copy-tails'
BASE = 'b72b7419b39b22765fe1eed6b5c636e63964b905'
PATCH = HERE / 'precise-spill-affinity.patch'
PROFILE = Path('benchmarks/mech-deflate/native-profile')
PACKET = PROFILE / 'native-compact-20261116'
PUBLISHED = PUBLIC / 'verified/shared-copies'
ENV = {k: v for k, v in os.environ.items() if not k.startswith(('MECH_', 'CARGO_PROFILE_'))}


def sha(path):
    with Path(path).open('rb') as stream:
        return file_digest(stream, 'sha256').hexdigest()


def read(path):
    return json.loads(path.read_text())


def write(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def run(command, log, *, cwd, env=ENV):
    command = list(map(str, command))
    with log.open('wb') as stream:
        subprocess.run(command, cwd=cwd, env=env, stdout=stream, stderr=subprocess.STDOUT, check=True)
    return command


def construct(out):
    checkout = out / 'checkout'
    out.mkdir()
    checkout.mkdir()
    entries = ('Cargo.lock', 'Cargo.toml', 'build.rs', 'README.md', 'LICENSE', '.cargo', 'include',
               'docs', 'examples', 'hosts', 'machines', 'src', 'vendor', 'tests')
    bench_files = subprocess.check_output(
        ['git', 'ls-tree', '--name-only', BASE, f'{PROFILE}/'], cwd=REPO, text=True,
    ).splitlines()
    sources = [path for path in bench_files if Path(path).suffix in {'.rs', '.toml', '.lock'}]
    sources.extend((str(PROFILE / 'compact-size'), str(PACKET / 'frontend.rs'), str(PACKET / 'program.mec'),
                    str(PACKET / 'recipe.json'), str(PACKET / 'profiles'), str(PACKET / 'fixtures.json')))
    with subprocess.Popen(['git', 'archive', '--format=tar', BASE, *entries, *sources],
                          cwd=REPO, stdout=subprocess.PIPE) as archive:
        subprocess.run(['/usr/bin/tar', '-xf', '-', '-C', checkout], stdin=archive.stdout, check=True)
        assert archive.wait() == 0
    subprocess.run(['git', 'apply', '--check', '--verbose', PATCH], cwd=checkout, check=True)
    subprocess.run(['git', 'apply', PATCH], cwd=checkout, check=True)
    target = out / 'cargo-target'
    subprocess.run(['/bin/cp', '-cR', QUAL / 'host-opt-levels/opt3-cargo', target], check=True)
    profile = {
        'CARGO_BUILD_JOBS': '1',
        'CARGO_PROFILE_RELEASE_OPT_LEVEL': '3',
        'CARGO_PROFILE_RELEASE_DEBUG': 'false',
        'CARGO_PROFILE_RELEASE_LTO': 'false',
        'CARGO_PROFILE_RELEASE_CODEGEN_UNITS': '16',
    }
    command = [
        'cargo', '+nightly-2026-03-03', 'build', '--offline', '--locked',
        '--release', '--config', 'profile.release.package.libmimalloc-sys.opt-level=1',
        '--manifest-path', checkout / PROFILE / 'Cargo.toml',
        '--target-dir', target, '--message-format=json', '--bin', 'compile_profile',
    ]
    record = {'base': BASE, 'patch_sha256': sha(PATCH), 'profile': profile, 'cargo': list(map(str, command))}
    write(out / 'build.json', record)
    run(command, out / 'cargo.log', cwd=checkout, env=ENV | profile)
    compiler = target / 'release/compile_profile'
    record['compiler'] = str(compiler)
    record['compiler_sha256'] = sha(compiler)
    write(out / 'build.json', record)


if __name__ == '__main__':
    construct(HERE / 'trial')
