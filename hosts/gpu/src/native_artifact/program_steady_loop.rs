//! An owned, helper-free version of the actual funded source graph. Cold
//! boundaries return a source continuation, never a whole-workload result.
//! The original generated CFG executes refills, insertions and exact errors.
use super::*;
use crate::native_artifact::{
    NativeCompileLimits,
    compiler::{compiled_stack_bytes, limit},
};
use cranelift_codegen::ir::{AbiParam, FuncRef, Opcode};
use cranelift_frontend::{FunctionBuilderContext, Switch};
use cranelift_module::{FuncId, Linkage, Module};
use mech_engine::native_artifact::NativeScalarOp as Op;

#[cfg(test)]
thread_local! {
    static DISABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static EMISSIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
#[cfg(test)]
pub(in crate::native_artifact) fn without_steady_loops<T>(f: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            DISABLED.with(|v| v.set(self.0));
        }
    }
    let _restore = Restore(DISABLED.with(|v| v.replace(true)));
    f()
}
#[cfg(test)]
pub(in crate::native_artifact) fn take_steady_emissions() -> usize {
    EMISSIONS.with(|v| v.replace(0))
}

pub(super) struct Proof {
    graph: tag_threading::Graph,
    bounds: loop_bounds::Proof,
    facts: edge_facts::Proof,
    lookups: Vec<packed_maps::LookupFact>,
    live: Vec<BTreeSet<usize>>,
    nodes: BTreeSet<usize>,
    present: Vec<bool>,
    active: usize,
}
pub(super) struct Definition {
    pub function: FuncId,
    pub stack: u64,
    pub code: u64,
    proof: Proof,
    continuations: Vec<usize>,
    pub extra_fixed_work: usize,
    pub source_work: usize,
}
pub(super) struct Call<'a> {
    pub function: FuncRef,
    pub definition: &'a Definition,
}
impl Definition {
    pub fn active_node(&self) -> usize {
        self.proof.active
    }
}

fn targets(
    graph: &tag_threading::Graph,
    bounds: &loop_bounds::Proof,
    node: usize,
) -> Vec<tag_threading::Target> {
    use tag_threading::Edge;
    let edge = &graph.nodes[node].edge;
    if bounds.later.contains(&node) {
        let Edge::Branch { no, .. } = edge else {
            return Vec::new();
        };
        return vec![*no];
    }
    match edge {
        Edge::Jump(target) => vec![*target],
        Edge::Branch { yes, no } => vec![*yes, *no],
        Edge::Dispatch { .. } | Edge::Reject => Vec::new(),
    }
}

