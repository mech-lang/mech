"""Unexecuted bounded Gleam1.19 original-source reader/semantic owner proposal.

No other language lexer/classifier is selected. This exposes nested original
owners for independent review, never qualified LOC or chart admission by itself.
Unsupported/malformed forms diagnose; offsets and comment/literal spans survive.
"""
import re

OPERATORS = ('>=.', '<=.', '==', '!=', '>=', '<=', '&&', '||', '|>', '->', '..',
             '+.', '-.', '*.', '/.', '<.', '>.', '<>', '<<', '>>')
PRECEDENCE = {'|>': 1, '||': 2, '&&': 3, '==': 4, '!=': 4, '<': 4, '>': 4,
              '<=': 4, '>=': 4, '<.': 4, '>.': 4, '<=.': 4, '>=.': 4,
              '<>': 5, '+': 5, '-': 5, '+.': 5, '-.': 5,
              '*': 6, '/': 6, '%': 6, '*.': 6, '/.': 6}
IDENTIFIER = re.compile(r'[A-Za-z_][A-Za-z_0-9]*')
NUMBER = re.compile(r'(?:0[xX][0-9a-fA-F](?:_?[0-9a-fA-F])*|0[bB][01](?:_?[01])*|0[oO][0-7](?:_?[0-7])*|[0-9](?:_?[0-9])*(?:\.[0-9](?:_?[0-9])*)?(?:[eE][+-]?[0-9](?:_?[0-9])*)?)')
VERSION = 'gleam-1.19-source-owner-candidate-v1'


class Unproved(Exception):
    def __init__(self, kind, offset, message):
        self.issue = dict(kind=kind, offset=offset, message=message)
        super().__init__(message)


def lex(source):
    tokens, comments, literals = [], [], []
    index = 0
    while index < len(source):
        if source[index].isspace(): index += 1; continue
        start = index
        if source.startswith('//', index):
            index = source.find('\n', index)
            if index < 0: index = len(source)
            comments.append(dict(start=start, end=index)); continue
        if source[index] == '"':
            index += 1
            while index < len(source) and source[index] != '"':
                if source[index] == '\\':
                    index += 1
                    if index >= len(source): raise Unproved('gleam-unclosed-string', start, 'String escape has no value.')
                    if source[index] not in '\\"nrtfu': raise Unproved('gleam-string-escape-review', index, 'Unknown string escape.')
                    if source[index] == 'u':
                        match = re.match(r'u\{[0-9a-fA-F]{1,6}\}', source[index:])
                        if not match: raise Unproved('gleam-string-escape-review', index, 'Unicode escape must use tagged brace syntax.')
                        scalar = int(match[0][2:-1], 16)
                        if scalar > 0x10ffff or 0xd800 <= scalar <= 0xdfff:
                            raise Unproved('gleam-string-escape-review', index, 'Unicode escape must denote a Unicode scalar.')
                        index += len(match[0]); continue
                index += 1
            if index >= len(source): raise Unproved('gleam-unclosed-string', start, 'String literal has no closing quote.')
            index += 1
            kind = 'string'; literals.append(dict(start=start, end=index))
        else:
            found = next((operator for operator in OPERATORS if source.startswith(operator, index)), None)
            match = IDENTIFIER.match(source, index) or NUMBER.match(source, index)
            if found:
                index += len(found); kind = 'symbol'
            elif match:
                index = match.end(); kind = 'number' if source[start].isdigit() else 'name'
                if kind == 'number' and index < len(source) and (source[index].isalpha() or source[index] == '_'):
                    raise Unproved('gleam-number-review', start, 'Malformed numeric suffix cannot become another statement.')
            elif source[index] in '{}()[]#@,:.=+-*/%<>!|':
                index += 1; kind = 'symbol'
            else:
                raise Unproved('gleam-unsupported-token', index, 'Token is outside the bounded tagged Gleam inventory.')
        tokens.append(dict(word=source[start:index], kind=kind, start=start, end=index))
    return dict(tokens=tokens, comment_spans=comments, literal_spans=literals)


