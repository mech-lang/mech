"""Build and qualify the current generic source executor without timing full runs."""
import fcntl
import hashlib
import importlib.util
import json
import math
import os
from pathlib import Path
import shutil
import subprocess
import sys
from datetime import datetime, timezone

HERE = Path(__file__).resolve().parent
E = HERE.parent
ROOT = E.parents[4]
N = ROOT / 'benchmarks/clbg/r-apl-uiua-comparison/20261011'
LOCK = Path('/tmp/mech-clbg-timing.lock')
APPLICATION_SHA = '433e116691eed736b3152d42f51b0a642b46d52a39f07e8f97e5d4aa83f58031'
POWER = dict(source='AC Power', low_power_mode=0)
WIDTHS = (1, 7, 8, 9, 13, 31, 63, 64, 65, 129, 200, 257)
SOURCE_ROWS = [
    ('scalar11', 'scalar-canonical', 11, '4e4b881e9c2368ba9ddfcbd2b8df49f78e6fd759833b26e7a10374b432de7b3a', ['--scalar']),
    ('row16', 'five-rows-canonical', 16, 'baf1cc933c9d5f1741f9b20319d66a488fca177940fa5a23bafdd2917a333cde', ['--fp-contract-loop']),
]


def now():
    return datetime.now(timezone.utc).isoformat()


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def save(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + '.tmp')
    temporary.write_text(json.dumps(value, indent=2) + '\n')
    temporary.replace(path)


def verify(pins):
    for path, digest in pins.items():
        assert sha(path) == digest, ('Pinned file changed', path)


