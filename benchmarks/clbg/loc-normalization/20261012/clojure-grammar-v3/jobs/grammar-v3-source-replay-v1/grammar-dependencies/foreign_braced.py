"""Unhooked, unexecuted C#/Dart source-unit proposal.

This is explicit foreign grammar, never an alias to the C/Java recognizers.
It shares only language-neutral owner/event accounting and parenthesis/span
helpers. No producer, collector, chart, source-hash, or benchmark hook lives
here. Every returned count is a proposal pending the locked fixture/source
review. Ambiguous syntax produces diagnostics instead of a certificate.

Lexical references consulted for the proposal, not runtime dependencies:
https://dart.dev/language/comments
https://dart.dev/language/built-in-types#strings
https://learn.microsoft.com/en-us/dotnet/csharp/language-reference/
language-specification/lexical-structure
"""
import re

from canonical import _header_start
from foreign_core import ASSIGN, brace_pairs, count_owned, replacement, structural


LANGUAGES = {'csharp', 'dart'}
PROPOSAL_VERSION = 'foreign-csharp-dart-unhooked-20261012-v1'
IDENT = r'@?[A-Za-z_$][\w$]*'
CONTROL = {'if', 'for', 'foreach', 'while', 'switch', 'catch'}
CS_CONTROL = CONTROL | {'using', 'lock', 'fixed'}
MODIFIERS = {
    'csharp': {'public', 'private', 'protected', 'internal', 'static', 'readonly',
               'const', 'volatile', 'unsafe', 'extern', 'partial', 'async', 'new',
               'sealed', 'virtual', 'override', 'abstract', 'ref', 'in', 'out'},
    'dart': {'external', 'static', 'late', 'covariant'},
}
NON_TYPES = {'return', 'throw', 'yield', 'await', 'break', 'continue', 'if', 'else',
             'for', 'foreach', 'while', 'do', 'switch', 'case', 'default', 'catch',
             'try', 'finally', 'new', 'using', 'import', 'export', 'part', 'library'}
CS_POINTER_BUILTINS = {'bool','byte','sbyte','short','ushort','int','uint','long',
                       'ulong','char','float','double','decimal','void','nint','nuint'}
CS_TYPE_BUILTINS = CS_POINTER_BUILTINS | {'string','object','dynamic'}


def _issue(kind, offset, message):
    return dict(kind=kind, offset=offset, message=message)


def _word_before(text, offset):
    found = re.search(r'(' + IDENT + r')\s*$', text[:offset])
    return found[1] if found else ''


def _descriptor(source, index, language):
    """Return (prefix length, quote delimiter, raw, verbatim, interpolation)."""
    if language == 'csharp':
        raw = re.match(r'\$*("{3,})', source[index:])
        if raw:
            return raw.end(), raw[1], True, False, raw[0].startswith('$')
        for prefix in ('@$"', '$@"', '@"', '$"', '"', "'"):
            if source.startswith(prefix, index):
                return len(prefix), prefix[-1], False, '@' in prefix, '$' in prefix
    else:
        before = source[index-1:index] if index else ''
        raw_prefix = source[index:index+1] in {'r', 'R'} and not (before and (before.isalnum() or before in '_$'))
        start = index + int(raw_prefix)
        for delimiter in ('"""', "'''", '"', "'"):
            if source.startswith(delimiter, start):
                return int(raw_prefix) + len(delimiter), delimiter, raw_prefix, False, False
    return None


def _comment_end(source, index, language):
    depth, cursor = 1, index + 2
    while cursor < len(source):
        if language == 'dart' and source.startswith('/*', cursor):
            depth += 1
            cursor += 2
        elif source.startswith('*/', cursor):
            depth -= 1
            cursor += 2
            if not depth:
                return cursor, True
        else:
            cursor += 1
    return len(source), False


def _interpolation_end(source, opening, language, depth):
    if depth > 32:
        return len(source), False
    level, cursor = 1, opening + 1
    while cursor < len(source):
        descriptor = _descriptor(source, cursor, language)
        if descriptor:
            cursor, complete, _ = _literal_end(source, cursor, language, descriptor, depth+1)
            if not complete:
                return cursor, False
        elif source.startswith('//', cursor):
            end = source.find('\n', cursor)
            cursor = len(source) if end < 0 else end
        elif source.startswith('/*', cursor):
            cursor, complete = _comment_end(source, cursor, language)
            if not complete:
                return cursor, False
        elif source[cursor] == '{':
            level += 1
            cursor += 1
        elif source[cursor] == '}':
            level -= 1
            cursor += 1
            if not level:
                return cursor, True
        else:
            cursor += 1
    return len(source), False


def _literal_end(source, start, language, descriptor, depth=0):
    prefix_length, delimiter, raw, verbatim, interpolated = descriptor
    cursor, has_interpolation = start + prefix_length, interpolated
    triple = len(delimiter) >= 3
    while cursor < len(source):
        if source.startswith(delimiter, cursor):
            if verbatim and source.startswith('""', cursor):
                cursor += 2
                continue
            return cursor + len(delimiter), True, has_interpolation
        char = source[cursor]
        if not raw and not verbatim and char == '\\':
            cursor += min(2, len(source)-cursor)
            continue
        if language == 'csharp' and interpolated and char == '{':
            if source.startswith('{{', cursor):
                cursor += 2
                continue
            cursor, complete = _interpolation_end(source, cursor, language, depth)
            if not complete:
                return cursor, False, True
            continue
        if language == 'dart' and not raw and char == '$':
            has_interpolation = True
            if source.startswith('${', cursor):
                cursor, complete = _interpolation_end(source, cursor+1, language, depth)
                if not complete:
                    return cursor, False, True
                continue
        if char in '\r\n' and not (triple or verbatim):
            return cursor, False, has_interpolation
        cursor += 1
    return len(source), False, has_interpolation