class Reader:
    def __init__(self, source, tokens):
        self.source, self.tokens, self.index = source, tokens, 0
    def word(self, ahead=0):
        index = self.index + ahead
        return self.tokens[index]['word'] if index < len(self.tokens) else ''
    def token(self):
        if self.index >= len(self.tokens): self.fail('gleam-unexpected-eof', 'Required source owner is absent.')
        token = self.tokens[self.index]; self.index += 1; return token
    def fail(self, kind, message):
        offset = self.tokens[self.index]['start'] if self.index < len(self.tokens) else len(self.source)
        raise Unproved(kind, offset, message)
    def expect(self, word):
        if self.word() != word: self.fail('gleam-owner-syntax-review', 'Expected ' + word + ', found ' + repr(self.word()))
        return self.token()
    def node(self, kind, start, end, units=0, children=(), **fields):
        return dict(kind=kind, start=start, end=end, proposed_units=units, children=list(children), **fields)
    def name(self):
        token = self.token()
        if token['kind'] != 'name': self.fail('gleam-name-review', 'Owner must have an explicit name.')
        return token
    def type(self):
        start = self.index
        if self.word() == 'fn':
            self.token(); self.type_group(); self.expect('->'); self.type()
        elif self.word() == '#':
            self.token(); self.type_group()
        else:
            self.name()
            if self.word() == '.': self.token(); self.name()
            if self.word() == '(': self.type_group()
        return self.node('type-reference', self.tokens[start]['start'], self.tokens[self.index-1]['end'])
    def type_group(self):
        self.expect('(')
        while self.word() != ')':
            self.type()
            if self.word() != ',': break
            self.token()
        self.expect(')')
    def formals(self):
        self.expect('(')
        result = []
        while self.word() != ')':
            first = self.name()
            if self.word() not in {':', ',', ')'}: self.fail('gleam-formal-review', 'Unproved labeled/default formal.')
            annotation = None
            if self.word() == ':': self.token(); annotation = self.type()
            result.append(self.node('formal', first['start'], self.tokens[self.index-1]['end'], annotation=annotation))
            if self.word() != ',': break
            self.token()
        self.expect(')'); return result
    def block(self):
        opening = self.expect('{'); statements = []
        while self.word() != '}':
            if not self.word(): self.fail('gleam-unclosed-block', 'Block owner has no closing brace.')
            if self.word() in {'use', 'assert'}:
                self.fail('gleam-unsupported-statement-owner', 'use/assert callback ownership needs a new bounded proposal.')
            if self.word() == 'let':
                first = self.token(); asserted = False
                if self.word() == 'assert': self.token(); asserted = True
                pattern = self.pattern({'=', ':'})
                annotation = None
                if self.word() == ':': self.token(); annotation = self.type()
                equals = self.expect('='); expression = self.expression()
                statements.append(self.node('binding', first['start'], expression['end'], 1,
                    children=[expression], header_end=equals['end'], pattern=pattern,
                    asserted=asserted, annotation=annotation))
            else:
                expression = self.expression()
                statements.append(self.node('expression-statement', expression['start'], expression['end'],
                    0 if expression['kind'] in {'case', 'anonymous-function', 'block'} else 1,
                    children=[expression]))
        closing = self.expect('}')
        return self.node('block', opening['start'], closing['end'], children=statements,
            opening_end=opening['end'], close_start=closing['start'], real_statement_count=len(statements))
    def pattern(self, stops):
        first = self.index; stack = []
        while self.word():
            word = self.word()
            if not stack and word in stops: break
            previous = self.tokens[self.index-1] if self.index > first else None
            if word == '(' and (previous is None or previous['word'] != '#' and not re.match(r'^[A-Z]', previous['word'])):
                self.fail('gleam-pattern-call-review', 'Only written tuple/constructor patterns own parentheses; lowercase calls are executable.')
            if word == '.' and not re.match(r'^[A-Z]', self.word(1)):
                self.fail('gleam-pattern-form-review', 'Qualified pattern must name a constructor, not a value projection.')
            if not stack and (word == ',' and '->' not in stops or word == '|' and '->' not in stops):
                self.fail('gleam-pattern-form-review', 'Bare multiple binding/alternative patterns have no assignment owner.')
            if word == 'as':
                self.fail('gleam-pattern-alias-review', 'Pattern alias ownership is outside the selected source inventory.')
            if previous is not None and previous['kind'] in {'name', 'number', 'string'} and self.tokens[self.index]['kind'] in {'name', 'number', 'string'}:
                self.fail('gleam-pattern-form-review', 'Adjacent pattern values cannot hide an extra binding owner.')
            token = self.token()
            if word in {'(', '['}: stack.append(word)
            elif word in {')', ']'}:
                if not stack or stack.pop() != {')':'(', ']':'['}[word]: self.fail('gleam-pattern-delimiter-review', 'Pattern delimiters differ.')
            elif word in {'{', '}', 'fn', 'case', 'let', 'use', '=', '@', '<-', '|>'}:
                self.fail('gleam-executable-pattern-review', 'Pattern cannot hide executable/binding bodies.')
            elif token['kind'] not in {'name', 'number', 'string'} and word not in {'#', '..', ',', '|', '.', '-', ':'}:
                self.fail('gleam-pattern-form-review', 'Pattern token is outside the selected constructor/list/tuple inventory.')
        if self.index == first or stack: self.fail('gleam-pattern-owner-review', 'Pattern is empty or unclosed.')
        return self.node('pattern', self.tokens[first]['start'], self.tokens[self.index-1]['end'])
    def expression(self, minimum=0):
        first = self.token(); word = first['word']
        if word in {'-', '!'}:
            right = self.expression(7); node = self.node('unary', first['start'], right['end'], children=[right], operator=word)
        elif word == 'case':
            subjects = [self.expression()]
            while self.word() == ',': self.token(); subjects.append(self.expression())
            opening = self.expect('{'); arms = []
            while self.word() != '}':
                pattern = self.pattern({'->', 'if'})
                if self.word() == 'if': self.fail('gleam-case-guard-review', 'Guard expression ownership needs independent review.')
                arrow = self.expect('->'); body = self.expression()
                arms.append(self.node('case-arm', pattern['start'], body['end'], 1,
                    children=[body], header_end=arrow['end'], pattern=pattern,
                    body_expression_units=0 if body['kind'] in {'case', 'anonymous-function', 'block'} else 1))
            closing = self.expect('}')
            node = self.node('case', first['start'], closing['end'], 2, subjects + arms,
                header_end=opening['end'], close_start=closing['start'])
        elif word == 'fn':
            formals = self.formals()
            if self.word() == '->': self.token(); self.type()
            body = self.block()
            node = self.node('anonymous-function', first['start'], body['end'], 2, [body],
                             header_end=body['opening_end'], formals=formals)
        elif word == '{':
            self.index -= 1; node = self.block()
        elif word in {'[', '#', '<<'}:
            closing = {'[':']', '#':')', '<<':'>>'}[word]
            if word == '#': self.expect('(')
            values = []
            while self.word() != closing:
                spread = self.word() == '..'
                if spread: self.token()
                value = self.expression(); modifier_name = None
                if word == '<<' and self.word() == ':':
                    self.token(); modifier = self.name()
                    if modifier['word'] not in {'utf8', 'bytes'}: self.fail('gleam-bit-segment-review', 'Unimplemented BitArray segment modifier.')
                    if self.word() not in {',', closing}: self.fail('gleam-bit-segment-review', 'Additional segment ownership is unproved.')
                    modifier_name = modifier['word']
                if word == '<<':
                    values.append(self.node('bit-segment', value['start'], self.tokens[self.index-1]['end'],
                                            children=[value], modifier=modifier_name))
                else:
                    values.append(value)
                if self.word() != ',': break
                self.token()
            end = self.expect(closing)
            node = self.node({'[':'list', '#':'tuple', '<<':'bit-array'}[word], first['start'], end['end'], children=values,
                             structural_data=True)
        elif word in {'panic', 'todo'}:
            children = []
            if self.word() == 'as': self.token(); children.append(self.expression())
            node = self.node('abort-expression', first['start'], self.tokens[self.index-1]['end'], children=children)
        elif first['kind'] in {'name', 'number', 'string'} and word not in {'let', 'use', 'import', 'pub', 'type', 'const', 'if', 'else', 'assert'}:
            node = self.node('atom', first['start'], first['end'], token_kind=first['kind'], word=word)
        else:
            self.fail('gleam-expression-owner-review', 'Unproved expression prefix: ' + word)
        while True:
            word = self.word()
            if word == '(':
                self.token(); arguments = []
                while self.word() != ')':
                    argument_start = self.tokens[self.index]['start']
                    spread = self.word() == '..'; label = None
                    if spread: self.token()
                    if self.word(1) == ':': label = self.name()['word']; self.token()
                    value = self.expression()
                    arguments.append(self.node('argument', argument_start, value['end'],
                                               children=[value], label=label, spread=spread))
                    if self.word() != ',': break
                    self.token()
                closing = self.expect(')')
                node = self.node('call', node['start'], closing['end'], children=[node] + arguments,
                                 structural_arguments=True)
            elif word == '.':
                self.token(); field = self.token()
                if field['kind'] not in {'name', 'number'}: self.fail('gleam-field-review', 'Field/projection has no owner.')
                node = self.node('field', node['start'], field['end'], children=[node], field=field['word'])
            elif word in PRECEDENCE and PRECEDENCE[word] >= minimum:
                operator = self.token(); right = self.expression(PRECEDENCE[word] + 1)
                node = self.node('binary', node['start'], right['end'], children=[node, right], operator=operator['word'])
            else:
                break
        return node
    def declaration(self):
        first = self.index; external = False; annotations = []
        while self.word() == '@':
            start = self.token(); name = self.name()
            if name['word'] != 'external': self.fail('gleam-annotation-review', 'Annotation requires its own qualified semantic owner.')
            self.expect('('); backend = self.name(); self.expect(','); module = self.token(); self.expect(','); function = self.token(); end = self.expect(')')
            if backend['word'] != 'erlang' or module['kind'] != 'string' or function['kind'] != 'string':
                self.fail('gleam-external-abi-review', 'Selected external declaration requires erlang and two literal names.')
            annotations.append(self.node('external-annotation', start['start'], end['end'], 1)); external = True
        if self.word() == 'pub': self.token()
        keyword = self.token()
        if keyword['word'] == 'import':
            self.name()
            while self.word() == '/': self.token(); self.name()
            if self.word() == '.':
                self.token(); self.expect('{')
                while self.word() != '}':
                    self.name()
                    if self.word() == 'as': self.token(); self.name()
                    if self.word() != ',': break
                    self.token()
                self.expect('}')
            if self.word() == 'as': self.token(); self.name()
            return self.node('import', self.tokens[first]['start'], self.tokens[self.index-1]['end'], 1)
        if keyword['word'] == 'fn':
            name = self.name(); formals = self.formals()
            if self.word() == '->': self.token(); self.type()
            if self.word() != '{':
                if not external: self.fail('gleam-bodyless-function-review', 'Bodyless function requires the selected explicit external owner.')
                return self.node('external-function', keyword['start'], self.tokens[self.index-1]['end'], 1, annotations, name=name['word'], formals=formals)
            if external: self.fail('gleam-external-body-review', 'External-plus-fallback body needs a new source-owner proposal.')
            body = self.block()
            return self.node('function', self.tokens[first]['start'], body['end'], 2, [body],
                             name=name['word'], header_end=body['opening_end'], formals=formals)
        if external: self.fail('gleam-annotation-target-review', 'External annotation belongs to an actual function declaration.')
        if keyword['word'] == 'type':
            name = self.name()
            if self.word() == '(': self.type_group()
            children = []
            if self.word() == '{':
                self.token()
                while self.word() != '}':
                    constructor = self.name(); fields = []
                    if self.word() == '(':
                        self.token()
                        while self.word() != ')':
                            field_start = self.tokens[self.index]['start']; label = None
                            if self.word(1) == ':': label = self.name()['word']; self.token()
                            field_type = self.type()
                            fields.append(self.node('constructor-field', field_start, field_type['end'],
                                                    children=[field_type], name=label))
                            if self.word() != ',': break
                            self.token()
                        self.expect(')')
                    children.append(self.node('type-constructor', constructor['start'], self.tokens[self.index-1]['end'], 1,
                                              children=fields, formals_structural=True))
                self.expect('}')
                units = 2
            else:
                units = 1
            return self.node('type-declaration', self.tokens[first]['start'], self.tokens[self.index-1]['end'], units, children,
                             name=name['word'])
        if keyword['word'] == 'const':
            self.name()
            if self.word() == ':': self.token(); self.type()
            self.expect('='); value = self.expression()
            if not literal_constant(value): self.fail('gleam-executable-const-review', 'Constant/data owner cannot contain executable calls/bodies.')
            return self.node('constant', self.tokens[first]['start'], value['end'], 1, [value])
        self.fail('gleam-top-level-owner-review', 'Unsupported top-level declaration: ' + keyword['word'])


