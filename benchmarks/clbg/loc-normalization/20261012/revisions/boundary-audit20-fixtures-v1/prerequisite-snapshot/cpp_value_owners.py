"""Unhooked bounded source-written C++ value ownership proposals.

Input is an unchanged offset-preserving lexical mask. The caller supplies the
existing positive type-prefix and ordinary declaration resolvers. This module
does not import those modules, lex source, read files, normalize/count LOC,
expand macros, infer value roles from missing types, or certify a benchmark.

``declaration_resolver(segment, environment, offset=absolute_start)`` must be
the ordinary pointer/reference/array/= declaration resolver, not the direct
object/prototype count-invariance helper. ``type_prefix_resolver(text,
environment, absolute_start)`` returns the existing proven type-prefix tuple.
``macro_token_gates(text, absolute_start, role=...)`` returns source-keyed
issues for active/uncertain macro ownership. If source writes define/undef and
this callback is absent, positive value owner admission is gated.

Every accepted owner has units=0. All-or-nothing declaration/formal admission
does not hide its initializer, body, assignments, literal/data rows or closes.
"""

import re


_NAME = r'[A-Za-z_]\w*'
_WORD = re.compile(_NAME)
_DIRECTIVE = re.compile(r'^[ \t]*#[ \t]*(\w+)\b', re.M)
_CONTROL = {'if', 'else', 'while', 'switch', 'do', 'try', 'catch'}
_NON_DECL = {'return', 'co_return', 'throw', 'break', 'continue', 'goto',
             'delete', 'new', 'static_assert', 'using', 'typedef', 'asm', '__asm__'}
_AGGREGATE = {'struct', 'class', 'union', 'enum', 'namespace', 'template'}
_TYPE_OWNER_KINDS = {'source-local-typedef', 'source-local-using',
                     'source-local-cpp-named-type', 'explicit-header-type-contract',
                     'explicit-standard-namespace-import',
                     'explicit-header-class-template-contract',
                     'explicit-standard-template-namespace-import'}
_MAX_DEPTH = 48


def _issue(kind, start, end, reason, **evidence):
    return dict(kind=kind, offset=start, end=end, reason=reason,
                review_required=True, **evidence)


def _space(text, at, end):
    while at < end and text[at].isspace():
        at += 1
    return at


def _pairs(text):
    result = {}
    stack = []
    closing = {'(': ')', '[': ']', '{': '}'}
    for at, char in enumerate(text):
        if char in closing:
            stack.append((at, closing[char]))
            if len(stack) > _MAX_DEPTH:
                return None
        elif char in ')]}':
            if not stack or stack[-1][1] != char:
                return None
            opening, _ = stack.pop()
            result[opening] = at + 1
    return None if stack else result


def _directives(cleaned):
    """Blank written directive logical lines without promoting their payload."""
    chars = list(cleaned)
    conditional = []
    stack = []
    written_macros = []
    continuation = False
    cursor = 0
    for line in cleaned.splitlines(keepends=True):
        found = re.match(r'^[ \t]*#[ \t]*(\w+)\b', line)
        directive = continuation or found is not None
        if found:
            word = found[1]
            if word in {'if', 'ifdef', 'ifndef'}:
                stack.append(cursor)
            elif word == 'endif' and stack:
                conditional.append((stack.pop(), cursor + len(line)))
            if word in {'define', 'undef'}:
                written_macros.append(cursor)
        if directive:
            for at in range(cursor, cursor + len(line)):
                if not chars[at].isspace():
                    chars[at] = ' '
        continuation = directive and line.rstrip('\r\n').endswith('\\')
        cursor += len(line)
    conditional.extend((start, len(cleaned)) for start in stack)
    return ''.join(chars), conditional, written_macros


def _top_parts(text, start, end, separator, pairs):
    result = []
    part = at = start
    while at < end:
        char = text[at]
        if char in '([{':
            close = pairs.get(at)
            if close is None or close > end:
                return None
            at = close
            continue
        if char in ')]}':
            return None
        if char == separator:
            result.append((part, at))
            part = at + 1
        at += 1
    result.append((part, end))
    return result


