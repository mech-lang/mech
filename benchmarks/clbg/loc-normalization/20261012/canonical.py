"""Canonical control boundaries for the source-keyed LOC review.

One explicit block/control/function header counts once, its executable opening
brace counts zero additional lines, and each explicit executable closing brace
or keyword counts once. Body statements are counted by the caller. An opening
brace on its own physical line is therefore equivalent to one attached to its
header. Data/argument delimiters and expression-only functions are untouched.

This is a conservative boundary recognizer, not a language parser. Unsupported
or ambiguous forms produce diagnostics and require source-specific review.
All spans use half-open Python character offsets in the supplied cleaned text.
"""

import re


BRACED = {"c", "cpp", "rust", "go", "java", "chapel", "javascript", "php", "perl", "ruby", "r"}
KEYWORD = {"lua", "ruby", "julia"}
CODE_KINDS = {"code", "bare-code"}
TOKEN = re.compile(r"\b[A-Za-z_]\w*\b|=>|[{}():;]")


def _line_start(text, index):
    return text.rfind("\n", 0, index) + 1


def _trim_start(text, start, end):
    while start < end and text[start].isspace():
        start += 1
    return start


def _structural(context):
    last_code = max((i for i, (_, kind, _) in enumerate(context) if kind in CODE_KINDS), default=-1)
    return any(kind in {"data", "args", "parameters", "for", "control"}
               for _, kind, _ in context[last_code + 1:])


def _julia_macro_prefix(prefix):
    """Recognize leading loop macros and a literal symbol option.

    Pygments masks a Julia symbol such as ``:static`` as literal ``S`` text.
    Both its raw and masked spellings belong to the same macro header; they
    must not become a separate body statement ahead of the loop keyword.
    """
    return bool(re.fullmatch(r"(?:[\w.]*@\w+(?:\s+(?::\w+|S+))?\s*)+", prefix))


def _at_statement_start(text, index, language):
    prefix = text[_line_start(text, index):index].strip()
    if not prefix or prefix.endswith((";", "{", "}")):
        return True
    if language == "python" and prefix == "async":
        return True
    if language == "julia" and _julia_macro_prefix(prefix):
        return True
    if language == "julia" and prefix in {"mutable", "abstract", "primitive"}:
        return True
    if language in KEYWORD | {'chapel'} and re.search(r"\b(?:then|do|else|begin|repeat)\s*$", prefix):
        return True
    return False


def _paren_end(text, opening):
    depth = 0
    for index in range(opening, len(text)):
        if text[index] == "(":
            depth += 1
        elif text[index] == ")":
            depth -= 1
            if not depth:
                return index + 1
    return None


def _paren_start(text, closing):
    depth = 0
    for index in range(closing, -1, -1):
        if text[index] == ")":
            depth += 1
        elif text[index] == "(":
            depth -= 1
            if not depth:
                return index
    return None


def _modifier_end(text, match, contexts):
    """End a simple Ruby/Perl statement modifier at its written row boundary.

    Function/control blocks embedded in its condition and continued conditions
    remain review cases. We do not reinterpret a leading statement control.
    """
    base = contexts[match.start()]
    end = next((i for i in range(match.end(), len(text))
        if text[i] in ';\n' and contexts[i] == base), len(text))
    condition = text[match.end():end].strip()
    if not condition or condition.endswith(('\\', '&&', '||', '+', '-', '*', '/', '=', ',')):
        return None
    if re.search(r'\b(?:if|unless|while|until|for|foreach|do|begin|end|sub|function|def)\b', condition):
        return None
    return end


def _rust_signature_balanced(value):
    """Accept bounded type/signature punctuation without macro or block syntax."""
    if not re.fullmatch(r"[A-Za-z_0-9\s:&'<>.,()\[\]+=?*-]*", value):
        return False
    stack = []
    for index, char in enumerate(value):
        if char in '([<':
            stack.append(char)
        elif char in ')]>':
            if char == '>' and index and value[index - 1] == '-':
                continue  # A function-pointer return arrow is not a generic close.
            wanted = {')': '(', ']': '[', '>': '<'}[char]
            if not stack or stack.pop() != wanted:
                return False
    return not stack


