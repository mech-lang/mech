"""Source-local owners for the measured typed GMP name adapter.

The caller supplies the original qualified Nim grammar; this module does not
import a mutable grammar or run a compiler.  It never admits a chart count.
Templates are written staged declarations with an independent forwarding body,
not the proc/func scalar-composition exemption.  No dedent closer is invented.
"""
import re

VERSION = 'nim-measured-gmp-template-owners-20261013-v1'
DIAGNOSTIC = 'nim-metaprogram-or-declaration-review'

# Exact typed source roles, including argument order and the upstream callee.
# Unknown template syntax, new actions, generic formals or includes stay gated.
FORWARDERS = (
    ('init', 'value: var Mpz', '', 'mpz_init', ('value',)),
    ('clear', 'value: var Mpz', '', 'mpz_clear', ('value',)),
    ('setUi', 'value: var Mpz, input: culong', '', 'mpz_set_ui', ('value', 'input')),
    ('add', 'destination: var Mpz, left: var Mpz, right: var Mpz', '', 'mpz_add', ('destination', 'left', 'right')),
    ('sub', 'destination: var Mpz, left: var Mpz, right: var Mpz', '', 'mpz_sub', ('destination', 'left', 'right')),
    ('mulUi', 'destination: var Mpz, left: var Mpz, right: culong', '', 'mpz_mul_ui', ('destination', 'left', 'right')),
    ('addmulUi', 'destination: var Mpz, left: var Mpz, right: culong', '', 'mpz_addmul_ui', ('destination', 'left', 'right')),
    ('divide', 'destination: var Mpz, numerator: var Mpz, denominator: var Mpz', '', 'mpz_tdiv_q', ('destination', 'numerator', 'denominator')),
    ('compare', 'left: var Mpz, right: var Mpz', ': cint', 'mpz_cmp', ('left', 'right')),
    ('getUi', 'value: var Mpz', ': culong', 'mpz_get_ui', ('value',)),
)


def spelling(value):
    """Only whitespace/wrapping varies; every nonspace token remains literal."""
    parts = re.findall(r'[A-Za-z_]\w*|[^\w\s]', value)
    result = re.escape(parts[0])
    for left, right in zip(parts, parts[1:]):
        gap = r'\s+' if left[-1].isalnum() and right[0].isalnum() else r'\s*'
        result += gap + re.escape(right)
    return result


def owners(cleaned):
    cursor = 0
    events, witnesses, resolved_offsets = [], [], []

    def statement(pattern, role, name, template=False, callee=None, arguments=()):
        nonlocal cursor
        match = re.compile(r'\s*' + pattern).match(cleaned, cursor)
        if match is None:
            raise ValueError('Unreviewed GMP adapter source role: ' + name)
        start, end = match.span('whole')
        if template:
            header = dict(kind='header', start=match.start('header'),
                end=match.end('header'), units=1, scope='nim-typed-template')
            body = dict(kind='statement', start=match.start('body'),
                end=match.end('body'), units=1, scope='nim-template-forwarding-body')
            events.extend((header, body))
            spans = [dict(role='written-template-declaration', **header),
                     dict(role='independent-forwarding-call', **body)]
        else:
            event = dict(kind='declaration' if role == 'type-alias' else 'include',
                start=start, end=end, units=1, scope='nim-authored-adapter')
            events.append(event)
            spans = [dict(role=role, **event)]
        witnesses.append(dict(name=name, role=role, start=start, end=end,
            units=sum(item['units'] for item in spans), spans=spans,
            upstream_callee=callee, forwarded_arguments=list(arguments)))
        if template or role == 'authored-source-include':
            resolved_offsets.append(start)
        cursor = end

    statement(r'(?P<whole>' + spelling('type csize = csize_t') + ')', 'type-alias', 'csize')
    statement(r'(?P<whole>' + spelling('include gmp/header') + ')', 'authored-source-include', 'gmp/header')
    statement(r'(?P<whole>' + spelling('type Mpz* = mpz_t') + ')', 'type-alias', 'Mpz')
    for name, formals, returned, callee, arguments in FORWARDERS:
        header = spelling('template ' + name + '*(' + formals + ')' + returned + ' =')
        body = spelling(callee + '(' + ', '.join(arguments) + ')')
        statement(r'(?P<whole>(?P<header>' + header + r')\s*(?P<body>' + body + '))',
            'typed-forwarding-template', name, template=True, callee=callee, arguments=arguments)
    if cleaned[cursor:].strip():
        raise ValueError('Unreviewed trailing GMP adapter source')
    return events, witnesses, resolved_offsets


def normalize(source, base):
    """Account exact reviewed owners through the unchanged shared counter."""
    masked = base.mask_nim(source)
    if masked['diagnostics'] or masked.get('directives'):
        raise ValueError('GMP adapter has an unreviewed lexical or pragma role')
    parsed = base.classify_scopes(masked['cleaned'], 'nim',
        literal_spans=masked.get('spans', ()), directives=())
    events, witnesses, resolved = owners(masked['cleaned'])
    # Resolve only diagnostics whose exact offsets name these proven include
    # and typed declarations.  Never discard another base-grammar diagnostic.
    actual = [(issue['kind'], issue['offset']) for issue in parsed['diagnostics']]
    expected = [(DIAGNOSTIC, offset) for offset in resolved]
    if actual != expected or parsed['events'] or parsed['executable_regions']:
        raise ValueError('Base grammar diagnostics/owners differ from the reviewed adapter')
    result = base.count_owned(source, masked['cleaned'], parsed['contexts'], events,
        [], 'nim', binding_counter=base.binding_counter, named_equals=True,
        assignment_pattern=base.ASSIGN)
    if result['diagnostics']:
        raise ValueError('Shared counter rejected adapter owners')
    result.update(proposal_version=VERSION, eligible_for_chart=False,
        pragma_evidence=[], source_role='counted-application-helper',
        owner_witnesses=witnesses, resolved_source_diagnostics=parsed['diagnostics'],
        physical_LOC_is_not_canonical_override=True,
        upstream_generic_library_counted=False)
    return result
