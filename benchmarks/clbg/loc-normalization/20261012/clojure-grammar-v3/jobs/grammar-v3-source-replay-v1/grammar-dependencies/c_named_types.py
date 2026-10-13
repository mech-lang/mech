"""Unhooked, bounded source-local C++ named aggregate owner proposals.

The caller supplies offset-preserving masked text, lexical brace ranges, and
preprocessor condition ranges. No files, lexers, compiler, normalizer, or timing
are invoked here. Entries describe type ownership only; no body span is hidden
and no source unit is contributed by this module.
"""

import re


_NAME = r'[A-Za-z_]\w*'
_HEADER = re.compile(r'\b(struct|class|union)\s+(' + _NAME + r')\b')


def _scope_at(ranges, offset):
    candidates = [entry for entry in ranges if entry[0] <= offset < entry[1]]
    return min(candidates, key=lambda entry: entry[1] - entry[0]) if candidates else None


def _conditional(regions, offset):
    return any(start <= offset < end for start, end in regions)


def _without_directives(cleaned):
    chars = list(cleaned)
    start = 0
    continuation = False
    for line in cleaned.splitlines(keepends=True):
        directive = continuation or bool(re.match(r'^[ \t]*#', line))
        if directive:
            for offset in range(start, start + len(line)):
                if not chars[offset].isspace():
                    chars[offset] = ' '
        continuation = directive and line.rstrip('\r\n').endswith('\\')
        start += len(line)
    return ''.join(chars)


def _balanced_end(text, opening):
    pairs = {'(': ')', '[': ']', '{': '}'}
    if opening >= len(text) or text[opening] not in pairs:
        return None
    stack = [pairs[text[opening]]]
    for offset in range(opening + 1, len(text)):
        char = text[offset]
        if char in pairs:
            stack.append(pairs[char])
        elif char in ')]}':
            if not stack or stack.pop() != char:
                return None
            if not stack:
                return offset + 1
    return None


def _statement_start(text, offset):
    return max(text.rfind(';', 0, offset), text.rfind('{', 0, offset),
               text.rfind('}', 0, offset)) + 1


def _without_owned_template_formals(text):
    """Hide only balanced written template parameter groups for owner tests.

    A `class T` formal is not an aggregate body declaration. This helper
    contributes no source events/types and leaves unbalanced groups visible.
    """
    chars=list(text)
    for match in re.finditer(r'\btemplate\s*<',text):
        opening=match.end()-1;depth=1;cursor=opening+1
        while cursor<len(text) and depth:
            if text[cursor] in '([{':
                end=_balanced_end(text,cursor)
                if end is None:break
                cursor=end;continue
            if text[cursor]=='<':depth+=1
            elif text[cursor]=='>':depth-=1
            cursor+=1
        if depth:continue
        for index in range(opening,cursor):
            if not chars[index].isspace():chars[index]=' '
    return ''.join(chars)


def collect_named_type_owners(cleaned, language, ranges, conditional_regions):
    """Collect plain C++ struct/class/union headers or forward declarations.

    Enum-class, template, dependent/qualified, inherited, and attributed
    headers are excluded in this bounded phase. The first name becomes a type
    at its declaration point through the enclosing lexical scope. Header spans
    are separate from body spans; fields and written closers stay visible.
    """
    result = dict(entries=[], diagnostics=[], evidence=[], header_spans=[])
    if language != 'cpp':
        return result
    scan = _without_directives(cleaned)
    for header in _HEADER.finditer(scan):
        start = header.start()
        prefix_start = _statement_start(scan, start)
        prefix = scan[prefix_start:start]
        if re.search(r'\benum\s*$', prefix):
            # Scoped enum ownership is a distinct grammar, not a class alias.
            continue
        cursor = header.end()
        while cursor < len(scan) and scan[cursor].isspace():
            cursor += 1
        if re.search(r'\btemplate\s*<', prefix):
            if cursor < len(scan) and scan[cursor] in '{:<':
                result['diagnostics'].append(dict(kind='cpp-named-type-template-owner-review',
                    offset=start, name=header[2]))
            continue
        if cursor >= len(scan):
            continue
        marker = scan[cursor]
        if marker not in '{;':
            if marker in ':<' or re.match(r'(?:final|alignas|__attribute__)\b', scan[cursor:]):
                result['diagnostics'].append(dict(kind='cpp-named-type-complex-header-owner-review',
                    offset=start, name=header[2]))
            # `struct X *p` is an elaborated declaration, not this owner form.
            continue
        owner = _scope_at(ranges, start)
        if owner is None:
            result['diagnostics'].append(dict(kind='cpp-named-type-scope-owner-review',
                                             offset=start, name=header[2]))
            continue
        body_end = _balanced_end(scan, cursor) if marker == '{' else None
        if marker == '{' and body_end is None:
            result['diagnostics'].append(dict(kind='cpp-named-type-unclosed-body-owner-review',
                                             offset=start, name=header[2]))
            continue
        entry = dict(name=header[2], type_name=header[2],
            kind='source-local-cpp-named-type', aggregate_kind=header[1],
            definition_start=start, definition_end=cursor + 1,
            visible_start=header.end(2), scope_start=owner[0], scope_end=owner[1],
            scope_opening=owner[2], conditional=_conditional(conditional_regions, start),
            source_text=cleaned[start:cursor + 1],
            header_start=start, header_end=cursor + 1,
            body_opening=cursor if marker == '{' else None, body_end=body_end,
            forward_declaration=marker == ';')
        result['entries'].append(entry)
        result['evidence'].append(dict(entry))
        result['header_spans'].append(dict(start=start, end=cursor + 1, name=header[2]))
        if entry['conditional']:
            result['diagnostics'].append(dict(kind='cpp-named-type-conditional-owner-review',
                                             offset=start, name=header[2]))
    return result


