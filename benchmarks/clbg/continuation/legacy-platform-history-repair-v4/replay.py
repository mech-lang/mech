"""Five literal routes over frozen v3; existing ten-ID guard remains in charge."""
import argparse
from copy import deepcopy
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
ROOT = CLBG.parents[1]
V3 = HERE.with_name('legacy-platform-history-repair-v3')
DIAG = HERE.with_name('legacy-platform-history-diagnosis-v2') / 'jobs/all133-v1'
OWNER = CLBG / 'continuation/owned-jobs/legacy-platform133-all-diagnosis-v2'
CLOSED = CLBG / 'continuation/jobs/legacy-platform133-all-diagnosis-v2-closed-review/receipt.json'
LOCK = Path('/tmp/mech-clbg-timing.lock')
DIAG_SHA = '61a0a7b0ca269d442eecc8245e285fab1d451f3ee8be8fb2224a2d83174260fb'
BODY_SHA = 'baaafa08e990844b95c1407c3c8d164cf1c439f1e9a04d97d02d66e6e1071940'
OWNER_SHA = '3bac46826dfecd5798dddee58890be6bead66a9414ded7448c70f044757ff4de'
V3_PINS = {
    'replay.py': '1c84d495a88aac6c545904b63bfdf043e2a68cba374c83a14311ecc79a557987',
    'history_repair.py': '654fe3a50ba73a96a1705957ff6a62410c9c4bab34c0817a44410344d0a40e06',
    'test_history_extensions.py': 'f252a635a26367cc9402a70529002834efcfde4651917018199abe059d015c80',
    'REVIEW.md': '0cf4507c2af66937de840288a29ac505c388d1d3dd4650fb0f659acb6b33c6be',
}
FILES = ('replay.py', 'literal_routes.py', 'test_literal_extensions.py', 'REVIEW.md')
PENDING = {
    'binarytrees:binarytrees-julia-3', 'binarytrees:binarytrees-julia-4',
    'fannkuchredux:fannkuchredux-julia-1', 'fannkuchredux:fannkuchredux-julia-2',
    'fasta:fasta-gcc-2-llvm', 'fasta:fasta-julia-3',
    'knucleotide:knucleotide-julia-1', 'knucleotide:knucleotide-julia-6',
    'mandelbrot:mandelbrot-julia-1', 'mandelbrot:mandelbrot-julia-7',
    'revcomp:revcomp-julia-7', 'revcomp:revcomp-julia-9', 'spectralnorm:spectralnorm-julia-4',
}


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def read(path, digest, pins):
    assert sha(path) == digest, ('Changed prerequisite', str(path))
    assert str(path) not in pins or pins[str(path)] == digest
    pins[str(path)] = digest
    return json.loads(Path(path).read_bytes())


