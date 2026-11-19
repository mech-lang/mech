//! Forward an already loaded vector's first unequal word to the existing
//! scalar mismatch continuation. The admitted pure scan supplies authority;
//! source effects, limit exits and scalar tails are not bypassed.
use super::*;
use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::ir::BlockCall;
use std::collections::BTreeMap;

type NodeId = usize;
enum Node {
    Existing(Value),
    Position,
    Difference,
    Copy(Inst, Vec<NodeId>),
}

fn available(dom: &DominatorTree, f: &Function, value: Value, at: Inst) -> bool {
    match f.dfg.value_def(value) {
        ValueDef::Result(inst, _) => dom.dominates(inst, at, &f.layout),
        ValueDef::Param(block, _) => dom.dominates(block, at, &f.layout),
        ValueDef::Union(..) => false,
    }
}

struct Builder<'a> {
    resolve: Resolve<'a>,
    entry: Inst,
    position: Value,
    difference: Value,
    bypassed: BTreeSet<Block>,
    nodes: Vec<Node>,
    cache: BTreeMap<Value, NodeId>,
}

impl Builder<'_> {
    fn value(&mut self, value: Value, depth: usize) -> Option<NodeId> {
        if depth > 32 || self.nodes.len() >= 512 {
            return None;
        }
        let value = self.resolve.value(value)?;
        if let Some(&node) = self.cache.get(&value) {
            return Some(node);
        }
        let f = &self.resolve.context.func;
        let node = if value == self.position {
            Node::Position
        } else if value == self.difference {
            Node::Difference
        } else if available(&self.resolve.context.domtree, f, value, self.entry) {
            Node::Existing(value)
        } else {
            let ValueDef::Result(inst, 0) = f.dfg.value_def(value) else {
                return None;
            };
            if !self.bypassed.contains(&f.layout.inst_block(inst)?)
                || f.dfg.inst_results(inst).len() != 1
                || !matches!(
                    f.dfg.insts[inst].opcode(),
                    Opcode::Iconst
                        | Opcode::Iadd
                        | Opcode::IaddImm
                        | Opcode::Icmp
                        | Opcode::IcmpImm
                        | Opcode::Uextend
                        | Opcode::Bxor
                        | Opcode::Band
                        | Opcode::BandImm
                )
            {
                return None;
            }
            // A separately observed source load cannot be reconstructed from
            // just its XOR. Decline rather than inventing a value or reading.
            let args = f
                .dfg
                .inst_args(inst)
                .to_vec()
                .into_iter()
                .map(|value| self.value(value, depth + 1))
                .collect::<Option<Vec<_>>>()?;
            Node::Copy(inst, args)
        };
        let id = self.nodes.len();
        self.nodes.push(node);
        self.cache.insert(value, id);
        Some(id)
    }
}

pub(super) struct Plan {
    source: Inst,
    edge: usize,
    target: Block,
    arguments: Vec<Value>,
    escaped: Vec<Value>,
    dominated: Vec<Block>,
    nodes: Vec<Node>,
    results: Vec<NodeId>,
}

pub(super) struct Handoff {
    target: Block,
    nodes: Vec<Node>,
    results: Vec<NodeId>,
}

pub(super) fn plan(context: &Context, cert: &Certificate, change: Change) -> Option<Plan> {
    if !cfg!(target_endian = "little") {
        return None;
    }
    let f = &context.func;
    let Data::Brif { blocks, .. } = f.dfg.insts[change.comparison] else {
        return None;
    };
    let call = blocks[change.mismatch_edge];
    let target = call.block(&f.dfg.value_lists);
    if blocks[1 - change.mismatch_edge].block(&f.dfg.value_lists) == target {
        return None;
    }
    let arguments = call
        .args(&f.dfg.value_lists)
        .map(|arg| arg.as_value())
        .collect::<Option<Vec<_>>>()?;
    let header = trace(f, cert, cert.header, false)?;
    if header.end != change.limit_check {
        return None;
    }
    let Data::Brif { blocks, .. } = f.dfg.insts[header.end] else {
        return None;
    };
    let body = blocks.iter().find_map(|call| {
        let body = trace(f, cert, call.block(&f.dfg.value_lists), false)?;
        (body.end == change.comparison).then_some(body)
    })?;
    let bypassed = header
        .blocks
        .union(&body.blocks)
        .copied()
        .collect::<BTreeSet<_>>();
    if bypassed.contains(&target) {
        return None;
    }
    // Analyze the additional path before changing production IR. Its new
    // predecessor has precisely the dominance of the future vector entry.
    let mut alternate = Context::new();
    alternate.func = f.clone();
    let bridge = alternate.func.dfg.make_block();
    let at = alternate.func.dfg.append_block_param(bridge, types::I64);
    alternate.func.layout.append_block(bridge);
    alternate
        .func
        .rewrite_branch_destination(change.entry, cert.header, bridge);
    let mut cursor = FuncCursor::new(&mut alternate.func).at_bottom(bridge);
    let condition = cursor.ins().iconst(types::I8, 1);
    let original = arguments
        .iter()
        .copied()
        .map(Into::into)
        .collect::<Vec<_>>();
    cursor
        .ins()
        .brif(condition, cert.header, &[at.into()], target, &original);
    alternate.flowgraph();
    let dominated = f
        .layout
        .blocks()
        .filter(|&block| context.domtree.block_dominates(target, block))
        .collect::<Vec<_>>();
    let dominated_set = dominated.iter().copied().collect::<BTreeSet<_>>();
    let mut escaped = BTreeSet::new();
    for block in f.layout.blocks() {
        for inst in f.layout.block_insts(block) {
            for value in f.dfg.inst_values(inst).map(|v| f.dfg.resolve_aliases(v)) {
                let owner = match f.dfg.value_def(value) {
                    ValueDef::Param(block, _) => block,
                    ValueDef::Result(inst, _) => f.layout.inst_block(inst)?,
                    ValueDef::Union(..) => return None,
                };
                if !bypassed.contains(&owner) {
                    continue;
                }
                if dominated_set.contains(&block) {
                    if !available(&context.domtree, f, value, f.layout.first_inst(target)?) {
                        return None;
                    }
                    escaped.insert(value);
                } else if !available(&alternate.domtree, &alternate.func, value, inst) {
                    return None;
                }
            }
        }
    }
    if escaped.len() > 16 {
        return None;
    }
    if !escaped.is_empty() {
        // A shared target already transports loop-local values through its
        // explicit arguments. Extra direct uses require this sole old edge.
        let mut incoming = context.cfg.pred_iter(target);
        if incoming.next()?.inst != change.comparison || incoming.next().is_some() {
            return None;
        }
    }
    let escaped = escaped.into_iter().collect::<Vec<_>>();
    let mut builder = Builder {
        resolve: Resolve {
            context,
            header: cert.header,
        },
        entry: change.entry,
        position: f.dfg.block_params(cert.header)[0],
        difference: change.difference,
        bypassed,
        nodes: Vec::new(),
        cache: BTreeMap::new(),
    };
    let results = arguments
        .iter()
        .chain(&escaped)
        .map(|&value| builder.value(value, 0))
        .collect::<Option<Vec<_>>>()?;
    Some(Plan {
        source: change.comparison,
        edge: change.mismatch_edge,
        target,
        arguments,
        escaped,
        dominated,
        nodes: builder.nodes,
        results,
    })
}