def _scalar_interpolation_body(body,language):
    """Prove written scalar composition, never infer purity from a call name.

    Independent bindings, sequencing, executable scopes and nested unproved
    interpolation are retained gates. Literal arguments have their own lexical
    owner before punctuation/operator validation.
    """
    chars=list(body);cursor=0
    while cursor<len(body):
        if body.startswith('//',cursor) or body.startswith('/*',cursor):return False
        descriptor=_descriptor(body,cursor,language)
        if descriptor:
            end,complete,interpolation=_literal_end(body,cursor,language,descriptor)
            if not complete or interpolation or language=='csharp' and len(descriptor[1])>=3:return False
            chars[cursor:end]=replacement(body[cursor:end],literal=True)
            cursor=end
        else:cursor+=1
    code=''.join(chars).strip()
    if not code or any(char in code for char in ';{}') or '=>' in code:return False
    if re.search(r'(?<![=!<>])(?:[+*/%&|^-]=|=(?!=|>))|\+\+|--',code):return False
    if re.search(r'\b(?:if|else|for|foreach|while|switch|try|catch|throw|return|yield|'
                 r'var|final|const|class|delegate|async|await)\b',code):return False
    if re.search(r'[^A-Za-z0-9_@$\s.,:?!+*/%&|^~<>=()\[\]-]',code):return False
    _,diagnostics=brace_pairs(code)
    return not diagnostics


def _interpolation_parts(source,start,end,language,descriptor):
    """Keep exact string-owned embedded scalar spans for source review."""
    prefix_length,delimiter,raw,verbatim,interpolated=descriptor
    cursor=start+prefix_length;limit=end-len(delimiter);parts=[]
    while cursor<limit:
        if not raw and not verbatim and source[cursor]=='\\':
            cursor+=2;continue
        if verbatim and source.startswith('""',cursor):
            cursor+=2;continue
        opening=None
        if language=='csharp' and interpolated and source[cursor]=='{':
            if source.startswith('{{',cursor):cursor+=2;continue
            opening=cursor
        elif language=='dart' and not raw and source[cursor]=='$':
            if source.startswith('${',cursor):opening=cursor+1
            else:
                name=re.match(IDENT,source[cursor+1:limit])
                stop=cursor+1+(name.end() if name else 0)
                parts.append(dict(start=cursor,end=stop,scalar_composition=bool(name),
                    owner='dart-unbraced-variable'))
                cursor=stop;continue
        if opening is None:cursor+=1;continue
        stop,complete=_interpolation_end(source,opening,language,0)
        if not complete or stop>limit:
            parts.append(dict(start=opening,end=stop,scalar_composition=False,owner='unproved-interpolation'))
            break
        parts.append(dict(start=opening,end=stop,
            scalar_composition=_scalar_interpolation_body(source[opening+1:stop-1],language),
            owner=language+'-braced-expression'))
        cursor=stop
    return parts


def _mask(source, language):
    chars, diagnostics, literals, directives = list(source), [], [], []
    if source.startswith('\ufeff'):
        chars[0] = ' '
    cursor = 0
    while cursor < len(source):
        if source.startswith('//', cursor):
            end = source.find('\n', cursor)
            end = len(source) if end < 0 else end
            chars[cursor:end] = replacement(source[cursor:end])
            cursor = end
        elif source.startswith('/*', cursor):
            end, complete = _comment_end(source, cursor, language)
            chars[cursor:end] = replacement(source[cursor:end])
            if not complete:
                diagnostics.append(_issue('foreign-unclosed-comment', cursor, 'Block comment has no matching close.'))
            cursor = end
        else:
            descriptor = _descriptor(source, cursor, language)
            if not descriptor:
                cursor += 1
                continue
            end, complete, interpolation = _literal_end(source, cursor, language, descriptor)
            chars[cursor:end] = replacement(source[cursor:end], literal=True)
            embedded=_interpolation_parts(source,cursor,end,language,descriptor) if interpolation and complete else []
            literals.append(dict(start=cursor, end=end, delimiter=descriptor[1],interpolations=embedded))
            if not complete:
                diagnostics.append(_issue('foreign-unclosed-literal', cursor, 'String/character literal has no recognized closing delimiter.'))
            if interpolation and (not complete or any(not part['scalar_composition'] for part in embedded)):
                diagnostics.append(_issue('foreign-interpolation-expression-review', cursor,
                    'Interpolation has independent executable work or an unproved nested expression owner; review before admission.'))
            if language == 'csharp' and len(descriptor[1]) >= 3:
                diagnostics.append(_issue('foreign-csharp-raw-string-review', cursor,
                    'Raw C# strings need dedicated delimiter/interpolation fixtures before admission.'))
            cursor = max(end, cursor+1)
    # Compiler directives retain one meaningful source unit. Their contents
    # cannot accidentally introduce braces; conditional compilation is gated.
    offset = 0
    for row in ''.join(chars).splitlines(keepends=True):
        content = row.lstrip()
        if content.startswith('#'):
            start = offset + len(row)-len(content)
            end = offset + len(row.rstrip('\r\n'))
            directives.append(dict(kind='pragma', start=start, end=end, units=1, scope='foreign-directive'))
            chars[start:end] = replacement(source[start:end], literal=True)
            diagnostics.append(_issue('foreign-directive-policy-review', start,
                'Meaningful directive retained; conditional/packed directive policy is not yet admitted.'))
        offset += len(row)
    cleaned = ''.join(chars)
    for match in re.finditer(r'\\[uU][0-9a-fA-F]', cleaned):
        diagnostics.append(_issue('foreign-escaped-identifier-review', match.start(), 'Escaped identifier grammar requires review.'))
    for index,char in enumerate(cleaned):
        if ord(char)>127 and char.isalpha():
            diagnostics.append(_issue('foreign-unreviewed-unicode-identifier',index,'Unicode identifier recognition is outside this initial proposal.'))
    return dict(cleaned=cleaned, diagnostics=diagnostics, literals=literals, directives=directives)


def mask_csharp(source):
    return _mask(source, 'csharp')


def mask_dart(source):
    return _mask(source, 'dart')


def _last_boundary(text, end, contexts, kinds):
    """Find an owned statement boundary; never use a literal's braces."""
    for index in range(end-1, -1, -1):
        char = text[index]
        if char == ';' and not structural(contexts[index]):
            return index+1
        if char == '{' and kinds.get(index) in {'code', 'bare-code', 'type'}:
            return index+1
        if char == '}' and contexts[index]:
            own = contexts[index][-1]
            if own[0] == '{' and own[1] in {'code', 'bare-code', 'type'}:
                return index+1
    return 0


def _trim(text, start, end):
    while start < end and text[start].isspace():
        start += 1
    return start


def _skip_space(text, cursor):
    while cursor < len(text) and text[cursor].isspace(): cursor += 1
    return cursor


