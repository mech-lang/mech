//! Share repeated checked regions with explicit continuation boundaries.
use crate::native_artifact::{
    NativeCompileError,
    compiler::{backend, compiled_stack_bytes},
};
use cranelift_codegen::{
    Context,
    control::ControlPlane,
    cursor::{Cursor, FuncCursor},
    dominator_tree::DominatorTree,
    flowgraph::ControlFlowGraph,
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
    pub resume: Option<Block>,
    pub start: Block,
    pub exit: Block,
    pub header: usize,
    pub span: usize,
}

impl Candidate {
    fn exits(self) -> [Option<Block>; 2] {
        [self.resume, Some(self.exit)]
    }
}

pub(super) fn controller_candidates(
    ranges: &str,
    source: &[Option<Block>],
) -> Result<Vec<Candidate>, NativeCompileError> {
    let mut result = Vec::new();
    for pair in ranges.split(',') {
        let (first, last) = pair
            .split_once(':')
            .ok_or_else(|| backend("expected controller entry:continuation"))?;
        let header = first.parse::<usize>().map_err(backend)?;
        let end = last.parse::<usize>().map_err(backend)?;
        if end <= header || end >= source.len() {
            return Err(backend("controller window is outside the requested Plan"));
        }
        let (Some(start), Some(exit)) = (source[header], source[end]) else {
            eprintln!("compact checked controller {header} has an unreachable boundary");
            continue;
        };
        result.push(Candidate {
            resume: None,
            start,
            exit,
            header,
            span: end - header,
        });
    }
    Ok(result)
}

struct Traversal {
    candidate: Candidate,
    blocks: BTreeSet<Block>,
    inputs: Vec<Value>,
    outputs: Vec<Value>,
    constants: Vec<Value>,
    slots: Vec<StackSlot>,
}

