"""Unexecuted, unhooked Clojure-owned reader and canonical owner proposal.

No reader evaluation, macro expansion, application imports, hashes or admission.
Import the separately pinned language-neutral accounting only during a reviewed,
mutex-held qualification. Unsupported forms retain explicit review diagnostics.
"""
from dataclasses import dataclass, field
import re

from foreign_core import count_owned, replacement

PROPOSAL_VERSION = 'foreign-clojure-reader-owners-20261012-v2'
RULE_VERSION = 'canonical-source-units-20261012-approved-rule-v2'
MATCH = {')': '(', ']': '[', '}': '{'}
KIND = {'(': 'list', '[': 'vector', '{': 'map'}
CORE_CALLS = set('''aclone aget alength aset aset-byte aset-char aset-double aset-int aset-long
assoc! bit-and bit-or bit-shift-left byte-array char char-array conj count dec double
double-array drop empty? even? first flush get inc int int-array long long-array map mapv
max min mod next nil? nth object-array peek persistent! pos? print println quot range
reduce reductions rem rest second seq sort-by str take to-array transient unchecked-byte
unsigned-bit-shift-right vals vec zero? zipmap + - * / < <= = > >= not and or assert
identity list vector hash-map set apply'''.split())
ALIASED_CALLS = {'s/argument', 's/input-text', 's/jobs', 's/printf', 'string/split'}
SCALAR_CALLS = set('+ - * / quot mod rem inc dec bit-and bit-or bit-shift-left unsigned-bit-shift-right < <= = > >= not and or zero? pos? even? int long double char'.split())
OWNERS = {'ns', 'def', 'defonce', 'defn', 'defn-', 'fn', 'let', 'loop', 'if', 'if-not',
          'if-let', 'when', 'when-not', 'when-let', 'do', 'doseq', 'dotimes', 'for',
          'cond', 'case', 'try', 'catch', 'finally', 'reify'}
UNSUPPORTED = {'defmacro', 'deftype', 'defrecord', 'defprotocol', 'extend-type',
    'extend-protocol', 'proxy', 'letfn', 'binding', 'with-open', 'with-redefs',
    'with-local-vars', 'locking', 'doto', '->', '->>', 'as->', 'some->', 'some->>',
    'cond->', 'cond->>', 'lazy-seq', 'future', 'comment'}


@dataclass
class Node:
    kind: str
    start: int
    end: int
    value: str = ''
    children: list = field(default_factory=list)


def bare(node):
    while node.kind == 'metadata':
        node = node.children[-1]
    return node


def head(node):
    node = bare(node)
    return bare(node.children[0]).value if node.kind == 'list' and node.children else ''


