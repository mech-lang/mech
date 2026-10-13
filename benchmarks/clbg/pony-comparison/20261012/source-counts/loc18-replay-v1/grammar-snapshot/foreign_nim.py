"""Unhooked Nim ownership proposal; no benchmark or chart admission.

Nim layout owns declarations, procedure bodies, and control bodies. Dedents
do not invent a written closer. This module is a separate grammar, not a
different language's lexer or a benchmark-specific LOC shortcut. Execute
imports, fixtures, and source replay only under the benchmark timing mutex.
"""
import re

from foreign_core import brace_pairs, count_owned, replacement, structural, top_parts
from foreign_future_mask import mask_future


LANGUAGES = {'nim'}
PROPOSAL_VERSION = 'foreign-nim-unhooked-20261012-v1'
IDENT = r'[A-Za-z_]\w*'
ASSIGN = re.compile(r'(?<![=!<>~:])(?:[+*/%&|^-]=|=(?!=|>))')
PROCEDURES = {'proc', 'func', 'method', 'iterator', 'converter'}
SECTIONS = {'type', 'var', 'let', 'const'}
CONTROL = {'if', 'elif', 'else', 'when', 'for', 'while', 'case', 'of', 'try',
           'except', 'finally', 'block', 'defer'}
IMPURE = re.compile(r'\b(?:var|let|const|for|while|return|yield|raise|break|continue|'
                    r'discard|defer|try|except|finally|block|new|inc|dec|echo|'
                    r'doAssert|assert|template|macro)\b')


def _event(kind, start, end, units=1, scope='nim-layout'):
    return dict(kind=kind, start=start, end=end, units=units, scope=scope)


def _issue(kind, offset, message):
    return dict(kind=kind, offset=offset, message=message)


def mask_nim(source):
    """Reuse only the explicit Nim mask, retaining its lexical review gates."""
    result = mask_future(source, 'nim')
    if source.startswith('\ufeff'):
        result['cleaned'] = ' ' + result['cleaned'][1:]
    return result


def _rows(text):
    rows = []
    offset = 0
    for value in text.splitlines(keepends=True):
        end = offset + len(value.rstrip('\r\n'))
        leading = len(value) - len(value.lstrip(' \t'))
        rows.append(dict(start=offset, end=end, code_start=offset + leading,
            indent=len(value[:leading].expandtabs(8)), code=text[offset+leading:end]))
        offset += len(value)
    return rows


def _top_positions(text, characters):
    """Punctuation outside argument, generic, literal, and pragma pairs."""
    stack = []
    found = []
    for index, char in enumerate(text):
        if char in '([{':
            stack.append(char)
        elif char in ')]}':
            if stack:
                stack.pop()
        elif not stack and char in characters:
            found.append(index)
    return found


def _body_end(rows, row_index, header_end, text):
    """A layout body ends at a sibling row, never at an imputed close."""
    row = rows[row_index]
    end = row['end']
    for candidate in rows[row_index+1:]:
        if candidate['end'] < header_end:
            continue
        if not candidate['code'].strip():
            continue
        if candidate['start'] < header_end:
            end = candidate['end']
            continue
        if candidate['indent'] <= row['indent']:
            break
        end = candidate['end']
    # Inline multiple statements are body work through the same physical row.
    return max(end, header_end)


def _assignment(text):
    return any(ASSIGN.match(text, index) for index in _top_positions(text, '='))