def import_file(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def scalar_reference(n):
    """Independent per-pixel, predicate-before-commit 50-step reference."""
    data = bytearray(f'P4\n{n} {n}\n'.encode())
    for row in range(n):
        packed = bytearray((n + 7) // 8)
        ci = row * (2.0 / n) - 1.0
        for pixel in range(n):
            cr = pixel * (2.0 / n) - 1.5
            zr, zi, iteration = 0.0, 0.0, 0
            while True:
                tr, ti = zr * zr, zi * zi
                inside = tr + ti <= 4.0
                if iteration >= 50 or not inside:
                    break
                zr, zi = tr - ti + cr, (zr + zr) * zi + ci
                iteration += 1
            if inside:
                packed[pixel // 8] |= 128 >> (pixel % 8)
        data.extend(packed)
    return bytes(data)


def source_timing(result, mode):
    assert mode in ('jit', 'resident'), ('Unsupported source timing mode', mode)
    records = [json.loads(line[7:]) for line in result['stderr'].splitlines() if line.startswith('TIMING ')]
    assert len(records) == 1, ('Expected exactly one TIMING record', result['stderr'])
    timing = records[0]
    assert set(timing) == {'prepare_seconds', 'kernel_seconds'}
    for key, value in timing.items():
        assert type(value) in (float, int) and math.isfinite(value)
        assert value >= 0 and (key != 'kernel_seconds' or value > 0)
    wall = result['seconds']
    assert type(wall) in (float, int) and math.isfinite(wall) and wall > 0
    assert timing['prepare_seconds'] + timing['kernel_seconds'] <= wall + 0.001
    return dict(wall_seconds=wall, **timing,
                reported_seconds=timing['kernel_seconds'] if mode == 'resident' else wall)


def native_small(harness, row, n, workers, references, lock_fd):
    """Retain frozen execution/cleanup guards; replace only the small PBM oracle."""
    candidate = dict(row, runtime_environment=dict(row['runtime_environment'], RAYON_NUM_THREADS=str(workers)))
    command = [part.replace('{n}', str(n)) for part in candidate['command']]
    output = HERE / 'validation' / f'{row["id"]}-{n}-workers-{workers}.pbm'
    original_validate, original_popen, original_work = harness.validate, harness.subprocess.Popen, harness.WORK
    native = None

    def validate(benchmark, full, stream):
        assert benchmark == 'mandelbrot' and full is False
        stream.seek(0)
        data = stream.read()
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_bytes(data)
        assert all(data == reference for reference in references), ('Independent PBM mismatch', row['id'], n, workers)
        return dict(bytes=len(data), sha256=hashlib.sha256(data).hexdigest(), reference_match=True)

    def popen(*args, **kwargs):
        nonlocal native
        argv = args[0] if args else kwargs.get('args')
        owned = list(argv) == command and kwargs.get('start_new_session')
        if owned:
            kwargs['pass_fds'] = tuple(sorted(set(kwargs.get('pass_fds', ())) | {lock_fd}))
        process = original_popen(*args, **kwargs)
        try:
            if owned:
                native = dict(pid=process.pid, pgid=process.pid, command=command, inherited_common_mutex=True)
        except BaseException:
            if owned:
                harness.kill_group(process.pid)
                process.wait()
            raise
        return process

    harness.validate, harness.subprocess.Popen = validate, popen
    harness.WORK = dict(original_work, mandelbrot=(n, original_work['mandelbrot'][1]))
    try:
        result = harness.execute(candidate, False, 90, 10240)
        assert native and result['exit_status'] == 0 and result['valid_environment'] is True
        assert result['workload_n'] == n and result['validation']['reference_match'] is True
        for key in ('error', 'resource_violation', 'resource_monitor_errors', 'monitor_errors',
                    'monitor_finish_error', 'cleanup_error', 'remaining_group_members',
                    'competing_compilers', 'power_changes'):
            assert not result.get(key), (key, result.get(key))
        assert not harness.group_members(native['pgid'])
        timing = source_timing(result, row['mode'])
        return dict(result, id=row['id'], size=n, workers=workers, timing=timing,
                    supervised_native_group=native, output=str(output), qualified=True)
    finally:
        harness.validate, harness.subprocess.Popen = original_validate, original_popen
        harness.WORK = original_work
        if native:
            harness.kill_group(native['pgid'])


def supervision_probes(harness, profiles, references, pins, lock_fd):
    """Exercise the new forked supervisor at N65; never a canonical full sample."""
    runner = import_file('canonical_modes_supervision_probe', HERE / 'full_study.py')
    runner.OUT = HERE / 'supervision-probes'
    runner.REPORT = runner.OUT / 'report.json'
    controls = json.loads((E / 'full-study/manifest.json').read_text())['profiles']
    rows = list(profiles)
    for old_id, new_id in (('scalar11', 'scalar11-aot-control'), ('five16-rows-loop-fma', 'row16-aot-control')):
        row = dict(next(item for item in controls if item['id'] == old_id))
        row.update(id=new_id, mode='aot-control', key='mandelbrot:canonical-source-modes:' + new_id)
        for path, digest in ((row['source'], row['source_sha256']), (row['binary'], row['binary_sha256'])):
            assert path not in pins or pins[path] == digest, ('AOT probe provenance differs', path)
            assert sha(path) == digest
            pins[path] = digest
        rows.append(row)
    probe_manifest = dict(size=65, workers=8, pins=pins, timeout_seconds=90, maximum_rss_mib=10240,
        full_output_reference=dict(bytes=len(references[0]), sha256=hashlib.sha256(references[0]).hexdigest()))
    original_validate, original_work = harness.validate, harness.WORK

    def validate(benchmark, full, stream):
        assert benchmark == 'mandelbrot' and full is True
        stream.seek(0)
        data = stream.read()
        assert all(data == reference for reference in references), 'Supervision-probe PBM mismatch'
        return dict(bytes=len(data), sha256=hashlib.sha256(data).hexdigest(), reference_match=True)

    report = dict(status='running', size=65, canonical_full_workload=False,
                  never_chart_eligible=True, attempts=[], active=None)
    runner.save(runner.REPORT, report)
    harness.validate, harness.WORK = validate, dict(original_work, mandelbrot=(65, 65))
    try:
        for row in rows:
            print('SUPERVISION PROBE', row['id'], 65, 8, flush=True)
            result = runner.one_attempt(harness, row, probe_manifest, report, 1, lock_fd)
            result.update(id=row['id'], canonical_full_workload=False, never_chart_eligible=True,
                          qualification_scope='Unscored N65 fork/PGID/mutex/timing/cleanup probe')
            report['attempts'].append(result)
            report['active'] = None
            runner.save(runner.REPORT, report)
            assert result['qualified'] is True, ('Supervision-probe qualification failed', row['id'], result)
        report.update(status='passed', checks=len(rows), finished_at=now())
        runner.save(runner.REPORT, report)
        return report
    except BaseException as error:
        report.update(status='failed', error=str(error), error_type=type(error).__name__, finished_at=now())
        runner.save(runner.REPORT, report)
        raise
    finally:
        harness.validate, harness.WORK = original_validate, original_work


def main():
    HERE.mkdir(parents=True, exist_ok=True)
    with LOCK.open('a+') as lock:
        print('Waiting for shared lock for isolated source-mode build and qualification.', flush=True)
        fcntl.flock(lock, fcntl.LOCK_EX)
        print('Acquired shared lock.', flush=True)
        assert not (HERE / 'compiler-manifest.json').exists(), 'An existing sealed compiler must not be rebuilt in place'
        assert not (HERE / 'qualification.json').exists(), 'Existing qualification needs explicit review; no automatic resumption'
        harness_manifest = N / 'source-audit-uiua.json'
        audit = json.loads(harness_manifest.read_text())
        helper_paths = [N / 'run.py', N / 'prepare.py', ROOT / 'benchmarks/clbg/compact/run.py',
                        ROOT / 'benchmarks/clbg/nbody/run.py', ROOT / 'benchmarks/clbg/array-comparison/all-20261009/run.py']
        for path in helper_paths:
            assert sha(path) == audit['pins'][str(path)]
        compiler_basis = E / 'evidence/compiler-pins.json'
        prior = json.loads(compiler_basis.read_text())
        application = ROOT / 'src/engine/src/native_sequence/application.rs'
        assert sha(application) == APPLICATION_SHA
        driver = ROOT / 'benchmarks/clbg/interpreter-integration/20261011/driver/Cargo.toml'
        driver_lock = driver.with_name('Cargo.lock')
        source_paths = set(prior['source_pins']) | {str(driver), str(driver_lock), str(Path(__file__)), str(compiler_basis),
            str(HERE / 'test_protocol.py'), str(HERE / 'README.md'), str(Path(sys.executable).resolve())}
        pins = {path: sha(path) for path in sorted(source_paths)}
        for tool in ('cargo', 'rustc'):
            assert sha(prior[tool]['path']) == prior[tool]['sha256']
            pins[prior[tool]['path']] = prior[tool]['sha256']
        toolchain_root = Path(prior['rustc']['path']).parents[1]
        toolchain_libraries = sorted(set(toolchain_root.glob('lib/*.dylib')) |
                                     set(toolchain_root.glob('lib/rustlib/aarch64-apple-darwin/lib/*.rlib')) |
                                     set(toolchain_root.glob('lib/rustlib/aarch64-apple-darwin/lib/*.dylib')))
        assert toolchain_libraries, 'Toolchain runtime/standard-library files are missing'
        pins.update({str(path): sha(path) for path in toolchain_libraries})
        frozen = json.loads((ROOT / 'benchmarks/clbg/interpreter-integration/20261011/baseline.json').read_text())
        frozen_bins = {frozen['binary']: frozen['binary_sha256'], frozen['after_binary']: frozen['after_binary_sha256']}
        verify(frozen_bins)
        pins.update(frozen_bins)
        for historical in ('benchmarks/clbg/interpreter-integration/20261011/baseline.json',
                           'benchmarks/clbg/interpreter-integration/20261011/run.py',
                           'benchmarks/clbg/execution-modes/20261010-final/run.py'):
            pins[str(ROOT / historical)] = sha(ROOT / historical)
        pins.update({str(path): sha(path) for path in helper_paths + [harness_manifest]})
        for native_input in ('/opt/homebrew/include/gmp.h', '/opt/homebrew/lib/libgmp.dylib', '/opt/homebrew/lib/libgmpxx.dylib'):
            native_path = Path(native_input)
            pins[str(native_path)] = sha(native_path)
            pins[str(native_path.resolve())] = sha(native_path.resolve())
        target = HERE / '.build/target'
        binary = HERE / '.build/mech-source-modes'
        protocol_log = HERE / 'protocol-tests.log'
        protocol_command = [sys.executable, str(HERE / 'test_protocol.py'), '-v']
        with protocol_log.open('wb') as stream:
            protocol = subprocess.run(protocol_command, stdout=stream, stderr=subprocess.STDOUT)
        assert protocol.returncode == 0, ('Timing protocol tests failed', str(protocol_log))
        pins[str(protocol_log)] = sha(protocol_log)
        command = [prior['cargo']['path'], 'build', '--release', '--offline', '--locked',
                   '--manifest-path', str(driver), '--bin', 'mech-interpreter-integrated']
        overrides = dict(CARGO_TARGET_DIR=str(target), RUSTC=prior['rustc']['path'],
                         C_INCLUDE_PATH='/opt/homebrew/include', LIBRARY_PATH='/opt/homebrew/lib',
                         RUSTFLAGS='-D warnings -C target-cpu=native -L native=/opt/homebrew/lib')
        for conflicting in ('CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER', 'RUSTC_BOOTSTRAP'):
            assert not os.environ.get(conflicting), ('Unexpected build override', conflicting)
        clang_path = subprocess.check_output(['/usr/bin/xcrun', '--find', 'clang'], text=True).strip()
        pins[clang_path] = sha(clang_path)
        pins['/usr/bin/xcrun'] = sha('/usr/bin/xcrun')
        native_toolchain = dict(clang=clang_path, clang_sha256=sha(clang_path),
            clang_version=subprocess.check_output([clang_path, '--version'], text=True),
            rustc_version_verbose=subprocess.check_output([prior['rustc']['path'], '--version', '--verbose'], text=True),
            cargo_version=subprocess.check_output([prior['cargo']['path'], '--version'], text=True))
        build_log = HERE / 'build.log'
        print('Building current generic source executor; original driver manifest/lock are read-only.', flush=True)
        with build_log.open('wb') as stream:
            completed = subprocess.run(command, env=dict(os.environ, **overrides), stdout=stream, stderr=subprocess.STDOUT)
        assert completed.returncode == 0, ('Isolated build failed; see log', str(build_log))
        verify(pins)
        pins[str(build_log)] = sha(build_log)
        shutil.copy2(target / 'release/mech-interpreter-integrated', binary)
        pins[str(binary)] = sha(binary)
        libraries = subprocess.check_output(['/usr/bin/otool', '-L', str(binary)], text=True)
        for line in libraries.splitlines()[1:]:
            path = line.strip().split(' (', 1)[0]
            if path.startswith('/') and not path.startswith(('/usr/lib/', '/System/Library/')):
                assert Path(path).is_file()
                pins[path] = sha(path)
                pins[str(Path(path).resolve())] = sha(Path(path).resolve())
        compiler = dict(status='sealed', created_at=now(), driver=str(binary), driver_sha256=sha(binary),
                        source_pins=dict(pins), application_source=str(application), application_source_sha256=APPLICATION_SHA,
                        cargo=prior['cargo'], rustc=prior['rustc'], build_command=command, build_environment=overrides,
                        native_toolchain=native_toolchain,
                        toolchain_library_pins={str(path): pins[str(path)] for path in toolchain_libraries},
                        compiler_lineage=dict(manifest=str(compiler_basis), sha256=sha(compiler_basis),
                            changed_source_pins={path: dict(previous=digest, current=pins[path])
                                for path, digest in prior['source_pins'].items() if pins[path] != digest}),
                        build_log=str(build_log), build_log_sha256=sha(build_log), linked_libraries=libraries,
                        protocol_tests=dict(command=protocol_command, status='passed', cases=6,
                                            log=str(protocol_log), log_sha256=sha(protocol_log)),
                        frozen_binaries_unchanged=True, specialized_host_kernel=False,
                        shared_timing_lock=str(LOCK), isolated_target=str(target))
        compiler_path = HERE / 'compiler-manifest.json'
        save(compiler_path, compiler)
        pins[str(compiler_path)] = sha(compiler_path)
        pins[str(HERE / 'full_study.py')] = sha(HERE / 'full_study.py')
        sys.path.insert(0, str(N))
        harness = import_file('canonical_mode_frozen_harness', N / 'run.py')
        harness.install_interrupt_handlers()
        harness.compact.wait_for_environment(harness.REQUIRED)
        reference_module = import_file('canonical_mode_independent_reference', E / 'validate_current.py')
        pins[str(E / 'validate_current.py')] = sha(E / 'validate_current.py')
        pins[str(ROOT / 'benchmarks/clbg/mandelbrot/upstream/mandelbrot-200.pbm')] = sha(ROOT / 'benchmarks/clbg/mandelbrot/upstream/mandelbrot-200.pbm')
        profiles = []
        mode_environment = {key: harness.compact.ENV[key] for key in ('RAYON_NUM_THREADS', 'JULIA_NUM_THREADS',
            'OMP_NUM_THREADS', 'OMP_DYNAMIC', 'CHPL_RT_NUM_THREADS_PER_LOCALE', 'GOMAXPROCS',
            'OPENBLAS_NUM_THREADS', 'PYTHONHASHSEED')}
        mode_environment['MECH_TIMINGS'] = '1'
        for identity, suffix, loc, digest, flags in SOURCE_ROWS:
            source = E / f'mandelbrot-array-state-{suffix}.mec'
            assert sha(source) == digest
            assert len([line for line in source.read_text().splitlines() if line.strip()]) == loc
            pins[str(source)] = digest
            for mode in ('jit', 'resident'):
                identifier = identity + '-' + mode
                profiles.append(dict(id=identifier, key='mandelbrot:canonical-source-modes:' + identifier,
                    benchmark='mandelbrot', language='mech-jit' if mode == 'jit' else 'mech-resident',
                    mode=mode, source=str(source), source_sha256=digest, loc=loc, physical_loc=loc, loc_expansions=[],
                    source_key='mandelbrot:array-state:' + ('scalar11' if identity == 'scalar11' else 'five16-rows-loop-fma'),
                    binary=str(binary), binary_sha256=sha(binary), cwd=str(ROOT),
                    flags=list(flags), simd=identity != 'scalar11', loop_fma=identity == 'row16',
                    command=[str(binary), str(source), mode, '{n}'] + flags,
                    runtime_environment=dict(mode_environment), maximum_workers=8,
                    timing_boundary='Fresh-process elapsed including startup, parsing, admission, JIT compilation, complete PBM and shutdown.' if mode == 'jit' else 'Loaded typed interpretation after source loading, graph optimization and activation; image computation, PBM output and flush included. Preparation and whole-process time retained separately.'))
        qualification = dict(status='running', created_at=now(), profiles=profiles, widths=list(WIDTHS),
                             worker_settings=[1, 8], images=[], source_changes=False,
                             compiler_manifest=str(compiler_path), compiler_manifest_sha256=sha(compiler_path),
                             source_metric='LOC only; same canonical11/16 authored sources, one independent binding/statement per line',
                             shared_timing_lock=str(LOCK), pins=pins)
        qualification_path = HERE / 'qualification.json'
        save(qualification_path, qualification)
        for n in WIDTHS:
            references = [scalar_reference(n), reference_module.reference(n, 1), reference_module.reference(n, 5)]
            assert references[0] == references[1] == references[2]
            if n == 200:
                assert references[0] == (ROOT / 'benchmarks/clbg/mandelbrot/upstream/mandelbrot-200.pbm').read_bytes()
            for profile in profiles:
                for workers in (1, 8):
                    print('SMALL', profile['id'], n, workers, flush=True)
                    result = native_small(harness, profile, n, workers, references, lock.fileno())
                    qualification['images'].append(result)
                    save(qualification_path, qualification)
        references = [scalar_reference(65), reference_module.reference(65, 1), reference_module.reference(65, 5)]
        qualification['supervision_probes'] = supervision_probes(harness, profiles, references, pins, lock.fileno())
        pins[str(HERE / 'supervision-probes/report.json')] = sha(HERE / 'supervision-probes/report.json')
        verify(pins)
        qualification.update(status='passed', finished_at=now(), pins_unchanged_at_end=True,
                             checks=len(qualification['images']), complete_pbms=True)
        assert qualification['checks'] == 96
        save(qualification_path, qualification)
        print(json.dumps(dict(status='passed', checks=96, qualification=str(qualification_path),
                              qualification_sha256=sha(qualification_path), binary=str(binary), binary_sha256=sha(binary)), indent=2), flush=True)


if __name__ == '__main__':
    try:
        main()
    except BaseException as error:
        save(HERE / 'preparation-failure.json', dict(finished_at=now(), error=str(error), error_type=type(error).__name__))
        path = HERE / 'qualification.json'
        if path.exists():
            report = json.loads(path.read_text())
            if report.get('status') == 'running':
                report.update(status='failed', finished_at=now(), error=str(error), error_type=type(error).__name__)
                save(path, report)
        raise
