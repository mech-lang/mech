"""Repair only count-entry module/service bindings; reuse actual6/34 fixtures.

Both original runners, fixture snapshots, counters and grammars stay unchanged.
Each invocation delegates one new20-source proposal job with zero fixture runs.
"""
import argparse
import fcntl
import importlib
import importlib.util
import json
from pathlib import Path
import re
import sys
from types import SimpleNamespace

CLBG = Path(__file__).resolve().parents[2]
SOURCE = Path(__file__).resolve()
LOCK = Path('/tmp/mech-clbg-timing.lock')
LOC = CLBG / 'loc-normalization/20261012'
SETTINGS = {
    'nim': dict(packet=LOC / 'nim-source-replay-v1', script='qualify_replay.py',
        script_sha='19a38d8aa24d1f0ebbdf0d7e4f5033345eb7db8e58bd11ea42b15063b1934cda',
        fixture='fixtures-v1', fixture_sha='aec967282855da29084de8d445b6f8311c42f3c1be5963f162078e1f167e2e52'),
    'gleam': dict(packet=LOC / 'gleam-proposal-v2', script='replay_sources.py',
        script_sha='9ab5cf3e0159e78277dea41e8fbff4e8728c66d96cc04c766cac1834bf68c343',
        fixture='fixtures-v2', fixture_sha='247610098f0151f75d4f6432369101664d2f2acddf6dbb716f9a041a4a048733'),
}


def repaired_nim(original, repair_pins, output):
    def qualified(pins):
        # Same actual private-v7 report and original validation function. The
        # work packet supplies fixture_runner; no root fixture_runner exists.
        original.merge(pins, repair_pins)
        raw = json.loads(original.QUALIFICATION.read_text())
        assert original.QUALIFICATION == original.NIM / 'qualification/private-v7/report.json'
        assert raw['proposal_sha256'] == original.PROPOSAL_SHA
        original.merge(pins, raw['pins'])
        original.merge(pins, {str(original.QUALIFICATION): original.sha(original.QUALIFICATION)})
        original.verify(pins)
        modules = {'qualify_work': original.QUALIFIER, 'tree_work': original.QUALIFIER,
            'fixture_runner': original.QUALIFIER, 'guarded_prepare_v7': original.NIM,
            'proof_reuse_v5': original.NIM, 'primitive_reuse_v6': original.NIM,
            'ownership_reuse_v7': original.NIM, 'references': original.NIM}
        cohort = {path.stem: path for path in original.NIM.glob('*.py')}
        assert not any(name in sys.modules for name in {*modules, *cohort})
        original.merge(pins, {str(path): original.sha(path) for path in original.QUALIFIER.iterdir()
            if path.is_file() and path.suffix in ('.py', '.md')})
        original.verify(pins)
        sys.path.insert(0, str(original.QUALIFIER))
        try:
            qualifier = importlib.import_module('qualify_work')
        finally:
            sys.path.pop(0)
        bindings = {}
        for name, folder in modules.items():
            path = folder / (name + '.py')
            assert Path(sys.modules[name].__file__).resolve() == path
            assert original.sha(path) == pins[str(path)]
            bindings[name] = dict(path=str(path), sha256=pins[str(path)])
        for name, path in cohort.items():
            if name in sys.modules:
                assert Path(sys.modules[name].__file__).resolve() == path
                assert original.sha(path) == pins[str(path)]
        original.verify(pins)
        assert Path(qualifier.preparation.__file__).resolve() == original.NIM / 'guarded_prepare_v7.py'
        guard = qualifier.preparation.load_guard()
        harness = guard.load_harness()
        report, reuse = qualifier.qualified_report(original.QUALIFICATION, guard)
        assert report['proposal_sha256'] == original.PROPOSAL_SHA and len(report['profiles']) == 20
        assert report['reused_bounded_cases'] == 154 and len(report['proofs']) == 24 and len(report['attempts']) == 20
        original.merge(pins, reuse['pins'])
        assert Path(guard.__file__).resolve() == qualifier.preparation.GUARD
        assert original.sha(guard.__file__) == qualifier.preparation.GUARD_SHA
        receipt = output / 'entry-repair.json'
        original.save(receipt, dict(status='exact-private-v7-count-entry-binding-repaired',
            repaired_function_source=str(SOURCE), original_count_runner=str(original.HERE / 'qualify_replay.py'),
            corrected_binding='fixture_runner is in revisions/work-full-v4, not the Nim root',
            actual_module_bindings=bindings, fixture_runner_and_counter_bytes_changed=False,
            actual_source_input_fixtures_retained=6, new_fixture_runs=0, benchmark_processes_started=0,
            qualification=dict(path=str(original.QUALIFICATION), sha256=pins[str(original.QUALIFICATION)])))
        original.merge(pins, {str(receipt): original.sha(receipt)})
        original.verify(pins)
        return qualifier, guard, harness, report, reuse

    original.qualified = qualified


