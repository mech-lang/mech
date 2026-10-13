"""Seal reviewed Clojure source units, with independently bound compiler inputs."""
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

HERE = Path(__file__).resolve().parent.parent
LOC = HERE.parents[1] / 'loc-normalization/20261012/clojure-grammar-v3'
LOCK = Path('/tmp/mech-clbg-timing.lock')
REPLAY = LOC / 'jobs/grammar-v3-source-replay-v1/source-proposals.json'
REPLAY_SHA = 'ef10308e6dad994658c626ca34e7c4f21d78e4e91afc92f2a7b58fbe7a6c4bd7'
PREPARATION = HERE / 'qualification/private-v2/report.json'
PREPARATION_SHA = '69bf7cdb025759e6d10cb8c1ae1c189839e253ced9aca50e4f9cc1e838d36d79'


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def new_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def merge(pins, values):
    for path, digest in values.items():
        assert path not in pins or pins[path] == digest, ('Conflicting evidence pin', path)
        pins[path] = digest


def verify(pins):
    for path, digest in pins.items():
        assert sha(path) == digest, ('Qualified source/evidence changed', path)


@contextmanager
def terminal(args):
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        held, named = os.fstat(lock.fileno()), LOCK.stat()
        assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino)
        root = HERE / 'source-counts' / args.revision
        journal = HERE / 'source-counts/overlay-seal-outcomes' / args.revision
        assert not journal.exists(), 'Exclusive terminal revision; no automatic retry'
        journal.mkdir(parents=True)
        new_json(journal / 'started.json', dict(status='started', common_timing_mutex=str(LOCK),
            actual_common_mutex_held=True, replay=dict(path=str(REPLAY), sha256=REPLAY_SHA),
            semantic_review=dict(path=str(args.review_file), sha256=args.review_sha256),
            chart_admission=False, original_sources_or_timings_mutated=False))
        result = dict(status='failed-before-output', actual_common_mutex_held=True,
            common_timing_mutex=str(LOCK), original_sources_or_timings_mutated=False, chart_admission=False)
        try:
            yield root
            result['status'] = 'completed'
        except BaseException as error:
            result.update(error=str(error) or type(error).__name__, error_type=type(error).__name__)
            (journal / 'actual-error.log').write_text(traceback.format_exc())
            if (root / 'qualified-loc-overlay.json').exists():
                result['status'] = 'failed-after-saved-overlay'
            raise
        finally:
            paths = [path for directory in (root, journal) if directory.exists()
                for path in directory.rglob('*') if path.is_file()]
            paths += [path for path in (Path(__file__).resolve(), REPLAY, args.review_file, PREPARATION) if path.is_file()]
            result['partial_output_overwritten'] = False
            available = {}
            try:
                for path in paths:
                    available[str(path)] = sha(path)
            except BaseException as error:
                result.update(status='failed-terminal-integrity', terminal_integrity_error=dict(
                    error=str(error) or type(error).__name__, error_type=type(error).__name__),
                    available_bytes_not_fully_pinned=True, actual_terminal_evidence_pins=available)
                new_json(journal / 'outcome.json', result)
                raise
            else:
                result.update(actual_terminal_evidence_pins=available, available_bytes_not_fully_pinned=False)
                new_json(journal / 'outcome.json', result)


