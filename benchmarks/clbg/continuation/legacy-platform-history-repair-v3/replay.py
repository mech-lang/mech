"""Additive exact-chart/source repair; extend and rerun the existing ten IDs."""
import argparse
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
PACKET = CLBG/'continuation/existing-native-platform-proposal-v1'
FAILED = PACKET/'jobs/fixtures-v1'
LOCK = Path('/tmp/mech-clbg-timing.lock')
RECEIPT_SHA = '01cf7f23accd926c6866218c5ccebe7b4a51c0b80e89ca8255ef22d791013be3'
LOG_SHA = '70a3ad5f4d85de5e3c4a1c53ba5954547c925e911ebfb885c5df04293755e9c6'
V2 = CLBG/'continuation/legacy-platform-entry-repair-v2'
HISTORY_FAILED = PACKET/'jobs/history-entry-repaired-v2'
HISTORY_FAILED_SHA = '5514a3482e56725605544519850e73839b975e295575f8a2e6b24916f0317ccc'
OLD_FIXTURE_SHA = '5d7c59f1f47796d0a103f7620a7cc47487b946c5384a62c86df7596a3e357271'
OLD_FIXTURE_TERMINAL_SHA = '1b6ba50635abae06ec8ea83f2b4adbc08dd1b1daaa0b7c213eb86487248d13e9'
OLD_DURABLE = CLBG/'continuation/owned-jobs/legacy133-platform-history-qualified-v2'
OLD_DURABLE_PINS = {
    'request.json': '583391af3559659a1f2317e3d68c46c88c6873b529c528d5194ede9bf557885a',
    'launch.json': '26240f6ad5d8eedd75eb9fb8d9efa506ffaf44e1502c3983c3d9439fb4ff47ff',
    'worker-started.json': '153893f7e1191dc723531795930de973dd0ef8b916aa75dbd66ffaa3e75ff44f',
    'command-started.json': '336ed1833b44b0eed154dfbd6ed3ac28dbc9b4c99d6ee71e06f7534d817c3ab5',
    'terminal.json': '09fb9e93c901aeadc1d1b4334d205b664e12bf666a7b2e6929168554ef246051',
}
ORIGINAL_FILES = {
    'guarded_qualify.py': 'ec9aa84f88c75ac62ff31c2ff801dfb60f34e5693f7fd09fabd2afc934737dc7',
    'qualify_history.py': '73f882444382630717701c556d695614dc33161e83a2a1738136a45ee10b1adb',
    'adapter.py': '90b6052eaa14f960a6fb2510364335cf87f43d4d62998a015223db42ae516342',
    'test_history.py': '19479b0f33f731c3873b58877ad2e42c0568ee43837ee9992f11d10a5ef29cd5',
    'test_adapter.py': '3e8c16642057f735c3dced79ca8bddde01e65f7d9b4fb9b2e0f66f667fca8ae2',
    'expected-program-keys.json': 'ee19373b3d0185ebcf1b7f7e52f43fd43004c2ab4ba45675aced44bc652e6140',
    'routes.json': '88dae9329d7e509f17b09f1794408938e9b73270b72824e4f93325f43f7ec191',
    'REVIEW.md': '2939009928ab312aaf3457783806bc6b6aff800dc923f3bb06149bd5442c5691',
    'extract_existing_evidence.jq': '2b11ff7c6131ae36e4576f537442dfa9293323f7080c3b1b4c4edcdf4b0a1512',
}


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def verify(pins):
    for path, digest in pins.items():
        assert sha(path) == digest, ('Original/adapter/evidence bytes changed', path)


def merge(pins, values):
    for path, digest in values.items():
        assert path not in pins or pins[path] == digest
        pins[path] = digest


def save(path, value):
    with path.open('x') as stream:
        json.dump(value, stream, indent=2); stream.write('\n')


