"""Additive preparation with retained fixtures and qualified success-halt runtime.

The new seal is metadata only. The separate prepare mode owns the original
twenty QLF profiles and 156 bounded cases. Work, LOC and timing remain pending.
"""
import argparse
from contextlib import ExitStack
import copy
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import sys

PACKET = Path(__file__).resolve().parent
HERE = PACKET.parents[1]
CLBG = HERE.parents[1]
LOCK = Path('/tmp/mech-clbg-timing.lock')
SERVICE = CLBG / 'c3-comparison/20261012/revisions/private-v1/guarded_prepare.py'
SERVICE_PROPOSAL = CLBG / 'c3-comparison/20261012/proposals/private-v1.json'
REFERENCES = SERVICE.with_name('references.py')
GENERIC_FIXTURE_RUNNER = SERVICE.with_name('fixture_runner.py')
ORIGINAL = HERE / 'drafts/private-v1'
OLD_PROPOSAL = HERE / 'proposals/private-v1.json'
OLD_PROPOSAL_SHA = '5b9186bb51e748fe69b16dcd35a632dd9ed8556abdbc30cbab45159a36a1bf88'
PROPOSAL = HERE / 'proposals/prepare-success-halt-v1.json'
REPAIRED = HERE / 'revisions/runtime-success-halt-v1'
RUNTIME = HERE / 'runtime-qualified/success-halt-v1/runtime-config.json'
RUNTIME_SHA = '7fa470a9827e60e84eb9c6ad343cc8385e411b400f0ae8a322fbefda488bb448'
SOURCE_REVIEW = RUNTIME.with_name('source-library-review.json')
SOURCE_REVIEW_SHA = 'eab625231fd424e78de80132566ca3c7fe1bb91b096b50874c0a515271bd6486'
CLOSED_REVIEW = CLBG / 'continuation/jobs/prolog-success-halt-closed-review-v1/review.json'
CLOSED_REVIEW_SHA = 'dca140a64f96733ad048cb41de496b980a475b8a7f02b9ad31def1a2317be1f8'
RUNTIME_REPORT = HERE / '.build/swi-runtime-success-halt-v1/report.json'
RUNTIME_REPORT_SHA = 'd744e831eb92c8ea88b3f07ab8dd247e6feaeacb0f003132364513d13471404b'
BENCHMARKS = ('regexredux', 'revcomp', 'pidigits', 'spectralnorm', 'mandelbrot',
              'fasta', 'knucleotide', 'binarytrees', 'nbody', 'fannkuchredux')
FILES = ('guarded_prepare.py', 'contracts.py', 'profiles.json', 'REVIEW.md',
         'support/clbg_io.pl', 'support/clbg_records.pl', 'tools/prepare_qlf.pl') + tuple(
             f'sources/{bench}/{form}.pl' for bench in BENCHMARKS for form in ('compact', 'optimized'))
OWNER_NAMES = ('prolog-private-v1-seal', 'prolog-runtime-success-halt-v1')
LOCAL_IDS = [
    'test_contracts.PreparationContracts.test_twenty_profiles_and_explicit_same_source_aliases',
    'test_contracts.PreparationContracts.test_compiler_inventory_requires_each_helper_and_no_foreign_authored_root',
    'test_contracts.PreparationContracts.test_actual_runtime_flags_reject_bounded_or_x86_or_disabled_gc',
    'test_contracts.PreparationContracts.test_native_counter_and_workload_boolean_substitution_is_rejected',
    'test_contracts.PreparationContracts.test_isolation_requires_exact_twenty_boolean_gates_and_clean_ownership',
    'test_contracts.PreparationContracts.test_full_admission_requires_actual_scored_allocation_not_source_assertion',
]
OWNERSHIP_IDS = [
    'test_guarded_prepare.OwnershipChecks.test_popen_failure_has_no_foreign_cleanup',
    'test_guarded_prepare.OwnershipChecks.test_identity_failure_cleans_known_child',
    'test_guarded_prepare.OwnershipChecks.test_journal_failure_cleans_known_child',
    'test_guarded_prepare.OwnershipChecks.test_timeout_preserves_complete_drained_output',
    'test_guarded_prepare.OwnershipChecks.test_orphan_compiler_child_is_fatal',
    'test_guarded_prepare.OwnershipChecks.test_cleanup_failure_is_fatal_and_waits',
    'test_guarded_prepare.OwnershipChecks.test_live_child_cleanup_failure_is_bounded_and_only_signals_owned_group',
    'test_guarded_prepare.ProbeRestorationChecks.test_setup_failure_restores_present_state',
    'test_guarded_prepare.ProbeRestorationChecks.test_absent_input_key_is_removed_on_failure',
    'test_guarded_prepare.ProbeRestorationChecks.test_parent_failure_archives_actual_sampler_bytes',
    'test_guarded_prepare.ProbeRestorationChecks.test_trace_failure_archives_and_stops',
]


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def save_new(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def verify(pins):
    for path, digest in pins.items():
        assert sha(path) == digest, ('Pinned bytes changed', path)


def merge(*groups):
    pins = {}
    for group in groups:
        for path, digest in group.items():
            assert path not in pins or pins[path] == digest, ('Conflicting pin', path)
            pins[path] = digest
    return pins


def module(path, name, pins):
    path = Path(path).resolve()
    assert name not in sys.modules, ('Cached candidate module', name)
    assert pins[str(path)] == sha(path)
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec)
    sys.modules[name] = result
    # Execute freshly verified source, never a sibling cached bytecode file.
    exec(compile(path.read_bytes(), str(path), 'exec'), result.__dict__)
    assert Path(result.__file__).resolve() == path and sys.modules[name] is result
    assert sha(path) == pins[str(path)]
    return result