#[derive(Clone, Copy)]
struct CallSlots {
    result: StackSlot,
    captures: Option<StackSlot>,
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

fn use_blocks(f: &Function) -> Vec<Vec<Block>> {
    let mut users = vec![Vec::new(); f.dfg.num_values()];
    for block in f.layout.blocks() {
        for inst in f.layout.block_insts(block) {
            for value in inputs(f, inst).map(|v| f.dfg.resolve_aliases(v)) {
                let blocks = &mut users[value.as_u32() as usize];
                if blocks.last() != Some(&block) {
                    blocks.push(block);
                }
            }
        }
    }
    users
}

fn plan(
    f: &Function,
    cfg: &ControlFlowGraph,
    users: &[Vec<Block>],
    dominance: Option<&DominatorTree>,
    candidate: Candidate,
) -> Result<Traversal, &'static str> {
    let exits = candidate.exits();
    let controller = candidate.resume.is_none();
    if exits.contains(&Some(candidate.start))
        || candidate.resume == Some(candidate.exit)
        || [
            Some(candidate.start),
            candidate.resume,
            Some(candidate.exit),
        ]
        .iter()
        .flatten()
        .any(|&b| !f.layout.is_block_inserted(b))
    {
        return Err("entry or continuation did not survive cleanup");
    }
    let mut blocks = BTreeSet::new();
    let mut todo = vec![candidate.start];
    while let Some(block) = todo.pop() {
        if exits.contains(&Some(block)) || !blocks.insert(block) {
            continue;
        }
        let block_limit = if controller { 12_288 } else { 3_072 };
        if blocks.len() > block_limit || f.layout.entry_block() == Some(block) {
            return Err("region exceeded the block budget or reached the function entry");
        }
        let end = f.layout.last_inst(block).ok_or("empty region block")?;
        if !matches!(
            f.dfg.insts[end].opcode(),
            Opcode::Jump | Opcode::Brif | Opcode::Return
        ) {
            return Err("unsupported region terminator");
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
    if !cfg
        .pred_iter(candidate.start)
        .any(|pred| !blocks.contains(&pred.block))
    {
        return Err("region has no external entry");
    }
    let mut outputs = Vec::new();
    let single_exit_dominance = dominance.filter(|_| {
        cfg.pred_iter(candidate.exit)
            .all(|pred| blocks.contains(&pred.block))
    });
    for &block in &blocks {
        let entry = block == candidate.start;
        if !(entry && controller)
            && cfg
                .pred_iter(block)
                .any(|pred| blocks.contains(&pred.block) == entry)
        {
            return Err("edge crosses the region entry boundary");
        }
        let definitions = f.dfg.block_params(block).iter().copied().chain(
            f.layout
                .block_insts(block)
                .flat_map(|inst| f.dfg.inst_results(inst).iter().copied()),
        );
        for value in definitions.filter(|value| {
            users[value.as_u32() as usize]
                .iter()
                .any(|user| !blocks.contains(user))
        }) {
            // Direct SSA uses can be repaired only on the single continuation
            // dominated by the old definition and entered solely by this region.
            if !controller
                || !single_exit_dominance
                    .is_some_and(|dom| dom.dominates(block, candidate.exit, &f.layout))
            {
                return Err("value escapes without a dominated continuation");
            }
            outputs.push(value);
        }
        for inst in f.layout.block_insts(block) {
            for value in inputs(f, inst)
                .map(|v| f.dfg.resolve_aliases(v))
                .filter(|&v| !value_block(f, v).is_some_and(|b| blocks.contains(&b)))
            {
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
                return Err("unsupported region instruction");
            }
            slots.extend(referenced_slot(&f.dfg.insts[inst]));
        }
    }
    arguments.extend(f.dfg.block_params(candidate.start));
    let (input_limit, output_limit) = if controller { (256, 256) } else { (40, 32) };
    let input_words = arguments.len() + slots.len();
    let finish = f.dfg.block_params(candidate.exit);
    let output_words = finish.len() + outputs.len();
    if controller {
        eprintln!(
            "compact checked controller {} boundary: {} blocks, {} inputs, {} outputs",
            candidate.header,
            blocks.len(),
            input_words,
            output_words,
        );
    }
    if input_words > input_limit
        || controller
            && arguments
                .iter()
                .any(|&value| f.dfg.value_type(value).bytes() > 8)
    {
        return Err("live inputs exceed the call boundary budget");
    }
    if output_words > output_limit
        || candidate
            .resume
            .is_some_and(|resume| f.dfg.block_params(resume).len() > output_limit)
        || exits
            .iter()
            .flatten()
            .flat_map(|&exit| f.dfg.block_params(exit))
            .chain(&outputs)
            .any(|&value| f.dfg.value_type(value).bytes() > 8)
    {
        return Err("live outputs exceed the call boundary budget");
    }
    Ok(Traversal {
        candidate,
        blocks,
        inputs: arguments.into_iter().collect(),
        outputs,
        constants: constants.into_iter().collect(),
        slots: slots.into_iter().collect(),
    })
}

impl Traversal {
    fn capture_words(&self) -> usize {
        self.inputs.len() + self.slots.len()
    }

    fn signature(&self, f: &Function) -> Signature {
        let mut result = Signature::new(f.signature.call_conv);
        if self.candidate.resume.is_none() {
            result.params = vec![AbiParam::new(types::I64); 2];
        } else {
            result.params = self
                .inputs
                .iter()
                .map(|&v| AbiParam::new(f.dfg.value_type(v)))
                .collect();
            result
                .params
                .extend((0..=self.slots.len()).map(|_| AbiParam::new(types::I64)));
        }
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
            .map(|p| body.dfg.append_block_param(entry, p.value_type))
            .collect::<Vec<_>>();
        let packet = params.pop().unwrap();
        if self.candidate.resume.is_none() {
            let capture = params[0];
            let capture_types = self
                .inputs
                .iter()
                .map(|&v| f.dfg.value_type(v))
                .chain(std::iter::repeat_n(types::I64, self.slots.len()));
            let mut pos = FuncCursor::new(&mut body);
            pos.goto_bottom(entry);
            params = capture_types
                .enumerate()
                .map(|(index, ty)| {
                    pos.ins().load(
                        ty,
                        MemFlags::new().with_notrap(),
                        capture,
                        (index * 8) as i32,
                    )
                })
                .collect();
        }
        let mut params = params.into_iter();
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
        let exit_blocks = self.candidate.exits();
        let stubs = exit_blocks.map(|exit| {
            exit.map(|block| {
                let stub = body.dfg.make_block();
                for &v in f.dfg.block_params(block) {
                    body.dfg.append_block_param(stub, f.dfg.value_type(v));
                }
                stub
            })
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
                        let base = self.candidate.header as i64;
                        let relative = match pos.func.dfg.value_def(origin) {
                            ValueDef::Result(producer, _) => match pos.func.dfg.insts[producer] {
                                InstructionData::UnaryImm {
                                    opcode: Opcode::Iconst,
                                    imm,
                                } => pos
                                    .ins()
                                    .iconst(types::I64, i64::from(imm).wrapping_sub(base)),
                                _ => pos.ins().iadd_imm(origin, -base),
                            },
                            _ => pos.ins().iadd_imm(origin, -base),
                        };
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
            let Some(stub) = stub else { continue };
            body.layout.append_block(stub);
            let mut words = body.dfg.block_params(stub).to_vec();
            if tag == 1 {
                words.extend(&self.outputs);
            }
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
        packets: CallSlots,
        observer: Option<FuncId>,
        cold: bool,
    ) {
        let candidate = self.candidate;
        let mut args = self.inputs.clone();
        for arg in &mut args {
            if self.outputs.contains(arg) {
                // The incoming value may be different from the one leaving the
                // controller after a backedge. Keep an independent entry parameter.
                let ty = root.dfg.value_type(*arg);
                *arg = root.dfg.replace_block_param(*arg, ty);
            }
        }
        for &block in &self.blocks {
            while let Some(inst) = root.layout.first_inst(block) {
                if !self.outputs.is_empty() {
                    root.dfg.detach_inst_results(inst);
                }
                root.layout.remove_inst(inst);
            }
            if block != candidate.start {
                if !self.outputs.is_empty() {
                    root.dfg.detach_block_params(block);
                }
                root.layout.remove_block(block);
            }
        }
        let failure = root.dfg.make_block();
        let cont = root.dfg.make_block();
        let arms = candidate
            .resume
            .map(|exit| (root.dfg.make_block(), root.dfg.make_block(), exit));
        let finish = arms.map_or(cont, |(_, finish, _)| finish);
        let continuations = [
            arms.map(|(resume, _, exit)| (resume, exit)),
            Some((finish, candidate.exit)),
        ];
        let mut previous = candidate.start;
        for block in [
            Some(cont),
            arms.map(|(resume, _, _)| resume),
            arms.map(|(_, finish, _)| finish),
            Some(failure),
        ]
        .into_iter()
        .flatten()
        {
            root.layout.insert_block_after(block, previous);
            previous = block;
        }
        root.layout.set_cold(failure);
        if cold {
            for block in [candidate.start, cont] {
                root.layout.set_cold(block);
            }
            if let Some((resume, finish, _)) = arms {
                root.layout.set_cold(resume);
                root.layout.set_cold(finish);
            }
        }
        let reference = module.declare_func_in_func(function, root);
        let observer = observer.map(|id| module.declare_func_in_func(id, root));
        let mut pos = FuncCursor::new(root);
        pos.goto_bottom(candidate.start);
        if let Some(observer) = observer {
            let header = pos.ins().iconst(types::I64, candidate.header as i64);
            pos.ins().call(observer, &[header]);
        }
        for &slot in &self.slots {
            args.push(pos.ins().stack_addr(types::I64, slot, 0));
        }
        if let Some(capture) = packets.captures {
            for (index, arg) in args.drain(..).enumerate() {
                pos.ins().stack_store(arg, capture, (index * 8) as i32);
            }
            args.push(pos.ins().stack_addr(types::I64, capture, 0));
        }
        args.push(pos.ins().stack_addr(types::I64, packets.result, 0));
        let call = pos.ins().call(reference, &args);
        let [tag, status] = *pos.func.dfg.inst_results(call) else {
            unreachable!()
        };
        let failed = pos.ins().icmp_imm(IntCC::Equal, tag, 2);
        pos.ins().brif(failed, failure, &[], cont, &[]);
        pos.goto_bottom(failure);
        let origin = pos.ins().stack_load(types::I64, packets.result, 0);
        let origin = pos.ins().iadd_imm(origin, candidate.header as i64);
        pos.ins().return_(&[status, origin]);
        if let Some((resume, finish, _)) = arms {
            pos.goto_bottom(cont);
            let repeat = pos.ins().icmp_imm(IntCC::Equal, tag, 0);
            pos.ins().brif(repeat, resume, &[], finish, &[]);
        }
        for (block, exit) in continuations.into_iter().flatten() {
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
                .map(|(i, ty)| {
                    pos.ins()
                        .stack_load(ty, packets.result, (i * 8) as i32)
                        .into()
                })
                .collect::<Vec<_>>();
            if exit == candidate.exit {
                for (index, &value) in self.outputs.iter().enumerate() {
                    let ty = pos.func.dfg.value_type(value);
                    let offset = ((args.len() + index) * 8) as i32;
                    let loaded = pos.ins().stack_load(ty, packets.result, offset);
                    pos.func.dfg.change_to_alias(value, loaded);
                }
            }
            pos.ins().jump(exit, &args);
        }
    }
}

fn retarget(
    edge: &mut cranelift_codegen::ir::BlockCall,
    pool: &mut cranelift_codegen::ir::ValueListPool,
    exits: [Option<Block>; 2],
    stubs: [Option<Block>; 2],
) {
    if let Some(index) = exits
        .iter()
        .position(|&block| block == Some(edge.block(pool)))
    {
        edge.set_block(stubs[index].unwrap(), pool);
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
    all_families: bool,
) -> Result<Option<serde_json::Value>, NativeCompileError> {
    let mut families = families
        .into_values()
        .filter(|group| group.len() > 1)
        .collect::<Vec<_>>();
    families.sort_by_key(|group| {
        std::cmp::Reverse((group.iter().map(|t| t.span).sum::<usize>(), group[0].header))
    });
    if !all_families {
        families.truncate(1);
    }
    share_families(module, root, families, true)
}

pub(super) fn share_controllers<M: Module>(
    module: &mut M,
    root: &mut Function,
    candidates: Vec<Candidate>,
) -> Result<Option<serde_json::Value>, NativeCompileError> {
    if candidates.len() < 2 {
        return Ok(None);
    }
    share_families(module, root, vec![candidates], false)
}

fn share_families<M: Module>(
    module: &mut M,
    root: &mut Function,
    families: Vec<Vec<Candidate>>,
    refusal_regions: bool,
) -> Result<Option<serde_json::Value>, NativeCompileError> {
    if families.is_empty() {
        return Ok(None);
    }
    let excluded = refusal_regions
        .then(|| std::env::var("MECH_COMPACT_TRAVERSAL_SKIP_HEADERS").ok())
        .flatten()
        .map(|list| {
            list.split(',')
                .map(|part| part.parse::<usize>().map_err(backend))
                .collect::<Result<BTreeSet<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();
    let mut groups: Vec<Prepared> = Vec::new();
    let mut proven = BTreeSet::new();
    let cfg = ControlFlowGraph::with_function(root);
    let users = use_blocks(root);
    let dominance = (!refusal_regions).then(|| DominatorTree::with_function(root, &cfg));
    for candidate in families.into_iter().flatten() {
        if excluded.contains(&candidate.header) {
            continue;
        }
        let traversal = match plan(root, &cfg, &users, dominance.as_ref(), candidate) {
            Ok(traversal) => traversal,
            Err(reason) => {
                eprintln!(
                    "compact checked region {} declined: {reason}",
                    candidate.header
                );
                continue;
            }
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
        let kind = if refusal_regions {
            "traversal"
        } else {
            "controller"
        };
        let function = module
            .declare_function(
                &format!("mech_compact_checked_{kind}_{}", candidate.header),
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
            "compact checked {kind} {} {} bytes, {} arguments, {} frame bytes",
            candidate.header,
            bytes.len(),
            signature.params.len(),
            stack
        );
        if !refusal_regions
            && let Some(folder) = std::env::var_os("MECH_COMPACT_DUMP_CONTROLLER_MATCHES")
        {
            let folder = std::path::PathBuf::from(folder);
            std::fs::create_dir_all(&folder).map_err(backend)?;
            let stem = candidate.header.to_string();
            std::fs::write(
                folder.join(format!("{stem}.clif")),
                context.func.to_string(),
            )
            .map_err(backend)?;
            std::fs::write(folder.join(format!("{stem}.bin")), bytes).map_err(backend)?;
            let relocations = relocs
                .iter()
                .map(|r| format!("{} {:?} {:?} {}", r.offset, r.kind, r.name, r.addend,))
                .collect::<Vec<_>>()
                .join("\n");
            std::fs::write(folder.join(format!("{stem}.relocs")), relocations).map_err(backend)?;
        }
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
    drop((cfg, users, dominance));
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
        .flat_map(|t| {
            let finish = root.dfg.block_params(t.candidate.exit).len() + t.outputs.len();
            let resume = t
                .candidate
                .resume
                .map_or(0, |block| root.dfg.block_params(block).len());
            [finish, resume]
        })
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
    let captures = (!refusal_regions).then(|| {
        let words = groups
            .iter()
            .flat_map(|group| &group.traversals)
            .map(Traversal::capture_words)
            .max()
            .unwrap_or(0)
            .max(1);
        root.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            (words * 8) as u32,
            3,
        ))
    });
    let packets = CallSlots { result, captures };
    let observer = (refusal_regions
        && std::env::var_os("MECH_COMPACT_OBSERVE_CHECKED_TRAVERSALS").is_some())
    .then_some(())
    .map(|_| {
        let mut signature = module.make_signature();
        signature.params.push(AbiParam::new(types::I64));
        module
            .declare_function(
                "mech_compact_checked_traversal_observe",
                Linkage::Import,
                &signature,
            )
            .map_err(backend)
    })
    .transpose()?;
    let cold = refusal_regions
        && std::env::var_os("MECH_COMPACT_CHECKED_TRAVERSALS_COLD_LAYOUT").is_some();
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
            traversal.replace(module, root, selected.function, packets, observer, cold);
        }
        report.push(serde_json::json!({
            "headers":selected.traversals.iter().map(|t| t.candidate.header).collect::<Vec<_>>(),
            "code_bytes":selected.bytes.len(),"stack_bytes":selected.stack,
            "input_words":selected.signature.params.len(),
        }));
    }
    Ok((!report.is_empty()).then_some(serde_json::Value::Array(report)))
}

#[cfg(test)]
#[path = "checked_traversal_tests.rs"]
mod tests;
