//! Store the biased form of a recurring unsigned reduction bound. The
//! ordinary value remains available to every use outside the opportunities
//! with the same current, nonwrapping comparison and distance.
use super::composed_reads::definition;
use super::selected_ranges;
use super::word_ranges::{Facts, Range, constant_offset, literal};
use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{
    Block, Function, Inst, InstBuilder, InstructionData as Data, Opcode as Op, Value, ValueDef,
    types,
};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
#[path = "reduction_bound/tests.rs"]
mod tests;

const MAX_PROOF_STEPS: usize = 256;

struct Plan {
    param: Value,
    bias: u64,
    transport: BTreeSet<Value>,
    incoming: Vec<Incoming>,
    comparisons: Vec<Inst>,
    distances: Vec<(Inst, u64)>,
    offsets: Vec<(Inst, u64)>,
    bound_base: Value,
}

enum Encoded {
    Word(Value),
    Literal(u64),
}

struct Incoming {
    from: Inst,
    edge: usize,
    lane: usize,
    value: Encoded,
}

fn unsigned_bound(
    f: &Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
    value: Value,
    block: Block,
) -> Option<Range> {
    let mut known = BTreeMap::new();
    let mut block = block;
    // Only selected edges dominating this use can refine the current word.
    for _ in 0..32 {
        for (word, range) in selected_ranges::incoming(f, cfg, block, &known) {
            let prior = known.entry(word).or_insert(range);
            *prior = Range(prior.0.max(range.0), prior.1.min(range.1));
            if prior.0 > prior.1 {
                return None;
            }
        }
        let Some(parent) = dom.idom(block) else {
            break;
        };
        block = parent;
    }
    Facts::bound(f, &known).value_range(value)
}

fn biased_base(f: &Function, value: Value) -> Option<(Value, u64)> {
    let (_, data) = definition(f, value)?;
    let (base, bias) = match data {
        Data::Binary {
            opcode: Op::Isub,
            args: [base, bias],
        } => (base, literal(f, bias)?),
        Data::BinaryImm64 {
            opcode: Op::IaddImm,
            arg,
            imm,
        } if imm.bits() < 0 => (arg, imm.bits().unsigned_abs()),
        _ => return None,
    };
    let base = f.dfg.resolve_aliases(base);
    (bias > 0 && bias <= i64::MAX as u64 && f.dfg.value_type(base) == types::I64)
        .then_some((base, bias))
}

