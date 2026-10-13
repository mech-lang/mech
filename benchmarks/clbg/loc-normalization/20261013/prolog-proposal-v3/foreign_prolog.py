"""Unexecuted, unhooked original Prolog owner proposal; never route by .pl.

Consumes a pinned read_term/3 receipt and the exact original UTF-8 bytes. No
file access, hashing, application imports, compiler calls, evaluation or chart
admission occurs here. The future caller must hold the common timing mutex.
"""
from bisect import bisect_right
from copy import deepcopy
import posixpath

PROPOSAL_VERSION = 'prolog-original-term-owners-20261013-v2'
RULE_VERSION = 'canonical-source-units-20261012-approved-rule-v2'
SETTINGS = dict(module='prolog_source_scan_v1', character_escapes=True,
    double_quotes='string', backquoted_string=False, cycles=False,
    dotlists=False, quasi_quotations='collected-not-evaluated')
CONTROLS = {(',', 2), (';', 2), ('->', 2), ('\\+', 1)}
EFFECTS = {('is', 2), ('=', 2), ('==', 2), ('\\=', 2), ('\\==', 2),
    ('=:=', 2), ('=\\=', 2), ('<', 2), ('>', 2), ('=<', 2), ('>=', 2)}
BUILTINS = set('''true/0 fail/0 !/0 nl/0 set_stream/2 flush_output/1 halt/1
print_message/2 current_prolog_flag/2 atom_number/2 integer/1 throw/1
read_string/3 read_string/5 sub_string/5 string_length/2 get_string_code/3
reverse/2 atomics_to_string/3 format/2 put_code/1 functor/3 arg/3 nb_setarg/3
keysort/2'''.split())
SYSTEM_META = {'call/1': {0: 0}, 'catch/3': {0: 0, 2: 0},
               'initialization/2': {0: 0}}
VENDOR_META = {('pcre', 're_foldl/6'): {0: 3}}
VENDOR_EXPORTS = {'pcre': {'re_foldl/6', 're_replace/5'},
    'hashtable': {'ht_new/1', 'ht_get/3', 'ht_put/3', 'ht_pairs/2'}}


def bare(node):
    while node.get('kind') == 'group':
        node = node['children'][0]
    return node


def signature(node):
    node = bare(node)
    if node['kind'] == 'term':
        return node['name'], len(node['children'])
    if node['kind'] == 'atom':
        return node['value'], 0
    return '', -1


def predicate(node):
    name, arity = signature(node)
    return name + '/' + str(arity)


def children(node):
    result = list(node.get('children', []))
    if node.get('tail') is not None:
        result.append(node['tail'])
    return result


def qualified(node, module):
    node = bare(node)
    if signature(node) == (':', 2):
        prefix, target = node['children']
        if bare(prefix)['kind'] != 'atom':
            return None, target
        return qualified(target, bare(prefix)['value'])
    return module, node


def indicators(node):
    node = bare(node)
    if node['kind'] == 'atom' and node['value'] == '[]':
        return set()
    if node['kind'] != 'list' or node.get('tail') is not None:
        raise ValueError('Expected a closed predicate-indicator list')
    result = set()
    for item in node['children']:
        item = bare(item)
        if signature(item) != ('/', 2):
            raise ValueError('Unsupported export/import indicator')
        name, arity = map(bare, item['children'])
        if name['kind'] != 'atom' or arity['kind'] != 'number':
            raise ValueError('Unsupported export/import indicator values')
        number = int(arity['value'])
        if number < 0 or str(number) != arity['value']:
            raise ValueError('Unselected predicate arity spelling')
        result.add(name['value'] + '/' + str(number))
    return result


