"""Unhooked canonical Zig/Odin source-count proposal; no chart collection.

Owners are recognized from language syntax, never benchmark names or inputs.
Unknown value/control/header forms remain explicit review diagnostics.
"""
import re

from foreign_core import (ASSIGN, EXECUTABLE, brace_pairs, count_owned,
                          mask_foreign, replacement, top_parts)


def mask_zig(source):
    """Add exact raw literal evidence to the existing offset-preserving mask.

    Mask confirmation prevents quotes inside comments becoming literals.
    Neither S identifiers nor whitespace alone are treated as literal proof.
    Quoted identifiers retain identifier grammar, not string-data ownership.
    """
    masked=mask_foreign(source,'zig');cleaned=masked['cleaned'];literals=[];cursor=0
    while cursor<len(source):
        if source[cursor] in '\"\'' and cleaned[cursor]=='S':
            start=cursor;quote=source[cursor];quoted_identifier=bool(cursor and source[cursor-1]=='@');cursor+=1;complete=False
            while cursor<len(source):
                if source[cursor]=='\\':cursor=min(len(source),cursor+2)
                elif source[cursor]==quote:cursor+=1;complete=True;break
                else:cursor+=1
            if not complete:
                masked['diagnostics'].append(dict(kind='zig-unclosed-literal-review',offset=start))
            elif not all(char.isspace() or char=='S' for char in cleaned[start:cursor]):
                masked['diagnostics'].append(dict(kind='zig-literal-span-review',offset=start))
            elif not quoted_identifier:literals.append(dict(start=start,end=cursor,role='quoted-literal'))
            continue
        line_start=_row_start(source,cursor)
        if source.startswith('\\\\',cursor) and not source[line_start:cursor].strip() and cleaned[cursor]=='S':
            start=cursor;end=source.find('\n',cursor);end=len(source) if end<0 else end
            while end<len(source):
                next_start=end+1;next_cursor=next_start
                while next_cursor<len(source) and source[next_cursor] in ' \t':next_cursor+=1
                if not source.startswith('\\\\',next_cursor) or cleaned[next_cursor]!='S':break
                next_end=source.find('\n',next_cursor);end=len(source) if next_end<0 else next_end
            literals.append(dict(start=start,end=end,role='multiline-literal'));cursor=end;continue
        cursor+=1
    masked['literals']=literals
    return masked


def mask_odin(source):
    """Odin programming syntax, not Pygments' biomedical ODIN alias."""
    chars=list(source);issues=[];index=0
    if source.startswith('\ufeff'):chars[0]=' '
    while index < len(source):
        if source.startswith('//',index):
            start=index;end=source.find('\n',index);index=len(source) if end<0 else end
            chars[start:index]=replacement(source[start:index])
        elif source.startswith('/*',index):
            start=index;index+=2;depth=1
            while index<len(source) and depth:
                if source.startswith('/*',index):depth+=1;index+=2
                elif source.startswith('*/',index):depth-=1;index+=2
                else:index+=1
            chars[start:index]=replacement(source[start:index])
            if depth:issues.append(dict(kind='odin-unterminated-comment',offset=start))
        elif source[index] in '\"\'`':
            start=index;quote=source[index];index+=1;closed=False
            while index<len(source):
                if quote!='`' and source[index]=='\\':index=min(index+2,len(source))
                elif source[index]==quote:index+=1;closed=True;break
                else:index+=1
            chars[start:index]=replacement(source[start:index],literal=True)
            if not closed:issues.append(dict(kind='odin-unterminated-literal',offset=start))
        else:index+=1
    return dict(cleaned=''.join(chars),diagnostics=issues,directives=[])


def _row_start(text,index):
    return text.rfind('\n',0,index)+1


def _trim(text,start,end):
    while start<end and text[start].isspace():start+=1
    return start


def _header_row_start(text,index):
    start=_row_start(text,index)
    # Only already completed braces/semicolons before this named declaration
    # are considered; formal/loop punctuation belongs to its own later span.
    prior=text[start:index]
    last=max(prior.rfind('{'),prior.rfind('}'),prior.rfind(';'))
    return _trim(text,start+last+1,index) if last>=0 else start


