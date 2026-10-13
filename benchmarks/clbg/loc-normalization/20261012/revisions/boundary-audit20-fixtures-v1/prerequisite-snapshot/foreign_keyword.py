"""Unhooked Ada/Pascal/Fortran keyword-owner proposal; no chart admission.

Grammar decisions live here, not in an alias to an existing language parser.
foreign_core supplies Ada/Fortran masks and shared accounting; Pascal uses its
explicit quoted-string/comment/directive masker to avoid restoring comment text.
All metrics require a later locked fixture/source/pin review. No producer,
collector, source mutation, benchmark kernel, or admission hook is included.
"""
import re

from foreign_core import brace_pairs, count_owned, mask_foreign, replacement, structural, top_parts


LANGUAGES = {'ada','pascal','fortran'}
LEXERS = {'ada':'ada','fortran':'fortran'}
PROPOSAL_VERSION = 'foreign-keyword-unhooked-20261012-v1'
IDENT = r'[A-Za-z_]\w*'
COLON_ASSIGN = re.compile(r':=')
FORTRAN_ASSIGN = re.compile(r'(?<![=!<>/])(?:=>|=(?!=|>))')
TOKENS = re.compile(r'\b[A-Za-z_]\w*\b|:=|=>|::|<=|>=|<>|==|/=|[^\s]')


def _issue(kind, offset, message):
    return dict(kind=kind,offset=offset,message=message)


def _event(kind,start,end,units=1,scope='foreign-keyword'):
    return dict(kind=kind,start=start,end=end,units=units,scope=scope)


def _mask_pascal(source):
    """Exact quoted strings/comments/directives, without restoring comment text."""
    chars=list(source); diagnostics=[]; directives=[]; cursor=0
    if source.startswith('\ufeff'): chars[0]=' '
    while cursor<len(source):
        start=cursor
        if source[cursor]=="'":
            cursor+=1; complete=False
            while cursor<len(source):
                if source[cursor]=="'":
                    if source.startswith("''",cursor): cursor+=2; continue
                    cursor+=1; complete=True; break
                cursor+=1
            chars[start:cursor]=replacement(source[start:cursor],literal=True)
            if not complete: diagnostics.append(_issue('foreign-unclosed-literal',start,'Pascal string has no matching quote.'))
        elif source.startswith('//',cursor):
            end=source.find('\n',cursor); cursor=len(source) if end<0 else end
            chars[start:cursor]=replacement(source[start:cursor])
        elif source.startswith(('{$','(*$'),cursor):
            close='}' if source[cursor]=='{' else '*)'
            end=source.find(close,cursor+(2 if close=='}' else 3))
            cursor=len(source) if end<0 else end+len(close)
            directives.append(_event('pragma',start,cursor,scope='foreign-pascal-directive'))
            value=source[start:cursor]
            if end<0: diagnostics.append(_issue('foreign-unclosed-directive',start,'Pascal directive has no matching delimiter.'))
            if re.match(r'(?:\{\$|\(\*\$)\s*(?:if|else|endif|include|i\b)',value,re.I):
                diagnostics.append(_issue('foreign-conditional-directive-review',start,'Conditional/include directives require exact compilation-policy review.'))
        elif source[cursor]=='{' or source.startswith('(*',cursor):
            stack=['}' if source[cursor]=='{' else '*)']; cursor+=1 if stack[-1]=='}' else 2
            nested=False
            while cursor<len(source) and stack:
                if source.startswith(stack[-1],cursor): cursor+=len(stack.pop())
                elif source[cursor]=='{' or source.startswith('(*',cursor):
                    stack.append('}' if source[cursor]=='{' else '*)'); cursor+=1 if stack[-1]=='}' else 2; nested=True
                else: cursor+=1
            chars[start:cursor]=replacement(source[start:cursor])
            if stack: diagnostics.append(_issue('foreign-unclosed-comment',start,'Pascal comment has no matching close.'))
            if nested: diagnostics.append(_issue('foreign-pascal-nested-comment-mode-review',start,'Nested comment interpretation requires the exact compiler mode.'))
        else: cursor+=1
    return dict(cleaned=''.join(chars),diagnostics=diagnostics,directives=directives)


def mask_keyword(source, language):
    if language not in LANGUAGES:
        raise ValueError('Only explicit Ada/Pascal/Fortran grammar is proposed.')
    if language=='pascal': return _mask_pascal(source)
    # Fortran's sentinel directives are meaningful, unlike ordinary ! comments.
    prefixes = ('!$omp','!$OMP','#') if language=='fortran' else ()
    masked = mask_foreign(source,LEXERS[language],semantic_directive_prefixes=prefixes)
    diagnostics, directives = list(masked['diagnostics']), list(masked.get('directives',[]))
    based_literals=[]
    if language=='ada':
        # Pinned Pygments reports the # delimiters of valid Ada based literals
        # as Error. Reconcile only a complete literal whose digits fit its base;
        # malformed/invalid/unknown spans retain their original lexer gates.
        chars=list(masked['cleaned'])
        digit=r'[0-9A-Fa-f](?:_?[0-9A-Fa-f])*'
        pattern=re.compile(r'(?<![\w\'])((?:[2-9]|1[0-6]))#('+digit+r'(?:\.'+digit+r')?)#(?:[Ee][+-]?[0-9](?:_?[0-9])*)?(?!\w)')
        for match in pattern.finditer(masked['cleaned']):
            base=int(match[1]);digits=match[2].replace('_','').replace('.','')
            if any(int(char,16)>=base for char in digits):continue
            chars[match.start():match.end()]=replacement(match[0],literal=True)
            based_literals.append(dict(start=match.start(),end=match.end(),base=base,
                spelling=source[match.start():match.end()],ownership='complete-reviewed-based-literal'))
        for offset,char in enumerate(masked['cleaned']):
            if char=='#' and not any(span['start']<=offset<span['end'] for span in based_literals):
                diagnostics.append(_issue('foreign-ada-based-literal-review',offset,
                    'Unowned # delimiter has no complete valid based-literal proof; malformed literals or preprocessing remain review-gated.'))
        masked['cleaned']=''.join(chars)
        diagnostics=[d for d in diagnostics if not (d['kind']=='foreign-lexer-error' and
            any(span['start']<=d['offset'] and d['offset']+d.get('length',1)<=span['end'] for span in based_literals))]
    if language=='fortran':
        chars=list(masked['cleaned']); offset=0
        for row in source.splitlines(keepends=True):
            code=row.lstrip()
            if code.lower().startswith('!$omp'):
                start=offset+len(row)-len(code); end=offset+len(row.rstrip('\r\n'))
                chars[start:end]=source[start:end]
                if not any(e['start']==start and e['end']==end for e in directives): directives.append(_event('pragma',start,end,scope='foreign-openmp-directive'))
            offset+=len(row)
        masked['cleaned']=''.join(chars)
        for event in directives:
            value = source[event['start']:event['end']].lstrip()
            if value.startswith('#'):
                diagnostics.append(_issue('foreign-conditional-directive-review',event['start'],'Preprocessed Fortran requires exact preprocessing/source-policy review.'))
    masked.update(diagnostics=diagnostics,directives=directives,based_literal_evidence=based_literals)
    return masked