impl Plan {
    pub(super) fn install(self, f: &mut Function) -> Handoff {
        let mut replacements = BTreeMap::new();
        let mut arguments = self.arguments;
        for value in self.escaped {
            let ty = f.dfg.value_type(value);
            let param = f.dfg.append_block_param(self.target, ty);
            replacements.insert(value, param);
            arguments.push(value);
        }
        let arguments = arguments.into_iter().map(Into::into).collect::<Vec<_>>();
        let target = BlockCall::new(self.target, arguments, &mut f.dfg.value_lists);
        let Data::Brif { blocks, .. } = &mut f.dfg.insts[self.source] else {
            unreachable!("proved scalar mismatch edge");
        };
        blocks[self.edge] = target;
        for block in self.dominated {
            for inst in f.layout.block_insts(block).collect::<Vec<_>>() {
                let mapped = f
                    .dfg
                    .inst_values(inst)
                    .map(|raw| {
                        let value = f.dfg.resolve_aliases(raw);
                        (raw, replacements.get(&value).copied().unwrap_or(value))
                    })
                    .collect::<BTreeMap<_, _>>();
                f.dfg
                    .map_inst_values(inst, |value| mapped.get(&value).copied().unwrap_or(value));
            }
        }
        Handoff {
            target: self.target,
            nodes: self.nodes,
            results: self.results,
        }
    }
}

impl Handoff {
    fn arguments(&self, c: &mut FuncCursor<'_>, at: Value, difference: Value) -> Vec<BlockArg> {
        fn emit(
            nodes: &[Node],
            cache: &mut BTreeMap<NodeId, Value>,
            c: &mut FuncCursor<'_>,
            at: Value,
            difference: Value,
            id: NodeId,
        ) -> Value {
            if let Some(&value) = cache.get(&id) {
                return value;
            }
            let value = match &nodes[id] {
                Node::Existing(value) => *value,
                Node::Position => at,
                Node::Difference => difference,
                Node::Copy(inst, args) => {
                    let args = args
                        .iter()
                        .map(|&arg| emit(nodes, cache, c, at, difference, arg))
                        .collect::<Vec<_>>();
                    let copy = c.func.dfg.clone_inst(*inst);
                    c.func.dfg.inst_args_mut(copy).copy_from_slice(&args);
                    c.insert_inst(copy);
                    c.func.dfg.first_result(copy)
                }
            };
            cache.insert(id, value);
            value
        }
        let mut cache = BTreeMap::new();
        self.results
            .iter()
            .map(|&id| emit(&self.nodes, &mut cache, c, at, difference, id).into())
            .collect()
    }

    /// The existing vector guard proves both complete little-endian words
    /// readable. This edge is entered only for a nonzero vector XOR. Retain
    /// the first unequal word, not the first differing bit of a later word.
    pub(super) fn emit16(self, c: &mut FuncCursor<'_>, at: Value, difference: Value) {
        let low = c.ins().extractlane(difference, 0);
        let high = c.ins().extractlane(difference, 1);
        let first = c.ins().icmp_imm(IntCC::NotEqual, low, 0);
        let later = c.ins().iadd_imm(at, 8);
        let position = c.ins().select(first, at, later);
        let word = c.ins().select(first, low, high);
        let args = self.arguments(c, position, word);
        c.ins().jump(self.target, &args);
    }
}

#[cfg(all(test, feature = "jit", target_pointer_width = "64"))]
mod tests;
