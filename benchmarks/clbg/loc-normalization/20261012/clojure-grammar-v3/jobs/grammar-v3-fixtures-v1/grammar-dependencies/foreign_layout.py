"""Unhooked F#/Roc/Elm source-owner proposal, never a chart admission.

These are three explicit grammars. Layout is not aliased to Julia or Rust.
The common accounting engine supplies offsets and canonical-unit arithmetic;
value_forms supplies only its language-neutral delimiter/whitespace helpers.
Execute/qualify only in a reviewed job holding the common benchmark mutex.
"""
import re

from foreign_core import brace_pairs, count_owned, replacement, structural, top_parts
from value_forms import _matching, _trim


LANGUAGES = {'fsharp', 'roc', 'elm'}
PROPOSAL_VERSION = 'foreign-layout-unhooked-20261012-v1'
IDENT = r"[A-Za-z_][\w']*[!]?"
ASSIGN = re.compile(r'<-|(?<![=!<>~:])=(?!=|>|~)')
TOKEN = re.compile(r'\[\||\|\]|\|>|<\||->|<-|=>|==|!=|<=|>=|::|&&|\|\||'+IDENT+r'|[^\s]')
BLOCK_WORDS = {'let','mutable','for','while','async','task','try','finally','return','yield','yield!','do','use','use!','let!','match!'}


def _issue(kind, offset, message):
    return dict(kind=kind, offset=offset, message=message)


def _event(kind, start, end, units=1, scope='foreign-layout'):
    return dict(kind=kind, start=start, end=end, units=units, scope=scope)


def _roc_quoted(source,opening,depth=0):
    """Locate a Roc string without confusing payload quotes with its close.

    Payloads are balanced lexically here, then proved as scalar expressions by
    a separate grammar. Exact original offsets are kept throughout.
    """
    if depth>16: return dict(end=len(source),complete=False,payloads=[],reason='nesting-limit')
    terminator='"""' if source.startswith('"""',opening) else '"'
    cursor=opening+len(terminator); payloads=[]
    while cursor<len(source):
        if source[cursor]=='\\': cursor=min(len(source),cursor+2); continue
        if source.startswith(terminator,cursor):
            return dict(end=cursor+len(terminator),complete=True,payloads=payloads,reason='complete')
        if source.startswith('${',cursor):
            start=cursor; body=cursor+2; cursor=body; stack=['{']
            while cursor<len(source) and stack:
                char=source[cursor]
                if char=='"':
                    nested=_roc_quoted(source,cursor,depth+1)
                    if not nested['complete']:
                        return dict(end=nested['end'],complete=False,payloads=payloads,reason='unclosed-payload-literal')
                    cursor=nested['end']; continue
                if char=="'":
                    character=re.match(r"'(?:[^'\\\r\n]|\\(?:u[0-9A-Fa-f]{4}|U[0-9A-Fa-f]{8}|x[0-9A-Fa-f]{2}|.))'",source[cursor:])
                    if not character:
                        return dict(end=cursor,complete=False,payloads=payloads,reason='unproved-payload-character')
                    cursor+=character.end(); continue
                if char=='#':
                    newline=source.find('\n',cursor); cursor=len(source) if newline<0 else newline; continue
                if char in '([{': stack.append(char)
                elif char in ')]}':
                    if not stack or stack[-1]!={')':'(',']':'[','}':'{'}[char]:
                        return dict(end=cursor+1,complete=False,payloads=payloads,reason='mismatched-payload-delimiter')
                    stack.pop()
                    if not stack:
                        payloads.append(dict(start=start,end=cursor+1,payload_start=body,payload_end=cursor))
                cursor+=1
            if stack: return dict(end=len(source),complete=False,payloads=payloads,reason='unclosed-payload')
            continue
        cursor+=1
    return dict(end=len(source),complete=False,payloads=payloads,reason='unclosed-string')


