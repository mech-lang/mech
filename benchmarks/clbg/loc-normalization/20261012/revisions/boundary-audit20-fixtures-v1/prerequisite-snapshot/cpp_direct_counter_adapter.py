"""Unhooked, single-row C++ declaration counter adapter.

This is a proposal API for a future normalizer caller. It does not replace
identify_decl for field, formal, type-shadow or constructor inventories. The
caller supplies an independently proved complete semicolon statement owner,
the unchanged lexical mask, effective contexts, all protected owners and the
actual literal/value/type inventories. No row/semicolon alone proves a whole
statement. This module reads no files, lexes nothing, rewrites no source and
creates no header/closer events. All dependencies and fixtures are executed
only by a future worker holding the shared timing mutex.
"""

import copy
import re

from c_direct_declarators import analyze_direct_decl
from c_preprocessor_type_macros import type_macro_token_gates
from c_type_environment import _declarators, _resolve, _type_prefix, identify_decl
from cpp_value_owners import resolve_cpp_value_owner


_NAME = re.compile(r'[A-Za-z_]\w*')
_QUALIFIED = re.compile(r'[A-Za-z_]\w*(?:\s*::\s*[A-Za-z_]\w*)*')
_CV = {'const', 'volatile', 'restrict', '__restrict'}
_REJECT_FLAGS = ('conditional', 'dependent', 'macro', 'unresolved', 'shadowed',
                 'namespace_prefix_review')
_CODE = {'code', 'bare-code'}


def _integer(value):
    return isinstance(value, int) and not isinstance(value, bool)


def _issue(kind, offset, message, **evidence):
    return dict(kind=kind, offset=offset, message=message, units=None,
                review_required=True, **evidence)


def _bounds(row, length):
    return (isinstance(row, dict) and _integer(row.get('start'))
            and _integer(row.get('end')) and 0 <= row['start'] < row['end'] <= length)


def _context(contexts, at, length):
    if (not isinstance(contexts, (list, tuple)) or len(contexts) != length
            or not 0 <= at < length or not isinstance(contexts[at], (list, tuple))):
        return None
    rows = contexts[at]
    if any(not isinstance(row, (list, tuple)) or len(row) != 3
           or row[0] not in {'(', '[', '{'} or not isinstance(row[1], str)
           or not _integer(row[2]) or not 0 <= row[2] < at for row in rows):
        return None
    return tuple(tuple(row) for row in rows)


def _independent(context):
    last = max((index for index, (_, kind, _) in enumerate(context)
                if kind in _CODE), default=-1)
    # Unlike an arbitrary parenthesized expression, a declaration statement
    # begins directly in its executable/translation-unit context.
    return not context[last + 1:]


