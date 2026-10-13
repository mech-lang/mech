"""Seal existing complete Gleam/Erlang counts after independent exact review.

No counting, parser, compiler, native subject, fixtures or live dataset writes.
The unchanged combiner and every successful prior result remain authoritative.
"""
import argparse
from contextlib import ExitStack
from copy import deepcopy
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import sys
import traceback

HERE = Path(__file__).resolve().parent
GLEAM = HERE.parent
CLBG = GLEAM.parents[1]
ROOT = CLBG.parents[1]
CONT = CLBG / 'continuation'
LOCK = Path('/tmp/mech-clbg-timing.lock')
GLEAM_COUNTS = CLBG / 'loc-normalization/20261012/gleam-proposal-v2/jobs/source20-repaired-v3/source-count-proposals.json'
GLEAM_SHA = '6eba15be3c2a53189d5fc11b560a51ff89516f96ad45893eb419e0a31cd4263b'
ERLANG_COUNTS = CLBG / 'loc-normalization/20261012/gleam-erlang-helpers-v1/jobs/helpers-entry-repaired-v2/helper-count-proposals.json'
ERLANG_SHA = '8ba774ecc399ec7dc94778db7afdce55ad20e1c0c1b8dbb0e808abc3be4e2d02'
PREPARATION = GLEAM / 'qualification/private-v5/report.json'
PREPARATION_SHA = 'a19f2d079b917f1172b54d09e494e72c5a7aea9d6a9301ef0974633b4d4f30dc'
SUITE = GLEAM / 'reconciliation/full-v4-suite-v2.json'
INDEX_SHA = 'd5eff42c8f88733af92a8e597b3e02c2b3e8212b24b827b17a494cfabd54497b'
LAUNCHER = '/opt/homebrew/opt/python@3.14/bin/python3.14'
FRAMEWORK = '/opt/homebrew/Cellar/python@3.14/3.14.8/Frameworks/Python.framework/Versions/3.14/Resources/Python.app/Contents/MacOS/Python'
HOST = CONT / 'jobs/python-framework-host-linkage-v1/host-linkage.json'
HOST_SHA = '9a1c0fa4fc330cf0c7c176349c220640ad4714c5e8e081eaead84c00b89729af'
BENCHMARKS = ('binarytrees', 'fannkuchredux', 'nbody', 'spectralnorm', 'mandelbrot',
              'pidigits', 'fasta', 'revcomp', 'knucleotide', 'regexredux')
IDS = {'gleam-' + benchmark + '-' + variant for benchmark in BENCHMARKS for variant in ('compact', 'optimized')}
JOB_COMMANDS = {
    'gleam-source20-entry-repaired-v3': [FRAMEWORK, '-B', '-u', str(CONT / 'source20-entry-repairs-v1/replay.py'), '--language', 'gleam', '--revision', 'source20-repaired-v3'],
    'gleam-erlang-helpers26-entry-repaired-v2': [FRAMEWORK, '-B', '-u', 'benchmarks/clbg/continuation/gleam-helper-entry-repair-v1/replay.py', '--revision', 'helpers-entry-repaired-v2'],
    'gleam-full90-independent-reconcile-v2': [LAUNCHER, '-B', '-u', str(CONT / 'reconcile_native_suite_v2.py'), '--language', 'gleam', '--study-root', str(GLEAM / 'studies/full-v4'), '--reviewed-index-sha256', INDEX_SHA, '--output', str(SUITE)],
}


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def read(path): return json.loads(Path(path).read_text())