def _roc_scalar_interpolation(payload,base=0,depth=0):
    """Prove one scalar Roc operand, not independent executable work.

    This consumes a bounded explicit expression grammar: scalar literals,
    identifiers, calls, member/index access, scalar list arguments, operators,
    and complete if/then/else expressions. Bindings, sequences, callbacks,
    declarations, and unrecognized forms remain review gated. Callee purity is
    not inferred, and no interpolation owns an outer literal's physical rows.
    """
    if depth>16 or len(payload)>65536: return dict(scalar=False,reason='nesting-or-size-limit',nested=[])
    tokens=[]; nested_evidence=[]; cursor=0
    atom=re.compile(r'(?:\d+(?:\.\d+)?(?:[eE][+-]?\d+)?(?:[iu](?:8|16|32|64|128)|f(?:32|64)|dec)?|'+IDENT+r')')
    operator=re.compile(r'\|>|\?\?|\*\*|//|==|!=|<=|>=|&&|\|\||[+*/%<>=!^;,:()\[\]{}|\\.-]')
    while cursor<len(payload):
        char=payload[cursor]
        if char.isspace(): cursor+=1; continue
        if char=='#':
            newline=payload.find('\n',cursor); cursor=len(payload) if newline<0 else newline; continue
        if char=='"':
            literal=_roc_quoted(payload,cursor,depth+1)
            if not literal['complete']: return dict(scalar=False,reason=literal['reason'],nested=nested_evidence)
            for span in literal['payloads']:
                proof=_roc_scalar_interpolation(payload[span['payload_start']:span['payload_end']],base+span['payload_start'],depth+1)
                evidence=dict(span,classification='scalar' if proof['scalar'] else 'review',reason=proof['reason'],
                    payload=payload[span['payload_start']:span['payload_end']])
                for key in ('start','end','payload_start','payload_end'): evidence[key]+=base
                nested_evidence.append(evidence); nested_evidence.extend(proof['nested'])
                if not proof['scalar']: return dict(scalar=False,reason='nested-interpolation-'+proof['reason'],nested=nested_evidence)
            tokens.append(('literal',payload[cursor:literal['end']])); cursor=literal['end']; continue
        if char=="'":
            found=re.match(r"'(?:[^'\\\r\n]|\\(?:u[0-9A-Fa-f]{4}|U[0-9A-Fa-f]{8}|x[0-9A-Fa-f]{2}|.))'",payload[cursor:])
            if not found: return dict(scalar=False,reason='unproved-character',nested=nested_evidence)
            tokens.append(('literal',found[0])); cursor+=found.end(); continue
        found=atom.match(payload,cursor)
        if found:
            value=found[0]; tokens.append(('number' if value[0].isdigit() else 'identifier',value)); cursor=found.end(); continue
        found=operator.match(payload,cursor)
        if not found: return dict(scalar=False,reason='unknown-token',nested=nested_evidence)
        tokens.append(('operator',found[0])); cursor=found.end()
    forbidden={'let','mutable','for','while','when','is','return','break','continue','expect','implements','where',
        'app','module','import','provides','exposes','package','do','yield','try','finally'}
    if any(value in forbidden or value in {'=',';',':','{','}','|','\\'} for _,value in tokens):
        return dict(scalar=False,reason='independent-or-unproved-executable-form',nested=nested_evidence)
    precedence={'|>':1,'??':2,'||':3,'&&':4,'==':5,'!=':5,'<':5,'>':5,'<=':5,'>=':5,
        '+':6,'-':6,'*':7,'/':7,'//':7,'%':7,'**':8,'^':8}
    if len(tokens)>4096: return dict(scalar=False,reason='token-limit',nested=nested_evidence)
    position=0; parse_depth=0
    def take(value):
        nonlocal position
        if position<len(tokens) and tokens[position][1]==value: position+=1; return True
        return False
    def expression(minimum=0,stops=frozenset()):
        nonlocal parse_depth
        parse_depth+=1
        if parse_depth>64:
            parse_depth-=1; return None
        shape=parse_expression(minimum,stops)
        parse_depth-=1
        return shape
    def parse_expression(minimum=0,stops=frozenset()):
        nonlocal position
        if position>=len(tokens) or tokens[position][1] in stops: return None
        kind,value=tokens[position]
        if value=='if':
            position+=1
            if expression(stops=frozenset({'then'})) is None or not take('then'): return None
            if expression(stops=frozenset({'else'})) is None or not take('else'): return None
            if expression(stops=stops) is None: return None
            shape='value'
        elif value in {'+','-','!'}:
            position+=1
            if expression(9,stops) is None: return None
            shape='value'
        elif value=='(':
            position+=1
            if expression(stops=frozenset({')'})) is None or not take(')'): return None
            shape='value'
        elif value=='[':
            position+=1
            if not take(']'):
                while True:
                    if expression(stops=frozenset({',',']'})) is None: return None
                    if take(']'): break
                    if not take(','): return None
                    if take(']'): break
            shape='value'
        elif kind in {'number','literal','identifier'} and value not in {'then','else'}:
            position+=1; shape='callable' if kind=='identifier' else 'value'
        else: return None
        while position<len(tokens):
            value=tokens[position][1]
            if value in stops: break
            if value=='.':
                position+=1
                if position>=len(tokens) or tokens[position][0]!='identifier': return None
                position+=1; shape='callable'; continue
            if value=='[':
                position+=1
                if expression(stops=frozenset({']'})) is None or not take(']'): return None
                shape='value'; continue
            if value=='(':
                if shape!='callable': return None
                position+=1
                if not take(')'):
                    while True:
                        if expression(stops=frozenset({',',')'})) is None: return None
                        if take(')'): break
                        if not take(','): return None
                shape='value'; continue
            level=precedence.get(value)
            if level is None or level<minimum: break
            position+=1
            if expression(level+1,stops) is None: return None
            shape='value'
        return shape
    scalar=bool(tokens) and expression() is not None and position==len(tokens)
    return dict(scalar=scalar,reason='one-scalar-expression' if scalar else 'unproved-expression-composition',nested=nested_evidence)


