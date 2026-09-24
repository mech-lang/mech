"""Source-count and source-preservation checks; no benchmark machinery."""
import unittest
from pathlib import Path
from measure import measure, classify_rust, ROOT
from lexer import tokens
from prepare_sources import prepare


def token_records(text, lang='rust'):
    return measure(text, lang)[1]


class AuditTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.before, cls.after = prepare()

    def test_mech_audit(self):
        m, _ = measure((ROOT/'selected/ekf.mec').read_text(), 'mech')
        self.assertEqual((m['normalized_characters'], m['original_name_characters'], m['width4_characters']), (1079, 2913, 1628))

    def test_simd_audit(self):
        m, _ = measure((ROOT/'selected/rust_simd.rs').read_text(), 'rust')
        self.assertEqual((m['normalized_characters'], m['original_name_characters'], m['width4_characters']), (5243, 8711, 7196))

    def test_scalar_normalized(self):
        m, _ = measure(self.after, 'rust')
        self.assertEqual((m['normalized_characters'], m['original_name_characters'], m['width4_characters']), (2355, 4142, 3390))

    def test_scalar_numeric_body_unchanged(self):
        def body(s):
            return s[s.index('    let dt = 0.1_f32;'):s.index('    if CHECKED')]
        self.assertEqual(body(self.before), body(self.after))

    def test_scalar_matrix_helpers_and_initialization_unchanged(self):
        start = '#[inline(always)]\nfn matmul('
        self.assertEqual(self.before[self.before.index(start):], self.after[self.after.index(start):])

    def test_block_checkpoint_outside_turn_loop(self):
        self.assertLess(self.after.index('let checkpoint ='), self.after.index('for turn in 0..turns'))
        self.assertIn('state.copy_from_slice(&checkpoint_state)', self.after)
        self.assertIn('covariance.copy_from_slice(&checkpoint_covariance)', self.after)

    def test_literals_keep_spaces_and_comment_markers(self):
        text = '"a // b /* c */ d"'
        m, _ = measure(text, 'rust')
        self.assertEqual(m['normalized_characters'], len(text))
        text = '"a -- b"'
        m, _ = measure(text, 'mech')
        self.assertEqual(m['normalized_characters'], len(text))

    def test_nested_comments_removed(self):
        m, _ = measure('1 /* a /* b */ c */ + 2 // x\n', 'rust')
        self.assertEqual(m['normalized_characters'], 3)

    def test_raw_string_retained(self):
        text = 'r##"a // b \\"##'
        m, _ = measure(text, 'rust')
        self.assertEqual(m['normalized_characters'], len(text))

    def test_attributes_retained(self):
        text = '#[inline(always)]'
        m, _ = measure(text, 'rust')
        self.assertEqual(m['normalized_characters'], len(text))

    def test_original_custom_simd_math_methods(self):
        r = token_records((ROOT/'selected/rust_simd.rs').read_text())
        for word, flags in [('splat', (17,1)), ('sin_cos', (3,1)), ('atan2', (3,1)), ('is_finite', (2,1)), ('scope', (2,1)), ('ZERO', (13,1))]:
            seen = [x['normalizable'] for x in r if x['token']==word]
            self.assertEqual((sum(seen), len(seen)-sum(seen)), flags, word)

    def test_scalar_math_and_sum_are_library_methods(self):
        r = token_records(self.after)
        for word in ['sin', 'cos', 'atan2', 'is_finite', 'sum']:
            seen = [x for x in r if x['token']==word]
            self.assertTrue(seen)
            self.assertTrue(all(not x['normalizable'] for x in seen), word)

    def test_trait_required_names_not_shortened(self):
        r = token_records((ROOT/'selected/rust_simd.rs').read_text())
        for word in ['Output', 'add', 'sub', 'mul', 'div', 'neg']:
            self.assertTrue(all(not x['normalizable'] for x in r if x['token']==word))

    def test_counter_ignores_layout_and_comments(self):
        a, _ = measure('let state = 1; state + 2', 'rust')
        b, _ = measure('let\n state\t = 1; /* excluded */ state\n + 2\n', 'rust')
        self.assertEqual(a, b)

    def test_equal_identifier_occurrence_weight(self):
        a, _ = measure('state+state', 'rust')
        b, _ = measure('covariance+covariance', 'rust')
        self.assertEqual(a['normalized_characters'], b['normalized_characters'])
        self.assertEqual(a['width4_characters'], b['width4_characters'])

    def test_mech_multiword_section_one_identifier(self):
        m, _ = measure('EKF step @compute', 'mech')
        self.assertEqual(m['normalized_characters'], 9)
        self.assertEqual(m['normalized_identifier_occurrences'], 1)

    def test_utf8_not_overcounted(self):
        m, _ = measure('·', 'mech')
        self.assertEqual(m['normalized_characters'], 1)


if __name__ == '__main__':
    unittest.main(verbosity=2)
