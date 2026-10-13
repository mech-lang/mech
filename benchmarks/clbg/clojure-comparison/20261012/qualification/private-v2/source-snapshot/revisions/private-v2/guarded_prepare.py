"""Additive Clojure two-parenthesis repair and exact24-case continuation.

No download/bootstrap rerun, original source mutation or implicit native retry.
Metadata fixtures/reuse hashes and fresh16 compiler/native work own one mutex.
"""
import argparse
from contextlib import contextmanager
import copy
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import signal
import sys
import traceback

HERE = Path(__file__).resolve().parents[2]
REVISION_SOURCE = Path(__file__).resolve().parent
CLBG = HERE.parents[1]
LOCK = Path('/tmp/mech-clbg-timing.lock')
ORIGINAL = HERE / 'revisions/private-v1/guarded_prepare.py'
SERVICE = CLBG / 'c3-comparison/20261012/revisions/private-v1/guarded_prepare.py'
SERVICE_FIXTURES = SERVICE.with_name('fixture_runner.py')
REFERENCE = SERVICE.with_name('references.py')
DNA_REFERENCE = CLBG / 'v-report-only/20261012/dna_oracles.py'
JDK_VERSION, CLOJURE_VERSION = '25.0.4.1+1', '1.12.6'
JDK_ARCHIVE = 'OpenJDK25U-jdk_aarch64_mac_hotspot_25.0.4.1_1.tar.gz'
ARTIFACTS = [('clojure', '1.12.6'), ('spec.alpha', '0.5.238'), ('core.specs.alpha', '0.4.74')]
BENCHMARKS = {'binarytrees', 'fannkuchredux', 'nbody', 'spectralnorm', 'mandelbrot', 'pidigits', 'fasta', 'revcomp', 'knucleotide', 'regexredux'}
JVM_FLAGS = ['-XX:ActiveProcessorCount=8', '-Xms64m', '-Xmx8g', '-XX:+UseG1GC']
GATES = {'sampler_exited_normally', 'authoritative_native_group_received', 'native_inherited_common_mutex',
    'native_exited_normally', 'no_execution_error', 'no_wrapper_integrity_error', 'no_resource_violation',
    'no_resource_monitor_errors', 'no_compiler_power_monitor_errors', 'no_monitor_finish_error',
    'no_execute_cleanup_error', 'no_remaining_execute_group', 'no_supervisor_cleanup', 'no_supervisor_remaining_group',
    'valid_environment', 'actual_full_workload', 'exact_command_and_group', 'complete_output_reference_match',
    'complete_output_bytes_and_hash', 'pins_unchanged'}


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def module(name, path):
    assert name not in sys.modules, 'Fresh pinned module identity'
    spec = importlib.util.spec_from_file_location(name, path)
    value = importlib.util.module_from_spec(spec)
    sys.modules[name] = value
    spec.loader.exec_module(value)
    assert Path(value.__file__).resolve() == Path(path).resolve()
    return value


def exact_true_checks(attempt):
    checks = attempt['qualification_checks']
    assert type(checks) is dict and set(checks) == GATES and all(type(value) is bool and value is True for value in checks.values())
    assert attempt['qualified'] is True and attempt['full_workload'] is False
    assert type(attempt['exit_status']) is type(attempt['sampler_wait_status']) is int
    assert attempt['exit_status'] == attempt['sampler_wait_status'] == 0 and attempt['pins_unchanged_at_end'] is True


def profiles(value):
    assert value['language'] == 'clojure' and value['compiler_version'] == CLOJURE_VERSION
    rows = value['profiles']
    assert len(rows) == len({row['id'] for row in rows}) == 20
    assert {(row['benchmark'], row['variant']) for row in rows} == {(benchmark, variant) for benchmark in BENCHMARKS for variant in ('compact', 'optimized')}
    for row in rows:
        namespace = 'clbg.' + row['benchmark'] + '.' + row['variant']
        name = 'sources/' + namespace.replace('.', '/') + '.clj'
        if row['benchmark'] == 'nbody':
            name = 'revisions/private-v2/' + name
        assert row['id'] == 'clojure-' + row['benchmark'] + '-' + row['variant']
        assert row['namespace'] == namespace and row['source'] == name
        assert row['compiled_source_bundle'] == [name, 'support/clbg/support.clj']
        assert type(row['maximum_compute_workers']) is int and 1 <= row['maximum_compute_workers'] <= 8
        assert row['loc'] is None and row['eligible_for_chart'] is False
        for item in row['compiled_source_bundle']:
            path = Path(item)
            assert not path.is_absolute() and '..' not in path.parts and path.suffix == '.clj' and (HERE / path).is_file()
    return rows


