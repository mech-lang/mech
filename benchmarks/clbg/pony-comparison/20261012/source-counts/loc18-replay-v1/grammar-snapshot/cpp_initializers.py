"""Unhooked, source-owned C++ constructor member-initializer proposals.

The caller supplies offset-preserving comment/string-masked source, contexts,
and a proven type environment. No lexer, compiler, normalizer, source files or
timing are invoked here. Unknown ownership produces gates, not partial source
admission. Parent integration must replace overlapping prefix/open/close events
and apply the returned grammar-frame overrides before counting body statements.
"""
import re


NAME = r'[A-Za-z_]\w*'
QUALIFIED = NAME+r'(?:\s*::\s*'+NAME+r')*'
MODIFIERS = {'inline','constexpr','consteval','explicit'}
CONTROLS = {'if','while','for','switch','catch','sizeof','alignof','decltype','noexcept'}
EFFECT = re.compile(r'(?<![=!<>])(?:<<=|>>=|[+*/%&|^-]=|=(?!=|>))|\+\+|--|;|=>')


def _space(text,index):
    while index<len(text) and text[index].isspace(): index+=1
    return index


def _pairs(text):
    stack=[]; pairs={}
    for index,char in enumerate(text):
        if char in '([{': stack.append((char,index))
        elif char in ')]}':
            if not stack or stack[-1][0]!={')':'(',']':'[','}':'{'}[char]: return None
            _,opening=stack.pop(); pairs[opening]=index
    return None if stack else pairs


def _without_directives(text):
    chars=list(text);offset=0;continued=False
    for row in text.splitlines(keepends=True):
        directive=continued or bool(re.match(r'\s*#',row))
        if directive:
            chars[offset:offset+len(row)]=[c if c.isspace() else ' ' for c in row]
        continued=directive and row.rstrip('\r\n').endswith('\\')
        offset+=len(row)
    return ''.join(chars)


def _conditional_ranges(text):
    stack=[];ranges=[]
    for match in re.finditer(r'^[ \t]*#[ \t]*(if|ifdef|ifndef|endif)\b[^\n]*',text,re.M):
        if match[1]=='endif':
            if stack: ranges.append((stack.pop(),match.end()))
        else: stack.append(match.start())
    ranges.extend((start,len(text)) for start in stack)
    return ranges


def _issue(kind,offset,message,**more):
    return dict(kind=kind,offset=offset,message=message,**more)


def _enclosing_brace(text,pairs,offset):
    return max((opening for opening,closing in pairs.items()
        if text[opening]=='{' and opening<offset<closing),default=None)


def _signature_start(text,name_start):
    boundary=max(text.rfind(';',0,name_start),text.rfind('{',0,name_start),text.rfind('}',0,name_start))+1
    start=_space(text,boundary)
    access=re.match(r'(?:public|private|protected)\s*:\s*',text[start:name_start])
    if access: start=_space(text,start+access.end())
    prefix=text[start:name_start].strip()
    if prefix and not all(word in MODIFIERS for word in prefix.split()):
        if re.search(r'\b(?:return|new|throw|co_return)\b|[=?:+*/|]',prefix): return None
        return start,False
    return start,True


def _simple_arguments(text):
    """Reject independently written work rather than hiding it in a binding."""
    if EFFECT.search(text) or re.search(r'\b(?:return|throw|co_return|for|while|if|switch|try|catch|goto|do)\b',text): return False
    # An actual lambda/executable argument needs separate header/body owners.
    if re.search(r'\][^;{}]*\{',text): return False
    return not re.search(r'[^A-Za-z0-9_\s.,:?+*/%&|^~!<>=()\[\]{}-]',text)


