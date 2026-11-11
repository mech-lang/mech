//! Propagate unsigned bit-width bounds through the current SSA arguments.
use super::{boolean_words, selected_ranges};
use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::condcodes::{CondCode, IntCC};
use cranelift_codegen::ir::{
    Block, BlockCall, Function, Inst, InstBuilder, InstructionData as Data, Opcode, Value, ValueDef,
};
use std::collections::{BTreeMap, VecDeque};

enum Rule {
    Fixed(u32),
    Max(Vec<Value>),
    Min([Value; 2]),
    Unary(Value, Unary),
}

enum Unary {
    Copy,
    Add(u32),
    Min(u32),
    Max(u32),
    Left(u32),
    Right(u32),
}

impl Rule {
    fn sources(&self) -> &[Value] {
        match self {
            Self::Fixed(_) => &[],
            Self::Max(values) => values,
            Self::Min(values) => values,
            Self::Unary(value, _) => std::slice::from_ref(value),
        }
    }

    fn bound(&self, mut width: impl FnMut(Value) -> u32) -> u32 {
        match self {
            Self::Fixed(n) => *n,
            Self::Max(values) => values.iter().copied().map(width).max().unwrap_or(64),
            Self::Min([a, b]) => width(*a).min(width(*b)),
            Self::Unary(value, op) => {
                let known = width(*value);
                match op {
                    Unary::Copy => known,
                    Unary::Add(n) if known == 0 || *n == 0 => known.max(*n),
                    Unary::Add(n) => known.max(*n) + 1,
                    Unary::Min(n) => known.min(*n),
                    Unary::Max(n) => known.max(*n),
                    Unary::Left(n) if known != 0 => known + n,
                    Unary::Left(_) => 0,
                    Unary::Right(n) => known.saturating_sub(*n),
                }
            }
        }
    }
}

fn bits(function: &Function, value: Value) -> Option<u32> {
    let ty = function.dfg.value_type(value);
    let width = ty.bits();
    (ty.is_int() && !ty.is_vector() && (1..=64).contains(&width)).then_some(width)
}

fn mask(width: u32) -> u64 {
    u64::MAX >> (64 - width)
}

fn used_bits(word: u64) -> u32 {
    64 - word.leading_zeros()
}

fn immediate(function: &Function, mut value: Value) -> Option<u64> {
    let mut kept = u64::MAX;
    loop {
        value = function.dfg.resolve_aliases(value);
        kept &= mask(bits(function, value)?);
        let ValueDef::Result(inst, 0) = function.dfg.value_def(value) else {
            return None;
        };
        match function.dfg.insts[inst] {
            Data::UnaryImm {
                opcode: Opcode::Iconst,
                imm,
            } => {
                return Some(imm.bits() as u64 & kept);
            }
            Data::BinaryImm64 {
                opcode: Opcode::BandImm,
                arg,
                imm,
            } => {
                kept &= imm.bits() as u64;
                value = arg;
            }
            _ => return None,
        }
    }
}

fn shift(op: Opcode, amount: u64, width: u32) -> Option<Unary> {
    // The integer shift uses the low bits of its count.
    if !width.is_power_of_two() {
        return None;
    }
    let amount = (amount & u64::from(width - 1)) as u32;
    match op {
        Opcode::Ishl | Opcode::IshlImm => Some(Unary::Left(amount)),
        Opcode::Ushr | Opcode::UshrImm => Some(Unary::Right(amount)),
        _ => None,
    }
}

fn constant_operand(function: &Function, args: [Value; 2]) -> Option<(Value, u64)> {
    if let Some(constant) = immediate(function, args[1]) {
        Some((function.dfg.resolve_aliases(args[0]), constant))
    } else {
        Some((
            function.dfg.resolve_aliases(args[1]),
            immediate(function, args[0])?,
        ))
    }
}

fn add_offset(function: &Function, value: Value) -> Option<(Value, u64)> {
    let ValueDef::Result(inst, 0) = function.dfg.value_def(value) else {
        return None;
    };
    match function.dfg.insts[inst] {
        Data::Binary {
            opcode: Opcode::Iadd,
            args,
        } => constant_operand(function, args),
        Data::BinaryImm64 {
            opcode: Opcode::IaddImm,
            arg,
            imm,
        } => Some((
            function.dfg.resolve_aliases(arg),
            imm.bits() as u64 & mask(bits(function, value)?),
        )),
        _ => None,
    }
}

