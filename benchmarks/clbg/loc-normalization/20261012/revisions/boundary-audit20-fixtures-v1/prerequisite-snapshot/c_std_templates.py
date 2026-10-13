"""Unhooked, bounded imported C++ standard class-template type proofs.

Callers supply the existing offset-preserving mask, semantic include records,
brace/conditional ranges and a proven, scope/order/shadow-aware resolver with
signature ``resolver(environment, name, absolute_offset)``. This helper neither
lexes nor changes source, counts bindings, hides spans, or admits LOC results.
Unknown template owners, dependent types and size expressions remain gated.
"""

import re
from c_preprocessor_type_macros import type_macro_token_gates, with_type_macro_owner_gate
from c_std_namespace_prefix_gate import collect_std_namespace_prefix_gates


_NAME = r'[A-Za-z_]\w*'
_QUALIFIED = re.compile(_NAME + r'(?:::' + _NAME + r')*')
_WORD = re.compile(_NAME)
_CV = {'const', 'volatile'}
_MODIFIERS = _CV | {'static', 'register', 'extern', 'inline', 'constexpr',
                    'mutable', 'thread_local', 'restrict', '__restrict'}
_BUILTIN = re.compile(
    r'(?:unsigned|signed)\s+__int128'
    r'|(?:unsigned|signed)(?:\s+(?:char|short(?:\s+int)?|long(?:\s+long)?(?:\s+int)?|int))?'
    r'|long\s+double|long(?:\s+long)?(?:\s+int)?|short(?:\s+int)?'
    r'|void|bool|char|int|float|double|_Bool|__int128')
_PLAIN_OWNERS = {'source-local-cpp-named-type', 'source-local-typedef',
                 'source-local-using', 'explicit-header-type-contract',
                 'explicit-standard-namespace-import'}
_MAX_DEPTH = 16
_UNBALANCED = {'c-type-environment-unbalanced-scope-review',
               'c-type-environment-unclosed-scope-review'}

# Primary C++ working-draft header synopses, read 2026-10-12. Defaults are
# contract-owned structural syntax; no allocator template is inferred/imported.
_CONTRACTS = {
    'vector': dict(name='std::vector', header='vector',
                   parameters=('type', 'type'), minimum_arguments=1,
                   defaults=(None, 'allocator<T>'),
                   primary_source='https://eel.is/c++draft/vector.syn'),
    'complex': dict(name='std::complex', header='complex',
                    parameters=('type',), minimum_arguments=1,
                    defaults=(None,),
                    primary_source='https://eel.is/c++draft/complex.syn'),
    'array': dict(name='std::array', header='array',
                  parameters=('type', 'size-value'), minimum_arguments=2,
                  defaults=(None, None),
                  primary_source='https://eel.is/c++draft/array.syn'),
}


def _space(text, cursor):
    while cursor < len(text) and text[cursor].isspace():
        cursor += 1
    return cursor


def _scope_at(ranges, offset):
    owners = [row for row in ranges if row[0] <= offset < row[1]]
    return min(owners, key=lambda row: row[1] - row[0]) if owners else None


def _conditional(regions, offset):
    return any(start <= offset < end for start, end in regions)


def _without_directives(cleaned):
    chars = list(cleaned)
    start = 0
    continuation = False
    for line in cleaned.splitlines(keepends=True):
        directive = continuation or bool(re.match(r'^[ \t]*#', line))
        if directive:
            for cursor in range(start, start + len(line)):
                if not chars[cursor].isspace():
                    chars[cursor] = ' '
        continuation = directive and line.rstrip('\r\n').endswith('\\')
        start += len(line)
    return ''.join(chars)


def _usable(definition, offset):
    return bool(definition and not any(definition.get(flag) for flag in
                ('conditional', 'dependent', 'template_formal', 'macro', 'unresolved',
                 'namespace_prefix_review'))
                and definition.get('visible_start', offset) <= offset
                < definition.get('scope_end', offset + 1))


