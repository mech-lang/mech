"""Install one pinned private V release and smoke-test its production backend.

This is unscored preparation, serialized with every benchmark. It never changes
the explorer, installed compiler defaults, or any existing benchmark artifact.
"""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import urllib.request
import zipfile
from datetime import datetime, timezone

HERE = Path(__file__).resolve().parent
LOCK = Path('/tmp/mech-clbg-timing.lock')
OUT = HERE / '.build'
VERSION = '0.5.2'
SOURCE_COMMIT = '7647ce1c6fad63b5578bc07883139906de74b2f8'
CLANG = Path('/opt/homebrew/opt/llvm@22/bin/clang-22')


def now():
    return datetime.now(timezone.utc).isoformat()


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def save_new(path, value):
    with Path(path).open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def run():
    print('WAIT common timing mutex for private V preparation', flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        assert not (HERE / 'bootstrap.json').exists(), 'Existing preparation must be inspected, never overwritten'
        OUT.mkdir(exist_ok=True)
        release_path = HERE / 'upstream/v-release.json'
        ref_path = HERE / 'upstream/v-ref.json'
        release = json.loads(release_path.read_text())
        ref = json.loads(ref_path.read_text())
        assert release['tag_name'] == VERSION and ref['object']['sha'] == SOURCE_COMMIT
        asset = next(row for row in release['assets'] if row['name'] == 'v_macos_arm64.zip')
        assert subprocess.check_output(['uname', '-m'], text=True).strip() == 'arm64'
        archive = OUT / asset['name']
        assert not archive.exists(), 'Archive already exists; inspect partial or completed preparation'
        request = urllib.request.Request(asset['browser_download_url'], headers={'User-Agent': 'mech-benchmark-source-audit'})
        with urllib.request.urlopen(request, timeout=120) as response, archive.open('xb') as stream:
            while chunk := response.read(1024 * 1024):
                stream.write(chunk)
        archive_sha = sha(archive)
        if asset.get('digest'):
            assert asset['digest'] == 'sha256:' + archive_sha, 'Official asset digest differs'
        checkout = OUT / 'toolchain'
        checkout.mkdir()
        with zipfile.ZipFile(archive) as bundle:
            for member in bundle.infolist():
                destination = (checkout / member.filename).resolve()
                assert destination.is_relative_to(checkout.resolve()), 'Archive path escapes private toolchain'
                bundle.extract(member, checkout)
                mode = member.external_attr >> 16
                if mode and destination.is_file():
                    destination.chmod(mode & 0o777)
        candidates = [path for path in checkout.rglob('v') if path.is_file()]
        assert len(candidates) == 1, ('Ambiguous release compiler', candidates)
        compiler = candidates[0]
        compiler.chmod(compiler.stat().st_mode | 0o111)
        environment = dict(os.environ, VMODULES=str(OUT / 'vmodules'), VTMP=str(OUT / 'tmp'),
                           VFLAGS='', LC_ALL='C', RAYON_NUM_THREADS='8', OMP_NUM_THREADS='8',
                           OPENBLAS_NUM_THREADS='1', VECLIB_MAXIMUM_THREADS='1')
        for name in ('vmodules', 'tmp'):
            (OUT / name).mkdir()
        version = subprocess.check_output([str(compiler), '-version'], env=environment, text=True).strip()
        assert version.startswith('V ' + VERSION), ('Release compiler version differs', version)
        clang_version = subprocess.check_output([str(CLANG), '--version'], text=True).splitlines()[0]
        help_result = subprocess.run([str(compiler), 'help', 'build'], env=environment, text=True,
                                     capture_output=True, check=True)
        (OUT / 'build-help.txt').write_text(help_result.stdout + help_result.stderr)
        source, binary = OUT / 'smoke.v', OUT / 'smoke'
        source.write_text("fn main() {\n    println('V bootstrap OK')\n}\n")
        command = [str(compiler), '-prod', '-cc', str(CLANG), '-o', str(binary), str(source)]
        build = subprocess.run(command, env=environment, capture_output=True, text=True)
        (OUT / 'smoke-build.log').write_text(build.stdout + build.stderr)
        assert build.returncode == 0, ('Production smoke build failed', build.returncode)
        smoke = subprocess.run([str(binary)], env=environment, capture_output=True, check=True)
        assert smoke.stdout == b'V bootstrap OK\n' and smoke.stderr == b''
        pins = {str(path): sha(path) for path in checkout.rglob('*') if path.is_file()}
        pins.update({str(path): sha(path) for path in (archive, release_path, ref_path, CLANG, source, binary, Path(__file__))})
        receipt = dict(status='prepared-smoke-passed', finished_at=now(), never_chart_eligible=True,
            report_only=True, source_commit=SOURCE_COMMIT, release=VERSION,
            compiler=dict(path=str(compiler), sha256=sha(compiler), version=version),
            clang=dict(path=str(CLANG), sha256=sha(CLANG), version=clang_version),
            archive=dict(path=str(archive), sha256=archive_sha, url=asset['browser_download_url'],
                         advertised_digest=asset.get('digest')),
            runtime_environment={key: environment[key] for key in ('VMODULES', 'VTMP', 'VFLAGS', 'LC_ALL')},
            production_smoke_command=command, smoke_stdout=smoke.stdout.decode(),
            shared_timing_mutex=str(LOCK), toolchain_checkout=str(checkout), pins=pins,
            scope='Compiler installation and one production hello-world only; no benchmark measurements, global runtime installation, or visualization changes.')
        save_new(HERE / 'bootstrap.json', receipt)
        print(json.dumps({key: receipt[key] for key in ('status', 'compiler', 'toolchain_checkout')}, indent=2), flush=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--detach', action='store_true')
    args = parser.parse_args()
    if not args.detach:
        run()
        return
    launch = HERE / 'bootstrap-launch.json'
    assert not launch.exists() and not (HERE / 'bootstrap.json').exists(), 'Do not duplicate private preparation'
    with (HERE / 'bootstrap.log').open('ab') as log:
        process = subprocess.Popen([sys.executable, '-u', str(Path(__file__).resolve())],
            stdin=subprocess.DEVNULL, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
    save_new(launch, dict(pid=process.pid, launched_at=now(), command=[sys.executable, '-u', str(Path(__file__).resolve())],
                         log=str(HERE / 'bootstrap.log'), report_only=True))
    print(json.dumps(json.loads(launch.read_text())), flush=True)


if __name__ == '__main__':
    try:
        main()
    except BaseException as error:
        save_new(HERE / ('bootstrap-error-' + str(os.getpid()) + '.json'),
                 dict(finished_at=now(), error=str(error), error_type=type(error).__name__, no_automatic_retry=True))
        raise
