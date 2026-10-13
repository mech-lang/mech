"""Unhooked, bounded C/C++ typedef ownership proposals.

Input must already have an offset-preserving comment/literal mask. This module
does not lex source, read files, run the compiler, or certify a LOC result.
Callers exclude parameter/control/data contexts before ``identify_decl``.
Pass the exact offset for lexical scope/order resolution. Without an offset,
only unconditional translation-unit aliases with no shadows are usable.
"""

import re
from compiler_attributes import classify_gnu_attributes
from c_named_types import collect_named_type_owners,collect_named_type_shadows


_NAME = r'[A-Za-z_]\w*'
_QUALIFIED = _NAME + r'(?:::' + _NAME + r')*'
_MODIFIERS = {'static', 'const', 'volatile', 'register', 'extern', 'inline',
              'constexpr', 'mutable', 'thread_local', 'restrict', '__restrict'}
_BUILTIN = re.compile(
    r'(?:unsigned|signed)\s+__int128'
    r'|(?:unsigned|signed)(?:\s+(?:char|short(?:\s+int)?|long(?:\s+long)?(?:\s+int)?|int))?'
    r'|long\s+double|long(?:\s+long)?(?:\s+int)?|short(?:\s+int)?'
    r'|void|bool|char|int|float|double|_Bool|__int128')
_ASSIGN = re.compile(r'(?<![=!<>])=(?!=|>)')
_DIRECTIVE = re.compile(r'^[ \t]*#[ \t]*(\w+)\b([^\n]*)', re.M)
_STD_INTEGERS=(tuple(f'{sign}int{width}_t' for sign in ('','u') for width in (8,16,32,64))
    +tuple(f'{sign}int_{kind}{width}_t' for sign in ('','u')
           for kind in ('least','fast') for width in (8,16,32,64))
    +('intptr_t','uintptr_t','intmax_t','uintmax_t'))
_INCLUDES = {
    'gmp.h': ('mpz_t',),
    'gmpxx.h': ('mpz_t', 'mpz_class'),
    'string': ('std::string',),
    'iostream': ('std::string',),
    'stdint.h': _STD_INTEGERS,
    'inttypes.h': _STD_INTEGERS,
    'stddef.h': ('size_t', 'ptrdiff_t'),
    'stdio.h': ('size_t',),
    'stdlib.h': ('size_t',),
    'string.h': ('size_t',),
    'pcre2.h': ('size_t','PCRE2_SIZE'),
    # Explicit GCC/Clang intrinsic-header contracts. SSE2's conditional
    # compatibility inclusion from xmmintrin.h does not prove SSE2 ownership.
    'xmmintrin.h': ('__m128',),
    'emmintrin.h': ('__m128', '__m128d', '__m128i'),
    'immintrin.h': ('__m128', '__m128d', '__m128i', '__m256', '__m256d',
                    '__m256i', '__m512', '__m512d', '__m512i'),
}


def _balanced_end(text, start):
    pairs = {'(': ')', '[': ']', '{': '}'}
    if start >= len(text) or text[start] not in pairs:
        return None
    stack = [pairs[text[start]]]
    for index in range(start + 1, len(text)):
        char = text[index]
        if char in pairs:
            stack.append(pairs[char])
        elif char in ')]}':
            if not stack or char != stack.pop():
                return None
            if not stack:
                return index + 1
    return None


def _split_top(text, separator=','):
    parts = []
    start = 0
    index = 0
    while index < len(text):
        char = text[index]
        if char in '([{':
            end = _balanced_end(text, index)
            if end is None:
                return None
            index = end
            continue
        if char in ')]}':
            return None
        if char == separator:
            parts.append((start, text[start:index].strip()))
            start = index + 1
        index += 1
    parts.append((start, text[start:].strip()))
    return parts