def _signature(text,opening,language,pairs):
    window=max(0,opening-1024);prefix=text[window:opening]
    expression=r'\bfn\s+[A-Za-z_]\w*\s*\(' if language=='zig' else r'\b[A-Za-z_]\w*\s*::\s*proc\s*\('
    for match in reversed(list(re.finditer(expression,prefix))):
        start=window+match.start();parameter=window+match.end()-1
        end=pairs.get(parameter)
        if end is None or end>=opening:continue
        suffix=text[end+1:opening].strip()
        if re.search(r'[{};]|\b(?:return|for|if|switch|defer)\b',suffix):continue
        if language=='odin' and suffix and not suffix.startswith(('->','#')):continue
        if language=='zig' and suffix:
            if not re.fullmatch(r"[\w\s@.!?*&\[\]():,'<>]+",suffix):continue
            # Explicit type-expression builtins have the same signature
            # owner as named return types. Unknown builtins stay gated.
            if any(name not in {'TypeOf','Vector','This'} for name in re.findall(r'@([A-Za-z_]\w*)',suffix)):continue
        return _header_row_start(text,start),parameter,end
    return None


def _zig_switch_arms(text,opening,closing,pairs):
    """Own arrow patterns/captures and comma separators at this switch only."""
    commas=[];arrows=[];stack=[];cursor=opening+1
    while cursor<closing:
        char=text[cursor]
        if char in '([{':stack.append(char)
        elif char in ')]}' and stack:stack.pop()
        elif not stack and char==',':commas.append(cursor)
        elif not stack and text.startswith('=>',cursor):
            arrows.append(cursor)
            capture_start=_trim(text,cursor+2,closing)
            capture=re.match(r'\|\s*\*?[A-Za-z_]\w*(?:\s*,\s*\*?[A-Za-z_]\w*)?\s*\|',text[capture_start:closing])
            if capture:
                cursor=capture_start+capture.end();continue
            cursor+=1
        cursor+=1
    events=[];blocks=[];issues=[];pattern_start=opening+1;chunk_start=pattern_start;imperative=False;arm_count=0
    for separator in [*commas,closing]:
        own_arrows=[arrow for arrow in arrows if chunk_start<=arrow<separator]
        if not own_arrows:
            if separator==closing and text[pattern_start:separator].strip():
                issues.append(dict(kind='zig-switch-arm-review',offset=pattern_start,message='Pending switch alternatives have no proven arrow/body.'))
            chunk_start=separator+1
            continue
        if len(own_arrows)!=1:
            issues.append(dict(kind='zig-switch-arm-review',offset=chunk_start,message='Multiple own-scope arrows require a dedicated arm grammar.'))
            pattern_start=chunk_start=separator+1;continue
        arrow=own_arrows[0];start=_trim(text,pattern_start,arrow);header_end=arrow+2
        pattern=text[start:arrow]
        if not pattern.strip() or re.search(r'\b(?:const|var|fn|proc|return|break|if|for|while|switch)\b|:=|;',pattern):
            issues.append(dict(kind='zig-switch-pattern-review',offset=start,message='Switch alternatives need structural pattern/type review.'))
        body_start=_trim(text,header_end,separator)
        if text[body_start:body_start+1]=='|':
            capture=re.match(r'\|\s*\*?[A-Za-z_]\w*(?:\s*,\s*\*?[A-Za-z_]\w*)?\s*\|',text[body_start:separator])
            if capture:header_end=body_start+capture.end();body_start=_trim(text,header_end,separator)
            else:issues.append(dict(kind='zig-switch-capture-review',offset=body_start,message='Complex arm capture has no proven formal boundary.'))
        events.append(dict(kind='branch',start=start,end=header_end,units=1,scope='zig-switch-arm'))
        if separator<closing:events.append(dict(kind='structural-separator',start=separator,end=separator+1,units=0,scope='zig-switch-arm'))
        if body_start>=separator:
            issues.append(dict(kind='zig-switch-arm-review',offset=header_end,message='Switch arm has no recognized body.'))
        elif text[body_start]=='{' and pairs.get(body_start) is not None and pairs[body_start]<separator:
            blocks.append(body_start);imperative=True
        elif re.match(r'(?:break|return|continue)\b',text[body_start:separator]):imperative=True
        arm_count+=1;pattern_start=chunk_start=separator+1
    if not arm_count:issues.append(dict(kind='zig-switch-arm-review',offset=opening,message='Switch has no proven arrow-arm owners.'))
    return dict(events=events,blocks=blocks,diagnostics=issues,imperative=imperative and not issues)


