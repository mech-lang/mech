"""Bounded written GNU attribute ownership; no generated-code source units.

Input is already position-preserving comment/string-masked source. Known
compiler settings are structural arguments within one explicit written group.
Unknown annotation names, macros and malformed groups remain review gates.
No language/source/timing admission occurs in this helper.
"""
import re


GNU_SETTINGS={'aligned','always_inline','artificial','cold','const','deprecated',
    'error','externally_visible','flatten','format','format_arg','gnu_inline',
    'hot','leaf','malloc','may_alias','noinline','nonnull','noreturn','nothrow',
    'optimize','packed','pure','section','sentinel','target','unused','used',
    'vector_size','visibility','warn_unused_result','warning','weak'}


def _end(text,opening):
    stack=[]
    pairs={'(':')','[':']','{':'}'}
    for index in range(opening,len(text)):
        char=text[index]
        if char in pairs:stack.append(pairs[char])
        elif char in ')]}':
            if not stack or stack.pop()!=char:return None
            if not stack:return index+1
    return None


def _parts(text):
    values=[];start=0;index=0
    while index<len(text):
        if text[index] in '([{':
            end=_end(text,index)
            if end is None:return None
            index=end;continue
        if text[index] in ')]}':return None
        if text[index]==',':values.append(text[start:index].strip());start=index+1
        index+=1
    values.append(text[start:].strip())
    return values


def classify_gnu_attributes(cleaned,language):
    """Return exact one-unit groups and an equally sized grammar-only mask."""
    chars=list(cleaned);events=[];diagnostics=[];evidence=[]
    if language not in {'c','cpp'}:
        return dict(cleaned=cleaned,events=[],diagnostics=[],evidence=[])
    for match in re.finditer(r'\b__attribute(?:__)?\b',cleaned):
        start=match.start();cursor=match.end()
        if any(event['start']<=start<event['end'] for event in events):continue
        line_start=cleaned.rfind('\n',0,start)+1
        prefix=cleaned[line_start:start]
        previous=line_start-1
        while previous>=0:
            previous_start=cleaned.rfind('\n',0,previous)+1
            previous_line=cleaned[previous_start:previous].rstrip()
            if not previous_line.endswith('\\'):break
            prefix=previous_line+'\n'+prefix
            previous=previous_start-1
        if prefix.lstrip().startswith('#'):
            diagnostics.append(dict(kind='gnu-attribute-macro-owner-review',offset=start,
                message='Written attribute occurs in a preprocessor owner; no generated annotation units inferred.'))
            continue
        while cursor<len(cleaned) and cleaned[cursor].isspace():cursor+=1
        outer=_end(cleaned,cursor) if cursor<len(cleaned) and cleaned[cursor]=='(' else None
        inner_start=cursor+1
        while inner_start<len(cleaned) and cleaned[inner_start].isspace():inner_start+=1
        inner=_end(cleaned,inner_start) if outer and inner_start<outer and cleaned[inner_start]=='(' else None
        if not inner or cleaned[inner:outer-1].strip():
            diagnostics.append(dict(kind='gnu-attribute-group-owner-review',offset=start,
                message='GNU attribute requires one balanced double-parenthesized group.'))
            continue
        settings=[];parts=_parts(cleaned[inner_start+1:inner-1])
        for part in parts or []:
            setting=re.match(r'([A-Za-z_]\w*)\b',part)
            name=setting[1].strip('_') if setting else None
            tail=part[setting.end():].strip() if setting else part
            arguments=_end(tail,0) if tail.startswith('(') else None
            imperative_arguments=bool(any(char in tail for char in ';{}') or
                re.search(r'(?<![=!<>])(?:[+*/%&|^-]=|=(?!=|>))',tail))
            if (name not in GNU_SETTINGS or (tail and arguments!=len(tail)) or imperative_arguments):
                settings=[];break
            settings.append(name)
        if not settings:
            diagnostics.append(dict(kind='gnu-attribute-setting-owner-review',offset=start,end=outer,
                message='Unknown meaningful annotation/macro setting remains source-reviewed.'))
            continue
        event=dict(kind='compiler-attribute',start=start,end=outer,units=1,
            scope='gnu-written-attribute',settings=settings,
            reason='One written meaningful compiler attribute group; arguments structural, no generated-code units.')
        events.append(event);evidence.append(dict(event,source_text=cleaned[start:outer]))
        for index in range(start,outer):
            if chars[index] not in '\r\n':chars[index]=' '
    return dict(cleaned=''.join(chars),events=events,diagnostics=diagnostics,evidence=evidence)
