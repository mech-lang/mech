#!/usr/bin/env python3
"""Count the reviewed EKF source fixtures; never rewrite runnable source.

This is a lexical counter, not a Rust/Mech parser or a name resolver. A reviewed,
SHA-256-pinned inventory says which identifier spellings unambiguously denote
renamable application symbols in each particular fixture. New source needs a
new inventory review. Literal spellings remain; measure.py classifies names.
"""
from __future__ import annotations

import argparse
from collections import Counter
from dataclasses import dataclass
import hashlib
import json
from pathlib import Path
import re
from typing import Iterator


@dataclass(frozen=True)
class Token:
    kind: str
    text: str
    offset: int


RUST_KEYWORDS = set("as async await break const continue crate dyn else enum extern false fn for if impl in let loop match mod move mut pub ref return self Self static struct super trait true type unsafe use where while abstract become box do final macro override priv typeof unsized virtual yield try union".split())
NUMBER = re.compile(r"(?:0[xX][0-9A-Fa-f_]+|0[bB][01_]+|0[oO][0-7_]+|[0-9][0-9_]*(?:\.(?!\.)(?:[0-9][0-9_]*)?)?(?:[eE][+-]?[0-9][0-9_]*)?)(?:[A-Za-z_][A-Za-z0-9_]*)?")
RAW_STRING = re.compile(r'(?:br|cr|r)(\#*)"')
CHAR = re.compile(r"(?:b)?'(?:[^'\\\r\n]|\\(?:[nrt0\\'\"]|x[0-9A-Fa-f]{2}|u\{[0-9A-Fa-f_]+\}))'")


def ident_start(char: str) -> bool:
    return char == "_" or char.isalpha()


def ident_part(char: str) -> bool:
    return char == "_" or char.isalnum()


def quoted_end(source: str, quote_at: int) -> int:
    pos = quote_at + 1
    while pos < len(source):
        if source[pos] == "\\":
            pos += 2
        elif source[pos] == '"':
            return pos + 1
        else:
            pos += 1
    raise ValueError(f"Unterminated quoted literal at offset {quote_at}")


def tokens(source: str, language: str) -> Iterator[Token]:
    if language not in {"rust", "mech"}:
        raise ValueError(f"Unsupported language: {language}")
    pos = 0
    while pos < len(source):
        start = pos
        char = source[pos]
        if char.isspace():
            while pos < len(source) and source[pos].isspace():
                pos += 1
            yield Token("whitespace", source[start:pos], start)
            continue
        # Mech's structural section underline is syntax, not a line comment.
        if language == "mech" and char == "-":
            line_start = source.rfind("\n", 0, pos) + 1
            line_end = source.find("\n", pos)
            if line_end < 0:
                line_end = len(source)
            line = source[line_start:line_end].strip()
            if len(line) >= 3 and set(line) == {"-"}:
                pos = line_end
                yield Token("structure", source[start:pos].rstrip(), start)
                continue
        if source.startswith("//" if language == "rust" else "--", pos):
            pos = source.find("\n", pos)
            if pos < 0:
                pos = len(source)
            yield Token("comment", source[start:pos], start)
            continue
        if language == "rust" and source.startswith("/*", pos):
            depth = 1
            pos += 2
            while pos < len(source) and depth:
                if source.startswith("/*", pos):
                    depth += 1
                    pos += 2
                elif source.startswith("*/", pos):
                    depth -= 1
                    pos += 2
                else:
                    pos += 1
            if depth:
                raise ValueError(f"Unterminated block comment at offset {start}")
            yield Token("comment", source[start:pos], start)
            continue
        if language == "rust":
            raw = RAW_STRING.match(source, pos)
            if raw:
                closing = '"' + raw.group(1)
                end = source.find(closing, raw.end())
                if end < 0:
                    raise ValueError(f"Unterminated raw string at offset {start}")
                pos = end + len(closing)
                yield Token("literal", source[start:pos], start)
                continue
            character = CHAR.match(source, pos)
            if character:
                pos = character.end()
                yield Token("literal", source[start:pos], start)
                continue
        if char == '"' or (language == "rust" and char in "bc" and source[pos + 1:pos + 2] == '"'):
            quote_at = pos if char == '"' else pos + 1
            pos = quoted_end(source, quote_at)
            yield Token("literal", source[start:pos], start)
            continue
        number = NUMBER.match(source, pos)
        if number:
            pos = number.end()
            yield Token("literal", source[start:pos], start)
            continue
        if ident_start(char):
            pos += 1
            while pos < len(source):
                if ident_part(source[pos]):
                    pos += 1
                elif language == "mech" and source[pos] == "-" and pos + 1 < len(source) and ident_start(source[pos + 1]):
                    pos += 2
                else:
                    break
            word = source[start:pos]
            kind = "keyword" if language == "rust" and word in RUST_KEYWORDS else "identifier"
            yield Token(kind, word, start)
            continue
        # Operators, punctuation, transpose apostrophes, and lifetime marks.
        pos += 1
        yield Token("syntax", source[start:pos], start)