class Reader:
    def __init__(self, source):
        self.source, self.cursor = source, 0
        self.cleaned = list(source)
        self.diagnostics = []
        self.anonymous_depth = 0
        if source.startswith('\ufeff'):
            self.cleaned[0] = ' '
            self.cursor = 1

    def issue(self, kind, offset, detail):
        self.diagnostics.append(dict(kind=kind, offset=offset, detail=detail))

    def mask(self, start, end, literal=False):
        self.cleaned[start:end] = replacement(self.source[start:end], literal=literal)

    def space(self):
        while self.cursor < len(self.source):
            char = self.source[self.cursor]
            if char.isspace() or char == ',':
                self.cursor += 1
            elif char == ';':
                start = self.cursor
                newline = self.source.find('\n', start)
                self.cursor = len(self.source) if newline < 0 else newline
                self.mask(start, self.cursor)
            else:
                break

    def next_active(self):
        while True:
            self.space()
            if self.cursor >= len(self.source) or self.source[self.cursor] in MATCH:
                return None
            node = self.form()
            if node is not None:
                return node

    def quoted_string(self, start, kind):
        self.cursor += 1  # Opening quote, after an optional dispatch prefix.
        escaped = False
        while self.cursor < len(self.source):
            char = self.source[self.cursor]
            self.cursor += 1
            if escaped:
                escaped = False
            elif char == '\\':
                escaped = True
            elif char == '"':
                self.mask(start, self.cursor, literal=True)
                return Node(kind, start, self.cursor)
        self.issue('foreign-clojure-unclosed-literal', start, kind)
        self.mask(start, self.cursor, literal=True)
        return Node(kind, start, self.cursor)

    def collection(self, start, opening, kind):
        self.cursor += 1
        children = []
        while True:
            self.space()
            if self.cursor >= len(self.source):
                self.issue('foreign-clojure-unclosed-delimiter', start, opening)
                break
            char = self.source[self.cursor]
            if char in MATCH:
                self.cursor += 1
                if MATCH[char] != opening:
                    self.issue('foreign-clojure-delimiter-mismatch', self.cursor-1, opening + char)
                break
            node = self.form()
            if node is not None:
                children.append(node)
        if kind == 'map' and len(children) % 2:
            self.issue('foreign-clojure-map-pairs-review', start, 'Odd map literal inventory')
        return Node(kind, start, self.cursor, children=children)

    def prefix(self, start, width, kind):
        self.cursor += width
        child = self.next_active()
        if child is None:
            self.issue('foreign-clojure-reader-prefix-review', start, kind + ' has no following active form')
            return Node(kind, start, self.cursor)
        return Node(kind, start, child.end, children=[child])

    def form(self):
        self.space()
        start, char = self.cursor, self.source[self.cursor]
        if char in KIND:
            return self.collection(start, char, KIND[char])
        if char == '"':
            return self.quoted_string(start, 'string')
        if char == '\\':
            self.cursor += 1
            if self.cursor == len(self.source):
                self.issue('foreign-clojure-character-review', start, 'Missing character')
            else:
                # Delimiter characters and semicolons are one literal, never
                # reader punctuation or comment starts.
                self.cursor += 1
                if self.source[self.cursor-1].isalpha():
                    while self.cursor < len(self.source) and not self.source[self.cursor].isspace() and self.source[self.cursor] not in '()[]{}",;':
                        self.cursor += 1
            token = self.source[start+1:self.cursor]
            if not (len(token) == 1 or token in {'newline', 'space', 'tab', 'formfeed', 'backspace', 'return'} or
                    re.fullmatch(r'u[0-9a-fA-F]{4}|o[0-7]{1,3}', token)):
                self.issue('foreign-clojure-character-review', start, token)
            self.mask(start, self.cursor, literal=True)
            return Node('character', start, self.cursor)
        if char == '^':
            self.cursor += 1
            metadata = self.next_active()
            target = self.next_active() if metadata else None
            if metadata is None or target is None:
                self.issue('foreign-clojure-metadata-review', start, 'Incomplete metadata/target')
                return Node('invalid', start, self.cursor)
            if bare(metadata).kind not in {'atom', 'string', 'map', 'vector'}:
                self.issue('foreign-clojure-metadata-review', start, 'Unsupported metadata owner')
            return Node('metadata', start, target.end, children=[metadata, target])
        if char == "'":
            return self.prefix(start, 1, 'quote')
        if char == '@':
            return self.prefix(start, 1, 'deref')
        if char in '`~':
            width = 2 if self.source.startswith('~@', start) else 1
            self.issue('foreign-clojure-syntax-quote-review', start, 'Generated/unquoted scope is unsupported')
            return self.prefix(start, width, 'unsupported-prefix')
        if char == '#':
            following = self.source[start+1:start+2]
            if following == '_':
                self.cursor += 2
                child = self.next_active()
                if child is None:
                    self.issue('foreign-clojure-discard-review', start, 'Missing next active form')
                self.mask(start, self.cursor)
                return None
            if following == '"':
                self.cursor += 1
                return self.quoted_string(start, 'regex')
            if following == '{':
                self.cursor += 1
                return self.collection(start, '{', 'set')
            if following == '(':
                if self.anonymous_depth:
                    self.issue('foreign-clojure-nested-anonymous-review', start, 'Nested #() is invalid')
                self.anonymous_depth += 1
                self.cursor += 1
                child = self.collection(start+1, '(', 'list')
                self.anonymous_depth -= 1
                return Node('anonymous', start, child.end, children=[child])
            if following == "'":
                return self.prefix(start, 2, 'var-quote')
            if self.source.startswith(('##Inf', '##-Inf', '##NaN'), start):
                pass  # Symbolic numeric values are non-evaluating atoms.
            else:
                self.issue('foreign-clojure-dispatch-review', start,
                    'Tagged readers, reader evaluation, namespaced maps and reader conditionals are unsupported')
                # Retain the unrecognized dispatch as an atom and parse the
                # subsequent collection separately, without executing readers.
        while self.cursor < len(self.source) and not self.source[self.cursor].isspace() and self.source[self.cursor] not in '()[]{}",;':
            self.cursor += 1
        if self.cursor == start:
            self.cursor += 1
            self.issue('foreign-clojure-reader-token-review', start, char)
        return Node('atom', start, self.cursor, self.source[start:self.cursor])

    def read(self):
        result = []
        while True:
            self.space()
            if self.cursor == len(self.source):
                break
            if self.source[self.cursor] in MATCH:
                self.issue('foreign-clojure-orphan-close', self.cursor, self.source[self.cursor])
                self.cursor += 1
                continue
            node = self.form()
            if node is not None:
                result.append(node)
        return result


