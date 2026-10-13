"""Unhooked Smalltalk source-unit grammar proposal; no chart admission.

The programming masker and file-in/block classifier are explicit Smalltalk
grammar. Shared foreign_core only accounts for proved owners and bindings.
Only proven category/chunk metadata, reflective fields, and independent cascade
sends use the reviewed rules; unknown roles remain gated. No source/report mutation, collector,
benchmark recognition, physical-count fallback certificate, or hook lives here.
Execute only in the common-mutex fixture/source audit.
"""
import re

from canonical import _structural_callback_suffix
from foreign_core import brace_pairs, count_owned, replacement, structural


PROPOSAL_VERSION = 'foreign-smalltalk-unhooked-20261012-v1'
IDENT = r'[A-Za-z_]\w*'
ASSIGN = re.compile(r':=')
NUMBER = re.compile(r'(?<![\w])\d+(?:r[0-9A-Za-z]+|(?:\.\d+)?(?:[eEdDqQ][+-]?\d+)?)')
BINARY = r'[+*/\\~<>=@%&?,|-]+'
CLASS_HEAD = re.compile(r'('+IDENT+r')\s+subclass:\s*#('+IDENT+r')\b')


def _issue(kind,offset,message):
    return dict(kind=kind,offset=offset,message=message)


def _event(kind,start,end,units=1,scope='foreign-smalltalk'):
    return dict(kind=kind,start=start,end=end,units=units,scope=scope)


def _trim(text,start,end):
    while start<end and text[start].isspace(): start+=1
    while end>start and text[end-1].isspace(): end-=1
    return start,end


def attached_unary_suffix(cleaned,closing):
    """Own only the three reviewed terminal unary block invocations.

    A keyword selector is not a unary selector prefix: ``value:`` and
    ``whileTrue:`` retain their surrounding-call review. This proves syntax
    ownership, never callee purity or a waiver for arbitrary unary sends.
    """
    start=closing+1
    while start<len(cleaned) and cleaned[start].isspace(): start+=1
    selector=re.match(IDENT,cleaned[start:])
    if not selector or selector[0] not in {'whileTrue','whileFalse','value'}: return None
    end=start+selector.end()
    if end<len(cleaned) and cleaned[end]==':': return None
    terminal=end
    while terminal<len(cleaned) and cleaned[terminal].isspace(): terminal+=1
    if terminal<len(cleaned) and cleaned[terminal] not in '.!)]};': return None
    return dict(start=start,end=end,selector=selector[0],units=0)


def mask_smalltalk(source):
    """Mask programming comments, strings, characters and numeric periods.

    Every source offset/newline is retained. Quoted strings use doubled single
    quotes; double quotes introduce comments. Only the explicit standard class
    field role may later inspect a quoted string's decoded contents.
    """
    chars=list(source); literals=[]; comments=[]; numbers=[]; diagnostics=[]; cursor=0
    if source.startswith('\ufeff'): chars[0]=' '
    while cursor<len(source):
        start=cursor; char=source[cursor]
        if char in "\"'":
            delimiter=char; cursor+=1; complete=False
            while cursor<len(source):
                if source[cursor]==delimiter:
                    if delimiter=="'" and source.startswith("''",cursor): cursor+=2; continue
                    cursor+=1; complete=True; break
                cursor+=1
            chars[start:cursor]=replacement(source[start:cursor],literal=delimiter=="'")
            span=dict(start=start,end=cursor,delimiter=delimiter)
            (literals if delimiter=="'" else comments).append(span)
            if not complete:
                diagnostics.append(_issue('foreign-unclosed-literal' if delimiter=="'" else 'foreign-unclosed-comment',start,'Smalltalk quote has no matching written close.'))
        elif char=='$':
            cursor+=1
            if cursor>=len(source) or source[cursor] in '\r\n':
                diagnostics.append(_issue('foreign-smalltalk-character-review',start,'Character literal needs one written character.'))
            else: cursor+=1
            chars[start:cursor]=replacement(source[start:cursor],literal=True)
            literals.append(dict(start=start,end=cursor,delimiter='$'))
        else:
            number=NUMBER.match(source,cursor)
            if number:
                cursor=number.end(); chars[start:cursor]=replacement(source[start:cursor],literal=True)
                numbers.append(dict(start=start,end=cursor))
            else: cursor+=1
    cleaned=''.join(chars)
    for match in re.finditer(r'[^\x00-\x7f]',cleaned):
        if match[0].isalpha():
            diagnostics.append(_issue('foreign-smalltalk-unicode-identifier-review',match.start(),'Initial named-selector grammar requires explicit Unicode review.'))
    return dict(cleaned=cleaned,literals=literals,comments=comments,numbers=numbers,diagnostics=diagnostics)


