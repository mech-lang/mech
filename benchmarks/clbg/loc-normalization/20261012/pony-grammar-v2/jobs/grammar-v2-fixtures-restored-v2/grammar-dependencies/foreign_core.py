"""Unhooked owner/event accounting shared by the foreign-language proposals.

Language classifiers supply proven header/body/data ownership and declarations.
This module never selects a candidate, hashes a source, imports a producer or
admits a chart count. Execute it only under the common benchmark timing mutex.
"""
import re
from bisect import bisect_right

from pygments.lexers import get_lexer_by_name
from pygments.token import Comment, Error, Literal
from canonical import _structural_callback_suffix


RULE_VERSION = 'canonical-source-units-20261012-approved-rule-v2'
EXECUTABLE = {'code', 'bare-code', 'type'}
ASSIGN = re.compile(r'(?<![=!<>~:])(?:\*\*=|<<=|>>=|//=|[+*/%&|^-]=|:=|=(?!=|>|~))')


def replacement(text, literal=False):
    return ''.join(c if c.isspace() else 'S' if literal else ' ' for c in text)


def mask_foreign(source, lexer_name, semantic_directive_prefixes=()):
    """Mask comments/literals with exact character and line positions.

    A supplied directive prefix is explicit language grammar, not a waiver for
    arbitrary comments. Directive rows are retained as meaningful source units;
    the language classifier must own them as one pragma event before parsing.
    """
    prepared = (' ' + source[1:]) if source.startswith('\ufeff') else source
    lexer = get_lexer_by_name(lexer_name, stripnl=False, ensurenl=False, tabsize=0)
    lexical = prepared if prepared.endswith('\n') else prepared + '\n'
    chars = list(prepared)
    diagnostics = []
    covered = 0
    for index, token, value in lexer.get_tokens_unprocessed(lexical):
        if index >= len(prepared):
            continue
        end = min(index + len(value), len(prepared))
        value = value[:end-index]
        if index != covered:
            diagnostics.append(dict(kind='foreign-lexer-span-discontinuity', offset=index))
        if lexical[index:end] != value:
            diagnostics.append(dict(kind='foreign-lexer-span-mismatch', offset=index))
            covered = max(covered, end)
            continue
        covered = max(covered, end)
        if token in Error:
            diagnostics.append(dict(kind='foreign-lexer-error', offset=index,
                length=len(value), token=str(token)))
        if token in Comment and token not in Comment.Preproc:
            chars[index:end] = replacement(value)
        elif token in Literal.String:
            chars[index:end] = replacement(value, literal=True)
    if covered != len(prepared):
        diagnostics.append(dict(kind='foreign-lexer-coverage-mismatch', offset=covered))
    directives = []
    offset = 0
    for row in prepared.splitlines(keepends=True):
        code = row.lstrip()
        if any(code.startswith(prefix) for prefix in semantic_directive_prefixes):
            start = offset + len(row)-len(code)
            end = offset + len(row.rstrip('\r\n'))
            chars[start:end] = prepared[start:end]
            directives.append(dict(kind='compiler-pragma', start=start, end=end,
                units=1, scope='foreign-pragma'))
        offset += len(row)
    return dict(cleaned=''.join(chars), diagnostics=diagnostics, directives=directives)


def brace_pairs(text):
    stack = []
    pairs = {}
    diagnostics = []
    for index, char in enumerate(text):
        if char in '([{':
            stack.append((char,index))
        elif char in ')]}':
            wanted = {')':'(', ']':'[', '}':'{'}[char]
            if not stack or stack[-1][0] != wanted:
                diagnostics.append(dict(kind='foreign-unbalanced-delimiter', offset=index,
                    character=char))
            else:
                opening, start = stack.pop()
                pairs[start] = index
    diagnostics.extend(dict(kind='foreign-unclosed-delimiter', offset=index,
        character=char) for char,index in stack)
    return pairs, diagnostics


def structural(context, assignment=False, named_equals=False):
    last_code = max((i for i,(_,kind,_) in enumerate(context) if kind in EXECUTABLE), default=-1)
    relevant = context[last_code+1:]
    if any(kind in {'data', 'parameters', 'for', 'control'} for _,kind,_ in relevant):
        return True
    return any(kind == 'args' for _,kind,_ in relevant) if not assignment or named_equals else False


def top_parts(text, separator=','):
    stack = []
    parts = []
    start = 0
    for index,char in enumerate(text):
        if char in '([{':
            stack.append(char)
        elif char in ')]}' and stack:
            stack.pop()
        elif char == separator and not stack:
            parts.append(text[start:index].strip())
            start = index + 1
    parts.append(text[start:].strip())
    return parts


