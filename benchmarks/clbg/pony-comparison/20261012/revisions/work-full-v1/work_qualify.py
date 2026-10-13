"""Additive production-work trace qualification before complete Pony studies."""
import argparse
import fcntl
import importlib.util
import json
import os
from pathlib import Path
import shutil
import sys
import time

HERE = Path(__file__).resolve().parent
COHORT = HERE.parents[1]
sys.path.insert(0, str(COHORT))
import guarded_prepare_v6 as preparation
import references
import generate_traces
import check_traces
import static_review
LOCK = preparation.LOCK
NUMERIC = dict(binarytrees=6, pidigits=97)


def new(path, value):
    preparation.new_json(path, value)


def seal(revision, qualification):
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        report_path = COHORT / 'qualification' / qualification / 'report.json'
        report = json.loads(report_path.read_text())
        assert report['status'] == 'bounded-output-primitives-passed-awaiting-work-worker-and-LOC-review'
        guard = preparation.load_guard()
        guard.verify(report['pins'])
        rows = report['profiles']
        assert len(rows) == 20 and len({row['id'] for row in rows}) == 20
        output = COHORT / 'work-qualification' / revision
        assert not output.exists(), 'Every work revision is additive and immutable'
        output.mkdir(parents=True)
        harness = guard.load_harness(); harness.install_interrupt_handlers()
        preparation.owned_command(guard, harness, [sys.executable, '-B', str(HERE / 'test_full_study.py')],
                                  dict(os.environ), output / 'fixtures', lock.fileno(), 30, 'full-failure-gates')
        snapshot = output / 'source-snapshot'
        snapshot.mkdir()
        shutil.copytree(COHORT / 'support-v2', snapshot / 'support-v2')
        pins = preparation.merge(report['pins'], {str(report_path): preparation.sha(report_path)})
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
        for source in HERE.glob('*.py'):
            destination = snapshot / 'scripts' / source.name
            destination.parent.mkdir(exist_ok=True)
            shutil.copyfile(source, destination)
        pins = preparation.merge(pins, preparation.pin_tree(snapshot), preparation.pin_tree(output / 'lineage'), preparation.pin_tree(output / 'fixtures'),
                                 {str(path): preparation.sha(path) for path in HERE.glob('*.py')})
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
        manifest = dict(status='prepared-awaiting-root-review', pins=pins, rows=trace_rows, qualification=str(report_path),
                        qualification_sha256=preparation.sha(report_path), source_work_profiles=20, reviewed_sources=reviewed_sources, source_work_trace_profiles=6, numeric_inputs=NUMERIC,
                        fixtures=fixtures, shared_lock=str(LOCK), diagnostic_native_apis=['write stderr', 'pthread_self', 'exit'],
                        workload_code_delegated_to_native=False, production_operations_preserved=True,
                        finalizers='Observed real Node GC finalizers; nondeterministic count retained, bounded by actual allocation count.',
                        stdout_policy='Complete independent expected output must still match the production diagnostic copy.',
                        full_performance_runs=0, new_native_cases_required=6, source_metric='qualified canonical LOC applied separately')
        new(output / 'manifest.json', manifest)
        print(json.dumps(dict(manifest=str(output / 'manifest.json'), sha256=preparation.sha(output / 'manifest.json')), indent=2), flush=True)


def traced_execution(guard, harness, command, environment, directory, lock_fd, stdin):
    """Retain complete diagnostic stderr with the unchanged owned PGID helper."""
    original = preparation.subprocess.Popen
    monitor = harness.compact.CompilerMonitor(harness.REQUIRED)
    resource = None
    native = None
    input_stream = Path(stdin).open('rb') if stdin else None
    def spawn(*args, **kwargs):
        nonlocal resource, native
        argv = args[0] if args else kwargs.get('args')
        selected = list(argv) == command and kwargs.get('start_new_session')
        if selected and input_stream is not None:
            kwargs['stdin'] = input_stream
        process = original(*args, **kwargs)
        if selected:
            native = dict(pid=process.pid, pgid=process.pid, command=command, inherited_common_mutex=lock_fd in kwargs.get('pass_fds', ()))
            try:
                resource = harness.ResourceGuard(process.pid, 10240)
                resource.start()
            except BaseException:
                harness.kill_group(process.pid)
                process.wait()
                raise
        return process
    power_start = harness.compact.power_profile()
    monitor.start()
    preparation.subprocess.Popen = spawn
    try:
        result = preparation.owned_command(guard, harness, command, environment, directory, lock_fd, 300, 'trace-native')
    finally:
        preparation.subprocess.Popen = original
        if input_stream: input_stream.close()
        if resource: resource.finish()
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