fn derive(
    code: CodePlan<'_>,
    closed: &Layout,
    id: usize,
    owner: &NativeMapOwnerWords,
    aggregates: &NativeScalarAggregates,
    slots: &BTreeSet<usize>,
) -> Option<Proof> {
    #[cfg(test)]
    if DISABLED.with(std::cell::Cell::get)
        || TAG_THREADING_DISABLED.with(std::cell::Cell::get)
        || LOOP_BOUNDS_DISABLED.with(std::cell::Cell::get)
        || LOOKUP_FACTS_DISABLED.with(std::cell::Cell::get)
        || EDGE_FACTS_DISABLED.with(std::cell::Cell::get)
    {
        return None;
    }
    if !invariants::enabled()
        || !mutation_facts::enabled()
        || !count_bounds::enabled()
        || !loop_constants::enabled()
        || !packed_maps::effects_enabled()
    {
        return None;
    }
    let graph = tag_threading::derive(code, closed, id, owner, aggregates)?;
    let bounds = loop_bounds::derive(code, closed, id, owner, aggregates, &graph, slots)?;
    bounds.single_insert_nodes.as_ref()?;
    bounds.count_values.as_ref()?;
    invariants::derive(code, closed, id, owner, &graph, &bounds, slots)?;
    let facts = edge_facts::derive(code, closed, id, owner, aggregates, slots)?;
    let lookups = lookup_facts::derive(code, closed, id, owner, aggregates, &graph)?;
    let live = loop_bounds::continuation_liveness(code, closed, id, owner, aggregates, &graph)?;
    let header = closed.regions[id].tail.header() as usize;
    let active = graph
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| {
            (node.site == header && node.header_tag == Some(bounds.active_tag)).then_some(index)
        })
        .collect::<Vec<_>>();
    if active.len() != 1 || slots.len() > 256 || graph.nodes.len() > 256 {
        return None;
    }
    let active = active[0];
    let mut nodes = BTreeSet::new();
    let mut pending = vec![active];
    while let Some(index) = pending.pop() {
        if !nodes.insert(index) {
            continue;
        }
        let node = &graph.nodes[index];
        if (node.site == header && index != active)
            || matches!(
                node.edge,
                tag_threading::Edge::Reject | tag_threading::Edge::Dispatch { .. }
            )
        {
            continue;
        }
        for target in targets(&graph, &bounds, index) {
            if let tag_threading::Target::Node(next) = target {
                pending.push(next);
            }
        }
    }
    if live.iter().any(|words| !words.is_subset(slots)) {
        return None;
    }
    // Every membership miss completes its original action, then resumes at
    // its original successor with false and a cached missing entry. No probe,
    // source action or traversal is repeated at that continuation.
    let mut memberships = 0;
    for &index in &nodes {
        let node = &graph.nodes[index];
        if owner.call_region(node.site) == closed.regions[id].map
            && let Action::LibraryCall { template, .. } = code.blocks()[node.site].action()
            && template.primitive() == P::MapContainsKey
        {
            if !matches!(node.edge, tag_threading::Edge::Jump(_)) {
                return None;
            }
            memberships += 1;
        }
    }
    if memberships == 0 {
        return None;
    }
    // All continuing membership/get/insert actions establish a live present
    // entry. Selection and headers invalidate it. Intersection covers every
    // syntactic source predecessor; misses leave this function instead.
    let mut incoming = vec![None; graph.nodes.len()];
    incoming[active] = Some(false);
    let mut pending = std::collections::VecDeque::from([active]);
    let mut steps = 0;
    while let Some(index) = pending.pop_front() {
        steps += 1;
        if steps > 4096 {
            return None;
        }
        let node = &graph.nodes[index];
        let mut after = incoming[index]?;
        if node.site == header {
            after = false;
        }
        if owner
            .span_site(node.site)
            .is_some_and(|span| owner.spans()[span].range_site() as usize == node.site)
        {
            after = false;
        }
        if owner.call_region(node.site) == closed.regions[id].map {
            after = true;
        }
        for target in targets(&graph, &bounds, index) {
            let tag_threading::Target::Node(next) = target else {
                continue;
            };
            if !nodes.contains(&next) {
                continue;
            }
            let updated = incoming[next].map_or(after, |old| old && after);
            if incoming[next] != Some(updated) {
                incoming[next] = Some(updated);
                pending.push_back(next);
            }
        }
    }
    let present = incoming
        .into_iter()
        .map(|state| state.unwrap_or(false))
        .collect::<Vec<_>>();
    // A productive source trace must not leave for an insertion on every
    // visit. Arbitrary storage programs retain the ordinary implementation.
    if bounds
        .single_insert_nodes
        .as_ref()?
        .iter()
        .any(|&node| nodes.contains(&node) && !present[node])
    {
        return None;
    }
    Some(Proof {
        graph,
        bounds,
        facts,
        lookups,
        live,
        nodes,
        present,
        active,
    })
}

impl Proof {
    fn live_at(&self, node: usize, returned: usize) -> BTreeSet<usize> {
        self.live
            .get(node)
            .cloned()
            .unwrap_or_else(|| BTreeSet::from([returned]))
    }
    fn target_index(&self, target: tag_threading::Target) -> usize {
        match target {
            tag_threading::Target::Node(node) => node,
            tag_threading::Target::Exit => self.graph.nodes.len(),
        }
    }
}

