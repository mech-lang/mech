"""Source-only exact LOC overlay sealer; no count replay or timing mutation.

Requires a separately reviewed semantic packet. All verification/hashing and
exclusive evidence writes occur inside the common native timing mutex.
"""
import argparse
import fcntl
import hashlib
import json
from pathlib import Path
import re

HERE = Path(__file__).resolve().parent.parent
LOCK = Path('/tmp/mech-clbg-timing.lock')
INDEX_SHA = 'f8b3f9f20265245461020f112bf0c7b2bd880c702f726c246ccf68a480147476'
REPLAY_SHA = 'b5d094c0404de992475e406436b63be1673675e8dfbfb940b138c15c8346f1c1'


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def new_json(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def add_pins(target, values):
    for path, digest in values.items():
        assert path not in target or target[path] == digest, ('Conflicting evidence pin', path)
        target[path] = digest


def verify(pins):
    for path, digest in pins.items():
        assert sha(path) == digest, ('Frozen source/evidence changed', path)


def assemble(args):
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        root = HERE / 'source-counts' / args.revision
        assert not root.exists(), 'Exclusive fresh overlay revision; no overwrite or retry'
        index_path = HERE / 'studies/full-v1/index.json'
        replay_path = HERE / 'source-counts/boundary-audit-18-source-proposals-v1.json'
        assert sha(index_path) == INDEX_SHA and sha(replay_path) == REPLAY_SHA
        assert sha(args.review_file) == args.review_sha256
        index = json.loads(index_path.read_text())
        replay = json.loads(replay_path.read_text())
        review = json.loads(args.review_file.read_text())
        assert index['status'] == 'sealed-awaiting-root-review'
        assert replay['status'] == 'source-proposals-awaiting-semantic-review' and replay['source_count'] == 20
        assert review['status'] == 'passed-exact-source-semantic-LOC-review'
        assert review['language'] == 'crystal' and review['review_identity']
        assert review['replay_sha256'] == REPLAY_SHA and review['unresolved'] == []
        assert review['complete_authored_source_bundles'] is True
        pins = dict(replay['pins'])
        add_pins(pins, {str(index_path): INDEX_SHA, str(replay_path): REPLAY_SHA,
            str(args.review_file): args.review_sha256, str(Path(__file__).resolve()): sha(__file__)})
        verify(pins)
        report_path = Path(index['qualified_preparation'])
        assert sha(report_path) == index['qualified_preparation_sha256']
        preparation = json.loads(report_path.read_text())
        assert preparation['status'] == 'bounded-output-ownership-work-passed-awaiting-LOC-and-full-review'
        assert preparation['pins_unchanged_at_end'] is True
        prior_path = Path(preparation['exact_prior_proof'])
        assert sha(prior_path) == preparation['exact_prior_proof_sha256']
        prior = json.loads(prior_path.read_text())
        assert prior['status'] == 'verified-exact-old-bounded-proof'
        add_pins(pins, preparation['pins'])
        add_pins(pins, {str(report_path): sha(report_path), str(prior_path): sha(prior_path)})
        rows = {row['id']: row for row in preparation['profiles']}
        counts = {row['id']: row for row in replay['profiles']}
        assert len(rows) == len(counts) == 20 and set(rows) == set(counts) == set(review['profiles'])
        measured = {}
        for benchmark, entry in index['benchmarks'].items():
            path = Path(entry['manifest'])
            assert sha(path) == entry['sha256']
            manifest = json.loads(path.read_text())
            add_pins(pins, manifest['pins'])
            pins[str(path)] = entry['sha256']
            for row in manifest['profiles']:
                if row['language'] == 'crystal':
                    assert row['id'] not in measured and row['benchmark'] == benchmark
                    measured[row['id']] = row
        assert set(measured) == set(rows)
        fixture_path = Path(replay['fixture'])
        fixture = json.loads(fixture_path.read_text())
        assert fixture['status'] == 'passed' and fixture['failures'] == fixture['errors'] == 0
        assert fixture['exclusive_mutex_held'] is True and fixture['common_timing_mutex'] == str(LOCK)
        assert 'test_foreign_crystal' in fixture['suites']
        grammar_pins = dict(fixture.get('frozen_dependency_pins', fixture.get('frozen_module_pins', {})))
        assert grammar_pins, 'Qualified immutable grammar inventory is required'
        add_pins(pins, grammar_pins)
        root.mkdir(parents=True)
        inventories, profiles = {}, {}
        for identity in sorted(rows):
            row, counted, semantic = measured[identity], counts[identity], review['profiles'][identity]
            assert (row['source'], row['source_sha256'], row['binary_sha256']) == (
                rows[identity]['source'], rows[identity]['source_sha256'], rows[identity]['binary_sha256'])
            assert row['benchmark'] == counted['benchmark'] and row['variant'] == counted['variant']
            assert counted['diagnostics'] == [] and counted['source_sha256'] == row['source_sha256']
            assert counted['proposal_only'] is True and counted['review_status'] == 'candidate-parser-pass'
            assert type(counted['normalized_loc']) is int and counted['normalized_loc'] > 0
            assert type(counted['physical_loc']) is int and counted['physical_loc'] > 0
            assert semantic['status'] == 'passed' and semantic['review_identity']
            assert semantic['source_sha256'] == row['source_sha256']
            assert semantic['counted_source'] == counted['source']
            assert semantic['physical_loc'] == counted['physical_loc']
            assert semantic['canonical_loc'] == counted['normalized_loc']
            assert semantic['unresolved'] == [] and semantic['ownership_disposition']
            assert semantic['compiled_input_roles'] == ['application']
            original, source, binary = map(Path, (counted['source'], row['source'], row['binary']))
            assert sha(original) == sha(source) == row['source_sha256'] and sha(binary) == row['binary_sha256']
            # The compiled command is an independently retained producer receipt,
            # not reconstructed from this overlay's own source_bundle claim.
            build_revision = 'private-layout-v2' if identity in prior['profiles'] else 'private-input-archive-filter-v6'
            build_path = HERE / 'qualification' / build_revision / 'builds' / identity / 'build.json'
            build = json.loads(build_path.read_text())
            assert build['exit_status'] == build['final_exit_status'] == 0
            assert build['inherited_common_mutex'] is True and 'error' not in build and 'cleanup_error' not in build
            command = build['command']
            assert command[-1] == str(source) and command[command.index('-o') + 1] == str(binary)
            assert [arg for arg in command if arg.endswith('.cr')] == [str(source)]
            if identity in prior['profiles']:
                assert command == prior['profiles'][identity]['original_build_command']
            # Only two Pi mains have a require, selecting the ordinary pinned
            # vendor big stdlib. Any local require/macro/foreign support input
            # invalidates the independently reviewed single-application role.
            requires = re.findall(r'^\s*require\s+"([^"]+)"', source.read_text(), re.MULTILINE)
            assert requires == (['big'] if row['benchmark'] == 'pidigits' else [])
            assert not re.search(r'^\s*(macro|lib)\s', source.read_text(), re.MULTILINE)
            add_pins(pins, {str(build_path): sha(build_path), str(source): row['source_sha256'],
                           str(binary): row['binary_sha256']})
            files = [dict(path=str(source), sha256=row['source_sha256'])]
            inventory_path = root / 'compiler-inputs' / (identity + '.json')
            evidence = dict(status='qualified-exact-authored-compiler-inputs', profile=identity,
                source_sha256=row['source_sha256'], binary_sha256=row['binary_sha256'],
                files=files, actual_build_receipt=dict(path=str(build_path), sha256=sha(build_path)),
                actual_build_command=command, qualified_preparation=dict(path=str(report_path), sha256=sha(report_path)),
                source_role_review=dict(path=str(args.review_file), sha256=args.review_sha256,
                    review_identity=semantic['review_identity'], disposition=semantic['ownership_disposition']),
                counted_source_alias=str(original), alias_bytes_equal=True,
                vendor_inputs='Pinned Crystal standard library/compiler/native dependencies; no local authored helper is compiled',
                source_inventory_is_derived_from_actual_compiler_receipt=True)
            new_json(inventory_path, evidence)
            pins[str(inventory_path)] = sha(inventory_path)
            inventories[identity] = dict(status=evidence['status'], source_sha256=row['source_sha256'],
                binary_sha256=row['binary_sha256'], evidence_path=str(inventory_path),
                evidence_sha256=pins[str(inventory_path)], files=files)
            ownership = dict(status='passed', review_identity=semantic['review_identity'],
                evidence_path=str(args.review_file), evidence_sha256=args.review_sha256)
            receipt = dict(path=str(replay_path), sha256=REPLAY_SHA, row_id=identity,
                counted_source=str(original), source_sha256=row['source_sha256'])
            profiles[identity] = dict(benchmark=row['benchmark'], variant=row['variant'],
                source=str(source), source_sha256=row['source_sha256'], producer_loc=counted['producer_loc'],
                physical_loc=counted['physical_loc'], canonical_loc=counted['normalized_loc'],
                expansions=counted['expansions'], semantic_review=ownership,
                source_bundle=[dict(path=str(source), sha256=row['source_sha256'], role='application',
                    physical_loc=counted['physical_loc'], normalized_loc=counted['normalized_loc'],
                    count_receipt=receipt, ownership_review=ownership, unresolved_diagnostics=[])])
        assert len(profiles) == len(inventories) == 20
        verify(pins)
        overlay = dict(schema_version=1, language='crystal', status='qualified-exact-source-LOC',
            profile_count=20, complete_authored_source_bundles=True, unresolved=[],
            normalization_policy=dict(rule_version='canonical-source-units-20261012-approved-rule-v2',
                evidence_path=str(fixture_path), evidence_sha256=sha(fixture_path)),
            fixture_receipt=dict(path=str(fixture_path), sha256=sha(fixture_path), status='passed'),
            replay_receipts=[dict(path=str(replay_path), sha256=REPLAY_SHA)],
            grammar_snapshot_pins=grammar_pins, compiler_bound_input_inventory=inventories,
            profiles=profiles, pins=pins, canonical_LOC_admitted=True, chart_admission=False,
            original_source_report_timings_mutated=False)
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
    args.review_file = args.review_file.resolve(strict=True)
    assert args.review_file.is_relative_to(HERE / 'source-counts')
    print('WAIT common mutex: Crystal separately qualified LOC overlay', flush=True)
    assemble(args)


if __name__ == '__main__':
    main()