def new(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def merge(pins, group):
    for path, digest in group.items():
        assert re.fullmatch('[a-f0-9]{64}', digest), ('Invalid evidence digest', path)
        assert path not in pins or pins[path] == digest, ('Conflicting evidence', path)
        pins[path] = digest


def verify(pins):
    for path, digest in pins.items(): assert sha(path) == digest, ('Changed original evidence', path)


def closed_jobs(review, pins, stack):
    assert set(review) == set(JOB_COMMANDS)
    host = read(HOST)
    assert host['status'] == 'root-reviewed-exact-python-framework-linkage'
    assert host['same_image_bytes_claimed'] is host['benchmark_runtime_image_changed'] is False
    assert host['framework']['path'] == FRAMEWORK
    merge(pins, {host['launcher']['resolved_path']: host['launcher']['sha256'], FRAMEWORK: host['framework']['sha256']})
    verify(pins)
    for name, command in JOB_COMMANDS.items():
        job = CONT / 'owned-jobs' / name
        required = {str(job / leaf) for leaf in ('request.json', 'launch.json', 'worker-started.json', 'command-started.json', 'terminal.json')}
        assert set(review[name]) == required
        merge(pins, review[name]); verify(review[name])
        lease = stack.enter_context((job / 'owner.lock').open('rb'))
        fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
        opened, named = os.fstat(lease.fileno()), (job / 'owner.lock').stat()
        assert (opened.st_dev, opened.st_ino) == (named.st_dev, named.st_ino)
        request, launch, worker, started, terminal = (read(job / leaf) for leaf in ('request.json', 'launch.json', 'worker-started.json', 'command-started.json', 'terminal.json'))
        assert request['command'] == launch['command'] == worker['command'] == started['command'] == command
        assert request['cwd'] == launch['cwd'] == worker['cwd'] == started['cwd'] == str(ROOT)
        assert request['common_mutex_owned_by_target_command'] is worker['common_mutex_owned_by_target_command'] is True
        assert request['automatic_retry'] is request['qualification_granted'] is worker['qualification_granted'] is False
        assert worker['inherited_private_lease'] is worker['detached_session'] is True
        assert terminal['status'] == 'command-exited-zero-awaiting-report-review'
        assert terminal['actual_command_started'] is True and type(terminal['exit_code']) is int and terminal['exit_code'] == 0
        assert terminal['stop_signals'] == [] and not any(terminal.get(key) for key in ('error', 'error_type', 'error_log_write_failure', 'exception_cleanup'))
        assert terminal['command_identity_observation_missing'] is started['command_identity_observation_missing'] is False
        assert terminal['command_pid'] == started['command_pid'] == terminal['command_identity']['pid']
        assert terminal['command_identity'] == started['command_identity']
        assert terminal['worker_identity'] == worker['worker_identity']
        initial, final = (value['worker_identity']['listing'].split(None, 8) for value in (launch, terminal))
        assert len(initial) == len(final) == 9
        assert initial[0] == final[0] == str(terminal['worker_identity']['pid']) and initial[2:7] == final[2:7]
        assert initial[7] == LAUNCHER and final[7] == FRAMEWORK
        rest = '-B -u ' + str(CONT / 'launch_durable_v1.py') + ' --worker ' + str(job) + ' --lease-fd 3 --go-fd 4'
        assert initial[8] == final[8] == rest and initial[1].isdigit() and final[1] == '1'
        image = terminal['command_identity']['listing'].split(None, 8)
        assert len(image) == 9 and image[0] == str(terminal['command_pid']) and image[1] == final[0]
        assert image[7] == FRAMEWORK and image[8] == ' '.join(command[1:])


def assemble(args, pins, stack, output):
    merge(pins, {str(args.combined): args.combined_sha256, str(SUITE): args.suite_sha256,
        str(args.review): args.review_sha256, str(PREPARATION): PREPARATION_SHA,
        str(GLEAM_COUNTS): GLEAM_SHA, str(ERLANG_COUNTS): ERLANG_SHA, str(HOST): HOST_SHA})
    verify(pins)
    combined, suite, review, preparation = read(args.combined), read(SUITE), read(args.review), read(PREPARATION)
    assert review['status'] == 'exact-gleam-erlang-source-owners-reviewed-awaiting-guarded-seal'
    assert review['language'] == 'gleam' and review['review_identity'] and review['unresolved'] == []
    assert review['canonical_LOC_admitted'] is review['chart_admission'] is review['peer_review_pending'] is False
    assert review['combined_proposals'] == dict(path=str(args.combined), sha256=args.combined_sha256)
    assert review['actual_suite'] == dict(path=str(SUITE), sha256=args.suite_sha256)
    peer = review['source_only_peer_review']
    assert peer['status'] == 'accepted-all-exact-source-metrics-and20-complete-bundles' and peer['review_identity'] and peer['review_identity'] != review['review_identity']
    assert review['pins'][peer['path']] == peer['sha256']
    merge(pins, review['pins']); verify(pins)
    actual_peer = read(peer['path'])
    assert actual_peer['status'] == peer['status'] and actual_peer['review_identity'] == peer['review_identity']
    assert actual_peer['combined_proposals'] == review['combined_proposals'] and actual_peer['actual_suite'] == review['actual_suite']
    assert actual_peer['profile_ids'] == sorted(IDS) and set(actual_peer['source_paths']) == set(review['sources'])
    assert actual_peer['all_owner_metrics_and_source_roles_reviewed'] is actual_peer['all20_compiler_bundles_and_totals_reviewed'] is True
    closed_jobs(review['closed_original_jobs'], pins, stack)
    assert combined['status'] == 'complete20-authored-source-count-proposals-awaiting-independent-semantic-review'
    assert combined['qualification'] == dict(path=str(PREPARATION), sha256=PREPARATION_SHA)
    assert combined['profile_count'] == 20 and type(combined['profile_count']) is int
    assert combined['actual_common_mutex_held'] is True
    assert combined['canonical_LOC_admitted'] is combined['chart_admission'] is combined['source_or_timing_mutated'] is combined['count_override'] is False
    assert type(combined['full_performance_runs']) is int and combined['full_performance_runs'] == 0
    assert combined['count_receipts'] == dict(gleam=dict(path=str(GLEAM_COUNTS), sha256=GLEAM_SHA), erlang=dict(path=str(ERLANG_COUNTS), sha256=ERLANG_SHA))
    assert suite['provenance'][str(GLEAM / 'studies/full-v4/index.json')] == INDEX_SHA
    merge(pins, combined['pins']); merge(pins, suite['provenance']); merge(pins, preparation['pins']); verify(pins)
    terminal_path = args.combined.with_suffix('.evidence') / 'terminal-pins.json'
    assert review['pins'][str(terminal_path)] == sha(terminal_path)
    terminal = read(terminal_path)
    assert terminal['status'] == 'completed-complete-bundle-proposals' and terminal['actual_common_mutex_held'] is True
    assert terminal['canonical_LOC_admitted'] is terminal['chart_admission'] is False
    assert terminal['pins'][str(args.combined)] == args.combined_sha256
    merge(pins, terminal['pins']); merge(pins, terminal['prerequisite_pins']); verify(pins)
    assert suite['language'] == 'gleam' and suite['status'] == 'ten-native-workloads-reconciled-awaiting-canonical-LOC-and-publication'
    assert set(suite['benchmarks']) == set(BENCHMARKS)
    assert type(suite['canonical_full_native_processes']) is int and suite['canonical_full_native_processes'] == 90
    assert type(suite['candidate_sources']) is int and suite['candidate_sources'] == 20
    assert type(suite['actual_samples_per_source']) is int and suite['actual_samples_per_source'] == 3
    assert suite['source_timing_relabels'] == suite['source_or_report_mutations'] == 0 and suite['chart_admitted'] is False
    original = read(GLEAM_COUNTS)
    fixture = original['fixture_receipt']; actual = read(fixture['path'])
    assert pins[fixture['path']] == fixture['sha256']
    assert actual['status'] == 'passed-awaiting-root-source-review' and actual['tests_run'] == actual['expected_count'] == 34
    assert len(set(actual['started_ids'])) == 34 and sorted(actual['expected_ids']) == sorted(actual['discovered_ids']) == sorted(actual['started_ids'])
    assert all(type(actual[key]) is int and actual[key] == 0 for key in ('failures', 'errors', 'skipped', 'expected_failures', 'unexpected_successes'))
    assert actual['pins_before'] == actual['pins_after'] and actual['actual_common_mutex_held'] is actual['pin_reconciliation_passed'] is True
    counts, meanings = combined['source_counts'], review['sources']
    assert set(counts) == set(meanings)
    for source, counted in counts.items():
        meaning, metric = meanings[source], counted['metric']
        assert meaning['status'] == 'exact-authored-source-owner-meaning-reviewed' and meaning['owner_review'] and meaning['source_role_review']
        assert meaning['source_sha256'] == counted['source_sha256'] == pins[source] == pins[counted['origin']]
        assert meaning['origin'] == counted['origin'] and meaning['role'] == counted['role']
        assert meaning['metric_reviewed'] == metric and meaning['unresolved'] == metric['diagnostics'] == []
        assert meaning['original_source_and_all_owner_spans_reviewed'] is True and meaning['all_source_resolution_certificates_reviewed'] is True
        assert type(metric['physical_loc']) is type(metric['normalized_loc']) is int and min(metric['physical_loc'], metric['normalized_loc']) > 0
        assert Path(source).read_bytes() == Path(counted['origin']).read_bytes()
    rows = {row['id']: row for row in preparation['profiles']}
    assert len(rows) == len(preparation['profiles']) == 20
    for bench in suite['benchmarks'].values():
        candidates = [row for row in bench['profiles'].values() if row['language'] == 'gleam']
        controls = [row for row in bench['profiles'].values() if row['language'] == 'mech']
        assert len(bench['profiles']) == 3 and len(candidates) == 2 and len(controls) == 1
        assert {row['variant'] for row in candidates} == {'compact', 'optimized'}
    scored = {row['id']: row for bench in suite['benchmarks'].values() for row in bench['profiles'].values() if row['language'] == 'gleam'}
    assert set(rows) == set(scored) == set(combined['profiles']) == set(combined['compiler_bound_input_inventory']) == set(review['profiles']) == IDS
    ownership = dict(status='passed', review_identity=review['review_identity'], evidence_path=str(args.review), evidence_sha256=args.review_sha256)
    profiles, inventories, staged, used = {}, {}, {}, set()
    for identity in sorted(IDS):
        row, timed, proposed, meaning = rows[identity], scored[identity], combined['profiles'][identity], review['profiles'][identity]
        assert row['source'] == timed['source'] == proposed['source'] and row['source_sha256'] == timed['source_sha256'] == proposed['source_sha256']
        assert row['binary_sha256'] == timed['binary_sha256'] == proposed['binary_sha256']
        assert row['source_bundle'] == timed['source_bundle'] and row['benchmark'] == timed['benchmark'] == meaning['benchmark']
        assert row['variant'] == timed['variant'] == meaning['variant']
        bound = combined['compiler_bound_input_inventory'][identity]; original_inventory = read(bound['path'])
        assert pins[bound['path']] == bound['sha256'] == row['compiled_input_inventory_sha256'] and bound['path'] == row['compiled_input_inventory']
        assert original_inventory['status'] == 'compiled-exact-authored-input-inventory' and original_inventory['profile'] == identity
        assert original_inventory['source_bundle'] == bound['source_bundle'] == row['source_bundle']
        assert original_inventory['source'] == row['source'] and original_inventory['source_sha256'] == row['source_sha256']
        assert original_inventory['compiled_artifacts'] == bound['compiled_artifacts'] == row['compiled_artifacts']
        assert original_inventory['runtime_classpath'] == bound['runtime_classpath'] == row['runtime_classpath']
        assert meaning['original_compiler_inventory_reviewed'] == original_inventory and meaning['complete_bundle_reviewed'] == proposed['source_bundle']
        assert meaning['authored_application_and_helpers_complete'] is True and meaning['unresolved'] == []
        assert proposed['canonical_loc'] is None and proposed['source_language_counts_complete'] is True
        assert proposed['independent_ownership_review_passed'] is proposed['independent_semantic_review_passed'] is False
        bundle = []
        assert len(proposed['source_bundle']) == len(row['source_bundle']) and row['source_bundle'][0]['path'] == row['source']
        for index, (item, original_item) in enumerate(zip(proposed['source_bundle'], row['source_bundle'])):
            counted = counts[item['path']]; metric = counted['metric']; used.add(item['path'])
            assert item == dict(original_item, normalized_loc=metric['normalized_loc'], physical_loc=metric['physical_loc'], diagnostics=[])
            assert item['role'] == ('application' if index == 0 else 'application-helper') and item['role'] == counted['role']
            assert item['sha256'] == counted['source_sha256'] and item['origin'] == counted['origin']
            receipt = combined['count_receipts']['erlang' if counted['source_language'] == 'erl' else 'gleam']
            bundle.append(dict(path=item['path'], sha256=item['sha256'], origin=item['origin'], role=item['role'],
                physical_loc=metric['physical_loc'], normalized_loc=metric['normalized_loc'], count_receipt=dict(receipt, counted_source=item['path']),
                ownership_review=deepcopy(ownership), unresolved_diagnostics=[]))
        physical, canonical = sum(item['physical_loc'] for item in bundle), sum(item['normalized_loc'] for item in bundle)
        assert canonical == proposed['proposed_complete_bundle_units'] == meaning['effective_normalized_loc'] and physical == meaning['effective_physical_loc']
        evidence = dict(status='qualified-exact-authored-compiler-inputs', profile=identity, source_sha256=row['source_sha256'], binary_sha256=row['binary_sha256'],
            files=[dict(path=item['path'], sha256=item['sha256']) for item in row['source_bundle']], original_compiler_inventory=bound,
            exact_source_role_review=dict(path=str(args.review), sha256=args.review_sha256), inventory_derived_from_original_actual_compile_receipt=True)
        evidence_path = output / 'compiler-inputs' / (identity + '.json')
        staged[identity] = (evidence_path, evidence)
        inventories[identity] = dict(status=evidence['status'], source_sha256=row['source_sha256'], binary_sha256=row['binary_sha256'],
            evidence_path=str(evidence_path), evidence_sha256=None, files=evidence['files'])
        profiles[identity] = dict(benchmark=row['benchmark'], variant=row['variant'], source=row['source'], source_sha256=row['source_sha256'],
            producer_loc=None, physical_loc=physical, canonical_loc=canonical, source_bundle=bundle, semantic_review=deepcopy(ownership))
    assert used == set(counts)
    verify(pins)
    helper_packet = CLBG / 'loc-normalization/20261012/gleam-erlang-helpers-v1'
    grammar = deepcopy(original['grammar_snapshot_pins'])
    for leaf in ('qualify_helpers.py', 'original_owner_probe.erl', 'erlang_owners.py', 'owner_fixture_cases.py', 'OWNER_RULES_PROPOSAL.md', 'REVIEW.md'):
        path = str(helper_packet / leaf); grammar[path] = pins[path]
    output.mkdir()
    for identity, (evidence_path, evidence) in staged.items():
        new(evidence_path, evidence); pins[str(evidence_path)] = sha(evidence_path)
        inventories[identity]['evidence_sha256'] = pins[str(evidence_path)]
    verify(pins)
    overlay = dict(schema_version=1, language='gleam', status='qualified-exact-source-LOC', profile_count=20,
        complete_authored_source_bundles=True, unresolved=[], normalization_policy=dict(gleam='source-bound-v2', erlang='original-source-owner-v1'),
        fixture_receipt=dict(path=fixture['path'], sha256=fixture['sha256'], status='passed', actual_methods=34),
        replay_receipts=[dict(path=str(args.combined), sha256=args.combined_sha256), *combined['count_receipts'].values()],
        grammar_snapshot_pins=grammar, compiler_bound_input_inventory=inventories, profiles=profiles, pins=deepcopy(pins),
        canonical_LOC_admitted=True, chart_admission=False, original_sources_or_timings_mutated=False,
        actual_native_subjects=0, actual_signals_sent=0, fixture_reexecutions=0, preserved_actual_suite=dict(path=str(SUITE), sha256=args.suite_sha256))
    path = output / 'qualified-loc-overlay.json'; new(path, overlay)
    print(json.dumps(dict(overlay=str(path), sha256=sha(path), profiles=20, native_processes=0, chart_admission=False)), flush=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--combined', type=Path, required=True); parser.add_argument('--combined-sha256', required=True)
    parser.add_argument('--suite-sha256', required=True); parser.add_argument('--review', type=Path, required=True)
    parser.add_argument('--review-sha256', required=True); parser.add_argument('--revision', required=True)
    args = parser.parse_args()
    assert re.fullmatch('[a-z0-9][a-z0-9-]{0,63}', args.revision)
    for value in (args.combined_sha256, args.suite_sha256, args.review_sha256): assert re.fullmatch('[a-f0-9]{64}', value)
    args.combined, args.review = args.combined.resolve(strict=True), args.review.resolve(strict=True)
    assert args.combined.is_relative_to(HERE) and args.review.is_relative_to(HERE)
    sys.dont_write_bytecode = True
    pins = {}; error = None; integrity = []
    print('WAIT common mutex: exact reviewed Gleam/Erlang LOC20 seal', flush=True)
    with LOCK.open('a+') as lock, ExitStack() as stack:
        fcntl.flock(lock, fcntl.LOCK_EX)
        opened, named = os.fstat(lock.fileno()), LOCK.stat()
        assert (opened.st_dev, opened.st_ino) == (named.st_dev, named.st_ino)
        output, journal = HERE / args.revision, HERE / 'overlay-seal-outcomes' / args.revision
        journal.mkdir(parents=True)
        try:
            assert not output.exists(), 'Never overwrite a prior or partial overlay'
            pins = {str(Path(__file__).resolve()): sha(__file__), str(HERE / 'REVIEW.md'): sha(HERE / 'REVIEW.md'), str(Path(sys.executable).resolve()): sha(Path(sys.executable).resolve())}
            assemble(args, pins, stack, output)
        except BaseException as failure:
            error = dict(error=str(failure) or type(failure).__name__, error_type=type(failure).__name__)
            (journal / 'actual-error.log').write_text(traceback.format_exc())
            raise
        finally:
            collected = {}
            candidates = dict(pins)
            candidates.update({str(path): None for folder in (output, journal) if folder.exists() for path in folder.rglob('*') if path.is_file()})
            for path, expected in candidates.items():
                try:
                    digest = sha(path)
                    assert expected is None or expected == digest, ('Changed terminal pin', path)
                    collected[path] = digest
                except BaseException as failure:
                    integrity.append(dict(path=path, error=str(failure) or type(failure).__name__, error_type=type(failure).__name__))
            new(journal / 'outcome.json', dict(status='failed-terminal-integrity' if integrity else 'failed-overlay-stage' if error else 'completed-overlay-stage',
                body_error=error, integrity_errors=integrity, pins=collected, actual_common_mutex_held=True,
                actual_native_subjects=0, actual_signals_sent=0, fixture_reexecutions=0, chart_admission=False, original_sources_or_timings_mutated=False))
            if integrity: raise RuntimeError('Exact Gleam LOC terminal integrity failed')


if __name__ == '__main__': main()