def controlled_environment(java_home):
    return dict(LC_ALL='C', LANG='C', JAVA_HOME=str(java_home), CLASSPATH='',
        JAVA_TOOL_OPTIONS='', JDK_JAVA_OPTIONS='', _JAVA_OPTIONS='', OMP_NUM_THREADS='1',
        OPENBLAS_NUM_THREADS='1', VECLIB_MAXIMUM_THREADS='1')


def compile_command(java, package, classes, jars, namespace):
    assert re.fullmatch(r'clbg\.[a-z]+\.(compact|optimized)', namespace)
    classpath = os.pathsep.join(map(str, [package / 'sources', package / 'support', *jars]))
    expression = '(binding [*compile-path* ' + json.dumps(str(classes)) + ' *compiler-options* {:direct-linking true}] (compile \'clbg.support) (compile \'' + namespace + '))'
    return [str(java), *JVM_FLAGS, '-cp', classpath, 'clojure.main', '-e', expression]


def runtime_command(java, classes, jars, namespace, n='{n}'):
    assert re.fullmatch(r'clbg\.[a-z]+\.(compact|optimized)', namespace)
    assert not any(path.name in ('sources', 'support') for path in [classes, *jars])
    return [str(java), *JVM_FLAGS, '-cp', os.pathsep.join(map(str, [classes, *jars])), namespace, str(n)]


def input_cases(service, references, benchmark):
    cases = tuple(service.inputs(benchmark, references))
    return cases + (('preheader', b'ACGT\n;comment\r\n>one\rA\n>empty'),) if benchmark == 'revcomp' else cases


def inventory(service, references):
    result = {}
    for benchmark in sorted(BENCHMARKS):
        cases = [('n-' + str(n), None) for n in service.NUMERIC[benchmark]] if benchmark in service.NUMERIC else input_cases(service, references, benchmark)
        labels = [name for name, data in cases]
        assert labels and len(labels) == len(set(labels)) and all(re.fullmatch(r'[a-z0-9][a-z0-9-]*', name) for name in labels)
        assert all(data is None or type(data) is bytes for name, data in cases)
        result[benchmark] = labels
    return result


def static_paths():
    rows = profiles(json.loads((REVISION_SOURCE / 'profiles.json').read_text()))
    return sorted({HERE / name for row in rows for name in row['compiled_source_bundle']} |
        set(REVISION_SOURCE.glob('*.py')) | set(REVISION_SOURCE.glob('*.md')) | {REVISION_SOURCE / 'profiles.json', ORIGINAL})


