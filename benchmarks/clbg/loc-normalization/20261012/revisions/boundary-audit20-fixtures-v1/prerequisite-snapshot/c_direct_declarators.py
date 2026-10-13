"""Unhooked, bounded C++ named direct-declarator proposals.

The caller supplies an unchanged offset-preserving mask, a proven type prefix,
an excluded-context decision, and optional exact-offset name/type resolvers.
This module does not import a lexer or normalizer, read files, rewrite bytes,
collect types, create events, or admit a LOC result. Its counts are proposals.
Calls are callee-agnostic syntactic composition, not a claim of runtime purity.

``type_prefix`` is the existing (end, spelling, evidence) tuple for the stripped,
semicolon-less segment. ``role_resolver(name, absolute_offset)`` returns a dict
with ``role`` equal to ``value`` or ``type`` plus source evidence, or None.
``type_id_resolver(text, absolute_offset)`` may prove a bounded type-id prefix
and return (end, evidence); end is relative to the supplied unchanged text.
``legacy_declarators(text)`` may delegate non-direct parts to the current
_declarators API. No supplied callback may normalize, mask, or mutate its input.

The analyze API always returns an envelope. The identify wrapper returns None
on every gate, so a truthy partial-count/diagnostics object cannot reach the
existing counter. Body/header/closer ownership remains outside this module.
"""


_MAX_DEPTH = 32
_CV = {'const', 'volatile', 'restrict', '__restrict'}
_BUILTIN_WORDS = {'void', 'bool', 'char', 'int', 'float', 'double', '_Bool',
                  '__int128', 'signed', 'unsigned', 'short', 'long',
                  'wchar_t', 'char8_t', 'char16_t', 'char32_t'}
_BUILTIN_TYPES = {
    'void', 'bool', 'char', 'int', 'float', 'double', '_Bool', '__int128',
    'wchar_t', 'char8_t', 'char16_t', 'char32_t', 'signed', 'unsigned',
    'signed char', 'unsigned char', 'signed int', 'unsigned int',
    'short', 'short int', 'signed short', 'signed short int',
    'unsigned short', 'unsigned short int', 'long', 'long int',
    'signed long', 'signed long int', 'unsigned long', 'unsigned long int',
    'long long', 'long long int', 'signed long long', 'signed long long int',
    'unsigned long long', 'unsigned long long int', 'long double',
    'signed __int128', 'unsigned __int128',
}
_CONTROL = {'if', 'else', 'for', 'while', 'do', 'switch', 'case', 'default',
            'return', 'co_return', 'co_await', 'co_yield', 'throw', 'try',
            'catch', 'goto', 'break', 'continue', 'new', 'delete', 'asm',
            '__asm__', 'typedef', 'using', 'class', 'struct', 'union', 'enum',
            'namespace', 'template', 'typename', 'operator', 'decltype',
            'sizeof', 'alignof', 'alignas', 'noexcept', 'typeid', 'requires'}
_CASTS = {'static_cast', 'const_cast', 'reinterpret_cast', 'dynamic_cast'}
_SCALARS = {'true', 'false', 'nullptr'}
_UNARY = {'+', '-', '!', '~', '*', '&', 'not', 'compl'}
_OPERATORS = {
    '||': 2, 'or': 2, '&&': 3, 'and': 3, '|': 4, 'bitor': 4,
    '^': 5, 'xor': 5, '&': 6, 'bitand': 6,
    '==': 7, '!=': 7, 'not_eq': 7,
    '<': 8, '>': 8, '<=': 8, '>=': 8, '<<': 9, '>>': 9,
    '+': 10, '-': 10, '*': 11, '/': 11, '%': 11,
}
_RESERVED_NAMES = (_CONTROL | _CV | _BUILTIN_WORDS | _SCALARS | _CASTS
    | set(_OPERATORS) | {'not', 'compl', 'and_eq', 'or_eq', 'xor_eq',
                        'auto', 'static', 'constexpr', 'consteval', 'constinit',
                        'extern', 'inline', 'register', 'thread_local', 'mutable',
                        'public', 'private', 'protected', 'friend', 'virtual',
                        'explicit', 'concept', 'export'})


def _space(text, at):
    while at < len(text) and text[at].isspace():
        at += 1
    return at


def _name_start(char):
    return char == '_' or 'A' <= char <= 'Z' or 'a' <= char <= 'z'


def _name_char(char):
    return _name_start(char) or '0' <= char <= '9'


def _word(text, at):
    if at >= len(text) or not _name_start(text[at]):
        return None
    end = at + 1
    while end < len(text) and _name_char(text[end]):
        end += 1
    return text[at:end], end


