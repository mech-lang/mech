"""Four meaningful synthetic full-133 identity/transaction fixtures; no I/O."""
from copy import deepcopy
import unittest

import adapter

COUNTS = {'c': 18, 'cpp': 18, 'rust': 19, 'mech': 15, 'mech-jit': 14,
          'mech-resident': 14, 'julia': 17, 'javascript': 18}


def fixture():
    data = dict(programs={}, files={}, provenance={}, axes=dict(x=[1, 200], y=[0.1, 20]),
                release_years=dict(c=1972, julia=2012), manual_selection=['c'])
    annotations, measured = {}, {}
    for group, (language, count) in enumerate(COUNTS.items(), 1):
        report, source_report, runtime_report = ('synthetic/' + language + '/' + name + '.json'
            for name in ('timings', 'sources', 'runtime'))
        for n, path in enumerate((report, source_report, runtime_report), 1):
            data['provenance'][path] = format(group * 10000 + n, '064x')
        for number in range(count):
            identifier = language + '-' + str(number).zfill(2); key = 'nbody:' + identifier
            file_key = format(group * 100 + number, '064x')
            data['files'][file_key] = dict(path='synthetic/' + identifier + '.source', sha256=file_key, code='original source')
            samples = [1.0, 1.1, 1.2]
            if language == 'mech' and number == 0: samples = [1.0, 1.1, 1.2, 1.3, 1.4, 1.5, 1.6]
            if language == 'cpp' and number == 0: samples = [1.0, 1.1, 1.2, 1.3, 1.4]
            row = dict(key=key, id=identifier, benchmark='nbody', language=language, files=[file_key],
                report=report, source_report=source_report, samples=samples, seconds=samples[len(samples)//2],
                loc=20 + number, x=20.0 + number, y=1.1, runtime_policy='preserved old prose')
            if language == 'mech-resident':
                row.update(timing_boundary='Loaded region; loading/activation excluded; input/work/output included',
                           whole_process_seconds=9.0, preparation_seconds=7.9)
            data['programs'][key] = row
            generic = language in ('julia', 'javascript')
            backend = ('javascript-runtime' if language == 'javascript' else 'native-runtime') if generic else (
                'native-jit' if language == 'mech-jit' else 'native-interpreter' if language == 'mech-resident' else 'native-aot')
            annotations[key] = dict(key=backend, execution=adapter.BACKENDS[backend], label=language + ' measured runtime', verified=True,
                confidence=dict(historical_platform_verified=True, historical_execution_verified=not generic,
                                historical_executable_bytes_verified=not generic),
                missing_evidence=['unrecorded execution mode', 'unrecorded historical executable bytes'] if generic else [],
                evidence=dict(verified=True, report=report, report_sha256=data['provenance'][report],
                    source_sha256=[file_key], command=['/actual/historical/runtime', 'original-input', '{n}'],
                    runtime_receipt=dict(verified=True, report=runtime_report, report_sha256=data['provenance'][runtime_report],
                        field='/saved_runtime', value='literal original observation', value_encoding='literal-saved-string')))
            measured[key] = dict(key=key, id=identifier, benchmark='nbody', language=language,
                files=[dict(path=data['files'][file_key]['path'], sha256=file_key)], report=report, source_report=source_report,
                samples=deepcopy(samples), seconds=row['seconds'], timing_boundary=row.get('timing_boundary'))
    qualification = dict(status='reconciled-with-unresolved-existing-platform-history', profile_count=133,
        annotations=annotations, measurement_identities=measured, provenance=deepcopy(data['provenance']))
    return data, deepcopy(data), qualification


class LegacyPlatformAdapterFixtures(unittest.TestCase):
    def test_independent_platform_with_unknown_mode_or_bytes(self):
        data, baseline, qualified = fixture(); before = deepcopy(data)
        changes, provenance = adapter.stage(data, baseline, qualified)
        self.assertEqual(data, before)
        javascript = 'nbody:javascript-00'; julia = 'nbody:julia-00'
        for key in (javascript, julia):
            self.assertTrue(changes[key]['measured_backend']['verified'])
            self.assertEqual(changes[key]['execution_mode'], 'unknown')
            self.assertFalse(changes[key]['measured_backend']['confidence']['historical_execution_verified'])
        unknown = qualified['annotations']['nbody:rust-18']
        unknown.update(key='unknown', execution='unknown', verified=False, missing_evidence=['missing cohort runtime observation'])
        unknown['confidence']['historical_platform_verified'] = unknown['confidence']['historical_execution_verified'] = False
        unknown['evidence']['verified'] = False; unknown['evidence']['runtime_receipt']['verified'] = False
        changes, _ = adapter.stage(data, baseline, qualified)
        self.assertEqual(changes['nbody:rust-18']['runtime_policy'], 'Unclassified')
        bad = deepcopy(qualified); bad['annotations'][javascript].update(key='javascript-jit', execution='jit')
        with self.assertRaises(ValueError): adapter.stage(data, baseline, bad)
        bad = deepcopy(qualified); bad['annotations'][julia]['confidence']['historical_execution_verified'] = True
        with self.assertRaises(ValueError): adapter.stage(data, baseline, bad)
        build = deepcopy(qualified); key = 'nbody:mech-00'
        build['annotations'][key]['evidence'].update(command=['/saved/native/compiler', '-o', '/saved/native/output'],
            command_role='native-build-link', missing_historical_timed_argv=True)
        build['annotations'][key]['missing_evidence'] = ['original timed argv not retained; native output hash is independently correlated']
        staged, _ = adapter.stage(data, baseline, build)
        self.assertTrue(staged[key]['measured_backend']['verified'])
        self.assertEqual(staged[key]['measured_backend']['evidence']['command_role'], 'native-build-link')
        self.assertTrue(staged[key]['measured_backend']['evidence']['missing_historical_timed_argv'])
        self.assertEqual(staged[key]['measured_backend']['evidence']['command'], ['/saved/native/compiler', '-o', '/saved/native/output'])
        build['annotations'][key]['evidence']['missing_historical_timed_argv'] = False
        with self.assertRaises(ValueError): adapter.stage(data, baseline, build)
        self.assertEqual(provenance, data['provenance'])

    def test_three_and_seven_samples_and_resident_boundary_are_preserved(self):
        data, baseline, qualified = fixture()
        first = data['programs']['nbody:c-00']['files'][0]
        data['files'][first].update(normalized_loc=45, count_style='new separately reviewed metadata')
        data['programs']['nbody:c-00'].update(loc=45, source_count_review='separate qualified count evidence')
        before = deepcopy(data)
        result = adapter.apply(data, baseline, qualified)
        self.assertEqual(result['programs'], 133)
        self.assertEqual(len(data['programs']['nbody:mech-00']['samples']), 7)
        self.assertEqual(len(data['programs']['nbody:c-00']['samples']), 3)
        self.assertEqual(len(data['programs']['nbody:cpp-00']['samples']), 5)
        resident = 'nbody:mech-resident-00'
        for name in ('samples', 'seconds', 'timing_boundary', 'whole_process_seconds', 'preparation_seconds'):
            self.assertEqual(data['programs'][resident][name], before['programs'][resident][name])
        for key, row in before['programs'].items():
            self.assertEqual({k:v for k,v in data['programs'][key].items() if k not in adapter.METADATA_FIELDS},
                             {k:v for k,v in row.items() if k not in adapter.METADATA_FIELDS})
        self.assertEqual(data['files'], before['files']); self.assertEqual(data['axes'], before['axes'])
        self.assertEqual(data['release_years'], before['release_years']); self.assertEqual(data['manual_selection'], before['manual_selection'])
        bad = deepcopy(qualified); bad['measurement_identities'][resident]['timing_boundary'] = 'Whole process including loading'
        with self.assertRaises(ValueError): adapter.stage(before, baseline, bad)
        bad = deepcopy(qualified); bad['measurement_identities']['nbody:mech-00']['samples'] = [1.0, 1.1, 1.2]
        with self.assertRaises(ValueError): adapter.stage(before, baseline, bad)

    def test_exact_old_current_source_sample_key_and_provenance_identity(self):
        initial, baseline, qualified = fixture(); key = 'nbody:javascript-00'
        def reject(mutator):
            data, saved, proof = deepcopy(initial), deepcopy(baseline), deepcopy(qualified)
            mutator(data, saved, proof); before = deepcopy(data)
            with self.assertRaises(ValueError): adapter.apply(data, saved, proof)
            self.assertEqual(data, before)
        reject(lambda d,b,q: d['programs'][key]['samples'].__setitem__(0, 2.0))
        reject(lambda d,b,q: b['files'][b['programs'][key]['files'][0]].update(path='synthetic/other.source'))
        reject(lambda d,b,q: d['files'][d['programs'][key]['files'][0]].update(code='changed source but stale hash'))
        reject(lambda d,b,q: q['measurement_identities'][key]['files'][0].update(sha256='f'*64))
        reject(lambda d,b,q: d['programs'][key].update(key='nbody:missing-target'))
        reject(lambda d,b,q: q['provenance'].update({d['programs'][key]['report']:'f'*64}))
        reject(lambda d,b,q: q['annotations'][key]['evidence']['runtime_receipt'].update(report_sha256='e'*64))
        reject(lambda d,b,q: q['measurement_identities'][key]['samples'].__setitem__(0, True))
        reject(lambda d,b,q: q['provenance'].update({'../foreign.json':'a'*64}))
        reject(lambda d,b,q: q.update(profile_count=True))

    def test_late_invalid_row_causes_zero_mutation(self):
        data, baseline, qualified = fixture(); last = sorted(qualified['annotations'])[-1]
        qualified['annotations'][last]['evidence']['source_sha256'] = ['f'*64]
        before = deepcopy(data)
        with self.assertRaises(ValueError): adapter.apply(data, baseline, qualified)
        self.assertEqual(data, before)
        qualified = fixture()[2]; del qualified['measurement_identities'][last]
        with self.assertRaises(ValueError): adapter.apply(data, baseline, qualified)
        self.assertEqual(data, before)
        qualified = fixture()[2]; qualified['annotations'][last]['confidence']['historical_platform_verified'] = 1
        with self.assertRaises(ValueError): adapter.apply(data, baseline, qualified)
        self.assertEqual(data, before)


if __name__ == '__main__': unittest.main()