def _contract(definition, environment, offset, resolver):
    if not _usable(definition, offset):
        return None
    kind = definition.get('kind')
    if kind not in {'explicit-header-class-template-contract',
                    'explicit-standard-template-namespace-import'}:
        return None
    supplied = definition.get('template_contract')
    if not isinstance(supplied, dict):
        return None
    contract = _CONTRACTS.get(supplied.get('header'))
    if contract is None or supplied != contract:
        return None
    if kind == 'explicit-standard-template-namespace-import':
        # A namespace using-directive cannot replace a visible source type.
        # Conservatively gate conflicting bare-name owners instead of relying
        # on the resolver's latest-visible declaration ranking for imports.
        if any(row.get('kind') not in {
                    'explicit-standard-template-namespace-import',
                    'explicit-standard-namespace-import'}
               and row.get('visible_start', offset) <= offset
               < row.get('scope_end', offset + 1)
               for row in environment.get('aliases', {}).get(definition['name'], ())):
            return None
        # Recheck the qualified owner at the use site, after later shadows.
        qualified = resolver(environment, contract['name'], offset)
        if (not _usable(qualified, offset)
                or qualified.get('kind') != 'explicit-header-class-template-contract'
                or qualified.get('template_contract') != contract):
            return None
    return contract


def collect_std_template_contracts(cleaned, language, environment, *, resolver,
                                   semantic_imports=(), scope_ranges,
                                   conditional_regions=()):
    """Return entries for exact angle-header imports and explicit std imports.

    ``environment`` is read only. A local aliases overlay lets the supplied
    resolver see these proposed header entries while proving ``using namespace
    std;``. The parent must merge entries before using the prefix API and must
    collect ordinary/formal/function/conditional shadows for the enlarged name
    set. The overlay does not replace those passes. Missing or unbalanced scope
    proof is a gate, and quoted/local/transitive headers establish no contract.
    """
    result = dict(entries=[], evidence=[], diagnostics=[], proposal_only=True,
                  namespace_directive_spans=[])
    if language != 'cpp' or environment.get('language') != 'cpp':
        return result
    if any(row.get('kind') in _UNBALANCED for row in environment.get('diagnostics', ())):
        result['diagnostics'].append(dict(kind='cpp-template-unbalanced-source-owner-review'))
        return result
    view = dict(environment)
    view['aliases'] = {name: list(rows) for name, rows in environment.get('aliases', {}).items()}

    def add(name, start, end, scope, kind, contract):
        entry = dict(name=name, type_name=contract['name'], kind=kind,
                     definition_start=start, definition_end=end, visible_start=end,
                     scope_start=scope[0], scope_end=scope[1], scope_opening=scope[2],
                     conditional=_conditional(conditional_regions, start),
                     source_text=cleaned[start:end], template_contract=dict(contract))
        macros = environment.get('type_macro_environment')
        if macros is not None:
            prior = [row for rows in view['aliases'].values() for row in rows]
            entry = with_type_macro_owner_gate(macros, entry, prior_contract_owners=prior)
            result['diagnostics'].extend(entry.get('macro_type_gates', ()))
        result['entries'].append(entry)
        result['evidence'].append(dict(entry))
        view['aliases'].setdefault(name, []).append(entry)
        return entry

    for semantic in semantic_imports:
        header = semantic.get('header')
        contract = _CONTRACTS.get(header)
        if contract is None:
            continue
        start, end = semantic.get('start'), semantic.get('end')
        valid_span = (isinstance(start, int) and not isinstance(start, bool)
                      and isinstance(end, int) and not isinstance(end, bool)
                      and 0 <= start < end <= len(cleaned))
        if valid_span:
            line_start = cleaned.rfind('\n', 0, start) + 1
            line_end = cleaned.find('\n', end)
            line_end = len(cleaned) if line_end < 0 else line_end
            valid_span = (semantic.get('spelling') == '<' + header + '>'
                          and end - start == len(semantic['spelling'])
                          and bool(re.fullmatch(r'[ \t]*#[ \t]*include[ \t]*',
                                               cleaned[line_start:start]))
                          and all(char in 'S \t' for char in cleaned[start:end])
                          and not cleaned[end:line_end].strip())
        if not valid_span:
            result['diagnostics'].append(dict(kind='cpp-template-import-span-owner-review',
                                              offset=start, header=header))
            continue
        scope = _scope_at(scope_ranges, line_start)
        if scope is None or scope[2] is not None:
            result['diagnostics'].append(dict(kind='cpp-template-import-scope-owner-review',
                                              offset=start, header=header))
            continue
        entry = add(contract['name'], line_start, end, scope,
                    'explicit-header-class-template-contract', contract)
        entry['semantic_import'] = dict(semantic)
        result['evidence'][-1]['semantic_import'] = dict(semantic)
        if entry['conditional']:
            result['diagnostics'].append(dict(kind='cpp-template-conditional-owner-review',
                                              offset=line_start, name=entry['name']))

    scan = _without_directives(cleaned)
    for using in re.finditer(r'\busing\s+namespace\s+std\s*;', scan):
        scope = _scope_at(scope_ranges, using.start())
        if scope is None or _conditional(conditional_regions, using.start()):
            result['diagnostics'].append(dict(kind='cpp-template-namespace-owner-review',
                                              offset=using.start()))
            continue
        directive_recorded = False
        for contract in _CONTRACTS.values():
            qualified = resolver(view, contract['name'], using.start())
            if _contract(qualified, view, using.start(), resolver) is not None:
                if not directive_recorded:
                    # Directive namespace/scope/order proof precedes member
                    # conflicts; a bare vector conflict cannot hide this span
                    # from the independent qualified std-prefix review.
                    directive = dict(name='std', kind='explicit-standard-namespace-directive-owner',
                        definition_start=using.start(), definition_end=using.end(),
                        visible_start=using.end(), scope_start=scope[0], scope_end=scope[1],
                        scope_opening=scope[2], conditional=False,
                        source_text=cleaned[using.start():using.end()], units=None,
                        accepted_type_owner=False)
                    macros = environment.get('type_macro_environment')
                    if macros is not None:
                        directive = with_type_macro_owner_gate(macros, directive)
                        result['diagnostics'].extend(directive.get('macro_type_gates', ()))
                    if not directive.get('macro'):
                        result['namespace_directive_spans'].append(directive)
                    directive_recorded = True
                entry = add(contract['name'].removeprefix('std::'), using.start(),
                            using.end(), scope,
                            'explicit-standard-template-namespace-import', contract)
                entry['qualified_owner'] = dict(qualified)
                result['evidence'][-1]['qualified_owner'] = dict(qualified)
    if result['entries']:
        macros = environment.get('type_macro_environment', {})
        prefix_gates = collect_std_namespace_prefix_gates(cleaned, language,
            owned_namespace_import_spans=result['namespace_directive_spans'],
            protected_literal_spans=macros.get('protected_literal_spans', ()),
            protected_directive_spans=macros.get('directive_spans', ()))
        result['namespace_prefix_review'] = prefix_gates['blocked']
        result['diagnostics'].extend(prefix_gates['diagnostics'])
        if prefix_gates['blocked']:
            for entry in result['entries'] + result['evidence']:
                entry['namespace_prefix_review'] = True
                entry['namespace_prefix_review_source_wide'] = True
    return result


