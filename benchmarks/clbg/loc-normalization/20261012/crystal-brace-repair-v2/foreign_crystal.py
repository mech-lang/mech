"""Unhooked Crystal source-unit proposal with independent lexical ownership.

This is Crystal grammar, not a Ruby alias. The shared foreign_core module only
accounts for supplied offset-preserving owner spans. No benchmark name, input,
producer count, source/report I/O, collector, or chart admission is used here.
Run imports, fixtures and source replay only under the common timing mutex.
"""
import re

from foreign_core import brace_pairs, count_owned, replacement, structural, top_parts


LANGUAGES = {'crystal'}
PROPOSAL_VERSION = 'foreign-crystal-unhooked-20261012-v2'
IDENT = r'(?:@@?|\$)?[A-Za-z_]\w*[!?]?'
ASSIGN = re.compile(r'(?<![=!<>~:])(?:\*\*=|<<=|>>=|//=|\|\|=|&&=|[+*/%&|^-]=|=(?!=|>|~))')
WORDS = re.compile(r'\b[A-Za-z_]\w*[!?]?\b')
SCALAR_FORBIDDEN = re.compile(r'\b(?:def|class|struct|module|lib|macro|if|unless|case|when|while|until|for|do|begin|end|return|break|next|yield|rescue|ensure)\b')
TYPE_HEADERS = {'class','struct','module','lib','enum','annotation'}
CONTROL_HEADERS = {'if','unless','while','until','for','case','select'}
BRANCHES = {'elsif','else','when','rescue','ensure'}


def _issue(kind, offset, message):
    return dict(kind=kind, offset=offset, message=message)


def _event(kind, start, end, units=1, scope='foreign-crystal'):
    return dict(kind=kind, start=start, end=end, units=units, scope=scope)


def _prepared(source):
    return ' ' + source[1:] if source.startswith('\ufeff') else source


def _quoted_end(source, start, quote, diagnostics, literal_spans, interpolation=True):
    """Find a Crystal quote while proving/gating interpolation code separately."""
    cursor = start + 1
    while cursor < len(source):
        if source[cursor] == '\\':
            cursor += 2
        elif interpolation and source.startswith('#{',cursor):
            end, cleaned = _interpolation_end(source,cursor+2,diagnostics,literal_spans)
            expression = cleaned.strip()
            if not expression or ASSIGN.search(cleaned) or ';' in cleaned or SCALAR_FORBIDDEN.search(cleaned) or '->' in cleaned:
                diagnostics.append(_issue('foreign-crystal-interpolation-code-review',cursor,
                    'Interpolation contains independent work or unproved executable ownership.'))
            elif re.search(r'(?<![\w])(?:\{|\})(?![\w])',cleaned):
                diagnostics.append(_issue('foreign-crystal-interpolation-code-review',cursor,
                    'Interpolation containing tuple/hash/block braces needs explicit expression ownership.'))
            cursor = end
        elif source[cursor] == quote:
            return cursor + 1
        else:
            cursor += 1
    diagnostics.append(_issue('foreign-unclosed-literal',start,'Crystal quoted literal has no matching quote.'))
    return len(source)


def _interpolation_end(source, start, diagnostics, literal_spans):
    """Balance interpolation, including nested quotes, without exposing literals."""
    chars = list(source)
    cursor, depth = start, 1
    while cursor < len(source):
        if source[cursor] in '\"\'':
            end = _quoted_end(source,cursor,source[cursor],diagnostics,literal_spans,
                interpolation=source[cursor]=='\"')
            chars[cursor:end] = replacement(source[cursor:end],literal=True)
            literal_spans.append(dict(start=cursor,end=end,kind='interpolation-literal'))
            cursor = end
        elif source[cursor] == '#':
            end = source.find('\n',cursor)
            end = len(source) if end < 0 else end
            chars[cursor:end] = replacement(source[cursor:end]); cursor = end
        elif source[cursor] == '{':
            depth += 1; cursor += 1
        elif source[cursor] == '}':
            depth -= 1
            if not depth:
                return cursor + 1, ''.join(chars[start:cursor])
            cursor += 1
        else:
            cursor += 1
    diagnostics.append(_issue('foreign-crystal-unclosed-interpolation',start-2,'Crystal interpolation has no written close.'))
    return len(source), ''.join(chars[start:])


