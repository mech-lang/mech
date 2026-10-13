"""Additive production-work trace qualification before complete Pony studies."""
import argparse
from contextlib import contextmanager
import fcntl
import importlib.util
import json
import os
from pathlib import Path
import shutil
import sys
import time
import traceback

HERE = Path(__file__).resolve().parent
COHORT = HERE.parents[1]
sys.path.insert(0, str(COHORT))
import guarded_prepare_v6 as original_preparation
preparation = original_preparation
import references
generate_traces = check_traces = static_review = reuse_work_v2 = None
LOCK = preparation.LOCK
NUMERIC = dict(binarytrees=6, pidigits=97)


def new(path, value):
    preparation.new_json(path, value)


def load_packet_helpers(guard, pins):
    """Import mutable work helpers only after actual held-lock pin review."""
    global generate_traces, check_traces, static_review, reuse_work_v2
    import importlib
    names = ('generate_traces', 'generate_traces_base', 'check_traces',
             'check_traces_base', 'static_review', 'reuse_work_v2')
    expected = {name: HERE / (name + '.py') for name in names}
    required = {str(path): pins[str(path)] for path in expected.values()}
    guard.verify(required)
    # A module imported before this reviewed packet cannot be certified merely
    # by hashing a potentially changed source file after it entered the cache.
    assert all(name not in sys.modules for name in names), 'Unreviewed cached mutable work helper'
    modules = {name: importlib.import_module(name) for name in ('generate_traces', 'check_traces', 'static_review', 'reuse_work_v2')}
    for name, path in expected.items():
        assert Path(sys.modules[name].__file__).resolve() == path.resolve()
    guard.verify(required)
    generate_traces, check_traces, static_review = (modules[name] for name in ('generate_traces', 'check_traces', 'static_review'))
    reuse_work_v2 = modules['reuse_work_v2']


def seal(revision, qualification):
    global preparation
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        with metadata_terminal(revision, lock):
            seal_locked(revision, qualification, lock)