def _zig_expression(text,start,end,literal_spans=()):
    """Prove a bounded scalar Zig expression with exact operand/data spans.

    Calls and casts are syntax compositions, without a callee-purity test.
    Written executable blocks, declarations, assignments, transfers, loops,
    switch arms, and anonymous functions are deliberately outside this proof.
    """
    if end-start>20000:return None
    token=re.compile(r'@?[A-Za-z_]\w*|(?:0[xX][0-9A-Fa-f_]+|\d[\d_]*(?:\.\d[\d_]*)?(?:[eE][+-]?\d+)?)|'
        r'\+%=|-\%=|\*%=|<<=|>>=|\+=|-=|\*=|/=|%=|&=|\|=|\^=|=>|:=|==|!=|<=|>=|<<|>>|\+%|-\%|\*%|\+\+|\*\*|\.\.|[^\s]')
    tokens=[dict(word=m[0],start=m.start(),end=m.end()) for m in token.finditer(text,start,end)]
    literal_evidence=[span for span in literal_spans if start<=span['start'] and span['end']<=end]
    if any(span['start']<end and start<span['end'] and span not in literal_evidence for span in literal_spans):return None
    for literal in literal_evidence:
        if any(entry['start']<literal['start']<entry['end'] or entry['start']<literal['end']<entry['end'] for entry in tokens):return None
        tokens=[entry for entry in tokens if not (literal['start']<=entry['start'] and entry['end']<=literal['end'])]
        tokens.append(dict(word='__zig_literal',start=literal['start'],end=literal['end'],literal=True))
    tokens.sort(key=lambda entry:entry['start'])
    if not tokens or len(tokens)>4096:return None
    forbidden={'const','var','fn','struct','union','enum','for','while','switch','return','break','continue',
        'defer','errdefer','unreachable','asm','nosuspend','suspend','resume','await','async','export','extern','inline'}
    precedence={'catch':1,'orelse':1,'or':2,'and':3,'==':4,'!=':4,'<':4,'>':4,'<=':4,'>=':4,
        '|':5,'^':6,'&':7,'<<':8,'>>':8,'+':9,'-':9,'+%':9,'-%':9,'++':9,
        '*':10,'/':10,'%':10,'*%':10,'**':10}
    position=0;level=0;ifs=[];data=[];captures=[]
    def word():return tokens[position]['word'] if position<len(tokens) else None
    def take(value):
        nonlocal position
        if word()==value:position+=1;return True
        return False
    def capture():
        nonlocal position
        if word()!='|':return True
        opening=tokens[position]['start'];position+=1;take('*')
        if word() is None or not re.fullmatch(r'[A-Za-z_]\w*',word()) or word() in forbidden:return False
        position+=1
        if not take('|'):return False
        captures.append(dict(start=opening,end=tokens[position-1]['end'],role='conditional-capture-formal'))
        return True
    def aggregate():
        nonlocal position
        if word()!='{':return False
        opening=tokens[position]['start'];position+=1
        if not take('}'):
            while True:
                # Named aggregate fields are structural associations, not
                # independent runtime bindings. Bare = elsewhere is rejected.
                if word()=='.' and position+2<len(tokens) and re.fullmatch(r'[A-Za-z_]\w*',tokens[position+1]['word']) and tokens[position+2]['word']=='=':
                    position+=3
                if expression(stops={',','}'}) is None:return False
                if take('}'):break
                if not take(','):return False
                if take('}'):break
        data.append(dict(start=opening,end=tokens[position-1]['end'],role='data-constructor'))
        return True
    def expression(minimum=0,stops=frozenset()):
        nonlocal level
        level+=1
        if level>64:level-=1;return None
        shape=parse_expression(minimum,stops)
        level-=1
        return shape
    def parse_expression(minimum,stops):
        nonlocal position
        current=word()
        if current is None or current in stops:return None
        if tokens[position].get('literal'):
            position+=1;shape='literal'
        elif current=='if':
            opening=tokens[position]['start'];position+=1
            if not take('('):return None
            condition_start=tokens[position-1]['start']
            if expression(stops={')'}) is None or not take(')'):return None
            condition_end=tokens[position-1]['end']
            if not capture():return None
            if expression(stops=set(stops)|{'else'}) is None or not take('else'):return None
            if not capture() or expression(stops=stops) is None:return None
            ifs.append(dict(start=opening,end=tokens[position-1]['end'],condition_start=condition_start,
                condition_end=condition_end,role='scalar-if-else'))
            shape='value'
        elif current in {'try','comptime','!','~','-','-%', '?','*','&'}:
            position+=1
            while current in {'?','*'} and word() in {'const','volatile','allowzero'}:position+=1
            if expression(11,stops) is None:return None
            shape='value'
        elif current=='(':
            position+=1
            if expression(stops={')'}) is None or not take(')'):return None
            shape='group'
        elif current=='.':
            position+=1
            if word()=='{':
                if not aggregate():return None
                shape='value'
            elif word() is not None and re.fullmatch(r'[A-Za-z_]\w*',word()) and word() not in forbidden:
                position+=1;shape='value'
            else:return None
        elif current=='[':
            # Array/slice type expressions and their constructors, never a
            # cross-language list literal. The type suffix must be explicit.
            position+=1
            if not take(']'):
                if word()=='*':
                    position+=1
                    if take(':') and expression(stops={']'}) is None:return None
                elif expression(stops={']'}) is None:return None
                if not take(']'):return None
            while word() in {'const','volatile','allowzero'}:position+=1
            if word() is None or not re.fullmatch(r'[A-Za-z_]\w*',word()) or word() in forbidden:return None
            position+=1;shape='type'
        elif re.fullmatch(r'@?[A-Za-z_]\w*|0[xX][0-9A-Fa-f_]+|\d[\d_]*(?:\.\d[\d_]*)?(?:[eE][+-]?\d+)?',current) and current not in forbidden|{'else','catch','orelse','and','or'}:
            position+=1;shape='atom' if re.fullmatch(r'@?[A-Za-z_]\w*',current) else 'literal'
        else:return None
        while position<len(tokens):
            current=word()
            if current in stops:break
            if current=='.':
                position+=1
                if word() in {'*','?'}:position+=1
                elif word() is not None and re.fullmatch(r'[A-Za-z_]\w*',word()) and word() not in forbidden:position+=1
                else:return None
                shape='atom';continue
            if current=='(':
                if shape not in {'atom','group'}:return None
                position+=1
                if not take(')'):
                    while True:
                        if expression(stops={',',')'}) is None:return None
                        if take(')'):break
                        if not take(','):return None
                shape='call';continue
            if current=='[':
                position+=1
                if word()!='..' and expression(stops={'..',']'}) is None:return None
                if take('..') and word()!=']' and expression(stops={']'}) is None:return None
                if not take(']'):return None
                shape='value';continue
            if current=='{':
                if shape not in {'atom','type'} or not aggregate():return None
                shape='value';continue
            power=precedence.get(current)
            if power is None or power<minimum:break
            position+=1
            if current=='catch' and not capture():return None
            if expression(power+1,stops) is None:return None
            shape='value'
        return shape
    shape=expression()
    if shape is None or position!=len(tokens):return None
    return dict(shape=shape,ifs=ifs,data=data,captures=captures,literals=literal_evidence)


