"""Unhooked C3 0.8.5 canonical source units for the authored CLBG suite.

C3 owns masking, fn/alias/struct/control/prototype/formal/type/value scopes.
Only approved language-neutral owner accounting is reused from foreign_core;
C/C++ lexers and classifiers are never selected. Execution requires the mutex.
Unsupported compile-time/macro/contract/asm/interface forms retain diagnostics.
"""
import re
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parent.parent
if str(ROOT) not in sys.path: sys.path.append(str(ROOT))
from foreign_core import brace_pairs, count_owned, replacement, structural, top_parts
from c3_mask import mask_c3, issue

VERSION = 'c3-source-owners-0.8.5-private-v2'
BUILTIN_TYPES = {'void', 'bool', 'char', 'ichar', 'short', 'ushort', 'int', 'uint', 'long', 'ulong',
                 'int128', 'uint128', 'float', 'double', 'float16', 'float128', 'sz', 'usz', 'iptr', 'uptr', 'any', 'typeid', 'fault'}
UNSUPPORTED = {'asm', 'macro', 'attrmacro', 'interface', 'bitstruct', 'enum', 'constset', 'excuse', 'faultdef', 'attrdef', 'constdef'}
ATTRIBUTES = {'cname', 'cdecl', 'inline', 'noinline', 'local', 'private', 'public', 'export', 'maydiscard', 'nodiscard', 'align', 'packed', 'noreturn'}
IDENT = r'[A-Za-z_][A-Za-z_0-9]*'
TOKEN = re.compile(r'::|=>|\.\.\.|\.\.|<<=|>>=|[+*/%&|^-]=|==|!=|<=|>=|&&|\|\||\+\+|--|<<|>>|' + IDENT + r'|[0-9]+(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?[a-zA-Z0-9_]*|[^\s]')
ASSIGN = re.compile(r'(?<![=!<>~:])(?:<<=|>>=|[+*/%&|^-]=|=(?!=|>))')


def _named_type(word):
    # Tagged C3 lexing separates lowercase values, uppercase constants, and
    # uppercase-initial type names that contain at least one lowercase letter.
    return bool(re.fullmatch(r'_*(?=[A-Z])[A-Za-z_0-9]*[a-z][A-Za-z_0-9]*', word))


def _type(text):
    value = text.strip()
    match = re.match(r'(?:(?:[a-z_][a-z_0-9]*::)*)(' + IDENT + r')', value)
    if not match or match[1] not in BUILTIN_TYPES and not _named_type(match[1]): return False
    suffix = value[match.end():].strip()
    while suffix:
        if suffix.startswith(('*', '?')): suffix = suffix[1:].lstrip(); continue
        if suffix.startswith('['):
            pairs, diagnostics = brace_pairs(suffix)
            close = pairs.get(0)
            if diagnostics or close is None: return False
            dimension = suffix[1:close].strip()
            if dimension and (ASSIGN.search(dimension) or any(char in dimension for char in '{};')): return False
            suffix = suffix[close+1:].lstrip(); continue
        if suffix.startswith('{'):
            pairs, diagnostics = brace_pairs(suffix); close = pairs.get(0)
            if diagnostics or close is None: return False
            if not all(_type(part) or re.fullmatch(r'[0-9]+|[A-Z_][A-Z_0-9]*', part) for part in top_parts(suffix[1:close])): return False
            suffix = suffix[close+1:].lstrip(); continue
        return False
    return True


def _without_attributes(text):
    return re.sub(r'@' + IDENT + r'(?:\s*\([^()]*\))?', '', text).strip()


def _declarators(text):
    value = _without_attributes(text.strip())
    value = re.sub(r'^(?:extern|const|static|tlocal)\s+', '', value)
    parts = top_parts(value)
    first = parts[0]
    # C3 places pointer/array/slice suffixes on the type, before the name.
    match = re.match(r'(.+?(?:\s|\*|\]|\?|\}))([a-z_][a-z_0-9]*|[A-Z_][A-Z_0-9]*)\s*(?:=(?!=|>)(.*))?$', first, re.S)
    if not match or not _type(match[1]): return None
    if match[3]:
        # An initializer cannot swallow a second typed owner when its required
        # semicolon is absent. Keep ordinary casts/calls/record literals valid.
        for hidden in re.finditer(r'(?<![A-Za-z_0-9])(' + IDENT + r'(?:\s*[*?])*)\s+' + IDENT + r'\s*=', match[3]):
            if _type(hidden[1]): return None
        for line in match[3].splitlines()[1:]:
            if line.strip() and _declarators(line.strip()) is not None: return None
    result = [match[2]]
    for part in parts[1:]:
        member = re.fullmatch(r'([a-z_][a-z_0-9]*|[A-Z_][A-Z_0-9]*)(?:\s*=(?!=|>).*|\s*)', part, re.S)
        if not member: return None
        result.append(member[1])
    return result