def seal_locked(revision, qualification, lock):
    global preparation
    report_path = COHORT / 'qualification' / qualification / 'report.json'
    assert qualification == 'private-v8' and original_preparation.sha(report_path) == '4da05f6dd351609923f5f1ca227d3db2b33a354925614b74f9c5dae39839d159'
    report = json.loads(report_path.read_text())
    assert report['status'] == 'bounded-output-primitives-passed-awaiting-work-worker-and-LOC-review'
    guard = preparation.load_guard()
    guard.verify(report['pins'])
    packet = sorted(set(HERE.glob('*.py')) | set(HERE.glob('*.md')))
    packet_pins = {str(path): preparation.sha(path) for path in packet}
    guard.verify(packet_pins)
    import preparation_service
    assert Path(preparation_service.__file__).resolve() == HERE / 'preparation_service.py'
    preparation = preparation_service.bind(original_preparation, guard, report['pins'], packet_pins)
    load_packet_helpers(guard, packet_pins)
    rows = report['profiles']
    assert len(rows) == 20 and len({row['id'] for row in rows}) == 20
    toolchain = report['toolchain']
    assert preparation.sha(toolchain['compiler']) == toolchain['compiler_sha256'] == report['pins'][toolchain['compiler']]
    output = COHORT / 'work-qualification' / revision
    assert not output.exists(), 'Every work revision is additive and immutable'
    output.mkdir(parents=True)
    harness = guard.load_harness(); harness.install_interrupt_handlers()
    reused = reuse_work_v2.reconstruct(preparation, guard, harness, generate_traces, check_traces, references, packet_pins)
    reuse_path = output / 'prior-work-reuse.json'
    new(reuse_path, reused)
    assert packet == sorted(set(HERE.glob('*.py')) | set(HERE.glob('*.md')))
    guard.verify(packet_pins)
    snapshot = output / 'source-snapshot'
    snapshot.mkdir()
    shutil.copytree(COHORT / 'support-v2', snapshot / 'support-v2')
    pins = preparation.merge(report['pins'], reused['pins'], {str(report_path): preparation.sha(report_path), str(reuse_path): preparation.sha(reuse_path)})
    trace_rows = []
    reviewed_sources = []
    for row in rows:
        source = Path(row['source'])
        assert preparation.sha(source) == row['source_sha256']
        reviewed_sources.append(dict(id=row['id'], source=str(source), source_sha256=row['source_sha256'], review=static_review.review(source.read_text(), row['benchmark'], row['variant'])))
        if row['benchmark'] not in {'binarytrees', 'knucleotide', 'pidigits'}:
            continue
        traced, changes = generate_traces.instrument(source.read_text(), row['benchmark'], row['variant'])
        trace_source = snapshot / 'sources' / row['benchmark'] / row['variant'] / 'main.pony'
        trace_source.parent.mkdir(parents=True)
        trace_source.write_text(traced)
        changes_path = output / 'lineage' / (row['id'] + '.json')
        new(changes_path, dict(source=str(source), source_sha256=row['source_sha256'],
                              trace_source=str(trace_source), trace_source_sha256=preparation.sha(trace_source),
                              changes=changes, diagnostic_header=generate_traces.TRACE))
        trace_rows.append(dict(row, trace_source=str(trace_source), lineage=str(changes_path)))
    for source in packet:
        destination = snapshot / 'scripts' / source.name
        destination.parent.mkdir(exist_ok=True)
        shutil.copyfile(source, destination)
    pins = preparation.merge(pins, preparation.pin_tree(snapshot), preparation.pin_tree(output / 'lineage'), preparation.pin_tree(output / 'fixtures'),
                             packet_pins)
    fixtures = {}
    for benchmark in ('knucleotide', 'regexredux', 'revcomp'):
        if benchmark != 'knucleotide':
            continue
        if benchmark == 'knucleotide':
            data = dict(references.dna_fixtures())['arbitrary-widths']
        elif benchmark == 'regexredux':
            data = dict(preparation.inputs(benchmark, references))['all-replacements']
        else:
            data = dict(preparation.inputs(benchmark, references))['iupac']
        path = output / 'inputs' / (benchmark + '.fasta')
        path.parent.mkdir(exist_ok=True)
        path.write_bytes(data)
        fixtures[benchmark] = str(path)
        pins[str(path)] = preparation.sha(path)
    guard.verify(pins)
    manifest = dict(status='prepared-awaiting-root-review', pins=pins, rows=trace_rows, qualification=str(report_path), toolchain=toolchain,
                    qualification_sha256=preparation.sha(report_path), source_work_profiles=20, reviewed_sources=reviewed_sources, source_work_trace_profiles=6, numeric_inputs=NUMERIC,
                    fixtures=fixtures, shared_lock=str(LOCK), diagnostic_native_apis=['write stderr', 'pthread_self', 'exit'],
                    prior_work_reuse=dict(path=str(reuse_path), sha256=preparation.sha(reuse_path)),
                    reused_work_trace_ids=[row['id'] for row in reused['attempts']],
                    reused_actual_work_traces=2, new_actual_work_traces_required=4,
                    reused_fixture_tests=14, fresh_fixture_tests=0,
                    observer_repair_revision='work-full-v3-isolated-map-observer',
                    workload_code_delegated_to_native=False, production_operations_preserved=True,
                    finalizers='Observed real Node GC finalizers; nondeterministic count retained, bounded by actual allocation count.',
                    stdout_policy='Complete independent expected output must still match the production diagnostic copy.',
                    full_performance_runs=0, new_native_cases_required=4, source_metric='qualified canonical LOC applied separately')
    new(output / 'manifest.json', manifest)
    print(json.dumps(dict(manifest=str(output / 'manifest.json'), sha256=preparation.sha(output / 'manifest.json')), indent=2), flush=True)

