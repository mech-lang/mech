"""Authoritative Crystal fixture proposals; execution requires timing mutex.

Expected counts follow written headers, independent statements/bindings and
written closes. Tests do not load application files, timings or producer LOC.
This draft has no execution receipt or source-normalization admission claim.
"""
import unittest

from foreign_crystal import classify_scopes,mask_crystal,normalize


class CrystalProposalFixtures(unittest.TestCase):
    def assert_units(self,source,expected):
        metric=normalize(source,'crystal')
        self.assertEqual(metric['diagnostics'],[],metric['diagnostics'])
        self.assertEqual(metric['normalized_loc'],expected)
        self.assertTrue(metric['proposal_only'])
        self.assertEqual(metric['proposal_status'],'unhooked-no-admission')
        return metric

    def assert_gate(self,source,kind):
        metric=normalize(source,'crystal')
        self.assertIn(kind,{item['kind'] for item in metric['diagnostics']})
        return metric

    def test_mask_exact_positions_bom_crlf_comments_and_literals(self):
        source='\ufeff# def fake; end\r\na = "end; # comment"\r\nb = \'/\'\r\n'
        masked=mask_crystal(source)
        self.assertEqual(len(masked['cleaned']),len(source))
        self.assertEqual([i for i,c in enumerate(masked['cleaned']) if c in '\r\n'],[i for i,c in enumerate(source) if c in '\r\n'])
        self.assertEqual(masked['diagnostics'],[])
        self.assert_units(source,2)

    def test_escaped_quotes_characters_do_not_open_scopes(self):
        self.assert_units('a = "\\\"end\\\""; b = \'\\n\'; c = \'\\\'\' ',3)
        self.assert_gate("a = 'too many'",'foreign-crystal-character-review')

    def test_floor_division_is_not_comment_and_slash_division_not_regex(self):
        self.assert_units('a = 9 // 2; b = x / 2; c //= 3',3)
        self.assert_units('value = left / right',1)
        self.assert_units('value = numerator / (denominator // 2 + 1)',1)
        self.assert_units('value = left / middle / right',1)

    def test_regex_literals_have_their_own_prefix_class_and_escape_owners(self):
        self.assert_units('patterns = [/a\\/b/, /[a/b]+/i, /end|do|if/]; value = text.gsub(/x/, "")',2)
        self.assert_units('okay = text =~ /word/',1)
        self.assert_gate('puts /word/', 'foreign-crystal-regex-ownership-review')

    def test_simple_interpolation_and_nested_scalar_literal_remain_composed(self):
        self.assert_units('puts "#{row[0]} #{"%.3f" % (100.0 * row[1] / total)}"',1)
        self.assert_units('puts "#{table[index + 2][key]}\\t#{query}"',1)
        self.assert_units('puts "#{"#{count}"}"',1)

    def test_interpolation_independent_work_and_control_are_gated(self):
        self.assert_gate('puts "#{x = 1}"','foreign-crystal-interpolation-code-review')
        self.assert_gate('puts "#{first(); second()}"','foreign-crystal-interpolation-code-review')
        self.assert_gate('puts "#{if x; y; else; z; end}"','foreign-crystal-interpolation-code-review')
        self.assert_gate('puts "#{rows.map { |x| x }}"','foreign-crystal-interpolation-code-review')

    def test_unclosed_literal_and_interpolation_are_gated(self):
        self.assert_gate('x = "missing', 'foreign-unclosed-literal')
        self.assert_gate('puts "#{value', 'foreign-crystal-unclosed-interpolation')

    def test_percent_heredoc_and_annotations_are_retained_and_gated(self):
        self.assert_gate('x = %q(end;)', 'foreign-crystal-percent-literal-review')
        self.assert_gate('x = <<-TEXT\nend;\nTEXT', 'foreign-crystal-heredoc-review')
        result=self.assert_gate('@[AlwaysInline]\ndef f; 1; end','foreign-crystal-annotation-review')
        self.assertEqual(result['normalized_loc'],4)

    def test_constructor_instance_formals_protected_no_generated_assignments(self):
        self.assert_units('class Node; def initialize(@left : Node? = nil, @right : Node? = nil); end; end',4)
        self.assert_units('class Node\ndef initialize(\n@left : Node? = nil,\n@right : Node? = nil)\nend\nend',4)

    def test_written_properties_independent_names_no_generated_accessors(self):
        self.assert_units('class Node; property left, right : Node?; def initialize(@left : Node? = nil, @right : Node? = nil); end; end',6)
        self.assert_units('class Node\nproperty left : Node?\nproperty right : Node?\ndef initialize(@left : Node? = nil, @right : Node? = nil)\nend\nend',6)
        self.assert_units('getter x, y; setter z, w : Float64',4)

    def test_property_generic_commas_and_initializer_chains_distinct(self):
        self.assert_units('property counts : Hash(String, Int64); property x = y = 1',3)
        self.assert_units('class Point; @x : Float64; @y : Float64; end',4)
        self.assert_gate('property {x, y}', 'foreign-crystal-property-declaration-review')
        self.assert_gate('getter {x, y}', 'foreign-crystal-property-declaration-review')
        self.assert_gate('setter {x, y}', 'foreign-crystal-property-declaration-review')
        self.assert_units('receiver.property { |x| x }',3)
        self.assert_units('receiver.getter { 1 }',3)
        self.assert_units('receiver.setter { |x| x }',3)

    def test_wrapped_method_formals_defaults_and_return_type_one_header(self):
        self.assert_units('def f(x : Int32 = 1, y : Array(Int32) = [1,2]) : Int32; x+y; end',3)
        self.assert_units('def f(\nx : Int32 = 1,\ny : Array(Int32) = [1,2]) : Int32\nx+y\nend',3)

    def test_setter_name_equals_not_body_assignment(self):
        self.assert_units('def self.last=(value); @@last = value; end',3)
        self.assert_units('def self.last=(value)\n@@last = value\nend',3)
        self.assert_units('abstract def value : Int32',1)

    def test_imperative_method_separate_body_statements_packed_expanded(self):
        self.assert_units('def f(x); a=x; b=a+1; b; end',5)
        self.assert_units('def f(x)\na=x\nb=a+1\nb\nend',5)

    def test_tuple_swap_single_producer_and_chained_bindings_expand(self):
        self.assert_units('left, right = right, left; sum, flips = result',2)
        self.assert_units('rand, value = nil, nil',1)
        self.assert_units('a = b = 1; table[k] = probability += p',4)
        self.assert_units('a = b = 1\ntable[k] = probability += p',4)

    def test_braced_callback_explicit_header_body_written_close(self):
        self.assert_units('run { a=1; b=2 }',4)
        self.assert_units('run {\na=1\nb=2\n}',4)
        self.assert_units('rows.each { |row| puts row }',3)
        self.assert_units('Array.new(7) { Hash(String, Int64).new(0) }',3)

    def test_do_callback_explicit_header_body_written_close(self):
        self.assert_units('rows.each do |row|; a=row; b=a+1; end',4)
        self.assert_units('rows.each do |row|\na=row\nb=a+1\nend',4)
        self.assert_units('workers.times do; done.receive; end',3)

    def test_destructured_callback_formals_structural_not_bindings(self):
        self.assert_units('table.each { |(key, probability)| table[key] = total += probability }',4)
        self.assert_units('table.each do |(key, probability)|\ntable[key] = total += probability\nend',4)

    def test_nested_callback_resets_outer_argument_data_ownership(self):
        self.assert_units('consume(Array.new(7) { |index| a=index; b=a+1 })',4)
        self.assert_units('consume(Array.new(7) { |index|\na=index\nb=a+1\n}\n)',4)
        self.assert_units('rows.map { |x| Worker.new { |y| y } }',5)

    def test_real_following_callback_composition_requires_review(self):
        self.assert_gate('rows.map { |x| x }.sort', 'foreign-crystal-callback-composition-review')
        self.assert_gate('consume(Worker.new { 1 }, other())', 'foreign-crystal-callback-composition-review')
        self.assert_gate('rows.map do |x|; x; end.sort', 'foreign-crystal-callback-composition-review')

    def test_hash_tuple_and_named_arguments_are_data_not_blocks(self):
        self.assert_units('value = {"key" => 1, "other" => 2}; return {sum, maximum}',2)
        self.assert_units('done.send({index, counts}); Channel(Tuple(Int32, Hash(UInt64, Int64))).new(7)',2)
        self.assert_units('0.step(to: 9, by: 2)',1)

    def test_literal_and_constructor_wrapping_preserved(self):
        self.assert_units('values = [\n1,2,\n3,4,\n]',4)
        self.assert_units('values = {\n\'a\' => 0.3,\n\'c\' => 0.2,\n}',4)
        self.assert_units('bodies = [\nBody.new(\n1.0,\n2.0),\n]',5)

    def test_controls_comparison_and_condition_assignment_stay_header(self):
        self.assert_units('while match = pattern.match(input, position); position = match.end; end',3)
        self.assert_units('while match = pattern.match(input, position)\nposition = match.end\nend',3)
        self.assert_units('while a==b do; x+=1; end',3)
        self.assert_units('for x in [1,2] do; total+=x; end',3)

    def test_member_end_is_not_keyword_close(self):
        self.assert_units('position = match.end; value = match.begin',2)
        self.assert_units('def f; match.end; end',3)

    def test_if_elsif_else_packed_and_expanded(self):
        self.assert_units('if ready then; a=1; else; a=2; end',5)
        self.assert_units('if ready\na=1\nelse\na=2\nend',5)
        self.assert_units('if a then; x=1; elsif b then; x=2; else; x=3; end',7)

    def test_postfix_controls_header_plus_written_action(self):
        self.assert_units('value=1 if ready; return 2 unless done',4)
        self.assert_units('value=1 if ready\nreturn 2 unless done',4)
        self.assert_units('rows.each { |row| value=false if row <= 4 }',4)
        self.assert_units('def f; rows.each { |row| value=false if row <= 4 }; end',6)
        self.assert_units('def f\nrows.each { |row|\nvalue=false if row <= 4\n}\nend',6)
        self.assert_units('def f; value=false if condition end',4)
        self.assert_units('def f\nvalue=false if condition\nend',4)
        self.assert_units('rows.each { |row| value=false if match.end <= row }',4)

    def test_case_imperative_raise_not_collapsed_as_value_expression(self):
        self.assert_units('case value; when 1 then; 0; when 2 then; 1; else; raise Error.new; end',8)
        self.assert_units('case value\nwhen 1 then 0\nwhen 2 then 1\nelse raise Error.new\nend',8)
        self.assert_gate('answer = if okay; 1; else; 2; end','foreign-crystal-value-control-review')

    def test_begin_open_zero_preserves_mixed_operand_work_and_written_end(self):
        self.assert_units('TABLE = begin; a=1; b=2; b; end',5)
        self.assert_units('TABLE = begin\na=1\nb=2\nb\nend',5)
        self.assert_units('begin; a=1; end',2)

    def test_expression_lambda_without_exec_block_and_explicit_lambda_differ(self):
        self.assert_units('f = ->increment(Int32)',1)
        self.assert_units('f = ->(x : Int32) { x+1 }',3)
        self.assert_units('f = ->(x : Int32)\n{ x+1 }',3)

    def test_ffi_prototypes_default_formals_one_each_lib_close_written(self):
        self.assert_units('lib C; fun add(x : Int32, y : Int32) : Int32; end',3)
        self.assert_units('lib C\nfun add(\nx : Int32,\ny : Int32) : Int32\nend',3)
        self.assert_gate('fun add(x : Int32) : Int32','foreign-crystal-ffi-owner-review')

    def test_macro_body_and_delimiters_are_not_hidden(self):
        result=self.assert_gate('macro make(x); a=1; end','foreign-crystal-macro-review')
        self.assertEqual(result['normalized_loc'],3)
        self.assert_gate('{{ name }}','foreign-crystal-macro-review')
        self.assert_gate('{% if flag %} x {% end %}','foreign-crystal-macro-review')

    def test_unknown_or_unclosed_owners_require_review(self):
        self.assert_gate('end','foreign-unmatched-keyword-close')
        self.assert_gate('def f\na=1','foreign-unclosed-keyword-owner')
        self.assert_gate('{ |x| x }','foreign-crystal-callback-owner-review')

    def test_other_languages_are_not_aliased(self):
        for language in ('ruby','julia','c','nim','pony'):
            with self.assertRaises(ValueError): normalize('x=1',language)
            with self.assertRaises(ValueError): classify_scopes('x=1',language)


if __name__=='__main__':
    unittest.main()