def _declarators(text, *, allow_initializer=True):
    """Only named pointers/references and balanced named array suffixes."""
    parts = _split_top(text)
    if not parts:
        return None
    result = []
    for part_offset, part in parts:
        declarator_text = part
        grouped_end = None
        if part.startswith('('):
            grouped_end = _balanced_end(part, 0)
            if grouped_end is None:
                return None
            declarator_text = part[1:grouped_end - 1].strip()
            if not declarator_text.startswith(('*', '&')):
                return None
        found = re.match(r'(?:(?:\*|&&|&)\s*(?:(?:const|volatile|restrict)\s+)*)*'
                         r'(' + _NAME + r')\b', declarator_text)
        if not found:
            return None
        if grouped_end is not None and declarator_text[found.end():].strip():
            return None
        cursor = grouped_end if grouped_end is not None else found.end()
        while True:
            while cursor < len(part) and part[cursor].isspace():
                cursor += 1
            if cursor >= len(part) or part[cursor] != '[':
                break
            end = _balanced_end(part, cursor)
            if end is None or ';' in part[cursor:end]:
                return None
            cursor = end
        tail = part[cursor:].strip()
        initialized = bool(_ASSIGN.match(tail))
        if tail and (not allow_initializer or not initialized or not tail[1:].strip()):
            return None
        # Relational/template commas cannot silently create extra declarators.
        if initialized and ('<' in tail or '>' in tail):
            return None
        local_name=(found.start(1) if grouped_end is None else
                    1+len(part[1:grouped_end-1])-len(part[1:grouped_end-1].lstrip())+found.start(1))
        leading=len(text[part_offset:])-len(text[part_offset:].lstrip())
        result.append(dict(name=found[1], initialized=initialized,
                           declarator=part, part_offset=part_offset,
                           name_offset=part_offset+leading+local_name))
    return result


def _scope_ranges(cleaned):
    ranges = [(0, len(cleaned), None)]
    stack = []
    diagnostics = []
    for index, char in enumerate(_without_directives(cleaned)):
        if char == '{':
            stack.append(index)
        elif char == '}':
            if stack:
                start = stack.pop()
                ranges.append((start, index + 1, start))
            else:
                diagnostics.append(dict(kind='c-type-environment-unbalanced-scope-review',
                                        offset=index))
    for start in stack:
        diagnostics.append(dict(kind='c-type-environment-unclosed-scope-review',
                                offset=start))
    return ranges, diagnostics


def _scope_at(ranges, offset):
    return min((scope for scope in ranges if scope[0] <= offset < scope[1]),
               key=lambda scope: scope[1] - scope[0])


def _conditional_regions(cleaned):
    regions = []
    stack = []
    for directive in _DIRECTIVE.finditer(cleaned):
        word = directive[1]
        if word in {'if', 'ifdef', 'ifndef'}:
            stack.append(directive.start())
        elif word == 'endif' and stack:
            regions.append((stack.pop(), directive.end()))
    regions.extend((start, len(cleaned)) for start in stack)
    return regions


def _conditional_at(regions, offset):
    return any(start <= offset < end for start, end in regions)


def _inside_directive(cleaned, offset):
    start = cleaned.rfind('\n', 0, offset) + 1
    if cleaned[start:offset].lstrip().startswith('#'):
        return True
    while start and cleaned[:start].rstrip('\n\r').endswith('\\'):
        start = cleaned.rfind('\n', 0, max(0, start - 1)) + 1
        if cleaned[start:offset].lstrip().startswith('#'):
            return True
    return False


def _without_directives(cleaned):
    """Directive replacement keeps positions; macro braces never own scopes."""
    chars = list(cleaned)
    continuation = False
    start = 0
    for line in cleaned.splitlines(keepends=True):
        directive = continuation or bool(re.match(r'^[ \t]*#', line))
        if directive:
            for index in range(start, start + len(line)):
                if not chars[index].isspace():
                    chars[index] = ' '
        continuation = directive and line.rstrip('\r\n').endswith('\\')
        start += len(line)
    return ''.join(chars)


def _resolve(environment, name, offset=None):
    definitions = environment.get('aliases', {}).get(name, [])
    if offset is None:
        eligible = [entry for entry in definitions if entry['scope_opening'] is None
                    and not entry.get('conditional')]
        if (len(eligible) != 1 or any(entry.get('conditional') for entry in definitions)
                or any(s['name'] == name for s in environment.get('shadows', []))):
            return None
        return eligible[0]
    eligible = [entry for entry in definitions
                if entry['visible_start'] <= offset < entry['scope_end']
                and not entry.get('conditional')]
    if not eligible:
        return None
    chosen = max(eligible, key=lambda entry: (entry['scope_start'], entry['visible_start']))
    if any(entry.get('conditional') and entry['visible_start'] <= offset < entry['scope_end']
           and entry['scope_start'] >= chosen['scope_start'] for entry in definitions):
        return None
    if any(shadow['name'] == name and shadow['start'] <= offset < shadow['end']
           and shadow['scope_start'] >= chosen['scope_start']
           for shadow in environment.get('shadows', [])):
        return None
    return chosen


