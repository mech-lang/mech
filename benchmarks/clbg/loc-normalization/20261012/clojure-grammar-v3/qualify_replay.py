"""Unexecuted locked Clojure fixture/exact-authored-source replay candidate."""
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
CLOJURE = CLBG / 'clojure-comparison/20261012'
LOCK = Path('/tmp/mech-clbg-timing.lock')
OLD_FIXTURE = LOC / 'jobs/boundary-audit-18/fixture-receipt.json'
OLD_SHA = 'b00afe351c40fb38e03a99ec81fe3d71bd69a82726f337aa87dfb40cfcaa6b6a'
PRIOR_REPLAY = LOC / 'clojure-grammar-v2/jobs/grammar-v2-fixtures-source-replay-v1/source-proposals.json'
PRIOR_REPLAY_SHA = 'f627e98645499fdcd90b91746c1aef81446d4e939ff7bcb9b4c8c759116cd3d2'
PROFILE_MANIFEST = CLOJURE / 'revisions/private-v2/profiles.json'
QUALIFIED_REPORT = CLOJURE / 'qualification/private-v2/report.json'
QUALIFIED_SHA = '69bf7cdb025759e6d10cb8c1ae1c189839e253ced9aca50e4f9cc1e838d36d79'
BENCHMARKS = ('binarytrees', 'fannkuchredux', 'nbody', 'spectralnorm', 'mandelbrot',
              'pidigits', 'fasta', 'revcomp', 'knucleotide', 'regexredux')
EXPECTED_METHODS = [
    'test_reader_positions_bom_crlf_comments_strings_regex_characters',
    'test_recursive_reader_discard_removes_only_next_active_form',
    'test_packed_and_expanded_independent_definitions',
    'test_literal_data_wrapping_and_structural_commas_preserved',
    'test_let_independent_initialized_bindings_packed_and_expanded',
    'test_destructuring_one_producer_metadata_no_generated_bindings',
    'test_loop_all_written_initialized_pairs_and_recur',
    'test_do_body_effects_are_not_argument_lists',
    'test_explicit_fn_exposes_all_written_bodies',
    'test_defn_doc_and_multiarity_body_owners',
    'test_callback_bodies_inside_ordinary_call_arguments',
    'test_callback_prefix_merge_preserves_effects_bindings_and_second_callbacks',
    'test_reify_method_owns_formals_body_and_written_closes',
    'test_callable_reify_inside_submit_keeps_nested_method_effects',
    'test_try_catch_finally_all_owned_body_sequences',
    'test_if_when_and_condition_bindings_keep_header_ownership',
    'test_iteration_header_data_and_body_effects',
    'test_cond_case_branch_owners_are_not_flat_data',
    'test_java_interop_set_and_throw_expressions',
    'test_scalar_anonymous_composition_has_no_generated_param_units',
    'test_stateful_anonymous_callback_has_no_blanket_scalar_waiver',
    'test_live_vector_argument_cannot_hide_nested_do_or_fn',
    'test_written_namespace_requires_imports_directives_not_generated_code',
    'test_unknown_macro_and_generated_scopes_require_review',
    'test_reader_eval_tagged_conditionals_and_syntax_quotes_diagnosed',
    'test_mismatched_unclosed_and_orphan_delimiters',
    'test_malformed_binding_metadata_and_generated_destructuring',
    'test_unsupported_iteration_modifier_and_nested_anonymous',
    'test_loc_language_alias_is_rejected',
    'test_iteration_callback_header_keeps_callable_body_close_and_all_work',
    'test_typed_array_read_prefix_rejects_shadowed_and_effectful_operands',
    'test_exact_BigInteger_value_prefix_rejects_changed_helper_type_and_target',
]
EXPECTED_IDS = sorted('test_foreign_clojure.ClojureProposalFixtures.' + name for name in EXPECTED_METHODS)


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


