"""Bounded authored scalar-method facts for explicit value controls.

Unknown/vendor/actor/ref calls retain real control owners. A method name or
return annotation alone never establishes scalar expression ownership.
"""
import re

IDENT = r"[A-Za-z_][\w']*"
CALL = re.compile(r'(?<![\w.])(' + IDENT + r'(?:\.' + IDENT + r')*)(?:\[[^\n\]]+\])?\s*\(')
SCALARS = {'Bool', 'U8', 'U16', 'U32', 'U64', 'U128', 'USize',
    'I8', 'I16', 'I32', 'I64', 'I128', 'ISize', 'F32', 'F64'}
KEYWORDS = {'if', 'match', 'try', 'recover', 'while', 'for', 'consume', 'digestof'}


def calls(text):
    return [match for match in CALL.finditer(text) if match[1] not in KEYWORDS]


class ScalarResolution:
    def __init__(self, source, rows, layout_end, pure_value):
        self.source, self.pure_value = source, pure_value
        self.members, self.types, self.facts, self.stack = [], {}, {}, []
        owner = None
        for index, row in enumerate(rows):
            code = row['code'].strip()
            declared = re.match(r'(actor|class|primitive|interface|trait|struct|type)\s+(?:(?:iso|trn|ref|val|box|tag)\s+)?(' + IDENT + r')\b', code)
            if declared:
                name = declared[2]
                self.types.setdefault(name, []).append(dict(kind=declared[1], start=row['code_start'], end=layout_end(rows, index, row['end']), fields={}, field_types={}))
                owner = self.types[name][-1]
                owner['name'] = name
                continue
            if owner is None or not (owner['start'] < row['code_start'] < owner['end']):
                continue
            field = re.fullmatch(r'(?:let|var|embed)\s+(' + IDENT + r')\s*:\s*([^=]*?)(?:\s*=.*)?', code)
            if field and row['indent'] == rows[next(i for i, item in enumerate(rows) if item['code_start'] == owner['start'])]['indent'] + 2:
                simple_type = re.fullmatch(r'(' + IDENT + r')(?:\s+(?:iso|trn|ref|val|box|tag))?', field[2].strip())
                owner['fields'][field[1]] = simple_type[1] if simple_type else None
                owner['field_types'][field[1]] = field[2].strip()
            signature = re.match(r'(fun|be|new)\s+(?:(iso|trn|ref|val|box|tag)\s+)?(' + IDENT + r')\s*\(([^\n]*)\)\s*(?::\s*(' + IDENT + r')(?:\s+val)?\s*)?\??\s*=>', code)
            if not signature:
                continue
            start = row['code_start']; header_end = start + signature.end()
            end = layout_end(rows, index, header_end)
            self.members.append(dict(owner=owner, name=signature[3], kind=signature[1], capability=signature[2] or 'box',
                return_type=signature[5], formals=signature[4], start=start, header_end=header_end, end=end,
                body=source[header_end:end]))

    def enclosing(self, start, end):
        choices = [item for item in self.members if item['header_end'] <= start < end <= item['end']]
        return choices[0] if len(choices) == 1 else None

    def bindings(self, member, point):
        result = dict(member['owner']['fields'])
        for name, kind in re.findall(r'(' + IDENT + r')\s*:\s*(' + IDENT + r')', member['formals']):
            result[name] = kind
        prefix = self.source[member['header_end']:point]
        for match in re.finditer(r'\b(?:let|var|embed)\s+(' + IDENT + r')\s*(?::\s*(' + IDENT + r'))?', prefix):
            if match[2]: result[match[1]] = match[2]
            else: result[match[1]] = None
        return result

    def target(self, caller, call, point):
        parts = call.split('.')
        if len(parts) == 1:
            owner, name = caller['owner']['name'], parts[0]
            if name in self.types:
                return None  # Any authored constructor has its own effectful source.
            if name in self.bindings(caller, point):
                return None  # A nearer callable value cannot use the method binding.
        elif len(parts) == 2:
            receiver, name = parts
            bindings = self.bindings(caller, point)
            owner = caller['owner']['name'] if receiver == 'this' else bindings.get(receiver)
            if owner is None and receiver not in bindings and receiver in self.types:
                owner = receiver
        else:
            return None  # Chained/vendor paths need separate exact target evidence.
        choices = [item for item in self.members if item['owner']['name'] == owner and item['name'] == name]
        if len(choices) != 1 or len(self.types.get(owner, [])) != 1:
            return None
        return choices[0]

    def pure_method(self, member):
        if (member['owner']['kind'] not in {'class', 'primitive', 'struct'}
                or member['kind'] != 'fun' or member['capability'] not in {'box', 'val'}):
            return False
        if member['return_type'] not in SCALARS or member['return_type'] in self.types:
            return False
        if member['formals'].strip():
            for formal in member['formals'].split(','):
                typed = re.fullmatch(r'\s*' + IDENT + r'\s*:\s*(' + IDENT + r')(?:\s+val)?\s*', formal)
                if typed is None or typed[1] not in SCALARS or typed[1] in self.types:
                    return False  # Unknown overloaded operands/callbacks are not scalar facts.
        if member['start'] in self.stack:
            return True  # Typed read-only recursion; surrounding body still must prove all branches.
        self.stack.append(member['start'])
        try:
            body = member['body'].strip()
            subject = re.match(r'match\s+\([^\n()]*\)', body)
            call_heads = calls(body)
            for name, kind in member['owner']['fields'].items():
                if kind in SCALARS and kind not in self.types:
                    continue
                for used in re.finditer(r'\b' + re.escape(name) + r'\b', body):
                    if subject and used.end() <= subject.end():
                        continue  # Read-only nominal match subject, never an arithmetic operand.
                    if any(call.start() == used.start() and call[1].startswith(name + '.') for call in call_heads):
                        continue  # The complete method target/body must still resolve below.
                    return False
            body_start = self.source.find(body, member['header_end'], member['end'])
            allowed_words = KEYWORDS | SCALARS | {'then', 'else', 'elseif', 'end', 'let',
                'iso', 'trn', 'ref', 'val', 'box', 'tag', 'and', 'or', 'xor', 'not', 'true', 'false', 'None'}
            for token in re.finditer(IDENT, body):
                if token[0] in allowed_words or token[0] in self.types:
                    continue
                if any(call.start() <= token.start() < call.end() for call in call_heads):
                    continue  # Full callee paths and all argument calls resolve below.
                if re.search(r'\b(?:let|var|embed)\s*$', body[:token.start()]):
                    continue  # Written nominal pattern binding, not a free arithmetic operand.
                if token[0] not in self.bindings(member, body_start + token.start()):
                    return False
            return (self.scalar_operands(member, body, body_start, call_heads)
                and self.pure_value(body, resolver=self)
                and (not calls(body) or self.prove(body)))
        finally:
            self.stack.pop()

    def nominal_binding(self, caller, name, point):
        bindings = self.bindings(caller, point)
        kind = bindings.get(name)
        declared = self.types.get(kind, [])
        if len(declared) == 1 and declared[0]['kind'] in {'class', 'primitive', 'struct'}:
            return True
        raw = caller['owner']['field_types'].get(name)
        if kind is not None or raw is None:
            return False
        if re.search(r'\b' + re.escape(name) + r'\s*:', caller['formals']):
            return False
        prefix = self.source[caller['header_end']:point]
        if re.search(r'\b(?:let|var|embed)\s+' + re.escape(name) + r'\b', prefix):
            return False  # A nearer unknown binding cannot reuse the field union.
        union = re.fullmatch(r'\(\s*' + IDENT + r'(?:\s*\|\s*' + IDENT + r')+\s*\)', raw)
        if union is None:
            return False
        kinds = re.findall(IDENT, raw)
        return all(item == 'None' or (len(self.types.get(item, [])) == 1
            and self.types[item][0]['kind'] in {'class', 'primitive', 'struct'}) for item in kinds)

    def scalar_operands(self, caller, value, start, call_heads):
        """Every value operand needs primitive facts, including call arguments.

        A resolved scalar callee does not type its caller's condition or actual
        argument expression. Nominal values are structural only in a bounded
        direct tuple match subject or written typed pattern, or as a complete
        receiver of a separately proved authored method call.
        """
        structural_ranges = []
        subject = re.match(r'match\s+\(\s*' + IDENT + r'(?:\s*,\s*' + IDENT + r')*\s*\)', value)
        if subject:
            subject_tokens = list(re.finditer(IDENT, value[subject.start():subject.end()]))[1:]
            nominal_subject = False
            for token in subject_tokens:
                point = start + subject.start() + token.start()
                kind = self.bindings(caller, point).get(token[0])
                if kind in SCALARS and kind not in self.types:
                    continue
                if not self.nominal_binding(caller, token[0], point):
                    return False
                nominal_subject = True
            if nominal_subject:
                arms = list(re.finditer(r'\|([^\n]*?)=>', value))
                typed = r'let\s+' + IDENT + r'\s*:\s*' + IDENT + r'(?:\s+(?:iso|trn|ref|val|box|tag))?'
                tuple_pattern = r'\(\s*' + typed + r'(?:\s*,\s*' + typed + r')*\s*\)'
                if not arms or any(re.fullmatch(tuple_pattern, arm[1].strip()) is None
                        or len(re.findall(r'\blet\b', arm[1])) != len(subject_tokens) for arm in arms):
                    return False  # Nominal subjects require actual typed tuple patterns, not overloadable equality patterns.
            structural_ranges.append(subject.span())
        for pattern in re.finditer(r'\blet\s+(' + IDENT + r')\s*:\s*(' + IDENT + r')(?:\s+(?:iso|trn|ref|val|box|tag))?', value):
            kind = pattern[2]
            declared = self.types.get(kind, [])
            if not (kind in SCALARS and not declared) and not (len(declared) == 1 and declared[0]['kind'] in {'class', 'primitive', 'struct'}):
                return False
            structural_ranges.append(pattern.span())
        syntax = KEYWORDS | {'then', 'else', 'elseif', 'end', 'let', 'iso', 'trn',
            'ref', 'val', 'box', 'tag', 'and', 'or', 'xor', 'not', 'true', 'false', 'None'}
        for token in re.finditer(IDENT, value):
            if token[0] in syntax:
                continue
            if any(a <= token.start() and token.end() <= b for a, b in structural_ranges):
                continue
            if any(call.start() <= token.start() and token.end() <= call.end() for call in call_heads):
                continue  # The complete callee/receiver resolves separately; not its arguments.
            kind = self.bindings(caller, start + token.start()).get(token[0])
            if kind not in SCALARS or kind in self.types:
                return False
        return True

    def prove(self, value):
        located = list(re.finditer(re.escape(value), self.source))
        if len(located) != 1:
            return False
        start, end = located[0].span()
        caller = self.enclosing(start, end)
        if caller is None:
            return False
        evidence = []
        call_heads = calls(value)
        for call in call_heads:
            point = start + call.start()
            target = self.target(caller, call[1], point)
            if target is None or not self.pure_method(target):
                return False
            evidence.append(dict(call=call[1], call_start=point, call_end=start + call.end(),
                target_owner=target['owner']['name'], target_method=target['name'],
                target_start=target['start'], target_end=target['end'], return_type=target['return_type'],
                capability=target['capability'], actual_entire_authored_body_checked=True))
        if not self.scalar_operands(caller, value, start, call_heads):
            return False
        if evidence:
            self.facts[(start, end)] = dict(control_start=start, control_end=end, targets=evidence,
                source_owned_resolution=True, caller_value_operands_checked=True,
                arbitrary_name_or_return_hint_is_not_proof=True)
        return True