def _type_prefix(code, environment, offset):
    cursor = 0
    while True:
        found = re.match(r'\s*(' + _NAME + r')\b', code[cursor:])
        if not found or found[1] not in _MODIFIERS:
            break
        cursor += found.end()
    while cursor < len(code) and code[cursor].isspace():
        cursor += 1
    builtin = _BUILTIN.match(code, cursor)
    if builtin and (builtin.end() == len(code) or not (code[builtin.end()].isalnum() or code[builtin.end()] == '_')):
        end = builtin.end()
        type_name = builtin[0]
        evidence = dict(kind='builtin-type', type_name=type_name)
    else:
        tag = re.match(r'(struct|union|enum)\s+(' + _NAME + r')\b', code[cursor:])
        if tag:
            end = cursor + tag.end()
            type_name = tag[0]
            evidence = dict(kind='explicit-tag-type', type_name=type_name)
        else:
            named = re.match(_QUALIFIED, code[cursor:])
            if not named:
                return None
            definition = _resolve(environment, named[0], offset)
            if definition is None:
                return None
            end = cursor + named.end()
            type_name = named[0]
            evidence = definition
    # A separated trailing cv qualifier is type syntax, not a new binding.
    trailing = re.match(r'(?:\s+(?:const|volatile)\b)*', code[end:])
    end += trailing.end()
    if end < len(code) and not (code[end].isspace() or code[end] in '*&'):
        return None
    return end, type_name, evidence


def identify_decl(segment, type_environment, *, offset=None):
    """Return proven independent binding count, or None for unknown grammar.

    Context exclusion is a caller responsibility. ``offset`` is the absolute
    beginning of this segment in the same masked source used for collection.
    Initializer '=' signs are not counted here: return ``uninitialized`` so the
    caller can add its own structural assignment count for chained bindings.
    """
    leading = len(segment) - len(segment.lstrip())
    code = segment.strip().removesuffix(';').rstrip()
    if not code or code.startswith(('typedef ', 'using ', '#')):
        return None
    at = None if offset is None else offset + leading
    typed = _type_prefix(code, type_environment, at)
    if typed is None:
        return None
    end, type_name, evidence = typed
    raw_declarators=code[end:]
    names = _declarators(raw_declarators.strip())
    if not names:
        return None
    return dict(count=len(names), declaration_bindings=len(names),
                uninitialized=sum(not name['initialized'] for name in names),
                reason='independent-proven-type-declaration-bindings',
                declarator_start=leading+end+len(raw_declarators)-len(raw_declarators.lstrip()),
                type_name=type_name, declarators=names, type_evidence=evidence)


def _template_end(text, opening):
    """Bounded potential template spelling, never proof of type ownership."""
    depth = 1
    cursor = opening + 1
    while cursor < len(text):
        char = text[cursor]
        if char in '([{':
            end = _balanced_end(text, cursor)
            if end is None:
                return None
            cursor = end
            continue
        if char == '<':
            depth += 1
        elif char == '>':
            depth -= 1
            if not depth:
                return cursor + 1
        elif char in ';{}':
            return None
        cursor += 1
    return None


def _potential_parts(text):
    parts = []
    start = cursor = 0
    while cursor < len(text):
        char = text[cursor]
        if char in '([{':
            end = _balanced_end(text, cursor)
            if end is None:
                return None
            cursor = end
            continue
        if char == '<':
            end = _template_end(text, cursor)
            if end is not None:
                cursor = end
                continue
        if char in ')]}':
            return None
        if char == ',':
            parts.append(text[start:cursor].strip())
            start = cursor + 1
        cursor += 1
    parts.append(text[start:].strip())
    return parts