def traced_execution(guard, harness, command, environment, directory, lock_fd, stdin):
    """Arm monitoring only after bounded service owns/journals exact Popen."""
    monitor = harness.compact.CompilerMonitor(harness.REQUIRED)
    resource = None
    resource_started = False
    native = None
    input_stream = Path(stdin).open('rb') if stdin else None
    def register(process):
        nonlocal resource, resource_started, native
        native = dict(pid=process.pid, pgid=process.pid, command=command, inherited_common_mutex=True,
                      observer_registered_after_owned_active_journal=True)
        resource = harness.ResourceGuard(process.pid, 10240)
        resource.start()
        resource_started = True
    power_start = harness.compact.power_profile()
    monitor.start()
    try:
        result = preparation.owned_command(guard, harness, command, environment, directory, lock_fd, 300, 'trace-native',
                                           on_spawn=register, stdin=input_stream)
    finally:
        if input_stream: input_stream.close()
        if resource and (resource_started or resource.thread.ident is not None): resource.finish()
        monitor.finish()
    observations = dict(power_at_start=power_start, power_at_end=harness.compact.power_profile(),
                        competing_compilers=monitor.observations, power_changes=monitor.power_changes,
                        monitor_errors=monitor.errors, resource_monitor_errors=resource.errors if resource else ['missing native'],
                        resource_violation=resource.violation if resource else 'missing native',
                        peak_process_group_rss_kib=resource.peak if resource else None, native=native)
    new(directory / 'observations.json', observations)
    assert native and native['inherited_common_mutex']
    assert observations['power_at_start'] == observations['power_at_end'] == harness.REQUIRED
    assert not any(observations[field] for field in ('competing_compilers', 'power_changes', 'monitor_errors', 'resource_monitor_errors', 'resource_violation'))
    return result, observations


@contextmanager
def metadata_terminal(revision, lock):
    """Exclusive metadata outcome; no rewrite of a prior work directory."""
    descriptor, named = os.fstat(lock.fileno()), LOCK.stat()
    assert (descriptor.st_dev, descriptor.st_ino) == (named.st_dev, named.st_ino)
    outcome = COHORT / 'work-seal-outcomes' / revision
    assert not outcome.exists(), 'Exclusive metadata attempt; preserve earlier outcomes'
    outcome.mkdir(parents=True)
    state = dict(status='failed-stop', revision=revision, actual_common_mutex_held=True,
                 common_mutex_fd=lock.fileno(), actual_native_subjects=0, chart_admission=False)
    try:
        yield
        state['status'] = 'completed-metadata-awaiting-root-review'
    except BaseException as error:
        state.update(error=str(error) or type(error).__name__, error_type=type(error).__name__)
        (outcome / 'actual-error.log').write_text(traceback.format_exc())
        raise
    finally:
        # Failed prerequisite/import/copy/pin stages may leave partial bytes.
        # Keep them and any available raw exception text inside the held mutex.
        output = COHORT / 'work-qualification' / revision
        try:
            state['available_evidence_pins'] = preparation.merge(preparation.pin_tree(outcome), preparation.pin_tree(output))
        except BaseException as error:
            state.update(status='failed-stop', terminal_integrity_error=dict(
                error=str(error) or type(error).__name__, error_type=type(error).__name__,
                available_bytes_not_fully_pinned=True))
            new(outcome / 'receipt.json', state)
            raise
        else:
            new(outcome / 'receipt.json', state)


@contextmanager
def native_terminal(output, report, guard):
    """Retain failed compiler/native raw bytes before releasing the same mutex."""
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        try:
            yield lock
        except BaseException as error:
            report.update(status='stopped', error=str(error) or type(error).__name__, error_type=type(error).__name__)
            raise
        finally:
            try:
                raw = preparation.pin_tree(output)
                raw.pop(str((output / 'report.json').resolve()), None)
                report['pins'] = preparation.merge(report.get('pins', {}), raw)
                guard.verify(report['pins'])
            except BaseException as error:
                report.update(status='stopped', terminal_integrity_error=dict(
                    error=str(error) or type(error).__name__, error_type=type(error).__name__,
                    available_pins_are_not_qualified=True))
                guard.save(output / 'report.json', report)
                raise
            else:
                guard.save(output / 'report.json', report)