def _type(text, cursor, environment, offset, resolver, depth, *, indirection=True,
          macro_gate_reporter=None):
    if depth > _MAX_DEPTH:
        return None
    cursor = _space(text, cursor)
    start = cursor
    while True:
        word = _WORD.match(text, cursor)
        if word is None or word[0] not in _CV:
            break
        cursor = _space(text, word.end())
    name_start = cursor
    builtin = _BUILTIN.match(text, cursor)
    if builtin and (builtin.end() == len(text)
                    or not (text[builtin.end()].isalnum() or text[builtin.end()] == '_')):
        cursor = builtin.end()
        evidence = dict(kind='builtin-type', type_name=builtin[0],
                        start=offset + name_start, end=offset + cursor)
    else:
        named = _QUALIFIED.match(text, cursor)
        if named is None:
            return None
        definition = resolver(environment, named[0], offset + cursor)
        if not _usable(definition, offset + cursor):
            return None
        cursor = _space(text, named.end())
        contract = _contract(definition, environment, offset + name_start, resolver)
        if cursor < len(text) and text[cursor] == '<':
            if contract is None:
                return None
            opening = cursor
            cursor += 1
            arguments = []
            while True:
                cursor = _space(text, cursor)
                parameter = len(arguments)
                if parameter >= len(contract['parameters']):
                    return None
                if contract['parameters'][parameter] == 'type':
                    argument = _type(text, cursor, environment, offset, resolver, depth + 1,
                                     macro_gate_reporter=macro_gate_reporter)
                    if argument is None:
                        return None
                    cursor, argument_evidence = argument
                else:
                    # Only a decimal literal is proven here, never a presumed
                    # value name, macro, expression, or dependent formal.
                    literal = re.match(r'(?:0|[1-9][0-9]*)(?![\w.])', text[cursor:])
                    if literal is None:
                        return None
                    argument_evidence = dict(kind='decimal-size-value',
                        start=offset + cursor, end=offset + cursor + literal.end(),
                        spelling=literal[0], units=0)
                    cursor += literal.end()
                arguments.append(argument_evidence)
                cursor = _space(text, cursor)
                if cursor >= len(text):
                    return None
                if text[cursor] == '>':
                    cursor += 1
                    break
                if text[cursor] != ',':
                    return None
                cursor += 1
            if len(arguments) < contract['minimum_arguments']:
                return None
            evidence = dict(kind='proven-standard-template-specialization',
                type_name=text[name_start:cursor], start=offset + name_start,
                end=offset + cursor, template_opening=offset + opening,
                owner=definition, contract=dict(contract), arguments=arguments,
                omitted_defaults=contract['defaults'][len(arguments):], units=0)
        else:
            if contract is not None or definition.get('kind') not in _PLAIN_OWNERS:
                return None
            cursor = named.end()
            evidence = dict(kind='resolved-source-or-header-type', type_name=named[0],
                            start=offset + name_start, end=offset + cursor,
                            owner=definition, units=0)
    # Bounded cv/pointer/reference type-id tails are structural argument syntax.
    while True:
        tail = _space(text, cursor)
        word = _WORD.match(text, tail)
        if word and word[0] in _CV:
            cursor = word.end()
        else:
            break
    if indirection:
        while True:
            tail = _space(text, cursor)
            if tail >= len(text) or text[tail] != '*':
                break
            cursor = tail + 1
            while True:
                tail = _space(text, cursor)
                word = _WORD.match(text, tail)
                if word is None or word[0] not in _CV:
                    break
                cursor = word.end()
        tail = _space(text, cursor)
        if text.startswith('&&', tail):
            cursor = tail + 2
        elif tail < len(text) and text[tail] == '&':
            cursor = tail + 1
    evidence = dict(evidence, type_id_start=offset + start,
                    type_id_end=offset + cursor, type_id=text[start:cursor], units=0)
    macros = environment.get('type_macro_environment')
    if macros is not None:
        gates = type_macro_token_gates(macros, text[start:cursor], offset + start,
                                      role='proven-standard-template-type-id')
        if gates:
            if macro_gate_reporter is not None:
                macro_gate_reporter(gates)
            return None
    return cursor, evidence


