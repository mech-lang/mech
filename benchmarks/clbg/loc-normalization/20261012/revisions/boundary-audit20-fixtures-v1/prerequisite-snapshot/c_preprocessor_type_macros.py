"""Unhooked source-written macro lifetime gates for proven C/C++ types.

No expansion, branch evaluation, lexer, compiler, source rewrite or LOC units.
The caller supplies an offset-preserving mask and, for exact logical directive
extents, the matching original source. Macro lifetime ignores lexical braces.
Missing source/extents, conditional events and function-like macros are gated.
"""

import re


_NAME = re.compile(r'[A-Za-z_]\w*')
_DIRECTIVE = re.compile(r'^\s*#\s*([A-Za-z_]\w*)\b')
_OPEN = {'if', 'ifdef', 'ifndef'}
_BRANCH = {'else', 'elif', 'elifdef', 'elifndef'}


def _gate(kind, offset, role, **evidence):
    return dict(kind=kind, offset=offset, role=role, units=None,
                review_required=True, **evidence)


def _logical_lines(cleaned, original):
    lines = original.splitlines(keepends=True)
    index = start = 0
    while index < len(lines):
        end = start
        splices = []
        while index < len(lines):
            line = lines[index]
            stem = line.rstrip('\r\n')
            continued = bool(len(stem) < len(line) and stem.endswith('\\'))
            if continued:
                splices.append(end + len(stem) - 1)
            end += len(line)
            index += 1
            if not continued:
                break
        chars = list(cleaned[start:end])
        for splice in splices:
            # Keep every position/newline, including a literal-masked splice.
            chars[splice - start] = ' '
        yield start, end, ''.join(chars), splices
        start = end


