"""Conservative, inspectable legacy LOC normalization; source bytes never change.

The lexer preserves physical positions. Semicolon segments are counted first,
then independent/chained bindings within each segment. Unknown forms are review
diagnostics, not a claim that a source is fully normalized.
"""
import re
from bisect import bisect_right
from lexical import mask_source
from canonical import canonical_units, _rust_function_start
from value_forms import classify_value_forms
from c_type_environment import (collect_typedef_aliases, identify_decl,
                                classify_alias_suffixes, possible_packed_decl,
                                _type_prefix)
from record_fields import classify_declaration_fields
from compiler_attributes import classify_gnu_attributes
from cpp_initializers import classify_cpp_initializers
from c_wrapped_type_declarations import classify_wrapped_type_declaration_gates

RULE_VERSION = "canonical-source-units-20261012-approved-rule-v2"
ASSIGN = re.compile(r"(?<![=!<>~])(?:\*\*=|<<=|>>=|//=|[+*/%&|^-]=|:=|=(?!=|>|~))")
R_ASSIGN=re.compile(r"(?<![=!<>~])(?:<<-|->>|<-|->|[+*/%-]=|=(?!=|>|~))")
WORDS = re.compile(r"[A-Za-z_$][\w$]*")
STATIC = {"c", "cpp", "java", "chapel"}
CONTROL = {"for", "foreach", "while", "until", "if", "unless", "switch", "catch", "with"}
EXECUTABLE_SCOPES = {"code", "bare-code"}
DECLARATION = re.compile(
    r"^(?:(?:static|const|constexpr|volatile|register|extern|unsigned|signed|long|short|"
    r"public|private|protected|final|var|let|auto|ref|in|out)\s+)*"
    r"(?:void|bool|boolean|char|byte|short|int|long|float|double|size_t|ssize_t|"
    r"int\d+_t|uint\d+_t|real|imag|complex|uint|signed|unsigned|"
    r"__int128|__m(?:128|256|512)[di]?|(?:struct|union|enum)\s+[A-Za-z_]\w*|"
    r"String|auto|var|let|[A-Z]\w*)"
    r"(?:\s*<[^;{}]+>)?(?:\s*\[\s*\])*(?:\s+|(?=[*&]))(.+)$", re.S)


def _word_before(text, index):
    match = re.search(r"([A-Za-z_$][\w$]*[!?]?)\s*$", text[:index])
    return match[1] if match else ""


def _go_struct_header(text, opening):
    """Recognize a named Go type declaration, never a struct constructor."""
    prefix = text[max(0, opening - 240):opening].rstrip()
    return bool(re.search(r"\btype\s+[A-Za-z_]\w*(?:\s*\[[^\]\n{}]*\])?\s+struct\s*$", prefix))


def _known_type_body(prefix, language):
    """Bounded declaration grammar for proven aggregate/class body forms."""
    tail = prefix.rsplit('\n', 1)[-1]
    if language == 'go':
        return bool(re.search(r"\btype\s+[A-Za-z_]\w*(?:\s*\[[^\]\n{}]*\])?\s+(?:struct|interface)\s*$", tail))
    if language == 'rust':
        return bool(re.search(r"\b(?:struct|enum|union|trait)\s+[A-Za-z_]\w*(?:\s*<[^{}\n;]+>)?(?:\s+where\s+[^{}\n;]+)?\s*$", tail))
    if language in {'c', 'cpp'}:
        return bool(re.search(r"\b(?:struct|union|enum)\s*(?:[A-Za-z_]\w*\s*)?$", tail))
    if language == 'php':
        return bool(re.search(r"\bnew\s+class(?:\s*\([^()\n]*\))?(?:\s+(?:extends|implements)\s+[\w\\,\s]+)*\s*$", tail))
    if language == 'java':
        return bool(re.search(r'\bstatic\s*$', tail) or
                    re.search(r'\)\s*throws\s+[A-Za-z_$][\w.$]*(?:\s*,\s*[A-Za-z_$][\w.$]*)*\s*$', prefix) or
                    re.search(r'\b(?:class|interface|enum)\s+[A-Za-z_$][\w$]*'
                              r'(?:\s*<[^{};]+>)?(?:\s+(?:extends|implements)\s+[\w.$,< >?]+)*\s*$', prefix))
    return False


def _perl_data_brace(text, opening):
    """Recognize proven hash dereference/subscript braces, not control bodies.

    An arrow immediately before a brace is Perl dereference syntax, including
    `call()->{key}`. Direct sigil receivers are deliberately limited to adjacent
    named variables; uncertain spaced or more elaborate receiver forms retain
    the existing review path instead of guessing their grammar.
    """
    prefix = text[max(0, opening - 180):opening]
    if re.search(r"->\s*$", prefix):
        return True
    if not prefix or prefix[-1].isspace():
        return False
    return bool(re.search(r"[$@%](?:[A-Za-z_]\w*(?:::[A-Za-z_]\w*)*|\d+)$", prefix))


def _expression_introducer(text, start):
    """Restrict anonymous headers to an expression or statement boundary."""
    prefix = text[:start].rstrip()
    return (not prefix or prefix[-1] in '=(:,?;{}' or
            bool(re.search(r'\breturn\s*$', prefix)))


def _balanced_end(text, opening, left, right):
    depth = 0
    for index in range(opening, len(text)):
        if text[index] == left:
            depth += 1
        elif text[index] == right:
            depth -= 1
            if depth == 0:
                return index + 1
    return None


