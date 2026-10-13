"""Additive v5-grammar exact C3 LOC overlay; actual replay/review required.

The original v1 candidate and rejected v4-grammar proposals remain unchanged.
No parser, compiler, native subject, fixture or chart admission runs here.
"""
import argparse
from contextlib import contextmanager
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
C3 = HERE.parent
LOC = C3.parents[1] / 'loc-normalization/20261012/c3-source-replay-v2'
REPLAY_ROOT = LOC / 'jobs/grammar5-source24-v1'
REPLAY = REPLAY_ROOT / 'source-count-proposals.json'
INVENTORY = REPLAY_ROOT / 'compiler-input-proposals.json'
WORKER = LOC / 'qualify_replay.py'
PREPARATION = C3 / 'qualification/private-v3/report.json'
PREPARATION_SHA = '7cb4a25819935f3e15b3c270595efc9df3dbc056982625892abc3b16bef2f20c'
LOCK = Path('/tmp/mech-clbg-timing.lock')
BENCHMARKS = ('binarytrees', 'fannkuchredux', 'nbody', 'spectralnorm', 'mandelbrot',
    'pidigits', 'fasta', 'revcomp', 'knucleotide', 'regexredux')
IDS = {'c3-' + name + '-' + form for name in BENCHMARKS for form in ('compact', 'optimized')}


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def read(path):
    return json.loads(Path(path).read_text())


