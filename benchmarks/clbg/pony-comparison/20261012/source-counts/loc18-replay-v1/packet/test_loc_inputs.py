"""Eight unexecuted completeness/identity/diagnostic fixtures; no native probes."""
import copy
import hashlib
from pathlib import Path
import tempfile
import unittest
import input_inventory as inputs
import overlay_candidate as overlay


class InventoryAndOverlay(unittest.TestCase):
    def fixture(self, root):
        root = Path(root).resolve()
        build_root = root / '.build/private-v3'
        compiler = build_root / 'release/0.74.0-95ad180/bin/ponyc'
        builtin = compiler.parent.parent / 'packages/builtin'
        regex = build_root / 'regex/regex-589eb008db250aab17d4c073a41c5562ac5b8dc7'
        gmp = build_root / 'gmp4pony/gmp4pony-b14db4ba347f6a2b840f135e1602e19f85625c39'
        snapshot = root / 'qualification/private-v3/source-snapshot'
        source = snapshot / 'sources/pidigits/compact/main.pony'
        helper = snapshot / 'support-v2/ponytest'
        binary = build_root / 'bin/pony-pidigits-compact/pony-pidigits-compact'
        for path, content in ((compiler, 'compiler fixture'), (binary, 'binary fixture'),
                (builtin / 'builtin.pony', 'primitive Builtin'), (regex / 'regex.pony', 'primitive Regex'),
                (gmp / 'gmp.pony', 'primitive Gmp'), (source, 'actor Main'),
                (helper / 'aliases.pony', 'type Alias is U64'), (helper / 'extra.pony', 'primitive Extra')):
            path.parent.mkdir(parents=True, exist_ok=True); path.write_text(content)
        pins = {str(path.resolve()): hashlib.sha256(path.read_bytes()).hexdigest()
                for path in root.rglob('*') if path.is_file()}
        command = [str(compiler), '--path', str(regex), '--path', str(gmp), '--path', str(snapshot / 'support-v2'),
                   '--output', str(binary.parent), '--bin-name', binary.name, str(source.parent)]
        row = dict(id=binary.name, benchmark='pidigits', variant='compact', source=str(source),
                   source_sha256=pins[str(source)], binary_sha256=pins[str(binary)], command=[str(binary), '{n}'],
                   production_build_receipt=str(root / 'qualification/private-v3/builds/pony-pidigits-compact/build.json'), producer_loc=None)
        prior = dict(declaration=dict(id=row['id']), source=row['source'], source_sha256=row['source_sha256'],
                     binary=str(binary), binary_sha256=row['binary_sha256'], original_build_command=command)
        receipt = dict(command=command, exit_status=0, final_exit_status=0, inherited_common_mutex=True)
        stderr = '\n'.join('Building ' + label + ' -> ' + str(path) for label, path in (
            ('builtin', builtin), (str(source.parent), source.parent), ('ponytest', helper)))
        stderr += '\nLinking ' + str(binary)
        def verify(path, supplied):
            digest = supplied[str(Path(path).resolve())]
            assert hashlib.sha256(Path(path).read_bytes()).hexdigest() == digest
            return digest
        return row, prior, receipt, stderr, pins, verify, helper

    def test_package_records_keep_both_streams_and_reject_duplicates(self):
        rows = inputs.package_records('Building main -> /fixture/main\n', 'Building builtin -> /fixture/builtin\n')
        self.assertEqual([row['stream'] for row in rows], ['stdout', 'stderr'])
        for stdout, stderr in (('', ''), ('Building bad', ''), ('Building one -> /same', 'Building two -> /same')):
            with self.assertRaises(AssertionError): inputs.package_records(stdout, stderr)

    def test_entire_loaded_authored_package_is_inventoried(self):
        with tempfile.TemporaryDirectory() as root:
            row, prior, build, stderr, pins, verify, helper = self.fixture(root)
            result = inputs.inventory_one(row, prior, build, '', stderr, pins, verify)
            self.assertEqual([item['path'] for item in result['files']], [row['source'], str(helper / 'aliases.pony'), str(helper / 'extra.pony')])
            self.assertEqual([item['role'] for item in result['files']], ['application', 'application-helper', 'application-helper'])
            self.assertFalse(result['source_role_reviewed'])
            self.assertEqual(result['native_cases_reexecuted'], 0)

    def test_available_but_unloaded_search_package_does_not_enter_bundle(self):
        with tempfile.TemporaryDirectory() as root:
            row, prior, build, stderr, pins, verify, helper = self.fixture(root)
            without_helper = '\n'.join(line for line in stderr.splitlines() if not line.startswith('Building ponytest -> '))
            result = inputs.inventory_one(row, prior, build, '', without_helper, pins, verify)
            self.assertEqual([item['path'] for item in result['files']], [row['source']])
            self.assertTrue((helper / 'aliases.pony').exists())

    def test_unpinned_extra_authored_file_stops_inventory(self):
        with tempfile.TemporaryDirectory() as root:
            row, prior, build, stderr, pins, verify, helper = self.fixture(root)
            (helper / 'new-hidden-work.pony').write_text('primitive NewWork')
            with self.assertRaises(KeyError): inputs.inventory_one(row, prior, build, '', stderr, pins, verify)

    def test_unknown_loaded_package_is_never_a_vendor_exception(self):
        with tempfile.TemporaryDirectory() as root:
            row, prior, build, stderr, pins, verify, _ = self.fixture(root)
            unknown = Path(root) / 'unreviewed-package'; unknown.mkdir()
            with self.assertRaisesRegex(AssertionError, 'no qualified source role'):
                inputs.inventory_one(row, prior, build, '', stderr + '\nBuilding injected -> ' + str(unknown), pins, verify)

    def test_changed_build_or_binary_identity_and_mutex_gate_stop(self):
        with tempfile.TemporaryDirectory() as root:
            row, prior, build, stderr, pins, verify, _ = self.fixture(root)
            for field, value in (('inherited_common_mutex', False), ('final_exit_status', 255),
                                 ('cleanup_error', 'failed'), ('cleanup_errors', [{'error': 'failed'}]),
                                 ('remaining_group_members', [123])):
                changed = dict(build, **{field: value})
                with self.assertRaises(AssertionError): inputs.inventory_one(row, prior, changed, '', stderr, pins, verify)
            changed = dict(build, command=build['command'] + ['--unreviewed'])
            with self.assertRaises(AssertionError): inputs.inventory_one(row, prior, changed, '', stderr, pins, verify)
            changed_row = dict(row, binary_sha256='changed')
            with self.assertRaises(AssertionError): inputs.inventory_one(changed_row, prior, build, '', stderr, pins, verify)

    def test_diagnostic_disposition_cannot_override_or_waive_an_unknown_owner(self):
        event = dict(kind='header', start=0, end=17, units=1, scope='pony-type')
        metric = dict(physical_loc=1, normalized_loc=1, canonical_events=[event], expansions=[],
                      diagnostics=[dict(kind='pony-type-alias-review', offset=0)])
        review = dict(status='passed', review_identity='fixture-only', source='/fixture/alias.pony', source_sha256='fixture-pin',
            physical_loc=1, normalized_loc=1, complete_selected_source_read=True, all_written_owners_retained=True,
            literal_data_and_callback_ownership_reviewed=True, actual_owner_events=[event], actual_expansions=[],
            source_character_length=17, diagnostic_dispositions=[dict(actual_diagnostic=metric['diagnostics'][0],
                status='accepted-exact-existing-owner-events', actual_source_span=dict(start=0, end=17),
                owner_event_evidence=[event], review_identity='fixture-only', count_override=False)],
            count_override=False, unresolved=[], evidence_path='/fixture/review', evidence_sha256='fixture-pin')
        self.assertEqual(overlay.reviewed_count(metric, review, review['source'], review['source_sha256'], 17)['status'], 'passed')
        for mutation in ('override', 'unknown-owner', 'different-owner-event', 'missing-disposition'):
            changed = copy.deepcopy(review)
            if mutation == 'override': changed['normalized_loc'] = 4; changed['count_override'] = True
            elif mutation == 'unknown-owner': changed['diagnostic_dispositions'][0]['actual_diagnostic']['kind'] = 'different'
            elif mutation == 'different-owner-event': changed['diagnostic_dispositions'][0]['owner_event_evidence'] = [dict(event, units=3)]
            else: changed['diagnostic_dispositions'] = []
            with self.assertRaises(AssertionError): overlay.reviewed_count(metric, changed, review['source'], review['source_sha256'], 17)
        unrelated = dict(kind='header', start=100, end=117, units=1, scope='pony-type')
        two_owners = dict(metric, physical_loc=2, normalized_loc=2, canonical_events=[event, unrelated])
        unrelated_review = copy.deepcopy(review)
        unrelated_review.update(physical_loc=2, normalized_loc=2, actual_owner_events=[event, unrelated],
                                source_character_length=117)
        unrelated_review['diagnostic_dispositions'][0]['owner_event_evidence'] = [unrelated]
        with self.assertRaisesRegex(AssertionError, 'does not intersect'):
            overlay.reviewed_count(two_owners, unrelated_review, review['source'], review['source_sha256'], 117)

    def test_partial_cohort_and_boolean_counts_cannot_create_overlay(self):
        for value in (True, False, None, -1, 1.5):
            with self.assertRaises(AssertionError): overlay.actual_count(value)
        inventory = dict(status='compiler-input-proposals-awaiting-role-review', profiles=[])
        replay = dict(status='actual-source-count-proposals-awaiting-selected-source-review', self_evidence={})
        review = dict(status='passed-exact-compiler-role-and-selected-source-review', inventory={}, replay={})
        with self.assertRaises(AssertionError): overlay.construct(inventory, replay, review, {}, {})


if __name__ == '__main__': unittest.main()