def prior_failure(pins):
    receipt_path, terminal_path = FAILED/'fixture-receipt.json', FAILED/'terminal.json'
    log_path = FAILED/'fixtures.log'
    assert sha(receipt_path) == RECEIPT_SHA and sha(log_path) == LOG_SHA
    receipt, terminal = (json.loads(path.read_text()) for path in (receipt_path, terminal_path))
    assert terminal['status'] == 'failed-stop' and terminal['integrity_error'] is None
    assert terminal['body_error'] == dict(error='AssertionError', error_type='AssertionError')
    assert terminal['available_bytes_not_fully_pinned'] is False
    assert receipt['status'] == 'failed-fixtures' and receipt['execution_completed'] is True
    assert receipt['successful'] is False and type(receipt['tests_run']) is int and receipt['tests_run'] == 10
    for key in ('expected_ids', 'discovered_ids', 'started_ids'):
        assert len(receipt[key]) == len(set(receipt[key])) == 10
        assert set(receipt[key]) == set(receipt['expected_ids'])
    assert len(receipt['failures']) == 1 and receipt['failures'][0][0] == 'test_history.HistoryEvidence.test_byte_identity_cannot_be_borrowed_from_equal_basename_or_version'
    assert "None != ('/later/node'" in receipt['failures'][0][1]
    assert all(receipt[key] == [] for key in ('errors', 'skipped', 'expected_failures', 'unexpected_successes'))
    for artifact in (receipt, terminal):
        assert artifact['exclusive_common_mutex_held'] is True and artifact['common_timing_mutex'] == str(LOCK)
        assert all(type(artifact[key]) is int and artifact[key] == 0 for key in ('actual_native_processes', 'runtime_version_reruns', 'benchmark_replays'))
        assert artifact['chart_admission'] is False
    assert terminal['pins'][str(receipt_path)] == RECEIPT_SHA and terminal['pins'][str(log_path)] == LOG_SHA
    for name, digest in ORIGINAL_FILES.items():
        assert terminal['pins'][str(PACKET/name)] == receipt['pins'][str(PACKET/name)] == digest
    verify(terminal['pins']); verify(receipt['pins'])
    merge(pins, terminal['pins']); merge(pins, receipt['pins'])
    merge(pins, {str(terminal_path): sha(terminal_path)})
    return receipt


def prior_history_failure(pins):
    old_sources = {
        str(V2/'replay.py'): '4c30fd63a95b2694875f13aecaf713798184650dd051a5a00b90a06f6b48d514',
        str(V2/'REVIEW.md'): '06ab45a21ba0b5c6804ad47c7613a57521195e4e544b69e3e074a6603fb15fc4',
        **{str(OLD_DURABLE/name): digest for name, digest in OLD_DURABLE_PINS.items()},
        str(HISTORY_FAILED/'terminal.json'): HISTORY_FAILED_SHA,
    }
    verify(old_sources); merge(pins, old_sources)
    request, launch, worker, started, terminal = (
        json.loads((OLD_DURABLE/name).read_bytes()) for name in OLD_DURABLE_PINS)
    command = ['/opt/homebrew/opt/python@3.14/bin/python3.14', '-B', '-u', str(V2/'replay.py'),
               '--stage', 'qualify', '--revision', 'history-entry-repaired-v2', '--fixture-receipt',
               str(PACKET/'jobs/fixtures-entry-repaired-v2/fixture-receipt.json'),
               '--fixture-sha256', OLD_FIXTURE_SHA, '--fixture-terminal-sha256', OLD_FIXTURE_TERMINAL_SHA]
    assert request['command'] == launch['command'] == worker['command'] == started['command'] == command
    assert request['cwd'] == launch['cwd'] == worker['cwd'] == started['cwd'] == str(CLBG.parents[1])
    assert request['common_mutex_owned_by_target_command'] is worker['common_mutex_owned_by_target_command'] is True
    assert worker['inherited_private_lease'] is worker['detached_session'] is True
    assert worker['worker_identity'] == terminal['worker_identity']
    assert launch['worker_identity']['pid'] == worker['worker_identity']['pid'] == 94734
    assert started['command_identity'] == terminal['command_identity'] and started['command_pid'] == terminal['command_pid'] == 94740
    assert started['command_identity_observation_missing'] is terminal['command_identity_observation_missing'] is False
    assert terminal['status'] == 'command-failed' and type(terminal['exit_code']) is int and terminal['exit_code'] == 1
    assert terminal['actual_command_started'] is True and terminal['stop_signals'] == []
    assert terminal['qualification_granted'] is terminal['automatic_retry'] is False
    outcome = json.loads((HISTORY_FAILED/'terminal.json').read_bytes())
    assert outcome['status'] == 'failed-stop' and outcome['integrity_error'] is None and outcome['available_bytes_not_fully_pinned'] is False
    assert outcome['body_error'] == dict(error='Exact historical row selector ambiguous or absent: binarytrees:mech-binarytrees-parallel', error_type='ValueError')
    assert outcome['exclusive_common_mutex_held'] is True and outcome['common_timing_mutex'] == str(LOCK)
    assert all(type(outcome[name]) is int and outcome[name] == 0 for name in ('actual_native_processes', 'runtime_version_reruns', 'benchmark_replays'))
    assert outcome['chart_admission'] is False
    assert not (HISTORY_FAILED/'qualified-history.json').exists() and not (HISTORY_FAILED/'qualification-receipt.json').exists()
    verify(outcome['pins']); merge(pins, outcome['pins'])


