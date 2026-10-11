from pathlib import Path
p=Path('hosts/gpu/src/control/compiler.rs');s=p.read_text().replace('use std::{collections::BTreeMap, path::Path};','use std::{collections::{BTreeMap, BTreeSet}, path::Path};')
s=s.replace('    heap: Option<FuncId>,','    arena: Option<FuncId>,\n    arena_callbacks: &\'c BTreeMap<String, FuncId>,\n    arena_context: Value,\n    active_inline: BTreeSet<String>,')
s=s.replace('        | Op::Heap { .. }\n','        | Op::Heap { .. }\n        | Op::Arena { .. }\n',1)
s=s.replace('        self.b.ins().return_(&[error]);','        self.failure(error);',1).replace('        self.b.ins().return_(&[status]);','        self.failure(status);',1)
a=s.index('    fn call(&mut self, name: &str, params: &[Value])');b=s.index('    fn status(&mut self, status: Value)',a)
s=s[:a]+'''    fn failure(&mut self, status: Value) {
        let kinds: Vec<_> = self.b.func.signature.returns.iter().skip(1).map(|p| p.value_type).collect();
        let mut values = vec![status];
        for kind in kinds {
            values.push(if kind == types::F64 { self.b.ins().f64const(0.0) }
                else { self.b.ins().iconst(kind, 0) });
        }
        self.b.ins().return_(&values);
    }
    fn call(&mut self, name: &str, params: &[Value]) -> Result<Values, String> {
        let f = self.program.functions.get(name).ok_or("missing native function")?;
        if self.functions.contains_key(name)
            && !self.active_inline.contains(name)
            && std::env::var("MECH_CONTROL_INLINE").as_deref() != Ok("0")
        {
            self.active_inline.insert(name.to_owned());
            let result = self.expression(&f.output, params, &mut BTreeMap::new());
            self.active_inline.remove(name);
            return result;
        }
        if let Some(&id) = self.functions.get(name) {
            let kinds = types_for(&f.output.kind);
            let register = register_result(&f.output.kind);
            let output = if register { None } else { Some(self.slot((kinds.len() * 16).max(16))) };
            let depth = self.b.ins().iadd_imm(self.depth, 1);
            let mut args = vec![depth, self.arena_context];
            args.extend_from_slice(params);
            if let Some(output) = output { args.push(output); }
            let callee = self.module.declare_func_in_func(id, self.b.func);
            let call = self.b.ins().call(callee, &args);
            let results = self.b.inst_results(call).to_vec();
            self.status(results[0]);
            if register { Ok(vec![results[1]]) }
            else { Ok(kinds.into_iter().enumerate().map(|(i, kind)| {
                self.b.ins().load(kind, MemFlags::new(), output.unwrap(), (i * 16) as i32)
            }).collect()) }
        } else {
            self.expression(&f.output, params, &mut BTreeMap::new())
        }
    }
'''+s[b:]
a=s.index('            Op::Heap {',s.index('    fn expression'));b=s.index('            Op::Print {',a)
s=s[:a]+'''            Op::Heap { action, arguments, arity, .. } => {
                if *action == 0 { vec![self.int(0)] }
                else {
                    let valid = self.b.ins().icmp_imm(IntCC::NotEqual, self.arena_context, 0);
                    self.guard(valid, 9);
                    let actual = self.b.ins().load(types::I64, MemFlags::new(), self.arena_context, 24);
                    let valid = self.b.ins().icmp_imm(IntCC::Equal, actual, *arity as i64);
                    self.guard(valid, 9);
                    let mut values = Vec::new();
                    for arg in arguments { values.extend(self.expression(arg, params, memo)?); }
                    let base = self.b.ins().load(types::I64, MemFlags::new(), self.arena_context, 0);
                    let cursor = self.b.ins().load(types::I64, MemFlags::new(), self.arena_context, 16);
                    if *action == 1 {
                        let capacity = self.b.ins().load(types::I64, MemFlags::new(), self.arena_context, 8);
                        let valid = self.b.ins().icmp(IntCC::UnsignedLessThan, cursor, capacity);
                        self.guard(valid, 9);
                        // Every Ref originates from null, an allocation, or a
                        // checked read in this live region. Scalar-only scope
                        // boundaries prevent cross-region and retired references.
                        let offset = self.b.ins().imul_imm(cursor, (*arity * 8) as i64);
                        let address = self.b.ins().iadd(base, offset);
                        for (i, value) in values.iter().enumerate() {
                            self.b.ins().store(MemFlags::new(), *value, address, (i * 8) as i32);
                        }
                        let handle = self.b.ins().iadd_imm(cursor, 1);
                        self.b.ins().store(MemFlags::new(), handle, self.arena_context, 16);
                        vec![handle]
                    } else if *action == 2 {
                        let handle = values[0];
                        let nonnull = self.b.ins().icmp_imm(IntCC::NotEqual, handle, 0);
                        let inbounds = self.b.ins().icmp(IntCC::UnsignedLessThanOrEqual, handle, cursor);
                        let valid = self.b.ins().band(nonnull, inbounds);
                        self.guard(valid, 9);
                        let index = self.b.ins().iadd_imm(handle, -1);
                        let offset = self.b.ins().imul_imm(index, (*arity * 8) as i64);
                        let address = self.b.ins().iadd(base, offset);
                        (0..*arity).map(|i| self.b.ins().load(types::I64, MemFlags::new(), address, (i * 8) as i32)).collect()
                    } else { return Err("unsupported reference record action".into()); }
                }
            }
            Op::Arena { capacity, arity, function, arguments, .. } => {
                let args = self.expression(arguments, params, memo)?;
                if let Some(capacity) = capacity {
                    let capacity = self.expression(capacity, params, memo)?[0];
                    let input = self.slot((args.len() * 8).max(16));
                    for (i, value) in args.iter().enumerate() {
                        self.b.ins().store(MemFlags::new(), *value, input, (i * 8) as i32);
                    }
                    let output = self.slot((types_for(&e.kind).len() * 8).max(16));
                    let arity = self.int(*arity as u64);
                    let count = self.int(args.len() as u64);
                    let callback = self.module.declare_func_in_func(self.arena_callbacks[function], self.b.func);
                    let address = self.b.ins().func_addr(types::I64, callback);
                    let callee = self.module.declare_func_in_func(self.arena.ok_or("missing arena ABI")?, self.b.func);
                    let call = self.b.ins().call(callee, &[capacity, arity, address, input, count, output]);
                    let status = self.b.inst_results(call)[0];
                    self.status(status);
                    types_for(&e.kind).iter().enumerate().map(|(i, kind)|
                        self.b.ins().load(*kind, MemFlags::new(), output, (i * 8) as i32)).collect()
                } else {
                    let valid = self.b.ins().icmp_imm(IntCC::NotEqual, self.arena_context, 0);
                    self.guard(valid, 9);
                    let mark = self.b.ins().load(types::I64, MemFlags::new(), self.arena_context, 16);
                    let values = self.call(function, &args)?;
                    self.b.ins().store(MemFlags::new(), mark, self.arena_context, 16);
                    values
                }
            }
'''+s[b:]
# Each traversal handles scope expressions.
s=s.replace('        Op::Field(e, _) => flatten_parameters(e, inputs),','''        Op::Arena { capacity, arguments, .. } => {
            if let Some(e) = capacity { flatten_parameters(e, inputs); }
            flatten_parameters(arguments, inputs);
        }
        Op::Field(e, _) => flatten_parameters(e, inputs),''')
