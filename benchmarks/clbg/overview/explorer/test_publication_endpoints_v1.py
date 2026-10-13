"""Meaningful additive-pool selection and transactional rejection fixtures."""
from copy import deepcopy
import unittest

from publication_endpoints_v1 import finalize_endpoints


def payload():
    data = dict(languages=[dict(key=key, label=key) for key in ('old', 'new')],
                benchmarks=[], programs={}, files={}, reference_summary=[])
    for benchmark in ('first', 'second'):
        groups = []
        for language in ('old', 'new'):
            keys = []
            for form, loc, seconds in (('compact', 10, 8.0), ('optimized', 30, 2.0), ('other', 40, 3.0)):
                key = ':'.join((benchmark, language, form))
                identity = key + '-source'
                data['files'][identity] = dict(code=key, role='application', existing_annotation='preserve')
                data['programs'][key] = dict(key=key, id=key, language=language, benchmark=benchmark,
                    loc=loc, seconds=seconds, samples=[seconds - .1, seconds, seconds + .1],
                    files=[identity], runtime_policy='preserved actual runtime',
                    sample_execution_metadata=['original-one', 'original-two', 'original-three'])
                keys.append(key)
            groups.append(dict(language=language, label=language, programs=keys))
        data['benchmarks'].append(dict(key=benchmark, groups=groups))
    return data


class PublicationEndpoints(unittest.TestCase):
    def test_added_forms_choose_actual_fast_and_small_and_preserve_observations(self):
        data = payload()
        before = deepcopy(data)
        maps = {key: data[key] for key in ('programs', 'files', 'languages', 'benchmarks')}
        result = finalize_endpoints(data)
        self.assertEqual(result, dict(retained_programs=8, retained_sources=8,
                                      complete_timed_suites=2, V_visualization_entries=0))
        for key, identity in maps.items():
            self.assertIs(data[key], identity)
        for benchmark in data['benchmarks']:
            self.assertEqual((benchmark['baseline_loc'], benchmark['baseline_seconds']), (10, 2.0))
            for group in benchmark['groups']:
                self.assertTrue(group['fast'].endswith(':optimized'))
                self.assertTrue(group['small'].endswith(':compact'))
                self.assertEqual(group['programs'], [group['fast'], group['small']])
        for key, row in data['programs'].items():
            self.assertEqual({k: v for k, v in row.items() if k not in ('x', 'y')}, before['programs'][key])
        self.assertEqual(data['files'], {key: before['files'][key] for key in data['files']})
        for kind, expected in (('fast', dict(x=3.0, y=1.0)), ('small', dict(x=1.0, y=4.0))):
            for axis, value in expected.items():
                self.assertAlmostEqual(data['reference_summary'][0][kind][axis], value)

    def test_source_only_small_remains_without_invented_time_or_complete_suite(self):
        data = payload()
        key = 'second:new:compact'
        data['programs'][key]['seconds'] = None
        data['programs'][key]['samples'] = []
        finalize_endpoints(data)
        self.assertIsNone(data['programs'][key]['seconds'])
        self.assertIsNone(data['programs'][key]['y'])
        self.assertEqual([row['lang'] for row in data['reference_summary']], ['old'])

    def test_one_source_can_own_both_endpoints_and_stable_ties_remain_stable(self):
        data = payload()
        row = data['programs']['first:new:compact']
        row['seconds'] = 1.0
        row['samples'] = [.9, 1.0, 1.1]
        finalize_endpoints(data)
        group = data['benchmarks'][0]['groups'][1]
        self.assertEqual(group['programs'], ['first:new:compact'])
        data = payload()
        data['programs']['first:new:other'].update(loc=30, seconds=2.0, samples=[1.9, 2.0, 2.1])
        finalize_endpoints(data)
        self.assertEqual(data['benchmarks'][0]['groups'][1]['fast'], 'first:new:optimized')

    def test_late_invalid_group_is_transactional(self):
        for mutation in ('foreign', 'missing', 'duplicate', 'bool-time', 'zero-loc', 'source',
                         'few-samples', 'partial', 'failed', 'median', 'bad-sample', 'V'):
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
            elif mutation == 'bad-sample': data['programs'][key]['samples'][0] = float('nan')
            else: data['programs'][key]['language'] = 'v'
            before = deepcopy(data)
            with self.assertRaises(ValueError): finalize_endpoints(data)
            self.assertEqual(data, before)


if __name__ == '__main__':
    unittest.main()
