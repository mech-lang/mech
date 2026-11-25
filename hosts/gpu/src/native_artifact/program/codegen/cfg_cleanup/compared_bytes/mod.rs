//! Reuse already compared immutable bytes on an edge to two defaulted reads.
//! Later bounds and zero defaults remain unless the original current read
//! proves those exact logical bounds. Only effect-free paths are bypassed;
//! no read permission, index bound or work allowance grows.
use super::window_bounds::comparison;
use crate::native_artifact::codegen::compilation_inputs::CompilationInputs;
use cranelift_codegen::{
    Context,
    cursor::{Cursor, FuncCursor},
    dominator_tree::DominatorTree,
    flowgraph::ControlFlowGraph,
    ir::{
        Block, BlockCall, Function, Inst, InstBuilder, InstructionData as Data, MemFlags,
        Opcode as Op, Value, ValueDef, condcodes::IntCC, types,
    },
};
use std::collections::{BTreeMap, BTreeSet};
mod symbols;
use symbols::{Address, Symbol, Symbols, current};
mod read_bounds;
use cranelift_codegen::ir::condcodes::CondCode;
mod word_order;
pub(in crate::native_artifact) use read_bounds::BoundedRead;
pub(in crate::native_artifact) use word_order::fold_word_order;

#[derive(PartialEq, Eq)]
struct Fact {
    cond: IntCC,
    left: Address,
    right: Address,
}

impl Fact {
    fn relates(&self, cond: IntCC, left: &Address, right: &Address) -> Option<bool> {
        let known = if &self.left == left && &self.right == right {
            self.cond
        } else if &self.left == right && &self.right == left {
            self.cond.swap_args()
        } else {
            return None;
        };
        if known == cond {
            Some(true)
        } else if known.complement() == cond {
            Some(false)
        } else {
            None
        }
    }
}

fn pure(op: Op) -> bool {
    matches!(
        op,
        Op::Iconst
            | Op::Iadd
            | Op::IaddImm
            | Op::Isub
            | Op::Imul
            | Op::ImulImm
            | Op::Band
            | Op::BandImm
            | Op::Bor
            | Op::BorImm
            | Op::Bxor
            | Op::BxorImm
            | Op::Ishl
            | Op::IshlImm
            | Op::Ushr
            | Op::UshrImm
            | Op::Icmp
            | Op::IcmpImm
            | Op::Uextend
            | Op::Ireduce
    )
}

fn unchanged(
    f: &Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
    load: Inst,
    to: Inst,
) -> bool {
    if !dom.dominates(load, to, &f.layout) {
        return false;
    }
    let Some(at) = f.layout.inst_block(to) else {
        return false;
    };
    let mut pending = vec![(at, Some(to))];
    let mut visited = BTreeSet::new();
    let mut steps = 0;
    while let Some((block, until)) = pending.pop() {
        if !visited.insert(block) {
            continue;
        }
        if visited.len() > 128 {
            return false;
        }
        let mut found = false;
        let insts = f
            .layout
            .block_insts(block)
            .take_while(|inst| Some(*inst) != until)
            .collect::<Vec<_>>();
        for inst in insts.into_iter().rev() {
            if inst == load {
                found = true;
                break;
            }
            steps += 1;
            if steps > 4096 {
                return false;
            }
            if !pure(f.dfg.insts[inst].opcode())
                && !matches!(
                    f.dfg.insts[inst],
                    Data::Jump { .. } | Data::Brif { .. } | Data::Load { .. }
                )
            {
                return false;
            }
        }
        if !found {
            for pred in cfg.pred_iter(block) {
                if dom.is_reachable(pred.block) {
                    pending.push((pred.block, None));
                }
            }
        }
    }
    true
}

struct Byte {
    value: Value,
    address: Address,
    flags: MemFlags,
    bounds: Option<(Address, Address)>,
    must_match_bound: bool,
}
struct Walk<'a> {
    f: &'a Function,
    cfg: &'a ControlFlowGraph,
    dom: &'a DominatorTree,
    origin: Inst,
    bytes: [Byte; 2],
    symbols: Symbols,
    facts: Vec<Fact>,
    word_difference: Option<Value>,
    byte_offset: Option<Symbol>,
}
type Values = BTreeMap<Value, Symbol>;
struct Arm {
    join: Block,
    args: Vec<Symbol>,
    reused: Vec<usize>,
    blocks: BTreeSet<Block>,
}
struct Plan {
    origin: Inst,
    edge: usize,
    join: Block,
    args: Vec<Symbol>,
    symbols: Symbols,
    values: Values,
    region: BTreeSet<Block>,
    proved_bounds: usize,
    reverse_words: bool,
}

