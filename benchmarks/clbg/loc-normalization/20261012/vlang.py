"""Unhooked, report-only V canonical LOC proposal.

Use the explicit language key ``vlang``; ``v`` is a Verilog lexer alias.
This module never collects chart candidates, changes source bytes or measures
programs. Execute normalization/fixtures only while owning the common mutex.
The shared canonical helpers are reused without modifying their frozen code.
"""
import re
from bisect import bisect_right

from canonical import canonical_units
from normalizer import ASSIGN, RULE_VERSION, _scope_map, _structural, _top_parts

V_RULE_VERSION = RULE_VERSION + '-vlang-report-only-proposal-1'
TYPE_CASTS = {'bool','byte','byteptr','char','charptr','int','i8','i16','i32','i64',
              'isize','u8','u16','u32','u64','usize','f32','f64','rune','string',
              'voidptr'}
VOID_CALLS = {'panic','exit','print','println','eprint','eprintln','assert'}


def _replacement(text, literal=False):
    return ''.join(c if c.isspace() else 'S' if literal else ' ' for c in text)


def mask_vlang_source(source):
    """Mask nested comments and literals with unchanged character positions.

    String interpolation belongs to its surrounding expression. Statement-like
    interpolation is diagnosed instead of being mistaken for program braces.
    Raw strings and rune literals have distinct escape treatment.
    """
    chars=list(source);diagnostics=[];literal_spans=[]

    def issue(kind,start,message):
        diagnostics.append(dict(kind=kind,offset=start,message=message))

    def comment(start):
        index=start+2;depth=1
        while index<len(source) and depth:
            if source.startswith('/*',index):depth+=1;index+=2
            elif source.startswith('*/',index):depth-=1;index+=2
            else:index+=1
        if depth:issue('vlang-unterminated-block-comment',start,'Nested block comment has no matching close.')
        return index

    def literal(start,raw=False):
        delimiter=source[start];index=start+1;closed=False
        while index<len(source):
            if not raw and source[index]=='\\':
                index=min(index+2,len(source));continue
            if not raw and delimiter!='`' and source.startswith('${',index):
                interpolation_start=index;index+=2;depth=1
                body_start=index;literal_or_comment_ranges=[]
                while index<len(source) and depth:
                    if source.startswith('/*',index):
                        nested_start=index;index=comment(index)
                        literal_or_comment_ranges.append((nested_start,index))
                    elif source.startswith('//',index):
                        nested_start=index
                        end=source.find('\n',index);index=len(source) if end<0 else end
                        literal_or_comment_ranges.append((nested_start,index))
                    elif source[index]=='r' and index+1<len(source) and source[index+1] in "'\"" and (index==body_start or not (source[index-1].isalnum() or source[index-1]=='_')):
                        nested_start=index;index=literal(index+1,raw=True)
                        literal_or_comment_ranges.append((nested_start,index))
                    elif source[index] in "'\"`":
                        nested_start=index;index=literal(index)
                        literal_or_comment_ranges.append((nested_start,index))
                    elif source[index]=='{':depth+=1;index+=1
                    elif source[index]=='}':depth-=1;index+=1
                    else:index+=1
                if depth:
                    issue('vlang-unterminated-interpolation',interpolation_start,'String interpolation has no matching brace.')
                body=list(source[body_start:max(body_start,index-1)])
                for a,b in literal_or_comment_ranges:
                    body[a-body_start:b-body_start]=_replacement(source[a:b],literal=True)
                body=''.join(body)
                if (';' in body or ASSIGN.search(body) or
                        re.search(r'\b(?:fn|if|for|match|unsafe|return|or)\b|[{}]',body)):
                    issue('vlang-interpolation-code-review',interpolation_start,
                          'Interpolation contains a binding/block/statement form; review its source semantics.')
                continue
            if source[index]==delimiter:index+=1;closed=True;break
            index+=1
        if not closed:issue('vlang-unterminated-literal',start,'Quoted string or rune literal is unterminated.')
        return index

    index=0
    while index<len(source):
        if source.startswith('//',index):
            start=index;end=source.find('\n',index);index=len(source) if end<0 else end
            chars[start:index]=_replacement(source[start:index])
        elif source.startswith('/*',index):
            start=index;index=comment(index);chars[start:index]=_replacement(source[start:index])
        elif source[index]=='r' and index+1<len(source) and source[index+1] in "'\"" and (index==0 or not (source[index-1].isalnum() or source[index-1]=='_')):
            start=index;index=literal(index+1,raw=True)
            chars[start:index]=_replacement(source[start:index],literal=True)
            literal_spans.append((start,index))
        elif source[index] in "'\"`":
            start=index;index=literal(index)
            chars[start:index]=_replacement(source[start:index],literal=True)
            literal_spans.append((start,index))
        else:index+=1
    return dict(cleaned=''.join(chars),diagnostics=diagnostics,literal_spans=literal_spans)