def _parse_text(cleaned,directives):
    chars = list(cleaned)
    for event in directives:
        chars[event['start']:event['end']] = replacement(cleaned[event['start']:event['end']])
    return ''.join(chars)


def _tokens(text):
    return [dict(word=m[0].lower(),start=m.start(),end=m.end()) for m in TOKENS.finditer(text)]


def _contexts(text,frames,events,language):
    """Keyword owners reset structural parents; real delimiters keep offsets."""
    contexts, delimiters = [], []
    header_spans = [(e['start'],e['end']) for e in events if e['kind'] in {'header','branch','declaration'}]
    for index,char in enumerate(text):
        active = [('keyword',f['kind'],f['start']) for f in frames if f['start']<=index<f['end']]
        active.sort(key=lambda entry:entry[2])
        header = [('header','parameters',start) for start,end in header_spans if start<=index<end]
        # Interleave by opening offset: an inner executable callback resets
        # its older outer argument/data context, not the other way around.
        contexts.append(tuple(sorted(active+delimiters+header,key=lambda entry:entry[2])))
        if char=='(':
            before = re.search(r'('+IDENT+r')\s*$',text[:index])
            word = before[1].lower() if before else ''
            kind = 'parameters' if language=='fortran' and word in {'integer','real','logical','complex','character','type','class','kind','len','dimension','intent','result','bind'} else 'args' if word else 'data'
            delimiters.append((char,kind,index))
        elif char=='[':
            delimiters.append((char,'data',index))
        elif char==')' and delimiters and delimiters[-1][0]=='(':
            delimiters.pop()
        elif char==']' and delimiters and delimiters[-1][0]=='[':
            delimiters.pop()
    return contexts


def _word_terminal(tokens,index,words,stop_at_semicolon=True):
    depth = 0
    for position in range(index,len(tokens)):
        word = tokens[position]['word']
        if word in {'(','['}: depth += 1
        elif word in {')',']'}: depth -= 1
        elif not depth:
            if word in words: return position
            if stop_at_semicolon and word==';': return None
    return None


def _statement_start(tokens,index):
    return index==0 or tokens[index-1]['word'] in {';','begin','then','else','loop','record','is','of','do','repeat',':','declare','=>'}


def _ada_declaration_start(tokens,index):
    if _statement_start(tokens,index):return index
    if index and tokens[index-1]['word']=='private':return index
    if index and tokens[index-1]['word']=='overriding':
        start=index-1
        if start and tokens[start-1]['word']=='not':start-=1
        if _statement_start(tokens,start) or (start and tokens[start-1]['word']=='private'):return start
    return None


def _ada_profiles(text,*,allow_index=False):
    """Typed formals are not arbitrary executable content inside parentheses."""
    text=text.strip();cursor=0;groups=[]
    pairs,issues=brace_pairs(text)
    if issues:return False
    while cursor<len(text):
        if text[cursor]!='(' or cursor not in pairs:return False
        groups.append(text[cursor+1:pairs[cursor]].strip());cursor=pairs[cursor]+1
        if len(groups)>2:return False
        while cursor<len(text) and text[cursor].isspace():cursor+=1
    formal=r'\s*'+IDENT+r'(?:\s*,\s*'+IDENT+r')*\s*:(?![=:])\s*(?:(?:aliased|in(?:\s+out)?|out|not\s+null|access|constant)\s+)*'+IDENT+r'(?:\.'+IDENT+r')*(?:\s*:=\s*[^{};]+)?\s*'
    def formals(value):
        return bool(value) and all(re.fullmatch(formal,part,re.I) and not re.search(r'\b(?:procedure|function|begin|end|declare|task|accept)\b|:=.*:=',part,re.I) for part in top_parts(value,';'))
    if not groups:return True
    if len(groups)==1 and formals(groups[0]):return True
    if not allow_index:return False
    index=groups[0]
    if not index or re.search(r'[:;{}]|\b(?:begin|end|declare|task|accept|procedure|function)\b',index,re.I):return False
    return len(groups)==1 or formals(groups[1])


def _ada_loop_header_start(tokens,index):
    if index>=2 and tokens[index-1]['word']==':' and re.fullmatch(IDENT,tokens[index-2]['word']) and _statement_start(tokens,index-2):
        return index-2,tokens[index-2]['word']
    return index,None


def _finish(frames,stack,token,end,events,diagnostics,wanted=None):
    if not stack:
        diagnostics.append(_issue('foreign-unmatched-keyword-close',token['start'],'Written close has no proven keyword owner.'))
    else:
        owner = stack.pop()
        if wanted and owner['owner'] not in wanted:
            diagnostics.append(_issue('foreign-keyword-owner-mismatch',token['start'],'Written close does not match its immediate owner.'))
        owner['body_end'] = token['start']
        owner['end'] = end
        frames.append(owner)
    events.append(_event('close',token['start'],end))


