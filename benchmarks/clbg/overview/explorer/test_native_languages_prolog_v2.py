"""Candidate eight-language fixtures; synthetic values never enter a dataset.

Source only. Root must freeze/freshly qualify this and unchanged base fixtures
under the common mutex before any hook. No benchmark/runtime/version is run.
"""
from copy import deepcopy
import unittest

import native_languages_data_v2 as importer
import native_language_groups_v2 as groups
import native_platform_metadata_v2 as platform
from test_native_languages_data_v2 import fixture as base_cohort
from test_native_platform_metadata_v1 import example as base_platform


def prolog_cohort():
    suite, counts, meta = base_cohort('prolog')
    binary = '/synthetic-only/swipl'
    review = dict(status='passed', review_identity='synthetic-only-never-published',
        evidence_path='/synthetic-only/vm-review', evidence_sha256='8'*64)
    meta.update(execution='Interpreted', scored_native_architecture='arm64', emulation_used=False,
        pins={binary:'b'*64, review['evidence_path']:'8'*64},
        measured_backend_template=dict(verified=True,key='native-interpreter',execution='interpreted'))
    identities = []
    for evidence in suite['benchmarks'].values():
        for row in evidence['profiles'].values():
            if row['language'] != 'prolog': continue
            identities.append(row['id'])
            artifact = dict(path='/synthetic-only/'+row['id']+'.qlf',sha256='7'*64,kind='swi-qlf-bytecode')
            row.update(command=[binary,'-q','-s',artifact['path']],runtime_artifacts=[artifact])
            inventory=counts['compiler_bound_input_inventory'][row['id']]
            runtime = dict(status='passed',path='/synthetic-only/'+row['id']+'-runtime.json',sha256='6'*64,
                binding_field='/runtime_binding',binding=importer.runtime_binding(row,inventory['files']))
            inventory.update(runtime_artifacts=deepcopy(row['runtime_artifacts']),runtime_receipt=runtime)
            for item in (artifact,runtime):
                counts['pins'][item['path']]=item['sha256']; meta['pins'][item['path']]=item['sha256']
    meta['interpreted_vm_evidence']=dict(status='qualified-actual-interpreted-native-vm',
        interpreted_vm_execution_verified=True,native_aot_proven=False,native_machine_code_jit_proven=False,
        runtime_binary=binary,runtime_binary_sha256='b'*64,profile_ids=identities,review=review)
    return suite,counts,meta


def rebol_cohort():
    suite, counts, meta = base_cohort('rebol')
    binary = '/synthetic-only/rebol3'
    review = dict(status='passed', review_identity='synthetic-only-never-published',
        evidence_path='/synthetic-only/rebol-execution-review', evidence_sha256='4'*64,
        execution='Interpreted', profile_ids=[])
    meta.update(execution='Interpreted', scored_native_architecture='arm64', emulation_used=False,
        pins={binary:'b'*64, review['evidence_path']:'4'*64}, execution_model_review=review,
        measured_backend_template=dict(verified=True,key='native-interpreter',execution='interpreted'),
        interpreted_vm_evidence=dict(status='qualified-actual-interpreted-native-vm',
            interpreted_vm_execution_verified=True,native_aot_proven=False,native_machine_code_jit_proven=False,
            runtime_binary=binary,runtime_binary_sha256='b'*64))
    for evidence in suite['benchmarks'].values():
        for row in evidence['profiles'].values():
            if row['language'] != 'rebol': continue
            review['profile_ids'].append(row['id'])
            row['command']=[binary,'-q',row['source']]
            inventory=counts['compiler_bound_input_inventory'][row['id']]
            runtime=dict(status='passed',path='/synthetic-only/'+row['id']+'-load.json',sha256='3'*64,
                binding_field='/runtime_binding',binding=importer.runtime_binding(row,inventory['files']))
            counts['compiler_bound_input_inventory'][row['id']]['runtime_receipt']=runtime
            counts['pins'][runtime['path']]=runtime['sha256']; meta['pins'][runtime['path']]=runtime['sha256']
    return suite,counts,meta