def _scalar_arrow(body, return_type, formals):
    """Prove bounded numeric/boolean formal arithmetic, never call-name purity."""
    scalar_types = BUILTIN_TYPES - {'void', 'any', 'fault', 'typeid'}
    if return_type.strip() not in scalar_types: return False
    names = set()
    for formal in top_parts(formals):
        if not formal: continue
        match = re.fullmatch(r'(' + '|'.join(sorted(scalar_types)) + r')\s+([a-z_][a-z_0-9]*)', formal)
        if not match: return False
        names.add(match[2])
    if ASSIGN.search(body) or re.search(r'\+\+|--|=>', body): return False
    tokens = [match[0] for match in TOKEN.finditer(body)]
    allowed_operators = {'+', '-', '*', '/', '%', '~', '&', '|', '^', '!', '&&', '||', '==', '!=', '<', '>', '<=', '>=', '<<', '>>', '(', ')', '?', ':'}
    for index, word in enumerate(tokens):
        if word in names | {'true', 'false'} | scalar_types:
            # A value followed by '(' is invocation, not arithmetic grouping.
            if index + 1 < len(tokens) and tokens[index + 1] == '(': return False
            continue
        if re.fullmatch(r'[0-9]+(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?[a-zA-Z0-9_]*', word): continue
        if word not in allowed_operators: return False
    return bool(tokens)


def _initializer_field_assignment(text, offset):
    """Prove a dot-designated field directly inside initializer braces."""
    if not re.search(r'(?:^|[{,])\s*\.' + IDENT + r'\s*$', text[:offset]): return False
    stack = []
    for char in text[:offset]:
        if char in '([{': stack.append(char)
        elif char in ')]}' and stack: stack.pop()
    return bool(stack and stack[-1] == '{')


