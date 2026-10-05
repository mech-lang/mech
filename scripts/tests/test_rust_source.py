import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from rust_source import rust_code


class RustSourceTests(unittest.TestCase):
    def test_comments_and_literals_preserve_offsets_and_newlines(self):
        for source in [
            '// hidden\nvisible',
            '/* hidden /* nested */ comment */ visible',
            'r###"hidden"### visible',
            'br##"hidden"## visible',
            'b"hidden" visible',
            '"escaped \\" hidden" visible',
            "'{' b'}' '\\n' '\\u{41}' visible",
        ]:
            with self.subTest(source=source):
                code = rust_code(source)
                self.assertEqual(len(code), len(source))
                self.assertNotIn("hidden", code)
                self.assertEqual(code.index("visible"), source.index("visible"))
                self.assertEqual([i for i, ch in enumerate(code) if ch == "\n"], [i for i, ch in enumerate(source) if ch == "\n"])

    def test_lifetimes_and_raw_identifiers_are_code(self):
        source = "fn r#read<'a>(input: &'a str) {}"
        self.assertEqual(rust_code(source), source)


if __name__ == "__main__":
    unittest.main()