def _boundary(text, start, end, pairs):
    """Find a top-level semicolon or body opener; parenthesis bodies stay owned."""
    at = start
    while at < end:
        char = text[at]
        if char in '([':
            close = pairs.get(at)
            if close is None or close > end:
                return None
            at = close
            continue
        if char in '{};':
            return at, char
        at += 1
    return None


def _trim_bounds(text, start, end):
    start = _space(text, start, end)
    while end > start and text[end - 1].isspace():
        end -= 1
    return start, end


def _candidate_names(text, start, end, type_prefix=None):
    """Names for blocked ownership only. This function never proves a value."""
    raw = text[start:end]
    typed_end = type_prefix[0] if type_prefix is not None else None
    if typed_end is None:
        possible = re.match(r'(?:(?:const|static|extern|volatile|constexpr)\s+)*'
                            + _NAME + r'(?:\s*::\s*' + _NAME + r')*\s+', raw)
        typed_end = possible.end() if possible else None
    if typed_end is None:
        return []
    # Unresolved arguments/angle syntax never own comma splitting. A single
    # written leading name can be conservatively blocked, with full owner gate.
    found = re.match(r'\s*[*&\s]*(' + _NAME + r')\b', raw[typed_end:])
    return [(found[1], start + typed_end + found.start(1))] if found else []