impl Walk<'_> {
    fn value(&mut self, values: &Values, value: Value) -> Option<Symbol> {
        if self.symbols.len() >= 512 {
            return None;
        }
        let value = self.f.dfg.resolve_aliases(value);
        if let Some(&symbol) = values.get(&value) {
            return Some(symbol);
        }
        let available = match self.f.dfg.value_def(value) {
            ValueDef::Param(block, _) => self.dom.dominates(block, self.origin, &self.f.layout),
            ValueDef::Result(inst, _) => {
                inst != self.origin && self.dom.dominates(inst, self.origin, &self.f.layout)
            }
            ValueDef::Union(..) => false,
        };
        available.then(|| self.symbols.existing(value))
    }
    fn arguments(&mut self, values: &Values, call: BlockCall) -> Option<Vec<Symbol>> {
        call.args(&self.f.dfg.value_lists)
            .map(|arg| self.value(values, arg.as_value()?))
            .collect()
    }
    fn enter(&self, values: &mut Values, block: Block, args: &[Symbol]) -> Option<()> {
        let params = self.f.dfg.block_params(block);
        if params.len() != args.len() {
            return None;
        }
        values.extend(
            params
                .iter()
                .map(|&v| self.f.dfg.resolve_aliases(v))
                .zip(args.iter().copied()),
        );
        Some(())
    }
    fn copy(&mut self, values: &mut Values, inst: Inst) -> Option<()> {
        if self.symbols.len() >= 512 {
            return None;
        }
        let opcode = self.f.dfg.insts[inst].opcode();
        if !(pure(opcode) || self.word_difference.is_some() && opcode == Op::Ctz) {
            return None;
        }
        let &[result] = self.f.dfg.inst_results(inst) else {
            return None;
        };
        let args = self
            .f
            .dfg
            .inst_args(inst)
            .iter()
            .map(|&v| self.value(values, v))
            .collect::<Option<Vec<_>>>()?;
        let symbol = self.symbols.copy(inst, args);
        if self.byte_offset.is_none()
            && let Some(difference) = self.word_difference
            && self
                .symbols
                .first_bad_byte(self.f, self.cfg, symbol, difference)
        {
            self.byte_offset = Some(symbol);
        }
        values.insert(self.f.dfg.resolve_aliases(result), symbol);
        Some(())
    }
    fn arm(&mut self, mut values: Values, mut call: BlockCall) -> Option<Arm> {
        let mut visited = BTreeSet::new();
        let mut reused = Vec::new();
        loop {
            let block = call.block(&self.f.dfg.value_lists);
            let args = self.arguments(&values, call)?;
            if self.cfg.pred_iter(block).count() > 1 {
                return Some(Arm {
                    join: block,
                    args,
                    reused,
                    blocks: visited,
                });
            }
            if visited.len() >= 64 || !visited.insert(block) {
                return None;
            }
            self.enter(&mut values, block, &args)?;
            let last = self.f.layout.last_inst(block)?;
            for inst in self.f.layout.block_insts(block).take_while(|&i| i != last) {
                if let Data::Load {
                    opcode: Op::Uload8,
                    arg,
                    flags,
                    offset,
                } = self.f.dfg.insts[inst]
                {
                    let result = self.f.dfg.first_result(inst);
                    if self.f.dfg.value_type(result) != types::I64 {
                        return None;
                    }
                    let arg = self.value(&values, arg)?;
                    let address = self.symbols.address_with_offset(
                        self.f,
                        self.cfg,
                        arg,
                        i32::from(offset),
                        self.byte_offset,
                    )?;
                    let at = self
                        .bytes
                        .iter()
                        .position(|byte| byte.address == address && byte.flags == flags)?;
                    let byte = self.bytes[at].value;
                    let symbol = if self.word_difference.is_some() {
                        self.symbols.word_byte(byte, self.byte_offset?)
                    } else {
                        self.symbols.existing(byte)
                    };
                    values.insert(result, symbol);
                    reused.push(at);
                } else {
                    self.copy(&mut values, inst)?;
                }
            }
            let Data::Jump { destination, .. } = self.f.dfg.insts[last] else {
                return None;
            };
            call = destination;
        }
    }
    fn trace(
        mut self,
        compiler_inputs: &CompilationInputs,
        edge: usize,
        mut call: BlockCall,
    ) -> Option<Plan> {
        let inspect_word = self.word_difference.is_some()
            && compiler_inputs
                .var_os("MECH_INSPECT_WORD_MISMATCH_ORDER")
                .is_some();
        let mut values = Values::new();
        let mut visited = BTreeSet::new();
        let mut reused = BTreeSet::new();
        let mut joined = None;
        let mut proved_bounds = 0;
        loop {
            let (block, args) = if let Some(join) = joined.take() {
                join
            } else {
                (
                    call.block(&self.f.dfg.value_lists),
                    self.arguments(&values, call)?,
                )
            };
            if visited.len() >= 128 || !visited.insert(block) {
                return None;
            }
            if !self.facts.is_empty()
                && self
                    .dom
                    .block_dominates(block, self.f.layout.inst_block(self.origin)?)
            {
                return None;
            }
            if inspect_word {
                eprintln!("word mismatch {} follows {block}", self.origin);
            }
            self.enter(&mut values, block, &args)?;
            let last = self.f.layout.last_inst(block)?;
            for inst in self.f.layout.block_insts(block).take_while(|&i| i != last) {
                if self.copy(&mut values, inst).is_none() {
                    if inspect_word {
                        eprintln!("word mismatch {} cannot replay {inst}", self.origin);
                    }
                    return None;
                }
            }
            match self.f.dfg.insts[last] {
                Data::Jump { destination, .. } => call = destination,
                Data::Brif { arg, blocks, .. } => {
                    let condition = self.value(&values, arg)?;
                    if !self.facts.is_empty()
                        && let Some(taken) = self.symbols.truth(
                            self.f,
                            self.cfg,
                            condition,
                            &self.facts,
                            self.byte_offset,
                            0,
                        )
                    {
                        if inspect_word {
                            eprintln!(
                                "word mismatch {} proved {block} outcome {taken}",
                                self.origin
                            );
                        }
                        call = blocks[usize::from(!taken)];
                        continue;
                    }
                    let yes = self.arm(values.clone(), blocks[0]);
                    if inspect_word {
                        eprintln!(
                            "word mismatch {} {block} read path: {}, offset: {:?}",
                            self.origin,
                            yes.is_some(),
                            self.byte_offset
                        );
                    }
                    let yes = yes?;
                    let no = self.arm(values.clone(), blocks[1])?;
                    if yes.join != no.join
                        || yes.args.len() != no.args.len()
                        || yes.reused.len() != 1
                        || !no.reused.is_empty()
                    {
                        return None;
                    }
                    let admitted = self.admitted(&values, arg, yes.reused[0]);
                    if self.bytes[yes.reused[0]].must_match_bound && !admitted {
                        return None;
                    }
                    proved_bounds += usize::from(admitted);
                    let mut merged = Vec::new();
                    let mut differences = 0;
                    for (&a, &b) in yes.args.iter().zip(&no.args) {
                        if a != b {
                            if !self.symbols.zero(self.f, b) {
                                return None;
                            }
                            differences += 1;
                        }
                        merged.push(if admitted {
                            a
                        } else {
                            self.symbols.choice(condition, a, b)
                        });
                    }
                    if differences != 1 || !reused.insert(yes.reused[0]) {
                        return None;
                    }
                    if reused.len() == 2 {
                        if visited.contains(&yes.join) {
                            return None;
                        }
                        visited.extend(yes.blocks);
                        visited.extend(no.blocks);
                        return Some(Plan {
                            origin: self.origin,
                            edge,
                            join: yes.join,
                            args: merged,
                            symbols: self.symbols,
                            values,
                            region: visited,
                            proved_bounds,
                            reverse_words: false,
                        });
                    }
                    visited.extend(yes.blocks);
                    visited.extend(no.blocks);
                    joined = Some((yes.join, merged));
                }
                _ => return None,
            }
        }
    }
}

