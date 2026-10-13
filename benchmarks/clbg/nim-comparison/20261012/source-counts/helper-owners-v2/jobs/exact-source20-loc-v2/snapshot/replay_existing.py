"""Offline LOC seal of the closed actual Nim source20 replay; no commands run.

Execute only after full root/peer acceptance of these five frozen source files.
The reviewed --review-sha256 is the caller's acceptance of the whole20 source
meaning packet, not a claim that a parser or compiler alone approved meaning.
"""
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
NIM = HERE.parents[1]
CLBG = NIM.parents[1]
ROOT = CLBG.parents[1]
REPLAY_ROOT = CLBG / 'loc-normalization/20261012/nim-source-replay-v1'
OLD = REPLAY_ROOT / 'jobs/source20-entry-repaired-v4'
PROPOSAL = OLD / 'source-count-proposals.json'
PROPOSAL_SHA = 'be3d2203a54d8528fcab612905182e514e8342e36865e0c5fb5d65bec5aa572a'
PREPARATION = NIM / 'qualification/private-v7/report.json'
PREPARATION_SHA = '6fb529f349ffaa8694d87a79c906ec86b84ecae2a23f5a7ab63a04f43a3f07da'
SUITE = NIM / 'reconciliation/full-v6-suite-v2.json'
DURABLE = CLBG / 'continuation/owned-jobs/nim-source20-entry-repaired-v4'
FAILED = HERE.with_name('helper-owners-v1') / 'jobs/exact-source20-loc-v1'
FAILED_OWNER = CLBG / 'continuation/owned-jobs/nim-exact-source20-loc-v1'
FAILED_OUTCOME_SHA = '3333db46ecff4bbfadd6971f329c2b613e33209ee12b064e349dd2c4dfa35df9'
FAILED_DURABLE_SHA = '6bfdb28c3adfbbdae3f3872a9502a0c3aa968d6c4ae912f7c355afdaea54d70b'
FAILED_FIXTURE_SHA = '0bfa8de6a71dd42200d39b4a94c40c019a639f3b16e07ecc7ab20ddf311170cf'
LOCK = Path('/tmp/mech-clbg-timing.lock')
SOURCE_FIXTURE_SHA = 'aec967282855da29084de8d445b6f8311c42f3c1be5963f162078e1f167e2e52'
HELPER_SHA = 'adad6363f378372c1f54b8382193a70c8e142565589bb2511ba6567838f8b3e4'
FILES = ('replay_existing.py', 'helper_owners.py', 'test_helper_owners.py', 'meaning-review.json', 'REVIEW.md')
METHODS = ('test_typed_declarations_and_independent_bodies',
    'test_wrapping_keeps_owner_units_and_exact_spans',
    'test_wrong_forwarding_and_extra_action_are_rejected',
    'test_unknown_include_generic_and_returned_type_are_rejected',
    'test_existing_proc_composition_policy_is_unchanged')
IDS = sorted('test_helper_owners.HelperOwnerFixtures.' + name for name in METHODS)


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def merge(pins, values):
    for path, digest in values.items():
        assert path not in pins or pins[path] == digest, ('Conflicting input pin', path)
        pins[path] = digest


def bind(pins, path, expected=None):
    path = Path(path)
    digest = sha(path)
    assert expected is None or digest == expected, ('Changed source/evidence', str(path))
    merge(pins, {str(path): digest})
    return digest


def read(pins, path, expected=None):
    bind(pins, path, expected)
    return json.loads(Path(path).read_text())


def verify(pins):
    for path, digest in pins.items():
        assert sha(path) == digest, ('Changed source/evidence', path)


