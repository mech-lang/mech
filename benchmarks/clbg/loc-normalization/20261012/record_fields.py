"""Unhooked, offset preserving named-field and enum declaration proposals.

The caller still owns type headers, opening delimiters and written closers.
Only a proven declaration body is replaced here. ``events`` are count-bearing
member spans; ``owned_spans`` own the entire declaration interior, including
separator-only rows, without adding units. They are separate so an integrator
can mask the interior without double counting overlapping event spans.

This module never reads source files, computes hashes, changes source bytes or
admits chart counts. Execution and its fixtures require the timing mutex.
"""

import re


LANGUAGES = {'rust', 'c', 'cpp', 'java'}
MAX_OWNER = 1200
MAX_BODY = 20000
IDENT = r'[A-Za-z_][A-Za-z_0-9]*'
JAVA_IDENT = r'[A-Za-z_$][A-Za-z_$0-9]*'
ASSIGN = re.compile(r'(?<![=!<>])(?:[+*/%&|^-]=|=(?!=|>))')


# Static evidence copied from the immutable job07 metadata, not a new hash
# sweep or a claim that the full source has received a policy certificate.
SOURCE_EVIDENCE = (
    dict(key='nbody:nbody-rust-1',
         path='nbody/upstream/nbody-rust-1.rs',
         sha256='ba284c090d7fb2ba19a12582be5c70c58e830db0d28ba1d5a0ac80d9b96e0f20',
         metadata='loc-normalization/20261012/jobs/boundary-audit-07/candidate-audit.json',
         lines=[63, 64, 65],
         form='x: f64, y: f64, z: f64; vx: f64, vy: f64, vz: f64; mass: f64',
         body_bindings=7,
         scope='static packed named-field evidence; full source unqualified'),
    dict(key='pidigits:pidigits-rust-1',
         path='pidigits/upstream/pidigits-rust-1.rs',
         sha256='e8a6f30107e53e12d52d172b1c421a41249f493136a6405daf2601fd8a567139',
         metadata='loc-normalization/20261012/jobs/boundary-audit-07/candidate-audit.json',
         lines=[15], form='pub enum c_void { __variant1, __variant2 }',
         body_bindings=2,
         scope='static packed enum evidence; full source unqualified'),
    dict(key='knucleotide:knucleotide-rust-4',
         path='knucleotide/upstream/knucleotide-rust-4.rs',
         sha256='82d3213c4b4fc4c819b0d45ffe77449647a28f00ddfcf9859bc37d14b784ae22',
         metadata='loc-normalization/20261012/jobs/boundary-audit-07/candidate-audit.json',
         lines=[120, 121, 122, 123], form="enum Item { Freq(usize), Occ(&'static str), }",
         body_bindings=2,
         scope='tuple variant names count; positional payload types do not split'),
    dict(key='knucleotide:knucleotide-rust-5',
         path='knucleotide/upstream/knucleotide-rust-5.rs',
         sha256='1c31017fbc96416addbc403cbf837ae5323dc8268b65df2431a60cc4574f87f0',
         metadata='loc-normalization/20261012/jobs/boundary-audit-07/candidate-audit.json',
         lines=[119, 120, 121, 122], form="enum Item { Freq(usize), Occ(&'static str), }",
         body_bindings=2,
         scope='tuple variant names count; positional payload types do not split'),
)


def _trim(text, start, end):
    while start < end and text[start].isspace():
        start += 1
    while end > start and text[end - 1].isspace():
        end -= 1
    return start, end


def _matching(text, opening):
    wanted = {'(': ')', '[': ']', '{': '}'}
    if opening >= len(text) or text[opening] not in wanted:
        return None
    stack = [wanted[text[opening]]]
    for index in range(opening + 1, min(len(text), opening + MAX_BODY)):
        char = text[index]
        if char in wanted:
            stack.append(wanted[char])
        elif char in ')]}':
            if not stack or stack.pop() != char:
                return None
            if not stack:
                return index
    return None


def _structural(context):
    last = max((i for i, (_, kind, _) in enumerate(context)
                if kind in {'code', 'bare-code', 'type'}), default=-1)
    return any(kind in {'data', 'args', 'parameters', 'for', 'control'}
               for _, kind, _ in context[last + 1:])


