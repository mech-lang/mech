//! Share repeated checked regions with explicit continuation boundaries.
use super::body_relocations::matching_relocations;
use crate::native_artifact::{
    NativeCompileError,
    compiler::{backend, compiled_stack_bytes},
    program::codegen::cfg_cleanup::{controller_passthrough, eliminate_dead_values},
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
use cranelift_module::{FuncId, Linkage, Module, ModuleError, ModuleReloc};
use std::collections::{BTreeMap, BTreeSet};

#[path = "checked_refusal_bounds.rs"]
mod refusal_bounds;
#[path = "checked_region_body.rs"]
mod region_body;
#[path = "checked_region_inputs.rs"]
mod region_inputs;
#[path = "checked_region_literals.rs"]
mod region_literals;
#[path = "checked_region_outputs.rs"]
mod region_outputs;

const MAX_PACKET_WORDS: usize = 256;
const MAX_DIRECT_INPUT_WORDS: usize = 40;
const FIRST_ERROR_TAG: i64 = 2;

#[derive(Clone, Copy)]
pub(super) struct Candidate {
    pub resume: Option<Block>,
    pub start: Block,
    pub exit: Block,
    pub header: usize,
    pub origin_base: Option<u32>,
    pub span: usize,
}

impl Candidate {
    fn origin_base(self) -> i64 {
        self.origin_base.map_or(self.header as i64, i64::from)
    }

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
            origin_base: None,
            span: end - header,
        });
    }
    Ok(result)
}