def _statement_gate(segment, offset, source_mask, contexts, statement_owner,
                    protected_spans):
    length = len(source_mask)
    if (not _bounds(statement_owner, length)
            or statement_owner.get('kind') != 'independent-semicolon-statement'
            or statement_owner.get('ownership_proven') is not True
            or statement_owner.get('terminator_offset') != statement_owner['end'] - 1):
        return _issue('cpp-direct-statement-owner-review', offset,
            'An independently proved complete statement owner is required; one row or semicolon is insufficient.')
    left, right = statement_owner['start'], statement_owner['end']
    if (not left <= offset < right or offset + len(segment) > right
            or source_mask[offset:offset + len(segment)] != segment
            or source_mask[right - 1] != ';'
            or source_mask[left:offset].strip()
            or source_mask[offset + len(segment):right].strip() not in {'', ';'}):
        return _issue('cpp-direct-statement-span-review', offset,
            'The unchanged segment must cover the complete supplied statement except whitespace and its final terminator.')
    whole = source_mask[left:right]
    if '\n' in whole or '\r' in whole:
        return _issue('cpp-direct-wrapped-statement-owner-review', left,
            'A complete wrapped declaration is outside this single-row adapter; preserve all written rows.',
            statement_start=left, statement_end=right)
    token_at = left + len(whole) - len(whole.lstrip())
    initial = _context(contexts, token_at, length)
    terminal = _context(contexts, right - 1, length)
    if initial is None or terminal is None or not _independent(initial) or terminal != initial:
        return _issue('cpp-direct-structural-statement-owner-review', token_at,
            'Formal, call, control, data and incomplete delimiter contexts are structural, not independent declaration statements.')
    for at in range(token_at, right - 1):
        if source_mask[at] == ';' and _context(contexts, at, length) == initial:
            return _issue('cpp-direct-multiple-statement-owner-review', at,
                'The supplied owner contains more than one independent semicolon statement.')
    for span in protected_spans:
        if not _bounds(span, length):
            return _issue('cpp-direct-protected-span-review', offset,
                'Every excluded header/formal/field/alias/data owner needs exact source bounds.')
        if span['start'] < right and left < span['end']:
            return _issue('cpp-direct-protected-statement-owner-review', token_at,
                'The candidate overlaps an existing structural or executable owner.',
                protected_owner=dict(span))
    boundary = statement_owner.get('start_boundary')
    if not isinstance(boundary, dict):
        return _issue('cpp-direct-statement-boundary-review', left,
            'The complete statement must retain its independently proved preceding boundary.')
    kind, at = boundary.get('kind'), boundary.get('offset')
    if kind == 'source-start':
        valid = at == 0 and not source_mask[:left].strip()
    elif kind in {'statement-terminator', 'executable-opener', 'executable-closer'}:
        expected = {'statement-terminator': ';', 'executable-opener': '{',
                    'executable-closer': '}'}[kind]
        valid = (_integer(at) and 0 <= at < left and source_mask[at] == expected
                 and not source_mask[at + 1:left].strip())
        prior = _context(contexts, at, length) if valid else None
        if kind == 'statement-terminator':
            valid = valid and prior == initial and _independent(prior)
        elif kind == 'executable-opener':
            valid = (valid and initial and initial[-1] == ('{', 'code', at)
                     and initial[:-1] == prior)
        else:
            valid = (valid and prior is not None and prior and prior[-1][0] == '{'
                     and prior[-1][1] in _CODE and prior[:-1] == initial)
    elif kind == 'owned-header-or-directive-boundary':
        owner = boundary.get('owner')
        valid = (_bounds(owner, length) and owner['end'] <= left
                 and at == owner['end'] and not source_mask[owner['end']:left].strip()
                 and owner in protected_spans)
    else:
        valid = False
    if not valid:
        return _issue('cpp-direct-statement-boundary-review', left,
            'The preceding boundary is neither a proven executable/statement delimiter nor an exact supplied owned span.')
    return None


def _literal_gate(spans, source_mask, offset):
    if not isinstance(spans, (list, tuple)):
        return _issue('cpp-direct-literal-provenance-review', offset,
            'The complete actual lexical literal inventory must be supplied, including an explicit empty inventory.')
    accepted = []
    for span in spans:
        if (not _bounds(span, len(source_mask)) or span.get('kind') != 'literal'
                or not isinstance(span.get('source_text'), str)
                or len(span['source_text']) != span['end'] - span['start']
                or span.get('provenance') != 'checked-pygments-string-mask-region'
                or not isinstance(span.get('token_kinds'), (list, tuple))
                or not span['token_kinds']
                or any(not isinstance(token, str) or not token.startswith('Token.Literal.String')
                       for token in span['token_kinds'])
                or source_mask[span['start']:span['end']] != ''.join(
                    char if char.isspace() else 'S' for char in span['source_text'])):
            return _issue('cpp-direct-literal-provenance-review', offset,
                'Literal spans require exact checked token provenance and same-width mask correspondence.')
        accepted.append(span)
    ordered = sorted(accepted, key=lambda span: span['start'])
    if any(a['end'] > b['start'] for a, b in zip(ordered, ordered[1:])):
        return _issue('cpp-direct-literal-provenance-review', offset,
            'Overlapping literal mask owners cannot prove independent source intervals.')
    return None


def _unknown(kind, **evidence):
    return dict(role='unknown', kind=kind, unresolved=True, rejected=True, **evidence)