def _initializer_list(text,colon,pairs):
    """Read complete independent named initializer groups to the body brace."""
    cursor=colon+1;members=[]
    while True:
        cursor=_space(text,cursor); start=cursor
        named=re.match(QUALIFIED,text[cursor:])
        if named is None: return None
        cursor=_space(text,cursor+named.end())
        if text[cursor:cursor+1] not in {'(','{'} or cursor not in pairs: return None
        opening=cursor; closing=pairs[opening]
        members.append(dict(name=re.sub(r'\s+','',named[0]),start=start,end=closing+1,
            argument_start=opening+1,argument_end=closing,opening=opening,
            delimiter=text[opening],scalar_arguments=_simple_arguments(text[opening+1:closing]),
            wrapped_arguments='\n' in text[opening:closing+1]))
        cursor=_space(text,closing+1)
        if text[cursor:cursor+1]==',':
            members[-1]['separator_start']=cursor
            members[-1]['separator_end']=cursor+1
            cursor+=1; continue
        if text[cursor:cursor+1]=='{' and cursor in pairs:
            return dict(members=members,body_opening=cursor,body_closing=pairs[cursor])
        return None


def _candidates(text,pairs):
    candidates=[]
    for named in re.finditer(r'(?<!\w)('+QUALIFIED+r')\s*\(',text):
        # A direct access label may end immediately before the constructor
        # name (`public:N()`). Do not recognize a partial qualified name.
        if named.start() and text[named.start()-1]==':' and (
                named.start()>1 and text[named.start()-2]==':'):
            continue
        name=re.sub(r'\s+','',named[1]); base=name.rsplit('::',1)[-1]
        if base in CONTROLS: continue
        opening=named.end()-1;closing=pairs.get(opening)
        if closing is None: continue
        cursor=_space(text,closing+1);complex_header=False
        noexc=re.match(r'noexcept\b',text[cursor:])
        if noexc:
            cursor=_space(text,cursor+noexc.end())
            if text[cursor:cursor+1]=='(':
                end=pairs.get(cursor)
                if end is None: continue
                complex_header=not _simple_arguments(text[cursor+1:end])
                cursor=_space(text,end+1)
        if text[cursor:cursor+1]!=':' or text[cursor:cursor+2]=='::': continue
        signature=_signature_start(text,named.start())
        if signature is None: continue  # A return/call/ternary is not a constructor signature.
        start,proven_prefix=signature
        formal=text[opening+1:closing]
        complex_header=complex_header or not proven_prefix or bool(re.search(r';|=>|\b(?:return|throw|for|while|if|switch|try)\b|\][^;{}]*\{|=\s*[^,]*=(?!=|>)',formal))
        parsed=_initializer_list(text,cursor,pairs)
        candidates.append(dict(name=name,base_name=base,start=start,name_start=named.start(),
            formal_opening=opening,formal_closing=closing,colon=cursor,
            complex_header=complex_header,parsed=parsed))
    return candidates


def _source_types(environment):
    return [entry for definitions in environment.get('aliases',{}).values() for entry in definitions
        if entry.get('kind')=='source-local-cpp-named-type'
        and entry.get('aggregate_kind') in {'class','struct'}
        and entry.get('body_opening') is not None and entry.get('body_end') is not None]


def _qualification(text,pairs,entry,types):
    """Prove explicit namespace/class ancestors; local executable scope gates."""
    containers={item['body_opening']:item['name'] for item in types}
    for match in re.finditer(r'\bnamespace\s+('+QUALIFIED+r')\s*\{',text):
        containers[match.end()-1]=re.sub(r'\s+','',match[1])
    ancestors=[];opening=entry.get('scope_opening')
    while opening is not None:
        if opening not in containers: return None
        ancestors.append(containers[opening])
        opening=_enclosing_brace(text,pairs,opening)
    return '::'.join(list(reversed(ancestors))+[entry['name']])


def _constructor_owner(text,pairs,candidate,types,environment):
    name=candidate['name'];base=candidate['base_name'];at=candidate['name_start']
    if '::' not in name:
        enclosing=_enclosing_brace(text,pairs,at)
        eligible=[entry for entry in types if entry['name']==base and entry['body_opening']==enclosing]
    else:
        owner_name=name.rsplit('::',1)[0]
        eligible=[]
        for entry in types:
            if entry['name']!=base: continue
            qualified=_qualification(text,pairs,entry,types)
            if qualified is None: continue
            in_original_scope=entry['visible_start']<=at<entry['scope_end']
            if owner_name==qualified or owner_name==base and in_original_scope: eligible.append(entry)
    if len(eligible)!=1: return None,'No unique source-local class/struct constructor declaration owner.'
    entry=eligible[0]
    if entry.get('conditional'): return None,'Conditional type declaration cannot prove a constructor owner.'
    if any(shadow['name']==entry['name'] and shadow['start']<=at<shadow['end']
            and shadow['scope_start']>=entry['scope_start'] for shadow in environment.get('shadows',[])):
        return None,'Source-local type name has a value/unknown shadow in the constructor scope.'
    return entry,None


