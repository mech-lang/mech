//! Version a proved closed packed Tail into its original CFG with SSA words.
//! Only proved tag branches are threaded; primitives, updates and errors keep
//! their source order, including the last terminal header traversal.
use super::super::{NativeCompileError, compiler::{backend, guard, offset, primitive, set_origin},
    iteration_code_plan::CodePlan, program_hooks::Hooks, program_packed_map::Frame,
    program_tail_layout::{Layout, Region}};
use super::packed_maps::{self, FrameState};
#[cfg(test)]
pub(in crate::native_artifact) use super::packed_maps::{take_frame_effect_emissions, without_frame_effects};
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, types, condcodes::{FloatCC, IntCC}};
use cranelift_frontend::{FunctionBuilder, Variable};
use mech_engine::native_artifact::{NativeAggregateWord as Word, NativeIterationAction as Action,
    NativeIterationTerminator as Term, NativeMapOwnerWords, NativeMatchAction as Scalar,
    NativeMatchTerminator as Branch, NativeMatchValue, NativeScalarAggregateAction as Aggregate,
    NativeScalarAggregates, NativeScalarKind, NativeCollectionPrimitive as P};
use std::collections::{BTreeMap, BTreeSet};
use super::super::iteration_compiler::{CodegenReport, RegionToken};
#[path = "program_scalar_edge_facts.rs"]
mod edge_facts;
#[path = "program_tag_threading.rs"]
mod tag_threading;

#[cfg(test)]
thread_local! {
    static EDGE_FACT_EMISSIONS: std::cell::Cell<(usize, usize)> = const { std::cell::Cell::new((0, 0)) };
    static EDGE_FACTS_DISABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static TAG_THREADING_DISABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static TAG_THREAD_EMISSIONS: std::cell::Cell<(usize, usize, usize)> = const { std::cell::Cell::new((0, 0, 0)) };
}
#[cfg(test)]
pub(in crate::native_artifact) fn take_edge_fact_emissions() -> (usize, usize) {
    EDGE_FACT_EMISSIONS.with(|counts| counts.replace((0, 0)))
}
#[cfg(test)]
pub(in crate::native_artifact) fn without_edge_facts<T>(f: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) { EDGE_FACTS_DISABLED.with(|disabled| disabled.set(self.0)); }
    }
    let _restore = Restore(EDGE_FACTS_DISABLED.with(|disabled| disabled.replace(true)));
    f()
}
#[cfg(test)]
pub(in crate::native_artifact) fn without_tag_threading<T>(f: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) { TAG_THREADING_DISABLED.with(|disabled| disabled.set(self.0)); }
    }
    let _restore = Restore(TAG_THREADING_DISABLED.with(|disabled| disabled.replace(true)));
    f()
}
#[cfg(test)]
pub(in crate::native_artifact) fn take_tag_thread_emissions() -> (usize, usize, usize) {
    TAG_THREAD_EMISSIONS.with(|counts| counts.replace((0, 0, 0)))
}
fn note_edge_fact(arithmetic: bool) {
    #[cfg(test)]
    EDGE_FACT_EMISSIONS.with(|counts| {
        let (guards, operations) = counts.get();
        counts.set((guards + usize::from(!arithmetic), operations + usize::from(arithmetic)));
    });
    #[cfg(not(test))]
    let _ = arithmetic;
}