pub(super) fn define<M: Module>(
    module: &mut M,
    symbol: &str,
    code: CodePlan<'_>,
    closed: &Layout,
    id: usize,
    owner: &NativeMapOwnerWords,
    aggregates: &NativeScalarAggregates,
    slots: &BTreeSet<usize>,
    limits: NativeCompileLimits,
    report: &mut CodegenReport,
) -> Result<Option<Definition>, NativeCompileError> {
    let Some(proof) = derive(code, closed, id, owner, aggregates, slots) else {
        return Ok(None);
    };
    let returned = closed.regions[id].tail.returned().get() as usize;
    let records = proof.nodes.len() + proof.live.iter().map(BTreeSet::len).sum::<usize>();
    if !report.reserve_steady_region(records) {
        return Ok(None);
    }
    let name = format!("{symbol}_funded");
    let mut signature = module.make_signature();
    // Invocation-owned scalar words, packed frame, two immutable hash values
    // and the already source-certified runtime endpoint. No Rust context or
    // helper binding is live in this native function.
    signature.params = vec![AbiParam::new(types::I64); 5];
    signature.returns.push(AbiParam::new(types::I64));
    let function = module
        .declare_function(&name, Linkage::Local, &signature)
        .map_err(backend)?;
    let mut context = module.make_context();
    context.func.signature = signature;
    let mut fb = FunctionBuilderContext::new();
    let (entry_work, source_work, snapshot_work, bail_blocks);
    let specialized_widths =
        (1..=crate::native_artifact::program_packed_map::MAX_WIDTH).collect::<Vec<_>>();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let args = b.block_params(entry).to_vec();
        // The private storage constructor admits only a bounded width domain.
        // Select on the ACTUAL immutable Frame width, never an input sample
        // or workload name. Every arm emits the identical source graph; only
        // width and its constructor-derived masks become native constants.
        let width = b.ins().load(
            types::I64,
            MemFlags::new(),
            args[1],
            core::mem::offset_of!(Frame, width) as i32,
        );
        let generic = b.create_block();
        let mut selector = Switch::new();
        let variants = specialized_widths
            .iter()
            .map(|&width| {
                let block = b.create_block();
                selector.set_entry(u128::from(width), block);
                (Some(width), block)
            })
            .collect::<Vec<_>>();
        selector.emit(&mut b, width, generic);
        let dispatch_work = b.func.dfg.num_insts();
        let mut maximum_entry = 0;
        let mut maximum_source = 0;
        let mut maximum_snapshot = 0;
        let mut all_bails = Vec::new();
        let check_edges = std::iter::once(None)
            .chain(
                proof
                    .nodes
                    .iter()
                    .copied()
                    .filter(|&index| {
                        targets(&proof.graph, &proof.bounds, index)
                            .contains(&tag_threading::Target::Node(proof.active))
                    })
                    .map(Some),
            )
            .collect::<Vec<_>>();
        let mut all_checks = Vec::new();
        let mut maximum_check = 0;
        for (width, variant) in std::iter::once((None, generic)).chain(variants) {
            b.switch_to_block(variant);
            let entry_start = b.func.dfg.num_insts();
            let words = Words::new(&mut b, args[0], code, slots.clone())?;
            let packed = FrameState::registers(
                &mut b,
                args[1],
                closed.regions[id].packed.as_ref().unwrap().packing_domain(),
            )
            .with_constant_values([args[2], args[3]])
            .with_specialized_width(width);
            let blocks = proof
                .nodes
                .iter()
                .map(|&node| (node, b.create_block()))
                .collect::<BTreeMap<_, _>>();
            let mut bail = proof
                .nodes
                .iter()
                .map(|&node| (node, b.create_block()))
                .collect::<BTreeMap<_, _>>();
            bail.insert(proof.graph.nodes.len(), b.create_block());
            let checks = check_edges
                .iter()
                .map(|&edge| (edge, b.create_block()))
                .collect::<BTreeMap<_, _>>();
            let zero = b.ins().iconst(types::I64, 0);
            b.ins().jump(checks[&None], &[]);
            maximum_entry = maximum_entry.max(b.func.dfg.num_insts() - entry_start);
            let mut maximum = 0;
            for &index in &proof.nodes {
                let target = |target| match target {
                    tag_threading::Target::Node(node) if node == proof.active => {
                        checks[&Some(index)]
                    }
                    tag_threading::Target::Node(node) => blocks[&node],
                    tag_threading::Target::Exit => bail[&proof.graph.nodes.len()],
                };
                let node = &proof.graph.nodes[index];
                b.switch_to_block(blocks[&index]);
                let start = b.func.dfg.num_insts();
                if (node.site == closed.regions[id].tail.header() as usize && index != proof.active)
                    || matches!(
                        node.edge,
                        tag_threading::Edge::Reject | tag_threading::Edge::Dispatch { .. }
                    )
                {
                    b.ins().jump(bail[&index], &[]);
                    continue;
                }
                let failed = b.create_block();
                b.append_block_param(failed, types::I64);
                b.set_cold_block(failed);
                if index == proof.active {
                    // Every incoming edge performs the original test before
                    // reaching this debit. Its cold snapshot is outside the
                    // loop header, so prior cache/frame values need not be
                    // carried through the next productive iteration.
                    let fuel = packed.read(&mut b, core::mem::offset_of!(Frame, fuel));
                    let fuel = b.ins().iadd_imm(fuel, -1);
                    packed.write(&mut b, core::mem::offset_of!(Frame, fuel), fuel);
                    let tag = b
                        .ins()
                        .iconst(types::I64, i64::from(proof.bounds.active_tag));
                    words.write(&mut b, proof.graph.carrier_word, tag);
                }
                if let Some((layout, action)) = aggregate_at(
                    owner,
                    aggregates,
                    closed.regions[id].map.unwrap(),
                    node.site,
                ) {
                    emit_aggregate_body(
                        &mut b,
                        &words,
                        layout,
                        action,
                        node.site,
                        &proof.facts,
                        failed,
                        None,
                    )?;
                } else if let Some(span) = owner.span_site(node.site).map(|i| &owner.spans()[i]) {
                    if span.range_site() as usize == node.site {
                        if !proof.bounds.ranges.contains(&index) {
                            return Err(backend("steady selection lacks original bound"));
                        }
                        let first = words.read(&mut b, span.operands()[0].get() as usize);
                        packed_maps::select_leaf(&mut b, &packed, first);
                    }
                } else if owner.call_region(node.site) == closed.regions[id].map {
                    let Action::LibraryCall {
                        template,
                        operands,
                        output,
                    } = code.blocks()[node.site].action()
                    else {
                        unreachable!()
                    };
                    let fact = proof.lookups[index];
                    let value = if template.primitive() == P::MapContainsKey {
                        let pointer = packed_maps::probe_leaf(
                            &mut b,
                            &packed,
                            fact,
                            proof.present[index],
                            bail[&index],
                        );
                        let hit = b.create_block();
                        let miss = b.create_block();
                        b.set_cold_block(miss);
                        b.ins().brif(pointer, hit, &[], miss, &[]);
                        b.switch_to_block(miss);
                        words.write(&mut b, output.get() as usize, zero);
                        let tag_threading::Edge::Jump(next) = node.edge else {
                            unreachable!()
                        };
                        b.ins().jump(bail[&proof.target_index(next)], &[]);
                        b.switch_to_block(hit);
                        b.ins().iconst(types::I64, 1)
                    } else {
                        let operation = if template.primitive() == P::MapKeyAccess {
                            4
                        } else {
                            5
                        };
                        let replacement = if operation == 5 {
                            words.read(&mut b, operands[2].get() as usize)
                        } else {
                            zero
                        };
                        let token = words.read(&mut b, operands[0].get() as usize);
                        // Unknown reads leave before probing; successful source
                        // membership dominates the normal read/update path.
                        if !proof.present[index] && fact != packed_maps::LookupFact::Present {
                            b.ins().jump(bail[&index], &[]);
                            b.switch_to_block(failed);
                            b.ins().jump(bail[&index], &[]);
                            continue;
                        }
                        packed_maps::operation_leaf(
                            &mut b,
                            &packed,
                            operation,
                            replacement,
                            token,
                            fact,
                            true,
                            proof
                                .bounds
                                .single_insert_nodes
                                .as_ref()
                                .unwrap()
                                .contains(&index),
                            bail[&index],
                        )
                    };
                    words.write(&mut b, output.get() as usize, value);
                } else {
                    match code.blocks()[node.site].action() {
                        Action::Scalar(Scalar::None) => {}
                        Action::Scalar(Scalar::Copy { source, output }) => {
                            let value = words.read(&mut b, source.get() as usize);
                            words.write(&mut b, output.get() as usize, value);
                        }
                        Action::Scalar(Scalar::Primitive {
                            op,
                            operands,
                            output,
                            ..
                        }) => {
                            let args = operands
                                .iter()
                                .map(|v| words.read(&mut b, v.get() as usize))
                                .collect::<Vec<_>>();
                            let value = if *op == Op::U64Add
                                && proof
                                    .bounds
                                    .count_values
                                    .as_ref()
                                    .unwrap()
                                    .additions
                                    .contains(&index)
                            {
                                b.ins().iadd(args[0], args[1])
                            } else if proof.facts.arithmetic(node.site, *op, operands) {
                                match op {
                                    Op::U64Add => b.ins().iadd(args[0], args[1]),
                                    Op::U64Sub => b.ins().isub(args[0], args[1]),
                                    _ => unreachable!(),
                                }
                            } else {
                                primitive(&mut b, *op, &args, failed)
                            };
                            words.write(&mut b, output.get() as usize, value);
                        }
                        _ => unreachable!("steady source action proof"),
                    }
                }
                if index == proof.bounds.first {
                    let cursor = words.read(&mut b, proof.bounds.index_word);
                    let stop = b
                        .ins()
                        .icmp(IntCC::UnsignedGreaterThanOrEqual, cursor, args[4]);
                    let tag_threading::Edge::Branch { yes, no } = node.edge else {
                        unreachable!()
                    };
                    b.ins().brif(stop, target(yes), &[], target(no), &[]);
                } else if proof.bounds.later.contains(&index) {
                    let tag_threading::Edge::Branch { no, .. } = node.edge else {
                        unreachable!()
                    };
                    b.ins().jump(target(no), &[]);
                } else {
                    match &node.edge {
                        tag_threading::Edge::Jump(next) => {
                            b.ins().jump(target(*next), &[]);
                        }
                        tag_threading::Edge::Branch { yes, no } => {
                            emit_source_terminator(
                                &mut b,
                                &words,
                                code,
                                code.blocks()[node.site].terminator(),
                                failed,
                                |source| {
                                    let source_yes = match code.blocks()[node.site].terminator() {
                                        Term::Scalar(
                                            Branch::Branch { yes, .. }
                                            | Branch::LiteralBranch { yes, .. },
                                        ) => yes,
                                        _ => unreachable!(),
                                    };
                                    target(if source == source_yes { *yes } else { *no })
                                },
                            );
                        }
                        _ => unreachable!(),
                    }
                }
                b.switch_to_block(failed);
                b.ins().jump(bail[&index], &[]);
                maximum = maximum.max(b.func.dfg.num_insts() - start);
            }
            maximum_source = maximum_source.max(maximum);
            for (&edge, &check) in &checks {
                b.switch_to_block(check);
                let start = b.func.dfg.num_insts();
                // These are exactly the old header-prefix operations. They
                // stay after the selected source predecessor and before any
                // original header action or debit. No source visit is skipped
                // or prepaid, including the final terminal traversal.
                let counter = words.read(&mut b, proof.bounds.index_word);
                packed.write(&mut b, core::mem::offset_of!(Frame, updates), counter);
                let fuel = packed.read(&mut b, core::mem::offset_of!(Frame, fuel));
                b.ins()
                    .brif(fuel, blocks[&proof.active], &[], bail[&proof.active], &[]);
                maximum_check = maximum_check.max(b.func.dfg.num_insts() - start);
                all_checks.push((edge, check));
            }
            let mut maximum = 0;
            for (&index, &block) in &bail {
                b.switch_to_block(block);
                b.set_cold_block(block);
                let start = b.func.dfg.num_insts();
                words.spill_live(&mut b, args[0], &proof.live_at(index, returned))?;
                packed.spill(&mut b);
                let continuation = b.ins().iconst(types::I64, (index + 1) as i64);
                b.ins().return_(&[continuation]);
                maximum = maximum.max(b.func.dfg.num_insts() - start);
            }
            maximum_snapshot = maximum_snapshot.max(maximum);
            all_bails.extend(bail);
        }
        entry_work = dispatch_work + maximum_entry + maximum_check;
        source_work = maximum_source + maximum_check;
        snapshot_work = maximum_snapshot;
        bail_blocks = (all_bails, all_checks, maximum_check);
        b.seal_all_blocks();
        b.finalize();
    }
    let initial_cleanup = super::super::cfg_cleanup::optimize(&mut context, module.isa())?;
    // The helper-free graph has no externally callable continuation. Fold its
    // scalar expressions before retiring constant control edges and their
    // phi inputs; otherwise the backend keeps source branches on literal
    // booleans. The large ordinary/checked function retains its single-pass
    // policy. The final backend still performs its ordinary speed pass.
    context
        .optimize(
            module.isa(),
            &mut cranelift_codegen::control::ControlPlane::default(),
        )
        .map_err(backend)?;
    let mut cleanup = super::super::cfg_cleanup::optimize(&mut context, module.isa())?;
    cleanup.pre_codegen_data_passes = 1;
    cleanup.constant_branches_removed += initial_cleanup.constant_branches_removed;
    cleanup.before_blocks = initial_cleanup.before_blocks;
    cleanup.before_instructions = initial_cleanup.before_instructions;
    let fuel_check_edges = bail_blocks
        .1
        .iter()
        .filter_map(|&(edge, block)| context.func.layout.is_block_inserted(block).then_some(edge))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let maximum_fuel_check_instructions = bail_blocks.2;
    let continuations = bail_blocks
        .0
        .iter()
        .filter_map(|&(index, block)| {
            context
                .func
                .layout
                .is_block_inserted(block)
                .then_some(index)
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let native_calls = context
        .func
        .layout
        .blocks()
        .flat_map(|block| context.func.layout.block_insts(block))
        .filter(|&inst| {
            matches!(
                context.func.dfg.insts[inst].opcode(),
                Opcode::Call
                    | Opcode::CallIndirect
                    | Opcode::ReturnCall
                    | Opcode::ReturnCallIndirect
            )
        })
        .count();
    if native_calls != 0 {
        return Err(backend(
            "steady source function contains a native helper call",
        ));
    }
    module
        .define_function(function, &mut context)
        .map_err(backend)?;
    let compiled = context
        .compiled_code()
        .ok_or_else(|| backend("steady function has no compiled code"))?;
    let stack = compiled_stack_bytes(&compiled.buffer)?;
    let bytes = u64::from(compiled.code_info().total_size);
    limit("Steady function stack", stack, limits.max_stack_bytes)?;
    limit("Steady function code", bytes, limits.max_code_bytes)?;
    report
        .steady_loops
        .push(crate::native_artifact::iteration_compiler::SteadyLoop {
            header: closed.regions[id].tail.header() as usize,
            active_node: proof.active,
            active_tag: proof.bounds.active_tag,
            symbol: name,
            source_nodes: proof
                .nodes
                .iter()
                .map(|&node| (node, proof.graph.nodes[node].site))
                .collect(),
            present_nodes: proof
                .nodes
                .iter()
                .copied()
                .filter(|&node| proof.present[node])
                .collect(),
            continuations: continuations
                .iter()
                .map(|&node| {
                    (
                        node + 1,
                        proof.graph.nodes.get(node).map(|n| n.site),
                        proof.live_at(node, returned).into_iter().collect(),
                    )
                })
                .collect(),
            entry_words: proof.live[proof.active].iter().copied().collect(),
            specialized_widths,
            fuel_check_edges,
            maximum_fuel_check_instructions,
            entry_instructions: entry_work,
            maximum_source_instructions: source_work,
            maximum_snapshot_instructions: snapshot_work,
            caller_instructions: 0,
            native_calls,
            stack_bytes: stack,
            code_bytes: bytes,
            cleanup,
        });
    #[cfg(test)]
    EMISSIONS.with(|v| v.set(v.get() + 1));
    Ok(Some(Definition {
        function,
        stack,
        code: bytes,
        proof,
        continuations,
        extra_fixed_work: entry_work + snapshot_work,
        source_work,
    }))
}

pub(super) fn emit_call(
    b: &mut FunctionBuilder<'_>,
    call: &Call<'_>,
    words: &Words,
    packed: &FrameState,
    locals: Value,
    pointer: Value,
    packet: Value,
    blocks: &[Block],
    stride: usize,
    exit: Block,
    error: Block,
    returned: usize,
) -> Result<usize, NativeCompileError> {
    let proof = &call.definition.proof;
    let start = b.func.dfg.num_insts();
    words.spill_live(b, locals, &proof.live[proof.active])?;
    packed.spill(b);
    let values = (0..3)
        .map(|word| {
            b.ins().load(
                types::I64,
                MemFlags::new().with_readonly(),
                packet,
                word * 8,
            )
        })
        .collect::<Vec<_>>();
    let result = b.ins().call(
        call.function,
        &[locals, pointer, values[0], values[1], values[2]],
    );
    let continuation = b.inst_results(result)[0];
    packed.reload_generated(b);
    let bad = b.create_block();
    b.set_cold_block(bad);
    let mut switch = Switch::new();
    let resumes = call
        .definition
        .continuations
        .iter()
        .map(|&node| {
            let block = b.create_block();
            switch.set_entry((node + 1) as u128, block);
            (node, block)
        })
        .collect::<Vec<_>>();
    switch.emit(b, continuation, bad);
    let common_work = b.func.dfg.num_insts() - start;
    let mut resume_work = 0;
    for (node, block) in resumes {
        b.switch_to_block(block);
        let start = b.func.dfg.num_insts();
        words.reload_live(b, locals, &proof.live_at(node, returned))?;
        b.ins().jump(
            if node == proof.graph.nodes.len() {
                exit
            } else {
                blocks[node + 3 * stride]
            },
            &[],
        );
        resume_work = resume_work.max(b.func.dfg.num_insts() - start);
    }
    b.switch_to_block(bad);
    let status = b
        .ins()
        .iconst(types::I64, crate::native_artifact::abi::STATUS_ABI as i64);
    b.ins().jump(error, &[status.into()]);
    Ok(common_work + resume_work + 2)
}