def _pure_expr(text, depth=0):
    """Prove a scalar composition, without hiding a mixed executable body.

    Only complete value conditionals/case arms recurse. Independent statement
    sequences, transfers, declarations, compiler directives, unknown layout,
    and multiline literal data are not collapsed into a function expression.
    """
    value = text.strip()
    if not value or depth > 16 or IMPURE.search(value) or '{.' in value:
        return False
    pairs, diagnostics = brace_pairs(value)
    if diagnostics or _assignment(value) or _top_positions(value, ';'):
        return False
    if value.startswith('(') and pairs.get(0) == len(value)-1:
        return _pure_expr(value[1:-1], depth+1)
    if any(value[start] in '[{' and '\n' in value[start:end+1]
           for start, end in pairs.items()):
        return False
    if re.match(r'(?:if|when)\b', value):
        # Colons/branch separators must be outside literal/formal groups.
        colons = _top_positions(value, ':')
        branches = list(re.finditer(r'\b(?:elif|else)\b', value))
        branches = [m for m in branches if not any(a < m.start() < b for a,b in pairs.items())]
        if not colons or not branches or branches[-1][0] != 'else':
            return False
        starts = [0] + [m.start() for m in branches]
        ends = starts[1:] + [len(value)]
        for start, end in zip(starts, ends):
            branch = value[start:end]
            own = _top_positions(branch, ':')
            if len(own) != 1:
                return False
            header, body = branch[:own[0]], branch[own[0]+1:]
            word = re.match(r'\s*(if|when|elif|else)\b', header)
            if not word or (word[1] != 'else' and not _pure_expr(header[word.end():], depth+1)):
                return False
            if not _pure_expr(body, depth+1):
                return False
        return True
    if re.match(r'case\b', value):
        rows = _rows(value)
        if not rows or not _pure_expr(rows[0]['code'][4:].strip().rstrip(':'), depth+1):
            return False
        arms = [r for r in rows[1:] if re.match(r'(?:of|else)\b', r['code'].strip())]
        if not arms or not arms[-1]['code'].lstrip().startswith('else'):
            return False
        for index, row in enumerate(arms):
            end = arms[index+1]['code_start'] if index+1 < len(arms) else len(value)
            arm = value[row['code_start']:end]
            own = _top_positions(arm, ':')
            if len(own) != 1 or not _pure_expr(arm[own[0]+1:], depth+1):
                return False
        return True
    if re.search(r'\b(?:if|when|elif|else|case|of)\b', value):
        # Nested proven conditional operands inside a pair are handled below.
        controls = list(re.finditer(r'\b(?:if|when|case)\b', value))
        for token in controls:
            enclosing = [(a,b) for a,b in pairs.items() if a < token.start() < b]
            if not enclosing:
                return False
            opening, closing = min(enclosing, key=lambda p:p[1]-p[0])
            if not _pure_expr(value[opening+1:closing], depth+1):
                return False
        if not controls:
            return False
    top_newlines = _top_positions(value, '\n')
    for point in top_newlines:
        before, after = value[:point].rstrip(), value[point+1:].lstrip()
        if after and not (re.search(r'(?:[+*/%&^,.-]|\b(?:and|or|xor|div|mod|shl|shr))$', before)
                          or re.match(r'(?:[+*/%&^.]|\b(?:and|or|xor|div|mod|shl|shr))', after)):
            return False
    return True


def _signature(text, start, pairs):
    """Find a procedure's real assignment boundary, not a formal default."""
    limit = min(len(text), start+4096)
    for position in _top_positions(text[start:limit], '='):
        end = start+position
        if not ASSIGN.match(text, end):
            continue
        signature = text[start:end]
        top = list(signature)
        for opening, closing in pairs.items():
            if start <= opening < closing < end:
                top[opening-start:closing-start+1] = replacement(signature[opening-start:closing-start+1])
        top = ''.join(top)
        if ';' in top or re.search(r'\b(?:let|var|const|return|for|while)\b', top):
            return None
        named = re.match(r'(proc|func|method|iterator|converter)\s+('+IDENT+r'|I+)\*?\b', signature)
        callback = re.match(r'(proc|func)\s*\(', signature)
        if not named and not callback:
            return None
        # Every formal/generic pair must finish before the body boundary.
        if any(o < end and c >= end for o,c in pairs.items() if o >= start):
            return None
        parameters = [o for o,c in pairs.items() if start <= o < c < end and text[o] == '(']
        return dict(start=start, end=end+1, named=bool(named), parameters=parameters)
    return None