def contains_owner(node):
    return node['kind'] in {'case', 'anonymous-function', 'block', 'binding'} or any(contains_owner(child) for child in node['children'])


def literal_constant(node):
    if node['kind'] == 'atom':
        return node['token_kind'] in {'number', 'string'} or node['word'] in {'True', 'False', 'Nil'}
    return node['kind'] in {'list', 'tuple', 'bit-array', 'bit-segment', 'unary'} and all(literal_constant(child) for child in node['children'])


def walk_owners(node):
    """Expose every nested written owner; it is not a flattened LOC counter."""
    yield node
    for child in node['children']:
        yield from walk_owners(child)


def parse_source(source):
    lexical = None; declarations = []
    try:
        lexical = lex(source); reader = Reader(source, lexical['tokens'])
        while reader.word(): declarations.append(reader.declaration())
        diagnostics = []
    except Unproved as error:
        diagnostics = [error.issue]
    return dict(status='source-owner-proposal-awaiting-grammar-and-semantic-review' if not diagnostics else 'unqualified-diagnostics',
        version=VERSION, declarations=declarations, diagnostics=diagnostics,
        comment_spans=lexical['comment_spans'] if lexical else [],
        literal_spans=lexical['literal_spans'] if lexical else [],
        canonical_loc=None, chart_admission=False, fixtures_executed=False,
        literal_data_wrapping_preserved_in_future_adapter=True)
