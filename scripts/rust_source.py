"""Small Rust lexical helpers for architecture checks, preserving source offsets."""

import re

RAW_LITERAL = re.compile(r'(?:br|rb|r)(?P<hashes>#{0,255})"')
CHAR_LITERAL = re.compile(
    r"(?:b)?'(?:\\(?:x[0-9A-Fa-f]{2}|u\{[0-9A-Fa-f_]+\}|[^\r\n])|[^\\'\r\n])'"
)


def rust_code(source: str) -> str:
    """Blank comments and literals while preserving useful token boundaries."""
    output = list(source)
    size = len(source)

    def blank(start: int, end: int) -> None:
        for offset in range(start, end):
            if output[offset] not in "\r\n":
                output[offset] = " "

    index = 0
    while index < size:
        if source.startswith("//", index):
            end = source.find("\n", index + 2)
            end = size if end < 0 else end
            blank(index, end)
            index = end
            continue
        if source.startswith("/*", index):
            depth, end = 1, index + 2
            while end < size and depth:
                if source.startswith("/*", end):
                    depth += 1
                    end += 2
                elif source.startswith("*/", end):
                    depth -= 1
                    end += 2
                else:
                    end += 1
            blank(index, end)
            index = end
            continue
        raw = RAW_LITERAL.match(source, index)
        if raw:
            delimiter = '"' + raw.group("hashes")
            end = source.find(delimiter, raw.end())
            end = size if end < 0 else end + len(delimiter)
            blank(index, end)
            index = end
            continue
        character = CHAR_LITERAL.match(source, index)
        if character:
            blank(index, character.end())
            index = character.end()
            continue
        prefix = 1 if source.startswith(('b"', "b'"), index) else 0
        quote = index + prefix
        if quote < size and source[quote] == '"':
            end, escaped = quote + 1, False
            while end < size:
                character = source[end]
                end += 1
                if character == '"' and not escaped:
                    break
                escaped = character == "\\" and not escaped
                if character != "\\":
                    escaped = False
            blank(index, end)
            index = end
            continue
        index += 1
    return "".join(output)