fn derive(
    f: &Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
    param: Value,
    bound: Value,
    base: Value,
    bias: u64,
    first: Inst,
) -> Option<Plan> {
    let ValueDef::Param(header, _) = f.dfg.value_def(param) else {
        return None;
    };
    if Some(header) == f.layout.entry_block() {
        return None;
    }
    let first_block = f.layout.inst_block(first)?;
    if unsigned_bound(f, cfg, dom, base, first_block)?.0 < bias {
        return None;
    }
    let mut plan = Plan {
        param,
        bias,
        transport: BTreeSet::new(),
        incoming: Vec::new(),
        comparisons: Vec::new(),
        distances: Vec::new(),
        offsets: Vec::new(),
        bound_base: base,
    };
    let mut pending = vec![param];
    let mut saw_latch = false;
    let mut saw_seed = false;
    let mut saw_bound = false;
    let mut steps = MAX_PROOF_STEPS;
    // Every reachable incoming value must be a representable seed, a
    // current raw bound whose subtraction cannot wrap, or another lane
    // proved by this same walk. That makes the bias valid on every visit.
    while let Some(word) = pending.pop() {
        if !plan.transport.insert(word) {
            continue;
        }
        let ValueDef::Param(block, lane) = f.dfg.value_def(word) else {
            return None;
        };
        if word == base
            || f.dfg.value_type(word) != types::I64
            || !dom.block_dominates(header, block)
        {
            return None;
        }
        // The verifier's type/SSA checks do not establish the numeric
        // invariant. Prove all edges which can actually enter this block.
        for edge in cfg
            .pred_iter(block)
            .filter(|edge| dom.is_reachable(edge.block))
        {
            steps = steps.checked_sub(1)?;
            let data = f.dfg.insts[edge.inst];
            if !matches!(data, Data::Jump { .. } | Data::Brif { .. }) {
                return None;
            }
            let mut calls = data
                .branch_destination(&f.dfg.jump_tables, &f.dfg.exception_tables)
                .iter()
                .enumerate()
                .filter(|(_, call)| call.block(&f.dfg.value_lists) == block);
            let (edge_index, call) = calls.next()?;
            if calls.next().is_some() {
                return None;
            }
            let argument = f
                .dfg
                .resolve_aliases(call.args(&f.dfg.value_lists).nth(lane)?.as_value()?);
            let inside = dom.block_dominates(header, edge.block);
            saw_latch |= block == header && inside;
            let value = if argument == bound && inside {
                if unsigned_bound(f, cfg, dom, base, edge.block)?.0 < bias {
                    return None;
                }
                saw_bound = true;
                Encoded::Word(base)
            } else if let Some(n) = literal(f, argument) {
                saw_seed |= block == header && !inside;
                Encoded::Literal(n.checked_add(bias)?)
            } else if inside {
                pending.push(argument);
                Encoded::Word(argument)
            } else {
                return None;
            };
            plan.incoming.push(Incoming {
                from: edge.inst,
                edge: edge_index,
                lane,
                value,
            });
        }
    }
    if !saw_latch || !saw_seed || !saw_bound {
        return None;
    }
    for block in f
        .layout
        .blocks()
        .filter(|&b| dom.block_dominates(header, b))
    {
        for inst in f.layout.block_insts(block) {
            match f.dfg.insts[inst] {
                Data::IntCompare {
                    cond: IntCC::UnsignedLessThan,
                    args,
                    ..
                } if args.map(|arg| f.dfg.resolve_aliases(arg)) == [param, bound]
                    && unsigned_bound(f, cfg, dom, base, block).is_some_and(|r| r.0 >= bias) =>
                {
                    plan.comparisons.push(inst);
                }
                Data::Binary {
                    opcode: Op::Isub,
                    args: [left, right],
                } if f.dfg.resolve_aliases(left) == bound => {
                    let (word, offset) = constant_offset(f, right);
                    if word == param {
                        plan.distances.push((inst, offset));
                    }
                }
                Data::Binary {
                    opcode: Op::Iadd, ..
                }
                | Data::BinaryImm64 {
                    opcode: Op::IaddImm,
                    ..
                } if f.dfg.value_type(f.dfg.first_result(inst)) == types::I64 => {
                    let (word, offset) = constant_offset(f, f.dfg.first_result(inst));
                    if word == param {
                        plan.offsets.push((inst, offset.wrapping_sub(bias)));
                    }
                }
                _ => {}
            }
        }
    }
    (!plan.comparisons.is_empty() && plan.distances.iter().any(|&(_, offset)| offset == 0))
        .then_some(plan)
}