def possible_packed_decl(segment):
    """Review gate for plausible declarations whose type is not yet proven.

    Supports qualified/template-looking prefixes and named arrays, pointers,
    initializer operands, or simple constructor suffixes. This function never
    contributes units: ``X*a,b`` remains an unproven composition until X's
    lexical role is resolved. Caller context exclusions still apply.
    """
    code = segment.strip().removesuffix(';').rstrip()
    cursor = 0
    while True:
        modifier = re.match(r'\s*(' + _NAME + r')\b', code[cursor:])
        if not modifier or modifier[1] not in _MODIFIERS | {'typename'}:
            break
        cursor += modifier.end()
    while cursor < len(code) and code[cursor].isspace():
        cursor += 1
    prefix_start = cursor
    builtin = _BUILTIN.match(code, cursor)
    if builtin and (builtin.end() == len(code) or not (code[builtin.end()].isalnum() or code[builtin.end()] == '_')):
        cursor = builtin.end()
    else:
        named = re.match(_QUALIFIED, code[cursor:])
        if not named or named[0] in {'return', 'co_return', 'throw', 'delete', 'new',
                'if', 'else', 'for', 'while', 'do', 'switch', 'case', 'default',
                'goto', 'break', 'continue', 'sizeof', 'alignof', 'typedef', 'using',
                'struct', 'union', 'enum', 'class', 'namespace', 'asm'}:
            return None
        cursor += named.end()
        if cursor < len(code) and code[cursor] == '<':
            end = _template_end(code, cursor)
            if end is None:
                return None
            cursor = end
        while code.startswith('::', cursor):
            member = re.match(_NAME, code[cursor + 2:])
            if not member:
                return None
            cursor += 2 + member.end()
            if cursor < len(code) and code[cursor] == '<':
                end = _template_end(code, cursor)
                if end is None:
                    return None
                cursor = end
    trailing = re.match(r'(?:\s+(?:const|volatile)\b)*', code[cursor:])
    cursor += trailing.end()
    type_end = cursor
    while cursor < len(code) and code[cursor].isspace():
        cursor += 1
    if cursor == type_end and (cursor >= len(code) or code[cursor] not in '*&'):
        return None
    parts = _potential_parts(code[cursor:])
    if not parts or len(parts) < 2:
        return None
    names = []
    for part in parts:
        names_only = _declarators(part)
        if names_only:
            names.append(names_only[0]['name'])
            continue
        # Constructor spellings are a potential declaration owner, not proof.
        found = re.match(r'[*&\s]*(' + _NAME + r')\b', part)
        if not found:
            return None
        at = found.end()
        while at < len(part) and part[at].isspace():
            at += 1
        while at < len(part) and part[at] == '[':
            end = _balanced_end(part, at)
            if end is None:
                return None
            at = end
            while at < len(part) and part[at].isspace():
                at += 1
        if at < len(part) and part[at] in '({':
            end = _balanced_end(part, at)
            if end is None or part[end:].strip():
                return None
        elif _ASSIGN.match(part[at:]) and part[at + 1:].strip():
            pass
        else:
            return None
        names.append(found[1])
    return dict(kind='possible-packed-declaration-type-owner-review',
                type_spelling=code[prefix_start:type_end], possible_names=names,
                possible_declarator_count=len(names), units=None,
                review_required=True,
                reason='possible-declaration-grammar-does-not-prove-lexical-type-ownership')


def _declaration_end(cleaned, start):
    cursor = start
    while cursor < len(cleaned):
        if cleaned[cursor] in '([{':
            end = _balanced_end(cleaned, cursor)
            if end is None:
                return None
            cursor = end
        elif cleaned[cursor] == ';':
            return cursor + 1
        elif cleaned[cursor] == '}':
            return None
        else:
            cursor += 1
    return None


