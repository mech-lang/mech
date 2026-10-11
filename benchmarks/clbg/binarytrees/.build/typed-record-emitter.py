from pathlib import Path
p=Path('hosts/gpu/src/control/mod.rs');s=p.read_text().replace('    arity: u64,','    arity: u64,\n    layout: u64,');s+='''
impl RefArenaContext {
    const BASE: i32 = core::mem::offset_of!(Self, base) as i32;
    const CAPACITY: i32 = core::mem::offset_of!(Self, capacity) as i32;
    const CURSOR: i32 = core::mem::offset_of!(Self, cursor) as i32;
    const LAYOUT: i32 = core::mem::offset_of!(Self, layout) as i32;
}
''';p.write_text(s)
p=Path('hosts/gpu/src/control/runtime.rs');s=p.read_text();s=s.replace('    arity: u64,\n    function: u64,','    arity: u64,\n    layout: u64,\n    function: u64,',1).replace('if capacity == 0 || !(1..=32).contains(&arity) || function == 0 || argument_count > 32','if capacity == 0 || !(1..=32).contains(&arity) || layout == 0 || function == 0 || argument_count > 32',1).replace('        arity,\n    };','        arity,\n        layout,\n    };',1);p.write_text(s)
p=Path('hosts/gpu/src/control/compiler.rs');s=p.read_text().replace('AbiParam, ConstantData, InstBuilder','AbiParam, Block, ConstantData, InstBuilder').replace('heap_functions','native_functions')
s=s.replace('        Kind::Tuple(k) => k.iter().flat_map(types_for).collect(),','        Kind::Tuple(k) => k.iter().flat_map(types_for).collect(),\n        Kind::Record(fields) => fields.iter().flat_map(|(_, k)| types_for(k)).collect(),\n        Kind::Type(_) => vec![],')
s=s.replace('        Op::Text(_) => vec![],','        Op::Text(_) | Op::Type => vec![],')
s=s.replace('            let Kind::Tuple(k) = &e.kind else {\n                unreachable!()\n            };','            let k = field_kinds(&e.kind);').replace('            let Kind::Tuple(kinds) = &value.kind else {\n                unreachable!()\n            };','            let kinds = field_kinds(&value.kind);')
s=s.replace('                let Kind::Tuple(kinds) = &value.kind else {\n                    return Err("tuple field type".into());\n                };','                let kinds = field_kinds(&value.kind);')
s=s.replace('            Op::Text(_) => vec![],','            Op::Text(_) | Op::Type => vec![],')
s=s.replace('    functions: &\'c BTreeMap<String, FuncId>,','''    functions: &'c BTreeMap<String, FuncId>,
    trusted_functions: &'c BTreeMap<String, FuncId>,
    function_layouts: &'c BTreeMap<String, Kind>,
    layouts: &'c BTreeMap<Kind, u64>,
    context_facts: ContextFacts,
    error_block: Option<Block>,''')
s=s.replace('arena_callbacks: &\'c BTreeMap<String, FuncId>', 'arena_callbacks: &\'c BTreeMap<(String, Kind), FuncId>')
# Shared cold errors in new native functions; preserve pure legacy objects.
marker='    fn guard(&mut self, valid: Value, status: i64) {'
s=s.replace(marker,'''    fn error_target(&mut self) -> Block {
        if let Some(block) = self.error_block { return block; }
        let block = self.b.create_block();
        self.b.append_block_param(block, types::I64);
        self.b.set_cold_block(block);
        self.error_block = Some(block);
        block
    }
    fn guard(&mut self, valid: Value, status: i64) {
        if self.program.native_functions {
            let good = self.b.create_block();
            let bad = self.error_target();
            let error = self.b.ins().iconst(types::I64, status);
            self.b.ins().brif(valid, good, &[], bad, &[error.into()]);
            self.b.switch_to_block(good);
            return;
        }
''',1)
marker='    fn status(&mut self, status: Value) {'
s=s.replace(marker,'''    fn status(&mut self, status: Value) {
        if self.program.native_functions {
            let good = self.b.create_block();
            let bad = self.error_target();
            let valid = self.b.ins().icmp_imm(IntCC::Equal, status, 0);
            self.b.ins().brif(valid, good, &[], bad, &[status.into()]);
            self.b.switch_to_block(good);
            return;
        }
''',1)
# Only a proven matching live context may select a trusted same-object entry.
s=s.replace('        if let Some(&id) = self.functions.get(name) {','''        if let Some(&normal) = self.functions.get(name) {
            let id = if self.context_facts.valid
                && self.context_facts.layout.as_ref() == self.function_layouts.get(name)
            { self.trusted_functions.get(name).copied().unwrap_or(normal) } else { normal };''',1)