def names(node):
    node = bare(node)
    if node.kind == 'atom':
        return {node.value} - {'&', '_'}
    if node.kind == 'vector':
        return set().union(*(names(child) for child in node.children)) if node.children else set()
    return set()


class Owners:
    def __init__(self, source, cleaned, forms, diagnostics):
        self.source, self.cleaned, self.forms = source, cleaned, forms
        self.diagnostics, self.events = list(diagnostics), []
        self.functions = {bare(bare(node).children[1]).value for node in forms
            if head(node) in {'defn', 'defn-'} and len(bare(node).children) > 1}
        for form in forms:
            self.metadata_owners(form)

    def issue(self, kind, node, detail):
        self.diagnostics.append(dict(kind=kind, offset=node.start, detail=detail))

    def event(self, start, end, units, kind, scope):
        if end > start:
            self.events.append(dict(start=start, end=end, units=units, kind=kind, scope=scope))

    def scalar(self, node):
        node = bare(node)
        return (node.kind in {'atom', 'character', 'string'} or
            node.kind == 'list' and head(node) in SCALAR_CALLS and
            all(self.scalar(child) for child in node.children[1:]))

    def literal_metadata(self, node):
        # Reader metadata is data, but defn attributes may request generated
        # executable code. Only the selected literal/signature grammar is
        # structural; no list, anonymous callback, deref or reader template.
        node = bare(node)
        return node.kind in {'atom', 'string', 'regex', 'character'} or (
            node.kind in {'map', 'vector', 'set'} and all(self.literal_metadata(child) for child in node.children))

    def metadata_owners(self, node):
        if node.kind == 'metadata' and not all(self.literal_metadata(child) for child in node.children[:-1]):
            self.issue('foreign-clojure-metadata-code-review', node, 'Unproved executable/generated metadata payload')
        for child in node.children:
            self.metadata_owners(child)

    def special(self, node):
        return node.kind == 'anonymous' or head(node) in OWNERS

    def scan_calls(self, node, env):
        raw = node
        node = bare(node)
        if raw.kind == 'metadata':
            for metadata in raw.children[:-1]:
                if self.frontier(metadata):
                    self.issue('foreign-clojure-metadata-code-review', metadata, 'Metadata contains executable owner')
        if node.kind in {'quote', 'var-quote', 'string', 'regex', 'character', 'atom', 'anonymous'}:
            return
        name = head(node)
        if name in OWNERS:
            return
        if node.kind == 'list' and name:
            known = (name in CORE_CALLS | ALIASED_CALLS | self.functions | env or
                name.startswith('.') or name.endswith('.') or re.match(r'[A-Z][\w.]*\/\w+', name))
            if name in UNSUPPORTED or not known:
                self.issue('foreign-clojure-call-owner-review', node, 'Unproved function/macro body positions: ' + name)
        for child in node.children:
            self.scan_calls(child, env)

    def frontier(self, node):
        node = bare(node)
        if node.kind in {'quote', 'var-quote', 'string', 'regex', 'character', 'atom'}:
            return []
        if node.kind == 'anonymous' and self.scalar(node.children[0]):
            return []  # Proved expression-only composition stays in its caller.
        if self.special(node):
            return [node]
        return [owner for child in node.children for owner in self.frontier(child)]

    def composed(self, node, env, units=None, scope='value-expression'):
        self.scan_calls(node, env)
        nested = self.frontier(node)
        if nested == [bare(node)]:
            self.walk(node, env)
            return
        if not nested:
            # Ordinary call/data/literal wrapping is preserved; argument
            # separators and nested scalar operators are not statements.
            amount = max(1, sum(bool(row.strip()) for row in self.cleaned[node.start:node.end].splitlines())) if units is None else units
            self.event(node.start, node.end, amount, 'expression', scope)
            return
        cursor, charged = node.start, False
        for index, child in enumerate(sorted(nested, key=lambda item: item.start)):
            # A scalar/data argument before a callback is structural. An
            # unproved executable preceding argument forbids prefix merging;
            # retain its expression unit and require review rather than hide it.
            preceding = []
            def earlier(value):
                value = bare(value)
                if value is not bare(node) and value.kind == 'list' and value.end <= child.start:
                    if not self.scalar(value):
                        preceding.append(value)
                    return
                if value.kind not in {'quote', 'var-quote'}:
                    for item in value.children:
                        earlier(item)
            if index == 0:
                earlier(node)
            merge_header = index == 0 and not preceding and child.start > node.start
            if preceding:
                self.issue('foreign-clojure-callback-prefix-work-review', preceding[0],
                    'Preceding unproved executable argument must retain independent ownership')
            if child.start > cursor:
                amount = (1 if units is None else units) if not charged else 0
                self.event(cursor, child.start, amount, 'expression' if amount else 'structural', scope)
                charged = True
            self.walk(child, env, merge_header=merge_header)
            cursor = child.end
        self.event(cursor, node.end, 0, 'structural', scope)

    def signature(self, node, start, params, bodies, env, scope):
        params = bare(params)
        if params.kind != 'vector':
            self.issue('foreign-clojure-function-signature-review', params, 'Expected written parameter vector')
            self.event(node.start, node.end, 1, 'expression', 'unresolved-function')
            return
        if self.frontier(params):
            self.issue('foreign-clojure-parameter-code-review', params, 'Executable owner in formals')
        if bodies and bare(bodies[0]).kind == 'map':
            self.issue('foreign-clojure-prepost-review', bodies[0], 'Pre/post generated scope requires separate review')
        self.event(start, params.end, 1, 'header', scope)
        for body in bodies:
            self.walk(body, env | names(params))
        self.event(node.end-1, node.end, 1, 'close', scope)

    def function(self, node, env, name):
        xs = node.children
        cursor = 2 if name in {'defn', 'defn-'} else 1
        if name == 'fn' and cursor < len(xs) and bare(xs[cursor]).kind == 'atom':
            env = env | {bare(xs[cursor]).value}
            cursor += 1
        while cursor < len(xs) and bare(xs[cursor]).kind in {'string', 'map'}:
            if bare(xs[cursor]).kind == 'map' and not self.literal_metadata(xs[cursor]):
                self.issue('foreign-clojure-function-attributes-review', xs[cursor],
                    'Executable/generated function attribute requires separate ownership review')
            cursor += 1  # Authored doc/attribute data belongs to the signature.
        if cursor == len(xs):
            self.issue('foreign-clojure-function-signature-review', node, 'Missing arity/formals')
            self.event(node.start, node.end, 1, 'expression', 'unresolved-definition')
        elif bare(xs[cursor]).kind == 'vector':
            self.signature(node, node.start, xs[cursor], xs[cursor+1:], env, 'foreign-clojure-function')
        else:
            self.event(node.start, xs[cursor].start, 1, 'header', 'foreign-clojure-function')
            for arity in xs[cursor:]:
                arity = bare(arity)
                if arity.kind != 'list' or not arity.children:
                    self.issue('foreign-clojure-function-signature-review', arity, 'Unproved multi-arity clause')
                    continue
                self.signature(arity, arity.start, arity.children[0], arity.children[1:], env, 'foreign-clojure-arity')
            self.event(node.end-1, node.end, 1, 'close', 'foreign-clojure-function')

    def bindings(self, node, env, name):
        xs = node.children
        if len(xs) < 2 or bare(xs[1]).kind != 'vector' or len(bare(xs[1]).children) % 2:
            self.issue('foreign-clojure-binding-vector-review', node, 'Expected even initialized binding pairs')
            self.event(node.start, node.end, 1, 'expression', 'unresolved-binding')
            return
        vector = bare(xs[1])
        self.event(node.start, vector.start+1, 1, 'header', 'foreign-clojure-' + name)
        live = set(env)
        for binding, value in zip(vector.children[::2], vector.children[1::2]):
            if bare(binding).kind not in {'atom', 'vector'}:
                self.issue('foreign-clojure-destructuring-review', binding, 'Map/other generated destructuring is unsupported')
            bound = names(binding)
            if not bound and bare(binding).value != '_':
                self.issue('foreign-clojure-binding-name-review', binding, 'Unproved binding owner')
            pair = Node('pair', binding.start, value.end, children=[value])
            self.composed(pair, live, units=1, scope='foreign-clojure-initialized-binding')
            live |= bound
        self.event(vector.end-1, vector.end, 0, 'structural', 'binding-vector')
        for body in xs[2:]:
            self.walk(body, live)
        self.event(node.end-1, node.end, 1, 'close', 'foreign-clojure-' + name)

    def namespace(self, node):
        xs = node.children
        if len(xs) < 2 or bare(xs[1]).kind != 'atom':
            self.issue('foreign-clojure-namespace-review', node, 'Missing namespace name')
            return
        self.event(node.start, xs[1].end, 1, 'header', 'foreign-clojure-namespace')
        for clause in xs[2:]:
            clause = bare(clause)
            keyword = head(clause)
            if keyword in {':require', ':import'}:
                self.event(clause.start, clause.children[0].end, 0, 'structural', 'namespace-clause')
                for declaration in clause.children[1:]:
                    value = bare(declaration)
                    if keyword == ':import' and value.kind == 'vector' and value.children:
                        if (bare(value.children[0]).kind != 'atom' or
                            not re.fullmatch(r'[a-z][\w.]*', bare(value.children[0]).value) or
                            not all(bare(klass).kind == 'atom' and re.fullmatch(r'[A-Z][\w$]*', bare(klass).value) for klass in value.children[1:])):
                            self.issue('foreign-clojure-namespace-review', value, 'Unproved imported package/class signature')
                        self.event(value.start, value.children[0].end, 0, 'structural', 'java-package')
                        for klass in value.children[1:]:
                            self.event(klass.start, klass.end, 1, 'declaration', 'foreign-clojure-import')
                        self.event(value.end-1, value.end, 0, 'structural', 'java-import-vector')
                    else:
                        if keyword == ':import' and value.kind == 'vector':
                            self.issue('foreign-clojure-namespace-review', value, 'Empty import vector')
                        if self.frontier(declaration):
                            self.issue('foreign-clojure-namespace-review', declaration, 'Executable namespace declaration')
                        if not self.literal_metadata(declaration) or value.kind == 'list':
                            self.issue('foreign-clojure-namespace-review', declaration, 'Unsupported grouped/executable import or require signature')
                        self.event(declaration.start, declaration.end, 1, 'declaration', 'foreign-clojure-require' if keyword == ':require' else 'foreign-clojure-import')
                self.event(clause.end-1, clause.end, 0, 'structural', 'namespace-clause')
            elif keyword in {':refer-clojure', ':gen-class'}:
                if keyword == ':gen-class' and len(clause.children) != 1:
                    self.issue('foreign-clojure-generated-class-review', clause, 'Only the selected bare gen-class directive is proved')
                if keyword == ':refer-clojure':
                    options = clause.children[1:]
                    if not (len(options) == 2 and bare(options[0]).value == ':exclude' and
                            bare(options[1]).kind == 'vector' and all(bare(child).kind == 'atom' and
                            re.fullmatch(r'[A-Za-z_][\w!?*+.-]*', bare(child).value) for child in bare(options[1]).children)):
                        self.issue('foreign-clojure-namespace-review', clause, 'Unproved refer-clojure options')
                if not self.literal_metadata(Node('vector', clause.start, clause.end, children=clause.children)):
                    self.issue('foreign-clojure-namespace-review', clause, 'Executable/generated namespace directive payload')
                self.event(clause.start, clause.end, 1, 'declaration', 'foreign-clojure-namespace-directive')
            else:
                self.issue('foreign-clojure-namespace-review', clause, 'Unsupported namespace clause')
                self.event(clause.start, clause.end, 1, 'declaration', 'unresolved-namespace')
        self.event(node.end-1, node.end, 1, 'close', 'foreign-clojure-namespace')

    def walk(self, raw, env=None, merge_header=False):
        first = len(self.events)
        self._walk(raw, env)
        if merge_header:
            node = bare(raw)
            events = self.events[first:]
            header = next((event for event in events if event['kind'] == 'header' and event['start'] == node.start), None)
            if header is not None:
                assert header['units'] == 1
                header['units'] = 0
                header['merged_containing_call_or_initializer_prefix'] = True

    def _walk(self, raw, env=None):
        env = set() if env is None else set(env)
        node = bare(raw)
        name, xs = head(node), node.children
        if raw.kind == 'metadata':
            self.event(raw.start, node.start, 0, 'structural', 'attached-metadata')
        if node.kind == 'anonymous':
            expression = xs[0]
            if self.scalar(expression):
                self.event(node.start, node.end, 1, 'expression', 'proved-scalar-anonymous-composition')
            elif head(expression) in OWNERS:
                self.event(node.start, expression.start, 0, 'structural', 'anonymous-reader-prefix')
                self.walk(expression, env | {'%', '%1', '%2', '%3', '%&'})
            else:
                self.event(node.start, expression.start+1, 1, 'header', 'foreign-clojure-anonymous-callback')
                inner = Node('list', expression.start+1, expression.end-1, children=expression.children)
                self.composed(inner, env | {'%', '%1', '%2', '%3', '%&'})
                self.event(node.end-1, node.end, 1, 'close', 'foreign-clojure-anonymous-callback')
        elif name == 'ns':
            self.namespace(node)
        elif name in {'defn', 'defn-', 'fn'}:
            self.function(node, env, name)
        elif name in {'let', 'loop'}:
            self.bindings(node, env, name)
        elif name in {'def', 'defonce'}:
            if len(xs) not in {2, 3, 4} or bare(xs[1]).kind != 'atom':
                self.issue('foreign-clojure-definition-review', node, 'Unproved definition signature')
            # The written name and its single producer initializer are one
            # declaration. Any nested callback/control still receives owners.
            pair = Node('definition', node.start, node.end, children=xs[2:])
            self.composed(pair, env, scope='foreign-clojure-definition')
        elif name in {'if', 'if-not', 'if-let', 'when', 'when-not', 'when-let', 'doseq', 'dotimes', 'for'}:
            if len(xs) < 2:
                self.issue('foreign-clojure-control-signature-review', node, 'Missing condition/iteration header')
                return
            condition = bare(xs[1])
            self.scan_calls(condition, env)
            if self.frontier(condition):
                self.issue('foreign-clojure-control-header-review', condition, 'Nested executable header needs independent ownership')
            live = set(env)
            if name in {'if-let', 'when-let', 'doseq', 'dotimes', 'for'}:
                if condition.kind != 'vector' or len(condition.children) % 2 or not condition.children:
                    self.issue('foreign-clojure-control-signature-review', condition, 'Expected written binding vector')
                else:
                    live |= set().union(*(names(child) for child in condition.children[::2]))
                    if any(bare(child).value.startswith(':') for child in condition.children[::2]):
                        self.issue('foreign-clojure-iteration-modifier-review', condition, 'Iteration modifier generated scope is unsupported')
            if name in {'if', 'if-not', 'if-let'} and len(xs) not in {3, 4}:
                self.issue('foreign-clojure-control-signature-review', node, 'Expected one then and optional else form')
            if name == 'for' and len(xs) != 3:
                self.issue('foreign-clojure-control-signature-review', node, 'For owns exactly one body expression')
            self.event(node.start, xs[1].end, 1, 'header', 'foreign-clojure-' + name)
            for body in xs[2:]:
                self.walk(body, live)
            self.event(node.end-1, node.end, 1, 'close', 'foreign-clojure-' + name)
        elif name in {'cond', 'case'}:
            if name == 'case' and len(xs) < 2:
                self.issue('foreign-clojure-control-signature-review', node, 'Case needs a selector')
                self.event(node.start, node.end, 1, 'expression', 'unresolved-case')
                return
            start = 2 if name == 'case' else 1
            if name == 'case':
                self.scan_calls(xs[1], env)
                if self.frontier(xs[1]):
                    self.issue('foreign-clojure-control-header-review', xs[1], 'Nested executable case selector needs independent ownership')
            self.event(node.start, xs[start-1].end, 1, 'header', 'foreign-clojure-' + name)
            branches = xs[start:]
            if name == 'cond' and len(branches) % 2:
                self.issue('foreign-clojure-branch-pairs-review', node, 'Cond needs test/expression pairs')
            paired = len(branches) - (len(branches) % 2)
            for predicate, result in zip(branches[:paired:2], branches[1:paired:2]):
                if name == 'cond':
                    self.scan_calls(predicate, env)
                    if self.frontier(predicate):
                        self.issue('foreign-clojure-control-header-review', predicate, 'Nested executable condition')
                self.event(predicate.start, predicate.end, 1, 'header', 'foreign-clojure-branch')
                self.walk(result, env)
            if len(branches) % 2:
                self.walk(branches[-1], env)
            self.event(node.end-1, node.end, 1, 'close', 'foreign-clojure-' + name)
        elif name in {'do', 'try', 'finally'}:
            self.event(node.start, xs[0].end, 1, 'header', 'foreign-clojure-' + name)
            for body in xs[1:]:
                self.walk(body, env)
            self.event(node.end-1, node.end, 1, 'close', 'foreign-clojure-' + name)
        elif name == 'catch':
            if len(xs) < 3:
                self.issue('foreign-clojure-catch-signature-review', node, 'Missing type/local')
                return
            self.event(node.start, xs[2].end, 1, 'header', 'foreign-clojure-catch')
            for body in xs[3:]:
                self.walk(body, env | names(xs[2]))
            self.event(node.end-1, node.end, 1, 'close', 'foreign-clojure-catch')
        elif name == 'reify':
            methods = [i for i, child in enumerate(xs[1:], 1) if bare(child).kind == 'list']
            cursor = methods[0] if methods else len(xs)
            self.event(node.start, xs[cursor-1].end, 1, 'header', 'foreign-clojure-reify')
            for method in xs[cursor:]:
                method = bare(method)
                if method.kind != 'list' or len(method.children) < 2:
                    self.issue('foreign-clojure-reify-method-review', method, 'Unsupported method/interface placement')
                    continue
                self.signature(method, method.start, method.children[1], method.children[2:], env, 'foreign-clojure-method')
            self.event(node.end-1, node.end, 1, 'close', 'foreign-clojure-reify')
        elif name in {'recur', 'set!', 'throw'}:
            self.composed(Node('effect', node.start, node.end, children=xs[1:]), env)
        else:
            self.composed(node, env)

    def normalize(self):
        for form in self.forms:
            self.walk(form)
        result = count_owned(self.source, self.cleaned, [()] * len(self.source), self.events,
            self.diagnostics, 'clojure', assignment_pattern=re.compile(r'(?!)'))
        result.update(proposal_only=True, proposal_status='unhooked-no-admission',
            proposal_version=PROPOSAL_VERSION, parser_pass_is_not_semantic_certification=True)
        return result


def normalize(source, language='clojure'):
    if language != 'clojure':
        raise ValueError('Clojure owns its reader and forms; no language aliases')
    reader = Reader(source)
    forms = reader.read()
    return Owners(source, ''.join(reader.cleaned), forms, reader.diagnostics).normalize()
