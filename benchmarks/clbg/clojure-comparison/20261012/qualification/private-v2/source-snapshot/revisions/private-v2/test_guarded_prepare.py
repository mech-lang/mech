"""Source-only candidate fixtures; no native subject, hash, signal or download."""
import copy
import json
from pathlib import Path
import unittest

import guarded_prepare as candidate


class CandidateInventory(unittest.TestCase):
    def test_missing_helper_or_substituted_profile_is_rejected(self):
        declared = json.loads((candidate.REVISION_SOURCE / 'profiles.json').read_text())
        self.assertEqual(len(candidate.profiles(declared)), 20)
        for mutation in ('helper', 'identity', 'duplicate'):
            changed = copy.deepcopy(declared)
            if mutation == 'helper':
                changed['profiles'][0]['compiled_source_bundle'].pop()
            elif mutation == 'identity':
                changed['profiles'][0]['namespace'] = 'clbg.nbody.optimized'
            else:
                changed['profiles'][1]['id'] = changed['profiles'][0]['id']
            with self.assertRaises(AssertionError):
                candidate.profiles(changed)

    def test_runtime_classpath_has_only_classes_and_reviewed_vendor_jars(self):
        java = Path('/candidate/jdk/bin/java')
        jars = [Path('/candidate/vendor/' + name + '.jar') for name, version in candidate.ARTIFACTS]
        classes = Path('/candidate/classes/profile')
        command = candidate.runtime_command(java, classes, jars, 'clbg.binarytrees.compact')
        self.assertEqual(command[command.index('-cp') + 1].split(candidate.os.pathsep), list(map(str, [classes, *jars])))
        self.assertEqual(command[-2:], ['clbg.binarytrees.compact', '{n}'])
        self.assertEqual(command[1:1 + len(candidate.JVM_FLAGS)], candidate.JVM_FLAGS)
        with self.assertRaises(AssertionError):
            candidate.runtime_command(java, Path('/candidate/sources'), jars, 'clbg.binarytrees.compact')
        with self.assertRaises(AssertionError):
            candidate.runtime_command(java, classes, jars, 'bad; namespace')

    def test_compiler_expression_quotes_paths_and_binds_exact_two_namespaces(self):
        package = Path('/candidate/source package')
        classes = Path('/candidate/classes/quote"newline\n')
        command = candidate.compile_command(Path('/jdk/bin/java'), package, classes,
            [Path('/vendor/clojure.jar')], 'clbg.pidigits.optimized')
        expression = command[-1]
        self.assertIn(json.dumps(str(classes)), expression)
        self.assertEqual(expression.count('(compile '), 2)
        self.assertIn("(compile 'clbg.support)", expression)
        self.assertIn("(compile 'clbg.pidigits.optimized)", expression)
        self.assertNotIn('\n', expression)
        self.assertEqual(command[-3:-1], ['clojure.main', '-e'])

    def test_exact_twenty_true_gates_reject_bool_like_missing_and_extra(self):
        good = dict(qualification_checks={name: True for name in candidate.GATES},
            qualified=True, full_workload=False, exit_status=0, sampler_wait_status=0,
            pins_unchanged_at_end=True)
        candidate.exact_true_checks(good)
        for mutation in ('missing', 'extra', 'integer', 'false', 'wait_bool'):
            changed = copy.deepcopy(good)
            name = next(iter(candidate.GATES))
            if mutation == 'missing':
                changed['qualification_checks'].pop(name)
            elif mutation == 'extra':
                changed['qualification_checks']['new_gate'] = True
            elif mutation == 'integer':
                changed['qualification_checks'][name] = 1
            elif mutation == 'false':
                changed['qualification_checks'][name] = False
            else:
                changed['sampler_wait_status'] = False
            with self.assertRaises(AssertionError):
                candidate.exact_true_checks(changed)

    def test_exact_two_nbody_source_aliases_preserve_all_other_scored_inputs(self):
        import proof_reuse
        rows = proof_reuse.source_changes(candidate)
        self.assertEqual(sum(row['source'].startswith('revisions/private-v2/') for row in rows), 2)
        for variant in ('compact', 'optimized'):
            before = (candidate.HERE / ('sources/clbg/nbody/' + variant + '.clj')).read_text()
            after = (candidate.REVISION_SOURCE / ('sources/clbg/nbody/' + variant + '.clj')).read_text()
            self.assertEqual(len(after), len(before)+1)
            changed_lines = [(old, new) for old, new in zip(before.splitlines(), after.splitlines()) if old != new]
            self.assertEqual(len(changed_lines), 1)
            self.assertEqual(changed_lines[0][1][:-1], changed_lines[0][0][:-1] + ')')
            self.assertTrue(changed_lines[0][0].endswith(']') and changed_lines[0][1].endswith(']'))

    def test_reuse_requires_every_original24_case_binding_and_exact_safety_gate(self):
        import proof_reuse
        rows, fingerprints, attempts = {}, {}, []
        for identity in proof_reuse.IDENTITIES:
            benchmark = 'binarytrees' if 'binarytrees' in identity else 'fannkuchredux'
            row = dict(id=identity, benchmark=benchmark, key=benchmark+':'+identity, source_sha256='a'*64,
                cwd='/saved/cohort', runtime_environment={'JAVA_HOME':'/saved/jdk'}, command=['/saved/java', identity, '{n}'])
            rows[identity], fingerprints[identity] = row, 'saved-fingerprint:' + identity
            for n in ((0,4,7,9) if benchmark == 'binarytrees' else range(1,9)):
                command = ['/saved/java', identity, str(n)]
                index = len(attempts)+1
                attempts.append(dict(id=identity, benchmark=benchmark, key=row['key'], language='clojure', attempt=index,
                    case='n-'+str(n), workload_n=n, source_sha256='a'*64, cwd=row['cwd'], runtime_environment=row['runtime_environment'],
                    command=command, execution_fingerprint=fingerprints[identity], full_workload=False,
                    preparation_only=True, never_chart_eligible=True, full_performance_run=False,
                    qualification_checks={name:True for name in candidate.GATES}, qualified=True, exit_status=0,
                    sampler_wait_status=0, pins_unchanged_at_end=True, valid_environment=True,
                    seconds=0.1, power_at_start=dict(source='AC Power', low_power_mode=0), power_at_end=dict(source='AC Power', low_power_mode=0),
                    timeout_seconds=120, maximum_rss_mib=10240, peak_process_group_rss_kib=1024,
                    competing_compilers=[], power_changes=[], stdout_bytes=1, stdout_sha256='b'*64,
                    validation=dict(reference_match=True, bytes=1, sha256='b'*64),
                    supervised_native_group=dict(type='native-group-started', pid=index+100, pgid=index+100,
                        command=command, inherited_common_mutex=True)))
        report = dict(status='stopped-unqualified', active=None, proposal_sha256=proof_reuse.FIRST_SHA,
            full_performance_runs=0, chart_admission=False, canonical_LOC_admitted=False, no_automatic_retry=True,
            error_type='AssertionError', error="('Owned command failed', 'compile', 1)", attempts=attempts)
        self.assertEqual(len(proof_reuse.validate_records(candidate, report, rows, fingerprints)),24)
        for mutation in ('omitted-case','duplicate-case','source','command','runtime','fingerprint','bool-gate','bool-ordinal','bool-n','bool-stdout','foreign-group','failed-cleanup'):
            changed=copy.deepcopy(report)
            if mutation=='omitted-case': changed['attempts'].pop()
            elif mutation=='duplicate-case': changed['attempts'][1]=copy.deepcopy(changed['attempts'][0])
            elif mutation=='source': changed['attempts'][0]['source_sha256']='c'*64
            elif mutation=='command': changed['attempts'][0]['command'][0]='/other/java'
            elif mutation=='runtime': changed['attempts'][0]['runtime_environment']={'JAVA_HOME':'/other/jdk'}
            elif mutation=='fingerprint': changed['attempts'][0]['execution_fingerprint']='other'
            elif mutation=='bool-gate': changed['attempts'][0]['qualification_checks']['pins_unchanged']=1
            elif mutation=='bool-ordinal': changed['attempts'][0]['attempt']=True
            elif mutation=='bool-n': changed['attempts'][0]['workload_n']=False
            elif mutation=='bool-stdout': changed['attempts'][0]['stdout_bytes']=True
            elif mutation=='foreign-group': changed['attempts'][0]['supervised_native_group']['pgid']=999
            else: changed['attempts'][0]['cleanup_error']='actual owned cleanup failed'
            with self.subTest(mutation=mutation), self.assertRaises(AssertionError):
                proof_reuse.validate_records(candidate, changed, rows, fingerprints)


if __name__ == '__main__':
    unittest.main()
