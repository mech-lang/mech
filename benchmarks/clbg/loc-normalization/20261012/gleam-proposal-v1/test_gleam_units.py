"""Unexecuted canonical-owner positives and anti-hiding corruption fixtures."""
import unittest
from gleam_units import propose_units


class GleamUnitProjection(unittest.TestCase):
    def units(self, source, expected):
        result = propose_units(source)
        self.assertEqual(result['diagnostics'], [])
        self.assertEqual(result['normalized_loc'], expected)
        self.assertIs(result['chart_admission'], False)
        self.assertIs(result['actual_fixtures_qualified'], False)
        for event in result['canonical_events']:
            self.assertLess(event['start'], event['end'])
            self.assertTrue(source[event['start']:event['end']].strip())
            self.assertLessEqual(event['end'], len(source))
        return result

    def test_packed_and_expanded_independent_bindings_and_body_agree(self):
        self.units('fn run() { let a = 1 let b = a + 2 b }', 5)
        self.units('fn run() {\nlet a = 1\nlet b = a + 2\nb\n}', 5)

    def test_let_case_prefix_combines_once_retaining_every_arm_body_close(self):
        result = self.units('fn run(x) { let y = case x { True -> 1 False -> 2 } y }', 9)
        headers = [event for event in result['canonical_events'] if event['kind'] == 'case-header']
        self.assertEqual(len(headers), 1)
        source = 'fn run(x) { let y = case x { True -> 1 False -> 2 } y }'
        self.assertEqual(source[headers[0]['start']:headers[0]['end']], 'let')
        self.units('fn run(x) {\nlet y = case x {\nTrue -> 1\nFalse -> 2\n}\ny\n}', 9)

    def test_distinct_callbacks_never_share_a_header_or_written_close(self):
        result = self.units('fn run() { consume(fn() { first() }, fn() { second() }) }', 8)
        headers = [event for event in result['canonical_events'] if event['kind'] == 'anonymous-function-header']
        self.assertEqual(len(headers), 2)
        self.assertNotEqual(headers[0]['start'], headers[1]['start'])
        self.assertEqual(len([event for event in result['canonical_events'] if event['kind'] == 'written-close']), 3)

    def test_separate_preceding_effect_is_not_combined_with_later_callback(self):
        self.units('fn run() { io.text("before") consume(fn() { io.text("a") io.text("b") }) }', 7)
        self.units('fn run() {\nio.text("before")\nconsume(fn() {\nio.text("a")\nio.text("b")\n})\n}', 7)

    def test_packed_callback_bindings_effects_tail_and_prefix_all_survive(self):
        result = self.units('fn run() { let task = process.start(fn() { let a = 1 let b = 2 emit(a) emit(b) 1 }) task }', 10)
        callback = next(event for event in result['canonical_events'] if event['kind'] == 'anonymous-function-header')
        self.assertEqual(callback['start'], result['original_owner_ast'][0]['children'][0]['children'][0]['start'])
        self.assertEqual(len([event for event in result['canonical_events'] if event['kind'] == 'binding']), 2)

    def test_numeric_grouping_remains_one_outer_expression(self):
        self.units('fn run(n) { let value = n * { n + 1 } / 2 value }', 4)
        self.units('fn run(n) { let value = n * n + n value }', 4)

    def test_nested_block_cannot_hide_multiple_bindings_effects_or_tail(self):
        self.units('fn run() { consume({ let value = mutate() emit(value) value }) }', 7)
        result = propose_units('fn run() { let value = { mutate() } value }')
        self.assertIn('gleam-callable-grouping-review', [issue['kind'] for issue in result['diagnostics']])
        self.assertIsNone(result['normalized_loc'])

    def test_unproved_earlier_callable_operand_is_not_silently_merged(self):
        result = propose_units('fn run() { consume(mutate(), fn() { work() }) }')
        self.assertIn('gleam-prefix-callee-review', [issue['kind'] for issue in result['diagnostics']])
        self.assertIsNone(result['normalized_loc'])

    def test_literal_data_wrapping_and_comment_exclusion_remain_explicit(self):
        result = self.units('fn run() {\nlet values = [\n1,2,\n// fake let hidden = 1\n3,4,\n]\nvalues\n}', 7)
        wrapping = [event for event in result['canonical_events'] if event['kind'] == 'preserved-data-wrapping']
        self.assertEqual(len(wrapping), 3)
        self.units('fn run() { let values = [1,2,3,4] values }', 4)

    def test_type_constructor_headers_and_ffi_directives_are_counted(self):
        self.units('type Tree { Leaf(Reference) Node(Reference, Tree, Tree) }', 4)
        self.units('@external(erlang, "clbg_io_ffi", "write") pub fn write(value: BitArray) -> Nil', 2)

    def test_unsupported_source_never_falls_back_to_a_physical_count(self):
        result = propose_units('fn run() { use value <- operation() value }')
        self.assertTrue(result['diagnostics'])
        self.assertIsNone(result['normalized_loc'])
        self.assertIs(result['chart_admission'], False)


if __name__ == '__main__': unittest.main()
