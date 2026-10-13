"""Enter the unchanged v5 Pony work runner with source imports under its mutex.

Source-only candidate. Root supplies the actual successful manifest digest.
No seal, retry, sample, dataset admission or extra native case is available.
"""
import argparse
import fcntl
import hashlib
import importlib.abc
import importlib.machinery
import importlib.util
import json
import os
from pathlib import Path
import re
import sys
import traceback

PACKET = Path(__file__).resolve().parent
SOURCE = Path(__file__).resolve()
CLBG = PACKET.parents[1]
COHORT = CLBG / 'pony-comparison/20261012'
WORK = COHORT / 'revisions/work-full-v5'
TARGET = WORK / 'work_qualify.py'
REVISION = 'private-v8-work-v5'
OUTPUT = COHORT / 'work-qualification' / REVISION
MANIFEST = OUTPUT / 'manifest.json'
QUALIFICATION = COHORT / 'qualification/private-v8/report.json'
QUALIFICATION_SHA = '4da05f6dd351609923f5f1ca227d3db2b33a354925614b74f9c5dae39839d159'
LOCK = Path('/tmp/mech-clbg-timing.lock')
PYTHON = Path('/opt/homebrew/opt/python@3.14/bin/python3.14')


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def save(path, value):
    with Path(path).open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def merge(pins, values):
    for path, digest in values.items():
        assert type(path) is str and re.fullmatch(r'[0-9a-f]{64}', digest)
        assert path not in pins or pins[path] == digest, ('Conflicting source prerequisite', path)
        pins[path] = digest


def verify(pins, verified):
    for path, digest in pins.items():
        assert sha(path) == digest, ('Changed exact prerequisite', path)
        verified[path] = digest


def descriptor(lock):
    held, named = os.fstat(lock.fileno()), LOCK.stat()
    assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino), 'Named common mutex changed'
    return lock.fileno()


def prerequisites(digest, pins, verified):
    assert sha(MANIFEST) == digest
    manifest = json.loads(MANIFEST.read_text())
    assert manifest['status'] == 'prepared-awaiting-root-review'
    assert manifest['shared_lock'] == str(LOCK)
    for key, expected in dict(source_work_profiles=20, source_work_trace_profiles=6,
            reused_actual_work_traces=4, new_actual_work_traces_required=2,
            reused_fixture_tests=14, fresh_fixture_tests=0, full_performance_runs=0,
            new_native_cases_required=2).items():
        assert type(manifest[key]) is int and manifest[key] == expected, key
    assert manifest['production_operations_preserved'] is True
    assert manifest['workload_code_delegated_to_native'] is False
    assert manifest['qualification'] == str(QUALIFICATION)
    assert manifest['qualification_sha256'] == QUALIFICATION_SHA == sha(QUALIFICATION)
    qualification = json.loads(QUALIFICATION.read_text())
    assert qualification['status'] == 'bounded-output-primitives-passed-awaiting-work-worker-and-LOC-review'
    assert len(qualification['profiles']) == len({row['id'] for row in qualification['profiles']}) == 20
    merge(pins, qualification['pins'])
    merge(pins, manifest['pins'])
    merge(pins, {str(MANIFEST): digest, str(QUALIFICATION): QUALIFICATION_SHA})
    reuse_path = Path(manifest['prior_work_reuse']['path'])
    assert reuse_path == OUTPUT / 'prior-work-reuse.json'
    assert sha(reuse_path) == manifest['prior_work_reuse']['sha256']
    reused = json.loads(reuse_path.read_text())
    assert reused['status'] == 'verified-four-actual-Pi-K-work-proofs-and-fourteen-fixtures'
    assert reused['reused_actual_work_traces'] == 4 and reused['new_actual_work_traces_required'] == 2
    assert reused['old_attempts_relabelled_as_new'] is False
    assert len(reused['attempts']) == len({row['id'] for row in reused['attempts']}) == 4
    assert [row['id'] for row in reused['attempts']] == manifest['reused_work_trace_ids']
    assert {row['benchmark'] for row in reused['attempts']} == {'pidigits', 'knucleotide'}
    assert len(manifest['rows']) == len({row['id'] for row in manifest['rows']}) == 6
    fresh = [row for row in manifest['rows'] if row['id'] not in manifest['reused_work_trace_ids']]
    assert len(fresh) == 2 and {row['benchmark'] for row in fresh} == {'binarytrees'}
    assert {row['variant'] for row in fresh} == {'compact', 'optimized'}
    merge(pins, reused['pins'])
    merge(pins, {str(reuse_path): manifest['prior_work_reuse']['sha256']})
    seal_path = COHORT / 'work-seal-outcomes' / REVISION / 'receipt.json'
    seal = json.loads(seal_path.read_text())
    assert seal['status'] == 'completed-metadata-awaiting-root-review'
    assert seal['revision'] == REVISION and seal['actual_common_mutex_held'] is True
    assert seal['chart_admission'] is False
    assert type(seal['actual_native_subjects']) is int and seal['actual_native_subjects'] == 0
    assert 'error' not in seal and 'terminal_integrity_error' not in seal
    assert seal['available_evidence_pins'][str(MANIFEST)] == digest
    merge(pins, seal['available_evidence_pins'])
    merge(pins, {str(seal_path): sha(seal_path)})
    assert Path(sys.executable).resolve(strict=True) == PYTHON.resolve(strict=True)
    assert sys.version_info[:2] == (3, 14) and sys.dont_write_bytecode is True
    assert str(Path(sys.executable).resolve()) in pins
    for path in (TARGET, COHORT / 'guarded_prepare_v6.py', COHORT / 'references.py'):
        assert str(path) in manifest['pins'], ('Unbound authored entry source', path)
    assert not (OUTPUT / 'report.json').exists(), 'No implicit work retry or resume'
    verify(pins, verified)
    return manifest


