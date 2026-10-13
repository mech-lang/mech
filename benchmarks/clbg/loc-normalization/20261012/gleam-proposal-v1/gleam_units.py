"""Unexecuted canonical unit projection from Gleam-owned source AST.

Root approved containing let/call prefix + first directly nested case/fn header
as one owner. No source/parser/fixture qualification or chart admission is implied.
Only source-bound independent semantic review can resolve unknown callable purity.
"""
from gleam_reader import contains_owner, lex, parse_source, walk_owners

VERSION = 'gleam-canonical-owner-projection-candidate-v1'


def direct_controls(node):
    if node['kind'] in {'case', 'anonymous-function'}:
        yield node
        return
    if node['kind'] == 'block':
        return
    for child in node['children']:
        yield from direct_controls(child)


def scalar_group(node):
    if node['kind'] == 'atom': return True
    if node['kind'] == 'field': return scalar_group(node['children'][0])
    if node['kind'] == 'unary': return scalar_group(node['children'][0])
    if node['kind'] == 'binary' and node['operator'] != '|>':
        return all(scalar_group(child) for child in node['children'])
    if node['kind'] == 'block' and len(node['children']) == 1:
        statement = node['children'][0]
        return statement['kind'] == 'expression-statement' and scalar_group(statement['children'][0])
    return False


