"""Reader-aware LOC, without packed independent forms or assignments discounts."""
from collections import defaultdict
from dataclasses import dataclass, field
import re


@dataclass
class Form:
    value: str
    start: int
    end: int
    line: int
    children: list = field(default_factory=list)


def read(source):
    clean = list(source)
    tokens = []
    i, line = 0, 1

    def erase(a, b):
        for p in range(a, b):
            if clean[p] != '\n':
                clean[p] = ' '

    while i < len(source):
        c, start, row = source[i], i, line
        if c.isspace():
            line += c == '\n'
            i += 1
            continue
        if c == ';':
            i = source.find('\n', i)
            if i < 0:
                i = len(source)
            erase(start, i)
            continue
        if source.startswith('#|', i):
            depth = 1
            i += 2
            while depth:
                if source.startswith('#|', i):
                    depth += 1
                    i += 2
                elif source.startswith('|#', i):
                    depth -= 1
                    i += 2
                else:
                    line += source[i] == '\n'
                    i += 1
            erase(start, i)
            continue
        if source.startswith('#\\', i):
            i += 2
            # A delimiter itself can be the character, e.g. #\( and #\;.
            if i < len(source):
                i += 1
                while i < len(source) and not source[i].isspace() and source[i] not in '()[]{}':
                    i += 1
        elif c == '"' or source.startswith('#"', i):
            i += 2 if c == '#' else 1
            while i < len(source):
                if source[i] == '\\':
                    i += 2
                elif source[i] == '"':
                    i += 1
                    break
                else:
                    line += source[i] == '\n'
                    i += 1
        elif c in '()[]{}':
            i += 1
        else:
            prefix = next((p for p in ('#,@', '#,', '#`', "#'", '#;', '#+', '#-', ',@', ',', "'", '`', '#.') if source.startswith(p, i)), None)
            if prefix:
                i += len(prefix)
            else:
                while i < len(source) and not source[i].isspace() and source[i] not in '()[]{};"\'`,':
                    i += 1
                if i == start:
                    i += 1
        tokens.append(Form(source[start:i], start, i, row))

    index = 0

    def one():
        nonlocal index
        token = tokens[index]
        index += 1
        if token.value in '([{':
            children = []
            while index < len(tokens) and tokens[index].value not in ')]}':
                child = one()
                if child is not None:
                    children.append(child)
            assert index < len(tokens), (token.line, 'unclosed form')
            end = tokens[index].end
            index += 1
            return Form('list', token.start, end, token.line, children)
        if token.value in ("'", '`', ',', ',@', "#'", '#`', '#,', '#,@', '#.', '#+', '#-', '#;'):
            children = []
            if token.value in ('#+', '#-'):
                children.append(one())
            child = one()
            if child is not None:
                children.append(child)
            end = children[-1].end if children else token.end
            if token.value == '#;':
                erase(token.start, end)
                return None
            return Form(token.value, token.start, end, token.line, children)
        return token

    forms = []
    while index < len(tokens):
        f = one()
        if f is not None:
            forms.append(f)
    return forms, ''.join(clean)


