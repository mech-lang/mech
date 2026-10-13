"""Unexecuted locked Pony fixture/exact-authored-source replay candidate."""
import argparse
import ast
from contextlib import contextmanager
from datetime import datetime, timezone
import fcntl
import hashlib
import importlib
import json
import os
from pathlib import Path
import re
import shutil
import sys
import sysconfig
import traceback

HERE = Path(__file__).resolve().parent
LOC = HERE.parent
CLBG = LOC.parents[1]
PONY = CLBG / 'pony-comparison/20261012'
LOCK = Path('/tmp/mech-clbg-timing.lock')
OLD_FIXTURE = LOC / 'jobs/boundary-audit-18/fixture-receipt.json'
OLD_SHA = 'b00afe351c40fb38e03a99ec81fe3d71bd69a82726f337aa87dfb40cfcaa6b6a'
INVENTORY = PONY / 'source-counts/compiler-input-v1/compiler-input-proposals.json'
INVENTORY_SHA = '008340b7e28051a6b042c15de713c6e35462cb8a2f108ed5114ed9342d52fb92'
PRIOR_REPLAY = PONY / 'source-counts/loc18-replay-v1/source-count-proposals.json'
PRIOR_REPLAY_SHA = '582d986ddccd9dd93b762f83ad8dca87445600a35286ff09b370b43faad2a511'
QUALIFIED_REPORT = PONY / 'qualification/private-v8/report.json'
QUALIFIED_SHA = '4da05f6dd351609923f5f1ca227d3db2b33a354925614b74f9c5dae39839d159'
FILES = ('foreign_pony.py', 'scalar_resolution.py', 'test_foreign_pony.py', 'qualify_replay.py', 'REVIEW.md')
EXPECTED_METHODS = (
    'test_type_and_member_layout_have_no_imputed_end', 'test_independent_constructor_statements',
    'test_pure_method_is_one_composition', 'test_imperative_method_exposes_locals_and_tail',
    'test_capabilities_partial_markers_and_formals_are_structural', 'test_written_control_end_and_branches',
    'test_for_iteration_patterns_are_header_owned', 'test_control_nested_in_method_keeps_bodies',
    'test_value_if_operand_is_one_outer_binding', 'test_pure_match_patterns_are_not_local_bindings',
    'test_imperative_match_arms_and_written_close', 'test_pure_recover_constructor_is_one_composition',
    'test_imperative_recover_exposes_header_body_and_end', 'test_embedded_imperative_recover_keeps_outer_binding',
    'test_receiving_binding_and_reset_are_two_independent_writes', 'test_where_named_arguments_are_not_assignments',
    'test_union_tuple_and_generic_data_are_structural', 'test_data_wrapping_is_preserved',
    'test_receiver_composition_keeps_written_wrapping', 'test_pure_partial_try_value_is_composition',
    'test_imperative_try_header_and_written_end', 'test_mixed_error_value_controls_remain_gated',
    'test_expression_only_anonymous_lambda_is_composition', 'test_ffi_declaration_is_source_not_generated_body',
    'test_unknown_type_annotation_and_control_forms_remain_gated', 'test_nested_comments_literals_and_apostrophe_names',
    'test_numeric_operators_and_partial_reference_are_not_bindings',
    'test_scheduling_output_and_actor_factory_controls_keep_all_written_owners',
    'test_embedded_unknown_or_mutating_calls_keep_binding_and_real_control',
    'test_scalar_method_requires_actual_entire_owned_source_body',
    'test_boxed_recursive_scalar_graph_check_has_actual_source_facts',
    'test_nearer_formal_local_and_type_shadow_reject_scalar_resolution',
    'test_return_hint_duplicate_helper_actor_and_vendor_calls_are_not_proof',
)
EXPECTED_IDS = sorted('test_foreign_pony.PonyFixtures.' + name for name in EXPECTED_METHODS)


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def verify(pins):
    for path, digest in pins.items():
        assert sha(path) == digest, ('Reviewed source/dependency changed', path)


