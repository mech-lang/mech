//! Share a checked traversal at repeated phase refusal branches.
use crate::native_artifact::{
    NativeCompileError,
    compiler::{backend, compiled_stack_bytes},
};
use cranelift_codegen::{
    Context,
    control::ControlPlane,
    cursor::{Cursor, FuncCursor},
    ir::{
        AbiParam, Block, BlockArg, Function, Inst, InstBuilder, InstructionData, Layout, MemFlags,
        Opcode, Signature, StackSlot, StackSlotData, StackSlotKind, Value, ValueDef,
        condcodes::IntCC, types,
    },
};
use cranelift_module::{FuncId, Linkage, Module, ModuleError, ModuleReloc, ModuleRelocTarget};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy)]
pub(super) struct Candidate {
    pub guard: Block,
    pub start: Block,
    pub exit: Block,
    pub header: usize,
    pub span: usize,
}

struct Traversal {
    candidate: Candidate,
    blocks: BTreeSet<Block>,
    inputs: Vec<Value>,
    constants: Vec<Value>,
    slots: Vec<StackSlot>,
}

fn targets(f: &Function, inst: Inst) -> &[cranelift_codegen::ir::BlockCall] {
    f.dfg.insts[inst].branch_destination(&f.dfg.jump_tables, &f.dfg.exception_tables)
}

fn inputs(f: &Function, inst: Inst) -> impl Iterator<Item = Value> + '_ {
    f.dfg.inst_args(inst).iter().copied().chain(
        targets(f, inst)
            .iter()
            .flat_map(|edge| edge.args(&f.dfg.value_lists))
            .filter_map(|arg| match arg {
                BlockArg::Value(value) => Some(value),
                _ => None,
            }),
    )
}

fn referenced_slot(data: &InstructionData) -> Option<StackSlot> {
    match *data {
        InstructionData::StackLoad { stack_slot, .. }
        | InstructionData::StackStore { stack_slot, .. } => Some(stack_slot),
        _ => None,
    }
}

fn value_block(f: &Function, value: Value) -> Option<Block> {
    match f.dfg.value_def(value) {
        ValueDef::Param(block, _) => Some(block),
        ValueDef::Result(inst, _) => f.layout.inst_block(inst),
        _ => None,
    }
}

fn plan(f: &Function, candidate: Candidate) -> Option<Traversal> {
    let exits = [candidate.guard, candidate.exit];
    if candidate.start == candidate.guard
        || candidate.guard == candidate.exit
        || [candidate.start, candidate.guard, candidate.exit]
            .iter()
            .any(|&b| !f.layout.is_block_inserted(b))
    {
        return None;
    }
    let mut blocks = BTreeSet::new();
    let mut todo = vec![candidate.start];
    while let Some(block) = todo.pop() {
        if exits.contains(&block) || !blocks.insert(block) {
            continue;
        }
        if blocks.len() > 3072 || f.layout.entry_block() == Some(block) {
            return None;
        }
        let end = f.layout.last_inst(block)?;
        if !matches!(
            f.dfg.insts[end].opcode(),
            Opcode::Jump | Opcode::Brif | Opcode::Return
        ) {
            return None;
        }
        todo.extend(
            targets(f, end)
                .iter()
                .map(|edge| edge.block(&f.dfg.value_lists)),
        );
    }
    let mut arguments = BTreeSet::new();
    let mut constants = BTreeSet::new();
    let mut slots = BTreeSet::new();
    let mut entered = false;
    for block in f.layout.blocks() {
        let inside = blocks.contains(&block);
        for inst in f.layout.block_insts(block) {
            let outgoing = targets(f, inst);
            for edge in outgoing {
                let target = edge.block(&f.dfg.value_lists);
                if !inside && blocks.contains(&target) {
                    if target != candidate.start {
                        return None;
                    }
                    entered = true;
                } else if inside && target == candidate.start {
                    return None;
                }
            }
            for value in inputs(f, inst).map(|v| f.dfg.resolve_aliases(v)) {
                let defined_inside = value_block(f, value).is_some_and(|b| blocks.contains(&b));
                if inside == defined_inside {
                    continue;
                }
                if !inside {
                    // A region result may leave only through a continuation argument.
                    return None;
                }
                if let ValueDef::Result(producer, _) = f.dfg.value_def(value)
                    && matches!(
                        f.dfg.insts[producer].opcode(),
                        Opcode::Iconst | Opcode::F32const | Opcode::F64const
                    )
                {
                    constants.insert(value);
                } else {
                    arguments.insert(value);
                }
            }
            if inside {
                if matches!(
                    f.dfg.insts[inst].opcode(),
                    Opcode::DynamicStackLoad
                        | Opcode::DynamicStackStore
                        | Opcode::DynamicStackAddr
                        | Opcode::ReturnCall
                        | Opcode::ReturnCallIndirect
                        | Opcode::TryCall
                        | Opcode::TryCallIndirect
                        | Opcode::BrTable
                ) {
                    return None;
                }
                slots.extend(referenced_slot(&f.dfg.insts[inst]));
            }
        }
    }
    arguments.extend(f.dfg.block_params(candidate.start));
    let output_bounded = exits.iter().all(|&exit| {
        f.dfg.block_params(exit).len() <= 32
            && f.dfg
                .block_params(exit)
                .iter()
                .all(|&v| f.dfg.value_type(v).bytes() <= 8)
    });
    if !entered || arguments.len() + slots.len() > 40 || !output_bounded {
        return None;
    }
    Some(Traversal {
        candidate,
        blocks,
        inputs: arguments.into_iter().collect(),
        constants: constants.into_iter().collect(),
        slots: slots.into_iter().collect(),
    })
}

