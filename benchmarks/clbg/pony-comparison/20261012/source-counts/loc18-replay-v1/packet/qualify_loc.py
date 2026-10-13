"""Unexecuted additive inventory/replay/overlay driver; all work uses common lock."""
import argparse
import fcntl
import json
import os
from pathlib import Path
import sys
import traceback

HERE = Path(__file__).resolve().parent
COHORT = HERE.parents[1]
LOC = COHORT.parents[1] / 'loc-normalization/20261012'
LOCK = Path('/tmp/mech-clbg-timing.lock')
LOC18 = LOC / 'jobs/boundary-audit-18/fixture-receipt.json'
LOC18_SHA = 'b00afe351c40fb38e03a99ec81fe3d71bd69a82726f337aa87dfb40cfcaa6b6a'
FIXTURE_IDS = tuple('test_loc_inputs.InventoryAndOverlay.' + name for name in (
    'test_package_records_keep_both_streams_and_reject_duplicates',
    'test_entire_loaded_authored_package_is_inventoried',
    'test_available_but_unloaded_search_package_does_not_enter_bundle',
    'test_unpinned_extra_authored_file_stops_inventory',
    'test_unknown_loaded_package_is_never_a_vendor_exception',
    'test_changed_build_or_binary_identity_and_mutex_gate_stop',
    'test_diagnostic_disposition_cannot_override_or_waive_an_unknown_owner',
    'test_partial_cohort_and_boolean_counts_cannot_create_overlay',
))


def new(path, value):
    with Path(path).open('x') as stream:
        json.dump(value, stream, indent=2); stream.write('\n')