def diagnosis(args, pins):
    closed = read(CLOSED, args.closed_review_sha256, pins)
    assert closed['status'] == 'completed-all133-diagnosis-reviewed' and closed['verified_pins'] == 384
    assert closed['diagnosis_sha256'] == DIAG_SHA and closed['body_terminal_sha256'] == BODY_SHA and closed['durable_terminal_sha256'] == OWNER_SHA
    assert closed['counts'] == {'resolved-v3-measurement':115, 'missing-v3-history':13, 'v3-mapping-error':5}
    done = read(OWNER / 'terminal.json', OWNER_SHA, pins)
    body = read(DIAG / 'terminal.json', BODY_SHA, pins)
    assert done['status'] == 'command-exited-zero-awaiting-report-review' and type(done['exit_code']) is int and done['exit_code'] == 0
    assert done['actual_command_started'] is True and done['stop_signals'] == []
    assert done['qualification_granted'] is done['automatic_retry'] is False
    assert body['status'] == 'completed-diagnosis' and body['body_error'] is None and body['integrity_errors'] == []
    assert body['available_bytes_not_fully_pinned'] is body['chart_admission'] is False and body['actual_common_mutex_held'] is True
    for name in ('annotations_admitted', 'native_processes', 'fixture_reexecutions', 'runtime_version_reruns', 'benchmark_replays'):
        assert type(body[name]) is int and body[name] == 0
    for path, digest in body['pins'].items():
        assert str(path) not in pins or pins[str(path)] == digest
        assert sha(path) == digest
        pins[str(path)] = digest
    request, launch, worker, started = [read(OWNER / name, sha(OWNER / name), pins)
        for name in ('request.json', 'launch.json', 'worker-started.json', 'command-started.json')]
    command = ['/opt/homebrew/opt/python@3.14/bin/python3.14', '-B', '-u', str(DIAG.parents[1] / 'audit.py'),
        '--worker-sha256', '2f5efde7c684d0fa04406ab80b998cef0b7f43471551c7733916ef0a6d224611',
        '--review-sha256', 'ea9a4cb43c09a051575686ee9cba3780c89f20c94fd29147e98d696785fccc7e',
        '--closed-review-sha256', 'd3f7634e318a3da25d7625467545a9525c877e9175789e1eee13d32b8317518f']
    assert all(item['command'] == command and item['cwd'] == str(ROOT) for item in (request, launch, worker, started))
    assert request['common_mutex_owned_by_target_command'] is worker['common_mutex_owned_by_target_command'] is True
    assert request['qualification_granted'] is request['automatic_retry'] is False
    assert worker['inherited_private_lease'] is worker['detached_session'] is True
    assert worker['worker_identity'] == done['worker_identity'] and launch['worker_identity']['pid'] == worker['worker_identity']['pid'] == 21649
    assert started['command_identity'] == done['command_identity'] and started['command_pid'] == done['command_pid'] == 21654
    assert started['command_identity_observation_missing'] is done['command_identity_observation_missing'] is False
    observed = started['command_identity']['listing'].split(None, 7)
    assert observed[:2] == ['21654', '21649'] and observed[7].split() == command
    value = read(DIAG / 'diagnosis.json', DIAG_SHA, pins)
    assert body['pins'][str(DIAG / 'diagnosis.json')] == DIAG_SHA
    assert value['status'] == 'completed-all133-diagnosis-awaiting-mapping-review' and value['profile_count'] == 133
    assert value['chart_admission'] is False
    for name in ('annotations_admitted', 'native_processes', 'fixture_reexecutions', 'runtime_version_reruns', 'benchmark_replays'):
        assert type(value[name]) is int and value[name] == 0
    rows = value['outcomes']
    assert len(rows) == 133 and {key for key, row in rows.items() if row['status'] == 'missing-v3-history'} == PENDING
    assert sum(row['status'] == 'resolved-v3-measurement' for row in rows.values()) == 115
    assert sum(row['status'] == 'v3-mapping-error' for row in rows.values()) == 5
    assert set(closed['issues']) == {key for key, row in rows.items() if row['status'] != 'resolved-v3-measurement'}
    for key, issue in closed['issues'].items():
        assert all(rows[key][name] == item for name, item in issue.items())
    return value


def preserve_result(core, value, result, reads, output):
    rows, facts = value['outcomes'], result['annotations']
    assert result['profile_count'] == len(facts) == 133 and set(facts) == set(rows)
    assert {key for key, fact in facts.items() if fact['verified'] is False} == PENDING
    assert result['qualified_platform_rows'] == 120
    for key, row in rows.items():
        assert result['measurement_identities'][key] == dict(row['identity'], files=row['files'])
        fact = facts[key]
        if row['status'] == 'missing-v3-history':
            assert fact['key'] == fact['execution'] == 'unknown' and fact['label'] == 'Unclassified'
            assert all(flag is False for flag in fact['confidence'].values())
            assert fact['missing_evidence'] == [row['error']]
            continue
        if row['status'] == 'resolved-v3-measurement':
            before, after = deepcopy(row['backend_diagnostic']), deepcopy(fact)
            old_producer, new_producer = before['evidence'].get('producer_binding'), after['evidence'].get('producer_binding')
            if old_producer is not None:
                assert new_producer is not None
                assert old_producer['source'] == str(DIAG / 'producer-snapshots' / old_producer['archive_sha256'] / 'run_remaining.py')
                assert new_producer['source'] == str(output / 'producer-snapshots' / old_producer['archive_sha256'] / 'run_remaining.py')
                assert new_producer['source_sha256'] == old_producer['source_sha256']
                reads.read(old_producer['source'], old_producer['source_sha256'])
                reads.read(new_producer['source'], old_producer['source_sha256'])
                new_producer['source'] = old_producer['source']
            assert after == before, ('Previously resolved metadata changed', key)
            continue
        proof, evidence = row['literal_route_diagnostic'], fact['evidence']
        assert fact['verified'] is True and fact['key'] == 'native-aot' and fact['execution'] == 'aot'
        assert evidence['measurement_selector'] == proof['saved_selector'] and evidence['actual_sample_array'] == proof['samples']
        assert evidence['command'] == proof['command'] and evidence['actual_native_build_command'] == proof['compile_command']
        assert evidence['source_receipt']['report'] == proof['source_report'] and evidence['source_receipt']['field'] == proof['source_selector']
        assert evidence['historical_executable']['path'] == proof['command'][0] and evidence['historical_executable']['sha256'] == proof['binary_sha256']
    reads.unchanged()