class ReviewedSources(importlib.abc.MetaPathFinder, importlib.abc.Loader):
    """Bounded same-process source loader; never reads or writes a .pyc."""
    def __init__(self, pins, preflight_lock, records):
        self.pins, self.preflight_lock, self.records = pins, preflight_lock, records
        self.original = None
        self.run_code = None
        self.executed = {}
        self.source_pins = {}
        for filename, digest in pins.items():
            path = Path(filename).resolve(strict=True)
            if path.is_relative_to(CLBG) and path.suffix == '.py':
                key = str(path)
                assert key not in self.source_pins or self.source_pins[key] == digest
                self.source_pins[key] = digest
        self.original_spec = importlib.util.spec_from_file_location

    def held_common(self):
        if self.preflight_lock is not None and not self.preflight_lock.closed:
            return descriptor(self.preflight_lock), 'entry-preflight'
        # Exact unchanged run() obtains initial_lock before load_guard/helpers,
        # and native_terminal yields lock only after taking the same flock.
        # Only those two audited locals in that exact function frame qualify.
        frame = sys._getframe(1)
        try:
            while frame is not None:
                if self.run_code is not None and frame.f_code is self.run_code:
                    for name in ('initial_lock', 'lock'):
                        lock = frame.f_locals.get(name)
                        if lock is not None and not lock.closed:
                            return descriptor(lock), 'unchanged-original-run:' + name
                frame = frame.f_back
        finally:
            del frame
        raise AssertionError('Authored source execution outside the reviewed held common mutex')

    def selected(self, location):
        path = Path(location).resolve(strict=True)
        if not path.is_relative_to(CLBG):
            return None
        assert path.suffix == '.py' and str(path) in self.source_pins, ('Unselected authored import', path)
        return path

    def source_spec(self, name, location, *, loader=None, submodule_search_locations=None):
        if location is not None:
            path = self.selected(location)
            if path is not None:
                assert loader is None or loader is self, 'Unreviewed explicit authored loader'
                self.held_common()
                assert sha(path) == self.source_pins[str(path)]
                return self.original_spec(name, path, loader=self,
                    submodule_search_locations=submodule_search_locations)
        return self.original_spec(name, location, loader=loader,
            submodule_search_locations=submodule_search_locations)

    def find_spec(self, fullname, path=None, target=None):
        spec = importlib.machinery.PathFinder.find_spec(fullname, path, target)
        if spec is None or spec.origin is None or spec.origin in {'built-in', 'frozen'}:
            return None
        selected = self.selected(spec.origin)
        if selected is None:
            return None
        return self.source_spec(fullname, selected,
            submodule_search_locations=spec.submodule_search_locations)

    def create_module(self, spec):
        return None

    def exec_module(self, module):
        lock_fd, lock_context = self.held_common()
        path = self.selected(module.__spec__.origin)
        assert path is not None and Path(module.__file__).resolve() == path
        assert id(module) not in self.executed, 'Do not execute a cached module twice'
        cached = sys.modules.get(module.__name__)
        assert cached is None or cached is module, 'Foreign cached authored module'
        expected = self.source_pins[str(path)]
        source = path.read_bytes()
        assert hashlib.sha256(source).hexdigest() == expected == sha(path)
        record = dict(name=module.__name__, path=str(path), sha256=expected,
            common_mutex_fd=lock_fd, common_mutex_context=lock_context,
            source_execution_started=True, source_execution_completed=False,
            bytecode_cache_used=False, exact_original_source_compiled=True)
        self.records.append(record)
        exec(compile(source, str(path), 'exec'), module.__dict__)
        assert Path(module.__file__).resolve() == path and module.__spec__.loader is self
        assert sha(path) == expected
        self.executed[id(module)] = module
        record['source_execution_completed'] = True

    def cold(self, name, path):
        assert name not in sys.modules, ('Unreviewed cached entry module', name)
        spec = self.source_spec(name, path)
        module = importlib.util.module_from_spec(spec)
        sys.modules[name] = module
        self.exec_module(module)
        return module


