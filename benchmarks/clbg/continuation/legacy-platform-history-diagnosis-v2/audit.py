"""Collect every preserved legacy133 mapping outcome; never qualify or retime."""
import argparse
from copy import deepcopy
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import sys
from types import SimpleNamespace
import traceback

HERE = Path(__file__).resolve().parent
CLBG = HERE.parents[1]
ROOT = CLBG.parents[1]
PACKET = CLBG / 'continuation/existing-native-platform-proposal-v1'
V3 = CLBG / 'continuation/legacy-platform-history-repair-v3'
OWNER = CLBG / 'continuation/owned-jobs/legacy-platform133-source-bound-v3-qualification'
CLOSED = CLBG / 'continuation/jobs/legacy-platform133-source-bound-v3-qualification-closed-review-v1/receipt.json'
BODY = PACKET / 'jobs/history-source-bound-v3/terminal.json'
BODY_SHA = '8d8a9d485935ba001fc892973560a38e2eaff16b6c18dcdbd63ea32205bcaa58'
OWNER_SHA = 'a5841b98ee3b53cdc5e44f84fa93f2468e8a1f8a9b57d76c38aede737e7b464b'
FIXTURE = PACKET / 'jobs/fixtures-source-bound-v3/fixture-receipt.json'
FIXTURE_SHA = '8ccd484cec74cb2a5c6ac6b720fe8b5bc4ddf3608948c8260a69983b8ea550bc'
FIXTURE_TERMINAL_SHA = '1b82a743b575e6d71467d070ea416519e3313be5a6062182821bcfc727fa9c5e'
V3_PINS = {
    'replay.py': '1c84d495a88aac6c545904b63bfdf043e2a68cba374c83a14311ecc79a557987',
    'history_repair.py': '654fe3a50ba73a96a1705957ff6a62410c9c4bab34c0817a44410344d0a40e06',
    'test_history_extensions.py': 'f252a635a26367cc9402a70529002834efcfde4651917018199abe059d015c80',
    'REVIEW.md': '0cf4507c2af66937de840288a29ac505c388d1d3dd4650fb0f659acb6b33c6be',
}
# Explicit source-read hypotheses only. These never enter record_candidates or
# measurement and cannot create an annotation. No suffix/sample-based search.
LITERALS = {
    ('work-audit/20261009/timings.json', 'fasta:mech-fasta-final-compact-v2'):
        ('fasta:mech-fasta-final-compact-v2:floating', None),
    ('fasta/investigations/20261009-floating-cdf/results.json', 'fasta:mech-fasta-final-expanded-v2'):
        ('mech-current-floating', 'fasta:mech-fasta-final-expanded-v2:floating'),
    ('nbody/experiments/20261011-shared-products/replication-50m.json', 'nbody:compact-baseline'):
        ('compact-regions8', None),
    ('nbody/experiments/20261011-shared-products/replication-50m.json', 'nbody:pairwise-mass-tails'):
        ('pairwise-seed8', None),
    ('fasta/investigations/20261009-floating-cdf/results.json', 'fasta:rust7-floating-16-local'):
        ('rust7-floating-16', None),
}


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def read(path, digest, pins):
    assert sha(path) == digest, ('Changed prerequisite', str(path))
    assert str(path) not in pins or pins[str(path)] == digest
    pins[str(path)] = digest
    return json.loads(Path(path).read_bytes())


def cold(path, digest, name, pins):
    assert name not in sys.modules and sha(path) == digest
    pins[str(path)] = digest
    source = Path(path).read_bytes()
    assert hashlib.sha256(source).hexdigest() == digest
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    exec(compile(source, str(path), 'exec'), module.__dict__)
    assert Path(module.__file__).resolve() == path and sys.modules[name] is module
    return module


def records(report):
    rows = report.get('programs')
    return list(rows.items()) if type(rows) is dict else list(enumerate(rows)) if type(rows) is list else []


