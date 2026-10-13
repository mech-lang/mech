"""Bounded MATLAB-owned source units for the two corrected regex functions.

Quote/transpose, comments, array/argument delimiters and function/for/end belong
to MATLAB. Only neutral owner accounting comes from the qualified foreign_core.
Unsupported readers/callbacks/control scopes retain diagnostics; no admission.
"""
import re

from foreign_core import count_owned, replacement

VERSION = 'matlab-regexp-owned-source-units-20261012-v1'
IDENT = r'[A-Za-z][A-Za-z_0-9]*'
ASSIGN = re.compile(r'(?<![=<>~])=(?!=)')
UNSUPPORTED_SCOPES = {'if', 'while', 'switch', 'try', 'parfor', 'spmd', 'classdef',
    'properties', 'methods', 'events', 'enumeration', 'arguments'}
UNSUPPORTED_BRANCHES = {'else', 'elseif', 'case', 'otherwise', 'catch'}
READERS = {'eval', 'evalin', 'feval', 'run', 'str2func'}


def issue(kind, offset, message):
    return dict(kind=kind, offset=offset, message=message)


def event(kind, start, end, scope, units=1):
    return dict(kind=kind, start=start, end=end, scope=scope, units=units)


def mask_matlab(source):
    """Preserve all source coordinates without using another language lexer."""
    chars = list(source)
    literals, transposes, continuations, diagnostics = [], [], [], []
    index, block = 0, None
    if source.startswith('\ufeff'): chars[0] = ' '
    while index < len(source):
        line_start = source.rfind('\n', 0, index) + 1
        line_end = source.find('\n', index)
        if line_end < 0: line_end = len(source)
        if block is not None:
            row = source[line_start:line_end].strip()
            if row == '%{':
                diagnostics.append(issue('matlab-nested-block-comment-review', index,
                    'Nested block comments are outside the selected bounded reader.'))
            chars[index:line_end] = replacement(source[index:line_end])
            if row == '%}': block = None
            index = line_end + (line_end < len(source))
            continue
        if source.startswith('%{', index) and source[line_start:line_end].strip() == '%{':
            block = index
            chars[index:line_end] = replacement(source[index:line_end])
            index = line_end + (line_end < len(source))
            continue
        if source[index] == '%':
            if source[line_start:line_end].strip() == '%}':
                diagnostics.append(issue('matlab-unmatched-block-comment', index, 'No owned opening block comment.'))
            chars[index:line_end] = replacement(source[index:line_end])
            index = line_end
            continue
        if source.startswith('...', index):
            continuations.append(dict(start=index, end=line_end))
            diagnostics.append(issue('matlab-continuation-scope-review', index,
                'Unselected logical line continuation requires separate owner qualification.'))
            chars[index:line_end] = replacement(source[index:line_end])
            index = line_end
            continue
        quote = source[index]
        if quote not in {'\'', '"'}:
            index += 1
            continue
        if quote == "'":
            previous = source[index - 1] if index else ''
            operand = bool(previous) and (previous.isalnum() or previous in '_)]}\'')
            nonconjugate = previous == '.' and index > 1 and (source[index - 2].isalnum() or source[index - 2] in '_)]}')
            if operand or nonconjugate:
                transposes.append(dict(start=index - 1 if nonconjugate else index, end=index + 1,
                    kind='nonconjugate' if nonconjugate else 'conjugate'))
                index += 1
                continue
            # Whitespace makes juxtaposed array text possible. Outside the
            # selected adjacency cases retain ambiguity rather than consuming
            # an arbitrary transpose as a character-vector opener.
            before = source[:index].rstrip(' \t')
            if previous in ' \t' and before and (before[-1].isalnum() or before[-1] in '_)]}'):
                diagnostics.append(issue('matlab-quote-context-review', index,
                    'Spaced operand/quote requires source-specific transpose or concatenation ownership.'))
        start = index
        index += 1
        closed = False
        while index < len(source) and source[index] not in '\r\n':
            if source[index] == quote:
                if index + 1 < len(source) and source[index + 1] == quote:
                    index += 2
                    continue
                index += 1
                closed = True
                break
            index += 1
        if not closed:
            diagnostics.append(issue('matlab-unclosed-text-literal', start,
                'Text literal must close on its physical source line.'))
        chars[start:index] = replacement(source[start:index], literal=True)
        literals.append(dict(start=start, end=index, quote=quote, closed=closed))
    if block is not None:
        diagnostics.append(issue('matlab-unclosed-block-comment', block, 'No alone-line closing block comment.'))
    return dict(cleaned=''.join(chars), literals=literals, transposes=transposes,
        continuations=continuations, diagnostics=diagnostics)


def delimiters(cleaned):
    contexts, stack, pairs, diagnostics = [], [], {}, []
    wanted = {')': '(', ']': '[', '}': '{'}
    for index, char in enumerate(cleaned):
        contexts.append(tuple(('D', 'args' if opening == '(' else 'data', start) for opening, start in stack))
        if char in '([{': stack.append((char, index))
        elif char in ')]}':
            if not stack or stack[-1][0] != wanted[char]:
                diagnostics.append(issue('matlab-unbalanced-delimiter', index, 'Delimiter has no matching owned opener.'))
            else:
                _, start = stack.pop(); pairs[start] = index
    diagnostics.extend(issue('matlab-unclosed-delimiter', start, 'Written delimiter remains unclosed.') for _, start in stack)
    return contexts, pairs, diagnostics


def segments(cleaned, contexts):
    start = 0
    for index, char in enumerate(cleaned):
        if not contexts[index] and char in ';,\n':
            if cleaned[start:index].strip(): yield start, index
            start = index + 1
    if cleaned[start:].strip(): yield start, len(cleaned)