def _unclosed(frames,stack,end,diagnostics):
    for owner in stack:
        owner['end'] = end
        owner['body_end'] = end
        frames.append(owner)
        diagnostics.append(_issue('foreign-unclosed-keyword-owner',owner['start'],'Keyword owner has no recognized written close.'))


def _ada(text):
    tokens, events, frames, stack, diagnostics = _tokens(text), [], [], [], []
    proved_complex_keywords=set()
    index = 0
    while index<len(tokens):
        token, word = tokens[index],tokens[index]['word']
        if word=='private':
            if index+1<len(tokens) and tokens[index+1]['word']=='with' and _statement_start(tokens,index):
                terminal=_word_terminal(tokens,index+2,{';'},False)
                names=text[tokens[index+2]['start']:tokens[terminal]['start']] if terminal is not None and index+2<terminal else ''
                qualified=IDENT+r'(?:\.'+IDENT+r')*'
                if re.fullmatch(r'\s*'+qualified+r'(?:\s*,\s*'+qualified+r')*\s*',names):
                    events.append(_event('declaration',token['start'],tokens[terminal]['end'],scope='foreign-ada-context-import'))
                    index=terminal+1;continue
            if stack and stack[-1]['owner'] in {'package','task-spec'}:
                events.append(_event('section',token['start'],token['end'],units=0,scope='foreign-declaration-section'))
            else:
                diagnostics.append(_issue('foreign-ada-private-section-review',token['start'],'Private section marker has no proved package owner.'))
            index+=1;continue
        if word=='pragma' and _statement_start(tokens,index):
            terminal=_word_terminal(tokens,index+1,{';'},False)
            if terminal is None or index+1>=len(tokens) or not re.fullmatch(IDENT,tokens[index+1]['word']):
                diagnostics.append(_issue('foreign-ada-pragma-owner-review',token['start'],'Written pragma group has no complete name/argument boundary.'))
                index+=1;continue
            events.append(_event('pragma',token['start'],tokens[terminal]['end'],scope='foreign-ada-written-pragma'))
            index=terminal+1;continue
        if word=='task' and _statement_start(tokens,index):
            terminal=_word_terminal(tokens,index+1,{'is',';'},False)
            name_pos=index+1+int(index+1<len(tokens) and tokens[index+1]['word'] in {'type','body'})
            name=tokens[name_pos]['word'] if name_pos<len(tokens) else ''
            tail=text[tokens[name_pos]['end']:tokens[terminal]['start']].strip() if terminal is not None and name_pos<terminal else None
            tail_pairs,tail_issues=brace_pairs(tail or '')
            simple_tail=tail=='' or bool(tail and tail.startswith('(') and tail_pairs.get(0)==len(tail)-1 and not tail_issues and _ada_profiles(tail))
            task_body=index+1<len(tokens) and tokens[index+1]['word']=='body'
            task_type=index+1<len(tokens) and tokens[index+1]['word']=='type'
            if terminal is None or not re.fullmatch(IDENT,name) or not simple_tail or (tail and not task_type) or (task_body and tokens[terminal]['word']!='is'):
                diagnostics.append(_issue('foreign-ada-task-header-review',token['start'],'Task name/discriminant/aspect boundary has no complete bounded owner.'))
                index+=1;continue
            end=tokens[terminal]['end'];proved_complex_keywords.add(token['start'])
            if tokens[terminal]['word']==';':
                events.append(_event('declaration',token['start'],end,scope='foreign-ada-task'))
            else:
                events.append(_event('header',token['start'],end,scope='foreign-ada-task'))
                if terminal+1<len(tokens) and tokens[terminal+1]['word']=='new':
                    diagnostics.append(_issue('foreign-ada-task-interface-review',token['start'],'Task interface inheritance needs its own complete header/type owner.'))
                stack.append(dict(owner='task-body' if task_body else 'task-spec',name=name,
                    start=end,kind='code' if task_body else 'type',pending_begin=task_body))
            index=terminal+1;continue
        if word=='entry' and _ada_declaration_start(tokens,index) is not None and stack and stack[-1]['owner']=='task-spec':
            terminal=_word_terminal(tokens,index+1,{';','is'},False)
            if terminal is not None and tokens[terminal]['word']==';' and index+1<terminal and re.fullmatch(IDENT,tokens[index+1]['word']) and _ada_profiles(text[tokens[index+1]['end']:tokens[terminal]['start']],allow_index=True):
                start=tokens[_ada_declaration_start(tokens,index)]['start']
                events.append(_event('declaration',start,tokens[terminal]['end'],scope='foreign-ada-entry'))
                proved_complex_keywords.add(token['start']);index=terminal+1;continue
        accept_start=_statement_start(tokens,index) or bool(index and tokens[index-1]['word'] in {'select','or'} and stack and stack[-1]['owner']=='select')
        if word=='accept' and accept_start and any(owner['owner']=='task-body' for owner in stack):
            terminal=_word_terminal(tokens,index+1,{'do',';'},False)
            if terminal is not None and index+1<terminal and re.fullmatch(IDENT,tokens[index+1]['word']) and _ada_profiles(text[tokens[index+1]['end']:tokens[terminal]['start']],allow_index=True):
                name=tokens[index+1]['word'];end=tokens[terminal]['end']
                if stack and stack[-1]['owner']=='select':stack[-1].update(alternative_pending=False,has_accept=True)
                events.append(_event('statement' if tokens[terminal]['word']==';' else 'header',token['start'],end,scope='foreign-ada-accept'))
                if tokens[terminal]['word']=='do':stack.append(dict(owner='accept',name=name,start=end,kind='code',pending_begin=False))
                proved_complex_keywords.add(token['start']);index=terminal+1;continue
        if word=='select' and _statement_start(tokens,index) and any(owner['owner']=='task-body' for owner in stack):
            events.append(_event('header',token['start'],token['end'],scope='foreign-ada-select'))
            stack.append(dict(owner='select',start=token['end'],kind='code',pending_begin=False,alternative_pending=True,has_accept=False))
            proved_complex_keywords.add(token['start']);index+=1;continue
        if word=='or' and _statement_start(tokens,index) and stack and stack[-1]['owner']=='select':
            events.append(_event('branch',token['start'],token['end'],scope='foreign-ada-select'))
            stack[-1]['alternative_pending']=True
            index+=1;continue
        if stack and stack[-1]['owner']=='select' and stack[-1].get('alternative_pending') and word not in {'when','accept','end'}:
            if word!='terminate':diagnostics.append(_issue('foreign-ada-select-variant-review',token['start'],'Selective alternative must start with a proved accept/terminate; other entry-call/select forms are not admitted.'))
            stack[-1]['alternative_pending']=False
        if word=='end':
            cursor=index+1
            suffix=tokens[cursor]['word'] if cursor<len(tokens) else ''
            wanted={'if':{'if'},'loop':{'loop'},'case':{'case'},'record':{'record'},'select':{'select'}}.get(suffix)
            close_name=None
            if suffix and suffix!=';':
                name_start=cursor
                cursor += 1
                # The optional loop name, or a qualified named definition's
                # close, belongs to this one written closer. It is not a new
                # expression on the suffix's physical row.
                if wanted:
                    if cursor<len(tokens) and re.fullmatch(IDENT,tokens[cursor]['word']):
                        if suffix=='loop' and stack and tokens[cursor]['word']!=stack[-1].get('construct_name'):
                            diagnostics.append(_issue('foreign-ada-loop-name-review',token['start'],'Named loop close does not match its proved loop header identifier.'))
                        cursor+=1
                else:
                    while cursor+1<len(tokens) and tokens[cursor]['word']=='.' and re.fullmatch(IDENT,tokens[cursor+1]['word']):cursor+=2
                    close_name=''.join(t['word'] for t in tokens[name_start:cursor])
            if close_name and stack and stack[-1].get('name') and close_name!=stack[-1]['name']:
                diagnostics.append(_issue('foreign-ada-close-name-review',token['start'],'Named definition close does not match its proved header name.'))
            if suffix=='select' and stack and stack[-1]['owner']=='select' and not stack[-1].get('has_accept'):
                diagnostics.append(_issue('foreign-ada-select-variant-review',token['start'],'Select owner has no proved accept alternative.'))
            end=tokens[cursor-1]['end'] if cursor>index+1 else token['end']
            if cursor<len(tokens) and tokens[cursor]['word']==';': end=tokens[cursor]['end']; cursor+=1
            _finish(frames,stack,token,end,events,diagnostics,wanted)
            if suffix=='select':proved_complex_keywords.add(tokens[index+1]['start'])
            index=cursor; continue
        declaration_start=_ada_declaration_start(tokens,index) if word in {'procedure','function','package'} else None
        if declaration_start is not None:
            start=tokens[declaration_start]['start']
            terminal=_word_terminal(tokens,index+1,{'is',';','renames'},False)
            if terminal is None:
                diagnostics.append(_issue('foreign-ada-header-review',token['start'],'Subprogram/package signature has no recognized terminal.')); index+=1; continue
            name_pos=index+1+int(word=='package' and index+1<len(tokens) and tokens[index+1]['word']=='body')
            name=tokens[name_pos]['word'] if name_pos<len(tokens) else ''
            name_cursor=name_pos+1
            while name_cursor+1<len(tokens) and tokens[name_cursor]['word']=='.' and re.fullmatch(IDENT,tokens[name_cursor+1]['word']):
                name+='.'+tokens[name_cursor+1]['word'];name_cursor+=2
            term=tokens[terminal]['word']
            following=tokens[terminal+1]['word'] if terminal+1<len(tokens) else ''
            if term=='renames' or (term=='is' and following in {'new','separate','abstract','null','('}):
                end_pos=_word_terminal(tokens,terminal+1,{';'},False)
                if end_pos is None:
                    diagnostics.append(_issue('foreign-ada-declaration-review',token['start'],'Renamed/instantiated declaration is incomplete.')); index+=1; continue
                events.append(_event('declaration',start,tokens[end_pos]['end']))
                if following=='(': diagnostics.append(_issue('foreign-ada-expression-function-review',token['start'],'Expression-function treatment requires dedicated review.'))
                index=end_pos+1; continue
            if term==';':
                events.append(_event('declaration',start,tokens[terminal]['end']))
            else:
                events.append(_event('header',start,tokens[terminal]['end']))
                stack.append(dict(owner=word,name=name,start=tokens[terminal]['end'],kind='type' if word=='package' else 'code',pending_begin=True))
            index=terminal+1; continue
        if word=='type' and _statement_start(tokens,index):
            terminal=_word_terminal(tokens,index+1,{'is',';'},False)
            record_start=_word_terminal(tokens,terminal+1,{'record',';'},False) if terminal is not None and tokens[terminal]['word']=='is' else None
            record_prefix=text[tokens[terminal]['end']:tokens[record_start]['start']].strip() if record_start is not None else ''
            modifiers=re.fullmatch(r'(?:(?:limited|tagged|abstract)\s*)*',record_prefix,re.I)
            derived=re.fullmatch(r'new\s+'+IDENT+r'(?:\.'+IDENT+r')*(?:\s*\([^{};]*\))?\s+with',record_prefix,re.I)
            if not modifiers and not derived:record_start=None
            if record_start is not None and tokens[record_start]['word']=='record':
                end=tokens[record_start]['end']
                events.append(_event('header',token['start'],end))
                stack.append(dict(owner='record',start=end,kind='type',pending_begin=False))
                index=record_start+1; continue
        if word=='begin':
            events.append(_event('open',token['start'],token['end'],units=0))
            if stack and stack[-1].get('pending_begin'):
                stack[-1]['pending_begin']=False
            else:
                stack.append(dict(owner='begin',start=token['end'],kind='bare-code',pending_begin=False))
            index+=1; continue
        if word=='declare' and _statement_start(tokens,index):
            events.append(_event('header',token['start'],token['end']))
            stack.append(dict(owner='declare',start=token['end'],kind='code',pending_begin=True)); index+=1; continue
        if word=='for' and _statement_start(tokens,index):
            use=_word_terminal(tokens,index+1,{'use','loop',';'},False)
            if use is not None and tokens[use]['word']=='use':
                prefix=text[token['end']:tokens[use]['start']].strip()
                attribute=re.fullmatch(IDENT+r'(?:\.'+IDENT+r')*\s*\'\s*('+IDENT+r')',prefix,re.I)
                terminal=_word_terminal(tokens,use+1,{';'},False)
                reviewed={'address','size','alignment','storage_size','external_tag','component_size','value_size','object_size'}
                if attribute and attribute[1].lower() in reviewed and terminal is not None:
                    value=text[tokens[use]['end']:tokens[terminal]['start']].strip()
                    if value and not re.search(r'[{};]|:=|\b(?:begin|end|declare|accept|task|procedure|function)\b',value,re.I):
                        events.append(_event('declaration',token['start'],tokens[terminal]['end'],scope='foreign-ada-written-attribute-clause'))
                        index=terminal+1;continue
                diagnostics.append(_issue('foreign-ada-representation-clause-review',token['start'],'Representation/attribute clause has no bounded complete declaration owner.'))
        if word in {'if','elsif','for','while','case'} and _statement_start(tokens,index):
            terminal=_word_terminal(tokens,index+1,{'then'} if word in {'if','elsif'} else {'is'} if word=='case' else {'loop'})
            if terminal is None:
                diagnostics.append(_issue('foreign-ada-control-review',token['start'],'Control header has no recognized terminal.')); index+=1; continue
            end=tokens[terminal]['end']
            header_start,construct_name=_ada_loop_header_start(tokens,index) if word in {'for','while'} else (index,None)
            events.append(_event('branch' if word=='elsif' else 'header',tokens[header_start]['start'],end))
            if word!='elsif': stack.append(dict(owner='if' if word=='if' else 'case' if word=='case' else 'loop',start=end,kind='code',pending_begin=False,construct_name=construct_name))
            index=terminal+1; continue
        if word=='loop' and _statement_start(tokens,index):
            header_start,construct_name=_ada_loop_header_start(tokens,index)
            events.append(_event('header',tokens[header_start]['start'],token['end']))
            stack.append(dict(owner='loop',start=token['end'],kind='code',pending_begin=False,construct_name=construct_name)); index+=1; continue
        if word in {'else','exception'}:
            events.append(_event('branch',token['start'],token['end'])); index+=1; continue
        if word=='exit' and _statement_start(tokens,index):
            terminal=_word_terminal(tokens,index+1,{';'},False)
            if terminal is None:
                diagnostics.append(_issue('foreign-ada-exit-review',token['start'],'Exit statement has no top-level terminating semicolon.'))
                index+=1; continue
            # EXIT [loop_name] [WHEN condition] is one action. Its WHEN is
            # neither a CASE/exception arm nor a separately owned branch.
            events.append(_event('statement',token['start'],tokens[terminal]['end']))
            index=terminal+1; continue
        if word=='when':
            terminal=_word_terminal(tokens,index+1,{'=>'})
            if terminal is None:
                diagnostics.append(_issue('foreign-ada-when-review',token['start'],'Case/exception arm has no recognized association arrow.')); index+=1; continue
            events.append(_event('branch',token['start'],tokens[terminal]['end'])); index=terminal+1; continue
        index+=1
    for match in re.finditer(r'\b(?:task|protected|select|accept|entry|generic)\b|\baccess\s+(?:procedure|function)\b',text,re.I):
        if match.start() in proved_complex_keywords:continue
        diagnostics.append(_issue('foreign-ada-complex-owner-review',match.start(),'Task/protected/select/generic/access-subprogram syntax needs a dedicated grammar rule.'))
    for frame in frames:
        if frame['owner']=='select' and re.search(r'\b(?:delay|abort)\b',text[frame['start']:frame['body_end']]):
            diagnostics.append(_issue('foreign-ada-select-variant-review',frame['start'],'Timed/conditional/asynchronous select needs a dedicated owner proof.'))
    _unclosed(frames,stack,len(text),diagnostics)
    return events,frames,diagnostics