def _source_role(environment, inventory, name, at, macro_guard):
    source = inventory['source_mask']
    found = _QUALIFIED.match(source, at)
    if (found is None or re.sub(r'\s+', '', found[0]) != name
            or at and (source[at - 1].isalnum() or source[at - 1] == '_')):
        return _unknown('cpp-direct-role-token-review', name=name, offset=at)
    gates = macro_guard(found[0], at, role='cpp-direct-source-role')
    if gates:
        return _unknown('cpp-direct-role-macro-review', source_evidence=gates)
    typed = _resolve(environment, name, at)
    if '::' in name:
        # The value inventory intentionally owns bare names only. A qualified
        # role query cannot be settled by pretending its missing bare owner
        # proves no competition. Complete type IDs have a separate adapter.
        return _unknown('cpp-direct-qualified-role-owner-review', name=name,
                        offset=at, source_type_owner=typed)
    value = resolve_cpp_value_owner(inventory, name, at, macro_token_gates=macro_guard)
    if value is not None and value.get('role') == 'unknown':
        type_only = (value.get('kind') == 'cpp-value-type-role-competition-review'
                     and value.get('source_type_owners')
                     and not value.get('source_value_owners') and typed is not None)
        if not type_only:
            return _unknown('cpp-direct-source-role-owner-review', source_evidence=value,
                            source_type_owner=typed)
        # Both inventories positively identify only the type. This is not
        # priority between competing type and value owners.
        value = None
    if typed is not None and any(typed.get(flag) for flag in _REJECT_FLAGS):
        return _unknown('cpp-direct-source-type-owner-review', source_type_owner=typed)
    if typed is not None and value is not None:
        return _unknown('cpp-direct-type-value-competition-review',
                        source_type_owner=typed, source_value_owner=value)
    if typed is not None:
        if typed.get('kind') in {'explicit-header-class-template-contract',
                                'explicit-standard-template-namespace-import'}:
            return _unknown('cpp-direct-unspecialized-template-role-review',
                            source_type_owner=typed)
        return dict(typed, role='type', query_offset=at, units=0)
    if value is not None:
        return value
    definitions = environment.get('aliases', {}).get(name, ())
    uncertain = [row for row in definitions if row['visible_start'] <= at < row['scope_end']]
    blocks = [row for row in environment.get('type_macro_lookup_blocks', ())
              if row['visible_start'] <= at < row['scope_end']]
    shadows = [row for row in environment.get('shadows', ())
               if row['name'] == name and row['start'] <= at < row['end']]
    if uncertain or blocks or shadows:
        return _unknown('cpp-direct-unresolved-source-type-role-review',
                        source_type_owners=uncertain, lookup_blocks=blocks, shadows=shadows)
    # Only actual absence can reach the direct helper's approved bare-name
    # equal-count profile. Compound leaves remain unresolved in that helper.
    return None


def _proven_type_conflict(code, at, prefix, environment, inventory, macro_guard):
    """Keep independent type proof from bypassing blocked value ownership.

    Positive imported/source type evidence supplies the name; source matching
    supplies only its use offset. A qualified positive type contract remains
    the type environment's responsibility (including its namespace gates),
    rather than querying the bare-only value inventory with a made-up token.
    """
    evidence = prefix[2]
    queries = []
    def visit(proof):
        owner = proof.get('owner')
        if isinstance(owner, dict) and isinstance(owner.get('name'), str):
            queries.append((owner['name'], proof.get('start')))
        for argument in proof.get('arguments', ()):
            if isinstance(argument, dict):
                visit(argument)
    visit(evidence)
    if isinstance(evidence.get('name'), str):
        spelling = evidence['name']
        for found in _QUALIFIED.finditer(code[:prefix[0]]):
            if re.sub(r'\s+', '', found[0]) == spelling:
                queries.append((spelling, at + found.start()))
                break
        else:
            return _issue('cpp-direct-type-use-span-review', at,
                'The supplied named type proof does not identify its actual written prefix token.')
    for name, use_at in queries:
        if '::' in name:
            continue
        if not _integer(use_at):
            return _issue('cpp-direct-type-use-span-review', at,
                'A named type proof needs its actual source use offset.')
        value = resolve_cpp_value_owner(inventory, name, use_at,
                                        macro_token_gates=macro_guard)
        if value is None:
            continue
        if (value.get('role') == 'unknown'
                and value.get('kind') == 'cpp-value-type-role-competition-review'
                and value.get('source_type_owners') and not value.get('source_value_owners')):
            continue  # Positive type only; no arbitrary namespace priority.
        return _issue('cpp-direct-type-value-prefix-owner-review', use_at,
            'A positive type prefix cannot override a blocked or competing source value owner.',
            type_name=name, source_value_evidence=value, source_type_evidence=evidence)
    return None


