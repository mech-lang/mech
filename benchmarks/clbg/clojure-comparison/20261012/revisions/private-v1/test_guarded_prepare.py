"""Source-only candidate fixtures; no native subject, hash, signal or download."""
import copy
import json
from pathlib import Path
import unittest

import guarded_prepare as candidate


class CandidateInventory(unittest.TestCase):
    def test_missing_helper_or_substituted_profile_is_rejected(self):
        declared = json.loads((candidate.HERE / 'profiles.json').read_text())
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


if __name__ == '__main__':
    unittest.main()