def _split_params(text):
    parts = []
    start = cursor = 0
    while cursor < len(text):
        if text[cursor] in '([{':
            end = _balanced_end(text, cursor)
            if end is None:
                return None
            cursor = end
            continue
        if text[cursor] == ',':
            parts.append((start, text[start:cursor]))
            start = cursor + 1
        cursor += 1
    parts.append((start, text[start:]))
    return parts


def collect_named_type_shadows(cleaned, environment, ranges, type_prefix, *, conditional_regions=()):
    """Find bounded value owners hiding new class names; unknowns stay blocked.

    ``type_prefix`` is the parent's proven type-prefix callback, with signature
    (code, environment, absolute_offset) -> (end,type,evidence) or None. The
    parent still performs its ordinary object/field shadow pass. This companion
    covers typed ordinary call-declarator names and formal parameter values;
    constructors/destructors never become ordinary same-name functions.
    """
    result = dict(shadows=[], blocked_owners=[], diagnostics=[], evidence=[])
    if environment.get('language') != 'cpp':
        return result
    type_entries = [entry for definitions in environment.get('aliases', {}).values()
                    for entry in definitions if entry.get('kind') == 'source-local-cpp-named-type']
    names = {entry['name'] for entry in type_entries}
    if not names:
        return result
    scan = _without_directives(cleaned)

    def add(name, start, owner_end, scope, kind, *, blocked=False):
        if scope is None:
            return
        if _conditional(conditional_regions, start):
            kind = 'cpp-named-type-conditional-shadow-owner-review'
            blocked = True
        entry = dict(name=name, start=start, end=scope[1], scope_start=scope[0],
                     scope_opening=scope[2], owner_start=start, owner_end=owner_end,
                     kind=kind, proven_value_owner=not blocked)
        result['blocked_owners' if blocked else 'shadows'].append(entry)
        result['evidence'].append(dict(entry))
        if blocked:
            result['diagnostics'].append(dict(kind=kind, offset=start, name=name))

    # `auto` explicitly declares values in C++, not a class-name type. The
    # parent's ordinary typed-object pass supplies the other storage forms.
    for value in re.finditer(r'\bauto\s*[*&]*\s*(' + _NAME + r')\b\s*(?=[=({;,])', scan):
        if value[1] not in names:
            continue
        enclosing = []
        for opening in (m.start() for m in re.finditer(r'\(', scan[:value.start()])):
            end = _balanced_end(scan, opening)
            if end is not None and opening < value.start() < end:
                enclosing.append((opening, end))
        if not enclosing:
            add(value[1], value.start(1), value.end(), _scope_at(ranges, value.start(1)),
                'cpp-named-type-auto-value-shadow')
        else:
            opening, close = min(enclosing, key=lambda owner: owner[1] - owner[0])
            if re.search(r'\bfor\s*$', scan[:opening]):
                cursor = close
                while cursor < len(scan) and scan[cursor].isspace():
                    cursor += 1
                braced = cursor < len(scan) and scan[cursor] == '{'
                add(value[1], value.start(1), close,
                    _scope_at(ranges, cursor if braced else value.start(1)),
                    'cpp-named-type-for-auto-value-shadow' if braced else
                    'cpp-named-type-loop-shadow-owner-review', blocked=not braced)

    # Unscoped enum enumerators are values in the enclosing scope. Scoped enum
    # members do not hide a bare class name outside their enum owner.
    for enumeration in re.finditer(r'\benum(?!\s+(?:class|struct)\b)\s*(?:' + _NAME
            + r'\b\s*)?(?::\s*' + _NAME + r'[\w:\s]*)?\{', scan):
        opening = scan.find('{', enumeration.start(), enumeration.end())
        close = _balanced_end(scan, opening)
        parts = _split_params(scan[opening + 1:close - 1]) if close else None
        if parts is None:
            continue
        for relative, part in parts:
            enumerator = re.match(r'\s*(' + _NAME + r')\b\s*(?:=.*)?$', part, re.S)
            if enumerator and enumerator[1] in names:
                at = opening + 1 + relative + enumerator.start(1)
                add(enumerator[1], at, close, _scope_at(ranges, enumeration.start()),
                    'cpp-named-type-enumerator-value-shadow')

    for opening in (match.start() for match in re.finditer(r'\(', scan)):
        close = _balanced_end(scan, opening)
        if close is None:
            continue
        suffix = re.match(r'\s*(?:(?:const|volatile|noexcept|override|final)\b\s*)*([;{])', scan[close:])
        marker_offset = close + suffix.end(1) - 1 if suffix else None
        owner_start = _statement_start(scan, opening)
        raw_prefix = scan[owner_start:opening]
        prefix = raw_prefix.strip()
        prefix_offset = owner_start + len(raw_prefix) - len(raw_prefix.lstrip())
        scope=_scope_at(ranges,opening)
        if scope and scope[2] is not None and any(
                entry['body_opening']==scope[2] for entry in type_entries):
            access=re.match(r'(?:public|private|protected)\s*:\s*',prefix)
            if access:
                # Direct class access metadata precedes the real declaration
                # prefix; it is not an unknown return type or value name.
                prefix_offset+=access.end()
                prefix=prefix[access.end():]
        typed = type_prefix(prefix, environment, prefix_offset)
        function = re.fullmatch(r'[*&\s]*(' + _NAME + r')', prefix[typed[0]:]) if typed else None
        typed_operator = bool(typed and re.fullmatch(r'\s*operator\s*\(\s*\)\s*', prefix[typed[0]:]))
        if function and function[1] in names:
            # A return type precedes the name, so this is not a constructor.
            # An ambiguous object/function call declarator still names a value
            # in both parses; this does not settle constructor LOC semantics.
            name_offset = prefix_offset + typed[0] + function.start(1)
            add(function[1], name_offset, close, _scope_at(ranges, name_offset),
                'cpp-named-type-ordinary-call-declarator-shadow' if suffix else
                'cpp-named-type-function-shadow-owner-review', blocked=suffix is None)
        elif typed is None:
            candidate = re.search(r'\b(' + _NAME + r')\s*$', prefix)
            if candidate and candidate[1] in names:
                before = prefix[:candidate.start()].strip()
                qualified_constructor = bool(re.fullmatch(
                    r'(?:(?:explicit|constexpr|consteval)\s+)*(?:' + _NAME + r'::)*'
                    + re.escape(candidate[1]) + r'::~?' + re.escape(candidate[1]), prefix))
                auto_value = bool(re.fullmatch(r'auto\s*[*&]*\s*' + re.escape(candidate[1]), prefix))
                # The written `new Type(...)` operand is construction syntax,
                # never a declaration of an ordinary function named Type.
                # Only the immediately adjacent unqualified type form is
                # proven here; placement/qualified/dependent forms retain
                # their existing owners or review gates.
                new_operand = bool(re.search(r'\bnew\s*$',before))
                expression_operand=bool(re.search(r'(?:\breturn|(?<![=!<>])=(?!=))\s*$',before))
                argument_operand=False
                if not expression_operand:
                    outer=[]
                    for prior in re.finditer(r'\(',scan[:prefix_offset+candidate.start(1)]):
                        end=_balanced_end(scan,prior.start())
                        if end is not None and prior.start()<opening<end:
                            outer.append((prior.start(),end))
                    if outer:
                        outer_open,_=min(outer,key=lambda row:row[1]-row[0])
                        outer_prefix=scan[_statement_start(scan,outer_open):outer_open].strip()
                        # A complete bare call prefix is argument syntax. A
                        # typed/unknown multiword declaration, function formal
                        # profile or dependent call prefix is not certified.
                        bare_call=bool(re.fullmatch(_NAME+r'(?:\s*::\s*'+_NAME+r')*',outer_prefix))
                        outer_scope=_scope_at(ranges,outer_open)
                        constructor_profile=bool(outer_scope and any(
                            entry['name']==outer_prefix and entry['body_opening']==outer_scope[2]
                            and entry['body_opening'] is not None for entry in type_entries))
                        qualified_profile=bool(re.search(r'\b('+_NAME+r')\s*::\s*\1\s*$',outer_prefix))
                        possible_aggregate_profile=False
                        if outer_scope and outer_scope[2] is not None:
                            aggregate_prefix=scan[_statement_start(scan,outer_scope[2]):outer_scope[2]]
                            aggregate_prefix=_without_owned_template_formals(aggregate_prefix)
                            possible_aggregate_profile=bool(re.search(
                                r'\b(?:class|struct|union)\s+'+_NAME+r'\b[^{};]*$',aggregate_prefix))
                        argument_operand=(bare_call and not constructor_profile and not qualified_profile
                            and not possible_aggregate_profile)
                if (before and before not in {'explicit', 'constexpr', 'consteval', 'operator'}
                        and not before.endswith('~') and not qualified_constructor and not auto_value
                        and not new_operand and not expression_operand and not argument_operand):
                    name_offset = prefix_offset + candidate.start(1)
                    add(candidate[1], name_offset, close, _scope_at(ranges, name_offset),
                        'cpp-named-type-function-shadow-owner-review', blocked=True)

        if typed and not prefix[typed[0]:].strip():
            pointer_value = re.fullmatch(r'\s*[*&]+\s*(' + _NAME + r')\s*', scan[opening + 1:close - 1])
            if pointer_value and pointer_value[1] in names:
                at = opening + 1 + pointer_value.start(1)
                add(pointer_value[1], at, close, _scope_at(ranges, opening),
                    'cpp-named-type-pointer-value-shadow')

        constructor = re.fullmatch(r'(?:(?:explicit|constexpr|consteval)\s+)*(' + _NAME + r')', prefix)
        constructor_owned = bool(constructor and any(
            entry['name'] == constructor[1] and entry['body_opening'] is not None
            and entry['body_opening'] < opening < entry['body_end'] for entry in type_entries))
        lambda_owned = prefix.endswith(']')
        formal_owner = bool(function or typed_operator or constructor_owned or lambda_owned)
        if not formal_owner:
            continue
        if suffix and suffix[1] == ';':
            continue
        body_scope = (_scope_at(ranges, marker_offset) if suffix else
                      _scope_at(ranges, opening))
        uncertain_lifetime = suffix is None
        parts = _split_params(scan[opening + 1:close - 1])
        if parts is None:
            continue
        for relative, raw_param in parts:
            param = raw_param.strip()
            at = opening + 1 + relative + len(raw_param) - len(raw_param.lstrip())
            typed_param = type_prefix(param, environment, at)
            if typed_param:
                raw_remainder = param[typed_param[0]:]
                remainder_at = typed_param[0] + len(raw_remainder) - len(raw_remainder.lstrip())
                remainder = raw_remainder.strip()
            elif re.match(r'auto\b', param):
                auto_prefix = re.match(r'auto\b\s*', param)
                remainder_at = auto_prefix.end()
                remainder = param[remainder_at:]
            else:
                remainder = None
            if remainder is not None:
                value = re.match(r'(?:(?:\*|&&|&)\s*(?:(?:const|volatile|restrict)\s+)*)*'
                                 r'(' + _NAME + r')\b(?:\s*\[[^\[\]]*\])*\s*(?:=.*)?$', remainder, re.S)
                pointer_value = re.match(r'\(\s*[*&]+\s*(' + _NAME + r')\s*\)\s*\([^{};]*\)\s*$', remainder)
                value = value or pointer_value
                if value:
                    if value[1] in names:
                        name_offset = at + remainder_at + value.start(1)
                        add(value[1], name_offset, close, body_scope,
                            'cpp-named-type-parameter-lifetime-owner-review' if uncertain_lifetime else
                            'cpp-named-type-formal-parameter-shadow', blocked=uncertain_lifetime)
                    continue
            # Do not treat a sole type-name occurrence as an unnamed value.
            for name in names:
                if re.fullmatch(r'(?:const\s+)?' + re.escape(name) + r'\s*[*&]*', param):
                    continue
                occurrence = re.search(r'\b' + re.escape(name) + r'\b', param)
                if occurrence:
                    add(name, at + occurrence.start(), close, body_scope,
                        'cpp-named-type-parameter-shadow-owner-review', blocked=True)
    return result