def _method_header(text,start,end):
    """Recognize a file-in selector/formal header, not an arbitrary send row."""
    start,_=_trim(text,start,end)
    newline=text.find('\n',start,end); line_end=end if newline<0 else newline
    row=text[start:line_end].rstrip()
    keyword=re.match(r'(?:'+IDENT+r':\s*'+IDENT+r'\s*)+',row)
    if keyword:
        finish=start+keyword.end()
        # Wrapped multipart selectors are allowed only while each entire
        # continuation row is another selector/formal pair.
        while not row[keyword.end():].strip() and newline>=0:
            next_start=newline+1
            while next_start<end and text[next_start] in ' \t\r': next_start+=1
            next_newline=text.find('\n',next_start,end)
            next_end=end if next_newline<0 else next_newline
            continued=text[next_start:next_end].rstrip()
            if not re.fullmatch(r'(?:'+IDENT+r':\s*'+IDENT+r'\s*)+',continued): break
            finish=next_end; newline=next_newline
        return start,finish
    binary=re.match(BINARY+r'\s+'+IDENT+r'(?=\s|$)',row)
    if binary:
        rest=row[binary.end():].strip()
        if not rest or rest.startswith(('^','|')): return start,start+binary.end()
        return None
    unary=re.match(IDENT+r'(?=\s|$)',row)
    if unary:
        rest=row[unary.end():].strip()
        if not rest or rest.startswith(('^','|')): return start,start+unary.end()
    return None


def _category(text,start,end,literals):
    """Prove quoted category/stamp roles, including whitespace in strings."""
    header=re.match(IDENT+r'(?:\s+class)?\s+methodsFor:\s*',text[start:end])
    if header is None: return False
    cursor=start+header.end()
    literal=next((span for span in literals if span['start']==cursor and span['delimiter']=="'" and span['end']<=end),None)
    if literal is None: return False
    cursor,_=_trim(text,literal['end'],end)
    if cursor==end: return True
    stamp=re.match(r'stamp:\s*',text[cursor:end])
    if stamp is None: return False
    cursor+=stamp.end()
    literal=next((span for span in literals if span['start']==cursor and span['delimiter']=="'" and span['end']<=end),None)
    return literal is not None and not text[literal['end']:end].strip()


def _class_events(text,start,end,source,literals):
    """Give only proven standard reflective declaration strings field roles."""
    head=CLASS_HEAD.match(text,start,end)
    if not head: return None
    events=[_event('header',start,head.end(),scope='foreign-smalltalk-type')]
    diagnostics=[]; cursor=head.end()
    while cursor<end:
        cursor,_=_trim(text,cursor,end)
        if cursor==end: break
        field=re.match(r'(instanceVariableNames|classVariableNames|poolDictionaries|category):\s*',text[cursor:end])
        if not field:
            diagnostics.append(_issue('foreign-smalltalk-class-definition-review',cursor,'Class definition contains an unproved metadata/field form.'))
            break
        value_start=cursor+field.end()
        literal=next((span for span in literals if span['start']==value_start and span['delimiter']=="'"),None)
        if literal is None or literal['end']>end:
            diagnostics.append(_issue('foreign-smalltalk-reflective-field-review',value_start,'Standard class property requires an exact quoted string role.'))
            break
        units=0; role='foreign-smalltalk-class-metadata'
        if field[1] in {'instanceVariableNames','classVariableNames'}:
            if source is None:
                diagnostics.append(_issue('foreign-smalltalk-reflective-field-review',value_start,'Original quoted field string is required to prove declared names.'))
            else:
                names=source[value_start+1:literal['end']-1].replace("''","'").split()
                if not all(re.fullmatch(IDENT,name) for name in names):
                    diagnostics.append(_issue('foreign-smalltalk-reflective-field-review',value_start,'Field string contains an unreviewed declarator name.'))
                else: units=len(names)
            role='foreign-smalltalk-reflective-fields'
        events.append(_event('declaration' if units else 'metadata',cursor,literal['end'],units=units,scope=role))
        cursor=literal['end']
    return events,diagnostics


