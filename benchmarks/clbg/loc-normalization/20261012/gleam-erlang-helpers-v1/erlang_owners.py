"""Bounded original Erlang scanner/AST owners for the five authored helpers."""
from bisect import bisect_right

CONTROLS = {'fun','case','receive','try','begin'}
UNSUPPORTED_AST = {'if','maybe','maybe_match','lc','bc','mc','bin','map','record','record_field','record_index','named_fun','catch'}
AST_NODES = {'attribute','function','clause','var','atom','integer','float','char','string','nil','cons','tuple',
    'call','remote','op','match','case','receive','fun','try','block',*UNSUPPORTED_AST}


def location(anno):
    if type(anno) is list and len(anno) == 2 and all(type(value) is int for value in anno): return anno
    assert type(anno) is list
    values = [value[1] for value in anno if type(value) is list and len(value) == 2 and value[0] == 'location']
    assert len(values) == 1 and type(values[0]) is list and len(values[0]) == 2
    assert all(type(value) is int and value > 0 for value in values[0])
    return values[0]


class Projection:
    def __init__(self,source,evidence):
        assert type(source) is str and evidence['status'] == 'original-source-scan-and-parse-passed'
        self.source = source; self.starts = [0]+[index+1 for index,value in enumerate(source) if value == '\n']
        self.tokens = []; self.events = []; self.functions = []; self.controls = []; self.expressions = []; self.form_order = []
        self.index = 0; self.clean = list(source); self.arm_headers = []
        previous_end = 0
        for value in evidence['tokens']:
            assert type(value) is dict and set(value) == {'kind','line','column','text','value'}
            assert type(value['line']) is type(value['column']) is int and value['line'] > 0 and value['column'] > 0
            assert value['line'] <= len(self.starts) and type(value['text']) is str and value['text']
            start = self.starts[value['line']-1]+value['column']-1; end = start+len(value['text'])
            assert previous_end <= start < end <= len(source) and source[start:end] == value['text']
            assert not source[previous_end:start].strip(), 'Scanner omitted nonwhitespace original source'
            previous_end = end
            if value['kind'] == 'comment':
                for offset in range(start,end):
                    if not self.clean[offset].isspace(): self.clean[offset] = ' '
            else: self.tokens.append(dict(value,start=start,end=end))
        assert not source[previous_end:].strip() and self.tokens
        self.ast = evidence['forms']

    def kind(self): return self.tokens[self.index]['kind'] if self.index < len(self.tokens) else None

    def take(self,kind=None):
        assert self.index < len(self.tokens), 'Unexpected source end'
        token = self.tokens[self.index]
        assert kind is None or token['kind'] == kind, ('Unexpected token',kind,token)
        self.index += 1; return token

    def event(self,kind,start,end,units=1,scope='erlang-original-owner'):
        assert 0 <= start < end <= len(self.source) and type(units) is int and units >= 0
        value = dict(kind=kind,start=start,end=end,units=units,scope=scope,original_source=self.source[start:end])
        self.events.append(value); return value

    def group(self,opening,closing):
        start = self.take(opening); depth = 1; commas = 0; has_values = False
        while depth:
            token = self.take()
            assert token['kind'] not in CONTROLS|{'?','#','<<','>>','||','after'}, 'Unproved executable/data/pattern owner'
            if token['kind'] == opening: depth += 1
            elif token['kind'] == closing: depth -= 1
            elif token['kind'] in {'(','[','{'}:
                self.index -= 1; self.group(token['kind'],{'(':')','[':']','{':'}'}[token['kind']]); has_values = True
            elif depth == 1 and token['kind'] == ',': commas += 1
            elif depth: has_values = True
        return start,token,commas+1 if has_values else 0

    def through_arrow(self,allow_catch=False):
        start = self.tokens[self.index]['start']
        if allow_catch: self.take('catch')
        while self.kind() != '->':
            assert self.kind() not in CONTROLS|{'after','end','?','#','<<','>>','||',None}, 'Unproved arm/guard header'
            if self.kind() in {'(','[','{'}: self.group(self.kind(),{'(':')','[':']','{':'}'}[self.kind()])
            else: self.take()
        end = self.take('->')['end']; return start,end

    def sequence(self,stops):
        body = []
        while self.kind() not in stops:
            assert self.kind() is not None
            expression = self.expression(stops|{','}); body.append(expression)
            if self.kind() == ',': self.take(',')
            else: break
        assert body, 'A written body cannot be empty'
        return body

    def prefix(self,values,control):
        # Only a binding followed by the actual first anonymous callback call
        # head, or a binding before case. Earlier call/data arguments are gated.
        equal = [index for index,value in enumerate(values) if value['kind'] == '=']
        tail = values[equal[-1]+1:] if equal else values
        if equal:
            assert len(equal) == 1 and all(value['kind'] in {'var','atom','{','}',',','[',']','|'} for value in values[:equal[0]])
        if control == 'case': return bool(equal) and not tail
        kinds = [value['kind'] for value in tail]
        return kinds in (['atom','('],['var','('],['atom',':','atom','('])

    def expression(self,stops):
        begin = self.index; start = self.tokens[begin]['start']; groups = []; controls = []; matches = 0
        first_control = None
        while self.kind() is not None:
            kind = self.kind()
            if not groups and kind in stops: break
            if kind in CONTROLS:
                control_begin = self.index; prior = self.tokens[begin:control_begin]
                if controls:
                    between = self.tokens[controls[-1]['end_index']:control_begin]
                    assert all(value['kind'] in {',',')',']','}'} for value in between), 'Unknown effect/argument between nested owners'
                node = self.control(); controls.append(node)
                if first_control is None:
                    first_control = control_begin
                    if prior:
                        assert kind in {'fun','case'} and self.prefix(prior,kind), 'Unknown effect/argument before callable/control prefix'
                        node['header'].update(start=start,original_source=self.source[start:node['header']['end']],scope='erlang-first-direct-prefix-composition')
                continue
            assert kind not in {'?','#','<<','>>','||','if','maybe','after','catch','end','of','when','->'}, 'Unsupported executable/source form'
            if kind in {'(','[','{'}: groups.append({'(':')','[':']','{':'}'}[kind])
            elif kind in {')',']','}'}:
                assert groups and groups.pop() == kind, 'Unmatched data/call group'
            if kind == '=': matches += 1
            self.take()
        assert self.index > begin and not groups
        end = self.tokens[self.index-1]['end']
        if not controls: self.event('body',start,end,max(1,matches),scope='erlang-independent-expression-and-matches')
        else:
            last = controls[-1]['end_index']
            suffix = self.tokens[last:self.index]
            assert all(value['kind'] in {')',']','}'} for value in suffix), 'Unknown effect after nested owner'
            if suffix: self.event('structural-suffix',suffix[0]['start'],suffix[-1]['end'],0)
        value = dict(start=start,end=end,control_count=len(controls)); self.expressions.append(value)
        return value

    def clauses(self,stops,catch=False):
        clauses = []; first = True
        while self.kind() not in stops:
            begin,end = self.through_arrow(catch and first)
            self.event('arm',begin,end)
            self.arm_headers.append(dict(start=begin,end=end))
            body = self.sequence(stops|{';'}); clauses.append(body)
            if self.kind() == ';': self.take(';')
            else: break
            first = False
        assert clauses
        return clauses

    def control(self):
        begin = self.take(); kind = begin['kind']; clauses = []
        if kind == 'case':
            selector_begin = self.index
            while self.kind() != 'of':
                assert self.kind() not in CONTROLS|{None,'->',';','?','#','<<','>>','||'}, 'Unproved case selector'
                if self.kind() in {'(','[','{'}: self.group(self.kind(),{'(':')','[':']','{':'}'}[self.kind()])
                else: self.take()
            assert not any(token['kind'] == '=' for token in self.tokens[selector_begin:self.index]), 'Written matches in a control selector require separate owners'
            end = self.take('of')['end']; header = self.event('header',begin['start'],end)
            clauses = self.clauses({'end'})
        elif kind == 'receive':
            header = self.event('header',begin['start'],begin['end']); clauses = self.clauses({'end'})
        elif kind == 'fun':
            self.group('(',')'); self.take('->')
            header = self.event('header',begin['start'],self.tokens[self.index-1]['end'])
            clauses = [self.sequence({';','end'})]
            while self.kind() == ';':
                self.take(';'); arm_begin = self.tokens[self.index]['start']; self.group('(',')'); arrow = self.take('->')
                self.event('arm',arm_begin,arrow['end']); self.arm_headers.append(dict(start=arm_begin,end=arrow['end']))
                clauses.append(self.sequence({';','end'}))
        elif kind == 'begin':
            header = self.event('header',begin['start'],begin['end']); clauses = [self.sequence({'end'})]
        elif kind == 'try':
            values = []; groups = []
            while self.kind() != 'of' or groups:
                token = self.take(); assert token['kind'] not in CONTROLS|{',',';','end','catch','after','?','#','<<','>>','||','='}, 'Unproved try expression sequence or written selector matches'
                if token['kind'] in {'(','[','{'}: groups.append({'(':')','[':']','{':'}'}[token['kind']])
                elif token['kind'] in {')',']','}'}: assert groups and groups.pop() == token['kind']
                values.append(token)
            assert values; end = self.take('of')['end']; header = self.event('header',begin['start'],end)
            clauses = self.clauses({'catch','end'})
            if self.kind() == 'catch': clauses += self.clauses({'end'},catch=True)
        else: raise AssertionError('Unknown selected control')
        close = self.take('end'); self.event('close',close['start'],close['end'])
        value = dict(kind=kind,start=begin['start'],header=header,end=close['end'],end_index=self.index,clauses=clauses)
        self.controls.append(value); return value

    def forms(self):
        while self.kind() is not None:
            if self.kind() == '-':
                begin = self.take('-'); name = self.take('atom')['value']; assert name in {'module','export'}
                first = self.index; self.group('(',')'); inside = self.tokens[first+1:self.index-1]
                if name == 'module':
                    assert len(inside) == 1 and inside[0]['kind'] == 'atom'; value = inside[0]['value']
                else:
                    assert inside[0]['kind'] == '[' and inside[-1]['kind'] == ']'
                    payload = inside[1:-1]; value = []; position = 0
                    while position < len(payload):
                        atom,slash,arity = payload[position:position+3]
                        assert atom['kind'] == 'atom' and slash['kind'] == '/' and arity['kind'] == 'integer'
                        assert type(arity['value']) is int and arity['value'] >= 0
                        value.append([atom['value'],arity['value']]); position += 3
                        if position < len(payload): assert payload[position]['kind'] == ','; position += 1
                end = self.take('dot'); self.event('directive',begin['start'],end['end'])
                self.form_order.append(dict(kind='attribute',name=name,value=value,start=begin['start'],end=end['end']))
            else:
                clauses = []; name = None; arity = None
                while True:
                    begin = self.take('atom'); assert name is None or name == begin['value']; name = begin['value']
                    _,_,arguments = self.group('(',')'); assert arity is None or arity == arguments; arity = arguments
                    if self.kind() == 'when':
                        self.take('when')
                        while self.kind() != '->':
                            assert self.kind() not in CONTROLS|{None,'?','#','<<','>>','||'}; self.take()
                    arrow = self.take('->'); self.event('function-header',begin['start'],arrow['end'])
                    clauses.append(self.sequence({';','dot'}))
                    if self.kind() != ';': break
                    self.take(';')
                self.take('dot'); self.functions.append(dict(name=name,arity=arity,clauses=clauses))
                self.form_order.append(dict(kind='function',name=name,arity=arity))

    def bind_ast(self):
        assert type(self.ast) is list
        actual = [value for value in self.ast if type(value) is list and value and value[0] == 'function']
        assert len(actual) == len(self.functions)
        def offset(anno):
            line,column = location(anno)
            assert 0 < line <= len(self.starts) and column > 0
            return self.starts[line-1]+column-1
        def body_correspondence(original,parsed):
            assert type(original) is list and len(original) == len(parsed)
            for node,expression in zip(original,parsed):
                assert type(node) is list and len(node) >= 2 and node[0] in AST_NODES
                assert expression['start'] <= offset(node[1]) < expression['end'], 'Original AST expression is outside its written body owner'
        assert len(self.ast) == len(self.form_order)
        for original,parsed in zip(self.ast,self.form_order):
            assert type(original) is list and original[0] == parsed['kind'] and original[2] == parsed['name']
            if parsed['kind'] == 'attribute':
                assert len(original) == 4 and original[3] == parsed['value']
                if parsed['name'] == 'export':
                    assert type(original[3]) is list and all(type(pair) is list and len(pair) == 2 and
                        type(pair[0]) is str and type(pair[1]) is int and pair[1] >= 0 for pair in original[3])
                assert parsed['start'] <= offset(original[1]) < parsed['end']
            else: assert type(original[3]) is int and original[3] == parsed['arity']
        for original,parsed in zip(actual,self.functions):
            assert len(original) == 5 and original[2] == parsed['name'] and type(original[3]) is int and original[3] == parsed['arity']
            assert len(original[4]) == len(parsed['clauses'])
            for clause,body in zip(original[4],parsed['clauses']):
                assert clause[0] == 'clause' and len(clause) == 5 and len(clause[2]) == parsed['arity'] and len(clause[4]) == len(body)
                body_correspondence(clause[4],body)
        ast_controls = []; ast_arms = []
        def walk(value):
            if type(value) is not list: return
            if value and type(value[0]) is str and value[0] in AST_NODES:
                assert value[0] not in UNSUPPORTED_AST, ('Unselected original AST executable/data scope',value[0])
                if value[0] in {'case','receive','fun','try','block'}: ast_controls.append(value)
                if value[0] == 'clause': ast_arms.append(value)
            for child in value: walk(child)
        walk(self.ast)
        expected = [(('block' if node['kind'] == 'begin' else node['kind']),
            [bisect_right(self.starts,node['start']),node['start']-self.starts[bisect_right(self.starts,node['start'])-1]+1]) for node in self.controls]
        assert sorted((node[0],location(node[1])) for node in ast_controls) == sorted(expected), 'Original AST and scanner owner controls differ'
        control_by_location = {(node[0],tuple(location(node[1]))):node for node in ast_controls}
        assert len(control_by_location) == len(ast_controls)
        def selector_without_writes(value):
            if type(value) is not list: return
            assert not (value and value[0] == 'match'), 'Original selector AST has a written match requiring separate owners'
            for child in value: selector_without_writes(child)
        for parsed,(kind,coord) in zip(self.controls,expected):
            node = control_by_location[(kind,tuple(coord))]
            if kind == 'case':
                selector_without_writes(node[2]); original_clauses = node[3]
            elif kind == 'receive':
                assert len(node) == 3; original_clauses = node[2]
            elif kind == 'fun':
                assert len(node) == 3 and node[2][0] == 'clauses'; original_clauses = node[2][1]
            elif kind == 'try':
                assert len(node) == 6 and len(node[2]) == 1 and node[5] == []
                selector_without_writes(node[2])
                original_clauses = node[3]+node[4]
            elif kind == 'block':
                body_correspondence(node[2],parsed['clauses'][0]); continue
            else: raise AssertionError('Unproved actual control AST')
            assert len(original_clauses) == len(parsed['clauses'])
            for clause,body in zip(original_clauses,parsed['clauses']):
                assert clause[0] == 'clause' and len(clause) == 5
                body_correspondence(clause[4],body)
        # Every written non-fun arm must contain one actual original clause
        # annotation, without borrowing an unrelated named/callback clause.
        for header in self.arm_headers:
            matches = [node for node in ast_arms if header['start'] <= offset(node[1]) < header['end']]
            assert len(matches) == 1, 'Original AST arm annotation is not bound to its written header'
        return dict(function_names_arities_and_clause_body_counts_verified=True,original_control_locations_verified=True,
            all_original_form_order_directives_and_no_error_forms_verified=True,
            every_original_control_body_sequence_verified=True,every_written_arm_annotation_verified=True,
            selected_case_and_try_headers_hide_no_written_matches=True,
            actual_function_forms=len(actual),actual_nested_controls=len(expected))

    def result(self):
        self.forms(); binding = self.bind_ast()
        events = sorted(self.events,key=lambda value:(value['start'],value['end']))
        for left,right in zip(events,events[1:]): assert left['end'] <= right['start'], 'Overlapping original Erlang owner spans'
        rows = self.source.splitlines(); clean = ''.join(self.clean).splitlines()
        physical = [number for number,row in enumerate(clean,1) if row.strip()]
        units = {number:0 for number in physical}; owned = set(); continuation = set()
        for event in events:
            first,last = bisect_right(self.starts,event['start']),bisect_right(self.starts,event['end']-1)
            units[first] += event['units']; owned.update(range(first,last+1))
            if event['kind'] == 'body': continuation.update(range(first+1,last+1))
        for number in physical:
            if not units[number] and (number not in owned or number in continuation):
                significant = [token for token in self.tokens if token['line'] == number and token['kind'] not in {'dot',';',','}]
                if significant: units[number] = 1
        expansions = [dict(line=number,original=rows[number-1],units=units[number],extra_lines=units[number]-1,
            canonical_events=[event for event in events if bisect_right(self.starts,event['start']) == number])
            for number in physical if units[number] != 1 or any(bisect_right(self.starts,event['start']) == number for event in events)]
        return dict(physical_loc=len(physical),normalized_loc=sum(units.values()),canonical_events=events,expansions=expansions,
            diagnostics=[],original_scanner_AST_binding=binding,proposal_only=True,generated_source_substitute=False,
            canonical_LOC_admitted=False,chart_admission=False)


def project(source,evidence):
    try: return Projection(source,evidence).result()
    except (AssertionError,KeyError,IndexError,TypeError,ValueError) as error:
        return dict(physical_loc=None,normalized_loc=None,canonical_events=[],expansions=[],
            diagnostics=[dict(kind='erlang-original-source-owner-review',message=str(error) or type(error).__name__)],
            proposal_only=True,generated_source_substitute=False,canonical_LOC_admitted=False,chart_admission=False)