def journal(output, filename, state, pins, verified, records):
    """Keep all readable partial bytes and per-path failures under held mutex."""
    available, errors = {}, []
    for path in sorted(output.rglob('*')):
        if path.is_file():
            try:
                available[str(path.resolve())] = sha(path)
            except BaseException as failure:
                errors.append(dict(path=str(path), error=str(failure) or type(failure).__name__))
    for path, digest in pins.items():
        try:
            assert sha(path) == digest
            verified[path] = digest
        except BaseException as failure:
            errors.append(dict(path=path, error=str(failure) or type(failure).__name__))
    state.update(source_imports=records, prerequisite_pins=pins,
        verified_prerequisite_pins=verified, available_entry_artifact_pins=available,
        terminal_integrity_errors=errors, available_bytes_not_fully_pinned=bool(errors))
    if errors:
        state.update(preceding_status=state['status'], status='failed-terminal-integrity')
    save(output / filename, state)
    save(output / (filename + '.pin.json'), dict(path=str(output / filename),
        sha256=sha(output / filename), status=state['status'], actual_common_mutex_held=True,
        chart_admission=False, full_timing_samples_authorized=0))
    if errors:
        raise RuntimeError('Pony entry terminal integrity failed; readable partial evidence retained')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--manifest-sha256', required=True)
    args = parser.parse_args()
    assert re.fullmatch(r'[0-9a-f]{64}', args.manifest_sha256)
    output = PACKET / 'jobs' / REVISION
    pins, verified, records = {}, {}, []
    state = dict(status='failed-preflight', revision=REVISION, manifest=str(MANIFEST),
        manifest_sha256=args.manifest_sha256, pid=os.getpid(), automatic_retry=False,
        chart_admission=False, full_timing_samples_authorized=0,
        preflight_actual_native_subjects=0, delegation_started=False)
    original = loader = None
    saved_path = list(sys.path)
    print('WAIT common mutex: Pony cold verified source entry', flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        descriptor(lock)
        assert not output.exists(), 'Exclusive entry attempt; preserve prior receipts'
        output.mkdir(parents=True)
        try:
            for path in (SOURCE, PACKET / 'REVIEW.md'):
                merge(pins, {str(path): sha(path)})
            prerequisites(args.manifest_sha256, pins, verified)
            for name, module in list(sys.modules.items()):
                filename = getattr(module, '__file__', None)
                if filename and Path(filename).resolve().is_relative_to(CLBG):
                    assert Path(filename).resolve() == SOURCE and name == '__main__', ('Foreign cached authored module', name, filename)
            loader = ReviewedSources(pins, lock, records)
            sys.meta_path.insert(0, loader)
            importlib.util.spec_from_file_location = loader.source_spec
            sys.path.insert(0, str(WORK))
            prep = loader.cold('guarded_prepare_v6', COHORT / 'guarded_prepare_v6.py')
            references = loader.cold('references', COHORT / 'references.py')
            original = loader.cold('pony_reviewed_work_full_v5', TARGET)
            loader.original = original
            loader.run_code = original.run.__code__
            assert original.original_preparation is original.preparation is prep
            assert original.references is references and original.HERE == WORK and original.COHORT == COHORT
            assert original.LOCK == LOCK
            for name in ('main', 'run'):
                assert Path(getattr(original, name).__code__.co_filename).resolve() == TARGET
            assert Path(original.native_terminal.__wrapped__.__code__.co_filename).resolve() == TARGET
            verify(pins, verified)
            state.update(status='prepared-cold-entry-awaiting-original-run',
                actual_common_mutex_held_for_imports=True,
                bounded_original_scope=dict(retained_actual_cases=4, retained_fixture_tests=14,
                    fresh_tree_cases=2, scored_allocation_evidence=True, full_samples=0),
                original_function_code_changed=False, stale_bytecode_execution_allowed=False)
        except BaseException as failure:
            state.update(error=str(failure) or type(failure).__name__, error_type=type(failure).__name__)
            try:
                with (output / 'preflight-error.log').open('x') as stream:
                    stream.write(traceback.format_exc())
            except BaseException as log_error:
                state['error_log_write_failure'] = str(log_error) or type(log_error).__name__
            raise
        finally:
            journal(output, 'preflight.json', state, pins, verified, records)
    # All frozen authored top imports are now exact source code in memory.
    # Close this descriptor before unchanged run() obtains its own descriptor.
    # Keep this same PID and every pre-existing durable owner lease untouched.
    error = None
    outcome = dict(status='failed-original-run', pid=os.getpid(), revision=REVISION,
        manifest_sha256=args.manifest_sha256, delegation_started=True,
        full_timing_samples_authorized=0, chart_admission=False,
        original_function_code_changed=False, original_common_mutex_observed=False)
    old_argv = sys.argv
    try:
        sys.argv = [str(TARGET), '--run', '--revision', REVISION,
                    '--manifest-sha256', args.manifest_sha256]
        original.main()
        outcome['status'] = 'original-run-returned-awaiting-root-evidence-review'
    except BaseException as failure:
        error = failure
        outcome.update(error=str(failure) or type(failure).__name__, error_type=type(failure).__name__)
        outcome['traceback'] = traceback.format_exc()
    finally:
        sys.argv = old_argv
        try:
            assert loader is not None and importlib.util.spec_from_file_location == loader.source_spec
            importlib.util.spec_from_file_location = loader.original_spec
            assert sys.meta_path[0] is loader
            sys.meta_path.pop(0)
            sys.path[:] = saved_path
        except BaseException as failure:
            outcome.update(status='failed-entry-hook-integrity',
                entry_hook_error=str(failure) or type(failure).__name__)
            if error is None:
                error = failure
        outcome['original_common_mutex_observed'] = any(
            item['common_mutex_context'].startswith('unchanged-original-run:') for item in records)
        with LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            descriptor(lock)
            report = OUTPUT / 'report.json'
            if report.is_file():
                try:
                    raw = json.loads(report.read_text())
                    assert raw['manifest_sha256'] == args.manifest_sha256 and raw['manifest'] == str(MANIFEST)
                    outcome.update(actual_original_report=str(report), actual_original_report_sha256=sha(report),
                        actual_original_status=raw['status'], actual_fresh_work_traces=raw['fresh_actual_work_traces'],
                        actual_retained_work_traces=raw['reused_actual_work_traces'])
                    if error is None:
                        assert raw['status'] == 'native-work-v2-passed-awaiting-scored-binary-allocation-review'
                        for key, expected in (('fresh_actual_work_traces', 2), ('reused_actual_work_traces', 4),
                                              ('full_performance_runs', 0)):
                            assert type(raw[key]) is int and raw[key] == expected
                        assert len(raw['attempts']) == len({row['id'] for row in raw['attempts']}) == 6
                except BaseException as failure:
                    outcome.update(status='failed-original-report-reconciliation',
                        report_error=str(failure) or type(failure).__name__)
                    if error is None:
                        error = failure
            elif error is None:
                error = AssertionError('Unchanged original run returned without its actual terminal report')
                outcome.update(status='failed-original-report-reconciliation', report_error=str(error))
            journal(output, 'delegate-outcome.json', outcome, pins, verified, records)
    if error is not None:
        raise error


if __name__ == '__main__':
    main()