def literal_proof(core, history, displayed):
    wanted = LITERALS.get((displayed['report'], displayed['key']))
    if wanted is None:
        return None
    name, build_name = wanted
    report = history.report(displayed['report'])
    found = [(index, row) for index, row in records(report)
             if type(row) is dict and (row.get('key', index) == name or row.get('id') == name)]
    core.require(len(found) == 1, 'Literal diagnostic candidate missing/ambiguous')
    index, saved = found[0]
    core.require(saved.get('benchmark', report.get('benchmark', displayed['benchmark'])) == displayed['benchmark'], 'Literal benchmark differs')
    core.require(saved.get('language', displayed['language']) == displayed['language'], 'Literal language differs')
    core.samples(displayed, saved)
    origin, origin_report, field = saved, displayed['report'], '/programs/' + core.token(index)
    if build_name is not None:
        origin_report = displayed['source_report']
        candidates = [(i, row) for i, row in records(history.report(origin_report))
                      if type(row) is dict and row.get('key') == build_name]
        core.require(len(candidates) == 1, 'Literal diagnostic build missing/ambiguous')
        i, origin = candidates[0]
        field = '/programs/' + core.token(i)
    core.require(core.command(saved.get('command')) and core.digest(saved.get('binary_sha256')), 'Literal lacks recorded timed binary')
    core.require(origin.get('binary', saved['command'][0]) == saved['command'][0]
                 and origin.get('binary_sha256') == saved['binary_sha256'], 'Literal timing/build binary differs')
    history.reads.read(saved['command'][0], saved['binary_sha256'])
    files = history.source_files(displayed, origin)
    recorded_sources = origin.get('source')
    if type(recorded_sources) is str:
        core.require(core.digest(origin.get('source_sha256')), 'Literal lacks recorded source digest')
        history.reads.read(recorded_sources, origin['source_sha256'])
    return dict(status='literal-route-proof-for-review-only', saved_selector='/programs/' + core.token(index),
        saved_id=saved.get('id'), saved_key=saved.get('key'), source_report=origin_report, source_selector=field,
        samples=deepcopy(saved['samples_seconds']), median_seconds=saved.get('median_seconds'),
        command=deepcopy(saved['command']), binary_sha256=saved['binary_sha256'],
        displayed_files=files, recorded_source=deepcopy(recorded_sources), recorded_source_sha256=origin.get('source_sha256'),
        compile_command=core.native_compile(origin, saved['command']), annotation_created=False)