def analyze_cpp_direct_counter_segment(segment, *, offset, source_mask,
                                       contexts, statement_owner,
                                       type_environment, value_inventory,
                                       literal_spans, protected_spans=(),
                                       direct_analyzer=analyze_direct_decl,
                                       type_prefix_resolver=_type_prefix,
                                       ordinary_resolver=identify_decl,
                                       legacy_declarators=_declarators):
    """Return a gated analysis envelope, never a partial count.

    ``statement_owner`` retains kind='independent-semicolon-statement', exact
    start/end, terminator_offset=end-1, ownership_proven=True and the positive
    ``start_boundary``. Its span comes from an independent complete-statement
    ownership pass, never from this adapter or a physical-row heuristic.

    ``fallback_allowed`` is true only after matched=False AND diagnostics=[]
    from the direct helper; only then is the unchanged ordinary resolver used.
    ``counter_terms`` supplies declaration_bindings and uninitialized, with
    written assignment tokens remaining entirely the normalizer's concern.
    Every admitted direct row keeps legacy initialized=False, so a(0),b=c=0
    yields uninitialized=1 plus the caller's two written assignment units.
    """
    result = dict(declaration=None, counter_terms=None, matched=False,
                  diagnostics=[], evidence=[], fallback_allowed=False,
                  ordinary_fallback_used=False, shadow_obligations=[], proposal_only=True)
    def reject(issue):
        result['diagnostics'].append(issue)
        return result
    if (not isinstance(segment, str) or not isinstance(source_mask, str)
            or not _integer(offset) or offset < 0
            or not isinstance(type_environment, dict)
            or type_environment.get('language') != 'cpp'
            or type_environment.get('source_length') != len(source_mask)
            or not isinstance(value_inventory, dict)
            or value_inventory.get('source_mask') != source_mask
            or value_inventory.get('source_length') != len(source_mask)
            or not isinstance(protected_spans, (list, tuple))
            or not all(callable(fn) for fn in (direct_analyzer, type_prefix_resolver,
                                              ordinary_resolver, legacy_declarators))):
        return reject(_issue('cpp-direct-counter-input-owner-review', offset,
            'Exact same-source type/value/context/literal inventories and callable bounded resolvers are required.'))
    gate = _statement_gate(segment, offset, source_mask, contexts,
                           statement_owner, protected_spans)
    if gate is not None:
        return reject(gate)
    gate = _literal_gate(literal_spans, source_mask, offset)
    if gate is not None:
        return reject(gate)
    # Existing prefix probes may append diagnostic metadata. Keep even that
    # mutation private: this proposal never repairs/replaces caller owners or
    # changes its environment, including its diagnostic list.
    type_environment = copy.deepcopy(type_environment)
    macros = type_environment.get('type_macro_environment')
    if not isinstance(macros, dict) or macros.get('source_length') != len(source_mask):
        return reject(_issue('cpp-direct-counter-macro-inventory-review', offset,
            'The actual source macro event inventory is required, even when the source has no macros.'))
    macro_guard = lambda text, at, *, role: type_macro_token_gates(macros, text, at, role=role)
    gates = macro_guard(segment, offset, role='cpp-direct-complete-statement-token')
    if gates:
        result['diagnostics'].extend(gates)
        return result
    leading = len(segment) - len(segment.lstrip())
    code = segment.strip().removesuffix(';').rstrip()
    exact_at = offset + leading
    def ordinary_fallback():
        result['fallback_allowed'] = True
        result['ordinary_fallback_used'] = True
        before_ordinary = len(type_environment.get('diagnostics', ()))
        declaration = ordinary_resolver(segment, type_environment, offset=offset)
        emitted = type_environment.get('diagnostics', ())[before_ordinary:]
        if emitted:
            result['diagnostics'].extend(dict(issue) for issue in emitted)
            result['declaration'] = None
            result['counter_terms'] = None
            result['fallback_allowed'] = False
        else:
            result['declaration'] = declaration
    before = len(type_environment.get('diagnostics', ()))
    prefix = type_prefix_resolver(code, type_environment, exact_at)
    prefix_issues = type_environment.get('diagnostics', ())[before:]
    if prefix_issues:
        result['diagnostics'].extend(dict(issue) for issue in prefix_issues)
        return result
    if prefix is None:
        # No possible-type/name promotion. Existing possible-declaration review
        # remains the future caller's responsibility after ordinary fallback.
        ordinary_fallback()
    else:
        if (not isinstance(prefix, tuple) or len(prefix) != 3
                or not _integer(prefix[0]) or not 0 < prefix[0] < len(code)
                or not isinstance(prefix[1], str) or not prefix[1]
                or not isinstance(prefix[2], dict) or not prefix[2].get('kind')
                or any(prefix[2].get(flag) for flag in _REJECT_FLAGS)):
            return reject(_issue('cpp-direct-counter-type-prefix-contract-review', exact_at,
                'The supplied prefix must be a complete positive type proof for this unchanged segment.'))
        conflict = _proven_type_conflict(code, exact_at, prefix, type_environment,
                                         value_inventory, macro_guard)
        if conflict is not None:
            return reject(conflict)
        role = lambda name, at: _source_role(type_environment, value_inventory,
                                            name, at, macro_guard)
        callback_issues = []
        def type_id(text, at):
            # Prove the entire bounded base and its CV/pointer/reference suffix,
            # rather than treating _type_prefix's base end as the full type ID.
            local_leading = len(text) - len(text.lstrip())
            before_local = len(type_environment.get('diagnostics', ()))
            typed = type_prefix_resolver(text.lstrip(), type_environment, at + local_leading)
            callback_issues.extend(dict(issue) for issue in
                type_environment.get('diagnostics', ())[before_local:])
            if typed is None:
                return None
            if (not isinstance(typed, tuple) or len(typed) != 3
                    or not _integer(typed[0]) or not 0 < typed[0] <= len(text.lstrip())
                    or not isinstance(typed[2], dict) or not typed[2].get('kind')):
                callback_issues.append(_issue('cpp-direct-counter-type-id-contract-review', at,
                    'A type ID requires a positive base proof before any supported structural suffix.'))
                return None
            end, _, evidence = typed
            if any(evidence.get(flag) for flag in _REJECT_FLAGS):
                return None
            conflict = _proven_type_conflict(text.lstrip(), at + local_leading,
                typed, type_environment, value_inventory, macro_guard)
            if conflict is not None:
                callback_issues.append(conflict)
                return None
            cursor = local_leading + end
            while True:
                next_at = cursor
                while next_at < len(text) and text[next_at].isspace():
                    next_at += 1
                word = _NAME.match(text, next_at)
                if word is not None and word[0] in _CV:
                    cursor = word.end()
                elif text[next_at:next_at + 1] == '*':
                    cursor = next_at + 1
                else:
                    break
            next_at = cursor
            while next_at < len(text) and text[next_at].isspace():
                next_at += 1
            if text.startswith('&&', next_at):
                cursor = next_at + 2
            elif text[next_at:next_at + 1] == '&':
                cursor = next_at + 1
            return cursor, dict(evidence, kind='cpp-direct-counter-proven-type-id',
                                type_id=text[:cursor], start=at, end=at + cursor, units=0)
        analyzed = direct_analyzer(segment, offset=offset, type_prefix=prefix,
            context='declaration', role_resolver=role, type_id_resolver=type_id,
            legacy_declarators=legacy_declarators, literal_spans=literal_spans)
        if (not isinstance(analyzed, dict) or not isinstance(analyzed.get('matched'), bool)
                or not isinstance(analyzed.get('diagnostics'), (list, tuple))
                or any(not isinstance(issue, dict) for issue in analyzed['diagnostics'])
                or not isinstance(analyzed.get('evidence'), (list, tuple))
                or any(not isinstance(item, dict) for item in analyzed['evidence'])):
            return reject(_issue('cpp-direct-counter-helper-contract-review', exact_at,
                'The direct helper must return its complete matched/diagnostics/evidence envelope.'))
        result['matched'] = analyzed['matched']
        result['diagnostics'].extend(analyzed['diagnostics'])
        result['diagnostics'].extend(callback_issues)
        result['evidence'].extend(analyzed['evidence'])
        if analyzed['matched'] and any(span['start'] < offset + len(segment)
                and offset < span['end'] for span in literal_spans):
            result['diagnostics'].append(_issue('cpp-direct-literal-data-owner-review', exact_at,
                'First-stage direct-form admission preserves and gates every actual literal overlap.'))
        if result['diagnostics']:
            return result
        if analyzed['matched']:
            if not isinstance(analyzed.get('declaration'), dict):
                return reject(_issue('cpp-direct-counter-helper-contract-review', exact_at,
                    'A matched, ungated result must contain the complete all-or-nothing declaration.'))
            declaration = analyzed['declaration']
            if (not _integer(declaration.get('declarator_start'))
                    or not isinstance(declaration.get('declarators'), (list, tuple))
                    or any(not isinstance(row, dict) or not isinstance(row.get('name'), str)
                           or not _integer(row.get('name_offset'))
                           for row in declaration['declarators'])):
                return reject(_issue('cpp-direct-counter-helper-contract-review', exact_at,
                    'Matched names need exact complete declarator metadata before any shadow obligation is considered.'))
            for row in declaration.get('declarators', ()):
                name = row.get('name')
                if name in type_environment.get('aliases', {}):
                    absolute_name = offset + declaration['declarator_start'] + row['name_offset']
                    obligation = _issue('cpp-direct-type-shadow-collection-review', absolute_name,
                        'Counting a direct name does not reconcile its later type-shadow ownership; the source collector must be reviewed separately.',
                        name=name, type_owners=type_environment['aliases'][name],
                        existing_shadow_records=[record for record in type_environment.get('shadows', ())
                            if record['name'] == name
                            and _integer(record.get('owner_start'))
                            and _integer(record.get('owner_end'))
                            and record['owner_start'] <= absolute_name
                            and absolute_name + len(name) <= record['owner_end']],
                        same_name_shadow_records=[record for record in type_environment.get('shadows', ())
                            if record['name'] == name],
                        shadow_record_match='same-name-and-enclosing-written-owner-span',
                        shadow_record_lookup_is_complete=False)
                    result['shadow_obligations'].append(obligation)
            if result['shadow_obligations']:
                result['diagnostics'].extend(result['shadow_obligations'])
                return result
            result['declaration'] = declaration
        else:
            if analyzed.get('declaration') is not None:
                return reject(_issue('cpp-direct-counter-helper-contract-review', exact_at,
                    'An unmatched helper result cannot contain declaration units.'))
            ordinary_fallback()
    if result['diagnostics']:
        return result
    declaration = result['declaration']
    if declaration is not None:
        terms = ('declaration_bindings', 'uninitialized')
        if (not isinstance(declaration, dict) or any(not _integer(declaration.get(key))
                or declaration[key] < 0 for key in terms)
                or declaration['uninitialized'] > declaration['declaration_bindings']
                or not isinstance(declaration.get('declarators'), (list, tuple))
                or declaration.get('count') != declaration['declaration_bindings']
                or len(declaration['declarators']) != declaration['declaration_bindings']
                or not _integer(declaration.get('declarator_start'))):
            result['declaration'] = None
            result['fallback_allowed'] = False
            return reject(_issue('cpp-direct-counter-declaration-contract-review', exact_at,
                'Only complete independent binding and legacy uninitialized terms may reach the caller.'))
        for row in declaration['declarators']:
            if (not isinstance(row, dict) or not isinstance(row.get('name'), str)
                    or not _NAME.fullmatch(row['name'])
                    or not _integer(row.get('name_offset')) or row['name_offset'] < 0
                    or not isinstance(row.get('initialized'), bool)):
                result['declaration'] = None
                result['fallback_allowed'] = False
                return reject(_issue('cpp-direct-counter-declaration-contract-review', exact_at,
                    'Every independently counted name needs the complete original declarator metadata.'))
            name_at = offset + declaration['declarator_start'] + row['name_offset']
            direct = row.get('direct_role') is not None
            if (source_mask[name_at:name_at + len(row['name'])] != row['name']
                    or direct and (row['initialized'] or row.get('units') != 1
                        or row.get('argument_units') != 0
                        or row['direct_role'] not in {'object', 'prototype', 'object-or-prototype'})):
                result['declaration'] = None
                result['fallback_allowed'] = False
                return reject(_issue('cpp-direct-counter-declaration-contract-review', exact_at,
                    'Direct names must match source and retain the existing additive uninitialized interpretation.'))
        if declaration['uninitialized'] != sum(not row['initialized'] for row in declaration['declarators']):
            result['declaration'] = None
            result['fallback_allowed'] = False
            return reject(_issue('cpp-direct-counter-declaration-contract-review', exact_at,
                'The legacy uninitialized term must equal all and only independently counted names without a top-level assignment.'))
        result['counter_terms'] = {key: declaration[key] for key in terms}
    return result
