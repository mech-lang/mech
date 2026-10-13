"""Build the unscored missing-native canary; runtime AOT remains unproved."""
from datetime import datetime, timezone
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import traceback

HERE = Path(__file__).resolve().parent
CLBG = HERE.parents[1]
LOCK = Path('/tmp/mech-clbg-timing.lock')
BOOTSTRAP = HERE / 'jobs/packagecompiler-bootstrap-v2/report.json'
PREPARATION = CLBG / 'c3-comparison/20261012/revisions/private-v3/guarded_prepare.py'
OUTPUT = HERE / 'jobs/canary-library-v1'
BUILD = HERE / '.build/canary-library-v1'


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def save(path, value):
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def main():
    print('WAIT common mutex: Julia missing-native canary library build', flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        opened, named = os.fstat(lock.fileno()), LOCK.stat()
        assert (opened.st_dev, opened.st_ino) == (named.st_dev, named.st_ino)
        assert not OUTPUT.exists() and not BUILD.exists()
        OUTPUT.mkdir(parents=True)
        pins = {str(Path(__file__).resolve()): sha(__file__)}
        receipt = dict(status='failed', exclusive_common_timing_mutex_held=True,
            common_timing_mutex=str(LOCK), actual_benchmark_runs=0,
            actual_canary_executions=0, runtime_aot_proven=False,
            chart_admission=False, automatic_retry=False,
            started_at=datetime.now(timezone.utc).isoformat())
        try:
            bootstrap = json.loads(BOOTSTRAP.read_text())
            assert bootstrap['status'] == 'exact-packagecompiler-bootstrap-complete-awaiting-library-pilot'
            assert bootstrap['pins_unchanged_at_end'] is True
            assert bootstrap['packagecompiler']['version'] == '2.4.3'
            pins.update(bootstrap['pins'])
            pins[str(BOOTSTRAP)] = sha(BOOTSTRAP)
            for path, digest in pins.items():
                assert sha(path) == digest, ('Qualified build toolchain changed', path)
            assert str(PREPARATION) in pins
            spec = importlib.util.spec_from_file_location('julia_canary_owned_commands', PREPARATION)
            preparation = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(preparation)
            guard = preparation.load_guard()
            harness = guard.load_harness()
            environment = dict(bootstrap['runtime_environment'])
            runtime = Path(bootstrap['runtime_restoration']['runtime'])
            julia = (runtime / 'bin/julia').resolve(strict=True)
            project = BUILD / 'ClbgAotNegativeCanary'
            (project / 'src').mkdir(parents=True)
            # Keep the completed bootstrap caches immutable. New canary/package
            # compilation writes only into this distinct build's copied depot.
            depot = BUILD / 'depot'
            shutil.copytree(bootstrap['depot'], depot, symlinks=True)
            environment['JULIA_DEPOT_PATH'] = str(depot)
            (project / 'Project.toml').write_text('name = "ClbgAotNegativeCanary"\nuuid = "aa5219af-133a-4b6c-acce-7674dcf83c92"\nversion = "0.1.0"\n')
            candidate = HERE / 'revisions/proposal-v1/negative_canary.jl'
            entry = project / 'src/ClbgAotNegativeCanary.jl'
            shutil.copyfile(candidate, entry)
            pins[str(candidate)] = pins[str(entry)] = sha(candidate)
            driver_source = HERE / 'revisions/proposal-v1/direct_driver.c'
            driver = BUILD / 'direct_driver.c'
            shutil.copyfile(driver_source, driver)
            pins[str(driver_source)] = pins[str(driver)] = sha(driver_source)
            precompile = BUILD / 'precompile.jl'
            # Compile the exported entry without executing the intentionally
            # uncovered method. Its actual missing coverage still needs review.
            precompile.write_text('using ClbgAotNegativeCanary\nprecompile(ClbgAotNegativeCanary.clbg_entry, (Cint, Ptr{Ptr{Cchar}}))\n')
            build_script = BUILD / 'build.jl'
            build_script.write_text('using PackageCompiler\nPackageCompiler.create_library(ARGS[1], ARGS[2]; lib_name="clbgaotcanary", incremental=false, filter_stdlibs=false, precompile_execution_file=ARGS[3], sysimage_build_args=`-O3`, force=false)\n')
            for path in (project / 'Project.toml', precompile, build_script):
                pins[str(path)] = sha(path)
            bundle = BUILD / 'bundle'
            preparation.owned_command(guard, harness,
                [str(julia), '--startup-file=no', '--history-file=no', '--threads=8,0',
                 '--project=' + bootstrap['project'], str(build_script), str(project), str(bundle), str(precompile)],
                environment, OUTPUT / 'build', lock.fileno(), 1800, 'create-library')
            qualification = json.loads((CLBG / 'c3-comparison/20261012/qualification/private-v3/report.json').read_text())
            clang = preparation.CLANG
            assert sha(clang) == qualification['pins'][str(clang)]
            pins[str(clang)] = sha(clang)
            sdk = Path(qualification['sdk'])
            for path, digest in qualification['pins'].items():
                if Path(path).is_relative_to(sdk):
                    assert sha(path) == digest
                    pins[path] = digest
            executable = BUILD / 'canary'
            preparation.owned_command(guard, harness,
                [str(clang), '-O3', '-arch', 'arm64', '-isysroot', qualification['sdk'],
                 str(driver), '-L' + str(bundle / 'lib'), '-lclbgaotcanary',
                 '-Wl,-rpath,' + str(bundle / 'lib'), '-o', str(executable)],
                environment, OUTPUT / 'link', lock.fileno(), 120, 'direct-c-launcher')
            pins.update(preparation.pin_tree(BUILD))
            for path, digest in pins.items():
                assert sha(path) == digest, ('Library build input changed', path)
            receipt.update(status='canary-library-built-awaiting-native-coverage-and-observer-calibration',
                library_bundle=str(bundle), direct_executable=str(executable),
                packagecompiler_version='2.4.3', julia_version=bootstrap['julia_version'],
                pins_unchanged_at_end=True, bootstrap=dict(path=str(BOOTSTRAP), sha256=pins[str(BOOTSTRAP)]))
        except BaseException as error:
            receipt.update(error=str(error) or type(error).__name__, error_type=type(error).__name__)
            (OUTPUT / 'failure.log').write_text(traceback.format_exc())
            raise
        finally:
            receipt.update(pins=pins, completed_at=datetime.now(timezone.utc).isoformat())
            save(OUTPUT / 'report.json', receipt)
            save(OUTPUT / 'terminal-pins.json', dict(terminal_status=receipt['status'],
                pins={str(path): sha(path) for path in OUTPUT.rglob('*') if path.is_file()},
                runtime_aot_proven=False, chart_admission=False, automatic_retry=False))
        print(json.dumps(dict(status=receipt['status'], report=str(OUTPUT / 'report.json'))), flush=True)


if __name__ == '__main__':
    main()