def static_paths():
    return [PACKET / name for name in FILES]


def retained_inputs(args):
    """Root captures this exact source/receipt inventory after source approval."""
    assert sha(args.packet_pins) == args.packet_pins_sha256
    packet = json.loads(args.packet_pins.read_text())
    assert set(packet['files']) == set(FILES)
    pins = {str(args.packet_pins): args.packet_pins_sha256}
    for name, digest in packet['files'].items():
        path = PACKET / name
        assert not path.stat().st_mode & 0o222 and sha(path) == digest, str(path)
        pins[str(path)] = digest
    expected = {str(HERE / 'proposal-seals/private-v1/terminal-pin.json')}
    for name in OWNER_NAMES:
        owner = CLBG / 'continuation/owned-jobs' / name
        expected.update(str(owner / item) for item in ('request.json', 'launch.json',
            'worker-started.json', 'command-started.json', 'terminal.json'))
    assert set(packet['retained_receipt_pins']) == expected
    pins = merge(pins, packet['retained_receipt_pins'])
    verify(pins)
    for name in OWNER_NAMES:
        owner = CLBG / 'continuation/owned-jobs' / name
        request, launch, worker, started, done = [json.loads((owner / item).read_text()) for item in
            ('request.json', 'launch.json', 'worker-started.json', 'command-started.json', 'terminal.json')]
        if name == 'prolog-private-v1-seal':
            command = ['/opt/homebrew/Cellar/python@3.14/3.14.8/Frameworks/Python.framework/Versions/3.14/Resources/Python.app/Contents/MacOS/Python',
                '-B', '-u', 'drafts/private-v1/guarded_prepare.py', '--seal', '--service-proposal-sha256',
                '43438603d675472393050d05928e488a20f16040493935aca676007fc0ada050']
            cwd = HERE
        else:
            command = ['/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3.12',
                '-I', '-S', '-B', '-u', str(REPAIRED / 'discover_runtime.py'), '--packet-pins',
                str(CLBG / 'continuation/jobs/prolog-success-halt-root-release-v1/packet-pins.json'),
                '--packet-pins-sha256', 'c460f24075de8ce40c1aaf7c6e524bb2b8709c10cec7c5b74b67e6e474cee6cc',
                '--closed-review-sha256', 'ca4e8b797d340e84ce517829fabcca3fa157c5db5dd820ab4c97d75910c330c0']
            cwd = CLBG.parents[1]
        assert all(item['command'] == command and item['cwd'] == str(cwd)
                   for item in (request, launch, worker, started))
        assert done['status'] == 'command-exited-zero-awaiting-report-review'
        assert type(done['exit_code']) is int and done['exit_code'] == 0
        assert done['stop_signals'] == [] and not done.get('error')
        assert done['actual_command_started'] is True and done['command_identity_observation_missing'] is False
        assert started['command_identity'] == done['command_identity']
        assert worker['worker_identity'] == done['worker_identity'] and worker['inherited_private_lease'] is True
        assert started['command_pid'] == done['command_pid'] == done['command_identity']['pid']
        assert launch['worker_identity']['pid'] == worker['worker_identity']['pid']
        assert launch['worker_identity']['listing'].split()[2:] == worker['worker_identity']['listing'].split()[2:]
        assert all(item.get('automatic_retry', False) is False for item in (request, launch, done))
    return pins


