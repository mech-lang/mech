"""Focused additive ownership assertions; run only in the common-mutex job."""
import unittest

import foreign_nim
import helper_owners


SOURCE = '\n'.join(('type csize = csize_t', 'include gmp/header', 'type Mpz* = mpz_t',
    *('template ' + name + '*(' + formals + ')' + returned + ' = ' + callee +
      '(' + ', '.join(arguments) + ')'
      for name, formals, returned, callee, arguments in helper_owners.FORWARDERS))) + '\n'


class HelperOwnerFixtures(unittest.TestCase):
    def test_typed_declarations_and_independent_bodies(self):
        value = helper_owners.normalize(SOURCE, foreign_nim)
        self.assertEqual((value['physical_loc'], value['normalized_loc']), (13, 23))
        self.assertEqual(len(value['canonical_events']), 23)
        self.assertEqual(sum(event['kind'] == 'header' for event in value['canonical_events']), 10)
        self.assertEqual(sum(event['kind'] == 'statement' for event in value['canonical_events']), 10)
        self.assertEqual([row['units'] for row in value['owner_witnesses']], [1, 1, 1] + [2] * 10)
        self.assertFalse(value['eligible_for_chart'])
        self.assertEqual(value['diagnostics'], [])
        self.assertEqual(len(value['resolved_source_diagnostics']), 11)

    def test_wrapping_keeps_owner_units_and_exact_spans(self):
        wrapped = SOURCE.replace(' = mpz_', ' =\n  mpz_')
        value = helper_owners.normalize(wrapped, foreign_nim)
        self.assertEqual(value['normalized_loc'], 23)
        self.assertEqual(value['physical_loc'], 23)
        for row in value['owner_witnesses'][3:]:
            header, body = row['spans']
            self.assertTrue(wrapped[header['start']:header['end']].startswith('template '))
            self.assertTrue(wrapped[header['start']:header['end']].endswith('='))
            self.assertTrue(wrapped[body['start']:body['end']].startswith(row['upstream_callee'] + '('))
            self.assertLessEqual(header['end'], body['start'])

    def test_wrong_forwarding_and_extra_action_are_rejected(self):
        for changed in (SOURCE.replace('mpz_set_ui(value, input)', 'mpz_set_ui(input, value)'),
                        SOURCE.replace('mpz_init(value)', 'mpz_init(value); mpz_clear(value)'),
                        SOURCE.replace('mpz_init(value)', '(var temporary = value; mpz_init(temporary))')):
            with self.subTest(source=changed), self.assertRaises(ValueError):
                helper_owners.normalize(changed, foreign_nim)

    def test_unknown_include_generic_and_returned_type_are_rejected(self):
        for changed in (SOURCE.replace('include gmp/header', 'include other/header'),
                        SOURCE.replace('value: var Mpz', 'value: untyped', 1),
                        SOURCE.replace(': cint =', ': culong ='),
                        SOURCE + 'template hidden(x: untyped) = x\n'):
            with self.subTest(source=changed), self.assertRaises(ValueError):
                helper_owners.normalize(changed, foreign_nim)

    def test_existing_proc_composition_policy_is_unchanged(self):
        self.assertEqual(foreign_nim.normalize('proc denominator(i,j:int):float64 = float64(i+j)')['normalized_loc'], 1)
        self.assertEqual(foreign_nim.normalize('proc product(x:var seq[float64]) = first(x); second(x)')['normalized_loc'], 3)
        self.assertEqual(foreign_nim.normalize('template f(x:untyped)=x')['review_status'], 'needs-review')


if __name__ == '__main__':
    unittest.main()