def _type_end(text, *, tuple_types=False, depth=0):
    """Read bounded type syntax, without claiming an identifier is a type.

    Tuple types are read only at an explicit C# type position. Each component
    has its own type and optional formal name; component names are not runtime
    bindings. Type/value ownership for pointer uses is checked separately.
    """
    if depth > 24: return None
    if tuple_types and text.startswith('('):
        cursor, components = 1, 0
        while True:
            cursor = _skip_space(text, cursor)
            component = _type_end(text[cursor:], tuple_types=True, depth=depth+1)
            if component is None: return None
            cursor += component
            after_type = _skip_space(text, cursor)
            name = re.match(IDENT, text[after_type:]) if after_type > cursor else None
            if name: cursor = _skip_space(text, after_type+name.end())
            else: cursor = after_type
            components += 1
            if text[cursor:cursor+1] == ',': cursor += 1; continue
            if text[cursor:cursor+1] != ')' or components < 2: return None
            end = cursor+1
            break
    else:
        match = re.match(IDENT + r'(?:\s*\.\s*' + IDENT + r')*', text)
        if not match or match[0] in NON_TYPES: return None
        end = match.end()
    while True:
        cursor = _skip_space(text, end)
        if cursor < len(text) and text[cursor] == '<':
            arguments = _type_arguments_end(text, cursor, tuple_types=tuple_types, depth=depth+1)
            if arguments is None: return None
            end = arguments
        elif text.startswith('[]', cursor):
            end = cursor+2
        elif cursor < len(text) and text[cursor] == '?':
            end = cursor+1
        else:
            return end


def _type_arguments_end(text, opening, *, tuple_types=False, depth=0):
    """Return the exclusive end of a complete comma-separated type list."""
    if depth > 24 or text[opening:opening+1] != '<': return None
    cursor = opening+1
    while True:
        cursor = _skip_space(text, cursor)
        end = _type_end(text[cursor:], tuple_types=tuple_types, depth=depth+1)
        if end is None: return None
        cursor = _skip_space(text, cursor+end)
        if text[cursor:cursor+1] == ',': cursor += 1; continue
        return cursor+1 if text[cursor:cursor+1] == '>' else None


def _generic_expression_spans(text, language):
    """Protect only a type list immediately owned by a call or new array.

    An arbitrary comparison/angle expression is never treated as a type list.
    No callee spelling or benchmark-specific name participates in this proof.
    """
    spans = []
    for match in re.finditer(IDENT+r'(?:\s*\.\s*'+IDENT+r')*\s*(?=<)', text):
        opening = _skip_space(text, match.end())
        end = _type_arguments_end(text, opening, tuple_types=language=='csharp')
        if end is None: continue
        following = _skip_space(text, end)
        call = text[following:following+1] == '('
        creation = text[following:following+1] == '[' and bool(re.search(r'\bnew\s*$', text[:match.start()]))
        if call or creation: spans.append((opening,end))
    return spans


def _unproved_generic_initializer(text, language):
    spans=_generic_expression_spans(text,language)
    for match in re.finditer(r'\bnew\s+'+IDENT+r'(?:\s*\.\s*'+IDENT+r')*\s*(?=<)',text):
        opening=_skip_space(text,match.end())
        if not any(left<=opening<right for left,right in spans): return True
    for match in re.finditer(IDENT+r'(?:\s*\.\s*'+IDENT+r')*\s*(?=<[^;{}]*>\s*\()',text):
        opening=_skip_space(text,match.end())
        if not any(left<=opening<right for left,right in spans): return True
    return False


def _strip_modifiers(text, language):
    cursor = 0
    while True:
        found = re.match(r'\s*(' + IDENT + r')\b\s+', text[cursor:])
        if not found or found[1] not in MODIFIERS[language]:
            return cursor
        cursor += found.end()


def _named_prefix(prefix, language, type_names):
    code = prefix.strip()
    position = _strip_modifiers(code, language)
    code = code[position:]
    if language == 'dart' and re.fullmatch(IDENT, code):
        return True
    if re.fullmatch(IDENT, code) and code in type_names:
        return True  # Explicit constructor in its declared type.
    end = _type_end(code, tuple_types=language=='csharp')
    if end is None:
        return False
    if code.startswith('(') and not _tuple_type_owned(code[:end],type_names): return False
    name = code[end:].strip()
    return bool(re.fullmatch(IDENT + r'(?:\s*<[^{};()=]+>)?', name))


def _type_header(prefix, language):
    words = 'class|struct|interface|enum|namespace|record' if language == 'csharp' else 'class|mixin|extension|enum'
    found = re.search(r'(?<![@\w])\b(' + words + r')\s+(' + IDENT + r')(?:\s*<[^;{}=]+>)?(?:\s*[^;{}=]*)$', prefix)
    if not found:
        return None
    # An expression containing an earlier type keyword does not own this brace.
    if '=' in prefix[:found.start()]:
        return None
    return found[1], found[2]


def _callback_header(text, opening, reverse_pairs, language):
    prefix = text[:opening].rstrip()
    if language == 'csharp' and prefix.endswith('=>'):
        return True
    if language == 'csharp' and re.search(r'\bdelegate\s*(?:\([^{};]*\))?\s*$', prefix):
        return True
    # Dart => {...} returns a map/set; its block callbacks have no arrow.
    if language == 'dart' and not prefix.endswith('=>'):
        stripped = re.sub(r'\b(?:async|sync)\s*\*?\s*$', '', prefix).rstrip()
        if stripped.endswith(')'):
            start = reverse_pairs.get(len(stripped)-1)
            if start is not None:
                before = text[:start].rstrip()
                return not before or before[-1:] in '(=,:[' or bool(re.search(r'\breturn$', before))
    return False


def _attribute_body(text):
    """Prove a C# attribute list; named settings stay inside its arguments."""
    value=text.strip()
    target=re.match(r'(assembly|module|return|field|method|param|property|event|type):\s*',value)
    if target: value=value[target.end():]
    parts=[]; start=0; depth=0
    for index,char in enumerate(value):
        if char=='(': depth+=1
        elif char==')': depth-=1
        elif char==',' and not depth: parts.append(value[start:index].strip()); start=index+1
    parts.append(value[start:].strip())
    for part in parts:
        named=re.match(IDENT+r'(?:\s*\.\s*'+IDENT+r')*',part)
        if named is None: return None
        rest=part[named.end():].strip()
        if rest:
            pairs,issues=brace_pairs(rest)
            if issues or not rest.startswith('(') or pairs.get(0)!=len(rest)-1: return None
    return target[1] if target else ''