def mask_layout(source, language):
    """Own comments, quoted literals and semantic directives without reflow.

    Roc // is arithmetic division. Elm {- -} and F# (* *) comments nest.
    Roc scalar interpolation operands are proved separately while their outer
    literal retains its written wrapping. Unknown interpolation is gated.
    A generic type variable 'T is not a character literal. No other-language
    lexer is substituted for Roc.
    """
    if language not in LANGUAGES:
        raise ValueError('Only explicit F#/Roc/Elm layout grammars are proposed.')
    chars=list(source); diagnostics=[]; directives=[]; interpolation_evidence=[]; index=0
    if source.startswith('\ufeff'): chars[0]=' '
    line_comment={'fsharp':'//','roc':'#','elm':'--'}[language]
    comment_pair={'fsharp':('(*','*)'),'elm':('{-','-}')}.get(language)
    while index<len(source):
        start=index
        if comment_pair and source.startswith(comment_pair[0],index):
            opening,closing=comment_pair; level=1; index+=len(opening)
            while index<len(source) and level:
                if source.startswith(opening,index): level+=1; index+=len(opening)
                elif source.startswith(closing,index): level-=1; index+=len(closing)
                else: index+=1
            chars[start:index]=replacement(source[start:index])
            if level: diagnostics.append(_issue('foreign-unclosed-comment',start,'Nested comment has no matching close.'))
            continue
        if source.startswith(line_comment,index):
            end=source.find('\n',index); index=len(source) if end<0 else end
            chars[start:index]=replacement(source[start:index]); continue
        if language=='fsharp' and source[index]=='#' and not source[source.rfind('\n',0,index)+1:index].strip():
            end=source.find('\n',index); index=len(source) if end<0 else end
            directives.append(_event('compiler-pragma',start,index,scope='foreign-fsharp-directive'))
            if re.match(r'#\s*(?:if|else|endif|load|r|I)\b',source[start:index]):
                diagnostics.append(_issue('foreign-directive-policy-review',start,'Conditional/reference/load directives require the exact compilation/source policy.'))
            continue
        prefix=''; quote_start=index
        if language=='fsharp':
            found=re.match(r'(?:\$@|@\$|\$|@)?(?="|\'[^\n])',source[index:])
            if found: prefix=found[0]; quote_start=index+len(prefix)
        if quote_start<len(source) and source[quote_start]=='"':
            if language=='roc':
                literal=_roc_quoted(source,quote_start); index=literal['end']
                chars[start:index]=replacement(source[start:index],literal=True)
                if not literal['complete']:
                    diagnostics.append(_issue('foreign-unclosed-literal',start,'Roc literal/interpolation is not lexically complete: '+literal['reason']+'.'))
                for span in literal['payloads']:
                    proof=_roc_scalar_interpolation(source[span['payload_start']:span['payload_end']],span['payload_start'])
                    evidence=dict(span,classification='scalar' if proof['scalar'] else 'review',reason=proof['reason'],
                        payload=source[span['payload_start']:span['payload_end']])
                    interpolation_evidence.append(evidence); interpolation_evidence.extend(proof['nested'])
                    if not proof['scalar']:
                        diagnostics.append(_issue('foreign-interpolation-expression-review',span['start'],
                            'Roc interpolation operand needs source review: '+proof['reason']+'.'))
                continue
            triple=source.startswith('"""',quote_start)
            terminator='"""' if triple else '"'
            verbatim=language=='fsharp' and '@' in prefix and not triple
            index=quote_start+len(terminator); complete=False
            while index<len(source):
                if verbatim and source.startswith('""',index): index+=2; continue
                if source.startswith(terminator,index): index+=len(terminator); complete=True; break
                if not verbatim and source[index]=='\\': index=min(len(source),index+2)
                else: index+=1
            value=source[start:index]
            chars[start:index]=replacement(value,literal=True)
            if not complete: diagnostics.append(_issue('foreign-unclosed-literal',start,'Quoted literal has no matching close.'))
            if '$' in prefix:
                diagnostics.append(_issue('foreign-interpolation-expression-review',start,'Interpolation operands require exact source-specific expression review.'))
            continue
        if source[index]=="'":
            # One character or one escaped character; longer type/identifier
            # apostrophes remain grammar, never hide the rest of a source row.
            found=re.match(r"'(?:[^'\\\r\n]|\\(?:u[0-9A-Fa-f]{4}|U[0-9A-Fa-f]{8}|x[0-9A-Fa-f]{2}|.))'",source[index:])
            if found:
                index+=len(found[0]); chars[start:index]=replacement(found[0],literal=True); continue
            if language in {'roc','elm'}:
                diagnostics.append(_issue('foreign-character-literal-review',index,'Unrecognized character/apostrophe form needs lexical review.'))
        if language=='fsharp' and source.startswith('``',index):
            end=source.find('``',index+2)
            if end<0: diagnostics.append(_issue('foreign-unclosed-identifier',index,'Quoted identifier has no matching close.')); index=len(source)
            else: index=end+2
            chars[start:index]=replacement(source[start:index],literal=True); continue
        index+=1
    return dict(cleaned=''.join(chars),diagnostics=diagnostics,directives=directives,interpolation_evidence=interpolation_evidence)


def _rows(text):
    rows=[]; offset=0
    for value in text.splitlines(keepends=True):
        stripped=value.lstrip(' \t\r\n'); leading=len(value)-len(stripped)
        rows.append(dict(start=offset,end=offset+len(value.rstrip('\r\n')),
            code_start=offset+leading,indent=len(value[:leading].expandtabs(8)),code=stripped.rstrip('\r\n')))
        offset+=len(value)
    return rows


def _tokens(text, start=0, end=None):
    return [dict(word=m[0],start=m.start(),end=m.end()) for m in TOKEN.finditer(text,start,len(text) if end is None else end)]


def _top_tokens(text):
    result=[]; stack=[]
    for token in _tokens(text):
        word=token['word']
        if word in {'(', '[', '{','[|'}: stack.append('[' if word=='[|' else word)
        elif word in {')',']','}','|]'}:
            if stack: stack.pop()
        elif not stack: result.append(token)
    return result


def _scalar(text,language,depth=0):
    """Prove one expression; reject independent binding/action sequences.

    Record fields, tuple components and call/formal arguments are structural.
    Multiline literal rows retain their physical wrapping rather than becoming
    an opaque scalar composition. Separate top-level action rows never merge.
    """
    if depth>16: return False
    value=text.strip().rstrip(',').rstrip()
    if not value: return False
    if value.startswith('(') and _matching(value,0)==len(value):
        return _scalar(value[1:-1],language,depth+1)
    if re.search(r'<-|\b(?:let!?|in|mutable|for|while|async|task|try|finally|return!?|yield!?|do!?|use!?)\b',value): return False
    unknown_declarations={'fsharp':r'\b(?:val|function|object|interface|abstract|inherit|override|exception|and)\b',
        'roc':r'\b(?:expect|implements|where)\b','elm':r'\b(?:infix|infixl|infixr)\b'}
    if re.search(unknown_declarations[language],value): return False
    if language=='fsharp' and re.search(r'\bseq\s*\{',value): return False
    if re.search(r'\b(?:if|then|else|match|when|case)\b',value): return _value_control(value,language,depth+1)
    pairs,issues=brace_pairs(value)
    if issues: return False
    stack=[]; top_newlines=[]
    for index,char in enumerate(value):
        if char in '([{': stack.append(char)
        elif char in ')]}' and stack: stack.pop()
        elif char==';' and not any(frame in '[{' for frame in stack): return False
        elif not stack and char=='\n': top_newlines.append(index)
        if not stack and ASSIGN.match(value,index): return False
    # Literal wrapping is independently retained. Calls/tuples may continue
    # inside a pair, but a matrix/record/list is not silently flattened.
    if any(value[o] in '[{' and '\n' in value[o:c+1] for o,c in pairs.items()): return False
    for point in top_newlines:
        before=value[:point].rstrip(); after=value[point+1:].lstrip()
        if not after: continue
        if not (re.search(r'(?:\|>|<\||[+*/%&^=,-])$',before) or re.match(r'(?:\|>|<\||[+*/%&^])',after)):
            return False
    if language=='elm' and re.search(r'\b(?:port|module|import|type)\b',value): return False
    return True