def validate_fixture(receipt, expected=32):
    assert receipt['status'] == 'passed' and receipt['successful'] is True
    assert receipt['tests_run'] == receipt['discovered_tests'] == expected
    ids = EXPECTED_IDS if expected == 32 else sorted('test_foreign_clojure.ClojureProposalFixtures.' + name for name in EXPECTED_METHODS[:-3])
    assert receipt['expected_test_ids'] == ids and sorted(receipt['actual_test_ids']) == ids
    assert len(receipt['actual_test_ids']) == len(set(receipt['actual_test_ids'])) == expected
    if expected == 32:
        assert sorted(receipt['discovered_test_ids']) == ids
        assert len(receipt['discovered_test_ids']) == len(set(receipt['discovered_test_ids'])) == expected
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
    assert [node.name for node in classes] == ['ClojureProposalFixtures']
    methods = [node.name for node in classes[0].body if isinstance(node, ast.FunctionDef) and node.name.startswith('test_')]
    assert len(methods) == len(set(methods))
    return sorted('test_foreign_clojure.ClojureProposalFixtures.' + name for name in methods)


def source_inventory(report_path, report_sha):
    assert report_path.resolve() == QUALIFIED_REPORT and report_sha == QUALIFIED_SHA
    assert sha(report_path) == report_sha
    qualified = json.loads(report_path.read_text())
    assert qualified['status'] == 'bounded-output-ownership-passed-awaiting-actual-work-and-full-review'
    assert qualified['active'] is None and qualified['pins_unchanged_at_end'] is True
    assert qualified['reused_profiles'] == 4 and qualified['fresh_profile_builds'] == 16
    assert qualified['reused_bounded_cases'] == 24 and qualified['fresh_native_attempts'] == 132
    assert qualified['old_cases_relabelled_as_new_attempts'] is False
    pins = dict(qualified['pins']); verify(pins)
    declared = json.loads(PROFILE_MANIFEST.read_text())
    assert declared['language'] == 'clojure' and declared['compiler_version'] == '1.12.6'
    rows = declared['profiles']
    assert len(rows) == len({row['id'] for row in rows}) == 20
    expected = {('clojure-' + benchmark + '-' + variant):
        [('revisions/private-v2/' if benchmark == 'nbody' else '') + 'sources/clbg/' + benchmark + '/' + variant + '.clj', 'support/clbg/support.clj']
        for benchmark in BENCHMARKS for variant in ('compact', 'optimized')}
    assert {row['id']: row['compiled_source_bundle'] for row in rows} == expected
    paths = sorted({CLOJURE / name for names in expected.values() for name in names})
    assert len(paths) == 21
    assert sha(PROFILE_MANIFEST) == pins[str(PROFILE_MANIFEST)]
    actual = {row['id']: row for row in qualified['profiles']}
    assert len(actual) == len(qualified['profiles']) == 20 and set(actual) == set(expected)
    evidence = {}
    for row in rows:
        native = actual[row['id']]
        inventory_path = Path(native['compiled_input_inventory'])
        assert sha(inventory_path) == native['compiled_input_inventory_sha256'] == pins[str(inventory_path)]
        compiled = json.loads(inventory_path.read_text())
        assert compiled['status'] == 'compiled-exact-authored-input-inventory' and compiled['profile'] == row['id']
        assert compiled['source_bundle'] == native['source_bundle'] and compiled['source'] == native['source']
        assert compiled['source_sha256'] == native['source_sha256']
        assert sha(compiled['compiler_jar']) == compiled['compiler_sha256'] == pins[compiled['compiler_jar']]
        assert compiled['compiled_artifacts'] == native['compiled_artifacts'] and compiled['runtime_classpath'] == native['runtime_classpath']
        assert compiled['compiled_artifacts'] and all(Path(name).suffix == '.class' and sha(name) == digest == pins[name] for name, digest in compiled['compiled_artifacts'].items())
        receipt_path = inventory_path.parent / 'compile.json'
        assert str(receipt_path) == native['compile_receipt']
        assert sha(receipt_path) == native['compile_receipt_sha256'] == pins[str(receipt_path)]
        receipt = json.loads(receipt_path.read_text())
        assert receipt['command'] == compiled['build_command'] and receipt['exit_status'] == receipt['final_exit_status'] == 0
        assert receipt['inherited_common_mutex'] is True and not any(receipt.get(name) for name in ('error', 'cleanup_error', 'cleanup_errors', 'remaining_group_members'))
        assert sha(receipt['command'][0]) == pins[receipt['command'][0]]
        assert len(compiled['source_bundle']) == 2
        bound = []
        for index, (relative, item) in enumerate(zip(row['compiled_source_bundle'], compiled['source_bundle'])):
            author = CLOJURE / relative; copy = Path(item['path']); origin = Path(item['origin'])
            assert item['role'] == ('application' if index == 0 else 'application-helper')
            assert sha(author) == sha(copy) == sha(origin) == item['sha256'] == pins[str(author)] == pins[str(copy)] == pins[str(origin)]
            bound.append(dict(authored_source=str(author), sha256=item['sha256'], role=item['role'], actual_compiled_copy=str(copy), actual_snapshot_origin=str(origin)))
        package = Path(compiled['source_bundle'][0]['path']).parents[3]
        assert [str(Path(item['path']).relative_to(package)) for item in compiled['source_bundle']] == ['sources/' + row['namespace'].replace('.', '/') + '.clj', 'support/clbg/support.clj']
        assert {path for path in package.rglob('*') if path.is_file()} == {Path(item['path']) for item in compiled['source_bundle']}
        evidence[row['id']] = dict(status='actual-compiler-inputs-bound-for-unadmitted-source-replay',
            inventory=str(inventory_path), inventory_sha256=pins[str(inventory_path)],
            compile_receipt=str(receipt_path), compile_receipt_sha256=pins[str(receipt_path)],
            compiler_jar=compiled['compiler_jar'], compiler_sha256=compiled['compiler_sha256'],
            build_command=compiled['build_command'], runtime_classpath=compiled['runtime_classpath'],
            compiled_artifacts=compiled['compiled_artifacts'], files=bound)
    return rows, paths, evidence, pins


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