def _qualified(text, at):
    first = _word(text, at)
    if first is None:
        return None
    spelling, end = first
    while True:
        colon = _space(text, end)
        if not text.startswith('::', colon):
            break
        member = _word(text, _space(text, colon + 2))
        if member is None or member[0] in _RESERVED_NAMES:
            return None
        spelling += '::' + member[0]
        end = member[1]
    return spelling, end


def _balanced_end(text, opening):
    closers = {'(': ')', '[': ']', '{': '}'}
    if opening >= len(text) or text[opening] not in closers:
        return None
    stack = [closers[text[opening]]]
    for at in range(opening + 1, len(text)):
        char = text[at]
        if char in closers:
            stack.append(closers[char])
            if len(stack) > _MAX_DEPTH:
                return None
        elif char in ')]}':
            if not stack or char != stack.pop():
                return None
            if not stack:
                return at + 1
    return None


def _parts(text, base):
    """Preserve raw offsets; only balanced ()/[]/{} own inner commas."""
    parts = []
    start = at = 0
    while at < len(text):
        char = text[at]
        if char in '([{':
            end = _balanced_end(text, at)
            if end is None:
                return None
            at = end
            continue
        if char in ')]}':
            return None
        if char == ',':
            parts.append((base + start, text[start:at]))
            start = at + 1
        at += 1
    parts.append((base + start, text[start:]))
    return parts


def _issue(kind, offset, message, **more):
    return dict(kind=kind, offset=offset, message=message, **more)


def _literal_overlap(spans, start, end):
    return any(span['start'] < end and start < span['end'] for span in spans)


def _role(resolver, spelling, at):
    if spelling in _BUILTIN_TYPES:
        return dict(role='type', kind='builtin-type', type_name=spelling,
                    start=at, end=at + len(spelling))
    proof = resolver(spelling, at) if resolver is not None else None
    if proof is None:
        return None
    if (not isinstance(proof, dict) or proof.get('role') not in {'value', 'type'}
            or any(proof.get(flag) for flag in
                   ('conditional', 'dependent', 'macro', 'unresolved', 'shadowed'))):
        return dict(role='unknown', rejected=True, source_evidence=proof)
    return proof


def _type_id(text, base, resolver, type_id_resolver, literal_spans=()):
    """Prove a type prefix without collecting/promoting any named type."""
    if _literal_overlap(literal_spans, base, base + len(text)):
        return None
    if type_id_resolver is not None:
        supplied = type_id_resolver(text, base)
        if supplied is not None:
            if (isinstance(supplied, tuple) and len(supplied) == 2
                    and isinstance(supplied[0], int) and not isinstance(supplied[0], bool)
                    and 0 < supplied[0] <= len(text) and isinstance(supplied[1], dict)
                    and supplied[1].get('kind')
                    and not any(supplied[1].get(flag) for flag in
                                ('conditional', 'dependent', 'macro', 'unresolved', 'shadowed'))):
                return supplied
            return None
    at = _space(text, 0)
    while True:
        word = _word(text, at)
        if word is None or word[0] not in _CV:
            break
        at = _space(text, word[1])
    start = at
    word = _word(text, at)
    if word is None:
        return None
    if word[0] in _BUILTIN_WORDS:
        words = []
        while word is not None and word[0] in _BUILTIN_WORDS:
            words.append(word[0])
            at = word[1]
            word = _word(text, _space(text, at))
        spelling = ' '.join(words)
        if spelling not in _BUILTIN_TYPES:
            return None
        proof = dict(kind='builtin-type', type_name=spelling,
                     start=base + start, end=base + at)
    else:
        named = _qualified(text, at)
        if named is None:
            return None
        proof = _role(resolver, named[0], base + at)
        if proof is None or proof['role'] != 'type':
            return None
        at = named[1]
    while True:
        next_at = _space(text, at)
        word = _word(text, next_at)
        if word is not None and word[0] in _CV:
            at = word[1]
            continue
        if next_at < len(text) and text[next_at] == '*':
            at = next_at + 1
            continue
        break
    next_at = _space(text, at)
    if text.startswith('&&', next_at):
        at = next_at + 2
    elif text[next_at:next_at + 1] == '&':
        at = next_at + 1
    return at, dict(proof, type_id=text[:at], start=base, end=base + at, units=0)