def _attribute_owners(text,pairs,language):
    if language!='csharp': return text,[],set()
    chars=list(text); events=[]; owned=set()
    for opening,char in enumerate(text):
        if char!='[' or opening in owned: continue
        closing=pairs.get(opening)
        if closing is None: continue
        role=_attribute_body(text[opening+1:closing])
        if role is None: continue
        # A declaration attachment or assembly/module target is mandatory.
        # Array indexing/data punctuation never acquires attribute ownership.
        before=text[:opening].rstrip()
        at_boundary=not before or before[-1:] in ';{}' or not text[text.rfind('\n',0,opening)+1:opening].strip()
        if not at_boundary and before.endswith(']'):
            at_boundary=any(event['end']==len(before) for event in events)
        if not at_boundary: continue
        tail_start=closing+1
        while tail_start<len(text) and text[tail_start].isspace(): tail_start+=1
        while tail_start<len(text) and text[tail_start]=='[':
            following=pairs.get(tail_start)
            if following is None or _attribute_body(text[tail_start+1:following]) is None: break
            tail_start=following+1
            while tail_start<len(text) and text[tail_start].isspace(): tail_start+=1
        tail=text[tail_start:]
        terminal=re.search(r'[;{]',tail)
        declaration=tail[:terminal.start()] if terminal else tail
        prefix=declaration.split('(',1)[0]
        attached=bool(_type_header(declaration,'csharp') or _named_prefix(prefix,'csharp',set()) or _declarators(declaration,'csharp'))
        if role not in {'assembly','module'} and not attached: continue
        events.append(dict(kind='attribute',start=opening,end=closing+1,units=1,scope='foreign-csharp-attribute'))
        chars[opening:closing+1]=replacement(text[opening:closing+1])
        owned.update(range(opening,closing+1))
    return ''.join(chars),events,owned