impl Traversal {
    fn signature(&self, f: &Function) -> Signature {
        let mut result = Signature::new(f.signature.call_conv);
        result.params = self
            .inputs
            .iter()
            .map(|&v| AbiParam::new(f.dfg.value_type(v)))
            .collect();
        result
            .params
            .extend((0..=self.slots.len()).map(|_| AbiParam::new(types::I64)));
        result
            .returns
            .extend((0..2).map(|_| AbiParam::new(types::I64)));
        result
    }

    fn body(&self, f: &Function, signature: &Signature) -> Function {
        let mut body = f.clone();
        body.layout = Layout::new();
        body.sized_stack_slots = Default::default();
        body.signature = signature.clone();
        let entry = body.dfg.make_block();
        body.layout.append_block(entry);
        let mut params = signature
            .params
            .iter()
            .map(|p| body.dfg.append_block_param(entry, p.value_type));
        let mut map = self
            .inputs
            .iter()
            .copied()
            .zip(params.by_ref())
            .collect::<BTreeMap<_, _>>();
        let slots = self
            .slots
            .iter()
            .copied()
            .zip(params.by_ref())
            .collect::<BTreeMap<_, _>>();
        let packet = params.next().unwrap();
        for &constant in &self.constants {
            let ValueDef::Result(inst, _) = f.dfg.value_def(constant) else {
                unreachable!()
            };
            let copied = body.dfg.clone_inst(inst);
            body.layout.append_inst(copied, entry);
            map.insert(constant, body.dfg.first_result(copied));
        }
        let entry_args = f
            .dfg
            .block_params(self.candidate.start)
            .iter()
            .map(|v| map[v].into())
            .collect::<Vec<_>>();
        let mut pos = FuncCursor::new(&mut body);
        pos.goto_bottom(entry);
        pos.ins().jump(self.candidate.start, &entry_args);
        // The copied entry block retains its own phi parameters.
        for value in f.dfg.block_params(self.candidate.start) {
            map.remove(value);
        }
        let exit_blocks = [self.candidate.guard, self.candidate.exit];
        let stubs = exit_blocks.map(|block| {
            let stub = body.dfg.make_block();
            for &v in f.dfg.block_params(block) {
                body.dfg.append_block_param(stub, f.dfg.value_type(v));
            }
            stub
        });
        for block in f.layout.blocks().filter(|b| self.blocks.contains(b)) {
            body.layout.append_block(block);
            if f.layout.is_cold(block) {
                body.layout.set_cold(block);
            }
            for inst in f.layout.block_insts(block) {
                body.layout.append_inst(inst, block);
                body.dfg.map_inst_values(inst, |value| {
                    if value.as_u32() >= f.dfg.num_values() as u32 {
                        return value;
                    }
                    let original = f.dfg.resolve_aliases(value);
                    map.get(&original).copied().unwrap_or(original)
                });
                match body.dfg.insts[inst] {
                    InstructionData::StackLoad {
                        opcode: Opcode::StackAddr,
                        stack_slot,
                        offset,
                    } => {
                        body.dfg
                            .replace(inst)
                            .iadd_imm(slots[&stack_slot], i64::from(i32::from(offset)));
                    }
                    InstructionData::StackLoad {
                        stack_slot, offset, ..
                    } => {
                        let ty = body.dfg.value_type(body.dfg.first_result(inst));
                        body.dfg.replace(inst).load(
                            ty,
                            MemFlags::new().with_notrap(),
                            slots[&stack_slot],
                            offset,
                        );
                    }
                    InstructionData::StackStore {
                        stack_slot,
                        offset,
                        arg,
                        ..
                    } => {
                        body.dfg.replace(inst).store(
                            MemFlags::new().with_notrap(),
                            arg,
                            slots[&stack_slot],
                            offset,
                        );
                    }
                    InstructionData::MultiAry {
                        opcode: Opcode::Return,
                        ..
                    } => {
                        let [status, origin] = *body.dfg.inst_args(inst) else {
                            unreachable!()
                        };
                        let mut pos = FuncCursor::new(&mut body).at_inst(inst);
                        let relative = pos.ins().iadd_imm(origin, -(self.candidate.header as i64));
                        pos.ins().store(MemFlags::new(), relative, packet, 0);
                        let tag = pos.ins().iconst(types::I64, 2);
                        pos.func.dfg.replace(inst).return_(&[tag, status]);
                    }
                    InstructionData::Jump {
                        opcode,
                        mut destination,
                    } => {
                        retarget(
                            &mut destination,
                            &mut body.dfg.value_lists,
                            exit_blocks,
                            stubs,
                        );
                        body.dfg.insts[inst] = InstructionData::Jump {
                            opcode,
                            destination,
                        };
                    }
                    InstructionData::Brif {
                        opcode,
                        arg,
                        mut blocks,
                    } => {
                        for destination in &mut blocks {
                            retarget(destination, &mut body.dfg.value_lists, exit_blocks, stubs);
                        }
                        body.dfg.insts[inst] = InstructionData::Brif {
                            opcode,
                            arg,
                            blocks,
                        };
                    }
                    _ => {}
                }
            }
        }
        for (tag, stub) in stubs.into_iter().enumerate() {
            body.layout.append_block(stub);
            let words = body.dfg.block_params(stub).to_vec();
            let mut pos = FuncCursor::new(&mut body);
            pos.goto_bottom(stub);
            for (index, value) in words.into_iter().enumerate() {
                pos.ins()
                    .store(MemFlags::new(), value, packet, (index * 8) as i32);
            }
            let tag = pos.ins().iconst(types::I64, tag as i64);
            let zero = pos.ins().iconst(types::I64, 0);
            pos.ins().return_(&[tag, zero]);
        }
        body
    }