def collect_typedef_aliases(cleaned, language, *, semantic_imports=()):
    """Collect only proven alias owners; unresolved forms keep diagnostics.

    Imported types have small explicit header contracts. Filename spans remain
    literal masks; a lexer supplies semantic_imports records {start,end,header}
    for exact filename token offsets and a delimiter-free header name. No
    imported alias is guessed from a type spelling. Full qualification still
    requires source/header provenance outside this proposal helper.
    """
    annotations=classify_gnu_attributes(cleaned,language)
    cleaned=annotations['cleaned']
    environment = dict(language=language, aliases={}, evidence=[], diagnostics=[],
                       shadows=[], source_length=len(cleaned), alias_spans=[],
                       aggregate_alias_suffixes=[])
    environment['compiler_attribute_evidence']=annotations['evidence']
    environment['diagnostics'].extend(annotations['diagnostics'])
    if language not in {'c', 'cpp'}:
        environment['diagnostics'].append(dict(kind='c-type-environment-language-review',
                                              language=language))
        return environment
    ranges, diagnostics = _scope_ranges(cleaned)
    environment['diagnostics'].extend(diagnostics)
    conditional = _conditional_regions(cleaned)
    named=collect_named_type_owners(cleaned,language,ranges,conditional)
    environment['diagnostics'].extend(named['diagnostics'])
    environment['named_type_headers']=named['header_spans']
    for entry in named['entries']:
        environment['aliases'].setdefault(entry['name'],[]).append(entry)
    environment['evidence'].extend(named['evidence'])

    def add(name, start, end, kind, type_name, *, scope=None):
        owner = scope or _scope_at(ranges, start)
        entry = dict(name=name, kind=kind, type_name=type_name,
                     definition_start=start, definition_end=end, visible_start=end,
                     scope_start=owner[0], scope_end=owner[1], scope_opening=owner[2],
                     conditional=_conditional_at(conditional, start),
                     source_text=cleaned[start:end])
        environment['aliases'].setdefault(name, []).append(entry)
        environment['evidence'].append(entry)
        if entry['conditional']:
            environment['diagnostics'].append(dict(kind='c-type-conditional-owner-review',
                                                  offset=start, name=name))

    for semantic in semantic_imports:
        header = semantic.get('header')
        if header not in _INCLUDES:
            continue
        start, end = semantic.get('start'), semantic.get('end')
        if (not isinstance(start, int) or not isinstance(end, int)
                or not 0 <= start < end <= len(cleaned)):
            environment['diagnostics'].append(dict(kind='c-type-import-span-owner-review',
                                                  offset=start, header=header))
            continue
        line_start = cleaned.rfind('\n', 0, start) + 1
        line_end = cleaned.find('\n', end)
        line_end = len(cleaned) if line_end < 0 else line_end
        prefix = cleaned[line_start:start]
        if (not re.fullmatch(r'[ \t]*#[ \t]*include[ \t]*', prefix)
                or any(char not in 'S \t' for char in cleaned[start:end])
                or cleaned[end:line_end].strip()):
            environment['diagnostics'].append(dict(kind='c-type-import-span-owner-review',
                                                  offset=start, header=header))
            continue
        if _scope_at(ranges, line_start)[2] is not None:
            environment['diagnostics'].append(dict(kind='c-type-import-scope-owner-review',
                                                  offset=start, header=header))
            continue
        for name in _INCLUDES[header]:
            if language == 'c' and ('::' in name or header in {'gmpxx.h', 'string', 'iostream'}):
                continue
            add(name, line_start, end, 'explicit-header-type-contract',
                header, scope=ranges[0])

    owners = [(match.start(), match, 'typedef') for match in re.finditer(r'\btypedef\b', cleaned)]
    if language == 'cpp':
        owners.extend((match.start(), match, 'using')
                      for match in re.finditer(r'\busing\s+' + _NAME + r'\s*=', cleaned))
        owners.extend((match.start(), match, 'standard-namespace')
                      for match in re.finditer(r'\busing\s+namespace\s+std\s*;', cleaned))
    for start, match, kind in sorted(owners, key=lambda owner: owner[0]):
        if _inside_directive(cleaned, start):
            continue
        if kind == 'standard-namespace':
            qualified = _resolve(environment, 'std::string', start)
            if qualified is not None:
                add('string', start, match.end(), 'explicit-standard-namespace-import', 'std::string')
            continue
        end = _declaration_end(cleaned, match.end())
        if end is None:
            environment['diagnostics'].append(dict(kind='c-typedef-unclosed-owner-review', offset=start))
            continue
        raw_body = cleaned[match.end():end - 1]
        body_start = match.end() + len(raw_body) - len(raw_body.lstrip())
        body = raw_body.strip()
        alias_names = None
        type_name = None
        if kind == 'using':
            name = re.match(r'using\s+(' + _NAME + r')\s*=', cleaned[start:match.end()])[1]
            # Parse RHS as a type followed by a synthetic bounded declarator.
            typed = _type_prefix(body + ' __alias_name', environment, start)
            if typed and _declarators((body + ' __alias_name')[typed[0]:].strip(), allow_initializer=False):
                alias_names, type_name = [name], body
        else:
            aggregate = re.match(r'(struct|union|enum)\b(?:\s+' + _NAME + r'\b)?\s*\{', body)
            if aggregate:
                opening = body.find('{', 0, aggregate.end())
                close = _balanced_end(body, opening)
                tail = _declarators(body[close:].strip(), allow_initializer=False) if close else None
                if tail:
                    alias_names = [name['name'] for name in tail]
                    type_name = body[:opening].strip() or aggregate[1]
                    suffix_start = body_start + close
                    environment['aggregate_alias_suffixes'].append(dict(
                        kind='typedef-aggregate-alias-suffix', start=suffix_start,
                        end=end, closing_brace=suffix_start - 1,
                        owner_start=start, owner_end=end, primary_alias=alias_names[0],
                        additional_aliases=alias_names[1:], alias_count=len(alias_names),
                        units=len(alias_names) - 1,
                        reason='first-alias-owned-by-type-header-additional-aliases-independent',
                        source_text=cleaned[suffix_start:end],
                        conditional=_conditional_at(conditional, start)))
            else:
                typed = _type_prefix(body, environment, start)
                tail = _declarators(body[typed[0]:].strip(), allow_initializer=False) if typed else None
                if tail:
                    alias_names = [name['name'] for name in tail]
                    type_name = typed[1]
        if not alias_names:
            environment['diagnostics'].append(dict(kind='c-typedef-declarator-owner-review',
                                                  offset=start, source_text=cleaned[start:end]))
            continue
        environment['alias_spans'].append((start, end))
        for name in alias_names:
            add(name, start, end, 'source-local-' + kind, type_name)

    _collect_shadows(cleaned, environment, ranges)
    named_shadows=collect_named_type_shadows(cleaned,environment,ranges,_type_prefix,
        conditional_regions=conditional)
    environment['shadows'].extend(named_shadows['shadows'])
    environment['shadows'].extend(named_shadows['blocked_owners'])
    environment['diagnostics'].extend(named_shadows['diagnostics'])
    environment['named_type_shadow_evidence']=named_shadows['evidence']
    return environment