def _brace_pairs(text):
    stack=[];pairs={}
    for index,char in enumerate(text):
        if char=='{':stack.append(index)
        elif char=='}' and stack:pairs[stack.pop()]=index
    return pairs


def _compiler_pragmas(text):
    """Keep written annotations/directives independently of header packing."""
    spans=[];diagnostics=[]
    for match in re.finditer(r'^\s*#[^\r\n]*',text,re.M):
        start=match.start()
        while start<match.end() and text[start].isspace():start+=1
        spans.append((start,match.end()))
    for match in re.finditer(r'@\[',text):
        if any(a<=match.start()<b for a,b in spans):continue
        index=match.end();depth=1
        while index<len(text) and depth:
            if text[index]=='[':depth+=1
            elif text[index]==']':depth-=1
            index+=1
        if depth:diagnostics.append(dict(kind='vlang-unterminated-annotation',offset=match.start(),
            message='Compiler annotation has no matching bracket.'))
        else:spans.append((match.start(),index))
    return spans,diagnostics


def _value_body(body):
    """Conservative expression classification, never a benchmark-specific rule."""
    code=body.strip()
    if not code:return 'unknown'
    if ASSIGN.search(code) or ';' in code or '++' in code or '--' in code:return 'imperative'
    if '<<' in code:return 'unknown' # scalar shift versus array mutation needs types
    if re.search(r'\b(?:return|break|continue|defer|for|assert)\b',code):return 'imperative'
    calls=re.findall(r'\b([A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*)\s*\(',code)
    if any(call in VOID_CALLS for call in calls):return 'imperative'
    if re.search(r'\b(?:if|else|match|fn|unsafe|or|lock|rlock)\b',code):return 'unknown'
    if any(call not in TYPE_CASTS for call in calls):return 'unknown'
    # Multiple top-level source rows could be two independent expressions.
    depth=0;top_rows=0
    for row in code.splitlines():
        if row.strip() and depth==0:top_rows+=1
        depth+=sum(row.count(c) for c in '([{')-sum(row.count(c) for c in ')]}')
    return 'value' if top_rows<=1 else 'unknown'


def _expression_prefix(text,index):
    start=text.rfind('\n',0,index)+1
    prefix=re.split(r'[;{}]',text[start:index])[-1].strip()
    if not prefix:
        previous=text[:start].rstrip().rsplit('\n',1)[-1]
        return bool(re.search(r'(?:\breturn|:=|(?<![=!<>])=|[\[(,+*/%-])\s*$',previous))
    return bool(ASSIGN.search(prefix) or re.search(r'\breturn\s*$',prefix) or
                re.search(r'[(,=+*/%?:-]\s*$',prefix))


def _pure_expressions(cleaned,pairs):
    """Recognize value-only if chains and or/unsafe fallback compositions."""
    spans=[];diagnostics=[]
    def covered(index):return any(a<=index<b for a,b in spans)
    def issue(kind,index):diagnostics.append(dict(kind=kind,offset=index,
        message='Expression block is not provably a single value expression; review exact source.'))
    for match in re.finditer(r'\b(?:if|or|unsafe)\b',cleaned):
        start=match.start();word=match[0]
        if covered(start):continue
        if word=='if' and not _expression_prefix(cleaned,start):continue
        if word=='unsafe' and not _expression_prefix(cleaned,start):continue
        opening=cleaned.find('{',match.end())
        if opening<0 or opening not in pairs:continue
        if '\n' in cleaned[match.end():opening] and word in {'or','unsafe'}:
            issue('vlang-expression-header-review',start);continue
        category=_value_body(cleaned[opening+1:pairs[opening]])
        if word in {'or','unsafe'}:
            if category=='value':spans.append((start,pairs[opening]+1))
            elif category=='unknown':issue('vlang-'+word+'-body-review',start)
            continue
        branches=[];end=pairs[opening]+1;ok=category=='value';branches.append(category)
        while True:
            tail=re.match(r'\s*else\b(\s+if\b)?',cleaned[end:])
            if not tail:ok=False;break
            branch_start=end+tail.end();branch_open=cleaned.find('{',branch_start)
            if branch_open<0 or branch_open not in pairs:ok=False;break
            category=_value_body(cleaned[branch_open+1:pairs[branch_open]])
            branches.append(category);ok=ok and category=='value';end=pairs[branch_open]+1
            if tail[1] is None:break
        if ok:spans.append((start,end))
        else:issue('vlang-conditional-expression-review',start)
    return spans,diagnostics