class BundleContext:
    """Only one profile's actual main plus its compiler-bound authored helpers.

    No global guessed callee whitelist: local definitions, actual module exports
    and selected library imports establish targets. Meta slots are declared or
    inferred from a written formal forwarded into an already proved meta slot.
    The caller must byte/pin-verify all receipt inputs before constructing this.
    """
    def __init__(self, packets):
        self.definitions, self.meta, self.exports, self.imports = {}, {}, {}, {}
        self.sources, self.diagnostics = {}, []
        self.vendor_bindings = {}
        self.packets = {}
        for supplied_packet in packets:
            try:
                packet = deepcopy(supplied_packet)
                forms = [bare(entry['node']) for entry in packet['forms']]
                modules = [form['children'][0] for form in forms
                    if signature(form) == (':-', 1) and
                    signature(form['children'][0]) == ('module', 2)]
                if len(modules) != 1 or bare(modules[0]['children'][0])['kind'] != 'atom':
                    raise ValueError('One explicit module declaration required')
                module = bare(modules[0]['children'][0])['value']
                if module in self.sources:
                    raise ValueError('Use one main per context; aliases are separate replays')
                self.sources[module] = packet['source']
                self.packets[module] = packet
                self.exports[module] = indicators(modules[0]['children'][1])
                self.imports[module] = []
                for form in forms:
                    if signature(form) == (':-', 1):
                        directive = bare(form['children'][0])
                        if signature(directive)[0] == 'use_module':
                            self.imports[module].append(directive)
                        elif signature(directive) == ('meta_predicate', 1):
                            declaration = bare(directive['children'][0])
                            slots = {}
                            for index, mode in enumerate(declaration.get('children', [])):
                                mode = bare(mode)
                                if mode['kind'] == 'number' and mode['value'].isdigit():
                                    slots[index] = int(mode['value'])
                                elif mode['kind'] != 'atom' or mode['value'] not in {'+', '-', '?', '@'}:
                                    raise ValueError('Unsupported meta-predicate mode')
                            self.meta[(module, predicate(declaration))] = slots
                    else:
                        head = form['children'][0] if signature(form) == (':-', 2) else form
                        if signature(head)[1] < 0 or signature(head) in CONTROLS:
                            raise ValueError('Unsupported clause dispatch head')
                        key = (module, predicate(head))
                        self.definitions.setdefault(key, []).append(form)
                for exported in self.exports[module]:
                    if (module, exported) not in self.definitions:
                        raise ValueError('Export has no written definition: ' + exported)
            except (KeyError, TypeError, ValueError) as error:
                self.diagnostics.append(dict(kind='prolog-bundle-context', detail=str(error)))
        try:
            for module in self.sources:
                # Probe all declarations, including an otherwise unused import.
                self.resolve(module, '__unselected_import_probe__/-1')
            for key, slots in self.meta.items():
                if key not in self.definitions or any(extra > 9 for extra in slots.values()):
                    raise ValueError('Unbound/unselected meta-predicate declaration')
        except (KeyError, TypeError, ValueError) as error:
            self.diagnostics.append(dict(kind='prolog-bundle-import-or-meta', detail=str(error)))
        # Bounded fixed point over the finite written formal positions. This
        # captures clbg_io:run_checked/1 without a name-based purity exception.
        changed = True
        while changed and not self.diagnostics:
            changed = False
            for key, clauses in self.definitions.items():
                for clause in clauses:
                    if signature(clause) != (':-', 2):
                        continue
                    head, body = clause['children']
                    formals = bare(head).get('children', [])
                    for variable, extra in self.forwarded(body, key[0]):
                        for index, formal in enumerate(formals):
                            formal = bare(formal)
                            if formal['kind'] == 'variable' and formal['id'] == variable['id']:
                                slots = self.meta.setdefault(key, {})
                                if index in slots and slots[index] != extra:
                                    self.diagnostics.append(dict(kind='prolog-meta-arity-conflict', detail=str(key)))
                                elif index not in slots:
                                    slots[index] = extra
                                    changed = True

    def resolve(self, module, target):
        key = (module, target)
        if key in self.definitions:
            return key
        for directive in self.imports.get(module, []):
            args = directive['children']
            if len(args) not in {1, 2}:
                raise ValueError('Unsupported use_module arity')
            source = bare(args[0])
            selected = indicators(args[1]) if len(args) == 2 else None
            if signature(source) == ('library', 1):
                library = bare(source['children'][0])
                if library['kind'] != 'atom' or library['value'] not in VENDOR_EXPORTS:
                    raise ValueError('Unselected vendor library')
                exports = VENDOR_EXPORTS[library['value']]
                if selected is None or not selected <= exports:
                    raise ValueError('Unselected vendor import surface')
                self.vendor_bindings.setdefault(library['value'], set()).update(selected)
                if target in selected:
                    return library['value'], target
            elif source['kind'] == 'atom':
                imported = source['value'].rsplit('/', 1)[-1].removesuffix('.pl')
                if imported not in self.exports:
                    raise ValueError('Unbound authored helper import')
                expected_path = posixpath.normpath(posixpath.join(
                    posixpath.dirname(self.sources[module]), source['value']))
                if not expected_path.endswith('.pl'):
                    expected_path += '.pl'
                if expected_path != posixpath.normpath(self.sources[imported]):
                    raise ValueError('Helper module name is not its actual imported source path')
                exports = self.exports[imported]
                if selected is not None and not selected <= exports:
                    raise ValueError('Import outside actual helper exports')
                if target in (exports if selected is None else selected):
                    return imported, target
            else:
                raise ValueError('Unsupported module source syntax')
        if target in self.vendor_bindings.get(module, set()):
            return module, target
        if target in BUILTINS | set(SYSTEM_META) and (module in self.sources or module == 'system'):
            return 'system', target
        return None

    def slots(self, key):
        if key is None:
            return {}
        return self.meta.get(key, SYSTEM_META.get(key[1], {}) if key[0] == 'system'
                             else VENDOR_META.get(key, {}))

    def forwarded(self, raw, module):
        node = bare(raw)
        if node['kind'] == 'variable':
            yield node, 0
            return
        if signature(node) in CONTROLS:
            for child in node['children']:
                yield from self.forwarded(child, module)
            return
        module, node = qualified(node, module)
        if module is None:
            return
        for index, extra in self.slots(self.resolve(module, predicate(node))).items():
            argument = bare(node['children'][index])
            if argument['kind'] == 'variable':
                yield argument, extra
            elif extra == 0:
                yield from self.forwarded(argument, module)