def original_proposal(pins):
    assert sha(OLD_PROPOSAL) == OLD_PROPOSAL_SHA
    sealed = json.loads(OLD_PROPOSAL.read_text())
    pins = merge(pins, sealed['pins'], {str(OLD_PROPOSAL): OLD_PROPOSAL_SHA})
    verify(pins)
    old_terminal = json.loads((HERE / 'proposal-seals/private-v1/terminal-pin.json').read_text())
    assert sealed['status'] == old_terminal['status'] == 'sealed-awaiting-root-review'
    assert old_terminal['report_sha256'] == OLD_PROPOSAL_SHA
    assert old_terminal['partial_evidence_pins'] == sealed['pins']
    old_report = HERE / 'proposal-seals/private-v1/report.json'
    assert old_terminal['report'] == str(old_report) and sha(old_report) == OLD_PROPOSAL_SHA
    pins = merge(pins, {str(old_report): OLD_PROPOSAL_SHA})
    assert sealed['terminal_evidence_complete'] is True
    receipt = HERE / 'proposal-seals/private-v1/fixture-receipt.json'
    fixture = json.loads(receipt.read_text())
    assert fixture == sealed['local_fixture_receipt'] and fixture['status'] == 'passed'
    assert fixture['expected_test_ids'] == LOCAL_IDS
    assert type(fixture['tests_run']) is type(fixture['discovered_tests']) is int
    assert fixture['tests_run'] == fixture['discovered_tests'] == 6
    assert fixture['execution_completed'] is fixture['inherited_common_mutex'] is True
    for field in ('actual_test_ids', 'discovered_test_ids'):
        assert len(fixture[field]) == len(set(fixture[field])) == 6 and set(fixture[field]) == set(LOCAL_IDS)
    assert all(fixture[field] == [] for field in ('failures', 'errors', 'skips', 'expected_failures', 'unexpected_successes'))
    assert sealed['reused_generic_fixture_count'] == 11 and sealed['total_bounded_cases'] == 156
    derivation = {}
    for name in FILES:
        if name in ('guarded_prepare.py', 'REVIEW.md'):
            continue
        origin = REPAIRED / name if name in ('support/clbg_io.pl', 'tools/prepare_qlf.pl') else ORIGINAL / name
        assert sha(origin) == pins[str(origin)]
        assert (PACKET / name).read_bytes() == origin.read_bytes(), ('Copied source changed', name)
        derivation[name] = dict(path=str(origin), sha256=pins[str(origin)])
    return sealed, pins, derivation


def reused_service(reviewed_digest):
    for loaded in list(sys.modules.values()):
        filename = getattr(loaded, '__file__', None)
        if filename:
            path = Path(filename).resolve()
            assert not path.is_relative_to(CLBG) or path.is_relative_to(PACKET), ('Foreign cached benchmark module', path)
    assert sha(SERVICE_PROPOSAL) == reviewed_digest
    original = json.loads(SERVICE_PROPOSAL.read_text())
    assert original['status'] == 'sealed-awaiting-root-approval'
    receipt_path = Path(original['actual_mock_fixture_receipt'])
    assert original['actual_mock_fixture_receipt_sha256'] == original['pins'][str(receipt_path)]
    # All saved source pins are verified before any generic service/harness code
    # imports. C3 benchmark binaries/algorithms are not reused or executed here.
    pins = {path: digest for path, digest in original['pins'].items()
            if Path(path).suffix == '.py'}
    for path in (SERVICE, REFERENCES, GENERIC_FIXTURE_RUNNER, receipt_path):
        pins[str(path)] = original['pins'][str(path)]
    for path, digest in original['pins'].items():
        if Path(path).parent == receipt_path.parent and Path(path).name.startswith('ownership-fixtures'):
            pins[path] = digest
    pins[str(SERVICE_PROPOSAL)] = reviewed_digest
    verify(pins)
    # Validate the retained actual eleven-method receipt without importing or
    # rerunning either unittest fixture suite in this continuation.
    fixture = json.loads(receipt_path.read_text())
    assert fixture['status'] == 'passed' and fixture['inventory'] == 'ownership'
    assert fixture['expected_test_ids'] == OWNERSHIP_IDS
    assert type(fixture['tests_run']) is int and fixture['tests_run'] == 11
    assert type(fixture['discovered_tests']) is int and fixture['discovered_tests'] == 11
    assert len(fixture['actual_test_ids']) == len(set(fixture['actual_test_ids'])) == 11
    assert set(fixture['actual_test_ids']) == set(OWNERSHIP_IDS)
    assert fixture['successful'] is True
    assert all(fixture[key] == [] for key in ('skipped','failures','errors','expected_failures','unexpected_successes'))
    service = module(SERVICE, 'prolog_reviewed_generic_service', pins)
    references = module(REFERENCES, 'prolog_reviewed_bounded_references', pins)
    guard = service.load_guard()
    harness = guard.load_harness()
    for loaded in (guard, harness, harness.compact, harness.compact.common):
        filename = str(Path(loaded.__file__).resolve())
        assert filename in pins and pins[filename] == sha(filename)
    verify(pins)
    return service, references, guard, harness, pins