impl Plan {
    fn apply(self, f: &mut Function, dom: &DominatorTree) {
        let mut uses = BTreeMap::<Inst, Vec<Value>>::new();
        for block in f.layout.blocks().filter(|&b| dom.is_reachable(b)) {
            for inst in f.layout.block_insts(block) {
                let values = || f.dfg.inst_values(inst).map(|v| f.dfg.resolve_aliases(v));
                if values().any(|v| self.transport.contains(&v)) {
                    uses.insert(inst, values().collect());
                }
            }
        }
        let plain_distances = self
            .distances
            .iter()
            .filter(|&&(_, offset)| offset == 0)
            .map(|&(i, _)| (i, f.dfg.first_result(i)))
            .collect::<Vec<_>>();
        let mut cursor = FuncCursor::new(f);
        let mut decoded = BTreeMap::new();
        for word in self.transport {
            let ValueDef::Param(block, _) = cursor.func.dfg.value_def(word) else {
                unreachable!();
            };
            cursor.goto_first_inst(block);
            let plain = cursor.ins().iadd_imm(word, -(self.bias as i64));
            decoded.insert(word, plain);
        }
        for (inst, args) in uses {
            detach_arguments(cursor.func, inst);
            cursor.func.dfg.overwrite_inst_values(
                inst,
                args.into_iter()
                    .map(|arg| decoded.get(&arg).copied().unwrap_or(arg)),
            );
        }
        for incoming in self.incoming {
            let value = match incoming.value {
                Encoded::Word(word) => word,
                Encoded::Literal(bits) => {
                    cursor.goto_inst(incoming.from);
                    cursor.ins().iconst(types::I64, bits as i64)
                }
            };
            let dfg = &mut cursor.func.dfg;
            let destination = &mut dfg.insts[incoming.from]
                .branch_destination_mut(&mut dfg.jump_tables, &mut dfg.exception_tables)
                [incoming.edge];
            *destination = destination.deep_clone(&mut dfg.value_lists);
            let mut ordinal = 0;
            destination.update_args(&mut dfg.value_lists, |arg| {
                let replace = ordinal == incoming.lane;
                ordinal += 1;
                if replace { value.into() } else { arg }
            });
        }
        for inst in self.comparisons {
            // `prior < raw - bias` and `prior + bias < raw` use the same
            // unsigned ordering when both expressions are in range.
            cursor.func.dfg.replace(inst).icmp(
                IntCC::UnsignedLessThan,
                self.param,
                self.bound_base,
            );
        }
        for (inst, offset) in self.offsets {
            cursor
                .func
                .dfg
                .replace(inst)
                .iadd_imm(self.param, offset as i64);
        }
        for (inst, offset) in self.distances {
            cursor.goto_inst(inst);
            if offset == 0 {
                cursor
                    .func
                    .dfg
                    .replace(inst)
                    .isub(self.bound_base, self.param);
            } else if let Some(&(_, span)) = plain_distances
                .iter()
                .find(|&&(other, _)| dom.dominates(other, inst, &cursor.func.layout))
            {
                cursor
                    .func
                    .dfg
                    .replace(inst)
                    .iadd_imm(span, offset.wrapping_neg() as i64);
            } else {
                let start = cursor.ins().iadd_imm(self.param, offset as i64);
                cursor.func.dfg.replace(inst).isub(self.bound_base, start);
            }
        }
    }
}

fn detach_arguments(f: &mut Function, inst: Inst) {
    // Some CFG rewrites share argument lists. Keep this change local to
    // the instruction whose inputs will be decoded.
    let dfg = &mut f.dfg;
    if let Data::MultiAry { args, .. } | Data::Call { args, .. } | Data::CallIndirect { args, .. } =
        &mut dfg.insts[inst]
    {
        *args = args.deep_clone(&mut dfg.value_lists);
    }
    for destination in
        dfg.insts[inst].branch_destination_mut(&mut dfg.jump_tables, &mut dfg.exception_tables)
    {
        *destination = destination.deep_clone(&mut dfg.value_lists);
    }
}

pub(in crate::native_artifact) fn carry(
    f: &mut Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
) -> bool {
    // Mutating arguments of a table branch can affect other instructions that
    // reference the same table. This variant only handles ordinary edges.
    if f.dfg.values_labels.is_some()
        || !f.dfg.jump_tables.is_empty()
        || !f.dfg.exception_tables.is_empty()
    {
        return false;
    }
    let plan = f
        .layout
        .blocks()
        .filter(|&b| dom.is_reachable(b))
        .flat_map(|b| f.layout.block_insts(b))
        .find_map(|inst| {
            let Data::IntCompare {
                cond: IntCC::UnsignedLessThan,
                args,
                ..
            } = f.dfg.insts[inst]
            else {
                return None;
            };
            let [param, bound] = args.map(|v| f.dfg.resolve_aliases(v));
            if f.dfg.value_type(param) != types::I64
                || !matches!(f.dfg.value_def(param), ValueDef::Param(..))
            {
                return None;
            }
            let (base, bias) = biased_base(f, bound)?;
            derive(f, cfg, dom, param, bound, base, bias, inst)
        });
    if let Some(plan) = plan {
        plan.apply(f, dom);
        true
    } else {
        false
    }
}
