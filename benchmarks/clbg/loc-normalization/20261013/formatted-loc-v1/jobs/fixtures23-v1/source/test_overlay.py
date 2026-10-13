"""Six synthetic source fixtures; no byte verification or execution claimed."""
from copy import deepcopy
import unittest

from overlay import apply, identity, stage


OVERLAY = frozenset(('formatted_loc', 'formatted_loc_status', 'formatted_loc_complete', 'formatted_loc_evidence'))


def fixture():
    """Two measured bundles include a shared, counted helper with zero lines."""
    raw = dict(programs={}, files={}, provenance={'original-report.json': 'd' * 64},
               languages=[dict(key='nim', label='Nim')], benchmarks=[dict(key='first', untouched='benchmark',
                   groups=[dict(language='nim', programs=['one', 'two'])])])
    for published, char in (('main-one', 'a'), ('main-two', 'b'), ('helper', 'c')):
        raw['files'][published] = dict(name=published + '.nim', path='published/' + published + '.nim',
            code='original source ' + published, sha256=char * 64, canonical_annotation='preserve')
    for key, seconds in (('one', 2.0), ('two', 3.0)):
        raw['programs'][key] = dict(key=key, id='measured-' + key, benchmark='first', language='nim',
            seconds=seconds, samples=[seconds - .1, seconds, seconds + .1], report='original-report.json',
            source_report='original-counts.json', timing_boundary='whole original process',
            runtime_arguments=['actual-runtime', key], files=['main-' + key, 'helper'], loc=10,
            binary_sha256='8' * 64,
            old_metadata=dict(source_note='unchanged'), loc_expansions=['canonical unchanged'])
    baseline = dict(data=deepcopy(raw), path='saved-source-dataset.json', sha256='e' * 64)
    q = dict(status='completed-formatted-loc-measurement', chart_admission=False,
        actual_application_processes=0, physical_definition='nonblank-noncomment-physical-lines-v1',
        source_dataset_path=baseline['path'], source_dataset_sha256=baseline['sha256'],
        artifact=dict(path='qualification.json', sha256='f' * 64), pins={}, receipts={},
        terminal_artifact=dict(path='terminal.json', sha256='7' * 64),
        provenance={'qualification.json': 'f' * 64, 'terminal.json': '7' * 64},
        rows={}, profile_keys=['one', 'two'], measurement_identities={key: identity(raw, key) for key in ('one', 'two')})

    def pin(path, sha256='1' * 64):
        q['pins'][path] = sha256
        return dict(path=path, sha256=sha256)

    def receipt(path, binding):
        value = pin(path)
        q['receipts'][path] = dict(actual=deepcopy(binding))
        value.update(binding_field='/actual', binding=deepcopy(binding))
        return value

    pin(baseline['path'], baseline['sha256'])
    for key in q['profile_keys']:
        files = []
        for published in raw['programs'][key]['files']:
            source = raw['files'][published]
            path = '/authored/' + source['name']
            original = pin(path, source['sha256'])
            output = pin('/formatted/' + key + '/' + source['name'], '2' * 64)
            tool = pin('/official/nimpretty', '3' * 64)
            tool['version'] = 'Nim 2.2.12 nimpretty 0.2 fixture'
            version = receipt('/receipts/' + key + '/' + published + '-version.json', tool['version'])
            formatter = dict(provider='language-project', official_sources=['https://nim-lang.org/'],
                tool=tool, config=pin('/config/nimpretty.json', '4' * 64), input=original, output=output,
                version_receipt=version, source_preservation_verified=True,
                ownership_verified=True, no_application_execution=True)
            formatter['receipt'] = receipt('/receipts/' + key + '/' + published + '-formatter.json', formatter)
            counter = dict(definition=q['physical_definition'], module_path='/source/lexical.py',
                module_sha256='5' * 64, function='mask_source', argument='nim', diagnostics=[],
                no_semantic_normalization=True)
            pin(counter['module_path'], counter['module_sha256'])
            count = 0 if published == 'helper' else 7
            binding = dict(counter, input_sha256=output['sha256'], physical_loc=count)
            counter['receipt'] = receipt('/receipts/' + key + '/' + published + '-counter.json', binding)
            files.append(dict(published_file=published, path=path, sha256=source['sha256'], language='nim',
                role='application-helper' if published == 'helper' else 'application', status='qualified',
                physical_loc=count, formatter_evidence=formatter, counter_evidence=counter))
        inventory = dict(status='qualified-exact-authored-compiler-inputs',
                         source_sha256=files[0]['sha256'], binary_sha256='8' * 64,
                         files=[dict(path=item['path'], sha256=item['sha256']) for item in files])
        proof = receipt('/compiler-proof/' + key + '.json', inventory)
        descriptor = dict(evidence_path=proof['path'], evidence_sha256=proof['sha256'],
                          binding_field=proof['binding_field'], binding=inventory)
        pin(raw['programs'][key]['runtime_arguments'][0], '8' * 64)
        measured = dict(id=raw['programs'][key]['id'], benchmark=raw['programs'][key]['benchmark'],
                        source_sha256=files[0]['sha256'], binary_sha256=inventory['binary_sha256'],
                        command=deepcopy(raw['programs'][key]['runtime_arguments']),
                        samples=deepcopy(raw['programs'][key]['samples']))
        measurement = receipt('/actual-measurement/' + key + '.json', measured)
        measurement['fields'] = dict(profile_id='/id', benchmark='/benchmark', source_sha256='/source_sha256',
                                     binary_sha256='/binary_sha256', command='/command', samples='/samples',
                                     command_image_sha256='/binary_sha256')
        q['rows'][key] = dict(key=key, language='nim', files=files, compiler_inventory=descriptor,
                              measurement_evidence=measurement, status='qualified', complete=True,
                              formatted_loc=7, reasons=[])
    close_terminal(q)
    return raw, baseline, q


