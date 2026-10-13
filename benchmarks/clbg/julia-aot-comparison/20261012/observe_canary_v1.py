"""Run one locked LLDB calibration against the unchanged built canary."""
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
BUILD_REPORT = HERE / 'jobs/canary-library-v1/report.json'
PREPARATION = CLBG / 'c3-comparison/20261012/revisions/private-v3/guarded_prepare.py'
OUTPUT = HERE / 'jobs/canary-observation-v1'
INTERPRETERS = {'jl_fptr_interpret_call', 'jl_interpret_opaque_closure',
    'jl_interpret_toplevel_thunk', 'jl_interpret_toplevel_expr_in'}


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def save(path, value):
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def main():
    print('WAIT common mutex: Julia actual missing-native LLDB calibration', flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        opened, named = os.fstat(lock.fileno()), LOCK.stat()
        assert (opened.st_dev, opened.st_ino) == (named.st_dev, named.st_ino)
        assert not OUTPUT.exists()
        OUTPUT.mkdir()
        pins = {str(Path(__file__).resolve()): sha(__file__)}
        receipt = dict(status='failed', runtime_aot_proven=False, chart_admission=False,
            actual_benchmark_runs=0, automatic_retry=False,
            exclusive_common_timing_mutex_held=True, common_timing_mutex=str(LOCK),
            started_at=datetime.now(timezone.utc).isoformat())
        try:
            build = json.loads(BUILD_REPORT.read_text())
            assert build['status'] == 'canary-library-built-awaiting-native-coverage-and-observer-calibration'
            assert build['pins_unchanged_at_end'] is True and build['actual_canary_executions'] == 0
            pins.update(build['pins'])
            pins[str(BUILD_REPORT)] = sha(BUILD_REPORT)
            for path, digest in pins.items():
                assert sha(path) == digest, ('Actual canary input changed', path)
            spec = importlib.util.spec_from_file_location('julia_observer_owned_commands', PREPARATION)
            preparation = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(preparation)
            guard = preparation.load_guard()
            harness = guard.load_harness()
            harness.install_interrupt_handlers()
            bootstrap = json.loads(Path(build['bootstrap']['path']).read_text())
            environment = dict(bootstrap['runtime_environment'])
            location = preparation.owned_command(guard, harness, ['/usr/bin/xcrun', '--find', 'lldb'],
                environment, OUTPUT / 'tools/lldb-location', lock.fileno(), 30, 'location')
            debugger = Path(location['stdout'].decode().strip()).resolve(strict=True)
            assert debugger.is_file()
            pins[str(debugger)] = sha(debugger)
            preparation.owned_command(guard, harness, [str(debugger), '--version'],
                environment, OUTPUT / 'tools/lldb-version', lock.fileno(), 30, 'version')
            python_path = preparation.owned_command(guard, harness, [str(debugger), '-P'],
                environment, OUTPUT / 'tools/lldb-python', lock.fileno(), 30, 'python-path')
            python_root = Path(python_path['stdout'].decode().strip()).resolve(strict=True)
            assert (python_root / 'lldb/__init__.py').is_file()
            for path in python_root.rglob('*'):
                if path.is_file() and path.suffix in ('.py', '.so', '.dylib'):
                    pins[str(path)] = sha(path)
                    pins[str(path.resolve())] = sha(path)
            framework = next(path for path in python_root.parents if path.name == 'LLDB.framework')
            framework_binary = (framework / 'LLDB').resolve(strict=True)
            pins[str(framework_binary)] = sha(framework_binary)
            snapshots = OUTPUT / 'snapshot'
            snapshots.mkdir()
            source = HERE / 'revisions/native-observer-v1/observe_lldb.py'
            observer = snapshots / source.name
            shutil.copyfile(source, observer)
            assert sha(source) == sha(observer)
            pins[str(source)] = pins[str(observer)] = sha(source)
            observer.chmod(0o444)
            symbol_path = HERE / 'jobs/runtime-symbols-v1/report.json'
            route_path = HERE / 'jobs/runtime-symbol-routes-v2/report.json'
            symbols, routes = json.loads(symbol_path.read_text()), json.loads(route_path.read_text())
            descriptors = {}
            for origin, inventory in ((symbol_path, symbols['hooks']), (route_path, routes['symbols'])):
                pins[str(origin)] = sha(origin)
                for route, records in inventory.items():
                    for record in records:
                        name = record['linkage_name'].removeprefix('_')
                        # Mach-O external underscore is not part of the LLDB
                        # symbol spelling; C++'s own _Z prefix remains intact.
                        descriptor = dict(name=name, kind='interpreter' if route in INTERPRETERS else
                            'engine-emission' if route == 'JuliaOJIT::addModule' else
                            'codegen-initialization' if route == 'jl_init_codegen' else 'codegen-route-entry',
                            origin=str(origin), original_image=record['image'],
                            original_image_sha256=record['image_sha256'],
                            original_file_symbol_value=record['file_symbol_value'])
                        if name in descriptors:
                            assert descriptors[name]['original_file_symbol_value'] == descriptor['original_file_symbol_value']
                        descriptors[name] = descriptor
            assert INTERPRETERS <= set(descriptors)
            assert any(item['kind'] == 'engine-emission' for item in descriptors.values())
            for base in ('jl_emit_codeinst_to_jit', 'jl_generate_fptr_for_unspecialized', 'jl_jit_abi_converter'):
                assert {base, base + '_impl', base + '_fallback'} <= set(descriptors)
            request = OUTPUT / 'request.json'
            save(request, dict(purpose='unscored-missing-native-canary-calibration',
                executable=build['direct_executable'], cwd=str(Path(build['direct_executable']).parent),
                arguments=['41', '--julia-args', '--compile=no', '--threads=8,0',
                    '--startup-file=no', '--history-file=no', '-O3'], environment=environment,
                output=str(OUTPUT / 'native-observation'), hooks=list(descriptors.values()),
                actual_common_mutex_held=True, all_benchmark_timings_excluded=True))
            pins[str(request)] = sha(request)
            command = [str(debugger), '--no-lldbinit', '--batch',
                '-o', 'command script import ' + str(observer),
                '-o', 'script observe_lldb.observe(lldb.debugger, ' + repr(str(request)) + ')',
                '-o', 'quit']
            preparation.owned_command(guard, harness, command, environment,
                OUTPUT / 'debugger', lock.fileno(), 300, 'actual-canary-observation')
            observed_path = OUTPUT / 'native-observation/report.json'
            observed = json.loads(observed_path.read_text())
            assert observed['status'] == 'raw-canary-observation-complete-awaiting-calibration-review'
            assert observed['hooks_armed_before_launch'] is observed['entry_and_shutdown_observed'] is True
            assert observed['runtime_aot_proven'] is observed['chart_admission'] is False
            assert not observed.get('owned_native_cleanup')
            actual_julia_images = []
            for item in observed['final_loaded_modules']:
                path = Path(item['path'])
                if 'julia' in path.name.lower() or 'clbgaotcanary' in path.name.lower():
                    assert path.is_file(), ('Loaded runtime image missing', item)
                    pins[str(path)] = sha(path)
                    actual_julia_images.append(dict(item, sha256=pins[str(path)]))
            pins.update(preparation.pin_tree(OUTPUT))
            for path, digest in pins.items():
                assert sha(path) == digest, ('Observer evidence changed', path)
            receipt.update(status='actual-canary-observation-saved-awaiting-identity-and-negative-control-review',
                observer_report=str(observed_path), actual_canary_executions=1,
                actual_loaded_julia_images=actual_julia_images,
                native_exit_status=observed['exit_status'], hook_event_count=len(observed['hook_events']),
                pending_hooks=observed['pending_hooks'], pins_unchanged_at_end=True,
                calibration_admitted=False, runtime_aot_proven=False)
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