fn field(layout: &NativeScalarAggregates, value: NativeMatchValue, index: usize) -> Option<usize> {
    let (base, count) = layout.slot(value)?;
    (index < count).then(|| base + index)
}
fn aggregate_words(layout: &NativeScalarAggregates, action: &Aggregate,
    words: &mut BTreeSet<usize>) -> Option<()>
{
    let output = match action {
        Aggregate::Pack { output, words: inputs } => {
            for input in inputs {
                match input {
                    Word::Scalar(value) => { words.insert(value.get() as usize); }
                    Word::Field(value, index) => { words.insert(field(layout, *value, *index)?); }
                    Word::Constant(_) => {}
                }
            }
            *output
        }
        Aggregate::Copy { source, output } => {
            let (_, count) = layout.slot(*output)?;
            for index in 0..count { words.insert(field(layout, *source, index)?); }
            *output
        }
        Aggregate::Project { source, offset, guards, output } => {
            for &(index, _) in guards { words.insert(field(layout, *source, index)?); }
            let (_, count) = layout.slot(*output)?;
            for index in 0..count { words.insert(field(layout, *source, offset.checked_add(index)?)?); }
            *output
        }
        Aggregate::Observe { source, offset, guards, output, .. } => {
            for &(index, _) in guards { words.insert(field(layout, *source, index)?); }
            words.insert(field(layout, *source, *offset)?);
            words.insert(output.get() as usize);
            return Some(());
        }
    };
    let (base, count) = layout.slot(output)?;
    words.extend(base..base + count);
    Some(())
}
fn aggregate_at<'a>(owner: &'a NativeMapOwnerWords, ordinary: &'a NativeScalarAggregates,
    owner_index: usize, site: usize) -> Option<(&'a NativeScalarAggregates, &'a Aggregate)>
{
    if owner.action_region(site) == Some(owner_index) {
        owner.aggregates().action(site).map(|action| (owner.aggregates(), action))
    } else {
        ordinary.action(site).map(|action| (ordinary, action))
    }
}

/// Additional emission proof: every action in the already closed, single-entry
/// Tail has a complete word interpretation. Unsupported actions keep the old
/// generated CFG. The existing owner proof supplies dominance, lifetime and
/// single-backedge invariants; no spelling of a source function is recognized.
fn words_for(code: CodePlan<'_>, closed: &Layout, id: usize,
    owner: &NativeMapOwnerWords, aggregates: &NativeScalarAggregates) -> Option<BTreeSet<usize>>
{
    let region = &closed.regions[id];
    let owner_index = region.map?;
    region.packed.as_ref()?;
    let mut words = BTreeSet::new();
    for (site, block) in code.blocks().iter().enumerate() {
        if closed.sites[site] != Some(id) { continue; }
        if let Some((layout, action)) = aggregate_at(owner, aggregates, owner_index, site) {
            aggregate_words(layout, action, &mut words)?;
        } else if let Some(span) = owner.span_site(site).map(|i| &owner.spans()[i]) {
            if span.region() != owner_index { return None; }
            words.extend(span.operands().iter().map(|v| v.get() as usize));
        } else if owner.call_region(site) == Some(owner_index) {
            let Action::LibraryCall { template, operands, output } = block.action() else { return None; };
            if !matches!(template.primitive(), P::MapContainsKey | P::MapKeyAccess | P::MapInsert) { return None; }
            words.insert(operands[0].get() as usize);
            if template.primitive() == P::MapInsert { words.insert(operands[2].get() as usize); }
            words.insert(output.get() as usize);
        } else {
            match block.action() {
                Action::Scalar(Scalar::None) => {}
                Action::Scalar(Scalar::Primitive { operands, output, .. }) => {
                    words.extend(operands.iter().map(|v| v.get() as usize));
                    words.insert(output.get() as usize);
                }
                Action::Scalar(Scalar::Copy { source, output }) if code.kind(*output).is_some() => {
                    words.insert(source.get() as usize);
                    words.insert(output.get() as usize);
                }
                _ => return None,
            }
        }
        match block.terminator() {
            Term::TailHeader { .. } | Term::Scalar(Branch::Jump(_) | Branch::Reject) => {}
            Term::Scalar(Branch::Branch { condition, .. }) => { words.insert(condition.get() as usize); }
            Term::Scalar(Branch::LiteralBranch { left, right, .. }) => {
                code.kind(left)?;
                words.insert(left.get() as usize); words.insert(right.get() as usize);
            }
            _ => return None,
        }
    }
    // Current-scope validation allows only this value to escape a Tail.
    // Its owner representation is the single map-token word.
    words.contains(&(region.tail.returned().get() as usize)).then_some(words)
}

