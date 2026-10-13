"""Offset-preserving lexical masks for the legacy benchmark LOC audit.

This module identifies comments and literal text only. It does not decide whether
an assignment, declaration, separator, or control header adds normalized LOC.
Offsets refer to Python string characters, not encoded UTF-8 byte offsets.
"""

import re

from pygments.lexers import get_lexer_by_name
from pygments.token import Comment, Error, Literal, Name, Punctuation, Whitespace


LEXERS = {
    "c": "c", "gcc": "c", "clang": "c",
    "c++": "cpp", "cpp": "cpp", "g++": "cpp", "gpp": "cpp",
    "rust": "rust", "go": "go", "java": "java", "chapel": "chapel",
    "julia": "julia", "lua": "lua", "luajit": "lua",
    "python": "python", "python3": "python", "pypy": "python",
    "ruby": "ruby", "perl": "perl", "php": "php",
    "r": "splus",
    "javascript": "javascript", "js": "javascript", "node": "javascript",
    "nodejs": "javascript",
}


def _replacement(text, literal=False):
    """Retain newlines, whitespace, offsets, and nonblank literal source lines."""
    return "".join(c if c.isspace() else "S" if literal else " " for c in text)


def _location(source, index):
    return {
        "offset": index,
        "line": source.count("\n", 0, index) + 1,
        "column": index - source.rfind("\n", 0, index),
    }


_JULIA_CHAR = re.compile(
    r"'(?:[^'\\\r\n]|\\(?:[abefnrtv0'\"\\]|x[0-9a-fA-F]{1,2}|"
    r"u[0-9a-fA-F]{1,4}|U[0-9a-fA-F]{1,8}|[0-7]{1,3}))'"
)


def _julia_comments(source):
    """Mask Julia nested block comments before lexing, safely outside literals.

    A lone apostrophe is Julia's adjoint operator, not a string opener. Character
    literals are recognized only when a complete one-character/escape literal
    closes. Double/triple strings and command literals are left for Pygments.
    """
    chars = list(source)
    diagnostics = []
    index = 0
    while index < len(source):
        if source.startswith('#=', index):
            start = index
            depth = 1
            index += 2
            while index < len(source) and depth:
                if source.startswith('#=', index):
                    depth += 1
                    index += 2
                elif source.startswith('=#', index):
                    depth -= 1
                    index += 2
                else:
                    index += 1
            chars[start:index] = _replacement(source[start:index])
            if depth:
                diagnostics.append({
                    "kind": "unterminated-julia-block-comment",
                    **_location(source, start),
                    "message": "Nested Julia #= comment has no matching =#.",
                })
        elif source[index] == '#':
            start = index
            end = source.find('\n', index)
            index = len(source) if end < 0 else end
            chars[start:index] = _replacement(source[start:index])
        elif source[index] in ('"', '`'):
            delimiter = '"""' if source.startswith('"""', index) else source[index]
            start = index
            index += len(delimiter)
            closed = False
            while index < len(source):
                if source[index] == '\\':
                    index = min(index + 2, len(source))
                elif source.startswith(delimiter, index):
                    index += len(delimiter)
                    closed = True
                    break
                else:
                    index += 1
            if not closed:
                diagnostics.append({
                    "kind": "unterminated-julia-literal",
                    **_location(source, start),
                    "message": "Julia string or command literal is unterminated.",
                })
            if re.search(r"\b(?:r|raw)$",source[:start]):
                # Pygments' Julia regex state does not recognize common
                # backslash escapes such as \|. These non-interpolating
                # literal macros are known string syntax, not executable code.
                chars[start:index] = _replacement(source[start:index],literal=True)
        elif source[index] == "'":
            char = _JULIA_CHAR.match(source, index)
            index = char.end() if char else index + 1
        else:
            index += 1
    return ''.join(chars), diagnostics


