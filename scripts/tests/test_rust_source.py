import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from rust_source import rust_code


class RustSourceTests(unittest.TestCase):
    def assert_masked_region(self, non_code):
        before = "fn α<'a>(value: &'a str) {}\r\n"
        after = "\r\nfn r#visible() {}"
        self.assertEqual(
            rust_code(before + non_code + after),
            before + "".join(ch if ch in "\r\n" else " " for ch in non_code) + after,
        )

    def test_comments_preserve_offsets_newlines_and_following_code(self):
        for comment in (
            "// hidden\r\n",
            "/* hidden /* nested */ comment */",
            "/* \"quotes\" // line marker\r\n /* nested */ tail */",
        ):
            with self.subTest(comment=comment):
                self.assert_masked_region(comment)

    def test_strings_and_characters_preserve_offsets_and_following_code(self):
        for literal in (
            '"hidden"',
            'b"hidden"',
            'c"hidden"',
            '"escaped \\" quote // hidden"',
            '"trailing \\\\"',
            '"continued \\\n hidden"',
            "'{'", "b'}'", "'λ'", "'\"'", "b'\"'",
            "'\\''", "b'\\''", "'\\\\'", "'\\n'", "'\\0'",
            "'\\u{2764}'", "'\\u{1_F980}'", "b'\\x7f'",
        ):
            with self.subTest(literal=literal):
                self.assert_masked_region(literal)

    def test_raw_literals_preserve_offsets_unicode_and_delimiter_boundaries(self):
        for prefix in ("r", "br", "rb", "cr"):
            for hashes in ("", "#", "###", "#" * 255):
                with self.subTest(prefix=prefix, hashes=len(hashes)):
                    content = "hidden // /* λ\r\n\\"
                    if hashes:
                        content += '"' + hashes[:-1] + " still hidden"
                    self.assert_masked_region(f'{prefix}{hashes}"{content}"{hashes}')

    def test_lifetimes_raw_identifiers_and_code_punctuation_are_unchanged(self):
        source = (
            "fn r#read<'a, 'static, 'α>(input: &'a str) -> &'a str {}\n"
            "'label: loop { break 'label; }\n"
            "r#mech_engine::r#expressions::OldSurface; &raw const x;"
        )
        self.assertEqual(rust_code(source), source)

    def test_masking_keeps_adjacent_tokens_separate(self):
        self.assertEqual(rust_code("left/**/right"), "left    right")
        self.assertEqual(rust_code('left"text"right'), "left      right")

    def test_unterminated_comments_and_strings_are_masked_to_end(self):
        for non_code in (
            "// hidden", "/* outer /* nested */ hidden", '"hidden\\',
            'b"hidden', 'c"hidden', 'r###"hidden"##', 'cr#"hidden',
        ):
            with self.subTest(non_code=non_code):
                self.assertEqual(rust_code("code " + non_code), "code " + " " * len(non_code))

    def test_masking_does_not_copy_unconsumed_source_suffixes(self):
        class BorrowedSource(str):
            def __getitem__(self, key):
                if isinstance(key, slice) and key.start is not None and key.stop is None:
                    raise AssertionError("masking copied the unconsumed source suffix")
                return super().__getitem__(key)

        code = "fn borrowed<'a>(value: &'a str) -> &'a str { value }\n" * 128
        literal = 'br###"hidden\nλ"###'
        source = BorrowedSource(code + literal + "\nvisible")
        self.assertEqual(
            rust_code(source),
            code + "".join(ch if ch == "\n" else " " for ch in literal) + "\nvisible",
        )


if __name__ == "__main__":
    unittest.main()
