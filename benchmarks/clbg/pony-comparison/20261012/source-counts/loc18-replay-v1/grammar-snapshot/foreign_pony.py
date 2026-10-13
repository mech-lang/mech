"""Unhooked Pony-owned canonical source units; no chart/source admission.

Type/member scopes use Pony layout. Keyword controls own only written ends.
Capabilities, partial markers, apostrophe identifiers, tuple/type/formal groups,
and where arguments are structural. Imports and qualification must hold the
common benchmark timing mutex. No benchmark name selects an accounting path.
"""
import re

from foreign_core import brace_pairs, count_owned, replacement, structural, top_parts
from foreign_future_mask import mask_future


LANGUAGES={'pony'}
PROPOSAL_VERSION='foreign-pony-unhooked-20261012-v1'
IDENT=r"[A-Za-z_][A-Za-z_0-9']*"
CAPABILITIES={'iso','trn','ref','val','box','tag'}
TYPE_WORDS={'actor','class','primitive','interface','trait','struct','type'}
MEMBER_WORDS={'new','fun','be'}
BLOCK_WORDS={'if','ifdef','iftype','while','repeat','for','try','match','recover','with'}
TOKEN=re.compile(r'=>|\.>|==|!=|<=|>=|<<|>>|'+IDENT+r'|[^\s]')
ASSIGN=re.compile(r'(?<![=!<>~:])=(?!=|>)')
IMPURE=re.compile(r'\b(?:let|var|embed|return|break|continue|error|for|while|repeat|'
                  r'with|ifdef|iftype|compile_error)\b')


def _event(kind,start,end,units=1,scope='pony-owned'):
    return dict(kind=kind,start=start,end=end,units=units,scope=scope)


def _issue(kind,offset,message):
    return dict(kind=kind,offset=offset,message=message)


def mask_pony(source):
    result=mask_future(source,'pony')
    if source.startswith('\ufeff'):
        result['cleaned']=' '+result['cleaned'][1:]
    return result


def _rows(text):
    rows=[];offset=0
    for value in text.splitlines(keepends=True):
        end=offset+len(value.rstrip('\r\n'))
        leading=len(value)-len(value.lstrip(' \t'))
        rows.append(dict(start=offset,end=end,code_start=offset+leading,
            indent=len(value[:leading].expandtabs(8)),code=text[offset+leading:end]))
        offset+=len(value)
    return rows


def _tokens(text):
    return [dict(word=m[0],start=m.start(),end=m.end()) for m in TOKEN.finditer(text)]


def _top_positions(text,characters):
    stack=[];result=[]
    for index,char in enumerate(text):
        if char in '([{': stack.append(char)
        elif char in ')]}' and stack: stack.pop()
        elif not stack and char in characters: result.append(index)
    return result


def _layout_end(rows,row_index,header_end):
    row=rows[row_index];end=max(row['end'],header_end)
    for later in rows[row_index+1:]:
        if later['end']<header_end or not later['code'].strip():continue
        if later['start']<header_end:end=later['end'];continue
        if later['indent']<=row['indent']:break
        end=later['end']
    return end