def _file_in(text,source,literals,comments):
    events=[]; diagnostics=[]; methods=[]; cursor=0; section=False
    bangs=[match.start() for match in re.finditer('!',text)]
    for number,bang in enumerate(bangs):
        start,end=_trim(text,cursor,bang); code=text[start:end]
        next_bang=bangs[number+1] if number+1<len(bangs) else len(text)
        next_start,next_end=_trim(text,bang+1,next_bang)
        if _category(text,start,end,literals):
            events.append(_event('metadata',start,bang+1,units=0,scope='foreign-smalltalk-category'))
            section=True
        elif code:
            class_roles=_class_events(text,start,end,source,literals)
            if class_roles is not None:
                class_events,class_diagnostics=class_roles; events.extend(class_events); diagnostics.extend(class_diagnostics)
                events.append(_event('close',bang,bang+1,scope='foreign-smalltalk-type'))
                section=False
            elif section:
                header=_method_header(text,start,end)
                if header is None:
                    diagnostics.append(_issue('foreign-smalltalk-method-header-review',start,'File-in method selector/formals were not proved.'))
                else:
                    events.append(_event('header',header[0],header[1],scope='foreign-smalltalk-method'))
                    events.append(_event('close',bang,bang+1,scope='foreign-smalltalk-method'))
                    methods.append(dict(start=header[1],end=bang,scope='foreign-smalltalk-method'))
            else:
                events.append(_event('metadata',bang,bang+1,units=0,scope='foreign-smalltalk-unproved-chunk'))
                diagnostics.append(_issue('foreign-smalltalk-chunk-role-review',bang,'Chunk has no proved category/method/type owner.'))
        elif section:
            events.append(_event('metadata',bang,bang+1,units=0,scope='foreign-smalltalk-category-close'))
            section=False
        elif _category(text,next_start,next_end,literals):
            events.append(_event('metadata',bang,bang+1,units=0,scope='foreign-smalltalk-category-open'))
        else:
            is_documentation=any(span['start']>=cursor and span['end']<=bang for span in comments)
            events.append(_event('metadata',bang,bang+1,units=0,scope='foreign-smalltalk-documentation' if is_documentation else 'foreign-smalltalk-unproved-chunk'))
            if not is_documentation:
                diagnostics.append(_issue('foreign-smalltalk-chunk-role-review',bang,'File-in delimiter has no proved documentation/category/method/type role.'))
        cursor=bang+1
    tail_start,tail_end=_trim(text,cursor,len(text))
    if section and tail_start<tail_end:
        diagnostics.append(_issue('foreign-smalltalk-unclosed-method-review',tail_start,'File-in method body has no written terminating chunk delimiter.'))
    elif tail_start<tail_end and (CLASS_HEAD.match(text,tail_start,tail_end) or 'methodsFor:' in text[tail_start:tail_end]):
        diagnostics.append(_issue('foreign-smalltalk-unclosed-file-in-owner-review',tail_start,'Type/category declaration has no proved terminal file-in delimiter.'))
    return dict(events=events,diagnostics=diagnostics,methods=methods)


def _block_formals(text,opening,closing):
    start,_=_trim(text,opening+1,closing)
    if start>=closing or text[start]!=':': return start,None
    matched=re.match(r'(?:\s*:'+IDENT+r')+\s*\|',text[start:closing])
    if matched is None: return start,False
    return start+matched.end(),(opening+1,start+matched.end())


def _cascade_message(text,initial=False):
    """Prove a send role without interpreting every semicolon as C syntax."""
    code=text.strip()
    if initial:
        code=re.sub(r'^\^\s*','',code)
        code=re.sub(r'^(?:'+IDENT+r'\s*:=\s*)+','',code)
        receiver=re.match(r'(?:'+IDENT+r'|S+)\s+',code)
        if receiver is None: return False
        code=code[receiver.end():]
    if re.fullmatch(r'(?:'+IDENT+r'\s+)*'+IDENT,code): return True
    return bool(re.match(r'(?:'+IDENT+r'\s+)*(?:'+IDENT+r':\s*\S|'+BINARY+r'\s*\S)',code))