def _regex_start(cleaned, index):
    """Proven prefix/data/call-argument positions; slash division stays visible."""
    prefix = cleaned[:index].rstrip()
    if not prefix or prefix[-1] in '=([{,:;':
        return True
    return bool(re.search(r'(?:\b(?:return|when|then)|[=!]~)\s*$',prefix))


def _regex_end(source, start, diagnostics, literal_spans):
    cursor, in_class = start+1, False
    while cursor < len(source):
        char = source[cursor]
        if char == '\\':
            cursor += 2
        elif source.startswith('#{',cursor):
            end, expression = _interpolation_end(source,cursor+2,diagnostics,literal_spans)
            if ASSIGN.search(expression) or ';' in expression or SCALAR_FORBIDDEN.search(expression):
                diagnostics.append(_issue('foreign-crystal-interpolation-code-review',cursor,
                    'Regex interpolation contains independent or unproved code.'))
            cursor = end
        elif char == '[':
            in_class = True; cursor += 1
        elif char == ']' and in_class:
            in_class = False; cursor += 1
        elif char == '/' and not in_class:
            cursor += 1
            while cursor < len(source) and source[cursor] in 'imx': cursor += 1
            return cursor
        elif char in '\r\n':
            diagnostics.append(_issue('foreign-crystal-regex-ownership-review',start,
                'An unescaped multiline slash form requires a dedicated regex rule.'))
            return cursor
        else:
            cursor += 1
    diagnostics.append(_issue('foreign-unclosed-literal',start,'Crystal regex literal has no matching slash.'))
    return len(source)


def _percent_end(source,start,diagnostics):
    match = re.match(r'%(?:[qQrwWiIx])?([^\w\s])',source[start:])
    if not match:
        return None
    opener = match[1]
    closer = {'(' : ')','[':']','{':'}','<':'>'}.get(opener,opener)
    cursor, depth = start+match.end(), 1
    while cursor < len(source):
        if source[cursor]=='\\': cursor+=2; continue
        if opener!=closer and source[cursor]==opener: depth+=1
        if source[cursor]==closer:
            depth-=1
            if not depth: return cursor+1
        cursor+=1
    diagnostics.append(_issue('foreign-unclosed-literal',start,'Crystal percent literal has no matching delimiter.'))
    return len(source)