def _rust_function_start(text, opening):
    """Find a proven named Rust function signature ending at this body brace.

    Generics, parameters, return types and where clauses may wrap across rows.
    Braces, macros, attributes and semicolons inside the signature are excluded;
    unsupported forms retain the caller's conservative fallback/review path.
    """
    window_start = max(0, opening - 1024)
    prefix = text[window_start:opening]
    candidates = list(re.finditer(r'\bfn\s+[A-Za-z_]\w*', prefix))
    for candidate in reversed(candidates):
        start = window_start + candidate.start()
        cursor = window_start + candidate.end()
        while cursor < opening and text[cursor].isspace():
            cursor += 1
        if cursor < opening and text[cursor] == '<':
            generic_start, depth = cursor, 1
            cursor += 1
            while cursor < opening and depth:
                if text[cursor] == '<':
                    depth += 1
                elif text[cursor] == '>' and text[cursor - 1] != '-':
                    depth -= 1
                cursor += 1
            if depth or not _rust_signature_balanced(text[generic_start:cursor]):
                continue
            while cursor < opening and text[cursor].isspace():
                cursor += 1
        if cursor >= opening or text[cursor] != '(':
            continue
        parameter_end = _paren_end(text, cursor)
        if parameter_end is None or parameter_end > opening:
            continue
        if not _rust_signature_balanced(text[cursor:parameter_end]):
            continue
        tail = text[parameter_end:opening].strip()
        if tail:
            if tail.startswith('->'):
                return_and_where = tail[2:].strip()
                clauses = re.split(r'\bwhere\b', return_and_where, maxsplit=1)
                result = clauses[0].strip()
                if not result or (result != '!' and not _rust_signature_balanced(result)):
                    continue
                if len(clauses) > 1 and (not clauses[1].strip() or
                                        not _rust_signature_balanced(clauses[1])):
                    continue
            elif re.match(r'where\b', tail):
                constraints = tail[5:].strip()
                if not constraints or not _rust_signature_balanced(constraints):
                    continue
            else:
                continue
            if re.search(r'\b(?:let|if|else|while|loop|match|return|break|continue|async|move)\b', tail):
                continue
        header_start = _line_start(text, start)
        # Written modifiers on their own preceding rows belong to this same
        # declaration. Require an exact modifier row rather than absorbing code.
        while header_start > window_start:
            previous_start = _line_start(text, header_start - 1)
            previous = text[previous_start:header_start].strip()
            if not re.fullmatch(r'(?:pub(?:\([^()\n]*\))?|async|const|unsafe|extern(?:\s+S+)?)', previous):
                break
            header_start = previous_start
        return header_start
    return None


def _structural_callback_suffix(text, closing, contexts):
    """Own only proved enclosing argument/data punctuation after a block.

    A real following argument, call or expression is not deleted. Whitespace
    may wrap the same structural suffix across source rows; literal-only data
    wrapping elsewhere is untouched. Returns a half-open span or None.
    """
    cursor=closing+1;end=cursor;proved=False
    while cursor<len(text):
        if text[cursor].isspace():cursor+=1;continue
        char=text[cursor];context=contexts[cursor]
        own=context[-1] if context else None
        if char in ')]' and own and own[0]=={')':'(',']':'['}[char] and own[1] in {'args','data'}:
            proved=True;cursor+=1;end=cursor;continue
        if char==',' and own and own[1] in {'args','data'}:
            proved=True;cursor+=1;end=cursor;break
        if char==';' and proved:
            cursor+=1;end=cursor
        break
    return (closing+1,end) if proved else None