def _pascal(text):
    tokens,events,frames,stack,diagnostics = _tokens(text),[],[],[],[]
    index,pending_control=0,False
    directives={'cdecl','stdcall','safecall','register','pascal','external','forward','overload','inline','virtual','override','abstract','reintroduce','static'}
    proven_class_modifiers=set()
    while index<len(tokens):
        token,word=tokens[index],tokens[index]['word']
        routine_start=index if word in {'function','procedure'} and _statement_start(tokens,index) else None
        if word in {'function','procedure'} and index and tokens[index-1]['word']=='class' and _statement_start(tokens,index-1):
            routine_start=index-1;proven_class_modifiers.add(tokens[index-1]['start'])
        if routine_start is not None:
            terminal=_word_terminal(tokens,index+1,{';'},False)
            if terminal is None:
                diagnostics.append(_issue('foreign-pascal-header-review',token['start'],'Routine signature has no top-level terminating semicolon.')); index+=1; continue
            end_pos=terminal
            cursor=terminal+1
            prototype=bool(stack and stack[-1]['kind']=='type')
            while cursor<len(tokens) and tokens[cursor]['word'] in directives:
                prototype=prototype or tokens[cursor]['word'] in {'external','forward','abstract'}
                part=_word_terminal(tokens,cursor+1,{';'},False)
                if part is None:
                    diagnostics.append(_issue('foreign-pascal-declaration-directive-review',tokens[cursor]['start'],'Declaration directive is incomplete.')); break
                end_pos=part; cursor=part+1
            events.append(_event('declaration' if prototype else 'header',tokens[routine_start]['start'],tokens[end_pos]['end']))
            if not prototype: stack.append(dict(owner='routine',start=tokens[end_pos]['end'],kind='code',pending_begin=True))
            index=cursor; continue
        if word=='program' and _statement_start(tokens,index):
            terminal=_word_terminal(tokens,index+1,{';'},False)
            if terminal is not None:
                events.append(_event('header',token['start'],tokens[terminal]['end']))
                stack.append(dict(owner='program',start=tokens[terminal]['end'],kind='code',pending_begin=True)); index=terminal+1; continue
        if word in {'var','const','type'} and _statement_start(tokens,index):
            # Root-approved structural introducer: no new lexical scope/close.
            # Count actual bindings/type headers, not an extra var/const/type.
            events.append(_event('section',token['start'],token['end'],units=0,scope='foreign-declaration-section')); index+=1; continue
        if word=='record':
            start=token['start']
            cursor=index-1
            while cursor>=0 and tokens[cursor]['word'] not in {';','type','begin','end'}: start=tokens[cursor]['start']; cursor-=1
            events.append(_event('header',start,token['end'],scope='foreign-record'))
            stack.append(dict(owner='record',start=token['end'],kind='type',pending_begin=False)); index+=1; continue
        if word=='begin':
            events.append(_event('open',token['start'],token['end'],units=0))
            if not pending_control and stack and stack[-1].get('pending_begin'):
                stack[-1]['pending_begin']=False
            else:
                stack.append(dict(owner='begin',start=token['end'],kind='code' if pending_control else 'bare-code',pending_begin=False))
            pending_control=False; index+=1; continue
        if word=='end':
            cursor=index+1
            end=token['end']
            if cursor<len(tokens) and tokens[cursor]['word'] in {';','.'}: end=tokens[cursor]['end']; cursor+=1
            _finish(frames,stack,token,end,events,diagnostics)
            pending_control=False; index=cursor; continue
        if word in {'if','for','while','case','with'} and _statement_start(tokens,index):
            terminal=_word_terminal(tokens,index+1,{'then'} if word=='if' else {'of'} if word=='case' else {'do'})
            if terminal is None:
                diagnostics.append(_issue('foreign-pascal-control-review',token['start'],'Control header has no recognized terminal.')); index+=1; continue
            end=tokens[terminal]['end']
            events.append(_event('header',token['start'],end))
            if word=='case': stack.append(dict(owner='case',start=end,kind='code',pending_begin=False)); pending_control=False
            else: pending_control=terminal+1<len(tokens) and tokens[terminal+1]['word']=='begin'
            index=terminal+1; continue
        if word=='else':
            events.append(_event('branch',token['start'],token['end']))
            pending_control=index+1<len(tokens) and tokens[index+1]['word']=='begin'; index+=1; continue
        if word=='repeat':
            events.append(_event('header',token['start'],token['end']))
            stack.append(dict(owner='repeat',start=token['end'],kind='code',pending_begin=False)); index+=1; continue
        if word=='until':
            terminal=_word_terminal(tokens,index+1,{';'},False)
            if terminal is None:
                diagnostics.append(_issue('foreign-pascal-until-review',token['start'],'Until condition must have a proven statement boundary.')); index+=1; continue
            _finish(frames,stack,token,tokens[terminal]['end'],events,diagnostics,{'repeat'}); index=terminal+1; continue
        if stack and stack[-1]['owner']=='case' and _statement_start(tokens,index):
            terminal=_word_terminal(tokens,index,{':'})
            if terminal is not None:
                label=text[token['start']:tokens[terminal]['start']]
                if re.fullmatch(r'[\w\s,\.\-+]+',label):
                    events.append(_event('branch',token['start'],tokens[terminal]['end'])); index=terminal+1; continue
                diagnostics.append(_issue('foreign-pascal-case-label-review',token['start'],'Case choice syntax requires dedicated review.'))
        index+=1
    for match in re.finditer(r'\b(?:unit|interface|implementation|initialization|finalization|class|object|try|except|finally|asm|property)\b|\breference\s+to\b|\b(?:function|procedure)\s*\(',text,re.I):
        if match[0].lower()=='class' and match.start() in proven_class_modifiers:continue
        diagnostics.append(_issue('foreign-pascal-complex-owner-review',match.start(),'Class/unit/exception/assembler/property/anonymous-routine grammar is not yet admitted.'))
    _unclosed(frames,stack,len(text),diagnostics)
    return events,frames,diagnostics


