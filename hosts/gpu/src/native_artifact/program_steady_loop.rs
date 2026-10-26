//! An owned, helper-free version of the actual funded source graph. Cold
//! boundaries return a source continuation, never a whole-workload result.
//! The original generated CFG executes refills, insertions and exact errors.
use super::*;
use crate::native_artifact::compilation::{CompilationControls, OptimizationPass};
use crate::native_artifact::{
    NativeCompileLimits,
    compiler::{compiled_stack_bytes, limit},
};
use cranelift_codegen::ir::{AbiParam, FuncRef, Opcode, StackSlot, StackSlotData, StackSlotKind};
use cranelift_frontend::{FunctionBuilderContext, Switch, Variable};
use cranelift_module::{FuncId, Linkage, Module};
use mech_engine::native_artifact::NativeScalarOp as Op;
#[path = "program_cold_recipes.rs"]
mod cold_recipes;

pub(super) struct Proof {
    graph: tag_threading::Graph,
    bounds: loop_bounds::Proof,
    facts: edge_facts::Proof,
    lookups: Vec<packed_maps::LookupFact>,
    live: Vec<BTreeSet<usize>>,
    nodes: BTreeSet<usize>,
    present: Vec<bool>,
    active: usize,
    snapshots: Option<cold_recipes::Proof>,
}
pub(super) struct Definition {
    pub function: FuncId,
    pub cached_function: Option<FuncId>,
    cache_max_width: u64,
    pub stack: u64,
    pub code: u64,
    proof: Proof,
    continuations: Vec<usize>,
    pub extra_fixed_work: usize,
    pub source_work: usize,
}
pub(super) struct Call<'a> {
    pub function: FuncRef,
    pub cached_function: Option<FuncRef>,
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
    controls: &CompilationControls,
    code: CodePlan<'_>,
    closed: &Layout,
    id: usize,
    owner: &NativeMapOwnerWords,
    aggregates: &NativeScalarAggregates,
    slots: &BTreeSet<usize>,
) -> Option<Proof> {
    if !controls.enabled(OptimizationPass::SteadyLoops)
        || !controls.enabled(OptimizationPass::TagThreading)
        || !controls.enabled(OptimizationPass::LoopBounds)
        || !controls.enabled(OptimizationPass::LookupFacts)
        || !controls.enabled(OptimizationPass::EdgeFacts)
    {
        return None;
    }
    if !invariants::enabled(controls)
        || !mutation_facts::enabled(controls)
        || !count_bounds::enabled(controls)
        || !loop_constants::enabled(controls)
        || !controls.enabled(OptimizationPass::FrameEffects)
    {
        return None;
    }
    let graph = tag_threading::derive(code, closed, id, owner, aggregates)?;
    let bounds = loop_bounds::derive(code, closed, id, owner, aggregates, &graph, slots)?;
    bounds.single_insert_nodes.as_ref()?;
    bounds.count_values.as_ref()?;
    invariants::derive(controls, code, closed, id, owner, &graph, &bounds, slots)?;
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
    // Each next active header must be the certified completed +1 source
    // recurrence. This permits one native induction variable to represent
    // consumed prepaid visits without mutating fuel on every source header.
    if nodes.iter().any(|&node| {
        targets(&graph, &bounds, node).contains(&tag_threading::Target::Node(active))
            && !bounds.recurrence_nodes.contains(&node)
    }) {
        return None;
    }
    let mut proof = Proof {
        graph,
        bounds,
        facts,
        lookups,
        live,
        nodes,
        present,
        active,
        snapshots: None,
    };
    proof.snapshots =
        cold_recipes::derive(controls, code, closed, id, owner, aggregates, &proof, slots);
    Some(proof)
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

// The function has no helper or external observation between its precise
// exits. Strip-mine the original source endpoint by its already admitted fuel.
// A cold-only stack word reconstructs the exact remaining prepaid visits at
// every original continuation. No credit is enlarged, renewed or omitted.
struct PrepaidExtent {
    cursor: Variable,
    bias: Value,
    base: StackSlot,
    stop: Value,
}
impl PrepaidExtent {
    fn new(
        b: &mut FunctionBuilder<'_>,
        base: StackSlot,
        index: Value,
        stop: Value,
        credit: Value,
        bias: Value,
    ) -> Self {
        let cursor = b.declare_var(types::I64);
        let pointer = b.ins().iadd(bias, index);
        b.def_var(cursor, pointer);
        // Both values are immutable private call arguments. Preparing them in
        // the caller prevents backend rematerialization of the min expression
        // inside the loop and avoids retaining all three of its scalar inputs.
        b.ins().stack_store(credit, base, 0);
        Self {
            cursor,
            bias,
            base,
            stop,
        }
    }
    fn pointer(&self, b: &mut FunctionBuilder<'_>) -> Value {
        b.use_var(self.cursor)
    }
    fn index(&self, b: &mut FunctionBuilder<'_>) -> Value {
        let pointer = self.pointer(b);
        b.ins().isub(pointer, self.bias)
    }
    fn snapshot_index(&self, b: &mut FunctionBuilder<'_>, packed: &FrameState) -> Value {
        let source = packed.snapshot_immutable(b, core::mem::offset_of!(Frame, source_ptr));
        let width = packed.snapshot_immutable(b, core::mem::offset_of!(Frame, width));
        let bias = b.ins().iadd(source, width);
        let bias = b.ins().iadd_imm(bias, -1);
        let pointer = self.pointer(b);
        b.ins().isub(pointer, bias)
    }
    fn advance(&self, b: &mut FunctionBuilder<'_>) {
        let pointer = self.pointer(b);
        let next = b.ins().iadd_imm(pointer, 1);
        b.def_var(self.cursor, next);
    }
    fn retire(&self, b: &mut FunctionBuilder<'_>, packed: &FrameState, header_debit: bool) {
        let credit = b.ins().stack_load(types::I64, self.base, 0);
        let pointer = self.pointer(b);
        let fuel = b.ins().isub(credit, pointer);
        let fuel = if header_debit {
            b.ins().iadd_imm(fuel, -1)
        } else {
            fuel
        };
        packed.write(b, core::mem::offset_of!(Frame, fuel), fuel);
    }
}

fn emit_snapshot(
    b: &mut FunctionBuilder<'_>,
    words: &Words,
    packed: &FrameState,
    locals: Value,
    proof: &Proof,
    extent: &PrepaidExtent,
    header_debit: bool,
    incoming: Option<usize>,
    node: usize,
    returned: usize,
    emissions: &mut Vec<(
        Block,
        crate::native_artifact::iteration_compiler::ColdSnapshot,
    )>,
) -> Result<usize, NativeCompileError> {
    let block = b.current_block().expect("snapshot block");
    b.set_cold_block(block);
    let start = b.func.dfg.num_insts();
    extent.retire(b, packed, header_debit);
    let live = proof.live_at(node, returned);
    let snapshot = proof.snapshots.as_ref().and_then(|facts| {
        if header_debit {
            facts.before.get(&node)
        } else if let Some(edge) = incoming {
            facts.completed.get(&edge)
        } else {
            facts.before.get(&proof.active)
        }
    });
    if let Some(snapshot) = snapshot {
        let index = (!snapshot.words.is_empty()
            || snapshot.updates.is_some()
            || snapshot.previous.is_some())
        .then(|| extent.snapshot_index(b, packed));
        let shifted = |b: &mut FunctionBuilder<'_>, offset: u64| {
            let index = index.expect("actual affine recipe");
            if offset == 0 {
                index
            } else {
                b.ins().iadd_imm(index, offset as i64)
            }
        };
        for &word in &live {
            let value = match snapshot.words.get(&word) {
                Some(&offset) => shifted(b, offset),
                None => words.read(b, word),
            };
            b.ins().store(MemFlags::new(), value, locals, offset(word)?);
        }
        if let Some(offset) = snapshot.updates {
            let value = shifted(b, offset);
            packed.write(b, core::mem::offset_of!(Frame, updates), value);
        }
        if let Some(offset) = snapshot.previous {
            let value = shifted(b, offset);
            packed.write(b, core::mem::offset_of!(Frame, previous_start), value);
        }
        if index.is_some() {
            emissions.push((
                block,
                crate::native_artifact::iteration_compiler::ColdSnapshot {
                    node,
                    header: !header_debit,
                    incoming,
                    words: snapshot
                        .words
                        .iter()
                        .map(|(&word, &offset)| (word, offset))
                        .collect(),
                    updates: snapshot.updates,
                    previous: snapshot.previous,
                    reload_words: [
                        core::mem::offset_of!(Frame, source_ptr) / 8,
                        core::mem::offset_of!(Frame, width) / 8,
                    ],
                },
            ));
        }
    } else {
        words.spill_live(b, locals, &live)?;
    }
    packed.spill(b);
    let continuation = b.ins().iconst(types::I64, (node + 1) as i64);
    b.ins().return_(&[continuation]);
    Ok(b.func.dfg.num_insts() - start)
}