def collect_type_macro_events(cleaned, language, *, original_source=None,
                              protected_literal_spans=()):
    """Collect written define/undef events without deciding any branch.

    Exact original text proves backslash-newline extents; the cleaned mask
    proves that a leading '#' is code rather than comment/literal content.
    With no original text, any macro directive leaves extent uncertainty
    through the TU end. That fallback never guesses an unconditional reset.
    External flags and macros introduced by header contents are outside scope.
    """
    result = dict(events=[], directive_spans=[], diagnostics=[],
                  extent_uncertainties=[], source_length=len(cleaned),
                  protected_literal_spans=[], language=language, proposal_only=True)
    if language not in {'c', 'cpp'}:
        return result
    for span in protected_literal_spans:
        start, end = (span.get('start'), span.get('end')) if isinstance(span, dict) else (None, None)
        if (isinstance(start, int) and not isinstance(start, bool)
                and isinstance(end, int) and not isinstance(end, bool)
                and 0 <= start < end <= len(cleaned)):
            result['protected_literal_spans'].append((start, end))
        else:
            result['diagnostics'].append(_gate('c-type-macro-literal-span-review',
                                              start, 'literal-token-provenance'))
            result['extent_uncertainties'].append((0, len(cleaned), 0))
    exact = (isinstance(original_source, str) and len(original_source) == len(cleaned)
             and all(left == right for left, right in zip(original_source, cleaned)
                     if left in '\r\n' or right in '\r\n'))
    result['original_source_proven'] = exact
    if original_source is not None and not exact:
        result['diagnostics'].append(_gate('c-type-macro-source-span-review', 0,
                                          'directive-source-provenance'))
        result['extent_uncertainties'].append((0, len(cleaned), 0))
    original = original_source if exact else cleaned
    conditions = []
    for start, end, logical, splices in _logical_lines(cleaned, original):
        if exact:
            for splice in splices:
                hidden_comment = any(cleaned[start + match.start():start + match.start() + 2] == '  '
                    for match in re.finditer(r'//', original[start:splice]))
                if not hidden_comment:
                    continue
                after = splice + (3 if original[splice + 1:splice + 3] == '\r\n' else 2)
                if after < end:
                    # A physical comment mask is not phase-2 ownership proof
                    # for apparent code on subsequent rows of this group.
                    result['diagnostics'].append(_gate('c-type-preprocessor-source-ownership-review',
                        after, 'unproved-phase2-comment-source-ownership',
                        logical_group_start=start, logical_group_end=end))
                    result['extent_uncertainties'].append((after, end, start))
                break
        directive = _DIRECTIVE.match(logical)
        if directive is None:
            if re.match(r'^\s*%:', logical):
                result['diagnostics'].append(_gate('c-type-macro-directive-spelling-review',
                                                  start, 'unproved-alternative-directive-introducer'))
                result['extent_uncertainties'].append((start, len(cleaned), start))
            continue
        word = directive[1]
        directive_owned = not any(char in '\r\n' for char in logical[:directive.end()])
        result['directive_spans'].append(dict(start=start, end=end, directive=word,
                                              hash_offset=start + logical.index('#'),
                                              directive_ownership_proven=directive_owned,
                                              extent_proven=exact,
                                              splices=tuple(splices)))
        if not directive_owned:
            # A physical mask alone cannot prove comment/directive ownership
            # after phase-2 splicing. No later undef clears this uncertainty.
            result['diagnostics'].append(_gate('c-type-preprocessor-directive-ownership-review',
                start + logical.index('#'), 'unproved-phase2-comment-directive-ownership',
                logical_group_start=start))
            result['extent_uncertainties'].append((start, len(cleaned), start))
        keyword_end = start + directive.end()
        if keyword_end in splices:
            # A directive keyword split across a splice is not reconstructed
            # from token fragments by this proposal.
            result['diagnostics'].append(_gate('c-type-macro-header-splice-review',
                                              start, 'unknown-spliced-directive-keyword'))
            result['extent_uncertainties'].append((start, len(cleaned), start))
        if word in _OPEN:
            conditions.append(start)
            continue
        if word in _BRANCH:
            if not conditions:
                result['diagnostics'].append(_gate('c-type-macro-conditional-owner-review',
                                                  start, 'unmatched-conditional-directive'))
                result['extent_uncertainties'].append((start, len(cleaned), start))
            continue
        if word == 'endif':
            if conditions:
                conditions.pop()
            else:
                result['diagnostics'].append(_gate('c-type-macro-conditional-owner-review',
                                                  start, 'unmatched-conditional-directive'))
                result['extent_uncertainties'].append((start, len(cleaned), start))
            continue
        if word not in {'define', 'undef'}:
            continue
        if not exact:
            result['diagnostics'].append(_gate('c-type-macro-directive-extent-review',
                                              start, 'unproven-logical-directive-extent'))
            result['extent_uncertainties'].append((start, len(cleaned), start))
        name = re.match(r'\s+([A-Za-z_]\w*)\b', logical[directive.end():])
        if name is None:
            result['diagnostics'].append(_gate('c-type-macro-directive-name-review',
                                              start, 'unknown-macro-name'))
            result['extent_uncertainties'].append((start, len(cleaned), start))
            continue
        name_start = start + directive.end() + name.start(1)
        name_end = start + directive.end() + name.end(1)
        tail = logical[name_end - start:]
        function_like = word == 'define' and tail.startswith('(')
        malformed = word == 'undef' and bool(tail.strip())
        if name_end in splices:
            malformed = True
            result['diagnostics'].append(_gate('c-type-macro-header-splice-review',
                                              start, 'unknown-spliced-macro-name'))
            result['extent_uncertainties'].append((start, len(cleaned), start))
        entry = dict(kind='source-written-macro-' + word, directive=word,
                     name=name[1], start=start, end=end, visible_start=end,
                     hash_offset=start + logical.index('#'),
                     name_start=name_start, name_end=name_end,
                     conditional=bool(conditions), function_like=function_like,
                     continued=bool(splices), malformed=malformed,
                     extent_proven=exact, directive_ownership_proven=directive_owned,
                     source_text=cleaned[start:end],
                     primary_scope_source='https://eel.is/c++draft/cpp.scope',
                     units=None)
        result['events'].append(entry)
        if malformed:
            result['diagnostics'].append(_gate('c-type-macro-directive-form-review',
                                              start, 'malformed-undef', name=name[1]))
    for start in conditions:
        result['diagnostics'].append(_gate('c-type-macro-conditional-owner-review',
                                          start, 'unclosed-conditional-directive'))
        result['extent_uncertainties'].append((start, len(cleaned), start))
    return result


def _state(environment, name, offset):
    trace = []
    state = 'inactive'
    for event in environment.get('events', ()):
        if event['name'] != name or offset is not None and event['visible_start'] > offset:
            continue
        trace.append(event)
        if (offset is None or event['conditional'] or event['malformed']
                or not event['extent_proven'] or not event['directive_ownership_proven']):
            state = 'uncertain'
        elif event['directive'] == 'define':
            state = 'active'
        else:
            state = 'inactive'
    return state, trace