def _contexts(text, pairs, regions, parameters):
    contexts = []
    stack = []
    for index, char in enumerate(text):
        active = [('L', r['kind'], r['start']) for r in regions if r['start'] <= index < r['end']]
        contexts.append(tuple(sorted(stack + active, key=lambda entry:entry[2])))
        if char in '([{':
            kind = 'parameters' if index in parameters else 'args' if char == '(' else 'data'
            stack.append((char, kind, index))
        elif char in ')]}' and stack and stack[-1][0] == {')':'(',']':'[','}':'{'}[char]:
            stack.pop()
    return contexts


def _callback_outer(text,start,body_end,pairs,rows,row_index):
    """Prove an enclosing single callback call/binding; keep other forms gated.

    Pure callback application is a composition. For an imperative callback the
    same prefix belongs to its one header, not to a fabricated extra call unit.
    """
    row=rows[row_index]
    prefix=text[row['code_start']:start]
    boundary=max(_top_positions(prefix,';'),default=-1)+1
    outer_start=row['code_start']+boundary
    prefix=text[outer_start:start].strip()
    enclosing=[(a,b) for a,b in pairs.items() if a < start < b]
    if not enclosing:
        if not prefix or re.fullmatch(r'(?:let|var|const)\s+'+IDENT+r'\s*=',prefix):
            return dict(start=outer_start,end=body_end,closing=None)
        return None
    opening,closing=min(enclosing,key=lambda pair:pair[1]-pair[0])
    if text[opening]!='(' or text[opening+1:start].strip():
        return None
    call_prefix=text[outer_start:opening].strip()
    if not re.fullmatch(r'(?:(?:let|var|const)\s+'+IDENT+r'\s*=\s*)?[\w.]+',call_prefix):
        return None
    if text[closing+1:row['end']].strip().strip(';'):
        return None
    return dict(start=outer_start,end=closing+1,closing=closing)