#[derive(Clone)]
struct Traversal {
    candidate: Candidate,
    blocks: BTreeSet<Block>,
    inputs: Vec<Value>,
    input_roles: BTreeMap<Inst, Vec<Option<usize>>>,
    outputs: Vec<Value>,
    aligned_outputs: Vec<Value>,
    finish: Vec<Value>,
    passthrough: BTreeMap<Value, Value>,
    live_captures: Vec<usize>,
    packed_inputs: bool,
    return_error_origin: bool,
    constants: Vec<Value>,
    slots: Vec<StackSlot>,
    literals: Vec<region_literals::Capture>,
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

fn bounded_relative_origin(f: &Function, origin: Value, base: i64) -> Option<i64> {
    let ValueDef::Result(inst, _) = f.dfg.value_def(f.dfg.resolve_aliases(origin)) else {
        return None;
    };
    let InstructionData::UnaryImm {
        opcode: Opcode::Iconst,
        imm,
    } = f.dfg.insts[inst]
    else {
        return None;
    };
    u32::try_from(i64::from(imm))
        .ok()?
        .checked_sub(u32::try_from(base).ok()?)
        .map(i64::from)
}

fn relative_error_origin(
    pos: &mut FuncCursor<'_>,
    origin: Value,
    base: i64,
    compose_origins: bool,
) -> Value {
    let mut source = origin;
    let mut offset = -base;
    if let ValueDef::Result(producer, _) = pos.func.dfg.value_def(origin) {
        match pos.func.dfg.insts[producer] {
            InstructionData::UnaryImm {
                opcode: Opcode::Iconst,
                imm,
            } => {
                return pos
                    .ins()
                    .iconst(types::I64, imm.bits().wrapping_add(offset));
            }
            InstructionData::BinaryImm64 {
                opcode: Opcode::IaddImm,
                arg,
                imm,
            } if compose_origins => {
                // A nested checked call already restored its origin relative
                // to the caller. Combine the two wrapping translations.
                source = arg;
                offset = offset.wrapping_add(imm.bits());
            }
            _ => {}
        }
    }
    pos.ins().iadd_imm(source, offset)
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

fn region_instructions(f: &Function, start: Block, blocks: &BTreeSet<Block>) -> Vec<Inst> {
    let mut pending = vec![start];
    let mut visited = BTreeSet::new();
    let mut result = Vec::new();
    while let Some(block) = pending.pop() {
        if !blocks.contains(&block) || !visited.insert(block) {
            continue;
        }
        result.extend(f.layout.block_insts(block));
        let inst = f
            .layout
            .last_inst(block)
            .expect("proved region block has a terminator");
        pending.extend(
            targets(f, inst)
                .iter()
                .rev()
                .map(|edge| edge.block(&f.dfg.value_lists)),
        );
    }
    result
}

fn in_use_order<T: Copy + Ord>(
    mut required: BTreeSet<T>,
    uses: impl IntoIterator<Item = T>,
) -> Vec<T> {
    let mut order = Vec::with_capacity(required.len());
    order.extend(uses.into_iter().filter(|value| required.remove(value)));
    order.extend(required);
    order
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RegionKind {
    Refusal,
    Controller,
    Phase,
    Continuation { direct_inputs: usize },
}

impl RegionKind {
    fn is_continuation(self) -> bool {
        matches!(self, Self::Continuation { .. })
    }
}

fn plan(
    f: &Function,
    cfg: &ControlFlowGraph,
    users: &[Vec<Block>],
    dominance: Option<&DominatorTree>,
    candidate: Candidate,
    pack_large_refusals: bool,
    order_captures: bool,
    kind: RegionKind,
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
        for value in definitions {
            let mut outside = users[value.as_u32() as usize]
                .iter()
                .copied()
                .filter(|user| !blocks.contains(user))
                .peekable();
            if outside.peek().is_none() {
                continue;
            }
            // The result packet restores direct uses on the normal exit. If
            // there is also a resume edge, its code must never need that value.
            if !(controller || kind == RegionKind::Phase)
                || !single_exit_dominance.is_some_and(|dom| {
                    dom.dominates(block, candidate.exit, &f.layout)
                        && (controller
                            || outside.all(|user| dom.dominates(candidate.exit, user, &f.layout)))
                })
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
    let input_words = arguments.len() + slots.len();
    let finish = f.dfg.block_params(candidate.exit);
    let output_words = finish.len() + outputs.len();
    let resume_words = candidate
        .resume
        .map_or(0, |resume| f.dfg.block_params(resume).len());
    let direct_continuation = matches!(kind, RegionKind::Continuation { direct_inputs }
        if direct_inputs != 0 && input_words <= direct_inputs && output_words <= 32);
    // A refusal that fits the original bounds keeps its existing native ABI.
    let packed_inputs = (controller && !direct_continuation)
        || kind == RegionKind::Phase
        || pack_large_refusals
            && (input_words > MAX_DIRECT_INPUT_WORDS || output_words.max(resume_words) > 32);
    let (input_limit, output_limit) = if packed_inputs {
        (MAX_PACKET_WORDS, MAX_PACKET_WORDS)
    } else {
        (MAX_DIRECT_INPUT_WORDS, 32)
    };
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
        || packed_inputs
            && arguments
                .iter()
                .any(|&value| f.dfg.value_type(value).bytes() > 8)
    {
        return Err("live inputs exceed the call boundary budget");
    }
    if output_words.max(resume_words) > output_limit
        || exits
            .iter()
            .flatten()
            .flat_map(|&exit| f.dfg.block_params(exit))
            .chain(&outputs)
            .any(|&value| f.dfg.value_type(value).bytes() > 8)
    {
        return Err("live outputs exceed the call boundary budget");
    }
    let passthrough = dominance
        .filter(|_| packed_inputs || direct_continuation)
        .map(|dom| {
            let resume = candidate
                .resume
                .iter()
                .flat_map(|&block| f.dfg.block_params(block));
            controller_passthrough(
                f,
                cfg,
                dom,
                candidate.start,
                &blocks,
                finish.iter().chain(&outputs).chain(resume).copied(),
            )
        })
        .unwrap_or_default();
    let (captures, constants, slots) = if order_captures {
        let instructions = region_instructions(f, candidate.start, &blocks);
        let operands = || {
            f.dfg
                .block_params(candidate.start)
                .iter()
                .copied()
                .chain(instructions.iter().flat_map(|&inst| inputs(f, inst)))
                .map(|value| f.dfg.resolve_aliases(value))
        };
        (
            in_use_order(arguments, operands()),
            in_use_order(constants, operands()),
            in_use_order(
                slots,
                instructions
                    .iter()
                    .filter_map(|&inst| referenced_slot(&f.dfg.insts[inst])),
            ),
        )
    } else {
        (
            arguments.into_iter().collect(),
            constants.into_iter().collect(),
            slots.into_iter().collect(),
        )
    };
    Ok(Traversal {
        candidate,
        blocks,
        inputs: captures,
        input_roles: BTreeMap::new(),
        outputs,
        aligned_outputs: Vec::new(),
        finish: finish.to_vec(),
        passthrough,
        live_captures: Vec::new(),
        packed_inputs,
        return_error_origin: false,
        constants,
        slots,
        literals: Vec::new(),
    })
}

impl Traversal {
    fn has_bounded_error_origins(&self, f: &Function) -> bool {
        let mut returns = self
            .blocks
            .iter()
            .filter_map(|&block| f.layout.last_inst(block))
            .filter(|&inst| f.dfg.insts[inst].opcode() == Opcode::Return)
            .peekable();
        returns.peek().is_some()
            && returns.all(|inst| {
                let [_, origin] = *f.dfg.inst_args(inst) else {
                    return false;
                };
                bounded_relative_origin(f, origin, self.candidate.origin_base()).is_some()
            })
    }

    fn output_words(&self) -> usize {
        if !self.aligned_outputs.is_empty() {
            return self.aligned_outputs.len();
        }
        self.finish
            .iter()
            .chain(&self.outputs)
            .filter(|v| !self.passthrough.contains_key(v))
            .count()
    }

    fn resume_output_words(&self, f: &Function) -> usize {
        self.candidate.resume.map_or(0, |resume| {
            f.dfg
                .block_params(resume)
                .iter()
                .filter(|v| !self.passthrough.contains_key(v))
                .count()
        })
    }

    fn signature(&self, f: &Function) -> Signature {
        let mut result = Signature::new(f.signature.call_conv);
        if self.packed_inputs {
            result.params = vec![AbiParam::new(types::I64); 2];
        } else {
            result.params = self
                .inputs
                .iter()
                .map(|&v| AbiParam::new(f.dfg.value_type(v)))
                .collect();
            result
                .params
                .extend((0..self.slots.len()).map(|_| AbiParam::new(types::I64)));
            result.params.extend(
                self.literals
                    .iter()
                    .map(|capture| AbiParam::new(capture.ty)),
            );
            result.params.push(AbiParam::new(types::I64));
        }
        result
            .returns
            .extend((0..2).map(|_| AbiParam::new(types::I64)));
        result
    }

    fn body(&mut self, f: &Function, signature: &Signature, compose_origins: bool) -> Function {
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
        let mut capture_loads = Vec::new();
        if self.packed_inputs {
            let capture = params[0];
            let capture_types = self
                .inputs
                .iter()
                .map(|&v| f.dfg.value_type(v))
                .chain(std::iter::repeat_n(types::I64, self.slots.len()))
                .chain(self.literals.iter().map(|literal| literal.ty));
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
            capture_loads = params
                .iter()
                .map(|&value| {
                    let ValueDef::Result(inst, _) = body.dfg.value_def(value) else {
                        unreachable!()
                    };
                    inst
                })
                .collect();
        }
        let input_roles = if self.input_roles.is_empty() {
            Vec::new()
        } else {
            params[..self.inputs.len()].to_vec()
        };
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
        map.extend(
            self.literals
                .iter()
                .zip(params)
                .flat_map(|(capture, parameter)| {
                    capture.values.iter().map(move |&value| (value, parameter))
                }),
        );
        for &constant in &self.constants {
            if map.contains_key(&constant) {
                continue;
            }
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
                let mut roles = self.input_roles.get(&inst).into_iter().flatten();
                body.dfg.map_inst_values(inst, |value| {
                    if let Some(Some(index)) = roles.next() {
                        debug_assert_eq!(
                            f.dfg.resolve_aliases(value),
                            f.dfg.resolve_aliases(self.inputs[*index])
                        );
                        return input_roles[*index];
                    }
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
                        let base = self.candidate.origin_base();
                        let tag = if self.return_error_origin {
                            // Zero and one are the two continuations. A proved
                            // relative u32 origin fits above them without wrap.
                            let relative = bounded_relative_origin(pos.func, origin, base)
                                .expect("proved origin must survive region lowering");
                            pos.ins().iconst(types::I64, relative + FIRST_ERROR_TAG)
                        } else {
                            let relative =
                                relative_error_origin(&mut pos, origin, base, compose_origins);
                            pos.ins().store(MemFlags::new(), relative, packet, 0);
                            pos.ins().iconst(types::I64, FIRST_ERROR_TAG)
                        };
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
            let original = exit_blocks[tag].unwrap();
            let params = body.dfg.block_params(stub);
            let words = if tag == 1 && !self.aligned_outputs.is_empty() {
                self.aligned_outputs
                    .iter()
                    .map(|&value| {
                        self.finish
                            .iter()
                            .position(|&formal| formal == value)
                            .map_or(value, |index| params[index])
                    })
                    .collect::<Vec<_>>()
            } else {
                let mut words = params
                    .iter()
                    .copied()
                    .zip(f.dfg.block_params(original))
                    .filter_map(|(param, value)| {
                        (!self.passthrough.contains_key(value)).then_some(param)
                    })
                    .collect::<Vec<_>>();
                if tag == 1 {
                    words.extend(
                        self.outputs
                            .iter()
                            .filter(|v| !self.passthrough.contains_key(v)),
                    );
                }
                words
            };
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
        if self.packed_inputs {
            eliminate_dead_values(&mut body);
            // DCE removes captures used only to transport an unchanged value.
            // The caller initializes only the surviving, densely packed lanes.
            for (index, inst) in capture_loads.into_iter().enumerate() {
                if body.layout.inst_block(inst).is_none() {
                    continue;
                }
                let InstructionData::Load { ref mut offset, .. } = body.dfg.insts[inst] else {
                    unreachable!()
                };
                *offset = ((self.live_captures.len() * 8) as i32).into();
                self.live_captures.push(index);
            }
        }
        body
    }

    fn restore_word(
        &self,
        pos: &mut FuncCursor<'_>,
        slot: StackSlot,
        original: Value,
        next: &mut i32,
    ) -> Value {
        if let Some(&source) = self.passthrough.get(&original) {
            return source;
        }
        let ty = pos.func.dfg.value_type(original);
        let offset = if self.aligned_outputs.is_empty() {
            let offset = *next;
            *next += 8;
            offset
        } else {
            let position = self
                .aligned_outputs
                .iter()
                .position(|&value| value == original)
                .expect("original result is in the shared output packet");
            (position * 8) as i32
        };
        pos.ins().stack_load(ty, slot, offset)
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
            if self.outputs.contains(arg) && self.passthrough.get(arg) != Some(&*arg) {
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
        args.extend(
            self.literals
                .iter()
                .map(|literal| pos.ins().iconst(literal.ty, literal.bits)),
        );
        if self.packed_inputs {
            let capture = packets.captures.expect("packed region input slot");
            for (index, &source) in self.live_captures.iter().enumerate() {
                pos.ins()
                    .stack_store(args[source], capture, (index * 8) as i32);
            }
            args.clear();
            args.push(pos.ins().stack_addr(types::I64, capture, 0));
        }
        args.push(pos.ins().stack_addr(types::I64, packets.result, 0));
        let call = pos.ins().call(reference, &args);
        let [tag, status] = *pos.func.dfg.inst_results(call) else {
            unreachable!()
        };
        let condition = if self.return_error_origin {
            IntCC::UnsignedGreaterThanOrEqual
        } else {
            IntCC::Equal
        };
        let failed = pos.ins().icmp_imm(condition, tag, FIRST_ERROR_TAG);
        pos.ins().brif(failed, failure, &[], cont, &[]);
        pos.goto_bottom(failure);
        let origin = if self.return_error_origin {
            pos.ins()
                .iadd_imm(tag, candidate.origin_base() - FIRST_ERROR_TAG)
        } else {
            let relative = pos.ins().stack_load(types::I64, packets.result, 0);
            pos.ins().iadd_imm(relative, candidate.origin_base())
        };
        pos.ins().return_(&[status, origin]);
        if let Some((resume, finish, _)) = arms {
            pos.goto_bottom(cont);
            let repeat = pos.ins().icmp_imm(IntCC::Equal, tag, 0);
            pos.ins().brif(repeat, resume, &[], finish, &[]);
        }
        for (block, exit) in continuations.into_iter().flatten() {
            let params = if exit == candidate.exit {
                self.finish.clone()
            } else {
                pos.func.dfg.block_params(exit).to_vec()
            };
            pos.goto_bottom(block);
            let mut next = 0;
            let args = params
                .into_iter()
                .map(|value| {
                    self.restore_word(&mut pos, packets.result, value, &mut next)
                        .into()
                })
                .collect::<Vec<_>>();
            if exit == candidate.exit {
                for &value in &self.outputs {
                    let restored = self.restore_word(&mut pos, packets.result, value, &mut next);
                    if restored != value {
                        pos.func.dfg.change_to_alias(value, restored);
                    }
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
    let pack_large_refusals = std::env::var_os("MECH_COMPACT_PACK_LARGE_REFUSALS").is_some();
    let literal_mode = std::env::var_os("MECH_COMPACT_BIND_REFUSAL_LITERALS");
    share_families(
        module,
        root,
        families,
        RegionKind::Refusal,
        pack_large_refusals,
        CaptureBindings {
            shared_literal_columns: literal_mode.is_some(),
            retry_refusal_bounds: literal_mode.is_some_and(|mode| mode == "retry"),
            ..CaptureBindings::default()
        },
    )
}

#[derive(Clone, Copy, Default)]
pub(super) struct CaptureBindings {
    pub(super) literals: bool,
    pub(super) shared_literal_columns: bool,
    pub(super) retry_refusal_bounds: bool,
    pub(super) input_roles: bool,
    pub(super) return_error_origin: bool,
    pub(super) align_outputs: bool,
    pub(super) pad_outputs: bool,
}

impl CaptureBindings {
    pub(super) fn from_environment() -> Self {
        Self {
            literals: std::env::var_os("MECH_COMPACT_BIND_REGION_LITERALS").is_some(),
            shared_literal_columns: false,
            retry_refusal_bounds: false,
            input_roles: std::env::var_os("MECH_COMPACT_BIND_REGION_INPUT_ROLES").is_some(),
            return_error_origin: std::env::var_os("MECH_COMPACT_RETURN_REGION_ERROR_ORIGIN")
                .is_some(),
            align_outputs: std::env::var_os("MECH_COMPACT_ALIGN_REGION_OUTPUTS").is_some(),
            pad_outputs: std::env::var_os("MECH_COMPACT_PAD_REGION_OUTPUTS").is_some(),
        }
    }
}

pub(super) fn share_controllers<M: Module>(
    module: &mut M,
    root: &mut Function,
    families: Vec<Vec<Candidate>>,
    bindings: CaptureBindings,
) -> Result<Option<serde_json::Value>, NativeCompileError> {
    if families.iter().map(Vec::len).sum::<usize>() < 2 {
        return Ok(None);
    }
    share_families(
        module,
        root,
        families,
        RegionKind::Controller,
        false,
        bindings,
    )
}

pub(super) fn share_phase_regions<M: Module>(
    module: &mut M,
    root: &mut Function,
    families: Vec<Vec<Candidate>>,
    bindings: CaptureBindings,
) -> Result<Option<serde_json::Value>, NativeCompileError> {
    // The resume path can be a shared failure continuation inside the phase.
    // Its original block arguments and preceding stores travel through the
    // same checked boundary as any other partial continuation.
    share_families(module, root, families, RegionKind::Phase, true, bindings)
}

pub(super) fn share_continuations<M: Module>(
    module: &mut M,
    root: &mut Function,
    families: Vec<Vec<Candidate>>,
) -> Result<Option<serde_json::Value>, NativeCompileError> {
    let direct_inputs = std::env::var("MECH_COMPACT_DIRECT_CONTINUATION_WORDS")
        .ok()
        .map(|n| n.parse::<usize>().map_err(backend))
        .transpose()?
        .unwrap_or(0);
    if direct_inputs > MAX_DIRECT_INPUT_WORDS {
        return Err(backend("direct continuation inputs exceed the ABI limit"));
    }
    share_families(
        module,
        root,
        families,
        RegionKind::Continuation { direct_inputs },
        false,
        CaptureBindings::from_environment(),
    )
}

fn prepare_bodies<M: Module>(
    module: &mut M,
    root: &Function,
    traversals: impl IntoIterator<Item = Traversal>,
    dominance: Option<&DominatorTree>,
    region_kind: RegionKind,
) -> Result<Vec<Prepared>, NativeCompileError> {
    let refusal_regions = region_kind == RegionKind::Refusal;
    let compose_origins = region_kind.is_continuation();
    let compact_layout_bodies = std::env::var_os("MECH_COMPACT_PREPARE_LAYOUT_REGION_IR").is_some();
    let compact_refusal_bodies =
        refusal_regions && std::env::var_os("MECH_COMPACT_PREPARE_REFUSAL_IR").is_some();
    let layout_regions_last = std::env::var_os("MECH_COMPACT_LAYOUT_REGIONS_LAST").is_some();
    let share_retirements = std::env::var_os("MECH_COMPACT_SHARE_OWNER_RETIREMENT")
        .is_some_and(|scope| scope == "helpers" || scope == "all");
    let pair_words = super::word_pairs::available(module.isa())
        && std::env::var_os("MECH_COMPACT_PAIR_WORDS")
            .is_some_and(|scope| scope == "helpers" || scope == "all");
    let mut groups: Vec<Prepared> = Vec::new();
    for mut traversal in traversals {
        traversal.live_captures.clear();
        let candidate = traversal.candidate;
        let kind = if region_kind.is_continuation() {
            "layout_continuation"
        } else if refusal_regions {
            "traversal"
        } else if layout_regions_last && candidate.origin_base.is_some() {
            "layout"
        } else {
            "controller"
        };
        let signature = traversal.signature(root);
        let body = if (compact_refusal_bodies
            || compact_layout_bodies && candidate.origin_base.is_some())
            && let Some(dominance) = dominance
            && let Some((seed, mut local)) = region_body::prepare(root, &traversal, dominance)
        {
            let body = local.body(&seed, &signature, compose_origins);
            traversal.live_captures = local.live_captures;
            eprintln!(
                "compact checked {kind} {} prepared local IR",
                candidate.header
            );
            body
        } else {
            traversal.body(root, &signature, compose_origins)
        };
        let mut context = Context::for_function(body);
        if share_retirements {
            let shared = super::owner_retirement::share(module, &mut context.func)?;
            if shared != 0 {
                eprintln!(
                    "compact checked {kind} {} retirement sites shared: {shared}",
                    candidate.header
                );
            }
        }
        if pair_words {
            // Every caller reserves the full result packet in its own frame.
            let entry = context.func.layout.entry_block().expect("region entry");
            let packet = *context
                .func
                .dfg
                .block_params(entry)
                .last()
                .expect("region result pointer");
            super::word_pairs::fold(
                &mut context.func,
                Some((packet, traversal.output_words())),
                false,
            );
        }
        if traversal.packed_inputs {
            eprintln!(
                "compact checked {kind} {} packet: {} captures, {} results",
                candidate.header,
                traversal.live_captures.len(),
                traversal.output_words()
            );
        }
        context.verify(module.isa()).map_err(backend)?;
        context
            .compile(module.isa(), &mut ControlPlane::default())
            .map_err(ModuleError::from)
            .map_err(backend)?;
        let compiled = context.compiled_code().unwrap();
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
        let dump = if refusal_regions {
            "MECH_COMPACT_DUMP_REFUSAL_MATCHES"
        } else {
            "MECH_COMPACT_DUMP_CONTROLLER_MATCHES"
        };
        if let Some(folder) = std::env::var_os(dump) {
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
        if let Some(winner) = groups.iter_mut().find(|other| {
            other.signature == signature
                && other.traversals[0].return_error_origin == traversal.return_error_origin
                && other.alignment == alignment
                && other.bytes.as_ref() == bytes
                && matching_relocations(other.function, &other.relocs, function, &relocs)
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
    Ok(groups)
}

fn share_families<M: Module>(
    module: &mut M,
    root: &mut Function,
    families: Vec<Vec<Candidate>>,
    region_kind: RegionKind,
    pack_large_refusals: bool,
    bindings: CaptureBindings,
) -> Result<Option<serde_json::Value>, NativeCompileError> {
    let refusal_regions = region_kind == RegionKind::Refusal;
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
    let order_layout_captures = std::env::var_os("MECH_COMPACT_ORDER_REGION_CAPTURES").is_some();
    let mut proven = BTreeSet::new();
    let cfg = ControlFlowGraph::with_function(root);
    let users = use_blocks(root);
    let dominance =
        (!refusal_regions || pack_large_refusals).then(|| DominatorTree::with_function(root, &cfg));
    let mut traversals = Vec::new();
    let mut family_ids = BTreeMap::new();
    for (family_index, family) in families.into_iter().enumerate() {
        let first = traversals.len();
        for candidate in family {
            if excluded.contains(&candidate.header) {
                continue;
            }
            let mut traversal = match plan(
                root,
                &cfg,
                &users,
                dominance.as_ref(),
                candidate,
                pack_large_refusals,
                order_layout_captures && candidate.origin_base.is_some(),
                region_kind,
            ) {
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
            proven.extend(&traversal.blocks);
            traversal.return_error_origin = bindings.return_error_origin
                && traversal.packed_inputs
                && traversal.has_bounded_error_origins(root);
            if bindings.align_outputs || bindings.pad_outputs {
                family_ids.insert(candidate.header, family_index);
            }
            traversals.push(traversal);
        }
        let family = &mut traversals[first..];
        if bindings.literals {
            let compact_columns = matches!(region_kind, RegionKind::Continuation { direct_inputs } if direct_inputs != 0);
            region_literals::bind(root, family, compact_columns);
        }
        if bindings.input_roles {
            region_inputs::bind(root, family);
        }
    }
    if bindings.shared_literal_columns {
        region_literals::bind(root, &mut traversals, true);
    }
    drop((users, proven));
    let mut groups = prepare_bodies(module, root, traversals, dominance.as_ref(), region_kind)?;
    if (bindings.align_outputs || bindings.pad_outputs)
        && let Some(dom) = dominance.as_ref()
    {
        // Preserve exact matches with their original packets. Only unmatched
        // members can justify wider output packets and another compilation.
        let mut unmatched = BTreeMap::<_, Vec<_>>::new();
        for strategy in [
            bindings
                .align_outputs
                .then_some(region_outputs::Strategy::Union),
            bindings
                .pad_outputs
                .then_some(region_outputs::Strategy::Pad),
        ]
        .into_iter()
        .flatten()
        {
            groups.retain_mut(|group| {
                if group.traversals.len() > 1 {
                    return true;
                }
                let mut traversal = group.traversals.pop().unwrap();
                traversal.aligned_outputs.clear();
                let family = family_ids[&traversal.candidate.header];
                unmatched.entry(family).or_default().push(traversal);
                false
            });
            for family in unmatched.values_mut() {
                region_outputs::bind(root, &cfg, dom, family, strategy);
            }
            let aligned = unmatched
                .values_mut()
                .flat_map(|family| family.extract_if(.., |t| !t.aligned_outputs.is_empty()));
            groups.extend(prepare_bodies(
                module,
                root,
                aligned,
                Some(dom),
                region_kind,
            )?);
        }
    }
    if refusal_regions && bindings.retry_refusal_bounds {
        refusal_bounds::retry(module, root, &mut groups, dominance.as_ref())?;
    }
    drop((cfg, dominance));
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
            let finish = t.output_words();
            [finish, t.resume_output_words(root)]
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
    let captures = groups
        .iter()
        .flat_map(|group| &group.traversals)
        .filter(|t| t.packed_inputs)
        .map(|t| t.live_captures.len())
        .max()
        .map(|words| {
            root.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                (words.max(1) * 8) as u32,
                3,
            ))
        });
    let packets = CallSlots { result, captures };
    let observe_refusals =
        refusal_regions && std::env::var_os("MECH_COMPACT_OBSERVE_CHECKED_TRAVERSALS").is_some();
    let observer_flag = if region_kind.is_continuation() {
        "MECH_COMPACT_OBSERVE_CONTINUATIONS"
    } else {
        "MECH_COMPACT_OBSERVE_LAYOUT_REGIONS"
    };
    let observe_layout = std::env::var_os(observer_flag).is_some()
        && groups.iter().any(|group| {
            group
                .traversals
                .iter()
                .any(|t| t.candidate.origin_base.is_some())
        });
    let observer = (observe_refusals || observe_layout)
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
        let mut entry = serde_json::json!({
            "headers":selected.traversals.iter().map(|t| t.candidate.header).collect::<Vec<_>>(),
            "code_bytes":selected.bytes.len(),"stack_bytes":selected.stack,
            "input_words":selected.signature.params.len(),
        });
        if selected.traversals[0].return_error_origin {
            entry["returns_error_origin"] = serde_json::json!(true);
        }
        if selected
            .traversals
            .iter()
            .any(|t| !t.aligned_outputs.is_empty())
        {
            entry["aligned_outputs"] = serde_json::json!(true);
        }
        if selected
            .traversals
            .iter()
            .any(|t| !t.input_roles.is_empty())
        {
            entry["aligned_input_words"] = serde_json::json!(
                selected
                    .traversals
                    .iter()
                    .map(|t| t.inputs.len())
                    .collect::<Vec<_>>()
            );
        }
        if selected.traversals.iter().any(|t| !t.literals.is_empty()) {
            entry["literal_inputs"] = serde_json::json!(
                selected
                    .traversals
                    .iter()
                    .map(|t| t.literals.iter().map(|c| c.bits).collect::<Vec<_>>())
                    .collect::<Vec<_>>()
            );
        }
        if selected.traversals.iter().any(|t| t.packed_inputs) {
            entry["packet_words"] = serde_json::json!(
                selected
                    .traversals
                    .iter()
                    .map(|t| [t.live_captures.len(), t.output_words()])
                    .collect::<Vec<_>>()
            );
            if refusal_regions
                || selected
                    .traversals
                    .iter()
                    .any(|t| t.candidate.resume.is_some())
            {
                entry["resume_packet_words"] = serde_json::json!(
                    selected
                        .traversals
                        .iter()
                        .map(|t| t.resume_output_words(root))
                        .collect::<Vec<_>>()
                );
            }
        }
        report.push(entry);
    }
    Ok((!report.is_empty()).then_some(serde_json::Value::Array(report)))
}

#[cfg(test)]
#[path = "checked_traversal_tests.rs"]
mod tests;
