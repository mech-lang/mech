"""Fresh private V setup using the installed SDK and owned native processes.

The original failed setup and its toolchain remain immutable evidence. This
revision re-extracts the verified official archive and never changes the global
compiler or SDK. All native preparation shares the benchmarking mutex.
"""
import argparse
import fcntl
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import zipfile

import bootstrap
import full_study
from prepare import command_build, merge_pins

HERE = Path(__file__).resolve().parent
OUT = HERE / '.build-sdk-v1'
PREREQUISITES = {'oracle-fixtures.json': 14, 'preparation-fixtures.json': 5,
                 'cleanup-fixtures.json': 8}


def prerequisites():
    receipts = {}
    for filename, tests in PREREQUISITES.items():
        path = HERE / filename
        if not path.exists():
            return None
        receipt = json.loads(path.read_text())
        assert receipt['status'] == 'passed' and receipt['tests_run'] == tests, (
            'V prerequisite failed; inspect retained evidence', path)
        receipts[filename] = receipt
    return receipts


def run():
    module = full_study.configure('nbody')
    print('WAIT V oracle, override and cleanup prerequisite receipts', flush=True)
    while prerequisites() is None:
        time.sleep(2)
    print('WAIT common mutex: private V installed-SDK revision', flush=True)
    with module.LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        assert not OUT.exists() and not (HERE / 'bootstrap.json').exists(), 'No duplicate or implicit recovery'
        receipts = prerequisites()
        harness = module.load_harness()
        harness.install_interrupt_handlers()
        pins = merge_pins(*(receipt['pins'] for receipt in receipts.values()))
        harness.PinGuard(pins).verify()
        OUT.mkdir()
        report = dict(status='preparing', created_at=module.now(), revision='installed-sdk-v1',
                      report_only=True, never_chart_eligible=True, full_performance_runs=0,
                      prior_failure=str(HERE / 'bootstrap-error-41555.json'))
        try:
            release_path, ref_path = HERE / 'upstream/v-release.json', HERE / 'upstream/v-ref.json'
            release, ref = json.loads(release_path.read_text()), json.loads(ref_path.read_text())
            assert release['tag_name'] == bootstrap.VERSION and ref['object']['sha'] == bootstrap.SOURCE_COMMIT
            asset = next(row for row in release['assets'] if row['name'] == 'v_macos_arm64.zip')
            archive = HERE / '.build' / asset['name']
            archive_sha = module.sha(archive)
            assert asset['digest'] == 'sha256:' + archive_sha, 'Official archive hash differs'

            environment = dict(os.environ, LC_ALL='C', VFLAGS='',
                               VMODULES=str(OUT / 'vmodules'), VTMP=str(OUT / 'tmp'),
                               RAYON_NUM_THREADS='8', OMP_NUM_THREADS='8',
                               OPENBLAS_NUM_THREADS='1', VECLIB_MAXIMUM_THREADS='1')

            def execute(name, command):
                directory = OUT / 'commands' / name
                directory.mkdir(parents=True)
                command_build(module, harness, command, environment, directory, lock.fileno())
                return (directory / 'build.stdout').read_bytes(), (directory / 'build.stderr').read_bytes()

            architecture, _ = execute('architecture', ['/usr/bin/uname', '-m'])
            assert architecture.strip() == b'arm64'
            sdk_bytes, _ = execute('sdk', ['/usr/bin/xcrun', '--show-sdk-path'])
            sdk = Path(sdk_bytes.decode().strip()).resolve(strict=True)
            assert sdk.is_relative_to(Path('/Library/Developer/CommandLineTools/SDKs'))
            assert (sdk / 'usr/include/pthread.h').is_file()
            # V applies CFLAGS to its bundled third-party objects as well as
            # the application (vlib/v/builder/cc.v thirdparty_object_args).
            environment.update(SDKROOT=str(sdk), CFLAGS='-isysroot ' + str(sdk))
            for name in ('vmodules', 'tmp'):
                (OUT / name).mkdir()
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
            assert len(candidates) == 1, ('Ambiguous V release compiler', candidates)
            compiler = candidates[0]
            compiler.chmod(compiler.stat().st_mode | 0o111)
            version_bytes, _ = execute('version', [str(compiler), '-version'])
            version = version_bytes.decode().strip()
            assert version.startswith('V ' + bootstrap.VERSION)
            clang_bytes, _ = execute('clang-version', [str(bootstrap.CLANG), '--version'])
            execute('help', [str(compiler), 'help', 'build'])
            source, binary = OUT / 'smoke.v', OUT / 'smoke'
            source.write_text("fn main() {\n    println('V bootstrap OK')\n}\n")
            command = [str(compiler), '-prod', '-cc', str(bootstrap.CLANG),
                       '-cflags', '-O3 -march=native -ffp-contract=off',
                       '-o', str(binary), str(source)]
            execute('production-smoke-build', command)
            stdout, stderr = execute('production-smoke-execution', [str(binary)])
            assert stdout == b'V bootstrap OK\n' and stderr == b''
            files = [path for path in checkout.rglob('*') if path.is_file()]
            files += [archive, release_path, ref_path, bootstrap.CLANG, source, binary,
                      Path(__file__).resolve(), HERE / 'bootstrap.py', HERE / 'prepare.py',
                      sdk / 'SDKSettings.json', sdk / 'usr/include/pthread.h']
            files += [HERE / filename for filename in PREREQUISITES]
            pins = merge_pins(pins, {str(path): module.sha(path) for path in files})
            harness.PinGuard(pins).verify()
            receipt = dict(report, status='prepared-smoke-passed', finished_at=module.now(),
                source_commit=bootstrap.SOURCE_COMMIT, release=bootstrap.VERSION,
                compiler=dict(path=str(compiler), sha256=module.sha(compiler), version=version),
                clang=dict(path=str(bootstrap.CLANG), sha256=module.sha(bootstrap.CLANG),
                           version=clang_bytes.decode().splitlines()[0]),
                sdk=dict(path=str(sdk), selected_by='xcrun --show-sdk-path; resolved existing SDK'),
                archive=dict(path=str(archive), sha256=archive_sha, url=asset['browser_download_url'],
                             advertised_digest=asset['digest']),
                runtime_environment={key: environment[key] for key in ('VMODULES', 'VTMP', 'VFLAGS', 'LC_ALL', 'SDKROOT', 'CFLAGS')},
                production_smoke_command=command, smoke_stdout=stdout.decode(),
                shared_timing_mutex=str(module.LOCK), toolchain_checkout=str(checkout),
                pins=pins, pins_unchanged_at_end=True)
            bootstrap.save_new(HERE / 'bootstrap.json', receipt)
            report = receipt
            print('PASS private V installed-SDK production smoke:', version, flush=True)
        except BaseException as error:
            report.update(status='stopped', error=str(error), error_type=type(error).__name__,
                          finished_at=module.now(), no_automatic_retry=True)
            raise
        finally:
            module.save(OUT / 'status.json', report)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--detach', action='store_true')
    args = parser.parse_args()
    if not args.detach:
        run()
        return
    module = full_study.configure('nbody')
    launch = HERE / 'bootstrap-sdk-launch.json'
    with (HERE / 'bootstrap-sdk-launch-owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        assert not launch.exists() and not OUT.exists() and not (HERE / 'bootstrap.json').exists()
        command = [sys.executable, '-u', str(Path(__file__).resolve())]
        with (HERE / 'bootstrap-sdk.log').open('xb') as log:
            process = subprocess.Popen(command, stdin=subprocess.DEVNULL, stdout=log,
                                       stderr=subprocess.STDOUT, start_new_session=True)
        bootstrap.save_new(launch, dict(pid=process.pid, identity=module.identity(process.pid),
                                       command=command, launched_at=module.now(),
                                       report_only=True, never_chart_eligible=True))
        print(json.dumps(json.loads(launch.read_text()), indent=2), flush=True)


if __name__ == '__main__':
    try:
        main()
    except BaseException as error:
        bootstrap.save_new(HERE / ('bootstrap-sdk-error-' + str(os.getpid()) + '.json'),
                           dict(status='stopped', finished_at=bootstrap.now(),
                                error=str(error), error_type=type(error).__name__,
                                report_only=True, never_chart_eligible=True,
                                full_performance_runs=0, no_automatic_retry=True))
        raise