def collect_cpp_value_owners(cleaned, language, type_environment, *,
                             declaration_resolver, type_prefix_resolver,
                             macro_token_gates=None, conditional_regions=(),
                             literal_spans=None):
    """Return bounded owners, blocked owners/regions, diagnostics and evidence.

    Accepted grammar: one-row ordinary declarations in translation-unit,
    named function or ordinary block scopes; one-row, positively typed named
    function-definition formals without defaults; classic for initializer
    declarations with an exactly braced or recursively bounded unbraced
    classic-for/simple-statement body. No direct prototype/object profile is
    accepted as a variable or data field.
    """
    result = dict(owners=[], type_owners=[], blocked_owners=[], blocked_regions=[],
                  diagnostics=[], evidence=[], proposal_only=True,
                  source_length=len(cleaned) if isinstance(cleaned, str) else None,
                  source_mask=cleaned if isinstance(cleaned, str) else None,
                  written_macro_offsets=[])
    if language != 'cpp':
        return result
    if (not isinstance(cleaned, str) or not isinstance(type_environment, dict)
            or not callable(declaration_resolver) or not callable(type_prefix_resolver)):
        result['diagnostics'].append(_issue('cpp-value-input-owner-review', 0, 0,
            'Exact masked source, type environment and positive ordinary resolvers are required.'))
        return result
    scan, derived_conditional, written_macros = _directives(cleaned)
    result['written_macro_offsets'] = written_macros
    pairs = _pairs(scan)
    if pairs is None:
        result['diagnostics'].append(_issue('cpp-value-delimiter-owner-review', 0, len(scan),
            'Unbalanced or excessive delimiter nesting cannot prove any value lifetime.'))
        result['blocked_regions'].append(dict(start=0, end=len(scan),
            kind='cpp-value-delimiter-owner-review', units=0))
        return result
    regions = tuple(conditional_regions) + tuple(derived_conditional)
    if literal_spans is None:
        result['diagnostics'].append(_issue('cpp-value-literal-provenance-review', 0, len(scan),
            'The caller must explicitly supply its complete literal mask inventory, including an empty inventory when none exist.'))
        result['blocked_regions'].append(dict(start=0, end=len(scan),
            kind='cpp-value-literal-provenance-review', units=0))
        return result
    validated_literals = []
    for span in literal_spans:
        if (not isinstance(span, dict) or not isinstance(span.get('start'), int)
                or isinstance(span.get('start'), bool) or not isinstance(span.get('end'), int)
                or isinstance(span.get('end'), bool)
                or not 0 <= span['start'] < span['end'] <= len(scan)
                or span.get('kind') != 'literal'
                or not isinstance(span.get('source_text'), str)
                or len(span['source_text']) != span['end'] - span['start']
                or cleaned[span['start']:span['end']] != ''.join(
                    char if char.isspace() else 'S' for char in span['source_text'])):
            result['diagnostics'].append(_issue('cpp-value-literal-provenance-review', 0, len(scan),
                'Literal mask provenance requires exact valid source bounds.'))
            result['blocked_regions'].append(dict(start=0, end=len(scan),
                kind='cpp-value-literal-provenance-review', units=0))
            return result
        validated_literals.append(span)
    ordered_literals = sorted(validated_literals, key=lambda row: row['start'])
    if any(left['end'] > right['start'] for left, right in zip(ordered_literals, ordered_literals[1:])):
        result['diagnostics'].append(_issue('cpp-value-literal-provenance-review', 0, len(scan),
            'Literal mask provenance overlaps and cannot provide independent source owners.'))
        result['blocked_regions'].append(dict(start=0, end=len(scan),
            kind='cpp-value-literal-provenance-review', units=0))
        return result
    aliases = type_environment.get('aliases', {})
    if not isinstance(aliases, dict):
        result['diagnostics'].append(_issue('cpp-value-type-environment-review', 0, len(scan),
            'Type-role competition requires the actual positive type owner mapping.'))
        result['blocked_regions'].append(dict(start=0, end=len(scan),
            kind='cpp-value-type-environment-review', units=0))
        return result
    for name, entries in aliases.items():
        if not isinstance(name, str) or not re.fullmatch(_NAME, name):
            continue  # A qualified owner is not evidence about this bare token.
        if not isinstance(entries, (list, tuple)):
            result['diagnostics'].append(_issue('cpp-value-type-environment-review', 0, len(scan),
                'Type owner entries must retain their supplied source metadata.'))
            result['blocked_regions'].append(dict(start=0, end=len(scan),
                kind='cpp-value-type-environment-review', units=0))
            return result
        for entry in entries:
            points = ('visible_start', 'scope_start', 'scope_end',
                      'definition_start', 'definition_end')
            if (not isinstance(entry, dict) or entry.get('name') != name
                    or entry.get('kind') not in _TYPE_OWNER_KINDS
                    or any(not isinstance(entry.get(key), int)
                           or isinstance(entry.get(key), bool) for key in points)
                    or not 0 <= entry['scope_start'] <= entry['visible_start'] <= entry['scope_end'] <= len(scan)
                    or not 0 <= entry['definition_start'] < entry['definition_end'] <= len(scan)):
                result['diagnostics'].append(_issue('cpp-value-type-owner-span-review', 0, len(scan),
                    'A supplied type owner lacks exact valid declaration/order/scope metadata.'))
                result['blocked_regions'].append(dict(start=0, end=len(scan),
                    kind='cpp-value-type-owner-span-review', units=0))
                return result
            owner = dict(name=name, visible_start=entry['visible_start'],
                scope_start=entry['scope_start'], scope_end=entry['scope_end'],
                owner_start=entry['definition_start'], owner_end=entry['definition_end'],
                kind='cpp-value-type-role-owner', source_kind=entry['kind'],
                source_evidence=dict(entry), units=0,
                conditional=bool(entry.get('conditional')), macro=bool(entry.get('macro')),
                unresolved=bool(entry.get('unresolved') or entry.get('dependent') or entry.get('shadowed')))
            result['type_owners'].append(owner)

    def typed(start, end):
        return type_prefix_resolver(scan[start:end], type_environment, start)

    def block(start, end, scope, kind, reason, *, declaration=None,
              type_proof=None, names=None, region=False, evidence=None):
        issue = _issue(kind, start, end, reason)
        if evidence:
            issue['source_evidence'] = evidence
        result['diagnostics'].append(issue)
        if declaration is not None:
            names = [(row['name'], start + declaration.get('declarator_start', 0)
                      + row.get('name_offset', 0)) for row in declaration['declarators']]
        elif names is None:
            names = _candidate_names(scan, start, end, type_proof)
        for name, at in names:
            owner = dict(name=name, name_start=at, name_end=at + len(name),
                visible_start=at, scope_start=scope[0], scope_end=scope[1],
                owner_start=start, owner_end=end, kind=kind, unresolved=True,
                source_text=cleaned[start:end], units=0)
            result['blocked_owners'].append(owner)
            result['evidence'].append(dict(owner))
        if region:
            result['blocked_regions'].append(dict(start=start, end=end, kind=kind, units=0))

    def macro_checks(start, end, role):
        issues = []
        if macro_token_gates is None:
            if written_macros:
                issues.append(_issue('cpp-value-macro-proof-review', start, end,
                    'Written define/undef requires an exact active-macro token gate before value admission.'))
        else:
            supplied = macro_token_gates(cleaned[start:end], start,
                                         role=role)
            if (not isinstance(supplied, (list, tuple))
                    or any(not isinstance(issue, dict) for issue in supplied)):
                issues.append(_issue('cpp-value-macro-proof-review', start, end,
                    'Macro token callback did not return an explicit issue sequence.'))
            else:
                issues.extend(dict(issue, from_macro_token_gate=True) for issue in supplied)
        return issues

    def checks(start, end):
        issues = []
        if any(left < end and start < right for left, right in regions):
            issues.append(_issue('cpp-value-conditional-owner-review', start, end,
                'A written conditional source owner has no selected branch proof.'))
        if any(span['start'] < end and start < span['end'] for span in validated_literals):
            issues.append(_issue('cpp-value-literal-owner-review', start, end,
                'This first-stage value inventory does not admit declarations/formals overlapping literal masks.'))
        issues.extend(macro_checks(start, end, 'source-value-owner'))
        return issues

    def statement_macro_gate(start, end, scope):
        start, end = _trim_bounds(scan, start, end)
        if start == end:
            return
        issues = macro_checks(start, end, 'source-value-statement')
        if issues:
            block(start, scope[1], scope, 'cpp-value-statement-macro-owner-review',
                'An unproved active/missing macro statement proof can introduce hidden bindings or control through the scope.',
                names=[], region=True, evidence=issues)

    def sweep_skipped_macros(start, end, scope, depth):
        """Inspect skipped body envelopes without admitting fields or values."""
        if depth > _MAX_DEPTH:
            statement_macro_gate(start, end, scope)
            return
        cursor = _space(scan, start, end)
        while cursor < end:
            boundary = _boundary(scan, cursor, end, pairs)
            if boundary is None:
                statement_macro_gate(cursor, end, scope)
                return
            at, delimiter = boundary
            statement_macro_gate(cursor, at + 1 if delimiter == ';' else at, scope)
            if delimiter == '{':
                close = pairs[at]
                sweep_skipped_macros(at + 1, close - 1, scope, depth + 1)
                cursor = close
            else:
                cursor = at + 1
            cursor = _space(scan, cursor, end)

    def visible_point(name_end, statement_end):
        at = name_end
        while True:
            next_at = _space(scan, at, statement_end)
            if next_at < statement_end and scan[next_at] == '[':
                close = pairs.get(next_at)
                if close is None or close > statement_end:
                    return None
                at = close
                continue
            return at

    def admit_declaration(start, end, scope, kind, *, body_start=None,
                          require_single=True):
        start, end = _trim_bounds(scan, start, end)
        if start == end:
            return False
        proof = typed(start, end)
        declaration = declaration_resolver(scan[start:end], type_environment, offset=start)
        if declaration is None:
            if proof is not None or re.match(r'(?:auto|decltype)\b', scan[start:end]):
                block(start, end, scope, 'cpp-value-declaration-owner-review',
                    'The ordinary resolver did not prove the complete written declaration.',
                    type_proof=proof)
            elif _candidate_names(scan, start, end):
                block(start, end, scope, 'cpp-value-unknown-type-owner-review',
                    'A possible written declaration has no positive source type owner.')
                result['blocked_regions'].append(dict(start=start, end=scope[1],
                    kind='cpp-value-unknown-type-owner-review', units=0))
            return False
        rows = declaration.get('declarators')
        if (proof is None or not isinstance(rows, list) or not rows
                or not isinstance(proof, tuple) or len(proof) != 3
                or not isinstance(proof[2], dict)
                or any(proof[2].get(flag) for flag in
                       ('conditional', 'dependent', 'macro', 'unresolved', 'shadowed'))
                or any(not isinstance(row, dict) or not isinstance(row.get('name'), str)
                       or row.get('direct_role') is not None
                       or row.get('direct_initialized')
                       or row.get('declarator', '').lstrip().startswith('(')
                       for row in rows)):
            block(start, end, scope, 'cpp-value-declaration-profile-review',
                'Only existing positively typed ordinary named declarators are value owners.',
                declaration=declaration, type_proof=proof)
            return False
        if require_single and len(rows) != 1:
            block(start, end, scope, 'cpp-value-formal-profile-review',
                'Each function formal must have one explicitly named ordinary declarator.',
                declaration=declaration)
            return False
        enclosing = [row for row in result['blocked_regions']
                     if row['start'] <= start < row['end']]
        if enclosing:
            block(start, end, scope, 'cpp-value-enclosing-owner-review',
                'The declaration occurs inside an unresolved imperative/formal/macro owner.',
                declaration=declaration, evidence=enclosing)
            return False
        if '\n' in scan[start:end] or '\r' in scan[start:end]:
            block(start, end, scope, 'cpp-value-wrapped-declaration-review',
                'Wrapped declaration/formal ownership remains outside this phase.',
                declaration=declaration)
            return False
        if any(char in scan[start:end] for char in '{}'):
            block(start, end, scope, 'cpp-value-executable-declaration-review',
                'An initializer callback/data body requires a separate owner proof.',
                declaration=declaration)
            return False
        issues = checks(start, end)
        if issues:
            block(start, end, scope, 'cpp-value-source-provenance-review',
                'Conditional/literal/macro source ownership blocks the complete declaration.',
                declaration=declaration, evidence=issues)
            return False
        additions = []
        for row in rows:
            at = start + declaration.get('declarator_start', 0) + row.get('name_offset', 0)
            name = row['name']
            if scan[at:at + len(name)] != name:
                block(start, end, scope, 'cpp-value-name-span-review',
                    'Resolver name offset does not match its exact source token.',
                    declaration=declaration)
                return False
            point = visible_point(at + len(name), end)
            if point is None:
                block(start, end, scope, 'cpp-value-declaration-point-review',
                    'Array declarator end is not completely source-owned.', declaration=declaration)
                return False
            additions.append(dict(name=name, name_start=at, name_end=at + len(name),
                declaration_point=point, visible_start=max(point, body_start or 0),
                scope_start=scope[0], scope_end=scope[1], scope_opening=scope[2],
                owner_start=start, owner_end=end, kind=kind,
                type_name=declaration['type_name'], type_evidence=declaration.get('type_evidence'),
                source_text=cleaned[start:end], role='value', conditional=False,
                macro=False, units=0))
        result['owners'].extend(additions)
        result['evidence'].extend(dict(row) for row in additions)
        return True

    def formal_parts(start, end):
        parts = []
        at = _space(scan, start, end)
        if at == end or scan[at:end].strip() == 'void':
            return []
        while at < end:
            proof = typed(at, end)
            if proof is None:
                return None
            cursor = at + proof[0]
            while cursor < end:
                char = scan[cursor]
                if char in '([{':
                    close = pairs.get(cursor)
                    if close is None or close > end:
                        return None
                    cursor = close
                    continue
                if char in '<>':
                    return None
                if char == ',':
                    break
                cursor += 1
            parts.append((at, cursor))
            if cursor == end:
                return parts
            at = _space(scan, cursor + 1, end)
            if at == end:
                return None
        return parts

    def function_profile(start, opening):
        """A simple written return type + ordinary name + (...) body header."""
        raw_start, raw_end = _trim_bounds(scan, start, opening)
        proof = typed(raw_start, raw_end)
        if proof is None:
            return None
        remainder = scan[raw_start + proof[0]:raw_end]
        found = re.fullmatch(r'[*&\s]*(' + _NAME + r')\s*', remainder)
        return (raw_start, proof, found[1]) if found else None

    def classic_for(start, limit):
        word = re.match(r'for\b', scan[start:limit])
        opening = _space(scan, start + word.end(), limit) if word else limit
        close = pairs.get(opening) if opening < limit and scan[opening] == '(' else None
        if close is None or close > limit:
            return None
        parts = _top_parts(scan, opening + 1, close - 1, ';', pairs)
        if parts is None or len(parts) != 3:
            return None
        return opening, close, parts

    def bounded_body_end(start, limit, depth=0):
        start = _space(scan, start, limit)
        if start >= limit or depth > _MAX_DEPTH:
            return None
        if scan[start] == '{':
            close = pairs.get(start)
            return close if close is not None and close <= limit else None
        word = _WORD.match(scan, start)
        if word and word[0] == 'for':
            profile = classic_for(start, limit)
            return bounded_body_end(profile[1], limit, depth + 1) if profile else None
        if word and word[0] in _CONTROL | _AGGREGATE:
            return None
        boundary = _boundary(scan, start, limit, pairs)
        if boundary is None or boundary[1] != ';':
            return None
        text = scan[start:boundary[0]]
        if ':' in text or any(char in text for char in '{}'):
            return None
        return boundary[0] + 1

    def parse_statement(start, limit, scope, depth):
        start = _space(scan, start, limit)
        if start >= limit:
            return limit
        if depth > _MAX_DEPTH:
            block(start, limit, scope, 'cpp-value-owner-depth-review',
                  'Statement nesting exceeds the bounded ownership grammar.', region=True)
            return limit
        if scan[start] == ';':
            return start + 1
        if scan[start] == '{':
            close = pairs[start]
            parse_sequence(start + 1, close - 1, (start, close, start), depth + 1)
            return close
        word = _WORD.match(scan, start)
        first = word[0] if word else ''
        if first == 'for':
            profile = classic_for(start, limit)
            body_end = bounded_body_end(profile[1], limit) if profile else None
            if profile is None or body_end is None:
                sweep_skipped_macros(start, limit, scope, depth + 1)
                header_end = profile[1] if profile else start + len(first)
                init = profile[2][0] if profile else (start, header_end)
                init_start, init_end = _trim_bounds(scan, *init)
                block(init_start, init_end, scope, 'cpp-value-for-lifetime-review',
                    'Only complete classic for headers with a braced or recursively bounded simple body are admitted.',
                    type_proof=typed(init_start, init_end))
                block(start, limit, scope, 'cpp-value-for-body-owner-review',
                    'Unknown loop body ownership prevents a precise source lifetime.', names=[], region=True)
                return limit
            loop_scope = (start, body_end, None)
            init_start, init_end = _trim_bounds(scan, *profile[2][0])
            statement_macro_gate(start, profile[1], scope)
            header_issues = checks(start, profile[1])
            conditional_body = any(left < body_end and profile[1] < right
                                   for left, right in regions)
            if header_issues or conditional_body:
                block(init_start, init_end, loop_scope, 'cpp-value-for-header-owner-review',
                    'Loop lifetime has unproved conditional/literal/macro control ownership.',
                    type_proof=typed(init_start, init_end),
                    evidence=header_issues)
                result['blocked_regions'].append(dict(start=start, end=body_end,
                    kind='cpp-value-for-header-owner-review', units=0))
            elif scan[init_start:init_end].strip():
                admit_declaration(init_start, init_end, loop_scope,
                                  'cpp-source-for-initializer-value', require_single=False)
            body_start = _space(scan, profile[1], body_end)
            parse_statement(body_start, body_end, loop_scope, depth + 1)
            return body_end
        boundary = _boundary(scan, start, limit, pairs)
        if boundary is None:
            statement_macro_gate(start, limit, scope)
            block(start, limit, scope, 'cpp-value-statement-end-review',
                'A source statement lacks a complete bounded delimiter owner.', region=True)
            return limit
        at, delimiter = boundary
        if delimiter == '}':
            return at + 1
        if delimiter == '{':
            close = pairs[at]
            statement_macro_gate(start, at, scope)
            if first in _AGGREGATE:
                block(start, close, scope, 'cpp-value-aggregate-body-review',
                    'Aggregate/member/namespace scopes require distinct value ownership.', names=[], region=True)
                sweep_skipped_macros(at + 1, close - 1, scope, depth + 1)
                tail_boundary = _boundary(scan, close, limit, pairs)
                tail = scan[close:tail_boundary[0]].strip() if tail_boundary and tail_boundary[1] == ';' else ''
                unscoped_enum = first == 'enum' and not re.match(r'enum\s+(?:class|struct)\b', scan[start:at])
                if unscoped_enum or tail:
                    block(start, scope[1], scope, 'cpp-value-aggregate-export-owner-review',
                        'Unscoped enumerators or an unresolved aggregate instance suffix can hide later values in the enclosing scope.',
                        names=[], region=True)
                return close + 1 if close < limit and scan[close] == ';' else close
            # The final balanced parenthesis must end immediately before this
            # body; qualifiers, callbacks, constructors and operators stay out.
            openings = [left for left, right in pairs.items()
                        if start <= left < at and scan[left] == '('
                        and not scan[right:at].strip()]
            opening = min(openings) if openings else None
            profile = function_profile(start, opening) if opening is not None else None
            if profile is None or first in _CONTROL:
                sweep_skipped_macros(at + 1, close - 1, scope, depth + 1)
                possible_type = typed(start, at) if first not in _CONTROL else None
                block(start, close, scope, 'cpp-value-imperative-owner-review',
                    'This executable/callback/control owner is outside the bounded named-function grammar.',
                    type_proof=possible_type,
                    names=None if possible_type is not None or first == 'auto' else [],
                    region=True)
                return close
            function_scope = (at, close, at)
            formal_start, formal_end = opening + 1, pairs[opening] - 1
            parts = formal_parts(formal_start, formal_end)
            header_issues = checks(start, at)
            if header_issues:
                block(start, at, function_scope, 'cpp-value-function-header-review',
                    'Named function body ownership is blocked by its source header provenance.',
                    names=[], evidence=header_issues)
                result['blocked_regions'].append(dict(start=at + 1, end=close - 1,
                    kind='cpp-value-function-header-review', units=0))
            if header_issues or parts is None or '\n' in scan[formal_start:formal_end] or '\r' in scan[formal_start:formal_end]:
                block(formal_start, formal_end, function_scope, 'cpp-value-formal-owner-review',
                    'Formal ownership requires a source-owned header and complete single-row positively typed ordinary parameters.',
                    names=[], region=False)
                # An unknown formal can hide any outer value in this body.
                result['blocked_regions'].append(dict(start=at + 1, end=close - 1,
                    kind='cpp-value-formal-owner-review', units=0))
            else:
                checkpoint = len(result['owners'])
                valid = True
                for left, right in parts:
                    if '=' in scan[left:right]:
                        block(left, right, function_scope, 'cpp-value-formal-default-review',
                            'Written formal defaults need separate structural ownership.',
                            type_proof=typed(left, right))
                        valid = False
                    else:
                        valid = admit_declaration(left, right, function_scope,
                            'cpp-source-function-formal-value', body_start=at + 1) and valid
                if not valid:
                    rejected = result['owners'][checkpoint:]
                    del result['owners'][checkpoint:]
                    for row in rejected:
                        blocked = dict(row, unresolved=True, role='unknown',
                                       kind='cpp-value-formal-list-owner-review')
                        result['blocked_owners'].append(blocked)
                        for evidence in result['evidence']:
                            if (evidence.get('name_start'), evidence.get('owner_start')) == (
                                    row['name_start'], row['owner_start']):
                                evidence.update(unresolved=True, role='unknown', accepted=False,
                                    kind='cpp-value-formal-list-owner-review')
                    result['blocked_regions'].append(dict(start=at + 1, end=close - 1,
                        kind='cpp-value-formal-list-owner-review', units=0))
            parse_sequence(at + 1, close - 1, function_scope, depth + 1)
            return close
        # Ordinary executable statements contribute no owner or LOC units.
        # Only the positive ordinary declaration resolver may add bindings.
        statement_macro_gate(start, at + 1, scope)
        if first in _CONTROL:
            block(start, at + 1, scope, 'cpp-value-imperative-owner-review',
                'An unbraced imperative control has no admitted local value ownership.',
                names=[], region=True)
        elif first not in _NON_DECL | _AGGREGATE:
            admit_declaration(start, at, scope, 'cpp-source-ordinary-value', require_single=False)
        return at + 1

    def parse_sequence(start, end, scope, depth):
        cursor = start
        while cursor < end:
            next_at = parse_statement(cursor, end, scope, depth)
            if next_at <= cursor:
                block(cursor, end, scope, 'cpp-value-sequence-progress-review',
                    'Statement owner did not consume its exact source extent.', region=True)
                return
            cursor = next_at

    parse_sequence(0, len(scan), (0, len(scan), None), 0)
    result['owners'].sort(key=lambda row: (row['name_start'], row['scope_end']))
    return result