    fn replace<M: Module>(
        &self,
        module: &mut M,
        root: &mut Function,
        function: FuncId,
        slot: StackSlot,
    ) {
        let candidate = self.candidate;
        for &block in &self.blocks {
            while let Some(inst) = root.layout.first_inst(block) {
                root.layout.remove_inst(inst);
            }
            if block != candidate.start {
                root.layout.remove_block(block);
            }
        }
        let exits = [candidate.guard, candidate.exit];
        let failure = root.dfg.make_block();
        let cont = root.dfg.make_block();
        let resume = root.dfg.make_block();
        let finish = root.dfg.make_block();
        let mut previous = candidate.start;
        for block in [cont, resume, finish, failure] {
            root.layout.insert_block_after(block, previous);
            previous = block;
        }
        root.layout.set_cold(failure);
        let reference = module.declare_func_in_func(function, root);
        let mut args = self.inputs.clone();
        let mut pos = FuncCursor::new(root);
        pos.goto_bottom(candidate.start);
        for &slot in &self.slots {
            args.push(pos.ins().stack_addr(types::I64, slot, 0));
        }
        args.push(pos.ins().stack_addr(types::I64, slot, 0));
        let call = pos.ins().call(reference, &args);
        let [tag, status] = *pos.func.dfg.inst_results(call) else {
            unreachable!()
        };
        let failed = pos.ins().icmp_imm(IntCC::Equal, tag, 2);
        pos.ins().brif(failed, failure, &[], cont, &[]);
        pos.goto_bottom(failure);
        let origin = pos.ins().stack_load(types::I64, slot, 0);
        let origin = pos.ins().iadd_imm(origin, candidate.header as i64);
        pos.ins().return_(&[status, origin]);
        pos.goto_bottom(cont);
        let repeat = pos.ins().icmp_imm(IntCC::Equal, tag, 0);
        pos.ins().brif(repeat, resume, &[], finish, &[]);
        for (block, exit) in [resume, finish].into_iter().zip(exits) {
            let types = pos
                .func
                .dfg
                .block_params(exit)
                .iter()
                .map(|&v| pos.func.dfg.value_type(v))
                .collect::<Vec<_>>();
            pos.goto_bottom(block);
            let args = types
                .into_iter()
                .enumerate()
                .map(|(i, ty)| pos.ins().stack_load(ty, slot, (i * 8) as i32).into())
                .collect::<Vec<_>>();
            pos.ins().jump(exit, &args);
        }
    }
}

fn retarget(
    edge: &mut cranelift_codegen::ir::BlockCall,
    pool: &mut cranelift_codegen::ir::ValueListPool,
    exits: [Block; 2],
    stubs: [Block; 2],
) {
    if let Some(index) = exits.iter().position(|&block| block == edge.block(pool)) {
        edge.set_block(stubs[index], pool);
    }
}