def _zig_value_window(text,position,pairs,owners):
    """Find one written statement without absorbing a surrounding close."""
    reverse={end:start for start,end in pairs.items()};cursor=position-1
    while cursor>=0:
        char=text[cursor]
        if char in ')]}' and cursor in reverse:
            opening=reverse[cursor]
            if char=='}' and owners.get(opening,('data',))[0] in EXECUTABLE:break
            cursor=opening-1;continue
        if char in ';{':break
        cursor-=1
    start=_trim(text,cursor+1,position);stack=[];cursor=start
    while cursor<len(text) and cursor-start<=20000:
        char=text[cursor]
        if char in '([{':stack.append(char)
        elif char in ')]}':
            if not stack:break
            if stack[-1]!={')':'(',']':'[','}':'{'}[char]:return None
            stack.pop()
        elif char==';' and not stack:break
        cursor+=1
    if cursor-start>20000:return None
    end=cursor
    while end>start and text[end-1].isspace():end-=1
    return (start,end) if start<end else None


def _zig_scalar_compositions(text,pairs,owners,literal_spans=()):
    """Own proven surrounding values, preserving literal/data physical rows."""
    spans=[];evidence=[];diagnostics=[];seen=set()
    for match in re.finditer(r'\bif\s*\(',text):
        if any(span['start']<=match.start()<span['end'] for span in spans):continue
        window=_zig_value_window(text,match.start(),pairs,owners)
        if window is None or window in seen:continue
        seen.add(window);start,end=window;value=text[start:end]
        declaration=re.match(r'(?:const|var)\s+[A-Za-z_]\w*\s*(?::\s*[\w.?!*\[\]@(),\s]+)?=\s*',value)
        returned=re.match(r'return\s+',value)
        assignment=re.match(r'[A-Za-z_]\w*(?:\.[A-Za-z_]\w*|\[[^{};=]+\])*\s*=(?!=|>)\s*',value)
        prefix=declaration or returned or assignment
        operand=start+prefix.end() if prefix else start
        proof=_zig_expression(text,operand,end,literal_spans)
        # A statement if remains a control. Only a surrounding call, binding,
        # or return is a proved value position; no callee-purity requirement.
        if not proof or not proof['ifs'] or (not prefix and proof['shape']!='call'):
            potential_call=re.match(r'@?[A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*\s*\(',value)
            potential_field=re.match(r'\.[A-Za-z_]\w*\s*=(?!=|>)',value)
            if proof and proof['ifs'] and not prefix:
                diagnostics.append(dict(kind='zig-if-value-position-review',offset=match.start(),
                    message='A standalone conditional has no proved surrounding value owner; retain executable branch review.'))
            elif prefix or potential_field or (potential_call and not re.match(r'(?:if|for|while|switch)\b',value)):
                diagnostics.append(dict(kind='zig-if-expression-review',offset=match.start(),
                    message='Conditional operand/branches need a complete scalar value grammar proof; no opaque composition.'))
            continue
        preserved=set()
        for span in proof['data']:
            fragment=text[span['start']:span['end']]
            if '\n' not in fragment:continue
            first=text.count('\n',0,span['start'])+1;last=text.count('\n',0,span['end']-1)+1
            preserved.update(range(first,last+1))
        literal_rows=set()
        for span in proof['literals']:
            first=text.count('\n',0,span['start'])+1;last=text.count('\n',0,span['end']-1)+1
            literal_rows.update(range(first,last+1))
        if len(literal_rows)>1:preserved.update(literal_rows)
        first=text.count('\n',0,start)+1
        units=len(preserved|{first})
        terminal=end
        while terminal<len(text) and text[terminal].isspace():terminal+=1
        statement_end=terminal+1 if terminal<len(text) and text[terminal]==';' else end
        span=dict(kind='composed-value',start=start,end=statement_end,units=units,scope='value-expression',
            preserved_data_rows=sorted(preserved),outer_value_units=1)
        spans.append(span)
        evidence.append(dict(start=start,end=statement_end,expression_end=end,operand_start=operand,position='binding' if declaration or assignment else 'return' if returned else 'call',
            ifs=proof['ifs'],captures=proof['captures'],data=proof['data'],literals=proof['literals'],preserved_data_rows=sorted(preserved)))
    return dict(spans=spans,evidence=evidence,diagnostics=diagnostics)