impl Plan {
    fn install_edge(&self, f: &mut Function) {
        let fast = f.dfg.make_block();
        f.layout.append_block(fast);
        let mut cursor = FuncCursor::new(f).at_bottom(fast);
        let args = self
            .symbols
            .emit(&mut cursor, &self.args, self.reverse_words)
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>();
        cursor.ins().jump(self.join, &args);
        let target = BlockCall::new(fast, [], &mut cursor.func.dfg.value_lists);
        let Data::Brif { blocks, .. } = &mut cursor.func.dfg.insts[self.origin] else {
            unreachable!("proved byte comparison");
        };
        blocks[self.edge] = target;
    }
}

fn repairs(
    f: &Function,
    dom: &DominatorTree,
    plan: &Plan,
    verify_other_routes: bool,
) -> Option<Vec<(Value, Symbol)>> {
    let mut alternate = None;
    let mut escaped = BTreeSet::new();
    for block in f
        .layout
        .blocks()
        .filter(|block| !plan.region.contains(block))
    {
        for inst in f.layout.block_insts(block) {
            for value in f.dfg.inst_values(inst).map(|v| f.dfg.resolve_aliases(v)) {
                let owner = match f.dfg.value_def(value) {
                    ValueDef::Param(block, _) => block,
                    ValueDef::Result(inst, _) => f.layout.inst_block(inst)?,
                    ValueDef::Union(..) => return None,
                };
                if plan.region.contains(&owner) {
                    if dom.block_dominates(plan.join, block) {
                        escaped.insert(value);
                        continue;
                    }
                    if !verify_other_routes {
                        return None;
                    }
                    // A definition on the bypassed path can still dominate
                    // another branch if the new edge never reaches that use
                    // before the definition. Check the actual proposed CFG.
                    let context = alternate.get_or_insert_with(|| {
                        let mut context = Context::for_function(f.clone());
                        plan.install_edge(&mut context.func);
                        context.flowgraph();
                        context
                    });
                    let still_dominates = match f.dfg.value_def(value) {
                        ValueDef::Param(block, _) => context
                            .domtree
                            .block_dominates(block, f.layout.inst_block(inst)?),
                        ValueDef::Result(definition, _) => {
                            context
                                .domtree
                                .dominates(definition, inst, &context.func.layout)
                        }
                        ValueDef::Union(..) => false,
                    };
                    if !still_dominates {
                        return None;
                    }
                }
            }
        }
    }
    if escaped.len() > 16 {
        return None;
    }
    escaped
        .into_iter()
        .map(|value| {
            let dominates = match f.dfg.value_def(value) {
                ValueDef::Param(block, _) => dom.block_dominates(block, plan.join),
                ValueDef::Result(inst, _) => dom.dominates(inst, plan.join, &f.layout),
                ValueDef::Union(..) => false,
            };
            dominates.then_some(())?;
            Some((value, *plan.values.get(&value)?))
        })
        .collect()
}