def _simple_anonymous_type(text, language):
    """Accept only bounded named types; patterns/compound types stay reviewed."""
    if language == 'rust':
        return bool(re.fullmatch(
            r"\s*(?:&\s*(?:'\w+\s*)?(?:mut\s+)?)?[A-Za-z_]\w*"
            r"(?:::[A-Za-z_]\w*)*(?:\s*<[^<>{}()\[\];=|]+>)?\s*", text))
    return bool(re.fullmatch(
        r'\s*(?:(?:const|volatile|unsigned|signed|long|short)\s+)*'
        r'[A-Za-z_]\w*(?:::[A-Za-z_]\w*)*(?:\s*<[^<>{}()\[\];=|]+>)?'
        r'[\s*&]*', text))


def _anonymous_brace(text, opening, language):
    """Recognize simple C++/Rust block lambdas; gate more complex headers.

    Returns `(known_executable, review_kind)`. A review result does not change a
    scope already established by another proven rule. Capture brackets must not
    be confused with an indexed receiver or ordinary array declaration.
    """
    window_start = max(0, opening - 512)
    header = text[window_start:opening]
    if language == 'cpp':
        for match in reversed(list(re.finditer(r'\[', header))):
            start = window_start + match.start()
            if not _expression_introducer(text, start):
                continue
            end = _balanced_end(text, start, '[', ']')
            if end is None or end > opening:
                continue
            suffix = text[end:opening]
            if any(char in suffix for char in ';{}'):
                continue  # A capture in an earlier completed expression.
            captures = text[start + 1:end - 1].strip()
            parts = [part.strip() for part in captures.split(',')] if captures else []
            simple_capture = r'(?:[=&]|&?\s*[A-Za-z_]\w*|\*\s*this)'
            if not all(re.fullmatch(simple_capture, part) for part in parts):
                return False, 'cpp-complex-lambda-header-review'
            tail = suffix.lstrip()
            if tail.startswith('('):
                parameter_end = _balanced_end(tail, 0, '(', ')')
                if parameter_end is None:
                    return False, 'cpp-complex-lambda-header-review'
                parameters = tail[1:parameter_end - 1]
                if re.search(r'[()\[\]{};]', parameters):
                    return False, 'cpp-complex-lambda-header-review'
                tail = tail[parameter_end:].lstrip()
            for specifier in ('mutable', 'noexcept'):
                spec = re.match(r'\b' + specifier + r'\b\s*', tail)
                if spec:
                    tail = tail[spec.end():].lstrip()
                    if specifier == 'noexcept' and tail.startswith('('):
                        return False, 'cpp-complex-lambda-header-review'
            if tail.startswith('->'):
                if not _simple_anonymous_type(tail[2:], 'cpp'):
                    return False, 'cpp-complex-lambda-header-review'
                tail = ''
            if tail.strip():
                return False, 'cpp-complex-lambda-header-review'
            return True, None
    elif language == 'rust':
        # The introducer restriction excludes bitwise-or expressions. Async
        # closures and destructuring patterns are deliberately still gated.
        for match in reversed(list(re.finditer(r'\|([^|]*)\|', header))):
            start = window_start + match.start()
            prefix = text[:start]
            modifier = re.search(r'\b(?:(async)\s+)?(move)\s*$', prefix)
            if modifier:
                start = modifier.start()
            else:
                modifier = re.search(r'\b(async)\s*$', prefix)
                if modifier:
                    start = modifier.start()
            if not _expression_introducer(text, start):
                continue
            suffix = header[match.end():].strip()
            if any(char in suffix for char in ';{}'):
                continue
            if suffix and not suffix.startswith('->'):
                # An expression-bodied closure in a surrounding iterator or
                # condition does not introduce this following block brace.
                continue
            if modifier and modifier[1] == 'async':
                return False, 'rust-complex-closure-header-review'
            parameters = match[1].strip()
            parts = [part.strip() for part in parameters.split(',')] if parameters else []
            for part in parts:
                parameter = re.fullmatch(r'(?:(?:ref\s+)?mut\s+|ref\s+)?[A-Za-z_]\w*(?:\s*:\s*(.+))?', part)
                if not parameter or (parameter[1] is not None and
                                     not _simple_anonymous_type(parameter[1], 'rust')):
                    return False, 'rust-complex-closure-header-review'
            if suffix and (not suffix.startswith('->') or
                           not _simple_anonymous_type(suffix[2:], 'rust')):
                return False, 'rust-complex-closure-header-review'
            return True, None
    return False, None


def _proven_bare_block(text, opening, stack, language):
    """Identify a standalone lexical block at an executable statement boundary.

    Initializers, argument/data scopes, type names and uncertain prior headers
    remain outside this rule. A bare opener deliberately has no header unit.
    """
    if language not in {'c', 'cpp', 'rust', 'go', 'java', 'chapel', 'javascript', 'php'}:
        return False
    line_start = text.rfind('\n', 0, opening) + 1
    inline_prefix = text[line_start:opening].strip()
    if inline_prefix and not text[:opening].rstrip().endswith((';', '{', '}')):
        return False
    last_code = max((i for i, (_, kind, _) in enumerate(stack) if kind in EXECUTABLE_SCOPES), default=-1)
    if last_code < 0 or any(kind in {'data', 'args', 'parameters', 'for', 'control'} for _, kind, _ in stack[last_code + 1:]):
        return False
    prefix = text[:opening].rstrip()
    prior = prefix.rsplit('\n', 1)[-1].strip()
    if not prior:
        return False
    # C/OpenMP structured-block directives remain counted semantic rows; their
    # following brace adds no header or opener unit of its own.
    if language in {'c', 'cpp'} and re.match(r'^#\s*pragma\s+omp\s+(?:parallel|single|sections|section|critical|master|task|taskgroup|ordered|teams|target)\b', prior):
        return True
    if prior.endswith((';', '{', '}')):
        return True
    # Go permits a completed assignment/declaration immediately before a new
    # lexical block without an explicit semicolon. Do not absorb that row.
    if language == 'go' and ASSIGN.search(prior):
        if not re.search(r'\b(?:if|for|switch|select|func|type)\b', prior) and not re.search(r'(?:=|:=|,|:|\(|\[)\s*$', prior):
            return True
    if language == 'go' and prior.endswith(')') and re.search(r'[A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*\s*\(', prior):
        if not re.search(r'\b(?:if|for|switch|select|func|type)\b', prior):
            return True
    if language == 'java' and prior == 'static':
        return True
    return False


