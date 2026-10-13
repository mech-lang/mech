"""Unexecuted mask-only drafts. Run/import only inside the common mutex."""

import unittest

from foreign_future_mask import mask_future, mask_nim, mask_pony


class FutureMaskFixtures(unittest.TestCase):
    def check_positions(self, source, result):
        self.assertEqual(len(source), len(result["cleaned"]))
        self.assertEqual(
            [(i, c) for i, c in enumerate(source) if c in "\r\n"],
            [(i, c) for i, c in enumerate(result["cleaned"]) if c in "\r\n"],
        )
        self.assertIsNone(result["normalized_loc"])
        self.assertFalse(result["eligible_for_chart"])

    def test_nim_nested_comment_is_opaque(self):
        source = "let a = 1 #[ x = 2 #[ y = 3 ]# z = 4 ]#\nlet b = 5\n"
        result = mask_nim(source)
        self.check_positions(source, result)
        self.assertEqual(result["diagnostics"], [])
        self.assertNotIn("x = 2", result["cleaned"])
        self.assertIn("let b = 5", result["cleaned"])

    def test_nim_line_comments_preserve_crlf(self):
        source = "var x = 1 # ; y = 2\r\nvar z = 3\r\n"
        result = mask_nim(source)
        self.check_positions(source, result)
        self.assertNotIn("y = 2", result["cleaned"])

    def test_nim_char_and_numeric_suffix_are_distinct(self):
        source = "let c = 'A'\nlet k = 0'u64\nlet h = 0xFF'u8\n"
        result = mask_nim(source)
        self.check_positions(source, result)
        self.assertEqual(result["diagnostics"], [])
        self.assertNotIn("'A'", result["cleaned"])
        self.assertIn("0'u64", result["cleaned"])
        self.assertIn("0xFF'u8", result["cleaned"])

    def test_nim_regular_escaped_quote(self):
        source = 'let x = "a\\\"#;b"\nlet y = 1\n'
        result = mask_nim(source)
        self.check_positions(source, result)
        self.assertEqual(result["diagnostics"], [])
        self.assertIn("let y = 1", result["cleaned"])
        self.assertNotIn("#;b", result["cleaned"])

    def test_nim_generalized_raw_string_doubled_quote(self):
        source = 'let x = re"a""#b\\n"\nlet y = 1\n'
        result = mask_nim(source)
        self.check_positions(source, result)
        self.assertEqual(result["diagnostics"], [])
        self.assertIn("re", result["cleaned"])
        self.assertNotIn("#b", result["cleaned"])
        self.assertIn("let y = 1", result["cleaned"])

    def test_nim_multiline_triple_string_keeps_rows(self):
        source = 'let x = """a\r\n#[ not comment ]#\n"""\nlet y = 1\n'
        result = mask_nim(source)
        self.check_positions(source, result)
        self.assertEqual(result["diagnostics"], [])
        self.assertIn("let y = 1", result["cleaned"])

    def test_nim_pragma_is_retained_as_pending_evidence(self):
        source = "proc rows(job: Job) {.thread.} =\n  discard job\n"
        result = mask_nim(source)
        self.check_positions(source, result)
        self.assertIn("{.thread.}", result["cleaned"])
        self.assertEqual(len(result["directives"]), 1)
        self.assertEqual(result["directives"][0]["count_role"], "pending-semantic-directive-review")

    def test_nim_pragma_quote_does_not_close_pragma(self):
        source = 'proc f() {.importc: "f.}name".}\n'
        result = mask_nim(source)
        self.check_positions(source, result)
        self.assertEqual(result["diagnostics"], [])
        self.assertEqual(result["directives"][0]["text"], '{.importc: "f.}name".}')
        self.assertNotIn("f.}name", result["cleaned"])

    def test_nim_backtick_identifier_keeps_header_position(self):
        source = "proc `+=`(x: var int, y: int) =\n  x = x + y\n"
        result = mask_nim(source)
        self.check_positions(source, result)
        self.assertEqual(result["diagnostics"], [])
        self.assertNotIn("+=", result["cleaned"])
        self.assertEqual(result["spans"][0]["kind"], "quoted-identifier")

    def test_nim_custom_literal_prefix_requires_review(self):
        source = 'let x = custom"#;\\n"\n'
        result = mask_nim(source)
        self.check_positions(source, result)
        self.assertEqual(result["diagnostics"][0]["kind"], "future-nim-custom-literal-review")

    def test_nim_unclosed_comment_is_gated(self):
        source = "let x = 1 #[\nlet y = 2\n"
        result = mask_nim(source)
        self.check_positions(source, result)
        self.assertEqual(result["diagnostics"][0]["kind"], "future-unclosed-comment")

    def test_nim_unclosed_quote_keeps_following_line(self):
        source = 'let x = "oops\nlet y = 2\n'
        result = mask_nim(source)
        self.check_positions(source, result)
        self.assertEqual(result["diagnostics"][0]["kind"], "future-unclosed-string")
        self.assertIn("let y = 2", result["cleaned"])

    def test_pony_identifier_apostrophe_is_not_character(self):
        source = "new val create(word': String, count': U64) =>\n  word = word'\n  count = count'\n"
        result = mask_pony(source)
        self.check_positions(source, result)
        self.assertEqual(result["diagnostics"], [])
        self.assertEqual(result["cleaned"], source)

    def test_pony_character_and_apostrophe_identifier(self):
        source = "let c = 'A'\nlet value' = '\\n'\n"
        result = mask_pony(source)
        self.check_positions(source, result)
        self.assertEqual(result["diagnostics"], [])
        self.assertIn("value'", result["cleaned"])
        self.assertNotIn("'A'", result["cleaned"])

    def test_pony_nested_comment_and_line_comment(self):
        source = "let x = 1 /* a /* b */ c */\r\n// while x do end\nlet y = 2\n"
        result = mask_pony(source)
        self.check_positions(source, result)
        self.assertEqual(result["diagnostics"], [])
        self.assertNotIn("while x", result["cleaned"])
        self.assertIn("let y = 2", result["cleaned"])

    def test_pony_string_comment_marker_is_literal(self):
        source = 'let text = "// /* end */"\nlet x = 1\n'
        result = mask_pony(source)
        self.check_positions(source, result)
        self.assertEqual(result["diagnostics"], [])
        self.assertNotIn("/*", result["cleaned"])
        self.assertIn("let x = 1", result["cleaned"])

    def test_pony_triple_string_preserves_rows(self):
        source = 'let text = """end\n// literal\n"""\nlet x = 1\n'
        result = mask_pony(source)
        self.check_positions(source, result)
        self.assertEqual(result["diagnostics"], [])
        self.assertIn("let x = 1", result["cleaned"])

    def test_pony_partial_capability_and_recover_remain_visible(self):
        source = "fun ref f(x: Array[U8] iso) ? =>\n  let y = x = recover Array[U8] end\n  g(consume y where mode = 1)?\n"
        result = mask_pony(source)
        self.check_positions(source, result)
        self.assertEqual(result["cleaned"], source)
        self.assertEqual(result["diagnostics"], [])

    def test_pony_unclosed_comment_is_gated(self):
        source = "let x = 1 /* never closes\n"
        result = mask_pony(source)
        self.check_positions(source, result)
        self.assertEqual(result["diagnostics"][0]["kind"], "future-unclosed-comment")

    def test_explicit_dispatch_and_crystal_stays_unsupported(self):
        self.assertEqual(mask_future("let x = 1", "nim")["language"], "nim")
        self.assertEqual(mask_future("let x = 1", "pony")["language"], "pony")
        result = mask_future('puts "#{x}"', "crystal")
        self.check_positions('puts "#{x}"', result)
        self.assertEqual(result["diagnostics"][0]["kind"], "future-mask-unsupported-language")


if __name__ == "__main__":
    unittest.main()