def run(revision, stage, qualification_report=None, qualification_sha=None, fixture_receipt=None, fixture_sha=None):
    print('WAIT common mutex: Clojure-owned grammar fixture/source proposals', flush=True)
    with LOCK.open('a+') as lock, terminal_evidence(revision, lock):
        descriptor, named = os.fstat(lock.fileno()), LOCK.stat()
        assert (descriptor.st_dev, descriptor.st_ino) == (named.st_dev, named.st_ino)
        assert sha(PRIOR_REPLAY) == PRIOR_REPLAY_SHA
        prior = json.loads(PRIOR_REPLAY.read_text())
        assert prior['status'] == 'source-proposals-needing-grammar-review' and len(prior['unresolved']) == 15
        assert prior['unique_source_count'] == 21 and prior['profile_count'] == 20 and prior['canonical_LOC_admitted'] is False
        verify(prior['pins'])
        prior_fixture = json.loads(Path(prior['fixture']).read_text())
        validate_fixture(prior_fixture, 29)
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
        profiles, sources, compiler_evidence, qualified_pins = source_inventory(qualification_report, qualification_sha) if stage == 'replay' else ([], [], {}, {})
        pins = merge(dependencies, prior['pins'], qualified_pins)
        pins[str(PRIOR_REPLAY)] = PRIOR_REPLAY_SHA
        pins.update({str(OLD_FIXTURE): OLD_SHA, str(pygments): sha(pygments),
                     str(PROFILE_MANIFEST): sha(PROFILE_MANIFEST)})
        if stage == 'replay': pins[str(qualification_report)] = qualification_sha
        pins.update({str(path): sha(path) for path in sources})
        output = HERE / 'jobs' / revision
        assert not output.exists(), 'Exclusive additive output; preserve failed receipts and old evidence'
        snapshot = output / 'snapshot'
        snapshot.mkdir(parents=True)
        for name in ('foreign_clojure.py', 'test_foreign_clojure.py', 'qualify_replay.py', 'REVIEW.md'):
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
            for name in ('foreign_clojure.py', 'test_foreign_clojure.py', 'qualify_replay.py', 'REVIEW.md'):
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
        assert inventory(snapshot / 'test_foreign_clojure.py') == EXPECTED_IDS
        assert len(EXPECTED_IDS) == len(set(EXPECTED_IDS)) == 32
        assert not any(name in sys.modules for name in ('foreign_clojure', 'test_foreign_clojure', 'foreign_core', 'canonical', 'pygments'))
        sys.dont_write_bytecode = True
        sys.path.insert(0, str(pygments.parent.parent))
        sys.path.insert(0, str(grammar_snapshot))
        sys.path.insert(0, str(snapshot))
        normalizer = importlib.import_module('foreign_clojure')
        tests = importlib.import_module('test_foreign_clojure')
        assert Path(normalizer.__file__).resolve() == (snapshot / 'foreign_clojure.py').resolve()
        assert Path(tests.__file__).resolve() == (snapshot / 'test_foreign_clojure.py').resolve()
        for name in ('foreign_core', 'canonical'):
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
                assert receipt['tests_run'] == receipt['discovered_tests'] == 32
                assert len(receipt['actual_test_ids']) == len(set(receipt['actual_test_ids'])) == 32
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
            print(json.dumps(dict(fixture=str(fixture_path), fixture_sha256=sha(fixture_path), actual_fixture_methods=32,
                source_counting_started=False, canonical_LOC_admitted=False)), flush=True)
            return
        fixture_path = Path(fixture_receipt)
        metrics = {}
        for source in sources:
            metric = normalizer.normalize(source.read_text(), 'clojure')
            assert metric['proposal_only'] is True
            metrics[str(source)] = dict(source=str(source), source_sha256=pins[str(source)], **metric)
        rows = [dict(id=row['id'], benchmark=row['benchmark'], variant=row['variant'], producer_loc=None,
            source_bundle=[metrics[str(CLOJURE / name)] for name in row['compiled_source_bundle']],
            physical_loc=sum(metrics[str(CLOJURE / name)]['physical_loc'] for name in row['compiled_source_bundle']),
            normalized_loc=sum(metrics[str(CLOJURE / name)]['normalized_loc'] for name in row['compiled_source_bundle']))
            for row in profiles]
        unresolved = [dict(source=path, diagnostic=item) for path, metric in metrics.items() for item in metric['diagnostics']]
        unittest_identities()
        verify(pins)
        replay = output / 'source-proposals.json'
        new_json(replay, dict(status='source-proposals-needing-grammar-review' if unresolved else 'source-proposals-awaiting-semantic-review',
            fixture=str(fixture_path), created_at=now(), pins=pins, profiles=rows, unique_source_count=21,
            profile_count=20, counted_helper=str(CLOJURE / 'support/clbg/support.clj'), unresolved=unresolved,
            qualification=dict(path=str(qualification_report), sha256=qualification_sha), compiler_input_evidence=compiler_evidence,
            prior_original_bad_source_replay=dict(path=str(PRIOR_REPLAY), sha256=PRIOR_REPLAY_SHA, preserved_unresolved=15),
            full_performance_runs=0, chart_admission=False, canonical_LOC_admitted=False,
            producer_LOC_mutated=False, parser_pass_is_not_semantic_certification=True))
        print(json.dumps(dict(fixture=str(fixture_path), fixture_sha256=sha(fixture_path),
            replay=str(replay), replay_sha256=sha(replay), actual_fixture_methods_reused=32,
            actual_unique_sources=21, actual_profiles=20, unresolved=len(unresolved), canonical_LOC_admitted=False)), flush=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--revision', required=True)
    parser.add_argument('--stage', choices=('fixtures', 'replay'), required=True)
    parser.add_argument('--qualification-report', type=Path)
    parser.add_argument('--qualification-sha256')
    parser.add_argument('--fixture-receipt', type=Path)
    parser.add_argument('--fixture-sha256')
    args = parser.parse_args()
    assert re.fullmatch(r'[a-z0-9][a-z0-9-]{0,63}', args.revision)
    if args.stage == 'replay': assert all((args.qualification_report, args.qualification_sha256, args.fixture_receipt, args.fixture_sha256))
    run(args.revision, args.stage, args.qualification_report, args.qualification_sha256, args.fixture_receipt, args.fixture_sha256)


if __name__ == '__main__':
    main()