def run(revision, digest):
    global preparation
    output = COHORT / 'work-qualification' / revision
    manifest_path = output / 'manifest.json'
    with (output / 'owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        assert not (output / 'report.json').exists(), 'Work attempts are never automatically resumed or overwritten'
        with LOCK.open('a+') as initial_lock:
            fcntl.flock(initial_lock, fcntl.LOCK_EX)
            guard = preparation.load_guard()
            assert preparation.sha(manifest_path) == digest
            initial_manifest = json.loads(manifest_path.read_text())
            guard.verify(initial_manifest['pins'])
            load_packet_helpers(guard, initial_manifest['pins'])
        report = dict(status='running', attempts=[], manifest=str(manifest_path), manifest_sha256=digest, full_performance_runs=0,
                      observer_repair_revision='work-full-v3-isolated-map-observer',
                      reused_actual_work_traces=2, fresh_actual_work_traces=0,
                      old_attempts_relabelled_as_new=False)
        guard.save(output / 'report.json', report)
        with native_terminal(output, report, guard) as lock:
            assert preparation.sha(manifest_path) == digest
            manifest = json.loads(manifest_path.read_text())
            guard.verify(manifest['pins'])
            prerequisite = Path(manifest['qualification'])
            assert preparation.sha(prerequisite) == manifest['qualification_sha256']
            qualified_report = json.loads(prerequisite.read_text())
            import preparation_service
            assert Path(preparation_service.__file__).resolve() == HERE / 'preparation_service.py'
            preparation = preparation_service.bind(original_preparation, guard, qualified_report['pins'], manifest['pins'])
            harness = guard.load_harness(); harness.install_interrupt_handlers()
            reused_path = Path(manifest['prior_work_reuse']['path'])
            assert preparation.sha(reused_path) == manifest['prior_work_reuse']['sha256']
            reused = json.loads(reused_path.read_text())
            actual_reuse = reuse_work_v2.reconstruct(preparation, guard, harness, generate_traces, check_traces, references, manifest['pins'])
            assert reused == actual_reuse
            assert manifest['reused_work_trace_ids'] == [row['id'] for row in reused['attempts']]
            report['attempts'] = reused['attempts']
            report['prior_work_reuse'] = manifest['prior_work_reuse']
            report['pins'] = manifest['pins']
            guard.save(output / 'report.json', report)
            qualification = Path(manifest['qualification']).parent.name
            toolchain = manifest['toolchain']
            compiler = Path(toolchain['compiler'])
            regex = Path(toolchain['regex_root']); gmp = Path(toolchain['gmp_root'])
            sdk = Path(toolchain['sdk'])
            pins = dict(manifest['pins'])
            for row in manifest['rows']:
                if row['id'] in manifest['reused_work_trace_ids']:
                    assert row['benchmark'] == 'pidigits'
                    assert preparation.sha(row['trace_source']) == next(item['trace_source_sha256'] for item in reused['attempts'] if item['id'] == row['id'])
                    continue
                # Fresh copies use the same release options and exact standard packages.
                identifier = row['id'] + '-trace'
                destination = output / 'bin' / identifier
                destination.mkdir(parents=True)
                binary = destination / identifier
                environment = dict(os.environ, SDKROOT=str(sdk), PATH='/usr/bin:/bin', **row['runtime_environment'])
                for key in ('PONY_PATH', 'PONYPATH', 'PONYC_OPTIONS', 'CFLAGS', 'CPPFLAGS', 'LDFLAGS', 'DYLD_INSERT_LIBRARIES', 'DYLD_FRAMEWORK_PATH', 'DYLD_FALLBACK_LIBRARY_PATH'):
                    environment.pop(key, None)
                command = preparation.build_command(compiler, sdk, regex, gmp, Path(row['trace_source']), destination, identifier)
                preparation.owned_command(guard, harness, command, environment, output / 'builds' / identifier, lock.fileno(), 600, 'trace-build')
                pins = preparation.merge(pins, preparation.pin_tree(destination), preparation.native_images(guard, harness, binary, environment, output / 'images' / identifier, lock.fileno(), [Path(toolchain['release_root']), preparation.GMP, preparation.PCRE]))
                benchmark = row['benchmark']; n = NUMERIC.get(benchmark, 1000)
                stdin = manifest['fixtures'].get(benchmark)
                data = Path(stdin).read_bytes() if stdin else None
                directory = output / 'executions' / identifier
                result, observations = traced_execution(guard, harness, [str(binary), str(n), '--ponymaxthreads', '8'], environment, directory, lock.fileno(), stdin)
                expected = getattr(references, benchmark)(data if data is not None else n)
                if benchmark == 'nbody':
                    harness.compact.common.validate(result['stdout'].decode(), [float(line) for line in expected.splitlines()])
                else:
                    assert result['stdout'] == expected, ('Complete trace-copy output', identifier)
                proof = check_traces.check(benchmark, result['stderr'], n, data, references, row['variant'])
                # Mutation checks use actual collected work records: missing work and
                # an out-of-cap scheduler identity must both fail the verifier.
                trace_lines = result['stderr'].splitlines(keepends=True)
                trace_index = next(index for index, line in enumerate(trace_lines) if not line.startswith(b'THREAD\t') and not line.startswith(b'FINAL\t'))
                missing = b''.join(trace_lines[:trace_index] + trace_lines[trace_index + 1:])
                out_of_cap = result['stderr'] + b''.join(b'THREAD\t' + str(index).encode() + b'\n' for index in range(1001, 1010))
                for case, mutated in [('missing-work-record', missing), ('too-many-scheduler-identities', out_of_cap)]:
                    try:
                        check_traces.check(benchmark, mutated, n, data, references, row['variant'])
                    except (AssertionError, KeyError, IndexError):
                        pass
                    else:
                        raise AssertionError(('Verifier accepted invalid actual-work mutation', identifier, case))
                proof['actual_record_mutation_rejections'] = ['missing-work-record', 'too-many-scheduler-identities']
                pins = preparation.merge(pins, preparation.pin_tree(output / 'builds' / identifier),
                                         preparation.pin_tree(output / 'images' / identifier), preparation.pin_tree(directory))
                guard.verify(pins)
                report['attempts'].append(dict(id=row['id'], benchmark=benchmark, source=row['source'], source_sha256=row['source_sha256'],
                                                trace_source=row['trace_source'], trace_source_sha256=preparation.sha(row['trace_source']),
                                                binary_sha256=preparation.sha(binary), input_n=n, stdin=stdin, complete_output_match=True,
                                                trace=str(directory / 'trace-native.stderr'), trace_sha256=preparation.sha(directory / 'trace-native.stderr'),
                                                proof=proof, observations=observations, qualified=True))
                report['fresh_actual_work_traces'] += 1
                guard.save(output / 'report.json', report)
            assert len(report['attempts']) == 6
            assert report['fresh_actual_work_traces'] == 4 and len({row['id'] for row in report['attempts']}) == 6
            allocation_module = HERE / 'allocation_evidence.py'
            allocation_pins_before = {str(allocation_module): manifest['pins'][str(allocation_module)]}
            guard.verify(allocation_pins_before)
            assert 'allocation_evidence' not in sys.modules, 'Unreviewed cached mutable allocation evidence helper'
            import allocation_evidence
            assert Path(allocation_evidence.__file__).resolve() == allocation_module
            guard.verify(allocation_pins_before)
            raw_allocation, allocation_pins = allocation_evidence.emit(preparation, guard, harness,
                manifest['qualification'], output / 'scored-tree-allocation', lock.fileno())
            pins = preparation.merge(pins, allocation_pins)
            report['scored_tree_raw_allocation_evidence'] = raw_allocation
            report.update(status='native-work-v2-passed-awaiting-scored-binary-allocation-review', pins=pins, source_work_profiles=20, source_work_trace_profiles=6, reviewed_sources=manifest['reviewed_sources'], unchanged_scored_binary_allocation_review_required=True, scheduler_gate='Production actor counts and maxthreads8 checked in source review; actual bounded native thread identities from six production trace copies and standard seven-actor ownership probe.')

def main():
    parser = argparse.ArgumentParser()
    modes = parser.add_mutually_exclusive_group(required=True)
    modes.add_argument('--seal', action='store_true'); modes.add_argument('--run', action='store_true')
    parser.add_argument('--revision', required=True); parser.add_argument('--qualification', default='private-v7')
    parser.add_argument('--manifest-sha256')
    args = parser.parse_args()
    assert args.revision and args.revision.replace('-', '').isalnum()
    if args.seal: seal(args.revision, args.qualification)
    else:
        assert args.manifest_sha256
        run(args.revision, args.manifest_sha256)

if __name__ == '__main__':
    main()
