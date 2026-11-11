//! A closed SSA transfer graph proves zero-or-one words, including loop phis.
//! Start with candidates and remove every node fed by an unproved value. This
//! greatest fixed point accepts copy cycles with Boolean seeds, but never an
//! unrestricted entry argument, fresh load, arithmetic or unsupported edge.
use super::word_ranges::Range;
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::{Function, InstructionData as Data, Opcode, Value, ValueDef};
use std::collections::{BTreeMap, VecDeque};

enum Rule {
    Fixed(bool),
    All(Vec<usize>),
    Any(Vec<usize>),
}
struct Node {
    value: Value,
    rule: Rule,
    users: Vec<usize>,
    possible: bool,
}
fn intern(value: Value, nodes: &mut Vec<Node>, indices: &mut BTreeMap<Value, usize>) -> usize {
    *indices.entry(value).or_insert_with(|| {
        let index = nodes.len();
        nodes.push(Node {
            value,
            rule: Rule::Fixed(false),
            users: Vec::new(),
            possible: true,
        });
        index
    })
}
fn scalar(f: &Function, value: Value) -> bool {
    let ty = f.dfg.value_type(value);
    ty.is_int() && !ty.is_vector() && (1..=64).contains(&ty.bits())
}
pub(super) fn inputs(f: &Function, cfg: &ControlFlowGraph) -> BTreeMap<Value, Vec<Value>> {
    let mut out = BTreeMap::new();
    for block in f.layout.blocks() {
        if Some(block) == f.layout.entry_block() {
            continue;
        }
        let params = f.dfg.block_params(block);
        if params.is_empty() {
            continue;
        }
        let mut args = vec![Some(Vec::new()); params.len()];
        for pred in cfg.pred_iter(block) {
            for call in f.dfg.insts[pred.inst]
                .branch_destination(&f.dfg.jump_tables, &f.dfg.exception_tables)
            {
                if call.block(&f.dfg.value_lists) != block {
                    continue;
                }
                for (arg, values) in call.args(&f.dfg.value_lists).zip(&mut args) {
                    if let Some(value) = arg.as_value() {
                        if let Some(values) = values {
                            values.push(f.dfg.resolve_aliases(value));
                        }
                    } else {
                        *values = None;
                    }
                }
            }
        }
        for (&param, values) in params.iter().zip(args) {
            // A frontend-removed phi may alias an unrestricted entry word.
            // Such an alias is not a new cyclic definition of that word.
            if f.dfg.resolve_aliases(param) == param {
                if let Some(values) = values.filter(|v| !v.is_empty()) {
                    out.insert(param, values);
                }
            }
        }
    }
    out
}

pub(super) fn derive(f: &Function, cfg: &ControlFlowGraph) -> BTreeMap<Value, Range> {
    let inputs = inputs(f, cfg);
    let mut nodes = Vec::new();
    let mut indices = BTreeMap::new();
    for &value in inputs.keys() {
        intern(value, &mut nodes, &mut indices);
    }
    let mut index = 0;
    while index < nodes.len() {
        let value = nodes[index].value;
        let mut dependencies = Vec::new();
        let mut any = false;
        let fixed = if !scalar(f, value) {
            Some(false)
        } else if let Some(args) = inputs.get(&value) {
            dependencies.extend(args.iter().copied());
            None
        } else if let ValueDef::Result(inst, 0) = f.dfg.value_def(value) {
            match f.dfg.insts[inst] {
                Data::UnaryImm {
                    opcode: Opcode::Iconst,
                    imm,
                } => {
                    let mask = u64::MAX >> (64 - f.dfg.value_type(value).bits());
                    Some((imm.bits() as u64 & mask) <= 1)
                }
                Data::IntCompare { .. } | Data::IntCompareImm { .. } => Some(true),
                Data::Unary {
                    opcode: Opcode::Uextend | Opcode::Ireduce,
                    arg,
                } => {
                    dependencies.push(f.dfg.resolve_aliases(arg));
                    None
                }
                Data::Binary {
                    opcode: Opcode::Band | Opcode::Bor | Opcode::Bxor,
                    args,
                } => {
                    any = f.dfg.insts[inst].opcode() == Opcode::Band;
                    dependencies.extend(args.into_iter().map(|v| f.dfg.resolve_aliases(v)));
                    None
                }
                Data::BinaryImm64 {
                    opcode: Opcode::BandImm,
                    arg,
                    imm,
                } => {
                    let mask = u64::MAX >> (64 - f.dfg.value_type(value).bits());
                    if (imm.bits() as u64 & mask) <= 1 {
                        Some(true)
                    } else {
                        dependencies.push(f.dfg.resolve_aliases(arg));
                        None
                    }
                }
                _ => Some(false),
            }
        } else {
            Some(false)
        };
        nodes[index].rule = if let Some(fixed) = fixed {
            Rule::Fixed(fixed)
        } else {
            let dependencies = dependencies
                .into_iter()
                .map(|value| {
                    let source = intern(value, &mut nodes, &mut indices);
                    nodes[source].users.push(index);
                    source
                })
                .collect();
            if any {
                Rule::Any(dependencies)
            } else {
                Rule::All(dependencies)
            }
        };
        index += 1;
    }
    let mut pending = (0..nodes.len()).collect::<VecDeque<_>>();
    let mut queued = vec![true; nodes.len()];
    while let Some(index) = pending.pop_front() {
        queued[index] = false;
        let holds = match &nodes[index].rule {
            Rule::Fixed(value) => *value,
            Rule::All(args) => args.iter().all(|&i| nodes[i].possible),
            Rule::Any(args) => args.iter().any(|&i| nodes[i].possible),
        };
        if nodes[index].possible && !holds {
            nodes[index].possible = false;
            for &user in &nodes[index].users {
                if !queued[user] {
                    queued[user] = true;
                    pending.push_back(user);
                }
            }
        }
    }
    nodes
        .into_iter()
        .filter(|n| n.possible && matches!(f.dfg.value_def(n.value), ValueDef::Param(_, _)))
        .map(|n| (n.value, Range(0, 1)))
        .collect()
}