def classify_scopes(cleaned, language='nim', literal_spans=(), directives=()):
    if language not in LANGUAGES:
        raise ValueError('Only the explicit Nim grammar is proposed.')
    rows = _rows(cleaned)
    pairs, diagnostics = brace_pairs(cleaned)
    events, regions, parameters = [], [], set()
    parsed = list(cleaned)
    pragma_spans = []
    known_pragmas = {'push','pop','inline','noinline','thread','acyclic','gcsafe',
        'raises','checks','boundchecks','overflowchecks','optimization','warning',
        'hint','deadcodeelim','used','nimcall','cdecl','stdcall','fastcall','closure',
        'nosideeffect','noreturn','discardable','noinit','stacktrace','linetrace'}
    for directive in directives:
        start, end = directive['start'], directive['end']
        parsed[start:end] = replacement(cleaned[start:end])
        pragma_spans.append((start,end))
        events.append(_event('pragma',start,end,scope='nim-compiler-pragma'))
        settings = top_parts(cleaned[start+2:end-2])
        if any(not re.match(r'\s*('+ '|'.join(sorted(known_pragmas)) +r')\b', setting, re.I)
               for setting in settings):
            diagnostics.append(_issue('nim-codegen-or-unknown-pragma-review',start,
                'Unknown/custom/code-generating pragma requires an exact compiler/source-role review.'))
    syntax = ''.join(parsed)

    def covered(start):
        return any(event['start'] <= start < event['end'] for event in events)

    # Declaration sections open layout without an extra semantic header.
    sections = []
    for index, row in enumerate(rows):
        code = syntax[row['code_start']:row['end']].strip()
        if code in SECTIONS:
            events.append(_event('declaration-section',row['code_start'],row['end'],0,'nim-section'))
            end = _body_end(rows,index,row['end'],syntax)
            sections.append(dict(start=row['end'],end=end,kind='section-'+code))

    # Type headers and field owners are different from data constructors.
    for index, row in enumerate(rows):
        code = syntax[row['code_start']:row['end']]
        in_type_section = any(r['kind']=='section-type' and r['start'] <= row['code_start'] < r['end'] for r in sections)
        match = re.match(r'(?:type\s+)?('+IDENT+r')\*?\s*=\s*((?:ref\s+|ptr\s+)?object|enum|tuple)\b',code)
        if not match or (not code.startswith('type ') and not in_type_section):
            continue
        start = row['code_start']
        header_end = start+match.end()
        if match[2]=='tuple' and syntax[header_end:].lstrip().startswith('['):
            opening = header_end+len(syntax[header_end:])-len(syntax[header_end:].lstrip())
            closing = pairs.get(opening)
            if closing is None:
                diagnostics.append(_issue('nim-tuple-type-field-review', opening, 'Named tuple type has no proved closing bracket.'))
                continue
            events.append(_event('header',start,opening+1,scope='nim-tuple-type'))
            fields = _tuple_fields(syntax,opening+1,closing)
            if fields is None:
                diagnostics.append(_issue('nim-tuple-type-field-review',opening,'Tuple field groups require explicit typed-name ownership.'))
            else:
                events.extend(_event('field',a,b,n,'nim-tuple-field') for a,b,n in fields)
                cursor=opening+1
                for left,right,units in fields:
                    if cursor<left:
                        events.append(_event('field-separator',cursor,left,0,'nim-tuple-type'))
                    cursor=right
                if cursor<closing:
                    events.append(_event('field-separator',cursor,closing,0,'nim-tuple-type'))
            events.append(_event('data-close',closing,closing+1,0,'nim-tuple-type'))
        else:
            events.append(_event('header',start,header_end,scope='nim-type'))
            end = _body_end(rows,index,header_end,syntax)
            regions.append(dict(start=header_end,end=end,kind='type'))
            if syntax[header_end:row['end']].strip():
                diagnostics.append(_issue('nim-type-header-suffix-review',header_end,'Object/enum header suffix needs explicit field/base ownership.'))

    # Named procedures may be expression compositions; explicit callbacks keep
    # their written header and body even for a single expression.
    for match in re.finditer(r'\b(?:proc|func|method|iterator|converter)\b',syntax):
        start = match.start()
        if covered(start):
            continue
        signature = _signature(syntax,start,pairs)
        if not signature:
            diagnostics.append(_issue('nim-procedure-header-review',start,'Procedure declaration/prototype has no proved body boundary.'))
            continue
        row_index = next((i for i,r in enumerate(rows) if r['start'] <= start <= r['end']),None)
        if row_index is None:
            continue
        end = _body_end(rows,row_index,signature['end'],syntax)
        callback_owner=None
        if not signature['named']:
            enclosing = [(a,b) for a,b in pairs.items() if a < start < b]
            if enclosing:
                end = min(end,min(enclosing,key=lambda pair:pair[1]-pair[0])[1])
            callback_owner=_callback_outer(syntax,start,end,pairs,rows,row_index)
            if callback_owner is None or _top_positions(syntax[signature['end']:end],','):
                diagnostics.append(_issue('nim-callback-enclosing-owner-review',start,
                    'Callback following arguments/composition or data-container ownership is not proved.'))
                callback_owner=None
        body_start = signature['end']
        body = syntax[body_start:end]
        parameters.update(signature['parameters'])
        if (signature['named'] or callback_owner) and _pure_expr(body):
            events.append(_event('expression-function',callback_owner['start'] if callback_owner else start,
                callback_owner['end'] if callback_owner else end,1,'value-expression'))
        else:
            events.append(_event('header',callback_owner['start'] if callback_owner else start,
                body_start,scope='nim-named-function' if signature['named'] else 'nim-callback'))
            if callback_owner and callback_owner['closing'] is not None:
                events.append(_event('data-close',callback_owner['closing'],callback_owner['end'],0,'nim-callback-call'))
            if body_start < end:
                regions.append(dict(start=body_start,end=end,kind='code'))

    # Contexts establish where a control keyword is an operand rather than an
    # independent statement, and reset surrounding data for callback bodies.
    contexts = _contexts(syntax,pairs,regions,parameters)
    for index, row in enumerate(rows):
        row_statement_control = False
        for match in re.finditer(r'\b(?:if|elif|else|when|for|while|case|of|try|except|finally|block|defer)\b',syntax[row['code_start']:row['end']]):
            start = row['code_start']+match.start()
            if covered(start):
                continue
            boundary=max([row['code_start'],*(e['end'] for e in events
                if row['code_start'] <= e['end'] <= start)])
            prior = syntax[boundary:start]
            # A receiver name, literal/formal group, or ordinary argument does
            # not become a keyword block. Value controls need a scalar proof.
            if start and syntax[start-1] == '.':
                continue
            if structural(contexts[start]):
                if match[0] in {'if','when','case'}:
                    enclosing=[(a,b) for a,b in pairs.items() if a < start < b]
                    if enclosing:
                        opening,closing=min(enclosing,key=lambda pair:pair[1]-pair[0])
                        if not _pure_expr(syntax[start:closing]):
                            diagnostics.append(_issue('nim-value-control-review',start,
                                'Conditional inside an argument/data group is not a proved scalar operand.'))
                continue
            own_prior = prior.rsplit(';',1)[-1]
            is_value = bool(ASSIGN.search(own_prior)) or bool(own_prior.strip() and not own_prior.rstrip().endswith(':'))
            if match[0] in {'elif','else','of','except','finally'} and row_statement_control:
                is_value = False
            if is_value and match[0] in {'if','when','case','else','elif'}:
                if match[0] in {'if','when','case'} and not _pure_expr(syntax[start:row['end']]):
                    diagnostics.append(_issue('nim-value-control-review',start,'Mixed/continued value control requires source-specific operand ownership.'))
                continue
            suffix = syntax[start:row['end']]
            colons = _top_positions(suffix,':')
            if match[0]=='case' and not colons:
                end = row['end']
            elif colons:
                end = start+colons[0]+1
            else:
                diagnostics.append(_issue('nim-control-header-review',start,'Control header has no proved colon/layout boundary.'))
                continue
            events.append(_event('branch' if match[0] in {'elif','else','of','except','finally'} else 'header',start,end,scope='nim-control'))
            row_statement_control = True
            body_end = _body_end(rows,index,end,syntax)
            if end < body_end:
                regions.append(dict(start=end,end=body_end,kind='code'))
    for match in re.finditer(r'\b(?:macro|template|static|asm|bind|mixin|include|export|from)\b',syntax):
        if not covered(match.start()):
            diagnostics.append(_issue('nim-metaprogram-or-declaration-review',match.start(),
                'Metaprogram, foreign body, or extended module declaration needs its own source role.'))
    for opening,closing in pairs.items():
        if syntax[opening]=='(' and opening not in parameters:
            body=syntax[opening+1:closing]
            unowned_semis=[opening+1+point for point in _top_positions(body,';')
                if not any(opening < r['start'] <= opening+1+point < r['end']
                           for r in regions if r['kind']=='code')]
            if unowned_semis and re.search(r'\b(?:var|let|const|return|for|while)\b',body):
                diagnostics.append(_issue('nim-executable-expression-group-review',opening,
                    'Statement-list expression needs explicit local/header/data ownership.'))
    for row in rows:
        code = syntax[row['code_start']:row['end']].strip()
        if re.match(r'(?:import\b)',code) and not re.fullmatch(r'import\s+[\w/.,\s\[\]]+',code):
            diagnostics.append(_issue('nim-import-form-review',row['code_start'],'Module alias/exclusion syntax requires declaration review.'))
        if re.search(r'\b(?:varargs|untyped|typedesc)\b',code):
            diagnostics.append(_issue('nim-generic-metaprogram-type-review',row['code_start'],'Generic/metaprogram formal constraints require a qualified header rule.'))
    # Pragmas are independent written units, including inside a named header
    # or pure procedure. Split the enclosing owner around the pragma span so
    # header/composition arithmetic does not count the pragma twice.
    split_events=[]
    for event in events:
        if event['kind']=='pragma':
            split_events.append(event);continue
        parts=[(event['start'],event['end'])]
        for opening,closing in pragma_spans:
            next_parts=[]
            for left,right in parts:
                if right<=opening or closing<=left: next_parts.append((left,right))
                else:
                    if left<opening: next_parts.append((left,opening))
                    if closing<right: next_parts.append((closing,right))
            parts=next_parts
        for index,(left,right) in enumerate(parts):
            if syntax[left:right].strip():
                split_events.append(dict(event,start=left,end=right,units=event['units'] if index==0 else 0))
            else:
                split_events.append(dict(event,start=left,end=right,units=0))
    return dict(contexts=_contexts(syntax,pairs,regions,parameters),events=split_events,
        diagnostics=diagnostics,owners=regions,executable_regions=[r for r in regions if r['kind']=='code'])