def _multiple_bindings(text):
    found=re.match(r'^\s*((?:mut\s+)?[A-Za-z_]\w*(?:\s*,\s*(?:mut\s+)?[A-Za-z_]\w*)+)\s*(?::=|=)(.*)$',text,re.S)
    if not found:return 0,None
    names=_top_parts(found[1]);values=_top_parts(found[2])
    if len(values)==1:return 1,'tuple-destructuring'
    if len(values)==len(names):return len(names),'independent-multiple-bindings'
    return 1,'ambiguous-multiple-binding-arity'


def normalize_vlang(source,language='vlang'):
    """Return a source-keyed review proposal using the approved canonical rule.

    The explicit ``vlang`` key is the only accepted language name. This API does
    not certify ambiguous forms; callers must retain diagnostics and audit pins.
    """
    if language!='vlang':raise ValueError('Use explicit vlang, never the Verilog alias v.')
    mask=mask_vlang_source(source);cleaned=mask['cleaned']
    pragmas,pragma_issues=_compiler_pragmas(cleaned)
    prepared=list(cleaned)
    for start,end in pragmas:prepared[start:end]=_replacement(cleaned[start:end])
    prepared=''.join(prepared)
    pairs=_brace_pairs(prepared)
    pure,diagnostics=_pure_expressions(prepared,pairs)
    diagnostics=[*mask['diagnostics'],*pragma_issues,*diagnostics]
    transformed=list(prepared)
    for start,end in pure:transformed[start:end]=_replacement(prepared[start:end])
    transformed=''.join(transformed)
    contexts,scope_issues=_scope_map(transformed,'go');diagnostics.extend(scope_issues)
    # V permits a counted loop's body brace on a later row. Its initializer,
    # condition and increment remain one structural header in both spellings.
    # Nested constructors/handlers in headers keep their existing review gates.
    loop_headers=[(m.start(),m.end()-1) for m in re.finditer(r'\bfor\b\s*[^{}]*\{',transformed)]
    overrides={};or_blocks={};type_blocks=set();type_closes={}
    for opening,closing in _brace_pairs(transformed).items():
        prefix=transformed[:opening].rstrip();line=prefix.rsplit('\n',1)[-1]
        if (re.search(r'\b(?:for|if|while)\b[^;{}]*$',line) and
                re.search(r'(?:\[\][A-Za-z_]\w*|map\[[^]]+\][A-Za-z_]\w*|[A-Z][A-Za-z_0-9]*)\s*$',line)):
            diagnostics.append(dict(kind='vlang-constructor-in-control-header-review',offset=opening,
                message='Possible typed constructor versus range/condition body needs full header grammar review.'))
        or_header=re.search(r'\bor\s*$',prefix)
        if or_header:
            overrides[opening]='code';or_blocks[opening]=or_header.start()
            control_prefix=re.split(r'[{}]',line)[-1]
            if re.search(r'\b(?:for|if|while)\b',control_prefix):
                diagnostics.append(dict(kind='vlang-handler-in-control-header-review',offset=or_header.start(),
                    message='Imperative handler inside a control header needs full nested-header grammar review.'))
        elif re.search(r'\b(?:struct|enum|interface|union)\s+[A-Za-z_]\w*\s*$',line):
            overrides[opening]='code';type_blocks.add(opening);type_closes[opening]=closing
        elif re.search(r'\bdefer\s*$',line):
            overrides[opening]='code'
        elif re.search(r'\b(?:unsafe|lock|rlock|match|select)\b[^;{}]*$',line):
            overrides[opening]='code'
            diagnostics.append(dict(kind='vlang-imperative-block-review',offset=opening,
                message='Review imperative unsafe/shared/match/select grammar before report admission.'))
        if any(end==opening for _,end in loop_headers):
            overrides[opening]='code'
    if overrides:
        contexts=[tuple((char,overrides.get(index,kind),index) for char,kind,index in row) for row in contexts]
    if loop_headers:
        contexts=[tuple(row)+tuple(('(','for',start) for start,end in loop_headers if start<=index<end)
                  for index,row in enumerate(contexts)]
    canonical=canonical_units(transformed,'go',contexts)
    # An imperative or handler has a separate header from its surrounding
    # assignment/return expression. Its open and explicit close follow the same
    # approved rule as an imperative if/function body.
    for event in canonical['events']:
        if event['kind']=='header' and event['end'] in or_blocks:
            event['start']=or_blocks[event['end']];event['scope']='vlang-or'
    diagnostics.extend(canonical['diagnostics'])
    for opening,closing in type_closes.items():
        for match in re.finditer(r'\b(?:pub\s+mut|pub|mut|global)\s*:',transformed[opening+1:closing]):
            start=opening+1+match.start();end=opening+1+match.end()
            context=contexts[start]
            if context and context[-1][2]==opening:
                canonical['events'].append(dict(kind='declaration-access-header',start=start,end=end,
                    units=1,scope='vlang-type-declaration'))
        for index in range(opening+1,closing):
            if transformed[index]==',' and contexts[index] and contexts[index][-1][2]==opening:
                diagnostics.append(dict(kind='vlang-packed-type-declaration-review',offset=index,
                    message='Comma-separated declaration fields/members require grammar review; constructors remain structural data.'))
    events=(canonical['events']+
        [dict(kind='value-expression',start=a,end=b,units=0,scope='vlang-pure-expression') for a,b in pure]+
        [dict(kind='compiler-pragma',start=a,end=b,units=1,scope='vlang-pragma') for a,b in pragmas])
    events.sort(key=lambda row:(row['start'],row['end']))
    for match in re.finditer(r'\$(?:if|for)|\bfn\s*(?:[A-Za-z_]\w*\s*)?\[|\bfn\s*\([^)]*\)\s+[A-Za-z_]\w*\s*\(',cleaned):
        diagnostics.append(dict(kind='vlang-complex-declaration-review',offset=match.start(),
            message='Compile-time/captured/receiver function form needs source-specific grammar review.'))
    body=list(cleaned)
    for event in events:
        body[event['start']:event['end']]=_replacement(cleaned[event['start']:event['end']])
    body=''.join(body)
    starts=[0]+[m.end() for m in re.finditer('\n',source)]
    def line_of(index):return bisect_right(starts,index)
    physical=[i for i,row in enumerate(cleaned.splitlines(keepends=True),1) if row.strip()]
    canonical_by_line={};owned=set();statement_events={i:[] for i in physical}
    for event in events:
        canonical_by_line.setdefault(line_of(event['start']),[]).append(event)
        owned.update(range(line_of(event['start']),line_of(max(event['start'],event['end']-1))+1))
    boundaries={index for event in events if event['scope']!='vlang-pure-expression'
                for index in (event['start'],event['end'])}
    body_lines=body.splitlines(keepends=True)
    for start,end in pure:
        if line_of(start)!=line_of(end-1):
            suffix=body_lines[line_of(end-1)-1][end-starts[line_of(end-1)-1]:].strip()
            if suffix.strip(';'):
                diagnostics.append(dict(kind='vlang-value-expression-continuation-review',offset=end,
                    message='Wrapped composed expression has a trailing outer continuation; review its logical statement span.'))
    for line_number in physical:
        row=body_lines[line_number-1];offset=starts[line_number-1]
        semicolons=[i for i,c in enumerate(row) if c==';' and not _structural(contexts[offset+i],language='go')]
        bounds=sorted({0,len(row),*(i+1 for i in semicolons),*(i-offset for i in boundaries if offset<i<offset+len(row))})
        for left,right in zip(bounds,bounds[1:]):
            text=row[left:right].rstrip().removesuffix(';').rstrip()
            if not text.strip('{}; \t\r\n'):continue
            assignments=[dict(column=left+m.start()+1,operator=m[0]) for m in ASSIGN.finditer(text)
                if not _structural(contexts[offset+left+m.start()],assignment=True,language='go')]
            declarations,reason=_multiple_bindings(text)
            if reason=='ambiguous-multiple-binding-arity':
                diagnostics.append(dict(kind='vlang-multiple-binding-review',offset=offset+left,
                    message='Unequal initializer arity requires tuple/independent-binding review.'))
            units=max(1,len(assignments),declarations)
            statement_events[line_number].append(dict(start_column=left+1,end_column=right+1,
                units=units,assignments=assignments,reason=reason or 'independent-statement-and-chained-bindings'))
    expansions=[];total=0;original_lines=source.splitlines()
    for line_number in physical:
        headers=canonical_by_line.get(line_number,[])
        statements=statement_events[line_number]
        units=sum(e['units'] for e in headers)+sum(e['units'] for e in statements)
        if units==0 and line_number not in owned:units=1
        total+=units
        if units!=1 or any(e['units']>1 for e in statements) or headers:
            expansions.append(dict(line=line_number,original=original_lines[line_number-1],
                physical_units=1,normalized_units=units,extra_lines=units-1,
                canonical_events=headers,statement_events=statements))
    return dict(rule_version=V_RULE_VERSION,language='vlang',physical_loc=len(physical),
        normalized_loc=total,expansions=expansions,canonical_events=events,
        diagnostics=diagnostics,review_status='needs-review' if diagnostics else 'candidate-parser-pass',
        chart_pool_eligible=False,scope='V report-only source normalization proposal')
