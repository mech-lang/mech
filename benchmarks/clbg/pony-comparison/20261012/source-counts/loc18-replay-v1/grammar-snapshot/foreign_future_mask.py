"""Unhooked, unqualified lexical-mask proposal for Nim and Pony.

This module assigns no LOC and makes no chart/source admission decision. It
does not import another language's lexer. Tests and imports require the common
timing mutex. Grammar owners, pragma event policy, qualified source identities,
and Pony recover roles remain separate gates.
"""


def _paint(chars, start, end, marker):
    for offset in range(start, end):
        if chars[offset] not in "\r\n":
            chars[offset] = marker


def _line_end(source, start):
    offset = start
    while offset < len(source) and source[offset] not in "\r\n":
        offset += 1
    return offset


def _block_end(source, start, opening, closing):
    depth = 1
    offset = start + len(opening)
    while offset < len(source):
        if source.startswith(opening, offset):
            depth += 1
            offset += len(opening)
        elif source.startswith(closing, offset):
            depth -= 1
            offset += len(closing)
            if depth == 0:
                return offset, True
        else:
            offset += 1
    return len(source), False


def _quoted_end(source, start, delimiter, *, raw=False, doubled=False):
    offset = start + len(delimiter)
    multiline = len(delimiter) > 1
    while offset < len(source):
        if not multiline and source[offset] in "\r\n":
            return offset, False
        if source.startswith(delimiter, offset):
            if doubled and source.startswith(delimiter + delimiter, offset):
                offset += 2 * len(delimiter)
                continue
            return offset + len(delimiter), True
        if not raw and source[offset] == "\\":
            offset += 2
        else:
            offset += 1
    return len(source), False


def _preceding_word(source, start):
    offset = start
    while offset > 0 and (source[offset - 1].isalnum() or source[offset - 1] == "_"):
        offset -= 1
    return source[offset:start]


def _nim_string_end(source, start):
    if source.startswith('"""', start):
        return _quoted_end(source, start, '"""', raw=True)
    # Generalized string literals (r"...", re"...", or another adjacent
    # identifier) use raw quote rules. The prefix itself remains visible.
    raw = bool(_preceding_word(source, start))
    return _quoted_end(source, start, '"', raw=raw, doubled=raw)


def _nim_pragma_end(source, start):
    offset = start + 2
    while offset < len(source):
        if source.startswith('.}', offset):
            return offset + 2, True
        if source[offset] == '"':
            offset, closed = _nim_string_end(source, offset)
            if not closed:
                return offset, False
        elif source[offset] == "'" and not _preceding_word(source, offset):
            offset, closed = _quoted_end(source, offset, "'")
            if not closed:
                return offset, False
        elif source.startswith('#[', offset):
            offset, closed = _block_end(source, offset, '#[', ']#')
            if not closed:
                return offset, False
        elif source[offset] == '#':
            offset = _line_end(source, offset)
        else:
            offset += 1
    return len(source), False


def _new_result(source, language):
    return {
        "language": language,
        "status": "unqualified-mask-proposal",
        "cleaned": source,
        "spans": [],
        "directives": [],
        "diagnostics": [],
        "normalized_loc": None,
        "eligible_for_chart": False,
    }


def _cover(result, chars, start, end, kind, *, closed=True):
    marker = " " if kind == "comment" else "I" if kind == "quoted-identifier" else "S"
    _paint(chars, start, end, marker)
    result["spans"].append({"start": start, "end": end, "kind": kind, "closed": closed})
    if not closed:
        result["diagnostics"].append({
            "kind": "future-unclosed-" + kind,
            "offset": start,
            "end": end,
        })