def _arm_spans(value,language,terminal):
    """Arrow arm patterns are structural, not assignment declarations."""
    top=_top_tokens(value); arrows=[t for t in top if t['word']=='->' and t['start']>terminal]
    if not arrows: return None
    spans=[]
    for index,arrow in enumerate(arrows):
        if index==0: start=terminal
        else:
            lower=arrows[index-1]['end']
            pipes=[t for t in top if lower<t['start']<arrow['start'] and t['word']=='|']
            if language=='fsharp' and pipes: start=pipes[-1]['start']
            else:
                start=value.rfind('\n',lower,arrow['start'])+1
                if start<=lower: return None
        pattern=value[start:arrow['start']].strip().lstrip('|').strip()
        if not pattern or re.search(r'\b(?:fun|let|if|when|case)\b|\\',pattern): return None
        pattern_top=_top_tokens(pattern)
        if any(t['word'] in {'=',';'} for t in pattern_top): return None
        if any(t['word']=='|' for t in pattern_top):
            atom=r'(?:[+-]?\d+\w*|S+|_)'
            if language!='roc' or not re.fullmatch(atom+r'(?:\s*\|\s*'+atom+r')+',pattern): return None
        spans.append(dict(start=start,header_end=arrow['end'],body_start=arrow['end']))
    for index,span in enumerate(spans): span['end']=spans[index+1]['start'] if index+1<len(spans) else len(value)
    return spans


def _value_control(value,language,depth=0):
    top=_top_tokens(value)
    if not top: return False
    if top[0]['word']=='if':
        # Match then/else at the current conditional depth. Nested scalar
        # conditions recurse; if/else without all branches is not a value.
        level=0; then=None; otherwise=None
        for token in top:
            word=token['word']
            if word=='if': level+=1
            elif word=='then' and level==1 and then is None: then=token
            elif word=='else':
                if level==1: otherwise=token; break
                level-=1
        if then is None or otherwise is None: return False
        condition=value[top[0]['end']:then['start']]
        return _scalar(condition,language,depth+1) and _scalar(value[then['end']:otherwise['start']],language,depth+1) and _scalar(value[otherwise['end']:],language,depth+1)
    header={'fsharp':('match','with'),'roc':('when','is'),'elm':('case','of')}[language]
    if top[0]['word']!=header[0]: return False
    terminal=next((t for t in top if t['word']==header[1]),None)
    if not terminal or not _scalar(value[top[0]['end']:terminal['start']],language,depth+1): return False
    arms=_arm_spans(value,language,terminal['end'])
    return bool(arms) and all(_scalar(value[a['body_start']:a['end']],language,depth+1) for a in arms)


def _type_prefix(text,opening,language):
    row_start=text.rfind('\n',0,opening)+1
    # The named type header may precede a standalone record opening.
    prefix=text[row_start:opening].strip()
    if not prefix:
        previous=text[:row_start].rstrip(); previous_start=previous.rfind('\n')+1
        prefix=previous[previous_start:].strip(); row_start=previous_start
    grammars={
        'fsharp':r'type\s+'+IDENT+r'(?:\s*<[^{};]+>)?\s*=\s*',
        'elm':r'type\s+alias\s+'+IDENT+r'(?:\s+'+IDENT+r')*\s*=\s*',
        'roc':r'[A-Z][A-Za-z0-9_]*\s*:\s*',
    }
    if re.fullmatch(grammars[language],prefix): return row_start+len(text[row_start:opening])-len(text[row_start:opening].lstrip())
    return None