def classify(source, masked):
    cleaned = masked['cleaned']
    contexts, pairs, diagnostics = delimiters(cleaned)
    diagnostics = list(masked['diagnostics']) + diagnostics
    events, scopes, stack = [], [], []
    # Neutral accounting splits semicolons; MATLAB's top-level command comma
    # is also a real boundary. Array rows/columns and call/formal commas stay data.
    for index, char in enumerate(cleaned):
        if char == ',' and not contexts[index]:
            events.append(event('separator', index, index + 1, 'matlab-command-separator', 0))
        if char == '@':
            diagnostics.append(issue('matlab-callback-owner-review', index,
                'Named/anonymous function handles require separately qualified callable ownership.'))
        if char == '!':
            diagnostics.append(issue('matlab-external-reader-review', index, 'Shell/external reader is unsupported.'))
    for opening, closing in pairs.items():
        if cleaned[opening] not in '[{': continue
        body = cleaned[opening + 1:closing]
        if ASSIGN.search(body) or re.search(IDENT + r'\s*\(', body) or '@' in body:
            diagnostics.append(issue('matlab-array-effect-owner-review', opening,
                'A data initializer cannot hide assignment, an unresolved call/index owner or a callback.'))
    for match in re.finditer(r'\b(' + '|'.join(sorted(READERS)) + r')\s*(?:\(|\s)', cleaned):
        diagnostics.append(issue('matlab-dynamic-reader-review', match.start(),
            'Dynamic source/function dispatch needs explicit source ownership.'))
    for left, right in segments(cleaned, contexts):
        value = cleaned[left:right].strip()
        start = left + len(cleaned[left:right]) - len(cleaned[left:right].lstrip())
        end = right - len(cleaned[left:right]) + len(cleaned[left:right].rstrip())
        first = re.match(IDENT, value)
        word = first[0] if first else ''
        if word == 'function':
            output = r'(?:(?:' + IDENT + r'|\[\s*' + IDENT + r'(?:\s*,\s*' + IDENT + r')*\s*\])\s*=\s*)?'
            formals = r'(?:\(\s*(?:' + IDENT + r'(?:\s*,\s*' + IDENT + r')*)?\s*\))?'
            if re.fullmatch(r'function\s+' + output + IDENT + r'\s*' + formals, value) is None:
                diagnostics.append(issue('matlab-function-header-review', start, 'Unproved output/formal/default/signature owner.'))
            owner = dict(kind='function', start=start, header_end=end, end=None)
            scopes.append(owner); stack.append(owner)
            events.append(event('header', start, end, 'matlab-function'))
        elif word == 'for':
            if re.fullmatch(r'for\s+' + IDENT + r'\s*=(?!=)\s*.+', value, re.S) is None or len(ASSIGN.findall(value)) != 1:
                diagnostics.append(issue('matlab-for-header-review', start,
                    'Only a single written iterator binding is structural in a for header.'))
            owner = dict(kind='for', start=start, header_end=end, end=None)
            scopes.append(owner); stack.append(owner)
            events.append(event('header', start, end, 'matlab-for'))
        elif word == 'end':
            if value != 'end':
                diagnostics.append(issue('matlab-end-owner-review', start, 'Scope close is not a standalone end statement.'))
            if stack:
                owner = stack.pop(); owner['end'] = end
                events.append(event('close', start, end, 'matlab-' + owner['kind']))
            else:
                diagnostics.append(issue('matlab-unmatched-keyword-end', start, 'No written function/control header owns this end.'))
                events.append(event('close', start, end, 'matlab-unmatched-end'))
        elif word in UNSUPPORTED_SCOPES:
            diagnostics.append(issue('matlab-unsupported-control-review', start,
                'This control/type scope is outside the two-source qualified grammar.'))
            owner = dict(kind=word, start=start, header_end=end, end=None)
            scopes.append(owner); stack.append(owner)
            events.append(event('header', start, end, 'matlab-' + word))
        elif word in UNSUPPORTED_BRANCHES:
            diagnostics.append(issue('matlab-unsupported-branch-review', start, 'Unselected branch owner remains gated.'))
            events.append(event('branch', start, end, 'matlab-' + word))
        else:
            for token in re.finditer(r'\b(?:function|for|if|while|switch|try|classdef)\b', value):
                diagnostics.append(issue('matlab-keyword-expression-review', start + token.start(),
                    'Executable keyword scope cannot hide inside an argument/data owner.'))
    for owner in stack:
        diagnostics.append(issue('matlab-unclosed-keyword-scope', owner['start'], 'Explicit function/loop has no written end.'))
    return dict(contexts=contexts, events=events, scopes=scopes, diagnostics=diagnostics)


def binding_counter(*, text, original, context, assignments):
    diagnostics = []
    if len(assignments) > 1:
        diagnostics.append(dict(kind='matlab-multiple-assignment-review', message='MATLAB chained/nested assignments have no qualified owner.'))
    if re.match(r'\s*\[', text) and ASSIGN.search(text):
        diagnostics.append(dict(kind='matlab-output-binding-review', message='Multiple receiving outputs need separate binding ownership qualification.'))
    return dict(diagnostics=diagnostics)


def normalize(source, language='matlab'):
    assert language == 'matlab'
    masked = mask_matlab(source)
    parsed = classify(source, masked)
    result = count_owned(source, masked['cleaned'], parsed['contexts'], parsed['events'], parsed['diagnostics'],
        'matlab', binding_counter=binding_counter, assignment_pattern=ASSIGN)
    result.update(proposal_version=VERSION, proposal_only=True, eligible_for_chart=False,
        literal_spans=masked['literals'], transpose_spans=masked['transposes'], scope_evidence=parsed['scopes'],
        source_owned_reader=True, parser_pass_is_not_semantic_certification=True)
    return result