def classify_scopes(cleaned, language):
    """Supply explicit grammar owners in foreign_core's character context form."""
    if language not in LANGUAGES:
        return dict(contexts=[()] * len(cleaned), events=[], executable_regions=[],
            diagnostics=[_issue('foreign-unsupported-language', 0, 'Only explicit C#/Dart grammar is proposed.')])
    pairs, diagnostics = brace_pairs(cleaned)
    original_cleaned=cleaned
    cleaned,attribute_events,attribute_owned=_attribute_owners(cleaned,pairs,language)
    reverse_pairs = {end:start for start,end in pairs.items()}
    generic_calls={_skip_space(cleaned,end) for _,end in _generic_expression_spans(cleaned,language)
        if cleaned[_skip_space(cleaned,end):_skip_space(cleaned,end)+1]=='('}
    contexts, stack, kinds, owners, type_names, type_owners = [], [], {}, {}, set(), []
    control_words = CS_CONTROL if language == 'csharp' else CONTROL
    for index, char in enumerate(cleaned):
        contexts.append(tuple(stack))
        if char == '(':
            word = _word_before(cleaned, index)
            kind = 'for' if word in {'for','foreach'} else 'control' if word in control_words else 'args' if word or index in generic_calls else 'group'
            end = pairs.get(index)
            tail = cleaned[end+1:].lstrip() if end is not None else ''
            body_tail = bool(re.match(r'(?:async\s*\*?\s*|sync\s*\*?\s*)?\{',tail))
            if end is not None and (tail.startswith('=>') or body_tail or tail.startswith(';')):
                start = _last_boundary(cleaned, index, contexts, kinds)
                prefix = cleaned[start:index]
                named = _named_prefix(prefix,language,type_names)
                prototype = named and (language=='csharp' or bool(re.search(r'\s',prefix.strip())) or prefix.strip() in type_names)
                if tail.startswith('=>') or (body_tail and (named or (language=='dart' and (not word or word=='return')))) or (tail.startswith(';') and prototype):
                    kind = 'parameters'
            stack.append((char, kind, index))
        elif char == '[':
            stack.append((char, 'data', index))
        elif char == '{':
            start = _trim(cleaned, _last_boundary(cleaned, index, contexts, kinds), index)
            prefix = cleaned[start:index].rstrip()
            own = dict(kind='data', start=None, scope='foreign-data', callback=False)
            nearest = next((entry for entry in reversed(stack) if entry[0]=='('), None)
            suffix = prefix
            parenthesized = re.sub(r'\b(?:async|sync)\s*\*?\s*$', '', suffix).rstrip()
            paren_start = reverse_pairs.get(start+len(parenthesized)-1) if parenthesized.endswith(')') else None
            named = paren_start is not None and _named_prefix(cleaned[start:paren_start], language, type_names)
            callback = _callback_header(cleaned, index, reverse_pairs, language)
            type_info = _type_header(prefix, language)
            control = bool(re.search(r'\b(?:else|try|finally|do|unsafe|checked|unchecked)\s*$', prefix))
            if paren_start is not None:
                control = control or _word_before(cleaned, paren_start) in control_words
            new_data = bool(re.search(r'(?:^|[=(,:?]|\breturn)\s*new\b[^;{}]*$', prefix))
            parameter_data = bool(nearest and nearest[1]=='parameters' and cleaned[:index].rstrip().endswith(('(',',')))
            if parameter_data or new_data or (language=='dart' and prefix.endswith('=>')):
                pass
            elif callback:
                own.update(kind='code', scope='foreign-callback', callback=True)
            elif type_info:
                keyword, name = type_info
                type_names.add(name)
                own.update(kind='type', start=start, scope='foreign-type')
                enclosing = next((entry[2] for entry in reversed(stack) if entry[0]=='{'),None)
                if keyword in {'class','struct','interface'}:
                    type_owners.append(dict(name=name,start=start,end=index,
                        visible_start=start,visible_end=pairs.get(enclosing,len(cleaned)),
                        scope_start=enclosing if enclosing is not None else 0,
                        kind='source-local-'+keyword))
                if keyword not in {'class','struct','interface','namespace'}:
                    diagnostics.append(_issue('foreign-type-policy-review', start, 'Enum/record/mixin/extension declaration grammar is not yet admitted.'))
            elif named:
                own.update(kind='code', start=start, scope='foreign-function')
            elif control:
                own.update(kind='code', scope='foreign-control')
            elif re.search(r'(?:=|,|:|\(|\breturn)\s*(?:<[^{};]*>)?\s*$', prefix) or structural(tuple(stack)):
                pass
            elif stack and stack[-1][1] in {'code','bare-code'} and (not prefix or cleaned[:index].rstrip().endswith((';','{','}'))):
                own.update(kind='bare-code', start=index, scope='foreign-bare-block')
            else:
                diagnostics.append(_issue('foreign-ambiguous-brace-review', index,
                    'Brace lacks a proven C#/Dart function/control/type/data owner; no fallback admission.'))
            if own['kind'] in {'code','bare-code','type'} and re.search(r'\b(?:get|set|operator|where|factory)\b|:\s*(?:base|this)\s*\(', prefix):
                diagnostics.append(_issue('foreign-complex-header-review', start, 'Accessor/operator/constraint/constructor-initializer header requires dedicated review.'))
            owners[index] = own
            kinds[index] = own['kind']
            stack.append((char, own['kind'], index))
        elif char in ')]}':
            wanted = {')':'(', ']':'[', '}':'{'}[char]
            if stack and stack[-1][0] == wanted:
                stack.pop()
    events, regions = list(attribute_events), []
    for opening, own in owners.items():
        if own['kind'] not in {'code','bare-code','type'}:
            continue
        start = own['start']
        if start is None:
            start = _header_start(cleaned, opening, contexts, events, language=None)
        own['resolved_start'] = start
        # This reuses a generic span helper only after foreign owner proof.
        if start < opening:
            events.append(dict(kind='header', start=start, end=opening, units=1, scope=own['scope']))
        events.append(dict(kind='open', start=opening, end=opening+1, units=0, scope=own['scope']))
        close = pairs.get(opening)
        if close is None:
            continue
        events.append(dict(kind='close', start=close, end=close+1, units=1, scope=own['scope']))
        regions.append(dict(start=opening+1,end=close,scope=own['scope']))
        if own['callback']:
            # Shared core adds zero-unit closing punctuation only after proving
            # its enclosing argument/data owner. Do not guess ')' or ',' here.
            cursor = close+1
            while cursor < len(cleaned) and cleaned[cursor] in ' \t\r': cursor += 1
            suffix_start = cursor
            while cursor < len(cleaned) and cleaned[cursor] in ')];': cursor += 1
            following = cursor
            while following < len(cleaned) and cleaned[following] in ' \t\r': following += 1
            if following < len(cleaned) and cleaned[following] in ',.' and ';' not in cleaned[suffix_start:cursor]:
                diagnostics.append(_issue('foreign-callback-surrounding-expression-review', following,
                    'Callback is followed by another argument/composition; review the surrounding expression owner.'))
    def covered(offset):
        return any(event['start']<=offset<event['end'] for event in events)
    pattern = r'(?<![@\w])\b(?:if|for|foreach|while|switch|catch|else|do|try|finally' + (r'|using|lock|fixed' if language=='csharp' else '') + r')\b'
    for match in re.finditer(pattern, cleaned):
        if covered(match.start()): continue
        if structural(contexts[match.start()]):
            if language=='dart' and match[0] in {'if','for'} and any(kind=='data' for _,kind,_ in contexts[match.start()]):
                diagnostics.append(_issue('foreign-dart-collection-control-review',match.start(),'Collection if/for requires reviewed data/executable ownership.'))
            continue
        word, cursor = match[0], match.end()
        while cursor<len(cleaned) and cleaned[cursor].isspace(): cursor += 1
        if word in control_words:
            if cursor >= len(cleaned) or cleaned[cursor]!='(':
                # A using directive/alias is an ordinary declaration.
                if word=='using': continue
                diagnostics.append(_issue('foreign-control-header-review',match.start(),'Parenthesized control header was not recognized.'))
                continue
            end = pairs.get(cursor)
            if end is None: continue
            start = match.start()
            if word=='if':
                branch = re.search(r'\belse\s*$',cleaned[:start])
                if branch: start=branch.start()
            preceding = cleaned[:start].rstrip()
            if word=='while' and preceding.endswith('}'):
                previous_close = len(preceding)-1
                opening = reverse_pairs.get(previous_close)
                owner = owners.get(opening,{})
                header_start = owner.get('resolved_start')
                if header_start is not None and re.match(r'do\b',cleaned[header_start:opening]):
                    events.append(dict(kind='continuation',start=start,end=end+1,units=0,scope='foreign-do-tail'))
                    continue
            events.append(dict(kind='header',start=start,end=end+1,units=1,scope='foreign-unbraced-control'))
        elif word=='else' and re.match(r'if\b',cleaned[cursor:]):
            continue  # The following if event owns the full else-if header.
        else:
            events.append(dict(kind='branch' if word=='else' else 'header',start=match.start(),end=match.end(),units=1,scope='foreign-unbraced-control'))
    for match in re.finditer(r'(?<![@\w])\b(?:case|default|record|with)\b',cleaned):
        tail=cleaned[match.end():]
        if language=='csharp' and match[0]=='default' and not re.match(r'\s*=>',tail) and re.match(r'\s*(?:[;,\)\]\}?=|&+*/%<>^.!-]|\(|$)',tail):
            # A default value/default(T) has expression ownership. It cannot
            # acquire a switch-label unit just because it shares the keyword.
            continue
        diagnostics.append(_issue('foreign-pattern-switch-record-review',match.start(),'Pattern/switch arm/record syntax requires a dedicated grammar decision.'))
    if language=='csharp':
        for index,char in enumerate(original_cleaned):
            if char!='[' or index in attribute_owned: continue
            if structural(contexts[index]):
                closing=pairs.get(index)
                if closing is not None and any(kind=='parameters' for _,kind,_ in contexts[index]) and _attribute_body(original_cleaned[index+1:closing]) is not None:
                    tail=original_cleaned[closing+1:]; end=_type_end(tail.lstrip())
                    if end is not None and re.match(r'\s+'+IDENT,tail.lstrip()[end:]):
                        diagnostics.append(_issue('foreign-csharp-formal-attribute-review',index,'Formal attribute needs a separately owned unit without overlapping the function header.'))
                continue
            start = _last_boundary(cleaned,index,contexts,kinds)
            if not cleaned[start:index].strip():
                diagnostics.append(_issue('foreign-csharp-attribute-policy-review',index,'Attribute group must have explicit declaration/packing ownership.'))
    if language=='csharp':
        aliases,alias_events,alias_issues = _type_alias_owners(cleaned,contexts,pairs,owners)
        type_owners.extend(aliases); events.extend(alias_events); diagnostics.extend(alias_issues)
    return dict(contexts=contexts,events=events,executable_regions=regions,
        diagnostics=diagnostics,type_owners=type_owners)