def _base_contexts(text,language,directives):
    pairs,diagnostics=brace_pairs(text); events=list(directives); regions=[]; owners=[]; brace_kinds={}; paren_kinds={}
    for opening,closing in sorted(pairs.items()):
        if text[opening]!='(': continue
        own=_top_tokens(text[opening+1:closing])
        executable_group=language=='fsharp' and any(t['word'] in {'let',';','<-','for','while'} for t in own)
        executable_group=executable_group or (language=='elm' and any(t['word']=='let' for t in own))
        if executable_group:
            paren_kinds[opening]='bare-code'
            events.extend([_event('open',opening,opening+1,units=0,scope='foreign-layout-expression-group'),
                _event('data-close',closing,closing+1,units=0,scope='foreign-layout-expression-group')])
    for opening,closing in sorted(pairs.items()):
        if text[opening]!='{': continue
        before=text[max(0,opening-100):opening]
        computation=re.search(r'\b(async|task|seq)\s*$',before) if language=='fsharp' else None
        type_start=_type_prefix(text,opening,language)
        if computation:
            start=opening-len(computation[0]); brace_kinds[opening]='code'
            events.extend([_event('header',start,opening+1,scope='foreign-fsharp-computation'),_event('close',closing,closing+1,scope='foreign-fsharp-computation')])
            regions.append(dict(start=opening+1,end=closing,scope='fsharp-computation'))
            owners.append(dict(start=start,end=closing+1,kind='code',owner='computation'))
        elif type_start is not None:
            brace_kinds[opening]='type'
            events.extend([_event('header',type_start,opening+1,scope='foreign-record-type'),_event('close',closing,closing+1,scope='foreign-record-type')])
            fields=text[opening+1:closing]; begin=opening+1
            # Field commas/semicolons separate declarations only in this
            # proven named type owner, not in record instances or formals.
            chunks=[]; stack=[]; cursor=begin
            for index in range(begin,closing):
                char=text[index]
                if char in '([{': stack.append(char)
                elif char in ')]}' and stack: stack.pop()
                elif not stack and char in ',;\n':
                    if char in ',;':
                        events.append(_event('structural-separator',index,index+1,units=0,scope='foreign-record-type-field-separator'))
                    chunks.append((cursor,index)); cursor=index+1
            chunks.append((cursor,closing))
            for start,end in chunks:
                start,end=_trim(text,start,end)
                if start==end: continue
                field=text[start:end]
                if re.fullmatch(r'(?:mutable\s+)?'+IDENT+r'\s*:\s*[^=;{}]+',field,re.S):
                    events.append(_event('declaration',start,end,scope='foreign-independent-record-field'))
                else: diagnostics.append(_issue('foreign-layout-type-field-review',start,'Record field type/attribute/grouped names need a dedicated declaration rule.'))
            owners.append(dict(start=type_start,end=closing+1,kind='type',owner='record-type'))
        else:
            brace_kinds[opening]='data'
            previous=re.search(r'('+IDENT+r')\s*$',before) if language=='fsharp' else None
            if previous and previous[1] not in {'yield','yield!','return'}:
                diagnostics.append(_issue('foreign-fsharp-computation-builder-review',opening,'Identifier-led brace needs a proven formal-record pattern or computation-builder owner.'))
    contexts=[]; stack=[]
    for index,char in enumerate(text):
        contexts.append(tuple(stack))
        if char in '([{':
            kind=brace_kinds.get(index,paren_kinds.get(index,'data'))
            if char=='[' and language=='fsharp':
                following=text[index+1:pairs.get(index,index)+1].lstrip('| \t\r\n')
                if re.match(r'(?:for|let|yield)\b',following): kind='bare-code'
            stack.append((char,kind,index))
        elif char in ')]}' and stack: stack.pop()
    return dict(contexts=contexts,events=events,diagnostics=diagnostics,owners=owners,executable_regions=regions,pairs=pairs)


def _definition(text,row,language):
    code=row['code']; start=row['code_start']
    if language=='fsharp':
        match=re.match(r'(?:let\s+(?:rec\s+|mutable\s+)?|member\s+(?:private\s+)?)(?P<name>'+IDENT+r'(?:\.'+IDENT+r')?)(?P<tail>.*)',code)
        if not match: return None
    else:
        match=re.match(r'(?P<name>'+IDENT+r')(?P<tail>.*)',code)
        if not match or match['name'] in {'if','else','then','let','type','port','module','import','app','when','case','in','of','is'}: return None
        if language=='elm' and match['name'] in {'infix','infixl','infixr'}: return None
    # The signature's first top-level assignment, never record formal fields.
    prefix_len=match.start('tail'); tokens=_top_tokens(code[prefix_len:])
    assignment=next((t for t in tokens if t['word']=='='),None)
    if assignment is None: return None
    equal=start+prefix_len+assignment['start']
    formals=code[prefix_len:prefix_len+assignment['start']].strip()
    if any(token['word'] in {';','<-','then','else','do','in'} for token in _top_tokens(formals)): return None
    body_start=equal+1
    is_function=bool(formals and not formals.startswith((':',',')))
    if language=='roc':
        suffix=text[body_start:row['end']].lstrip()
        pipe=re.match(r'\|[^|\n]*\|',suffix)
        if pipe:
            if not re.fullmatch(r'[\w\s,()_]+',pipe[0][1:-1]):
                return dict(start=start,header_end=body_start,body_start=body_start,is_function=False,invalid_pipe=True)
            body_start+=len(text[body_start:row['end']])-len(suffix)+pipe.end(); is_function=True
        elif suffix.startswith('|'):
            return dict(start=start,header_end=body_start,body_start=body_start,is_function=False,invalid_pipe=True)
    return dict(start=start,header_end=body_start,body_start=body_start,is_function=is_function,invalid_pipe=False)


def _definition_end(text,rows,index,definition,pairs,language):
    row=rows[index]; end=len(text)
    inline_conditional=bool(re.match(r'if\b',text[definition['body_start']:row['end']].lstrip()))
    # A callback/definition inside data belongs only to that exact pair.
    containing=[close for opening,close in pairs.items() if opening<definition['start']<close]
    limit=min(containing,default=len(text))
    for later in rows[index+1:]:
        if later['code_start']>=limit: break
        if later['code'] and later['indent']<=row['indent']:
            if any(definition['body_start']<=opening<later['code_start']<=close for opening,close in pairs.items()): continue
            if language=='fsharp' and text[later['code_start']:later['code_start']+1]=='{' and later['code_start'] in pairs and re.fullmatch(r'\s*(?:async|task|seq)\s*',text[definition['body_start']:later['code_start']]): continue
            if inline_conditional and later['indent']==row['indent'] and re.match(r'else\b',later['code']): continue
            end=later['start']; break
    end=min(end,limit)
    # Packed independent bindings/statements on the header row do not become
    # one expression. The semicolon is retained for the common segment parser.
    iteration_header=False; local_lets=0
    for token in _top_tokens(text[definition['body_start']:row['end']]):
        word=token['word']
        if word==';' and not definition['is_function']:
            end=min(end,definition['body_start']+token['start']); break
        if language=='fsharp' and word=='for': iteration_header=True
        elif language=='fsharp' and word=='do': iteration_header=False
        elif word=='let': local_lets+=1
        elif word=='in':
            # A loop's iteration marker and a nested local scope's close are
            # inside this definition's body. Only an enclosing declaration
            # close ends the value being defined here.
            if language=='fsharp' and iteration_header: continue
            if local_lets: local_lets-=1; continue
            end=min(end,definition['body_start']+token['start']); break
    return _trim(text,definition['body_start'],end)[1]


def _covered(events,start,end):
    return any(e['start']<=start and end<=e['end'] for e in events)