def repaired_gleam(original, repair_pins, output):
    original_merge = original.merge
    original.merge = lambda *groups: original_merge(*groups, repair_pins)

    def bound_import(name):
        value = importlib.import_module(name)
        if name != 'qualify_work':
            return value
        assert Path(value.__file__).resolve() == original.QUALIFIER / 'qualify_work.py'
        preparation = value.preparation
        expected = original.ROOT / 'revisions/private-v5/guarded_prepare.py'
        assert Path(preparation.__file__).resolve() == expected
        assert not hasattr(preparation, 'load_guard'), 'This adapter only repairs the original absent interface'

        def load_actual_guard():
            actual = json.loads(original.QUALIFICATION.read_text())
            assert original.QUALIFICATION == original.ROOT / 'qualification/private-v5/report.json'
            assert actual['proposal_sha256'] == original.PROPOSAL_SHA
            original.verify(actual['pins'])
            assert original.sha(expected) == actual['pins'][str(expected)]
            assert original.sha(preparation.SERVICE) == actual['pins'][str(preparation.SERVICE)]
            service = preparation.load_service()
            assert Path(service.__file__).resolve() == preparation.SERVICE
            guard = service.load_guard()
            assert Path(guard.__file__).resolve() == service.GUARD
            assert original.sha(guard.__file__) == service.GUARD_SHA == actual['pins'][str(service.GUARD)]
            receipt = output / 'entry-repair.json'
            original.save_new(receipt, dict(status='exact-private-v5-count-entry-service-binding-repaired',
                repaired_function_source=str(SOURCE), original_count_runner=str(original.PACKET / 'replay_sources.py'),
                adapter='private-v5 preparation.load_service().load_guard()',
                preparation=dict(path=str(expected), sha256=actual['pins'][str(expected)]),
                actual_input_service=dict(path=str(preparation.SERVICE), sha256=actual['pins'][str(preparation.SERVICE)]),
                actual_native_guard=dict(path=str(service.GUARD), sha256=service.GUARD_SHA),
                original_fixture_and_counter_bytes_changed=False, actual_fixtures_retained=34,
                new_fixture_runs=0, benchmark_processes_started=0))
            repair_pins[str(receipt)] = original.sha(receipt)
            return guard

        # The added interface is an explicit adapter in this count entry only.
        # No original preparation file or source/parser fixture is rewritten.
        preparation.load_guard = load_actual_guard
        return value

    # Keep the proxy local to the original runner; global importlib is intact.
    original.importlib = SimpleNamespace(util=importlib.util, import_module=bound_import)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--language', choices=SETTINGS, required=True)
    parser.add_argument('--revision', required=True)
    args = parser.parse_args()
    assert re.fullmatch(r'[a-z0-9][a-z0-9-]{0,63}', args.revision)
    assert args.revision != 'source20-restored-v2'
    settings = SETTINGS[args.language]
    packet = settings['packet']
    script = packet / settings['script']
    fixture = packet / 'jobs' / settings['fixture'] / 'fixture-receipt.json'
    failed = packet / 'jobs/source20-restored-v2'
    output = packet / 'jobs' / args.revision
    print('WAIT common mutex: unchanged fixture/count-entry repair', args.language, flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        assert not output.exists(), 'Exclusive additive count revision; no overwrite/retry'
        # Verify before import, then delegate to the original mutex-owning run.
        import hashlib
        def sha(path):
            with Path(path).open('rb') as stream:
                return hashlib.file_digest(stream, 'sha256').hexdigest()
        assert sha(script) == settings['script_sha']
        assert sha(fixture) == settings['fixture_sha']
        spec = importlib.util.spec_from_file_location('preserved_' + args.language + '_source_count_entry', script)
        original = importlib.util.module_from_spec(spec)
        sys.dont_write_bytecode = True
        spec.loader.exec_module(original)
        repair_pins = {str(SOURCE): sha(SOURCE), str(script): settings['script_sha']}
        failed_terminal = json.loads((failed / 'terminal-pins.json').read_text())
        assert failed_terminal['status'] == 'failed-stop'
        original.verify(failed_terminal['pins'])
        original.verify(failed_terminal['prerequisite_pins'])
        for path, digest in failed_terminal['prerequisite_pins'].items():
            assert path not in repair_pins or repair_pins[path] == digest
            repair_pins[path] = digest
        for path in failed.iterdir():
            if path.is_file():
                repair_pins[str(path)] = sha(path)
        if args.language == 'nim':
            stopped = json.loads((failed / 'stopped-report.json').read_text())
            assert stopped['compiler_commands_started'] == 0 and stopped['error_type'] == 'AssertionError'
            repaired_nim(original, repair_pins, output)
        else:
            stopped = json.loads((failed / 'failure.json').read_text())
            assert stopped['error_type'] == 'AttributeError'
            assert "has no attribute 'load_guard'" in stopped['error']
            repaired_gleam(original, repair_pins, output)
    original.run(SimpleNamespace(revision=args.revision, fixtures_only=False,
        fixture_receipt=fixture, fixture_sha256=settings['fixture_sha']))


if __name__ == '__main__':
    main()