def terminal(output, report, pins, error=None):
    """Pin incrementally, preserving partial closure even if a late read fails."""
    partial = dict(pins)
    capture_errors = []
    for path in sorted(output.rglob('*')):
        if path.is_file() and path.name not in ('report.json', 'terminal-pin.json'):
            try:
                partial = merge(partial, {str(path): sha(path)})
            except BaseException as failure:
                capture_errors.append(dict(path=str(path), error=str(failure) or type(failure).__name__,
                                           error_type=type(failure).__name__))
    if error is not None:
        report.update(status='stopped-unqualified', error=str(error) or type(error).__name__,
                      error_type=type(error).__name__)
    for path, digest in partial.items():
        try:
            assert sha(path) == digest, ('Pinned bytes changed', path)
        except BaseException as failure:
            capture_errors.append(dict(path=path, error=str(failure) or type(failure).__name__,
                                       error_type=type(failure).__name__))
    if capture_errors:
        report.update(status='failed-terminal-integrity', terminal_artifact_errors=capture_errors,
                      terminal_evidence_complete=False)
    else:
        report['terminal_evidence_complete'] = True
    report['pins'] = partial
    # report is a mutable ledger until this terminal save; never self-hash it.
    with (output / 'report.json').open('w') as stream:
        json.dump(report, stream, indent=2)
        stream.write('\n')
    save_new(output / 'terminal-pin.json', dict(status=report['status'],
        report=str(output / 'report.json'), report_sha256=sha(output / 'report.json'),
        partial_evidence_pins=partial, inherited_common_mutex=True,
        chart_admission=False, full_processes_authorized=0))
    if capture_errors:
        raise RuntimeError('Terminal integrity failed; all readable partial evidence preserved')


def seal(args):
    output = HERE / 'proposal-seals/prepare-success-halt-v1'
    output.mkdir(parents=True)
    pins = dict(args.retained_pins)
    report = dict(status='sealing', profiles=20, chart_admission=False,
                  full_processes_authorized=0, inherited_common_mutex=True)
    error = None
    try:
        cfg, runtime_pins, binary, home = runtime_config(args)
        pins = merge(pins, runtime_pins)
        old, pins, derivation = original_proposal(pins)
        service, references, guard, harness, service_pins = reused_service(old['service_proposal_sha256'])
        pins = merge(pins, service_pins)
        contracts = module(PACKET / 'contracts.py', 'contracts', pins)
        rows = contracts.profiles(json.loads((PACKET / 'profiles.json').read_text()))
        assert rows == old['profiles']
        for row in rows:
            if row['variant'] == 'optimized':
                baseline = PACKET / 'sources' / row['benchmark'] / 'compact.pl'
                assert (PACKET / row['source']).read_bytes() == baseline.read_bytes(), 'Alias changed required work'
        cases = bounded_cases(service, references)
        assert sum(len(cases[row['benchmark']]) for row in rows) == 156
        assert {bench: [name for name, _, _ in cases[bench]] for bench in contracts.BENCHMARKS} == old['case_inventory']
        verify(pins)
        report.update(status='sealed-awaiting-root-review', local_fixture_receipt=old['local_fixture_receipt'],
            reused_generic_fixture_count=11, source_work_qualified=False, canonical_loc=None,
            native_processes_authorized=0, total_bounded_cases=156,
            runtime_candidate='SWI-Prolog10.0.2 native bytecode VM / QLF',
            runtime_verified=True, maximum_workers=1, profiles=rows,
            case_inventory={bench: [name for name, _, _ in cases[bench]] for bench in contracts.BENCHMARKS},
            service_proposal=str(SERVICE_PROPOSAL), service_proposal_sha256=old['service_proposal_sha256'],
            original_proposal=str(OLD_PROPOSAL), original_proposal_sha256=OLD_PROPOSAL_SHA,
            source_derivation=derivation, fixture_reexecutions=0, runtime_reexecutions=0,
            packet_pins=str(args.packet_pins), packet_pins_sha256=args.packet_pins_sha256,
            runtime_config=str(RUNTIME), runtime_config_sha256=RUNTIME_SHA,
            source_alias_policy='identical source files; independent QLF/profile records; no distinct optimization claim')
    except BaseException as failure:
        error = failure
    finally:
        terminal(output, report, pins, error)
    if error is not None:
        raise error
    save_new(PROPOSAL, report)
    print(json.dumps(dict(proposal=str(PROPOSAL), sha256=sha(PROPOSAL), status=report['status'])), flush=True)