def mask_nim(source):
    """Mask declared Nim forms; preserve offsets, lines, and pragma syntax.

    Numeric apostrophe suffixes are retained. Meaningful pragmas are evidence,
    not count events: their header/attachment policy requires separate review.
    Custom generalized-literal prefix behavior is reported for grammar review.
    """
    result = _new_result(source, "nim")
    chars = list(source)
    offset = 0
    while offset < len(source):
        start = offset
        if source.startswith('#[', offset):
            offset, closed = _block_end(source, offset, '#[', ']#')
            _cover(result, chars, start, offset, "comment", closed=closed)
        elif source[offset] == '#':
            offset = _line_end(source, offset)
            _cover(result, chars, start, offset, "comment")
        elif source.startswith('{.', offset):
            end, closed = _nim_pragma_end(source, offset)
            result["directives"].append({
                "start": start,
                "end": end,
                "kind": "nim-pragma",
                "text": source[start:end],
                "closed": closed,
                "count_role": "pending-semantic-directive-review",
            })
            if not closed:
                result["diagnostics"].append({
                    "kind": "future-unclosed-nim-pragma", "offset": start, "end": end,
                })
            # Keep punctuation/header evidence visible; the normal loop masks
            # any comments and literals inside this retained pragma span.
            offset += 2
        elif source[offset] == '"':
            prefix = _preceding_word(source, offset)
            offset, closed = _nim_string_end(source, offset)
            _cover(result, chars, start, offset, "string", closed=closed)
            if prefix and prefix not in {"r", "R", "re"}:
                result["diagnostics"].append({
                    "kind": "future-nim-custom-literal-review",
                    "offset": start - len(prefix), "prefix": prefix,
                })
        elif source[offset] == "'":
            word = _preceding_word(source, offset)
            if word and word[0].isdigit():
                # 0'u64, 1'i64, and 0xFF'u8 are numeric tokens, not chars.
                offset += 1
            elif word:
                result["diagnostics"].append({
                    "kind": "future-nim-apostrophe-context-review", "offset": start,
                })
                offset += 1
            else:
                offset, closed = _quoted_end(source, offset, "'")
                _cover(result, chars, start, offset, "character", closed=closed)
        elif source[offset] == '`':
            offset, closed = _quoted_end(source, offset, '`', raw=True)
            _cover(result, chars, start, offset, "quoted-identifier", closed=closed)
        else:
            offset += 1
    result["cleaned"] = "".join(chars)
    return result


def mask_pony(source):
    """Mask Pony comments/literals without consuming apostrophe identifiers.

    Partial markers, capabilities, =>, receiver chains, controls, and recover
    remain visible for a future Pony-owned grammar pass. This is no lexer or
    source qualification certificate.
    """
    result = _new_result(source, "pony")
    chars = list(source)
    offset = 0
    while offset < len(source):
        start = offset
        if source.startswith('//', offset):
            offset = _line_end(source, offset)
            _cover(result, chars, start, offset, "comment")
        elif source.startswith('/*', offset):
            offset, closed = _block_end(source, offset, '/*', '*/')
            _cover(result, chars, start, offset, "comment", closed=closed)
        elif source.startswith('"""', offset):
            offset, closed = _quoted_end(source, offset, '"""', raw=True)
            _cover(result, chars, start, offset, "string", closed=closed)
        elif source[offset] == '"':
            offset, closed = _quoted_end(source, offset, '"')
            _cover(result, chars, start, offset, "string", closed=closed)
        elif source[offset] == "'":
            if offset > 0 and (source[offset - 1].isalnum() or source[offset - 1] in "_'"):
                # word', word'', and parameter' retain identifier punctuation.
                offset += 1
            else:
                offset, closed = _quoted_end(source, offset, "'")
                _cover(result, chars, start, offset, "character", closed=closed)
        else:
            offset += 1
    result["cleaned"] = "".join(chars)
    return result


def mask_future(source, language):
    """Explicit dispatch only; Crystal remains unsupported by this draft."""
    if language == "nim":
        return mask_nim(source)
    if language == "pony":
        return mask_pony(source)
    result = _new_result(source, language)
    result["diagnostics"].append({"kind": "future-mask-unsupported-language", "language": language})
    return result