s=s.replace('        Op::Field(e, _) => parallel_names(e, out),','''        Op::Arena { capacity, arguments, .. } => {
            if let Some(e) = capacity { parallel_names(e, out); }
            parallel_names(arguments, out);
        }
        Op::Field(e, _) => parallel_names(e, out),''')
s=s.replace('        Op::Field(e, _) => scan_names(e, out),','''        Op::Arena { capacity, arguments, .. } => {
            if let Some(e) = capacity { scan_names(e, out); }
            scan_names(arguments, out);
        }
        Op::Field(e, _) => scan_names(e, out),''')
# Scope wrappers use a complete walk (including conditional/loop states).
marker='pub fn emit(source: &str, directory: &Path, simd: bool)'
extra='''fn register_result(kind: &Kind) -> bool {
    let kinds = types_for(kind);
    kinds.len() == 1 && matches!(kinds[0], types::I64 | types::F64 | types::I8)
}
fn arena_names(e: &Expr, out: &mut BTreeSet<String>) {
    match &e.op {
        Op::Arena { capacity, function, arguments, .. } => {
            if let Some(capacity) = capacity { out.insert(function.clone()); arena_names(capacity, out); }
            arena_names(arguments, out);
        }
        Op::Sequence { effects, output } => {
            for e in effects { arena_names(e, out); }
            arena_names(output, out);
        }
        Op::Tuple(a) | Op::Call(_, a) | Op::Primitive(_, a)
        | Op::Invoke { arguments: a, .. } | Op::Heap { arguments: a, .. }
        | Op::Print { values: a, .. } => { for e in a { arena_names(e, out); } }
        Op::Field(e, _) | Op::Loop { initial: e, .. } => arena_names(e, out),
        Op::Branch { condition, state, .. } => { arena_names(condition, out); arena_names(state, out); }
        Op::Parallel { start, end, grain, arguments, .. } => {
            for e in [start, end, grain, arguments] { arena_names(e, out); }
        }
        _ => {}
    }
}
'''
s=s.replace(marker,extra+marker)
s=s.replace('let (heap, print) = if program.heap_functions {','let (arena, print) = if program.heap_functions {')
s=s.replace('sig.params = vec![AbiParam::new(types::I64); 5];','sig.params = vec![AbiParam::new(types::I64); 6];')
s=s.replace('let heap = module\n            .declare_function("mech_control_heap"','let arena = module\n            .declare_function("mech_control_ref_arena"')
s=s.replace('        sig.params.pop();\n        let print','        sig.params.truncate(4);\n        let print').replace('(Some(heap), Some(print))','(Some(arena), Some(print))')
s=s.replace('            sig.params.push(AbiParam::new(types::I64)); // checked recursion depth','            sig.params.push(AbiParam::new(types::I64)); // checked recursion depth\n            sig.params.push(AbiParam::new(types::I64)); // exclusive live arena context')
s=s.replace('            sig.params.push(AbiParam::new(types::I64)); // exclusive output storage\n            sig.returns.push(AbiParam::new(types::I64));','''            if !register_result(&f.output.kind) { sig.params.push(AbiParam::new(types::I64)); }
            sig.returns.push(AbiParam::new(types::I64));
            if register_result(&f.output.kind) { sig.returns.push(AbiParam::new(types_for(&f.output.kind)[0])); }''')