def new(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def merge(pins, values):
    for path, digest in values.items():
        assert path not in pins or pins[path] == digest, ('Conflicting pin', path)
        pins[path] = digest


def verify(pins):
    for path, digest in pins.items():
        assert sha(path) == digest, ('Changed exact input/evidence', path)


@contextmanager
def terminal(args, state):
    with LOCK.open('a+') as lock:
        print('WAIT common mutex: reviewed exact C3 LOC24 overlay', flush=True)
        fcntl.flock(lock, fcntl.LOCK_EX)
        opened, named = os.fstat(lock.fileno()), LOCK.stat()
        assert (opened.st_dev, opened.st_ino) == (named.st_dev, named.st_ino)
        output = HERE / args.revision
        journal = HERE / 'overlay-seal-outcomes' / args.revision
        journal.mkdir(parents=True)  # Exclusive outcome, including preflight failure.
        error, pin_error = None, None
        try:
            assert not output.exists(), 'Never overwrite an earlier or partial overlay'
            yield output
        except BaseException as failure:
            error = dict(error=str(failure) or type(failure).__name__, error_type=type(failure).__name__)
            (journal / 'actual-error.log').write_text(traceback.format_exc())
            raise
        finally:
            collected = {}
            candidates = dict(state.get('pins', {}))
            candidates.update({str(path): None for root in (output, journal) if root.exists()
                for path in root.rglob('*') if path.is_file()})
            for path, expected in candidates.items():
                try:
                    digest = sha(path)
                    if expected is not None: assert digest == expected, ('Terminal changed pin', path)
                    collected[path] = digest
                except BaseException as failure:
                    if pin_error is None:
                        pin_error = dict(path=path, error=str(failure) or type(failure).__name__,
                            error_type=type(failure).__name__)
            new(journal / 'outcome.json', dict(status='failed-terminal-integrity' if pin_error else
                ('failed-overlay-stage' if error else 'completed-overlay-stage'), body_error=error,
                integrity_error=pin_error, pins=collected, available_bytes_not_fully_pinned=pin_error is not None,
                actual_common_mutex_held=True, common_timing_mutex=str(LOCK), actual_native_subjects=0,
                actual_signals_sent=0, original_sources_or_timings_mutated=False, chart_admission=False))
            if pin_error is not None: raise RuntimeError('C3 overlay terminal integrity failure: ' + str(pin_error))


def source_review(item, source, counted):
    assert item['status'] == 'source-owner-meaning-reviewed-awaiting-guarded-seal'
    assert item['source'] == source and item['source_sha256'] == counted['source_sha256']
    assert item['unresolved'] == [] and item['owner_review'] and item['source_role_review']
    metric = counted['metric']
    assert metric['proposal_only'] is True and metric['eligible_for_chart'] is False
    assert metric['diagnostics'] == [], 'A new grammar/source-specific evidence packet is required; no blanket diagnostic waiver'
    assert type(metric['physical_loc']) is type(metric['normalized_loc']) is int
    assert type(item['physical_loc']) is type(item['normalized_loc']) is int
    assert metric['physical_loc'] > 0 and metric['normalized_loc'] > 0
    assert item['physical_loc'] == metric['physical_loc'] and item['normalized_loc'] == metric['normalized_loc']
    assert item['canonical_owner_events_reviewed'] is True
    assert item['canonical_events_reviewed'] == metric['canonical_events']
    assert item['expansions_reviewed'] == metric['expansions']
    return metric


def assemble(args):
    state = dict(pins={})
    with terminal(args, state) as output:
        state['pins'] = {str(REPLAY): args.replay_sha256, str(PREPARATION): PREPARATION_SHA,
            str(args.review_file): args.review_sha256, str(Path(__file__).resolve()): sha(__file__)}
        verify(state['pins'])
        replay, preparation, review = read(REPLAY), read(PREPARATION), read(args.review_file)
        assert replay['status'] == 'source-proposals-awaiting-semantic-review'
        assert replay['profile_count'] == 20 and type(replay['profile_count']) is int
        assert type(replay['unique_authored_source_count']) is type(replay['unique_compiler_copy_count']) is int
        assert replay['unique_authored_source_count'] == 24 and replay['unique_compiler_copy_count'] == 25
        assert replay['unresolved'] == [] and replay['actual_common_mutex_held'] is True
        assert replay['canonical_LOC_admitted'] is replay['chart_admission'] is replay['producer_LOC_mutated'] is replay['count_override'] is False
        for key in ('actual_native_subjects', 'actual_signals_sent', 'full_performance_runs'):
            assert type(replay[key]) is int and replay[key] == 0
        assert replay['qualification'] == dict(path=str(PREPARATION), sha256=PREPARATION_SHA)
        assert type(replay['actual_fixture_methods_reused']) is int and replay['actual_fixture_methods_reused'] == 23
        assert replay['normalization_policy']['grammar'] == 'c3-source-owners-0.8.5-private-v5'
        prior = replay['preserved_prior_replay']
        assert type(prior['original_diagnostics']) is int and prior['original_diagnostics'] == 46
        assert type(prior['source_or_report_mutations']) is int and prior['source_or_report_mutations'] == 0
        assert prior['prior_units_not_overwritten'] is True
        assert review['status'] == 'exact24-source-owner-meaning-reviewed-awaiting-guarded-seal'
        assert review['language'] == 'c3' and review['review_identity']
        assert review['actual_source_replay'] == dict(path=str(REPLAY), sha256=args.replay_sha256)
        assert review['actual_qualified_preparation'] == replay['qualification']
        assert review['unresolved'] == [] and review['peer_review_pending'] is False
        assert review['source_only_peer_review']['status'] == 'accepted-all24-source-owners-all20-bundles-and-effective-totals'
        assert review['counts_transcribed_from_actual_replay'] is True
        assert review['canonical_LOC_admitted'] is review['chart_admission'] is False
        merge(state['pins'], replay['pins'])
        pins = state['pins']; verify(pins)
        terminal_path = REPLAY_ROOT / 'terminal-pins.json'
        completed = read(terminal_path)
        assert completed['status'] == 'completed-source-proposal-evidence-retained'
        assert completed['exclusive_mutex_held'] is True and completed['common_timing_mutex'] == str(LOCK)
        assert completed['canonical_LOC_admitted'] is completed['chart_admission'] is False
        assert completed['pins'][str(REPLAY)] == args.replay_sha256
        assert completed.get('available_bytes_not_fully_pinned', False) is False
        merge(pins, completed['pins']); pins[str(terminal_path)] = sha(terminal_path); verify(pins)
        assert replay['compiler_bound_input_inventory']['path'] == str(INVENTORY)
        assert sha(INVENTORY) == replay['compiler_bound_input_inventory']['sha256'] == pins[str(INVENTORY)]
        original = read(INVENTORY)
        assert original['status'] == 'compiler-input-proposals-awaiting-source-role-review'
        assert original['unknown_compiler_inputs'] == [] and original['complete_printed_application_inputs'] is True
        assert original['source_role_review_passed'] is original['canonical_LOC_admitted'] is original['chart_admission'] is False
        assert original['qualification'] == replay['qualification']
        assert all(type(original[key]) is int for key in ('profile_count', 'unique_authored_source_count', 'unique_compiler_copy_count'))
        assert original['profile_count'] == 20 and original['unique_authored_source_count'] == 24 and original['unique_compiler_copy_count'] == 25
        verify(original['pins'])
        # Revalidate only the retained compiler-input service, never the parser,
        # native workloads or fixtures. Exact path/cache identity is mandatory.
        assert sha(WORKER) == pins[str(WORKER)]
        name = 'c3_overlay_v5_original_compiler_inventory'
        assert name not in sys.modules
        sys.dont_write_bytecode = True
        spec = importlib.util.spec_from_file_location(name, WORKER)
        worker = importlib.util.module_from_spec(spec)
        sys.modules[name] = worker; spec.loader.exec_module(worker)
        assert Path(worker.__file__).resolve(strict=True) == WORKER
        verify({str(WORKER): pins[str(WORKER)]})
        # These four values are command-line prerequisites of the actual replay.
        # An imported worker has None defaults; bind only values already pinned
        # by its completed actual replay, never live or unreviewed replacements.
        assert worker.FIXTURE == Path(replay['fixture_receipt']['path'])
        worker.FIXTURE_SHA = replay['fixture_receipt']['sha256']
        worker.RAW_FIXTURE_SHA = pins[str(worker.FIXTURE_JOB / 'fixtures.log')]
        worker.FIXTURE_TERMINAL_SHA = pins[str(worker.FIXTURE_JOB / 'fixture-receipt-pin.json')]
        worker.PRIOR_REPLAY_TERMINAL_SHA = prior['terminal_sha256']
        assert all(re.fullmatch('[a-f0-9]{64}', value) for value in (
            worker.FIXTURE_SHA, worker.RAW_FIXTURE_SHA, worker.FIXTURE_TERMINAL_SHA,
            worker.PRIOR_REPLAY_TERMINAL_SHA))
        assert pins[str(worker.FIXTURE)] == worker.FIXTURE_SHA
        prior_path = worker.PRIOR_REPLAY_ROOT / 'source-count-proposals.json'
        prior_terminal_path = worker.PRIOR_REPLAY_ROOT / 'terminal-pins.json'
        assert prior['path'] == str(prior_path) and pins[str(prior_path)] == prior['sha256']
        assert pins[str(prior_terminal_path)] == worker.PRIOR_REPLAY_TERMINAL_SHA
        prior_terminal = read(prior_terminal_path)
        assert prior_terminal['status'] == 'completed-source-proposal-evidence-retained'
        assert prior_terminal['exclusive_mutex_held'] is True and prior_terminal['common_timing_mutex'] == str(LOCK)
        assert prior_terminal['canonical_LOC_admitted'] is prior_terminal['chart_admission'] is False
        assert prior_terminal.get('available_bytes_not_fully_pinned', False) is False
        merge(pins, prior_terminal['pins']); verify(pins)
        preserved = read(prior_path)
        assert preserved['status'] == 'source-proposals-needing-grammar-review'
        assert len(preserved['unresolved']) == prior['original_diagnostics']
        assert (preserved['profile_count'], preserved['unique_authored_source_count'],
            preserved['unique_compiler_copy_count']) == (20, 24, 25)
        assert preserved['canonical_LOC_admitted'] is preserved['chart_admission'] is False
        independent, authors, compiler_pins = worker.compiler_inventory()
        fixture, fixture_pins = worker.qualified_grammar()
        python = Path(sys.executable).resolve(strict=True)
        assert str(python) in fixture['pins_before'] and sha(python) == fixture['pins_before'][str(python)]
        assert independent == original['profiles']
        merge(pins, compiler_pins); merge(pins, fixture_pins); verify(pins)
        fixture_path = Path(replay['fixture_receipt']['path'])
        assert fixture_path == worker.FIXTURE and replay['fixture_receipt'] == dict(path=str(worker.FIXTURE), sha256=worker.FIXTURE_SHA, status='passed')
        counts, meanings = replay['source_counts'], review['sources']
        assert set(counts) == set(meanings) == {str(path) for path in authors} and len(counts) == 24
        assert set(counts) == set(preserved['source_counts'])
        assert all(item['source_sha256'] == preserved['source_counts'][source]['source_sha256']
            for source, item in counts.items()), 'New grammar must count exactly the preserved original source bytes'
        metrics = {source: source_review(meanings[source], source, counted) for source, counted in counts.items()}
        rows = {row['id']: row for row in preparation['profiles']}
        profiles, totals = replay['profiles'], review['profiles']
        assert len(rows) == len(preparation['profiles']) == len(profiles) == len(totals) == 20
        assert set(rows) == set(profiles) == set(totals) == set(independent) == IDS
        overlays, inventories, staged = {}, {}, {}
        all_copies, all_authors = set(), set()
        ownership = dict(status='passed', review_identity=review['review_identity'],
            evidence_path=str(args.review_file), evidence_sha256=args.review_sha256)
        for identity in sorted(IDS):
            row, counted, bound, total = rows[identity], profiles[identity], independent[identity], totals[identity]
            assert row['source'] == counted['source'] == bound['source']
            assert row['source_sha256'] == counted['source_sha256'] == bound['source_sha256']
            assert row['binary_sha256'] == counted['binary_sha256'] == bound['binary_sha256']
            assert row['benchmark'] == counted['benchmark'] == bound['benchmark'] == total['benchmark']
            assert row['variant'] == counted['variant'] == bound['variant'] == total['variant']
            assert row['loc'] is counted['producer_loc'] is bound['producer_loc'] is None
            assert counted['source_bundle'] and len(row['source_bundle']) == len(counted['source_bundle']) == len(bound['files'])
            bundle, expansions = [], []
            for index, (item, actual, input_file) in enumerate(zip(counted['source_bundle'], row['source_bundle'], bound['files'])):
                source, copy = input_file['path'], input_file['compiler_input_copy']
                metric = metrics[source]
                assert item == dict(input_file, physical_loc=metric['physical_loc'], normalized_loc=metric['normalized_loc'], diagnostics=[])
                assert actual['path'] == copy and actual['sha256'] == item['sha256'] == counts[source]['source_sha256']
                role = 'application' if index == 0 else 'application-helper'
                assert item['role'] == role and item['compiler_input_sha256'] == item['sha256']
                assert meanings[source]['role'] == role
                assert sha(source) == sha(copy) == item['sha256'] == pins[source] == pins[copy]
                if index == 0: assert copy == row['source'] and source == bound['original_authored_source']
                all_copies.add(copy); all_authors.add(source)
                bundle.append(dict(path=copy, sha256=item['sha256'], role=role,
                    physical_loc=metric['physical_loc'], normalized_loc=metric['normalized_loc'],
                    count_receipt=dict(path=str(REPLAY), sha256=args.replay_sha256, row_id=identity,
                        source_bundle_index=index, counted_source=source, source_sha256=item['sha256']),
                    ownership_review=dict(ownership, reviewed_authored_source=source), unresolved_diagnostics=[]))
                expansions.append(dict(source=copy, counted_source=source, source_sha256=item['sha256'],
                    actual_expansions=metric['expansions']))
            physical, canonical = sum(item['physical_loc'] for item in bundle), sum(item['normalized_loc'] for item in bundle)
            assert type(counted['physical_loc']) is type(counted['normalized_loc']) is int
            assert type(total['effective_physical_loc']) is type(total['effective_normalized_loc']) is int
            assert counted['physical_loc'] == physical == total['effective_physical_loc']
            assert counted['normalized_loc'] == canonical == total['effective_normalized_loc']
            assert total['complete_authored_source_bundle'] == row['compiled_source_bundle'] and total['unresolved'] == []
            path = output / 'compiler-inputs' / (identity + '.json')
            files = [dict(path=item['path'], sha256=item['sha256']) for item in row['source_bundle']]
            evidence = dict(status='qualified-exact-authored-compiler-inputs', profile=identity,
                source_sha256=row['source_sha256'], binary_sha256=row['binary_sha256'], files=files,
                independently_retained_compiler_input_evidence=bound,
                original_compiler_inventory=dict(path=str(INVENTORY), sha256=pins[str(INVENTORY)]),
                qualified_preparation=replay['qualification'], exact_source_role_review=dict(path=str(args.review_file), sha256=args.review_sha256),
                inventory_derived_from_original_actual_compile_receipt=True, authored_application_helpers_complete=True,
                vendor_standard_library_counted_as_source=False, observer_sources_counted_as_source=False)
            staged[identity] = (path, evidence)
            inventories[identity] = dict(status=evidence['status'], source_sha256=row['source_sha256'],
                binary_sha256=row['binary_sha256'], evidence_path=str(path), evidence_sha256=None, files=files)
            overlays[identity] = dict(benchmark=row['benchmark'], variant=row['variant'], source=row['source'],
                source_sha256=row['source_sha256'], producer_loc=None, physical_loc=physical, canonical_loc=canonical,
                source_bundle=bundle, expansions=expansions, semantic_review=ownership)
        assert all_authors == set(counts) and len(all_copies) == 25
        verify(pins)
        output.mkdir()
        for identity, (path, evidence) in staged.items():
            new(path, evidence); pins[str(path)] = sha(path)
            inventories[identity]['evidence_sha256'] = pins[str(path)]
        verify(pins)
        overlay = dict(schema_version=1, language='c3', status='qualified-exact-source-LOC', profile_count=20,
            complete_authored_source_bundles=True, unresolved=[], normalization_policy=replay['normalization_policy'],
            fixture_receipt=replay['fixture_receipt'], replay_receipts=[dict(path=str(REPLAY), sha256=args.replay_sha256)],
            grammar_snapshot_pins=replay['grammar_snapshot_pins'], compiler_bound_input_inventory=inventories,
            profiles=overlays, pins=pins, canonical_LOC_admitted=True, chart_admission=False,
            original_sources_or_timings_mutated=False)
        path = output / 'qualified-loc-overlay.json'; new(path, overlay)
        print(json.dumps(dict(overlay=str(path), sha256=sha(path), profiles=20, authored_inputs=24, compiler_copies=25)), flush=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--revision', required=True)
    parser.add_argument('--replay-sha256', required=True)
    parser.add_argument('--review-file', required=True, type=Path)
    parser.add_argument('--review-sha256', required=True)
    args = parser.parse_args()
    assert re.fullmatch('[a-z0-9][a-z0-9-]{0,63}', args.revision)
    assert all(re.fullmatch('[a-f0-9]{64}', digest) for digest in (args.replay_sha256, args.review_sha256))
    args.review_file = args.review_file.resolve(strict=True)
    assert args.review_file.is_relative_to(HERE)
    assemble(args)


if __name__ == '__main__':
    main()