# Arena validity and layout proofs are scoped to dominating control flow.
marker='    fn expression(\n'
extra='''    fn check_context(&mut self) {
        if !self.context_facts.valid {
            let valid = self.b.ins().icmp_imm(IntCC::NotEqual, self.arena_context, 0);
            self.guard(valid, 9);
            self.context_facts.valid = true;
        }
    }
    fn check_layout(&mut self, layout: &Kind) {
        self.check_context();
        if self.context_facts.layout.as_ref() != Some(layout) {
            let actual = self.b.ins().load(types::I64, MemFlags::new(), self.arena_context, super::RefArenaContext::LAYOUT);
            let valid = self.b.ins().icmp_imm(IntCC::Equal, actual, self.layouts[layout] as i64);
            self.guard(valid, 9);
            self.context_facts.layout = Some(layout.clone());
        }
    }
    fn arena_base(&mut self) -> Value {
        if let Some(value) = self.context_facts.base { return value; }
        let value = self.b.ins().load(types::I64, MemFlags::new(), self.arena_context, super::RefArenaContext::BASE);
        self.context_facts.base = Some(value);
        value
    }
    fn arena_capacity(&mut self) -> Value {
        if let Some(value) = self.context_facts.capacity { return value; }
        let value = self.b.ins().load(types::I64, MemFlags::new(), self.arena_context, super::RefArenaContext::CAPACITY);
        self.context_facts.capacity = Some(value);
        value
    }
'''
s=s.replace(marker,extra+marker,1)
# Replace the context checks; layout identity also proves exact lane arity.
a=s.index('            Op::Heap {',s.index('    fn expression'));b=s.index('            Op::Arena {',a);piece=s[a:b]
piece=piece.replace('                arity,\n                ..','                arity,\n                layout,\n                ..',1)
a2=piece.index('                    let valid =');b2=piece.index('                    let mut values',a2)
piece=piece[:a2]+'                    self.check_layout(layout.as_ref().ok_or("record layout")?);\n'+piece[b2:]
a2=piece.index('                    let base =');b2=piece.index('                    let cursor =',a2)
piece=piece[:a2]+'                    let base = self.arena_base();\n'+piece[b2:]
piece=piece.replace('self.arena_context, 16','self.arena_context, super::RefArenaContext::CURSOR').replace('self.arena_context,\n                        16,','self.arena_context,\n                        super::RefArenaContext::CURSOR,')
a2=piece.index('                        let capacity =');b2=piece.index('                        let valid =',a2)
piece=piece[:a2]+'                        let capacity = self.arena_capacity();\n'+piece[b2:]
piece=piece.replace('                            self.b.ins().store(','                            let value = self.encoded_scalar(*value);\n                            self.b.ins().store(').replace('                                *value,','                                value,')
a2=piece.index('                        let nonnull =');b2=piece.index('                        let offset =',a2)
piece=piece[:a2]+'''                        let index = self.b.ins().iadd_imm(handle, -1);
                        let valid = self.b.ins().icmp(IntCC::UnsignedLessThan, index, cursor);
                        self.guard(valid, 9);
'''+piece[b2:]
a2=piece.index('                        (0..*arity)');b2=piece.index('                    } else {',a2)
piece=piece[:a2]+'''                        types_for(&e.kind).into_iter().enumerate().map(|(i, kind)| {
                            let bits = self.b.ins().load(types::I64, MemFlags::new(), address, (i * 8) as i32);
                            self.decoded_scalar(bits, kind)
                        }).collect()
'''+piece[b2:]
s=s[:a]+piece+s[b:]
a=s.index('            Op::Arena {',s.index('    fn expression'));b=s.index('            Op::Print {',a);piece=s[a:b]
piece=piece.replace('                arity,\n                function,','                arity,\n                layout,\n                function,',1)
piece=piece.replace('                        self.b\n                            .ins()\n                            .store(MemFlags::new(), *value, input, (i * 8) as i32);','                        let value = self.encoded_scalar(*value);\n                        self.b.ins().store(MemFlags::new(), value, input, (i * 8) as i32);')
piece=piece.replace('self.arena_callbacks[function]','self.arena_callbacks[&(function.clone(), layout.clone())]')
piece=piece.replace('let arity = self.int(*arity as u64);','let layout_id = self.int(self.layouts[layout]);\n                    let arity = self.int(*arity as u64);')
piece=piece.replace('&[capacity, arity, address, input, count, output]','&[capacity, arity, layout_id, address, input, count, output]')
a2=piece.index('                    let valid = self');b2=piece.index('                    let mark =',a2);piece=piece[:a2]+'                    self.check_context();\n'+piece[b2:]
piece=piece.replace('self.arena_context, 16','self.arena_context, super::RefArenaContext::CURSOR').replace('self.arena_context,\n                        16,','self.arena_context,\n                        super::RefArenaContext::CURSOR,')
s=s[:a]+piece+s[b:]
# Do not let proofs from one branch or a possibly empty loop dominate another.
s=s.replace('                let initial_values = self.expression(initial, params, memo)?;','                let initial_values = self.expression(initial, params, memo)?;\n                let loop_context = self.context_facts.clone();',1)
s=s.replace('                self.b.switch_to_block(exit);\n                self.b\n                    .block_params(exit)','                self.context_facts = loop_context;\n                self.b.switch_to_block(exit);\n                self.b\n                    .block_params(exit)',1)
s=s.replace('                let branch_facts = self.facts.clone();','                let branch_facts = self.facts.clone();\n                let branch_context = self.context_facts.clone();',1)
s=s.replace('                self.facts = branch_facts.clone();','                self.facts = branch_facts.clone();\n                self.context_facts = branch_context.clone();',1)
s=s.replace('                self.facts = branch_facts;','                self.facts = branch_facts;\n                self.context_facts = branch_context;',1)
# Scope wrapper identity includes its exact layout.
s=s.replace('fn arena_names(e: &Expr, out: &mut BTreeSet<String>)','fn arena_names(e: &Expr, out: &mut BTreeSet<(String, Kind)>)')
a=s.index('fn arena_names(');b=s.index('pub fn emit(',a);piece=s[a:b].replace('            function,\n            arguments,','            function,\n            layout,\n            arguments,',1).replace('out.insert(function.clone());','out.insert((function.clone(), layout.clone()));');s=s[:a]+piece+s[b:]
# Required layouts follow same-context calls only. Nested arenas are distinct.
marker='pub fn emit('
extra='''#[derive(Clone, Default)]
struct ContextFacts {
    valid: bool,
    layout: Option<Kind>,
    base: Option<Value>,
    capacity: Option<Value>,
}
fn field_kinds(kind: &Kind) -> Vec<Kind> {
    match kind { Kind::Tuple(kinds) => kinds.clone(),
        Kind::Record(fields) => fields.iter().map(|(_, k)| k.clone()).collect(),
        _ => unreachable!("aggregate field kind") }
}
fn memory_layouts(e: &Expr, program: &Program, active: &mut BTreeSet<String>, out: &mut BTreeSet<Kind>, regions: bool) {
    let visit = |name: &str, active: &mut BTreeSet<String>, out: &mut BTreeSet<Kind>| {
        if active.insert(name.to_owned()) {
            memory_layouts(&program.functions[name].output, program, active, out, regions);
            active.remove(name);
        }
    };
    match &e.op {
        Op::Heap { layout, arguments, .. } => {
            if let Some(layout) = layout { out.insert(layout.clone()); }
            for e in arguments { memory_layouts(e, program, active, out, regions); }
        }
        Op::Arena { capacity, layout, function, arguments, .. } => {
            if let Some(capacity) = capacity {
                if regions { out.insert(layout.clone()); }
                memory_layouts(capacity, program, active, out, regions);
            } else { visit(function, active, out); }
            memory_layouts(arguments, program, active, out, regions);
        }
        Op::Invoke { name, arguments, .. } | Op::Call(name, arguments) => {
            visit(name, active, out);
            for e in arguments { memory_layouts(e, program, active, out, regions); }
        }
        Op::Sequence { effects, output } => {
            for e in effects { memory_layouts(e, program, active, out, regions); }
            memory_layouts(output, program, active, out, regions);
        }
        Op::Tuple(a) | Op::Primitive(_, a) | Op::Print { values: a, .. } => {
            for e in a { memory_layouts(e, program, active, out, regions); }
        }
        Op::Field(e, _) => memory_layouts(e, program, active, out, regions),
        Op::Loop { initial, predicate, step, .. } => {
            memory_layouts(initial, program, active, out, regions); visit(predicate, active, out); visit(step, active, out);
        }
        Op::Branch { condition, state, yes, no, .. } => {
            memory_layouts(condition, program, active, out, regions); memory_layouts(state, program, active, out, regions);
            visit(yes, active, out); visit(no, active, out);
        }
        Op::Parallel { start, end, grain, arguments, .. } => {
            for e in [start, end, grain, arguments] { memory_layouts(e, program, active, out, regions); }
        }
        _ => {}
    }
}
'''
s=s.replace(marker,extra+marker,1)
s=s.replace('sig.params = vec![AbiParam::new(types::I64); 6];','sig.params = vec![AbiParam::new(types::I64); 7];',1)
marker='    let mut functions = BTreeMap::new();'
extra='''    let mut all_layouts = BTreeSet::new();
    for e in program.effects.iter().chain(std::iter::once(&program.output)).chain(program.functions.values().map(|f| &f.output)) {
        memory_layouts(e, &program, &mut BTreeSet::new(), &mut all_layouts, true);
    }
    let layouts = all_layouts.into_iter().enumerate().map(|(i, layout)| (layout, i as u64 + 1)).collect::<BTreeMap<_, _>>();
    let mut function_layouts = BTreeMap::new();
    for (name, f) in &program.functions {
        let mut required = BTreeSet::new();
        memory_layouts(&f.output, &program, &mut BTreeSet::from([name.clone()]), &mut required, false);
        if required.len() == 1 { function_layouts.insert(name.clone(), required.into_iter().next().unwrap()); }
    }
    let mut trusted_functions = BTreeMap::new();
'''
s=s.replace(marker,extra+marker,1)
marker='            units.push((Some(name.clone()), id, 4));'
s=s.replace(marker,marker+'''
            if function_layouts.contains_key(name) {
                let trusted = module.declare_function(&format!("mech_control_trusted_{i}"), Linkage::Local, &sig).map_err(|e| e.to_string())?;
                trusted_functions.insert(name.clone(), trusted);
                units.push((Some(name.clone()), trusted, 6));
            }''',1)
