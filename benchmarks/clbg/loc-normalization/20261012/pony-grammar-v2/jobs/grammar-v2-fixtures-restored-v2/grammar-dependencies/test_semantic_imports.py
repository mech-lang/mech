"""Offset-owned import identity fixtures; filename punctuation stays literal."""
import unittest
from lexical import mask_source
from normalizer import normalize


class SemanticIncludeFixtures(unittest.TestCase):
    def test_explicit_angle_and_quote_header_identity_is_preserved(self):
        for source, header in (('#include <gmp.h>\n', 'gmp.h'),
                               ('# include "gmpxx.h"\n', 'gmpxx.h')):
            row = mask_source(source, 'cpp')
            self.assertEqual(row['diagnostics'], [], row)
            self.assertEqual(len(row['semantic_imports']), 1, row)
            imported = row['semantic_imports'][0]
            self.assertEqual(imported['header'], header)
            self.assertEqual(source[imported['start']:imported['end']], imported['spelling'])
            self.assertEqual(len(row['cleaned']), len(source))

    def test_comments_and_ordinary_strings_never_create_import_proofs(self):
        source = '/* #include <gmp.h> */\nconst char *s="#include <iostream>";\n'
        row = mask_source(source, 'c')
        self.assertEqual(row['diagnostics'], [], row)
        self.assertEqual(row['semantic_imports'], [])
        self.assertEqual(normalize(source, 'c')['normalized_loc'], 1)

    def test_filename_semicolons_and_equals_do_not_create_bindings(self):
        for source in ('#include "a;b=c.h"\n', '#include <a;b=c.h>\n'):
            row = normalize(source, 'cpp')
            self.assertEqual(row['normalized_loc'], 1, row)
            self.assertEqual(row['diagnostics'], [], row)
            self.assertFalse(any(e.get('assignments') for expansion in row['expansions']
                                 for e in expansion['events']))

    def test_macro_include_is_not_an_explicit_header_contract(self):
        row = mask_source('#include HEADER_NAME\n', 'c')
        self.assertEqual(row['semantic_imports'], [])

    def test_directive_with_trailing_comment_preserves_only_its_filename(self):
        row = mask_source('#include <string> // <gmp.h>; x=y\n', 'cpp')
        self.assertEqual([r['header'] for r in row['semantic_imports']], ['string'])
        self.assertNotIn('x=y', row['cleaned'])

    def test_bom_does_not_hide_an_explicit_header_identity(self):
        row = mask_source('\ufeff#include <gmp.h>\n', 'c')
        self.assertEqual(row['diagnostics'], [], row)
        self.assertEqual([r['header'] for r in row['semantic_imports']], ['gmp.h'])


if __name__ == '__main__':
    from audit import timing_boundary
    with timing_boundary():
        unittest.main()