def _header_start(text, opening, contexts, existing, language=None):
    """Find the immediate header without absorbing earlier packed statements."""
    end = opening
    while end and text[end - 1].isspace():
        end -= 1
    start = _line_start(text, max(0, end - 1))
    preserve_start = False
    if end:
        # Synthetic unparenthesized loop/control frames begin at their written
        # keyword rather than a real `(`. Their entire multiline signature is
        # one header, including nested iterator-expression argument wrapping.
        synthetic = [offset for delimiter, kind, offset in contexts[end-1]
            if delimiter == '(' and kind in {'for','control'} and
            text[offset:offset+1] != '(' and
            re.match(r'(?:for|if|switch|while|match)\b',text[offset:])]
        if synthetic:
            start = _line_start(text,min(synthetic))
            preserve_start = True
        prefix_start = max(0,end-400)
        if language == 'rust':
            iteration = re.search(r'\bfor\s+[A-Za-z_]\w*\s+in\s+[^;{}]+$',text[prefix_start:end])
            if iteration:
                start = _line_start(text,prefix_start+iteration.start())
                preserve_start = True
            elif not preserve_start:
                function_start = _rust_function_start(text, opening)
                if function_start is not None:
                    start = function_start
                    preserve_start = True
        if language == 'java':
            declaration = re.search(r'\b(?:class|interface|enum)\s+[A-Za-z_$][\w$]*'
                r'(?:\s*<[^{};]+>)?(?:\s+(?:extends|implements)\s+[\w.$,< >?]+)*\s*$',
                text[prefix_start:end])
            if declaration:
                start = _line_start(text,prefix_start+declaration.start())
                preserve_start = True
        suffix = re.search(r'\)\s+throws\s+[A-Za-z_$][\w.$,\s]*$',text[max(0,end-400):end])
        if suffix:
            closing = max(0,end-400)+suffix.start()
            parameter_opening = _paren_start(text,closing)
            if parameter_opening is not None:
                name = re.search(r'([A-Za-z_$]\w*)\s*$',text[:parameter_opening])
                if name:
                    start = _line_start(text,name.start())
    if end and text[end - 1] == ")":
        paren = _paren_start(text, end - 1)
        if paren is not None:
            before = re.search(r"([A-Za-z_]\w*)\s*$", text[:paren])
            if before:
                candidate_start = before.start(1) if before[1] in {"if", "for", "while", "switch", "catch", "foreach", "unless", "until"} else _line_start(text, before.start(1))
                start = min(start,candidate_start) if preserve_start else candidate_start
                if before[1] == "if":
                    branch = re.search(r"\belse\s*$", text[:start])
                    if branch:
                        start = branch.start()
    for index in range(start, opening):
        structural = _structural(contexts[index])
        if text[index] == '{' and index+1 < len(contexts):
            own = next((entry for entry in reversed(contexts[index+1])
                        if entry[0] == '{' and entry[2] == index),None)
            structural = structural or bool(own and own[1] == 'data')
        if text[index] in ";{}" and not structural:
            start = index + 1
    for event in existing:
        if start <= event["end"] <= opening and event["kind"] in {"close", "header", "branch", "pragma", "parameters"}:
            start = event["end"]
    return _trim_start(text, start, opening)