def _conditional_limit(rows,start,text_end):
    """A same-indent else continues its own if; it is not the next owner."""
    row=next((r for r in rows if r['start']<=start<=r['end']),None)
    if row is None: return text_end
    for later in rows:
        if later['start']<=row['start'] or not later['code']: continue
        if later['indent']>row['indent']: continue
        if later['indent']==row['indent'] and re.match(r'else\b',later['code']): continue
        return later['start']
    return text_end


def _fsharp_val_fields(text,rows):
    """Only val fields in a proven named layout type, never expression data."""
    events=[]; field_spans=[]
    for index,row in enumerate(rows):
        if not re.fullmatch(r'type\s+'+IDENT+r'\s*=',row['code']): continue
        known=[]; field_indent=None
        for later in rows[index+1:]:
            if not later['code']: continue
            if later['indent']<=row['indent']: break
            if field_indent is None: field_indent=later['indent']
            if later['indent']!=field_indent: continue
            offset=later['code_start']
            for part in top_parts(later['code'],separator=';'):
                position=text.find(part,offset,later['end']) if part else -1
                if position<0: continue
                offset=position+len(part)
                if re.fullmatch(r'val\s+(?:mutable\s+)?'+IDENT+r"\s*:\s*[A-Za-z_'][\w.\s'<>*\[\](),]*",part):
                    event=_event('declaration',position,offset,scope='foreign-fsharp-val-field')
                    known.append(event); field_spans.append((position,offset))
        if known:
            events.append(_event('header',row['code_start'],row['end'],scope='foreign-fsharp-layout-type'))
            events.extend(known)
    return events,field_spans


def _control_events(text,language,contexts,opaque,rows):
    events=[]; diagnostics=[]; tokens=_tokens(text)
    for index,token in enumerate(tokens):
        word=token['word']; start=token['start']
        if _covered(opaque,start,token['end']) or structural(contexts[start]): continue
        if word=='if':
            level=0; terminal=None
            for later in tokens[index+1:]:
                if later['start']-start>20000: break
                if contexts[later['start']]!=contexts[start]: continue
                if later['word']=='if': level+=1
                elif later['word']=='then' and level==0: terminal=later; break
                elif later['word']==';': break
            if terminal:
                previous=tokens[index-1] if index else None
                header_start=previous['start'] if previous and previous['word']=='else' and text[previous['end']:start].strip()=='' else start
                events.append(_event('header',header_start,terminal['end'],scope='foreign-layout-if'))
                if language!='fsharp':
                    limit=_conditional_limit(rows,start,len(text))
                    if not any(t['word']=='else' and terminal['end']<=t['start']<limit and contexts[t['start']]==contexts[start] for t in tokens):
                        diagnostics.append(_issue('foreign-layout-missing-else-review',start,'Value conditional requires a proven else in its own layout scope.'))
            else: diagnostics.append(_issue('foreign-layout-if-review',start,'Conditional has no proven same-owner then terminal.'))
        elif word=='else':
            following=tokens[index+1] if index+1<len(tokens) else None
            if not following or following['word']!='if': events.append(_event('branch',start,token['end'],scope='foreign-layout-if'))
        elif word in {'match','when','case'}:
            expected={'fsharp':('match','with'),'roc':('when','is'),'elm':('case','of')}[language]
            if word!=expected[0]:
                diagnostics.append(_issue('foreign-layout-control-review',start,'Control keyword is outside this language\'s proven arm grammar.')); continue
            terminal=next((t for t in tokens[index+1:] if contexts[t['start']]==contexts[start] and t['word']==expected[1]),None)
            if not terminal or terminal['start']-start>20000:
                diagnostics.append(_issue('foreign-layout-arm-header-review',start,'Pattern control has no proven own-scope terminal.')); continue
            events.append(_event('header',start,terminal['end'],scope='foreign-layout-arms'))
            row=next((r for r in rows if r['start']<=start<=r['end']),None)
            limit=len(text)
            if row:
                for later in rows:
                    if later['start']>row['start'] and later['code'] and later['indent']<=row['indent']:
                        limit=later['start']; break
            arms=_arm_spans(text[start:limit],language,terminal['end']-start)
            if not arms: diagnostics.append(_issue('foreign-layout-arm-pattern-review',start,'Pattern/guard/inline arm ownership is not proven.'))
            else:
                for arm in arms: events.append(_event('branch',start+arm['start'],start+arm['header_end'],scope='foreign-layout-arm-pattern'))
        elif language=='fsharp' and word in {'for','while'}:
            terminal=next((t for t in tokens[index+1:] if contexts[t['start']]==contexts[start] and t['word']=='do'),None)
            if terminal and terminal['start']-start<20000: events.append(_event('header',start,terminal['end'],scope='foreign-fsharp-loop'))
            else: diagnostics.append(_issue('foreign-layout-loop-review',start,'Loop has no proven same-owner do terminal.'))
    return events,diagnostics


def _local_scope_events(text,language,contexts,owned):
    events=[]; diagnostics=[]; stack=[]
    for token in _tokens(text):
        containing=[event for event in owned if event['start']<=token['start'] and token['end']<=event['end']]
        binding_start=language=='fsharp' and token['word']=='let' and any(e['kind']=='composed-definition' and e['start']==token['start'] for e in containing)
        if (containing and not binding_start) or structural(contexts[token['start']]): continue
        if token['word']=='let':
            stack.append(token)
            if language=='elm': events.append(_event('open',token['start'],token['end'],units=0,scope='foreign-elm-local-declarations'))
        elif token['word']=='in':
            matching=next((index for index in range(len(stack)-1,-1,-1) if contexts[stack[index]['start']]==contexts[token['start']]),None)
            if matching is None:
                diagnostics.append(_issue('foreign-layout-in-owner-review',token['start'],'Written in has no proven local-declaration let owner.'))
            else:
                stack.pop(matching); events.append(_event('close',token['start'],token['end'],scope='foreign-'+language+'-local-declarations'))
    if language=='elm':
        for token in stack: diagnostics.append(_issue('foreign-layout-let-owner-review',token['start'],'Local declaration scope has no matching written in.'))
    return events,diagnostics


