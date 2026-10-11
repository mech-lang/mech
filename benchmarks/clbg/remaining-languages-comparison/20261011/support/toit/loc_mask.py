"""Lexical masking for Toit LOC analysis; comments are blank, literals are S.

The result has exactly the input length and newline positions. Nonempty physical
lines within multiline literals remain nonempty, so their source is still
counted. Literal punctuation and comment text cannot look like statements.
"""


def mask_toit(source: str) -> str:
    result = list(source)
    length = len(source)

    def replace(start, end, symbol):
        for offset in range(start, end):
            if source[offset] not in "\r\n":
                result[offset] = symbol

    def block_end(start):
        offset, depth = start + 2, 1
        while offset < length and depth:
            if source.startswith("/*", offset):
                offset, depth = offset + 2, depth + 1
            elif source.startswith("*/", offset):
                offset, depth = offset + 2, depth - 1
            else:
                offset += 1
        return offset

    def interpolation_end(start):
        offset, depth = start, 1
        while offset < length:
            if source.startswith("//", offset):
                end = source.find("\n", offset)
                offset = length if end < 0 else end
            elif source.startswith("/*", offset):
                offset = block_end(offset)
            elif source[offset] in "\"'":
                offset = literal_end(offset)
            elif source[offset] == "(":
                offset, depth = offset + 1, depth + 1
            elif source[offset] == ")":
                offset, depth = offset + 1, depth - 1
                if not depth:
                    return offset
            else:
                offset += 1
        return offset

    def literal_end(start):
        quote = source[start]
        delimiter = '"""' if quote == '"' and source.startswith('"""', start) else quote
        offset = start + len(delimiter)
        while offset < length:
            if source[offset] == "\\":
                offset = min(length, offset + 2)
            elif quote == '"' and source.startswith("$(", offset):
                offset = interpolation_end(offset + 2)
            elif source.startswith(delimiter, offset):
                return offset + len(delimiter)
            else:
                offset += 1
        return offset

    offset = 0
    while offset < length:
        if source.startswith("//", offset):
            end = source.find("\n", offset)
            end = length if end < 0 else end
            replace(offset, end, " ")
            offset = end
        elif source.startswith("/*", offset):
            end = block_end(offset)
            replace(offset, end, " ")
            offset = end
        elif source[offset] in "\"'":
            end = literal_end(offset)
            replace(offset, end, "S")
            offset = end
        else:
            offset += 1
    return "".join(result)


def _tests():
    cases = [
        ('x := "https://x/*a*/" // comment\ny := 1\n', 'x := ', 'y := 1'),
        ('x := \'\\\'\'; y := \'/\' // rune\n', 'x := ', '; y := '),
        ('/* first\n /* nested */ last */\nx := 1\n', 'x := 1', None),
        ('x := """\n abc; // content\\\n /*literal*/\n """\ny := 2\n', 'x := ', 'y := 2'),
        ('x := "$(foo (\"; /*literal*/\") (bar 1)) ;"\ny := 3\n', 'x := ', 'y := 3'),
        ('#primitive.core.a\nx := #[1, 2] // comment\n', '#primitive.core.a', 'x := #[1, 2]'),
        ('x := "escaped\\\";text"; y := 4', 'x := ', '; y := 4'),
        ('// unterminated comment', None, None),
        ('x := "unterminated\n', 'x := ', None),
    ]
    for source, first, last in cases:
        clean = mask_toit(source)
        assert len(clean) == len(source), (source, clean)
        assert [i for i, c in enumerate(clean) if c in "\r\n"] == [i for i, c in enumerate(source) if c in "\r\n"]
        if first:
            assert first in clean, (source, clean)
        if last:
            assert last in clean, (source, clean)
        assert 'comment' not in clean and 'literal' not in clean
    assert mask_toit('x := "a;b"; y := 2').count(';') == 1
    assert mask_toit('x := "$(f (\";\"))"; y := 2').count(';') == 1
    literal = 'x := """\n abc\n def\n """\n'
    assert sum(bool(s.strip()) for s in mask_toit(literal).splitlines()) == 4
    print('PASS Toit masker: 12 cases, preserved source length/newline positions and literal lines')


if __name__ == '__main__':
    _tests()
