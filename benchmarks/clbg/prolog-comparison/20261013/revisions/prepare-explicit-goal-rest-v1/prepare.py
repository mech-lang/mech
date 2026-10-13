"""Remaining155 bounded cases, reusing the first successful explicit-goal case."""
import argparse
import copy
from contextlib import ExitStack
import fcntl
import hashlib
import importlib.util
import json
import os
import shutil
from pathlib import Path
import sys
import traceback

P = Path(__file__).resolve().parent
H = P.parents[1]
C = H.parents[1] / 'continuation'
LOCK = Path('/tmp/mech-clbg-timing.lock')
PRIOR = H / 'qualification/prepare-success-halt-v1'
OWNER = C / 'owned-jobs/prolog-prepare-success-halt-native-v1'
REVIEW = C / 'jobs/prolog-prepare-native-closed-review-v1/receipt.json'
PREP = H / 'revisions/prepare-success-halt-v1/guarded_prepare.py'
PREP_SHA = '8d7cefd54b0342b563aec6131e8cdb443203309c0719b804a26d991e0c227ecb'
REPORT_SHA = '1b2e0f5c08c3f2105d6f97046145cb18b905220199852915c9dfd4673719da20'
BODY_SHA = '59d7b133c142f7bb1d4010d739bda779bb6fd033aea222d2aab1c608439dc04c'
OWNER_SHA = 'ba6f8f9f2c10e4bc2f1489441e3d98480e9b9b86caf6bc706879c179abd1721a'
ID = 'prolog-regexredux-compact'
GOAL = 'clbg_io:run(clbg_regexredux:main)'
FIRST = H / 'qualification/explicit-goal-first-case-v1'
FIRST_OWNER = C / 'owned-jobs/prolog-explicit-goal-first-case-v1'
FIRST_REVIEW = C / 'jobs/prolog-explicit-goal-closed-review-v1/receipt.json'
FIRST_REPORT_SHA = 'b3eac8a465ad303f7941925714e71ed6d7fabca757af2f1179156c060485c580'
FIRST_BODY_SHA = 'edd1fdd25987bceea986fdc7cfa78d13679e8ba3aa63373bc5cd6183d652e025'
FIRST_OWNER_SHA = '7a68d7d47bcb75af741892836fec1137978b2399b39aed4ea143946b92df5b12'
FIRST_WORKER_SHA = '87be372a3541895e7de036f1b1d68fb245ca7f7d08ade7faab7ce1a3832a7fb4'
FIRST_NOTE_SHA = 'e72b3d2a7a2a8ee6d4e3954510aee8e154b1a64b89b7e1b7ee6cea7244a882ea'
FAILED_REVIEW_SHA = '785fb78443a3a46384533b522ab124b34c2ae87a7e11da4da22a083289099b4e'


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def bind(path, digest, pins):
    path = Path(path).absolute()
    assert sha(path) == digest and (str(path) not in pins or pins[str(path)] == digest), str(path)
    pins[str(path)] = digest
    return path


def read(path, digest, pins):
    return json.loads(bind(path, digest, pins).read_bytes())


def cold(path, name, pins):
    path = Path(path).resolve(strict=True)
    assert name not in sys.modules and sha(path) == pins[str(path)]
    raw = path.read_bytes()
    assert hashlib.sha256(raw).hexdigest() == pins[str(path)]
    spec = importlib.util.spec_from_file_location(name, path)
    loaded = importlib.util.module_from_spec(spec)
    sys.modules[name] = loaded
    exec(compile(raw, str(path), 'exec'), loaded.__dict__)
    assert Path(loaded.__file__).resolve() == path and sys.modules[name] is loaded
    return loaded


