"""Unhooked, conservative composed-value recognition for LOC policy review.

Input is already lexically masked and offset preserving. This proposal changes
no source or counts. Its spans describe operands that a caller may make opaque
before canonical boundary splitting; full covered rows must be owned to avoid
physical-line fallback. A span in a binding/return/call operand adds zero units
because its enclosing expression already counts. A proven function tail adds
one expression unit. Imperative and uncertain forms never become opaque.
"""

import re


CODE = {'code', 'bare-code'}
MAX_SPAN = 20000
MAX_DEPTH = 16
WORD = re.compile(r'\b[A-Za-z_]\w*\b')
ASSIGN = re.compile(r'(?<![=!<>~])(?:\*\*=|<<=|>>=|[+*/%&|^-]=|=(?!=|>|~))')
R_ASSIGN = re.compile(r'<<-|->>|<-|->|(?<![=!<>~])=(?!=|>|~)')
JULIA_OPEN = {'if', 'function', 'for', 'while', 'let', 'begin', 'try',
              'quote', 'struct', 'module', 'baremodule', 'macro', 'do'}
TRANSFERS = {'return', 'break', 'continue', 'next', 'yield', 'goto'}
DECLARATIONS = {'let', 'var', 'const', 'local', 'global', 'static'}
CONTROLS = {'for', 'while', 'loop', 'repeat', 'try', 'catch', 'finally',
            'begin', 'quote', 'unsafe', 'async'}


def _trim(text, start, end):
    while start < end and text[start].isspace():
        start += 1
    while end > start and text[end - 1].isspace():
        end -= 1
    return start, end


def _matching(text, opening, limit=None):
    limit = min(len(text), opening + MAX_SPAN) if limit is None else min(limit, opening + MAX_SPAN)
    pairs = {'(': ')', '[': ']', '{': '}'}
    if opening >= limit or text[opening] not in pairs:
        return None
    stack = [pairs[text[opening]]]
    for index in range(opening + 1, limit):
        char = text[index]
        if char in pairs:
            stack.append(pairs[char])
        elif char in ')]}':
            if not stack or stack.pop() != char:
                return None
            if not stack:
                return index + 1
    return None


def _relevant(context):
    last = max((i for i, (_, kind, _) in enumerate(context) if kind in CODE), default=-1)
    return context[last + 1:]


def _structural(context):
    return any(kind in {'data', 'args', 'parameters', 'for', 'control'}
               for _, kind, _ in _relevant(context))


def _own_kind(contexts, index):
    if index + 1 >= len(contexts):
        return None
    own = next((entry for entry in reversed(contexts[index + 1]) if entry[2] == index), None)
    return own[1] if own else None


def _node(state, start, end, reason, **extra):
    return dict(state=state, start=start, end=end, reason=reason, **extra)


