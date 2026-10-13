"""Unexecuted additive Crystal grammar fixture/source replay candidate.

One process holds the common mutex; it executes no native subject or child.
It freezes the candidate and imports unchanged qualified LOC18 dependencies.
"""
import argparse
import ast
from datetime import datetime, timezone
import fcntl
import hashlib
import importlib
import json
from pathlib import Path
import shutil
import sys
import unittest

HERE = Path(__file__).resolve().parent
LOC = HERE.parent
CLBG = LOC.parents[1]
CRYSTAL = CLBG / 'crystal-comparison/20261012'
LOCK = Path('/tmp/mech-clbg-timing.lock')
OLD_FIXTURE = LOC / 'jobs/boundary-audit-18/fixture-receipt.json'
OLD_FIXTURE_SHA = 'b00afe351c40fb38e03a99ec81fe3d71bd69a82726f337aa87dfb40cfcaa6b6a'
OLD_REPLAY = CRYSTAL / 'source-counts/boundary-audit-18-source-proposals-v1.json'
OLD_REPLAY_SHA = 'b5d094c0404de992475e406436b63be1673675e8dfbfb940b138c15c8346f1c1'
REPAIRED_ID = 'crystal-fannkuchredux-optimized'
REMOVED_CALLBACK_SPANS = {('header', 1310, 1311), ('close', 1328, 1329)}
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


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def new_json(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def verify(pins):
    for path, digest in pins.items():
        assert sha(path) == digest, ('Exact source/dependency changed', path)


def now():
    return datetime.now(timezone.utc).isoformat()


def test_inventory(path):
    tree = ast.parse(path.read_text())
    classes = [node for node in tree.body if isinstance(node, ast.ClassDef)]
    assert [node.name for node in classes] == ['CrystalProposalFixtures']
    methods = [node.name for node in classes[0].body if isinstance(node, ast.FunctionDef) and node.name.startswith('test_')]
    assert len(methods) == len(set(methods))
    return sorted('test_foreign_crystal.CrystalProposalFixtures.' + name for name in methods)


class RecordedResult(unittest.TextTestResult):
    def __init__(self, *args):
        super().__init__(*args)
        self.actual_test_ids = []
    def startTest(self, test):
        self.actual_test_ids.append(test.id())
        super().startTest(test)


def canonical_events(metric):
    return [event for expansion in metric['expansions'] for event in expansion.get('canonical_events', [])]


def run(revision):
    print('WAIT common mutex: additive Crystal tuple-owner fixtures and exact20 replay', flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        assert sha(OLD_FIXTURE) == OLD_FIXTURE_SHA and sha(OLD_REPLAY) == OLD_REPLAY_SHA
        old_fixture = json.loads(OLD_FIXTURE.read_text())
        old_replay = json.loads(OLD_REPLAY.read_text())
        assert old_fixture['status'] == 'passed' and old_fixture['tests_run'] == 672
        assert old_fixture['failures'] == old_fixture['errors'] == 0
        assert old_fixture['exclusive_mutex_held'] is True and old_fixture['common_timing_mutex'] == str(LOCK)
        assert 'test_foreign_crystal' in old_fixture['suites']
        assert sys.version.split()[0] == old_fixture['python_version'].split()[0]
        assert old_replay['status'] == 'source-proposals-awaiting-semantic-review' and old_replay['source_count'] == 20
        dependencies = dict(old_fixture.get('frozen_dependency_pins', old_fixture.get('frozen_module_pins', {})))
        assert dependencies
        verify(dependencies)
        verify(old_replay['pins'])
        pygments = Path(old_fixture['pygments_module'])
        assert sha(pygments) == old_fixture['pygments_module_sha256']
        output = HERE / 'jobs' / revision
        assert not output.exists(), 'Exclusive additive revision; no old evidence is replaced'
        snapshot = output / 'snapshot'
        snapshot.mkdir(parents=True)
        pins = dict(dependencies)
        pins.update({str(OLD_FIXTURE): OLD_FIXTURE_SHA, str(OLD_REPLAY): OLD_REPLAY_SHA,
                     str(pygments): sha(pygments)})
        for name in ('foreign_crystal.py', 'test_foreign_crystal.py', 'qualify_replay.py', 'PLAN.md'):
            source, target = HERE / name, snapshot / name
            shutil.copyfile(source, target)
            assert sha(target) == sha(source)
            pins[str(target)] = sha(target)
            pins[str(source)] = sha(source)
            target.chmod(0o444)
        old_snapshot = OLD_FIXTURE.parent / 'snapshot'
        expected = EXPECTED_IDS
        assert len(expected) == len(set(expected)) == 36
        assert test_inventory(snapshot / 'test_foreign_crystal.py') == expected
        previous = test_inventory(old_snapshot / 'test_foreign_crystal.py')
        additions = set(expected) - set(previous)
        assert set(previous) <= set(expected) and additions == {
            'test_foreign_crystal.CrystalProposalFixtures.test_standalone_tuple_after_written_end_has_no_callback_owner',
            'test_foreign_crystal.CrystalProposalFixtures.test_same_line_zero_formal_callback_remains_executable',
            'test_foreign_crystal.CrystalProposalFixtures.test_tuple_repair_cannot_hide_packed_callback_assignments'}
        # Imports use the new immutable front end/test files, the exact old
        # dependency snapshot, and the old pinned lexical dependency package.
        assert 'foreign_crystal' not in sys.modules and 'test_foreign_crystal' not in sys.modules
        sys.dont_write_bytecode = True
        sys.path.insert(0, str(pygments.parent.parent))
        sys.path.insert(0, str(old_snapshot))
        sys.path.insert(0, str(snapshot))
        normalizer = importlib.import_module('foreign_crystal')
        tests = importlib.import_module('test_foreign_crystal')
        assert Path(normalizer.__file__).resolve() == (snapshot / 'foreign_crystal.py').resolve()
        assert Path(tests.__file__).resolve() == (snapshot / 'test_foreign_crystal.py').resolve()
        receipt = dict(status='failed-fixtures', started_at=now(), expected_test_ids=expected,
            actual_test_ids=[], tests_run=0, discovered_tests=0, successful=False,
            original_exact_fixture_ids=previous, added_exact_fixture_ids=sorted(additions),
            original_qualified_fixture=dict(path=str(OLD_FIXTURE), sha256=OLD_FIXTURE_SHA, tests_run=672),
            common_timing_mutex=str(LOCK), exclusive_mutex_held=True, common_mutex_fd=lock.fileno(),
            suites=['test_foreign_crystal'], python_version=sys.version, pygments_module=str(pygments),
            pygments_module_sha256=sha(pygments), actual_native_subjects=0, actual_signals_sent=0,
            frozen_dependency_pins=pins, chart_admission=False, canonical_LOC_admitted=False)
        try:
            suite = unittest.defaultTestLoader.loadTestsFromModule(tests)
            receipt['discovered_tests'] = suite.countTestCases()
            log_path = output / 'fixtures.log'
            with log_path.open('x') as log:
                result = unittest.TextTestRunner(stream=log, verbosity=2, resultclass=RecordedResult).run(suite)
            receipt.update(tests_run=result.testsRun, actual_test_ids=result.actual_test_ids,
                successful=result.wasSuccessful(), skipped=[(test.id(), why) for test, why in result.skipped],
                failures=[(test.id(), detail) for test, detail in result.failures],
                errors=[(test.id(), detail) for test, detail in result.errors],
                expected_failures=[(test.id(), detail) for test, detail in result.expectedFailures],
                unexpected_successes=[test.id() for test in result.unexpectedSuccesses])
            assert receipt['tests_run'] == receipt['discovered_tests'] == len(expected)
            assert len(receipt['actual_test_ids']) == len(set(receipt['actual_test_ids'])) == len(expected)
            assert set(receipt['actual_test_ids']) == set(expected)
            assert receipt['successful'] is True
            assert receipt['skipped'] == receipt['failures'] == receipt['errors'] == []
            assert receipt['expected_failures'] == receipt['unexpected_successes'] == []
            pins[str(log_path)] = sha(log_path)
            verify(pins)
            receipt.update(status='passed', finished_at=now())
        except BaseException as error:
            receipt.update(status='failed-fixtures', finished_at=now(), error=str(error) or type(error).__name__, error_type=type(error).__name__)
            raise
        finally:
            new_json(output / 'fixture-receipt.json', receipt)
        fixture_path = output / 'fixture-receipt.json'
        pins[str(fixture_path)] = sha(fixture_path)
        rows, comparison = [], []
        for before in old_replay['profiles']:
            source = Path(before['source'])
            assert source.is_relative_to(CRYSTAL / 'sources') and sha(source) == before['source_sha256']
            metric = normalizer.normalize(source.read_text(), 'crystal')
            assert metric['diagnostics'] == [] and metric['proposal_only'] is True
            assert metric['physical_loc'] == before['physical_loc']
            reduction = 2 if before['id'] == REPAIRED_ID else 0
            assert metric['normalized_loc'] == before['normalized_loc'] - reduction
            old_callbacks = {(event['kind'], event['start'], event['end']) for event in canonical_events(before)
                             if event.get('scope') == 'foreign-crystal-callback'}
            new_callbacks = {(event['kind'], event['start'], event['end']) for event in canonical_events(metric)
                             if event.get('scope') == 'foreign-crystal-callback'}
            assert old_callbacks - new_callbacks == (REMOVED_CALLBACK_SPANS if reduction else set())
            assert new_callbacks - old_callbacks == set()
            if reduction:
                masked = normalizer.mask_crystal(source.read_text())
                owners = normalizer.classify_scopes(masked['cleaned'])
                assert owners['brace_kinds'][1310] == 'data'
                assert source.read_text()[1310:1329] == '{checksum, maximum}'
            pins[str(source)] = before['source_sha256']
            rows.append(dict(id=before['id'], benchmark=before['benchmark'], variant=before['variant'],
                source=str(source), source_sha256=before['source_sha256'], producer_loc=before['producer_loc'], **metric))
            comparison.append(dict(id=before['id'], source_sha256=before['source_sha256'],
                previous_normalized_loc=before['normalized_loc'], actual_normalized_loc=metric['normalized_loc'],
                removed_callback_events=sorted(old_callbacks-new_callbacks), added_callback_events=[],
                exact_scoped_owner_change=True))
        assert len(rows) == len({row['id'] for row in rows}) == 20
        verify(pins)
        replay_path = output / 'source-proposals.json'
        new_json(replay_path, dict(status='source-proposals-awaiting-semantic-review', created_at=now(),
            fixture=str(fixture_path), pins=pins, profiles=rows, source_count=20, full_performance_runs=0,
            chart_admission=False, producer_LOC_mutated=False, parser_pass_is_not_semantic_certification=True,
            prior_proposal=dict(path=str(OLD_REPLAY), sha256=OLD_REPLAY_SHA), owner_comparison=comparison))
        print(json.dumps(dict(fixture=str(fixture_path), fixture_sha256=sha(fixture_path),
            replay=str(replay_path), replay_sha256=sha(replay_path), actual_fixture_methods=len(expected),
            actual_sources=20, changed_owner_profiles=[REPAIRED_ID], canonical_LOC_admitted=False)), flush=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--revision', required=True)
    args = parser.parse_args()
    assert args.revision and all(char in 'abcdefghijklmnopqrstuvwxyz0123456789-' for char in args.revision)
    run(args.revision)


if __name__ == '__main__':
    main()