class Parser:
    def __init__(self, text, literals=()):
        self.text = text
        self.tokens = [dict(word=match[0], start=match.start(), end=match.end()) for match in TOKEN.finditer(text)]
        self.index = 0; self.events = []; self.diagnostics = []; self.scopes = []; self.evidence = []
        char_pairs, issues = brace_pairs(text); self.diagnostics.extend(issues)
        offsets = {token['start']: index for index, token in enumerate(self.tokens)}
        self.pairs = {offsets[opening]: offsets[closing] for opening, closing in char_pairs.items() if opening in offsets and closing in offsets}
        self.literals = literals
        self.block_starts = []
        self.scalar_switch_blocks = set()

    def word(self, ahead=0):
        index = self.index + ahead
        return self.tokens[index]['word'] if index < len(self.tokens) else ''

    def event(self, kind, first, last, units=1, scope='c3-owned'):
        start, end = self.tokens[first]['start'], self.tokens[last]['end']
        self.events.append(dict(kind=kind, start=start, end=end, units=units, scope=scope))

    def gate(self, kind, index, message):
        offset = self.tokens[index]['start'] if index < len(self.tokens) else len(self.text)
        self.diagnostics.append(issue(kind, offset, message))

    def group(self, kind):
        opening = self.index; closing = self.pairs.get(opening)
        if closing is None:
            self.gate('c3-unclosed-group-review', opening, 'Delimiter group has no proved matching close.'); self.index += 1; return opening
        self.scopes.append((self.tokens[opening]['start'], self.tokens[closing]['start'], kind))
        if kind in {'args', 'parameters', 'data'}:
            inner = self.text[self.tokens[opening]['end']:self.tokens[closing]['start']]
            if ';' in inner or re.search(r'\b(?:fn|macro|defer|for|while)\b|=>', inner):
                self.gate('c3-executable-data-owner-review' if kind == 'data' else 'c3-executable-call-argument-review', opening,
                          'Structural composition contains an unproved executable body or statement sequence.')
            if kind in {'data', 'args'} and re.search(r'\+\+|--', inner):
                self.gate('c3-data-owner-mutation-review' if kind == 'data' else 'c3-call-argument-mutation-review', opening,
                          'Index/initializer/argument mutation cannot be hidden as structural composition.')
            if kind in {'args', 'data'}:
                for assigned in ASSIGN.finditer(inner):
                    # For a brace group the current outer brace is implicit in
                    # inner. Array indices never receive that initializer owner.
                    owned_text = ('{' + inner) if kind == 'data' and self.tokens[opening]['word'] == '{' else inner
                    position = assigned.start() + (len(owned_text) - len(inner))
                    if _initializer_field_assignment(owned_text, position):
                        continue
                    self.gate('c3-data-owner-write-review' if kind == 'data' else 'c3-call-argument-write-review', opening,
                              'Written assignment is an effect unless a direct initializer field is proved; named C3 arguments use colon.')
        self.index = closing + 1
        return closing

    def attributes(self):
        while self.word() == '@':
            start = self.index; self.index += 1
            name = self.word(); self.index += bool(name)
            if name not in ATTRIBUTES: self.gate('c3-attribute-review', start, 'Attribute is outside the qualified authored-suite inventory.')
            if self.word() == '(': self.group('args')

    def simple(self, kind=None):
        first = self.index; reached = False
        while self.index < len(self.tokens):
            word = self.word()
            if word == ';': self.index += 1; reached = True; break
            if word in {'(', '[', '{'}:
                if word == '{':
                    before = self.tokens[self.index-1]['word'] if self.index else ''
                    prefix = self.text[self.tokens[first]['start']:self.tokens[self.index]['start']]
                    if ASSIGN.search(prefix) or before in {'=', ',', '{'} or _named_type(before): self.group('data'); continue
                    self.gate('c3-unproved-expression-block-review', self.index, 'Expression brace has no proved initializer/type-argument owner.'); self.group('data'); continue
                self.group('args' if word == '(' else 'data'); continue
            if word == '}': break
            if word in UNSUPPORTED: self.gate('c3-extended-declaration-review', self.index, 'Extended declaration has no admitted C3 source owner.')
            if word == '@': self.attributes(); continue
            self.index += 1
        if not reached: self.gate('c3-statement-terminator-review', first, 'Statement has no written semicolon; no newline alias is assumed.')
        last = max(first, self.index-1)
        if kind: self.event(kind, first, last)
        return first, last

    def block(self, first, kind, prefix_end=None):
        opening = self.index; close = self.pairs.get(opening)
        if close is None: self.gate('c3-unclosed-block-review', opening, 'Executable/type block has no written close.'); self.index += 1; return
        self.scopes.append((self.tokens[opening]['start'], self.tokens[close]['start'], kind))
        if prefix_end is None and kind == 'bare-code' and first == opening:
            self.event('open', opening, opening, units=0, scope='c3-written-open')
        elif prefix_end is None: self.event('header', first, opening, scope='c3-' + kind + '-header')
        else:
            self.event('header', first, prefix_end, scope='c3-' + kind + '-header')
            self.event('open', opening, opening, units=0, scope='c3-written-open')
        self.index += 1
        self.block_starts.append(opening)
        while self.index < close:
            before = self.index; self.statement(type_scope=kind == 'type')
            if self.index <= before: self.gate('c3-parser-no-progress', before, 'Statement ownership failed to advance.'); self.index += 1
        self.block_starts.pop()
        self.event('close', close, close, scope='c3-written-close'); self.index = close + 1

    def function(self):
        first = self.index
        external = self.word() == 'extern'
        if external: self.index += 1
        self.index += 1  # fn
        header_first = self.index
        while self.index < len(self.tokens) and self.word() != '(':
            if self.word() in {'[', '{'}: self.group('data')
            else: self.index += 1
        formal = self.index
        if self.word() != '(':
            self.gate('c3-function-signature-review', first, 'fn has no proved formal group.'); return
        header = self.text[self.tokens[header_first]['start']:self.tokens[formal]['start']].strip()
        name = re.match(r'(.+?)\s+((?:' + IDENT + r'\.)?' + IDENT + r')$', header, re.S)
        if not name or not _type(name[1]): self.gate('c3-function-signature-review', first, 'Return type/function name require exact C3 ownership.')
        formal_close = self.group('parameters'); self.attributes()
        if self.word() == '{':
            if external: self.gate('c3-external-function-body-review', first, 'Tagged C3 extern functions must remain prototypes.')
            self.block(first, 'code', prefix_end=self.index-1); return
        if self.word() == ';': self.event('header', first, self.index, scope='c3-fn-prototype'); self.index += 1; return
        if self.word() == '=>':
            if external: self.gate('c3-external-function-body-review', first, 'Tagged C3 extern functions must remain prototypes.')
            arrow = self.index; self.index += 1; body_first = self.index
            _, last = self.simple()
            body = self.text[self.tokens[body_first]['start']:self.tokens[last]['start']].strip()
            formal_text = self.text[self.tokens[formal]['end']:self.tokens[formal_close]['start']]
            return_type = name[1] if name else ''
            if not _scalar_arrow(body, return_type, formal_text):
                self.event('header', first, arrow, scope='c3-imperative-arrow-header')
                self.gate('c3-imperative-arrow-review', arrow, 'Only proven builtin scalar formal arithmetic is composed; calls/void/custom operands/side effects need separate body semantic review.')
            else: self.event('value-expression', first, last, scope='value-expression')
            return
        self.gate('c3-function-body-review', self.index, 'fn body is neither a block, scalar arrow nor prototype.')
        self.simple()

    def scalar_case(self, first, last):
        if last < first: return False
        start, end = self.tokens[first]['start'], self.tokens[last]['end']
        text = self.text[start:end].strip()
        literals = [item for item in self.literals if item['start'] < end and start < item['end']]
        if literals:
            return len(literals) == 1 and literals[0]['start'] == start and literals[0]['end'] == end and literals[0]['complete'] and literals[0]['delimiter'] == "'"
        return bool(re.fullmatch(r'[+-]?(?:0[xX][0-9a-fA-F_]+|[0-9][0-9_]*)(?:[uUlL]{0,3})|true|false|(?:[a-z_][a-z_0-9]*::)*[A-Z_][A-Z_0-9]*', text))

    def switch_arm(self):
        first = self.index; word = self.word(); self.index += 1
        value_first = self.index
        while self.index < len(self.tokens) and self.word() not in {':', ';', '{', '}'}:
            if self.word() in {'(', '['}: self.group('control')
            else: self.index += 1
        value_last = self.index - 1
        owned = bool(self.block_starts and self.block_starts[-1] in self.scalar_switch_blocks)
        value_ok = self.scalar_case(value_first, value_last) if word == 'case' else value_last < value_first
        if not owned or not value_ok:
            self.gate('c3-switch-arm-review', first, 'Only direct switch scalar literal/constant arms and plain default have a proved owner; range/type/dynamic arms require review.')
        if self.word() == ':':
            self.event('branch', first, self.index, scope='c3-scalar-switch-arm'); self.index += 1
        else:
            self.gate('c3-switch-arm-review', first, 'Scalar switch arm has no written top-level colon.')
            self.simple()

    def statement(self, type_scope=False):
        word = self.word(); first = self.index
        if not word: return
        if word == ';': self.index += 1; return
        if word == 'extern' and self.word(1) == 'fn' or word == 'fn': self.function(); return
        if word == 'extern':
            _, last = self.simple()
            text = self.text[self.tokens[first]['start']:self.tokens[last]['start']].strip()
            names = _declarators(text)
            if not names or len(names) != 1 or ASSIGN.search(text):
                self.gate('c3-external-variable-owner-review', first, 'Only one typed external variable with no initializer is in the selected ABI inventory.')
            else:
                self.event('header', first, last, scope='c3-external-variable')
            return
        if word in {'case', 'default'}: self.switch_arm(); return
        if word in {'module', 'import', 'alias', 'typedef', 'def'}:
            _, last = self.simple('header')
            code = self.text[self.tokens[first]['start']:self.tokens[last]['start']].strip()
            if word in {'alias', 'typedef', 'def'}:
                if word == 'def': self.gate('c3-legacy-alias-review', first, 'Legacy def is outside the selected 0.8.5 authored syntax.')
                match = re.fullmatch(r'(?:alias|typedef|def)\s+(' + IDENT + r')\s*=\s*(.*)', code, re.S)
                rhs = _without_attributes(match[2]) if match else ''
                signature = re.fullmatch(r'fn\s+(.+?)\((.*)\)', rhs, re.S)
                signature_ok = bool(signature and _type(signature[1]) and not any(char in signature[2] for char in '{};'))
                if not match or not (_type(rhs.removeprefix('inline ').strip()) or signature_ok):
                    self.gate('c3-alias-owner-review', first, 'Alias RHS is neither a proved type/generic type nor function-pointer type; executable bodies are never hidden.')
            return
        if word in {'struct', 'union'}:
            self.index += 1
            if self.word() == '{' and not type_scope: self.gate('c3-type-name-review', self.index, 'Top-level type cannot silently omit its tagged type name.')
            if self.word() != '{':
                if not type_scope and not _named_type(self.word()): self.gate('c3-type-name-review', self.index, 'Top-level struct/union requires a tagged C3 type identifier.')
                self.index += 1
            self.attributes()
            if self.word() == '{': self.block(first, 'type'); return
            self.gate('c3-type-header-review', first, 'Struct/union name/body ownership is not proved.'); self.simple(); return
        if word in {'if', 'while', 'for', 'foreach', 'foreach_r', 'switch'}:
            self.index += 1
            # Authored suite uses no loop/switch labels or try/catch conditions.
            if self.word() != '(':
                self.gate('c3-control-header-review', first, 'Control requires its parenthesized authored-suite header.'); self.simple(); return
            opening = self.index; close = self.group('for' if word in {'for', 'foreach', 'foreach_r'} else 'control')
            header = self.text[self.tokens[opening]['end']:self.tokens[close]['start']]
            if re.search(r'\b(?:try|catch)\b', header): self.gate('c3-condition-binding-review', opening, 'Optional unwrapping condition has additional bindings requiring semantic review.')
            if word in {'foreach', 'foreach_r'} and ':' not in header: self.gate('c3-foreach-header-review', opening, 'Foreach has no index/value iterable colon.')
            if self.word() == '{':
                if word == 'switch': self.scalar_switch_blocks.add(self.index)
                self.block(first, 'code', prefix_end=close)
            else:
                if word == 'switch': self.gate('c3-switch-arm-review', first, 'Selected scalar switch requires a written block.')
                if word in {'if', 'while', 'for'} and self.index < len(self.tokens) and '\n' in self.text[self.tokens[close]['end']:self.tokens[self.index]['start']]:
                    self.gate('c3-unbraced-control-layout-review', self.index, 'Tagged C3 requires this unbraced control body on the closing-header line.')
                self.event('header', first, close, scope='c3-unbraced-control'); self.statement()
            if word == 'if' and self.word() == 'else':
                branch = self.index; self.index += 1
                if self.word() == '{': self.block(branch, 'code')
                elif self.word() == 'if':
                    event_start = len(self.events)
                    self.statement()
                    header_event = next((event for event in self.events[event_start:] if event['kind'] == 'header'), None)
                    if header_event: header_event['start'] = self.tokens[branch]['start']; header_event['scope'] = 'c3-else-if-header'
                    else: self.gate('c3-else-if-owner-review', branch, 'else-if has no combined proved branch header.')
                else: self.event('branch', branch, branch, scope='c3-else'); self.statement()
            return
        if word == 'defer':
            self.index += 1
            if self.word() == '{': self.block(first, 'code'); return
            if self.word() in {'try', 'catch', '(', 'if', 'for', 'while', 'defer'}:
                self.gate('c3-extended-defer-review', first, 'Conditional/binding/compound deferred control has no scalar-call owner.')
            body_start, last = self.simple()
            body = self.text[self.tokens[body_start]['start']:self.tokens[last]['start']].strip()
            call = re.match(r'(?:' + IDENT + r'::)*(?:' + IDENT + r'\.)*' + IDENT + r'\s*\(', body)
            pairs, diagnostics = brace_pairs(body)
            opening = call.end() - 1 if call else None
            if not call or diagnostics or pairs.get(opening) != len(body) - 1:
                self.gate('c3-extended-defer-review', first, 'Only one proven deferred call composes; other executable deferred statements require review.')
            self.event('defer-call', first, last, scope='c3-simple-deferred-statement'); return
        if word == 'do':
            self.index += 1
            if self.word() == '{': self.block(first, 'code')
            else: self.gate('c3-do-body-review', first, 'do requires a written authored-suite block.'); self.statement()
            if self.word() == 'while':
                tail = self.index; self.index += 1
                if self.word() == '(': self.group('control')
                if self.word() == ';': self.event('branch', tail, self.index, scope='c3-do-while-tail'); self.index += 1
                else: self.gate('c3-do-tail-review', tail, 'do-while tail requires written semicolon.')
            elif self.word() == ';': self.index += 1
            return
        if word == '{': self.block(first, 'bare-code'); return
        if word in UNSUPPORTED or word in {'case', 'default', 'else'}:
            self.gate('c3-extended-statement-review', first, 'Written construct has no admitted C3 owner.')
        self.simple()

    def parse(self):
        while self.index < len(self.tokens):
            previous = self.index; self.statement()
            if self.index <= previous: self.gate('c3-parser-no-progress', previous, 'Top-level ownership did not advance.'); self.index += 1
        contexts = []
        for offset in range(len(self.text)):
            contexts.append(tuple((self.text[start], kind, start) for start, end, kind in self.scopes if start <= offset <= end))
        return dict(events=self.events, diagnostics=self.diagnostics, contexts=contexts, scopes=self.scopes)