pub(super) fn define<M: Module>(
    controls: &CompilationControls,
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
    if !controls.enabled(OptimizationPass::LookupSplit) {
        return define_version(
            controls,
            module,
            symbol,
            code,
            closed,
            id,
            owner,
            aggregates,
            slots,
            limits,
            report,
            packed_maps::LookupMode::Dynamic,
        );
    }
    let Some(mut ordinary) = define_version(
        controls,
        module,
        symbol,
        code,
        closed,
        id,
        owner,
        aggregates,
        slots,
        limits,
        report,
        packed_maps::LookupMode::Hash,
    )?
    else {
        return Ok(None);
    };
    let index = report.steady_loops.len() - 1;
    if let Some(cached) = define_version(
        controls,
        module,
        symbol,
        code,
        closed,
        id,
        owner,
        aggregates,
        slots,
        limits,
        report,
        packed_maps::LookupMode::Direct,
    )? {
        controls.nest_steady_region();
        let cached_report = report
            .steady_loops
            .pop()
            .expect("just emitted cache function");
        // Hash probing has every source continuation the direct lookup needs.
        // Keep one caller dispatch, with the same live state at every exit.
        if !cached
            .continuations
            .iter()
            .all(|node| ordinary.continuations.contains(node))
        {
            return Err(backend(
                "cache function requires an unshared source continuation",
            ));
        }
        ordinary.cached_function = Some(cached.function);
        ordinary.cache_max_width = *cached_report.specialized_widths.last().unwrap();
        ordinary.stack = ordinary.stack.max(cached.stack);
        ordinary.code = ordinary
            .code
            .checked_add(cached.code)
            .ok_or_else(|| backend("lookup function code overflow"))?;
        ordinary.extra_fixed_work = ordinary.extra_fixed_work.max(cached.extra_fixed_work);
        ordinary.source_work = ordinary.source_work.max(cached.source_work);
        let record = &mut report.steady_loops[index];
        record.lookup_cache_selection_word = Some(core::mem::offset_of!(Frame, lookup_cache) / 8);
        record.lookup_cache_function = Some(Box::new(cached_report));
    }
    Ok(Some(ordinary))
}