def run(revision, digest):
    output = COHORT / 'work-qualification' / revision
    manifest_path = output / 'manifest.json'
    with (output / 'owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        assert not (output / 'report.json').exists(), 'Work attempts are never automatically resumed or overwritten'
        with LOCK.open('a+') as initial_lock:
            fcntl.flock(initial_lock, fcntl.LOCK_EX)
            guard = preparation.load_guard()
        report = dict(status='running', attempts=[], manifest=str(manifest_path), manifest_sha256=digest, full_performance_runs=0)
        guard.save(output / 'report.json', report)
        try:
            with LOCK.open('a+') as lock:
                fcntl.flock(lock, fcntl.LOCK_EX)
                assert preparation.sha(manifest_path) == digest
                manifest = json.loads(manifest_path.read_text())
                guard.verify(manifest['pins'])
                harness = guard.load_harness(); harness.install_interrupt_handlers()
                qualification = Path(manifest['qualification']).parent.name
                build_root = COHORT / '.build' / qualification
                compiler = next((build_root / 'release').glob('*/bin/ponyc'))
                regex = next((build_root / 'regex').glob('*/regex/regex.pony')).parents[1]
                gmp = next((build_root / 'gmp4pony').glob('*/gmp/mpz.pony')).parents[1]
                sdk = Path(json.loads((COHORT / 'proposals' / (qualification + '.json')).read_text())['sdk'])
                pins = dict(manifest['pins'])
                for row in manifest['rows']:
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
                    pins = preparation.merge(pins, preparation.pin_tree(destination), preparation.native_images(guard, harness, binary, environment, output / 'images' / identifier, lock.fileno(), [build_root / 'release', preparation.GMP, preparation.PCRE]))
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
                    proof = check_traces.check(benchmark, result['stderr'], n, data, references)
                    # Mutation checks use actual collected work records: missing work and
                    # an out-of-cap scheduler identity must both fail the verifier.
                    trace_lines = result['stderr'].splitlines(keepends=True)
                    trace_index = next(index for index, line in enumerate(trace_lines) if not line.startswith(b'THREAD\t') and not line.startswith(b'FINAL\t'))
                    missing = b''.join(trace_lines[:trace_index] + trace_lines[trace_index + 1:])
                    out_of_cap = result['stderr'] + b''.join(b'THREAD\t' + str(index).encode() + b'\n' for index in range(1001, 1010))
                    for case, mutated in [('missing-work-record', missing), ('too-many-scheduler-identities', out_of_cap)]:
                        try:
                            check_traces.check(benchmark, mutated, n, data, references)
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
                    guard.save(output / 'report.json', report)
                assert len(report['attempts']) == 6
                report.update(status='complete-production-work-qualified', pins=pins, source_work_profiles=20, source_work_trace_profiles=6, reviewed_sources=manifest['reviewed_sources'], scheduler_gate='Production actor counts and maxthreads8 checked in source review; actual bounded native thread identities from six production trace copies and standard seven-actor ownership probe.')
        except BaseException as error:
            report.update(status='stopped', error=str(error) or type(error).__name__, error_type=type(error).__name__)
            raise
        finally:
            guard.save(output / 'report.json', report)


def main():
    parser = argparse.ArgumentParser()
    modes = parser.add_mutually_exclusive_group(required=True)
    modes.add_argument('--seal', action='store_true'); modes.add_argument('--run', action='store_true')
    parser.add_argument('--revision', required=True); parser.add_argument('--qualification', default='private-v6')
    parser.add_argument('--manifest-sha256')
    args = parser.parse_args()
    assert args.revision and args.revision.replace('-', '').isalnum()
    if args.seal: seal(args.revision, args.qualification)
    else:
        assert args.manifest_sha256
        run(args.revision, args.manifest_sha256)

if __name__ == '__main__':
    main()
