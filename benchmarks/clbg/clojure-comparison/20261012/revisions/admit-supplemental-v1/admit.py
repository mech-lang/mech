"""Admit the reviewed Clojure v2 supplement through the frozen v1 validator.

Metadata only. One original owner lease and one common mutex cover all hashes,
imports, original admission and final rechecks. No native commands or retries.
"""
import argparse
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import signal
import sys
import traceback

HERE = Path(__file__).resolve().parents[2]
SOURCE = Path(__file__).resolve()
LOCK = Path('/tmp/mech-clbg-timing.lock')
PACKET = HERE / 'revisions/work-full-v1'
SNAPSHOT = HERE / 'work-qualification/proposal-seals/work-v1/snapshot'
ACTUAL = HERE / 'work-qualification/jobs/work-v1/report.json'
ACTUAL_SHA = 'e193dfcebfada205469caef3f2b80c8cdffa59ff0cbb955d7e6ff240fe29c371'
REVIEW = HERE / 'work-qualification/reviews/source-bytecode-semantic-review-v2.json'
EVIDENCE = HERE / 'work-qualification/reviews/nested-bytecode-review-v1/actual-evidence'
TERMINAL = EVIDENCE / 'terminal.json'
TERMINAL_SHA = 'd9206dff2d50905fba36134e14afc650a3bb29b34f814e539148507d7cfa7a6f'
INDEX = EVIDENCE / 'review-input-index.json'
INDEX_SHA = '03f286aef41cfb21a05dbc125fee7f5f15f84e4095809a57fa5212869d644db0'
CONTROLLER_SHA = '5f7de095821e2ffd5d7b34e6d2123b38244f6b5cbd6b82a6468ce3c00c9db6eb'
STATIC_FILES = ('qualify_work.py', 'observer.clj', 'instrument_sources.py', 'native_fd_observer.py',
    'work_proofs.py', 'test_work_proofs.py', 'work_fixture_runner.py', 'test_native_fd_observer.py',
    'full_study.py', 'seal_full_studies.py', 'test_full_study.py', 'REVIEW.md')
CLASSES = (
    ('scored-original', 'clbg.knucleotide.compact$histogram$fn__159'),
    ('observer', 'clbg.knucleotideobserver.compact$histogram_raw$fn__250'),
    ('observer', 'clbg.knucleotideobserver.optimized$histogram_raw$fn__252'),
    ('pinned-vendor-transient-storage', 'clojure.lang.PersistentHashMap'),
    ('pinned-vendor-transient-storage', 'clojure.lang.PersistentHashMap$TransientHashMap'))


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def new_json(path, value):
    with Path(path).open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def verify(pins):
    for path, digest in pins.items():
        assert re.fullmatch(r'[0-9a-f]{64}', digest)
        assert sha(path) == digest, ('Reviewed input changed', path)