def _fortran_statements(text):
    """Logical header spans preserve source offsets; data rows stay written."""
    result=[]
    start=0; depth=0
    for index,char in enumerate(text):
        if char in '([': depth+=1
        elif char in ')]': depth-=1
        boundary=char==';' and not depth
        if char=='\n':
            before=text[start:index].rstrip()
            boundary=not before.endswith('&')
        if boundary:
            if text[start:index].strip(): result.append((start,index))
            start=index+1
    if text[start:].strip(): result.append((start,len(text)))
    return result


def _fortran(text):
    events,frames,stack,diagnostics=[],[],[],[]
    pairs,_=brace_pairs(text)
    for raw_start,end in _fortran_statements(text):
        start=raw_start
        while start<end and text[start].isspace(): start+=1
        value=text[start:end]
        code=value.replace('&',' ').strip().lower()
        construct=re.match(IDENT+r'\s*:\s*(?=do(?:\s|$)|if\s*\(|select\s+case\b|block\s*$)',code)
        if construct:
            # Retain the label in the one owned header span. END DO/IF/... may
            # carry the same name without creating another statement or owner.
            code=code[construct.end():]
        closing=re.fullmatch(r'end\s*(?:(program|module|subroutine|function|interface|type|do|if|select|block)\b\s*('+IDENT+r')?)?',code)
        if closing:
            word=closing[1]
            wanted={word} if word else None
            if stack and stack[-1].get('construct_name') and closing[2] and closing[2]!=stack[-1]['construct_name']:
                diagnostics.append(_issue('foreign-fortran-construct-name-review',start,'Written construct-close name does not match its proved header name.'))
            _finish(frames,stack,dict(start=start),end,events,diagnostics,wanted); continue
        branch=re.match(r'(?:else\s*if|elseif)\s*\(',code)
        if branch:
            if not re.search(r'\bthen\s*$',code): diagnostics.append(_issue('foreign-fortran-elseif-review',start,'Else-if needs its block-header terminal.'))
            events.append(_event('branch',start,end)); continue
        if code=='else': events.append(_event('branch',start,end)); continue
        if re.match(r'case\s*(?:\(|default\b)',code):
            opening=text.find('(',start,end); close=pairs.get(opening)
            if close is not None and text[close+1:end].lstrip().startswith('='):
                continue  # An array named CASE is not a reserved keyword.
            if not stack or stack[-1]['owner']!='select':
                diagnostics.append(_issue('foreign-fortran-orphan-case-review',start,'Case branch has no proven SELECT owner.'))
            events.append(_event('branch',start,end)); continue
        if code=='contains':
            events.append(_event('section',start,end,units=0,scope='foreign-declaration-section'))
            continue
        if code=='private' and stack and stack[-1]['owner'] in {'type','module'}:
            events.append(_event('declaration',start,end,scope='foreign-fortran-written-access-control'))
            continue
        if code=='public' and stack and stack[-1]['owner'] in {'type','module'}:
            diagnostics.append(_issue('foreign-fortran-access-section-owner-review',start,'Accessibility statement/section needs an explicit header/section-unit decision; no inferred removal.'))
        routine=re.match(r'(?:(?:recursive|pure|elemental|impure|module)\s+)*(?:(?:integer|real|logical|complex|character|double\s+precision|type)\s*(?:\*\s*\d+|\([^)]*\))?\s+)?(subroutine|function)\s+('+IDENT+r')\s*\(',code)
        owner=None
        if routine: owner=routine[1]
        elif re.fullmatch(r'(?:(?:recursive|pure|elemental|impure|module)\s+)*subroutine\s+'+IDENT,code): owner='subroutine'
        elif re.fullmatch(r'(program|module)\s+'+IDENT,code): owner=code.split()[0]
        elif re.fullmatch(r'interface(?:\s+'+IDENT+r')?',code): owner='interface'
        elif re.fullmatch(r'type\s*(?:::)?\s+'+IDENT,code): owner='type'
        elif re.fullmatch(r'type\s*,\s*bind\s*\(\s*c\s*\)\s*::\s*'+IDENT,code):
            # A written BIND(C) clause is structural type-header metadata,
            # not a variable declaration or a generated interoperability unit.
            owner='type'
        elif re.match(r'type\s*,[^:]*::\s*'+IDENT+r'\s*$',code):
            owner='type'
            diagnostics.append(_issue('foreign-fortran-derived-type-attributes-review',start,'Derived type attributes require exact type-scope review.'))
        elif re.match(r'if\s*\(',code):
            opening=text.find('(',start,end)
            close=pairs.get(opening)
            if close is None or close>=end:
                diagnostics.append(_issue('foreign-fortran-if-review',start,'If condition has no matching parenthesis.')); continue
            tail=text[close+1:end].replace('&',' ').strip().lower()
            if tail.startswith('='):
                continue  # IF may also be an ordinary array identifier.
            if tail=='then': owner='if'
            else:
                events.append(_event('header',start,close+1))
                if re.match(r'\d',tail): diagnostics.append(_issue('foreign-fortran-arithmetic-if-review',start,'Labelled arithmetic-if requires a separate rule.'))
                continue
        elif re.match(r'do(?:\s|$)',code):
            if re.match(r'do\s+\d',code): diagnostics.append(_issue('foreign-fortran-labelled-do-review',start,'Label-terminated loops require exact label ownership.')); continue
            owner='do'
            if re.match(r'do\s+concurrent\b',code): diagnostics.append(_issue('foreign-fortran-concurrent-do-review',start,'Concurrent-loop clauses require dedicated review.'))
        elif re.match(r'select\s+case\s*\(',code): owner='select'
        elif code=='block': owner='block'
        if owner:
            events.append(_event('header',start,end))
            stack.append(dict(owner=owner,start=end,kind='type' if owner in {'module','interface','type'} else 'code',pending_begin=False,
                construct_name=re.match(IDENT,value.strip())[0].lower() if construct else None)); continue
        if re.match(r'(?:associate|where|forall|select\s+type|select\s+rank|procedure|module\s+procedure|abstract\s+interface|entry|enum)\b',code):
            diagnostics.append(_issue('foreign-fortran-complex-owner-review',start,'Associate/where/forall/polymorphic/procedure/legacy owner needs a dedicated grammar rule.'))
        if re.match(r'\d+\s',code): diagnostics.append(_issue('foreign-fortran-statement-label-review',start,'Labelled statement ownership has not been normalized.'))
    _unclosed(frames,stack,len(text),diagnostics)
    return events,frames,diagnostics