def mask_crystal(source):
    """Mask exact positions; unsupported literal/metaprogramming forms are gated."""
    source = _prepared(source)
    chars = list(source)
    diagnostics, literal_spans, comment_spans, directives = [], [], [], []
    cursor = 0
    while cursor < len(source):
        start, char = cursor,source[cursor]
        if char == '#':
            end = source.find('\n',cursor)
            cursor = len(source) if end < 0 else end
            chars[start:cursor] = replacement(source[start:cursor])
            comment_spans.append(dict(start=start,end=cursor,kind='line-comment'))
        elif char in '\"\'':
            cursor = _quoted_end(source,start,char,diagnostics,literal_spans,interpolation=char=='\"')
            chars[start:cursor] = replacement(source[start:cursor],literal=True)
            literal_spans.append(dict(start=start,end=cursor,kind='string' if char=='\"' else 'character'))
            if char=="'" and cursor<=len(source) and source[cursor-1:cursor]=="'":
                content=source[start+1:cursor-1]
                if len(content)!=1 and not re.fullmatch(r'\\(?:[abefnrtv0\\\'\"]|x[0-9A-Fa-f]{2}|u[0-9A-Fa-f]{4}|u\{[0-9A-Fa-f]+\})',content):
                    diagnostics.append(_issue('foreign-crystal-character-review',start,
                        'Single-quoted form is not one proven character or escape.'))
        elif source.startswith('//',cursor):
            # Crystal floor division is an operator, never a C++ line comment.
            cursor += 2
            if cursor < len(source) and source[cursor]=='=': cursor += 1
        elif char=='/' and _regex_start(''.join(chars[:cursor]),cursor):
            cursor = _regex_end(source,start,diagnostics,literal_spans)
            chars[start:cursor] = replacement(source[start:cursor],literal=True)
            literal_spans.append(dict(start=start,end=cursor,kind='regex'))
        elif char=='/' and re.search(r'(?m)(?:^|;)\s*[A-Za-z_]\w*[!?]?[ \t]+$',source[:cursor]) and source.find('/',cursor+1,source.find('\n',cursor) if source.find('\n',cursor)>=0 else len(source))>=0:
            # A bare-call regex and arithmetic division can share this spelling.
            # Masking is conservative and gated, never a silent lexer waiver.
            diagnostics.append(_issue('foreign-crystal-regex-ownership-review',start,
                'Bare-call slash literal versus division needs explicit expression ownership.'))
            cursor = _regex_end(source,start,diagnostics,literal_spans)
            chars[start:cursor] = replacement(source[start:cursor],literal=True)
            literal_spans.append(dict(start=start,end=cursor,kind='ambiguous-slash-form'))
        elif char=='%' and _regex_start(''.join(chars[:cursor]),cursor):
            end = _percent_end(source,start,diagnostics)
            if end is None:
                cursor += 1
            else:
                diagnostics.append(_issue('foreign-crystal-percent-literal-review',start,
                    'Percent literal interpolation/word-array ownership requires a focused rule.'))
                cursor = end
                chars[start:cursor] = replacement(source[start:cursor],literal=True)
                literal_spans.append(dict(start=start,end=cursor,kind='percent-literal'))
        elif source.startswith('<<-',cursor):
            opener = re.match(r'<<-([A-Za-z_]\w*)',source[cursor:])
            if opener:
                newline = source.find('\n',cursor)
                terminal = re.search(r'(?m)^\s*'+re.escape(opener[1])+r'\s*$',source[newline+1:]) if newline>=0 else None
                cursor = len(source) if terminal is None else newline+1+terminal.end()
                chars[start:cursor] = replacement(source[start:cursor],literal=True)
                literal_spans.append(dict(start=start,end=cursor,kind='heredoc'))
                diagnostics.append(_issue('foreign-crystal-heredoc-review',start,
                    'Heredoc interpolation and terminator ownership require explicit review.'))
            else:
                cursor += 1
        else:
            cursor += 1
    cleaned = ''.join(chars)
    pairs, _ = brace_pairs(cleaned)
    for match in re.finditer(r'@\[',cleaned):
        opening = match.end()-1
        end = pairs.get(opening)
        if end is None:
            diagnostics.append(_issue('foreign-crystal-annotation-review',match.start(),'Annotation has no proven matching delimiter.'))
        else:
            directives.append(_event('pragma',match.start(),end+1,scope='foreign-crystal-annotation'))
            diagnostics.append(_issue('foreign-crystal-annotation-review',match.start(),
                'Meaningful annotation retained; exact compiler/source role must be reviewed.'))
    return dict(cleaned=cleaned,diagnostics=diagnostics,literal_spans=literal_spans,
        comment_spans=comment_spans,directives=directives)


def _header_end(text,start,pairs,stop_words=(),*,postfix=False):
    """A top-level newline/semicolon/then/do closes a written header."""
    cursor = start
    while cursor < len(text):
        char = text[cursor]
        if char in '([' and cursor in pairs:
            cursor = pairs[cursor]+1; continue
        if char in '\r\n;':
            return cursor
        if postfix and char=='}':return cursor
        if char=='{' and cursor in pairs:
            return cursor
        word = WORDS.match(text,cursor)
        if word:
            if postfix and word[0] in {'end','else','elsif','rescue','ensure'} and not _receiver_keyword(text,cursor):
                return cursor
            if word[0] in stop_words: return word.end()
            cursor = word.end(); continue
        cursor += 1
    return len(text)


