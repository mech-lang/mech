"""Source-authored fixtures; execution requires the owner's shared lock queue."""
from copy import deepcopy
import unittest

from endpoints import finalize_endpoints


OVERLAY_FIELDS = frozenset(('formatted_loc', 'formatted_loc_status', 'formatted_loc_complete',
                            'formatted_loc_evidence', 'formatted_x'))


def payload():
    data = dict(languages=[dict(key=key, label=key) for key in ('old', 'new')],
                benchmarks=[], programs={}, files={}, reference_summary=[])
    for benchmark in ('first', 'second'):
        groups = []
        for language in ('old', 'new'):
            keys = []
            for form, loc, seconds, formatted_loc in (
                    ('compact', 10, 8.0, 50), ('optimized', 30, 2.0, 80), ('other', 40, 3.0, 5)):
                key = ':'.join((benchmark, language, form))
                identity = key + '-source'
                data['files'][identity] = dict(code='original source bytes for ' + key,
                                              role='application', existing_annotation='preserve')
                data['programs'][key] = dict(key=key, id=key, language=language, benchmark=benchmark,
                    loc=loc, seconds=seconds, samples=[seconds - .1, seconds, seconds + .1],
                    x=loc / 10, y=seconds / 2,
                    files=[identity], runtime_policy='preserved actual runtime',
                    sample_execution_metadata=['original-one', 'original-two', 'original-three'],
                    formatted_loc=formatted_loc, formatted_loc_status='qualified',
                    formatted_loc_complete=True,
                    formatted_loc_evidence=dict(receipt=key + '-qualified-fixture'))
                keys.append(key)
            groups.append(dict(language=language, label=language, programs=keys))
        data['benchmarks'].append(dict(key=benchmark, groups=groups, baseline_loc=10, baseline_seconds=2.0))
    data['interpreter_integration_audit'] = dict(selected_programs=list(data['programs']),
                                               plotted_programs=[])
    return data


def unavailable(row, status='unavailable'):
    row.update(formatted_loc=None, formatted_loc_status=status, formatted_loc_complete=False,
               formatted_loc_evidence=dict(reason='fixture status ' + status))