def _effects(text, base):
    """Only written syntax is classified; calls have no semantic effect proof."""
    at = 0
    while at < len(text):
        if text.startswith(('++', '--'), at):
            return _issue('cpp-direct-argument-update-review', base + at,
                          'Written update in an argument/formal cannot be hidden in a name unit.')
        if text.startswith(('<<=', '>>=', '+=', '-=', '*=', '/=', '%=', '&=', '|=', '^='), at):
            return _issue('cpp-direct-argument-assignment-review', base + at,
                          'Written assignment in an argument/formal requires independent ownership.')
        if text[at] == '=' and not (at and text[at - 1] in '=!<>') and not text.startswith('==', at):
            return _issue('cpp-direct-argument-assignment-review', base + at,
                          'Argument assignment or formal default remains explicitly gated.')
        if text[at] in ';{}#' or text.startswith('=>', at):
            return _issue('cpp-direct-argument-executable-review', base + at,
                          'Sequence, callback body, data braces, or directive needs a separate owner.')
        word = _word(text, at)
        if word is not None:
            if word[0] in {'and_eq', 'or_eq', 'xor_eq'}:
                return _issue('cpp-direct-argument-assignment-review', base + at,
                              'Written alternative-token assignment needs independent ownership.')
            if word[0] in _CONTROL:
                return _issue('cpp-direct-argument-control-review', base + at,
                              'Control/declaration/executable argument syntax is outside this proposal.')
            at = word[1]
        else:
            at += 1
    return None


def _number_end(text, at):
    """Small scalar spelling grammar; no tokenization or literal masking."""
    start = at
    floating = False
    if text.startswith(('0x', '0X'), at):
        at += 2
        digits = at
        while at < len(text) and text[at] in '0123456789abcdefABCDEF':
            at += 1
        if at == digits:
            return None
        if at < len(text) and text[at] == '.':
            floating = True
            at += 1
            while at < len(text) and text[at] in '0123456789abcdefABCDEF':
                at += 1
        exponent = 'pP'
        hex_number = True
    elif text.startswith(('0b', '0B'), at):
        at += 2
        digits = at
        while at < len(text) and text[at] in '01':
            at += 1
        if at == digits:
            return None
        exponent = ''
        hex_number = False
    else:
        while at < len(text) and '0' <= text[at] <= '9':
            at += 1
        if at < len(text) and text[at] == '.':
            floating = True
            at += 1
            while at < len(text) and '0' <= text[at] <= '9':
                at += 1
        if at == start or (at == start + 1 and text[start] == '.'):
            return None
        exponent = 'eE'
        hex_number = False
    exponent_seen = False
    if at < len(text) and exponent and text[at] in exponent:
        floating = exponent_seen = True
        at += 1
        if at < len(text) and text[at] in '+-':
            at += 1
        digits = at
        while at < len(text) and '0' <= text[at] <= '9':
            at += 1
        if at == digits:
            return None
    if hex_number and floating and not exponent_seen:
        return None
    suffix = at
    while at < len(text) and text[at] in 'uUlLfF':
        at += 1
    allowed = {'', 'f', 'l'} if floating else {'', 'u', 'l', 'll', 'ul', 'lu', 'ull', 'llu'}
    if text[suffix:at].lower() not in allowed:
        return None
    integer = text[start:suffix]
    if (not floating and integer.startswith('0') and len(integer) > 1
            and not integer.startswith(('0x', '0X', '0b', '0B'))
            and any(char not in '01234567' for char in integer)):
        return None
    if at < len(text) and (_name_char(text[at]) or text[at] == '.'):
        return None
    return at