def save(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')
        stream.flush()
        os.fsync(stream.fileno())


def load(name, path, pins):
    assert name not in sys.modules
    bind(pins, path, pins[str(path)])
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    assert Path(module.__file__).resolve() == path.resolve()
    return module


def closed_replay(args, pins):
    command = ['/opt/homebrew/opt/python@3.14/bin/python3.14', '-B', '-u',
               str(CLBG / 'continuation/launch_nim_source20_v4.py')]
    request = read(pins, DURABLE / 'request.json')
    started = read(pins, DURABLE / 'command-started.json')
    worker = read(pins, DURABLE / 'worker-started.json')
    terminal = read(pins, DURABLE / 'terminal.json', args.durable_terminal_sha256)
    assert request['schema_version'] == 1
    assert all(value['command'] == command and value['cwd'] == str(ROOT)
               for value in (request, started, worker))
    assert request['qualification_granted'] is request['automatic_retry'] is False
    assert request['common_mutex_owned_by_target_command'] is True
    assert worker['inherited_private_lease'] is worker['detached_session'] is True
    assert worker['worker_identity'] == terminal['worker_identity']
    assert terminal['status'] == 'command-exited-zero-awaiting-report-review'
    assert type(terminal['exit_code']) is int and terminal['exit_code'] == 0
    assert terminal['actual_command_started'] is True and terminal['stop_signals'] == []
    assert terminal['qualification_granted'] is terminal['automatic_retry'] is False
    assert started['command_identity_observation_missing'] is terminal['command_identity_observation_missing'] is False
    assert started['command_pid'] == terminal['command_pid'] == 20457
    assert started['command_identity'] == terminal['command_identity']
    assert terminal['worker_identity']['pid'] == 20453
    listing = terminal['command_identity']['listing'].split(None, 7)
    assert listing[:2] == ['20457', '20453'] and listing[7] == ' '.join(command)
    output_terminal = read(pins, OLD / 'terminal-pins.json', args.output_terminal_sha256)
    assert output_terminal['status'] == 'completed-source-count-proposals'
    assert output_terminal['actual_common_mutex_held'] is True
    assert output_terminal['canonical_LOC_admitted'] is output_terminal['chart_admission'] is False
    assert output_terminal['pins'][str(PROPOSAL)] == PROPOSAL_SHA
    merge(pins, output_terminal['pins'])
    merge(pins, output_terminal['prerequisite_pins'])
    verify(pins)
    return dict(durable_terminal=dict(path=str(DURABLE / 'terminal.json'), sha256=args.durable_terminal_sha256),
                output_terminal=dict(path=str(OLD / 'terminal-pins.json'), sha256=args.output_terminal_sha256))


def failed_v1_closure(args, pins):
    """Retain actual prior failure; do not turn its four passes into a seal."""
    assert args.failed_outcome_sha256 == FAILED_OUTCOME_SHA
    assert args.failed_durable_terminal_sha256 == FAILED_DURABLE_SHA
    done = read(pins, FAILED_OWNER / 'terminal.json', args.failed_durable_terminal_sha256)
    started = read(pins, FAILED_OWNER / 'command-started.json')
    worker = read(pins, FAILED_OWNER / 'worker-started.json')
    request = read(pins, FAILED_OWNER / 'request.json')
    assert done['status'] == 'command-failed' and type(done['exit_code']) is int and done['exit_code'] == 1
    assert done['actual_command_started'] is True and done['stop_signals'] == []
    assert done['qualification_granted'] is done['automatic_retry'] is False
    assert done['worker_identity'] == worker['worker_identity']
    assert done['command_identity'] == started['command_identity']
    assert done['command_pid'] == started['command_pid'] == 22090 and done['worker_identity']['pid'] == 22085
    assert started['command_identity_observation_missing'] is done['command_identity_observation_missing'] is False
    assert request['command'] == worker['command'] == started['command'] and request['cwd'] == worker['cwd'] == started['cwd'] == str(ROOT)
    assert request['command'][:4] == ['/opt/homebrew/opt/python@3.14/bin/python3.14', '-B', '-u', str(HERE.with_name('helper-owners-v1') / 'replay_existing.py')]
    observed = done['command_identity']['listing'].split(None, 7)
    framework = '/opt/homebrew/Cellar/python@3.14/3.14.8/Frameworks/Python.framework/Versions/3.14/Resources/Python.app/Contents/MacOS/Python'
    assert observed[:2] == ['22090', '22085']
    assert observed[7].split() == [framework, *request['command'][1:]]
    outcome = read(pins, FAILED / 'outcome.json', args.failed_outcome_sha256)
    assert outcome['status'] == 'failed-before-output' and outcome['error_type'] == 'AssertionError'
    assert outcome['actual_common_mutex_held'] is True and outcome['available_bytes_not_fully_pinned'] is False
    assert outcome['actual_compiler_processes'] == outcome['actual_benchmark_processes'] == 0
    merge(pins, outcome['prerequisite_pins'])
    merge(pins, outcome['actual_terminal_evidence_pins'])
    fixture_path = FAILED / 'helper-fixture-receipt.json'
    assert outcome['actual_terminal_evidence_pins'][str(fixture_path)] == FAILED_FIXTURE_SHA
    fixture = read(pins, fixture_path, outcome['actual_terminal_evidence_pins'][str(fixture_path)])
    assert fixture['status'] == 'failed-helper-owner-fixtures' and fixture['tests_run'] == 5 and fixture['failures'] == 1
    assert all(fixture[key] == 0 for key in ('errors', 'skipped', 'expected_failures', 'unexpected_successes', 'actual_compiler_processes', 'actual_benchmark_processes'))
    assert sorted(fixture['expected_ids']) == sorted(fixture['discovered_ids']) == sorted(fixture['started_ids']) == IDS
    assert not (FAILED / 'qualified-loc-overlay.json').exists()
    failure_log = (FAILED / 'helper-fixtures.log').read_text()
    assert 'FAIL: test_wrapping_keeps_owner_units_and_exact_spans' in failure_log
    assert 'AssertionError: 24 != 23' in failure_log
    original = (FAILED / 'snapshot/test_helper_owners.py').read_bytes()
    before = b"        wrapped = SOURCE.replace(' = mpz_', ' =\\n  mpz_')\n"
    after = b"        wrapped = ''.join(line.replace(' = mpz_', ' =\\n  mpz_', 1)\n            if line.startswith('template ') else line\n            for line in SOURCE.splitlines(keepends=True))\n"
    assert original.count(before) == 1
    assert (HERE / 'test_helper_owners.py').read_bytes() == original.replace(before, after)
    for name in ('helper_owners.py', 'meaning-review.json'):
        assert (HERE / name).read_bytes() == (FAILED / 'snapshot' / name).read_bytes()
    verify(pins)
    return dict(status='actual-failed-v1-preserved-only-test-wrapping-corrected',
        failed_outcome=dict(path=str(FAILED / 'outcome.json'), sha256=args.failed_outcome_sha256),
        failed_durable_terminal=dict(path=str(FAILED_OWNER / 'terminal.json'), sha256=args.failed_durable_terminal_sha256),
        prior_failure_relabelled_as_success=False, original_rule_and_meaning_bytes_preserved=True)


def fixtures(directory, normalizer, helper, owner, old, pins):
    # Reuse the original passed input and grammar fixtures.  Only five new
    # helper assertions execute, once, against the same frozen base grammar.
    from types import SimpleNamespace
    path = Path(old['source_input_fixture']['path'])
    assert old['source_input_fixture']['sha256'] == SOURCE_FIXTURE_SHA
    owner.fixture_prerequisite(SimpleNamespace(fixture_receipt=path,
        fixture_sha256=SOURCE_FIXTURE_SHA), pins)
    prior = json.loads(path.read_text())
    runtime = Path(prior['unittest_module']).resolve().parents[1]
    assert not any(name == 'unittest' or name.startswith('unittest.') for name in sys.modules)
    sys.path.insert(0, str(runtime))
    try:
        import unittest
    finally:
        sys.path.pop(0)
    assert Path(unittest.__file__).resolve() == Path(prior['unittest_module']).resolve()
    assert sha(unittest.__file__) == prior['unittest_module_sha256']
    tests = load('test_helper_owners', directory / 'test_helper_owners.py', pins)
    suite = unittest.defaultTestLoader.loadTestsFromModule(tests)
    def ids(group):
        result = []
        for item in group:
            result.extend(ids(item) if isinstance(item, unittest.TestSuite) else [item.id()])
        return result
    discovered = ids(suite)
    assert sorted(discovered) == IDS and len(discovered) == len(set(discovered)) == 5
    started = []
    class Result(unittest.TextTestResult):
        def startTest(self, test):
            started.append(test.id())
            super().startTest(test)
    with (directory.parent / 'helper-fixtures.log').open('x') as stream:
        result = unittest.TextTestRunner(stream=stream, verbosity=2, resultclass=Result).run(suite)
    receipt = dict(status='passed' if result.wasSuccessful() else 'failed-helper-owner-fixtures',
        fixture_kind='additive-Nim-helper-owner-fixtures',
        expected_ids=IDS, discovered_ids=discovered, started_ids=started, tests_run=result.testsRun,
        failures=len(result.failures), errors=len(result.errors), skipped=len(result.skipped),
        expected_failures=len(result.expectedFailures), unexpected_successes=len(result.unexpectedSuccesses),
        original_input_fixture=old['source_input_fixture'], original_grammar_fixture=old['grammar_fixture'],
        original_fixture_methods_rerun=0, actual_common_mutex_held=True,
        actual_compiler_processes=0, actual_benchmark_processes=0, python_version=sys.version,
        unittest_module=str(Path(unittest.__file__).resolve()), fixture_source_pins=dict(pins))
    save(directory.parent / 'helper-fixture-receipt.json', receipt)
    assert sorted(started) == IDS and result.testsRun == 5 and result.wasSuccessful()
    assert not any(receipt[name] for name in ('failures', 'errors', 'skipped', 'expected_failures', 'unexpected_successes'))
    return receipt


def seal(args, output, pins):
    failure_closure = failed_v1_closure(args, pins)
    closure = closed_replay(args, pins)
    old = read(pins, PROPOSAL, PROPOSAL_SHA)
    preparation = read(pins, PREPARATION, PREPARATION_SHA)
    suite = read(pins, SUITE, args.suite_sha256)
    assert suite['language'] == 'nim' and suite['status'] == 'ten-native-workloads-reconciled-awaiting-canonical-LOC-and-publication'
    assert suite['canonical_full_native_processes'] == 90 and suite['candidate_sources'] == 20 and suite['actual_samples_per_source'] == 3
    assert len(suite['benchmarks']) == 10
    merge(pins, suite['provenance'])
    assert old['status'] == 'actual20-Nim-compiler-input-and-LOC18-proposals-awaiting-semantic-review'
    assert old['profile_count'] == 20 and len(old['source_counts']) == 21
    assert old['qualification'] == dict(path=str(PREPARATION), sha256=PREPARATION_SHA)
    assert old['actual_frontend_compiler_commands'] == 20 and old['actual_native_backend_compilations'] == old['actual_benchmark_processes'] == 0
    assert old['canonical_LOC_admitted'] is old['chart_admission'] is False
    merge(pins, old['pins'])
    directory = output / 'snapshot'
    directory.mkdir()
    expected = (args.worker_sha256, args.owners_sha256, args.tests_sha256, args.review_sha256, args.notes_sha256)
    for name, digest in zip(FILES, expected):
        original, copied = HERE / name, directory / name
        bind(pins, original, digest)
        assert not original.stat().st_mode & 0o222, ('Source packet must be frozen', name)
        copied.write_bytes(original.read_bytes())
        copied.chmod(0o444)
        bind(pins, copied, digest)
    review = read(pins, directory / 'meaning-review.json', args.review_sha256)
    assert review['status'] == 'author-full20-main-and-helper-meaning-review-awaiting-independent-source-acceptance'
    assert review['actual_source_replay_sha256'] == PROPOSAL_SHA and review['unresolved'] == []
    assert review['author_read_all_twenty_main_sources'] is review['author_read_all_twenty_actual_canonical_event_arrays'] is True
    assert review['canonical_LOC_admitted'] is review['chart_admission'] is False
    worker = load('nim_original_input_receipt_service', REPLAY_ROOT / 'qualify_replay.py', pins)
    normalizer = worker.grammar(pins)
    helper = load('helper_owners', directory / 'helper_owners.py', pins)
    fixture = fixtures(directory, normalizer, helper, worker, old, pins)
    adapter = load('nim_actual_processing_receipt_adapter', OLD / 'snapshot/input_inventory.py', pins)
    rows = {row['id']: row for row in preparation['profiles']}
    assert len(rows) == len(preparation['profiles']) == 20
    assert set(rows) == set(old['profiles']) == set(old['compiler_bound_input_inventory']) == set(review['main_counts'])
    saved_candidates, actual_samples = {}, 0
    for benchmark, workload in suite['benchmarks'].items():
        assert len(workload['profiles']) == 3
        for identifier, profile in workload['profiles'].items():
            assert profile['benchmark'] == benchmark
            assert len(profile['samples_seconds']) == len(profile['sample_execution_metadata']) == len(profile['sample_validation']) == 3
            assert all(sample['qualified'] is sample['full_performance_run'] is True
                       for sample in profile['sample_execution_metadata'])
            assert all(item['reference_match'] is True for item in profile['sample_validation'])
            actual_samples += len(profile['sample_execution_metadata'])
            if profile['language'] == 'nim':
                assert identifier not in saved_candidates
                saved_candidates[identifier] = profile
            else:
                assert profile['language'] == 'mech' and identifier == 'mech-aot-control'
    assert actual_samples == 90 and set(saved_candidates) == set(rows)
    counts = {}
    for path, counted in old['source_counts'].items():
        source, origin = Path(path), Path(counted['origin'])
        assert counted['original_bytes_equal'] is True and counted['generated_source_substitute'] is False
        assert source.read_bytes() == origin.read_bytes()
        bind(pins, source, counted['source_sha256'])
        bind(pins, origin, counted['source_sha256'])
        # Replay all original main metrics and compare their COMPLETE arrays;
        # this cannot silently alter a formerly reviewed owner/span/expansion.
        base = normalizer.normalize(source.read_text(), 'nim')
        assert base == counted['metric'], ('Original source metric changed', path)
        if origin == NIM / 'support/gmp_ops.nim':
            assert counted['source_sha256'] == HELPER_SHA and counted['role'] == 'application-helper'
            metric = helper.normalize(source.read_text(), normalizer)
            assert metric['physical_loc'] == review['helper']['physical_loc'] == 13
            assert metric['normalized_loc'] == review['helper']['normalized_loc'] == 23
            assert len(metric['resolved_source_diagnostics']) == 11
        else:
            assert counted['role'] == 'application' and not base['diagnostics']
            metric = base
        assert metric['diagnostics'] == [] and metric['review_status'] == 'candidate-parser-pass'
        counts[path] = dict(counted, metric=metric)
    assert len([value for value in counts.values() if value['role'] == 'application-helper']) == 1
    assert len(old['unresolved']) == 11 and all(item['source'] == str(NIM / 'qualification/private-v3/source-snapshot/support/gmp_ops.nim') for item in old['unresolved'])
    ownership = dict(status='passed', review_identity='nim-exact20-main-helper23-source-meaning-20261013',
        evidence_path=str(HERE / 'meaning-review.json'), evidence_sha256=args.review_sha256,
        source_review_acceptance_required_before_launch=True, counted_helper_included=True)
    inventories, profiles, used = {}, {}, set()
    for identifier in sorted(rows):
        row, prior = rows[identifier], old['profiles'][identifier]
        bound = old['compiler_bound_input_inventory'][identifier]
        actual = read(pins, Path(bound['path']), bound['sha256'])
        assert actual['id'] == prior['id'] == row['id'] == identifier
        assert row['benchmark'] == prior['benchmark'] and row['variant'] == prior['variant']
        assert row['source'] == prior['source'] == actual['source']
        assert row['source_sha256'] == prior['source_sha256'] == actual['source_sha256'] == bound['source_sha256']
        assert row['binary_sha256'] == prior['binary_sha256'] == actual['binary_sha256'] == bound['binary_sha256']
        saved = saved_candidates[identifier]
        assert saved['source'] == row['source'] and saved['source_sha256'] == row['source_sha256']
        assert saved['binary_sha256'] == row['binary_sha256'] and saved['command'] == row['command']
        assert row['command'][0] == actual['binary']
        assert actual['source_bundle'] == bound['source_bundle'] == prior['source_bundle']
        command = actual['original_build_command']
        vendors = [Path(value.split(':', 1)[1]) for value in command if value.startswith(('--lib:', '--path:'))]
        assert len(vendors) == 3 and list(map(str, vendors)) == actual['vendor_roots']
        assert command[-1] == row['source'] and '--out:' + row['command'][0] in command
        build = actual['original_build_receipt']
        adapter.completed_owned_receipt(read(pins, Path(build['path']), build['sha256']), command)
        front = Path(actual['audit_receipt'])
        adapter.completed_owned_receipt(read(pins, front, pins[str(front)]), actual['audit_command'])
        assert actual['audit_command'] == adapter.audit_command(command, front.parent / 'cache', front.parent / 'never-scored-binary')
        assert actual['original_binary_rebuilt'] is False and actual['compiler_frontend_only'] is True
        assert not (front.parent / 'never-scored-binary').exists()
        helpers = [Path(row['source']).parents[2] / name for name in (*row['application_helpers'], *row['general_library_support'])]
        parsed = adapter.processing_inputs(front.with_suffix('.stderr').read_text(), Path(row['source']), helpers, list(map(Path, actual['vendor_roots'])))
        for item in parsed['files']:
            item['sha256'] = pins[item['path']]
            assert all(pins[alias] == item['sha256'] for alias in item['lookup_aliases'])
        assert parsed == actual['processing_evidence']
        assert [item['path'] for item in parsed['authored_files']] == [item['path'] for item in prior['source_bundle']]
        bundle, expansions = [], []
        for index, item in enumerate(prior['source_bundle']):
            path = item['path']
            counted, metric = counts[path], counts[path]['metric']
            assert item['sha256'] == counted['source_sha256'] and item['role'] == counted['role']
            assert item['role'] == ('application' if index == 0 else 'application-helper')
            if index == 0:
                assert review['main_counts'][identifier] == [metric['physical_loc'], metric['normalized_loc']]
            used.add(path)
            bundle.append(dict(path=path, sha256=item['sha256'], role=item['role'],
                physical_loc=metric['physical_loc'], normalized_loc=metric['normalized_loc'],
                count_receipt=dict(path=str(output / 'source-counts.json'), source=path),
                ownership_review=ownership, unresolved_diagnostics=[]))
            expansions.append(dict(source=path, source_sha256=item['sha256'], actual_expansions=metric['expansions']))
        physical, canonical = sum(item['physical_loc'] for item in bundle), sum(item['normalized_loc'] for item in bundle)
        if identifier == 'nim-pidigits-optimized':
            assert (physical, canonical) == (74, 84)
        evidence_path = output / 'compiler-inputs' / (identifier + '.json')
        files = [dict(path=item['path'], sha256=item['sha256']) for item in bundle]
        evidence = dict(status='qualified-exact-authored-compiler-inputs', profile=identifier,
            source_sha256=row['source_sha256'], binary_sha256=row['binary_sha256'], files=files,
            original_compiler_input_evidence=dict(path=bound['path'], sha256=bound['sha256']),
            independently_retained_compiler_input_evidence=parsed, qualified_preparation=old['qualification'],
            exact_source_role_review=ownership, inventory_derived_from_original_actual_compile_receipt=True,
            authored_application_helpers_complete=True, vendor_standard_library_counted_as_source=False,
            observer_sources_counted_as_source=False)
        save(evidence_path, evidence)
        inventories[identifier] = dict(status=evidence['status'], source_sha256=row['source_sha256'],
            binary_sha256=row['binary_sha256'], evidence_path=str(evidence_path), evidence_sha256=bind(pins, evidence_path), files=files)
        profiles[identifier] = dict(benchmark=row['benchmark'], variant=row['variant'], source=row['source'],
            source_sha256=row['source_sha256'], producer_loc=None, physical_loc=physical,
            canonical_loc=canonical, source_bundle=bundle, expansions=expansions, semantic_review=ownership)
    assert used == set(counts) and len(used) == 21
    save(output / 'source-counts.json', dict(status='reviewed-exact21-Nim-source-counts', source_counts=counts,
        preserved_prior_failure=failure_closure,
        original_replay=dict(path=str(PROPOSAL), sha256=PROPOSAL_SHA), replay_closure=closure,
        unchanged_independent_full90_suite=dict(path=str(SUITE), sha256=args.suite_sha256),
        meaning_review=ownership, unresolved=[], original_main_metrics_unchanged=True,
        actual_frontend_compiler_commands=0, actual_native_backend_compilations=0,
        actual_benchmark_processes=0, actual_common_mutex_held=True))
    count_digest = bind(pins, output / 'source-counts.json')
    for profile in profiles.values():
        for item in profile['source_bundle']:
            item['count_receipt']['sha256'] = count_digest
    bind(pins, output / 'helper-fixture-receipt.json')
    verify(pins)
    overlay = dict(schema_version=1, language='nim', status='qualified-exact-source-LOC', profile_count=20,
        complete_authored_source_bundles=True, unresolved=[], normalization_policy=review['rule_version'],
        fixture_receipt=dict(path=str(output / 'helper-fixture-receipt.json'), sha256=pins[str(output / 'helper-fixture-receipt.json')], status='passed'),
        reused_grammar_fixture=old['grammar_fixture'], reused_source_input_fixture=old['source_input_fixture'],
        replay_receipts=[dict(path=str(PROPOSAL), sha256=PROPOSAL_SHA), dict(path=str(output / 'source-counts.json'), sha256=count_digest)],
        grammar_snapshot_pins=old['grammar_snapshot_pins'], compiler_bound_input_inventory=inventories,
        unchanged_independent_full90_suite=dict(path=str(SUITE), sha256=args.suite_sha256),
        profiles=profiles, pins=dict(pins), canonical_LOC_admitted=True, chart_admission=False,
        preserved_prior_failure=failure_closure,
        original_sources_or_timings_mutated=False, actual_native_backend_compilations=0,
        actual_benchmark_processes=0)
    save(output / 'qualified-loc-overlay.json', overlay)
    return dict(overlay=str(output / 'qualified-loc-overlay.json'), sha256=sha(output / 'qualified-loc-overlay.json'),
                profiles=20, authored_inputs=21, optimized_pidigits_canonical_loc=84)


def run(args):
    print('WAIT common mutex: receipt-only Nim LOC ownership seal; zero native work', flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        held, named = os.fstat(lock.fileno()), LOCK.stat()
        assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino)
        with (DURABLE / 'owner.lock').open('r') as lease, (FAILED_OWNER / 'owner.lock').open('r') as failed_lease:
            fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
            fcntl.flock(failed_lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
            held_lease, named_lease = os.fstat(lease.fileno()), (DURABLE / 'owner.lock').stat()
            assert (held_lease.st_dev, held_lease.st_ino) == (named_lease.st_dev, named_lease.st_ino)
            held_failed, named_failed = os.fstat(failed_lease.fileno()), (FAILED_OWNER / 'owner.lock').stat()
            assert (held_failed.st_dev, held_failed.st_ino) == (named_failed.st_dev, named_failed.st_ino)
            output = HERE / 'jobs' / args.revision
            output.mkdir(parents=True)  # Exclusive; an existing failed output is not reused.
            pins, outcome = {}, dict(status='failed-before-output', actual_common_mutex_held=True,
                actual_compiler_processes=0, actual_benchmark_processes=0, original_sources_or_timings_mutated=False,
                chart_admission=False, automatic_retry=False)
            try:
                outcome.update(seal(args, output, pins), status='completed-qualified-loc-overlay')
            except BaseException as error:
                outcome.update(error_type=type(error).__name__, error=str(error) or type(error).__name__)
                (output / 'actual-error.log').write_text(traceback.format_exc())
                raise
            finally:
                captured = {}
                try:
                    verify(pins)
                    for path in sorted(output.rglob('*')):
                        if path.is_file():
                            captured[str(path)] = sha(path)
                    outcome.update(prerequisite_pins=pins, actual_terminal_evidence_pins=captured,
                                   available_bytes_not_fully_pinned=False)
                except BaseException as error:
                    outcome.update(status='failed-terminal-integrity', terminal_integrity_error=dict(
                        error_type=type(error).__name__, error=str(error) or type(error).__name__),
                        prerequisite_pins=pins, actual_terminal_evidence_pins=captured,
                        available_bytes_not_fully_pinned=True)
                    save(output / 'outcome.json', outcome)
                    raise
                save(output / 'outcome.json', outcome)
            print(json.dumps(outcome | dict(prerequisite_pins=None, actual_terminal_evidence_pins=None)), flush=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--revision', required=True)
    for name in ('durable-terminal', 'output-terminal', 'suite', 'worker', 'owners', 'tests', 'review', 'notes', 'failed-outcome', 'failed-durable-terminal'):
        parser.add_argument('--' + name + '-sha256', required=True)
    args = parser.parse_args()
    assert re.fullmatch(r'[a-z0-9][a-z0-9-]{0,63}', args.revision)
    assert all(re.fullmatch(r'[a-f0-9]{64}', value) for key, value in vars(args).items() if key.endswith('_sha256'))
    sys.dont_write_bytecode = True
    run(args)


if __name__ == '__main__':
    main()
