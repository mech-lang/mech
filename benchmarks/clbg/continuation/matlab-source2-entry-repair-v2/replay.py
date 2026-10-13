"""Replay the unchanged MATLAB parser using exact saved fingerprint semantics."""
from contextlib import contextmanager
import fcntl
import hashlib
import importlib.util
import json
from pathlib import Path
import sys
from types import FunctionType

HERE = Path(__file__).resolve().parent
CLBG = HERE.parents[1]
ORIGINAL = CLBG / 'loc-normalization/20261012/matlab-regexp-grammar-v1/qualify_replay.py'
ORIGINAL_SHA = '379104af95910a652697512f3a9aa170ca8c4614f7855e433052a3bac09f4acc'
FAILED = CLBG / 'continuation/owned-jobs/matlab-regexp-source2-v1'
LOCK = Path('/tmp/mech-clbg-timing.lock')


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


# Import and capture the actual failed entry only while holding the same mutex.
# The unchanged runner obtains it again for its complete replay transaction.
with LOCK.open('a+') as held:
    fcntl.flock(held, fcntl.LOCK_EX)
    assert sha(ORIGINAL) == ORIGINAL_SHA
    assert json.loads((FAILED / 'terminal.json').read_text())['exit_code'] == 1
    assert 'actual_attempt' in (FAILED / 'command.stderr').read_text()
    retained = {str(path): sha(path) for path in FAILED.iterdir() if path.is_file()}
    retained[str(Path(__file__).resolve())] = sha(__file__)
    sys.dont_write_bytecode = True
    name = 'matlab_exact_original_source_replay_entry_v2'
    assert name not in sys.modules
    spec = importlib.util.spec_from_file_location(name, ORIGINAL)
    original = importlib.util.module_from_spec(spec)
    sys.modules[name] = original
    exec(compile(ORIGINAL.read_bytes(), str(ORIGINAL), 'exec'), original.__dict__)
    assert sha(ORIGINAL) == ORIGINAL_SHA


def fingerprint(row):
    support = {}
    if row['language'] == 'mech':
        # This cohort's exact Mech row has one source-support entry in its
        # saved harness fingerprint. Derive it from the independently pinned
        # source identity, rather than copying a value out of an attempt.
        assert row['id'] == 'mech-aot-control' and row['benchmark'] == 'regexredux'
        support[Path(row['source']).relative_to(original.CLBG).as_posix()] = row['source_sha256']
    return json.dumps(dict(binary_sha256=row.get('binary_sha256'),
        command=row['command'], cwd=row.get('cwd'),
        environment=row.get('runtime_environment', {}),
        source_sha256=row['source_sha256'], support=support), sort_keys=True)


def copy_function(function, **bindings):
    result = FunctionType(function.__code__, dict(function.__globals__, **bindings),
        function.__name__, function.__defaults__, function.__closure__)
    result.__kwdefaults__ = function.__kwdefaults__
    return result


attempt = copy_function(original.actual_attempt, fingerprint=fingerprint)
inventory = copy_function(original.source_inventory, actual_attempt=attempt)


@contextmanager
def terminal(revision, state):
    with original.terminal(revision, state) as (output, lock):
        assert sha(ORIGINAL) == ORIGINAL_SHA
        for path, digest in retained.items():
            assert sha(path) == digest
            assert path not in state['pins'] or state['pins'][path] == digest
            state['pins'][path] = digest
        original.new(output / 'entry-repair.json', dict(
            status='additive-fingerprint-schema-repair', original=str(ORIGINAL),
            original_sha256=ORIGINAL_SHA, preserved_failed_entry_pins=retained,
            changes=['Mech source-support entry derived from exact row source',
                     'absent legacy binary identity retained as JSON null'],
            parser_changed=False, fixture_methods_changed=False,
            original_native_records_changed=False, benchmark_runs=0,
            identity_checks_removed=0, chart_admission=False))
        yield output, lock


original.run = copy_function(original.run, source_inventory=inventory, terminal=terminal)
original.main()