fn install(context: &mut Context, mut plan: Plan, repairs: Vec<(Value, Symbol)>) {
    let predecessors = context
        .cfg
        .pred_iter(plan.join)
        .map(|p| p.inst)
        .collect::<BTreeSet<_>>();
    let mut replacement = BTreeMap::new();
    for (value, symbol) in repairs {
        let ty = context.func.dfg.value_type(value);
        let parameter = context.func.dfg.append_block_param(plan.join, ty);
        let dfg = &mut context.func.dfg;
        for &inst in &predecessors {
            for call in dfg.insts[inst]
                .branch_destination_mut(&mut dfg.jump_tables, &mut dfg.exception_tables)
            {
                if call.block(&dfg.value_lists) == plan.join {
                    call.append_argument(value, &mut dfg.value_lists);
                }
            }
        }
        replacement.insert(value, parameter);
        plan.args.push(symbol);
    }
    for block in context.func.layout.blocks().collect::<Vec<_>>() {
        if context.domtree.block_dominates(plan.join, block) {
            for inst in context.func.layout.block_insts(block).collect::<Vec<_>>() {
                let aliases = &context.func.dfg;
                let mapped = aliases
                    .inst_values(inst)
                    .map(|raw| {
                        let value = aliases.resolve_aliases(raw);
                        (raw, replacement.get(&value).copied().unwrap_or(value))
                    })
                    .collect::<BTreeMap<_, _>>();
                context
                    .func
                    .dfg
                    .map_inst_values(inst, |value| mapped.get(&value).copied().unwrap_or(value));
            }
        }
    }
    plan.install_edge(&mut context.func);
}