def main():
    parser = argparse.ArgumentParser()
    for name in ('worker', 'review', 'closed-review', 'success-review'):
        parser.add_argument('--' + name + '-sha256', required=True)
    args = parser.parse_args()
    assert sys.dont_write_bytecode
    print('WAIT common mutex: remaining155 Prolog preparation cases', flush=True)
    with LOCK.open('r+') as common, ExitStack() as leases:
        fcntl.flock(common, fcntl.LOCK_EX)
        held, named = os.fstat(common.fileno()), LOCK.stat()
        assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino) and held.st_ino == 79859134
        for owner in (OWNER, FIRST_OWNER):
            lease = leases.enter_context((owner / 'owner.lock').open('r+'))
            fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
            held, named = os.fstat(lease.fileno()), (owner / 'owner.lock').stat()
            assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino)
        output = H / 'qualification/prepare-explicit-goal-rest-v1'
        output.mkdir(parents=True, exist_ok=False)
        pins, prep, failure = {}, None, None
        report = dict(status='running-remaining155-Prolog-preparation', profiles={}, attempts=[], reused_attempts=[], case_results=[], active=None,
            chart_admission=False, source_work_qualified=False, canonical_loc=None, full_processes_authorized=0,
            reused_profile_recompilations=0, successful_case_reexecutions=0, new_source_profiles_built=0,
            new_bounded_cases=0, fixture_reexecutions=0, version_reexecutions=0, runtime_image_reexecutions=0,
            original_failure_relabelled=False, maximum_workers=1)
        try:
            for path, digest in ((P / 'prepare.py', args.worker_sha256), (P / 'REVIEW.md', args.review_sha256)):
                assert not path.stat().st_mode & 0o222
                bind(path, digest, pins)
            assert all(not getattr(item, '__file__', None) or
                       not Path(item.__file__).resolve().is_relative_to(H.parents[1]) or
                       Path(item.__file__).resolve() == P / 'prepare.py' for item in list(sys.modules.values()))
            assert args.closed_review_sha256 == FAILED_REVIEW_SHA
            review = read(REVIEW, args.closed_review_sha256, pins)
            assert review['status'] == 'closed-failed-first-Prolog-output-case' and review['verified_pins'] == 13471
            assert review['report_sha256'] == REPORT_SHA and review['body_terminal_sha256'] == BODY_SHA
            assert review['durable_terminal_sha256'] == OWNER_SHA and review['attempts'] == 1
            assert review['native_exit_status'] == 0 and review['failed_case'] == 'all-replacements'
            old = read(PRIOR / 'report.json', REPORT_SHA, pins)
            ended = read(PRIOR / 'terminal-pin.json', BODY_SHA, pins)
            assert old['status'] == ended['status'] == 'stopped-unqualified' and old['terminal_evidence_complete'] is True
            assert ended['report_sha256'] == REPORT_SHA and ended['partial_evidence_pins'] == old['pins']
            assert len(old['attempts']) == 1 and old['attempts'][0]['qualified'] is False
            assert set(old['profiles']) == {ID} and old['attempts'][0]['case'] == 'all-replacements'
            for path, digest in old['pins'].items():
                bind(path, digest, pins)
            done = read(OWNER / 'terminal.json', OWNER_SHA, pins)
            request, launch, worker, start = [read(OWNER / name, sha(OWNER / name), pins) for name in
                ('request.json', 'launch.json', 'worker-started.json', 'command-started.json')]
            command = ['/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3.12',
                '-I', '-S', '-B', '-u', str(PREP), '--prepare', '--packet-pins',
                str(C / 'jobs/prolog-prepare-source-freeze-v1/packet-pins.json'), '--packet-pins-sha256',
                '37076a11022e0cfbb07c86b420866eec4f81f1d722b606e186c41c25dcdc2660', '--proposal-sha256',
                'c36911c1e2f9b8c2f8897667e15474ecda619cd3284115ed590d52f0d43e8f4b']
            assert all(item['command'] == command and item['cwd'] == str(H.parents[3])
                       for item in (request, launch, worker, start))
            assert done['status'] == 'command-failed' and type(done['exit_code']) is int and done['exit_code'] == 1
            assert done['actual_command_started'] is True and done['stop_signals'] == []
            assert done['automatic_retry'] is done['qualification_granted'] is False
            assert worker['worker_identity'] == done['worker_identity'] and worker['inherited_private_lease'] is True
            assert start['command_identity'] == done['command_identity'] and start['command_pid'] == done['command_pid'] == 31352
            assert worker['worker_identity']['pid'] == 31348 and launch['worker_identity']['pid'] == 31348
            assert start['command_identity_observation_missing'] is done['command_identity_observation_missing'] is False
            observed = start['command_identity']['listing'].split(None, 7)
            assert observed[:2] == ['31352', '31348'] and observed[7].split() == command
            success = read(FIRST_REVIEW, args.success_review_sha256, pins)
            assert success['status'] == 'closed-qualified-first-explicit-goal-bounded-case'
            assert success['verified_pins'] == 13486 and success['report_sha256'] == FIRST_REPORT_SHA
            assert success['body_terminal_sha256'] == FIRST_BODY_SHA and success['durable_terminal_sha256'] == FIRST_OWNER_SHA
            assert success['cases'] == 1 and success['qualification_checks_passed'] == 20
            assert success['stdout_bytes'] == 254 and success['stderr_empty'] is True
            assert success['stdout_sha256'] == 'cab36666f9ebc62cafa303203e3f7031a7eabec7e1b44d00429773b72b153b53'
            assert success['successful_compiler_commands_reused'] == 3 and success['successful_QLF_artifacts_reused'] == 2
            assert success['recompilations'] == success['fixture_reexecutions'] == success['benchmark_runs'] == 0
            assert success['remaining_preparation_cases'] == 155 and success['remaining_source_profiles_to_build'] == 19
            first = read(FIRST / 'report.json', FIRST_REPORT_SHA, pins)
            first_ended = read(FIRST / 'terminal-pin.json', FIRST_BODY_SHA, pins)
            assert first['status'] == first_ended['status'] == 'explicit-goal-first-bounded-case-passed-awaiting-remaining-preparation'
            assert first['terminal_evidence_complete'] is True and first_ended['report_sha256'] == FIRST_REPORT_SHA
            assert first_ended['partial_evidence_pins'] == first['pins']
            assert set(first['profiles']) == {ID} and len(first['attempts']) == 1
            for path, digest in first['pins'].items(): bind(path, digest, pins)
            first_done = read(FIRST_OWNER / 'terminal.json', FIRST_OWNER_SHA, pins)
            first_request, first_launch, first_worker, first_start = [
                read(FIRST_OWNER / name, sha(FIRST_OWNER / name), pins) for name in
                ('request.json', 'launch.json', 'worker-started.json', 'command-started.json')]
            first_command = [command[0], '-I', '-S', '-B', '-u',
                str(H / 'revisions/explicit-goal-v1/diagnose.py'),
                '--worker-sha256', FIRST_WORKER_SHA, '--review-sha256', FIRST_NOTE_SHA,
                '--closed-review-sha256', FAILED_REVIEW_SHA]
            assert all(item['command'] == first_command and item['cwd'] == str(H.parents[3])
                       for item in (first_request, first_launch, first_worker, first_start))
            assert first_request['common_mutex_owned_by_target_command'] is True
            assert first_done['status'] == 'command-exited-zero-awaiting-report-review'
            assert type(first_done['exit_code']) is int and first_done['exit_code'] == 0
            assert first_done['actual_command_started'] is True and first_done['stop_signals'] == []
            assert first_done['automatic_retry'] is first_done['qualification_granted'] is False
            assert first_worker['worker_identity'] == first_done['worker_identity'] and first_worker['inherited_private_lease'] is True
            assert first_start['command_identity'] == first_done['command_identity']
            assert first_start['command_pid'] == first_done['command_pid'] == 34284
            assert first_worker['worker_identity']['pid'] == first_launch['worker_identity']['pid'] == 34280
            assert first_launch['worker_identity']['listing'].split()[2:] == first_worker['worker_identity']['listing'].split()[2:]
            assert first_start.get('command_identity_observation_missing', False) is first_done['command_identity_observation_missing'] is False
            first_observed = first_start['command_identity']['listing'].split(None, 7)
            assert first_observed[:2] == ['34284', '34280'] and first_observed[7].split() == first_command
            bind(H / 'revisions/explicit-goal-v1/diagnose.py', FIRST_WORKER_SHA, pins)
            bind(H / 'revisions/explicit-goal-v1/REVIEW.md', FIRST_NOTE_SHA, pins)
            bind(PREP, PREP_SHA, pins)
            prep = cold(PREP, 'remaining_preparation_original_controller', pins)
            prep.verify(pins)
            cfg, runtime_pins, binary, home = prep.runtime_config(argparse.Namespace(
                runtime_config=prep.RUNTIME, runtime_config_sha256=prep.RUNTIME_SHA))
            pins = prep.merge(pins, runtime_pins)
            # Direct cold loading is explicit. Calling the old reused_service
            # here would reject this new entry under its original cache rule.
            service = cold(prep.SERVICE, 'remaining_preparation_original_owned_service', pins)
            references = cold(prep.REFERENCES, 'remaining_preparation_original_references', pins)
            guard = cold(service.GUARD, 'remaining_preparation_original_guard', pins)
            contracts = cold(PREP.parent / 'contracts.py', 'remaining_preparation_original_contracts', pins)
            harness = guard.load_harness()
            for loaded in (harness, harness.compact, harness.compact.common):
                path = Path(loaded.__file__).resolve(strict=True)
                bind(path, pins[str(path)], pins)
            prep.verify(pins)
            row = copy.deepcopy(old['profiles'][ID])
            assert row['module'] == 'clbg_regexredux' and row['benchmark'] == 'regexredux' and row['stdin'] is True
            source = bind(row['measured_source'], row['source_sha256'], pins)
            assert source.read_text().splitlines()[:4] == [
                ':- module(clbg_regexredux, [main/0]).',
                ":- use_module('../../support/clbg_io').",
                ':- use_module(library(pcre), [re_foldl/6, re_replace/5]).',
                ':- initialization(' + GOAL + ', main).']
            for path, digest in row['application_artifact_pins'].items():
                bind(path, digest, pins)
            for item in review['successful_QLF_compile_and_inspect_commands']:
                native = read(item['path'], item['sha256'], pins)
                prep.owned_receipt(native, native['command'])
            assert len(review['successful_QLF_compile_and_inspect_commands']) == 3
            for item in row['compile_receipts']:
                native = read(item['receipt'], item['receipt_sha256'], pins)
                prep.owned_receipt(native, item['command'])
                inventory = read(item['input_inventory_receipt'], item['input_inventory_receipt_sha256'], pins)
                contracts.runtime_flags(inventory['runtime'], home)
                prep.compiler_libraries(inventory, cfg)
            compiler = row['compiler_bound_input_inventory']
            loaded = read(compiler['receipt'], compiler['receipt_sha256'], pins)
            contracts.runtime_flags(loaded['runtime'], home)
            prep.compiler_libraries(loaded, cfg)
            expected_sources = [Path(item['path']) for item in row['source_bundle']]
            assert contracts.compiler_inputs(loaded['loaded_sources'], expected_sources, home,
                PREP.parent / 'tools/prepare_qlf.pl') == compiler['inputs']
            bind(compiler['original_scored_wam'], compiler['original_scored_wam_sha256'], pins)
            assert row['command'] == prep.prefix(binary) + ['-l', str(expected_sources[0].with_suffix('.qlf')),
                '-s', str(source.with_suffix('.qlf'))]
            row['command'] = row['command'] + ['-g', GOAL]
            original_case = PRIOR / 'bounded-output' / ID / 'all-replacements'
            payload = bind(original_case / 'stdin.fasta', pins[str(original_case / 'stdin.fasta')], pins).read_bytes()
            expected = references.regexredux(payload)
            assert (original_case / 'expected.stdout').read_bytes() == expected
            passed_row = first['profiles'][ID]
            assert passed_row == row
            reused = copy.deepcopy(first['attempts'][0])
            assert reused['id'] == ID and reused['case'] == 'all-replacements-explicit-goal'
            assert reused['command'] == passed_row['command'] and reused['source_sha256'] == row['source_sha256']
            assert json.loads(reused['execution_fingerprint']) == dict(binary_sha256=row['binary_sha256'],
                command=row['command'], cwd=row['cwd'], environment=row['runtime_environment'],
                source_sha256=row['source_sha256'], support={item['path']:item['sha256'] for item in row['source_bundle']})
            contracts.complete_attempt(reused, False); prep.native_raw_receipts(reused)
            assert reused['stdout_bytes'] == 254 and reused['stdout_sha256'] == success['stdout_sha256']
            assert reused['stderr'] == '' and reused['validation']['sha256'] == success['stdout_sha256']
            assert Path(reused['result_path']).resolve().is_relative_to(FIRST / 'attempts')
            assert Path(reused['supervisor_result_path']).resolve().is_relative_to(FIRST / 'attempts')
            raw = read(reused['result_path'], pins[reused['result_path']], pins)
            enriched = read(reused['supervisor_result_path'], pins[reused['supervisor_result_path']], pins)
            assert raw['execution_fingerprint'] == enriched['execution_fingerprint'] == reused['execution_fingerprint']
            sealed = read(prep.PROPOSAL, 'c36911c1e2f9b8c2f8897667e15474ecda619cd3284115ed590d52f0d43e8f4b', pins)
            assert sealed['status'] == 'sealed-awaiting-root-review'
            assert sealed['local_fixture_receipt']['status'] == 'passed' and sealed['local_fixture_receipt']['tests_run'] == 6
            assert sealed['reused_generic_fixture_count'] == 11 and sealed['fixture_reexecutions'] == sealed['runtime_reexecutions'] == 0
            original, pins, derivation = prep.original_proposal(pins)
            assert derivation == sealed['source_derivation']
            service_proposal = read(prep.SERVICE_PROPOSAL, sealed['service_proposal_sha256'], pins)
            fixture = read(service_proposal['actual_mock_fixture_receipt'],
                service_proposal['actual_mock_fixture_receipt_sha256'], pins)
            assert fixture['status'] == 'passed' and fixture['inventory'] == 'ownership'
            assert fixture['expected_test_ids'] == prep.OWNERSHIP_IDS
            assert type(fixture['tests_run']) is type(fixture['discovered_tests']) is int
            assert fixture['tests_run'] == fixture['discovered_tests'] == 11 and fixture['successful'] is True
            assert len(fixture['actual_test_ids']) == len(set(fixture['actual_test_ids'])) == 11
            assert set(fixture['actual_test_ids']) == set(prep.OWNERSHIP_IDS)
            assert all(fixture[key] == [] for key in ('skipped', 'failures', 'errors', 'expected_failures', 'unexpected_successes'))
            rows = contracts.profiles(read(prep.PACKET / 'profiles.json', pins[str(prep.PACKET / 'profiles.json')], pins))
            assert rows == sealed['profiles'] == original['profiles']
            cases = prep.bounded_cases(service, references)
            expected_cases = {(item['id'], label) for item in rows for label, _, _ in cases[item['benchmark']]}
            assert len(expected_cases) == sum(len(cases[item['benchmark']]) for item in rows) == 156
            assert {bench:[label for label,_,_ in cases[bench]] for bench in contracts.BENCHMARKS} == sealed['case_inventory']
            guard.OUT, guard.REPORT = output, output / 'report.json'
            harness.install_interrupt_handlers()
            harness.compact.wait_for_environment(harness.REQUIRED)
            environment = dict(harness.compact.ENV) | cfg['environment']
            report.update(original_failed_report=dict(path=str(PRIOR / 'report.json'), sha256=REPORT_SHA),
                reused_success_report=dict(path=str(FIRST / 'report.json'), sha256=FIRST_REPORT_SHA),
                retained_contract_fixture_count=6, retained_ownership_fixture_count=11,
                runtime_config=str(prep.RUNTIME), runtime_config_sha256=prep.RUNTIME_SHA,
                successful_compiler_commands_reused=3, successful_QLF_artifacts_reused=2)
            report['reused_attempts'] = [reused]
            report['case_results'] = [dict(id=ID, case='all-replacements', reused=True,
                actual_case=reused['case'], result_path=reused['result_path'], supervisor_result_path=reused['supervisor_result_path'])]
            completed = {(ID, 'all-replacements')}
            for declared in rows:
                goal = 'clbg_io:run(' + declared['module'] + ':main)'
                original_source = bind(prep.PACKET / declared['source'], pins[str(prep.PACKET / declared['source'])], pins)
                lines = original_source.read_text().splitlines()
                assert lines[0] == ':- module(' + declared['module'] + ', [main/0]).'
                assert lines.count(':- initialization(' + goal + ', main).') == 1
                if declared['variant'] == 'optimized':
                    assert original_source.read_bytes() == (prep.PACKET / 'sources' / declared['benchmark'] / 'compact.pl').read_bytes()
                prep.verify(pins)
                if declared['id'] == ID:
                    row = copy.deepcopy(passed_row)
                else:
                    row = copy.deepcopy(declared)
                    build = output / 'builds' / row['id']
                    input_root = build / 'input'
                    expected_authored = []
                    bundles = []
                    artifacts = []
                    compile_receipts = []
                    for relative in row['support_files'] + [row['source']]:
                        original = prep.PACKET / relative
                        target = input_root / relative
                        target.parent.mkdir(parents=True, exist_ok=True)
                        assert not target.exists(), 'Never replace a prior source copy'
                        shutil.copyfile(original, target)
                        assert prep.sha(target) == pins[str(original.resolve())]
                        pins[str(target)] = prep.sha(target)
                        expected_authored.append(target)
                        bundles.append(dict(path=str(target), original_path=str(original),
                            sha256=prep.sha(target), role='main-source' if relative == row['source'] else 'counted-authored-helper'))
                        receipt = build / (target.stem + '-compile.json')
                        wam = build / (target.stem + '-compile.wam')
                        assert not receipt.exists() and not wam.exists()
                        command = prep.compiler_command(binary, 'compile', target, receipt, wam, input_root)
                        service.owned_command(guard, harness, command, environment, build, common.fileno(), 120,
                                              target.stem + '-compile-command')
                        actual = json.loads(receipt.read_text())
                        contracts.runtime_flags(actual['runtime'], home)
                        prep.compiler_libraries(actual, cfg)
                        owned_receipt = build / (target.stem + '-compile-command.json')
                        owned = json.loads(owned_receipt.read_text())
                        assert owned['command'] == command and owned['inherited_common_mutex'] is True
                        assert type(owned['exit_status']) is int and owned['exit_status'] == 0
                        assert type(owned['final_exit_status']) is int and owned['final_exit_status'] == 0
                        assert not owned.get('error') and not owned.get('cleanup_errors')
                        compile_receipts.append(dict(source=str(target), command=command,
                            receipt=str(owned_receipt), receipt_sha256=prep.sha(owned_receipt),
                            input_inventory_receipt=str(receipt), input_inventory_receipt_sha256=prep.sha(receipt)))
                        artifact = target.with_suffix('.qlf')
                        assert artifact.is_file()
                        artifacts.append(artifact)
                    main = expected_authored[-1]
                    preloads = artifacts[:-1]
                    inspect_receipt = build / 'loaded-inputs.json'
                    inspect_wam = build / 'original-scored.wam'
                    command = prep.compiler_command(binary, 'inspect', artifacts[-1], inspect_receipt, inspect_wam,
                                               input_root, preloads)
                    service.owned_command(guard, harness, command, environment, build, common.fileno(), 120, 'inspect-command')
                    actual = json.loads(inspect_receipt.read_text())
                    assert inspect_wam.stat().st_size > 0, 'Missing actual original scored application VM code'
                    contracts.runtime_flags(actual['runtime'], home)
                    prep.compiler_libraries(actual, cfg)
                    roles = contracts.compiler_inputs(actual['loaded_sources'], expected_authored,
                                                      home, prep.PACKET / 'tools/prepare_qlf.pl')
                    # Every vendor module/boot/library is bound to the reviewed private
                    # runtime corpus. Original source inventories and QLF artifacts have
                    # separate roles; generated WAM is never the source-count substitute.
                    for role in roles:
                        target = role['resolved_path']
                        pins[target] = prep.sha(target)
                        if role['role'] == 'vendor-runtime-library':
                            assert pins[target] == cfg['pins'][target], ('Unreviewed loaded runtime source', target)
                    local = {str(path): prep.sha(path) for path in build.rglob('*') if path.is_file()}
                    pins = prep.merge(pins, local)
                    invocation = prep.prefix(binary)
                    for artifact in preloads:
                        invocation += ['-l', str(artifact)]
                    invocation += ['-s', str(artifacts[-1]), '-g', goal]
                    if not row['stdin']:
                        invocation += ['--', '{n}']
                    row.update(source=str(prep.PACKET / declared['source']), measured_source=str(main),
                        source_sha256=prep.sha(main), source_bundle=bundles, binary=str(binary), binary_sha256=prep.sha(binary),
                        command=invocation, cwd=str(input_root), runtime_environment=cfg['environment'],
                        application_artifact_pins={str(path): prep.sha(path) for path in artifacts},
                        compile_receipts=compile_receipts,
                        compiler_bound_input_inventory=dict(status='qualified-exact-compiled-inputs',
                            receipt=str(inspect_receipt), receipt_sha256=prep.sha(inspect_receipt), inputs=roles,
                            original_scored_wam=str(inspect_wam), original_scored_wam_sha256=prep.sha(inspect_wam)),
                        compiler_identity=cfg['version_receipt'], runtime_closure_pins=runtime_pins,
                        execution_mode='SWI-Prolog native bytecode VM; QLF application loading inside timer',
                        runtime_policy='Default term/atom/clause garbage collection; uniform8g stack ceiling',
                        backend_verified=True, bounded_output_qualified=False)

                    report['new_source_profiles_built'] += 1
                report['profiles'][row['id']] = row
                guard.save(guard.REPORT, report)
                for label, n, payload in cases[row['benchmark']]:
                    identity = (row['id'], label)
                    if identity == (ID, 'all-replacements'):
                        assert identity in completed
                        continue
                    assert identity not in completed
                    oracle = getattr(references, row['benchmark'])
                    expected = oracle(payload if payload is not None else n)
                    attempt = service.probe(guard, harness, row, n, payload, expected, pins,
                        output / 'bounded-output', common.fileno(), report, label)
                    contracts.complete_attempt(attempt, False); prep.native_raw_receipts(attempt)
                    assert attempt['stderr'] == ''
                    completed.add(identity)
                    report['new_bounded_cases'] += 1
                    report['case_results'].append(dict(id=row['id'], case=label, reused=False,
                        actual_case=attempt['case'], result_path=attempt['result_path'],
                        supervisor_result_path=attempt['supervisor_result_path']))
                    guard.save(guard.REPORT, report)
                row['bounded_output_qualified'] = True
            assert completed == expected_cases and len(report['case_results']) == 156
            assert len(report['profiles']) == 20 and len(report['attempts']) == report['new_bounded_cases'] == 155
            assert len(report['reused_attempts']) == 1 and report['new_source_profiles_built'] == 19
            assert all(attempt['qualified'] is True for attempt in report['attempts'] + report['reused_attempts'])
            report.update(status='bounded-output-ownership-passed-awaiting-work-and-LOC-review',
                all20_bounded_outputs_qualified=True, total_bounded_cases=156)
        except BaseException as error:
            failure = error
            try:
                (output / 'error.log').write_text(traceback.format_exc())
            except BaseException as journal_error:
                report['error_log_write_failure'] = str(journal_error)
        finally:
            if prep is not None:
                prep.terminal(output, report, pins, failure)
            else:
                errors, actual = [], {}
                candidates = dict(pins)
                candidates.update({str(path): None for path in output.rglob('*') if path.is_file() and str(path) not in candidates})
                for path, expected in candidates.items():
                    try:
                        actual[path] = sha(path)
                        assert expected is None or actual[path] == expected
                    except BaseException as error:
                        errors.append(dict(path=path, error=str(error)))
                report.update(status='failed-before-service', error=str(failure), pins=pins, available_pins=actual,
                              pin_errors=errors)
                with (output / 'report.json').open('x') as stream:
                    json.dump(report, stream, indent=2)
            if failure is not None:
                raise failure


if __name__ == '__main__':
    main()