def binding_counter(**kwargs):
    text = kwargs['text'].strip(); context = kwargs['context']
    names = _declarators(text)
    if names:
        if re.search(r'\n\s*(?:' + '|'.join(sorted(BUILTIN_TYPES)) + r')\s+[a-z_]', text):
            return dict(diagnostics=[dict(kind='c3-unproved-typed-declaration-review', message='Nested declaration lacks a written semicolon boundary.')])
        initialized = sum(bool(ASSIGN.search(part)) for part in top_parts(text))
        units = len(names) + max(0, len(kwargs['assignments']) - initialized)
        diagnostics = []
        if len(names) > 1 and initialized:
            diagnostics.append(dict(kind='c3-initialized-multi-declaration-review', message='Tagged C3 rejects initialized multiple declarations; independent declarations must retain written boundaries.'))
        return dict(units=units, declaration_bindings=len(names), diagnostics=diagnostics,
                    reason='C3 independent declared names plus additional chained writes')
    diagnostics = []
    if re.search(r'\n\s*(?:' + '|'.join(sorted(BUILTIN_TYPES)) + r')\s+[a-z_]', text):
        diagnostics.append(dict(kind='c3-unproved-typed-declaration-review', message='A nested typed declaration is not owned by a written statement boundary.'))
    if re.match(r'(?:const\s+)?' + IDENT + r'\s+' + IDENT + r'(?:\s*=|\s*,|\s*$)', text) and not re.match(r'(?:return|break|continue|nextcase)\b', text):
        diagnostics.append(dict(kind='c3-unproved-typed-declaration-review', message='Typed declaration cannot be proven from selected C3 grammar.'))
    # A comma outside data/formal/call/control owners is sequencing, not a
    # language-independent exemption. Expose it for conservative review.
    if len(top_parts(text)) > 1: diagnostics.append(dict(kind='c3-comma-expression-review', message='Top-level comma sequence requires exact independent operation review.'))
    if re.search(r'\b(?:try|catch|var|static|tlocal)\b', text) and names is None:
        diagnostics.append(dict(kind='c3-extended-binding-review', message='Unproved storage/compile-time/optional binding remains gated.'))
    return dict(diagnostics=diagnostics)


def normalize(source, language='c3'):
    assert language == 'c3'
    masked = mask_c3(source); parsed = Parser(masked['cleaned'], masked['literals']).parse()
    result = count_owned(source, masked['cleaned'], parsed['contexts'], parsed['events'],
                         masked['diagnostics'] + parsed['diagnostics'], 'c3', binding_counter=binding_counter,
                         assignment_pattern=ASSIGN)
    result.update(proposal_version=VERSION, compiler_version='0.8.5', proposal_only=True,
                  proposal_status='unhooked-no-admission', eligible_for_chart=False,
                  literal_spans=masked['literals'], scope_evidence=parsed['scopes'])
    return result