marker='    units.push((None, main, 0));'
s=s.replace(marker,'''    let mut arena_names_set = BTreeSet::new();
    arena_names(&program.output, &mut arena_names_set);
    for e in &program.effects { arena_names(e, &mut arena_names_set); }
    for f in program.functions.values() { arena_names(&f.output, &mut arena_names_set); }
    let mut arena_callbacks = BTreeMap::new();
    for (i, name) in arena_names_set.into_iter().enumerate() {
        let mut sig = module.make_signature();
        sig.params = vec![AbiParam::new(types::I64); 3];
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function(&format!("mech_control_arena_{i}"), Linkage::Local, &sig).map_err(|e| e.to_string())?;
        arena_callbacks.insert(name.clone(), id);
        units.push((Some(name), id, 5));
    }
'''+marker)
s=s.replace('        let is_function = scan_stage == 4;','''        let is_function = scan_stage == 4;
        let is_arena = scan_stage == 5;
        let register = is_function && register_result(&program.functions[name.as_ref().unwrap()].output.kind);''')
s=s.replace('let arguments = if is_function {','let arguments = if is_function {').replace('        } else if is_scan {\n            p[1]','        } else if is_scan || is_arena {\n            p[1]',1)
s=s.replace('        let output = if is_function {\n            *p.last().unwrap()','        let output = if register {\n            b.ins().iconst(types::I64, 0)\n        } else if is_function {\n            *p.last().unwrap()\n        } else if is_arena {\n            p[2]')
s=s.replace('            heap,','''            arena,
            arena_callbacks: &arena_callbacks,
            arena_context: if is_function { p[1] } else if is_arena { p[0] } else { depth },
            active_inline: if is_function { BTreeSet::from([name.clone().unwrap()]) } else { BTreeSet::new() },''')
s=s.replace('                &p[1..p.len() - 1],','                &p[2..p.len() - usize::from(!register)],')
s=s.replace('        } else if scan_stage == 3 {','''        } else if is_arena {
            let name = name.as_ref().unwrap();
            let count = program.functions[name].inputs.len();
            let args = (0..count).map(|i| emit.b.ins().load(types::I64, MemFlags::new(), arguments, (i * 8) as i32)).collect::<Vec<_>>();
            emit.call(name, &args)?
        } else if scan_stage == 3 {''',1)
s=s.replace('        for (i, v) in values.iter().enumerate() {\n            emit.b.ins().store(', '        if !register {\n        for (i, v) in values.iter().enumerate() {\n            emit.b.ins().store(',1)
s=s.replace('        let zero = emit.int(0);\n        emit.b.ins().return_(&[zero]);','''        }
        let zero = emit.int(0);
        if register { emit.b.ins().return_(&[zero, values[0]]); }
        else { emit.b.ins().return_(&[zero]); }''',1)
p.write_text(s)