fn rule(function: &Function, incoming: &BTreeMap<Value, Vec<Value>>, value: Value) -> Rule {
    let Some(width) = bits(function, value) else {
        return Rule::Fixed(64);
    };
    if let Some(values) = incoming.get(&value) {
        return Rule::Max(values.clone());
    }
    let ValueDef::Result(inst, 0) = function.dfg.value_def(value) else {
        return Rule::Fixed(width);
    };
    if let Some((base, offset)) = add_offset(function, value) {
        if let Some((original, prior)) = add_offset(function, base) {
            // The two offsets cancel at the actual modular integer width.
            if offset.wrapping_add(prior) & mask(width) == 0 {
                return Rule::Unary(original, Unary::Copy);
            }
        }
        return Rule::Unary(base, Unary::Add(used_bits(offset)));
    }
    let alias = |arg| function.dfg.resolve_aliases(arg);
    match function.dfg.insts[inst] {
        Data::UnaryImm {
            opcode: Opcode::Iconst,
            imm,
        } => Rule::Fixed(used_bits(imm.bits() as u64 & mask(width))),
        Data::Unary {
            opcode: Opcode::Uextend | Opcode::Ireduce,
            arg,
        } => Rule::Unary(alias(arg), Unary::Copy),
        Data::IntCompare { .. } | Data::IntCompareImm { .. } => Rule::Fixed(1),
        Data::Binary {
            opcode: Opcode::Imul,
            args,
        } => {
            if let Some((base, factor)) = constant_operand(function, args) {
                if factor.is_power_of_two() {
                    return Rule::Unary(base, Unary::Left(factor.trailing_zeros()));
                }
            }
            Rule::Fixed(width)
        }
        Data::Binary {
            opcode: Opcode::Band,
            args,
        } => Rule::Min(args.map(alias)),
        Data::Binary {
            opcode: Opcode::Bor | Opcode::Bxor,
            args,
        } => Rule::Max(args.map(alias).into()),
        Data::Ternary {
            opcode: Opcode::Select,
            args: [_, yes, no],
        } => Rule::Max(vec![alias(yes), alias(no)]),
        Data::Binary {
            opcode: op @ (Opcode::Ishl | Opcode::Ushr),
            args: [arg, amount],
        } => {
            let Some(op) = immediate(function, amount).and_then(|n| shift(op, n, width)) else {
                return Rule::Fixed(width);
            };
            Rule::Unary(alias(arg), op)
        }
        Data::BinaryImm64 { opcode, arg, imm } => {
            let constant = imm.bits() as u64;
            let op = match opcode {
                Opcode::BandImm => Some(Unary::Min(used_bits(constant & mask(width)))),
                Opcode::BorImm | Opcode::BxorImm => {
                    Some(Unary::Max(used_bits(constant & mask(width))))
                }
                Opcode::ImulImm if (constant & mask(width)).is_power_of_two() => {
                    Some(Unary::Left((constant & mask(width)).trailing_zeros()))
                }
                _ => shift(opcode, constant, width),
            };
            op.map_or(Rule::Fixed(width), |op| Rule::Unary(alias(arg), op))
        }
        _ => Rule::Fixed(width),
    }
}

struct Node {
    value: Value,
    rule: Rule,
    users: Vec<usize>,
    width: u32,
}

fn intern(value: Value, nodes: &mut Vec<Node>, indices: &mut BTreeMap<Value, usize>) -> usize {
    *indices.entry(value).or_insert_with(|| {
        let index = nodes.len();
        nodes.push(Node {
            value,
            rule: Rule::Fixed(64),
            users: Vec::new(),
            width: 0,
        });
        index
    })
}

fn infer(
    function: &Function,
    cfg: &ControlFlowGraph,
    roots: impl Iterator<Item = Value>,
) -> BTreeMap<Value, u32> {
    let incoming = boolean_words::inputs(function, cfg);
    let mut nodes = Vec::new();
    let mut indices = BTreeMap::new();
    for value in roots {
        intern(value, &mut nodes, &mut indices);
    }
    let mut index = 0;
    while index < nodes.len() {
        let transfer = rule(function, &incoming, nodes[index].value);
        for &source in transfer.sources() {
            let source = intern(source, &mut nodes, &mut indices);
            nodes[source].users.push(index);
        }
        nodes[index].rule = transfer;
        index += 1;
    }

    // For a reached loop each initial argument comes from a real incoming
    // edge. Join over all edges and propagate every possible increase. Unknown
    // definitions and unsupported incoming values use their full type widths.
    let mut work = (0..nodes.len()).collect::<VecDeque<_>>();
    let mut queued = vec![true; nodes.len()];
    while let Some(index) = work.pop_front() {
        queued[index] = false;
        let node = &nodes[index];
        let bound = node
            .rule
            .bound(|source| nodes[indices[&source]].width)
            .min(bits(function, node.value).unwrap_or(64));
        if bound <= node.width {
            continue;
        }
        nodes[index].width = bound;
        for &user in &nodes[index].users {
            if !queued[user] {
                queued[user] = true;
                work.push_back(user);
            }
        }
    }
    nodes
        .into_iter()
        .map(|node| (node.value, node.width))
        .collect()
}