def eight_platform():
    data,metadata,actual,provenance=base_platform()
    for language, constructor in (('prolog',prolog_cohort),('rebol',rebol_cohort)):
        suite,counts,meta=constructor()
        meta.update(memory_strategies=['gc'],memory_label='Synthetic VM GC, never published',
            publication_metadata_evidence=dict(path=language+'/metadata.json',sha256='d'*64))
        meta['measured_backend_template'].update(label='Synthetic native interpreter, never published',
            runtime_receipt=dict(verified=True,report=language+'/runtime.json',report_sha256='c'*64,
                field='/actual_version',value='Actual saved version\n',value_encoding='literal-saved-string'))
        metadata[language]=meta; actual[language]=dict(actual_version='Actual saved version\n')
        provenance.update({language+'/runtime.json':'c'*64,language+'/metadata.json':'d'*64})
        data['provenance'][language+'/measurement.json']='b'*64
        data['native_language_audit']['languages'][language]=dict(source_count=20,actual_full_native_processes=90,sample_count=3,
            artifacts=dict(metadata=dict(path=language+'/metadata.json',sha256='d'*64)))
        data['languages'].append(dict(key=language))
        for evidence in suite['benchmarks'].values():
            for row in evidence['profiles'].values():
                if row['language'] != language: continue
                key='benchmark:'+row['id']
                data['programs'][key]=dict(key=key,id=row['id'],language=language,execution_mode='interpreted',
                    required_work_validated=True,report=language+'/measurement.json',files=['a'*64],binary_sha256='b'*64,
                    runtime_arguments=deepcopy(row['command']),source_path=row['source'],
                    runtime_input_bundle=deepcopy(counts['compiler_bound_input_inventory'][row['id']]['files']),
                    runtime_load_evidence=deepcopy(counts['compiler_bound_input_inventory'][row['id']]['runtime_receipt']))
                # Synthetic shared helper is present in the complete admitted
                # bundle; content IDs never stand for omitted authored inputs.
                data['files']['c'*64]=dict(sha256='c'*64)
                data['programs'][key]['files'].append('c'*64)
                if language=='prolog': data['programs'][key]['runtime_artifacts']=deepcopy(row['runtime_artifacts'])
    for row in data['languages']: row['execution']=platform.BACKENDS[row['key']][1].title()
    data['language_groups']=[dict(key=key,languages=[]) for key in sorted(groups.GROUPS)]
    data['protocol']=dict(language_groups='Language filters intersect Execution, Typing, Language style and Memory management categories.')
    return data,metadata,actual,provenance