class FormattedEndpoints(unittest.TestCase):
    def test_full_inventory_survives_endpoint_pruning_with_source_closure(self):
        data = payload()
        data['program_inventory'] = deepcopy(data['programs'])
        for benchmark in data['benchmarks']:
            for group in benchmark['groups']:
                stem = benchmark['key']+':'+group['language']+':'
                group['programs'] = [stem+'optimized', stem+'compact']
                del data['programs'][stem+'other']
        for pool in (data['programs'], data['program_inventory']):
            for row in pool.values(): unavailable(row, 'pending')
        data['program_inventory']['first:new:other']['inventory_note'] = 'preserve'
        before = deepcopy(data)
        inventory_map = data['program_inventory']
        result = finalize_endpoints(data)
        self.assertEqual(result['retained_programs'], 8)
        self.assertEqual(result['retained_sources'], 12)
        self.assertIs(data['program_inventory'], inventory_map)
        self.assertEqual(data['files'], before['files'])
        self.assertEqual(len(data['program_inventory']), 12)
        for key, row in data['program_inventory'].items():
            self.assertTrue(all(identity in data['files'] for identity in row['files']))
            self.assertIsNone(row['formatted_loc']); self.assertIsNone(row['formatted_x'])
            self.assertEqual(row['seconds'], before['program_inventory'][key]['seconds'])
            self.assertEqual(row['samples'], before['program_inventory'][key]['samples'])
            if key in data['programs']:
                for field in OVERLAY_FIELDS:
                    self.assertEqual(row[field], data['programs'][key][field])
        self.assertEqual(data['program_inventory']['first:new:other']['inventory_note'], 'preserve')
        # A formatter-only winner must still be recovered from the full
        # inventory when it was absent from the already pruned chart map.
        promoted = payload(); promoted['program_inventory'] = deepcopy(promoted['programs'])
        del promoted['programs']['first:new:other']
        promoted['benchmarks'][0]['groups'][1]['programs'].remove('first:new:other')
        finalize_endpoints(promoted)
        self.assertEqual(promoted['benchmarks'][0]['groups'][1]['formatted_small'], 'first:new:other')
        self.assertIn('first:new:other', promoted['programs'])
        self.assertEqual(promoted['programs']['first:new:other']['formatted_x'], promoted['program_inventory']['first:new:other']['formatted_x'])

    def test_inventory_disagreements_and_excluded_rows_reject_transactionally(self):
        for corruption in ('time', 'source', 'canonical', 'excluded', 'unknown-group', 'missing-file'):
            with self.subTest(corruption=corruption):
                data = payload(); data['program_inventory'] = deepcopy(data['programs'])
                row = data['program_inventory']['first:new:other']
                if corruption == 'time': row['seconds'] = 99.0
                elif corruption == 'source': row['files'] = ['first:new:compact-source']
                elif corruption == 'canonical': row['x'] = 99.0
                elif corruption == 'excluded': row['language'] = 'V'
                elif corruption == 'unknown-group':
                    del data['programs']['first:new:other']
                    data['benchmarks'][0]['groups'][1]['programs'].remove('first:new:other')
                    row['language'] = 'unregistered'
                else:
                    del data['programs']['first:new:other']
                    data['benchmarks'][0]['groups'][1]['programs'].remove('first:new:other')
                    del data['files']['first:new:other-source']
                before = deepcopy(data)
                with self.assertRaises(ValueError): finalize_endpoints(data)
                self.assertEqual(data, before)

    def test_third_source_is_promoted_before_pruning_with_unchanged_canonical_baselines(self):
        data = payload()
        before = deepcopy(data)
        maps = {key: data[key] for key in ('programs', 'files', 'languages', 'benchmarks')}
        result = finalize_endpoints(data)
        self.assertEqual(result, dict(retained_programs=12, retained_sources=12, complete_timed_suites=2,
            complete_formatted_timed_suites=2, formatted_endpoint_programs=8,
            V_visualization_entries=0, Red_visualization_entries=0))
        for key, identity in maps.items():
            self.assertIs(data[key], identity)
        for benchmark in data['benchmarks']:
            self.assertEqual((benchmark['baseline_loc'], benchmark['baseline_seconds'],
                              benchmark['baseline_formatted_loc']), (10, 2.0, 5))
            for group in benchmark['groups']:
                stem = benchmark['key'] + ':' + group['language'] + ':'
                self.assertEqual((group['fast'], group['small']), (stem + 'optimized', stem + 'compact'))
                self.assertEqual((group['formatted_fast'], group['formatted_small']),
                                 (stem + 'optimized', stem + 'other'))
                self.assertEqual(group['programs'], [stem + 'optimized', stem + 'compact', stem + 'other'])
                third = data['programs'][stem + 'other']
                self.assertEqual((third['x'], third['y'], third['formatted_x']), (4.0, 1.5, 1.0))
        for key, row in data['programs'].items():
            self.assertEqual({k: v for k, v in row.items() if k != 'formatted_x'},
                             before['programs'][key])
        self.assertEqual(data['files'], before['files'])
        self.assertEqual(data['interpreter_integration_audit']['plotted_programs'], sorted(data['programs']))
        for entry in data['reference_summary']:
            for kind, expected in (('fast', dict(x=3.0, y=1.0)), ('small', dict(x=1.0, y=4.0))):
                for axis, value in expected.items():
                    self.assertAlmostEqual(entry[kind][axis], value)
        for entry in data['formatted_reference_summary']:
            for kind, expected in (('fast', dict(x=16.0, y=1.0)), ('small', dict(x=1.0, y=1.5))):
                for axis, value in expected.items():
                    self.assertAlmostEqual(entry[kind][axis], value)

    def test_absent_and_unavailable_fields_keep_the_canonical_pool_with_nulls(self):
        for mode in ('absent', 'unavailable', 'pending', 'failed'):
            with self.subTest(mode=mode):
                data = payload()
                for row in data['programs'].values():
                    if mode == 'absent':
                        for field in OVERLAY_FIELDS:
                            row.pop(field, None)
                    else:
                        unavailable(row, mode)
                result = finalize_endpoints(data)
                self.assertEqual(result['retained_programs'], 8)
                self.assertEqual(result['complete_formatted_timed_suites'], 0)
                self.assertEqual(result['formatted_endpoint_programs'], 0)
                self.assertEqual(data['formatted_reference_summary'], [])
                for benchmark in data['benchmarks']:
                    self.assertEqual((benchmark['baseline_loc'], benchmark['baseline_seconds']), (10, 2.0))
                    self.assertIsNone(benchmark['baseline_formatted_loc'])
                    for group in benchmark['groups']:
                        self.assertEqual(group['programs'], [group['fast'], group['small']])
                        self.assertIsNone(group['formatted_fast'])
                        self.assertIsNone(group['formatted_small'])
                for row in data['programs'].values():
                    self.assertIsNone(row['formatted_loc'])
                    self.assertIsNone(row['formatted_x'])
                    self.assertFalse(row['formatted_loc_complete'])

    def test_formatted_baseline_uses_only_complete_qualified_sources(self):
        data = payload()
        for row in data['programs'].values():
            unavailable(row)
        data['programs']['first:new:optimized'].update(formatted_loc=80, formatted_loc_status='qualified',
                                                     formatted_loc_complete=True)
        finalize_endpoints(data)
        first, second = data['benchmarks']
        self.assertEqual((first['baseline_loc'], first['baseline_seconds'], first['baseline_formatted_loc']),
                         (10, 2.0, 80))
        self.assertIsNone(second['baseline_formatted_loc'])
        self.assertEqual(data['programs']['first:new:optimized']['formatted_x'], 1.0)
        self.assertIsNone(data['programs']['first:new:compact']['formatted_x'])
        self.assertEqual(data['formatted_reference_summary'], [])

    def test_partial_file_receipt_has_no_count_and_does_not_supply_a_formatted_baseline(self):
        data = payload()
        row = data['programs']['second:new:other']
        unavailable(row, 'pending')
        row['formatted_loc_evidence'] = dict(files=[dict(status='qualified', loc=2), dict(status='pending', loc=None)])
        finalize_endpoints(data)
        group = data['benchmarks'][1]['groups'][1]
        self.assertEqual(group['formatted_small'], 'second:new:compact')
        self.assertNotIn('second:new:other', data['programs'])
        self.assertEqual(data['benchmarks'][1]['baseline_formatted_loc'], 5)

    def test_formatted_fast_can_differ_without_changing_original_fast_or_time(self):
        data = payload()
        unavailable(data['programs']['first:new:optimized'])
        finalize_endpoints(data)
        group = data['benchmarks'][0]['groups'][1]
        self.assertEqual(group['fast'], 'first:new:optimized')
        self.assertEqual(group['formatted_fast'], 'first:new:other')
        self.assertEqual(group['formatted_small'], 'first:new:other')
        self.assertEqual(data['benchmarks'][0]['baseline_seconds'], 2.0)
        self.assertEqual(data['programs']['first:new:optimized']['seconds'], 2.0)
        self.assertIsNone(data['programs']['first:new:optimized']['formatted_x'])

    def test_source_only_formatted_small_has_no_invented_timing_or_complete_suite(self):
        data = payload()
        row = data['programs']['second:new:other']
        row.update(seconds=None, samples=[], y=None)
        finalize_endpoints(data)
        self.assertEqual(data['benchmarks'][1]['groups'][1]['formatted_small'], 'second:new:other')
        self.assertIsNone(data['programs']['second:new:other']['seconds'])
        self.assertIsNone(data['programs']['second:new:other']['y'])
        self.assertEqual(data['programs']['second:new:other']['formatted_x'], 1.0)
        self.assertEqual([entry['lang'] for entry in data['reference_summary']], ['old', 'new'])
        self.assertEqual([entry['lang'] for entry in data['formatted_reference_summary']], ['old'])

    def test_stable_formatted_ties_and_single_source_union(self):
        data = payload()
        row = data['programs']['first:new:compact']
        row.update(seconds=1.0, samples=[.9, 1.0, 1.1], y=.5, formatted_loc=1)
        finalize_endpoints(data)
        self.assertEqual(data['benchmarks'][0]['groups'][1]['programs'], ['first:new:compact'])
        data = payload()
        data['programs']['first:new:other'].update(seconds=2.0, samples=[1.9, 2.0, 2.1], y=1.0, formatted_loc=80)
        finalize_endpoints(data)
        group = data['benchmarks'][0]['groups'][1]
        self.assertEqual(group['fast'], 'first:new:optimized')
        self.assertEqual(group['formatted_fast'], 'first:new:optimized')

    def test_late_corrupt_formatted_record_rejects_the_whole_transaction(self):
        for mutation in ('bool', 'zero', 'float', 'negative', 'null-qualified', 'incomplete',
                         'nonbool-complete', 'numeric-pending', 'complete-failed', 'unknown-status', 'bad-evidence'):
            with self.subTest(mutation=mutation):
                data = payload()
                row = data['programs']['second:new:other']
                if mutation == 'bool': row['formatted_loc'] = True
                elif mutation == 'zero': row['formatted_loc'] = 0
                elif mutation == 'float': row['formatted_loc'] = 5.0
                elif mutation == 'negative': row['formatted_loc'] = -5
                elif mutation == 'null-qualified': row['formatted_loc'] = None
                elif mutation == 'incomplete': row['formatted_loc_complete'] = False
                elif mutation == 'nonbool-complete': row['formatted_loc_complete'] = 1
                elif mutation == 'numeric-pending': row.update(formatted_loc_status='pending', formatted_loc_complete=False)
                elif mutation == 'complete-failed': row.update(formatted_loc_status='failed', formatted_loc=None)
                elif mutation == 'unknown-status': row['formatted_loc_status'] = 'partial'
                else: row['formatted_loc_evidence'] = []
                before = deepcopy(data)
                with self.assertRaises(ValueError):
                    finalize_endpoints(data)
                self.assertEqual(data, before)

    def test_late_canonical_failure_still_rejects_the_whole_transaction(self):
        for mutation in ('foreign', 'missing', 'duplicate', 'bool-time', 'zero-loc', 'source',
                         'few-samples', 'partial', 'failed', 'median', 'bad-sample'):
            with self.subTest(mutation=mutation):
                data = payload()
                key = 'second:new:other'
                if mutation == 'foreign': data['programs'][key]['benchmark'] = 'first'
                elif mutation == 'missing': del data['programs'][key]
                elif mutation == 'duplicate': data['benchmarks'][-1]['groups'][-1]['programs'].append(key)
                elif mutation == 'bool-time': data['programs'][key]['seconds'] = True
                elif mutation == 'zero-loc': data['programs'][key]['loc'] = 0
                elif mutation == 'source': data['programs'][key]['files'] = ['absent']
                elif mutation == 'few-samples': data['programs'][key]['samples'] = [3.0, 3.1]
                elif mutation == 'partial': data['programs'][key]['measurement_status'] = 'partial'
                elif mutation == 'failed': data['programs'][key]['measurement_status'] = 'failed-full-source-unscored'
                elif mutation == 'median': data['programs'][key]['seconds'] = 9.0
                else: data['programs'][key]['samples'][0] = float('nan')
                before = deepcopy(data)
                with self.assertRaises(ValueError):
                    finalize_endpoints(data)
                self.assertEqual(data, before)

    def test_v_and_red_cannot_become_visualized_candidates(self):
        for language in ('v', 'V', 'red', 'Red'):
            with self.subTest(language=language):
                data = payload()
                data['programs']['second:new:other']['language'] = language
                before = deepcopy(data)
                with self.assertRaises(ValueError):
                    finalize_endpoints(data)
                self.assertEqual(data, before)

    def test_formatted_selection_preserves_saved_canonical_baselines_and_coordinates(self):
        data = payload()
        for benchmark in data['benchmarks']:
            # The saved cohort can have a historical baseline below every
            # currently retained candidate. Formatter selection cannot move it.
            benchmark.update(baseline_loc=2, baseline_seconds=.5)
        for row in data['programs'].values():
            row.update(x=row['loc'] / 2, y=row['seconds'] / .5)
        before = deepcopy(data)
        finalize_endpoints(data)
        for benchmark in data['benchmarks']:
            self.assertEqual((benchmark['baseline_loc'], benchmark['baseline_seconds']), (2, .5))
            self.assertEqual(benchmark['baseline_formatted_loc'], 5)
        for key, row in data['programs'].items():
            self.assertEqual((row['x'], row['y']), (before['programs'][key]['x'], before['programs'][key]['y']))
        self.assertEqual(data['programs']['first:new:other']['formatted_x'], 1.0)


if __name__ == '__main__':
    unittest.main()