def bounded_cases(service, references):
    result = {}
    for bench, values in service.NUMERIC.items():
        result[bench] = [('n-' + str(n), n, None) for n in values]
    for bench in ('regexredux', 'revcomp', 'knucleotide'):
        result[bench] = [(name, 0, payload) for name, payload in service.inputs(bench, references)]
    result['revcomp'].append(('preheader-ignored', 0, b'ACGT\n>one\nA\n'))
    return result


def runtime_config(args):
    filename = Path(args.runtime_config).resolve()
    assert filename == RUNTIME and args.runtime_config_sha256 == RUNTIME_SHA
    assert sha(filename) == RUNTIME_SHA
    value = json.loads(filename.read_text())
    assert value['status'] == 'qualified-runtime-discovery'
    assert value['shared_timing_lock'] == str(LOCK) and value['chart_admission'] is False
    assert value['version_components'] == [10, 0, 2]
    assert value['architecture'] == 'arm64' and value['runtime_backend'] == 'swi-native-bytecode-vm'
    binary = Path(value['binary']).resolve(strict=True)
    home = Path(value['home']).resolve(strict=True)
    assert value['pins'][str(binary)] == value['binary_sha256']
    assert value['version_receipt']['status'] == 'passed'
    assert value['archive_or_existing_installation_receipt']['status'] == 'passed'
    assert value['default_gc_review']['status'] == 'passed'
    verify(value['pins'])
    extra = {str(filename): RUNTIME_SHA, str(SOURCE_REVIEW): SOURCE_REVIEW_SHA,
             str(CLOSED_REVIEW): CLOSED_REVIEW_SHA, str(RUNTIME_REPORT): RUNTIME_REPORT_SHA}
    verify(extra)
    assert value['closed_review'] == dict(path=str(CLOSED_REVIEW), sha256=CLOSED_REVIEW_SHA)
    assert value['source_library_review'] == dict(path=str(SOURCE_REVIEW), sha256=SOURCE_REVIEW_SHA)
    review = json.loads(CLOSED_REVIEW.read_text())
    assert review['status'] == 'corrected-feature-and-private-images-passed' and review['verified_pins'] == 13399
    assert review['report_sha256'] == RUNTIME_REPORT_SHA
    assert review['feature_exit'] == 0 and review['native_images'] == 7 and review['actual_native_image_commands'] == 35
    assert review['benchmarks'] == review['version_reexecutions'] == review['fixture_reexecutions'] == 0
    body_terminal = RUNTIME_REPORT.with_name('terminal-pin.json')
    durable_terminal = CLBG / 'continuation/owned-jobs/prolog-runtime-success-halt-v1/terminal.json'
    extra = merge(extra, {str(body_terminal): review['body_terminal_sha256'],
                         str(durable_terminal): review['durable_terminal_sha256']})
    verify(extra)
    body = json.loads(RUNTIME_REPORT.read_text())
    ended = json.loads(body_terminal.read_text())
    assert body['status'] == ended['status'] == 'runtime-discovered-awaiting-root-source-library-review'
    assert body['terminal_evidence_complete'] is True and ended['report_sha256'] == RUNTIME_REPORT_SHA
    assert ended['partial_evidence_pins'] == body['pins']
    verify(body['pins'])
    library = json.loads(SOURCE_REVIEW.read_text())
    assert library['status'] == 'passed' and library['root_review'] is True
    verify(library['source_pins'])
    closure_path = RUNTIME_REPORT.parent / 'images/closure.json'
    closure_sha = library['native_image_closure_sha256']
    assert body['pins'][str(closure_path)] == closure_sha and sha(closure_path) == closure_sha
    closure = json.loads(closure_path.read_text())
    assert closure['status'] == 'passed-native-image-closure'
    assert closure['executable'] == str(binary) and closure['selected_architecture'] == 'arm64'
    assert set(closure['image_pins']) == set(review['images']) == set(value['loaded_native_images'])
    assert len(closure['image_pins']) == len(closure['contexts']) == 7
    verify(closure['image_pins'])
    for image, digest in closure['image_pins'].items():
        assert value['pins'][image] == body['pins'][image] == digest
    command_count = 0
    for context in closure['contexts']:
        assert context['selected_architecture'] == 'arm64' and context['image'] in closure['image_pins']
        assert len(context['commands']) == 5
        for command in context['commands'].values():
            path = Path(command['receipt'])
            assert sha(path) == body['pins'][str(path)]
            actual = json.loads(path.read_text())
            owned_receipt(actual, command['command'])
            command_count += 1
    assert command_count == 35
    feature = json.loads(Path(value['feature_probe_owned_receipt']['receipt']).read_text())
    owned_receipt(feature, value['feature_probe_owned_receipt']['command'])
    return value, merge(value['pins'], body['pins'], library['source_pins'], extra,
                        {str(closure_path): closure_sha}), binary, home


