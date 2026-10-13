"""Unhooked canonical language proposals for current foreign-source review.

Execution-mode aliases share a grammar only; callers still reconcile every
exact source hash, producer pin, sample set and endpoint independently.
V is report-only and intentionally absent. New suite languages are planned,
never treated as normalized merely because their producer printed physical LOC.
"""


CURRENT_FAMILIES = {'csharp','dart','zig','odin','ada','pascal','fortran',
                    'fsharp','roc','elm','smalltalk'}
PLANNED_FAMILIES = {'nim','crystal','pony'}


def family_of(language):
    family=language.lower()
    for suffix in ('-aot','-jit','-resident'):
        family=family.removesuffix(suffix)
    return {'c#':'csharp','f#':'fsharp','delphi':'pascal'}.get(family,family)


def normalize_foreign(source,language):
    family=family_of(language)
    if family in {'csharp','dart'}:
        from foreign_braced import normalize
    elif family in {'zig','odin'}:
        from zig_odin import normalize
    elif family in {'ada','pascal','fortran'}:
        from foreign_keyword import normalize
    elif family in {'fsharp','roc','elm'}:
        from foreign_layout import normalize
    elif family=='smalltalk':
        from foreign_smalltalk import normalize
    else:
        return dict(language=family,physical_loc=None,normalized_loc=None,
            diagnostics=[dict(kind='unsupported-foreign-canonical-grammar',language=family,
                message='No qualified general grammar pathway exists; physical LOC is not a normalized fallback.')],
            review_status='needs-review',proposal_only=True)
    value=normalize(source,family)
    value['execution_mode_alias']=language
    value['grammar_family']=family
    value['proposal_only']=True
    return value