pub(in crate::native_artifact) fn fold(
    compiler_inputs: &CompilationInputs,
    context: &mut Context,
    immutable_bytes: &BTreeSet<Value>,
    bounded_reads: &[BoundedRead],
    views: &BTreeMap<usize, (Value, Value)>,
) -> usize {
    if immutable_bytes.is_empty() {
        return 0;
    }
    let mut total = 0;
    let mut proved_bounds = 0;
    for _ in 0..8 {
        context.flowgraph();
        let f = &context.func;
        let mut plan = None;
        'search: for block in f
            .layout
            .blocks()
            .filter(|&b| context.domtree.is_reachable(b))
        {
            let Some(origin) = f.layout.last_inst(block) else {
                continue;
            };
            let Data::Brif { arg, blocks, .. } = f.dfg.insts[origin] else {
                continue;
            };
            let Some(compare) = comparison(f, arg) else {
                continue;
            };
            let Data::IntCompare {
                cond: IntCC::Equal | IntCC::NotEqual,
                args,
                ..
            } = f.dfg.insts[compare]
            else {
                continue;
            };
            let mut symbols = Symbols::default();
            let mut bytes = Vec::new();
            for value in args {
                let value = current(f, &context.cfg, value);
                let ValueDef::Result(load, 0) = f.dfg.value_def(value) else {
                    break;
                };
                let Data::Load {
                    opcode: Op::Uload8,
                    arg,
                    flags,
                    offset,
                } = f.dfg.insts[load]
                else {
                    break;
                };
                if f.dfg.value_type(value) != types::I64
                    || !unchanged(f, &context.cfg, &context.domtree, load, origin)
                {
                    break;
                }
                let symbol = symbols.existing(arg);
                let Some(address) = symbols.address(f, &context.cfg, symbol, i32::from(offset))
                else {
                    break;
                };
                if !immutable_bytes
                    .iter()
                    .any(|&base| address.words.get(&current(f, &context.cfg, base)) == Some(&1))
                {
                    break;
                }
                bytes.push(Byte {
                    value,
                    address,
                    flags,
                    must_match_bound: false,
                    bounds: bounded_reads
                        .iter()
                        .find_map(|read| read.keys(f, &context.cfg, value, views)),
                });
            }
            let Ok(bytes) = <[Byte; 2]>::try_from(bytes) else {
                continue;
            };
            for (edge, call) in blocks.into_iter().enumerate() {
                let walk = Walk {
                    f,
                    cfg: &context.cfg,
                    dom: &context.domtree,
                    origin,
                    bytes: [
                        Byte {
                            value: bytes[0].value,
                            address: bytes[0].address.clone(),
                            flags: bytes[0].flags,
                            bounds: bytes[0].bounds.clone(),
                            must_match_bound: false,
                        },
                        Byte {
                            value: bytes[1].value,
                            address: bytes[1].address.clone(),
                            flags: bytes[1].flags,
                            bounds: bytes[1].bounds.clone(),
                            must_match_bound: false,
                        },
                    ],
                    symbols: Symbols::default(),
                    facts: Vec::new(),
                    word_difference: None,
                    byte_offset: None,
                };
                if let Some(found) = walk.trace(compiler_inputs, edge, call) {
                    if let Some(repairs) = repairs(f, &context.domtree, &found, false) {
                        plan = Some((found, repairs));
                        break 'search;
                    }
                }
            }
        }
        let Some((plan, repairs)) = plan else {
            break;
        };
        proved_bounds += plan.proved_bounds;
        install(context, plan, repairs);
        total += 1;
    }
    context.flowgraph();
    if !bounded_reads.is_empty() {
        eprintln!("compared byte source bounds proved: {proved_bounds}");
    }
    total
}

#[cfg(all(test, feature = "jit", target_pointer_width = "64"))]
mod tests;