def _member_declarations(text,pairs,entry,candidates,environment,resolver,conditional):
    """Read direct class data declarations; skip all nested executable bodies."""
    fields={};evidence=[];cursor=entry['body_opening']+1;start=cursor
    stop=entry['body_end']-1
    ctor_spans={candidate['start']:candidate['parsed']['body_closing']+1 for candidate in candidates
        if candidate['parsed'] and _enclosing_brace(text,pairs,candidate['name_start'])==entry['body_opening']}
    while cursor<stop:
        if cursor==_space(text,start):
            access=re.match(r'(?:public|private|protected)\s*:\s*',text[cursor:stop])
            if access: cursor+=access.end();start=cursor;continue
            ctor_end=ctor_spans.get(cursor)
            if ctor_end is not None: cursor=ctor_end;start=cursor;continue
        char=text[cursor]
        if char in '([':
            closing=pairs.get(cursor)
            if closing is None: break
            cursor=closing+1;continue
        if char=='{':
            closing=pairs.get(cursor)
            if closing is None: break
            prefix=text[start:cursor]
            executable='(' in prefix or bool(re.search(r'\b(?:class|struct|union|enum)\b',prefix))
            cursor=closing+1
            if executable: start=cursor
            continue
        if char==';':
            left=_space(text,start);segment=text[left:cursor]
            declaration=resolver(segment,environment,offset=left) if segment.strip() else None
            if declaration:
                static=bool(re.search(r'\b(?:static|thread_local)\b',segment))
                conditional_field=any(begin<=left<end for begin,end in conditional)
                for item in declaration['declarators']:
                    row=dict(name=item['name'],start=left,end=cursor+1,
                        static=static,conditional=conditional_field,type_name=declaration['type_name'],
                        type_evidence=declaration.get('type_evidence'),source_text=segment)
                    evidence.append(row)
                    if not static and not conditional_field: fields.setdefault(row['name'],[]).append(row)
            cursor+=1;start=cursor;continue
        cursor+=1
    return fields,evidence