def canonical_units(cleaned, language, contexts):
    """Return owned header/closure spans and boundaries for caller accounting.

    ``events`` spans must be removed from ordinary statement counting and added
    once using their ``units``. ``boundaries`` split any adjacent body text.
    ``zero_unit_spans`` is the subset of opening delimiters; do not retain a
    fallback physical line for a line containing only such a delimiter.
    ``executable_regions`` describe recognized keyword block bodies. A caller
    may use them to reset outer argument/data contexts, or diagnose such nested
    callbacks rather than silently suppressing their executable statements.
    """
    language = {"luajit": "lua", "c++": "cpp", "node": "javascript", "python3": "python"}.get(language, language)
    if len(contexts) != len(cleaned):
        raise ValueError("Canonical contexts must preserve one entry per source character.")
    events, diagnostics, regions = [], [], []

    def diagnostic(kind, index, message):
        diagnostics.append(dict(kind=kind, offset=index, message=message))

    def emit(kind, start, end, units=1, scope=None):
        if end <= start:
            return
        events.append(dict(kind=kind, start=start, end=end, units=units, scope=scope))

    def covered(index):
        return any(event["start"] <= index < event["end"] for event in events)

    # Parent's structural map distinguishes executable blocks from literal
    # braces. A context before a closing brace contains its exact opening span.
    if language in BRACED:
        openings = {}
        callbacks = set()
        if language == 'rust':
            # A compiler attribute is a meaningful unit even when its function
            # declaration follows on the same physical line. Nested attribute
            # arguments are structural; the lexer already masked their strings.
            for attribute in re.finditer(r'#!?\[', cleaned):
                depth = 1
                end = attribute.end()
                while end < len(cleaned) and depth:
                    if cleaned[end] == '[':
                        depth += 1
                    elif cleaned[end] == ']':
                        depth -= 1
                    end += 1
                if depth:
                    diagnostic('canonical-rust-attribute-review', attribute.start(),
                        'Rust compiler attribute has no recognized matching bracket.')
                else:
                    emit('pragma', attribute.start(), end, scope='rust-attribute')
        for index, char in enumerate(cleaned):
            if language in {'c', 'cpp'} and cleaned[_line_start(cleaned,index):index].lstrip().startswith('#'):
                continue
            if char == "{":
                inner = contexts[index + 1] if index + 1 < len(contexts) else ()
                own = next((entry for entry in reversed(inner) if entry[0] == "{" and entry[2] == index), None)
                if own and own[1] in CODE_KINDS:
                    start = index if own[1] == "bare-code" else _header_start(cleaned, index, contexts, events,language)
                    if language == 'chapel' and re.match(r'with\b', cleaned[start:index]):
                        # A multiline task-intent clause belongs to its loop's
                        # one header, not a separate `with` statement/header.
                        prior_line = cleaned[:start].rstrip().rsplit('\n', 1)[-1]
                        loop = re.match(r'\s*(?:for|forall|coforall)\b', prior_line)
                        if loop:
                            prior_end = len(cleaned[:start].rstrip())
                            prior_start = _line_start(cleaned, max(0, prior_end-1))
                            start = _trim_start(cleaned, prior_start, prior_end)
                    if language == 'ruby':
                        # `end.each { ... }` closes a preceding keyword block
                        # before applying a method/callback to its result. Do
                        # not hide that close inside the callback header span.
                        receiver_close = re.match(r'end(?=\s*\.)', cleaned[start:index])
                        if receiver_close:
                            start += receiver_close.end()
                    if start < index:
                        header = cleaned[start:index]
                        branch = bool(re.match(r"(?:else|catch|finally|case|default)\b", header))
                        emit("branch" if branch else "header", start, index, scope="braced")
                        lambda_form=(
                            (language=="javascript" and "=>" in header) or
                            (language=="java" and bool(re.search(r'(?:\([^(){}]*\)|[A-Za-z_]\w*)\s*->\s*$',header))) or
                            (language=="rust" and bool(re.search(r"\|[^|]*\|\s*(?:->[^{}]+)?$",header))) or
                            (language=="cpp" and bool(re.search(r"(?:^|[=(,:?])\s*\[[^]]*\]\s*(?:\([^)]*\))?\s*(?:mutable\s*)?(?:noexcept\s*)?(?:->[^{}]+)?$",header))) or
                            (language=="chapel" and bool(re.search(r"\b(?:lambda|proc)\s*\(",header))) or
                            (language=="php" and bool(re.search(r"\bfunction\s*&?\s*\(",header))))
                        last_code=max((i for i,(_,kind,_) in enumerate(contexts[index]) if kind in CODE_KINDS),default=-1)
                        known_callback = lambda_form or language == 'ruby' or (language == 'perl' and bool(re.search(r'\bsub\s*$', header)))
                        if known_callback:
                            callbacks.add(index)
                        if not known_callback and any(kind in {'args','parameters'} for _,kind,_ in contexts[index][last_code+1:]):
                            diagnostic('canonical-braced-callback-review',start,
                                'Executable callback is embedded in outer arguments; review its expression/block boundaries.')
                    emit("open", index, index + 1, units=0, scope="braced")
                    openings[index] = index + 1
                    if language == "ruby":
                        parameters = re.match(r"\s*\|[^|\n]*\|", cleaned[index + 1:])
                        if parameters:
                            emit("parameters", index + 1, index + 1 + parameters.end(), units=0, scope="braced")
                elif own and own[1] == "data":
                    prefix = cleaned[_line_start(cleaned, index):index].strip()
                    last_code=max((i for i,(_,kind,_) in enumerate(contexts[index]) if kind in CODE_KINDS),default=-1)
                    known_parameter_data = any(kind == 'parameters' for _, kind, _ in contexts[index][last_code+1:])
                    receiver = cleaned[max(0, index-180):index]
                    known_perl_data = language == 'perl' and bool(
                        re.search(r'->\s*$', receiver) or
                        (receiver and not receiver[-1].isspace() and
                         re.search(r'[$@%](?:[A-Za-z_]\w*(?:::[A-Za-z_]\w*)*|\d+)$', receiver)))
                    known_go_type_data = language == 'go' and bool(re.search(
                        r'\b(?:struct|interface)(?:\s*\{\s*\})?\s*\Z',receiver))
                    # Nested aggregate rows inside an immediate C-family
                    # data brace are data, even at the beginning of a row in
                    # a function. An executable lambda/block already takes
                    # the code branch above. A GNU ({...}) group is not an
                    # immediate data-brace parent and retains its review.
                    known_nested_c_data = (language in {'c','cpp'}
                        and contexts[index] and contexts[index][-1][0]=='{'
                        and contexts[index][-1][1]=='data')
                    if not known_parameter_data and not known_perl_data and not known_go_type_data and not known_nested_c_data and re.search(r"\b(?:if|for|while|switch|catch|fn|func|function|proc|sub|impl|trait|namespace|struct|class|enum|union)\b", prefix):
                        diagnostic("canonical-unrecognized-block-review", index,
                            "Possible executable brace was classified as data; review its grammar.")
                    elif not known_nested_c_data and not prefix and any(kind in CODE_KINDS for _, kind, _ in contexts[index]):
                        diagnostic("canonical-bare-block-review", index,
                            "Standalone nested brace could introduce an executable block.")
                    elif not known_nested_c_data and not prefix:
                        prior=cleaned[:index].rstrip().rsplit('\n',1)[-1].strip()
                        if not re.search(r'(?:=|:=|:|,|\[|\()\s*$',prior):
                            diagnostic('canonical-standalone-brace-context-review',index,
                                'Standalone brace lacks an unambiguous literal or executable header.')
            elif char == "}":
                own = contexts[index][-1] if contexts[index] else None
                if own and own[0] == "{" and own[1] in CODE_KINDS:
                    emit("close", index, index + 1, scope="braced")
                    suffix=_structural_callback_suffix(cleaned,index,contexts)
                    if suffix:
                        emit('structural-suffix',suffix[0],suffix[1],units=0,scope='callback-argument-data')
                    if own[2] in callbacks:
                        continuation=suffix[1] if suffix else index+1
                        if not suffix or cleaned[suffix[1]-1] != ';':
                            following=re.match(r'\s*([^\s])',cleaned[continuation:])
                            if following and (following[1] in '.(+-*/?' or suffix and cleaned[suffix[1]-1] == ',' and following[1] not in ')]'):
                                diagnostic('canonical-callback-composition-review',continuation+following.end()-1,
                                    'A real following argument/call/composed expression needs outer-expression ownership review; only structural callback suffix punctuation was suppressed.')
                    if own[2] in openings:
                        regions.append(dict(start=openings[own[2]], end=index, scope="braced"))

        # Unbraced C-family controls still have an independent header. The
        # matching ')' excludes all for-header/condition/argument separators.
        if language == 'perl':
            for modifier in re.finditer(r'\b(?:if|unless|while|until|for|foreach)\b', cleaned):
                start = modifier.start()
                if covered(start) or _structural(contexts[start]) or _at_statement_start(cleaned,start,language):
                    continue
                end = _modifier_end(cleaned, modifier, contexts)
                if end is None:
                    diagnostic('canonical-perl-modifier-review',start,
                        'Perl modifier condition has an unsupported nested/continued boundary.')
                else:
                    emit('header', start, end, scope='perl-modifier')
        if language in {"c", "cpp", "java", "javascript", "php", "perl", "r"}:
            words = r'if|unless|for|foreach|while|until|switch|catch|else|do|try|finally' if language == 'perl' else r'if|for|foreach|while|switch|catch|else|do|try|finally'
            for match in re.finditer(r"\b(?:"+words+r")\b", cleaned):
                start, word = match.start(), match[0]
                if cleaned[_line_start(cleaned,start):start].lstrip().startswith('#'):
                    continue
                if covered(start) or _structural(contexts[start]):
                    continue
                if language == 'perl' and word in {'if','unless','for','foreach','while','until'} and not _at_statement_start(cleaned,start,language):
                    # A complex modifier already received its own gate above;
                    # never truncate its condition at an incidental `(...)`.
                    continue
                if word in {"else", "do", "try", "finally"}:
                    if word == "else":
                        following_if = re.match(r"\s+if\s*\(", cleaned[match.end():])
                        if following_if:
                            end = _paren_end(cleaned, match.end() + following_if.end() - 1)
                            if end is not None:
                                emit("branch", start, end, scope="unbraced")
                                continue
                    emit("branch" if word != "do" else "header", start, match.end(), scope="unbraced")
                    continue
                following = re.match(r"\s*(?:constexpr\s*)?\(", cleaned[match.end():])
                if not following:
                    diagnostic("canonical-control-header-review", start,
                        "Control header has an unsupported condition/delimiter form.")
                    continue
                opening = match.end() + following.end() - 1
                end = _paren_end(cleaned, opening)
                if end is None:
                    diagnostic("canonical-control-header-review", start,
                        "Control header has an unclosed condition.")
                else:
                    emit("header", start, end, scope="unbraced")
            for match in re.finditer(r"\b(?:case|default)\b", cleaned):
                start = match.start()
                if covered(start) or _structural(contexts[start]) or not _at_statement_start(cleaned, start, language):
                    continue
                if match[0] == "default" and not re.match(r"\s*:", cleaned[match.end():]):
                    continue
                conditional = 0
                end = None
                for index in range(match.end(), len(cleaned)):
                    if contexts[index] != contexts[start]:
                        continue
                    char = cleaned[index]
                    if char in "\n;{}":
                        break
                    if char == "?":
                        conditional += 1
                    elif char == ":":
                        if conditional:
                            conditional -= 1
                        else:
                            end = index + 1
                            break
                if end is None:
                    diagnostic("canonical-case-label-review", start,
                        "Case/default label needs an explicit top-level colon review.")
                else:
                    emit("branch", start, end, scope="case")

    if language == "python":
        controls = {"if", "elif", "else", "for", "while", "with", "try", "except", "finally", "def", "class", "match", "case"}
        for match in TOKEN.finditer(cleaned):
            if match[0] not in controls or not _at_statement_start(cleaned, match.start(), language) or _structural(contexts[match.start()]):
                continue
            if match[0] in {'match','case'} and not re.match(r'\s+',cleaned[match.end():]):
                # Python's match/case are soft keywords. Ordinary bindings
                # named `match` or `case` do not introduce control headers.
                continue
            start = _trim_start(cleaned, _line_start(cleaned, match.start()), match.start())
            end = None
            for index in range(match.end(), len(cleaned)):
                if cleaned[index] == ":" and not _structural(contexts[index]):
                    end = index + 1
                    break
                if cleaned[index] == "\n" and not contexts[index]:
                    if cleaned[:index].rstrip().endswith('\\'):
                        continue
                    break
            if end is None:
                diagnostic("canonical-python-header-review", match.start(),
                    "Python statement header has no recognized top-level colon.")
            else:
                emit("branch" if match[0] in {"elif", "else", "except", "finally", "case"} else "header", start, end, scope="python")

    if language == "chapel":
        # `proc f(params) do statement` has a function header and body without
        # a written executable close. Default arguments remain parameters.
        for procedure in re.finditer(r'\bproc\s+[A-Za-z_]\w*\s*\(', cleaned):
            if covered(procedure.start()) or _structural(contexts[procedure.start()]):
                continue
            opening = procedure.end()-1
            end = _paren_end(cleaned, opening)
            if end is None:
                continue
            following = re.match(r'\s*(?::\s*[^;{}\n]+?)?\s*\bdo\b', cleaned[end:])
            if following:
                start = _trim_start(cleaned, _line_start(cleaned, procedure.start()), procedure.start())
                emit('header', start, end+following.end(), scope='chapel-proc-do')
        # Chapel also supports unbraced `if ... then ...` and `for ... do ...`.
        # Conditional expressions after an assignment are kept as expressions.
        for match in re.finditer(r"\b(?:if|for|forall|coforall|while|else)\b", cleaned):
            start, word = match.start(), match[0]
            if covered(start) or _structural(contexts[start]) or not _at_statement_start(cleaned, start, language):
                continue
            if word == "else":
                emit("branch", start, match.end(), scope="chapel-unbraced")
                continue
            if word == 'while' and re.search(r'}\s*$',cleaned[:start]):
                # Chapel do { ... } while condition; has a written trailing
                # condition header, with no invented `do` or extra close.
                end = next((i for i in range(match.end(),len(cleaned))
                    if cleaned[i] in ';\n' and contexts[i] == contexts[start]),len(cleaned))
                emit('header',start,end,scope='chapel-repeat-condition')
                continue
            terminal = "then" if word == "if" else "do"
            end = None
            for token in TOKEN.finditer(cleaned, match.end()):
                if contexts[token.start()] != contexts[start]:
                    continue
                if token[0] == terminal:
                    end = token.end()
                    break
                if token[0] in {";", "{"}:
                    break
                if "\n" in cleaned[match.end():token.start()]:
                    break
            if end is None:
                diagnostic("canonical-chapel-control-review", start,
                    "Unbraced Chapel control has no recognized then/do boundary.")
            else:
                emit("header", start, end, scope="chapel-unbraced")

    if language in KEYWORD:
        tokens = list(TOKEN.finditer(cleaned))
        tokens_at = {match.start(): match for match in tokens}
        stack = []

        def header_end(position, word):
            match = tokens[position]
            base = contexts[match.start()]
            if word in {"function", "def", "macro"}:
                following = re.search(r"\(", cleaned[match.end():])
                if following:
                    opening = match.end() + following.start()
                    if "\n" not in cleaned[match.end():opening] and ";" not in cleaned[match.end():opening]:
                        end = _paren_end(cleaned, opening)
                        if end is not None:
                            tail = cleaned[end:cleaned.find("\n", end) if "\n" in cleaned[end:] else len(cleaned)].lstrip()
                            if tail.startswith(("::", "where", "->")):
                                if language == 'julia' and tail.startswith(('::', 'where')):
                                    # A typed/where signature followed by an
                                    # explicit same-scope newline or semicolon
                                    # has a definite body boundary. An inline
                                    # typed header without that boundary stays
                                    # gated rather than guessing its type span.
                                    boundary = next((i for i in range(end, len(cleaned))
                                        if cleaned[i] in ';\n' and contexts[i] == base), None)
                                    if boundary is not None:
                                        return boundary
                                diagnostic("canonical-typed-keyword-function-review", match.start(),
                                    "Typed/where keyword function signature needs explicit header review.")
                                return None
                            return end
                if language == "lua":
                    return None
            if word in {"begin", "repeat", "try", "else", "quote"}:
                return match.end()
            if word == "do" and language == "lua":
                return match.end()
            if word == "do" and language == "ruby":
                parameters = re.match(r"\s*\|[^|\n]*\|", cleaned[match.end():])
                return match.end() + parameters.end() if parameters else match.end()
            for index in range(match.end(), len(cleaned)):
                if contexts[index] != base:
                    continue
                if cleaned[index] == ";" or (cleaned[index] == "\n" and language != "lua"):
                    return index
                next_word = tokens_at.get(index)
                if next_word is None:
                    continue
                terminal = {"then"} if word in {"if", "elseif", "elsif"} else {"do"} if word in {"for", "while", "until"} else set()
                if next_word[0] in terminal:
                    return next_word.end()
                if next_word[0] == "end":
                    return None
            return None

        starters = {"if", "for", "while", "function", "do", "repeat"} if language == "lua" else {"if", "unless", "while", "until", "for", "def", "class", "module", "begin", "case", "do"} if language == "ruby" else {"if", "for", "while", "function", "let", "begin", "try", "quote", "struct", "module", "baremodule", "macro", "type", "do"}
        branches = {"elseif", "else"} if language == "lua" else {"elsif", "else", "when", "rescue", "ensure"} if language == "ruby" else {"elseif", "else", "catch", "finally"}
        for position, match in enumerate(tokens):
            word, start = match[0], match.start()
            if covered(start):
                continue
            # Julia indexing's `end`, comprehension headers, keys and symbols
            # are data, not closures. An anonymous Lua function can itself
            # appear inside call arguments and requires executable body review.
            structural = _structural(contexts[start])
            if structural and not (language == "lua" and word == "function"):
                if not stack or contexts[start] != stack[-1]["base"]:
                    continue
            if start and cleaned[start - 1] in ".:":
                continue
            if word == "end":
                if not stack:
                    diagnostic("canonical-unmatched-keyword-close", start,
                        "Keyword close has no recognized executable opener.")
                    continue
                opened = stack.pop()
                emit("close", start, match.end(), scope="keyword")
                regions.append(dict(start=opened["body_start"], end=start, scope="keyword"))
                continue
            if language == "lua" and word == "until":
                if not stack or stack[-1]["word"] != "repeat":
                    diagnostic("canonical-unmatched-repeat-close", start,
                        "Lua until has no recognized repeat block.")
                    continue
                opened = stack.pop()
                end = next((i for i in range(match.end(), len(cleaned))
                    if cleaned[i] in ";\n" and contexts[i] == contexts[start]), len(cleaned))
                emit("close", start, end, scope="keyword")
                regions.append(dict(start=opened["body_start"], end=start, scope="keyword"))
                continue
            if word in branches:
                if not stack:
                    diagnostic("canonical-unmatched-keyword-branch", start,
                        "Keyword branch has no recognized executable opener.")
                    continue
                end = header_end(position, word)
                if end is None:
                    diagnostic("canonical-keyword-header-review", start,
                        "Branch condition/body boundary is ambiguous.")
                    continue
                emit("branch", start, end, scope="keyword")
                continue
            if word not in starters:
                continue
            is_function = word in {"function", "def", "macro"}
            at_start = _at_statement_start(cleaned, start, language)
            ruby_value_block = language == 'ruby' and word in {'if', 'unless', 'case', 'begin'} and bool(
                re.search(r'(?<![=!<>])=\s*$', cleaned[_line_start(cleaned,start):start]))
            julia_value_block = language == 'julia' and (
                word in {'begin','if','let','try','quote'} or
                (word in {'for','while'} and '@' in cleaned[_line_start(cleaned,start):start]))
            if not at_start and word != "do" and not is_function:
                if ruby_value_block:
                    # Track this expression block so its `end` cannot consume
                    # an outer control's close. Counting/collapsing its value
                    # branches still needs exact-source semantic review.
                    diagnostic('canonical-expression-block-review',start,
                        'Ruby assignment-valued block needs expression/body semantic review.')
                elif julia_value_block:
                    diagnostic('canonical-expression-block-review',start,
                        'Julia expression/macro-valued block needs expression/body semantic review.')
                else:
                    if language=="ruby" and word in {"if","unless","while","until"}:
                        end = _modifier_end(cleaned,match,contexts)
                        if end is not None:
                            emit('header',start,end,scope='ruby-modifier')
                            continue
                        diagnostic("canonical-postfix-control-review",start,
                            "Ruby modifier condition has an unsupported nested/continued boundary.")
                    continue
            end = header_end(position, word)
            if end is None:
                diagnostic("canonical-keyword-header-review", start,
                    "Keyword block header/body boundary is ambiguous.")
                continue
            if word == "do" or is_function:
                header_start = _trim_start(cleaned, _line_start(cleaned, start), start)
                prior = [e["end"] for e in events if header_start <= e["end"] <= start]
                if prior:
                    header_start = max(prior)
                    header_start = _trim_start(cleaned, header_start, start)
            else:
                header_start = start
                if language == "julia":
                    prefix = cleaned[_line_start(cleaned, start):start].strip()
                    if prefix in {"mutable", "abstract", "primitive"} or _julia_macro_prefix(prefix):
                        header_start = _trim_start(cleaned, _line_start(cleaned, start), start)
            emit("header", header_start, end, scope="keyword")
            stack.append(dict(word=word, start=start, body_start=end, base=contexts[start]))
            if structural:
                diagnostic("canonical-keyword-callback-review", start,
                    "Keyword function inside an outer argument/data scope requires executable-context review.")
        for opened in stack:
            diagnostic("canonical-unclosed-keyword-block", opened["start"],
                "Keyword block has no recognized explicit closing keyword.")

    if language == "mech":
        # A signature ending at := starts a multiline block. The old `=` output
        # signature and the new `=> <type>` signature both count one header.
        # An inline => ... := expression remains an expression, not a block.
        signature = re.compile(r"(?m)^[ \t]*([A-Za-z_]\w*\s*\([^\n]*\)\s*(?:=\s*\([^\n]*\)|=>\s*<[^\n]+>)\s*:=)[ \t]*\r?$")
        headers = list(signature.finditer(cleaned))
        for position, match in enumerate(headers):
            emit("header", match.start(1), match.end(1), scope="mech-block")
            next_header = headers[position + 1].start() if position + 1 < len(headers) else len(cleaned)
            closing = None
            for ending in re.finditer(r"(?m)\.(?=[ \t]*(?:\r?\n|$))", cleaned[match.end():next_header]):
                dot = match.end() + ending.start()
                # A final dot at the function's own delimiter scope is the
                # executable close, including `count := flips + 1.`. Interior
                # decimal/member dots never match the end-of-line boundary.
                before = cleaned[dot - 1:dot]
                if before and not before.isspace() and not _structural(contexts[dot]):
                    closing = dot
                    break
            if closing is None:
                diagnostic("canonical-mech-block-close-review", match.start(1),
                    "Mech block function has no recognized terminal dot; review its exact grammar.")
            else:
                emit("close", closing, closing + 1, scope="mech-block")
                regions.append(dict(start=match.end(), end=closing, scope="mech-block"))
        for match in re.finditer(r"(?m)^\s*[A-Za-z_]\w*\s*\([^\n]*\)[^\n]*:=", cleaned):
            word_start = _trim_start(cleaned, match.start(), match.end())
            if not covered(word_start) and "=>" not in match[0]:
                diagnostic("canonical-mech-function-form-review", word_start,
                    "Mech function signature is outside the reviewed old-block/new-expression forms.")

    events.sort(key=lambda event: (event["start"], event["end"]))
    for left, right in zip(events, events[1:]):
        if left["end"] > right["start"]:
            diagnostic("canonical-overlapping-spans-review", right["start"],
                "Canonical header/closure spans overlap; do not admit without review.")
    return dict(events=events,
        boundaries=sorted({offset for event in events for offset in (event["start"], event["end"])}),
        zero_unit_spans=[event for event in events if event["units"] == 0],
        executable_regions=regions, diagnostics=diagnostics,
        rule="One explicit header, zero extra opening-brace units, separate body units, one explicit close; data wrapping and expression forms retained.")