def classify_scopes(cleaned,language,*,literal_spans=()):
    if language not in {'zig','odin'}:raise ValueError('Use explicit zig or odin programming syntax.')
    pairs,issues=brace_pairs(cleaned)
    scalar_values=dict(spans=[],evidence=[],diagnostics=[])
    contexts=[];stack=[];events=[];owners={};control_spans=[];parameters={}
    def emit(kind,start,end,units=1,scope='braced'):
        if start<end:events.append(dict(kind=kind,start=start,end=end,units=units,scope=scope))
    def issue(kind,index,message):
        if not any(entry['kind']==kind and entry.get('offset')==index for entry in issues):
            issues.append(dict(kind=kind,offset=index,message=message))
    def covered(index):return any(e['start']<=index<e['end'] for e in events)
    # Explicit compiler annotations retain one written semantic unit.
    if language=='odin':
        for match in re.finditer(r'@\(',cleaned):
            closing=pairs.get(match.end()-1)
            if closing is None:issue('odin-annotation-review',match.start(),'Unclosed compiler annotation.')
            else:emit('pragma',match.start(),closing+1,scope='compiler-pragma')
        for match in re.finditer(r'#(?:force_inline|no_inline|bounds_check|no_bounds_check|type_assert)\b',cleaned):
            emit('pragma',match.start(),match.end(),scope='compiler-pragma')
    # First classify brace owners from exact language header forms.
    for opening in sorted(i for i,c in enumerate(cleaned) if c=='{'):
        prefix=cleaned[:opening].rstrip();row=prefix.rsplit('\n',1)[-1]
        signature=_signature(cleaned,opening,language,pairs)
        start=None;kind='data';reason='literal-or-data'
        if signature:
            start,parameter,end=signature;kind='code';reason='named-function'
            parameters[parameter]=end
        else:
            type_header=re.search(r'\b(?:struct|union|enum|bit_set|bit_field)\s*(?:\([^{};]*\))?\s*$',row)
            if type_header:
                start=_header_row_start(cleaned,len(prefix)-len(row)+type_header.start());kind='type';reason='type-declaration'
            else:
                # Named and array constructors stay data even when a prior
                # condition/control keyword exists earlier on the source row.
                constructor=bool(re.search(r'(?:\.(?:\?)?|\b[A-Z]\w*|(?:\[[^\]\n]*\])+[\w.]+)\s*$',row))
                # A preceding packed executable block has already ended. Its
                # control/function header cannot own this following branch.
                row_boundary=max(row.rfind('{'),row.rfind('}'))+1
                control_row=row[row_boundary:]
                controls=list(re.finditer(r'\b(?:if|else|for|while|switch|defer|comptime)\b',control_row))
                control=controls[0] if controls else None
                if control and not constructor:
                    start=len(prefix)-len(row)+row_boundary+control.start();kind='code';reason='control'
                    header=cleaned[start:opening]
                    if language=='zig' and re.search(r'\b(?:switch|if)\b',header) and re.search(r'\breturn\b|(?<![=!<>])=(?!=|>)',cleaned[_row_start(cleaned,start):start]):
                        issue('zig-value-control-review',start,'Value-only branches/arms versus statement bodies need explicit composition review.')
                    control_spans.append((start,opening,'for' if re.match(r'\s*(?:for|while)\b',header) else 'control'))
                elif row.endswith(':') and not constructor:
                    label=re.search(r'\b[A-Za-z_]\w*\s*:\s*$',row)
                    if label:start=len(prefix)-len(row)+label.start();kind='code';reason='labelled-block'
                elif not row and prefix.endswith((';','}')):
                    kind='bare-code';reason='standalone-block'
                elif re.search(r'\b(?:fn|proc|for|while|if|switch|defer|struct|union)\b',row) and not constructor:
                    issue(language+'-unrecognized-brace-header-review',opening,'Brace may belong to an unsupported executable header.')
        owners[opening]=(kind,reason,start)
        if kind in EXECUTABLE:
            if start is not None:
                start=_trim(cleaned,start,opening)
                preceding=[e['end'] for e in events if start<=e['end']<=opening]
                if preceding:start=_trim(cleaned,max(preceding),opening)
                emit('branch' if re.match(r'else\b',cleaned[start:opening]) else 'header',start,opening)
            emit('open',opening,opening+1,0)
            closing=pairs.get(opening)
            if closing is not None:emit('close',closing,closing+1)
    if language=='zig':
        scalar_values=_zig_scalar_compositions(cleaned,pairs,owners,literal_spans)
        issues.extend(scalar_values['diagnostics'])
        for opening,(kind,reason,start) in list(owners.items()):
            if kind!='code' or reason!='control' or start is None or not re.match(r'switch\b',cleaned[start:opening]):continue
            closing=pairs.get(opening)
            if closing is None:continue
            arms=_zig_switch_arms(cleaned,opening,closing,pairs)
            events.extend(arms['events']);issues.extend(arms['diagnostics'])
            for block in arms['blocks']:
                old=owners.get(block)
                if old is None:continue
                arm=next((event for event in arms['events'] if event['kind']=='branch' and event['end']<=block and not cleaned[event['end']:block].strip()),None)
                if arm is not None:
                    # The arrow/capture owns this header. In particular,
                    # else=>{...} is not also a generic else block header.
                    events=[event for event in events if not (event.get('scope')=='braced' and event['kind'] in {'header','branch'} and
                        arm['start']<=event['start']<=arm['end'] and event['end']==block)]
                issues=[diagnostic for diagnostic in issues if not (diagnostic['kind']=='zig-unrecognized-brace-header-review' and diagnostic['offset']==block)]
                control_spans=[span for span in control_spans if not (span[0]==old[2] and span[1]==block)]
                owners[block]=('code','switch-arm-block',None)
                if old[0] not in EXECUTABLE:
                    emit('open',block,block+1,0,scope='zig-switch-arm-block')
                    emit('close',pairs[block],pairs[block]+1,scope='zig-switch-arm-block')
            if arms['imperative']:
                issues=[diagnostic for diagnostic in issues if not (diagnostic['kind']=='zig-value-control-review' and diagnostic['offset']==start)]
    # Remove only boundaries inside a complete proved value composition.
    # Its constructors are data; surrounding functions/controls remain owned.
    for value in scalar_values['spans']:
        start,end=value['start'],value['end']
        events=[event for event in events if not (start<=event['start'] and event['end']<=end)]
        issues=[diagnostic for diagnostic in issues if not (diagnostic['kind'] in {
            'zig-value-control-review','zig-unrecognized-brace-header-review'} and start<=diagnostic['offset']<end)]
        control_spans=[span for span in control_spans if not (start<=span[0] and span[1]<=end)]
        own_evidence=next(entry for entry in scalar_values['evidence'] if entry['start']==start and entry['end']==end)
        for constructor in own_evidence['data']:owners[constructor['start']]=('data','scalar-value-constructor',None)
        events.append(value)
    for index,char in enumerate(cleaned):
        synthetic=[('(',kind,start) for start,end,kind in control_spans if start<=index<end]
        contexts.append(tuple(stack+synthetic))
        if char=='{':stack.append((char,owners[index][0],index))
        elif char=='[':stack.append((char,'data',index))
        elif char=='(':
            previous=re.search(r'([A-Za-z_]\w*)\s*$',cleaned[:index])
            word=previous[1] if previous else ''
            kind='parameters' if index in parameters else 'control' if word in {'if','switch'} else 'for' if word in {'for','while'} else 'args' if word else 'group'
            stack.append((char,kind,index))
        elif char in ')]}' and stack and stack[-1][0]=={')':'(',']':'[','}':'{'}[char]:stack.pop()
    # Packed unbraced Zig if bodies carry a separate header, without invented
    # executable closers. Captures/else continuation need a dedicated gate.
    if language=='zig':
        for match in re.finditer(r'\bif\s*\(',cleaned):
            start=match.start();opening=match.end()-1;closing=pairs.get(opening)
            if covered(start) or closing is None:continue
            base=contexts[start]
            last=max((i for i,(_,kind,_) in enumerate(base) if kind in EXECUTABLE),default=-1)
            if any(kind in {'args','data','parameters','for','control'} for _,kind,_ in base[last+1:]):continue
            prefix=cleaned[_row_start(cleaned,start):start]
            if re.search(r'\breturn\b|(?<![=!<>])=(?!=|>)',prefix):
                issue('zig-if-expression-review',start,'Unbraced conditional expression remains one operand; review logical span ownership.')
            elif re.match(r'\s*\|',cleaned[closing+1:]):
                issue('zig-unbraced-capture-review',start,'Capture/body boundary needs explicit unbraced grammar.')
            else:emit('header',start,closing+1,scope='unbraced')
    # Odin case labels introduce arm headers. Their alternatives are data.
    if language=='odin':
        for match in re.finditer(r'\bcase\b',cleaned):
            if covered(match.start()):continue
            end=next((i for i in range(match.end(),len(cleaned)) if cleaned[i] in ':\n{}'),None)
            if end is None or cleaned[end]!=':':issue('odin-case-header-review',match.start(),'Case header has no proved colon boundary.')
            else:emit('branch',match.start(),end+1,scope='case')
    return dict(contexts=contexts,events=events,diagnostics=issues,owners=owners,value_evidence=scalar_values['evidence'])