def _tuple_fields(text,start,end):
    """Named tuple-type fields, not tuple values or generic type arguments."""
    chunks = []
    stack = []
    left = start
    for index in range(start,end):
        char = text[index]
        if char in '([{':
            stack.append(char)
        elif char in ')]}' and stack:
            stack.pop()
        elif char in ',;' and not stack:
            chunks.append((left,index))
            left=index+1
    chunks.append((left,end))
    fields = []
    pending = []
    for left,right in chunks:
        while left<right and text[left].isspace(): left+=1
        while right>left and text[right-1].isspace(): right-=1
        value=text[left:right]
        if re.fullmatch(IDENT+r'\*?',value):
            pending.append((left,right));continue
        match=re.fullmatch(r'('+IDENT+r'\*?)\s*:\s*(.+)',value,re.S)
        if not match or not match[2].strip(): return None
        count=len(pending)+1
        fields.append((pending[0][0] if pending else left,right,count))
        pending=[]
    return None if pending else fields


def binding_counter(*,text,original,context,assignments):
    code=text.strip()
    code=re.sub(r'^\s*(?:var|let|const)\s+','',code)
    # Explicit parenthesized destructuring names share one tuple producer.
    if re.match(r'^\(\s*'+IDENT+r'(?:\s*,\s*'+IDENT+r')+\s*\)\s*=',code):
        return dict(units=max(1,len(assignments)),declaration_bindings=1,reason='explicit-tuple-binding-producer')
    declaration=re.match(r'^('+IDENT+r'\*?(?:\s*,\s*'+IDENT+r'\*?)*)\s*(?::(?!:)|=(?!=))',code)
    if declaration:
        names=top_parts(declaration[1])
        if len(names)>1:
            return dict(units=max(len(names),len(assignments)),declaration_bindings=len(names),
                reason='independent-Nim-declaration-bindings')
    if any(kind=='type' for _,kind,_ in context) and not re.fullmatch(r'\s*'+IDENT+r'\*?\s*:\s*.+',code):
        return dict(diagnostics=[dict(kind='nim-type-field-form-review',
            message='Object field/variant/enum role requires explicit type-owner review.')])
    return {}


def normalize(source,language='nim'):
    if language not in LANGUAGES:
        raise ValueError('Only the explicit Nim grammar is proposed.')
    masked=mask_nim(source)
    parsed=classify_scopes(masked['cleaned'],language,
        literal_spans=masked.get('spans',()),directives=masked.get('directives',()))
    result=count_owned(source,masked['cleaned'],parsed['contexts'],parsed['events'],
        [*masked['diagnostics'],*parsed['diagnostics']],language,
        binding_counter=binding_counter,executable_regions=parsed['executable_regions'],
        named_equals=True,assignment_pattern=ASSIGN)
    result['proposal_version']=PROPOSAL_VERSION
    result['eligible_for_chart']=False
    result['pragma_evidence']=masked.get('directives',[])
    return result