fn unsigned_guard(function: &Function, value: Value) -> Option<(Value, u32, bool)> {
    let value = function.dfg.resolve_aliases(value);
    let ValueDef::Result(inst, 0) = function.dfg.value_def(value) else {
        return None;
    };
    let (condition, word, limit) = match function.dfg.insts[inst] {
        Data::IntCompareImm {
            opcode: Opcode::IcmpImm,
            cond,
            arg,
            imm,
        } => (cond, arg, imm.bits() as u64),
        Data::IntCompare {
            opcode: Opcode::Icmp,
            cond,
            args: [left, right],
        } => {
            if let Some(value) = immediate(function, right) {
                (cond, left, value)
            } else {
                (cond.swap_args(), right, immediate(function, left)?)
            }
        }
        _ => return None,
    };
    let truth = match condition {
        IntCC::UnsignedLessThanOrEqual => true,
        IntCC::UnsignedGreaterThan => false,
        _ => return None,
    };
    let word = function.dfg.resolve_aliases(word);
    let width = bits(function, word)?;
    let limit = limit & mask(width);
    if limit == mask(width) || limit & (limit + 1) != 0 {
        return None;
    }
    Some((word, used_bits(limit), truth))
}

type WidthGuard = (Inst, Value, u32, BlockCall);

fn current_length_guards(
    function: &Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
) -> Vec<WidthGuard> {
    let mut floors = BTreeMap::<Value, Vec<(Block, u32)>>::new();
    for block in function.layout.blocks().filter(|&b| dom.is_reachable(b)) {
        for (word, range) in selected_ranges::incoming(function, cfg, block, &BTreeMap::new()) {
            if let Some(width) = range.0.checked_ilog2() {
                floors.entry(word).or_default().push((block, width));
            }
        }
    }

    function
        .layout
        .blocks()
        .filter(|&block| dom.is_reachable(block))
        .filter_map(|block| {
            let inst = function.layout.last_inst(block)?;
            let Data::Brif { arg, blocks, .. } = function.dfg.insts[inst] else {
                return None;
            };
            let condition = function.dfg.resolve_aliases(arg);
            let ValueDef::Result(compare, 0) = function.dfg.value_def(condition) else {
                return None;
            };
            let Data::IntCompare {
                cond,
                args: [index, length],
                ..
            } = function.dfg.insts[compare]
            else {
                return None;
            };
            let truth = match cond {
                IntCC::UnsignedLessThan => true,
                IntCC::UnsignedGreaterThanOrEqual => false,
                _ => return None,
            };
            let index = function.dfg.resolve_aliases(index);
            bits(function, index)?;
            let length = function.dfg.resolve_aliases(length);
            // Use only the same SSA length on an actually selected edge.
            // An index of this many bits is strictly below its proven floor.
            let width = floors
                .get(&length)?
                .iter()
                .filter(|&&(selected, _)| dom.dominates(selected, block, &function.layout))
                .map(|&(_, width)| width)
                .max()?;
            Some((inst, index, width, blocks[usize::from(!truth)]))
        })
        .collect()
}

pub(super) fn fold(function: &mut Function, cfg: &ControlFlowGraph, dom: &DominatorTree) -> usize {
    if function.layout.blocks().any(|block| {
        function
            .layout
            .block_insts(block)
            .any(|inst| function.dfg.insts[inst].exception_table().is_some())
    }) {
        return 0;
    }
    let mut guards = function
        .layout
        .blocks()
        .filter_map(|block| {
            let inst = function.layout.last_inst(block)?;
            let Data::Brif { arg, blocks, .. } = function.dfg.insts[inst] else {
                return None;
            };
            let (word, width, truth) = unsigned_guard(function, arg)?;
            Some((inst, word, width, blocks[usize::from(!truth)]))
        })
        .collect::<Vec<_>>();
    guards.extend(current_length_guards(function, cfg, dom));
    if guards.is_empty() {
        return 0;
    }
    let widths = infer(function, cfg, guards.iter().map(|(_, word, _, _)| *word));
    let mut removed = 0;
    for (inst, word, width, target) in guards {
        if widths[&word] > width {
            continue;
        }
        let block = target.block(&function.dfg.value_lists);
        let args = target.args(&function.dfg.value_lists).collect::<Vec<_>>();
        function.dfg.replace(inst).jump(block, &args);
        removed += 1;
    }
    removed
}

#[cfg(test)]
#[path = "known_widths_tests.rs"]
mod tests;