def _type_alias_owners(text, contexts, pairs, owners):
    """Own an explicit namespace/translation-unit C# type alias once.

    Alias components are type/formal structure, including written wrapping.
    Method-local aliases, unbounded/unknown RHS forms and duplicate aliases
    remain review gates; no alias name is promoted by capitalisation.
    """
    entries, events, diagnostics = [], [], []
    kinds={opening:owner['kind'] for opening,owner in owners.items()}
    for match in re.finditer(r'(?<![@\w])using\s+('+IDENT+r')\s*=\s*',text):
        context=contexts[match.start()]
        boundary=_last_boundary(text,match.start(),contexts,kinds)
        if text[boundary:match.start()].strip() or structural(context) or any(kind in {'code','bare-code'} for _,kind,_ in context):
            diagnostics.append(_issue('foreign-csharp-type-alias-owner-review',match.start(),'Type alias lacks a namespace/translation-unit declaration owner.'))
            continue
        enclosing=next((opening for char,kind,opening in reversed(context) if char=='{'),None)
        if enclosing is not None:
            owner=owners.get(enclosing,{})
            prefix=text[owner.get('resolved_start',enclosing):enclosing]
            if not re.search(r'\bnamespace\s+',prefix):
                diagnostics.append(_issue('foreign-csharp-type-alias-owner-review',match.start(),'Class/type-contained using alias is not a proven legal namespace owner.'))
                continue
        rhs=match.end(); end=_type_end(text[rhs:],tuple_types=True)
        stop=_skip_space(text,rhs+end) if end is not None else rhs
        if end is None or text[stop:stop+1]!=';':
            diagnostics.append(_issue('foreign-csharp-type-alias-owner-review',match.start(),'Alias RHS requires an exact bounded type form followed by its own semicolon.'))
            continue
        scope_end=pairs.get(enclosing,len(text))
        entry=dict(name=match[1],start=match.start(),end=stop+1,
            visible_start=stop+1,visible_end=scope_end,
            scope_start=enclosing if enclosing is not None else 0,
            kind='source-local-csharp-type-alias')
        if any(other['name']==entry['name'] and other['scope_start']==entry['scope_start'] for other in entries):
            diagnostics.append(_issue('foreign-csharp-type-alias-shadow-review',match.start(),'Duplicate same-scope type alias requires a visibility decision.'))
        entries.append(entry)
        events.append(dict(kind='type-alias',start=match.start(),end=stop+1,units=1,scope='foreign-csharp-type-alias'))
    return entries,events,diagnostics


def _split_declarators(rest, language):
    generic_spans = _generic_expression_spans(rest,language)
    parts, start, stack = [], 0, []
    for index,char in enumerate(rest):
        if any(left <= index < right for left,right in generic_spans): continue
        if char in '([{': stack.append(char)
        elif char in ')]}' and stack: stack.pop()
        elif char==',' and not stack:
            parts.append(rest[start:index].strip()); start=index+1
    parts.append(rest[start:].strip())
    if not all(re.fullmatch(IDENT+r'(?:\s*=.*)?',part,re.S) for part in parts): return None
    return parts


def _tuple_type_owned(text, type_names):
    """Require explicit type evidence for every named tuple component."""
    cursor = 1
    while cursor < len(text):
        cursor = _skip_space(text,cursor)
        end = _type_end(text[cursor:],tuple_types=True)
        if end is None: return False
        component=text[cursor:cursor+end].strip()
        if component.startswith('('):
            if not _tuple_type_owned(component,type_names): return False
        else:
            base=re.match(IDENT+r'(?:\s*\.\s*'+IDENT+r')*',component)
            if base is None or base[0] not in CS_TYPE_BUILTINS and base[0] not in type_names:
                return False
        cursor = _skip_space(text,cursor+end)
        name = re.match(IDENT,text[cursor:])
        if name: cursor = _skip_space(text,cursor+name.end())
        if text[cursor:cursor+1]==',': cursor+=1; continue
        return text[cursor:cursor+1]==')'
    return False


def _declarators(code, language, type_names=()):
    position = _strip_modifiers(code, language)
    rest = code[position:]
    inferred = re.match(r'(?:var|final|const)\b\s*', rest) if language=='dart' else re.match(r'var\b\s*',rest)
    if inferred:
        rest = rest[inferred.end():]
        # Dart final/const can also carry an explicit type.
        type_end = _type_end(rest,tuple_types=language=='csharp')
        if language=='dart' and type_end is not None and re.match(r'\s+'+IDENT+r'\s*(?:=|,|$)',rest[type_end:]):
            rest = rest[type_end:].lstrip()
    else:
        end = _type_end(rest,tuple_types=language=='csharp')
        if end is None or end==len(rest): return None
        if rest.startswith('(') and not _tuple_type_owned(rest[:end],type_names): return None
        pointer = re.match(r'\s*\*+\s*',rest[end:]) if language=='csharp' else None
        if pointer:
            type_name=rest[:end].strip()
            if type_name not in CS_POINTER_BUILTINS and type_name not in type_names: return None
            end += pointer.end()
        elif not rest[end].isspace(): return None
        rest = rest[end:].lstrip()
    if rest.startswith(('(', '[', '{')): return None
    return _split_declarators(rest,language)


def _method_prototype(code, language, type_names=()):
    """Prove a typed declaration with an entirely owned formal list.

    This is deliberately narrower than a call-shaped expression: an explicit
    return type and separate method name are required, and no initializer or
    body may follow the matched parameters. Constructor prototypes already
    have their named-type/formal owner in ``classify_scopes``.
    """
    prefix = code[_strip_modifiers(code, language):]
    end = _type_end(prefix,tuple_types=language=='csharp')
    if end is None or end == len(prefix) or not prefix[end].isspace():
        return False
    if prefix.startswith('(') and not _tuple_type_owned(prefix[:end],type_names): return False
    named = re.match(r'\s+' + IDENT + r'(?:\s*<[^{};()=]+>)?\s*\(', prefix[end:])
    if not named:
        return False
    opening = end + named.end()-1
    pairs, diagnostics = brace_pairs(prefix)
    closing = pairs.get(opening)
    return not diagnostics and closing is not None and not prefix[closing+1:].strip()