def _continued_declarations(text, language, contexts, events):
    """Prove continued independent name groups; never absorb data wrapping.

    Existing header/formal and call/data owners are protected. Only a complete
    typed declaration with a reviewed scalar type/initializer is covered by a
    canonical declaration event. Literal/array row layout stays in the common
    accounting path, and unknown type/entity syntax retains a review gate.
    """
    proposals=[]
    def protected(start,end):
        return structural(contexts[start]) or any(e['start']<end and start<e['end']
            for e in events)
    def type_and_initializer(tail,initializer):
        prefix=re.match(r'\s*(?:(?:aliased|constant|access|not\s+null|packed)\s+)*'+IDENT+r'(?:\.'+IDENT+r')*',tail,re.I)
        if not prefix:return False
        rest=tail[prefix.end():].lstrip()
        if rest.startswith(('(','[')):
            pairs,issues=brace_pairs(rest)
            if issues or 0 not in pairs:return False
            rest=rest[pairs[0]+1:].lstrip()
        if not rest:return True
        if not rest.startswith(initializer):return False
        value=rest[len(initializer):].strip()
        return bool(value) and '\n' not in value and not re.search(r'[{};]|:=|(?<![=!<>])=(?!=)',value)
    def scalar_entity(entity):
        name=re.match(IDENT,entity)
        if not name:return False
        rest=entity[name.end():].lstrip()
        if rest.startswith('('):
            pairs,issues=brace_pairs(rest)
            if issues or 0 not in pairs:return False
            rest=rest[pairs[0]+1:].lstrip()
        if not rest:return True
        initializer=re.match(r'=>|=(?![=>])',rest)
        if not initializer:return False
        value=rest[initializer.end():].strip()
        return bool(value) and '\n' not in value and not re.search(r'[{};]',value)
    if language in {'ada','pascal'}:
        pattern=re.compile(r'('+IDENT+r'(?:\s*,\s*'+IDENT+r')+)\s*:(?![=:])')
        for match in pattern.finditer(text):
            if '\n' not in match[1] or protected(match.start(),match.end()):continue
            cursor=match.end();depth=0;end=None
            while cursor<len(text):
                char=text[cursor]
                if char in '([':depth+=1
                elif char in ')]':depth-=1
                elif char==';' and not depth:end=cursor+1;break
                cursor+=1
            if end is None or protected(match.start(),end):continue
            tail=text[match.end():end-1]
            # Scalar types, type members, bounds and a scalar initializer may
            # be wrapped as whitespace. An actual wrapped data literal cannot
            # be collapsed by proving only its preceding declared names.
            initializer=':=' if language=='ada' else '='
            if not type_and_initializer(tail,initializer):continue
            proposals.append(_event('declaration',match.start(),end,
                units=len(top_parts(match[1])),scope='foreign-continued-independent-declaration'))
    else:
        allowed_attributes={'pointer','target','allocatable','parameter','value','optional',
            'save','public','private','contiguous','volatile','asynchronous'}
        for raw_start,end in _fortran_statements(text):
            start=raw_start
            while start<end and text[start].isspace():start+=1
            value=text[start:end]
            if '\n' not in value or '::' not in value or protected(start,end):continue
            prefix,entities=value.split('::',1)
            prefix=prefix.replace('&',' ').strip()
            pieces=top_parts(prefix)
            scalar_type=re.fullmatch(r'(?:integer|real|logical|complex|character|double\s+precision|type|class)(?:\s*\*\s*\d+|\s*\([^{};]*\))?',pieces[0],re.I) if pieces else None
            if not scalar_type:continue
            if any(not (part.lower() in allowed_attributes or re.fullmatch(r'(?:intent|dimension)\s*\([^{};]*\)',part,re.I)) for part in pieces[1:]):continue
            names=top_parts(entities.replace('&',' '))
            # A continuation immediately following a comma is whitespace in a
            # declaration list. A newline inside an initializer remains data
            # layout; it does not qualify as a simple scalar entity here.
            if not names or not all(scalar_entity(name) for name in names):continue
            proposals.append(_event('declaration',start,end,units=len(names),
                scope='foreign-continued-independent-declaration'))
    return proposals