def classify_value_forms(cleaned, language, contexts):
    """Return proposed opaque spans, diagnostics and source-local evidence.

    Caller contract: remove all canonical events/boundaries inside each span;
    own every listed row, even if masking leaves it empty; count the enclosing
    binding/return/call once for units=0, or add one value unit for units=1. Do
    not hook these proposals into chart admission without fixture qualification
    and a source-keyed policy decision. Offsets are half-open character spans.
    """
    language = {'rs': 'rust', 'jl': 'julia', 'R': 'r'}.get(language, language)
    if len(contexts) != len(cleaned):
        raise ValueError('Value-form contexts must preserve one entry per source character.')
    if language not in {'rust', 'julia', 'r'}:
        return dict(spans=[], diagnostics=[], evidence=[], status='unsupported-language')
    spans, diagnostics, evidence = [], [], []

    def review(node):
        diagnostics.append(dict(kind='value-form-review', offset=node['start'],
            end=node['end'], reason=node['reason'],
            message='Composed-value form requires source-specific review before becoming opaque.'))

    def brace_after(start, limit):
        base = contexts[start] if start < len(contexts) else ()
        for index in range(start, min(limit, start + MAX_SPAN)):
            if cleaned[index] == '{' and contexts[index] == base and _own_kind(contexts, index) in CODE:
                return index
            if cleaned[index] == ';' and not _structural(contexts[index]):
                return None
        return None

    def scalar(start, end, depth=0):
        start, end = _trim(cleaned, start, end)
        if language in {'julia', 'r'} and start < end and cleaned[end - 1] == ';':
            start, end = _trim(cleaned, start, end - 1)
        value = cleaned[start:end]
        if not value:
            return _node('review', start, end, 'empty-result-expression')
        if depth > MAX_DEPTH or end - start > MAX_SPAN:
            return _node('review', start, end, 'bounded-parser-limit')
        first = WORD.match(cleaned, start)
        if first and first[0] in {'if', 'match'}:
            nested = parse(start, language, depth + 1)
            if nested['end'] == end:
                return nested
            return _node('review', start, end, 'mixed-nested-value-form')
        if re.search(r'\b[A-Za-z_]\w*(?:::\w+)*\s*!\s*[({\[]|@\w+', value):
            return _node('review', start, end, 'macro-expression')
        if re.search(r'=>|(?:^|[=(,:?])\s*(?:move\s*)?\|', value):
            return _node('review', start, end, 'nested-function-or-callback')
        words = {match[0] for match in WORD.finditer(value)}
        if words & {'if', 'else', 'elseif', 'end', 'match'}:
            return _node('review', start, end, 'nested-control-expression')
        if words & (TRANSFERS | DECLARATIONS | CONTROLS | {'fn', 'function', 'do'}):
            return _node('imperative', start, end, 'independent-statement-or-control')
        assignment = R_ASSIGN if language == 'r' else ASSIGN
        for match in assignment.finditer(value):
            index = start + match.start()
            relevant = _relevant(contexts[index])
            # R/Julia keyword arguments are structural only in the immediate
            # call-argument scope. An assignment inside a nested grouping is
            # not automatically a named argument and cannot be hidden.
            named_argument = False
            if language in {'r', 'julia'} and match[0] == '=' and relevant and relevant[-1][1] == 'args':
                argument_start = relevant[-1][2] + 1
                for separator in range(argument_start, index):
                    if cleaned[separator] == ',' and contexts[separator] == contexts[index]:
                        argument_start = separator + 1
                named_argument = bool(re.fullmatch(r'\s*[A-Za-z_.]\w*\s*', cleaned[argument_start:index]))
            if not named_argument:
                return _node('imperative', start, end, 'independent-binding')
        for index in range(start, end):
            if cleaned[index] == ';' and not _structural(contexts[index]):
                return _node('imperative', start, end, 'statement-sequence')
            if cleaned[index] == '{' and _own_kind(contexts, index) in CODE:
                return _node('review', start, end, 'nested-executable-block')
        if re.search(r'\+\+|--', value):
            return _node('review', start, end, 'uncertain-mutation-or-operator')
        if '\n' in value:
            if re.search(r'[\[\]{()]', value) or re.fullmatch(r'[S\s]+', value):
                return _node('review', start, end, 'mixed-literal-or-call-wrapping-rows')
            rows = [row.strip() for row in value.splitlines() if row.strip()]
            if any(not re.search(r'(?:[+*/%^&|?:,.=-]|&&|\|\||\band|\bor)\s*$', row)
                   for row in rows[:-1]):
                return _node('imperative', start, end, 'multiple-expression-rows')
        for index in range(start, end):
            if cleaned[index] != '(':
                continue
            prefix = cleaned[start:index].rstrip()
            if prefix.endswith((')', ']', '}')):
                return _node('review', start, end, 'complex-call-target')
        if not re.fullmatch(r"[A-Za-z_0-9\s.:'$+*/%^&|<>=!?~-]*[A-Za-z_0-9\s.:'$+*/%^&|<>=!?~(),\[\]{}-]*", value):
            return _node('review', start, end, 'unsupported-expression-punctuation')
        return _node('pure', start, end, 'one-result-expression')

    def rust_if(start, depth):
        opening = brace_after(start + 2, len(cleaned))
        if opening is None:
            return _node('review', start, start + 2, 'unrecognized-if-body')
        closing = _matching(cleaned, opening)
        if closing is None:
            return _node('review', start, opening + 1, 'unbalanced-if-body')
        condition = scalar(start + 2, opening, depth + 1)
        consequent = scalar(opening + 1, closing - 1, depth + 1)
        cursor, _ = _trim(cleaned, closing, len(cleaned))
        alternate = re.match(r'else\b', cleaned[cursor:])
        if not alternate:
            return _node('imperative', start, closing, 'conditional-without-complete-else')
        body_start, _ = _trim(cleaned, cursor + alternate.end(), len(cleaned))
        if re.match(r'if\b', cleaned[body_start:]):
            other = parse(body_start, 'rust', depth + 1)
            end = other['end']
        elif body_start < len(cleaned) and cleaned[body_start] == '{':
            other_end = _matching(cleaned, body_start)
            if other_end is None:
                return _node('review', start, body_start + 1, 'unbalanced-else-body')
            other = scalar(body_start + 1, other_end - 1, depth + 1)
            end = other_end
        else:
            return _node('review', start, body_start, 'unsupported-else-form')
        children = [condition, consequent, other]
        state = 'imperative' if any(n['state'] == 'imperative' for n in children) else \
                'review' if any(n['state'] == 'review' for n in children) else 'pure'
        reason = next((n['reason'] for n in children if n['state'] == state), 'complete-value-conditional')
        return _node(state, start, end, reason, form='if', branches=2)

    def rust_match(start, depth):
        opening = brace_after(start + 5, len(cleaned))
        closing = _matching(cleaned, opening) if opening is not None else None
        if closing is None:
            return _node('review', start, start + 5, 'unrecognized-match-body')
        selector = scalar(start + 5, opening, depth + 1)
        arms, cursor, stack, arrow = [], opening + 1, [], None
        pairs = {'(': ')', '[': ']', '{': '}'}
        index = cursor
        while index < closing - 1:
            char = cleaned[index]
            if char in pairs:
                stack.append(pairs[char])
            elif char in ')]}':
                if not stack or stack.pop() != char:
                    return _node('review', start, closing, 'unbalanced-match-arm')
            elif not stack and cleaned.startswith('=>', index):
                if arrow is not None:
                    return _node('review', start, closing, 'multiple-arm-arrows')
                arrow = index
                index += 1
            elif char == ',' and not stack:
                arms.append((cursor, arrow, index))
                cursor, arrow = index + 1, None
            index += 1
        if cleaned[cursor:closing - 1].strip():
            arms.append((cursor, arrow, closing - 1))
        if not arms:
            return _node('review', start, closing, 'empty-match')
        children = [selector]
        for arm_start, arm_arrow, arm_end in arms:
            if arm_arrow is None:
                return _node('review', start, closing, 'missing-match-arm-arrow')
            pattern = cleaned[arm_start:arm_arrow]
            if re.search(r'\bif\b|\b[A-Za-z_]\w*\s*!|@', pattern):
                return _node('review', start, closing, 'match-guard-or-complex-pattern')
            if '\n' in pattern.strip() and re.search(r'[\[{}]', pattern):
                return _node('review', start, closing, 'multiline-pattern-data-rows')
            result_start, result_end = _trim(cleaned, arm_arrow + 2, arm_end)
            if result_start < result_end and cleaned[result_start] == '{':
                block_end = _matching(cleaned, result_start, result_end)
                if block_end != result_end:
                    return _node('review', start, closing, 'mixed-match-arm-block')
                result_start += 1
                result_end -= 1
            children.append(scalar(result_start, result_end, depth + 1))
        state = 'imperative' if any(n['state'] == 'imperative' for n in children) else \
                'review' if any(n['state'] == 'review' for n in children) else 'pure'
        reason = next((n['reason'] for n in children if n['state'] == state), 'value-match-with-structural-patterns')
        return _node(state, start, closing, reason, form='match', branches=len(arms))

    def julia_header_end(start):
        base = contexts[start]
        for index in range(start, min(len(cleaned), start + MAX_SPAN)):
            if cleaned[index] in ';\n' and contexts[index] == base:
                return index
        return None

    def julia_if(start, depth):
        base_context = contexts[start]
        header_end = julia_header_end(start)
        if header_end is None:
            return _node('review', start, start + 2, 'unrecognized-keyword-header')
        children = [scalar(start + 2, header_end, depth + 1)]
        branch_start, stack, has_else = header_end + 1, ['if'], False
        for match in WORD.finditer(cleaned, header_end + 1, min(len(cleaned), start + MAX_SPAN)):
            word, index = match[0], match.start()
            if ((index and cleaned[index - 1] in '.:') or
                    (_structural(contexts[index]) and contexts[index] != base_context)):
                continue
            if word in JULIA_OPEN:
                stack.append(word)
            elif word == 'end':
                if len(stack) > 1:
                    stack.pop()
                else:
                    children.append(scalar(branch_start, index, depth + 1))
                    if not has_else:
                        return _node('imperative', start, match.end(), 'conditional-without-complete-else')
                    state = 'imperative' if any(n['state'] == 'imperative' for n in children) else \
                            'review' if any(n['state'] == 'review' for n in children) else 'pure'
                    reason = next((n['reason'] for n in children if n['state'] == state), 'complete-value-conditional')
                    return _node(state, start, match.end(), reason, form='if', branches=len(children) // 2 + 1)
            elif word in {'else', 'elseif'} and len(stack) == 1:
                children.append(scalar(branch_start, index, depth + 1))
                if word == 'else':
                    has_else, branch_start = True, match.end()
                    delimiter, _ = _trim(cleaned, branch_start, len(cleaned))
                    if cleaned[delimiter:delimiter + 1] == ';':
                        branch_start = delimiter + 1
                else:
                    following = julia_header_end(index)
                    if following is None:
                        return _node('review', start, match.end(), 'unrecognized-elseif-header')
                    children.append(scalar(match.end(), following, depth + 1))
                    branch_start = following + 1
        return _node('review', start, header_end, 'unclosed-keyword-conditional')

    def r_expression_end(start):
        stack = []
        pairs = {'(': ')', '[': ']', '{': '}'}
        for index in range(start, min(len(cleaned), start + MAX_SPAN)):
            char = cleaned[index]
            if not stack and char in ',;\n)]}':
                return index
            if char in pairs:
                stack.append(pairs[char])
            elif char in ')]}':
                if not stack or stack.pop() != char:
                    return index
        return min(len(cleaned), start + MAX_SPAN)

    def r_if(start, depth):
        condition_start, _ = _trim(cleaned, start + 2, len(cleaned))
        condition_end = _matching(cleaned, condition_start) if cleaned[condition_start:condition_start + 1] == '(' else None
        if condition_end is None:
            return _node('review', start, start + 2, 'unrecognized-r-condition')
        consequent_start, _ = _trim(cleaned, condition_end, len(cleaned))
        if cleaned[consequent_start:consequent_start + 1] == '{':
            consequent_end = _matching(cleaned, consequent_start)
            if consequent_end is None:
                return _node('review', start, consequent_start + 1, 'unbalanced-r-branch')
            consequent = scalar(consequent_start + 1, consequent_end - 1, depth + 1)
            else_start, _ = _trim(cleaned, consequent_end, len(cleaned))
        else:
            if re.match(r'if\b', cleaned[consequent_start:]):
                return _node('review', start, r_expression_end(consequent_start), 'nested-unbraced-r-conditional')
            stack, else_start = [], None
            for match in re.finditer(r'\belse\b|[()\[\]{};\n]', cleaned[consequent_start:]):
                token, index = match[0], consequent_start + match.start()
                if token in '([{':
                    stack.append(token)
                elif token in ')]}':
                    if stack:
                        stack.pop()
                    else:
                        break
                elif not stack and token == 'else':
                    else_start = index
                    break
                elif not stack and token in ';\n':
                    break
            if else_start is None:
                return _node('imperative', start, r_expression_end(consequent_start), 'conditional-without-complete-else')
            consequent_end = else_start
            consequent = scalar(consequent_start, else_start, depth + 1)
        alternate = re.match(r'else\b', cleaned[else_start:])
        if not alternate:
            return _node('imperative', start, consequent_end, 'conditional-without-complete-else')
        other_start, _ = _trim(cleaned, else_start + alternate.end(), len(cleaned))
        if cleaned[other_start:other_start + 1] == '{':
            end = _matching(cleaned, other_start)
            if end is None:
                return _node('review', start, other_start + 1, 'unbalanced-r-branch')
            other = scalar(other_start + 1, end - 1, depth + 1)
        else:
            end = r_expression_end(other_start)
            other = scalar(other_start, end, depth + 1)
        children = [scalar(condition_start + 1, condition_end - 1, depth + 1), consequent, other]
        state = 'imperative' if any(n['state'] == 'imperative' for n in children) else \
                'review' if any(n['state'] == 'review' for n in children) else 'pure'
        reason = next((n['reason'] for n in children if n['state'] == state), 'complete-value-conditional')
        return _node(state, start, end, reason, form='if', branches=2)

    def parse(start, lang, depth=0):
        if depth > MAX_DEPTH:
            return _node('review', start, start + 2, 'bounded-parser-limit')
        if lang == 'rust':
            return rust_match(start, depth) if cleaned.startswith('match', start) else rust_if(start, depth)
        return julia_if(start, depth) if lang == 'julia' else r_if(start, depth)

    def keyword_function_tail(start, end):
        stack = []
        for match in WORD.finditer(cleaned, 0, start):
            index, word = match.start(), match[0]
            if (index and cleaned[index - 1] in '.:') or _structural(contexts[index]):
                continue
            if word in JULIA_OPEN:
                stack.append((word, index))
            elif word == 'end' and stack:
                stack.pop()
        if not stack or stack[-1][0] != 'function':
            return False
        depth = 1
        for match in WORD.finditer(cleaned, end, min(len(cleaned), end + MAX_SPAN)):
            index, word = match.start(), match[0]
            if (index and cleaned[index - 1] in '.:') or _structural(contexts[index]):
                continue
            if word in JULIA_OPEN:
                depth += 1
            elif word == 'end':
                depth -= 1
                if not depth:
                    return not cleaned[end:index].strip('; \t\r\n')
        return False

    def position(start, end):
        prefix_start = max(0, start - 512)
        prefix = cleaned[prefix_start:start]
        assignment = R_ASSIGN if language == 'r' else ASSIGN
        matches = list(assignment.finditer(prefix))
        if matches and re.fullmatch(r'\s*\(*\s*', prefix[matches[-1].end():]):
            return 'binding-operand', 0
        if re.search(r'\breturn\s*\(*\s*$', prefix):
            return 'return-operand', 0
        relevant = _relevant(contexts[start])
        if relevant and relevant[-1][1] == 'args':
            argument_start = relevant[-1][2] + 1
            if re.fullmatch(r'\s*', cleaned[argument_start:start]) or re.search(r',\s*$', prefix):
                return 'argument-operand', 0
        if language == 'r':
            functions = list(re.finditer(r'\bfunction\s*\(', prefix))
            for function in reversed(functions):
                opening = prefix_start + function.end() - 1
                closing = _matching(cleaned, opening, start)
                if closing is not None and not cleaned[closing:start].strip():
                    return 'expression-function-body', 0
        if language in {'rust', 'r'}:
            own = next((entry for entry in reversed(contexts[start]) if entry[0] == '{' and entry[1] in CODE), None)
            if own:
                owner_prefix = cleaned[max(0, own[2] - 512):own[2]]
                function = (re.search(r'\bfn\s+[A-Za-z_]\w*[^;{}]*$', owner_prefix) if language == 'rust'
                            else re.search(r'\bfunction\s*\([^{};]*\)\s*$', owner_prefix))
                owner_end = _matching(cleaned, own[2])
                tail = cleaned[end:owner_end - 1] if owner_end else ''
                empty_tail = not tail.strip('; \t\r\n') if language == 'r' else not tail.strip()
                if function and owner_end and empty_tail:
                    return 'function-tail', 1
        if language == 'julia' and keyword_function_tail(start, end):
            return 'function-tail', 1
        return None

    for candidate in re.finditer(r'\b(?:if|match)\b' if language == 'rust' else r'\bif\b', cleaned):
        start = candidate.start()
        if (start and cleaned[start - 1] in '.:') or any(span['start'] <= start < span['end'] for span in spans):
            continue
        node = parse(start, language)
        evidence.append(dict(start=node['start'], end=node['end'], state=node['state'], reason=node['reason']))
        if node['state'] == 'review':
            review(node)
            continue
        if node['state'] != 'pure':
            continue
        location = position(start, node['end'])
        if location is None:
            review(_node('review', start, node['end'], 'unproven-value-position'))
            continue
        name, units = location
        spans.append(dict(start=start, end=node['end'], units=units, position=name,
            form=node.get('form', 'conditional'), branches=node.get('branches'),
            owned_rows=list(range(cleaned.count('\n', 0, start) + 1,
                                  cleaned.count('\n', 0, node['end'] - 1) + 2)),
            exclude_internal_boundaries=True,
            evidence='Complete value branches with one result expression each; no independent statements.'))
    return dict(spans=spans, diagnostics=diagnostics, evidence=evidence, status='unhooked-proposal')