def _perl_tokens(lexer, source):
    """Repair known Perl lexer states in internal input, never source text.

    Pygments treats every slash-delimited pair as a regex. A slash after a
    complete scalar/numeric/string/array operand or a non-control closing
    parenthesis is instead division. Only a following scalar/numeric operand
    qualifies here. Explicit regex operators, regex arguments and a control
    header followed by a regex statement are left literal. Complex cases keep
    the existing review path rather than receiving a lexical waiver.
    """
    lexical = source
    adjustments = []
    controls = {'if','unless','while','until','for','foreach','given','when'}
    for _ in range(64):
        tokens = list(lexer.get_tokens_unprocessed(lexical))
        parens = []
        closed = {}
        previous = None
        repaired = False
        for index, token, value in tokens:
            if token in Error and value == '*':
                operator = re.search(r'->[ \t]*[@%$&*]$', lexical[:index])
                end = index + 1
                if operator and (end == len(lexical) or
                        not lexical[end].isalnum() and lexical[end] != '_'):
                    start = operator.start()
                    original = source[start:end]
                    # A complete ->sigil* is standard postfix dereference.
                    # Substitute one ordinary arithmetic expression only in
                    # the lexer's same-width input, then restart its state.
                    # Masked output retains the original operator verbatim.
                    surrogate = '+' + ''.join(c if c.isspace() else ' '
                        for c in lexical[start+1:end-1]) + '0'
                    lexical = lexical[:start] + surrogate + lexical[end:]
                    adjustments.append(dict(kind='recognized-perl-postdereference',
                        offset=start, end=end, source_operator=original,
                        lexical_operator=surrogate,
                        message='Retokenized a complete standard ->sigil* operator through a same-width expression surrogate; original source and output offsets are unchanged.'))
                    repaired = True
                    break
            if token in Punctuation:
                for offset, character in enumerate(value):
                    position = index+offset
                    if character == '(':
                        word = re.search(r'([A-Za-z_]\w*)\s*$', lexical[:position])
                        parens.append(word[1] if word else '')
                    elif character == ')' and parens:
                        closed[position] = parens.pop()
            if token in Literal.String.Regex and value.startswith('/') and previous is not None:
                prior_index, prior_token, prior_value = previous
                prior_end = prior_index+len(prior_value)
                uninterrupted = not lexical[prior_end:index].strip()
                operand = prior_token in Literal.Number or prior_token in Name.Variable or prior_token in Literal.String
                if prior_token in Punctuation and prior_value.endswith((')', ']')):
                    operand = prior_value.endswith(']') or closed.get(prior_end-1) not in controls
                divisor = re.match(r'/\s*(?:\$[A-Za-z_0-9]|\d)', lexical[index:])
                if uninterrupted and operand and divisor:
                    lexical = lexical[:index]+'*'+lexical[index+1:]
                    adjustments.append(dict(kind='recognized-perl-division', offset=index,
                        source_operator='/', lexical_operator='*',
                        message='A complete operand followed by scalar/numeric division is code, not a slash regex. Source/output offsets and source bytes are unchanged.'))
                    repaired = True
                    break
            if token not in Comment and token not in Whitespace:
                previous = (index, token, value)
        if not repaired:
            return lexical, tokens, adjustments, []
    return lexical, list(lexer.get_tokens_unprocessed(lexical)), adjustments, [dict(
        kind='perl-lexical-rewrite-limit',
        message='Bounded Perl lexical-state classification exceeded its limit; source needs review.')]