def run(args):
    print('WAIT shared mutex: Pony ' + args.stage + ' source evidence only', flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        import hashlib
        import importlib
        import sysconfig
        def sha(path): return hashlib.sha256(Path(path).read_bytes()).hexdigest()
        def verify(pins):
            for path, digest in pins.items():
                assert sha(path) == digest, ('Pinned source/evidence changed', path)
        def check_file(path, pins):
            path = Path(path).resolve(strict=True)
            digest = pins[str(path)]
            assert sha(path) == digest
            return digest
        def document(path, digest):
            path = Path(path).resolve(strict=True)
            assert sha(path) == digest
            return json.loads(path.read_text()), dict(path=str(path), sha256=digest)
        descriptor, named = os.fstat(lock.fileno()), LOCK.stat()
        assert (descriptor.st_dev, descriptor.st_ino) == (named.st_dev, named.st_ino)
        output = args.output.resolve()
        assert not output.exists(), 'Preserve every prior proposal and failed receipt'
        output.mkdir(parents=True)
        state = dict(status='running-source-only', stage=args.stage, actual_common_mutex_held=True,
                     native_processes=0, canonical_LOC_admitted=False, chart_admission=False)
        try:
            assert sys.version_info[:2] == (3, 14)
            packet = sorted(set(HERE.glob('*.py')) | set(HERE.glob('*.md')))
            pins = {str(path): sha(path) for path in packet}
            python_executable = Path(sys.executable).resolve(strict=True)
            unittest_directory = (Path(sysconfig.get_path('stdlib')) / 'unittest').resolve(strict=True)
            runtime_paths = [python_executable] + sorted(unittest_directory.rglob('*.py'))
            runtime_pins = {str(path): sha(path) for path in runtime_paths}
            pins.update(runtime_pins)
            assert not any(name == 'unittest' or name.startswith('unittest.') for name in sys.modules)
            import unittest
            assert Path(unittest.__file__).resolve().parent == unittest_directory
            for name, module in list(sys.modules.items()):
                if name == 'unittest' or name.startswith('unittest.'):
                    loaded = Path(module.__file__).resolve(strict=True)
                    assert loaded.is_relative_to(unittest_directory) and str(loaded) in runtime_pins
            verify(runtime_pins)
            state.update(python_executable=str(python_executable), python_version=sys.version,
                         runtime_pins=runtime_pins, unittest_loaded_module_identities={
                             name: str(Path(module.__file__).resolve()) for name, module in sys.modules.items()
                             if name == 'unittest' or name.startswith('unittest.')})
            frozen = output / 'packet'; frozen.mkdir()
            for path in packet:
                target = frozen / path.name; target.write_bytes(path.read_bytes()); target.chmod(0o444)
                assert sha(path) == sha(target); pins[str(target)] = sha(target)
            prior_inventory = prior_inventory_evidence = None
            if args.stage != 'inventory':
                prior_inventory, prior_inventory_evidence = document(args.inventory, args.inventory_sha256)
                assert prior_inventory['status'] == 'compiler-input-proposals-awaiting-role-review'
                verify(prior_inventory['pins'])
                for path in packet:
                    assert pins[str(path)] == prior_inventory['pins'][str(path)], 'Later stages require exact previously qualified packet bytes'
                pins.update(prior_inventory['pins'])
                pins[prior_inventory_evidence['path']] = prior_inventory_evidence['sha256']
            sys.path.insert(0, str(frozen))
            for name in ('input_inventory', 'overlay_candidate', 'test_loc_inputs'):
                assert name not in sys.modules, 'Do not certify an already cached mutable module'
            inventory_module = importlib.import_module('input_inventory')
            overlay_module = importlib.import_module('overlay_candidate')
            for name in ('input_inventory', 'overlay_candidate'):
                assert Path(sys.modules[name].__file__).resolve() == frozen / (name + '.py')
            verify(pins)
            packet_fixture_evidence = None
            if args.stage == 'inventory':
                tests = importlib.import_module('test_loc_inputs')
                assert Path(tests.__file__).resolve() == frozen / 'test_loc_inputs.py'
                suite = unittest.defaultTestLoader.loadTestsFromModule(tests)
                def ids(value):
                    for test in value:
                        if isinstance(test, unittest.TestSuite): yield from ids(test)
                        else: yield test.id()
                discovered = list(ids(suite))
                assert sorted(discovered) == sorted(FIXTURE_IDS) and suite.countTestCases() == 8
                class Started(unittest.TextTestResult):
                    def __init__(self, *values, **fields):
                        super().__init__(*values, **fields); self.ids = []
                    def startTest(self, test): self.ids.append(test.id()); super().startTest(test)
                with (output / 'fixtures.log').open('x') as log:
                    result = unittest.TextTestRunner(stream=log, verbosity=2, resultclass=Started).run(suite)
                fixture = dict(status='passed' if result.wasSuccessful() else 'failed', expected_ids=FIXTURE_IDS,
                    discovered_ids=discovered, started_ids=result.ids, tests_run=result.testsRun,
                    failures=len(result.failures), errors=len(result.errors), skipped=len(result.skipped),
                    expected_failures=len(result.expectedFailures), unexpected_successes=len(result.unexpectedSuccesses),
                    actual_common_mutex_held=True, native_processes=0, chart_admission=False)
                fixture.update(python_executable=str(python_executable), python_version=sys.version,
                               runtime_pins=runtime_pins, unittest_loaded_module_identities=state['unittest_loaded_module_identities'])
                new(output / 'fixture-receipt.json', fixture)
                assert result.testsRun == len(result.ids) == len(set(result.ids)) == 8
                assert sorted(result.ids) == sorted(FIXTURE_IDS) and result.wasSuccessful()
                assert not any((result.failures, result.errors, result.skipped, result.expectedFailures, result.unexpectedSuccesses))
                pins[str(output / 'fixtures.log')] = sha(output / 'fixtures.log')
                pins[str(output / 'fixture-receipt.json')] = sha(output / 'fixture-receipt.json')
                packet_fixture_evidence = dict(path=str(output / 'fixture-receipt.json'), sha256=pins[str(output / 'fixture-receipt.json')])
                verify(pins)
            if args.stage == 'inventory':
                report, report_evidence = document(args.qualification_report, args.qualification_sha256)
                assert report['status'] == 'bounded-output-primitives-passed-awaiting-work-worker-and-LOC-review'
                assert report['fresh_unscored_primitive_cases'] == 1 and report['reused_bounded_cases'] == 154
                assert len(report['profiles']) == 20
                verify(report['pins']); pins.update(report['pins'])
                pins[report_evidence['path']] = report_evidence['sha256']
                reuse_path = Path(report['prior_proof_reuse'])
                reuse, reuse_evidence = document(reuse_path, pins[str(reuse_path)])
                assert reuse['status'] == 'verified-exact-old-bounded-proof' and reuse['reused_bounded_cases'] == 154
                assert {row['id'] for row in report['profiles']} == set(reuse['profiles'])
                verify(reuse['pins']); pins.update(reuse['pins'])
                rows = []
                for row in report['profiles']:
                    receipt_path = Path(row['production_build_receipt'])
                    build, build_evidence = document(receipt_path, pins[str(receipt_path)])
                    stdout, stderr = receipt_path.with_suffix('.stdout'), receipt_path.with_suffix('.stderr')
                    check_file(stdout, pins); check_file(stderr, pins)
                    item = inventory_module.inventory_one(row, reuse['profiles'][row['id']], build,
                        stdout.read_text(), stderr.read_text(), pins, check_file)
                    item['actual_build_evidence'] = build_evidence
                    item['actual_build_streams'] = [{str(path): pins[str(path)]} for path in (stdout, stderr)]
                    rows.append(item)
                assert {(row['benchmark'], row['variant']) for row in rows} == overlay_module.EXPECTED
                verify(pins)
                new(output / 'compiler-input-proposals.json', dict(status='compiler-input-proposals-awaiting-role-review',
                    qualification=report_evidence, reuse=reuse_evidence, profiles=rows, profile_count=20,
                    packet_fixture_receipt=packet_fixture_evidence,
                    unresolved=[issue for row in rows for issue in row['unresolved']], pins=pins,
                    actual_common_mutex_held=True, native_cases_reexecuted=0,
                    canonical_LOC_admitted=False, chart_admission=False))
            else:
                inventory, inventory_evidence = prior_inventory, prior_inventory_evidence
                assert not inventory['unresolved'] and len(inventory['profiles']) == 20
                packet_fixture_evidence = inventory['packet_fixture_receipt']
                fixture, actual_fixture = document(packet_fixture_evidence['path'], packet_fixture_evidence['sha256'])
                assert actual_fixture == packet_fixture_evidence
                assert fixture['status'] == 'passed' and fixture['actual_common_mutex_held'] is True
                assert fixture['tests_run'] == 8
                assert sorted(fixture['expected_ids']) == sorted(fixture['discovered_ids']) == sorted(fixture['started_ids']) == sorted(FIXTURE_IDS)
                assert len(fixture['started_ids']) == len(set(fixture['started_ids'])) == 8
                assert all(fixture[field] == 0 for field in ('failures', 'errors', 'skipped', 'expected_failures', 'unexpected_successes'))
                if args.stage == 'replay':
                    old, old_evidence = document(LOC18, LOC18_SHA)
                    assert old['status'] == 'passed' and old['tests_run'] == 672
                    assert old['failures'] == old['errors'] == 0 and old['exclusive_mutex_held'] is True
                    assert old['common_timing_mutex'] == str(LOCK)
                    assert old['python_version'].split()[0] == sys.version.split()[0] and old['pygments_version'] == '2.19.2'
                    grammar = old['frozen_dependency_pins']; verify(grammar); pins.update(grammar)
                    grammar_directory = output / 'grammar-snapshot'; grammar_directory.mkdir()
                    imported_grammar = dict(grammar)
                    original_grammar_directory = LOC18.parent / 'snapshot'
                    for path, digest in grammar.items():
                        original = Path(path)
                        assert original.is_relative_to(original_grammar_directory)
                        target = grammar_directory / original.relative_to(original_grammar_directory)
                        target.parent.mkdir(parents=True, exist_ok=True)
                        target.write_bytes(original.read_bytes()); target.chmod(0o444)
                        assert sha(target) == digest
                        imported_grammar[str(target)] = digest
                    pins.update(imported_grammar)
                    pins[old_evidence['path']] = old_evidence['sha256']
                    pygments_path = Path(old['pygments_module'])
                    assert sha(pygments_path) == old['pygments_module_sha256']
                    pins[str(pygments_path)] = old['pygments_module_sha256']
                    lexical_pins = {str(path): sha(path) for path in pygments_path.parent.rglob('*.py')}
                    pins.update(lexical_pins)
                    imported_grammar.update(lexical_pins)
                    assert 'foreign_pony' not in sys.modules and 'foreign_core' not in sys.modules
                    assert 'pygments' not in sys.modules, 'Lexical dependency must load only after pin review'
                    sys.path.insert(0, str(pygments_path.parent.parent))
                    sys.path.insert(0, str(grammar_directory))
                    normalizer = importlib.import_module('foreign_pony')
                    assert Path(normalizer.__file__).resolve() == grammar_directory / 'foreign_pony.py'
                    for module_name in ('foreign_core', 'foreign_future_mask', 'canonical'):
                        assert Path(sys.modules[module_name].__file__).resolve() == grammar_directory / (module_name + '.py')
                    assert Path(sys.modules['pygments'].__file__).resolve() == pygments_path.resolve()
                    assert sys.modules['pygments'].__version__ == '2.19.2'
                    verify(imported_grammar); verify(lexical_pins)
                    sources = {}; rows = {}
                    for row in inventory['profiles']:
                        bundle = []
                        for item in row['files']:
                            assert item['source_language'] == 'pony'
                            check_file(item['path'], pins)
                            if item['path'] not in sources:
                                text = Path(item['path']).read_text()
                                metric = normalizer.normalize(text, 'pony')
                                assert metric['eligible_for_chart'] is False
                                sources[item['path']] = dict(source_sha256=item['sha256'], source_character_length=len(text), metric=metric)
                            bundle.append({key: item[key] for key in ('path', 'sha256', 'role')})
                        rows[row['id']] = dict(id=row['id'], source_sha256=row['source_sha256'], binary_sha256=row['binary_sha256'],
                            source_bundle=bundle, producer_loc=row['producer_loc'])
                    verify(pins)
                    new(output / 'source-count-proposals.json', dict(status='actual-source-count-proposals-awaiting-selected-source-review',
                        inventory=inventory_evidence, source_counts=sources, profiles=rows,
                        normalization_policy=dict(rule_version='canonical-source-units-20261012-approved-rule-v2',
                            grammar=normalizer.PROPOSAL_VERSION,
                            evidence_path=str(grammar_directory / 'foreign_core.py'),
                            evidence_sha256=imported_grammar[str(grammar_directory / 'foreign_core.py')]),
                        fixture_receipt=dict(old_evidence, status='passed'), grammar_snapshot_pins=imported_grammar,
                        packet_fixture_receipt=packet_fixture_evidence,
                        lexical_dependency_pins=lexical_pins,
                        unresolved=[dict(source=path, diagnostic=issue) for path, value in sources.items() for issue in value['metric']['diagnostics']],
                        pins=pins, actual_common_mutex_held=True, canonical_LOC_admitted=False, chart_admission=False))
                else:
                    replay, replay_evidence = document(args.replay, args.replay_sha256)
                    assert replay['inventory'] == inventory_evidence
                    verify(replay['pins']); pins.update(replay['pins'])
                    pins[replay_evidence['path']] = replay_evidence['sha256']
                    review, review_evidence = document(args.semantic_review, args.semantic_review_sha256)
                    pins[review_evidence['path']] = review_evidence['sha256']
                    verify(review['pins']); pins.update(review['pins'])
                    replay['self_evidence'] = replay_evidence
                    review_reference = dict(evidence_path=review_evidence['path'], evidence_sha256=review_evidence['sha256'])
                    payload = overlay_module.construct(inventory, replay, review, inventory_evidence, review_reference)
                    qualified_path = output / 'compiler-bound-input-inventory.json'
                    new(qualified_path, dict(status='qualified-exact-authored-compiler-inputs', language='pony',
                        profile_count=20, original_compiler_inventory=inventory_evidence,
                        source_role_review=review_evidence, profiles=payload['qualified_inputs'], pins=pins))
                    pins[str(qualified_path)] = sha(qualified_path)
                    for value in payload['qualified_inputs'].values():
                        value['evidence_path'] = str(qualified_path); value['evidence_sha256'] = pins[str(qualified_path)]
                    verify(pins)
                    new(output / 'qualified-loc-overlay.json', dict(schema_version=1, language='pony',
                        status='qualified-exact-source-LOC', profile_count=20, complete_authored_source_bundles=True,
                        unresolved=[], normalization_policy=replay['normalization_policy'],
                        fixture_receipt=replay['fixture_receipt'], replay_receipts=[replay_evidence],
                        grammar_snapshot_pins=replay['grammar_snapshot_pins'],
                        compiler_bound_input_inventory=payload['qualified_inputs'], profiles=payload['profiles'],
                        pins=pins, canonical_LOC_admitted=True, chart_admission=False))
            verify(pins)
            state['status'] = 'completed-source-evidence-awaiting-root-review'
        except BaseException as error:
            state.update(status='failed-stop', error=str(error), error_type=type(error).__name__)
            with (output / 'failure.log').open('x') as stream: stream.write(traceback.format_exc())
            raise
        finally:
            new(output / 'terminal-receipt.json', state)
            available = {str(path): sha(path) for path in output.rglob('*') if path.is_file()}
            new(output / 'terminal-pins.json', dict(pins=available, actual_common_mutex_held=True,
                terminal_status=state['status'], chart_admission=False, automatic_retry=False))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--stage', choices=('inventory', 'replay', 'overlay'), required=True)
    parser.add_argument('--output', type=Path, required=True)
    for flag in ('qualification-report', 'inventory', 'replay', 'semantic-review'):
        parser.add_argument('--' + flag, type=Path)
        parser.add_argument('--' + flag.replace('-report', '') + '-sha256')
    args = parser.parse_args()
    required = {'inventory': ('qualification_report', 'qualification_sha256'),
                'replay': ('inventory', 'inventory_sha256'),
                'overlay': ('inventory', 'inventory_sha256', 'replay', 'replay_sha256', 'semantic_review', 'semantic_review_sha256')}
    for field in required[args.stage]: assert getattr(args, field), ('Required exact evidence', field)
    run(args)


if __name__ == '__main__': main()