def _block_records(text):
    """Explicit keyword scopes, independently of member/type layout.

    Receiver members never become keywords. Delimiter depth distinguishes a
    match arm from a tuple/type-union bar. Unsupported guards remain gated.
    """
    tokens=_tokens(text);stack=[];delimiters=[];records=[];diagnostics=[]
    for token in tokens:
        word=token['word'];start=token['start']
        if word in {'(','[','{'}:delimiters.append((word,start));continue
        if word in {')',']','}'}:
            if delimiters:delimiters.pop()
            continue
        receiver=start>0 and text[start-1]=='.'
        if receiver:continue
        if word in BLOCK_WORDS:
            record=dict(word=word,start=start,word_end=token['end'],header_end=token['end'],
                delim_depth=len(delimiters),parent=stack[-1]['start'] if stack else None,
                branches=[],arms=[],end=None,close_start=None)
            records.append(record);stack.append(record)
            if word in {'recover'}:
                after=next((t for t in tokens if t['start']>=token['end']),None)
                if after and after['word'] in CAPABILITIES:record['header_end']=after['end']
            elif word=='match':
                newline=text.find('\n',token['end'])
                record['header_end']=len(text) if newline<0 else newline
        elif word in {'then','do'} and stack:
            own=stack[-1]
            expected='then' if own['word'] in {'if','ifdef','iftype'} else 'do' if own['word'] in {'for','while','with'} else None
            if expected==word and not own.get('body_started'):
                own['header_end']=token['end'];own['body_started']=True
            elif word=='then' and own['branches'] and own['branches'][-1]['word']=='elseif':
                own['branches'][-1]['header_end']=token['end']
        elif word in {'elseif','else','until'}:
            if not stack:
                diagnostics.append(_issue('pony-unmatched-keyword-branch',start,'Written branch has no proved explicit control owner.'));continue
            own=stack[-1]
            own['branches'].append(dict(word=word,start=start,header_end=token['end']))
        elif word=='|' and stack and stack[-1]['word']=='match' and len(delimiters)==stack[-1]['delim_depth']:
            own=stack[-1]
            if not own['arms']:own['header_end']=start
            own['arms'].append(dict(start=start,header_end=None))
        elif word=='=>' and stack and stack[-1]['word']=='match' and stack[-1]['arms']:
            arm=stack[-1]['arms'][-1]
            if arm['header_end'] is None:arm['header_end']=token['end']
        elif word=='end':
            if not stack:
                diagnostics.append(_issue('pony-unmatched-keyword-end',start,'Written end has no proved explicit control owner.'));continue
            own=stack.pop();own['close_start']=start;own['end']=token['end']
    for own in stack:
        diagnostics.append(_issue('pony-unclosed-keyword-scope',own['start'],'Explicit keyword control has no written end.'))
    for own in records:
        if own['word'] in {'if','while','for'} and not own.get('body_started'):
            diagnostics.append(_issue('pony-control-header-review',own['start'],'Control has no proved then/do header boundary.'))
        if own['word'] in {'ifdef','iftype','with','repeat'}:
            diagnostics.append(_issue('pony-extended-control-review',own['start'],'Conditional-type/resource/repeat grammar requires exact ownership qualification.'))
        if own['word']=='match' and any(a['header_end'] is None for a in own['arms']):
            diagnostics.append(_issue('pony-match-arm-header-review',own['start'],'Match arm has no proved pattern/body arrow.'))
    return records,diagnostics


def _slices(record):
    branches=sorted(record['branches']+record['arms'],key=lambda item:item['start'])
    first=branches[0]['start'] if branches else record['close_start']
    result=[(record['header_end'],first)]
    for index,branch in enumerate(branches):
        end=branches[index+1]['start'] if index+1<len(branches) else record['close_start']
        result.append((branch['header_end'],end))
    return result


def _pure_value(text,depth=0):
    """Prove expression composition while preserving literal/data rows."""
    value=text.strip()
    if not value or depth>16:return False
    pairs,diagnostics=brace_pairs(value)
    if diagnostics:return False
    if value.startswith('(') and pairs.get(0)==len(value)-1:
        return _pure_value(value[1:-1],depth+1)
    records,issues=_block_records(value)
    if issues:return False
    first=next((r for r in records if r['start']==0 and r['end']==len(value)),None)
    if first:
        if first['word'] not in {'if','match','recover','try'}:return False
        if first['word'] in {'if','try','match'} and not any(b['word']=='else' for b in first['branches']):return False
        if first['word']=='if':
            condition=value[first['word_end']:first['header_end']].rstrip()
            condition=re.sub(r'\s*then\s*$','',condition)
            if not _pure_value(condition,depth+1):return False
            for branch in first['branches']:
                if branch['word']=='elseif':
                    cond=value[branch['start']+6:branch['header_end']]
                    cond=re.sub(r'\s*then\s*$','',cond)
                    if not _pure_value(cond,depth+1):return False
        if first['word']=='match':
            if not first['arms']:return False
            if not _pure_value(value[first['word_end']:first['header_end']],depth+1):return False
            for arm in first['arms']:
                pattern=value[arm['start']+1:arm['header_end']-2]
                if re.search(r'\b(?:if|where|error|return)\b',pattern) or ASSIGN.search(pattern):return False
        bodies=_slices(first)
        # A match header has no initial body before its first pattern.
        if first['word']=='match':bodies=bodies[1:]
        return bool(bodies) and all(a is not None and _pure_value(value[a:b],depth+1) for a,b in bodies)
    if IMPURE.search(value) or _top_positions(value,';'):return False
    for match in ASSIGN.finditer(value):
        # Assignment is work even when parenthesized. Only a proved call's
        # `where` argument list is structural named-argument syntax.
        enclosing=[(a,b) for a,b in pairs.items() if a<match.start()<b and value[a]=='(']
        if not enclosing:return False
        opening,closing=min(enclosing,key=lambda pair:pair[1]-pair[0])
        if not re.search(r'\bwhere\b',value[opening+1:match.start()]):return False
    for record in records:
        if record['end'] is None:return False
        if record['parent'] is None and not _pure_value(value[record['start']:record['end']],depth+1):return False
    if any(value[a] in '[{' and '\n' in value[a:b+1] for a,b in pairs.items()):return False
    for point in _top_positions(value,'\n'):
        before,after=value[:point].rstrip(),value[point+1:].lstrip()
        if after and not (re.search(r'(?:[+*/%&^,.>-]|\b(?:and|or|xor))$',before)
                          or re.match(r'(?:[+*/%&^.]|\b(?:and|or|xor))',after)):
            return False
    return True