class _Expression:
    """Balanced, complete value composition; no assignment/comma grammar."""

    def __init__(self, text, base, resolver, type_id_resolver, literal_spans):
        self.text = text
        self.base = base
        self.resolver = resolver
        self.type_id_resolver = type_id_resolver
        self.literals = literal_spans
        self.at = 0
        self.depth = 0
        self.unknown = []
        self.evidence = []

    def _operator(self):
        self.at = _space(self.text, self.at)
        for spelling in ('||', '&&', '==', '!=', '<=', '>=', '<<', '>>'):
            if self.text.startswith(spelling, self.at):
                return spelling
        word = _word(self.text, self.at)
        if word is not None and word[0] in _OPERATORS:
            return word[0]
        char = self.text[self.at:self.at + 1]
        return char if char in _OPERATORS else None

    def expression(self, minimum=1):
        self.depth += 1
        if self.depth > _MAX_DEPTH or not self.unary():
            self.depth -= 1
            return False
        while True:
            self.at = _space(self.text, self.at)
            if minimum <= 1 and self.text[self.at:self.at + 1] == '?':
                self.at += 1
                if not self.expression() or self.text[self.at:self.at + 1] != ':':
                    self.depth -= 1
                    return False
                self.at += 1
                if not self.expression():
                    self.depth -= 1
                    return False
                continue
            operator = self._operator()
            precedence = _OPERATORS.get(operator, 0)
            if precedence < minimum:
                break
            self.at += len(operator)
            if not self.expression(precedence + 1):
                self.depth -= 1
                return False
        self.depth -= 1
        return True

    def unary(self):
        self.at = _space(self.text, self.at)
        word = _word(self.text, self.at)
        operator = word[0] if word is not None and word[0] in _UNARY else self.text[self.at:self.at + 1]
        if operator in _UNARY:
            self.at += len(operator)
            self.depth += 1
            valid = self.depth <= _MAX_DEPTH and self.unary()
            self.depth -= 1
            return valid
        if word is not None and word[0] in _CASTS:
            start = self.at
            opening = _space(self.text, word[1])
            if self.text[opening:opening + 1] != '<':
                return False
            depth = 1
            closing = opening + 1
            while closing < len(self.text) and depth:
                if self.text[closing] == '<':
                    depth += 1
                elif self.text[closing] == '>':
                    depth -= 1
                closing += 1
            if depth:
                return False
            inner = self.text[opening + 1:closing - 1]
            typed = _type_id(inner, self.base + opening + 1,
                             self.resolver, self.type_id_resolver, self.literals)
            if typed is None or inner[typed[0]:].strip():
                return False
            self.at = _space(self.text, closing)
            if self.text[self.at:self.at + 1] != '(':
                return False
            self.at += 1
            if not self.expression() or self.text[self.at:self.at + 1] != ')':
                return False
            self.at += 1
            self.evidence.append(dict(kind='explicit-proven-type-cast',
                start=self.base + start, end=self.base + self.at, type_evidence=typed[1], units=0))
            return self.postfix_tail(None)
        if self.text[self.at:self.at + 1] == '(':
            end = _balanced_end(self.text, self.at)
            if end is None:
                return False
            inner = self.text[self.at + 1:end - 1]
            typed = _type_id(inner, self.base + self.at + 1,
                             self.resolver, self.type_id_resolver, self.literals)
            if typed is not None and not inner[typed[0]:].strip():
                cast_start = self.at
                self.at = end
                self.depth += 1
                valid = self.depth <= _MAX_DEPTH and self.unary()
                self.depth -= 1
                if not valid:
                    return False
                self.evidence.append(dict(kind='c-style-proven-type-cast',
                    start=self.base + cast_start, end=self.base + end,
                    type_evidence=typed[1], units=0))
                return True
        return self.primary()

    def primary(self):
        self.at = _space(self.text, self.at)
        start = self.at
        owned = [span for span in self.literals if span['start'] == self.base + start]
        if owned:
            self.at += owned[0]['end'] - owned[0]['start']
            self.evidence.append(dict(owned[0], units=0))
            return self.postfix_tail(None)
        number = _number_end(self.text, start)
        if number is not None:
            self.at = number
            return self.postfix_tail(None)
        if self.text[start:start + 1] == '(':
            self.at += 1
            if not self.expression() or self.text[self.at:self.at + 1] != ')':
                return False
            self.at += 1
            return self.postfix_tail(None)
        named = _qualified(self.text, start)
        if (named is None or named[0] in _CONTROL or named[0] in _CV
                or named[0] in _RESERVED_NAMES - _BUILTIN_TYPES - _SCALARS):
            return False
        if _literal_overlap(self.literals, self.base + start, self.base + named[1]):
            return False
        self.at = named[1]
        pending = None if named[0] in _SCALARS else (named[0], self.base + start)
        return self.postfix_tail(pending)

    def postfix_tail(self, pending):
        called = False
        while True:
            self.at = _space(self.text, self.at)
            if self.text.startswith('->', self.at) or self.text[self.at:self.at + 1] == '.':
                self.at += 2 if self.text.startswith('->', self.at) else 1
                member = _word(self.text, _space(self.text, self.at))
                if member is None or member[0] in _RESERVED_NAMES:
                    return False
                member_start = _space(self.text, self.at)
                if _literal_overlap(self.literals, self.base + member_start, self.base + member[1]):
                    return False
                self.at = member[1]
                continue
            char = self.text[self.at:self.at + 1]
            if char == '[':
                self.at += 1
                if not self.expression() or self.text[self.at:self.at + 1] != ']':
                    return False
                self.at += 1
                continue
            if char == '(':
                previous = self.at - 1
                while previous >= 0 and self.text[previous].isspace():
                    previous -= 1
                if previous >= 0 and _literal_overlap(self.literals,
                        self.base + previous, self.base + previous + 1):
                    return False
                called = True
                opening = self.at
                self.at = _space(self.text, self.at + 1)
                if self.text[self.at:self.at + 1] != ')':
                    while True:
                        if not self.expression():
                            return False
                        if self.text[self.at:self.at + 1] != ',':
                            break
                        self.at += 1
                if self.text[self.at:self.at + 1] != ')':
                    return False
                self.at += 1
                self.evidence.append(dict(kind='callee-agnostic-call-composition',
                    start=self.base + opening, end=self.base + self.at, units=0))
                continue
            break
        if pending is not None and not called:
            proof = _role(self.resolver, pending[0], pending[1])
            if proof is None or proof['role'] != 'value':
                self.unknown.append(dict(name=pending[0], offset=pending[1], proof=proof))
            else:
                self.evidence.append(dict(kind='resolved-value-operand',
                    name=pending[0], offset=pending[1], source_evidence=proof, units=0))
        return True