def _lambda_events(text,language,contexts,opaque,rows,pairs):
    """Expose independent locals in expression lambdas, never blanket mask args."""
    events=[]; regions=[]; diagnostics=[]
    patterns={'fsharp':r'\bfun\s+','roc':r'(?<![|>])\|(?=[A-Za-z_(])','elm':r'\\(?=[A-Za-z_(])'}
    for match in re.finditer(patterns[language],text):
        start=match.start()
        if _covered(opaque,start,match.end()): continue
        # A record update pipe is not a Roc lambda; a known named function's
        # formals are already owned by its definition header.
        if language=='roc':
            terminal=text.find('|',match.end()); terminal=terminal+1 if terminal>=0 else None
            if terminal and ('\n' in text[match.end():terminal] or not re.fullmatch(r'[\w\s,()_]+',text[match.end():terminal-1])): terminal=None
        else:
            arrow=text.find('->',match.end()); terminal=arrow+2 if arrow>=0 else None
            if terminal and '\n' in text[match.end():terminal]: terminal=None
        if terminal is None:
            diagnostics.append(_issue('foreign-layout-lambda-formals-review',start,'Anonymous function has no proven own formal terminal.')); continue
        containing=[close for opening,close in pairs.items() if opening<start<close]
        end=min(containing,default=len(text))
        row=next((r for r in rows if r['start']<=start<=r['end']),None)
        if row:
            for later in rows:
                if later['start']>row['start'] and later['code'] and later['indent']<=row['indent']:
                    end=min(end,later['start']); break
        body=text[terminal:end].rstrip().rstrip(',')
        if _scalar(body,language): continue
        if re.search(r'\b(?:let|mutable)\b|<-|(?<![=!<>:])=(?!=|>)',body):
            events.append(_event('header',start,terminal,scope='foreign-layout-lambda'))
            regions.append(dict(start=terminal,end=end,scope='layout-lambda'))
            suffix_end=text.find('\n',end); suffix_end=len(text) if suffix_end<0 else suffix_end
            suffix=text[end:suffix_end].strip(' \t\r,;)')
            if suffix:
                diagnostics.append(_issue('foreign-layout-lambda-surrounding-expression-review',end,'Imperative lambda has following operands/application text requiring exact outer-expression ownership review.'))
        else: diagnostics.append(_issue('foreign-layout-lambda-body-review',start,'Anonymous body is not a proven one-expression composition or local-binding body.'))
    return events,regions,diagnostics