def new_json(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


@contextmanager
def operation(mode, revision):
    with (HERE / 'preparation-owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        print('WAIT common mutex: Clojure additive', mode, revision, flush=True)
        with LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            named, opened = LOCK.stat(), os.fstat(lock.fileno())
            assert (named.st_dev, named.st_ino) == (opened.st_dev, opened.st_ino)
            journal = HERE / 'continuation-outcomes/private-v2' / mode / revision
            assert not journal.exists(), 'Exclusive terminal revision, no retry'
            journal.mkdir(parents=True)
            outcome = dict(status='failed', mode=mode, revision=revision, common_mutex_held=True,
                shared_timing_lock=str(LOCK), full_performance_runs=0, chart_admission=False)
            new_json(journal / 'started.json', dict(outcome, argv=sys.argv, inherited_fd=lock.fileno()))
            try:
                yield lock
                outcome['status'] = 'completed'
            except BaseException as error:
                outcome.update(error=str(error) or type(error).__name__, error_type=type(error).__name__)
                (journal / 'actual-error.log').write_text(traceback.format_exc())
                raise
            finally:
                stage = HERE / ('proposal-seals' if mode == 'seal' else 'qualification') / revision
                outcome['actual_terminal_evidence_pins'] = {str(path): sha(path) for root in (stage, journal)
                    if root.exists() for path in root.rglob('*') if path.is_file()}
                new_json(journal / 'outcome.json', outcome)


def dependencies():
    service = module('clojure_v2_frozen_owned_service', SERVICE)
    guard = service.load_guard()
    harness = guard.load_harness()
    harness.install_interrupt_handlers()
    return service, guard, harness


def seal(proposal_file):
    with operation('seal', proposal_file.stem) as lock:
        assert not proposal_file.exists()
        service, guard, harness = dependencies()
        reuse_module = module('clojure_v2_reuse', REVISION_SOURCE / 'proof_reuse.py')
        reuse = reuse_module.reconstruct(sys.modules[__name__], service, guard, harness)
        paths = static_paths()
        source_pins = {str(path): sha(path) for path in paths}
        output = HERE / 'proposal-seals' / proposal_file.stem
        assert not output.exists()
        output.mkdir(parents=True)
        for name, runner, tail, target in (
                ('ownership-fixtures', SERVICE_FIXTURES, ['--set', 'ownership'], 'generic-ownership-inventory.json'),
                ('candidate-fixtures', REVISION_SOURCE / 'fixture_runner.py', [], 'clojure-command-reuse-inventory.json')):
            receipt = output / target
            service.owned_command(guard, harness, [sys.executable, '-B', str(runner), *tail, '--receipt', str(receipt)],
                dict(os.environ, PYTHONPATH=str(runner.parent), CLBG_MUTEX_FD=str(lock.fileno())), output, lock.fileno(), 30, name)
            catalog = module('clojure_v2_' + name.replace('-', '_'), runner)
            value = json.loads(receipt.read_text())
            catalog.validate_receipt(value, 'ownership') if tail else catalog.validate_receipt(value)
        guard.verify(source_pins)
        assert static_paths() == paths
        reference = module('clojure_v2_seal_references', REFERENCE)
        cases = inventory(service, reference)
        first = json.loads(Path(reuse['first_proposal']).read_text())
        assert cases == first['case_inventory']
        proof = output / 'proof-reuse.json'
        new_json(proof, reuse)
        pins = service.merge(reuse['pins'], source_pins, service.pin_tree(output), {str(proof): sha(proof)})
        guard.verify(pins)
        total = sum(len(cases[row['benchmark']]) for row in profiles(json.loads((REVISION_SOURCE / 'profiles.json').read_text())))
        new_json(proposal_file, dict(status='sealed-awaiting-root-review', language='clojure', created_at=guard.now(),
            profiles=profiles(json.loads((REVISION_SOURCE / 'profiles.json').read_text())), snapshot_paths=[str(path) for path in paths],
            pins=pins, case_inventory=cases, actual_fixture_methods=17,
            actual_fixture_receipts=[str(output / 'generic-ownership-inventory.json'), str(output / 'clojure-command-reuse-inventory.json')],
            prior_proof_reuse=str(proof), prior_proof_reuse_sha256=sha(proof), reused_profiles=4, reused_bounded_cases=24,
            fresh_profile_builds=16, fresh_bounded_cases=total-24, intended_total_bounded_cases=total,
            old_cases_relabelled_as_new_attempts=False, compiler_version=CLOJURE_VERSION,
            runtime_flags=JVM_FLAGS, reused_runtime_receipt=reuse['runtime_receipt'],
            reused_runtime_receipt_sha256=reuse['runtime_receipt_sha256'], shared_timing_lock=str(LOCK),
            guard_sha256=service.GUARD_SHA, executable_payload_downloads=0, runtime_identity_reruns=0,
            full_performance_runs=0, full_processes_authorized=0, chart_admission=False,
            canonical_LOC_admitted=False, no_automatic_retry=True))
        print(json.dumps(dict(proposal=str(proposal_file), sha256=sha(proposal_file), fresh_builds=16, reused_cases=24, fresh_cases=total-24)), flush=True)


def prepare(proposal_file, digest, revision):
    with operation('prepare', revision) as lock:
        assert sha(proposal_file) == digest
        proposal = json.loads(proposal_file.read_text())
        assert proposal['status'] == 'sealed-awaiting-root-review' and proposal['no_automatic_retry'] is True
        for path, pin in proposal['pins'].items():
            assert sha(path) == pin
        service, guard, harness = dependencies()
        reuse_module = module('clojure_v2_native_reuse', REVISION_SOURCE / 'proof_reuse.py')
        proof = Path(proposal['prior_proof_reuse'])
        assert sha(proof) == proposal['prior_proof_reuse_sha256']
        reuse = json.loads(proof.read_text())
        assert reuse_module.reconstruct(sys.modules[__name__], service, guard, harness) == reuse
        output, build = HERE / 'qualification' / revision, HERE / '.build' / revision
        assert not output.exists() and not build.exists(), 'Fresh additive revision, preserve old failed roots'
        output.mkdir(parents=True); build.mkdir(parents=True)
        guard.OUT, guard.REPORT = output, output / 'report.json'
        pins = service.merge(proposal['pins'], {str(proposal_file): digest})
        rows = list(copy.deepcopy(reuse['profiles']).values())
        report = dict(status='preparing', started_at=guard.now(), proposal_sha256=digest,
            attempts=copy.deepcopy(reuse['attempts']), profiles=rows, pins=pins, active=None,
            prior_proof_reuse=str(proof), prior_proof_reuse_sha256=proposal['prior_proof_reuse_sha256'],
            reused_bounded_cases=24, old_cases_relabelled_as_new_attempts=False, fresh_native_attempts=0,
            full_performance_runs=0, full_manifests_sealed=0, chart_admission=False,
            canonical_LOC_admitted=False, no_automatic_retry=True)
        guard.save(guard.REPORT, report)
        try:
            runtime = reuse['runtime']
            java_home, java = Path(runtime['java_home']), Path(runtime['java'])
            jars = [Path(entry['path']) for entry in runtime['vendor_jars']]
            environment = dict(os.environ, **controlled_environment(java_home))
            runtime_path = output / 'runtime-receipt.json'
            shutil.copyfile(reuse['runtime_receipt'], runtime_path)
            assert sha(runtime_path) == reuse['runtime_receipt_sha256']
            report['runtime_receipt_origin'] = reuse['runtime_receipt']
            snapshot = output / 'source-snapshot'
            for name in proposal['snapshot_paths']:
                source = Path(name); target = snapshot / source.relative_to(HERE)
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(source, target)
                assert sha(target) == proposal['pins'][name]
            pins = service.merge(pins, service.pin_tree(snapshot), {str(runtime_path): sha(runtime_path)})
            reference = module('clojure_v2_native_references', REFERENCE)
            dna = module('clojure_v2_native_dna', DNA_REFERENCE)
            reference.knucleotide = dna.knucleotide
            assert inventory(service, reference) == proposal['case_inventory']
            for declared in proposal['profiles']:
                if declared['id'] in reuse['profiles']:
                    continue
                package, classes = build / 'packages' / declared['id'], build / 'classes' / declared['id']
                classes.mkdir(parents=True)
                inputs = []
                for index, name in enumerate(declared['compiled_source_bundle']):
                    source = snapshot / name
                    logical = 'sources/' + declared['namespace'].replace('.', '/') + '.clj' if index == 0 else 'support/clbg/support.clj'
                    target = package / logical
                    target.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copyfile(source, target)
                    assert sha(target) == sha(source) == proposal['pins'][str(HERE / name)]
                    inputs.append(dict(path=str(target), sha256=sha(target), origin=str(source),
                        role='application' if index == 0 else 'application-helper'))
                command = compile_command(java, package, classes, jars, declared['namespace'])
                service.owned_command(guard, harness, command, environment, output / 'builds' / declared['id'], lock.fileno(), 180, 'compile')
                assert (classes / (declared['namespace'].replace('.', '/') + '.class')).is_file() and (classes / 'clbg/support__init.class').is_file()
                artifacts = service.pin_tree(classes)
                assert artifacts and all(Path(path).suffix == '.class' for path in artifacts)
                pins = service.merge(pins, service.pin_tree(package), artifacts)
                inventory_path = output / 'builds' / declared['id'] / 'compiler-input-inventory.json'
                new_json(inventory_path, dict(status='compiled-exact-authored-input-inventory', profile=declared['id'],
                    source=inputs[0]['path'], source_sha256=inputs[0]['sha256'], source_bundle=inputs,
                    compiler_jar=str(jars[0]), compiler_sha256=sha(jars[0]), build_command=command,
                    compiled_artifacts=artifacts, runtime_classpath=[str(classes), *map(str, jars)]))
                pins[str(inventory_path)] = sha(inventory_path)
                compile_receipt = output / 'builds' / declared['id'] / 'compile.json'
                row = dict(declared, key=declared['benchmark'] + ':' + declared['id'], language='clojure',
                    source=inputs[0]['path'], source_sha256=inputs[0]['sha256'], source_bundle=inputs,
                    binary=str(java), binary_sha256=sha(java), command=runtime_command(java, classes, jars, declared['namespace']),
                    cwd=str(HERE), runtime_environment=controlled_environment(java_home),
                    compiler_version=CLOJURE_VERSION, runtime_version=JDK_VERSION, measured_architecture='arm64', backend='JVM HotSpot JIT',
                    support=service.merge(artifacts, service.pin_tree(java_home), {str(path): sha(path) for path in jars},
                        {entry['path']: entry['sha256'] for entry in inputs[1:]}),
                    compiled_input_inventory=str(inventory_path), compiled_input_inventory_sha256=sha(inventory_path),
                    compiled_artifacts=artifacts, runtime_classpath=[str(classes), *map(str, jars)],
                    compile_receipt=str(compile_receipt), compile_receipt_sha256=sha(compile_receipt), bounded_origin='fresh-private-v2')
                guard.verify(pins)
                for n in service.NUMERIC.get(row['benchmark'], ()):
                    attempt = service.probe(guard, harness, row, n, None, getattr(reference, row['benchmark'])(n), pins, output, lock.fileno(), report, 'n-' + str(n))
                    exact_true_checks(attempt)
                if row['benchmark'] not in service.NUMERIC:
                    for case, data in input_cases(service, reference, row['benchmark']):
                        attempt = service.probe(guard, harness, row, harness.WORK[row['benchmark']][0], data,
                            getattr(reference, row['benchmark'])(data), pins, output, lock.fileno(), report, case)
                        exact_true_checks(attempt)
                rows.append(row)
                report.update(profiles=rows, fresh_native_attempts=len(report['attempts'])-24)
                guard.save(guard.REPORT, report)
            assert len(rows) == 20 and len(report['attempts']) == proposal['intended_total_bounded_cases']
            assert report['fresh_native_attempts'] == proposal['fresh_bounded_cases']
            for row in rows:
                observed = [attempt for attempt in report['attempts'] if attempt['id'] == row['id']]
                assert {item['case'] for item in observed} == set(proposal['case_inventory'][row['benchmark']])
                assert len(observed) == len({item['case'] for item in observed})
                for attempt in observed:
                    exact_true_checks(attempt)
                    assert attempt['source_sha256'] == row['source_sha256'] and attempt['execution_fingerprint'] == harness.execution_fingerprint(row)
            pins = service.merge(pins, {str(path): sha(path) for path in output.rglob('*') if path.is_file() and path != guard.REPORT})
            guard.verify(pins)
            report.update(status='bounded-output-ownership-passed-awaiting-actual-work-and-full-review',
                pins=pins, pins_unchanged_at_end=True, active=None, finished_at=guard.now(),
                fresh_profile_builds=16, reused_profiles=4, full_manifests_sealed=0)
        except BaseException as error:
            report.update(status='stopped-unqualified', error=str(error) or type(error).__name__, error_type=type(error).__name__, active=None)
            raise
        finally:
            report['pins'] = service.merge(pins, {str(path): sha(path) for path in output.rglob('*') if path.is_file() and path != guard.REPORT})
            guard.save(guard.REPORT, report)
        print(json.dumps(dict(report=str(guard.REPORT), sha256=sha(guard.REPORT), reused_cases=24, fresh_cases=report['fresh_native_attempts'])), flush=True)


def main():
    parser = argparse.ArgumentParser()
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--seal', action='store_true'); mode.add_argument('--prepare', action='store_true')
    parser.add_argument('--proposal-file', type=Path, required=True)
    parser.add_argument('--proposal-sha256'); parser.add_argument('--revision')
    args = parser.parse_args()
    proposal = args.proposal_file.resolve()
    assert proposal.is_relative_to(HERE / 'proposals') and proposal.suffix == '.json'
    def interrupted(actual, frame):
        raise KeyboardInterrupt('Clojure additive preparation interrupted ' + str(actual))
    for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(signum, interrupted)
    if args.seal:
        seal(proposal)
    else:
        assert re.fullmatch(r'[0-9a-f]{64}', args.proposal_sha256 or '')
        assert re.fullmatch(r'[a-z0-9][a-z0-9-]{0,63}', args.revision or '')
        prepare(proposal, args.proposal_sha256, args.revision)


if __name__ == '__main__':
    main()