def _scope_map(text, language):
    """Map characters to structural scopes; retain inline executable bodies."""
    contexts = []
    stack = []
    diagnostics = []
    go_headers=[]
    if language=="go":
        go_headers=[(m.start(),m.end()-1,'for') for m in re.finditer(r"\bfor\s+(?!\()[^\n{]*\{",text)]
        go_headers.extend((m.start(),m.end()-1,'control')
                          for m in re.finditer(r'\bif\s+[^\n{]*\{',text))
    def following_close(index):
        depth=0
        for j in range(index,len(text)):
            if text[j]=="(":depth+=1
            elif text[j]==")":
                depth-=1
                if depth==0:return text[j+1:j+120]
        return ""
    for index, char in enumerate(text):
        synthetic=[("(",kind,start) for start,end,kind in go_headers if start<=index<end]
        contexts.append(tuple(stack+synthetic))
        if char == "(":
            word = _word_before(text, index)
            prefix = text[max(0, index - 160):index]
            kind = "for" if word in {"for", "foreach"} else "control" if word in CONTROL else "args" if word else "group"
            signature=re.search(r"\b(?:function|def|fn|func|proc)(?:\s+[\w$]+)?\s*$",prefix)
            if signature or (language=="javascript" and re.match(r"\s*=>",following_close(index))):
                kind = "parameters"
            stack.append((char, kind, index))
        elif char == "[":
            stack.append((char, "data", index))
        elif char == "{":
            prefix = text[max(0, index - 180):index].rstrip()
            tail = prefix.rsplit("\n", 1)[-1]
            previous = prefix[-1:] or ""
            code = previous == ")" or bool(re.search(r"(?:=>|\belse|\btry|\bfinally|\bdo|\bloop)\s*$", prefix))
            code = code or bool(re.search(r"\b(?:struct|class|enum|union|impl|trait|namespace)\s+\w*\s*$", tail))
            code = code or _known_type_body(prefix, language)
            code = code or bool(re.search(r"\bnamespace(?:\s+[\w:]+)?$",tail))
            code = code or bool(re.search(r"\)\s*(?:(?:const|noexcept|override|final)\s*|&&?\s*)+$",prefix))
            # Typed Rust/Go return signatures and unparenthesized control
            # headers still introduce executable bodies, not record literals.
            code = code or bool(re.search(r"\b(?:fn|func|proc|for|forall|coforall|cobegin|while|if|when|match|select)\b[^;{}]*$",tail))
            if language == 'rust':
                code = code or bool(re.search(r'\bfor\s+[A-Za-z_]\w*\s+in\s+[^;{}]+$',prefix))
                code = code or _rust_function_start(text, index) is not None
            code = code or bool(re.search(r"\|\|\s*$",prefix))
            if language == 'java':
                code = code or bool(re.search(r'(?:\([^(){}]*\)|[A-Za-z_]\w*)\s*->\s*$',prefix))
            code = code or bool(re.search(r"\bsub(?:\s+\w+)?\s*$",prefix))
            if language == 'perl':
                code = code or bool(re.search(r'\b(?:unless|until)\b[^;{}]*$',tail))
            code = code or (language=="go" and any(end==index for _,end,_ in go_headers))
            if language in {'cpp', 'rust'}:
                anonymous, anonymous_review = _anonymous_brace(text, index, language)
                code = code or anonymous
                if anonymous_review:
                    diagnostics.append(dict(kind=anonymous_review, offset=index,
                        message='Anonymous function header has a complex capture, parameter, or declarator form requiring review.'))
            if language in {"c","cpp"}:
                cast=re.search(r"\(\s*(?:(?:const|volatile|struct|union|enum|unsigned|signed|long|short)\s+)*[A-Za-z_]\w*(?:\s*\*)*\s*\)\s*$",prefix)
                if cast and _word_before(prefix,cast.start()) in {"","return"}:
                    # `(type){...}` is a compound/data literal, not a function
                    # or control header followed by an executable block.
                    code=False
                if (not code and previous=='(' and stack and stack[-1][0]=='('
                        and stack[-1][1]=='group'):
                    diagnostics.append(dict(kind='c-statement-expression-owner-review',
                        offset=index,
                        message='GNU ({...}) statement-expression body needs explicit executable binding/scope ownership; it cannot be certified as aggregate data.'))
            if not tail and prefix:
                prior = prefix.rsplit("\n", 1)[-1]
                code = bool(re.search(r"\b(?:if|while|for|switch|else|try|class|struct|fn|function)\b", prior))
            if language == "ruby":
                # Curly Ruby callbacks require a callable receiver. A bare
                # hash, returned hash or ternary hash remains literal data.
                receiver = _word_before(prefix, len(prefix))
                code = previous in {')', ']'} or bool(receiver)
                if receiver in {'return', 'yield', 'break', 'next', 'if', 'unless',
                                'while', 'until', 'then', 'else', 'when'}:
                    code = False
            # Destructuring/default object parameters are data even when a
            # function keyword occurs earlier on the same signature row.
            if language == 'javascript' and previous in {'(', ',', '=', ':'}:
                nearest_paren = next((entry for entry in reversed(stack) if entry[0] == '('), None)
                if nearest_paren and nearest_paren[1] == 'parameters':
                    code = False
            if language == 'perl' and _perl_data_brace(text, index):
                # A preceding modifier/control keyword elsewhere on the row
                # does not turn `$h{key}` or `expr->{key}` into executable code.
                code = False
            go_function_body = False
            go_data_brace = False
            if language == 'go':
                if (re.search(r'\b(?:struct|interface)\s*$',prefix) and
                        not _known_type_body(prefix,'go')):
                    # `make(chan struct{})` and function return `interface{}`
                    # contain type braces, including inside executable headers.
                    code = False
                    go_data_brace = True
                elif re.search(r'\bfunc(?:\s+[A-Za-z_]\w*)?\s*\([^{};]*\)\s*'
                               r'(?:struct|interface)\s*\{\s*\}\s*$',prefix):
                    code = True
                    go_function_body = True
                elif re.search(r'\b(?:struct|interface)\s*\{\s*\}[ \t\r]*\Z',
                               text[max(0,index-180):index]):
                    # The second brace in `struct{}{}` constructs data. The
                    # preceding function-return-type rule takes priority. A
                    # newline after a complete type can end a declaration and
                    # precede a separate bare block instead of a constructor.
                    code = False
                    go_data_brace = True
                elif re.search(r'\bfunc(?:\s+[A-Za-z_]\w*)?\s*\([^{};]*\)\s*'
                               r'(?:struct|interface)\s*\{[^{}]*\}\s*$',prefix):
                    diagnostics.append(dict(kind='go-complex-anonymous-return-type-review',offset=index,
                        message='Function body follows a nonempty anonymous return type requiring exact header review.'))
            bare = not (go_function_body or go_data_brace) and _proven_bare_block(text, index, stack, language)
            stack.append((char, 'bare-code' if bare else 'code' if code else 'data', index))
        elif char in ")]}":
            wanted = {")": "(", "]": "[", "}": "{"}[char]
            if not stack or stack[-1][0] != wanted:
                diagnostics.append(dict(kind="unbalanced-delimiter", offset=index, character=char))
            else:
                stack.pop()
    if stack:
        diagnostics.extend(dict(kind="unclosed-delimiter", offset=i, character=c) for c, _, i in stack)
    return contexts, diagnostics