def _formal(text, base, resolver, type_id_resolver, literal_spans):
    typed = _type_id(text, base, resolver, type_id_resolver, literal_spans)
    if typed is None:
        return None
    at, evidence = typed
    at = _space(text, at)
    named = _word(text, at)
    if named is not None:
        if named[0] in _RESERVED_NAMES:
            return None
        at = _space(text, named[1])
    # Bounded parenthesized named parameter and abstract empty function formal
    # retain the most-vexing cases without admitting callback declarator bodies.
    if text[at:at + 1] == '(':
        if named is not None:
            return None
        end = _balanced_end(text, at)
        if end is None:
            return None
        inner = text[at + 1:end - 1].strip()
        parenthesized_name = _word(inner, 0)
        if inner and (parenthesized_name is None or parenthesized_name[1] != len(inner)
                      or parenthesized_name[0] in _RESERVED_NAMES):
            return None
        at = _space(text, end)
    while text[at:at + 1] == '[':
        end = _balanced_end(text, at)
        if end is None:
            return None
        bound = text[at + 1:end - 1]
        if bound.strip():
            parsed = _Expression(bound, base + at + 1, resolver, type_id_resolver, literal_spans)
            if not parsed.expression() or bound[parsed.at:].strip() or parsed.unknown:
                return None
        at = _space(text, end)
    if at != len(text):
        return None
    return dict(kind='proven-bounded-formal', start=base, end=base + len(text),
                type_evidence=evidence, units=0)


def _template_comma_gate(text, base):
    """Possible template-callee commas cannot become comparison arguments."""
    cast_ranges = []
    cursor = 0
    while cursor < len(text):
        word = _word(text, cursor)
        if word is None:
            cursor += 1
            continue
        angle = _space(text, word[1])
        if word[0] in _CASTS and text[angle:angle + 1] == '<':
            depth = 1
            closing = angle + 1
            while closing < len(text) and depth:
                if text[closing] == '<':
                    depth += 1
                elif text[closing] == '>':
                    depth -= 1
                closing += 1
            if not depth:
                cast_ranges.append((angle, closing))
                cursor = closing
                continue
        cursor = word[1]
    for opening, char in enumerate(text):
        if char != '<' or text.startswith(('<=', '<<'), opening) or (opening and text[opening - 1] == '<'):
            continue
        if any(left <= opening < right for left, right in cast_ranges):
            continue
        previous = opening - 1
        while previous >= 0 and text[previous].isspace():
            previous -= 1
        if previous < 0 or not (_name_char(text[previous]) or text[previous] in ')]>'):
            continue
        comma = False
        at = opening + 1
        while at < len(text):
            if text[at] in '([':
                end = _balanced_end(text, at)
                if end is None:
                    break
                at = end
                continue
            if text[at] == '>':
                after = _space(text, at + 1)
                if comma and text[after:after + 1] == '(':
                    return _issue('cpp-direct-template-argument-role-review', base + opening,
                        'Possible template-callee commas need a proven template argument profile; comparison fallback cannot split them.')
            elif text[at] == ',':
                comma = True
            at += 1
    return None


