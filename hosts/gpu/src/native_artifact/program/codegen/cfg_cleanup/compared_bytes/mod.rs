//! Reuse already compared immutable bytes on an edge to two defaulted reads.
//! Later bounds and zero defaults remain unless the original current read
//! proves those exact logical bounds. Only effect-free paths are bypassed;
//! no read permission, index bound or work allowance grows.
use super::window_bounds::comparison;
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
pub(in crate::native_artifact) use read_bounds::BoundedRead;

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
}
struct Walk<'a> {
    f: &'a Function,
    cfg: &'a ControlFlowGraph,
    dom: &'a DominatorTree,
    origin: Inst,
    bytes: [Byte; 2],
    symbols: Symbols,
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
        if !pure(self.f.dfg.insts[inst].opcode()) {
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
                    let address = self
                        .symbols
                        .address(self.f, self.cfg, arg, i32::from(offset))?;
                    let at = self
                        .bytes
                        .iter()
                        .position(|byte| byte.address == address && byte.flags == flags)?;
                    let symbol = self.symbols.existing(self.bytes[at].value);
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
    fn trace(mut self, edge: usize, mut call: BlockCall) -> Option<Plan> {
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
            self.enter(&mut values, block, &args)?;
            let last = self.f.layout.last_inst(block)?;
            for inst in self.f.layout.block_insts(block).take_while(|&i| i != last) {
                self.copy(&mut values, inst)?;
            }
            match self.f.dfg.insts[last] {
                Data::Jump { destination, .. } => call = destination,
                Data::Brif { arg, blocks, .. } => {
                    let condition = self.value(&values, arg)?;
                    let yes = self.arm(values.clone(), blocks[0])?;
                    let no = self.arm(values.clone(), blocks[1])?;
                    if yes.join != no.join
                        || yes.args.len() != no.args.len()
                        || yes.reused.len() != 1
                        || !no.reused.is_empty()
                    {
                        return None;
                    }
                    let admitted = self.admitted(&values, arg, yes.reused[0]);
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

fn repairs(f: &Function, dom: &DominatorTree, plan: &Plan) -> Option<Vec<(Value, Symbol)>> {
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
                    if !dom.block_dominates(plan.join, block) {
                        return None;
                    }
                    escaped.insert(value);
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

pub(in crate::native_artifact) fn fold(
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
                        },
                        Byte {
                            value: bytes[1].value,
                            address: bytes[1].address.clone(),
                            flags: bytes[1].flags,
                            bounds: bytes[1].bounds.clone(),
                        },
                    ],
                    symbols: Symbols::default(),
                };
                if let Some(found) = walk.trace(edge, call) {
                    if let Some(repairs) = repairs(f, &context.domtree, &found) {
                        plan = Some((found, repairs));
                        break 'search;
                    }
                }
            }
        }
        let Some((mut plan, repairs)) = plan else {
            break;
        };
        proved_bounds += plan.proved_bounds;
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
                    context.func.dfg.map_inst_values(inst, |value| {
                        mapped.get(&value).copied().unwrap_or(value)
                    });
                }
            }
        }
        let fast = context.func.dfg.make_block();
        context.func.layout.append_block(fast);
        let mut cursor = FuncCursor::new(&mut context.func).at_bottom(fast);
        let args = plan
            .symbols
            .emit(&mut cursor, &plan.args)
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>();
        cursor.ins().jump(plan.join, &args);
        let target = BlockCall::new(fast, [], &mut cursor.func.dfg.value_lists);
        let Data::Brif { blocks, .. } = &mut cursor.func.dfg.insts[plan.origin] else {
            unreachable!("proved byte comparison");
        };
        blocks[plan.edge] = target;
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
