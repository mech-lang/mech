"""Inspect the unchanged installed Julia image; does not launch Julia or build.

This is a preliminary symbol-availability inventory, never an AOT proof. All
hashes, tool invocations and immutable journals hold the shared timing mutex.
"""
import argparse
from datetime import datetime, timezone
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import sys
import traceback

HERE = Path(__file__).resolve().parent
CLBG = HERE.parents[1]
RUNTIME = Path('/tmp/mech-clbg-julia/julia-1.13.1')
LOCK = Path('/tmp/mech-clbg-timing.lock')
SERVICE = CLBG / 'c3-comparison/20261012/revisions/private-v1/guarded_prepare.py'
SERVICE_PROPOSAL = CLBG / 'c3-comparison/20261012/proposals/private-v2.json'
SERVICE_PROPOSAL_SHA = 'aacefa87b453aafa39ae30b3cb8a78324b5ad7dceb28dad9a7c1f8ffa35990f1'
IMAGES = ('lib/libjulia.1.13.1.dylib',
          'lib/julia/libjulia-internal.1.13.1.dylib',
          'lib/julia/libjulia-codegen.1.13.1.dylib', 'lib/julia/sys.dylib')
C_HOOKS = ('jl_fptr_interpret_call', 'jl_interpret_opaque_closure',
           'jl_interpret_toplevel_thunk', 'jl_interpret_toplevel_expr_in',
           'jl_emit_codeinst_to_jit', 'jl_generate_fptr_for_unspecialized',
           'jl_jit_abi_converter_impl')


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def save(path, value):
    with path.open('x') as stream:
        json.dump(value, stream, indent=2, allow_nan=False)
        stream.write('\n')


def now():
    return datetime.now(timezone.utc).isoformat()