def classify_alias_suffixes(cleaned, language, type_environment=None):
    """Propose only exact typedef-owned aggregate suffix units.

    The first alias belongs to the type header; extra aliases add one each.
    The aggregate's executable written brace closer is a separate unit owned
    by canonical scope counting. Non-typedef instance tails are never hidden.
    """
    environment = type_environment if type_environment is not None else collect_typedef_aliases(cleaned, language)
    spans = []
    diagnostics = []
    for suffix in environment.get('aggregate_alias_suffixes', []):
        if suffix['conditional']:
            diagnostics.append(dict(kind='c-typedef-suffix-conditional-owner-review',
                                    offset=suffix['start']))
        else:
            spans.append(dict(suffix))
    owners = environment.get('alias_spans', [])
    for aggregate in re.finditer(r'\b(?:struct|union|enum)\b(?:\s+' + _NAME + r'\b)?\s*\{',
                                 _without_directives(cleaned)):
        if any(start <= aggregate.start() < end for start, end in owners):
            continue
        opening = cleaned.find('{', aggregate.start(), aggregate.end())
        close = _balanced_end(cleaned, opening)
        if close is None:
            continue
        end = _declaration_end(cleaned, close)
        if end is None:
            continue
        tail = cleaned[close:end - 1].strip()
        names = _declarators(tail)
        if names:
            diagnostics.append(dict(kind='c-aggregate-instance-suffix-review',
                                    offset=close, end=end, instance_names=[name['name'] for name in names],
                                    required_independent_instance_units=len(names),
                                    reason='ordinary-instance-bindings-are-not-typedef-aliases'))
        elif tail:
            diagnostics.append(dict(kind='c-aggregate-suffix-owner-review', offset=close, end=end))
    return dict(spans=spans, diagnostics=diagnostics, proposal_only=True)


