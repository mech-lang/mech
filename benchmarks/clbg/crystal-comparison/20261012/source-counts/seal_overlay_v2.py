"""Unexecuted additive exact LOC overlay sealer for the repaired Crystal replay.

Requires a separately reviewed semantic packet. All verification/hashing and
exclusive evidence writes occur inside the common native timing mutex.
"""
import argparse
from contextlib import contextmanager
import fcntl
import hashlib
import json
from pathlib import Path
import re
import traceback

HERE = Path(__file__).resolve().parent.parent
LOCK = Path('/tmp/mech-clbg-timing.lock')
INDEX_SHA = 'f8b3f9f20265245461020f112bf0c7b2bd880c702f726c246ccf68a480147476'
REPLAY_SHA = 'b5d094c0404de992475e406436b63be1673675e8dfbfb940b138c15c8346f1c1'
OLD_FIXTURE_SHA = 'b00afe351c40fb38e03a99ec81fe3d71bd69a82726f337aa87dfb40cfcaa6b6a'
BRACE = HERE.parents[1] / 'loc-normalization/20261012/crystal-brace-repair-v2'
REPLAY = BRACE / 'jobs/fixtures-replay-v1/source-proposals.json'
EXPECTED_METHODS = [
    'test_mask_exact_positions_bom_crlf_comments_and_literals',
    'test_escaped_quotes_characters_do_not_open_scopes',
    'test_floor_division_is_not_comment_and_slash_division_not_regex',
    'test_regex_literals_have_their_own_prefix_class_and_escape_owners',
    'test_simple_interpolation_and_nested_scalar_literal_remain_composed',
    'test_interpolation_independent_work_and_control_are_gated',
    'test_unclosed_literal_and_interpolation_are_gated',
    'test_percent_heredoc_and_annotations_are_retained_and_gated',
    'test_constructor_instance_formals_protected_no_generated_assignments',
    'test_written_properties_independent_names_no_generated_accessors',
    'test_property_generic_commas_and_initializer_chains_distinct',
    'test_wrapped_method_formals_defaults_and_return_type_one_header',
    'test_setter_name_equals_not_body_assignment',
    'test_imperative_method_separate_body_statements_packed_expanded',
    'test_tuple_swap_single_producer_and_chained_bindings_expand',
    'test_braced_callback_explicit_header_body_written_close',
    'test_do_callback_explicit_header_body_written_close',
    'test_destructured_callback_formals_structural_not_bindings',
    'test_nested_callback_resets_outer_argument_data_ownership',
    'test_real_following_callback_composition_requires_review',
    'test_hash_tuple_and_named_arguments_are_data_not_blocks',
    'test_standalone_tuple_after_written_end_has_no_callback_owner',
    'test_same_line_zero_formal_callback_remains_executable',
    'test_tuple_repair_cannot_hide_packed_callback_assignments',
    'test_literal_and_constructor_wrapping_preserved',
    'test_controls_comparison_and_condition_assignment_stay_header',
    'test_member_end_is_not_keyword_close',
    'test_if_elsif_else_packed_and_expanded',
    'test_postfix_controls_header_plus_written_action',
    'test_case_imperative_raise_not_collapsed_as_value_expression',
    'test_begin_open_zero_preserves_mixed_operand_work_and_written_end',
    'test_expression_lambda_without_exec_block_and_explicit_lambda_differ',
    'test_ffi_prototypes_default_formals_one_each_lib_close_written',
    'test_macro_body_and_delimiters_are_not_hidden',
    'test_unknown_or_unclosed_owners_require_review',
    'test_other_languages_are_not_aliased',
]
EXPECTED_IDS = sorted('test_foreign_crystal.CrystalProposalFixtures.' + name for name in EXPECTED_METHODS)


def qualified_repair(replay, fixture):
    assert replay['prior_proposal']['sha256'] == REPLAY_SHA
    assert fixture['status'] == 'passed' and fixture['successful'] is True
    assert fixture['tests_run'] == fixture['discovered_tests'] == 36
    assert fixture['expected_test_ids'] == EXPECTED_IDS
    assert len(fixture['actual_test_ids']) == len(set(fixture['actual_test_ids'])) == 36
    assert sorted(fixture['actual_test_ids']) == EXPECTED_IDS
    for name in ('failures', 'errors', 'skipped', 'expected_failures', 'unexpected_successes'):
        assert fixture[name] == []
    assert fixture['actual_native_subjects'] == fixture['actual_signals_sent'] == 0
    assert fixture['exclusive_mutex_held'] is True and fixture['common_timing_mutex'] == str(LOCK)
    assert fixture['original_qualified_fixture']['sha256'] == OLD_FIXTURE_SHA
    assert fixture['original_qualified_fixture']['tests_run'] == 672
    comparisons = replay['owner_comparison']
    assert len(comparisons) == len({item['id'] for item in comparisons}) == 20
    profiles = {row['id']: row for row in replay['profiles']}
    assert set(profiles) == {item['id'] for item in comparisons}
    for item in comparisons:
        assert item['source_sha256'] == profiles[item['id']]['source_sha256']
        assert item['actual_normalized_loc'] == profiles[item['id']]['normalized_loc']
        repaired = item['id'] == 'crystal-fannkuchredux-optimized'
        assert item['exact_scoped_owner_change'] is True and item['added_callback_events'] == []
        assert item['previous_normalized_loc'] - item['actual_normalized_loc'] == (2 if repaired else 0)
        assert item['removed_callback_events'] == ([['close', 1328, 1329], ['header', 1310, 1311]] if repaired else [])


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