def _odin_call_producer(value):
    """One whole call may contain data literals, never anonymous code blocks."""
    value=value.strip()
    match=re.match(r'[A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*\s*\(',value)
    if not match:return False
    pairs,issues=brace_pairs(value)
    if issues or pairs.get(match.end()-1)!=len(value)-1:return False
    if re.search(r'\b(?:proc|for|if|switch|defer|return)\b|:=|;',value):return False
    data_owners=[]
    for opening in (index for index,char in enumerate(value) if char=='{'):
        before=value[:opening].rstrip()
        # Empty/anonymous flags and named/array constructors are data. An
        # immediately preceding ')' is deliberately not a constructor proof.
        field_literal=before.endswith('=') and any(start<opening<end for start,end in data_owners)
        argument_literal=False
        if before.endswith('='):
            label=re.search(r'[A-Za-z_]\w*\s*=\s*$',before)
            containers=[start for start,end in pairs.items() if start<opening<end]
            owner=max(containers,default=-1)
            if label and owner>=0 and value[owner]=='(' and re.search(r'[A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*\s*$',value[:owner]):
                prefix=before[:label.start()].rstrip()
                argument_literal=bool(prefix.endswith(',')) or len(prefix)==owner+1
        if not (field_literal or argument_literal or before.endswith(('(', ',', '[', ':', '.')) or re.search(r'[A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*\s*$',before) or before.endswith(']')):
            return False
        data_owners.append((opening,pairs[opening]))
    return True