def _structural(context, *, assignment=False, language=None):
    # A function/lambda body inside a call opens a new executable scope.
    last_code = max((i for i, (_, kind, _) in enumerate(context) if kind in EXECUTABLE_SCOPES), default=-1)
    relevant = context[last_code + 1:]
    if any(kind in {"for", "parameters", "data"} for _, kind, _ in relevant):
        return True
    if assignment:
        # Only these languages use = for named/default call arguments.
        return language in {"python", "julia", "r", "chapel"} and any(kind == "args" for _, kind, _ in relevant)
    return any(kind in {"args", "control"} for _, kind, _ in relevant)


def _top_parts(text, separator=","):
    stack = []
    pieces = []
    start = 0
    for index, char in enumerate(text):
        if char in "([{":
            stack.append(char)
        elif char in ")]}":
            if stack:
                stack.pop()
        elif char == separator and not stack:
            pieces.append(text[start:index].strip())
            start = index + 1
    pieces.append(text[start:].strip())
    return pieces


def _lua_rhs_kind(value, original_value=None):
    """Prove a lone Lua multiple-return producer; never infer one from arity."""
    value = value.strip()
    if value == '...':
        return 'producer'
    if value.startswith('(') and _balanced_end(value, 0, '(', ')') == len(value):
        return 'single'  # Parentheses explicitly suppress multiple returns.
    call = re.match(r'[A-Za-z_]\w*(?:[.:][A-Za-z_]\w*)*\s*\(', value)
    if call and _balanced_end(value, call.end() - 1, '(', ')') == len(value):
        return 'producer'
    original = original_value.strip() if original_value is not None else ''
    if (original.startswith(('"', "'")) or
            re.match(r'\[=*\[', original) or value.startswith('{')):
        return 'single'
    stack = []
    for match in re.finditer(r'[A-Za-z_]\w*|[()\[\]{}]|\.{2,3}|[+*/%^#&|<>=~-]', value):
        token = match[0]
        if token in {'(', '[', '{'}:
            stack.append(token)
        elif token in {')', ']', '}'}:
            if stack:
                stack.pop()
        elif not stack and (token in {'and', 'or', 'not', '..'} or
                            token[0] in '+-*/%^#&|<>=~'):
            return 'single'
    if re.fullmatch(r'[A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*', value):
        return 'single'
    if re.fullmatch(r'(?:0[xX][0-9A-Fa-f]+(?:\.[0-9A-Fa-f]*)?(?:[pP][+-]?\d+)?|'
                    r'(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?)', value):
        return 'single'
    if re.fullmatch(r'[A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*(?:\s*\[[^\[\]]+\])+', value):
        return 'single'
    return 'review'