def run(output):
    assert output.parent == HERE / 'jobs' and not output.exists()
    print('WAIT common mutex: unchanged Julia runtime symbol inspection', flush=True)
    with (HERE / 'runtime-symbol-inspection-owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        with LOCK.open('a+') as mutex:
            fcntl.flock(mutex, fcntl.LOCK_EX)
            named, opened = LOCK.stat(), os.fstat(mutex.fileno())
            assert (named.st_dev, named.st_ino) == (opened.st_dev, opened.st_ino)
            assert not output.exists()
            output.mkdir(parents=True)
            receipt = dict(status='failed', started_at=now(), common_timing_mutex=str(LOCK),
                exclusive_mutex_held=True, mutex_inode=opened.st_ino, actual_julia_processes=0,
                builds=0, benchmark_runs=0, runtime_aot_proven=False, chart_admission=False,
                automatic_retry=False, loaded_image_addresses_resolved=False, pins={})
            pins = receipt['pins']
            try:
                assert sha(SERVICE_PROPOSAL) == SERVICE_PROPOSAL_SHA
                proposal = json.loads(SERVICE_PROPOSAL.read_text())
                assert sha(SERVICE) == proposal['pins'][str(SERVICE)]
                pins.update({str(SERVICE_PROPOSAL): SERVICE_PROPOSAL_SHA,
                    str(SERVICE): sha(SERVICE), str(Path(__file__).resolve()): sha(__file__),
                    str(Path(sys.executable).resolve()): sha(sys.executable)})
                spec = importlib.util.spec_from_file_location('julia_symbol_owned_command_service', SERVICE)
                service = importlib.util.module_from_spec(spec)
                spec.loader.exec_module(service)
                assert Path(service.__file__).resolve() == SERVICE
                guard = service.load_guard()
                pins[str(service.GUARD)] = service.GUARD_SHA
                for name in ('run.py', 'prepare.py'):
                    path = service.SUITE / name
                    assert sha(path) == proposal['pins'][str(path)]
                    pins[str(path)] = sha(path)
                harness = guard.load_harness()
                assert Path(harness.__file__).resolve() == service.SUITE / 'run.py'
                runtime_root = RUNTIME.resolve(strict=True)
                environment = dict(os.environ, LC_ALL='C', LANG='C')
                tools = {name: Path('/usr/bin') / name for name in ('xcrun', 'lipo', 'otool')}
                for path in tools.values(): pins[str(path)] = sha(path)
                result = service.owned_command(guard, harness, [str(tools['xcrun']), '--find', 'nm'],
                    environment, output / 'tools', mutex.fileno(), 30, 'find-nm')
                nm = Path(result['stdout'].decode().strip())
                assert nm.is_absolute() and nm.is_file() and '\n' not in str(nm)
                pins[str(nm)] = sha(nm)
                records = []
                found = {name: [] for name in (*C_HOOKS, 'JuliaOJIT::addModule')}
                for relative in IMAGES:
                    image = (runtime_root / relative).resolve(strict=True)
                    assert image.is_relative_to(runtime_root)
                    pins[str(image)] = sha(image)
                    directory = output / 'images' / Path(relative).stem
                    arch = service.owned_command(guard, harness, [str(tools['lipo']), '-archs', str(image)],
                        environment, directory, mutex.fileno(), 30, 'architecture')
                    assert arch['stdout'].decode().split() == ['arm64']
                    service.owned_command(guard, harness, [str(tools['otool']), '-L', str(image)],
                        environment, directory, mutex.fileno(), 30, 'linked-images')
                    result = service.owned_command(guard, harness, [str(nm), '-a', str(image)],
                        environment, directory, mutex.fileno(), 60, 'symbols')
                    for line in result['stdout'].decode().splitlines():
                        parts = line.split()
                        # U entries identify imports, not callable definitions.
                        if len(parts) != 3 or not re.fullmatch(r'[0-9a-fA-F]+', parts[0]) or parts[1].upper() == 'U':
                            continue
                        offset, kind, linkage = parts
                        hooks = [name for name in C_HOOKS if linkage == '_' + name]
                        if 'JuliaOJIT' in linkage and 'addModule' in linkage:
                            hooks.append('JuliaOJIT::addModule')
                        for hook in hooks:
                            found[hook].append(dict(image=str(image), image_sha256=pins[str(image)],
                                linkage_name=linkage, nm_type=kind, file_symbol_value=offset,
                                raw_line=line, source_route_review_pending=True))
                    records.append(dict(image=str(image), sha256=pins[str(image)],
                        raw_symbols=str(directory / 'symbols.stdout'),
                        raw_symbols_sha256=sha(directory / 'symbols.stdout')))
                for path, digest in pins.items(): assert sha(path) == digest, ('Input changed', path)
                receipt.update(status='symbol-availability-inventory-awaiting-route-and-loaded-address-review',
                    images=records, hooks=found, unavailable_hooks=[name for name, entries in found.items() if not entries],
                    pins_unchanged_at_end=True, actual_inspection_tools='xcrun/nm/lipo/otool',
                    runtime_version='directory/version-labelled only; actual stored version receipt review pending',
                    required_next=['tagged route and mangled overload review', 'actual loaded image/address resolution',
                        'observer installed before initialization across all threads',
                        'missing-native canary rejection', 'actual library/module-context and source coverage review'])
            except BaseException as error:
                receipt.update(status='failed', error_type=type(error).__name__, error=str(error) or type(error).__name__)
                with (output / 'failure.log').open('x') as stream: stream.write(traceback.format_exc())
                raise
            finally:
                receipt['finished_at'] = now()
                path = output / 'report.json'
                save(path, receipt)
                terminal = {str(item): sha(item) for item in output.rglob('*') if item.is_file()}
                save(output / 'terminal-pins.json', dict(status=receipt['status'], pins=terminal,
                    report_sha256=sha(path), exclusive_mutex_held=True, runtime_aot_proven=False,
                    chart_admission=False, automatic_retry=False))
            print(json.dumps(dict(status=receipt['status'], unavailable_hooks=receipt['unavailable_hooks'],
                report=str(path))), flush=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', type=Path, required=True)
    run(parser.parse_args().output.resolve())
