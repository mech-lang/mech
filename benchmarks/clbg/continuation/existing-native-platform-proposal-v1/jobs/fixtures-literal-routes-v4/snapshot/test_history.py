"""Meaningful pure legacy selectors/confidence fixtures; not executed."""
import unittest
from qualify_history import observer_binding,record_candidates,samples,historical_pin,native_compile,build_candidates,MissingHistory


class HistoryEvidence(unittest.TestCase):
    def test_runtime_observer_must_use_the_preserved_sample_symbol(self):
        source='''def toolchains():
 commands={'julia':[JULIA,'--version'],'node':[RUNTIME['javascript'],'--version']}
def prepare(language):
 if language=='julia': argv=[JULIA,'--startup-file=no']
 elif language in RUNTIME: argv=[RUNTIME[language],'source']
'''
        self.assertIsNotNone(observer_binding(source,'julia'))
        self.assertIsNotNone(observer_binding(source,'javascript'))
        for changed in (source.replace("'node':[RUNTIME['javascript']","'node':[RUNTIME['ruby']"),source.replace("argv=[JULIA","argv=[OTHER"),source.replace("'--version'","'--not-version'")):
            self.assertIsNone(observer_binding(changed,'javascript') if 'ruby' in changed or '--not-version' in changed else observer_binding(changed,'julia'))
        self.assertIsNone(observer_binding(source.replace("language=='julia'","language=='python'"),'julia'))
        self.assertIsNone(observer_binding(source.replace('language in RUNTIME','language in OTHER'),'javascript'))
        self.assertIsNone(observer_binding(source+" if language=='julia': argv=[JULIA,'unrelated']\n",'julia'))

    def test_resident_selector_requires_the_actual_after_phase(self):
        displayed=dict(id='tree:resident',key='binarytrees:tree:resident',benchmark='binarytrees',language='mech-resident')
        before=dict(source_key='binarytrees:tree',phase='before',benchmark='binarytrees',language='mech-resident')
        after=dict(before,phase='after')
        self.assertEqual(record_candidates(dict(programs={'old':before,'new':after}),displayed),[('new',after)])
        self.assertEqual(record_candidates(dict(programs={'old':before}),displayed),[])
        self.assertEqual(record_candidates(dict(programs={'bad':dict(after,source_key='binarytrees:other')}),displayed),[])

    def test_three_five_seven_samples_are_exact_not_converted(self):
        for values in ([1.,2.,3.],[1.,2.,3.,4.,5.],[1.,2.,3.,4.,5.,6.,7.]):
            displayed=dict(samples=values,seconds=values[len(values)//2]); saved=dict(samples_seconds=values,median_seconds=displayed['seconds'])
            samples(displayed,saved)
            for wrong in (dict(saved,samples_seconds=values[:2]),dict(saved,samples_seconds=[True,*values[1:]]),dict(saved,median_seconds=99.)):
                with self.assertRaises(ValueError):samples(displayed,wrong)

    def test_byte_identity_cannot_be_borrowed_from_equal_basename_or_version(self):
        cmd=['/later/node','source']; pin='a'*64
        measured=dict(saved=dict(command_assets=['/older/node']),origin={},report=dict(asset_hashes={'/older/node':pin},toolchains={'node':'v24.19.0'}))
        self.assertIsNone(historical_pin(measured,cmd))
        measured['saved']['command_assets']=['/later/node']; measured['report']['asset_hashes']['/later/node']=pin
        self.assertEqual(historical_pin(measured,cmd),('/later/node',pin))

    def test_native_link_requires_exact_output_and_cargo_artifact_pin(self):
        linked=['/compiler','-O3','source','-o','/native/after']
        self.assertEqual(native_compile(dict(commands=[['/emitter','source'],linked]),['/native/after']),linked)
        self.assertEqual(native_compile(dict(link=dict(command=linked,returncode=0)),['/native/after']),linked)
        with self.assertRaises(MissingHistory):native_compile(dict(commands=[linked]),['/native/before'])
        with self.assertRaises(MissingHistory):native_compile(dict(link=dict(command=linked)),['/native/before'])
        cargo=['cargo','build','--release','--manifest-path','/original/Cargo.toml']
        with self.assertRaises(MissingHistory):native_compile(dict(compile_command=cargo),['/native/app'])
        self.assertEqual(native_compile(dict(compile_command=cargo,executable_sha256='a'*64),['/native/app']),cargo)

    def test_duplicate_or_wrong_language_record_is_never_a_sample_join(self):
        displayed=dict(id='program',key='nbody:program',benchmark='nbody',language='c')
        saved=dict(id='program',benchmark='nbody',language='c')
        self.assertEqual(len(record_candidates(dict(programs=[saved,dict(saved)]),displayed)),2)
        self.assertEqual(record_candidates(dict(programs=[dict(saved,language='cpp')]),displayed),[])
        self.assertEqual(record_candidates(dict(programs=[dict(saved,benchmark='spectralnorm')]),displayed),[])
        build=dict(binary='/actual/native',binary_sha256='a'*64,build_command=['/compiler','-o','/actual/native'])
        self.assertEqual(build_candidates(dict(programs={'exact':build}),'/actual/native','a'*64),[('/programs/exact',build)])
        self.assertEqual(build_candidates(dict(programs={'wrong':dict(build,binary_sha256='b'*64)}),'/actual/native','a'*64),[])
        self.assertEqual(len(build_candidates(dict(programs=[build,dict(build)]),'/actual/native','a'*64)),2)


if __name__=='__main__':unittest.main()