class Projection:
    def __init__(self, source, parsed):
        self.source = source
        cleaned = list(source)
        for span in parsed['comment_spans']:
            for index in range(span['start'], span['end']):
                if not cleaned[index].isspace(): cleaned[index] = ' '
        self.comment_free = ''.join(cleaned)
        self.parsed = parsed
        self.units, self.diagnostics = [], list(parsed['diagnostics'])
        self.tokens = lex(source)['tokens'] if not self.diagnostics else []
        self.by_start = {token['start']: token for token in self.tokens}
    def emit(self, kind, start, end, full_start=None, full_end=None):
        self.units.append(dict(kind=kind, start=start, end=end, units=1,
            full_owner_start=start if full_start is None else full_start,
            full_owner_end=end if full_end is None else full_end))
    def header(self, node, prefix=None):
        start = node['start'] if prefix is None else prefix
        token = self.by_start[start]
        self.emit(node['kind'] + '-header', start, token['end'],
                  full_start=start, full_end=node['header_end'])
    def close(self, node):
        self.emit('written-close', node['close_start'], node['end'])
    def gate(self, node, kind, message):
        if not any(issue['kind'] == kind and issue['offset'] == node['start'] for issue in self.diagnostics):
            self.diagnostics.append(dict(kind=kind, offset=node['start'], message=message))
    def anchor_end(self, token):
        newline = self.source.find('\n', token['start'], token['end'])
        return token['end'] if newline < 0 else newline
    def declaration(self, node):
        kind = node['kind']
        if kind == 'function':
            self.header(node)
            body = node['children'][0]
            self.block(body, function_body=True)
            self.close(body)
        elif kind == 'external-function':
            for annotation in node['children']:
                self.emit('semantic-external-annotation', annotation['start'], annotation['end'])
            self.emit('external-signature', node['start'], node['end'])
        elif kind == 'type-declaration':
            token = self.by_start[node['start']]
            self.emit('type-declaration', token['start'], token['end'], full_end=node['end'])
            for constructor in node['children']:
                self.emit('type-constructor-declaration', constructor['start'], self.by_start[constructor['start']]['end'],
                          full_end=constructor['end'])
                self.data_wrapping(constructor, constructor['start'])
            if node['proposed_units'] == 2:
                self.emit('type-written-close', node['end']-1, node['end'])
        else:
            token = self.by_start[node['start']]
            self.emit(kind, token['start'], token['end'], full_end=node['end'])
            for child in node['children']: self.data_wrapping(child, node['start'])
    def block(self, node, function_body=False):
        for statement in node['children']:
            self.statement(statement)
        if not function_body: self.close(node)
    def statement(self, node):
        expression = node['children'][0]
        controls = list(direct_controls(expression))
        if expression['kind'] in {'case', 'anonymous-function'}:
            self.control(expression, node['start'])
            return
        if controls:
            # Prefix belongs only to its first directly nested written header.
            # Other callbacks stay separate, including all body statements.
            self.expression(expression, prefix_owner=node['start'])
            return
        token = self.by_start[node['start']]
        self.emit(node['kind'], token['start'], self.anchor_end(token), full_end=node['end'])
        self.expression(expression)
        self.data_wrapping(expression, node['start'])
    def control(self, node, prefix=None):
        self.header(node, prefix)
        if node['kind'] == 'anonymous-function':
            body = node['children'][0]
            self.block(body, function_body=True)
            self.close(body)
            return
        for child in node['children']:
            if child['kind'] != 'case-arm':
                if contains_owner(child):
                    self.gate(child, 'gleam-case-subject-owner-review', 'Nested executable case-subject body needs an explicit header composition review.')
                continue
            token = self.by_start[child['start']]
            self.emit('case-arm-header', token['start'], token['end'], full_end=child['header_end'])
            body = child['children'][0]
            if body['kind'] in {'case', 'anonymous-function'}:
                self.control(body)
            elif body['kind'] == 'block' and not scalar_group(body):
                self.block_value(body)
            else:
                self.statement(dict(kind='expression-statement', start=body['start'],
                                    end=body['end'], children=[body]))
        self.close(node)
    def expression(self, node, prefix_owner=None):
        if node['kind'] in {'case', 'anonymous-function'}:
            self.control(node, prefix_owner)
            return
        if node['kind'] == 'block':
            if not scalar_group(node): self.block_value(node)
            return
        controls = list(direct_controls(node))
        first = controls[0] if controls and prefix_owner is not None else None
        if first is not None:
            # An independently evaluated earlier call cannot disappear merely
            # because a later callback/control shares its enclosing expression.
            for candidate in walk_owners(node):
                if candidate['kind'] == 'call' and candidate['end'] <= first['start']:
                    self.gate(candidate, 'gleam-prefix-callee-review', 'Earlier callable operand requires source-bound semantic ownership review.')
        for child in node['children']:
            child_controls = list(direct_controls(child))
            prefix = prefix_owner if first is not None and first in child_controls else None
            self.expression(child, prefix)
    def block_value(self, node):
        if (len(node['children']) == 1 and node['children'][0]['kind'] == 'expression-statement'
                and not contains_owner(node['children'][0]['children'][0])):
            self.gate(node, 'gleam-callable-grouping-review', 'Single callable/control block is not proved scalar grouping; retain explicit source review.')
        self.block(node)
    def data_wrapping(self, node, base_start):
        # Preserve wrapping only inside language-owned data/signature regions,
        # not arbitrary physical rows as executable units or fallback counts.
        regions = []
        def regions_in(value):
            if value['kind'] in {'case', 'anonymous-function', 'block'}:
                return
            yield value
            for child in value['children']:
                yield from regions_in(child)
        for candidate in regions_in(node):
            if candidate['kind'] in {'list', 'tuple', 'bit-array', 'type-constructor'} and not contains_owner(candidate):
                regions.append((candidate['start'], candidate['end']))
            elif candidate['kind'] == 'atom' and candidate.get('token_kind') == 'string':
                regions.append((candidate['start'], candidate['end']))
        base_line = self.source.count('\n', 0, base_start)
        seen = set()
        for start, end in regions:
            cursor = start
            for row in self.comment_free[start:end].splitlines(keepends=True):
                line = self.source.count('\n', 0, cursor)
                code = row.strip()
                if code and line != base_line and line not in seen:
                    left = cursor + len(row)-len(row.lstrip())
                    right = cursor + len(row.rstrip())
                    self.emit('preserved-data-wrapping', left, right)
                    seen.add(line)
                cursor += len(row)


def propose_units(source):
    parsed = parse_source(source)
    projection = Projection(source, parsed)
    if not projection.diagnostics:
        for declaration in parsed['declarations']:
            projection.declaration(declaration)
    return dict(version=VERSION, rule='containing-prefix-first-control-owner',
        normalized_loc=None if projection.diagnostics else sum(unit['units'] for unit in projection.units),
        canonical_events=projection.units, diagnostics=projection.diagnostics,
        original_owner_ast=parsed['declarations'], proposal_only=True,
        status='unqualified-diagnostics' if projection.diagnostics else 'proposed-awaiting-actual-fixtures-and-source-review',
        source_hash_verified=False, actual_fixtures_qualified=False,
        chart_admission=False, numeric_count_is_not_semantic_certification=True)