def mask_source(source, language):
    """Return ``cleaned`` text and review diagnostics without changing offsets.

    Preprocessor directives remain source lines. Lexer Error tokens and any
    uncovered, mismatched, or overlapping token spans require review; callers
    must not silently present those sources as fully audited.
    """
    language = language.lower()
    for suffix in ('-aot', '-jit'):
        language = language.removesuffix(suffix)
    if language == 'mech':
        chars=list(source);diagnostics=[];index=0
        while index<len(source):
            if source.startswith('--',index):
                start=index;end=source.find('\n',index);index=len(source) if end<0 else end
                chars[start:index]=_replacement(source[start:index])
            elif source[index]=='"':
                start=index;index+=1;closed=False
                while index<len(source):
                    if source[index]=='\\':index=min(index+2,len(source))
                    elif source[index]=='"':index+=1;closed=True;break
                    else:index+=1
                chars[start:index]=_replacement(source[start:index],literal=True)
                if not closed:diagnostics.append(dict(kind='unterminated-mech-string',**_location(source,start)))
            else:index+=1
        return {'cleaned':''.join(chars),'diagnostics':diagnostics}
    if language not in LEXERS:
        return {"cleaned": source, "diagnostics": [{
            "kind": "unsupported-language", "language": language,
            "message": "No reviewed legacy lexer mapping is available.",
        }]}
    diagnostics = []
    if language == 'julia':
        prepared, diagnostics = _julia_comments(source)
    else:
        prepared = source
    if prepared.startswith('\ufeff'):
        # UTF-8 BOM is encoding metadata. Retain its source character offset,
        # but do not count it as code or report it as a lexer error.
        prepared=' '+prepared[1:]
    lexer = get_lexer_by_name(LEXERS[language], stripnl=False, ensurenl=False,
                              tabsize=0)
    chars = list(prepared)
    covered = 0
    # Some lexer rules (including C // comments) require a final newline even
    # when a real source file ends at EOF. Supply a virtual lexical terminator
    # without altering the source or output. Clip every token before coverage,
    # diagnostics, and masking so the sentinel never gains a source position.
    source_length = len(prepared)
    lexical_input = prepared if prepared.endswith('\n') else prepared + '\n'
    lexical_adjustments = []
    semantic_imports = []
    # Exact mask provenance, distinct from identifiers spelled S/SSS. This is
    # deliberately a C-family inventory of *masked* string-token regions,
    # not a claim to parse their expression/prefix/concatenation ownership.
    # Future direct-declarator consumers can conservatively gate overlap.
    masked_literal_spans = []
    if language == 'perl':
        lexical_input, tokens, lexical_adjustments, extra_diagnostics = _perl_tokens(lexer,lexical_input)
        diagnostics.extend(extra_diagnostics)
    else:
        tokens = lexer.get_tokens_unprocessed(lexical_input)
    # get_tokens_unprocessed avoids Pygments' newline/tab preprocessing. Every
    # real token span is checked against the original text before masking.
    for index, token, value in tokens:
        if index >= source_length:
            continue
        end = min(index + len(value), source_length)
        value = value[:end - index]
        if index != covered:
            diagnostics.append({
                "kind": "lexer-span-discontinuity", **_location(source, index),
                "message": "Lexer token spans do not cover source consecutively.",
            })
        if lexical_input[index:end] != value:
            diagnostics.append({
                "kind": "lexer-span-mismatch", **_location(source, index),
                "message": "Lexer token value does not match its source span.",
            })
            covered = max(covered, end)
            continue
        covered = max(covered, end)
        if token in Error:
            diagnostics.append({
                "kind": "lexer-error", **_location(source, index),
                "length": len(value), "token": str(token),
                "message": "Lexer could not classify this source span.",
            })
        if token in Comment.PreprocFile:
            # Include filenames are literals, never executable punctuation.
            # Preserve their offset-owned import identity separately so a
            # declaration parser can resolve explicit imported type contracts.
            prefix = prepared[prepared.rfind('\n', 0, index) + 1:index]
            pair = re.fullmatch(r'(<[^>\r\n]+>|"[^"\r\n]+")', value)
            if LEXERS.get(language) in {'c', 'cpp'} and pair and re.fullmatch(r'\s*#\s*include\s*', prefix):
                semantic_imports.append(dict(start=index,end=end,header=value[1:-1],
                    spelling=value,kind='explicit-c-family-include-literal'))
            chars[index:end] = _replacement(value, literal=True)
        elif token in Comment and token not in Comment.Preproc:
            chars[index:end] = _replacement(value)
        elif token in Literal.String:
            chars[index:end] = _replacement(value, literal=True)
            if LEXERS.get(language) in {'c', 'cpp'}:
                if masked_literal_spans and masked_literal_spans[-1]['end'] == index:
                    row = masked_literal_spans[-1]
                    row['end'] = end
                    row['source_text'] += source[index:end]
                    row['token_kinds'].append(str(token))
                else:
                    masked_literal_spans.append(dict(start=index, end=end,
                        kind='literal', source_text=source[index:end],
                        token_kinds=[str(token)],
                        provenance='checked-pygments-string-mask-region'))
    if covered != len(prepared):
        diagnostics.append({
            "kind": "lexer-uncovered-tail", **_location(source, covered),
            "message": "Lexer did not cover the complete source.",
        })
    cleaned = ''.join(chars)
    assert len(cleaned) == len(source)
    assert [i for i, c in enumerate(cleaned) if c in '\r\n'] == [
        i for i, c in enumerate(source) if c in '\r\n'
    ]
    for adjustment in lexical_adjustments:
        adjustment.update(_location(source,adjustment['offset']))
    return {"cleaned": cleaned, "diagnostics": diagnostics,
            'lexical_adjustments':lexical_adjustments,
            'semantic_imports':semantic_imports,
            'masked_literal_spans':masked_literal_spans}