def owned_receipt(actual, command):
    assert actual['command'] == command and actual['inherited_common_mutex'] is True
    assert type(actual['exit_status']) is type(actual['final_exit_status']) is int
    assert actual['exit_status'] == actual['final_exit_status'] == 0
    assert not actual.get('error') and not actual.get('cleanup_errors')


def compiler_libraries(actual, cfg):
    """Every actually loaded compiler plugin must belong to the saved closure."""
    foreign = actual['foreign_libraries']
    assert isinstance(foreign, list)
    for entry in foreign:
        path = str(Path(entry['resolved_path']).resolve(strict=True))
        assert path in cfg['loaded_native_images'] and sha(path) == cfg['pins'][path], path


def prefix(binary):
    # Uniform stack allowance is a resource ceiling, not GC disabling. Actual
    # reported byte limit and default GC must be validated for every compiler.
    return [str(binary), '-q', '-O', '-f', 'none', '--stack-limit=8g']


def compiler_command(binary, mode, target, receipt, wam, authored_root, preloads=()):
    command = prefix(binary)
    for preload in preloads:
        command += ['-l', str(preload)]
    return command + ['-l', str(PACKET / 'tools/prepare_qlf.pl'), '-g', 'clbg_prepare:main',
        '--', mode, str(target), str(receipt), str(wam), str(authored_root)]


def native_raw_receipts(attempt):
    sampled = json.loads(Path(attempt['result_path']).read_text())
    enriched = json.loads(Path(attempt['supervisor_result_path']).read_text())
    for record in (sampled, enriched, attempt):
        assert record['full_workload'] is False
        assert type(record['stdout_bytes']) is int and record['stdout_bytes'] >= 0
        assert type(record['validation']['bytes']) is int and record['validation']['bytes'] >= 0
        assert record['validation']['reference_match'] is True
    for record in (enriched, attempt):
        for field in ('attempt', 'workload_n', 'sampler_wait_status'):
            assert type(record[field]) is int
        assert record['sampler_wait_status'] == 0
    for field in ('command', 'cwd', 'runtime_environment', 'execution_fingerprint', 'workload_n',
                  'full_workload', 'seconds', 'exit_status', 'stdout_bytes', 'stdout_sha256', 'validation'):
        assert sampled[field] == enriched[field] == attempt[field], ('Original native evidence differs', field)