def merge(*groups):
    result = {}
    for group in groups:
        for path, digest in group.items():
            assert path not in result or result[path] == digest, ('Conflicting actual pin', path)
            result[path] = digest
    return result


def validate_fixture(receipt):
    assert receipt['status'] == 'passed' and receipt['successful'] is True
    assert receipt['tests_run'] == receipt['discovered_tests'] == 33
    assert receipt['expected_test_ids'] == EXPECTED_IDS
    for key in ('actual_test_ids', 'discovered_test_ids'):
        assert sorted(receipt[key]) == EXPECTED_IDS and len(receipt[key]) == len(set(receipt[key])) == 33
    assert receipt['exclusive_mutex_held'] is True and receipt['common_timing_mutex'] == str(LOCK)
    assert type(receipt['actual_native_subjects']) is type(receipt['actual_signals_sent']) is int
    assert receipt['actual_native_subjects'] == receipt['actual_signals_sent'] == 0
    assert receipt['canonical_LOC_admitted'] is receipt['chart_admission'] is False
    assert all(receipt[name] == [] for name in ('failures', 'errors', 'skipped', 'expected_failures', 'unexpected_successes'))


def new_json(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def now():
    return datetime.now(timezone.utc).isoformat()


def inventory(test_path):
    tree = ast.parse(test_path.read_text())
    classes = [node for node in tree.body if isinstance(node, ast.ClassDef)]
    assert [node.name for node in classes] == ['PonyFixtures']
    methods = [node.name for node in classes[0].body if isinstance(node, ast.FunctionDef) and node.name.startswith('test_')]
    assert len(methods) == len(set(methods))
    return sorted('test_foreign_pony.PonyFixtures.' + name for name in methods)


def source_inventory():
    assert sha(INVENTORY) == INVENTORY_SHA and sha(PRIOR_REPLAY) == PRIOR_REPLAY_SHA
    assert sha(QUALIFIED_REPORT) == QUALIFIED_SHA
    inventory_doc = json.loads(INVENTORY.read_text())
    prior = json.loads(PRIOR_REPLAY.read_text())
    qualified = json.loads(QUALIFIED_REPORT.read_text())
    assert inventory_doc['status'] == 'compiler-input-proposals-awaiting-role-review'
    assert inventory_doc['unresolved'] == [] and inventory_doc['profile_count'] == len(inventory_doc['profiles']) == 20
    assert inventory_doc['qualification'] == dict(path=str(QUALIFIED_REPORT), sha256=QUALIFIED_SHA)
    assert qualified['status'] == 'bounded-output-primitives-passed-awaiting-work-worker-and-LOC-review'
    assert qualified['reused_bounded_cases'] == 154 and qualified['fresh_unscored_primitive_cases'] == 1
    assert len(qualified['profiles']) == 20
    assert prior['status'] == 'actual-source-count-proposals-awaiting-selected-source-review'
    assert prior['inventory'] == dict(path=str(INVENTORY), sha256=INVENTORY_SHA)
    assert prior['canonical_LOC_admitted'] is prior['chart_admission'] is False
    rows = inventory_doc['profiles']; native = {row['id']: row for row in qualified['profiles']}
    assert len(native) == 20 and set(native) == {row['id'] for row in rows} == set(prior['profiles'])
    fixture = inventory_doc['packet_fixture_receipt']; assert sha(fixture['path']) == fixture['sha256']
    actual8 = json.loads(Path(fixture['path']).read_text())
    assert actual8['status'] == 'passed' and actual8['actual_common_mutex_held'] is True and actual8['tests_run'] == 8
    assert sorted(actual8['expected_ids']) == sorted(actual8['discovered_ids']) == sorted(actual8['started_ids'])
    assert len(actual8['started_ids']) == len(set(actual8['started_ids'])) == 8
    for key in ('failures', 'errors', 'skipped', 'expected_failures', 'unexpected_successes', 'native_processes'):
        assert type(actual8[key]) is int and actual8[key] == 0
    pins = merge(inventory_doc['pins'], prior['pins'], qualified['pins'],
        {str(INVENTORY): INVENTORY_SHA, str(PRIOR_REPLAY): PRIOR_REPLAY_SHA, str(QUALIFIED_REPORT): QUALIFIED_SHA})
    verify(pins)
    paths = set()
    for row in rows:
        actual = native[row['id']]
        assert row['source'] == actual['source'] and row['source_sha256'] == actual['source_sha256']
        assert row['binary'] == actual['command'][0] and row['binary_sha256'] == actual['binary_sha256']
        assert row['actual_original_compile_command'] == json.loads(Path(row['production_build_receipt']).read_text())['command']
        expected_bundle = [{key: item[key] for key in ('path', 'sha256', 'role')} for item in row['files']]
        assert prior['profiles'][row['id']]['source_bundle'] == expected_bundle
        assert row['files'] and row['files'][0]['path'] == row['source'] and row['files'][0]['role'] == 'application'
        assert len(row['files']) == (2 if row['benchmark'] == 'pidigits' else 1)
        for item in row['files']:
            path = Path(item['path']); assert item['source_language'] == 'pony'
            assert sha(path) == item['sha256'] == pins[str(path)] == prior['source_counts'][str(path)]['source_sha256']
            paths.add(path)
    assert len(paths) == len(prior['source_counts']) == 21
    assert sum(len(value['metric']['diagnostics']) for value in prior['source_counts'].values()) == 6
    return inventory_doc, prior, sorted(paths), pins


@contextmanager
def terminal_evidence(revision, lock):
    # This context acquires the very same opened descriptor before any source,
    # fixture or import gate; early exceptions therefore retain actual evidence.
    fcntl.flock(lock, fcntl.LOCK_EX)
    output = HERE / 'jobs' / revision
    assert not output.exists(), 'Exclusive additive revision; no overwrite of terminal evidence'
    failed = False
    try:
        yield
    except BaseException as error:
        failed = True
        output.mkdir(parents=True, exist_ok=True)
        fixture_path = output / 'fixture-receipt.json'
        fixture_status = json.loads(fixture_path.read_text())['status'] if fixture_path.exists() else 'not-started-or-not-saved'
        error_path = output / 'terminal-error.log'
        with error_path.open('x') as stream:
            stream.write(traceback.format_exc())
        new_json(output / 'terminal-error.json', dict(status='failed-after-saved-passed-fixtures' if fixture_status == 'passed' else 'failed-fixture-or-preflight',
            error=str(error) or type(error).__name__, error_type=type(error).__name__, fixture_status=fixture_status,
            error_log=str(error_path), error_log_sha256=sha(error_path), exclusive_mutex_held=True,
            common_timing_mutex=str(LOCK), actual_native_subjects=0, actual_signals_sent=0))
        raise
    finally:
        if output.exists():
            terminal = {str(path): sha(path) for path in output.rglob('*') if path.is_file()}
            new_json(output / 'terminal-pins.json', dict(status='failed-terminal-evidence-retained' if failed else 'completed-proposal-evidence-retained',
                pins=terminal, exclusive_mutex_held=True, common_timing_mutex=str(LOCK),
                canonical_LOC_admitted=False, chart_admission=False))


def run(revision, stage, fixture_receipt=None, fixture_sha=None):
    print('WAIT common mutex: Pony-owned grammar fixture/source proposals', flush=True)
    with LOCK.open('a+') as lock, terminal_evidence(revision, lock):
        descriptor, named = os.fstat(lock.fileno()), LOCK.stat()
        assert (descriptor.st_dev, descriptor.st_ino) == (named.st_dev, named.st_ino)
        inventory_doc, prior, actual_sources, qualified_pins = source_inventory()
        assert sha(OLD_FIXTURE) == OLD_SHA
        old = json.loads(OLD_FIXTURE.read_text())
        assert old['status'] == 'passed' and old['tests_run'] == 672
        assert old['failures'] == old['errors'] == 0 and old['exclusive_mutex_held'] is True
        assert old['common_timing_mutex'] == str(LOCK)
        assert sys.version.split()[0] == old['python_version'].split()[0]
        assert sys.version_info[:2] == (3, 14) and old['pygments_version'] == '2.19.2'
        dependencies = dict(old.get('frozen_dependency_pins', old.get('frozen_module_pins', {})))
        assert dependencies
        verify(dependencies)
        pygments = Path(old['pygments_module'])
        assert sha(pygments) == old['pygments_module_sha256']
        sources = actual_sources if stage == 'replay' else []
        pins = merge(dependencies, qualified_pins)
        pins.update({str(OLD_FIXTURE): OLD_SHA, str(pygments): sha(pygments)})
        output = HERE / 'jobs' / revision
        assert not output.exists(), 'Exclusive additive output; preserve failed receipts and old evidence'
        snapshot = output / 'snapshot'
        snapshot.mkdir(parents=True)
        for name in FILES:
            source, target = HERE / name, snapshot / name
            shutil.copyfile(source, target)
            assert sha(source) == sha(target)
            pins[str(source)] = pins[str(target)] = sha(source)
            target.chmod(0o444)
        grammar_snapshot = output / 'grammar-dependencies'
        for filename, digest in dependencies.items():
            source = Path(filename)
            assert source.is_relative_to(OLD_FIXTURE.parent / 'snapshot')
            target = grammar_snapshot / source.relative_to(OLD_FIXTURE.parent / 'snapshot')
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, target); assert sha(target) == digest
            pins[str(target)] = digest; target.chmod(0o444)
        lexical_pins = {str(path): sha(path) for path in pygments.parent.rglob('*.py')}
        pins = merge(pins, lexical_pins)
        if stage == 'replay':
            assert sha(fixture_receipt) == fixture_sha
            saved_fixture = json.loads(fixture_receipt.read_text()); validate_fixture(saved_fixture)
            assert saved_fixture['python_version'].split()[0] == sys.version.split()[0]
            verify(saved_fixture['frozen_dependency_pins'])
            for name in FILES:
                assert pins[str(HERE / name)] == saved_fixture['frozen_dependency_pins'][str(HERE / name)]
            pins = merge(pins, saved_fixture['frozen_dependency_pins'], {str(fixture_receipt): fixture_sha})
        python = Path(sys.executable).resolve(strict=True)
        pins[str(python)] = sha(python)
        unittest_root = Path(sysconfig.get_path('stdlib')).resolve(strict=True) / 'unittest'
        unittest_snapshot = output / 'python-dependencies/unittest'
        python_dependency_pins = {str(python): pins[str(python)]}
        assert not any(name == 'unittest' or name.startswith('unittest.') for name in sys.modules), 'No cached unittest code before locked source pinning'
        for source in sorted(unittest_root.rglob('*.py')):
            target = unittest_snapshot / source.relative_to(unittest_root)
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, target)
            assert sha(source) == sha(target)
            pins[str(source)] = pins[str(target)] = sha(source)
            python_dependency_pins[str(source)] = python_dependency_pins[str(target)] = sha(source)
            target.chmod(0o444)
        assert python_dependency_pins and (unittest_snapshot / '__init__.py').is_file()
        assert not list(unittest_snapshot.rglob('*.pyc'))
        sys.dont_write_bytecode = True
        sys.path.insert(0, str(unittest_snapshot.parent))
        unittest = importlib.import_module('unittest')
        assert Path(unittest.__file__).resolve() == (unittest_snapshot / '__init__.py').resolve()
        def unittest_identities():
            identities = {}
            for name, loaded in sys.modules.items():
                if name == 'unittest' or name.startswith('unittest.'):
                    path = Path(loaded.__file__).resolve(strict=True)
                    assert path.is_relative_to(unittest_snapshot) and str(path) in pins
                    identities[name] = str(path)
            assert identities
            return identities
        unittest_identities()
        class RecordedResult(unittest.TextTestResult):
            def __init__(self, *args):
                super().__init__(*args)
                self.actual_test_ids = []
            def startTest(self, test):
                self.actual_test_ids.append(test.id())
                super().startTest(test)
        assert inventory(snapshot / 'test_foreign_pony.py') == EXPECTED_IDS
        assert len(EXPECTED_IDS) == len(set(EXPECTED_IDS)) == 33
        assert not any(name in sys.modules for name in ('foreign_pony', 'scalar_resolution', 'test_foreign_pony', 'foreign_core', 'foreign_future_mask', 'canonical', 'pygments'))
        sys.dont_write_bytecode = True
        sys.path.insert(0, str(pygments.parent.parent))
        sys.path.insert(0, str(grammar_snapshot))
        sys.path.insert(0, str(snapshot))
        normalizer = importlib.import_module('foreign_pony')
        tests = importlib.import_module('test_foreign_pony')
        assert Path(normalizer.__file__).resolve() == (snapshot / 'foreign_pony.py').resolve()
        assert Path(tests.__file__).resolve() == (snapshot / 'test_foreign_pony.py').resolve()
        assert Path(sys.modules['scalar_resolution'].__file__).resolve() == snapshot / 'scalar_resolution.py'
        for name in ('foreign_core', 'foreign_future_mask', 'canonical'):
            assert Path(sys.modules[name].__file__).resolve() == grammar_snapshot / (name + '.py')
        assert Path(sys.modules['pygments'].__file__).resolve() == pygments.resolve()
        assert sys.modules['pygments'].__version__ == '2.19.2'
        verify(pins)
        receipt = dict(status='failed-fixtures', started_at=now(), expected_test_ids=EXPECTED_IDS,
            actual_test_ids=[], tests_run=0, discovered_tests=0, successful=False,
            common_timing_mutex=str(LOCK), exclusive_mutex_held=True, common_mutex_fd=lock.fileno(),
            python_version=sys.version, pygments_module=str(pygments), pygments_module_sha256=sha(pygments),
            python_executable=str(python), python_executable_sha256=pins[str(python)],
            python_unittest_dependency_pins=python_dependency_pins, unittest_module_identities=unittest_identities(),
            frozen_dependency_pins=pins, original_qualified_fixture=dict(path=str(OLD_FIXTURE), sha256=OLD_SHA),
            actual_native_subjects=0, actual_signals_sent=0, canonical_LOC_admitted=False, chart_admission=False)
        if stage == 'fixtures':
            try:
                suite = unittest.defaultTestLoader.loadTestsFromModule(tests)
                receipt['discovered_tests'] = suite.countTestCases()
                def ids(value):
                    for item in value:
                        if isinstance(item, unittest.TestSuite): yield from ids(item)
                        else: yield item.id()
                receipt['discovered_test_ids'] = list(ids(suite))
                assert sorted(receipt['discovered_test_ids']) == EXPECTED_IDS
                log_path = output / 'fixtures.log'
                with log_path.open('x') as log:
                    result = unittest.TextTestRunner(stream=log, verbosity=2, resultclass=RecordedResult).run(suite)
                pins[str(log_path)] = sha(log_path)
                receipt.update(tests_run=result.testsRun, actual_test_ids=result.actual_test_ids,
                    successful=result.wasSuccessful(), failures=[(test.id(), detail) for test, detail in result.failures],
                    errors=[(test.id(), detail) for test, detail in result.errors],
                    skipped=[(test.id(), why) for test, why in result.skipped],
                    expected_failures=[(test.id(), detail) for test, detail in result.expectedFailures],
                    unexpected_successes=[test.id() for test in result.unexpectedSuccesses])
                assert receipt['tests_run'] == receipt['discovered_tests'] == 33
                assert len(receipt['actual_test_ids']) == len(set(receipt['actual_test_ids'])) == 33
                assert sorted(receipt['actual_test_ids']) == EXPECTED_IDS and receipt['successful'] is True
                for name in ('failures', 'errors', 'skipped', 'expected_failures', 'unexpected_successes'): assert receipt[name] == []
                receipt['unittest_module_identities'] = unittest_identities()
                verify(pins); receipt.update(status='passed', finished_at=now())
            except BaseException as error:
                receipt.update(status='failed-fixtures', finished_at=now(), error=str(error) or type(error).__name__, error_type=type(error).__name__)
                raise
            finally:
                new_json(output / 'fixture-receipt.json', receipt)
            fixture_path = output / 'fixture-receipt.json'
            print(json.dumps(dict(fixture=str(fixture_path), fixture_sha256=sha(fixture_path), actual_fixture_methods=33,
                source_counting_started=False, canonical_LOC_admitted=False)), flush=True)
            return
        fixture_path = Path(fixture_receipt)
        counts = {}
        for source in sources:
            text = source.read_text()
            metric = normalizer.normalize(text, 'pony')
            assert metric['eligible_for_chart'] is False
            counts[str(source)] = dict(source_sha256=pins[str(source)], source_character_length=len(text), metric=metric)
        rows = {row['id']: dict(id=row['id'], source_sha256=row['source_sha256'], binary_sha256=row['binary_sha256'],
            source_bundle=[{key: item[key] for key in ('path', 'sha256', 'role')} for item in row['files']],
            producer_loc=row['producer_loc']) for row in inventory_doc['profiles']}
        unittest_identities(); verify(pins)
        replay = output / 'source-count-proposals.json'
        new_json(replay, dict(status='actual-source-count-proposals-awaiting-selected-source-review',
            inventory=dict(path=str(INVENTORY), sha256=INVENTORY_SHA), source_counts=counts, profiles=rows,
            normalization_policy=dict(rule_version='canonical-source-units-20261012-approved-rule-v2',
                grammar=normalizer.PROPOSAL_VERSION, evidence_path=str(grammar_snapshot / 'foreign_core.py'),
                evidence_sha256=pins[str(grammar_snapshot / 'foreign_core.py')]),
            fixture_receipt=dict(path=str(fixture_path), sha256=fixture_sha, status='passed'),
            qualified_neutral_fixture=dict(path=str(OLD_FIXTURE), sha256=OLD_SHA, actual_tests=672),
            packet_fixture_receipt=inventory_doc['packet_fixture_receipt'], grammar_snapshot_pins={
                path: digest for path, digest in pins.items() if Path(path).is_relative_to(snapshot) or Path(path).is_relative_to(grammar_snapshot)},
            prior_original_replay=dict(path=str(PRIOR_REPLAY), sha256=PRIOR_REPLAY_SHA,
                preserved_emitted_diagnostics=6, lost_effect_control_owners_not_waived=True),
            pins=pins, actual_common_mutex_held=True, canonical_LOC_admitted=False, chart_admission=False,
            parser_pass_is_not_semantic_certification=True, count_override=False, full_performance_runs=0))
        print(json.dumps(dict(fixture=str(fixture_path), fixture_sha256=fixture_sha,
            replay=str(replay), replay_sha256=sha(replay), actual_fixture_methods_reused=33,
            actual_unique_sources=21, actual_profiles=20, canonical_LOC_admitted=False)), flush=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--revision', required=True)
    parser.add_argument('--stage', choices=('fixtures', 'replay'), required=True)
    parser.add_argument('--fixture-receipt', type=Path)
    parser.add_argument('--fixture-sha256')
    args = parser.parse_args()
    assert re.fullmatch(r'[a-z0-9][a-z0-9-]{0,63}', args.revision)
    if args.stage == 'replay': assert all((args.fixture_receipt, args.fixture_sha256))
    run(args.revision, args.stage, fixture_receipt=args.fixture_receipt, fixture_sha=args.fixture_sha256)


if __name__ == '__main__':
    main()