def bind(v3, guard, pins, output, stage, repair, old_extensions, routes, extensions, value):
    v3.bind_original(guard, pins, output, stage, repair, old_extensions)
    fresh, prior = guard.fresh_module, guard.prior_fixture
    def module(name, path, current):
        loaded = fresh(name, path, current)
        if name == 'qualify_history':
            assert value['baseline'] == dict(path=str(guard.BASELINE), sha256=guard.BASELINE_SHA)
            assert {key for key, row in value['outcomes'].items() if row['status'] == 'v3-mapping-error'} == set(routes.ROUTES)
            changes = routes.install(loaded, value['outcomes'])
            qualified = loaded.qualify
            def checked(*args):
                result = qualified(*args)
                preserve_result(loaded, value, result, args[3], args[4])
                return result
            loaded.qualify = checked
            receipt = output / 'literal-route-bindings.json'
            guard.new(receipt, dict(status='five-actual-literal-routes-bound', stage=stage, changes=changes,
                diagnosis=dict(path=str(DIAG / 'diagnosis.json'), sha256=DIAG_SHA),
                source=str(HERE / 'literal_routes.py'), source_sha256=current[str(HERE / 'literal_routes.py')],
                old_115_metadata_preservation_required=True, pending_13_preservation_required=True,
                original_measurement_body_preserved=True))
        elif name == 'test_history':
            methods = extensions.extend(loaded, sys.modules['qualify_history'], routes, value['outcomes'])
            receipt = output / 'literal-fixture-bindings.json'
            guard.new(receipt, dict(status='bound-two-existing-methods-with-five-route-assertions',
                expected_ids=guard.IDS, expected_count=10, methods=methods,
                source=str(HERE / 'test_literal_extensions.py'), source_sha256=current[str(HERE / 'test_literal_extensions.py')],
                full_original_and_v3_methods_executed_first=True, new_fixture_ids=0))
        else:
            return loaded
        guard.merge(current, {str(receipt):sha(receipt)}); guard.verify(current)
        return loaded
    def fixture(args, current):
        prior(args, current)
        path = args.fixture_receipt.resolve(strict=True)
        receipt, terminal = guard.read(path), guard.read(path.parent / 'terminal.json')
        for name in FILES:
            source = str(HERE / name)
            assert receipt['pins'][source] == terminal['pins'][source] == current[source] == pins[source]
        for name in ('literal-route-bindings.json', 'literal-fixture-bindings.json'):
            source = path.parent / name
            assert sha(source) == receipt['pins'][str(source)] == terminal['pins'][str(source)]
            artifact = guard.read(source)
            assert artifact['source_sha256'] == current[artifact['source']]
            if name == 'literal-route-bindings.json':
                assert artifact['status'] == 'five-actual-literal-routes-bound' and artifact['stage'] == 'fixtures'
                assert artifact['diagnosis'] == dict(path=str(DIAG / 'diagnosis.json'), sha256=DIAG_SHA)
            else:
                assert artifact['status'] == 'bound-two-existing-methods-with-five-route-assertions'
                assert artifact['expected_ids'] == guard.IDS and artifact['expected_count'] == 10 and artifact['new_fixture_ids'] == 0
                assert artifact['full_original_and_v3_methods_executed_first'] is True
                assert set(artifact['methods']) == {'test_duplicate_or_wrong_language_record_is_never_a_sample_join', 'test_native_link_requires_exact_output_and_cargo_artifact_pin'}
                assert all(item['prior_method_source'] == str(V3 / 'test_history_extensions.py') and item['extension_method_source'] == item['extra_assertion_source'] == str(HERE / 'test_literal_extensions.py') and item['full_original_and_v3_methods_executed_first'] is True for item in artifact['methods'].values())
        guard.verify(current)
    guard.fresh_module, guard.prior_fixture = module, fixture