def main():
    parser = argparse.ArgumentParser()
    for name in ('worker', 'review', 'closed-review'):
        parser.add_argument('--' + name + '-sha256', required=True)
    args = parser.parse_args()
    assert sys.dont_write_bytecode and sys.version_info[:3] == (3, 14, 8)
    lockpath = Path('/tmp/mech-clbg-timing.lock')
    print('WAIT common mutex: all133 preserved history diagnosis only', flush=True)
    with lockpath.open('a+') as lock, (OWNER / 'owner.lock').open('r+') as lease:
        fcntl.flock(lock, fcntl.LOCK_EX)
        held, named = os.fstat(lock.fileno()), lockpath.stat()
        assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino) and held.st_ino == 79859134
        fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
        lh, ln = os.fstat(lease.fileno()), (OWNER / 'owner.lock').stat()
        assert (lh.st_dev, lh.st_ino) == (ln.st_dev, ln.st_ino)
        output = HERE / 'jobs/all133-v1'
        output.mkdir(parents=True, exist_ok=False)
        pins, failure = {}, None
        try:
            for path, digest in ((Path(__file__).resolve(), args.worker_sha256), (HERE / 'REVIEW.md', args.review_sha256)):
                assert not (path.stat().st_mode & 0o222) and sha(path) == digest
                pins[str(path)] = digest
            closed = read(CLOSED, args.closed_review_sha256, pins)
            assert closed['status'] == 'closed-qualification-failure-preserved' and closed['verified_pins'] == 207
            assert closed['durable_terminal_sha256'] == OWNER_SHA and closed['body_terminal_sha256'] == BODY_SHA
            done = read(OWNER / 'terminal.json', OWNER_SHA, pins)
            body = read(BODY, BODY_SHA, pins)
            assert done['status'] == 'command-failed' and type(done['exit_code']) is int and done['exit_code'] == 1
            assert done['actual_command_started'] is True and done['stop_signals'] == []
            assert body['status'] == 'failed-stop' and body['integrity_error'] is None and body['available_bytes_not_fully_pinned'] is False
            assert body['body_error'] == closed['failure']
            assert body['exclusive_common_mutex_held'] is True and body['common_timing_mutex'] == str(lockpath) and body['chart_admission'] is False
            assert all(type(body[name]) is int and body[name] == 0 for name in ('actual_native_processes', 'runtime_version_reruns', 'benchmark_replays'))
            for path, digest in body['pins'].items():
                assert str(path) not in pins or pins[str(path)] == digest
                assert sha(path) == digest
                pins[str(path)] = digest
            request, worker, started = [read(OWNER / name, sha(OWNER / name), pins) for name in ('request.json', 'worker-started.json', 'command-started.json')]
            expected = ['/opt/homebrew/opt/python@3.14/bin/python3.14', '-B', '-u', str(V3 / 'replay.py'), '--stage', 'qualify', '--revision', 'history-source-bound-v3', '--fixture-receipt', str(FIXTURE), '--fixture-sha256', FIXTURE_SHA, '--fixture-terminal-sha256', FIXTURE_TERMINAL_SHA]
            assert request['command'] == worker['command'] == started['command'] == expected
            assert request['cwd'] == worker['cwd'] == started['cwd'] == str(ROOT)
            assert request['common_mutex_owned_by_target_command'] is worker['common_mutex_owned_by_target_command'] is True
            assert worker['worker_identity'] == done['worker_identity'] and worker['inherited_private_lease'] is worker['detached_session'] is True
            assert started['command_identity'] == done['command_identity'] and started['command_pid'] == done['command_pid'] == 16556
            assert started['command_identity_observation_missing'] is done['command_identity_observation_missing'] is False
            # The saved process listing names Homebrew's actual Framework
            # process image; request/start.command names its launcher. Bind the
            # exact observed image name and every remaining argument separately.
            observed = started['command_identity']['listing'].split()[7:]
            framework = '/opt/homebrew/Cellar/python@3.14/3.14.8/Frameworks/Python.framework/Versions/3.14/Resources/Python.app/Contents/MacOS/Python'
            assert observed == [framework, *expected[1:]]
            assert worker['worker_identity']['pid'] == 16552 and done['qualification_granted'] is done['automatic_retry'] is False
            for name, digest in V3_PINS.items():
                assert not ((V3 / name).stat().st_mode & 0o222) and sha(V3 / name) == digest
                pins[str(V3 / name)] = digest
            v3 = cold(V3 / 'replay.py', V3_PINS['replay.py'], 'diagnostic_v3_entry', pins)
            v3.prior_failure(pins); v3.prior_history_failure(pins)
            guard = cold(PACKET / 'guarded_qualify.py', v3.ORIGINAL_FILES['guarded_qualify.py'], 'diagnostic_original_guard', pins)
            repair = cold(V3 / 'history_repair.py', V3_PINS['history_repair.py'], 'diagnostic_v3_repair', pins)
            extensions = cold(V3 / 'test_history_extensions.py', V3_PINS['test_history_extensions.py'], 'diagnostic_v3_extensions', pins)
            v3.bind(guard, pins, output, 'diagnose', repair, extensions)
            directory = guard.snapshot(output, pins)
            guard.prior_fixture(SimpleNamespace(fixture_receipt=FIXTURE, fixture_sha256=FIXTURE_SHA, fixture_terminal_sha256=FIXTURE_TERMINAL_SHA), pins)
            assert not any(name in sys.modules for name in ('qualify_history', 'adapter', 'test_history', 'test_adapter'))
            core = guard.fresh_module('qualify_history', directory / 'qualify_history.py', pins)
            prior = read(guard.PRIOR, guard.PRIOR_SHA, pins)
            assert prior['status'] == 'reconciled-with-unresolved-platform-metadata' and prior['pins_unchanged_at_end'] is True
            assert prior['source_pins'][str(guard.BASELINE)] == guard.BASELINE_SHA
            reader = cold(guard.READER, prior['source_pins'][str(guard.READER)], 'diagnostic_original_reader', pins)
            data = read(guard.BASELINE, guard.BASELINE_SHA, pins)
            selected = core.inventory(data, json.loads((directory / 'expected-program-keys.json').read_bytes()))
            reads = reader.PinnedReads(CLBG, pins)
            history = core.History(CLBG, data, reads, output)
            outcomes = {}
            for key, displayed in sorted(selected.items()):
                row = dict(identity={name:deepcopy(displayed.get(name)) for name in core.IDENTITY_FIELDS},
                    files=[dict(path=data['files'][file]['path'], sha256=data['files'][file]['sha256']) for file in displayed['files']])
                try:
                    for item in row['files']: reads.read(CLBG / item['path'], item['sha256'])
                    measured = core.measurement(history, displayed)
                    files = history.source_files(displayed, measured['origin'])
                    fact = core.annotation(history, displayed, measured, files)
                    row.update(status='resolved-v3-measurement' if fact['verified'] else 'unresolved-v3-measurement',
                        measurement_selector=measured['selector'], source_report=measured['source_relative'], source_selector=measured['origin_field'], backend_diagnostic=fact)
                except Exception as error:
                    row.update(status='missing-v3-history' if isinstance(error, core.MissingHistory) else 'v3-mapping-error', error=str(error) or type(error).__name__, error_type=type(error).__name__)
                try:
                    if (displayed['report'], key) in LITERALS:
                        row['literal_route_diagnostic'] = literal_proof(core, history, displayed)
                except Exception as error:
                    row['literal_route_diagnostic'] = dict(status='literal-route-proof-failed', error=str(error) or type(error).__name__, error_type=type(error).__name__)
                outcomes[key] = row
            assert len(outcomes) == 133 and set(outcomes) == set(selected)
            reads.unchanged(); guard.verify(pins)
            guard.new(output / 'diagnosis.json', dict(status='completed-all133-diagnosis-awaiting-mapping-review',
                profile_count=133, outcomes=outcomes, annotations_admitted=0, chart_admission=False,
                baseline=dict(path=str(guard.BASELINE), sha256=guard.BASELINE_SHA),
                native_processes=0, fixture_reexecutions=0, runtime_version_reruns=0, benchmark_replays=0))
        except BaseException as error:
            failure = dict(error=str(error) or type(error).__name__, error_type=type(error).__name__)
            (output / 'terminal-error.log').write_text(traceback.format_exc())
            raise
        finally:
            captured, errors = {}, []
            candidates = dict(pins)
            candidates.update({str(path):None for path in output.rglob('*') if path.is_file()})
            for path, digest in candidates.items():
                try:
                    actual = sha(path)
                    assert digest is None or actual == digest
                    captured[path] = actual
                except BaseException as error:
                    errors.append(dict(path=path, error=str(error) or type(error).__name__))
            with (output / 'terminal.json').open('x') as stream:
                json.dump(dict(status='failed-terminal-integrity' if errors else 'failed-diagnosis' if failure else 'completed-diagnosis',
                    pins=captured, body_error=failure, integrity_errors=errors, available_bytes_not_fully_pinned=bool(errors),
                    actual_common_mutex_held=True, annotations_admitted=0, chart_admission=False, native_processes=0,
                    fixture_reexecutions=0, runtime_version_reruns=0, benchmark_replays=0), stream, indent=2)
                stream.write('\n')
            if errors: raise RuntimeError('Diagnostic terminal integrity failed; partial bytes preserved')
        print(json.dumps(dict(status='completed-diagnosis', output=str(output), diagnosis_sha256=sha(output / 'diagnosis.json'), terminal_sha256=sha(output / 'terminal.json'))), flush=True)


if __name__ == '__main__':
    main()
