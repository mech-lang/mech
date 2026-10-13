"""Restore missing temporary lexer bytes against two actual complete pin maps.

No version substitution, dependency import, source normalization or benchmark.
The saved qualified receipts remain unchanged. Retain a durable byte snapshot.
"""
from datetime import datetime, timezone
import fcntl
import hashlib
import json
import os
from pathlib import Path
import sys
import traceback

HERE = Path(__file__).resolve().parent
CLBG = HERE.parent
LOCK = Path('/tmp/mech-clbg-timing.lock')
TARGET = Path('/private/tmp/mech-clbg-compact-libs/pygments')
OUTPUT = HERE / 'jobs/exact-pygments-restoration-v2'
EVIDENCE = (
    (CLBG / 'loc-normalization/20261012/jobs/boundary-audit-18/fixture-receipt.json',
     'b00afe351c40fb38e03a99ec81fe3d71bd69a82726f337aa87dfb40cfcaa6b6a'),
    (CLBG / 'loc-normalization/20261012/c3-proposal-v4/jobs/fixtures-v4/fixture-receipt.json',
     'a3e1197e6371c7e2b77c160763e44e1b4e62c397b1704dfd3f345052753647da'),
    (CLBG / 'pony-comparison/20261012/source-counts/loc18-replay-v1/source-count-proposals.json',
     '582d986ddccd9dd93b762f83ad8dca87445600a35286ff09b370b43faad2a511'),
)
CANDIDATES = (
    Path('/tmp/mech-nbody-plot-runtime/lib/python3.12/site-packages/pygments'),
    Path('/tmp/mech-nbody-plot-runtime/lib/python3.12/site-packages/pip/_vendor/pygments'),
    Path('/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/lib/python3.12/site-packages/pip/_vendor/pygments'),
)


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def save(path, value):
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def selected(pins):
    return {path: value for path, value in pins.items()
            if Path(path).is_relative_to(TARGET)}