def classify_scopes(cleaned,language,directives=()):
    if language not in LANGUAGES: raise ValueError('No layout grammar alias is permitted.')
    parsed=list(cleaned)
    for event in directives: parsed[event['start']:event['end']]=replacement(cleaned[event['start']:event['end']])
    text=''.join(parsed); rows=_rows(text); base=_base_contexts(text,language,directives)
    contexts=list(base['contexts']); events=list(base['events']); diagnostics=list(base['diagnostics']); regions=list(base['executable_regions']); owners=list(base['owners']); opaque=[]; definitions=[]
    # Attributes contain semicolons but are source declarations, not body
    # sequences. Their own one-unit event has exact written delimiters.
    val_fields=[]
    if language=='fsharp':
        for match in re.finditer(r'\[<[^\]]*>\]',text,re.S): events.append(_event('attribute',match.start(),match.end(),scope='foreign-fsharp-attribute'))
        val_events,val_fields=_fsharp_val_fields(text,rows); events+=val_events
    for index,row in enumerate(rows):
        if not row['code']: continue
        start=row['code_start']
        if _covered(events,start,row['end']) or structural(contexts[start]): continue
        definition=_definition(text,row,language)
        if not definition: continue
        if definition['invalid_pipe']:
            diagnostics.append(_issue('foreign-roc-function-formals-review',start,'Named function pipe formals are incomplete or complex.')); continue
        end=_definition_end(text,rows,index,definition,base['pairs'],language)
        body=text[definition['body_start']:end].strip()
        definitions.append(dict(definition,end=end))
        if _scalar(body,language):
            opaque.append(_event('composed-definition',definition['start'],end,scope='value-expression'))
        elif '\n' in body and any(char in body for char in '[{') and _scalar(body.replace('\n',' '),language):
            # One composed data expression, but preserve its original data
            # rows. The signature and any first-row literal are the same row,
            # not an extra body unit added to that written declaration row.
            if definition['is_function']:
                events.append(_event('header',definition['start'],row['end'],scope='foreign-layout-data-function'))
        elif definition['is_function'] or re.search(r'\b(?:let|for|while|case|when|match|if)\b',body):
            if definition['is_function']:
                events.append(_event('header',definition['start'],definition['header_end'],scope='foreign-layout-function'))
            owners.append(dict(start=definition['start'],end=end,kind='code',owner='function'))
            regions.append(dict(start=definition['body_start'],end=end,scope='layout-function'))
            own_tokens=_top_tokens(body)
            if re.search(r'\b(?:if|match|when|case)\b',body) and own_tokens and own_tokens[0]['word'] not in {'if','match','when','case','let'} and not any(t['word'] in {'=','<-','let'} for t in own_tokens):
                diagnostics.append(_issue('foreign-layout-nested-value-composition-review',definition['body_start'],'Conditional/pattern composition inside a call/data operand needs its own grammar proof; no scalar collapse.'))
            body_rows=[line.strip() for line in body.splitlines() if line.strip()]
            independent_fsharp_rows=language=='fsharp' and len(body_rows)>1 and all(_scalar(line,language) for line in body_rows)
            if not independent_fsharp_rows and not re.search(r'\b(?:let|for|while|case|when|match|if|async|task|try)\b|<-|(?<![=!<>:])=(?!=|>)',body) and '\n' in body and not any(c in body for c in '[{'):
                diagnostics.append(_issue('foreign-layout-multiline-expression-review',definition['body_start'],'Multiline application/action ownership is not proven; no opaque collapse.'))
    # Nested pure definitions are subsumed only by a proven pure outer body.
    opaque=sorted(opaque,key=lambda e:(e['start'],-e['end']))
    opaque=[event for index,event in enumerate(opaque) if not any(old['start']<=event['start'] and event['end']<=old['end'] for old in opaque[:index])]
    def effective_at(index):
        active=[r for r in regions if r['start']<=index<r['end']]
        if not active: return contexts[index]
        reset=max(r['start'] for r in active)
        return (('layout','code',reset),)+tuple(frame for frame in contexts[index] if frame[2]>=reset)
    effective=[effective_at(i) for i in range(len(text))]
    lambda_events,lambda_regions,lambda_issues=_lambda_events(text,language,effective,opaque+events,rows,base['pairs'])
    events+=lambda_events; regions+=lambda_regions; diagnostics+=lambda_issues
    for region in lambda_regions:
        for index in range(region['start'],region['end']):
            effective[index]=(('layout','code',region['start']),)+tuple(frame for frame in effective[index] if frame[2]>=region['start'])
    controls,issues=_control_events(text,language,effective,opaque+events,rows)
    events+=controls; diagnostics+=issues
    if language in {'elm','fsharp'}:
        let_events,issues=_local_scope_events(text,language,effective,opaque+events); events+=let_events; diagnostics+=issues
    # Drop only events fully inside a proven composed expression. Partial
    # overlaps are left for the accounting engine to diagnose, never guessed.
    events=[event for event in events if not _covered(opaque,event['start'],event['end'])]+opaque
    if language=='fsharp':
        for close in list(events):
            if close['kind']!='close' or close['scope']!='foreign-fsharp-computation': continue
            cursor=close['end']
            while cursor<len(text) and text[cursor].isspace(): cursor+=1
            frame=effective[cursor][-1] if cursor<len(effective) and effective[cursor] else None
            if text.startswith('|]',cursor) and frame and frame[0]=='[' and frame[1] in {'data','bare-code'}:
                events.append(_event('structural-suffix',close['end'],cursor+2,units=0,scope='foreign-fsharp-array-suffix'))
    # A formal record pattern may have an identifier immediately before its
    # brace. Only a proven definition/lambda signature discharges that gate.
    diagnostics=[issue for issue in diagnostics if not (issue['kind']=='foreign-fsharp-computation-builder-review' and
        any(e['kind']=='header' and e['start']<=issue['offset']<e['end'] for e in events)) and
        not (issue['kind']=='foreign-fsharp-computation-builder-review' and _covered(opaque,issue['offset'],issue['offset']+1))]
    # Pattern/formal/header operators are structural. Executable body regions
    # reset inherited array/call data, including F# async within [|...|].
    for event in events:
        if event['kind'] in {'header','branch','attribute'}:
            for index in range(event['start'],event['end']): effective[index]=effective[index]+(('header','parameters',event['start']),)
    unsupported={
        'fsharp':r'\b(?:object|interface|abstract|inherit|override|and|exception|val|function)\b|\b(?:and|use|let|match)!(?=\s|$)',
        'roc':r'\b(?:expect|implements|where)\b',
        'elm':r'\b(?:infix|infixl|infixr)\b',
    }
    for match in re.finditer(unsupported[language],text):
        if language=='fsharp' and match[0]=='val' and any(start<=match.start()<end for start,end in val_fields): continue
        if not _covered(opaque,match.start(),match.end()): diagnostics.append(_issue('foreign-layout-complex-declaration-review',match.start(),'This declaration/computation form needs a dedicated source-specific grammar review.'))
    if language!='fsharp':
        for match in re.finditer(';',text): diagnostics.append(_issue('foreign-layout-semicolon-review',match.start(),'Semicolon sequencing is not part of this proposed Roc/Elm grammar.'))
    return dict(contexts=effective,events=events,diagnostics=diagnostics,owners=owners,executable_regions=regions,
        value_spans=opaque,grammar_language=language)


def binding_counter(language):
    if language not in LANGUAGES: raise ValueError('No declaration grammar alias is permitted.')
    def counter(*,text,original,context,assignments):
        if structural(context,assignment=True,named_equals=True): return dict(reason='structural-record-list-tuple-formal-or-argument-data')
        value=text.strip()
        if language=='fsharp':
            declaration=re.match(r'^let\s+(?:mutable\s+)?(?P<names>'+IDENT+r'(?:\s*,\s*'+IDENT+r')*)\s*=',value)
            if declaration and ',' in declaration['names']:
                # F# tuple destructuring is one producer; no per-target split.
                return dict(units=1,declaration_bindings=1,reason='one-tuple-producer-destructuring-binding')
        # Ordinary function/value definitions have one binding. Independent
        # mutable writes and genuine semicolon sequences remain separate.
        return {}
    return counter


def normalize(source,language):
    masked=mask_layout(source,language)
    classified=classify_scopes(masked['cleaned'],language,masked['directives'])
    result=count_owned(source,masked['cleaned'],classified['contexts'],classified['events'],
        masked['diagnostics']+classified['diagnostics'],language,binding_counter=binding_counter(language),
        named_equals=True,assignment_pattern=ASSIGN)
    result.update(proposal_version=PROPOSAL_VERSION,proposal_only=True,proposal_status='unhooked-no-admission',
        owners=classified['owners'],executable_regions=classified['executable_regions'],value_spans=classified['value_spans'],
        interpolation_evidence=masked['interpolation_evidence'],
        admission='Not admitted: verify exact source/producer pins, locked fixtures, and all source-specific diagnostics before any chart overlay.')
    return result