def _own_kind(contexts, opening):
    if opening + 1 >= len(contexts):
        return None
    owner = next((entry for entry in reversed(contexts[opening + 1])
                  if entry[2] == opening and entry[0] == '{'), None)
    return owner[1] if owner else None


def _parts(text, start, end, *, angles=False, semicolon_stop=False):
    """Return comma group spans; generic commas are protected only in types."""
    pairs = {'(': ')', '[': ']', '{': '}'}
    stack, pieces, cursor, initializer = [], [], start, False
    for index in range(start, end):
        char = text[index]
        if char in pairs:
            stack.append(pairs[char])
        elif char in ')]}':
            if not stack or stack.pop() != char:
                return None, None, 'unbalanced-member-delimiter'
        elif angles and not initializer and char == '<':
            stack.append('>')
        elif angles and not initializer and char == '>' and (index == start or text[index - 1] != '-'):
            if not stack or stack.pop() != '>':
                return None, None, 'unbalanced-generic-type'
        elif not stack and char == '=':
            initializer = True
        elif not stack and char == ',':
            pieces.append((cursor, index, index + 1))
            cursor = index + 1
            initializer = False
        elif not stack and char == ';':
            if semicolon_stop:
                pieces.append((cursor, index, index + 1))
                return pieces, index + 1, None
            return None, None, 'unexpected-member-semicolon'
    if stack:
        return None, None, 'unbalanced-member-delimiter'
    pieces.append((cursor, end, end))
    return pieces, end, None


def _simple_rust_type(value):
    """Bounded named/reference/tuple/array types with balanced generic groups.

    Associated types, type macros, const-generic blocks, function/trait object
    types and defaults are deliberately reviewed rather than guessed.
    """
    value = value.strip()
    if not value or re.search(r'\b(?:fn|dyn|impl|for|where|as)\b|[#{}=!]', value):
        return False
    if value.startswith('<') or not re.fullmatch(r"[A-Za-z_0-9\s:'&*<>,()\[\];+?_-]+", value):
        return False
    # A single colon is neither a named type path nor a tuple payload type.
    if re.search(r'(?<!:):(?!:)', value):
        return False
    _, _, error = _parts(value, 0, len(value), angles=True)
    if error:
        return False
    stack = []
    for index, char in enumerate(value):
        if char in '([<':
            stack.append({'(': ')', '[': ']', '<': '>'}[char])
        elif char in ')]>':
            if not stack or stack.pop() != char:
                return False
        elif char == ';' and (not stack or stack[-1] != ']'):
            return False
        elif char == ',' and not stack:
            return False
    return not stack and bool(re.search(IDENT, value))


def _rust_field(text):
    match = re.fullmatch(r'\s*(?:pub(?:\s*\(\s*(?:crate|self|super)\s*\))?\s+)?'
                         r'(' + IDENT + r')\s*:\s*(.+?)\s*', text, re.S)
    if not match or not _simple_rust_type(match[2]):
        return None, 'unrecognized-named-field-type'
    return dict(name=match[1], units=1, form='named-field'), None


def _scalar_initializer(value):
    if not value.strip() or re.search(r'[#{};]|\b(?:let|var|const|return|new|fn)\b', value):
        return False
    if re.search(r'[A-Za-z_]\w*\s*!\s*[\[(]', value):
        return False
    if not re.fullmatch(r"[A-Za-z_$0-9\s:.'+*/%&|^<>=!?~(),\[\]-]+", value):
        return False
    if '\n' in value.strip() and any(char in value for char in '([S'):
        return False  # Preserve mixed literal/argument wrapping until reviewed.
    return _matching('(' + value + ')', 0) == len(value) + 1