def prepare(args):
    proposal = PROPOSAL
    assert sha(proposal) == args.proposal_sha256
    sealed = json.loads(proposal.read_text())
    assert sealed['status'] == 'sealed-awaiting-root-review'
    assert sealed['local_fixture_receipt']['status'] == 'passed'
    assert sealed['local_fixture_receipt']['tests_run'] == 6
    pins = merge(args.retained_pins, sealed['pins'], {str(proposal): args.proposal_sha256})
    verify(pins)
    assert sealed['packet_pins'] == str(args.packet_pins) and sealed['packet_pins_sha256'] == args.packet_pins_sha256
    assert sealed['runtime_config'] == str(RUNTIME) and sealed['runtime_config_sha256'] == RUNTIME_SHA
    assert sealed['original_proposal'] == str(OLD_PROPOSAL) and sealed['original_proposal_sha256'] == OLD_PROPOSAL_SHA
    assert sealed['fixture_reexecutions'] == sealed['runtime_reexecutions'] == 0
    _, pins, derivation = original_proposal(pins)
    assert derivation == sealed['source_derivation']
    service, references, guard, harness, saved = reused_service(sealed['service_proposal_sha256'])
    pins = merge(pins, saved)
    contracts = module(PACKET / 'contracts.py', 'contracts', pins)
    cfg, runtime_pins, binary, home = runtime_config(args)
    pins = merge(pins, runtime_pins)
    rows = contracts.profiles(json.loads((PACKET / 'profiles.json').read_text()))
    output = HERE / 'qualification/prepare-success-halt-v1'
    output.mkdir(parents=True)
    guard.OUT = output
    guard.REPORT = output / 'report.json'
    report = dict(status='running', profiles={}, attempts=[], active=None, chart_admission=False,
        full_processes_authorized=0, source_work_qualified=False, canonical_loc=None,
        proposal=str(proposal), proposal_sha256=args.proposal_sha256,
        retained_contract_fixture_count=6, retained_ownership_fixture_count=11,
        fixture_reexecutions=0, version_reexecutions=0, image_command_reexecutions=0,
        runtime_config=str(Path(args.runtime_config).resolve()), runtime_config_sha256=args.runtime_config_sha256)
    error = None
    try:
        harness.compact.wait_for_environment(harness.REQUIRED)
        environment = dict(harness.compact.ENV) | cfg['environment']
        harness.install_interrupt_handlers()
        verify(pins)
        cases = bounded_cases(service, references)
        assert sum(len(cases[row['benchmark']]) for row in rows) == 156
        assert {bench: [name for name, _, _ in cases[bench]] for bench in contracts.BENCHMARKS} == sealed['case_inventory']
        for declared in rows:
            row = copy.deepcopy(declared)
            build = output / 'builds' / row['id']
            input_root = build / 'input'
            expected_authored = []
            bundles = []
            artifacts = []
            compile_receipts = []
            for relative in row['support_files'] + [row['source']]:
                original = PACKET / relative
                target = input_root / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(original, target)
                assert sha(target) == pins[str(original.resolve())]
                pins[str(target)] = sha(target)
                expected_authored.append(target)
                bundles.append(dict(path=str(target), original_path=str(original),
                    sha256=sha(target), role='main-source' if relative == row['source'] else 'counted-authored-helper'))
                receipt = build / (target.stem + '-compile.json')
                wam = build / (target.stem + '-compile.wam')
                assert not receipt.exists() and not wam.exists()
                command = compiler_command(binary, 'compile', target, receipt, wam, input_root)
                service.owned_command(guard, harness, command, environment, build, args.lock_fd, 120,
                                      target.stem + '-compile-command')
                actual = json.loads(receipt.read_text())
                contracts.runtime_flags(actual['runtime'], home)
                compiler_libraries(actual, cfg)
                owned_receipt = build / (target.stem + '-compile-command.json')
                owned = json.loads(owned_receipt.read_text())
                assert owned['command'] == command and owned['inherited_common_mutex'] is True
                assert type(owned['exit_status']) is int and owned['exit_status'] == 0
                assert type(owned['final_exit_status']) is int and owned['final_exit_status'] == 0
                assert not owned.get('error') and not owned.get('cleanup_errors')
                compile_receipts.append(dict(source=str(target), command=command,
                    receipt=str(owned_receipt), receipt_sha256=sha(owned_receipt),
                    input_inventory_receipt=str(receipt), input_inventory_receipt_sha256=sha(receipt)))
                artifact = target.with_suffix('.qlf')
                assert artifact.is_file()
                artifacts.append(artifact)
            main = expected_authored[-1]
            preloads = artifacts[:-1]
            inspect_receipt = build / 'loaded-inputs.json'
            inspect_wam = build / 'original-scored.wam'
            command = compiler_command(binary, 'inspect', artifacts[-1], inspect_receipt, inspect_wam,
                                       input_root, preloads)
            service.owned_command(guard, harness, command, environment, build, args.lock_fd, 120, 'inspect-command')
            actual = json.loads(inspect_receipt.read_text())
            assert inspect_wam.stat().st_size > 0, 'Missing actual original scored application VM code'
            contracts.runtime_flags(actual['runtime'], home)
            compiler_libraries(actual, cfg)
            roles = contracts.compiler_inputs(actual['loaded_sources'], expected_authored,
                                              home, PACKET / 'tools/prepare_qlf.pl')
            # Every vendor module/boot/library is bound to the reviewed private
            # runtime corpus. Original source inventories and QLF artifacts have
            # separate roles; generated WAM is never the source-count substitute.
            for role in roles:
                target = role['resolved_path']
                pins[target] = sha(target)
                if role['role'] == 'vendor-runtime-library':
                    assert pins[target] == cfg['pins'][target], ('Unreviewed loaded runtime source', target)
            local = {str(path): sha(path) for path in build.rglob('*') if path.is_file()}
            pins = merge(pins, local)
            invocation = prefix(binary)
            for artifact in preloads:
                invocation += ['-l', str(artifact)]
            invocation += ['-s', str(artifacts[-1])]
            if not row['stdin']:
                invocation += ['--', '{n}']
            row.update(source=str(PACKET / declared['source']), measured_source=str(main),
                source_sha256=sha(main), source_bundle=bundles, binary=str(binary), binary_sha256=sha(binary),
                command=invocation, cwd=str(input_root), runtime_environment=cfg['environment'],
                application_artifact_pins={str(path): sha(path) for path in artifacts},
                compile_receipts=compile_receipts,
                compiler_bound_input_inventory=dict(status='qualified-exact-compiled-inputs',
                    receipt=str(inspect_receipt), receipt_sha256=sha(inspect_receipt), inputs=roles,
                    original_scored_wam=str(inspect_wam), original_scored_wam_sha256=sha(inspect_wam)),
                compiler_identity=cfg['version_receipt'], runtime_closure_pins=runtime_pins,
                execution_mode='SWI-Prolog native bytecode VM; QLF application loading inside timer',
                runtime_policy='Default term/atom/clause garbage collection; uniform8g stack ceiling',
                backend_verified=True, bounded_output_qualified=False)
            report['profiles'][row['id']] = row
            guard.save(guard.REPORT, report)
            for label, n, payload in cases[row['benchmark']]:
                oracle = getattr(references, row['benchmark'])
                expected = oracle(payload if payload is not None else n)
                attempt = service.probe(guard, harness, row, n, payload, expected, pins,
                    output / 'bounded-output', args.lock_fd, report, label)
                contracts.complete_attempt(attempt, False)
                native_raw_receipts(attempt)
            row['bounded_output_qualified'] = True
        assert len(report['profiles']) == 20 and len(report['attempts']) == 156
        assert all(attempt['qualified'] is True for attempt in report['attempts'])
        report.update(status='bounded-output-ownership-passed-awaiting-work-and-LOC-review',
                      all20_bounded_outputs_qualified=True, total_bounded_cases=156)
    except BaseException as failure:
        error = failure
    finally:
        terminal(output, report, pins, error)
    if error is not None:
        raise error


