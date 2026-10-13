"""Install an isolated, exact PackageCompiler toolchain; no AOT admission."""
from datetime import datetime, timezone
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import tomllib
import traceback

HERE = Path(__file__).resolve().parent
CLBG = HERE.parents[1]
LOCK = Path('/tmp/mech-clbg-timing.lock')
JULIA = Path('/tmp/mech-clbg-julia/julia-1.13.1/bin/julia')
PREPARATION = CLBG / 'c3-comparison/20261012/revisions/private-v3/guarded_prepare.py'
OUTPUT = HERE / 'jobs/packagecompiler-bootstrap-v1'
BUILD = HERE / '.build/packagecompiler-bootstrap-v1'


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def save(path, value):
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def main():
    print('WAIT common mutex: isolated Julia PackageCompiler bootstrap', flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        opened, named = os.fstat(lock.fileno()), LOCK.stat()
        assert (opened.st_dev, opened.st_ino) == (named.st_dev, named.st_ino)
        assert not OUTPUT.exists() and not BUILD.exists(), 'No duplicate bootstrap or partial reuse'
        OUTPUT.mkdir(parents=True)
        pins = {str(Path(__file__).resolve()): sha(__file__)}
        receipt = dict(status='failed', exclusive_common_timing_mutex_held=True,
            common_timing_mutex=str(LOCK), actual_benchmark_runs=0, library_builds=0,
            runtime_aot_proven=False, chart_admission=False, automatic_retry=False,
            started_at=datetime.now(timezone.utc).isoformat())
        try:
            qualified = json.loads((CLBG / 'c3-comparison/20261012/qualification/private-v3/report.json').read_text())
            assert sha(PREPARATION) == qualified['pins'][str(PREPARATION)]
            pins[str(PREPARATION)] = sha(PREPARATION)
            spec = importlib.util.spec_from_file_location('julia_bootstrap_owned_commands', PREPARATION)
            preparation = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(preparation)
            guard = preparation.load_guard()
            harness = guard.load_harness()
            for module in (guard, harness):
                path = Path(module.__file__).resolve()
                pins[str(path)] = sha(path)
            runtime = JULIA.resolve(strict=True).parents[1]
            # Bind the existing runtime to its previously inspected actual bytes.
            symbols_path = HERE / 'jobs/runtime-symbols-v1/report.json'
            symbols = json.loads(symbols_path.read_text())
            for path, digest in symbols['pins'].items():
                if Path(path).is_relative_to(runtime):
                    assert sha(path) == digest
                    pins[path] = digest
            pins[str(symbols_path)] = sha(symbols_path)
            julia = JULIA.resolve(strict=True)
            pins[str(julia)] = sha(julia)
            project, depot = BUILD / 'project', BUILD / 'depot'
            project.mkdir(parents=True)
            depot.mkdir()
            environment = dict(PATH=str(julia.parent) + ':/usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin',
                HOME=os.environ['HOME'], TMPDIR=os.environ.get('TMPDIR', '/tmp/'),
                LANG='C', LC_ALL='C', JULIA_DEPOT_PATH=str(depot),
                JULIA_LOAD_PATH='@:@stdlib', JULIA_PKG_PRECOMPILE_AUTO='0',
                JULIA_NUM_PRECOMPILE_TASKS='1')
            version = preparation.owned_command(guard, harness, [str(julia), '--version'],
                environment, OUTPUT / 'version', lock.fileno(), 30, 'julia-version')
            assert version['stdout'].decode().strip() == 'julia version 1.13.1'
            installer = BUILD / 'install.jl'
            installer.write_text('using Pkg\nPkg.activate(ARGS[1])\nPkg.add(PackageSpec(name="PackageCompiler", version="2.4.3"))\nPkg.precompile()\n')
            pins[str(installer)] = sha(installer)
            preparation.owned_command(guard, harness,
                [str(julia), '--startup-file=no', '--history-file=no', '--threads=1',
                 str(installer), str(project)], environment, OUTPUT / 'installation',
                lock.fileno(), 1800, 'packagecompiler-install')
            manifest = tomllib.loads((project / 'Manifest.toml').read_text())
            package = manifest['deps']['PackageCompiler']
            assert len(package) == 1 and package[0]['version'] == '2.4.3'
            assert package[0]['git-tree-sha1']
            # Package code, manifest, archives/artifacts and generated cache are
            # evidence for later builds, never source LOC or runtime samples.
            pins.update(preparation.pin_tree(BUILD))
            for path, digest in pins.items():
                assert sha(path) == digest, ('Bootstrap input changed', path)
            receipt.update(status='exact-packagecompiler-bootstrap-complete-awaiting-library-pilot',
                julia_version=version['stdout'].decode().strip(),
                packagecompiler=package[0], project=str(project), depot=str(depot),
                runtime_environment=environment, pins_unchanged_at_end=True)
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