def resolve_cpp_value_owner(inventory, name, offset, *, macro_token_gates=None):
    """Positive exact source value role, explicit blocked proof, or None.

    No owner is generated from absence of a type or a familiar spelling. A
    blocked region rejects even a visible outer value because the unsupported
    imperative owner can have independent shadow/lifetime semantics.
    """
    if (not isinstance(inventory, dict) or not isinstance(name, str)
            or not re.fullmatch(_NAME, name) or not isinstance(offset, int)
            or isinstance(offset, bool) or offset < 0):
        return dict(role='unknown', kind='cpp-value-query-owner-review', unresolved=True)
    source = inventory.get('source_mask')
    if (not isinstance(source, str) or offset + len(name) > len(source)
            or source[offset:offset + len(name)] != name
            or (offset and (source[offset - 1].isalnum() or source[offset - 1] == '_'))
            or (offset + len(name) < len(source) and
                (source[offset + len(name)].isalnum() or source[offset + len(name)] == '_'))):
        return dict(role='unknown', kind='cpp-value-query-token-review', unresolved=True)
    if macro_token_gates is None:
        if inventory.get('written_macro_offsets'):
            return dict(role='unknown', kind='cpp-value-use-macro-proof-review', unresolved=True)
    else:
        macro_issues = macro_token_gates(name, offset, role='source-value-use')
        if (not isinstance(macro_issues, (list, tuple))
                or any(not isinstance(row, dict) for row in macro_issues)):
            return dict(role='unknown', kind='cpp-value-use-macro-proof-review', unresolved=True)
        if macro_issues:
            return dict(role='unknown', kind='cpp-value-use-macro-owner-review', macro=True,
                        source_evidence=macro_issues)
    blocked_regions = [row for row in inventory.get('blocked_regions', ())
                       if row['start'] <= offset < row['end']]
    if blocked_regions:
        return dict(role='unknown', kind='cpp-value-region-owner-review', unresolved=True,
                    source_evidence=blocked_regions)
    eligible = [row for row in inventory.get('owners', ()) if row['name'] == name
                and row['visible_start'] <= offset < row['scope_end']]
    blocked = [row for row in inventory.get('blocked_owners', ()) if row['name'] == name
               and row['visible_start'] <= offset < row['scope_end']]
    if blocked:
        return dict(role='unknown', kind='cpp-value-blocked-owner-review', unresolved=True,
                    source_evidence=blocked)
    types = [row for row in inventory.get('type_owners', ()) if row['name'] == name
             and row['visible_start'] <= offset < row['scope_end']]
    if types:
        # This inventory proves values only. It deliberately does not choose
        # between source type/value namespaces, nor infer that a value hides
        # an outer class/type merely because it is the nearest value record.
        return dict(role='unknown', kind='cpp-value-type-role-competition-review', unresolved=True,
                    source_type_owners=types, source_value_owners=eligible)
    if not eligible:
        return None
    best = min(eligible, key=lambda row: (row['scope_end'] - row['scope_start'],
                                         -row['visible_start']))
    same_scope = [row for row in eligible if row['scope_start'] == best['scope_start']
                  and row['scope_end'] == best['scope_end']]
    if len(same_scope) != 1:
        return dict(role='unknown', kind='cpp-value-competing-owner-review', unresolved=True,
                    source_evidence=same_scope)
    return dict(best, role='value', kind='resolved-source-written-cpp-value',
                owner_kind=best['kind'], query_offset=offset)
