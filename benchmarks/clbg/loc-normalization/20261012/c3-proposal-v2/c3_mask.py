"""C3 0.8.5 offset-preserving masks; no C lexer alias or chart admission.

Run only under /tmp/mech-clbg-timing.lock. Nested slash-star comments and
backtick raw strings with doubled backticks follow the official tagged lexer.
Documentation contracts remain explicit review gates, never erased as comments.
"""
import re


def replacement(value, literal=False):
    return ''.join(char if char.isspace() else 'S' if literal else ' ' for char in value)


def issue(kind, offset, message):
    return dict(kind=kind, offset=offset, message=message)


def mask_c3(source):
    chars = list(source); diagnostics = []; literals = []; index = 0
    if source.startswith('\ufeff'): chars[0] = ' '; index = 1
    if source.startswith('#!'):
        end = source.find('\n'); end = len(source) if end < 0 else end
        chars[:end] = replacement(source[:end]); index = end
    while index < len(source):
        start = index
        if source.startswith('//', index):
            end = source.find('\n', index); index = len(source) if end < 0 else end
            chars[start:index] = replacement(source[start:index]); continue
        if source.startswith('/*', index):
            depth = 1; index += 2
            while index < len(source) and depth:
                if source.startswith('/*', index): depth += 1; index += 2
                elif source.startswith('*/', index): depth -= 1; index += 2
                else: index += 1
            chars[start:index] = replacement(source[start:index])
            if depth: diagnostics.append(issue('c3-unterminated-block-comment', start, 'Nested comment has no matching close.'))
            continue
        if source.startswith('<*', index):
            end = source.find('*>', index + 2); index = len(source) if end < 0 else end + 2
            diagnostics.append(issue('c3-documentation-contract-review', start, 'Doc contracts can execute compiler checks; exact semantic ownership requires review.'))
            chars[start:index] = replacement(source[start:index], literal=True)
            continue
        encoded = re.match(r'(?:x|b64)([\'"`])', source[index:])
        if encoded:
            diagnostics.append(issue('c3-encoded-literal-review', start, 'Hex/base64 byte literals require exact tagged-grammar validation.'))
            index += len(encoded[0]) - 1
        if source[index:index+1] in ('"', "'", '`'):
            delimiter = source[index]; index += 1; closed = False
            while index < len(source):
                if delimiter == '`' and source.startswith('``', index): index += 2; continue
                if source[index] == delimiter: index += 1; closed = True; break
                if delimiter != '`' and source[index] == '\\': index += min(2, len(source) - index); continue
                if delimiter != '`' and source[index] in '\r\n': break
                index += 1
            chars[start:index] = replacement(source[start:index], literal=True)
            literals.append(dict(start=start, end=index, delimiter=delimiter, complete=closed))
            if not closed: diagnostics.append(issue('c3-unterminated-literal', start, 'Quoted C3 literal has no exact closing delimiter.'))
            continue
        if source[index] in '$#':
            diagnostics.append(issue('c3-compile-time-or-macro-review', index, 'Compile-time/reflection/macro identifiers have no admitted authored-suite owner.'))
        if source[index] == '\x00': diagnostics.append(issue('c3-nul-source-review', index, 'NUL outside a quoted literal cannot be silently ignored.'))
        index += 1
    return dict(cleaned=''.join(chars), diagnostics=diagnostics, literals=literals)
