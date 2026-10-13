"""Six meaningful unexecuted contract corruption fixtures; no native signals."""
import copy
import json
from pathlib import Path
import unittest
import contracts

HERE = Path(__file__).resolve().parent


class PreparationContracts(unittest.TestCase):
    def test_twenty_profiles_and_explicit_same_source_aliases(self):
        original = json.loads((HERE / 'profiles.json').read_text())
        self.assertEqual(len(contracts.profiles(original)), 20)
        for mutate in (
            lambda rows: rows.pop(),
            lambda rows: rows[0]['support_files'].append('support/hidden_kernel.pl'),
            lambda rows: rows[1].update(source_alias_of=None),
            lambda rows: rows[0].update(maximum_workers=True),
        ):
            changed = copy.deepcopy(original)
            mutate(changed['profiles'])
            with self.assertRaises(AssertionError):
                contracts.profiles(changed)

    def test_compiler_inventory_requires_each_helper_and_no_foreign_authored_root(self):
        main, helper, vendor, tool = map(Path, ('/candidate/main.pl', '/candidate/io.pl',
                                               '/vendor/swi', '/tools/inventory.pl'))
        valid = [str(main), str(helper), str(vendor / 'library/hashtable.pl'), str(tool)]
        self.assertEqual(len(contracts.compiler_inputs(valid, [main, helper], vendor, tool)), 4)
        for invalid in (valid[1:], valid + ['/external/gravity.pl'], valid + [str(main)]):
            with self.assertRaises(AssertionError):
                contracts.compiler_inputs(invalid, [main, helper], vendor, tool)

    def test_actual_runtime_flags_reject_bounded_or_x86_or_disabled_gc(self):
        original = dict(version_components=[10, 0, 2], bounded=False, gc=True,
                        architecture='aarch64-darwin', home='/vendor/swi', stack_limit=8*1024**3)
        contracts.runtime_flags(original, '/vendor/swi')
        for changes in (dict(bounded=True), dict(gc=False), dict(architecture='x86_64-darwin'),
                        dict(version_components=[10, False, 2]), dict(home='/other/swi'),
                        dict(stack_limit=True)):
            with self.assertRaises(AssertionError):
                contracts.runtime_flags(original | changes, '/vendor/swi')

    def test_native_counter_and_workload_boolean_substitution_is_rejected(self):
        original = dict(qualification_checks={gate: True for gate in contracts.GATES},
            qualified=True, full_workload=False, attempt=1, workload_n=0, sampler_wait_status=0,
            stdout_bytes=0, exit_status=0, validation=dict(bytes=0, reference_match=True),
            supervised_native_group=dict(pid=123, pgid=123))
        contracts.complete_attempt(original, False)
        for changes in (dict(sampler_wait_status=False), dict(stdout_bytes=False), dict(attempt=True),
                        dict(full_workload=True), dict(validation=dict(bytes=False, reference_match=True)),
                        dict(supervised_native_group=dict(pid=123, pgid=True))):
            with self.assertRaises(AssertionError):
                contracts.complete_attempt(original | changes, False)

    def test_isolation_requires_exact_twenty_boolean_gates_and_clean_ownership(self):
        attempt = dict(error='Actual output mismatch',
                       qualification_checks={key: True for key in contracts.GATES})
        attempt['qualification_checks']['no_execution_error'] = False
        self.assertTrue(contracts.isolatable(attempt))
        for key, value in (('pins_unchanged', False), ('valid_environment', 1),
                           ('no_remaining_execute_group', False)):
            changed = copy.deepcopy(attempt)
            changed['qualification_checks'][key] = value
            self.assertFalse(contracts.isolatable(changed))
        for key in ('pins_unchanged', 'unknown'):
            changed = copy.deepcopy(attempt)
            if key == 'unknown':
                changed['qualification_checks'][key] = True
            else:
                changed['qualification_checks'].pop(key)
            self.assertFalse(contracts.isolatable(changed))

    def test_full_admission_requires_actual_scored_allocation_not_source_assertion(self):
        row = dict(id='prolog-binarytrees-compact', benchmark='binarytrees', source_sha256='source',
                   binary_sha256='runtime', application_artifact_pins={'main.qlf': 'qlf'})
        proof = dict(status='passed', source_sha256='source', binary_sha256='runtime',
            application_artifact_pins={'main.qlf': 'qlf'}, source_review=dict(status='passed'),
            compiler_bound_input_inventory=dict(status='qualified-exact-compiled-inputs'),
            allocation_retention_witness=dict(status='passed'),
            original_scored_vm_allocation_review=dict(status='passed'))
        receipt = dict(status='qualified-required-source-work', chart_admission=False,
                       language='prolog', profiles={row['id']: proof}, pins={'proof': 'actual'})
        observed = []
        contracts.full_work_receipt(receipt, [row], lambda pins: observed.append(pins))
        self.assertEqual(observed, [{'proof': 'actual'}])
        for changes in (dict(original_scored_vm_allocation_review=dict(status='pending')),
                        dict(application_artifact_pins={'different.qlf': 'qlf'}),
                        dict(allocation_retention_witness=dict(status='source-strategy-only'))):
            changed = copy.deepcopy(receipt)
            changed['profiles'][row['id']].update(changes)
            with self.assertRaises(AssertionError):
                contracts.full_work_receipt(changed, [row], lambda pins: None)