def prove_std_template_prefix(code, environment, offset, *, resolver,
                              macro_gate_reporter=None):
    """Return (end, exact template spelling, proof) or None; never count units.

    ``offset`` is the absolute start of ``code`` in the collected masked source.
    Only explicit imported vector/complex/array specializations qualify. The
    caller owns declaration/context exclusion, independent names, constructor
    suffixes, initializer effects and all literal/data wrapping decisions.
    Template member suffixes, source template aliases/formals and expressions
    containing angle operators are not promoted into type ownership.
    """
    if (environment.get('language') != 'cpp' or not isinstance(offset, int)
            or isinstance(offset, bool) or offset < 0
            or offset >= environment.get('source_length', offset + 1)
            or any(row.get('kind') in _UNBALANCED for row in environment.get('diagnostics', ()))):
        return None
    cursor = _space(code, 0)
    while True:
        word = _WORD.match(code, cursor)
        if word is None or word[0] not in _MODIFIERS:
            break
        cursor = _space(code, word.end())
    parsed = _type(code, cursor, environment, offset, resolver, 0, indirection=False,
                   macro_gate_reporter=macro_gate_reporter)
    if parsed is None:
        return None
    end, evidence = parsed
    if evidence['kind'] != 'proven-standard-template-specialization':
        return None
    if end < len(code) and not (code[end].isspace() or code[end] in '*&'):
        return None
    return end, evidence['type_name'], evidence