def _binding_prefix(prefix):
    """Prove a receiving binding/ordinary lvalue chain before a value block."""
    pieces=ASSIGN.split(prefix.strip())
    if len(pieces)<2 or pieces[-1].strip():return False
    first=pieces[0].strip()
    declared=re.fullmatch(r'(?:let|var)\s+'+IDENT+r'(?:\s*:\s*[^=;{}]+)?',first)
    lvalue=re.fullmatch(IDENT+r'(?:\.[\w\']+|\([^;{}]*\)\??)*',first)
    if not declared and not lvalue:return False
    return all(re.fullmatch(IDENT+r'(?:\.[\w\']+|\([^;{}]*\)\??)*',piece.strip())
               for piece in pieces[1:-1])


def _call_operand(text,start,end,pairs):
    """Prove a single ordinary argument's containing call and close suffix."""
    enclosing=[(a,b) for a,b in pairs.items() if a<start<end<=b and text[a]=='(']
    if not enclosing:return None
    opening,closing=min(enclosing,key=lambda pair:pair[1]-pair[0])
    before=text[:opening].rstrip()
    if not re.search(IDENT+r'(?:\[[^\n]+\])?\s*$',before):return None
    if text[opening+1:start].strip():return None
    if text[end:closing].strip():return None
    return dict(opening=opening,closing=closing)


def _contexts(text,regions,parameters,header_regions):
    contexts=[];stack=[]
    for index,char in enumerate(text):
        layout=[('L',r['kind'],r['start']) for r in regions if r['start']<=index<r['end']]
        headers=[('H','control',r['start']) for r in header_regions if r['start']<=index<r['end']]
        contexts.append(tuple(sorted(stack+layout+headers,key=lambda item:item[2])))
        if char in '([{':
            stack.append((char,'parameters' if index in parameters else 'args' if char=='(' else 'data',index))
        elif char in ')]}' and stack and stack[-1][0]=={')':'(',']':'[','}':'{'}[char]:stack.pop()
    return contexts


