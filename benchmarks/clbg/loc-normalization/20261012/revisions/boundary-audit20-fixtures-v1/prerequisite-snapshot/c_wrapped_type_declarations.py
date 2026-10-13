"""Diagnostics for unowned declarations with cross-row type prefixes.

No LOC units, canonical events, source masks, or source rewriting are returned.
The caller supplies exact contexts and consolidated ownership evidence. Type
and optional named-tail proof use supplied callbacks at actual source offsets.
"""

import re


_CODE = {'code', 'bare-code'}
_HEADER_SCOPES = {'braced', 'unbraced', 'case', 'cpp-constructor-header'}
_BRACE_SCOPES = {'braced', 'cpp-constructor-body'}
_PROTECTED = {'field', 'formal', 'function-profile', 'prototype', 'control-header',
              'type-header', 'whole-alias', 'literal', 'data', 'initializer'}
_PROFILE = {'function-profile', 'prototype', 'control-header', 'type-header'}


def _gate(kind, offset, **evidence):
    return dict(kind=kind, offset=offset, units=None, review_required=True, **evidence)


def _span(row, length, *, allow_empty=False):
    if not isinstance(row, dict):
        return None
    start, end = row.get('start'), row.get('end')
    if (isinstance(start, int) and not isinstance(start, bool)
            and isinstance(end, int) and not isinstance(end, bool)
            and 0 <= start <= end <= length and (allow_empty or start < end)):
        return start, end
    return None


def _structural(context):
    last = max((index for index, (_, kind, _) in enumerate(context) if kind in _CODE),
               default=-1)
    return any(char in '([' or kind == 'data' for char, kind, _ in context[last + 1:])