def normalize(source, language):
    forms, clean = read(source)
    extra = defaultdict(int)
    reasons = defaultdict(set)

    def sequence(items, reason):
        rows = defaultdict(int)
        for item in items:
            if item is not None:
                rows[item.line] += 1
        for row, count in rows.items():
            if count > 1:
                extra[row] += count - 1
                reasons[row].add(reason)

    def visit(node):
        if not node.children:
            return
        if node.value == "'":
            return
        if node.value != 'list':
            for child in node.children:
                visit(child)
            return
        xs = node.children
        head = xs[0].value.lower() if xs else ''
        body = None
        if head in ('defun', 'defmacro', 'defmethod'):
            body = xs[3:]
        elif head in ('lambda', 'λ'):
            body = xs[2:]
        elif head in ('define', 'define-syntax', 'define-syntax-rule') and len(xs) > 1 and xs[1].children:
            body = xs[2:]
        elif head in ('begin', 'progn', 'locally', 'eval-when', 'module+', 'block'):
            body = xs[2:] if head in ('eval-when', 'module+', 'block') else xs[1:]
        elif head in ('when', 'unless', 'dotimes', 'dolist', 'with-output-to-string', 'with-open-file', 'with-open-stream', 'with-mutex', 'sb-thread:with-mutex', 'multiple-value-bind', 'destructuring-bind'):
            body = xs[3:] if head in ('multiple-value-bind', 'destructuring-bind') else xs[2:]
        elif head in ('do', 'do*'):
            body = xs[3:]
            if len(xs) > 2:
                sequence(xs[2].children[1:], 'Packed independent exit expressions')
        elif head in ('let', 'let*', 'let-values', 'let*-values', 'letrec', 'letrec*'):
            named = head == 'let' and len(xs) > 1 and not xs[1].children and xs[1].value != 'list'
            offset = 3 if named else 2
            if len(xs) >= offset:
                sequence(xs[offset - 1].children, 'Packed independent initialized bindings')
            body = xs[offset:]
        elif head in ('flet', 'labels', 'macrolet'):
            if len(xs) > 1:
                sequence(xs[1].children, 'Packed local function definitions')
                for definition in xs[1].children:
                    sequence(definition.children[2:], 'Packed independent local function body forms')
            body = xs[2:]
        elif re.match(r'^(?:unsafe-)?for(?:[/\*-]|$)', head):
            offset = 3 if 'fold' in head else 2
            body = xs[offset:]
            if 'fold' in head and len(xs) > 1:
                sequence(xs[1].children, 'Packed independent accumulator bindings')
        elif head in ('cond', 'case', 'ecase', 'typecase'):
            for clause in xs[1 if head == 'cond' else 2:]:
                if not any(c.value == '=>' for c in clause.children):
                    sequence(clause.children[1:], 'Packed independent conditional body forms')
        elif head in ('setf', 'setq', 'psetf', 'psetq'):
            sequence(xs[1::2], 'Packed independent assignments')
        elif head == 'gmp':
            sequence([node] + [child for child in xs[1:] if child.value == ','], 'Packed independent assignments/reads in the source GMP arithmetic macro')
        elif head == 'loop' and language == 'lisp':
            pending = []
            for x in xs[1:]:
                if x.children:
                    if pending is not None:
                        pending.append(x)
                else:
                    if pending:
                        sequence(pending, 'Packed independent loop body forms')
                    pending = [] if x.value.lower().lstrip(':') in ('do', 'doing', 'finally', 'initially') else None
            if pending:
                sequence(pending, 'Packed independent loop body forms')
        if body:
            sequence(body, 'Packed independent body expressions')
        if head == 'quote':
            return
        if head in ('defun', 'defmacro', 'defmethod'):
            children = xs[3:]
        elif head in ('lambda', 'λ'):
            children = xs[2:]
        elif head in ('define', 'define-syntax', 'define-syntax-rule'):
            children = xs[2:]
        else:
            children = xs[1:]
        for child in children:
            visit(child)

    # Reader directives (#lang) are semantic LOC, but not executable forms.
    sequence([f for f in forms if f.children], 'Packed independent top-level forms')
    for f in forms:
        visit(f)
    physical = sum(bool(line.strip()) for line in clean.splitlines())
    expansions = [dict(line=row, extra=count, reason='; '.join(sorted(reasons[row])), source=source.splitlines()[row - 1]) for row, count in sorted(extra.items()) if count]
    return dict(physical_loc=physical, normalized_loc=physical + sum(extra.values()), expansions=expansions,
                metric='statement-normalized', policy='Nonblank/noncomment physical LOC, semantic declarations and every application helper, plus packed independent body forms, initialized bindings and assignments. Structural argument, literal-data and parameter separators are excluded. Reader comments are removed. No compressed-size metric.')