def main():
    parser = argparse.ArgumentParser()
    for name in ('worker', 'routes', 'tests', 'review', 'closed-review'):
        parser.add_argument('--' + name + '-sha256', required=True)
    own, remaining = parser.parse_known_args()
    stage_parser = argparse.ArgumentParser()
    stage_parser.add_argument('--stage', choices=('fixtures', 'qualify'), required=True)
    stage_parser.add_argument('--revision', required=True)
    stage, _ = stage_parser.parse_known_args(remaining)
    assert re.fullmatch(r'[a-z0-9][a-z0-9-]{0,63}', stage.revision)
    assert sys.dont_write_bytecode and sys.version_info[:3] == (3, 14, 8)
    print('WAIT common mutex: five exact historical routes; metadata/fixtures only', flush=True)
    with (OWNER / 'owner.lock').open('r') as lease:
        with LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            held, named = os.fstat(lock.fileno()), LOCK.stat()
            assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino) and held.st_ino == 79859134
            fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
            held, named = os.fstat(lease.fileno()), (OWNER / 'owner.lock').stat()
            assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino)
            output = HERE / 'jobs' / stage.revision
            output.mkdir(parents=True, exist_ok=False)
            pins, failure, v3 = {}, None, None
            try:
                for name, digest in zip(FILES, (own.worker_sha256, own.routes_sha256, own.tests_sha256, own.review_sha256)):
                    path = HERE / name
                    assert not (path.stat().st_mode & 0o222) and sha(path) == digest
                    pins[str(path)] = digest
                value = diagnosis(own, pins)
                for name, digest in V3_PINS.items():
                    path = V3 / name
                    assert not (path.stat().st_mode & 0o222) and sha(path) == digest
                    pins[str(path)] = digest
                name, path = 'literal_routes_frozen_v3_entry', V3 / 'replay.py'
                assert name not in sys.modules
                spec = importlib.util.spec_from_file_location(name, path)
                v3 = importlib.util.module_from_spec(spec); sys.modules[name] = v3
                source = path.read_bytes(); assert hashlib.sha256(source).hexdigest() == V3_PINS['replay.py']
                exec(compile(source, str(path), 'exec'), v3.__dict__)
                assert Path(v3.__file__).resolve() == path and sys.modules[name] is v3
                routes = v3.cold_module('legacy133_literal_five_routes', HERE / 'literal_routes.py', pins)
                extensions = v3.cold_module('legacy133_literal_five_assertions', HERE / 'test_literal_extensions.py', pins)
                v3.bind_original = v3.bind
                def augmented(guard, current, target, mode, repair, old_extensions):
                    v3.merge(current, pins)
                    bind(v3, guard, current, target, mode, repair, old_extensions, routes, extensions, value)
                v3.bind = augmented
                v3.verify(pins)
            except BaseException as error:
                failure = dict(error=str(error) or type(error).__name__, error_type=type(error).__name__)
                (output / 'terminal-error.log').write_text(traceback.format_exc())
                raise
            finally:
                captured, errors = {}, []
                candidates = dict(pins)
                candidates.update({str(item):None for item in output.rglob('*') if item.is_file()})
                for path, expected in candidates.items():
                    try:
                        actual = sha(path); assert expected is None or actual == expected
                        captured[path] = actual
                    except BaseException as error:
                        errors.append(dict(path=path, error=str(error) or type(error).__name__))
                with (output / 'terminal.json').open('x') as stream:
                    json.dump(dict(status='failed-terminal-integrity' if errors else 'failed-preflight' if failure else 'prepared-for-preserved-v3-guard',
                        pins=captured, body_error=failure, integrity_errors=errors, actual_common_mutex_held=True,
                        available_bytes_not_fully_pinned=bool(errors), native_processes=0, benchmark_replays=0, chart_admission=False), stream, indent=2)
                    stream.write('\n')
                if errors: raise RuntimeError('Preflight terminal integrity failed; partial evidence retained')
                pins[str(output / 'terminal.json')] = sha(output / 'terminal.json')
        # The preserved entry owns common again; keep only the closed diagnosis
        # private lease. It writes fresh v3-preflight and original-guard outputs.
        sys.argv = [str(V3 / 'replay.py'), *remaining]
        v3.main()


if __name__ == '__main__':
    main()