def _enum_member(text, language):
    identifier = JAVA_IDENT if language == 'java' else IDENT
    match = re.match(r'\s*(' + identifier + r')\b', text)
    if not match:
        return None, 'unrecognized-enum-member'
    tail = text[match.end():].strip()
    units, form = 1, 'enum-constant'
    if not tail:
        return dict(name=match[1], units=units, form=form), None
    if language == 'rust' and tail.startswith('('):
        closing = _matching(tail, 0)
        if closing != len(tail) - 1:
            return None, 'complex-enum-variant-payload'
        parts, _, error = _parts(tail, 1, closing, angles=True)
        if error or not all(_simple_rust_type(tail[a:b]) for a, b, _ in parts
                            if tail[a:b].strip()):
            return None, 'complex-enum-variant-payload'
        if not tail[1:closing].strip():
            return None, 'empty-enum-variant-payload'
        return dict(name=match[1], units=1, form='enum-tuple-variant'), None
    if language == 'java' and tail.startswith('('):
        closing = _matching(tail, 0)
        if (closing != len(tail) - 1 or '\n' in tail[1:closing].strip() or
                not _scalar_initializer(tail[1:closing] or '0')):
            return None, 'complex-java-enum-constructor-or-literal-wrapping'
        if ASSIGN.search(tail):
            return None, 'enum-constructor-independent-binding-review'
        return dict(name=match[1], units=1, form='enum-constructor-constant'), None
    if language != 'java' and tail.startswith('=') and not tail.startswith('=='):
        value = tail[1:]
        if not _scalar_initializer(value):
            return None, 'complex-enum-initializer'
        # The initializer's first equals binds the constant itself. Subsequent
        # direct assignments are additional independent binding operations.
        depth, aliases = [], 0
        for item in re.finditer(r'[()\[\]]|(?<![=!<>])(?:[+*/%&|^-]=|=(?!=|>))', value):
            token = item[0]
            if token in '([':
                depth.append(token)
            elif token in ')]':
                if depth:
                    depth.pop()
            elif depth:
                return None, 'nested-enum-initializer-binding-review'
            else:
                prefix = value[:item.start()]
                if not re.search(IDENT + r'\s*$', prefix):
                    return None, 'complex-enum-initializer-binding'
                aliases += 1
        return dict(name=match[1], units=1 + aliases, form='enum-initialized-constant'), None
    return None, 'complex-enum-member-or-anonymous-body'


def _header(text, keyword, language):
    """Find only complete named declaration owners, never a constructor."""
    start, word = keyword.start(), keyword[0]
    limit = min(len(text), start + MAX_OWNER)
    next_brace = text.find('{', keyword.end(), limit)
    if next_brace < 0:
        return None, None
    tail = text[keyword.end():next_brace]
    if any(char in tail for char in ';}='):
        return None, None
    if language == 'rust':
        # Tuple structs end with a semicolon and are positional type data.
        if '(' in tail and word == 'struct':
            return None, None
        match = re.fullmatch(r'\s+(' + IDENT + r')(.*)', tail, re.S)
        if not match:
            return None, 'unknown-rust-type-owner'
        suffix = match[2].strip()
        if suffix:
            if not suffix.startswith('<') or re.search(r'[{};=]|\bwhere\b', suffix):
                return None, 'complex-rust-type-owner'
            depth = 0
            for char in suffix:
                if char == '<':
                    depth += 1
                elif char == '>':
                    depth -= 1
                if depth < 0:
                    return None, 'complex-rust-type-owner'
            if depth or not suffix.endswith('>'):
                return None, 'complex-rust-type-owner'
        name = match[1]
    elif language in {'c', 'cpp'}:
        match = re.fullmatch(r'\s*(?:(?:class|struct)\s+)?(' + IDENT + r')?'
                             r'(?:\s*:\s*(?:unsigned\s+|signed\s+)?' + IDENT + r')?\s*', tail)
        if not match or (language == 'c' and re.search(r'\b(?:class|struct)\b|:', tail)):
            return None, 'complex-c-enum-owner'
        name = match[1] or '<anonymous>'
    else:
        match = re.fullmatch(r'\s+(' + JAVA_IDENT + r')(?:\s+implements\s+' +
                             JAVA_IDENT + r'(?:\s*,\s*' + JAVA_IDENT + r')*)?\s*', tail)
        if not match:
            return None, 'complex-java-enum-owner'
        name = match[1]
    return dict(start=start, opening=next_brace, kind=word, name=name), None