def classify_scopes(cleaned,language='smalltalk',*,source=None,literals=(),comments=()):
    """Return inspectable method/block/data owners with conservative gates."""
    if language!='smalltalk': raise ValueError('Only explicit Smalltalk grammar is proposed.')
    file_in=_file_in(cleaned,source,literals,comments)
    events=list(file_in['events']); diagnostics=list(file_in['diagnostics'])
    methods=file_in['methods']; pairs,pair_diagnostics=brace_pairs(cleaned)
    diagnostics.extend(pair_diagnostics)
    contexts=[]; stack=[]; blocks=[]; literal_data=set()
    owned=lambda offset:any(event['start']<=offset<event['end'] for event in events)
    for index,char in enumerate(cleaned):
        frames=[('{','code',method['start']) for method in methods if method['start']<=index<=method['end']]
        contexts.append(tuple(sorted(frames+stack,key=lambda entry:entry[2])))
        if owned(index): continue
        if char in '([{':
            before=cleaned[:index].rstrip(); previous=before[-1:] if before else ''
            literal=previous=='#' or any(entry[2] in literal_data for entry in stack)
            kind='data' if literal or char=='{' else 'args' if char=='(' else 'code'
            if literal: literal_data.add(index)
            if char=='{':
                diagnostics.append(_issue('foreign-smalltalk-dynamic-array-review',index,'Dynamic array element sequencing requires separate reviewed ownership.'))
            stack.append((char,kind,index))
            if char=='[' and kind=='code':
                closing=pairs.get(index)
                if closing is not None:
                    body,formals=_block_formals(cleaned,index,closing)
                    if formals is False:
                        diagnostics.append(_issue('foreign-smalltalk-block-formal-review',index,'Block argument header has no proved closing formal bar.'))
                    blocks.append(dict(open=index,close=closing,body=body,formals=formals))
        elif char in ')]}':
            wanted={')':'(',']':'[','}':'{'}[char]
            if stack and stack[-1][0]==wanted: stack.pop()
    # Bars at the initial method/block body position are independent temporary
    # declarations. Later bars are binary selectors/composition, never guessed.
    for region in methods+blocks:
        start=region.get('body',region.get('start'))
        end=region.get('close',region.get('end'))
        start,_=_trim(cleaned,start,end)
        region['body_start']=start
        if start<end and cleaned[start]=='|':
            closing=cleaned.find('|',start+1,end)
            if closing<0:
                diagnostics.append(_issue('foreign-smalltalk-temporary-review',start,'Temporary declaration bar has no written closing bar.'))
                continue
            names=cleaned[start+1:closing].split()
            if not all(re.fullmatch(IDENT,name) for name in names):
                diagnostics.append(_issue('foreign-smalltalk-temporary-review',start,'Temporary group is not a proved independent identifier list.'))
                continue
            events.append(_event('declaration',start,closing+1,units=len(names),scope='foreign-smalltalk-temporaries'))
            region['body_start']=closing+1
    for block in blocks:
        opening,closing=block['open'],block['close']
        lower=max([0]+[method['body_start'] for method in methods if method['body_start']<=opening<method['end']]+[
            other['body_start'] for other in blocks if other['open']<opening<other['close']])
        # A previous written statement or already owned header/close is a real
        # boundary; line wrapping alone is not. Prefix ownership also includes
        # enclosing argument parentheses, preventing phantom callback tails.
        for index in range(lower,opening):
            if cleaned[index] in '.;' and not structural(contexts[index]): lower=index+1
        for event in events:
            if lower<=event['end']<=opening: lower=max(lower,event['end'])
        header_start,_=_trim(cleaned,lower,opening)
        if header_start<opening:
            events.append(_event('header',header_start,opening,scope='foreign-smalltalk-block'))
            events.append(_event('open',opening,opening+1,units=0,scope='foreign-smalltalk-block'))
        else:
            events.append(_event('header',opening,opening+1,scope='foreign-smalltalk-block'))
        if block['formals']:
            events.append(_event('parameters',block['formals'][0],block['formals'][1],units=0,scope='foreign-smalltalk-block-formals'))
        events.append(_event('close',closing,closing+1,scope='foreign-smalltalk-block'))
        following=closing+1
        while following<len(cleaned) and cleaned[following].isspace(): following+=1
        suffix=attached_unary_suffix(cleaned,closing)
        if suffix:
            events.append(_event('continuation',suffix['start'],suffix['end'],units=0,scope='foreign-smalltalk-unary-block-invocation'))
        else:
            selector=re.match(IDENT,cleaned[following:])
            known_keyword=bool(selector and selector[0] in {'whileTrue','whileFalse','value'}
                and cleaned[following+selector.end():].startswith(':'))
            known_unary=bool(selector and selector[0] in {'whileTrue','whileFalse','value'} and not known_keyword)
            if following<len(cleaned) and cleaned[following] not in '.!)]};[' and (
                    known_keyword or known_unary or not re.match(IDENT+r':',cleaned[following:])):
                message=('Keyword block invocation needs complete surrounding call ownership.' if known_keyword
                    else 'A following send/operator expression needs explicit surrounding ownership.')
                diagnostics.append(_issue('foreign-smalltalk-block-surrounding-expression-review',following,message))
    accounting=list(cleaned); cascades=[]
    for index,char in enumerate(cleaned):
        if owned(index): continue
        if char==';':
            callback_suffix=any(
                (suffix:=_structural_callback_suffix(cleaned,block['close'],contexts))
                and suffix[0]<=index<suffix[1] for block in blocks if block['close']<index)
            if callback_suffix:
                cascades.append(dict(offset=index,role='proved-callback-enclosing-punctuation'))
                continue  # Shared core owns this exact suffix, not the next send.
            if structural(contexts[index]):
                accounting[index]=' '
                cascades.append(dict(offset=index,role='owned-argument-or-data-composition'))
                continue
            lower=max([0]+[event['end'] for event in events if event['end']<=index])
            previous=None
            for cursor in range(lower,index):
                if cleaned[cursor]=='.' and not structural(contexts[cursor]): lower=cursor+1; previous=None
                elif cleaned[cursor]==';' and not structural(contexts[cursor]): previous=cursor
            start=previous+1 if previous is not None else lower
            finish=index+1
            while finish<len(cleaned):
                if owned(finish):
                    callback=next((event for event in events if event['kind']=='header' and event['scope']=='foreign-smalltalk-block' and event['start']<=finish<event['end']),None)
                    if callback and callback['end']<len(cleaned) and cleaned[callback['end']]=='[':
                        finish=callback['end']+1  # Proven block argument is a send value.
                    break
                if cleaned[finish] in '.;' and not structural(contexts[finish]): break
                finish+=1
            proven=_cascade_message(cleaned[start:index],initial=previous is None) and _cascade_message(cleaned[index+1:finish])
            cascades.append(dict(offset=index,role='independent-message-send' if proven else 'unproved-send'))
            if not proven:
                accounting[index]=' '
                diagnostics.append(_issue('foreign-smalltalk-cascade-role-review',index,'Cascade receiver/subsequent selector roles were not proved; no generic semicolon waiver.'))
        elif char=='.':
            if structural(contexts[index]):
                if not any(kind=='data' for _,kind,_ in contexts[index]):
                    diagnostics.append(_issue('foreign-smalltalk-parenthesized-sequence-review',index,'Period inside ordinary argument/group ownership requires grammar review.'))
            else: accounting[index]=';'
    regions=[dict(start=block['body_start'],end=block['close'],scope='foreign-smalltalk-block') for block in blocks]
    regions.extend(dict(start=method['body_start'],end=method['end'],scope='foreign-smalltalk-method') for method in methods)
    return dict(contexts=contexts,events=events,diagnostics=diagnostics,
        executable_regions=regions,accounting_cleaned=''.join(accounting),methods=methods,blocks=blocks,cascades=cascades)