def assemble(args):
    with terminal(args) as root:
        assert not root.exists(), 'Exclusive new overlay; never mutate a partial overlay'
        assert sha(REPLAY) == REPLAY_SHA and sha(PREPARATION) == PREPARATION_SHA
        assert sha(args.review_file) == args.review_sha256
        replay = json.loads(REPLAY.read_text())
        qualified = json.loads(PREPARATION.read_text())
        review = json.loads(args.review_file.read_text())
        assert replay['status'] == 'source-proposals-awaiting-semantic-review'
        assert type(replay['unique_source_count']) is type(replay['profile_count']) is int
        assert replay['unique_source_count'] == 21 and replay['profile_count'] == 20 and replay['unresolved'] == []
        assert replay['qualification'] == dict(path=str(PREPARATION), sha256=PREPARATION_SHA)
        assert replay['canonical_LOC_admitted'] is replay['chart_admission'] is False
        assert review['status'] == 'exact21-source-unit-meaning-reviewed-awaiting-guarded-seal'
        assert review['language'] == 'clojure' and review['review_identity'] and review['unresolved'] == []
        assert review['actual_source_replay'] == dict(path=str(REPLAY), sha256=REPLAY_SHA)
        assert review['actual_qualified_preparation'] == replay['qualification']
        assert review['peer_review_pending'] is False and review['counts_transcribed_from_actual_replay'] is True
        assert review['source_only_peer_review']['status'] == 'accepted-all20-main-metrics-helper-effective-totals-and-selected-owner-facts'
        assert review['canonical_LOC_admitted'] is review['chart_admission'] is False
        pins = dict(replay['pins'])
        merge(pins, {str(REPLAY): REPLAY_SHA, str(PREPARATION): PREPARATION_SHA,
            str(args.review_file): args.review_sha256, str(Path(__file__).resolve()): sha(__file__)})
        verify(pins)
        # Reuse the actual frozen replay producer's independent compiler package
        # validator, not this overlay's source_bundle claims or regenerated code.
        worker = LOC / 'qualify_replay.py'
        assert sha(worker) == pins[str(worker)]
        sys.dont_write_bytecode = True
        name = 'clojure_overlay_exact_compiler_inventory'
        assert name not in sys.modules
        spec = importlib.util.spec_from_file_location(name, worker)
        owner = importlib.util.module_from_spec(spec)
        sys.modules[name] = owner
        spec.loader.exec_module(owner)
        assert Path(owner.__file__).resolve() == worker
        verify({str(worker): pins[str(worker)]})
        declared, authors, independent, compiler_pins = owner.source_inventory(PREPARATION, PREPARATION_SHA)
        assert independent == replay['compiler_input_evidence']
        merge(pins, compiler_pins)
        fixture_path = Path(replay['fixture'])
        fixture = json.loads(fixture_path.read_text())
        assert sha(fixture_path) == pins[str(fixture_path)]
        owner.validate_fixture(fixture)
        assert type(fixture['tests_run']) is type(fixture['discovered_tests']) is int
        assert fixture['tests_run'] == fixture['discovered_tests'] == 32
        assert len(fixture['discovered_test_ids']) == len(set(fixture['discovered_test_ids'])) == 32
        assert sorted(fixture['discovered_test_ids']) == owner.EXPECTED_IDS
        grammar_pins = fixture['frozen_dependency_pins']
        assert grammar_pins
        merge(pins, grammar_pins)
        rows = {row['id']: row for row in qualified['profiles']}
        counts = {row['id']: row for row in replay['profiles']}
        semantics = {row['id']: row for row in review['profiles']}
        assert len(rows) == len(qualified['profiles']) == len(counts) == len(replay['profiles']) == len(semantics) == len(review['profiles']) == 20
        assert set(rows) == set(counts) == set(semantics) == {row['id'] for row in declared}
        assert len(authors) == len(set(authors)) == 21
        inventories, profiles, evidence_files = {}, {}, {}
        all_counted = set()
        for identity in sorted(rows):
            row, counted, semantic, bound = rows[identity], counts[identity], semantics[identity], independent[identity]
            assert row['benchmark'] == counted['benchmark'] and row['variant'] == counted['variant']
            assert counted['producer_loc'] is None and row['loc'] is None
            assert semantic['source_sha256'] == row['source_sha256'] and semantic['unresolved'] == []
            assert semantic['owner_review'] and semantic['status'] == 'source-owner-meaning-reviewed-awaiting-guarded-seal'
            assert len(counted['source_bundle']) == len(row['source_bundle']) == len(bound['files']) == 2
            assert semantic['source'] == counted['source_bundle'][0]['source']
            assert semantic['complete_authored_source_bundle'] == row['compiled_source_bundle']
            bundle, expansions = [], []
            for index, (metric, compiled, input_file) in enumerate(zip(counted['source_bundle'], row['source_bundle'], bound['files'])):
                authored = str(HERE / row['compiled_source_bundle'][index])
                all_counted.add(authored)
                assert metric['source'] == input_file['authored_source'] == authored
                assert metric['source_sha256'] == compiled['sha256'] == input_file['sha256']
                assert compiled['path'] == input_file['actual_compiled_copy']
                assert metric['proposal_only'] is True and metric['diagnostics'] == []
                assert type(metric['physical_loc']) is type(metric['normalized_loc']) is int
                assert metric['physical_loc'] > 0 and metric['normalized_loc'] > 0
                assert compiled['role'] == input_file['role'] == ('application' if index == 0 else 'application-helper')
                meaning = semantic if index == 0 else review['shared_helper']
                assert meaning['source'] == authored and meaning['source_sha256'] == metric['source_sha256']
                assert meaning['physical_loc'] == metric['physical_loc'] and meaning['normalized_loc'] == metric['normalized_loc']
                assert meaning['owner_review'] and meaning['unresolved'] == []
                ownership = dict(status='passed', review_identity=review['review_identity'],
                    evidence_path=str(args.review_file), evidence_sha256=args.review_sha256,
                    reviewed_authored_source=authored)
                bundle.append(dict(path=compiled['path'], sha256=compiled['sha256'], role=compiled['role'],
                    physical_loc=metric['physical_loc'], normalized_loc=metric['normalized_loc'],
                    count_receipt=dict(path=str(REPLAY), sha256=REPLAY_SHA, row_id=identity,
                        source_bundle_index=index, counted_source=authored, source_sha256=metric['source_sha256']),
                    ownership_review=ownership, unresolved_diagnostics=[]))
                expansions.append(dict(source=compiled['path'], counted_source=authored,
                    source_sha256=metric['source_sha256'], actual_expansions=metric['expansions']))
            physical, canonical = sum(item['physical_loc'] for item in bundle), sum(item['normalized_loc'] for item in bundle)
            assert type(counted['physical_loc']) is type(counted['normalized_loc']) is int
            assert counted['physical_loc'] == physical == semantic['effective_physical_loc']
            assert counted['normalized_loc'] == canonical == semantic['effective_normalized_loc']
            inventory_path = root / 'compiler-inputs' / (identity + '.json')
            files = [dict(path=item['path'], sha256=item['sha256']) for item in row['source_bundle']]
            evidence = dict(status='qualified-exact-authored-compiler-inputs', profile=identity,
                source_sha256=row['source_sha256'], binary_sha256=row['binary_sha256'], files=files,
                independently_retained_compiler_input_evidence=bound,
                original_compiler_inventory=dict(path=row['compiled_input_inventory'], sha256=row['compiled_input_inventory_sha256']),
                qualified_preparation=dict(path=str(PREPARATION), sha256=PREPARATION_SHA),
                exact_source_role_review=dict(path=str(args.review_file), sha256=args.review_sha256),
                inventory_derived_from_original_actual_compile_receipt=True,
                authored_application_helpers_complete=True, generated_class_files_counted_as_source=False)
            evidence_files[identity] = (inventory_path, evidence)
            inventories[identity] = dict(status=evidence['status'], source_sha256=row['source_sha256'],
                binary_sha256=row['binary_sha256'], evidence_path=str(inventory_path), evidence_sha256=None, files=files)
            profiles[identity] = dict(benchmark=row['benchmark'], variant=row['variant'],
                source=row['source'], source_sha256=row['source_sha256'], producer_loc=None,
                physical_loc=physical, canonical_loc=canonical, source_bundle=bundle, expansions=expansions,
                semantic_review=dict(status='passed', review_identity=review['review_identity'],
                    evidence_path=str(args.review_file), evidence_sha256=args.review_sha256))
        assert all_counted == {str(path) for path in authors}
        verify(pins)
        root.mkdir(parents=True)
        for identity, (path, evidence) in evidence_files.items():
            new_json(path, evidence)
            pins[str(path)] = sha(path)
            inventories[identity]['evidence_sha256'] = pins[str(path)]
        verify(pins)
        overlay = dict(schema_version=1, language='clojure', status='qualified-exact-source-LOC',
            profile_count=20, complete_authored_source_bundles=True, unresolved=[],
            normalization_policy=dict(rule_version='canonical-source-units-clojure-grammar-v3-20261012',
                evidence_path=str(fixture_path), evidence_sha256=sha(fixture_path)),
            fixture_receipt=dict(path=str(fixture_path), sha256=sha(fixture_path), status='passed'),
            replay_receipts=[dict(path=str(REPLAY), sha256=REPLAY_SHA)], grammar_snapshot_pins=grammar_pins,
            compiler_bound_input_inventory=inventories, profiles=profiles, pins=pins,
            canonical_LOC_admitted=True, chart_admission=False, original_sources_or_timings_mutated=False)
        output = root / 'qualified-loc-overlay.json'
        new_json(output, overlay)
        print(json.dumps(dict(overlay=str(output), sha256=sha(output), profiles=20)), flush=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--review-file', type=Path, required=True)
    parser.add_argument('--review-sha256', required=True)
    parser.add_argument('--revision', required=True)
    args = parser.parse_args()
    assert re.fullmatch(r'[a-z0-9][a-z0-9-]{0,63}', args.revision)
    assert re.fullmatch(r'[0-9a-f]{64}', args.review_sha256)
    args.review_file = args.review_file.resolve(strict=True)
    assert args.review_file.is_relative_to(HERE / 'source-counts')
    print('WAIT common mutex: separately qualified Clojure LOC21 overlay', flush=True)
    assemble(args)


if __name__ == '__main__':
    main()