def cold_module(name, path, pins):
    assert name not in sys.modules and sha(path) == pins[str(path)]
    source = path.read_bytes(); assert hashlib.sha256(source).hexdigest() == pins[str(path)]
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec); sys.modules[name] = module
    exec(compile(source, str(path), 'exec'), module.__dict__)
    assert module is sys.modules[name] and Path(module.__file__).resolve() == path
    verify(pins); return module


def bind(original, repair_pins, output, stage, repair, extensions):
    snapshot, fresh, prior_fixture = original.snapshot, original.fresh_module, original.prior_fixture

    def pinned_snapshot(directory, pins):
        original.merge(pins, repair_pins)
        result = snapshot(directory, pins)
        original.verify(pins)
        return result

    def current_fixture(args, pins):
        prior_fixture(args, pins)
        path = args.fixture_receipt.resolve(strict=True)
        fixture, terminal = (json.loads(item.read_bytes()) for item in (path, path.parent/'terminal.json'))
        for name in ('replay.py', 'history_repair.py', 'test_history_extensions.py', 'REVIEW.md'):
            authored = str(HERE/name)
            assert fixture['pins'][authored] == terminal['pins'][authored] == repair_pins[authored] == pins[authored]
        bindings_path, helper_path = path.parent/'fixture-extension-bindings.json', path.parent/'helper-entry-repair.json'
        for item in (bindings_path, helper_path):
            assert sha(item) == fixture['pins'][str(item)] == terminal['pins'][str(item)]
        bindings, helper = (json.loads(item.read_bytes()) for item in (bindings_path, helper_path))
        assert bindings['status'] == 'bound-two-extended-existing-fixture-methods'
        assert bindings['expected_fixture_ids'] == original.IDS and type(bindings['expected_fixture_count']) is int and bindings['expected_fixture_count'] == 10
        assert bindings['extension_source'] == str(HERE/'test_history_extensions.py')
        assert bindings['extension_source_sha256'] == pins[bindings['extension_source']]
        assert bindings['original_fixture_sources_changed'] is False and bindings['original_assertions_executed_first'] is True
        assert set(bindings['methods']) == {'test_duplicate_or_wrong_language_record_is_never_a_sample_join', 'test_native_link_requires_exact_output_and_cargo_artifact_pin'}
        for method in bindings['methods'].values():
            assert method['original_method_source'] == str(path.parent/'snapshot/test_history.py')
            assert method['extension_method_source'] == method['extra_assertion_source'] == str(HERE/'test_history_extensions.py')
            assert method['full_original_method_executed_first'] is True
        assert helper['status'] == 'exact-chart-and-source-helper-repaired-awaiting-current-fixtures-or-history' and helper['stage'] == 'fixtures'
        assert helper['repair_source'] == str(HERE/'history_repair.py') and helper['repair_source_sha256'] == pins[helper['repair_source']]
        original.verify(pins)

    def repaired_module(name, path, pins):
        module = fresh(name, path, pins)
        if name == 'test_history':
            bindings = extensions.extend(module, sys.modules['qualify_history'])
            receipt = output/'fixture-extension-bindings.json'
            original.new(receipt, dict(status='bound-two-extended-existing-fixture-methods',
                methods=bindings, expected_fixture_ids=original.IDS, expected_fixture_count=10,
                extension_source=str(HERE/'test_history_extensions.py'),
                extension_source_sha256=pins[str(HERE/'test_history_extensions.py')],
                original_fixture_sources_changed=False, original_assertions_executed_first=True,
                actual_native_processes=0, runtime_version_reruns=0, benchmark_replays=0))
            original.merge(pins, {str(receipt): sha(receipt)}); original.verify(pins)
            return module
        if name != 'qualify_history':
            return module
        assert path.name == 'qualify_history.py' and sha(path) == ORIGINAL_FILES[path.name]
        preserved_pin = module.historical_pin

        def exact_alias_pin(measured, cmd):
            # The old helper always prepends cmd[0]. Remove only duplicate exact
            # strings from its complete alias list, then give it the remainder.
            # Distinct paths, runtime versions, hashes and all lookup rules stay.
            assert type(cmd) is list and cmd and type(cmd[0]) is str
            saved = measured['saved']
            aliases = [cmd[0], *saved.get('command_assets', [])]
            assert all(type(alias) is str for alias in aliases)
            unique = list(dict.fromkeys(aliases))
            adapted = dict(measured, saved=dict(saved, command_assets=unique[1:]))
            return preserved_pin(adapted, cmd)

        module.historical_pin = exact_alias_pin
        semantic_changes = repair.install(module)
        receipt = output/'helper-entry-repair.json'
        original.new(receipt, dict(status='exact-chart-and-source-helper-repaired-awaiting-current-fixtures-or-history',
            stage=stage, repaired_function_source=str(Path(__file__).resolve()),
            original_core=str(path), original_core_sha256=ORIGINAL_FILES[path.name],
            adapter='deduplicate exact alias strings before original historical_pin lookup',
            additional_semantic_changes=semantic_changes,
            repair_source=str(HERE/'history_repair.py'), repair_source_sha256=pins[str(HERE/'history_repair.py')],
            preserved_failed_history=dict(path=str(HISTORY_FAILED/'terminal.json'), sha256=HISTORY_FAILED_SHA),
            distinct_path_or_version_only_rules_changed=False, original_nine_files=ORIGINAL_FILES,
            original_source_files_changed=False, helper_semantics_changed=True,
            preserved_failed_fixture=dict(path=str(FAILED/'fixture-receipt.json'), sha256=RECEIPT_SHA),
            prior_completed_fixture_ids=10, prior_successful_fixture_ids=9,
            prior_failed_fixture_ids=1, prior_fixture_success_reused=False,
            current_fixture_ids_required=10, actual_native_processes=0,
            benchmark_replays=0, chart_admission=False, dataset_mutated=False))
        original.merge(pins, {str(receipt): sha(receipt)})
        original.verify(pins)
        return module

    # Process-local hooks only. The frozen guard/core/test files and global
    # importlib remain unchanged. The existing test module's two bound methods
    # explicitly run the frozen assertions plus independently pinned additions.
    original.snapshot, original.fresh_module, original.prior_fixture = pinned_snapshot, repaired_module, current_fixture


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--stage', choices=('fixtures', 'qualify'), required=True)
    parser.add_argument('--revision', required=True)
    parser.add_argument('--fixture-receipt', type=Path)
    parser.add_argument('--fixture-sha256'); parser.add_argument('--fixture-terminal-sha256')
    args = parser.parse_args()
    assert re.fullmatch(r'[a-z0-9][a-z0-9-]{0,63}', args.revision) and args.revision != 'fixtures-v1'
    if args.stage == 'qualify':
        assert all((args.fixture_receipt, args.fixture_sha256, args.fixture_terminal_sha256))
    output, preflight = PACKET/'jobs'/args.revision, HERE/'jobs'/args.revision
    print('WAIT common mutex: exact-chart/source legacy-platform history repair', flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        held, named = os.fstat(lock.fileno()), LOCK.stat()
        assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino)
        assert not output.exists() and not preflight.exists(), 'Exclusive additive metadata attempt; no overwrite/resume'
        preflight.mkdir(parents=True)
        pins = {}; error = None; original = None
        try:
            for path in (Path(__file__).resolve(), HERE/'history_repair.py', HERE/'test_history_extensions.py', HERE/'REVIEW.md'):
                pins[str(path)] = sha(path)
            failed = prior_failure(pins)
            prior_history_failure(pins)
            script = PACKET/'guarded_qualify.py'
            assert sha(script) == ORIGINAL_FILES[script.name]
            name = 'preserved_legacy133_platform_source_bound_guard_entry'
            assert name not in sys.modules
            source = script.read_bytes()
            assert hashlib.sha256(source).hexdigest() == ORIGINAL_FILES[script.name]
            spec = importlib.util.spec_from_file_location(name, script)
            original = importlib.util.module_from_spec(spec); sys.modules[name] = original
            sys.dont_write_bytecode = True
            exec(compile(source, str(script), 'exec'), original.__dict__)
            assert original is sys.modules[name] and Path(original.__file__).resolve() == script
            assert original.HERE == PACKET and original.CLBG == CLBG
            assert set(original.FILES) == set(ORIGINAL_FILES) and original.IDS == sorted(failed['expected_ids'])
            repair = cold_module('legacy133_exact_chart_source_repair', HERE/'history_repair.py', pins)
            extensions = cold_module('legacy133_source_bound_fixture_extensions', HERE/'test_history_extensions.py', pins)
            bind(original, pins, output, args.stage, repair, extensions)
            verify(pins)
        except BaseException as failure:
            error = failure
            (preflight/'actual-error.log').write_text(traceback.format_exc())
            raise
        finally:
            captured, failures = {}, []
            candidates = dict(pins)
            candidates.update({str(path): None for path in preflight.rglob('*') if path.is_file()})
            for path, expected in candidates.items():
                try:
                    digest = sha(path)
                    assert expected is None or digest == expected
                    captured[path] = digest
                except BaseException as failure:
                    failures.append(dict(path=path, error=str(failure) or type(failure).__name__))
            receipt = preflight/'terminal.json'
            save(receipt, dict(status='failed-terminal-integrity' if failures else 'failed-preflight' if error else 'prepared-for-original-guard-run',
                source_pin_errors=failures, available_bytes_not_fully_pinned=bool(failures), pins=captured,
                error=str(error) or type(error).__name__ if error else None,
                actual_common_mutex_held=True, native_processes_started=0,
                current_fixture_ids_started=0, benchmark_replays=0, chart_admission=False))
            if failures:
                raise RuntimeError('Preflight terminal integrity failed; available bytes preserved')
            pins[str(receipt)] = sha(receipt)
    # Original.run owns its own common-lock FD; never wait while holding ours.
    # Its snapshot hook first verifies all wrapper/failed-stage source pins.
    original.run(args)


if __name__ == '__main__':
    main()