def count_owned(source, cleaned, contexts, events, diagnostics, language,
                binding_counter=None, executable_regions=(), named_equals=False,
                assignment_pattern=ASSIGN):
    """Count approved owner events plus independent remaining source units.

    ``binding_counter`` receives keyword arguments ``text``, ``original``,
    ``context`` and ``assignments`` and returns a mapping with optional units,
    declaration_bindings, reason and diagnostics. Opaque composed-value events
    use scope ``value-expression`` and do not split the outer statement.
    All spans are half-open character offsets; no supplied source is mutated.
    """
    if len(contexts) != len(cleaned) or len(cleaned) != len(source):
        raise ValueError('Foreign ownership must preserve every source character.')
    diagnostics = list(diagnostics)
    events = list(events)
    for close in list(events):
        if close['kind'] != 'close':
            continue
        suffix = _structural_callback_suffix(cleaned,close['end']-1,contexts)
        if suffix and not any(e['start']<=suffix[0] and suffix[1]<=e['end'] for e in events):
            events.append(dict(kind='structural-suffix',start=suffix[0],end=suffix[1],
                units=0,scope='callback-argument-data'))
    events = sorted(events, key=lambda e:(e['start'],e['end']))
    for left,right in zip(events,events[1:]):
        if left['end'] > right['start']:
            diagnostics.append(dict(kind='foreign-overlapping-owner-events', offset=right['start']))
    body = list(cleaned)
    for event in events:
        if not 0 <= event['start'] < event['end'] <= len(cleaned):
            raise ValueError('Foreign owner span is outside exact source positions.')
        body[event['start']:event['end']] = replacement(cleaned[event['start']:event['end']])
    body = ''.join(body)
    effective = list(contexts)
    for region in executable_regions:
        for index in range(region['start'],region['end']):
            effective[index] = tuple(entry for entry in contexts[index]
                if entry[2] >= region['start'])
    starts = [0] + [m.end() for m in re.finditer('\n',source)]
    def line_of(index):
        return bisect_right(starts,index)
    physical = [i for i,row in enumerate(cleaned.splitlines(keepends=True),1) if row.strip()]
    body_rows = body.splitlines(keepends=True)
    statements = {i:[] for i in physical}
    by_line = {}
    owned = set()
    boundaries = {offset for event in events if event.get('scope') != 'value-expression'
        for offset in (event['start'],event['end'])}
    for event in events:
        by_line.setdefault(line_of(event['start']),[]).append(event)
        owned.update(range(line_of(event['start']),line_of(event['end']-1)+1))
    for line_number in physical:
        offset = starts[line_number-1]
        row = body_rows[line_number-1]
        separators = [i for i,c in enumerate(row) if c == ';' and not structural(effective[offset+i])]
        bounds = sorted({0,len(row),*(i+1 for i in separators),
            *(i-offset for i in boundaries if offset < i < offset+len(row))})
        for left,right in zip(bounds,bounds[1:]):
            text = row[left:right].rstrip().removesuffix(';').rstrip()
            if not text.strip('{}; \t\r\n'):
                continue
            assignments = [dict(column=left+m.start()+1,operator=m[0])
                for m in assignment_pattern.finditer(text)
                if not structural(effective[offset+left+m.start()], assignment=True,
                                  named_equals=named_equals)]
            index = offset+left+len(text)-len(text.lstrip())
            decision = binding_counter(text=text, original=source[offset+left:offset+left+len(text)],
                context=effective[index] if index < len(effective) else (), assignments=assignments) if binding_counter else {}
            decision = decision or {}
            units = max(1,len(assignments),decision.get('units',0))
            diagnostics.extend(dict(item,offset=item.get('offset',offset+left))
                for item in decision.get('diagnostics',[]))
            statements[line_number].append(dict(start_column=left+1,end_column=right+1,
                units=units,assignments=assignments,
                declaration_bindings=decision.get('declaration_bindings',0),
                reason=decision.get('reason','independent-statement-and-chained-bindings')))
    expansions = []
    total = 0
    original_rows = source.splitlines()
    for number in physical:
        headers = by_line.get(number,[])
        units = sum(e['units'] for e in headers) + sum(e['units'] for e in statements[number])
        if not units and number not in owned:
            units = 1
        total += units
        if units != 1 or headers or any(e['units'] > 1 for e in statements[number]):
            expansions.append(dict(line=number,original=original_rows[number-1],
                units=units,extra_lines=units-1,canonical_events=headers,events=statements[number],
                reason='Approved owner/header/body/close units; preserve literal/data wrapping.'))
    for issue in diagnostics:
        if 'offset' in issue:
            line = line_of(issue['offset'])
            issue['line'] = line
            issue['column'] = issue['offset']-starts[line-1]+1
    return dict(rule_version=RULE_VERSION,language=language,physical_loc=len(physical),
        normalized_loc=total,expansions=expansions,canonical_events=events,
        diagnostics=diagnostics,review_status='needs-review' if diagnostics else 'candidate-parser-pass',
        admission='Unhooked source-count proposal; caller must verify exact source/producer pins and review all diagnostics.')