def _statement_start(text,index,events=()):
    start = max(text.rfind('\n',0,index),text.rfind(';',0,index))+1
    # A parent executable opening/header on the same row starts its body.
    for event in events:
        if event['end'] <= index and event['end'] > start:
            start = event['end']
    while start < index and text[start].isspace(): start += 1
    return start


def _callback_start(text,opening,pairs,events):
    start = _statement_start(text,opening,events)
    # The proved containing call belongs to the callback header; its structural
    # prefix must not become another statement. Prior executable owner events
    # still delimit an actual nested callback's immediate header.
    if not text[start:opening].strip() and start:
        previous_start=text.rfind('\n',0,start-1)+1
        previous=text[previous_start:start].strip()
        if re.fullmatch(r'[^;{}\n]*->\s*\([^{};]*\)',previous):
            start=previous_start
    while start < opening and text[start].isspace(): start += 1
    return start


def _brace_owners(text,pairs):
    kinds, formals, diagnostics = {}, {}, []
    for opening,closing in sorted(pairs.items()):
        if text[opening]!='{': continue
        cursor=opening+1
        while cursor<closing and text[cursor].isspace(): cursor+=1
        explicit = cursor<closing and text[cursor]=='|' and not text.startswith('||',cursor)
        prefix = text[:opening].rstrip()
        declaration=re.search(r'\b(?:property|getter|setter)[!?]?\s*$',prefix)
        if declaration and not _receiver_keyword(text,declaration.start()):
            kinds[opening]='data'
            diagnostics.append(_issue('foreign-crystal-property-declaration-review',declaration.start(),
                'Property/getter/setter brace has no proven independently named declaration owner.'))
            continue
        # A newline/semicolon ends the written preceding expression. Looking
        # across it makes a standalone tuple after `end` look like a callback
        # of that keyword. Keep the existing explicitly written wrapped lambda
        # continuation, whose preceding `->(formals)` is an actual owner.
        boundary = max(text.rfind('\n',0,opening),text.rfind(';',0,opening))+1
        callable_prefix = text[boundary:opening].rstrip()
        if not callable_prefix.strip() and boundary:
            previous_start = text.rfind('\n',0,boundary-1)+1
            previous = text[previous_start:boundary].strip()
            if re.fullmatch(r'[^;{}\n]*->\s*\([^{};]*\)',previous):
                callable_prefix = previous
        trailing = re.search(r'(?:'+IDENT+r'|\))\s*$',callable_prefix)
        data_prefix = bool(re.search(r'(?:\b(?:return|yield|when)|[=,(\[{:]|=>)\s*$',callable_prefix))
        callback = explicit or (trailing and not data_prefix)
        # A literal tuple starts at a statement position, not after a callable.
        if not callback:
            kinds[opening]='data'; continue
        kinds[opening]='code'
        if explicit:
            end=text.find('|',cursor+1,closing)
            if end<0:
                diagnostics.append(_issue('foreign-crystal-callback-formals-review',cursor,'Callback formal group has no closing pipe.'))
                formals[opening]=opening+1
            else:
                formals[opening]=end+1
                content=text[cursor+1:end]
                if ASSIGN.search(content) or ';' in content:
                    diagnostics.append(_issue('foreign-crystal-callback-formals-review',cursor,
                        'Callback defaults/semicolon formals need a dedicated signature rule.'))
        else:
            formals[opening]=opening+1
        if not callable_prefix.strip() or not trailing and '->' not in callable_prefix[max(0,len(callable_prefix)-80):]:
            diagnostics.append(_issue('foreign-crystal-callback-owner-review',opening,
                'Pipe block has no proven callable or explicit lambda owner.'))
    return kinds,formals,diagnostics


def _outside_headers(index,events):
    return not any(event['start'] <= index < event['end'] for event in events)


def _receiver_keyword(text,start):
    return bool(re.search(r'(?:\.|::)\s*$',text[:start]))