def _expression_method_body(code, language, type_names=()):
    """Own typed scalar expression methods; formal commas are structural."""
    prefix=code[_strip_modifiers(code,language):]
    end=_type_end(prefix,tuple_types=language=='csharp')
    if end is None or end==len(prefix) or not prefix[end].isspace(): return None
    if prefix.startswith('(') and not _tuple_type_owned(prefix[:end],type_names): return None
    named=re.match(r'\s+'+IDENT+r'(?:\s*<[^{};()=]+>)?\s*\(',prefix[end:])
    if not named: return None
    opening=end+named.end()-1
    pairs,diagnostics=brace_pairs(prefix)
    closing=pairs.get(opening)
    if diagnostics or closing is None: return None
    body=prefix[closing+1:].strip()
    return body[2:].strip() if body.startswith('=>') else None


def _remaining_statements(text, contexts, events):
    """Return exact ordinary statement spans after separately owned headers."""
    chars=list(text)
    boundaries={0,len(text)}
    for event in events:
        chars[event['start']:event['end']]=replacement(text[event['start']:event['end']])
        boundaries.update((event['start'],event['end']))
    remainder=''.join(chars)
    boundaries.update(index+1 for index,char in enumerate(remainder)
        if char==';' and not structural(contexts[index]))
    ordered=sorted(boundaries); statements=[]
    for left,right in zip(ordered,ordered[1:]):
        start=_skip_space(remainder,left)
        code=remainder[start:right].strip().removesuffix(';').rstrip()
        if code and not structural(contexts[start]): statements.append((start,right,code))
    return statements


def _identifier_assignment_body(body, language, depth=0):
    """Count only written identifier assignments ending in scalar composition.

    This does not infer effects from a callee name. Nested/parenthesized effects,
    callbacks, sequences and unknown executable forms retain their review gate.
    A chain's numeric count is exposed even while its body/scope owner remains
    review gated.
    """
    if depth>32: return None
    assigned=re.match(IDENT+r'\s*=(?!=|>)\s*',body)
    if assigned is None: return None
    rhs=body[assigned.end():].strip()
    chained=_identifier_assignment_body(rhs,language,depth+1)
    if chained is not None: return chained+1
    return 1 if _scalar_interpolation_body(rhs,language) else None


def _method_statement_owners(text, classified, language):
    """Own complete typed prototypes and explicit assignment arrow headers.

    Prototype formals belong to one declaration header regardless wrapping.
    Attribute groups already have separate owners. A mutating arrow has an
    explicit named header plus its written assignment body, without any
    imputed closing unit. Scalar arrows keep their existing single definition
    unit. Events never overlap separately owned executable/formal work.
    """
    events,diagnostics,evidence=[],[],[]
    for start,end,code in _remaining_statements(text,classified['contexts'],classified['events']):
        if text[end-1:end]!=';': continue
        names=_visible_type_names(classified['type_owners'],(),start)
        if _method_prototype(code,language,names):
            events.append(dict(kind='header',start=start,end=end,units=1,scope='foreign-method-prototype'))
            evidence.append(dict(start=start,end=end,kind='typed-method-prototype',header_units=1,written_close_units=0))
            continue
        body=_expression_method_body(code,language,names)
        if body is None: continue
        assignments=_identifier_assignment_body(body,language)
        if assignments is None: continue
        body_start=start+len(code)-len(body)
        events.append(dict(kind='header',start=start,end=body_start,units=1,scope='foreign-assignment-arrow-header'))
        evidence.append(dict(start=start,end=end,body_start=body_start,
            kind='named-assignment-arrow',header_units=1,
            body_assignment_units=assignments,written_close_units=0,
            body_scope_review=assignments>1))
        if assignments>1:
            diagnostics.append(_issue('foreign-expression-method-body-review',body_start,
                'Named header is owned; full chained body/scope ownership still requires review. Proposal exposes header1 plus '+str(assignments)+' written assignments with no imputed close.'))
    return events,diagnostics,evidence


def _type_value_shadows(text, contexts, events, pairs, type_owners, language):
    """Retain source-local value shadows before proving named pointer types.

    Only an explicit declaration/formal spelling creates a shadow record. An
    unresolved use of a name does not become a type by capitalisation. Records
    are conservative evidence/gates, not new count units.
    """
    if language!='csharp': return []
    names={entry['name'] for entry in type_owners}; shadows=[]
    def add(name,start,end,context,scope=None):
        if name not in names: return
        enclosing=scope if scope is not None else next((opening for char,kind,opening in reversed(context) if char=='{' and kind in {'code','bare-code','type'}),None)
        field=scope is None and any(char=='{' and kind=='type' and opening==enclosing for char,kind,opening in context)
        shadows.append(dict(name=name,start=start,end=end,
            visible_start=enclosing if field else start,visible_end=pairs.get(enclosing,len(text)),
            scope_start=enclosing if enclosing is not None else 0))
    for start,end,code in _remaining_statements(text,contexts,events):
        parts=_declarators(code,language,names)
        if parts is None: continue
        for part in parts:
            named=re.match(IDENT,part)
            if named: add(named[0],start,end,contexts[start])
    for opening,closing in pairs.items():
        if text[opening]!='(' or opening+1>=len(contexts): continue
        if not any(kind=='parameters' and owner==opening for _,kind,owner in contexts[opening+1]): continue
        following=_skip_space(text,closing+1)
        scope=following if text[following:following+1]=='{' else opening
        # Generic types protect commas; tuple/attribute/unknown formal syntax
        # is left unresolved rather than flattening its names into shadows.
        formals=text[opening+1:closing]
        comma_mask=list(formals)
        for match in re.finditer(IDENT+r'\s*(?=<)',formals):
            begin=_skip_space(formals,match.end()); stop=_type_arguments_end(formals,begin,tuple_types=True)
            if stop is not None:
                for index in range(begin,stop):
                    if comma_mask[index]==',': comma_mask[index]=' '
        parts=[]; part_start=0; stack=[]
        for index,char in enumerate(''.join(comma_mask)):
            if char in '([{': stack.append(char)
            elif char in ')]}' and stack: stack.pop()
            elif char==',' and not stack: parts.append((part_start,formals[part_start:index])); part_start=index+1
        parts.append((part_start,formals[part_start:]))
        for relative,part in parts:
            prefix=part[_strip_modifiers(part,language):].lstrip()
            end=_type_end(prefix,tuple_types=True)
            if end is None: continue
            pointer=re.match(r'\s*\*+\s*',prefix[end:])
            if pointer:
                end+=pointer.end()
                named=re.match(r'('+IDENT+r')(?:\s*=.*)?$',prefix[end:],re.S)
            else:
                named=re.match(r'\s+('+IDENT+r')(?:\s*=.*)?$',prefix[end:],re.S)
            if named: add(named[1],opening+1+relative,closing,contexts[opening],scope)
    return shadows