def _arguments(text, base, resolver, type_id_resolver, literal_spans):
    effect = _effects(text, base)
    if effect is not None:
        return None, [effect], []
    if '\n' in text or '\r' in text:
        return None, [_issue('cpp-direct-wrapped-arguments-review', base,
            'Every multiline argument/formal group remains gated; preserve all written rows.')], []
    template = _template_comma_gate(text, base)
    if template is not None:
        return None, [template], []
    if not text.strip():
        return dict(role='prototype', arguments=[], units=0), [], []
    pieces = _parts(text, base)
    if not pieces:
        return None, [_issue('cpp-direct-argument-delimiter-review', base,
                            'Argument/formal delimiters are not completely balanced.')], []
    rows = []
    evidence = []
    possible_object = possible_prototype = True
    for start, raw in pieces:
        leading = len(raw) - len(raw.lstrip())
        argument = raw.strip()
        start += leading
        if not argument:
            return None, [_issue('cpp-direct-empty-argument-review', start,
                                'Empty comma-separated argument/formal is outside the bounded grammar.')], evidence
        formal = _formal(argument, start, resolver, type_id_resolver, literal_spans)
        expression = _Expression(argument, start, resolver, type_id_resolver, literal_spans)
        expression_ok = expression.expression() and not argument[expression.at:].strip()
        bare = _word(argument, 0)
        unknown_bare = (bare is not None and bare[1] == len(argument)
                        and len(expression.unknown) == 1
                        and expression.unknown[0]['proof'] is None)
        if unknown_bare:
            # Both possible bare-value and unnamed-type grammar profiles have
            # one owner name and zero argument/formal units. This is evidence
            # of count invariance, never a new named-type promotion.
            expression_ok = True
            formal = dict(kind='bare-name-object-or-formal-ambiguity',
                          name=bare[0], start=start, end=start + len(argument),
                          value_units=0, formal_units=0, units=0)
            evidence.append(dict(formal, review_required=False,
                                 reason='both-bounded-roles-preserve-the-same-independent-name-unit'))
        elif expression.unknown:
            expression_ok = False
        if formal is None and not expression_ok:
            unknown = expression.unknown
            kind = ('cpp-direct-argument-role-review' if unknown
                    else 'cpp-direct-argument-grammar-review')
            return None, [_issue(kind, start,
                'Unknown argument/formal grammar cannot hide independent work in a declarator.',
                unknown_names=unknown, source_text=argument)], evidence
        possible_object = possible_object and expression_ok
        possible_prototype = possible_prototype and formal is not None
        row = dict(start=start, end=start + len(argument), source_text=argument,
                   value_profile=expression_ok, formal_profile=formal, units=0)
        rows.append(row)
        if expression_ok:
            evidence.extend(expression.evidence)
        if formal is not None:
            evidence.append(formal)
    if not possible_object and not possible_prototype:
        return None, [_issue('cpp-direct-mixed-argument-role-review', base,
            'The complete list is neither a bounded object argument list nor a bounded prototype formal list.')], evidence
    role = ('object-or-prototype' if possible_object and possible_prototype
            else 'object' if possible_object else 'prototype')
    return dict(role=role, arguments=rows, units=0), [], evidence


def _named_head(part):
    at = _space(part, 0)
    while True:
        if part.startswith('&&', at):
            at += 2
        elif part[at:at + 1] in {'*', '&'}:
            at += 1
        else:
            break
        at = _space(part, at)
        while True:
            qualifier = _word(part, at)
            if qualifier is None or qualifier[0] not in _CV:
                break
            at = _space(part, qualifier[1])
    named = _word(part, at)
    if named is None:
        return None
    return named[0], at, _space(part, named[1])


def _literal_contract(spans, start, end, segment):
    result = []
    for span in spans:
        if not isinstance(span, dict):
            return None
        left, right = span.get('start'), span.get('end')
        if (not isinstance(left, int) or isinstance(left, bool)
                or not isinstance(right, int) or isinstance(right, bool) or left < 0 or right <= left):
            return None
        if left < end and start < right:
            source_text = span.get('source_text')
            if (left < start or right > end or span.get('kind') != 'literal'
                    or not isinstance(source_text, str) or len(source_text) != right - left
                    or '\n' in source_text or '\r' in source_text):
                return None
            masked = segment[left - start:right - start]
            if not all(masked_char == source_char if source_char.isspace() else masked_char == 'S'
                       for masked_char, source_char in zip(masked, source_text)):
                return None
            result.append(dict(span))
    result.sort(key=lambda span: span['start'])
    if any(left['end'] > right['start'] for left, right in zip(result, result[1:])):
        return None
    return result


