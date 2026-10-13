"""Bounded source-bound composition facts; no name-only purity exemptions."""
import copy
import hashlib
import re

from gleam_reader import lex, parse_source, walk_owners


def compact_type(source, node):
    return None if node is None else re.sub(r'\s+', '', source[node['start']:node['end']])


def names_in_pattern(source, node):
    return [token['word'] for token in lex(source[node['start']:node['end']])['tokens']
            if token['kind'] == 'name' and token['word'][0].islower() and token['word'] != '_']


def arguments(node):
    values = node['children'][1:]
    if any(value.get('spread') or value.get('label') for value in values): return None
    return [value['children'][0] for value in values]


class SourceResolution:
    def __init__(self, source, parsed, source_path, context):
        self.source, self.parsed, self.path = source, parsed, source_path
        self.context = context
        self.certificates = []
        self.modules = {}
        self.valid = bool(context and source_path in context.get('sources', {}))
        if not self.valid: return
        for name, record in context['sources'].items():
            if hashlib.sha256(record['text'].encode()).hexdigest() != record['sha256']:
                self.valid = False; return
            self.modules[name] = record
        self.valid = self.modules[source_path]['text'] == source
        if any(node['kind'] == 'type-declaration' and node['name'] in ('Int','Float','Bool','BitArray','String')
               for node in parsed['declarations']): self.valid = False
        self.imports = {}
        self.functions = {}
        self.global_values = {node['name'] for node in parsed['declarations'] if node['kind'] == 'constant'}
        self.fields = {}
        for node in parsed['declarations']:
            if node['kind'] == 'import':
                self.imports.setdefault(node['alias'], []).append(node)
            elif node['kind'] in ('function', 'external-function'):
                self.functions.setdefault(node['name'], []).append(node)
            elif node['kind'] == 'type-declaration':
                for constructor in node['children']:
                    for field in constructor['children']:
                        if field.get('name'):
                            self.fields[(node['name'],field['name'])] = compact_type(source,field['children'][0])

    def declaration_ref(self, path, node, role):
        record = self.modules[path]
        result = dict(path=path, sha256=record['sha256'], start=node['start'], end=node['end'], role=role,
                      exact_authored_or_vendor_source=record['text'][node['start']:node['end']])
        if node.get('kind') == 'external-function':
            result['external_annotations'] = [dict(start=annotation['start'],end=annotation['end'],
                backend=annotation['backend'],module=annotation['module'],function=annotation['function'],
                exact_source=record['text'][annotation['start']:annotation['end']]) for annotation in node['children']]
            result['signature'] = self.signature(path,node)
        return result

    def module_alias(self, alias, env):
        if alias in env or alias in self.functions or alias in self.global_values: return None
        choices = self.imports.get(alias, [])
        return choices[0] if len(choices) == 1 else None

    def function_target(self, callee, env):
        if callee['kind'] == 'atom':
            name = callee['word']
            if name in env or name in self.global_values: return None
            nodes = self.functions.get(name, [])
            return (self.path,nodes[0]) if len(nodes) == 1 else None
        if callee['kind'] != 'field' or callee['children'][0]['kind'] != 'atom': return None
        imported = self.module_alias(callee['children'][0]['word'], env)
        if imported is None: return None
        module = imported['module']
        records = [(path,record) for path,record in self.modules.items() if record.get('module') == module]
        if len(records) != 1: return None
        path, record = records[0]
        parsed = parse_source(record['text'])
        if parsed['diagnostics']: return None
        if any(node['kind'] == 'constant' and node['name'] == callee['field'] for node in parsed['declarations']): return None
        if any(node['kind'] == 'type-declaration' and node['name'] in ('Int','Float','Bool','BitArray','String')
               for node in parsed['declarations']): return None
        choices = [node for node in parsed['declarations'] if node['kind'] in ('function','external-function') and node['name'] == callee['field']]
        return (path,choices[0]) if len(choices) == 1 else None

    def signature(self, path, node):
        text = self.modules[path]['text']
        return ([compact_type(text,f['annotation']) for f in node['formals']], compact_type(text,node.get('returns')))

    def environment(self, target):
        functions = [node for node in self.parsed['declarations'] if node['kind'] == 'function'
                     and node['start'] <= target['start'] < target['end'] <= node['end']]
        if len(functions) != 1: return None
        function = functions[0]
        env = {formal['name']:compact_type(self.source,formal['annotation']) for formal in function['formals']}
        def descend(node, bindings):
            if node is target: return bindings
            if not (node['start'] <= target['start'] < target['end'] <= node['end']): return None
            if node['kind'] == 'block':
                current = dict(bindings)
                for statement in node['children']:
                    if statement['end'] <= target['start']:
                        if statement['kind'] == 'binding':
                            kind = compact_type(self.source,statement.get('annotation')) or self.infer(statement['children'][0],current,scalar_calls=False)
                            names = names_in_pattern(self.source,statement['pattern'])
                            for name in names: current[name] = kind if len(names) == 1 else None
                    else:
                        result = descend(statement,current)
                        if result is not None: return result
                return None
            if node['kind'] == 'anonymous-function':
                bindings = dict(bindings)
                for formal in node['formals']: bindings[formal['name']] = compact_type(self.source,formal['annotation'])
            if node['kind'] == 'case-arm':
                bindings = dict(bindings)
                for name in names_in_pattern(self.source,node['pattern']): bindings[name] = None
            for child in node['children']:
                result = descend(child,bindings)
                if result is not None: return result
            return None
        return function, descend(function['children'][0],env)

    def enum_value(self, name, env):
        if name in env or name in self.global_values: return None
        candidates = [(item,selector) for items in self.imports.values() for item in items
                      for selector in item.get('selectors',[]) if selector['local'] == name]
        if len(candidates) != 1 or candidates[0][0]['module'] != 'gleam/order': return None
        selected = candidates[0][1]['name']
        if selected not in ('Lt','Eq','Gt'): return None
        if self.vendor_definition('gleam/order', 'Order') is None: return None
        return 'order.Order'

    def infer(self, node, env, scalar_calls=True, evidence=None):
        kind = node['kind']
        if kind == 'atom':
            if node['token_kind'] == 'number':
                return 'Float' if not node['word'].lower().startswith(('0x','0b','0o')) and any(c in node['word'] for c in '.eE') else 'Int'
            if node['token_kind'] == 'string': return 'String'
            if node['word'] in ('True','False'): return 'Bool'
            return env.get(node['word']) or self.enum_value(node['word'],env)
        if kind == 'field':
            base = self.infer(node['children'][0],env,scalar_calls,evidence)
            if base and node['field'].isdigit() and base.startswith('#(') and base.endswith(')'):
                values = base[2:-1].split(','); index = int(node['field'])
                return values[index] if index < len(values) else None
            return self.fields.get((base,node['field']))
        if kind == 'bit-array':
            for segment in node['children']:
                value = self.infer(segment['children'][0],env,scalar_calls,evidence)
                expected = {'utf8':'String','bits':'BitArray',None:'Int'}.get(segment['modifier'])
                if expected is None or value != expected: return None
            return 'BitArray'
        if kind == 'unary':
            value = self.infer(node['children'][0],env,scalar_calls,evidence)
            return value if (node['operator'] == '-' and value in ('Int','Float') or node['operator'] == '!' and value == 'Bool') else None
        if kind == 'binary':
            left,right = [self.infer(child,env,scalar_calls,evidence) for child in node['children']]
            operator = node['operator']
            if operator in ('&&','||'): return 'Bool' if left == right == 'Bool' else None
            if operator in ('+','-','*','/','%'): return 'Int' if left == right == 'Int' else None
            if operator in ('+.','-.','*.','/.'): return 'Float' if left == right == 'Float' else None
            if operator in ('==','!=','<','>','<=','>='): return 'Bool' if left == right and left in ('Int','Bool') else None
            if operator in ('<.','>.','<=.','>=.'): return 'Bool' if left == right == 'Float' else None
            return None
        if kind == 'block':
            if len(node['children']) != 1 or node['children'][0]['kind'] != 'expression-statement': return None
            return self.infer(node['children'][0]['children'][0],env,scalar_calls,evidence)
        if kind == 'call':
            actuals = arguments(node)
            target = self.function_target(node['children'][0],env)
            if actuals is None or target is None: return None
            path, declaration = target
            parameters, result = self.signature(path,declaration)
            if [self.infer(value,env,scalar_calls,evidence) for value in actuals] != parameters: return None
            if scalar_calls:
                if not self.scalar_target(path,declaration): return None
                if evidence is not None: evidence.append(self.declaration_ref(path,declaration,'scalar-callee'))
            return result
        return None

    def scalar_target(self, path, node):
        if node['kind'] == 'external-function':
            parameters,result = self.signature(path,node)
            if len(node['children']) != 1: return False
            external = node['children'][0]
            return external.get('backend') == 'erlang' and (
                (external.get('module'),external.get('function'),parameters,result) == ('math','sqrt',['Float'],'Float') or
                (external.get('module'),external.get('function'),parameters,result) == ('binary','at',['BitArray','Int'],'Int'))
        parameters,result = self.signature(path,node)
        if path != self.path or parameters != ['Int'] or result != 'Int': return False
        block = node['children'][0]
        if len(block['children']) != 1 or block['children'][0]['kind'] != 'expression-statement': return False
        expression = block['children'][0]['children'][0]
        if expression['kind'] != 'case': return False
        subject, *arms = expression['children']
        if subject['kind'] != 'atom' or subject['word'] != node['formals'][0]['name'] or len(arms) != 5: return False
        patterns = []; outputs = []
        for arm in arms[:-1]:
            pattern = self.source[arm['pattern']['start']:arm['pattern']['end']]
            if not re.fullmatch(r'\s*\d+\s*\|\s*\d+\s*',pattern): return False
            patterns.extend(int(value) for value in re.findall(r'\d+',pattern))
            body = arm['children'][0]
            if body['kind'] != 'atom' or body['token_kind'] != 'number' or not body['word'].isdigit(): return False
            outputs.append(int(body['word']))
        last = arms[-1]
        abort = last['children'][0]
        return (len(set(patterns)) == 8 and outputs == [0,1,2,3] and
            self.source[last['pattern']['start']:last['pattern']['end']].strip() == '_' and
            abort['kind'] == 'abort-expression' and self.source[abort['start']:abort['start']+5] == 'panic' and
            len(abort['children']) == 1 and abort['children'][0]['kind'] == 'atom' and abort['children'][0]['token_kind'] == 'string')

    def vendor_definition(self, module, name):
        records = [(path,record) for path,record in self.modules.items() if record.get('module') == module and record.get('role') == 'vendor']
        if len(records) != 1 or self.context.get('vendor_version') != '1.0.5': return None
        path,record = records[0]
        tokens = lex(record['text'])['tokens']
        starts = [i for i,token in enumerate(tokens) if token['word'] in ('fn','type') and i+1 < len(tokens) and tokens[i+1]['word'] == name]
        if len(starts) != 1: return None
        start = starts[0]; opening = next((i for i in range(start,len(tokens)) if tokens[i]['word'] == '{'),None)
        if opening is None: return None
        depth = 0; close = None
        for i in range(opening,len(tokens)):
            if tokens[i]['word'] == '{': depth += 1
            elif tokens[i]['word'] == '}':
                depth -= 1
                if depth == 0: close = i; break
        if close is None: return None
        header = re.sub(r'\s+','',record['text'][tokens[start]['start']:tokens[opening]['start']])
        expected = {'to_list':'fnto_list(dict:Dict(k,v))->List(#(k,v))',
                    'sort':'fnsort(list:List(a),bycompare:fn(a,a)->Order)->List(a)',
                    'Order':'typeOrder'}.get(name)
        if header != expected: return None
        if name == 'Order' and [token['word'] for token in tokens[opening+1:close]] != ['Lt','Eq','Gt']: return None
        return self.declaration_ref(path,dict(start=tokens[start]['start'],end=tokens[close]['end']),'generic-vendor-value-declaration')

    def comparator(self, node, env):
        if node['kind'] != 'atom': return None
        target = self.function_target(node,env)
        if target is None or target[0] != self.path: return None
        path,function = target
        parameters,result = self.signature(path,function)
        if parameters != ['#(Int,Int)','#(Int,Int)'] or result != 'order.Order': return None
        block = function['children'][0]
        if len(block['children']) != 1 or block['children'][0]['kind'] != 'expression-statement': return None
        local = {formal['name']:kind for formal,kind in zip(function['formals'],parameters)}
        order = self.module_alias('order',local)
        if order is None or order['module'] != 'gleam/order': return None
        def ordered(value):
            if value['kind'] == 'atom': return self.enum_value(value['word'],local) == 'order.Order'
            if value['kind'] != 'case': return False
            subject,*arms = value['children']
            if self.infer(subject,local) != 'Bool' or len(arms) != 2: return False
            patterns = [self.source[arm['pattern']['start']:arm['pattern']['end']].strip() for arm in arms]
            return patterns == ['True','False'] and all(ordered(arm['children'][0]) for arm in arms)
        return self.declaration_ref(path,function,'separately-counted-comparator-body') if ordered(block['children'][0]['children'][0]) else None

    def sorted_prefix(self, node, env):
        if node['kind'] != 'call': return None
        callee = node['children'][0]; actuals = arguments(node)
        if callee['kind'] != 'field' or callee['children'][0]['kind'] != 'atom' or actuals is None or len(actuals) != 2: return None
        imported = self.module_alias(callee['children'][0]['word'],env)
        if imported is None or imported['module'] != 'gleam/list' or callee['field'] != 'sort': return None
        collection, compare = actuals
        if collection['kind'] != 'call': return None
        dict_callee = collection['children'][0]; dict_args = arguments(collection)
        if dict_callee['kind'] != 'field' or dict_callee['children'][0]['kind'] != 'atom' or dict_args is None or len(dict_args) != 1: return None
        dictionary = self.module_alias(dict_callee['children'][0]['word'],env)
        if dictionary is None or dictionary['module'] != 'gleam/dict' or dict_callee['field'] != 'to_list': return None
        if self.infer(dict_args[0],env) != 'dict.Dict(Int,Int)': return None
        comparator = self.comparator(compare,env)
        sort = self.vendor_definition('gleam/list','sort'); to_list = self.vendor_definition('gleam/dict','to_list')
        if comparator is None or sort is None or to_list is None: return None
        return [sort,to_list,comparator,self.vendor_definition('gleam/order','Order')]

    def certify(self, node, contract, function, env, evidence, extra=None):
        if not evidence or any(value is None for value in evidence): return False
        certificate = dict(contract=contract, caller_path=self.path, caller_sha256=self.modules[self.path]['sha256'],
            start=node['start'], end=node['end'], original_owner_ast=copy.deepcopy(node),
            enclosing_function=dict(name=function['name'],start=function['start'],end=function['end']),
            lexical_bindings=dict(env), resolved_declarations=evidence,
            binding_context_status=self.context.get('status'), vendor_version=self.context.get('vendor_version'),
            caller_helper_vendor_pins={path:record['sha256'] for path,record in self.modules.items()},
            source_bound_proposal=True, semantic_admission=False)
        certificate.update(extra or {})
        if certificate not in self.certificates: self.certificates.append(certificate)
        return True

    def scalar_group(self, node):
        if not self.valid or node['kind'] != 'block' or len(node['children']) != 1: return False
        statement = node['children'][0]
        if statement['kind'] != 'expression-statement': return False
        expression = statement['children'][0]
        if any(item['kind'] in ('binding','anonymous-function','case','abort-expression') for item in walk_owners(expression)): return False
        located = self.environment(node)
        if located is None or located[1] is None: return False
        function,env = located; evidence = []
        if self.infer(expression,env,evidence=evidence) not in ('Int','Float','Bool'): return False
        return self.certify(node,'one-expression-typed-scalar-grouping',function,env,evidence)

    def prefix(self, candidate, whole, first):
        if not self.valid: return False
        located = self.environment(candidate)
        if located is None or located[1] is None: return False
        function,env = located
        sorts = [node for node in walk_owners(whole) if node['kind'] == 'call' and node['end'] <= first['start']
                 and node['start'] <= candidate['start'] < candidate['end'] <= node['end']]
        for sort in sorts:
            evidence = self.sorted_prefix(sort,env)
            if evidence is not None:
                return self.certify(candidate,'collection-value-prefix-before-first-direct-callback',function,env,evidence,
                    dict(containing_expression_ast=copy.deepcopy(whole),first_direct_callback_ast=copy.deepcopy(first)))
        return False