class Projection:
    def __init__(self, source_bytes, packet, language, context):
        self.source = source_bytes.decode('utf8', errors='strict')
        self.packet, self.language, self.context = packet, language, context
        self.diagnostics, self.events = [], []
        self.data_rows = set()
        self.cleaned = list(self.source)
        self.byte_offsets = [0]
        for character in self.source:
            self.byte_offsets.append(self.byte_offsets[-1] + len(character.encode('utf8')))
        self.lines = [0] + [index + 1 for index, char in enumerate(self.source) if char == '\n']

    def issue(self, kind, node, detail):
        self.diagnostics.append(dict(kind=kind, offset=node.get('start', 0), detail=detail))

    def emit(self, node, kind, units=1, span=None, **evidence):
        start, end = span or (node.get('functor_start', node['start']),
                             node.get('functor_end', node['end']))
        self.events.append(dict(start=start, end=end, units=units, kind=kind,
            scope='prolog-' + kind, full_owner_start=min(node['start'], start),
            full_owner_end=max(node['end'], end), **evidence))

    def validate(self, node, parent=None):
        if node.get('kind') not in {'term', 'group', 'list', 'atom', 'number', 'string', 'variable'}:
            raise ValueError('Unsupported reader position/node: ' + str(node.get('kind')))
        start, end = node['start'], node['end']
        if type(start) is not int or type(end) is not int or not 0 <= start < end <= len(self.source):
            raise ValueError('Invalid character interval')
        if parent and not parent['start'] <= start < end <= parent['end']:
            raise ValueError('Child outside parent position')
        if node['kind'] == 'group' and (self.source[start] != '(' or self.source[end-1] != ')'):
            raise ValueError('Parenthesis positions do not match original source')
        if node['kind'] == 'term':
            if not (type(node['functor_start']) is int and type(node['functor_end']) is int and
                    start <= node['functor_start'] < node['functor_end'] <= end):
                raise ValueError('Invalid functor position')
        previous = start
        for child in children(node):
            self.validate(child, node)
            if child['start'] < previous:
                raise ValueError('Overlapping/reordered written child positions')
            previous = child['end']

    def wrapping(self, node, owner_start):
        # Literal/data continuation rows only. Call/formal argument wrapping,
        # arithmetic expression wraps and pattern commas do not gain units.
        base_line = bisect_right(self.lines, owner_start)
        def visit(value):
            value = bare(value)
            if value['kind'] in {'list', 'string'} or value['kind'] == 'atom' and value.get('value') == '[]':
                for line, start in enumerate(self.lines, 1):
                    row_key = owner_start, line
                    if line == base_line or row_key in self.data_rows:
                        continue
                    left = max(start, value['start'])
                    right = min(self.lines[line] if line < len(self.lines) else len(self.source), value['end'])
                    active = next((index for index in range(left, right) if not self.cleaned[index].isspace()), None)
                    if active is not None:
                        self.emit(value, 'literal-data-continuation', span=(active, active + 1))
                        self.data_rows.add(row_key)
                return
            for child in children(value):
                visit(child)
        visit(node)

    def data(self, node, owner_start, allow_control=False):
        def inspect(value):
            value = bare(value)
            if not allow_control and signature(value) in CONTROLS:
                self.issue('prolog-hidden-goal-data', value, 'Stored/control-valued payload needs source-bound flow review')
            if signature(value) in {('op', 3), ('-->', 2), ('term_expansion', 2), ('goal_expansion', 2)}:
                self.issue('prolog-expansion-syntax', value, 'Unselected operator/expansion/DCG grammar')
            for child in children(value):
                inspect(child)
        inspect(node)
        self.wrapping(node, owner_start)

    def goal(self, raw, module, formal_meta=frozenset()):
        if raw['kind'] == 'group':
            inner = bare(raw)
            self.goal(raw['children'][0], module, formal_meta)
            if signature(inner) in CONTROLS:
                self.emit(raw, 'written-control-close', span=(raw['end']-1, raw['end']))
            return
        node = bare(raw)
        sig = signature(node)
        if sig == (',', 2):
            for child in node['children']:
                self.goal(child, module, formal_meta)
            return
        if sig == (';', 2):
            self.emit(node, 'branch-header')
            for child in node['children']:
                self.goal(child, module, formal_meta)
            return
        if sig in {('->', 2), ('\\+', 1)}:
            self.emit(node, 'control-header')
            before = len(self.events)
            self.goal(node['children'][0], module, formal_meta)
            first = next((event for event in self.events[before:] if event['units']), None)
            if first is not None and first['kind'] in {
                    'call', 'cut', 'unification', 'arithmetic-binding',
                    'comparison', 'variable-meta-call'}:
                first['units'] = 0
                first['merged_with_containing_control_header'] = True
            for child in node['children'][1:]:
                self.goal(child, module, formal_meta)
            return
        if sig == (':', 2):
            target_module, target = qualified(node, module)
            if target_module is None:
                self.issue('prolog-dynamic-module', node, 'Variable module qualification is unsupported')
            else:
                self.goal(target, target_module, formal_meta)
            return
        if node['kind'] == 'variable':
            if node['id'] not in formal_meta:
                self.issue('prolog-unbound-meta-target', node, 'Local/unknown variable goal has no reviewed formal flow')
            self.emit(node, 'variable-meta-call')
            return
        if sig in EFFECTS:
            self.emit(node, 'arithmetic-binding' if sig == ('is', 2) else 'unification' if sig == ('=', 2) else 'comparison')
            if sig == ('=', 2) and any(bare(child)['kind'] == 'variable' and bare(child)['id'] in formal_meta
                                     for child in node['children']):
                self.issue('prolog-meta-construction', node, 'Binding a meta formal needs separate source-bound goal-flow review')
            self.data(node, node['start'])
            return
        key = self.context.resolve(module, predicate(node))
        if key is None:
            self.issue('prolog-unproved-call-target', node, 'No selected definition/import/builtin: ' + predicate(node))
        slots = self.context.slots(key)
        self.emit(node, 'meta-call-header' if slots else 'cut' if sig == ('!', 0) else 'call', resolved_target=key)
        for index, argument in enumerate(node.get('children', [])):
            if index not in slots:
                self.data(argument, node['start'])
                continue
            value, extra = bare(argument), slots[index]
            if value['kind'] == 'variable':
                if value['id'] not in formal_meta:
                    self.issue('prolog-unbound-meta-target', value, 'Unproved dynamically constructed goal')
                self.emit(value, 'formal-meta-reference', 0, extra_arguments=extra)
            elif extra:
                target_module, target = qualified(value, module)
                target_name, target_arity = signature(target)
                resolved = self.context.resolve(target_module, target_name + '/' + str(target_arity + extra)) if target_module else None
                if target_arity < 0 or resolved is None or signature(target) in CONTROLS:
                    self.issue('prolog-unproved-closure-target', value, 'No source-bound closure at appended arity')
                self.emit(value, 'named-closure-reference', 0, extra_arguments=extra, resolved_target=resolved)
                self.data(value, node['start'])
            else:
                self.goal(argument, module, formal_meta)
        if slots and node['kind'] == 'term' and self.source[node['end']-1] == ')':
            self.emit(node, 'written-meta-close', span=(node['end']-1, node['end']))

    def run(self):
        try:
            if self.language != 'prolog':
                raise ValueError('Explicit language=prolog required; .pl does not select this grammar')
            raw = self.packet
            if not raw['source'].endswith('.pl'):
                raise ValueError('Original Prolog source path required; QLF/WAM is not a source-count input')
            if raw['schema'] != 'prolog-source-reader-v1' or raw['source_text'] != self.source:
                raise ValueError('Receipt does not contain these exact original UTF-8 bytes')
            if raw['version'] != [10, 0, 2] or raw['reader_settings'] != SETTINGS:
                raise ValueError('Unselected reader version/settings')
            if not raw['operators_before'] or raw['operators_before'] != raw['operators_after']:
                raise ValueError('Reader operator inventory changed or missing')
            if type(raw['eof_char_count']) is not int or raw['eof_char_count'] != len(self.source):
                raise ValueError('Reader did not consume the complete source')
            for comment in raw['comments']:
                start, end = comment['start'], comment['end']
                if type(start) is not int or type(end) is not int or not 0 <= start < end <= len(self.source):
                    raise ValueError('Invalid comment interval')
                if self.source[start:end] != comment['text'] or not comment['text'].startswith(('%', '/*')):
                    raise ValueError('Comment position/text mismatch')
                self.cleaned[start:end] = [char if char.isspace() else ' ' for char in self.source[start:end]]
            self.diagnostics.extend(self.context.diagnostics)
            modules = [module for module, path in self.context.sources.items() if path == raw['source']]
            if len(modules) != 1:
                raise ValueError('Receipt source missing/ambiguous in this actual authored bundle')
            module = modules[0]
            if self.context.packets.get(module) != raw:
                raise ValueError('Counted packet differs from the exact bundle context receipt')
            cursor = 0
            for entry in raw['forms']:
                if type(entry['quasi_quotation_count']) is not int or entry['quasi_quotation_count'] != 0:
                    raise ValueError('Quasi quotation is collected, unsupported, never evaluated')
                form = entry['node']
                self.validate(form)
                if (type(entry['read_start']) is not int or type(entry['read_end']) is not int or
                    not 0 <= entry['read_start'] <= form['start'] < form['end'] <= entry['read_end'] <= len(self.source)):
                    raise ValueError('Reader frame does not contain the written term')
                if form['start'] < cursor or ''.join(self.cleaned[cursor:form['start']]).strip():
                    raise ValueError('Unread/reordered source before clause')
                term = bare(form)
                sig = signature(term)
                period = form['end']
                while period < len(self.source) and self.cleaned[period].isspace():
                    period += 1
                if period >= len(self.source) or self.source[period] != '.':
                    raise ValueError('Missing written clause full stop')
                after = period + 1
                if after > entry['read_end']:
                    raise ValueError('Full stop lies outside the actual reader frame')
                if after < len(self.source) and not self.source[after].isspace() and not self.source.startswith(('%', '/*'), after):
                    raise ValueError('Full stop lacks a Prolog layout/comment/EOF boundary')
                if sig == (':-', 2):
                    head, body = term['children']
                    self.emit(head, 'clause-dispatch-header')
                    self.data(head, head['start'], allow_control=True)
                    key = (module, predicate(head))
                    formals = bare(head).get('children', [])
                    formal_meta = frozenset(bare(formals[index])['id'] for index in self.context.slots(key)
                        if bare(formals[index])['kind'] == 'variable')
                    self.goal(body, module, formal_meta)
                    self.emit(form, 'written-clause-close', span=(period, after))
                elif sig == (':-', 1):
                    directive = bare(term['children'][0])
                    if signature(directive)[0] not in {'module', 'use_module', 'meta_predicate', 'initialization'}:
                        self.issue('prolog-unselected-directive', directive, 'Directive/reader/expansion effects are not selected')
                    self.emit(term, 'directive-header')
                    if signature(directive) == ('initialization', 2):
                        if bare(directive['children'][1]).get('value') != 'main':
                            self.issue('prolog-unselected-initialization-mode', directive, 'Only explicit main mode selected')
                        before = len(self.events)
                        self.goal(directive, module)
                        first = next((event for event in self.events[before:] if event['units']), None)
                        if first:
                            first['units'] = 0
                            first['merged_with_directive_header'] = True
                        self.emit(form, 'written-clause-close', span=(period, after))
                    else:
                        self.data(directive, term['start'], allow_control=True)
                else:
                    if sig in CONTROLS or sig[0] in {'-->', 'term_expansion', 'goal_expansion', 'op'} or sig[1] < 0:
                        self.issue('prolog-unselected-clause', term, 'Unsupported dispatch/expansion/DCG')
                    self.emit(term, 'fact-dispatch-header')
                    self.data(term, term['start'], allow_control=True)
                cursor = after
            if ''.join(self.cleaned[cursor:]).strip():
                raise ValueError('Unread meaningful source after final clause (including literal end_of_file)')
            self.events.sort(key=lambda event: (event['start'], event['end']))
            for left, right in zip(self.events, self.events[1:]):
                if left['end'] > right['start']:
                    raise ValueError('Overlapping independent owner anchors')
        except (KeyError, TypeError, ValueError, IndexError) as error:
            self.issue('prolog-reader-owner-integrity', {}, str(error))
        for event in self.events:
            event.update(byte_start=self.byte_offsets[event['start']], byte_end=self.byte_offsets[event['end']],
                         line=bisect_right(self.lines, event['start']))
        for issue in self.diagnostics:
            issue['line'] = bisect_right(self.lines, issue.get('offset', 0))
        physical = sum(bool(row.strip()) for row in ''.join(self.cleaned).splitlines())
        return dict(proposal_version=PROPOSAL_VERSION, rule_version=RULE_VERSION,
            language=self.language, physical_loc=physical,
            normalized_loc=None if self.diagnostics else sum(event['units'] for event in self.events),
            canonical_events=self.events, diagnostics=self.diagnostics,
            character_offsets_reconciled_to_original_utf8_bytes=not self.diagnostics,
            status='unqualified-diagnostics' if self.diagnostics else 'proposed-awaiting-actual-fixtures-and-source-review',
            source_hash_verified=False, actual_fixtures_qualified=False,
            numeric_count_is_not_semantic_certification=True, chart_admission=False)


def count_prolog(source_bytes, packet, *, language, context):
    if type(source_bytes) is not bytes:
        raise TypeError('Supply exact original UTF-8 bytes, never generated QLF/WAM text')
    return Projection(source_bytes, packet, language, context).run()