def _collect_shadows(cleaned, environment, ranges):
    """Proven ordinary declarations shadow aliases; complex owners stay gated."""
    aliases = set(environment['aliases'])
    if not aliases:
        return
    scan = _without_directives(cleaned)

    def inside_alias(start, end):
        for left,right in environment['alias_spans']:
            if not (left<=start<right or left<end<=right):continue
            aggregate=next((row for row in environment.get('aggregate_alias_suffixes',[])
                if row['owner_start']==left and row['owner_end']==right),None)
            if aggregate:
                opening=cleaned.find('{',left,aggregate['closing_brace'])
                if opening<start<aggregate['closing_brace']:
                    continue # Field/value declarations inside the body stay visible.
            return True
        return False

    def record(declaration, start, end, scope, kind):
        for entry in declaration['declarators']:
            name = entry['name']
            if name in aliases:
                point=start+declaration.get('declarator_start',0)+entry.get('name_offset',0)+len(name)
                environment['shadows'].append(dict(name=name, start=point, end=scope[1],
                    scope_start=scope[0], scope_opening=scope[2], owner_start=start,
                    owner_end=end, kind=kind))

    # Ordinary semicolon statements. Parentheses are retained, so function
    # prototypes and control headers do not accidentally become field bindings.
    start = 0
    paren_depth = bracket_depth = 0
    for index, char in enumerate(scan):
        if char == '(':
            paren_depth += 1
        elif char == ')':
            paren_depth = max(0, paren_depth - 1)
        elif char == '[':
            bracket_depth += 1
        elif char == ']':
            bracket_depth = max(0, bracket_depth - 1)
        elif not paren_depth and not bracket_depth and char in '{};':
            if char in ';{' and not inside_alias(start, index + 1):
                segment = scan[start:index]
                segment = re.sub(r'^\s*(?:public|private|protected)\s*:', '', segment)
                # At a data initializer opener, prove only the written lhs;
                # a synthetic scalar completes the already present '='.
                probe = segment + '0' if char == '{' and segment.rstrip().endswith('=') else segment
                declaration = identify_decl(probe, environment, offset=start)
                if declaration:
                    scope_offset = max(start, index - 1) if char == '{' else index
                    record(declaration, start, index + 1, _scope_at(ranges, scope_offset),
                           'ordinary-declaration-shadow')
            start = index + 1

    # Bounded named function definitions: typed parameters belong to the body
    # scope. Calls, tuple values, and prototypes never establish such a scope.
    for opening in (match.start() for match in re.finditer(r'\(', scan)):
        close = _balanced_end(scan, opening)
        if close is None:
            continue
        cursor = close
        while cursor < len(scan) and scan[cursor].isspace():
            cursor += 1
        if cursor >= len(scan) or scan[cursor] != '{':
            continue
        prefix_start = max(scan.rfind(';', 0, opening), scan.rfind('{', 0, opening),
                           scan.rfind('}', 0, opening)) + 1
        prefix = scan[prefix_start:opening].strip()
        typed = _type_prefix(prefix, environment, prefix_start)
        if typed is None or not re.fullmatch(r'[*&\s]*' + _NAME, prefix[typed[0]:]):
            continue
        scope = _scope_at(ranges, cursor)
        params = _split_top(scan[opening + 1:close - 1])
        if params is None:
            environment['diagnostics'].append(dict(kind='c-type-parameter-shadow-owner-review', offset=opening))
            continue
        for relative, param in params:
            at = opening + 1 + relative
            at += len(scan[at:])-len(scan[at:].lstrip())
            declaration = identify_decl(param, environment, offset=at)
            if declaration:
                record(declaration, at, close, scope, 'function-parameter-shadow')
            elif any(re.search(r'\b' + re.escape(name) + r'\b', param) for name in aliases):
                environment['diagnostics'].append(dict(kind='c-type-parameter-shadow-owner-review',
                                                      offset=at, source_text=param))

    for loop in re.finditer(r'\bfor\s*\(', scan):
        opening = scan.find('(', loop.start(), loop.end())
        close = _balanced_end(scan, opening)
        if close is None:
            continue
        parts = _split_top(scan[opening + 1:close - 1], ';')
        if not parts or len(parts) != 3:
            continue
        at=opening+1
        at+=len(scan[at:])-len(scan[at:].lstrip())
        declaration = identify_decl(parts[0][1], environment, offset=at)
        if not declaration:
            continue
        cursor = close
        while cursor < len(scan) and scan[cursor].isspace():
            cursor += 1
        # Braced loops have an exact lifetime. Unbraced loop ownership stays
        # conservatively blocked through the enclosing scope for this draft.
        scope = _scope_at(ranges, cursor if cursor < len(scan) and scan[cursor] == '{' else loop.start())
        record(declaration, at, close, scope, 'for-initializer-shadow')