def _perl_uninitialized_names(code):
    """Prove a declaration list with no RHS; assigned lists stay tuples."""
    variable = r'[$@%][A-Za-z_]\w*(?:::[A-Za-z_]\w*)*'
    names = variable + r'(?:\s*,\s*' + variable + r')*'
    found = re.fullmatch(r'(?:my|our|state|local)\b\s*(?:\(\s*(' + names +
                        r')\s*\)|(' + variable + r'))', code, re.S)
    return _top_parts(found[1] or found[2]) if found else None


def _php_declaration_parts(code):
    """Modifier-led properties/statics declare names, unlike container entries."""
    found = re.fullmatch(
        r'(?:(?:public|protected|private|static|var|readonly)\b\s*)+'
        r'(?:\??[\\A-Za-z_][\\\w]*(?:[|&]\??[\\A-Za-z_][\\\w]*)*\s+)?(.+)',
        code, re.S)
    if not found:
        return None
    parts = _top_parts(found[1])
    if all(re.fullmatch(r'\$[A-Za-z_]\w*\s*(?:=.*)?', part, re.S) for part in parts):
        return parts
    return None


def _declaration_bindings(segment, language, original_segment=None):
    """Separate bare declaration names; explicit destructuring stays one unit."""
    code = segment.strip() if language == 'lua' else segment.strip().strip("{} ")
    if language == 'perl':
        names = _perl_uninitialized_names(code)
        if names is not None:
            return len(names), 'independent-uninitialized-declaration-bindings'
    if language == 'php':
        parts = _php_declaration_parts(code)
        if parts is not None:
            return len(parts), 'independent-declaration-bindings'
    if language == "lua":
        declaration = r"^(?:local\s+)?([A-Za-z_]\w*(?:\s*,\s*[A-Za-z_]\w*)+)\s*(?:=(.*))?$"
        found = re.match(declaration, code, re.S)
        if not found:
            return 0, None
        names = _top_parts(found[1])
        if found[2] is None:
            return len(names), "independent-declaration-bindings"
        values = _top_parts(found[2])
        if len(values) == len(names):
            return len(names), "independent-multiple-bindings"
        if len(values) != 1:
            return len(names), 'lua-multiple-rhs-review'
        original_found = (re.match(declaration, original_segment.strip(), re.S)
                          if original_segment is not None else None)
        original_value = original_found[2] if original_found else None
        rhs_kind = _lua_rhs_kind(values[0], original_value)
        if rhs_kind == 'producer':
            return 1, 'tuple-destructuring'
        if rhs_kind == 'single':
            return len(names), 'independent-bindings-with-nil-tail'
        return len(names), 'lua-multiple-return-producer-review'
    if language == "go":
        found = re.match(r"^(?:var\s+)?([A-Za-z_]\w*(?:\s*,\s*[A-Za-z_]\w*)+)\s*(?::=|=)(.*)$", code, re.S)
        if found:
            names, values = _top_parts(found[1]), _top_parts(found[2])
            return (len(names), "independent-multiple-bindings") if len(values) == len(names) else (1, "tuple-destructuring")
        found = re.match(r"^var\s+([A-Za-z_]\w*(?:\s*,\s*[A-Za-z_]\w*)+)\s+[\w\[\]*.]+$", code)
        if found:
            return len(_top_parts(found[1])), "independent-declaration-bindings"
    if language in {"javascript", "chapel"}:
        found = re.match(r"^(?:var|let|const|param|ref)\s+(.+)$", code, re.S)
        if found and not found[1].lstrip().startswith(("[", "(", "{")):
            parts = _top_parts(found[1])
            if all(re.match(r"^[A-Za-z_$][\w$]*(?:\s*[:=].*)?$", p, re.S) for p in parts):
                return len(parts), "independent-declaration-bindings"
    if language in STATIC:
        found = DECLARATION.match(code)
        if found and not found[1].lstrip().startswith(("[", "(", "{")):
            parts = _top_parts(found[1])
            if all(re.match(r"^[*&\s]*[A-Za-z_]\w*(?:\s*\[[^]]*\])*\s*(?:=.*)?$", p, re.S) for p in parts):
                return len(parts), "independent-declaration-bindings"
    return 0, None


def _uninitialized_declarators(segment, language):
    code=segment.strip().strip("{} ")
    if language == 'php':
        parts = _php_declaration_parts(code)
        return sum(not ASSIGN.search(part) for part in parts) if parts is not None else 0
    found=DECLARATION.match(code) if language in STATIC else re.match(r"^(?:var|let|const|param|ref)\s+(.+)$",code,re.S) if language=="javascript" else None
    if not found:
        return 0
    parts=_top_parts(found[1])
    if not all(re.match(r"^[*&\s]*[A-Za-z_$][\w$]*(?:\s*\[[^]]*\])*\s*(?:=.*)?$",p,re.S) for p in parts):
        return 0
    return sum(not ASSIGN.search(part) for part in parts)


def _unknown_c_packed_declaration(segment, language):
    """Gate unknown lowercase type spellings instead of guessing declarations.

    `node *a,*b` can be a typedef declaration or multiplication/comma syntax.
    Without verified type/scope information it retains its expression count and
    requires review. Calls, explicit tuples and ordinary comma expressions do
    not become declarations merely because they contain a comma.
    """
    if language not in {'c', 'cpp'}:
        return False
    code = segment.strip().strip('{} ')
    if DECLARATION.match(code):
        return False
    found = re.match(
        r'^(?:(?:static|const|volatile|register|extern|inline)\s+)*'
        r'([a-z_]\w*)(?:\s+|(?=[*&]))(.+)$', code, re.S)
    if not found or found[1] in {'return', 'co_return', 'throw', 'delete', 'new',
        'if', 'else', 'for', 'while', 'do', 'switch', 'case', 'default', 'goto',
        'break', 'continue', 'sizeof', 'alignof', 'typedef', 'using', 'struct',
        'union', 'enum', 'class', 'namespace', 'asm'}:
        return False
    parts = _top_parts(found[2])
    return (len(parts) > 1 and
            all(re.fullmatch(r'[*&\s]*[A-Za-z_]\w*(?:\s*\[[^]]*\])*\s*(?:=.*)?',
                             part, re.S) for part in parts) and
            any(not ASSIGN.search(part) for part in parts))