def _bindings(language):
    def count(*,text,original,context,assignments):
        code=text.strip();own=next((entry for entry in reversed(context) if entry[0]=='{'),None)
        if own and own[1]=='type':
            if language=='odin':
                field=re.fullmatch(r'([A-Za-z_]\w*(?:\s*,\s*[A-Za-z_]\w*)*)\s*:\s*[^{};]+',code)
                if field:
                    size=len(top_parts(field[1]));return dict(units=size,declaration_bindings=size,reason='independent-type-field-bindings')
            else:
                fields=top_parts(code.rstrip(','))
                if all(re.fullmatch(r'[A-Za-z_]\w*\s*:\s*[^{};]+',field) for field in fields):
                    return dict(units=len(fields),declaration_bindings=len(fields),reason='independent-type-field-bindings')
        if language=='odin':
            multiple=re.fullmatch(r'([A-Za-z_]\w*(?:\s*,\s*[A-Za-z_]\w*)+)\s*(?::=|=)(.*)',code,re.S)
            if multiple:
                names,values=top_parts(multiple[1]),top_parts(multiple[2])
                if len(names)==len(values):return dict(units=len(names),declaration_bindings=len(names),reason='independent-multiple-bindings')
                if len(values)==1 and _odin_call_producer(values[0]):
                    return dict(units=1,reason='tuple-multiple-return-binding')
                return dict(units=len(names),diagnostics=[dict(kind='odin-multiple-binding-producer-review',message='Unequal initializer arity needs producer/type review.')])
            declared=re.fullmatch(r'([A-Za-z_]\w*(?:\s*,\s*[A-Za-z_]\w*)+)\s*:\s*[^:=;{}]+',code)
            if declared:
                size=len(top_parts(declared[1]));return dict(units=size,declaration_bindings=size,reason='independent-uninitialized-declaration-bindings')
        return {}
    return count


def normalize(source,language):
    if language not in {'zig','odin'}:raise ValueError('Supported explicit languages: zig, odin.')
    masked=mask_zig(source) if language=='zig' else mask_odin(source)
    parsed=classify_scopes(masked['cleaned'],language,literal_spans=masked.get('literals',()))
    result=count_owned(source,masked['cleaned'],parsed['contexts'],parsed['events'],
        [*masked['diagnostics'],*parsed['diagnostics']],language,binding_counter=_bindings(language),
        named_equals=language=='odin')
    result['value_evidence']=parsed['value_evidence']
    return result