@contextmanager
def held_terminal(args):
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        journal = HERE / 'source-counts/overlay-seal-outcomes' / args.revision
        assert not journal.exists(), 'Exclusive actual terminal journal, no overwrite'
        journal.mkdir(parents=True)
        root = HERE / 'source-counts' / args.revision
        new_json(journal / 'started.json', dict(status='started', revision=args.revision,
            actual_common_mutex_held=True, common_timing_mutex=str(LOCK),
            reviewed_replay=dict(path=str(args.replay_file), sha256=args.replay_sha256),
            reviewed_source_semantics=dict(path=str(args.review_file), sha256=args.review_sha256),
            original_source_report_timings_mutated=False, chart_admission=False))
        result = dict(status='failed-before-output-completion', actual_common_mutex_held=True,
            common_timing_mutex=str(LOCK), original_source_report_timings_mutated=False, chart_admission=False)
        try:
            yield
            result['status'] = 'completed'
        except BaseException as error:
            result.update(error=str(error) or type(error).__name__, error_type=type(error).__name__)
            (journal / 'actual-error.log').write_text(traceback.format_exc())
            if (root / 'qualified-loc-overlay.json').exists():
                result['status'] = 'failed-after-saved-output'
            raise
        finally:
            paths = [path for directory in (root, journal) if directory.exists()
                for path in directory.rglob('*') if path.is_file()]
            paths.extend(path for path in (Path(__file__).resolve(), args.replay_file, args.review_file,
                args.replay_file.with_name('fixture-receipt.json')) if path.is_file())
            result['actual_terminal_evidence_pins'] = {str(path): sha(path) for path in paths}
            result['existing_partial_overlay_mutated_by_journal'] = False
            new_json(journal / 'outcome.json', result)


def assemble(args):
    with held_terminal(args):
        root = HERE / 'source-counts' / args.revision
        assert not root.exists(), 'Exclusive fresh overlay revision; no overwrite or retry'
        index_path = HERE / 'studies/full-v1/index.json'
        replay_path = args.replay_file
        assert replay_path == REPLAY.resolve(strict=True)
        assert sha(index_path) == INDEX_SHA and sha(replay_path) == args.replay_sha256
        assert sha(args.review_file) == args.review_sha256
        index = json.loads(index_path.read_text())
        replay = json.loads(replay_path.read_text())
        review = json.loads(args.review_file.read_text())
        assert index['status'] == 'sealed-awaiting-root-review'
        assert replay['status'] == 'source-proposals-awaiting-semantic-review' and replay['source_count'] == 20
        assert review['status'] == 'passed-exact-source-semantic-LOC-review'
        assert review['language'] == 'crystal' and review['review_identity']
        assert review['replay_sha256'] == args.replay_sha256 and review['unresolved'] == []
        assert review['complete_authored_source_bundles'] is True
        pins = dict(replay['pins'])
        add_pins(pins, {str(index_path): INDEX_SHA, str(replay_path): args.replay_sha256,
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
        assert sha(fixture_path) == replay['pins'][str(fixture_path)]
        assert fixture_path == replay_path.with_name('fixture-receipt.json')
        qualified_repair(replay, fixture)
        assert 'test_foreign_crystal' in fixture['suites']
        grammar_pins = dict(fixture.get('frozen_dependency_pins', fixture.get('frozen_module_pins', {})))
        assert grammar_pins, 'Qualified immutable grammar inventory is required'
        add_pins(pins, grammar_pins)
        inventories, profiles, inventory_evidence = {}, {}, {}
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
            inventory_evidence[identity] = (inventory_path, evidence)
            inventories[identity] = dict(status=evidence['status'], source_sha256=row['source_sha256'],
                binary_sha256=row['binary_sha256'], evidence_path=str(inventory_path),
                evidence_sha256=None, files=files)
            ownership = dict(status='passed', review_identity=semantic['review_identity'],
                evidence_path=str(args.review_file), evidence_sha256=args.review_sha256)
            receipt = dict(path=str(replay_path), sha256=args.replay_sha256, row_id=identity,
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
        # No output directory or evidence file is created until all twenty
        # actual compiler inventories and semantic rows have passed together.
        root.mkdir(parents=True)
        for identity, (inventory_path, evidence) in inventory_evidence.items():
            new_json(inventory_path, evidence)
            pins[str(inventory_path)] = sha(inventory_path)
            inventories[identity]['evidence_sha256'] = pins[str(inventory_path)]
        verify(pins)
        overlay = dict(schema_version=1, language='crystal', status='qualified-exact-source-LOC',
            profile_count=20, complete_authored_source_bundles=True, unresolved=[],
            normalization_policy=dict(rule_version='canonical-source-units-20261012-approved-rule-v2',
                evidence_path=str(fixture_path), evidence_sha256=sha(fixture_path)),
            fixture_receipt=dict(path=str(fixture_path), sha256=sha(fixture_path), status='passed'),
            replay_receipts=[dict(path=str(replay_path), sha256=args.replay_sha256)],
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
    parser.add_argument('--replay-file', type=Path, required=True)
    parser.add_argument('--replay-sha256', required=True)
    parser.add_argument('--revision', required=True)
    args = parser.parse_args()
    assert re.fullmatch(r'[a-z0-9][a-z0-9-]{0,63}', args.revision)
    args.review_file = args.review_file.resolve(strict=True)
    args.replay_file = args.replay_file.resolve(strict=True)
    assert re.fullmatch(r'[0-9a-f]{64}', args.review_sha256)
    assert re.fullmatch(r'[0-9a-f]{64}', args.replay_sha256)
    assert args.review_file.is_relative_to(HERE / 'source-counts')
    print('WAIT common mutex: Crystal separately qualified LOC overlay', flush=True)
    assemble(args)


if __name__ == '__main__':
    main()