struct Words { variables: BTreeMap<usize, Variable> }
impl Words {
    fn new(b: &mut FunctionBuilder<'_>, locals: Value, code: CodePlan<'_>, slots: BTreeSet<usize>)
        -> Result<Self, NativeCompileError>
    {
        let constants = (0..code.constant_count()).map(|index| code.constant(index))
            .collect::<Result<Vec<_>, _>>()?.into_iter()
            .filter_map(|(value, word)| word.map(|word| (value.get() as usize, word)))
            .collect::<BTreeMap<_, _>>();
        let mut variables = BTreeMap::new();
        for slot in slots {
            let variable = b.declare_var(types::I64);
            let value = if let Some(&word) = constants.get(&slot) {
                b.ins().iconst(types::I64, word as i64)
            } else { b.ins().load(types::I64, MemFlags::new(), locals, offset(slot)?) };
            b.def_var(variable, value);
            variables.insert(slot, variable);
        }
        Ok(Self { variables })
    }
    fn read(&self, b: &mut FunctionBuilder<'_>, slot: usize) -> Value { b.use_var(self.variables[&slot]) }
    fn write(&self, b: &mut FunctionBuilder<'_>, slot: usize, value: Value) { b.def_var(self.variables[&slot], value); }
}

fn emit_aggregate(b: &mut FunctionBuilder<'_>, state: &Words, layout: &NativeScalarAggregates,
    action: &Aggregate, site: usize, facts: &edge_facts::Proof, error: Block,
    report: &mut CodegenReport, report_region: Option<RegionToken>) -> Result<(), NativeCompileError>
{
    let at = |value, index| field(layout, value, index).ok_or_else(|| backend("packed SSA aggregate field"));
    let (output, values) = match action {
        Aggregate::Pack { output, words } => {
            let values = words.iter().map(|word| Ok(match word {
                Word::Scalar(source) => state.read(b, source.get() as usize),
                Word::Field(source, index) => state.read(b, at(*source, *index)?),
                Word::Constant(word) => b.ins().iconst(types::I64, *word as i64),
            })).collect::<Result<Vec<_>, NativeCompileError>>()?;
            (*output, values)
        }
        Aggregate::Copy { source, output } => {
            let count = layout.slot(*output).unwrap().1;
            // Materialize every source before changing an overlapping carrier.
            let values = (0..count).map(|i| Ok(state.read(b, at(*source, i)?)))
                .collect::<Result<Vec<_>, NativeCompileError>>()?;
            (*output, values)
        }
        Aggregate::Project { source, offset, guards, output } => {
            for &(index, ordinal) in guards {
                if facts.guard(site, *source, index, ordinal) {
                    note_edge_fact(false);
                    report.guard(report_region, site, *source, index, ordinal);
                    continue;
                }
                let tag = state.read(b, at(*source, index)?);
                let valid = b.ins().icmp_imm(IntCC::Equal, tag, i64::from(ordinal));
                guard(b, valid, error, super::super::abi::STATUS_ABI);
            }
            let count = layout.slot(*output).unwrap().1;
            let values = (0..count).map(|i| Ok(state.read(b, at(*source, offset + i)?)))
                .collect::<Result<Vec<_>, NativeCompileError>>()?;
            (*output, values)
        }
        Aggregate::Observe { source, offset, ordinal, guards, output } => {
            for &(index, ordinal) in guards {
                if facts.guard(site, *source, index, ordinal) {
                    note_edge_fact(false);
                    report.guard(report_region, site, *source, index, ordinal);
                    continue;
                }
                let tag = state.read(b, at(*source, index)?);
                let valid = b.ins().icmp_imm(IntCC::Equal, tag, i64::from(ordinal));
                guard(b, valid, error, super::super::abi::STATUS_ABI);
            }
            let value = state.read(b, at(*source, *offset)?);
            let value = if let Some(ordinal) = ordinal {
                let equal = b.ins().icmp_imm(IntCC::Equal, value, i64::from(*ordinal));
                b.ins().uextend(types::I64, equal)
            } else { value };
            state.write(b, output.get() as usize, value);
            return Ok(());
        }
    };
    for (index, value) in values.into_iter().enumerate() { state.write(b, at(output, index)?, value); }
    Ok(())
}

fn spill(b: &mut FunctionBuilder<'_>, packed: &FrameState,
    locals: Value, region: &Region) -> Result<(), NativeCompileError>
{
    packed.spill(b);
    let fuel = packed.read(b, core::mem::offset_of!(Frame, fuel));
    b.ins().store(MemFlags::new(), fuel, locals, offset(region.words + 1)?);
    Ok(())
}

/// Called at the ordinary header before its per-visit dispatch. The fast
/// version returns only at the original exit/error boundary; declined runtime
/// entry continues emitting and executing the untouched ordinary header.
pub(in crate::native_artifact) fn emit_entry(b: &mut FunctionBuilder<'_>, frame: Value, locals: Value,
    code: CodePlan<'_>, closed: &Layout, owner: Option<&NativeMapOwnerWords>,
    aggregates: &NativeScalarAggregates, site: usize, hooks: &Hooks,
    ordinary_blocks: &[Option<Block>], error: Block, report: &mut CodegenReport) -> Result<(), NativeCompileError>
{
    let Some(id) = closed.sites.get(site).copied().flatten() else { return Ok(()); };
    let region = &closed.regions[id];
    if region.tail.header() as usize != site { return Ok(()); }
    let Some(owner) = owner else { return Ok(()); };
    let Some(slots) = words_for(code, closed, id, owner, aggregates) else { return Ok(()); };
    if slots.len().saturating_add(core::mem::size_of::<Frame>() / 8) > 512 {
        return Ok(());
    }
    let facts = edge_facts::derive(code, closed, id, owner, aggregates, &slots).unwrap_or_default();
    let threaded = tag_threading::derive(code, closed, id, owner, aggregates);
    #[cfg(test)]
    let threaded = if TAG_THREADING_DISABLED.with(std::cell::Cell::get) { None } else { threaded };
    #[cfg(test)]
    let facts = if EDGE_FACTS_DISABLED.with(std::cell::Cell::get) { edge_facts::Proof::default() } else { facts };
    let report_region = report.packed_ssa_region(site);
    let owner_index = region.map.unwrap();
    let entry_template_start = b.func.dfg.num_insts();
    let ordinary = b.create_block();
    let inspect = b.create_block();
    let selected = b.create_block();
    let ready = b.ins().load(types::I64, MemFlags::new(), locals, offset(region.words)?);
    b.ins().brif(ready, inspect, &[], ordinary, &[]);
    b.switch_to_block(inspect);
    let context = b.ins().load(types::I64, MemFlags::new(), locals, offset(region.words + 4)?);
    let pointer = b.ins().load(types::I64, MemFlags::new(), context, 0);
    b.ins().brif(pointer, selected, &[], ordinary, &[]);
    b.switch_to_block(selected);
    set_origin(b, frame, site as u64);
    let function = b.ins().load(types::I64, MemFlags::new(), locals, offset(region.words + 3)?);
    let zero = b.ins().iconst(types::I64, 0);
    // One actual-selection receipt per versioned region, never per iteration.
    hooks.bound_call(b, function, context, site, 13, [zero; 3], error);
    let setup = b.func.dfg.num_insts();
    let words = Words::new(b, locals, code, slots)?;
    let packed = FrameState::registers(b, pointer, region.packed.as_ref().unwrap().packing_domain());
    let entry_work = (b.func.dfg.num_insts() - setup) as u64;
    // The existing closed-body envelope includes 1024 fixed units per
    // traversal. Keep the original setup bound; check all fixed templates below.
    if entry_work > 512 { return Err(backend("packed SSA entry exceeds closed setup envelope")); }
    let exit = b.create_block();
    let failed = b.create_block();
    b.append_block_param(failed, types::I64);
    b.set_cold_block(failed);
    let source_sites = closed.sites.iter().enumerate().filter_map(|(index, candidate)|
        (*candidate == Some(id)).then_some(index)).collect::<Vec<_>>();
    let emitted_sites = threaded.as_ref().map_or_else(|| source_sites.clone(),
        |graph| graph.nodes.iter().map(|node| node.site).collect());
    let blocks = emitted_sites.iter().map(|_| b.create_block()).collect::<Vec<_>>();
    let original_target = |next: u32| if next == region.tail.exit() { exit } else {
        blocks[source_sites.binary_search(&(next as usize)).unwrap()]
    };
    let graph_target = |target: tag_threading::Target| match target {
        tag_threading::Target::Node(index) => blocks[index], tag_threading::Target::Exit => exit,
    };
    b.ins().jump(if threaded.is_some() { blocks[0] } else { original_target(site as u32) }, &[]);
    let entry_template_work = b.func.dfg.num_insts() - entry_template_start;
    let mut dispatch_template_work = 0;
    let mut folded_branches = Vec::new();
    let mut known_latches = Vec::new();
    let mut header_versions = 0;
    for (version, &index) in emitted_sites.iter().enumerate() {
        let block = &code.blocks()[index];
        let current = blocks[version];
        let threaded_node = threaded.as_ref().map(|graph| &graph.nodes[version]);
        // The entry frame observes a source origin only when control leaves
        // this private CFG. Each helper already receives its precise site.
        // Preserve that origin on every failing edge, without writing it for
        // successful empty blocks, projections, arithmetic or map operations.
        let fail_at = b.create_block();
        b.append_block_param(fail_at, types::I64);
        b.set_cold_block(fail_at);
        b.switch_to_block(current);
        if index == site {
            header_versions += 1;
            let fuel = packed.read(b, core::mem::offset_of!(Frame, fuel));
            let replenish = b.create_block();
            let funded = b.create_block();
            b.ins().brif(fuel, funded, &[], replenish, &[]);
            b.switch_to_block(replenish);
            packed.call(b, hooks, function, context, index, 0, [zero; 3], fail_at);
            let count = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
            packed.write(b, core::mem::offset_of!(Frame, fuel), count);
            b.ins().jump(funded, &[]);
            b.switch_to_block(funded);
            let fuel = packed.read(b, core::mem::offset_of!(Frame, fuel));
            let fuel = b.ins().iadd_imm(fuel, -1);
            packed.write(b, core::mem::offset_of!(Frame, fuel), fuel);
            if let Some(tag) = threaded_node.and_then(|node| node.header_tag) {
                // Proven by the actual incoming dispatcher or completed
                // source latch copy, never by an assumed initializer.
                let value = b.ins().iconst(types::I64, i64::from(tag));
                words.write(b, threaded.as_ref().unwrap().carrier_word, value);
            }
        }
        if let Some((layout, action)) = aggregate_at(owner, aggregates, owner_index, index) {
            emit_aggregate(b, &words, layout, action, index, &facts, fail_at, report, report_region)?;
        } else if let Some(span) = owner.span_site(index).map(|i| &owner.spans()[i]) {
            if span.range_site() as usize == index {
                let first = words.read(b, span.operands()[0].get() as usize);
                let last = words.read(b, span.operands()[1].get() as usize);
                packed_maps::select_state(b, &packed, first, last, hooks, function, context, index, fail_at);
            }
        } else if owner.call_region(index) == Some(owner_index) {
            let Action::LibraryCall { template, operands, output } = block.action() else { unreachable!() };
            let operation = match template.primitive() { P::MapContainsKey => 3, P::MapKeyAccess => 4, P::MapInsert => 5, _ => unreachable!() };
            let replacement = if operation == 5 { words.read(b, operands[2].get() as usize) } else { zero };
            let token = words.read(b, operands[0].get() as usize);
            let value = packed_maps::operation_state(b, &packed, operation, replacement,
                token, hooks, function, context, index, fail_at);
            words.write(b, output.get() as usize, value);
        } else {
            match block.action() {
                Action::Scalar(Scalar::None) => {}
                Action::Scalar(Scalar::Primitive { op, operands, output, .. }) => {
                    let args = operands.iter().map(|v| words.read(b, v.get() as usize)).collect::<Vec<_>>();
                    let value = if facts.arithmetic(index, *op, operands) {
                        use mech_engine::native_artifact::NativeScalarOp as Op;
                        note_edge_fact(true);
                        let value = match op {
                            Op::U64Add => b.ins().iadd(args[0], args[1]),
                            Op::U64Sub => b.ins().isub(args[0], args[1]),
                            _ => unreachable!("edge proof certifies only unsigned add/subtract"),
                        };
                        report.arithmetic(report_region, index, *op, [operands[0], operands[1]]);
                        value
                    } else { primitive(b, *op, &args, fail_at) };
                    words.write(b, output.get() as usize, value);
                }
                Action::Scalar(Scalar::Copy { source, output }) => {
                    let value = words.read(b, source.get() as usize);
                    words.write(b, output.get() as usize, value);
                }
                _ => unreachable!("checked packed SSA action"),
            }
        }
        if let Some(node) = threaded_node {
            if node.folded_branch { folded_branches.push(index); }
            if node.known_latch { known_latches.push(index); }
            match &node.edge {
                tag_threading::Edge::Jump(target) => { b.ins().jump(graph_target(*target), &[]); }
                tag_threading::Edge::Dispatch { cases, other } => {
                    // Dispatch follows the original header debit. Each arm
                    // enters a body version, so no traversal is charged twice.
                    let value = words.read(b, threaded.as_ref().unwrap().carrier_word);
                    let before = b.func.dfg.num_insts();
                    for &(ordinal, target) in cases {
                        let next = b.create_block();
                        let equal = b.ins().icmp_imm(IntCC::Equal, value, i64::from(ordinal));
                        b.ins().brif(equal, graph_target(target), &[], next, &[]);
                        b.switch_to_block(next);
                    }
                    b.ins().jump(graph_target(*other), &[]);
                    // At most eight comparisons/branches plus a jump. Count
                    // this on EVERY traversal, including unknown backedges.
                    let work = b.func.dfg.num_insts() - before;
                    if work > 32 {
                        return Err(backend("tag dispatcher exceeds existing per-visit envelope"));
                    }
                    dispatch_template_work = dispatch_template_work.max(work);
                }
                tag_threading::Edge::Branch { .. } | tag_threading::Edge::Reject => {
                    emit_source_terminator(b, &words, code, block.terminator(), fail_at, |next| {
                        let tag_threading::Edge::Branch { yes, no } = &node.edge else { unreachable!() };
                        let source_yes = match block.terminator() {
                            Term::Scalar(Branch::Branch { yes, .. } | Branch::LiteralBranch { yes, .. }) => yes,
                            _ => unreachable!(),
                        };
                        graph_target(if next == source_yes { *yes } else { *no })
                    });
                }
            }
        } else {
            emit_source_terminator(b, &words, code, block.terminator(), fail_at, original_target);
        }
        b.switch_to_block(fail_at);
        let status = b.block_params(fail_at)[0];
        set_origin(b, frame, index as u64);
        b.ins().jump(failed, &[status.into()]);
    }
    if let Some(graph) = &threaded {
        #[cfg(test)]
        TAG_THREAD_EMISSIONS.with(|counts| {
            let previous = counts.get();
            counts.set((previous.0 + graph.nodes.len(), previous.1 + folded_branches.len(), previous.2 + known_latches.len()));
        });
        report.tag_threading(site, graph.carrier_word, graph.variants.clone(), source_sites.len(),
            graph.nodes.len(), header_versions, folded_branches, known_latches);
    }
    let boundary_start = b.func.dfg.num_insts();
    b.switch_to_block(exit);
    // Tail validation confines source locals/carriers to this invocation and
    // permits only `returned` at its parent-scope exit. Do not make dead source
    // temporaries loop-carried merely to spill them into abandoned slots.
    let returned = region.tail.returned().get() as usize;
    let token = words.read(b, returned);
    b.ins().store(MemFlags::new(), token, locals, offset(returned)?);
    spill(b, &packed, locals, region)?;
    b.ins().jump(ordinary_blocks[region.tail.exit() as usize].unwrap(), &[]);
    b.switch_to_block(failed);
    let status = b.block_params(failed)[0];
    // Abort consumes the packed frame/fuel and original error site, never
    // source scalar slots. Synchronize ownership without extending their lives.
    spill(b, &packed, locals, region)?;
    b.ins().jump(error, &[status.into()]);
    let boundary_work = b.func.dfg.num_insts() - boundary_start;
    // Layout.compute includes this fixed lane on every original traversal,
    // in addition to visit.work + OVERHEAD for every original source site.
    // Even the first traversal pays enough for the entire one-time setup,
    // a repeated unknown-tag dispatcher, AND both mutually exclusive exits.
    // Known-tag header rematerialization is one instruction in that original
    // header's unchanged OVERHEAD. No charge is removed or renewed here.
    if entry_template_work + dispatch_template_work + boundary_work
        > super::super::program_tail_layout::FIXED_OVERHEAD as usize
    {
        return Err(backend("packed SSA templates exceed fixed per-traversal envelope"));
    }
    report.frame_effects(site, packed.effect_emissions());
    b.switch_to_block(ordinary);
    Ok(())
}

fn emit_source_terminator(b: &mut FunctionBuilder<'_>, words: &Words, code: CodePlan<'_>,
    terminator: Term, fail_at: Block, target: impl Fn(u32) -> Block)
{
    match terminator {
        Term::TailHeader { body, .. } | Term::Scalar(Branch::Jump(body)) => { b.ins().jump(target(body), &[]); }
        Term::Scalar(Branch::Branch { condition, yes, no }) => {
            let value = words.read(b, condition.get() as usize);
            b.ins().brif(value, target(yes), &[], target(no), &[]);
        }
        Term::Scalar(Branch::LiteralBranch { left, right, yes, no }) => {
            let kind = code.kind(left).unwrap();
            let left = words.read(b, left.get() as usize);
            let right = words.read(b, right.get() as usize);
            let equal = if kind == NativeScalarKind::F64 {
                let left = b.ins().bitcast(types::F64, MemFlags::new(), left);
                let right = b.ins().bitcast(types::F64, MemFlags::new(), right);
                b.ins().fcmp(FloatCC::Equal, left, right)
            } else { b.ins().icmp(IntCC::Equal, left, right) };
            b.ins().brif(equal, target(yes), &[], target(no), &[]);
        }
        Term::Scalar(Branch::Reject) => {
            // Exhaustive source matches retain their fail-closed default
            // arm. Preserve its origin and unwind the owned frame/fuel.
            let status = b.ins().iconst(types::I64, super::super::control_abi::STATUS_REJECT as i64);
            b.ins().jump(fail_at, &[status.into()]);
        }
        _ => unreachable!("checked packed SSA terminator"),
    }
}