s=s.replace('    let mut arena_callbacks = BTreeMap::new();\n    for (i, name) in arena_names_set.into_iter().enumerate() {','    let mut arena_callbacks = BTreeMap::new();\n    let mut arena_unit_layouts = BTreeMap::new();\n    for (i, (name, layout)) in arena_names_set.into_iter().enumerate() {')
s=s.replace('        arena_callbacks.insert(name.clone(), id);','        arena_callbacks.insert((name.clone(), layout.clone()), id);\n        arena_unit_layouts.insert(id, layout);',1)
s=s.replace('        let is_function = scan_stage == 4;','        let is_function = scan_stage == 4 || scan_stage == 6;')
s=s.replace('            functions: &functions,','''            functions: &functions,
            trusted_functions: &trusted_functions,
            function_layouts: &function_layouts,
            layouts: &layouts,
            context_facts: ContextFacts {
                valid: is_arena || scan_stage == 6,
                layout: if is_arena { Some(arena_unit_layouts[&id].clone()) }
                    else if scan_stage == 6 { Some(function_layouts[name.as_ref().unwrap()].clone()) } else { None },
                ..ContextFacts::default()
            },
            error_block: None,''',1)
# Decode source callback scalar arguments from initialized u64 ABI slots.
s=s.replace('                .map(|i| {\n                    emit.b.ins().load(\n                        types::I64,\n                        MemFlags::new(),\n                        arguments,\n                        (i * 8) as i32,\n                    )\n                })','''                .map(|i| {
                    let bits = emit.b.ins().load(types::I64, MemFlags::new(), arguments, (i * 8) as i32);
                    emit.decoded_scalar(bits, types_for(&program.functions[name].inputs[i])[0])
                })''',1)
# Complete the shared error block after the normal return.
marker='        drop(emit);\n        b.seal_all_blocks();'
s=s.replace(marker,'''        if let Some(error) = emit.error_block {
            emit.b.switch_to_block(error);
            let status = emit.b.block_params(error)[0];
            emit.failure(status);
        }
'''+marker,1)
p.write_text(s)
