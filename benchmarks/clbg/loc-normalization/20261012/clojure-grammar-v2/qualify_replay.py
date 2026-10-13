"""Unexecuted locked Clojure fixture/exact-authored-source replay candidate."""
import argparse
import ast
from contextlib import contextmanager
from datetime import datetime, timezone
import fcntl
import hashlib
import importlib
import json
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
]
EXPECTED_IDS = sorted('test_foreign_clojure.ClojureProposalFixtures.' + name for name in EXPECTED_METHODS)


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def verify(pins):
    for path, digest in pins.items():
        assert sha(path) == digest, ('Reviewed source/dependency changed', path)


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


def source_inventory():
    declared = json.loads((CLOJURE / 'profiles.json').read_text())
    assert declared['language'] == 'clojure' and declared['compiler_version'] == '1.12.6'
    rows = declared['profiles']
    assert len(rows) == len({row['id'] for row in rows}) == 20
    expected = {('clojure-' + benchmark + '-' + variant):
        ['sources/clbg/' + benchmark + '/' + variant + '.clj', 'support/clbg/support.clj']
        for benchmark in BENCHMARKS for variant in ('compact', 'optimized')}
    assert {row['id']: row['compiled_source_bundle'] for row in rows} == expected
    paths = sorted({CLOJURE / name for names in expected.values() for name in names})
    assert len(paths) == 21
    assert set(CLOJURE.glob('sources/**/*.clj')) | set(CLOJURE.glob('support/**/*.clj')) == set(paths)
    return rows, paths


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


def run(revision):
    print('WAIT common mutex: Clojure-owned grammar fixture/source proposals', flush=True)
    with LOCK.open('a+') as lock, terminal_evidence(revision, lock):
        assert sha(OLD_FIXTURE) == OLD_SHA
        old = json.loads(OLD_FIXTURE.read_text())
        assert old['status'] == 'passed' and old['tests_run'] == 672
        assert old['failures'] == old['errors'] == 0 and old['exclusive_mutex_held'] is True
        assert old['common_timing_mutex'] == str(LOCK)
        assert sys.version.split()[0] == old['python_version'].split()[0]
        dependencies = dict(old.get('frozen_dependency_pins', old.get('frozen_module_pins', {})))
        assert dependencies
        verify(dependencies)
        pygments = Path(old['pygments_module'])
        assert sha(pygments) == old['pygments_module_sha256']
        profiles, sources = source_inventory()
        pins = dict(dependencies)
        pins.update({str(OLD_FIXTURE): OLD_SHA, str(pygments): sha(pygments),
                     str(CLOJURE / 'profiles.json'): sha(CLOJURE / 'profiles.json')})
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
        assert len(EXPECTED_IDS) == len(set(EXPECTED_IDS)) == 29
        assert 'foreign_clojure' not in sys.modules and 'test_foreign_clojure' not in sys.modules
        sys.dont_write_bytecode = True
        sys.path.insert(0, str(pygments.parent.parent))
        sys.path.insert(0, str(OLD_FIXTURE.parent / 'snapshot'))
        sys.path.insert(0, str(snapshot))
        normalizer = importlib.import_module('foreign_clojure')
        tests = importlib.import_module('test_foreign_clojure')
        assert Path(normalizer.__file__).resolve() == (snapshot / 'foreign_clojure.py').resolve()
        assert Path(tests.__file__).resolve() == (snapshot / 'test_foreign_clojure.py').resolve()
        receipt = dict(status='failed-fixtures', started_at=now(), expected_test_ids=EXPECTED_IDS,
            actual_test_ids=[], tests_run=0, discovered_tests=0, successful=False,
            common_timing_mutex=str(LOCK), exclusive_mutex_held=True, common_mutex_fd=lock.fileno(),
            python_version=sys.version, pygments_module=str(pygments), pygments_module_sha256=sha(pygments),
            python_executable=str(python), python_executable_sha256=pins[str(python)],
            python_unittest_dependency_pins=python_dependency_pins, unittest_module_identities=unittest_identities(),
            frozen_dependency_pins=pins, original_qualified_fixture=dict(path=str(OLD_FIXTURE), sha256=OLD_SHA),
            actual_native_subjects=0, actual_signals_sent=0, canonical_LOC_admitted=False, chart_admission=False)
        try:
            suite = unittest.defaultTestLoader.loadTestsFromModule(tests)
            receipt['discovered_tests'] = suite.countTestCases()
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
            assert receipt['tests_run'] == receipt['discovered_tests'] == 29
            assert len(receipt['actual_test_ids']) == len(set(receipt['actual_test_ids'])) == 29
            assert sorted(receipt['actual_test_ids']) == EXPECTED_IDS and receipt['successful'] is True
            for name in ('failures', 'errors', 'skipped', 'expected_failures', 'unexpected_successes'):
                assert receipt[name] == []
            receipt['unittest_module_identities'] = unittest_identities()
            verify(pins)
            receipt.update(status='passed', finished_at=now())
        except BaseException as error:
            receipt.update(status='failed-fixtures', finished_at=now(), error=str(error) or type(error).__name__, error_type=type(error).__name__)
            raise
        finally:
            new_json(output / 'fixture-receipt.json', receipt)
        fixture_path = output / 'fixture-receipt.json'
        pins[str(fixture_path)] = sha(fixture_path)
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
            full_performance_runs=0, chart_admission=False, canonical_LOC_admitted=False,
            producer_LOC_mutated=False, parser_pass_is_not_semantic_certification=True))
        print(json.dumps(dict(fixture=str(fixture_path), fixture_sha256=sha(fixture_path),
            replay=str(replay), replay_sha256=sha(replay), actual_fixture_methods=29,
            actual_unique_sources=21, actual_profiles=20, unresolved=len(unresolved), canonical_LOC_admitted=False)), flush=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--revision', required=True)
    args = parser.parse_args()
    assert re.fullmatch(r'[a-z0-9][a-z0-9-]{0,63}', args.revision)
    run(args.revision)


if __name__ == '__main__':
    main()
