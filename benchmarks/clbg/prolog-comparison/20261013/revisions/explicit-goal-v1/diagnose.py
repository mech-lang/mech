"""One explicit-goal bounded case using the already successful original QLF."""
import argparse
import copy
import fcntl
import hashlib
import importlib.util
import json
import os
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
    for name in ('worker', 'review', 'closed-review'):
        parser.add_argument('--' + name + '-sha256', required=True)
    args = parser.parse_args()
    assert sys.dont_write_bytecode
    print('WAIT common mutex: one source-bound explicit-goal diagnostic', flush=True)
    with LOCK.open('r+') as common, (OWNER / 'owner.lock').open('r+') as lease:
        fcntl.flock(common, fcntl.LOCK_EX)
        held, named = os.fstat(common.fileno()), LOCK.stat()
        assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino) and held.st_ino == 79859134
        fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
        held, named = os.fstat(lease.fileno()), (OWNER / 'owner.lock').stat()
        assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino)
        output = H / 'qualification/explicit-goal-first-case-v1'
        output.mkdir(parents=True, exist_ok=False)
        pins, prep, failure = {}, None, None
        report = dict(status='running-one-explicit-goal-diagnostic', profiles={}, attempts=[], active=None,
            chart_admission=False, source_work_qualified=False, canonical_loc=None, full_processes_authorized=0,
            recompilations=0, fixture_reexecutions=0, version_reexecutions=0, runtime_image_reexecutions=0,
            original_failure_relabelled=False, maximum_workers=1)
        try:
            for path, digest in ((P / 'diagnose.py', args.worker_sha256), (P / 'REVIEW.md', args.review_sha256)):
                assert not path.stat().st_mode & 0o222
                bind(path, digest, pins)
            assert all(not getattr(item, '__file__', None) or
                       not Path(item.__file__).resolve().is_relative_to(H.parents[1]) or
                       Path(item.__file__).resolve() == P / 'diagnose.py' for item in list(sys.modules.values()))
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
            bind(PREP, PREP_SHA, pins)
            prep = cold(PREP, 'explicit_goal_original_preparation', pins)
            prep.verify(pins)
            cfg, runtime_pins, binary, home = prep.runtime_config(argparse.Namespace(
                runtime_config=prep.RUNTIME, runtime_config_sha256=prep.RUNTIME_SHA))
            pins = prep.merge(pins, runtime_pins)
            # Direct cold loading is explicit. Calling the old reused_service
            # here would reject this new entry under its original cache rule.
            service = cold(prep.SERVICE, 'explicit_goal_original_owned_service', pins)
            references = cold(prep.REFERENCES, 'explicit_goal_original_references', pins)
            guard = cold(service.GUARD, 'explicit_goal_original_guard', pins)
            contracts = cold(PREP.parent / 'contracts.py', 'explicit_goal_original_contracts', pins)
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
            guard.OUT, guard.REPORT = output, output / 'report.json'
            harness.install_interrupt_handlers()
            harness.compact.wait_for_environment(harness.REQUIRED)
            report['profiles'][ID] = row
            report['original_failed_report'] = dict(path=str(PRIOR / 'report.json'), sha256=REPORT_SHA)
            report['corrected_command'] = row['command']
            attempt = service.probe(guard, harness, row, 0, payload, expected, pins,
                output / 'bounded-output', common.fileno(), report, 'all-replacements-explicit-goal')
            contracts.complete_attempt(attempt, False)
            prep.native_raw_receipts(attempt)
            assert attempt['stderr'] == '' and len(report['attempts']) == 1
            report.update(status='explicit-goal-first-bounded-case-passed-awaiting-remaining-preparation',
                successful_compiler_commands_reused=3, successful_QLF_artifacts_reused=2,
                actual_bounded_cases=1, all20_preparation_qualified=False)
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
