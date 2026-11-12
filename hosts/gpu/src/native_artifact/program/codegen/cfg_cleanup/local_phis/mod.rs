//! A changing outer phi is still one immutable value within a child activation.
//! Solve phi-only dependencies under a dominating loop header, treating values
//! defined before that header as distinct atoms. No instruction is speculated,
//! duplicated, or removed here; only redundant block arguments become aliases.
use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::{Block, BlockCall, Function, Value, ValueDef};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub(super) mod equal_arguments;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Fact {
    Bottom,
    One(Value),
    Many,
}
impl Fact {
    fn join(self, other: Self) -> Self {
        match (self, other) {
            (Self::Bottom, v) | (v, Self::Bottom) => v,
            (Self::One(a), Self::One(b)) if a == b => Self::One(a),
            _ => Self::Many,
        }
    }
}
enum Input {
    Local(usize),
    Fixed(Fact),
}
struct Node {
    value: Value,
    inputs: Vec<Input>,
    users: Vec<usize>,
    fact: Fact,
}
fn intern(value: Value, nodes: &mut Vec<Node>, indices: &mut BTreeMap<Value, usize>) -> usize {
    *indices.entry(value).or_insert_with(|| {
        let index = nodes.len();
        nodes.push(Node {
            value,
            inputs: Vec::new(),
            users: Vec::new(),
            fact: Fact::Bottom,
        });
        index
    })
}

fn graph(
    function: &Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
    header: Block,
) -> Vec<Node> {
    let mut nodes = Vec::new();
    let mut indices = BTreeMap::new();
    for &value in function.dfg.block_params(header) {
        intern(value, &mut nodes, &mut indices);
    }
    let mut index = 0;
    while index < nodes.len() {
        let formal = nodes[index].value;
        let ValueDef::Param(block, ordinal) = function.dfg.value_def(formal) else {
            unreachable!("local phi graph contains only current formals");
        };
        let mut inputs = Vec::new();
        for pred in cfg.pred_iter(block) {
            for call in function.dfg.insts[pred.inst]
                .branch_destination(&function.dfg.jump_tables, &function.dfg.exception_tables)
            {
                if call.block(&function.dfg.value_lists) != block {
                    continue;
                }
                let value = call
                    .args(&function.dfg.value_lists)
                    .nth(ordinal)
                    .and_then(|arg| arg.as_value())
                    .map(|v| function.dfg.resolve_aliases(v));
                let input = value.map_or(Input::Fixed(Fact::Many), |value| {
                    if function.dfg.value_type(value) != function.dfg.value_type(formal) {
                        return Input::Fixed(Fact::Many);
                    }
                    match function.dfg.value_def(value) {
                        ValueDef::Param(owner, _)
                            if dom.is_reachable(owner) && dom.block_dominates(header, owner) =>
                        {
                            let source = intern(value, &mut nodes, &mut indices);
                            nodes[source].users.push(index);
                            Input::Local(source)
                        }
                        ValueDef::Param(owner, _)
                            if dom.is_reachable(owner) && dom.block_dominates(owner, header) =>
                        {
                            Input::Fixed(Fact::One(value))
                        }
                        ValueDef::Result(inst, _)
                            if dom.dominates(inst, header, &function.layout) =>
                        {
                            Input::Fixed(Fact::One(value))
                        }
                        _ => Input::Fixed(Fact::Many),
                    }
                });
                inputs.push(input);
            }
        }
        nodes[index].inputs = inputs;
        index += 1;
    }
    nodes
}

fn solve(mut nodes: Vec<Node>) -> BTreeMap<Value, Value> {
    let mut pending = (0..nodes.len()).collect::<VecDeque<_>>();
    let mut queued = vec![true; nodes.len()];
    while let Some(index) = pending.pop_front() {
        queued[index] = false;
        let derived = nodes[index]
            .inputs
            .iter()
            .fold(Fact::Bottom, |fact, input| {
                fact.join(match input {
                    Input::Local(source) => nodes[*source].fact,
                    Input::Fixed(value) => *value,
                })
            });
        let next = nodes[index].fact.join(derived);
        if next != nodes[index].fact {
            nodes[index].fact = next;
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
        .filter_map(|node| match node.fact {
            Fact::One(value) => Some((node.value, value)),
            _ => None,
        })
        .collect()
}

fn apply(function: &mut Function, cfg: &ControlFlowGraph, replacements: &BTreeMap<Value, Value>) {
    let mut blocks = BTreeMap::new();
    for &value in replacements.keys() {
        let ValueDef::Param(block, _) = function.dfg.value_def(value) else {
            unreachable!("phi replacement lost its current formal");
        };
        blocks
            .entry(block)
            .or_insert_with(|| function.dfg.block_params(block).to_vec());
    }
    let incoming = blocks
        .keys()
        .flat_map(|&block| cfg.pred_iter(block).map(|pred| pred.inst))
        .collect::<BTreeSet<_>>();
    for params in blocks.values() {
        for &value in params.iter().rev() {
            if let Some(&source) = replacements.get(&value) {
                function.dfg.remove_block_param(value);
                function.dfg.change_to_alias(value, source);
            }
        }
    }
    for inst in incoming {
        let dfg = &mut function.dfg;
        for call in
            dfg.insts[inst].branch_destination_mut(&mut dfg.jump_tables, &mut dfg.exception_tables)
        {
            let target = call.block(&dfg.value_lists);
            let Some(params) = blocks.get(&target) else {
                continue;
            };
            let args = call.args(&dfg.value_lists).collect::<Vec<_>>();
            debug_assert_eq!(args.len(), params.len());
            let kept = args
                .into_iter()
                .zip(params)
                .filter_map(|(arg, formal)| (!replacements.contains_key(formal)).then_some(arg));
            *call = BlockCall::new(target, kept, &mut dfg.value_lists);
        }
    }
}

pub(super) fn simplify(
    function: &mut Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
) -> usize {
    // Inner headers precede their dominating parents. Applying aliases leaves
    // all CFG edges intact and makes subsequent outer queries smaller.
    let headers = dom
        .cfg_postorder()
        .iter()
        .copied()
        .filter(|&header| {
            Some(header) != function.layout.entry_block()
                && !function.dfg.block_params(header).is_empty()
                && cfg
                    .pred_iter(header)
                    .any(|pred| dom.block_dominates(header, pred.block))
        })
        .collect::<Vec<_>>();
    let mut removed = 0;
    for header in headers {
        let replacements = solve(graph(function, cfg, dom, header));
        removed += replacements.len();
        apply(function, cfg, &replacements);
    }
    removed
}

#[cfg(test)]
mod tests;