def _go_field_bindings(segment, cleaned, context):
    """Count grouped field names only in an exact named Go struct body."""
    own = next((entry for entry in reversed(context) if entry[0] == '{'), None)
    if not own or own[1] not in EXECUTABLE_SCOPES or not _go_struct_header(cleaned, own[2]):
        return 0, None
    code = segment.strip()
    found = re.match(r'^([A-Za-z_]\w*(?:\s*,\s*[A-Za-z_]\w*)+)\s+(.+)$', code, re.S)
    if not found or ASSIGN.search(found[2]) or found[2].lstrip().startswith(('{', '=', ':=')):
        return 0, None
    return len(_top_parts(found[1])), 'independent-struct-field-declarations'


def _owned_context_overrides(contexts, overrides):
    """Replace only a proved owner's grammar frame, retaining nested owners.

    A constructor prefix is later opaque, so its body frame may no longer be
    present in the surface scope map. Insert that exact frame before any nested
    frames; do not discard outer scopes or alter unrelated positions.
    """
    result=list(contexts)
    for owner in overrides:
        frame=(owner['delimiter'],owner['kind'],owner['opening'])
        for index in range(owner['start'],owner['end']):
            entries=[entry for entry in result[index] if entry[2]!=owner['opening']]
            before=[entry for entry in entries if entry[2]<owner['opening']]
            after=[entry for entry in entries if entry[2]>owner['opening']]
            result[index]=tuple(before+[frame]+after)
    return result