def classify_cpp_initializers(cleaned,language,contexts,type_environment,*,declaration_resolver):
    """Return unhooked events, exact owner evidence and integration contracts.

    ``declaration_resolver`` follows identify_decl(segment,environment,offset=)
    and must return explicit declarator names with source/type provenance.
    Non-C++ input is empty. No arbitrary alias/uppercase/name is constructor
    proof. Each accepted owner replaces only its prefix/open/close canonical
    events; body bindings, body owners and its written closing unit remain.
    """
    result=dict(events=[],owners=[],evidence=[],diagnostics=[],context_overrides=[],
        executable_regions=[],suppress_canonical_spans=[],proposal_only=True)
    if language!='cpp': return result
    if len(contexts)!=len(cleaned): raise ValueError('C++ initializer context offsets must match exact cleaned text.')
    scan=_without_directives(cleaned);pairs=_pairs(scan)
    if pairs is None:
        result['diagnostics'].append(_issue('cpp-initializer-delimiter-owner-review',0,'Unbalanced source cannot prove initializer ownership.'))
        return result
    conditional=_conditional_ranges(cleaned)
    types=_source_types(type_environment);candidates=_candidates(scan,pairs);field_cache={}
    for candidate in candidates:
        start=candidate['start'];parsed=candidate['parsed']
        owner,reason=_constructor_owner(scan,pairs,candidate,types,type_environment)
        if reason:
            result['diagnostics'].append(_issue('cpp-initializer-constructor-owner-review',start,reason,name=candidate['name']))
            continue
        if parsed is None or candidate['complex_header']:
            result['diagnostics'].append(_issue('cpp-initializer-list-owner-review',start,'Initializer list/header lacks complete balanced bounded ownership.'))
            continue
        if any(begin<parsed['body_opening'] and start<end for begin,end in conditional):
            result['diagnostics'].append(_issue('cpp-initializer-conditional-owner-review',start,'Conditional constructor header/initializer list requires an explicit source configuration owner.'))
            continue
        key=owner['body_opening']
        if key not in field_cache: field_cache[key]=_member_declarations(scan,pairs,owner,candidates,type_environment,declaration_resolver,conditional)
        fields,field_evidence=field_cache[key];seen=set();issues=[]
        for member in parsed['members']:
            name=member['name']
            if name==owner['name'] or name.endswith('::'+owner['name']):
                issues.append(_issue('cpp-delegating-initializer-owner-review',member['start'],'Delegating constructor initialization is a distinct unapproved owner.',name=name))
            elif '::' in name or len(fields.get(name,[]))!=1:
                issues.append(_issue('cpp-member-base-initializer-owner-review',member['start'],'Initializer is not one uniquely proven direct nonstatic member; base/static/unknown roles remain gated.',name=name))
            elif name in seen:
                issues.append(_issue('cpp-duplicate-member-initializer-review',member['start'],'Duplicate member initializer cannot supply an independent admitted binding.',name=name))
            if not member['scalar_arguments']:
                issues.append(_issue('cpp-initializer-argument-work-review',member['argument_start'],'Initializer arguments contain written effects/executable/unknown work that cannot be hidden in one member unit.',name=name))
            if member['wrapped_arguments']:
                issues.append(_issue('cpp-wrapped-initializer-data-review',member['opening'],'Multiline initializer arguments need exact literal/data-row preservation evidence before one member event can own their span.',name=name))
            seen.add(name)
        if issues:
            result['diagnostics'].extend(issues);continue
        opening=parsed['body_opening'];closing=parsed['body_closing']
        result['events'].append(dict(kind='header',start=start,end=candidate['colon']+1,units=1,scope='cpp-constructor-header'))
        for member in parsed['members']:
            result['events'].append(dict(kind='member-initializer',start=member['start'],end=member.get('separator_end',member['end']),units=1,
                scope='cpp-explicit-member-initializer',name=member['name'],member_evidence=fields[member['name']][0]))
        result['events'].extend([dict(kind='open',start=opening,end=opening+1,units=0,scope='cpp-constructor-body'),
            dict(kind='close',start=closing,end=closing+1,units=1,scope='cpp-constructor-body')])
        row=dict(start=start,end=closing+1,prefix_end=opening,body_opening=opening,body_closing=closing,
            formal_opening=candidate['formal_opening'],formal_closing=candidate['formal_closing'],
            constructor_name=candidate['name'],type_owner=owner,members=parsed['members'],
            member_declarations=field_evidence,header_units=1,member_units=len(parsed['members']),written_close_units=1,
            body_units='Caller counts every independent written body statement/binding without replacement.')
        result['owners'].append(row);result['evidence'].append(dict(row))
        result['suppress_canonical_spans'].extend([dict(start=start,end=opening,role='constructor-prefix'),
            dict(start=opening,end=opening+1,role='constructor-open'),dict(start=closing,end=closing+1,role='constructor-close')])
        result['context_overrides'].append(dict(start=candidate['formal_opening']+1,end=candidate['formal_closing']+1,
            opening=candidate['formal_opening'],delimiter='(',kind='parameters'))
        for member in parsed['members']:
            result['context_overrides'].append(dict(start=member['opening']+1,end=member['end'],opening=member['opening'],
                delimiter=member['delimiter'],kind='data' if member['delimiter']=='{' else 'args'))
        result['context_overrides'].append(dict(start=opening+1,end=closing+1,opening=opening,delimiter='{',kind='code'))
        result['executable_regions'].append(dict(start=opening+1,end=closing,scope='cpp-constructor-body'))
    return result