def _visible_type_names(type_owners, shadows, offset):
    names={entry['name'] for entry in type_owners
        if entry['visible_start']<=offset<entry['visible_end']}
    blocked={entry['name'] for entry in shadows
        if entry['visible_start']<=offset<entry['visible_end']}
    return names-blocked


def _wrapped_declaration_owners(text, classified, language, shadows):
    """Prove whole comma declarations before accepting row continuations.

    This supplies metadata only. Initializer/literal wrapping still contributes
    its written rows through foreign_core; it is never hidden in one event.
    """
    owners=[]
    for start,end,code in _remaining_statements(text,classified['contexts'],classified['events']):
        if '\n' not in code: continue
        names=_visible_type_names(classified['type_owners'],shadows,start)
        parts=_declarators(code,language,names)
        if parts is not None and len(parts)>1:
            owners.append(dict(start=start,end=end,parts=parts))
    return owners


def binding_counter(language, *, type_owners=(), shadows=(), wrapped_owners=()):
    def counter(*, text, original, context, assignments):
        if structural(context):
            return dict(reason='structural-data-parameter-or-control-row')
        code = text.strip()
        offset=next((index for _,kind,index in reversed(context) if kind=='foreign-source-position'),-1)
        type_names=_visible_type_names(type_owners,shadows,offset)
        tuple_lhs=re.sub(r'^(?:var|final)\s+','',code)
        if tuple_lhs.startswith('('):
            tuple_pairs,tuple_issues=brace_pairs(tuple_lhs)
            tuple_close=tuple_pairs.get(0)
            if not tuple_issues and tuple_close is not None and re.match(r'\s*=(?!=|>)',tuple_lhs[tuple_close+1:]):
                return dict(units=1,declaration_bindings=1,reason='explicit-tuple-destructuring')
        method_body=_expression_method_body(code,language,type_names)
        if method_body is not None:
            if _scalar_interpolation_body(method_body,language):
                return dict(units=1,declaration_bindings=0,reason='proven-scalar-method-with-structural-formals')
            return dict(diagnostics=[dict(kind='foreign-expression-method-body-review',message='Expression method contains non-scalar or unresolved binding/executable composition.')])
        wrapped=next((entry for entry in wrapped_owners if entry['start']<=offset<entry['end']),None)
        row_code=code.rstrip().removesuffix(',').rstrip() if wrapped else code
        parts = _declarators(row_code,language,type_names)
        if parts is None and wrapped:
            parts=_split_declarators(row_code,language)
        diagnostics = []
        if parts is not None:
            if any(_unproved_generic_initializer(part,language) for part in parts):
                diagnostics.append(dict(kind='foreign-generic-initializer-review',message='Initializer generic syntax has no complete attached type-argument/call owner.'))
            bare = sum(not ASSIGN.search(part) for part in parts)
            return dict(units=max(len(parts),len(assignments)+bare),declaration_bindings=len(parts),
                reason='independent-initialized-uninitialized-and-chained-declarators',diagnostics=diagnostics)
        if _method_prototype(code, language,type_names):
            return dict(units=1, declaration_bindings=0,
                reason='proven-method-prototype-with-structural-formals')
        prefix = code[_strip_modifiers(code,language):]
        type_end = _type_end(prefix,tuple_types=language=='csharp')
        typed = type_end is not None and bool(re.match(r'\s+[*&]?\s*'+IDENT,prefix[type_end:]))
        pointer = type_end is not None and bool(re.match(r'\s*\*+\s*'+IDENT,prefix[type_end:]))
        tuple_type = bool(re.match(r'^\([^{};]*\)\s+'+IDENT,code))
        if pointer or tuple_type or re.search(r'\bdelegate\s*\*',code):
            diagnostics.append(dict(kind='foreign-complex-type-declaration-review',message='Pointer/function-pointer/tuple-type declarations require reviewed declarator grammar.'))
        if _unproved_generic_initializer(code,language):
            diagnostics.append(dict(kind='foreign-generic-initializer-review',message='Unresolved initializer type list/call/continuation retains its review gate.'))
        if ',' in code and (typed or re.match(r'^(?:var|final|const|public|private|protected|internal|static|readonly|late)\b',code)):
            diagnostics.append(dict(kind='foreign-complex-declaration-review',message='Possible packed declaration has an unresolved type/initializer/continuation form.'))
        return dict(diagnostics=diagnostics)
    return counter


def normalize(source, language):
    """Return proposal metrics only; no hook/certificate/admission is implied."""
    # Runtime mode labels name the same explicitly proposed source grammar.
    language = {'csharp-aot':'csharp','csharp-jit':'csharp','dart-aot':'dart','dart-jit':'dart'}.get(language,language)
    if language not in LANGUAGES:
        raise ValueError('foreign_braced proposes C#/Dart only; Zig/Odin remain unsupported.')
    masked = mask_csharp(source) if language=='csharp' else mask_dart(source)
    classified = classify_scopes(masked['cleaned'],language)
    method_events,method_issues,method_evidence=_method_statement_owners(masked['cleaned'],classified,language)
    classified['events'].extend(method_events); classified['diagnostics'].extend(method_issues)
    pairs,_=brace_pairs(masked['cleaned'])
    shadows=_type_value_shadows(masked['cleaned'],classified['contexts'],classified['events'],pairs,classified['type_owners'],language)
    wrapped=_wrapped_declaration_owners(masked['cleaned'],classified,language,shadows)
    # Offset metadata precedes grammar frames. Shared suffix ownership uses
    # context[-1] as the immediate real argument/data owner.
    contexts=[(('.', 'foreign-source-position',index),)+context for index,context in enumerate(classified['contexts'])]
    result = count_owned(source,masked['cleaned'],contexts,
        masked['directives']+classified['events'],masked['diagnostics']+classified['diagnostics'],
        language,binding_counter=binding_counter(language,type_owners=classified['type_owners'],shadows=shadows,wrapped_owners=wrapped))
    result.update(proposal_version=PROPOSAL_VERSION,proposal_only=True,proposal_status='unhooked-no-admission',
        admission='Not admitted: unhooked C#/Dart proposal awaiting locked fixtures, exact source/report pins, and root grammar review.',
        lexical_spans=masked['literals'],executable_regions=classified['executable_regions'],
        source_type_owners=classified['type_owners'],source_type_value_shadows=shadows,
        wrapped_declaration_owners=wrapped,method_statement_owners=method_evidence)
    return result