def classify_scopes(cleaned,language='crystal',literal_spans=(),directives=()):
    """Supply explicit Crystal keyword/braced executable and structural owners."""
    if language not in LANGUAGES:
        raise ValueError('Only explicit Crystal grammar is proposed.')
    chars=list(cleaned)
    for event in directives:
        chars[event['start']:event['end']]=replacement(cleaned[event['start']:event['end']])
    text=''.join(chars)
    pairs,diagnostics=brace_pairs(text)
    kinds,formals,brace_issues=_brace_owners(text,pairs)
    diagnostics.extend(brace_issues)
    events=list(directives)
    frames,stack,callbacks=[],[],[]
    # Brace callbacks are independently owned even inside outer data/arguments.
    for opening in sorted(kinds):
        if kinds[opening]!='code': continue
        start=_callback_start(text,opening,pairs,events)
        end=formals[opening]
        events.append(_event('header',start,end,scope='foreign-crystal-callback'))
        closing=pairs[opening]
        events.append(_event('close',closing,closing+1,scope='foreign-crystal-callback'))
        frames.append(dict(start=opening,body_start=end,body_end=closing,end=closing+1,kind='code',owner='callback'))
        callbacks.append((opening,closing))
    matches=list(WORDS.finditer(text))
    cursor=0
    while cursor<len(matches):
        token=matches[cursor]; word=token[0]; start=token.start()
        cursor+=1
        if not _outside_headers(start,events) or _receiver_keyword(text,start): continue
        # Keywords embedded in a literal tuple/type/formal are data. Recognized
        # executable begin/if operands receive a gate rather than disappear.
        relevant=[opening for opening,closing in pairs.items() if opening<start<closing and text[opening] in '([{']
        last_code=max((opening for opening in relevant if kinds.get(opening)=='code'),default=-1)
        structural_owner=any(opening>last_code and (text[opening] in '([' or kinds.get(opening)=='data') for opening in relevant)
        if structural_owner and word not in CONTROL_HEADERS|{'begin','do'}:
            continue
        prefix_start=_statement_start(text,start,events)
        prefix=text[prefix_start:start].strip()
        if word=='end':
            if not stack:
                diagnostics.append(_issue('foreign-unmatched-keyword-close',start,'Written end has no proven Crystal owner.'))
            else:
                owner=stack.pop(); owner.update(body_end=start,end=token.end()); frames.append(owner)
                if owner['owner']=='do': callbacks.append((owner['start'],token.end()-1))
            events.append(_event('close',start,token.end()))
            continue
        if word in TYPE_HEADERS|{'def','macro'}:
            if prefix and prefix not in {'private','protected','abstract'}:
                diagnostics.append(_issue('foreign-crystal-declaration-owner-review',start,'Declaration follows an unproved expression prefix.'))
            end=_header_end(text,token.end(),pairs)
            events.append(_event('header',prefix_start if prefix in {'private','protected','abstract'} else start,end))
            stack.append(dict(owner=word,start=end,body_start=end,kind='type' if word in TYPE_HEADERS else 'code'))
            if word=='macro':
                diagnostics.append(_issue('foreign-crystal-macro-review',start,'Written macro body retained; expansion/compiler ownership is not yet qualified.'))
            if 'abstract'==prefix and word=='def':
                stack.pop()
            continue
        if word=='fun':
            end=_header_end(text,token.end(),pairs)
            events.append(_event('declaration',start,end,scope='foreign-crystal-ffi'))
            if not any(owner['owner']=='lib' for owner in stack):
                diagnostics.append(_issue('foreign-crystal-ffi-owner-review',start,'FFI fun prototype is outside a proven lib owner.'))
            continue
        if word=='begin':
            events.append(_event('open',start,token.end(),units=0))
            stack.append(dict(owner=word,start=token.end(),body_start=token.end(),kind='bare-code'))
            continue
        if word=='do':
            # for/while ... do is part of the already recognized loop header.
            end=token.end(); after=end
            while after<len(text) and text[after].isspace() and text[after] not in '\r\n': after+=1
            if after<len(text) and text[after] in '\r\n':
                following=after
                while following<len(text) and text[following].isspace(): following+=1
                if following<len(text) and text[following]=='|':
                    diagnostics.append(_issue('foreign-crystal-callback-formals-review',following,
                        'Wrapped do formal-group ownership requires a dedicated signature rule.'))
            if after<len(text) and text[after]=='|':
                close=text.find('|',after+1)
                if close<0:
                    diagnostics.append(_issue('foreign-crystal-callback-formals-review',after,'do callback has no formal closing pipe.'))
                else:
                    end=close+1
                    if ASSIGN.search(text[after+1:close]) or ';' in text[after+1:close]:
                        diagnostics.append(_issue('foreign-crystal-callback-formals-review',after,'do callback defaults need explicit review.'))
            header_start=_callback_start(text,start,pairs,events)
            if header_start==start:
                diagnostics.append(_issue('foreign-crystal-callback-owner-review',start,'do has no proven callable prefix.'))
            events.append(_event('header',header_start,end,scope='foreign-crystal-callback'))
            stack.append(dict(owner='do',start=end,body_start=end,kind='code'))
            continue
        if word in CONTROL_HEADERS:
            end=_header_end(text,token.end(),pairs,{'then'} if word in {'if','unless'} else {'do'} if word in {'while','until','for'} else ())
            if word in {'if','unless','while','until'} and prefix and not re.search(r'(?:=|\(|\[|,|return)\s*$',prefix):
                end=_header_end(text,token.end(),pairs,postfix=True)
                events.append(_event('header',start,end,scope='foreign-crystal-postfix-control'))
                continue
            events.append(_event('header',start,end,scope='foreign-crystal-control'))
            stack.append(dict(owner=word,start=end,body_start=end,kind='code'))
            if prefix or structural_owner:
                diagnostics.append(_issue('foreign-crystal-value-control-review',start,
                    'Operand if/case/control needs a proved expression-only owner; independent work is retained.'))
            continue
        if word in BRANCHES:
            if not stack or stack[-1]['owner'] not in CONTROL_HEADERS|{'begin','def'}:
                diagnostics.append(_issue('foreign-crystal-branch-owner-review',start,'Branch has no proven compatible keyword owner.'))
            end=_header_end(text,token.end(),pairs,{'then'}) if word in {'elsif','when','rescue'} else token.end()
            events.append(_event('branch',start,end))
            continue
        if word=='then':
            diagnostics.append(_issue('foreign-crystal-then-owner-review',start,'then is outside a proved header.'))
        if word in {'out','pointerof','sizeof','typeof','instance_sizeof','offsetof'}:
            # Ordinary unary/type expressions are deliberately not new scopes.
            continue
    for owner in stack:
        owner.update(body_end=len(text),end=len(text)); frames.append(owner)
        diagnostics.append(_issue('foreign-unclosed-keyword-owner',owner['start'],'Crystal keyword owner has no recognized written end.'))
    # Generic macro source must not be masked as tuple data or string content.
    for match in re.finditer(r'\{\{|\{%|%\}',text):
        diagnostics.append(_issue('foreign-crystal-macro-review',match.start(),'Crystal compile-time macro delimiters require explicit ownership.'))
    # A following actual operation/argument is not a structural callback suffix.
    for opening,closing in callbacks:
        following=re.match(r'\s*([^\s])',text[closing+1:])
        if following and following[1] not in ';,)]}\n' and '\n' not in following[0][:-1]:
            diagnostics.append(_issue('foreign-crystal-callback-composition-review',closing+1+following.end()-1,
                'Following callback composition requires outer-expression ownership review.'))
        suffix=re.match(r'[ \t]*[)\]]*[ \t]*,[ \t]*([^\s)\]}])',text[closing+1:])
        if suffix:
            diagnostics.append(_issue('foreign-crystal-callback-composition-review',closing+1+suffix.end()-1,
                'A real following argument/data expression requires outer owner review.'))
    header_spans=[(event['start'],event['end']) for event in events if event['kind'] in {'header','declaration','branch'}]
    contexts,delimiters=[],[]
    for index,char in enumerate(text):
        active=[('keyword',frame['kind'],frame['start']) for frame in frames if frame['start']<=index<frame['end']]
        protected=[('header','parameters',start) for start,end in header_spans if start<=index<end]
        contexts.append(tuple(sorted(active+delimiters+protected,key=lambda entry:entry[2])))
        if char=='(':
            before=re.search(r'(?:'+IDENT+r'|\))\s*$',text[:index])
            delimiters.append((char,'args' if before else 'data',index))
        elif char=='[':
            delimiters.append((char,'data',index))
        elif char=='{':
            delimiters.append((char,kinds.get(index,'data'),index))
        elif char in ')]}' and delimiters:
            wanted={')':'(',']':'[','}':'{'}[char]
            if delimiters[-1][0]==wanted: delimiters.pop()
    executable_regions=[dict(start=frame['body_start'],end=frame['body_end'],scope='foreign-crystal') for frame in frames if frame['body_start']<frame['body_end']]
    return dict(contexts=contexts,events=events,diagnostics=diagnostics,
        executable_regions=executable_regions,frames=frames,brace_kinds=kinds)