def classify_scopes(cleaned,language='pony',literal_spans=(),directives=()):
    if language not in LANGUAGES:raise ValueError('Only the explicit Pony grammar is proposed.')
    rows=_rows(cleaned);pairs,diagnostics=brace_pairs(cleaned)
    records,block_issues=_block_records(cleaned);diagnostics.extend(block_issues)
    events=[];regions=[];parameters=set();headers=[]
    def covered(start):return any(e['start']<=start<e['end'] for e in events)
    # Type and member boundaries are layout, never synthetic written end.
    for row_index,row in enumerate(rows):
        code=row['code'].strip()
        type_match=re.match(r'(actor|class|primitive|interface|trait|struct|type)\b',code)
        if type_match:
            if not re.match(r'(?:actor|class|primitive|interface|trait|struct|type)\s+(?:(?:iso|trn|ref|val|box|tag)\s+)?'+IDENT+r'\b',code):
                diagnostics.append(_issue('pony-type-header-review',row['code_start'],'Type header name/capability is not proved.'))
            events.append(_event('header',row['code_start'],row['end'],scope='pony-type'))
            end=_layout_end(rows,row_index,row['end'])
            regions.append(dict(start=row['end'],end=end,kind='type'))
            if type_match[1] in {'type','interface','trait','struct'}:
                diagnostics.append(_issue('pony-extended-type-review',row['code_start'],'Alias/interface/trait/struct members need dedicated source fixtures.'))
        member=re.match(r'(new|fun|be)\b',code)
        if not member:continue
        start=row['code_start'];limit=min(len(cleaned),start+4096)
        arrows=[m for m in re.finditer(r'=>',cleaned[start:limit])
            if not any(a<start+m.start()<b for a,b in pairs.items())]
        if not arrows:
            diagnostics.append(_issue('pony-member-header-review',start,'Member has no proved body arrow; prototype/abstract ownership requires review.'));continue
        end_header=start+arrows[0].end()
        signature=cleaned[start:end_header-2]
        if not re.match(r'(?:new|fun|be)\s+(?:(?:iso|trn|ref|val|box|tag)\s+)?'+IDENT+r'(?:\[[^\n]*\])?\s*\(',signature):
            diagnostics.append(_issue('pony-member-header-review',start,'Member capability/name/generic/formal signature is not proved.'))
        if re.search(r'[;{}]',signature):
            diagnostics.append(_issue('pony-member-header-review',start,'Member signature contains unqualified capture/body punctuation.'))
        parameters.update(a for a,b in pairs.items() if start<=a<b<end_header and cleaned[a]=='(')
        end=_layout_end(rows,row_index,end_header)
        body=cleaned[end_header:end]
        if _pure_value(body):events.append(_event('expression-member',start,end,1,'value-expression'))
        else:
            events.append(_event('header',start,end_header,scope='pony-member'))
            if end_header<end:regions.append(dict(start=end_header,end=end,kind='code'))
    base_contexts=_contexts(cleaned,regions,parameters,headers)
    # A pure control operand owns no extra unit beyond its enclosing binding
    # or expression. A standalone pure recover itself is one composition.
    for record in records:
        start=record['start'];end=record['end']
        if end is None or covered(start):continue
        body=cleaned[start:end]
        row_start=cleaned.rfind('\n',0,start)+1
        boundary=max([row_start,*(e['end'] for e in events if row_start<=e['end']<=start)])
        prefix=cleaned[boundary:start].strip()
        embedded=bool(prefix and not prefix.endswith(';')) or structural(base_contexts[start])
        if _pure_value(body):
            events.append(_event('value-control',start,end,0 if embedded else 1,'value-expression'))
            continue
        if record['word']=='recover' and embedded:
            containing_call=_call_operand(cleaned,start,end,pairs)
            if not _binding_prefix(prefix) and not containing_call:
                diagnostics.append(_issue('pony-imperative-recover-enclosing-review',start,
                    'Imperative recover has no proved outer binding or single ordinary call argument.'))
            if containing_call:
                events.append(_event('structural-suffix',end,containing_call['closing']+1,0,'pony-value-argument'))
        if embedded and record['word'] in {'if','match','try'}:
            diagnostics.append(_issue('pony-mixed-value-control-review',start,
                'Mixed/error-bearing control operand must expose its independent body work.'))
        if record['word']=='recover' and re.search(r'\berror\b',body):
            diagnostics.append(_issue('pony-recover-error-review',start,'Error-bearing recover requires exact capability/value ownership review.'))
        header_end=record['header_end']
        if header_end>start:
            events.append(_event('header',start,header_end,scope='pony-'+record['word']))
            headers.append(dict(start=start,end=header_end))
        for branch in record['branches']:
            events.append(_event('branch',branch['start'],branch['header_end'],scope='pony-branch'))
            headers.append(dict(start=branch['start'],end=branch['header_end']))
        for arm in record['arms']:
            if arm['header_end'] is not None:
                events.append(_event('branch',arm['start'],arm['header_end'],scope='pony-match-pattern'))
                headers.append(dict(start=arm['start'],end=arm['header_end']))
        events.append(_event('close',record['close_start'],end,scope='pony-keyword-end'))
        for left,right in _slices(record):
            if left is not None and left<right:regions.append(dict(start=left,end=right,kind='code'))
    # One bounded explicit anonymous-lambda signature. Other captures and
    # imperative composition owners remain exact source-review gates.
    for opening,closing in pairs.items():
        if cleaned[opening]=='(':
            inner=cleaned[opening+1:closing]
            unowned_semis=[opening+1+point for point in _top_positions(inner,';')
                if not any(r['start'] <= opening+1+point < r['end']
                           for r in records if r['end'] is not None)]
            if unowned_semis and (IMPURE.search(inner) or ASSIGN.search(inner)):
                diagnostics.append(_issue('pony-executable-expression-group-review',opening,
                    'Statement-list expression needs explicit independent local/action ownership.'))
        if cleaned[opening]!='{' or covered(opening):continue
        body=cleaned[opening+1:closing]
        signature=re.match(r'\s*\([^{}]*\)\s*(?::\s*[^{}=]+)?\s*=>',body)
        if signature and _pure_value(body[signature.end():]):
            context=base_contexts[opening]
            prefix=cleaned[cleaned.rfind('\n',0,opening)+1:opening]
            embedded=structural(context) or bool(prefix.strip())
            events=[e for e in events if not (opening<e['start'] and e['end']<closing
                and e.get('scope')=='value-expression')]
            events.append(_event('expression-lambda',opening,closing+1,0 if embedded else 1,'value-expression'))
        elif signature:
            end_header=opening+1+signature.end()
            diagnostics.append(_issue('pony-imperative-lambda-composition-review',opening,
                'Imperative lambda captures/enclosing expression must retain explicit header/body/close ownership.'))
            events.append(_event('header',opening,end_header,scope='pony-lambda'))
            events.append(_event('close',closing,closing+1,scope='pony-lambda'))
            regions.append(dict(start=end_header,end=closing,kind='code'))
        else:
            diagnostics.append(_issue('pony-brace-owner-review',opening,'Brace literal/capture/lambda ownership is not proved.'))
    for match in re.finditer(r'\\[^\\\n]+\\',cleaned):
        diagnostics.append(_issue('pony-compiler-annotation-review',match.start(),
            'Written compiler annotation requires semantic pragma ownership qualification.'))
    for row in rows:
        code=row['code'].strip()
        if code.startswith('use @'):
            if re.fullmatch(r'use\s+@'+IDENT+r'\s*\[[^\n]+\]\s*\([^\n]*\)\s*\??',code):
                events.append(_event('ffi-declaration',row['code_start'],row['end'],scope='pony-ffi'))
            else:diagnostics.append(_issue('pony-ffi-declaration-review',row['code_start'],'FFI generic/formal/source declaration role is not proved.'))
        if re.search(r'\bwhere\b',code) and not any(a<=row['code_start']+code.find('where')<b for a,b in pairs.items() if cleaned[a]=='('):
            diagnostics.append(_issue('pony-where-argument-owner-review',row['code_start'],'Unparenthesized named arguments need a proved containing call.'))
    return dict(contexts=_contexts(cleaned,regions,parameters,headers),events=events,
        diagnostics=diagnostics,owners=regions,executable_regions=[r for r in regions if r['kind']=='code'])


