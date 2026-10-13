"""Exact C-family mask provenance; proposed fixtures, not yet executed."""
import unittest

from lexical import mask_source


class CLiteralMaskProvenanceFixtures(unittest.TestCase):
    def mask(self, source, language='cpp'):
        result = mask_source(source, language)
        self.assertEqual(result['diagnostics'], [], result)
        self.assertEqual(len(result['cleaned']), len(source))
        self.assertEqual([i for i, c in enumerate(result['cleaned']) if c in '\r\n'],
                         [i for i, c in enumerate(source) if c in '\r\n'])
        for row in result['masked_literal_spans']:
            self.assertEqual(row['source_text'], source[row['start']:row['end']])
            self.assertEqual(row['kind'], 'literal')
            self.assertEqual(result['cleaned'][row['start']:row['end']],
                ''.join(c if c.isspace() else 'S' for c in row['source_text']))
        return result

    def test_actual_s_identifiers_have_no_literal_provenance(self):
        row = self.mask('int S=0,SSS=1;T a(SSS);')
        self.assertEqual(row['masked_literal_spans'], [])
        self.assertEqual(row['cleaned'], 'int S=0,SSS=1;T a(SSS);')

    def test_complete_string_fragments_merge_only_at_adjacent_offsets(self):
        source = 'T a("x\\n;=y");'
        row = self.mask(source)
        self.assertEqual(len(row['masked_literal_spans']), 1)
        self.assertEqual(row['masked_literal_spans'][0]['source_text'], '"x\\n;=y"')
        self.assertTrue(row['masked_literal_spans'][0]['token_kinds'])

    def test_whitespace_between_adjacent_strings_keeps_two_mask_regions(self):
        row = self.mask('T a("x"  "y");')
        self.assertEqual([r['source_text'] for r in row['masked_literal_spans']],
                         ['"x"', '"y"'])

    def test_raw_string_rows_are_preserved_with_exact_region(self):
        source = 'T a(R"tag(x;\ny=1)tag");'
        row = self.mask(source)
        self.assertEqual(len(row['masked_literal_spans']), 1)
        self.assertEqual(row['masked_literal_spans'][0]['source_text'],
                         'R"tag(x;\ny=1)tag"')

    def test_include_filename_uses_separate_import_owner(self):
        source = '#include <vector>\nT a("vector");'
        row = self.mask(source)
        self.assertEqual([r['source_text'] for r in row['masked_literal_spans']],
                         ['"vector"'])
        self.assertEqual(row['semantic_imports'][0]['header'], 'vector')
        self.assertLess(row['semantic_imports'][0]['end'],
                        row['masked_literal_spans'][0]['start'])

    def test_comment_payload_is_never_literal_provenance(self):
        row = self.mask('/* "S;=" */ T a(0); // "x"\n')
        self.assertEqual(row['masked_literal_spans'], [])

    def test_char_literals_retain_their_exact_ownership(self):
        row = self.mask("T a('x'),b('\\n');", 'c')
        self.assertEqual([r['source_text'] for r in row['masked_literal_spans']],
                         ["'x'", "'\\n'"])

    def test_prefix_tokens_are_mask_provenance_not_type_or_value_roles(self):
        source = 'T a(u8"text");'
        row = self.mask(source)
        spans = row['masked_literal_spans']
        self.assertTrue(spans)
        # The lexer may include the encoding prefix in its string token. The
        # consumer needs only exact overlap provenance; it must not promote
        # masked payload into a name or claim full literal grammar ownership.
        quote = source.index('"')
        self.assertTrue(any(r['start'] <= quote < r['end'] for r in spans))


if __name__ == '__main__':
    from audit import timing_boundary
    with timing_boundary():
        unittest.main()