def type_macro_name_gates(environment, name, offset, *, role='type-use',
                          source_token=True):
    """Gate each identifier component of a known type/contract name.

    For a source token, component offsets are real offsets into that spelling.
    A header contract uses source_token=False: the offset anchors the imported
    contract, and no claim is made that its protected filename is expanded.
    Unknown function-like invocation grammar stays conservative in this phase.
    """
    result = []
    if offset is not None and (not isinstance(offset, int) or isinstance(offset, bool)
                               or not 0 <= offset <= environment.get('source_length', -1)):
        return [_gate('c-type-macro-token-span-review', offset, role)]
    for token in _NAME.finditer(name):
        at = offset + token.start() if offset is not None and source_token else offset
        if source_token and at is not None and any(
                start <= at < end for start, end in environment.get('protected_literal_spans', ())):
            continue
        state, trace = _state(environment, token[0], at)
        extents = [start for start, end, _ in environment.get('extent_uncertainties', ())
                   if at is None or start <= at < end]
        if state != 'inactive' or extents:
            result.append(_gate('c-type-preprocessor-macro-owner-review', at, role,
                token_name=token[0], macro_state='uncertain' if extents else state,
                source_token=source_token, directive_offsets=[row['start'] for row in trace],
                macro_name_offsets=[row['name_start'] for row in trace],
                extent_uncertainty_offsets=extents,
                function_like=any(row['function_like'] for row in trace),
                continued=any(row['continued'] for row in trace)))
    return result


def type_macro_token_gates(environment, text, offset, *, role='type-use'):
    """Query an already proven source type span; never discover its grammar."""
    result = type_macro_name_gates(environment, text, offset, role=role)
    if offset is not None:
        for span in environment.get('directive_spans', ()):
            if span['start'] <= offset < span['end']:
                result.append(_gate('c-type-macro-directive-token-owner-review', offset, role,
                                    directive_start=span['start'], directive_end=span['end']))
                break
    return result


def type_macro_owner_gates(environment, entry, *, prior_contract_owners=()):
    """Query the actual declaration point; later undef cannot repair that owner.

    Standard class-header contract names are not filename tokens. Their macro
    exposure at import is unknown contract provenance until physical header
    declaration ownership is reviewed. Namespace imports query only their
    actual written directive, so a clean directive can remain usable after an
    unrelated vector macro is unconditionally undefined.
    """
    kind = entry.get('kind', '')
    if kind in {'explicit-header-class-template-contract', 'explicit-header-type-contract'}:
        name = entry.get('template_contract', {}).get('name', entry['name'])
        gates = type_macro_name_gates(environment, name, entry['definition_end'],
            role='imported-type-contract-macro-exposure', source_token=False)
        for span in environment.get('directive_spans', ()):
            if (span['start'] <= entry['definition_start'] < span['end']
                    and not span.get('directive_ownership_proven')):
                gates.append(_gate('c-type-preprocessor-directive-ownership-review',
                    entry['definition_start'], 'imported-contract-directive-ownership-unproven',
                    hash_offset=span['hash_offset'], logical_group_start=span['start']))
            if (span['start'] <= entry['definition_start'] < span['end']
                    and span['directive'] != 'include'):
                gates.append(_gate('c-type-macro-directive-token-owner-review',
                    entry['definition_start'], 'imported-contract-inside-other-directive',
                    directive_start=span['start'], directive_end=span['end']))
        for prior in prior_contract_owners:
            if (prior.get('macro') and prior.get('name') == entry['name']
                    and prior.get('kind') in {'explicit-header-class-template-contract',
                                              'explicit-header-type-contract'}
                    and prior['definition_start'] < entry['definition_start']):
                # An include guard may prevent a clean repeated include from
                # supplying a fresh declaration. No physical header is read.
                gates.append(_gate('c-type-preprocessor-macro-owner-review',
                    entry['definition_start'], 'prior-imported-type-contract-macro-exposure',
                    contract_name=name, source_token=False,
                    prior_contract_start=prior['definition_start']))
        return gates
    text = entry.get('source_text')
    start = entry.get('definition_start')
    if not isinstance(text, str) or not isinstance(start, int):
        return [_gate('c-type-macro-owner-source-review', start, 'type-declaration-provenance')]
    return type_macro_token_gates(environment, text, start, role='type-declaration-point')


def with_type_macro_owner_gate(environment, entry, *, prior_contract_owners=()):
    """Return a labelled copy; clean later declarations have separate copies."""
    gates = type_macro_owner_gates(environment, entry,
                                   prior_contract_owners=prior_contract_owners)
    return dict(entry, macro=True, macro_type_gates=gates) if gates else dict(entry)


def without_type_macro_directives(cleaned, environment):
    """Scratch-only exclusion for type/scope collectors, preserving positions.

    The caller retains its original mask for directive/data/physical counting.
    This projection only prevents continued macro replacement text from owning
    a source declaration; it never supplies replacement tokens or LOC units.
    """
    chars = list(cleaned)
    for span in environment.get('directive_spans', ()):
        for offset in range(span['start'], span['end']):
            if not chars[offset].isspace():
                chars[offset] = ' '
    return ''.join(chars)