def binding_counter(*,text,original,context,assignments):
    code=text.strip()
    if re.match(r'^\(\s*(?:let|var)\s+'+IDENT+r'.*\)\s*=',code):
        return dict(units=max(1,len(assignments)),declaration_bindings=1,reason='explicit-Pony-tuple-pattern-producer')
    group=re.match(r'^(?:let|var|embed)\s+('+IDENT+r'(?:\s*,\s*'+IDENT+r')+)\s*:',code)
    if group:
        size=len(top_parts(group[1]))
        return dict(units=max(size,len(assignments)),declaration_bindings=size,
            reason='independent-Pony-declared-bindings',diagnostics=[dict(kind='pony-grouped-declaration-grammar-review',
                message='Grouped field/local syntax requires exact Pony grammar qualification.')])
    return {}


def normalize(source,language='pony'):
    if language not in LANGUAGES:raise ValueError('Only the explicit Pony grammar is proposed.')
    masked=mask_pony(source)
    parsed=classify_scopes(masked['cleaned'],language,literal_spans=masked.get('spans',()),
        directives=masked.get('directives',()))
    result=count_owned(source,masked['cleaned'],parsed['contexts'],parsed['events'],
        [*masked['diagnostics'],*parsed['diagnostics']],language,binding_counter=binding_counter,
        executable_regions=parsed['executable_regions'],named_equals=True,assignment_pattern=ASSIGN)
    result['proposal_version']=PROPOSAL_VERSION
    result['eligible_for_chart']=False
    return result