fn define_version<M: Module>(
    controls: &CompilationControls,
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
    mode: packed_maps::LookupMode,
) -> Result<Option<Definition>, NativeCompileError> {
    let Some(mut proof) = derive(controls, code, closed, id, owner, aggregates, slots) else {
        return Ok(None);
    };
    let direct = mode == packed_maps::LookupMode::Direct;
    let domain = closed.regions[id].packed.as_ref().unwrap().packing_domain();
    let returned = closed.regions[id].tail.returned().get() as usize;
    let specialized_widths = (1..=crate::native_artifact::program_packed_map::MAX_WIDTH)
        .filter(|&width| {
            !direct
                || crate::native_artifact::program_packed_map::lookup_cache_entries(domain, width)
                    .is_some()
        })
        .collect::<Vec<_>>();
    if specialized_widths.is_empty() {
        return Ok(None);
    }
    assert!(
        !direct
            || specialized_widths
                .iter()
                .copied()
                .eq(1..=*specialized_widths.last().unwrap())
    );
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
    let records = 2 * proof.nodes.len()
        + proof.live.iter().map(BTreeSet::len).sum::<usize>()
        + specialized_widths.len()
        + 4 * check_edges.len()
        + 2;
    let extra_records = proof
        .snapshots
        .as_ref()
        .map_or(0, cold_recipes::Proof::records);
    if !controls.reserve_steady_region(records + extra_records) {
        // Metadata admission is unchanged. If the optional recipes cannot
        // fit, retain the already admitted checked snapshot implementation.
        proof.snapshots = None;
        if !controls.reserve_steady_region(records) {
            return Ok(None);
        }
    }
    let name = if direct {
        format!("{symbol}_lookup_cache_funded")
    } else {
        format!("{symbol}_funded")
    };
    let mut signature = module.make_signature();
    // Invocation-owned scalar words, packed frame, two immutable hash values,
    // the prepaid cursor stop and its cold fuel base. No Rust context or helper
    // binding is live in this native function.
    signature.params = vec![AbiParam::new(types::I64); 6];
    signature.returns.push(AbiParam::new(types::I64));
    let function = module
        .declare_function(&name, Linkage::Local, &signature)
        .map_err(backend)?;
    let mut context = module.make_context();
    context.func.signature = signature;
    let mut fb = FunctionBuilderContext::new();
    let (entry_work, source_work, snapshot_work, bail_blocks);
    let mut candidate_value_loads = 0;
    let mut paired_candidate_loads = 0;
    let mut snapshot_emissions = Vec::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let args = b.block_params(entry).to_vec();
        // Exclusive width arms reuse the same actual eight-byte callee slot.
        // Its final aligned native stack extent is admitted below as usual.
        let credit_base =
            b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 3));
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
        let mut variants = specialized_widths
            .iter()
            .map(|&width| {
                let block = b.create_block();
                selector.set_entry(u128::from(width), block);
                (Some(width), block)
            })
            .collect::<Vec<_>>();
        selector.emit(&mut b, width, generic);
        if direct {
            // The caller also checks this exact immutable width domain. A
            // malformed internal call returns an invalid continuation before
            // accessing source/cache storage; the caller reports ABI failure.
            b.switch_to_block(generic);
            let invalid = b.ins().iconst(types::I64, 0);
            b.ins().return_(&[invalid]);
        } else {
            variants.insert(0, (None, generic));
        }
        let dispatch_work = b.func.dfg.num_insts();
        let mut maximum_entry = 0;
        let mut maximum_source = 0;
        let mut maximum_snapshot = 0;
        let mut all_bails = Vec::new();
        let mut all_checks = Vec::new();
        let mut all_header_snapshots = Vec::new();
        let mut peeled_probe_nodes = BTreeSet::new();
        let mut maximum_check = 0;
        for (width, variant) in variants {
            b.switch_to_block(variant);
            let entry_start = b.func.dfg.num_insts();
            let words = Words::new(&mut b, args[0], code, slots.clone())?;
            let packed = FrameState::registers(
                &mut b,
                args[1],
                closed.regions[id].packed.as_ref().unwrap().packing_domain(),
                controls.options(),
            )
            .with_constant_values([args[2], args[3]])
            .with_specialized_width(width)
            .with_lookup_mode(mode)
            .with_candidate_value_loads(&mut b);
            let first_index = words.read(&mut b, proof.bounds.index_word);
            // The bound certificate proves every selected range is precisely
            // [index+1, index+width], over a pinned one-byte-grapheme String.
            // A cursor names its final byte. Recover scalar indices only for
            // source actions/precise exits, rather than forming an address on
            // every adjacent load. Invalid/empty initial extents compare equal
            // and never dereference the wrapping preparation value.
            let source = packed.read(&mut b, core::mem::offset_of!(Frame, source_ptr));
            let width = packed.read(&mut b, core::mem::offset_of!(Frame, width));
            let bias = b.ins().iadd(source, width);
            let bias = b.ins().iadd_imm(bias, -1);
            let extent =
                PrepaidExtent::new(&mut b, credit_base, first_index, args[4], args[5], bias);
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
                    // Each incoming check established this exact equality.
                    // Canonicalize the source carrier to the native induction
                    // value rather than retaining equivalent loop phis.
                    let source_index = extent.index(&mut b);
                    words.write(&mut b, proof.bounds.index_word, source_index);
                    // Every incoming edge admits the original next traversal
                    // before this header. Its cold snapshot is outside the
                    // loop header, so prior cache/frame values need not be
                    // carried through the next productive iteration.
                    // Its prepaid debit is represented by extent.cursor and
                    // is reconstructed exactly before every observable exit.
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
                    emit_with_fact_guards(
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
                        let address = extent.pointer(&mut b);
                        packed_maps::select_leaf(&mut b, &packed, first, address);
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
                        let hit = b.create_block();
                        let miss = b.create_block();
                        b.set_cold_block(miss);
                        packed_maps::probe_leaf_present(
                            &mut b,
                            &packed,
                            fact,
                            proof.present[index],
                            bail[&index],
                            miss,
                        );
                        if fact == packed_maps::LookupFact::Fresh && !proof.present[index] {
                            peeled_probe_nodes.insert(index);
                        }
                        b.ins().jump(hit, &[]);
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
                    // The incoming-edge guard admitted a productive original
                    // traversal. A terminal traversal returns BEFORE its
                    // header, so the ordinary source CFG performs its exact
                    // debit, guards, state transition and terminal visit.
                    let tag_threading::Edge::Branch { no, .. } = node.edge else {
                        unreachable!()
                    };
                    b.ins().jump(target(no), &[]);
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
            let mut header_snapshots = Vec::new();
            for (&edge, &check) in &checks {
                b.switch_to_block(check);
                let start = b.func.dfg.num_insts();
                // Retire source progress after its original predecessor. The
                // entry keeps the original guards; completed recurrences use
                // their precomputed intersection of endpoint and prepaid fuel.
                // Each stopped edge remains before the next original debit,
                // including the final terminal traversal.
                // Each incoming source edge owns its cold snapshot. Joining
                // the initial and completed-iteration states in one exit
                // creates phi arguments that force cold constants and old
                // cache values to be materialized on every hot traversal.
                // Both outcomes still return before the SAME original header.
                let stopped = b.create_block();
                if edge.is_some() {
                    extent.advance(&mut b);
                    let counter = extent.index(&mut b);
                    // The original completed recurrence proved this equality.
                    // Its sole range selected the previous zero-based index;
                    // rematerialize that cold cache field from the NEW cursor
                    // so an old induction value need not cross the backedge.
                    words.write(&mut b, proof.bounds.index_word, counter);
                    let previous = b.ins().iadd_imm(counter, -1);
                    packed.write(
                        &mut b,
                        core::mem::offset_of!(Frame, previous_start),
                        previous,
                    );
                }
                let counter = words.read(&mut b, proof.bounds.index_word);
                packed.write(&mut b, core::mem::offset_of!(Frame, updates), counter);
                if edge.is_none() {
                    let funded = b.create_block();
                    let fuel = packed.read(&mut b, core::mem::offset_of!(Frame, fuel));
                    b.ins().brif(fuel, funded, &[], stopped, &[]);
                    b.switch_to_block(funded);
                }
                let pointer = extent.pointer(&mut b);
                let productive = b.ins().icmp(IntCC::UnsignedLessThan, pointer, extent.stop);
                b.ins()
                    .brif(productive, blocks[&proof.active], &[], stopped, &[]);
                maximum_check = maximum_check.max(b.func.dfg.num_insts() - start);
                all_checks.push((edge, check));
                header_snapshots.push((edge, stopped));
            }
            let mut maximum = 0;
            for (&index, &block) in &bail {
                b.switch_to_block(block);
                maximum = maximum.max(emit_snapshot(
                    &mut b,
                    &words,
                    &packed,
                    args[0],
                    &proof,
                    &extent,
                    true,
                    None,
                    index,
                    returned,
                    &mut snapshot_emissions,
                )?);
            }
            for &(edge, block) in &header_snapshots {
                b.switch_to_block(block);
                maximum = maximum.max(emit_snapshot(
                    &mut b,
                    &words,
                    &packed,
                    args[0],
                    &proof,
                    &extent,
                    false,
                    edge,
                    proof.active,
                    returned,
                    &mut snapshot_emissions,
                )?);
                all_bails.push((proof.active, block));
                all_header_snapshots.push((edge, block));
            }
            maximum_snapshot = maximum_snapshot.max(maximum);
            all_bails.extend(bail);
            candidate_value_loads += packed.candidate_value_loads();
            paired_candidate_loads += packed.paired_candidate_loads();
        }
        entry_work = dispatch_work + maximum_entry + maximum_check;
        source_work = maximum_source + maximum_check;
        snapshot_work = maximum_snapshot;
        bail_blocks = (
            all_bails,
            all_checks,
            maximum_check,
            all_header_snapshots,
            peeled_probe_nodes,
        );
        b.seal_all_blocks();
        b.finalize();
    }
    let initial_cleanup =
        super::super::cfg_cleanup::optimize(controls, &mut context, module.isa())?;
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
    let mut cleanup = super::super::cfg_cleanup::optimize(controls, &mut context, module.isa())?;
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
    let prepaid_stop_edges = fuel_check_edges
        .iter()
        .copied()
        .flatten()
        .collect::<Vec<_>>();
    let header_snapshot_edges = bail_blocks
        .3
        .iter()
        .filter_map(|&(edge, block)| context.func.layout.is_block_inserted(block).then_some(edge))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
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
    let cold_snapshot_recipes = snapshot_emissions
        .into_iter()
        .filter_map(|(block, recipe)| {
            context
                .func
                .layout
                .is_block_inserted(block)
                .then_some(((recipe.node, recipe.header, recipe.incoming), recipe))
        })
        .collect::<BTreeMap<_, _>>()
        .into_values()
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
            lookup_mode: match mode {
                packed_maps::LookupMode::Dynamic => "dynamic",
                packed_maps::LookupMode::Hash => "hash",
                packed_maps::LookupMode::Direct => "direct",
            },
            lookup_cache_function: None,
            lookup_cache_selection_word: None,
            candidate_value_loads,
            paired_candidate_loads,
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
            header_snapshot_edges,
            prepaid_stop_edges: prepaid_stop_edges.clone(),
            fuel_reconstructed_at_exits: true,
            prepaid_extent_arguments: 2,
            cursor_range_site: proof.bounds.range_site,
            cursor_index_word: proof.bounds.index_word,
            cursor_recurrence_edges: prepaid_stop_edges,
            cold_snapshot_recipes,
            peeled_probe_nodes: bail_blocks.4.into_iter().collect(),
            entry_stop_index_word: Some(proof.bounds.index_word),
            entry_stop_endpoint_words: Some([proof.bounds.width_word, proof.bounds.limit_word]),
            entry_instructions: entry_work,
            maximum_source_instructions: source_work,
            maximum_snapshot_instructions: snapshot_work,
            caller_instructions: 0,
            native_calls,
            stack_bytes: stack,
            code_bytes: bytes,
            cleanup,
        });
    if !direct {}
    Ok(Some(Definition {
        function,
        cached_function: None,
        cache_max_width: 0,
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
    let index = words.read(b, proof.bounds.index_word);
    let fuel = packed.read(b, core::mem::offset_of!(Frame, fuel));
    // An unproductive source index receives a zero extent. Wrapping
    // preparation has no externally visible effect; on productive entries
    // count <= endpoint-index, so the actual stop cannot overflow.
    let remaining = b.ins().isub(values[2], index);
    let smaller = b.ins().icmp(IntCC::UnsignedLessThan, fuel, remaining);
    let count = b.ins().select(smaller, fuel, remaining);
    let productive = b.ins().icmp(IntCC::UnsignedLessThan, index, values[2]);
    let zero = b.ins().iconst(types::I64, 0);
    let count = b.ins().select(productive, count, zero);
    let stop = b.ins().iadd(index, count);
    // The modular identity remains exact even if this preparation sum wraps.
    let credit = b.ins().iadd(fuel, index);
    // This certificate selects the final byte at source[index+width-1].
    // Productive cursors and their one-past stop remain inside the actual
    // pinned allocation; a zero extent has equal start/stop even if unused
    // integer preparation wraps. Relocate both opaque private arguments in
    // the caller so the backend cannot rebuild their bias inside the loop.
    let source = packed.read(b, core::mem::offset_of!(Frame, source_ptr));
    let width = packed.read(b, core::mem::offset_of!(Frame, width));
    let bias = b.ins().iadd(source, width);
    let bias = b.ins().iadd_imm(bias, -1);
    let stop = b.ins().iadd(stop, bias);
    let credit = b.ins().iadd(credit, bias);
    let arguments = [locals, pointer, values[0], values[1], stop, credit];
    let continuation = if let Some(cached) = call.cached_function {
        let cache = packed.read(b, core::mem::offset_of!(Frame, lookup_cache));
        let installed = b.ins().icmp_imm(IntCC::NotEqual, cache, 0);
        let width_index = b.ins().iadd_imm(width, -1);
        let supported = b.ins().icmp_imm(
            IntCC::UnsignedLessThan,
            width_index,
            call.definition.cache_max_width as i64,
        );
        let selected = b.ins().band(installed, supported);
        let direct = b.create_block();
        let hashed = b.create_block();
        let joined = b.create_block();
        b.append_block_param(joined, types::I64);
        b.ins().brif(selected, direct, &[], hashed, &[]);
        for (block, function) in [(direct, cached), (hashed, call.function)] {
            b.switch_to_block(block);
            let result = b.ins().call(function, &arguments);
            let value = b.inst_results(result)[0];
            b.ins().jump(joined, &[value.into()]);
        }
        b.switch_to_block(joined);
        b.block_params(joined)[0]
    } else {
        let result = b.ins().call(call.function, &arguments);
        b.inst_results(result)[0]
    };
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
