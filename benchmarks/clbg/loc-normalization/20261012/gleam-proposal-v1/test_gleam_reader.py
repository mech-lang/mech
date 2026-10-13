"""Unexecuted independent selected-form and hidden-owner fixtures."""
import unittest
from gleam_reader import parse_source, walk_owners


class GleamOwners(unittest.TestCase):
    def owners(self, source):
        result = parse_source(source)
        self.assertEqual(result['diagnostics'], [])
        self.assertIsNone(result['canonical_loc'])
        self.assertIs(result['chart_admission'], False)
        return [owner for declaration in result['declarations'] for owner in walk_owners(declaration)]
    def kinds(self, source):
        return [owner['kind'] for owner in self.owners(source)]
    def gate(self, source, kind):
        result = parse_source(source)
        self.assertIn(kind, [issue['kind'] for issue in result['diagnostics']])
        self.assertIsNone(result['canonical_loc'])
        self.assertIs(result['chart_admission'], False)

    def test_import_selectors_and_namespace_aliases_are_semantic_imports(self):
        owners = self.owners('import gleam/order.{Eq, Gt, Lt} import clbg_io as io')
        self.assertEqual([owner['kind'] for owner in owners], ['import', 'import'])
        self.assertEqual(sum(owner['proposed_units'] for owner in owners), 2)

    def test_packed_and_expanded_independent_let_bindings_have_two_owners(self):
        for source in ('fn run() { let a = 1 let b = a + 2 b }',
                       'fn run() {\n let a = 1\n let b = a + 2\n b\n}'):
            kinds = self.kinds(source)
            self.assertEqual(kinds.count('binding'), 2)
            self.assertEqual(kinds.count('expression-statement'), 1)

    def test_function_signature_formals_and_type_returns_are_structural(self):
        owners = self.owners('fn run(values: List(#(Int, Float)), work: fn(Int) -> Int) -> #(Int, Int) { #(1, 2) }')
        function = owners[0]
        self.assertEqual(function['kind'], 'function')
        self.assertEqual(len(function['formals']), 2)
        self.assertEqual([owner['kind'] for owner in owners].count('binding'), 0)

    def test_external_annotation_and_bodyless_ffi_declaration_have_real_owners(self):
        owners = self.owners('@external(erlang, "clbg_io_ffi", "write") pub fn write(value: BitArray) -> Nil')
        self.assertEqual(owners[0]['kind'], 'external-function')
        self.assertEqual(len(owners[0]['formals']), 1)
        self.assertEqual([owner['kind'] for owner in owners].count('external-annotation'), 1)
        self.gate('fn write(value: BitArray) -> Nil', 'gleam-bodyless-function-review')

    def test_generic_external_types_and_custom_constructor_fields_are_structural(self):
        owners = self.owners('pub type Task(value) type Tree { Leaf(Reference) Node(Reference, Tree, Tree) }')
        self.assertEqual([owner['kind'] for owner in owners].count('type-declaration'), 2)
        self.assertEqual([owner['kind'] for owner in owners].count('type-constructor'), 2)
        self.assertEqual([owner['kind'] for owner in owners].count('binding'), 0)

    def test_packed_case_arm_patterns_headers_and_body_values_remain_distinct(self):
        owners = self.owners('fn code(value: Int) { case value { 65 | 97 -> 0 67 | 99 -> 1 _ -> panic as "Invalid" } }')
        arms = [owner for owner in owners if owner['kind'] == 'case-arm']
        self.assertEqual(len(arms), 3)
        self.assertTrue(all(owner['body_expression_units'] == 1 for owner in arms))

    def test_nested_case_initializer_and_arm_block_keep_real_binding_work(self):
        owners = self.owners('fn run(x) { let y = case x { True -> { let a = 1 let b = 2 #(a, b) } False -> #(0, 0) } y }')
        kinds = [owner['kind'] for owner in owners]
        self.assertEqual(kinds.count('case'), 1)
        self.assertEqual(kinds.count('case-arm'), 2)
        self.assertEqual(kinds.count('binding'), 3)
        self.assertEqual(len([owner for owner in owners if owner['kind'] == 'block' and owner['real_statement_count'] == 3]), 1)

    def test_callback_body_inside_constructor_argument_cannot_hide_two_effects(self):
        owners = self.owners('fn run() { let task = process.start(fn() { io.text("one") io.text("two") 7 }) task }')
        self.assertEqual([owner['kind'] for owner in owners].count('anonymous-function'), 1)
        callback = next(owner for owner in owners if owner['kind'] == 'anonymous-function')
        self.assertEqual(callback['children'][0]['real_statement_count'], 3)

    def test_nested_worker_callbacks_and_dict_maps_remain_gleam_owned(self):
        owners = self.owners('fn run(sequence) { list.map([1, 2, 3], fn(width) { process.start(fn() { histogram(sequence, width, dict.new()) }) }) }')
        self.assertEqual([owner['kind'] for owner in owners].count('anonymous-function'), 2)
        self.assertEqual([owner['kind'] for owner in owners].count('list'), 1)

    def test_let_assert_destructuring_is_one_binding_from_one_producer(self):
        owners = self.owners('fn run() { let assert [one, two, ..rest] = counts let #(line, lines) = produce() #(one, line) }')
        bindings = [owner for owner in owners if owner['kind'] == 'binding']
        self.assertEqual(len(bindings), 2)
        self.assertIs(bindings[0]['asserted'], True)
        self.assertIs(bindings[1]['asserted'], False)

    def test_tuple_projection_list_spread_and_record_update_arguments_are_structural(self):
        owners = self.owners('fn run(body, rest, total) { [Body(..body, x: total.0 +. 0.01 *. body.vx), ..rest] }')
        self.assertEqual([owner['kind'] for owner in owners].count('binding'), 0)
        self.assertEqual([owner['kind'] for owner in owners].count('call'), 1)
        self.assertEqual([owner['kind'] for owner in owners].count('field'), 2)

    def test_bit_array_utf8_bytes_and_default_byte_segments_have_owned_expressions(self):
        owners = self.owners('fn run(header, word) { <<header:bytes, 10>> <<word:utf8>> <<">THREE":utf8>> <<>> }')
        self.assertEqual([owner['kind'] for owner in owners].count('bit-array'), 4)
        self.gate('fn run(word) { <<word:unknown>> }', 'gleam-bit-segment-review')

    def test_scalar_grouping_braces_are_visible_for_owner_review_not_assumed_data(self):
        owners = self.owners('fn run(n) { let x = n * { n + 1 } / 2 x }')
        blocks = [owner for owner in owners if owner['kind'] == 'block']
        self.assertEqual(len(blocks), 2)
        self.assertEqual(blocks[1]['real_statement_count'], 1)
        self.assertEqual(blocks[1]['children'][0]['children'][0]['kind'], 'binary')
        hidden = self.owners('fn run() { consume({ let a = mutate() emit(a) a }) }')
        self.assertEqual([owner['kind'] for owner in hidden].count('binding'), 1)
        self.assertTrue(any(owner['kind'] == 'block' and owner['real_statement_count'] == 3 for owner in hidden))

    def test_pipeline_composition_preserves_nested_anonymous_body(self):
        owners = self.owners('fn run(tasks) { tasks |> list.map(fn(task) { process.join(task) }) |> list.each(io.write) }')
        self.assertEqual([owner['kind'] for owner in owners].count('anonymous-function'), 1)
        self.assertEqual(len([owner for owner in owners if owner['kind'] == 'binary' and owner['operator'] == '|>']), 2)

    def test_comments_strings_unicode_and_exact_source_spans_remain_separate(self):
        source = '// let hidden = 2\nfn run() { "let fake = { } \\u{1f34e}" }'
        result = parse_source(source)
        self.assertEqual(result['diagnostics'], [])
        self.assertEqual(len(result['comment_spans']), 1)
        self.assertEqual(len(result['literal_spans']), 1)
        literal = result['literal_spans'][0]
        self.assertEqual(source[literal['start']:literal['end']], '"let fake = { } \\u{1f34e}"')
        self.assertEqual(self.kinds(source).count('binding'), 0)

    def test_unknown_annotation_guard_use_and_malformed_forms_diagnose(self):
        self.gate('@target(javascript) fn run() { 1 }', 'gleam-annotation-review')
        self.gate('fn run(x) { case x { value if value > 0 -> value } }', 'gleam-case-guard-review')
        self.gate('fn run() { use value <- operation() value }', 'gleam-unsupported-statement-owner')
        self.gate('fn run() { let a = 0xG }', 'gleam-number-review')
        self.gate('fn run() { "unclosed }', 'gleam-unclosed-string')
        self.gate('fn run() { 1', 'gleam-unclosed-block')
        self.gate('fn run() { let a = (1 + 2) a }', 'gleam-expression-owner-review')
        self.gate('fn run() { let mutate(value) = producer() value }', 'gleam-pattern-call-review')
        self.gate('fn run() { let a, b = producer() a }', 'gleam-pattern-form-review')
        self.gate('fn run() { let a b = producer() a }', 'gleam-pattern-form-review')

    def test_const_literal_data_never_hides_function_or_runtime_call_work(self):
        self.owners('const alu = "ACGT" const values = [1, 2, 3]')
        self.gate('const data = io.read_all()', 'gleam-executable-const-review')
        self.gate('const callback = fn() { emit() mutate() }', 'gleam-executable-const-review')


if __name__ == '__main__': unittest.main()