struct Prepared {
    function: FuncId,
    signature: Signature,
    bytes: Box<[u8]>,
    alignment: u32,
    relocs: Vec<ModuleReloc>,
    stack: u64,
    traversals: Vec<Traversal>,
}

fn same_relocation(a: &ModuleReloc, b: &ModuleReloc) -> bool {
    use ModuleRelocTarget::{FunctionOffset, KnownSymbol, LibCall, User};
    a.offset == b.offset
        && a.kind == b.kind
        && a.addend == b.addend
        && match (&a.name, &b.name) {
            (FunctionOffset(_, a), FunctionOffset(_, b)) => a == b,
            (
                User {
                    namespace: a,
                    index: x,
                },
                User {
                    namespace: b,
                    index: y,
                },
            ) => a == b && x == y,
            (KnownSymbol(a), KnownSymbol(b)) => a == b,
            (LibCall(a), LibCall(b)) => a == b,
            _ => false,
        }
}

pub(super) fn share<M: Module>(
    module: &mut M,
    root: &mut Function,
    families: BTreeMap<FuncId, Vec<Candidate>>,
) -> Result<Option<serde_json::Value>, NativeCompileError> {
    let Some(family) = families
        .into_values()
        .filter(|group| group.len() > 1)
        .max_by_key(|group| (group.iter().map(|t| t.span).sum::<usize>(), group[0].header))
    else {
        return Ok(None);
    };
    let mut groups: Vec<Prepared> = Vec::new();
    let mut proven = BTreeSet::new();
    for candidate in family {
        let Some(traversal) = plan(root, candidate) else {
            eprintln!(
                "compact checked traversal {} declined by boundary proof",
                candidate.header
            );
            continue;
        };
        if !proven.is_disjoint(&traversal.blocks) {
            continue;
        }
        let signature = traversal.signature(root);
        let mut context = Context::for_function(traversal.body(root, &signature));
        context.verify(module.isa()).map_err(backend)?;
        context
            .compile(module.isa(), &mut ControlPlane::default())
            .map_err(ModuleError::from)
            .map_err(backend)?;
        let compiled = context.compiled_code().unwrap();
        let function = module
            .declare_function(
                &format!("mech_compact_checked_traversal_{}", candidate.header),
                Linkage::Local,
                &signature,
            )
            .map_err(backend)?;
        let relocs = compiled
            .buffer
            .relocs()
            .iter()
            .map(|reloc| ModuleReloc::from_mach_reloc(reloc, &context.func, function))
            .collect::<Vec<_>>();
        let bytes = compiled.buffer.data();
        let alignment = compiled.buffer.alignment;
        let stack = compiled_stack_bytes(&compiled.buffer)?;
        eprintln!(
            "compact checked traversal {} {} bytes, {} arguments, {} frame bytes",
            candidate.header,
            bytes.len(),
            signature.params.len(),
            stack
        );
        proven.extend(&traversal.blocks);
        if let Some(winner) = groups.iter_mut().find(|other| {
            other.signature == signature
                && other.alignment == alignment
                && other.bytes.as_ref() == bytes
                && other.relocs.len() == relocs.len()
                && other
                    .relocs
                    .iter()
                    .zip(&relocs)
                    .all(|(a, b)| same_relocation(a, b))
        }) {
            winner.traversals.push(traversal);
        } else {
            groups.push(Prepared {
                function,
                signature,
                bytes: bytes.into(),
                alignment,
                relocs,
                stack,
                traversals: vec![traversal],
            });
        }
    }
    let groups = groups
        .into_iter()
        .filter(|group| group.traversals.len() > 1)
        .collect::<Vec<_>>();
    if groups.is_empty() {
        return Ok(None);
    }
    let words = groups
        .iter()
        .flat_map(|group| &group.traversals)
        .flat_map(|t| [t.candidate.guard, t.candidate.exit])
        .map(|block| root.dfg.block_params(block).len())
        .max()
        .unwrap_or(0)
        .max(1);
    // These calls are synchronous and publish their packet before any
    // continuation begins a later traversal.
    let result = root.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        (words * 8) as u32,
        3,
    ));
    let mut report = Vec::new();
    for selected in groups {
        module
            .define_function_bytes(
                selected.function,
                selected.alignment.into(),
                &selected.bytes,
                &selected.relocs,
            )
            .map_err(backend)?;
        for traversal in &selected.traversals {
            traversal.replace(module, root, selected.function, result);
        }
        report.push(serde_json::json!({
            "headers":selected.traversals.iter().map(|t| t.candidate.header).collect::<Vec<_>>(),
            "code_bytes":selected.bytes.len(),"stack_bytes":selected.stack,
            "input_words":selected.signature.params.len(),
        }));
    }
    Ok((!report.is_empty()).then_some(serde_json::Value::Array(report)))
}