def classify_declaration_fields(cleaned, language, contexts):
    """Return conservative body binding events and exact interior ownership.

    Caller must mask accepted ``owned_spans`` before ordinary body counting;
    suppress canonical events/boundaries *inside* them; add each ``event`` once;
    retain the outer type header and closer. Diagnostics gate the entire owner:
    no partially parsed member body is claimed. Nested record variant bodies,
    attributes, Java anonymous enum bodies and uncertain owners stay reviewed.
    """
    language = {'rs': 'rust', 'c++': 'cpp'}.get(language, language)
    if len(contexts) != len(cleaned):
        raise ValueError('Declaration contexts must preserve source offsets.')
    if language not in LANGUAGES:
        return dict(events=[], spans=[], owned_spans=[], diagnostics=[], evidence=[],
                    status='unsupported-language')
    events, ownership, diagnostics, evidence = [], [], [], []
    keywords = r'\b(?:struct|enum)\b' if language == 'rust' else r'\benum\b'
    accepted_until = -1

    def review(offset, reason, end=None):
        diagnostics.append(dict(kind='declaration-field-review', offset=offset,
            end=end, reason=reason,
            message='Declaration owner or member grammar requires exact source review.'))

    for keyword in re.finditer(keywords, cleaned):
        if keyword.start() < accepted_until or _structural(contexts[keyword.start()]):
            continue
        header, error = _header(cleaned, keyword, language)
        if header is None:
            if error:
                review(keyword.start(), error)
            continue
        opening = header['opening']
        closing = _matching(cleaned, opening)
        if closing is None:
            review(keyword.start(), 'unbalanced-or-bounded-type-body', opening + 1)
            continue
        if _own_kind(contexts, opening) not in {'code', 'bare-code', 'type'}:
            review(keyword.start(), 'type-owner-not-proven-executable', closing + 1)
            continue
        pieces, end, error = _parts(cleaned, opening + 1, closing,
            angles=language == 'rust', semicolon_stop=language == 'java')
        if error:
            review(keyword.start(), error, closing + 1)
            continue
        member_body = cleaned[opening + 1:end]
        if '#' in member_body or re.search(r'\b(?:fn|class|struct|enum|typedef)\b', member_body):
            review(keyword.start(), 'member-attribute-or-nested-declaration', end)
            continue
        proposed, member_error = [], None
        for piece_index, (begin, finish, separator_end) in enumerate(pieces):
            begin, finish = _trim(cleaned, begin, finish)
            if begin == finish:
                # An empty trailing comma group is structural. Empty interior
                # groups, however, are not accepted as valid declarations.
                empty_java_section = (language == 'java' and len(pieces) == 1 and
                                      cleaned[separator_end - 1:separator_end] == ';')
                if piece_index != len(pieces) - 1 and not empty_java_section:
                    member_error = 'empty-interior-member-group'
                    break
                continue
            item, member_error = (_rust_field(cleaned[begin:finish])
                if header['kind'] == 'struct'
                else _enum_member(cleaned[begin:finish], language))
            if member_error:
                break
            proposed.append(dict(start=begin, end=separator_end,
                units=item['units'], kind='declaration-field', scope=header['kind'],
                owner_name=header['name'], owner_opening=opening,
                name=item['name'], form=item['form'],
                owned_rows=list(range(cleaned.count('\n', 0, begin) + 1,
                                      cleaned.count('\n', 0, max(begin, separator_end - 1)) + 2))))
        if member_error:
            review(keyword.start(), member_error, closing + 1)
            continue
        # Java declarations after the enum constant section are deliberately
        # left with the existing declaration/function/control pathways.
        interior_end = end if language == 'java' else closing
        ownership.append(dict(start=opening + 1, end=interior_end, units=0,
            kind='declaration-interior-ownership', scope=header['kind'],
            owner_name=header['name'], owner_opening=opening,
            exclude_internal_boundaries=True))
        events.extend(proposed)
        evidence.append(dict(start=keyword.start(), opening=opening, closing=closing,
            name=header['name'], form=header['kind'], body_bindings=sum(item['units'] for item in proposed),
            names=[item['name'] for item in proposed], reason='proven-independent-named-declarations'))
        accepted_until = interior_end
    return dict(events=events, spans=events, owned_spans=ownership,
                diagnostics=diagnostics, evidence=evidence,
                status='needs-review' if diagnostics else 'proposal-only')