def admission(revision, lock, owner, output):
    result = dict(status='failed-before-completion', revision=revision,
        actual_common_mutex_held=True, original_owner_lease_held=True,
        full_performance_runs=0, native_commands=0, no_automatic_retry=True)
    pins = {}
    qualified_path = HERE / 'work-qualification/qualified' / revision / 'report.json'
    try:
        assert not qualified_path.parent.exists(), 'Exclusive additive admission; no replacement/resume'
        pins = {str(SOURCE): sha(SOURCE), str(ACTUAL): ACTUAL_SHA}
        verify(pins)
        actual = json.loads(ACTUAL.read_text())
        assert actual['status'] == 'actual-work-output-ownership-passed-awaiting-source-bytecode-review'
        assert actual['active'] is None and actual['pins_unchanged_at_end'] is True
        # The exact v2 path and all its meaning are independently reviewed by
        # root before launch. Its digest is computed here under the same lock
        # and retained in the actual input receipt and qualified report.
        review_sha = sha(REVIEW)
        pins[str(REVIEW)] = review_sha
        review = json.loads(REVIEW.read_text())
        assert review['status'] == 'passed-exact-source-actual-bytecode-semantic-review'
        assert review['language'] == 'clojure' and review['unresolved'] == []
        actual_reference = dict(path=str(ACTUAL), sha256=ACTUAL_SHA)
        assert review['actual_work_report'] == actual_reference
        nested = review['supplemental_nested_bytecode_review']
        assert nested['status'] == 'passed-manual-nested-bytecode-meaning-review'
        assert nested['unresolved'] == []
        assert nested['terminal'] == dict(path=str(TERMINAL), sha256=TERMINAL_SHA)
        assert nested['review_input_index'] == dict(path=str(INDEX), sha256=INDEX_SHA)
        supplemental = review['supplemental_evidence_pins']
        assert len(supplemental) == 22
        assert supplemental[str(TERMINAL)] == TERMINAL_SHA
        assert supplemental[str(INDEX)] == INDEX_SHA
        assert all(Path(path).resolve() == Path(path) and Path(path).parent == EVIDENCE
            for path in supplemental)
        verify(supplemental)
        terminal = json.loads(TERMINAL.read_text())
        index = json.loads(INDEX.read_text())
        assert terminal['status'] == 'actual-additive-nested-class-bytecode-awaiting-semantic-review'
        assert terminal['pins_unchanged_at_end'] is True
        assert terminal['compilation_commands'] == terminal['native_benchmark_subjects'] == 0
        assert terminal['scored_classes_mutated'] is terminal['observer_classes_mutated'] is False
        assert terminal['actual_work_report'] == index['actual_work_report'] == actual_reference
        assert index['status'] == 'verified-qualified-inputs-for-manual-review'
        assert set(index['profiles']) == set(review['profiles']) and len(index['profiles']) == 20
        for identity, row in index['profiles'].items():
            semantic = review['profiles'][identity]
            assert all(row[key] == semantic[key] for key in
                ('source_sha256', 'binary_sha256', 'compiled_input_inventory_sha256'))
        assert [(row['role'], row['class_name']) for row in terminal['commands']] == list(CLASSES)
        expected_paths = {str(TERMINAL), str(INDEX)}
        for row in terminal['commands']:
            receipt = Path(row['receipt'])
            assert receipt.parent == EVIDENCE and row['stdout'] == str(receipt.with_suffix('.stdout'))
            expected_paths.update(str(receipt.with_suffix(suffix))
                for suffix in ('.json', '.stdout', '.stderr', '.active.json'))
            record = json.loads(receipt.read_text())
            assert record['command'] == row['command']
            assert record['inherited_common_mutex'] is True
            assert type(record['exit_status']) is type(record['final_exit_status']) is int
            assert record['exit_status'] == record['final_exit_status'] == 0
            assert 'cleanup_error' not in record and 'cleanup_errors' not in record
            assert len(row['command']) == 6 and row['command'][-1] == row['class_name']
            assert row['command'][1:4] == ['-c', '-p', '-cp']
        assert set(supplemental) == expected_paths
        verify(terminal['pins'])
        verify(actual['pins'])
        controller_pins = {}
        for name in STATIC_FILES:
            original, frozen = PACKET / name, SNAPSHOT / name
            assert actual['pins'][str(original)] == actual['pins'][str(frozen)]
            assert original.read_bytes() == frozen.read_bytes(), ('Frozen controller changed', name)
            controller_pins[str(original)] = actual['pins'][str(original)]
            controller_pins[str(frozen)] = actual['pins'][str(frozen)]
        assert controller_pins[str(PACKET / 'qualify_work.py')] == CONTROLLER_SHA
        verify(controller_pins)
        pins.update(supplemental)
        pins.update(controller_pins)
        input_receipt = output / 'verified-inputs.json'
        new_json(input_receipt, dict(status='verified-before-original-admission',
            actual_argv=sys.argv, guard_source=dict(path=str(SOURCE), sha256=pins[str(SOURCE)]),
            actual_work_report=actual_reference, source_review=dict(path=str(REVIEW), sha256=review_sha),
            supplemental_evidence_pins=supplemental, original_and_snapshot_pins=controller_pins,
            actual_common_descriptor=lock.fileno(), common_mutex=str(LOCK),
            common_device_inode=[os.fstat(lock.fileno()).st_dev, os.fstat(lock.fileno()).st_ino],
            original_owner_descriptor=owner.fileno(), full_performance_runs=0, native_commands=0))
        pins[str(input_receipt)] = sha(input_receipt)
        # Load the verified original location, preserving its HERE/PACKET
        # semantics. The byte-identical snapshot is checked before any import.
        sys.dont_write_bytecode = True
        name = 'clojure_reviewed_supplemental_admission'
        assert name not in sys.modules
        spec = importlib.util.spec_from_file_location(name, PACKET / 'qualify_work.py')
        work = importlib.util.module_from_spec(spec)
        sys.modules[name] = work
        spec.loader.exec_module(work)
        assert work.HERE == HERE and work.PACKET == PACKET and work.LOCK == LOCK
        assert tuple(work.STATIC_FILES) == STATIC_FILES
        original_merge = work.merge
        merge_calls = []

        def merge_supplement(target, values):
            assert set(values) == {str(ACTUAL), str(REVIEW)}, 'Only original admission input merge'
            original_merge(target, values)
            original_merge(target, pins)
            merge_calls.append(True)

        work.merge = merge_supplement
        args = argparse.Namespace(native_report=ACTUAL, native_report_sha256=ACTUAL_SHA,
            source_review=REVIEW, source_review_sha256=review_sha, revision=revision)
        work.admit(args)
        assert merge_calls == [True]
        verify(pins)
        verify(supplemental)
        admitted = json.loads(qualified_path.read_text())
        assert admitted['status'] == 'passed-exact-source-compiled-JVM-work-review'
        assert all(admitted['pins'][path] == digest for path, digest in pins.items())
        assert admitted['source_bytecode_review'] == dict(path=str(REVIEW), sha256=review_sha)
        result.update(status='completed', supplemental_evidence_count=22,
            supplemental_pins_unchanged_at_end=True, original_and_snapshot_unchanged=True,
            qualified_report=dict(path=str(qualified_path)),
            source_review=dict(path=str(REVIEW), sha256=review_sha), input_receipt=str(input_receipt))
    except BaseException as error:
        result.update(error=str(error) or type(error).__name__, error_type=type(error).__name__)
        try:
            with (output / 'actual-error.log').open('x') as stream:
                stream.write(traceback.format_exc())
        except BaseException as retention_error:
            result['error_log_retention_error'] = dict(error_type=type(retention_error).__name__,
                error=str(retention_error) or type(retention_error).__name__)
        raise
    finally:
        result['reviewed_input_pins'] = pins
        retained = result['retained_actual_evidence_pins'] = {}
        integrity_errors = []
        preceding_status = result['status']

        def integrity_error(path, error):
            integrity_errors.append(dict(path=str(path), error_type=type(error).__name__,
                error=str(error) or type(error).__name__))
            result.update(status='failed-terminal-integrity', preceding_status=preceding_status,
                terminal_integrity_errors=integrity_errors)

        try:
            for path in output.iterdir():
                try:
                    if path.is_file():
                        retained[str(path)] = sha(path)
                except BaseException as error:
                    integrity_error(path, error)
        except BaseException as error:
            integrity_error(output, error)
        try:
            if qualified_path.exists():
                result['retained_qualified_report'] = dict(path=str(qualified_path), sha256=sha(qualified_path))
                if preceding_status == 'completed':
                    result['qualified_report'] = dict(result['retained_qualified_report'])
        except BaseException as error:
            integrity_error(qualified_path, error)
        # Preserve every hash already captured even when a later file fails.
        # No terminal hash is needed here; root reviews this immutable raw
        # outcome under the common mutex after observing our actual exit code.
        try:
            new_json(output / 'outcome.json', result)
        except BaseException as error:
            integrity_error(output / 'outcome.json', error)
            raise
        print(json.dumps(dict(status=result['status'], outcome=str(output / 'outcome.json'))), flush=True)
        if integrity_errors:
            raise RuntimeError('Supplemental admission terminal integrity failed; partial evidence retained')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--revision', required=True)
    args = parser.parse_args()
    assert re.fullmatch(r'[a-z0-9][a-z0-9-]{0,63}', args.revision)

    def interrupted(signum, frame):
        raise KeyboardInterrupt('Supplemental admission interrupted ' + str(signum))

    for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(signum, interrupted)
    with (PACKET / 'work-owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        print('WAIT common mutex: Clojure reviewed supplemental admission', flush=True)
        with LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            output = HERE / 'work-qualification/controller-outcomes/admit' / args.revision
            assert not output.exists(), 'Exclusive actual outcome; no overwrite/resume/retry'
            output.mkdir(parents=True)
            new_json(output / 'started.json', dict(status='started', revision=args.revision,
                actual_argv=sys.argv, actual_common_mutex_held=True, original_owner_lease_held=True,
                actual_common_descriptor=lock.fileno(), full_performance_runs=0, native_commands=0))
            admission(args.revision, lock, owner, output)


if __name__ == '__main__':
    main()