def close_terminal(q):
    q['terminal'] = dict(status=q['status'], body_error=None, integrity_errors=[],
        available_bytes_not_fully_pinned=False, inherited_common_mutex=True, verified_pins=deepcopy(q['pins']),
        qualification=q['artifact']['path'], qualification_sha256=q['artifact']['sha256'])


class FormattedOverlay(unittest.TestCase):
    def test_inventory_only_and_shared_profiles_bind_exact_sources_and_measurements(self):
        data, baseline, q = fixture()
        data['program_inventory'] = deepcopy(data['programs'])
        baseline['data']['program_inventory'] = deepcopy(baseline['data']['programs'])
        del data['programs']['two']; del baseline['data']['programs']['two']
        before = deepcopy(data)
        self.assertEqual(apply(data, baseline, q), dict(updated_programs=2, qualified_programs=2))
        self.assertEqual(data['programs']['one']['formatted_loc_evidence'], data['program_inventory']['one']['formatted_loc_evidence'])
        self.assertEqual(data['program_inventory']['two']['formatted_loc'], 7)
        self.assertEqual(identity(data, 'two'), identity(baseline['data'], 'two'))
        for key, row in data['program_inventory'].items():
            self.assertEqual({field: value for field, value in row.items() if field not in OVERLAY}, before['program_inventory'][key])
        for corruption in ('time', 'source', 'canonical', 'missing-file', 'excluded'):
            with self.subTest(corruption=corruption):
                current, saved, qualification = fixture()
                current['program_inventory'] = deepcopy(current['programs'])
                saved['data']['program_inventory'] = deepcopy(saved['data']['programs'])
                row = current['program_inventory']['one']
                if corruption == 'time': row['seconds'] = 99.0
                elif corruption == 'source': row['files'] = ['main-two', 'helper']
                elif corruption == 'canonical': row['loc'] = 99
                elif corruption == 'missing-file':
                    del current['programs']['two']
                    del current['files']['main-two']
                else:
                    extra = deepcopy(row); extra.update(key='excluded', language='Red')
                    current['program_inventory']['excluded'] = extra
                original = deepcopy(current)
                with self.assertRaises(ValueError): apply(current, saved, qualification)
                self.assertEqual(current, original)

    def test_actual_measured_execution_fields_and_independent_receipt_are_required(self):
        # An old dataset need not already contain the binary digest if the
        # actual independent measured row binds the exact source/executable/
        # command/samples to this profile and the compiler input inventory.
        data, baseline, q = fixture()
        for wrapper in (data, baseline['data']):
            del wrapper['programs']['two']['binary_sha256']
        q['measurement_identities']['two'] = identity(data, 'two')
        self.assertEqual(apply(data, baseline, q)['qualified_programs'], 2)
        # Native legacy metadata can retain its compiler distribution digest
        # in runtime_sha256; the recorded launched native image stays binary.
        current, saved, qualification = fixture()
        for wrapper in (current, saved['data']):
            wrapper['programs']['two'].update(runtime_sha256='9' * 64, execution_mode='aot')
        qualification['measurement_identities']['two'] = identity(current, 'two')
        self.assertEqual(apply(current, saved, qualification)['qualified_programs'], 2)
        for missing in ('receipt', 'saved-command', 'compiler-inventory', 'recorded-image'):
            with self.subTest(missing=missing):
                data, baseline, q = fixture()
                row = q['rows']['two']
                if missing == 'receipt': row['measurement_evidence'] = None
                elif missing == 'compiler-inventory': row['compiler_inventory'] = None
                elif missing == 'recorded-image': del row['measurement_evidence']['fields']['command_image_sha256']
                else:
                    for wrapper in (data, baseline['data']):
                        del wrapper['programs']['two']['runtime_arguments']
                    q['measurement_identities']['two'] = identity(data, 'two')
                before = deepcopy(data)
                with self.assertRaises(ValueError): apply(data, baseline, q)
                self.assertEqual(data, before)
                reason = 'Independent actual measured execution is not bound'
                for item in row['files']:
                    item.update(status='pending', physical_loc=None, formatter_evidence=None,
                                counter_evidence=None, reason=reason)
                row.update(status='pending', complete=False, formatted_loc=None, reasons=[reason])
                self.assertEqual(apply(data, baseline, q), dict(updated_programs=2, qualified_programs=1))
                self.assertIsNone(data['programs']['two']['formatted_loc'])
        for field in ('id', 'benchmark', 'source_sha256', 'binary_sha256', 'command', 'samples',
                      'actual-record', 'field-pointer', 'compiler-artifact', 'dataset-artifact',
                      'image-pin', 'image-pin-digest', 'image-pointer'):
            with self.subTest(field=field):
                data, baseline, q = fixture()
                evidence = q['rows']['two']['measurement_evidence']
                if field in ('id', 'benchmark', 'source_sha256', 'binary_sha256'):
                    evidence['binding'][field] = 'different-profile-value'
                    q['receipts'][evidence['path']]['actual'] = deepcopy(evidence['binding'])
                elif field in ('command', 'samples'):
                    evidence['binding'][field] = ['different-command'] if field == 'command' else [3, 3, 3]
                    q['receipts'][evidence['path']]['actual'] = deepcopy(evidence['binding'])
                elif field == 'actual-record': q['receipts'][evidence['path']]['actual']['id'] = 'different'
                elif field == 'field-pointer': evidence['fields']['profile_id'] = '/benchmark'
                elif field == 'image-pointer': evidence['fields']['command_image_sha256'] = '/source_sha256'
                elif field == 'image-pin':
                    del q['pins'][data['programs']['two']['runtime_arguments'][0]]
                    close_terminal(q)
                elif field == 'image-pin-digest':
                    # The replacement image has a valid actual pin, but those
                    # bytes differ from the independent recorded binary SHA.
                    q['pins'][data['programs']['two']['runtime_arguments'][0]] = '9' * 64
                    close_terminal(q)
                else:
                    evidence['path'] = (q['rows']['two']['compiler_inventory']['evidence_path']
                                        if field == 'compiler-artifact' else baseline['path'])
                    evidence['sha256'] = q['pins'][evidence['path']]
                    q['receipts'].setdefault(evidence['path'], {})['measurement'] = deepcopy(evidence['binding'])
                    evidence['binding_field'] = '/measurement'
                before = deepcopy(data)
                with self.assertRaises(ValueError): apply(data, baseline, q)
                self.assertEqual(data, before)

        def interpreter_fixture():
            current, saved, qualification = fixture()
            command = ['/runtime/interpreter', '/compiled/two.module']
            for wrapper in (current, saved['data']):
                wrapper['programs']['two'].update(runtime_arguments=deepcopy(command),
                    runtime_sha256='9' * 64, execution_mode='interpreted')
            evidence = qualification['rows']['two']['measurement_evidence']
            evidence['binding'].update(command=deepcopy(command), runtime_sha256='9' * 64)
            evidence['fields']['command_image_sha256'] = '/runtime_sha256'
            qualification['receipts'][evidence['path']]['actual'] = deepcopy(evidence['binding'])
            qualification['pins'][command[0]] = '9' * 64
            qualification['pins'][command[1]] = '8' * 64
            qualification['measurement_identities']['two'] = identity(current, 'two')
            close_terminal(qualification)
            return current, saved, qualification

        # A separately recorded interpreter image need not have the digest of
        # the compiled artifact; both identities remain exact and unchanged.
        current, saved, qualification = interpreter_fixture()
        self.assertEqual(apply(current, saved, qualification)['qualified_programs'], 2)
        self.assertEqual(current['programs']['two']['binary_sha256'], '8' * 64)
        self.assertEqual(current['programs']['two']['runtime_sha256'], '9' * 64)
        for corruption in ('image-pin', 'actual-runtime-record', 'saved-runtime', 'artifact-as-runtime'):
            with self.subTest(interpreter_corruption=corruption):
                current, saved, qualification = interpreter_fixture()
                evidence = qualification['rows']['two']['measurement_evidence']
                if corruption == 'image-pin':
                    qualification['pins']['/runtime/interpreter'] = '8' * 64
                    close_terminal(qualification)
                elif corruption == 'actual-runtime-record':
                    qualification['receipts'][evidence['path']]['actual']['runtime_sha256'] = '8' * 64
                elif corruption == 'saved-runtime':
                    for wrapper in (current, saved['data']): wrapper['programs']['two']['runtime_sha256'] = '7' * 64
                    qualification['measurement_identities']['two'] = identity(current, 'two')
                else:
                    evidence['fields']['command_image_sha256'] = '/binary_sha256'
                    # Re-pinning the interpreter to artifact bytes must not
                    # override the actual/saved runtime image observation.
                    qualification['pins']['/runtime/interpreter'] = '8' * 64
                    close_terminal(qualification)
                before = deepcopy(current)
                with self.assertRaises(ValueError): apply(current, saved, qualification)
                self.assertEqual(current, before)

    def test_whole_bundle_requires_every_authored_file_and_accepts_a_zero_line_helper(self):
        data, baseline, q = fixture()
        before = deepcopy(data)
        inputs = deepcopy((baseline, q))
        changes, provenance = stage(data, baseline, q)
        self.assertEqual(data, before)
        self.assertEqual((baseline, q), inputs)
        self.assertEqual(changes['one']['formatted_loc'], 7)
        self.assertTrue(changes['one']['formatted_loc_complete'])
        self.assertEqual(changes['one']['formatted_loc_evidence']['files'][1]['physical_loc'], 0)
        self.assertEqual(provenance['qualification.json'], 'f' * 64)
        self.assertEqual(provenance['terminal.json'], '7' * 64)
        self.assertEqual(apply(data, baseline, q), dict(updated_programs=2, qualified_programs=2))
        current, saved, qualification = fixture()
        count_receipt = qualification['rows']['two']['files'][0]['counter_evidence']['receipt']
        qualification['receipts'][count_receipt['path']] = deepcopy(count_receipt['binding'])
        count_receipt['binding_field'] = ''
        self.assertEqual(apply(current, saved, qualification)['qualified_programs'], 2)
        for corruption in ('omit-helper', 'wrong-total', 'all-zero-total', 'alias-without-inventory',
                           'no-inventory-samepath', 'extra-compiler-input'):
            with self.subTest(corruption=corruption):
                current, saved, qualification = fixture()
                row = qualification['rows']['two']
                if corruption == 'omit-helper': row['files'].pop()
                elif corruption == 'wrong-total': row['formatted_loc'] = 8
                elif corruption == 'all-zero-total':
                    row['formatted_loc'] = 0
                    for item in row['files']:
                        item['physical_loc'] = 0
                        receipt = item['counter_evidence']['receipt']
                        receipt['binding']['physical_loc'] = 0
                        qualification['receipts'][receipt['path']]['actual'] = deepcopy(receipt['binding'])
                elif corruption == 'alias-without-inventory': row['compiler_inventory'] = None
                elif corruption == 'no-inventory-samepath':
                    for item in row['files']:
                        current['files'][item['published_file']]['path'] = item['path']
                        saved['data']['files'][item['published_file']]['path'] = item['path']
                    row['compiler_inventory'] = None
                else: row['compiler_inventory']['binding']['files'].append(dict(path='/authored/hidden.nim', sha256='6' * 64))
                original = deepcopy(current)
                with self.assertRaises(ValueError):
                    apply(current, saved, qualification)
                self.assertEqual(current, original)

    def test_unavailable_helper_keeps_bundle_null_without_main_file_fallback(self):
        data, baseline, q = fixture()
        row = q['rows']['one']
        row['files'][1].update(status='unavailable', physical_loc=None, formatter_evidence=None,
                              counter_evidence=None, reason='official helper formatter route not verified')
        row.update(status='unavailable', complete=False, formatted_loc=None,
                   reasons=['official helper formatter route not verified'])
        apply(data, baseline, q)
        self.assertIsNone(data['programs']['one']['formatted_loc'])
        self.assertEqual(data['programs']['one']['formatted_loc_status'], 'unavailable')
        self.assertFalse(data['programs']['one']['formatted_loc_complete'])
        self.assertEqual(data['programs']['one']['loc'], 10)
        self.assertEqual(data['programs']['one']['seconds'], 2.0)
        self.assertEqual(data['files'], baseline['data']['files'])
        data, baseline, q = fixture()
        row = q['rows']['two']
        row['files'][1].update(status='unavailable', physical_loc=None, formatter_evidence=None,
                              counter_evidence=None, reason='helper unavailable')
        before = deepcopy(data)
        with self.assertRaises(ValueError):
            apply(data, baseline, q)
        self.assertEqual(data, before)

    def test_measurement_and_source_identities_are_exact_while_new_metadata_is_preserved(self):
        data, baseline, q = fixture()
        data['programs']['one'].update(loc=31, loc_report='new-canonical-LOC.json', new_display=dict(note='later annotation'))
        data['files']['main-one'].update(normalized_loc=31, new_note='later source annotation')
        before = deepcopy(data)
        apply(data, baseline, q)
        self.assertEqual(data['files'], before['files'])
        self.assertEqual(data['languages'], before['languages'])
        self.assertEqual(data['benchmarks'], before['benchmarks'])
        for key, row in data['programs'].items():
            self.assertEqual({field: value for field, value in row.items() if field not in OVERLAY}, before['programs'][key])
        for field in ('seconds', 'seconds-type', 'samples', 'report', 'source_report',
                      'timing_boundary', 'runtime_arguments', 'code', 'path'):
            with self.subTest(field=field):
                current, saved, qualification = fixture()
                if field in ('code', 'path'):
                    current['files']['main-two'][field] = 'changed original source'
                elif field == 'seconds': current['programs']['two'][field] = 4.0
                elif field == 'seconds-type': current['programs']['two']['seconds'] = 3
                elif field == 'samples': current['programs']['two'][field] = [3.0, 4.0, 5.0]
                elif field == 'runtime_arguments': current['programs']['two'][field] = ['different-runtime', 'two']
                else: current['programs']['two'][field] = 'changed measurement identity'
                before = deepcopy(current)
                with self.assertRaises(ValueError):
                    apply(current, saved, qualification)
                self.assertEqual(current, before)

    def test_late_receipt_pin_terminal_and_selection_failures_are_transactional(self):
        for corruption in ('actual-counter', 'actual-formatter', 'tool-pin', 'terminal', 'body-error',
                           'profile-omission', 'published-digest', 'inventory-receipt', 'excluded-language',
                           'unselected-red', 'all-qualified-unavailable', 'binary-mismatch', 'terminal-provenance'):
            with self.subTest(corruption=corruption):
                data, baseline, q = fixture()
                last = q['rows']['two']['files'][0]
                if corruption == 'actual-counter':
                    q['receipts'][last['counter_evidence']['receipt']['path']]['actual']['physical_loc'] = 99
                elif corruption == 'actual-formatter':
                    q['receipts'][last['formatter_evidence']['receipt']['path']]['actual']['input']['sha256'] = '9' * 64
                elif corruption == 'tool-pin':
                    q['pins']['/official/nimpretty'] = '9' * 64
                    close_terminal(q)
                elif corruption == 'terminal': q['terminal']['verified_pins'].pop('/official/nimpretty')
                elif corruption == 'body-error': q['terminal']['body_error'] = dict(error='late worker failure')
                elif corruption == 'profile-omission':
                    del q['rows']['two']
                    del q['measurement_identities']['two']
                elif corruption == 'published-digest': last['sha256'] = '9' * 64
                elif corruption == 'inventory-receipt':
                    descriptor = q['rows']['two']['compiler_inventory']
                    q['receipts'][descriptor['evidence_path']]['actual']['files'].pop()
                elif corruption == 'excluded-language': last['language'] = 'Red'
                elif corruption == 'unselected-red': data['languages'].append(dict(key='Red', label='excluded'))
                elif corruption == 'all-qualified-unavailable':
                    q['rows']['two'].update(status='unavailable', complete=False, formatted_loc=None, reasons=['contradictory'])
                elif corruption == 'binary-mismatch': data['programs']['two']['binary_sha256'] = '9' * 64
                else: q['provenance']['terminal.json'] = '9' * 64
                before = deepcopy(data)
                with self.assertRaises(ValueError):
                    apply(data, baseline, q)
                self.assertEqual(data, before)


if __name__ == '__main__':
    unittest.main()