def _ordinary_argument_gates(part, base, resolver, type_id_resolver, literals):
    """Inspect delegated argument scopes without replacing '=' RHS owners."""
    at = 0
    while at < len(part):
        if part[at] == '(':
            end = _balanced_end(part, at)
            if end is None:
                return [_issue('cpp-direct-ordinary-argument-delimiter-review', base + at,
                    'Delegated RHS argument grammar must retain a complete owner.')], []
            previous = at - 1
            while previous >= 0 and part[previous].isspace():
                previous -= 1
            if previous >= 0 and (_name_char(part[previous]) or part[previous] in ')]>'):
                if _literal_overlap(literals, base + previous, base + previous + 1):
                    return [_issue('cpp-direct-literal-name-owner-review', base + previous,
                        'Owned literal bytes cannot supply a delegated RHS callee name.')], []
                group, issues, evidence = _arguments(part[at + 1:end - 1], base + at + 1,
                    resolver, type_id_resolver, literals)
                if issues:
                    return issues, evidence
                if group['role'] == 'prototype' and group['arguments']:
                    return [_issue('cpp-direct-ordinary-argument-role-review', base + at + 1,
                        'An ordinary RHS call cannot absorb a type/formal-only argument profile.')], evidence
        elif part[at] == '[':
            end = _balanced_end(part, at)
            if end is None:
                return [_issue('cpp-direct-ordinary-argument-delimiter-review', base + at,
                    'Delegated RHS bracket grammar must retain a complete owner.')], []
            after = _space(part, end)
            previous = at - 1
            while previous >= 0 and part[previous].isspace():
                previous -= 1
            possible_capture = (previous < 0 or not
                (_name_char(part[previous]) or part[previous] in ')]'))
            if possible_capture and '{' in part[end:]:
                return [_issue('cpp-direct-ordinary-callback-review', base + at,
                    'A capture/body with an unresolved callback header needs separate executable owners.')], []
            if part[after:after + 1] == '(':
                parameters = _balanced_end(part, after)
                if parameters is not None:
                    after = _space(part, parameters)
            if part[after:after + 1] == '{':
                return [_issue('cpp-direct-ordinary-callback-review', base + at,
                    'An executable RHS callback needs separate header/body/closer owners.')], []
        at += 1
    return [], []