def classify_scopes(cleaned,language,directives=()):
    if language not in LANGUAGES:
        raise ValueError('No keyword grammar alias is permitted.')
    parsed=_parse_text(cleaned,directives)
    events,frames,diagnostics=({'ada':_ada,'pascal':_pascal,'fortran':_fortran}[language])(parsed)
    _,pair_issues=brace_pairs(parsed)
    diagnostics.extend(pair_issues)
    contexts=_contexts(parsed,frames,events,language)
    continued=_continued_declarations(parsed,language,contexts,events)
    events.extend(continued)
    if language in {'ada','pascal'}:
        for match in re.finditer(IDENT+r'(?:\s*,\s*'+IDENT+r')*\s*,\s*\n',parsed):
            if not structural(contexts[match.start()]) and not any(e['start']<=match.start()<e['end'] for e in continued):
                diagnostics.append(_issue('foreign-multiline-declarator-review',match.start(),'A continued name group requires a full declaration span before counting individual bindings.'))
    return dict(contexts=contexts,events=list(directives)+events,
        diagnostics=diagnostics,owners=frames,executable_regions=[dict(start=f['start'],end=f.get('body_end',f['end']),scope=f['owner']) for f in frames if f['kind'] in {'code','bare-code'}])


def binding_counter(language):
    def counter(*,text,original,context,assignments):
        if structural(context,assignment=True,named_equals=language=='fortran'):
            return dict(reason='structural-formal-bound-attribute-argument-or-data-row')
        code=text.strip()
        names=None
        if language in {'ada','pascal'}:
            found=re.match(r'^('+IDENT+r'(?:\s*,\s*'+IDENT+r')*)\s*:(?![=:])\s*\S',code,re.S)
            if found: names=top_parts(found[1])
        else:
            value=code.replace('&',' ').strip()
            type_match=re.match(r'^(?:integer|real|logical|complex|character|double\s+precision|type|class)\b',value,re.I)
            if type_match:
                if '::' in value: rest=value.split('::',1)[1].strip()
                else:
                    rest=value[type_match.end():].lstrip()
                    if rest.startswith('*'): rest=re.sub(r'^\*\s*\d+','',rest).lstrip()
                    elif rest.startswith('('):
                        pair,_=brace_pairs(rest); end=pair.get(0)
                        rest=rest[end+1:].lstrip() if end is not None else ''
                parts=top_parts(rest)
                if parts and all(re.fullmatch(IDENT+r'(?:\s*\([^)]*\))?\s*(?:(?:=>|=)(?![=>]).*)?',part,re.S) for part in parts): names=parts
                elif ',' in rest: return dict(diagnostics=[dict(kind='foreign-fortran-declarator-review',message='Packed/multiline entity declarators require exact type/continuation ownership.')])
        if names is not None:
            return dict(units=max(len(names),len(assignments)),declaration_bindings=len(names),reason='independent-keyword-language-declaration-bindings')
        return {}
    return counter


def normalize(source,language):
    if language not in LANGUAGES:
        raise ValueError('Only Ada/Pascal/Fortran keyword grammar is proposed.')
    masked=mask_keyword(source,language)
    classified=classify_scopes(masked['cleaned'],language,masked['directives'])
    result=count_owned(source,masked['cleaned'],classified['contexts'],classified['events'],
        masked['diagnostics']+classified['diagnostics'],language,binding_counter=binding_counter(language),
        named_equals=language=='fortran',assignment_pattern=FORTRAN_ASSIGN if language=='fortran' else COLON_ASSIGN)
    result.update(proposal_version=PROPOSAL_VERSION,proposal_only=True,proposal_status='unhooked-no-admission',
        based_literal_evidence=masked.get('based_literal_evidence',[]),
        owners=classified['owners'],executable_regions=classified['executable_regions'],
        admission='Not admitted: verify source/producer/tool pins and review locked fixtures/diagnostics before any display overlay.')
    return result
