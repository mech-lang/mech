import copy
import struct
import unittest
import qualify_metadata as q


class MetadataEvidence(unittest.TestCase):
    def test_java_saved_jit_and_classpath_are_required(self):
        row = dict(command=['/opt/homebrew/opt/openjdk/bin/java', '-cp', '/saved/classes', 'app'],
            compile_command=['/opt/homebrew/opt/openjdk/bin/javac', '-d', '/saved/classes', '/saved/app.java'],
            execution_mode='OpenJDK JIT; upstream GraalVM AOT source')
        self.assertEqual(q.java_backend(row), row['command'])
        for change in ({'execution_mode': 'GraalVM NativeAOT'},
                       {'command': ['/saved/native-image']},
                       {'command': ['/opt/homebrew/opt/openjdk/bin/java', '-Xint', '-cp', '/saved/classes', 'app']},
                       {'compile_command': ['/opt/homebrew/opt/openjdk/bin/javac', '-d', '/other/classes', '/saved/app.java']}):
            invalid = copy.deepcopy(row)
            invalid.update(change)
            with self.assertRaises(q.UnresolvedBackend):
                q.java_backend(invalid)
        original = dict(row, source={'paths':['upstream/app.java'],'sha256':'a'*64}, sha256='a'*64)
        for benchmark, sizes, argument in (('binarytrees',[10,21],[]),
                ('nbody',[1000,50000000],['50000000']),('spectralnorm',[100,5500],['{n}']),
                ('revcomp',[1000,25000000],[])):
            before = copy.deepcopy(original)
            before['command'] += argument
            after = copy.deepcopy(before)
            if benchmark != 'revcomp': after['command'] = list(row['command']) + ['{n}']
            lineage = q.compare_retime_java_lineage(after,before,benchmark,{benchmark:sizes})
            self.assertEqual(lineage['original_command'],before['command'])
            self.assertEqual(lineage['measured_command'],after['command'])
            self.assertIs(lineage['old_command_relabelled_as_measured'],False)
        before = copy.deepcopy(original)
        after = copy.deepcopy(before)
        after['command'].append('{n}')
        for mutation in ('flag','classpath','main','wrong-old-n','source','compile','workload-type'):
            old,new,workloads = copy.deepcopy(before),copy.deepcopy(after),{'binarytrees':[10,21]}
            if mutation == 'flag': new['command'].insert(1,'-XX:ActiveProcessorCount=4')
            elif mutation == 'classpath': new['command'][2]='/other/classes'
            elif mutation == 'main': new['command'][3]='different'
            elif mutation == 'wrong-old-n': old['command'].append('22')
            elif mutation == 'source': new['sha256']='b'*64
            elif mutation == 'compile': new['compile_command'][-1]='/other/app.java'
            else: workloads['binarytrees']=[10,True]
            with self.subTest(lineage=mutation), self.assertRaises(ValueError):
                q.compare_retime_java_lineage(new,old,'binarytrees',workloads)
        multiple = copy.deepcopy(row)
        multiple['command'][2]='/saved/classes:/saved/dependency.jar'
        multiple['compile_command']=['/opt/homebrew/opt/openjdk/bin/javac','-cp',multiple['command'][2],
            '-d','/saved/classes','/saved/app.java']
        self.assertEqual(q.java_backend(multiple),multiple['command'])
        for classpath in ('/saved/classes:', '/saved/classes:/saved/classes', 'relative'):
            invalid = copy.deepcopy(multiple)
            invalid['command'][2]=classpath
            with self.assertRaises(q.UnresolvedBackend): q.java_backend(invalid)

    def test_dotnet_host_dll_cannot_be_reclassified_as_native_aot(self):
        runtime = dict(path='/saved/dotnet', asset={'rid': 'osx-arm64'})
        row = dict(command=['/saved/dotnet', '/saved/app.dll', '{n}'], binary='/saved/app.dll',
            build_command=['/saved/dotnet', 'build', '/saved/app.csproj'], build_status='passed', execution_mode='jit')
        self.assertEqual(q.dotnet_backend(row, runtime, False), row['command'])
        with self.assertRaises(q.UnresolvedBackend):
            q.dotnet_backend(row, runtime, True)
        invalid = copy.deepcopy(row)
        invalid['command'][0] = '/another/dotnet'
        with self.assertRaises(q.UnresolvedBackend):
            q.dotnet_backend(invalid, runtime, False)

    def test_native_aot_requires_saved_publish_and_native_command(self):
        runtime = dict(path='/saved/dotnet', asset={'rid': 'osx-arm64'})
        row = dict(command=['/saved/app', '{n}'], binary='/saved/app', build_status='passed', execution_mode='aot',
            build_command=['/saved/dotnet', 'publish', '/saved/app.csproj', '-r', 'osx-arm64', '--self-contained', 'true'])
        self.assertEqual(q.dotnet_backend(row, runtime, True), row['command'])
        for replacement in ('osx-x64', 'false'):
            invalid = copy.deepcopy(row)
            index = 4 if replacement == 'osx-x64' else 6
            invalid['build_command'][index] = replacement
            with self.assertRaises(q.UnresolvedBackend):
                q.dotnet_backend(invalid, runtime, True)

    def test_native_arm64_header_rejects_x86_and_non_native_bytes(self):
        self.assertTrue(q.is_native_arm64(b'\xcf\xfa\xed\xfe' + struct.pack('<I', 0x0100000c)))
        self.assertFalse(q.is_native_arm64(b'\xcf\xfa\xed\xfe' + struct.pack('<I', 0x01000007)))
        self.assertFalse(q.is_native_arm64(b'MZ' + b'\x00' * 20))
        self.assertFalse(q.is_native_arm64(b''))

    def test_changed_partial_nan_or_fabricated_samples_are_rejected(self):
        displayed = dict(samples=[1.0, 3.0, 2.0], seconds=2.0)
        saved = dict(samples_seconds=[1.0, 3.0, 2.0], median_seconds=2.0)
        q.validate_samples(displayed, saved)
        for samples, median in (([1.0, 2.0], 1.5), ([1.0, 2.0, float('nan')], 2.0),
                                ([1.0, 3.0, 2.0], 100.0), ([1.0, 4.0, 3.0], 3.0)):
            with self.assertRaises(ValueError):
                q.validate_samples(displayed, dict(samples_seconds=samples, median_seconds=median))

    def test_exact_inventory_and_preserved_identity_are_required(self):
        programs = {}
        for language, count in q.LANGUAGES.items():
            for ordinal in range(count):
                identity = language + '-' + str(ordinal)
                key = 'work:' + identity
                programs[key] = dict(key=key, id=identity, benchmark='work', language=language)
        data, expected = dict(programs=programs), sorted(programs)
        self.assertEqual(len(q.validate_inventory(data, expected)), 100)
        for invalid in (expected[:-1], expected[:-1] + [expected[0]]):
            with self.assertRaises(ValueError):
                q.validate_inventory(data, invalid)
        changed = copy.deepcopy(data)
        changed['programs'][expected[-1]]['id'] = 'other'
        with self.assertRaises(ValueError):
            q.validate_inventory(changed, expected)