def analyze_direct_decl(segment, *, offset, type_prefix, context='declaration',
                        role_resolver=None, type_id_resolver=None,
                        legacy_declarators=None, literal_spans=()):
    """Return {declaration: dict|None, diagnostics, evidence, proposal_only}.

    At least one direct suffix is required. A caller uses its current ordinary
    declaration fallback only if ``matched`` is false and diagnostics are
    empty; every diagnosed result is a gate. No parameter/control/data/header
    context is admitted.

    ``initialized`` retains the legacy counter's top-level-'=' meaning. Direct
    names have initialized=False, direct_initialized=True, and add one to the
    legacy ``uninitialized`` term. Thus `T a(0),b=c=0;` keeps a+b+c additive.
    Actual initialization/prototype status is in initialization/direct_role.
    """
    result = dict(declaration=None, diagnostics=[], evidence=[], matched=False,
                  proposal_only=True)
    if context != 'declaration':
        result['diagnostics'].append(_issue('cpp-direct-context-review', offset,
            'Caller must prove an independent declaration context before this proposal.'))
        return result
    if (not isinstance(segment, str) or not isinstance(offset, int)
            or isinstance(offset, bool) or offset < 0
            or not isinstance(type_prefix, tuple) or len(type_prefix) != 3):
        result['diagnostics'].append(_issue('cpp-direct-input-proof-review', offset,
            'Exact source offset and supplied proven type-prefix tuple are required.'))
        return result
    leading = len(segment) - len(segment.lstrip())
    code = segment.strip().removesuffix(';').rstrip()
    end, type_name, type_evidence = type_prefix
    if (not isinstance(end, int) or isinstance(end, bool) or not 0 < end < len(code)
            or not isinstance(type_name, str) or not type_name
            or not isinstance(type_evidence, dict) or not type_evidence.get('kind')
            or any(type_evidence.get(flag) for flag in
                   ('conditional', 'dependent', 'macro', 'unresolved', 'shadowed'))
            or not (code[end].isspace() or code[end] in '*&')):
        result['diagnostics'].append(_issue('cpp-direct-type-prefix-review', offset + leading,
            'The caller must supply a complete, provenance-owned type prefix at exact offsets.'))
        return result
    declarator_start = _space(code, end)
    raw = code[declarator_start:]
    base = offset + leading + declarator_start
    parts = _parts(raw, 0)
    if not parts:
        result['diagnostics'].append(_issue('cpp-direct-declarator-delimiter-review', base,
            'All independent declarators must have completely balanced delimiters.'))
        return result
    direct_parts = []
    for part_offset, part in parts:
        head = _named_head(part)
        direct_parts.append(bool(head and part[head[2]:head[2] + 1] == '('))
    if not any(direct_parts):
        return result
    result['matched'] = True
    literals = _literal_contract(literal_spans, offset, offset + len(segment), segment)
    if literals is None:
        result['diagnostics'].append(_issue('cpp-direct-literal-data-owner-review', offset,
            'Literal/data spans need exact nonoverlapping single-row source ownership; preserve every written row.'))
        return result
    if _literal_overlap(literals, offset + leading, offset + leading + end):
        result['diagnostics'].append(_issue('cpp-direct-literal-name-owner-review', offset + leading,
            'Owned literal bytes cannot supply a proven declaration type prefix.'))
        return result
    rows = []
    for (part_offset, part), direct in zip(parts, direct_parts):
        if not direct:
            delegated = legacy_declarators(part) if legacy_declarators is not None else None
            if not delegated or len(delegated) != 1:
                result['diagnostics'].append(_issue('cpp-direct-ordinary-declarator-review', base + part_offset,
                    'Non-direct pointer/reference/array/= part requires the existing complete declarator fallback.',
                    source_text=part))
                continue
            row = dict(delegated[0])
            row['part_offset'] = part_offset
            row['name_offset'] = part_offset + delegated[0]['name_offset']
            row['source_start'] = base + part_offset
            row['source_end'] = base + part_offset + len(part)
            row['direct_initialized'] = False
            row['initialization'] = 'assignment' if row['initialized'] else 'none'
            name_at = base + row['name_offset']
            if _literal_overlap(literals, name_at, name_at + len(row['name'])):
                result['diagnostics'].append(_issue('cpp-direct-literal-name-owner-review', name_at,
                    'Owned literal bytes cannot supply an ordinary delegated binding name.'))
                continue
            rows.append(row)
            # Preserve the delegated RHS completely. This helper neither
            # suppresses its assignments nor substitutes expression units.
            if row['initialized']:
                issues, evidence = _ordinary_argument_gates(part, base + part_offset,
                    role_resolver, type_id_resolver, literals)
                result['diagnostics'].extend(issues)
                result['evidence'].extend(evidence)
            continue
        name, local_name, opening = _named_head(part)
        if _literal_overlap(literals, base + part_offset + local_name,
                             base + part_offset + local_name + len(name)):
            result['diagnostics'].append(_issue('cpp-direct-literal-name-owner-review',
                base + part_offset + local_name,
                'Owned literal bytes cannot supply a named direct declarator.'))
            continue
        if name in _RESERVED_NAMES:
            result['diagnostics'].append(_issue('cpp-direct-reserved-declarator-name-review',
                base + part_offset + local_name,
                'A reserved word cannot supply a named direct declarator.', name=name))
            continue
        closing = _balanced_end(part, opening)
        if closing is None or part[closing:].strip():
            result['diagnostics'].append(_issue('cpp-direct-declarator-tail-review', base + part_offset + opening,
                'A named direct suffix must be completely consumed; trailing grammar remains gated.',
                name=name, source_text=part))
            continue
        argument_text = part[opening + 1:closing - 1]
        group, issues, evidence = _arguments(argument_text,
            base + part_offset + opening + 1, role_resolver, type_id_resolver, literals)
        result['diagnostics'].extend(issues)
        result['evidence'].extend(evidence)
        if group is None:
            continue
        row = dict(name=name, initialized=False, direct_initialized=group['role'] != 'prototype',
            initialization='prototype' if group['role'] == 'prototype' else 'direct',
            direct_role=group['role'], declarator=part.strip(), part_offset=part_offset,
            name_offset=part_offset + local_name, source_start=base + part_offset,
            source_end=base + part_offset + len(part),
            argument_opening=base + part_offset + opening,
            argument_end=base + part_offset + closing,
            argument_source_text=argument_text, arguments=group['arguments'], argument_units=0,
            units=1)
        if group['role'] == 'object-or-prototype':
            result['evidence'].append(dict(kind='cpp-direct-object-prototype-count-invariance',
                name=name, start=row['argument_opening'], end=row['argument_end'],
                object_name_units=1, prototype_name_units=1,
                object_argument_units=0, prototype_formal_units=0,
                review_required=False,
                reason='constructor/prototype-role-does-not-change-independent-units-or-gates'))
        rows.append(row)
    if result['diagnostics']:
        return result
    result['declaration'] = dict(count=len(rows), declaration_bindings=len(rows),
        uninitialized=sum(not row['initialized'] for row in rows),
        reason='independent-proven-type-direct-declaration-bindings',
        declarator_start=leading + declarator_start, type_name=type_name,
        declarators=rows, type_evidence=type_evidence, proposal_only=True,
        evidence=result['evidence'])
    return result


def identify_direct_decl(segment, **proofs):
    """Current counter-compatible dictionary or None; diagnostics via analyze."""
    return analyze_direct_decl(segment, **proofs)['declaration']