def main():
    print('WAIT common mutex: restore exact338 qualified Pygments source bytes', flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        opened, named = os.fstat(lock.fileno()), LOCK.stat()
        assert (opened.st_dev, opened.st_ino) == (named.st_dev, named.st_ino)
        OUTPUT.mkdir(parents=True)
        pins, restored, comparisons, touched, cache_moves = {}, [], [], [], []
        expected, body_error = {}, None
        try:
            pins[str(Path(__file__).resolve())] = sha(__file__)
            pins[str(Path(sys.executable).resolve())] = sha(Path(sys.executable).resolve())
            documents = []
            for path, digest in EVIDENCE:
                assert sha(path) == digest
                pins[str(path)] = digest
                documents.append(json.loads(path.read_text()))
            neutral, c3, pony = documents
            assert neutral['status'] == 'passed' and neutral['tests_run'] == 672
            assert neutral['failures'] == neutral['errors'] == 0
            assert c3['status'] == 'passed-awaiting-source-review' and c3['tests_run'] == 19
            assert c3['pins_before'] == c3['pins_after'] and c3['pin_reconciliation_passed'] is True
            assert pony['status'] == 'actual-source-count-proposals-awaiting-selected-source-review'
            expected = selected(c3['pins_before'])
            assert expected == selected(pony['pins']) and len(expected) == 338
            assert neutral['pygments_module'] == str(TARGET / '__init__.py')
            assert expected[neutral['pygments_module']] == neutral['pygments_module_sha256']
            assert all(Path(path).suffix == '.py' for path in expected)
            relative = {Path(path).relative_to(TARGET): digest for path, digest in expected.items()}
            chosen = None
            for candidate in CANDIDATES:
                mismatches = []
                for name, digest in relative.items():
                    path = candidate / name
                    if not path.is_file() or path.is_symlink() or sha(path) != digest:
                        mismatches.append(str(name))
                comparisons.append(dict(candidate=str(candidate), mismatched_source_files=mismatches))
                if not mismatches:
                    chosen = candidate
                    break
            assert chosen is not None, 'No candidate reproduces all338 exact qualified source digests'
            snapshot = OUTPUT / 'snapshot/pygments'
            for name, digest in relative.items():
                source, target = chosen / name, snapshot / name
                raw = source.read_bytes()
                assert hashlib.sha256(raw).hexdigest() == digest
                target.parent.mkdir(parents=True, exist_ok=True)
                with target.open('xb') as stream:
                    stream.write(raw)
                target.chmod(0o444)
                assert sha(target) == digest
                pins[str(source)] = pins[str(target)] = digest
            # Complete candidate/snapshot and existing-target preflight before
            # any restoration. Never overwrite changed or unrecorded code.
            assert not TARGET.is_symlink()
            current = {str(path) for path in TARGET.rglob('*.py')} if TARGET.exists() else set()
            assert current <= set(expected)
            caches = sorted(TARGET.rglob('*.pyc'))
            for cache in caches:
                assert cache.is_file() and not cache.is_symlink()
                ancestor = cache
                while ancestor.is_relative_to(TARGET.parent):
                    assert not ancestor.is_symlink()
                    ancestor = ancestor.parent
            for name, digest in relative.items():
                path = TARGET / name
                ancestor = path
                while ancestor.is_relative_to(TARGET.parent):
                    assert not ancestor.is_symlink(), ('Symlink in restoration target', str(ancestor))
                    if ancestor != path and ancestor.exists():
                        assert ancestor.is_dir()
                    ancestor = ancestor.parent
                if path.exists():
                    assert path.is_file() and sha(path) == digest
            # Unrecorded bytecode is never imported or discarded. Preserve it
            # at an exclusive durable path before restoring the pinned sources.
            for cache in caches:
                destination = OUTPUT / 'quarantined-bytecode' / cache.relative_to(TARGET)
                destination.parent.mkdir(parents=True, exist_ok=True)
                assert not destination.exists()
                digest = sha(cache)
                entry = dict(original_path=str(cache), preserved_path=str(destination), sha256=digest,
                             moved=False)
                cache_moves.append(entry)
                cache.rename(destination)
                entry['moved'] = True
                pins[str(destination)] = digest
                assert sha(destination) == digest
            assert not list(TARGET.rglob('*.pyc'))
            for name, digest in relative.items():
                target = TARGET / name
                if not target.exists():
                    touched.append(str(target))
                    target.parent.mkdir(parents=True, exist_ok=True)
                    with target.open('xb') as stream:
                        stream.write((snapshot / name).read_bytes())
                    target.chmod(0o444)
                    restored.append(str(target))
                assert sha(target) == digest
                pins[str(target)] = digest
            for path, digest in pins.items():
                assert sha(path) == digest
            save(OUTPUT / 'receipt.json', dict(status='exact-qualified-source-dependency-restored',
                expected_source_files=338, restored_source_files=len(restored), selected_candidate=str(chosen),
                snapshot=str(snapshot), restored_paths=restored, preserved_bytecode=cache_moves, candidate_comparisons=comparisons, pins=pins,
                dependency_version_substituted=False, qualified_receipts_mutated=False,
                actual_dependency_imports=0, actual_fixture_tests=0, actual_native_subjects=0,
                exclusive_common_timing_mutex_held=True, common_timing_mutex=str(LOCK),
                canonical_LOC_admitted=False, chart_admission=False,
                finished_at=datetime.now(timezone.utc).isoformat()))
            print(json.dumps(dict(status='exact-qualified-source-dependency-restored',
                source_files=338, restored_files=len(restored), receipt=str(OUTPUT / 'receipt.json'))), flush=True)
        except BaseException as error:
            body_error = dict(error=str(error) or type(error).__name__, error_type=type(error).__name__)
            (OUTPUT / 'failure.log').write_text(traceback.format_exc())
            save(OUTPUT / 'failed-receipt.json', dict(status='failed-exact-dependency-restoration',
                error=str(error) or type(error).__name__, available_pins=pins, restored_paths=restored,
                touched_target_paths=touched, preserved_bytecode=cache_moves,
                candidate_comparisons=comparisons, exclusive_common_timing_mutex_held=True,
                qualified_receipts_mutated=False, actual_dependency_imports=0, actual_native_subjects=0))
            raise
        finally:
            captured, failures = {}, []
            candidates = set(pins) | set(touched)
            candidates.update(str(path) for path in OUTPUT.rglob('*') if path.is_file())
            for name in sorted(candidates):
                try:
                    path = Path(name)
                    assert path.is_file() and not path.is_symlink()
                    actual = sha(path)
                    captured[name] = actual
                    wanted = pins.get(name, expected.get(name))
                    if wanted is not None:
                        assert actual == wanted, ('Partial or changed restoration input/output', name)
                except BaseException as error:
                    failures.append(dict(path=name, error=str(error) or type(error).__name__,
                                         error_type=type(error).__name__))
            status = ('failed-terminal-integrity' if failures else
                      'failed-restoration-stage' if body_error else 'completed-exact-restoration')
            save(OUTPUT / 'terminal.json', dict(status=status, body_error=body_error,
                integrity_errors=failures, pins=captured, touched_target_paths=touched, preserved_bytecode=cache_moves,
                restoration_incomplete=body_error is not None or bool(failures),
                available_bytes_not_fully_pinned=bool(failures),
                exclusive_common_timing_mutex_held=True, actual_native_subjects=0,
                automatic_retry=False, qualified_receipts_mutated=False))
            if failures:
                raise RuntimeError('Restoration terminal integrity failure retained under common mutex')


if __name__ == '__main__':
    main()