def normalize(source, language):
    language = {"luajit": "lua", "python3": "python", "node": "javascript", "c++": "cpp","mech-jit":"mech","mech-resident":"mech"}.get(language, language)
    masked = mask_source(source, language)
    assignment_pattern=R_ASSIGN if language=="r" else ASSIGN
    source_cleaned=masked['cleaned']
    annotations=classify_gnu_attributes(source_cleaned,language)
    original_cleaned = annotations['cleaned']
    type_environment = (collect_typedef_aliases(original_cleaned, language,
        semantic_imports=masked.get('semantic_imports', ()), preprocessor_source=source,
        protected_literal_spans=masked.get('masked_literal_spans', ()))
        if language in {'c', 'cpp'} else None)
    alias_suffixes = (classify_alias_suffixes(original_cleaned, language, type_environment)
                      if type_environment is not None else dict(spans=[], diagnostics=[]))
    contexts, scope_diagnostics = _scope_map(original_cleaned, language)
    initializers=classify_cpp_initializers(original_cleaned,language,contexts,
        type_environment, declaration_resolver=identify_decl)
    contexts=_owned_context_overrides(contexts,initializers['context_overrides'])
    values = classify_value_forms(original_cleaned, language, contexts)
    fields = classify_declaration_fields(original_cleaned, language, contexts)
    # A proven composed value is one operand, not an executable block. Make it
    # opaque before both scope classification and canonical event splitting;
    # retain source offsets and newlines for exact evidence and physical LOC.
    opaque_chars = list(original_cleaned)
    for span in values["spans"]:
        for index in range(span["start"], span["end"]):
            if not opaque_chars[index].isspace():
                opaque_chars[index] = " "
        opaque_chars[span["start"]] = "V"
    for span in fields['owned_spans']:
        for index in range(span['start'],span['end']):
            if not opaque_chars[index].isspace():
                opaque_chars[index]=' '
    # The full proof owns only written constructor prefix/open/close syntax.
    # Body statements remain visible. Mask these exact ranges before canonical
    # scanning to prevent a stale surface grammar from inventing a second
    # header or treating a member's data initializer as an executable block.
    for span in initializers['suppress_canonical_spans']:
        for index in range(span['start'],span['end']):
            if not opaque_chars[index].isspace():opaque_chars[index]=' '
    cleaned = ''.join(opaque_chars)
    contexts, opaque_diagnostics = _scope_map(cleaned, language)
    contexts=_owned_context_overrides(contexts,initializers['context_overrides'])
    diagnostics = list(masked["diagnostics"]) + scope_diagnostics
    diagnostics.extend(item for item in opaque_diagnostics if item not in scope_diagnostics)
    diagnostics.extend(values["diagnostics"])
    diagnostics.extend(fields['diagnostics'])
    diagnostics.extend(annotations['diagnostics'])
    diagnostics.extend(initializers['diagnostics'])
    if type_environment is not None:
        diagnostics.extend(type_environment['diagnostics'])
    diagnostics.extend(alias_suffixes['diagnostics'])
    canonical=canonical_units(cleaned,language,contexts)
    canonical['events'].extend(initializers['events'])
    canonical['executable_regions'].extend(initializers['executable_regions'])
    diagnostics.extend(canonical["diagnostics"])
    if type_environment is not None:
        for suffix in type_environment['aggregate_alias_suffixes']:
            opening = cleaned.find('{', suffix['owner_start'], suffix['closing_brace'])
            owner = next((event for event in canonical['events']
                          if event['kind']=='header' and event['end']==opening
                          and event['start']>=suffix['owner_start']), None)
            if owner is not None:
                owner['start']=suffix['owner_start']
                owner['typedef_primary_alias']=suffix['primary_alias']
            else:
                diagnostics.append(dict(kind='c-typedef-aggregate-header-owner-review',
                    offset=suffix['owner_start'],end=suffix['owner_end']))
    canonical["events"].extend(dict(start=span["start"], end=span["end"],
        units=span["units"], kind="value-expression", scope="composed-value", position=span["position"],
        reason=span["evidence"], form=span["form"], owned_rows=span["owned_rows"])
        for span in values["spans"])
    canonical['events'].extend(fields['owned_spans'])
    canonical['events'].extend(fields['events'])
    canonical['events'].extend(annotations['events'])
    canonical["events"].extend(dict(start=span['start'],end=span['end'],
        units=span['units'],kind='typedef-alias-suffix',scope='typedef-aggregate',
        primary_alias=span['primary_alias'],additional_aliases=span['additional_aliases'],
        reason=span['reason']) for span in alias_suffixes['spans'])
    if type_environment is not None:
        aggregate_owners={span['owner_start'] for span in type_environment['aggregate_alias_suffixes']}
        for start, end in type_environment['alias_spans']:
            if start in aggregate_owners:
                continue
            names={row['name'] for row in type_environment['evidence']
                   if row['definition_start']==start and row['definition_end']==end
                   and row['kind'] in {'source-local-typedef','source-local-using'}}
            if names:
                canonical['events'].append(dict(start=start,end=end,units=len(names),
                    kind='type-alias-binding',scope='source-local-type-alias',
                    names=sorted(names),reason='Each explicitly named independent type alias is one binding.'))
    canonical["events"].sort(key=lambda event: (event["start"], event["end"]))
    body_chars=list(cleaned)
    for event in canonical["events"]:
        for index in range(event["start"],event["end"]):
            if not body_chars[index].isspace():body_chars[index]=" "
    body_cleaned=''.join(body_chars)
    # Keyword callbacks introduce an executable region even when enclosed in
    # the caller's argument list. Inner call/data delimiters remain structural.
    effective_contexts=list(contexts)
    for region in canonical["executable_regions"]:
        if region["scope"] in {"keyword","mech-block"}:
            for index in range(region["start"],region["end"]):
                effective_contexts[index]=tuple(entry for entry in contexts[index] if entry[2]>=region["start"])
    # This owner check returns diagnostics only. Physical-row fallback must
    # not certify an independent declaration whose type or written names
    # cross rows without complete statement ownership. The pre-event mask
    # restores tokens covered by a partial canonical header, while accepted
    # operands/fields remain protected by their own exact source evidence.
    wrapped_declarations = dict(diagnostics=[], proposal_only=True)
    if type_environment is not None:
        protected = [dict(start=span['start'], end=span['end'],
                          role='field', owner_proven=True)
                     for span in fields['owned_spans']
                     if span['start'] < span['end']]
        protected.extend(dict(start=span['start'], end=span['end'],
                              role='initializer', owner_proven=True)
                         for span in values['spans'])
        protected.extend(dict(start=span['start'], end=span['end'],
                              role='literal', owner_proven=True)
                         for span in masked.get('masked_literal_spans', ()))
        wrapped_declarations = classify_wrapped_type_declaration_gates(
            body_cleaned, language, effective_contexts, type_environment,
            type_source=cleaned, type_prefix=_type_prefix,
            declaration_probe=identify_decl,
            owned_boundary_events=canonical['events'], protected_spans=protected,
            preprocessor_directive_spans=type_environment.get(
                'type_macro_environment', {}).get('directive_spans', ()))
        diagnostics.extend(wrapped_declarations['diagnostics'])
    starts = [0]
    starts.extend(match.end() for match in re.finditer(r"\n", source))
    def location(index):
        line = bisect_right(starts, index)
        return line, index - starts[line - 1] + 1
    cleaned_lines = body_cleaned.splitlines(keepends=True)
    original_cleaned_lines=source_cleaned.splitlines(keepends=True)
    original_lines = source.splitlines()
    physical_lines = [i for i, line in enumerate(original_cleaned_lines, 1) if line.strip()]
    events = {line: [] for line in physical_lines}
    canonical_by_line={line:[] for line in physical_lines}
    for event in canonical["events"]:
        line,_=location(event["start"])
        canonical_by_line.setdefault(line,[]).append(event)
    owned_lines=set()
    for event in canonical["events"]:
        first,_=location(event["start"]);last,_=location(max(event["start"],event["end"]-1))
        owned_lines.update(range(first,last+1))
    # Existing literal/data wrapping remains physical. Canonical executable
    # headers/openers/closers replace their raw spelling, including packed forms.
    for line_no in physical_lines:
        offset = starts[line_no - 1]
        line = cleaned_lines[line_no - 1]
        if re.search(r"^\s*#\s*define\b", line) and (";" in line or ASSIGN.search(line)):
            diagnostics.append(dict(kind="executable-preprocessor-macro-review", offset=offset))
        if language=="rust" and re.search(r"\w+!\s*\{",line) and (";" in line or ASSIGN.search(line)):
            diagnostics.append(dict(kind="rust-braced-macro-review",offset=offset))
        if language=="python" and re.search(r"\blambda\b[^:]*=",line):
            diagnostics.append(dict(kind="unparenthesized-default-parameter-review",offset=offset))
        if language in {"c","cpp","java","chapel","rust","go"} and re.search(r"\([^)]*=.*\)",line):
            if not re.match(r'^\s*new\b',line) and re.search(r"^\s*(?:static\s+|inline\s+|const\s+)*(?:\w+\s+)+\w+\s*\([^)]*=",line):
                diagnostics.append(dict(kind="typed-default-parameter-review",offset=offset))
        separators = [i for i, char in enumerate(line) if char == ";" and not _structural(effective_contexts[offset + i], language=language)]
        boundaries=[index-offset for index in canonical["boundaries"] if offset<index<offset+len(line)]
        # All bounds are between characters, preserving exact statement spans.
        bounds=sorted({0,len(line),*(i+1 for i in separators),*boundaries})
        for left, right in zip(bounds, bounds[1:]):
            text = line[left:right].rstrip().removesuffix(';').rstrip()
            if not text.strip("{}; \t\r\n"):
                continue
            start = offset + left
            assignments = []
            for match in assignment_pattern.finditer(text):
                index = start + match.start()
                argument_language=None if language=="r" and match[0]!="=" else language
                if not _structural(effective_contexts[index], assignment=True, language=argument_language):
                    assignments.append(dict(column=index-offset+1, operator=match[0]))
            declaration_count, declaration_reason = _declaration_bindings(
                text, language, original_segment=source[start:start + len(text)])
            proven_declaration = None
            field_index = start + len(text) - len(text.lstrip())
            if type_environment is not None and not _structural(effective_contexts[field_index], language=language):
                proven_declaration = identify_decl(text, type_environment, offset=start)
                if proven_declaration is not None:
                    declaration_count = proven_declaration['declaration_bindings']
                    declaration_reason = proven_declaration['reason']
                else:
                    possible = possible_packed_decl(text)
                    if possible is not None:
                        declaration_count = 0
                        declaration_reason = None
                        diagnostics.append(dict(possible, offset=start,
                            message='Packed declaration ownership is not proven by the source type environment; independent units require review.'))
            if declaration_reason in {'lua-multiple-rhs-review', 'lua-multiple-return-producer-review'}:
                diagnostics.append(dict(kind=declaration_reason, offset=start,
                    message='Lua multiple assignment has an unresolved producer or RHS grouping; review before admitting its unit count.'))
            if not declaration_count and _unknown_c_packed_declaration(text, language):
                diagnostics.append(dict(kind='unknown-c-declaration-type-review', offset=start,
                    message='Possible packed declaration uses an unresolved lowercase type; verify its typedef and scope before assigning declaration units.'))
            if language == 'go':
                field_index = start + len(text) - len(text.lstrip())
                field_count, field_reason = _go_field_bindings(text, cleaned, effective_contexts[field_index])
                if field_count > declaration_count:
                    declaration_count, declaration_reason = field_count, field_reason
            # Chained assignments plus an uninitialized declarator are additive:
            # `int a=b=0,c;` has three bindings, not max(2 declarations,2 '=')=2.
            uninitialized=(proven_declaration['uninitialized'] if proven_declaration is not None
                           else _uninitialized_declarators(text,language) if declaration_count else 0)
            units = max(1, len(assignments)+uninitialized, declaration_count)
            if units > 1 or separators:
                events[line_no].append(dict(start_column=left+1, end_column=right+1,
                    units=units, assignments=assignments,
                    declaration_bindings=declaration_count,
                    reason=declaration_reason or "independent-statement-and-chained-bindings"))
            else:
                events[line_no].append(dict(start_column=left+1, end_column=right+1, units=1))
        # A semicolon inside a non-call grouping can be sequencing or tuple/data
        # syntax depending on the language. Surface it for explicit review.
        for i, char in enumerate(line):
            if char == ";" and language in {"julia", "ruby", "perl"}:
                relevant = effective_contexts[offset+i]
                if relevant and relevant[-1][1] == "group":
                    diagnostics.append(dict(kind="parenthesized-semicolon-review", offset=offset+i))
    expansions = [];canonical_total=0
    for line_no, line_events in events.items():
        header_close=canonical_by_line.get(line_no,[])
        units=sum(e["units"] for e in line_events)+sum(e["units"] for e in header_close)
        if not units and line_no not in owned_lines:
            units=1 # retain a standalone literal/data delimiter or wrapped row
        canonical_total+=units
        if units!=1:
            expansions.append(dict(line=line_no, extra_lines=units-1, units=units,
                original=original_lines[line_no-1], events=line_events,
                canonical_events=header_close,
                reason="Canonical header/open/close units and independent body bindings; retain literal wrapping."))
    # Exact-offset prefix probes can discover fresh macro/type-use gates.
    # Retain those emitted after the environment's initial collection too.
    if type_environment is not None:
        diagnostics.extend(item for item in type_environment['diagnostics']
                           if item not in diagnostics)
    for item in diagnostics:
        if "offset" in item:
            item["line"], item["column"] = location(item["offset"])
    return dict(rule_version=RULE_VERSION, physical_loc=len(physical_lines),
        normalized_loc=canonical_total,
        expansions=expansions, diagnostics=diagnostics,
        canonical_events=canonical["events"],
        value_form_evidence=values["evidence"],
        declaration_field_evidence=fields['evidence'],
        compiler_attribute_evidence=annotations['evidence'],
        cpp_initializer_evidence=initializers['evidence'],
        lexical_adjustments=masked.get('lexical_adjustments',[]),
        semantic_imports=masked.get('semantic_imports',[]),
        type_environment_evidence=type_environment['evidence'] if type_environment is not None else [],
        wrapped_declaration_review=wrapped_declarations,
        review_status="needs-review" if diagnostics else "candidate-parser-pass")