def classify_wrapped_type_declaration_gates(body_cleaned, language, contexts,
        type_environment, *, type_prefix, declaration_probe=None, type_source=None,
        owned_boundary_events=(), protected_spans=(), preprocessor_directive_spans=()):
    """Return exact source-span review diagnostics, never declaration counts.

    body_cleaned has consolidated canonical events already blanked. Optional
    type_source is the checked mask immediately before that blanking; it must
    match surviving tokens and every newline. It restores type tokens covered
    by partial canonical headers without certifying a complete function profile.
    Protected spans require owner_proven=True and an exact documented role.
    Canonical event kind/scope records supply their existing ownership proof.
    Directives require exact extent and directive ownership metadata.

    Complete independent semicolon windows continue across physical rows.
    Only proven header/open/close/whole-alias/directive boundaries reset them;
    operand, value-expression and annotation endpoints never do. Actual masked
    independent semicolons remain boundaries when type_source is supplied.
    Aggregate aliases protect their header/suffix, not their unowned field body.
    """
    result = dict(diagnostics=[], proposal_only=True,
                  source_length=len(body_cleaned) if isinstance(body_cleaned, str) else None)
    if language not in {'c', 'cpp'}:
        return result
    if not isinstance(body_cleaned, str):
        result['diagnostics'].append(_gate('cpp-wrapped-type-source-provenance-review', 0,
                                          role='unproved-body-source-text'))
        return result
    length = len(body_cleaned)
    source = body_cleaned if type_source is None else type_source
    if not all(isinstance(rows, (tuple, list)) for rows in
               (owned_boundary_events, protected_spans, preprocessor_directive_spans)):
        result['diagnostics'].append(_gate('cpp-wrapped-type-source-provenance-review', 0,
                                          role='unproved-ownership-span-collections'))
        return result
    events = list(owned_boundary_events)
    protected = list(protected_spans)
    directives = list(preprocessor_directive_spans)

    def bad(offset, role):
        result['diagnostics'].append(_gate('cpp-wrapped-type-source-provenance-review',
                                          offset, role=role))

    def uncertain(row):
        return any(row.get(flag) for flag in ('review_required', 'conditional', 'macro',
                                             'unresolved', 'namespace_prefix_review'))

    if (not isinstance(type_environment, dict) or not isinstance(source, str) or len(source) != length
            or not isinstance(contexts, (tuple, list))
            or type_environment.get('source_length') != length
            or len(contexts) != length):
        bad(0, 'source-context-environment-extent-mismatch')
        return result
    aliases = type_environment.get('aliases', {})
    diagnostics = type_environment.get('diagnostics', ())
    if (not isinstance(aliases, dict)
            or any(not isinstance(rows, (tuple, list))
                   or any(not isinstance(row, dict) for row in rows) for rows in aliases.values())
            or not isinstance(diagnostics, (tuple, list))
            or any(not isinstance(row, dict) for row in diagnostics)):
        bad(0, 'unproved-type-environment-shape')
        return result
    if not callable(type_prefix) or declaration_probe is not None and not callable(declaration_probe):
        bad(0, 'unproved-type-or-declaration-callback')
        return result
    for index, context in enumerate(contexts):
        if (not isinstance(context, (tuple, list)) or any(
                not isinstance(entry, (tuple, list)) or len(entry) != 3
                or entry[0] not in {'(', '[', '{'} or not isinstance(entry[1], str)
                or not isinstance(entry[2], int) or isinstance(entry[2], bool)
                or not 0 <= entry[2] < index for entry in context)):
            bad(index, 'unproved-context-frame-offset')
            return result
    for row in events:
        span = _span(row, length, allow_empty=True)
        if span is None or any(not char.isspace() for char in body_cleaned[span[0]:span[1]]):
            bad(span[0] if span is not None else 0, 'canonical-event-not-exactly-blanked')
    for row in protected:
        span = _span(row, length)
        if (span is None or row.get('role') not in _PROTECTED
                or row.get('owner_proven') is not True or uncertain(row)):
            bad(span[0] if span is not None else 0, 'unproved-protected-owner-span')
    for row in directives:
        span = _span(row, length)
        if (span is None or row.get('extent_proven') is not True
                or row.get('directive_ownership_proven') is not True):
            bad(span[0] if span is not None else 0, 'unproved-preprocessor-boundary-span')
    if result['diagnostics']:
        return result
    event_spans = [_span(row, length, allow_empty=True) for row in events]
    for index, (old, current) in enumerate(zip(source, body_cleaned)):
        if ((old in '\r\n' or current in '\r\n') and old != current
                or not current.isspace() and old != current
                or old != current and not any(start <= index < end for start, end in event_spans)):
            bad(index, 'source-projection-token-or-newline-mismatch')
            return result

    anchors = []
    for row in events:
        if row['start'] == row['end']:
            continue # Empty enum/interior ownership is no reset anchor.
        kind, scope = row.get('kind'), row.get('scope')
        reset = ((kind in {'header', 'branch'} and scope in _HEADER_SCOPES)
                 or kind in {'open', 'close'} and scope in _BRACE_SCOPES
                 or kind == 'type-alias-binding' and scope == 'source-local-type-alias')
        if reset and not uncertain(row):
            anchors.append(dict(row, boundary_role='canonical-' + kind))
        elif reset:
            bad(row['start'], 'unproved-owned-reset-boundary')
    aggregates = {}
    for row in protected:
        start, end = row['start'], row['end']
        if row['role'] == 'whole-alias':
            aggregate = next((entry for entry in type_environment.get('aggregate_alias_suffixes', ())
                              if entry['owner_start'] == start and entry['owner_end'] == end), None)
            if aggregate is not None:
                close = aggregate.get('closing_brace')
                opening = source.find('{', start, close) if isinstance(close, int) else -1
                if (opening < start or not isinstance(close, int) or not opening < close < end
                        or source[close] != '}' or uncertain(aggregate)):
                    bad(start, 'unproved-aggregate-alias-body-exception')
                    continue
                aggregates[id(row)] = (opening + 1, close)
                anchors.extend((dict(start=start, end=opening + 1, boundary_role='aggregate-alias-header'),
                                dict(start=close, end=end, boundary_role='aggregate-alias-suffix')))
            else:
                anchors.append(dict(row, boundary_role='whole-alias'))
        elif row['role'] in _PROFILE:
            anchors.append(dict(row, boundary_role='exact-' + row['role']))
    anchors.extend(dict(row, boundary_role='preprocessor-directive') for row in directives)
    if result['diagnostics']:
        return result
    anchors.sort(key=lambda row: (row['start'], -row['end']))

    def excluded(at, end):
        for row in protected:
            body = aggregates.get(id(row))
            if body is not None and body[0] <= at < body[1]:
                continue
            if not row['start'] <= at < row['end']:
                continue
            if row['role'] == 'field' and end > row['end']:
                continue
            return True
        return False

    def inspect(start, end, terminator=None, boundary=None):
        visible_end = boundary['start'] if boundary is not None else end
        segment = body_cleaned[start:visible_end]
        leading = len(segment) - len(segment.lstrip())
        at = start + leading
        if at >= visible_end or _structural(contexts[at]) or excluded(at, end):
            return
        probe = source[at:end]
        typed = type_prefix(probe, type_environment, at)
        if typed is None:
            if (boundary is not None and boundary.get('kind') in {'header', 'branch'}
                    and '\n' in segment[leading:]):
                result['diagnostics'].append(_gate('cpp-wrapped-header-owner-review', at,
                    statement_start=start, statement_end=end, boundary_start=boundary['start'],
                    boundary_role=boundary['boundary_role'], type_proof_available=False,
                    reason='cross-row-residual-before-partial-header-has-no-complete-profile-owner'))
            return
        prefix_end, type_name, evidence = typed
        if (not isinstance(prefix_end, int) or isinstance(prefix_end, bool)
                or not 0 < prefix_end <= len(probe)):
            bad(at, 'invalid-type-prefix-span-proof')
            return
        type_end = at + prefix_end
        declaration = (declaration_probe(probe, type_environment, offset=at)
                       if declaration_probe is not None and terminator is not None else None)
        names = [row['name'] for row in declaration.get('declarators', ())] if declaration else []
        if names:
            base = declaration.get('declarator_start')
            points = []
            if not isinstance(base, int) or isinstance(base, bool) or not prefix_end <= base < len(probe):
                bad(at, 'unproved-declarator-start-offset')
                return
            for row in declaration['declarators']:
                relative, name = row.get('name_offset'), row['name']
                if (not isinstance(relative, int) or isinstance(relative, bool) or relative < 0
                        or not isinstance(name, str) or not name):
                    bad(at, 'unproved-declarator-name-offset')
                    return
                point = at + base + relative
                if not type_end <= point < end or source[point:point + len(name)] != name:
                    bad(at, 'declarator-name-offset-source-mismatch')
                    return
                points.append(point)
            rows = [source.count('\n', 0, point) + 1 for point in points]
            type_row = source.count('\n', 0, at) + 1
            if any(row != type_row for row in rows):
                result['diagnostics'].append(_gate('cpp-wrapped-independent-declarators-owner-review', at,
                    statement_start=start, statement_end=end, terminator_offset=terminator,
                    type_start=at, type_end=type_end, type_name=type_name, type_evidence=evidence,
                    statement_span=dict(start=start, end=end), type_span=dict(start=at, end=type_end),
                    type_start_row=type_row,
                    declaration_names=names, declaration_name_offsets=points, declaration_name_rows=rows,
                    source_text=source[start:end],
                    reason='complete-proven-declarator-name-is-on-a-different-row-from-type-start'))
        if '\n' not in source[at:type_end]:
            return
        tail = probe[prefix_end:].lstrip()
        named_tail = bool(re.match(r'(?:[*&]\s*)*[A-Za-z_]\w*\b', tail)
                          or re.match(r'\(\s*[*&]+\s*[A-Za-z_]\w*\b', tail))
        if not names and not named_tail:
            # Constructor/cast temporaries beginning directly with ()/{} do
            # not supply independent declaration names.
            return
        result['diagnostics'].append(_gate('cpp-wrapped-proven-type-declaration-owner-review', at,
            statement_start=start, statement_end=end, terminator_offset=terminator,
            type_start=at, type_end=type_end,
            statement_span=dict(start=start, end=end), type_span=dict(start=at, end=type_end),
            type_name=type_name, type_evidence=evidence,
            declaration_owner_proven=bool(names), declaration_names=names,
            tail_review_required=not bool(names), boundary_role=(boundary or {}).get('boundary_role'),
            source_text=source[start:end],
            reason='proven-type-prefix-crosses-physical-rows-without-complete-statement-owner'))

    start = index = anchor_index = 0
    while index < length:
        while anchor_index < len(anchors) and anchors[anchor_index]['end'] <= index:
            anchor_index += 1
        if anchor_index < len(anchors) and anchors[anchor_index]['start'] <= index:
            anchor = anchors[anchor_index]
            inspect(start, anchor['end'], boundary=anchor)
            start = index = anchor['end']
            anchor_index += 1
            continue
        if source[index] == ';' and not _structural(contexts[index]):
            inspect(start, index + 1, terminator=index)
            start = index + 1
        index += 1
    return result