def binding_counter(*,text,original,context,assignments):
    """Written property/field declarations differ from header formals/data."""
    if structural(context,assignment=True,named_equals=True):
        return {}
    code=text.strip()
    property_decl=re.match(r'(?:(?:private|protected)\s+)?(?:property|getter|setter)(?:[!?])?\s+(.+)$',code,re.S)
    if property_decl:
        content=property_decl[1]
        # Each independent written name counts, never its generated accessors.
        names=top_parts(content)
        accepted=[]
        for part in names:
            match=re.fullmatch(r'('+IDENT+r')(?:\s*:\s*[^=]+)?(?:\s*=\s*.+)?',part,re.S)
            if not match:
                return dict(diagnostics=[dict(kind='foreign-crystal-property-declaration-review',
                    message='Property/getter/setter declaration has an unproved name/type/initializer owner.')])
            accepted.append(match[1])
        return dict(units=len(accepted),declaration_bindings=len(accepted),reason='independently-written-property-names-no-generated-work')
    typed=re.fullmatch(r'('+IDENT+r'(?:\s*,\s*'+IDENT+r')*)\s*:\s*([^=]+)',code,re.S)
    if typed:
        names=top_parts(typed[1])
        return dict(units=len(names),declaration_bindings=len(names),reason='independently-written-typed-storage')
    if code.startswith('alias '):
        return dict(units=1,declaration_bindings=1,reason='named-type-alias')
    if re.match(r'^(?:include|extend|require)\b',code):
        return dict(units=1,reason='written-import-or-type-composition')
    # Explicit tuple/swap binding has one producer; equals chains remain visible.
    if len(assignments)==1 and ',' in code[:code.find(assignments[0]['operator'])]:
        return dict(units=1,declaration_bindings=1,reason='explicit-tuple-binding-single-producer')
    return {}


def normalize(source,language='crystal'):
    if language not in LANGUAGES:
        raise ValueError('Only explicit Crystal grammar is proposed.')
    masked=mask_crystal(source)
    owners=classify_scopes(masked['cleaned'],language,
        literal_spans=masked['literal_spans'],directives=masked['directives'])
    metric=count_owned(source,masked['cleaned'],owners['contexts'],owners['events'],
        masked['diagnostics']+owners['diagnostics'],language,binding_counter=binding_counter,
        executable_regions=owners['executable_regions'],named_equals=True,assignment_pattern=ASSIGN)
    metric.update(proposal_only=True,proposal_status='unhooked-no-admission',
        proposal_version=PROPOSAL_VERSION,literal_spans=masked['literal_spans'],
        comment_spans=masked['comment_spans'],ownership_frames=owners['frames'])
    return metric