def main():
    parser = argparse.ArgumentParser()
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--seal', action='store_true')
    mode.add_argument('--prepare', action='store_true')
    parser.add_argument('--packet-pins', required=True, type=Path)
    parser.add_argument('--packet-pins-sha256', required=True)
    parser.add_argument('--proposal-sha256')
    parser.add_argument('--runtime-config', type=Path, default=RUNTIME)
    parser.add_argument('--runtime-config-sha256', default=RUNTIME_SHA)
    args = parser.parse_args()
    assert sys.dont_write_bytecode
    args.packet_pins = args.packet_pins.resolve(strict=True)
    if args.prepare:
        assert args.proposal_sha256
    print('WAIT common mutex: Prolog candidate metadata/preparation', flush=True)
    with LOCK.open('r+') as lock, ExitStack() as leases:
        fcntl.flock(lock, fcntl.LOCK_EX)
        held, named = os.fstat(lock.fileno()), LOCK.stat()
        assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino) and held.st_ino == 79859134
        args.lock_fd = lock.fileno()
        for name in OWNER_NAMES:
            path = CLBG / 'continuation/owned-jobs' / name / 'owner.lock'
            lease = leases.enter_context(path.open('r+'))
            fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
            held, named = os.fstat(lease.fileno()), path.stat()
            assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino)
        args.retained_pins = retained_inputs(args)
        (seal if args.seal else prepare)(args)


if __name__ == '__main__':
    main()
