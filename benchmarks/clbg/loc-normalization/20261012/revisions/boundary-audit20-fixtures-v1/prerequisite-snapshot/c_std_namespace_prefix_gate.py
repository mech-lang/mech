"""Unhooked whole-source gate for unsupported written std prefix owners.

The input is a checked, offset-preserving C++ comment/literal mask. This first
stage proves no per-use namespace lookup, expands no macros, and adds no LOC
units. A full-string std::vector contract cannot override this retained gate.
"""

import re


_STD = re.compile(r'\bstd\b')
_USING = re.compile(r'\busing\s+namespace\b')
_EXACT_STD_IMPORT = re.compile(r'using\s+namespace\s+std\s*;')


def _gate(kind, offset, role, **evidence):
    return dict(kind=kind, offset=offset, role=role, units=None,
                review_required=True, **evidence)


def _span(row, length, *, owner=False):
    if isinstance(row, dict):
        start = row.get('definition_start') if owner else row.get('start')
        end = row.get('definition_end') if owner else row.get('end')
    elif not owner and isinstance(row, (tuple, list)) and len(row) == 2:
        start, end = row
    else:
        return None
    if (isinstance(start, int) and not isinstance(start, bool)
            and isinstance(end, int) and not isinstance(end, bool)
            and 0 <= start < end <= length):
        return start, end
    return None


def collect_std_namespace_prefix_gates(cleaned, language, *,
                                       owned_namespace_import_spans=(),
                                       protected_literal_spans=(),
                                       protected_directive_spans=()):
    """Return exact-offset source-wide gates, never a scoped lookup proof.

    Only std followed by :: and collector-proven exact using-namespace-std
    directives are exempt. Other std tokens are potential namespace/type/
    template owners; they are not asserted to semantically hide a namespace.
    Other namespace imports remain unknown competing-prefix provenance.
    Caller-proven namespace entries must already pass header/scope/conditional
    and macro checks. Missing or invalid metadata never creates an exemption.
    """
    result = dict(blocked=False, diagnostics=[], source_length=len(cleaned),
                  proposal_only=True, per_use_scope_proof=False)
    if language != 'cpp':
        return result
    protected = []
    for role, rows in (('literal', protected_literal_spans),
                       ('directive', protected_directive_spans)):
        for row in rows:
            span = _span(row, len(cleaned))
            proven = (role != 'directive' or isinstance(row, dict)
                      and row.get('extent_proven') is True
                      and row.get('directive_ownership_proven') is True)
            if span is None or not proven:
                result['diagnostics'].append(_gate('cpp-standard-namespace-prefix-provenance-review',
                    span[0] if span is not None else 0, 'unproved-' + role + '-token-protection'))
            else:
                protected.append(span)

    def protected_token(at):
        return any(start <= at < end for start, end in protected)

    owned = []
    for row in owned_namespace_import_spans:
        span = _span(row, len(cleaned), owner=True)
        if (span is None or not _EXACT_STD_IMPORT.fullmatch(cleaned[span[0]:span[1]])
                or any(left < span[1] and span[0] < right for left, right in protected)):
            result['diagnostics'].append(_gate('cpp-standard-namespace-prefix-provenance-review',
                span[0] if span is not None else 0, 'unproved-standard-namespace-import-span'))
        else:
            owned.append(span)
    for token in _STD.finditer(cleaned):
        at = token.start()
        if protected_token(at) or re.match(r'\s*::', cleaned[token.end():]):
            continue
        if any(start <= at < end for start, end in owned):
            continue
        result['diagnostics'].append(_gate('cpp-standard-namespace-prefix-owner-review',
            at, 'potential-source-std-prefix-owner', source_text=token[0],
            token_end=token.end(), source_wide_gate=True))
    for using in _USING.finditer(cleaned):
        if protected_token(using.start()):
            continue
        if any(start == using.start() for start, _ in owned):
            continue
        end = cleaned.find(';', using.end())
        end = len(cleaned) if end < 0 else end + 1
        text = cleaned[using.start():end]
        role = ('unproved-standard-namespace-import'
                if _EXACT_STD_IMPORT.fullmatch(text) else 'unproved-other-namespace-import')
        result['diagnostics'].append(_gate('cpp-standard-namespace-prefix-owner-review',
            using.start(), role, source_text=text, namespace_import_end=end,
            source_wide_gate=True))
    result['blocked'] = bool(result['diagnostics'])
    return result