def binding_counter(*,text,original,context,assignments):
    if structural(context,assignment=True): return dict(reason='structural-literal-or-message-argument-row')
    return dict(units=max(1,len(assignments)),declaration_bindings=len(assignments),
        reason='independent-period-statements-and-chained-smalltalk-bindings')


def normalize(source,language='smalltalk'):
    """Return an unadmitted grammar proposal with exact ownership evidence."""
    if language!='smalltalk': raise ValueError('Only explicit Smalltalk grammar is proposed.')
    masked=mask_smalltalk(source)
    classified=classify_scopes(masked['cleaned'],language,source=source,
        literals=masked['literals'],comments=masked['comments'])
    result=count_owned(source,classified['accounting_cleaned'],classified['contexts'],
        classified['events'],masked['diagnostics']+classified['diagnostics'],language,
        binding_counter=binding_counter,executable_regions=classified['executable_regions'],
        assignment_pattern=ASSIGN)
    result.update(proposal_version=PROPOSAL_VERSION,proposal_only=True,
        proposal_status='unhooked-no-admission',
        admission='Not admitted: locked fixtures, exact source/report pins, grammar policy, and source-specific diagnostics must be reviewed.',
        lexical_spans={key:masked[key] for key in ('literals','comments','numbers')},
        methods=classified['methods'],blocks=classified['blocks'],cascades=classified['cascades'])
    return result
