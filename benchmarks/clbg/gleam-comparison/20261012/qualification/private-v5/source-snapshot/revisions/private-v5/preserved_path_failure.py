"""Bind the actual v3 nine-method failed receipt and owned fixture process."""
import json
import importlib.util
from pathlib import Path

HERE = Path(__file__).resolve().parents[2]
FAILED = HERE / 'proposal-seals/private-v3'
LOCAL_ID = 'test_guarded_prepare.AdditiveRepair.test_exact_six_grouping_pairs_change_only_the_five_reviewed_sources'


def validate():
    fixture_spec=importlib.util.spec_from_file_location('gleam_retained_path_fixture_inventory',HERE/'revisions/private-v4/fixture_runner.py')
    fixture_runner=importlib.util.module_from_spec(fixture_spec);fixture_spec.loader.exec_module(fixture_runner)
    local = json.loads((FAILED / 'gleam-bundle-inventory.json').read_text())
    generic = json.loads((FAILED / 'generic-ownership-inventory.json').read_text())
    expected = fixture_runner.INVENTORY
    assert len(expected) == 9 and local['expected_test_ids'] == expected
    assert local['status'] == 'failed' and local['successful'] is False
    assert type(local['tests_run']) is type(local['discovered_tests']) is int
    assert local['tests_run'] == local['discovered_tests'] == 9
    assert len(local['actual_test_ids']) == len(set(local['actual_test_ids'])) == 9
    assert set(local['actual_test_ids']) == set(expected)
    assert all(local[key] == [] for key in ('failures','skipped','expected_failures','unexpected_successes'))
    assert len(local['errors']) == 1 and local['errors'][0][0] == LOCAL_ID
    trace = local['errors'][0][1]
    missing = HERE / 'revisions/private-v3/sources/binarytrees/optimized.gleam'
    assert "assert (REVISION/'sources'/relative).read_text()==before" in trace
    assert trace.endswith("FileNotFoundError: [Errno 2] No such file or directory: '" + str(missing) + "'\n")
    assert generic['status'] == 'passed' and generic['successful'] is True
    assert type(generic['tests_run']) is type(generic['discovered_tests']) is int
    assert generic['tests_run'] == generic['discovered_tests'] == 11
    assert len(generic['expected_test_ids']) == len(set(generic['expected_test_ids'])) == 11
    assert len(generic['actual_test_ids']) == len(set(generic['actual_test_ids'])) == 11
    assert set(generic['actual_test_ids']) == set(generic['expected_test_ids'])
    assert all(generic[key] == [] for key in ('errors','failures','skipped','expected_failures','unexpected_successes'))
    for basename, tail, code in (
        ('generic-ownership-fixtures', ['--set','ownership','--receipt',str(FAILED/'generic-ownership-inventory.json')], 0),
        ('gleam-bundle-fixtures', ['--receipt',str(FAILED/'gleam-bundle-inventory.json')], 1)):
        record=json.loads((FAILED/(basename+'.json')).read_text())
        script = HERE.parents[1]/'c3-comparison/20261012/revisions/private-v1/fixture_runner.py' if code == 0 else HERE/'revisions/private-v3/fixture_runner.py'
        assert record['command'][1:] == ['-B',str(script),*tail]
        assert record['inherited_common_mutex'] is True
        assert type(record['exit_status']) is type(record['final_exit_status']) is int
        assert record['exit_status'] == record['final_exit_status'] == code
        assert not any(record.get(key) for key in ('cleanup_error','cleanup_errors','output_drain_error','remaining_group_members'))
        if code:
            assert record['error_type'] == 'AssertionError'
            assert record['error'] == "('Owned command failed', 'gleam-bundle-fixtures', 1)"
            assert trace in (FAILED/(basename+'.stderr')).read_text()
    assert not (HERE/'proposals/private-v3.json').exists()
    assert not (HERE/'qualification/private-v3').exists() and not (HERE/'.build/private-v3').exists()
    return dict(status='preserved-actual-v3-retained-source-path-error',
        actual_started_methods=20,failed_fixture_methods=1,passed_methods=19,
        native_benchmark_processes=0,original_receipts_mutated=False,
        failure_stage='metadata-local-source-lineage-fixture',
        recorded_in_completed_preparation_pins=False,newly_captured_saved_failure_evidence=True)


def preserve(preparation, service, guard):
    value=validate()
    module_spec=importlib.util.spec_from_file_location(
        'gleam_v4_prior_generic_inventory',preparation.SERVICE_FIXTURES)
    module=importlib.util.module_from_spec(module_spec)
    module_spec.loader.exec_module(module)
    module.validate_receipt(json.loads((FAILED/'generic-ownership-inventory.json').read_text()),'ownership')
    value['pins']=service.pin_tree(FAILED)
    guard.verify(value['pins'])
    return value