class NativeEightAdmission(unittest.TestCase):
    def test_complete_original_qlf_bundle_and_six_existing_modes(self):
        suite,counts,meta=prolog_cohort(); before=deepcopy((suite,counts,meta))
        rows=importer.validate_cohort('prolog',suite,counts,meta)
        self.assertEqual(len(rows),20)
        self.assertTrue(all(item['metric']['runtime_artifacts']==item['row']['runtime_artifacts'] for item in rows))
        self.assertEqual((suite,counts,meta),before)
        suite,counts,meta=rebol_cohort(); before=deepcopy((suite,counts,meta))
        self.assertEqual(len(importer.validate_cohort('rebol',suite,counts,meta)),20)
        self.assertEqual((suite,counts,meta),before)
        for language in importer.LANGUAGES-{'prolog','rebol'}:
            suite,counts,meta=base_cohort(language)
            self.assertEqual(len(importer.validate_cohort(language,suite,counts,meta)),20)

    def test_guessed_native_codegen_missing_vm_or_artifact_reject(self):
        mutations=[
            lambda s,c,m: m.update(execution='AOT'),
            lambda s,c,m: m['measured_backend_template'].update(key='jvm-jit',execution='jit'),
            lambda s,c,m: m['interpreted_vm_evidence'].update(interpreted_vm_execution_verified=False),
            lambda s,c,m: m['interpreted_vm_evidence'].update(native_machine_code_jit_proven=True),
            lambda s,c,m: m['interpreted_vm_evidence'].update(native_aot_proven=True),
            lambda s,c,m: m.update(emulation_used=True),
            lambda s,c,m: m.update(scored_native_architecture='x86_64'),
            lambda s,c,m: m['interpreted_vm_evidence']['profile_ids'].pop(),
            lambda s,c,m: m['pins'].pop('/synthetic-only/vm-review'),
            lambda s,c,m: next(iter(c['compiler_bound_input_inventory'].values()))['runtime_artifacts'].clear(),
            lambda s,c,m: next(iter(c['compiler_bound_input_inventory'].values()))['runtime_receipt'].update(sha256='5'*64),
            lambda s,c,m: next(row for e in s['benchmarks'].values() for row in e['profiles'].values()
                                  if row['language']=='prolog')['command'].__setitem__(-1,'/different/unscored.qlf'),
            lambda s,c,m: next(iter(c['compiler_bound_input_inventory'].values()))['runtime_receipt']['binding']['authored_inputs'].pop(),
        ]
        for change in mutations:
            with self.subTest(change=change):
                suite,counts,meta=prolog_cohort(); change(suite,counts,meta); before=deepcopy((suite,counts,meta))
                with self.assertRaises((ValueError,KeyError)): importer.validate_cohort('prolog',suite,counts,meta)
                self.assertEqual((suite,counts,meta),before)
        rebol_mutations=[
            lambda s,c,m: m.update(execution='AOT'),
            lambda s,c,m: m.update(emulation_used=True),
            lambda s,c,m: m.update(scored_native_architecture='x86_64'),
            lambda s,c,m: m['execution_model_review']['profile_ids'].pop(),
            lambda s,c,m: m['pins'].pop('/synthetic-only/rebol-execution-review'),
            lambda s,c,m: m['interpreted_vm_evidence'].update(native_machine_code_jit_proven=True),
            lambda s,c,m: next(iter(c['compiler_bound_input_inventory'].values()))['runtime_receipt'].update(sha256='2'*64),
            lambda s,c,m: next(row for e in s['benchmarks'].values() for row in e['profiles'].values()
                                  if row['language']=='rebol')['command'].__setitem__(0,'/different/rebol'),
            lambda s,c,m: next(row for e in s['benchmarks'].values() for row in e['profiles'].values()
                                  if row['language']=='rebol')['command'].__setitem__(-1,'/different/unscored.reb'),
        ]
        for change in rebol_mutations:
            with self.subTest(change=change):
                suite,counts,meta=rebol_cohort(); change(suite,counts,meta); before=deepcopy((suite,counts,meta))
                with self.assertRaises((ValueError,KeyError)): importer.validate_cohort('rebol',suite,counts,meta)
                self.assertEqual((suite,counts,meta),before)
        for constructor in (prolog_cohort,rebol_cohort):
            _,counts,_=constructor()
            receipt=next(iter(counts['compiler_bound_input_inventory'].values()))['runtime_receipt']
            actual=dict(runtime_binding=deepcopy(receipt['binding']))
            importer.validate_runtime_load_value(receipt,actual)
            for field in ('command','authored_inputs','runtime_artifacts'):
                changed=deepcopy(actual)
                changed['runtime_binding'][field].append('/different/unscored-input')
                with self.subTest(field=field),self.assertRaises(ValueError):
                    importer.validate_runtime_load_value(receipt,changed)
            for flag in ('source_loading_verified','startup_input_closure_verified'):
                changed=deepcopy(actual); changed['runtime_binding'][flag]=1
                with self.subTest(flag=flag),self.assertRaises(ValueError):
                    importer.validate_runtime_load_value(receipt,changed)
        for excluded in ('red','v'):
            suite,counts,meta=base_cohort(excluded)
            with self.assertRaises(ValueError): importer.validate_cohort(excluded,suite,counts,meta)

    def test_actual_interpreted_platform_and_group_membership(self):
        data,meta,actual,pins=eight_platform(); before=deepcopy(data)
        changes,memory,_=platform.stage(data,meta,actual,pins)
        self.assertEqual(data,before); self.assertEqual(len(changes),160)
        self.assertEqual(set(memory),importer.LANGUAGES)
        for key,value in changes.items(): data['programs'][key]['measured_backend']=value
        rows=[row for row in data['programs'].values() if row['language']=='prolog']
        self.assertTrue(all(row['measured_backend']['key']=='native-interpreter' for row in rows))
        groups.add_native_language_groups(data)
        members={row['key']:row['languages'] for row in data['language_groups']}
        self.assertIn('prolog',members['interpreted']); self.assertIn('prolog',members['dynamic'])
        self.assertIn('rebol',members['interpreted'])
        self.assertIn('rebol',members['dynamic'])
        for key in ('aot','jit','functional','array'): self.assertNotIn('prolog',members[key])
        for key in ('aot','jit','functional','array'): self.assertNotIn('rebol',members[key])

    def test_partial_eight_wrong_vm_command_and_excluded_languages_are_transactional(self):
        mutations=[
            lambda d,m: m.pop('prolog'),
            lambda d,m: m['prolog']['measured_backend_template'].update(key='native-aot',execution='aot'),
            lambda d,m: d['native_language_audit']['languages']['prolog'].update(actual_full_native_processes=89),
            lambda d,m: next(row for row in d['programs'].values() if row['language']=='prolog')['runtime_arguments'].__setitem__(0,'/different/swipl'),
            lambda d,m: next(row for row in d['programs'].values() if row['language']=='rebol')['runtime_arguments'].__setitem__(0,'/different/rebol'),
            lambda d,m: next(row for row in d['programs'].values() if row['language']=='prolog')['runtime_arguments'].__setitem__(-1,'/different/unscored.qlf'),
            lambda d,m: next(row for row in d['programs'].values() if row['language']=='rebol')['runtime_arguments'].__setitem__(-1,'/different/unscored.reb'),
            lambda d,m: next(row for row in d['programs'].values() if row['language']=='rebol')['runtime_load_evidence']['binding']['authored_inputs'].pop(),
            lambda d,m: m['rebol'].update(emulation_used=True),
            lambda d,m: m['rebol']['execution_model_review']['profile_ids'].pop(),
            lambda d,m: m['rebol']['interpreted_vm_evidence'].update(native_aot_proven=True),
            lambda d,m: m['rebol']['measured_backend_template'].update(key='native-aot',execution='aot'),
            lambda d,m: m.update(red=deepcopy(m['rebol'])),
            lambda d,m: d['languages'].append(dict(key='red',execution='Interpreted')),
            lambda d,m: d['languages'].append(dict(key='v',execution='AOT')),
        ]
        for change in mutations:
            with self.subTest(change=change):
                data,meta,actual,pins=eight_platform(); change(data,meta); before=deepcopy(data)
                with self.assertRaises((ValueError,KeyError)): platform.stage(data,meta,actual,pins)
                self.assertEqual(data,before)
        data,meta,actual,pins=eight_platform(); changes,_,_=platform.stage(data,meta,actual,pins)
        for key,value in changes.items(): data['programs'][key]['measured_backend']=value
        next(row for row in data['programs'].values() if row['language']=='prolog')['measured_backend']['verified']=False
        before=deepcopy(data)
        with self.assertRaises(ValueError): groups.add_native_language_groups(data)
        self.assertEqual(data,before)
