"""Unhooked Smalltalk fixture proposals; execute only under common mutex.

Expectations are manually derived from approved source-unit rules, rather than
copied from parser output. No producer/source/report I/O occurs in these tests.
"""
import unittest

from foreign_smalltalk import attached_unary_suffix,classify_scopes,mask_smalltalk,normalize


class SmalltalkProposalFixtures(unittest.TestCase):
    def assert_units(self,source,expected):
        metric=normalize(source,'smalltalk')
        self.assertEqual(metric['diagnostics'],[],metric['diagnostics'])
        self.assertEqual(metric['normalized_loc'],expected)
        self.assertTrue(metric['proposal_only'])
        self.assertEqual(metric['proposal_status'],'unhooked-no-admission')
        return metric

    def assert_gate(self,source,kind):
        metric=normalize(source,'smalltalk')
        self.assertIn(kind,{issue['kind'] for issue in metric['diagnostics']})
        return metric

    def test_one_expression_blocks_are_explicit_callbacks(self):
        self.assert_units('2 timesRepeat: [1].',3)
        self.assert_units('items collect: [:x | x + 1].',3)
        self.assert_units('[1]',3)

    def test_only_proven_attached_unary_block_invocation_suffix_is_zero(self):
        self.assert_units('[a:=1. a<3] whileTrue.',4)
        self.assert_units('[\na:=1.\na<3\n]\nwhileTrue.',4)
        self.assert_units('[false] whileFalse.',3)
        self.assert_units('[1] value.',3)
        for source in ('[1] size.','[1] value: 2.','[1] value + 1.','[true] whileTrue: [2].'):
            self.assert_gate(source,'foreign-smalltalk-block-surrounding-expression-review')

    def test_exact_unary_selector_span_excludes_keyword_prefixes_and_chains(self):
        source='[1]\n value . next:=1.'
        suffix=attached_unary_suffix(source,source.index(']'))
        self.assertEqual(source[suffix['start']:suffix['end']],'value')
        self.assertEqual(suffix['units'],0)
        self.assert_units(source,4)
        for source in ('[1] value: 2.','[true] whileTrue: [2].','[false] whileFalse: [2].',
                '[1] valueNext.','[1] value resume.','[1] size.','[1] value + 1.'):
            self.assertIsNone(attached_unary_suffix(source,source.index(']')))
            self.assert_gate(source,'foreign-smalltalk-block-surrounding-expression-review')

    def test_keyword_block_invocations_remain_explicitly_gated(self):
        for source in ('[true] whileTrue: [a:=1. b:=2].',
                '[true]\nwhileTrue: [\na:=1.\nb:=2\n].', '[false] whileFalse: [1].','[1] value: 2.'):
            metric=self.assert_gate(source,'foreign-smalltalk-block-surrounding-expression-review')
            self.assertFalse(any(e['scope']=='foreign-smalltalk-unary-block-invocation' for e in metric['canonical_events']))
            self.assertTrue(any('Keyword block invocation' in d['message'] for d in metric['diagnostics']))

    def test_packed_expanded_callback_actions(self):
        self.assert_units('1 to: 2 do: [:i | a := i. b := a + 1].',4)
        self.assert_units('1 to: 2 do: [:i |\na := i.\nb := a + 1\n].',4)

    def test_temporary_names_are_independent_formals_are_structural(self):
        self.assert_units('items collect: [:x | | a b | a := x. b := a + 1. b].',7)
        self.assert_units('items collect: [:x |\n| a b |\na := x.\nb := a + 1.\nb\n].',7)

    def test_nested_blocks_reset_enclosing_argument_owners(self):
        self.assert_units('xs do: [:x | ys do: [:y | a := x. b := y]].',6)
        self.assert_units('self consume: (items collect: [:x | a := x. b := a]).',4)
        self.assert_units('self consume: (items collect: [:x |\na := x.\nb := a\n]\n).',4)

    def test_independent_periods_and_chained_bindings(self):
        self.assert_units('a := b := c := 0.',3)
        self.assert_units('a := 1. b := 2. c := 3.',3)
        self.assert_units('a := 1.\nb := 2.\nc := 3.',3)

    def test_decimal_exponent_periods_are_not_statement_boundaries(self):
        self.assert_units("a := 365.24d0. b := 1.2e-3. c := 'a''b.;[]!'.",3)
        self.assert_units('a := 16rFF. b := 0.5d0.',2)

    def test_character_literals_protect_punctuation(self):
        self.assert_units('a := $.. b := $]. c := $!.',3)
        self.assert_units('a := $". b := $\'.',2)

    def test_literal_arrays_are_data_and_preserve_written_wrapping(self):
        for source in ('a := #(1 2).','a := #[1 2].','a := #([1 2] (3 4)).'):
            metric=self.assert_units(source,1)
            self.assertEqual(metric['blocks'],[])
        metric=self.assert_units('a := #(\n1\n2\n).',4)
        self.assertEqual(metric['blocks'],[])

    def test_category_metadata_is_zero_method_header_body_close_count(self):
        self.assert_units("!C methodsFor: 'testing'!\nf: x\n| a b |\na := x.\n^a\n! !",6)
        self.assert_units("!C methodsFor: 'testing'! f: x | a b | a := x. ^a ! !",6)

    def test_unary_binary_multipart_selector_headers(self):
        self.assert_units("!C methodsFor: 'testing'!\none\n^1!\n+ x\n^x!\nat: x put: y\n^y! !",9)
        self.assert_units("!C class methodsFor: 'testing' stamp: 'abc'!\nat: x\nput: y\n^y! !",3)
        self.assert_units("!C methodsFor: 'general PCRE2 bindings' stamp: 'ab 10/12/2026'!\none\n^1! !",3)

    def test_standard_class_fields_have_explicit_declaration_roles(self):
        self.assert_units("Object subclass: #C instanceVariableNames: 'a b' classVariableNames: '' poolDictionaries: '' category: 'X'!",4)
        self.assert_units("Object subclass: #C\ninstanceVariableNames: 'a b'\nclassVariableNames: ''\npoolDictionaries: ''\ncategory: 'X'!",4)
        self.assert_units("Object subclass: #C instanceVariableNames: 'a' classVariableNames: 'B C' category: 'X'!",5)
        self.assert_units("Object subclass: #C instanceVariableNames: '' classVariableNames: '' category: 'X'!",2)
        self.assert_units("s := 'a b c'.",1)

    def test_approved_independent_cascade_sends_packed_expanded(self):
        self.assert_units("out nextPutAll: 'x'; print: 1; lf.",3)
        self.assert_units("out nextPutAll: 'x';\nprint: 1;\nlf.",3)
        self.assert_units('bodies := OrderedCollection new; add: Body sun; yourself.',3)
        self.assert_units('items do: [:x | out print: x; lf].',4)
        self.assert_units('out print: 1; ifTrue: [2].',4)
        self.assert_units('out print: (self find: [0]); lf.',4)
        self.assert_units('out print: (self find: [0]);\nlf.',4)

    def test_argument_data_composition_does_not_become_c_statements(self):
        self.assert_units('self consume: (out print: 1; lf).',1)
        self.assert_units("a := #(';').",1)

    def test_programming_mask_preserves_offsets_and_lines(self):
        source='\ufeff"comment [ ] ! ."\na := \'quote\'\' ;![]\'. b := $]. c := 1.5d-3.'
        masked=mask_smalltalk(source)
        self.assertEqual(len(masked['cleaned']),len(source))
        self.assertEqual([i for i,c in enumerate(masked['cleaned']) if c=='\n'],[i for i,c in enumerate(source) if c=='\n'])
        self.assertEqual(masked['diagnostics'],[])
        self.assert_units(source,3)

    def test_unproved_roles_require_review(self):
        self.assert_gate('x; y.','foreign-smalltalk-cascade-role-review')
        self.assert_gate('unknown!','foreign-smalltalk-chunk-role-review')
        self.assert_gate('!C methodsFor: S!\nf\n^1! !','foreign-smalltalk-chunk-role-review')
        self.assert_gate("Object subclass: #C instanceVariableNames: 'a-b' category: 'X'!",'foreign-smalltalk-reflective-field-review')
        self.assert_gate("Object subclass: #C instanceVariableNames: 'a'",'foreign-smalltalk-unclosed-file-in-owner-review')
        self.assert_gate('items do: [:x x].','foreign-smalltalk-block-formal-review')
        self.assert_gate('a := {1. 2}.','foreign-smalltalk-dynamic-array-review')
        self.assert_gate('self call: (a := 1. b := 2).','foreign-smalltalk-parenthesized-sequence-review')
        self.assert_gate('items collect: [:x | x] + 1.','foreign-smalltalk-block-surrounding-expression-review')

    def test_only_proven_comment_only_documentation_chunk_is_zero(self):
        self.assert_units('"documentation"!',0)
        self.assert_units('"first"\n"second"!',0)
        self.assert_gate('"documentation" \'value\'!','foreign-smalltalk-chunk-role-review')

    def test_unclosed_comments_strings_characters_delimiters_gate(self):
        self.assert_gate('"unfinished','foreign-unclosed-comment')
        self.assert_gate("'unfinished",'foreign-unclosed-literal')
        self.assert_gate('a := $','foreign-smalltalk-character-review')
        self.assert_gate('a := [1','foreign-unclosed-delimiter')

    def test_language_is_never_silently_aliased(self):
        for language in ('ruby','c','smalltalk-jit','fsharp'):
            with self.assertRaises(ValueError): normalize('a:=1.',language)
            with self.assertRaises(ValueError): classify_scopes('a:=1.',language)


if __name__=='__main__':
    unittest.main()
