//! Version a proved closed packed Tail into its original CFG with SSA words.
//! Only proved tag branches are threaded; primitives, updates and errors keep
//! their source order, including the last terminal header traversal.
use super::super::iteration_compiler::{CodegenReport, RegionToken};
use super::super::{
    NativeCompileError,
    compiler::{backend, guard, offset, primitive, set_origin},
    iteration_code_plan::CodePlan,
    program_hooks::Hooks,
    program_packed_map::Frame,
    program_tail_layout::{Layout, Region},
};
use super::packed_maps::{self, FrameState};
#[cfg(test)]
pub(in crate::native_artifact) use super::packed_maps::{
    take_candidate_value_emissions, without_candidate_value_loads,
};
#[cfg(test)]
pub(in crate::native_artifact) use super::packed_maps::{
    take_frame_effect_emissions, without_frame_effects,
};
#[cfg(test)]
pub(in crate::native_artifact) use super::packed_maps::{
    take_paired_candidate_emissions, without_paired_candidate_loads,
};
use super::ssa_words::Words;
use cranelift_codegen::ir::{
    Block, InstBuilder, MemFlags, Value,
    condcodes::{FloatCC, IntCC},
    types,
};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::{
    NativeAggregateWord as Word, NativeCollectionPrimitive as P, NativeIterationAction as Action,
    NativeIterationTerminator as Term, NativeMapOwnerWords, NativeMatchAction as Scalar,
    NativeMatchTerminator as Branch, NativeMatchValue, NativeScalarAggregateAction as Aggregate,
    NativeScalarAggregates, NativeScalarKind,
};
use std::collections::{BTreeMap, BTreeSet};
#[path = "program_loop_count_bounds.rs"]
mod count_bounds;
#[path = "program_scalar_edge_facts.rs"]
mod edge_facts;
#[path = "program_loop_invariants.rs"]
mod invariants;
#[path = "program_lookup_facts.rs"]
mod lookup_facts;
#[path = "program_loop_bounds.rs"]
mod loop_bounds;
#[path = "program_loop_constants.rs"]
mod loop_constants;
#[path = "program_loop_mutation_facts.rs"]
mod mutation_facts;
#[path = "program_packed_loop_functions.rs"]
mod outlined;
#[path = "program_steady_loop.rs"]
mod steady;
#[path = "program_tag_threading.rs"]
pub(super) mod tag_threading;
#[cfg(test)]
pub(in crate::native_artifact) use count_bounds::{
    take_count_bound_emissions, without_count_bounds,
};
pub(in crate::native_artifact) use invariants::enabled as invariant_loops_enabled;
#[cfg(test)]
pub(in crate::native_artifact) use invariants::{
    take_invariant_emissions, without_loop_invariants,
};
#[cfg(test)]
pub(in crate::native_artifact) use loop_constants::{
    take_constant_emissions, without_loop_constants,
};
#[cfg(test)]
pub(in crate::native_artifact) use mutation_facts::{
    take_mutation_emissions, without_mutation_facts,
};
pub(in crate::native_artifact) use outlined::define_functions;
#[cfg(test)]
pub(in crate::native_artifact) use outlined::{take_outline_emissions, without_loop_outlining};
#[cfg(test)]
pub(in crate::native_artifact) use steady::without_lookup_loop_split;
#[cfg(test)]
pub(in crate::native_artifact) use steady::{take_cold_recipe_emissions, without_cold_recipes};
#[cfg(test)]
pub(in crate::native_artifact) use steady::{take_steady_emissions, without_steady_loops};

// The bridge's extra fixed template is admitted only for the same bounded
// graph and narrow helper effects that fit the unchanged source-site lane.
// Unsupported or deliberately disabled proofs retain ordinary native calls.
fn preserving_calls_proved(
    code: CodePlan<'_>,
    closed: &Layout,
    id: usize,
    owner: &NativeMapOwnerWords,
    aggregates: &NativeScalarAggregates,
    slots: &BTreeSet<usize>,
) -> bool {
    if !packed_maps::effects_enabled() || !invariants::enabled() {
        return false;
    }
    #[cfg(test)]
    if TAG_THREADING_DISABLED.with(std::cell::Cell::get)
        || LOOKUP_FACTS_DISABLED.with(std::cell::Cell::get)
        || LOOP_BOUNDS_DISABLED.with(std::cell::Cell::get)
    {
        return false;
    }
    let Some(graph) = tag_threading::derive(code, closed, id, owner, aggregates) else {
        return false;
    };
    let Some(bounds) = loop_bounds::derive(code, closed, id, owner, aggregates, &graph, slots)
    else {
        return false;
    };
    if invariants::derive(code, closed, id, owner, &graph, &bounds, slots).is_none() {
        return false;
    }
    let Some(facts) = lookup_facts::derive(code, closed, id, owner, aggregates, &graph) else {
        return false;
    };
    graph.nodes.iter().zip(facts).all(|(node, fact)| {
        let Action::LibraryCall { template, .. } = code.blocks()[node.site].action() else {
            return true;
        };
        match template.primitive() {
            P::MapKeyAccess => fact == packed_maps::LookupFact::Present,
            P::MapInsert => matches!(
                fact,
                packed_maps::LookupFact::Cached
                    | packed_maps::LookupFact::Present
                    | packed_maps::LookupFact::Absent
            ),
            _ => true,
        }
    })
}

#[cfg(test)]
thread_local! {
    static EDGE_FACT_EMISSIONS: std::cell::Cell<(usize, usize)> = const { std::cell::Cell::new((0, 0)) };
    static EDGE_FACTS_DISABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static TAG_THREADING_DISABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static TAG_THREAD_EMISSIONS: std::cell::Cell<(usize, usize, usize)> = const { std::cell::Cell::new((0, 0, 0)) };
    static LOOP_BOUNDS_DISABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static LOOP_BOUNDS_EMISSIONS: std::cell::Cell<(usize, usize)> = const { std::cell::Cell::new((0, 0)) };
    static LOOKUP_FACTS_DISABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static LOOKUP_FACT_EMISSIONS: std::cell::Cell<(usize, usize)> = const { std::cell::Cell::new((0, 0)) };
}
#[cfg(test)]
pub(in crate::native_artifact) fn without_lookup_facts<T>(f: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            LOOKUP_FACTS_DISABLED.with(|v| v.set(self.0));
        }
    }
    let _restore = Restore(LOOKUP_FACTS_DISABLED.with(|v| v.replace(true)));
    f()
}
#[cfg(test)]
pub(in crate::native_artifact) fn take_lookup_fact_emissions() -> (usize, usize) {
    LOOKUP_FACT_EMISSIONS.with(|v| v.replace((0, 0)))
}
#[cfg(test)]
pub(in crate::native_artifact) fn without_loop_bounds<T>(f: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            LOOP_BOUNDS_DISABLED.with(|v| v.set(self.0));
        }
    }
    let _restore = Restore(LOOP_BOUNDS_DISABLED.with(|v| v.replace(true)));
    f()
}
#[cfg(test)]
pub(in crate::native_artifact) fn take_loop_bounds_emissions() -> (usize, usize) {
    LOOP_BOUNDS_EMISSIONS.with(|v| v.replace((0, 0)))
}
#[cfg(test)]
pub(in crate::native_artifact) fn take_edge_fact_emissions() -> (usize, usize) {
    EDGE_FACT_EMISSIONS.with(|counts| counts.replace((0, 0)))
}
#[cfg(test)]
pub(in crate::native_artifact) fn without_edge_facts<T>(f: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            EDGE_FACTS_DISABLED.with(|disabled| disabled.set(self.0));
        }
    }
    let _restore = Restore(EDGE_FACTS_DISABLED.with(|disabled| disabled.replace(true)));
    f()
}
#[cfg(test)]
pub(in crate::native_artifact) fn without_tag_threading<T>(f: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            TAG_THREADING_DISABLED.with(|disabled| disabled.set(self.0));
        }
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
        counts.set((
            guards + usize::from(!arithmetic),
            operations + usize::from(arithmetic),
        ));
    });
    #[cfg(not(test))]
    let _ = arithmetic;
}

fn field(layout: &NativeScalarAggregates, value: NativeMatchValue, index: usize) -> Option<usize> {
    let (base, count) = layout.slot(value)?;
    (index < count).then(|| base + index)
}
pub(super) fn aggregate_words(
    layout: &NativeScalarAggregates,
    action: &Aggregate,
    words: &mut BTreeSet<usize>,
) -> Option<()> {
    let output = match action {
        Aggregate::Pack {
            output,
            words: inputs,
        } => {
            for input in inputs {
                match input {
                    Word::Scalar(value) => {
                        words.insert(value.get() as usize);
                    }
                    Word::Field(value, index) => {
                        words.insert(field(layout, *value, *index)?);
                    }
                    Word::Constant(_) => {}
                }
            }
            *output
        }
        Aggregate::Copy { source, output } => {
            let (_, count) = layout.slot(*output)?;
            for index in 0..count {
                words.insert(field(layout, *source, index)?);
            }
            *output
        }
        Aggregate::Project {
            source,
            offset,
            guards,
            output,
        } => {
            for &(index, _) in guards {
                words.insert(field(layout, *source, index)?);
            }
            let (_, count) = layout.slot(*output)?;
            for index in 0..count {
                words.insert(field(layout, *source, offset.checked_add(index)?)?);
            }
            *output
        }
        Aggregate::Observe {
            source,
            offset,
            guards,
            output,
            ..
        } => {
            for &(index, _) in guards {
                words.insert(field(layout, *source, index)?);
            }
            words.insert(field(layout, *source, *offset)?);
            words.insert(output.get() as usize);
            return Some(());
        }
    };
    let (base, count) = layout.slot(output)?;
    words.extend(base..base + count);
    Some(())
}
fn aggregate_at<'a>(
    owner: &'a NativeMapOwnerWords,
    ordinary: &'a NativeScalarAggregates,
    owner_index: usize,
    site: usize,
) -> Option<(&'a NativeScalarAggregates, &'a Aggregate)> {
    if owner.action_region(site) == Some(owner_index) {
        owner
            .aggregates()
            .action(site)
            .map(|action| (owner.aggregates(), action))
    } else {
        ordinary.action(site).map(|action| (ordinary, action))
    }
}

/// Additional emission proof: every action in the already closed, single-entry
/// Tail has a complete word interpretation. Unsupported actions keep the old
/// generated CFG. The existing owner proof supplies dominance, lifetime and
/// single-backedge invariants; no spelling of a source function is recognized.
fn words_for(
    code: CodePlan<'_>,
    closed: &Layout,
    id: usize,
    owner: &NativeMapOwnerWords,
    aggregates: &NativeScalarAggregates,
) -> Option<BTreeSet<usize>> {
    let region = &closed.regions[id];
    let owner_index = region.map?;
    region.packed.as_ref()?;
    let mut words = BTreeSet::new();
    for (site, block) in code.blocks().iter().enumerate() {
        if closed.sites[site] != Some(id) {
            continue;
        }
        if let Some((layout, action)) = aggregate_at(owner, aggregates, owner_index, site) {
            aggregate_words(layout, action, &mut words)?;
        } else if let Some(span) = owner.span_site(site).map(|i| &owner.spans()[i]) {
            if span.region() != owner_index {
                return None;
            }
            words.extend(span.operands().iter().map(|v| v.get() as usize));
        } else if owner.call_region(site) == Some(owner_index) {
            let Action::LibraryCall {
                template,
                operands,
                output,
            } = block.action()
            else {
                return None;
            };
            if !matches!(
                template.primitive(),
                P::MapContainsKey | P::MapKeyAccess | P::MapInsert
            ) {
                return None;
            }
            words.insert(operands[0].get() as usize);
            if template.primitive() == P::MapInsert {
                words.insert(operands[2].get() as usize);
            }
            words.insert(output.get() as usize);
        } else {
            match block.action() {
                Action::Scalar(Scalar::None) => {}
                Action::Scalar(Scalar::Primitive {
                    operands, output, ..
                }) => {
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
            Term::Scalar(Branch::Branch { condition, .. }) => {
                words.insert(condition.get() as usize);
            }
            Term::Scalar(Branch::LiteralBranch { left, right, .. }) => {
                code.kind(left)?;
                words.insert(left.get() as usize);
                words.insert(right.get() as usize);
            }
            _ => return None,
        }
    }
    // Current-scope validation allows only this value to escape a Tail.
    // Its owner representation is the single map-token word.
    words
        .contains(&(region.tail.returned().get() as usize))
        .then_some(words)
}

pub(super) fn emit_scalar_aggregate(
    b: &mut FunctionBuilder<'_>,
    state: &Words,
    layout: &NativeScalarAggregates,
    action: &Aggregate,
    site: usize,
    error: Block,
) -> Result<(), NativeCompileError> {
    emit_aggregate_body(
        b,
        state,
        layout,
        action,
        site,
        &edge_facts::Proof::default(),
        error,
        None,
    )
}

fn emit_aggregate(
    b: &mut FunctionBuilder<'_>,
    state: &Words,
    layout: &NativeScalarAggregates,
    action: &Aggregate,
    site: usize,
    facts: &edge_facts::Proof,
    error: Block,
    report: &mut CodegenReport,
    report_region: Option<RegionToken>,
) -> Result<(), NativeCompileError> {
    emit_aggregate_body(
        b,
        state,
        layout,
        action,
        site,
        facts,
        error,
        Some((report, report_region)),
    )
}
fn emit_aggregate_body(
    b: &mut FunctionBuilder<'_>,
    state: &Words,
    layout: &NativeScalarAggregates,
    action: &Aggregate,
    site: usize,
    facts: &edge_facts::Proof,
    error: Block,
    mut evidence: Option<(&mut CodegenReport, Option<RegionToken>)>,
) -> Result<(), NativeCompileError> {
    let at = |value, index| {
        field(layout, value, index).ok_or_else(|| backend("packed SSA aggregate field"))
    };
    let (output, values) = match action {
        Aggregate::Pack { output, words } => {
            let values = words
                .iter()
                .map(|word| {
                    Ok(match word {
                        Word::Scalar(source) => state.read(b, source.get() as usize),
                        Word::Field(source, index) => state.read(b, at(*source, *index)?),
                        Word::Constant(word) => b.ins().iconst(types::I64, *word as i64),
                    })
                })
                .collect::<Result<Vec<_>, NativeCompileError>>()?;
            (*output, values)
        }
        Aggregate::Copy { source, output } => {
            let count = layout.slot(*output).unwrap().1;
            // Materialize every source before changing an overlapping carrier.
            let values = (0..count)
                .map(|i| Ok(state.read(b, at(*source, i)?)))
                .collect::<Result<Vec<_>, NativeCompileError>>()?;
            (*output, values)
        }
        Aggregate::Project {
            source,
            offset,
            guards,
            output,
        } => {
            for &(index, ordinal) in guards {
                if facts.guard(site, *source, index, ordinal) {
                    if let Some((report, region)) = evidence.as_mut() {
                        note_edge_fact(false);
                        report.guard(*region, site, *source, index, ordinal);
                    }
                    continue;
                }
                let tag = state.read(b, at(*source, index)?);
                let valid = b.ins().icmp_imm(IntCC::Equal, tag, i64::from(ordinal));
                guard(b, valid, error, super::super::abi::STATUS_ABI);
            }
            let count = layout.slot(*output).unwrap().1;
            let values = (0..count)
                .map(|i| Ok(state.read(b, at(*source, offset + i)?)))
                .collect::<Result<Vec<_>, NativeCompileError>>()?;
            (*output, values)
        }
        Aggregate::Observe {
            source,
            offset,
            ordinal,
            guards,
            output,
        } => {
            for &(index, ordinal) in guards {
                if facts.guard(site, *source, index, ordinal) {
                    if let Some((report, region)) = evidence.as_mut() {
                        note_edge_fact(false);
                        report.guard(*region, site, *source, index, ordinal);
                    }
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
            } else {
                value
            };
            state.write(b, output.get() as usize, value);
            return Ok(());
        }
    };
    for (index, value) in values.into_iter().enumerate() {
        state.write(b, at(output, index)?, value);
    }
    Ok(())
}

fn spill(
    b: &mut FunctionBuilder<'_>,
    packed: &FrameState,
    locals: Value,
    region: &Region,
) -> Result<(), NativeCompileError> {
    packed.spill(b);
    let fuel = packed.read(b, core::mem::offset_of!(Frame, fuel));
    b.ins()
        .store(MemFlags::new(), fuel, locals, offset(region.words + 1)?);
    Ok(())
}

/// Called at the ordinary header before its per-visit dispatch. The fast
/// version returns only at the original exit/error boundary; declined runtime
/// entry continues emitting and executing the untouched ordinary header.
pub(in crate::native_artifact) fn emit_entry(
    b: &mut FunctionBuilder<'_>,
    frame: Value,
    locals: Value,
    user_context: Value,
    code: CodePlan<'_>,
    closed: &Layout,
    owner: Option<&NativeMapOwnerWords>,
    aggregates: &NativeScalarAggregates,
    site: usize,
    hooks: &Hooks,
    ordinary_blocks: &[Option<Block>],
    error: Block,
    outlined: Option<cranelift_codegen::ir::FuncRef>,
    report: &mut CodegenReport,
) -> Result<(), NativeCompileError> {
    let Some(id) = closed.sites.get(site).copied().flatten() else {
        return Ok(());
    };
    let region = &closed.regions[id];
    if region.tail.header() as usize != site {
        return Ok(());
    }
    let Some(owner) = owner else {
        return Ok(());
    };
    let Some(slots) = eligible_slots(code, closed, id, owner, aggregates) else {
        return Ok(());
    };
    let entry_template_start = b.func.dfg.num_insts();
    let ordinary = b.create_block();
    let inspect = b.create_block();
    let selected = b.create_block();
    let ready = b
        .ins()
        .load(types::I64, MemFlags::new(), locals, offset(region.words)?);
    b.ins().brif(ready, inspect, &[], ordinary, &[]);
    b.switch_to_block(inspect);
    let context = b.ins().load(
        types::I64,
        MemFlags::new(),
        locals,
        offset(region.words + 4)?,
    );
    let pointer = b.ins().load(types::I64, MemFlags::new(), context, 0);
    b.ins().brif(pointer, selected, &[], ordinary, &[]);
    b.switch_to_block(selected);
    set_origin(b, frame, site as u64);
    let function = b.ins().load(
        types::I64,
        MemFlags::new(),
        locals,
        offset(region.words + 3)?,
    );
    let zero = b.ins().iconst(types::I64, 0);
    // One actual-selection receipt per versioned region, never per iteration.
    hooks.bound_call(b, function, context, site, 13, [zero; 3], error);
    if let Some(function_ref) = outlined {
        let endpoint = loop_constants::enabled()
            .then(|| loop_constants::endpoint_words(code, closed, id, owner, aggregates, &slots))
            .flatten();
        let mut arguments = vec![frame, locals, pointer, function, context, user_context];
        if loop_constants::enabled() {
            arguments.push(loop_constants::bind(b, locals, code, endpoint)?);
        }
        let call = b.ins().call(function_ref, &arguments);
        let status = b.inst_results(call)[0];
        let ok = b.ins().icmp_imm(IntCC::Equal, status, 0);
        b.ins().brif(
            ok,
            ordinary_blocks[region.tail.exit() as usize].unwrap(),
            &[],
            error,
            &[status.into()],
        );
        let caller_work = b.func.dfg.num_insts() - entry_template_start;
        if loop_constants::enabled() {
            report.loop_constant_caller(site, loop_constants::BYTES, endpoint, caller_work);
        }
        if caller_work > outlined::CALLER_TEMPLATE_BOUND {
            return Err(backend(
                "outlined packed loop caller exceeds fixed template",
            ));
        }
    } else {
        let prefix_work = b.func.dfg.num_insts() - entry_template_start;
        emit_body(
            b,
            frame,
            locals,
            pointer,
            function,
            context,
            code,
            closed,
            id,
            owner,
            aggregates,
            slots,
            hooks,
            ordinary_blocks[region.tail.exit() as usize].unwrap(),
            error,
            prefix_work,
            None,
            None,
            report,
        )?;
    }
    b.switch_to_block(ordinary);
    Ok(())
}

fn eligible_slots(
    code: CodePlan<'_>,
    closed: &Layout,
    id: usize,
    owner: &NativeMapOwnerWords,
    aggregates: &NativeScalarAggregates,
) -> Option<BTreeSet<usize>> {
    let slots = words_for(code, closed, id, owner, aggregates)?;
    (slots
        .len()
        .saturating_add(core::mem::size_of::<Frame>() / 8)
        <= 512)
        .then_some(slots)
}

// The same proved body is emitted inline or into an owned native function.
// Only its two exit edges differ; source actions and finite loans are shared.
fn emit_body(
    b: &mut FunctionBuilder<'_>,
    frame: Value,
    locals: Value,
    pointer: Value,
    function: Value,
    context: Value,
    code: CodePlan<'_>,
    closed: &Layout,
    id: usize,
    owner: &NativeMapOwnerWords,
    aggregates: &NativeScalarAggregates,
    slots: BTreeSet<usize>,
    hooks: &Hooks,
    source_exit: Block,
    error: Block,
    entry_prefix_work: usize,
    constant_packet: Option<Value>,
    steady: Option<steady::Call<'_>>,
    report: &mut CodegenReport,
) -> Result<(usize, usize), NativeCompileError> {
    let region = &closed.regions[id];
    let site = region.tail.header() as usize;
    let zero = b.ins().iconst(types::I64, 0);
    let entry_template_start = b.func.dfg.num_insts() - 1;
    let entry_shim_work = hooks.bound_shim_work();
    let facts = edge_facts::derive(code, closed, id, owner, aggregates, &slots).unwrap_or_default();
    let threaded = tag_threading::derive(code, closed, id, owner, aggregates);
    #[cfg(test)]
    let threaded = if TAG_THREADING_DISABLED.with(std::cell::Cell::get) {
        None
    } else {
        threaded
    };
    let lookups = threaded
        .as_ref()
        .and_then(|graph| lookup_facts::derive(code, closed, id, owner, aggregates, graph));
    #[cfg(test)]
    let lookups = if LOOKUP_FACTS_DISABLED.with(std::cell::Cell::get) {
        None
    } else {
        lookups
    };
    let bounds = threaded
        .as_ref()
        .and_then(|graph| loop_bounds::derive(code, closed, id, owner, aggregates, graph, &slots));
    #[cfg(test)]
    let bounds = if LOOP_BOUNDS_DISABLED.with(std::cell::Cell::get) {
        None
    } else {
        bounds
    };
    #[cfg(test)]
    let facts = if EDGE_FACTS_DISABLED.with(std::cell::Cell::get) {
        edge_facts::Proof::default()
    } else {
        facts
    };
    let report_region = report.packed_ssa_region(site);
    let invariant = bounds
        .as_ref()
        .zip(threaded.as_ref())
        .and_then(|(bounds, graph)| {
            invariants::derive(code, closed, id, owner, graph, bounds, &slots)
        });
    let invariant = if invariant.is_some()
        && report_region.is_some()
        && report.reserve_invariant_region(threaded.as_ref().unwrap().nodes.len() * 4)
    {
        invariant
    } else {
        None
    };
    let mutation = bounds
        .as_ref()
        .filter(|proof| proof.single_insert_nodes.is_some() && mutation_facts::enabled())
        .filter(|_| report.reserve_mutation_region(threaded.as_ref().unwrap().nodes.len() * 8));
    let count_values = mutation
        .filter(|proof| proof.count_values.is_some() && count_bounds::enabled())
        .filter(|_| report.reserve_count_value_region(threaded.as_ref().unwrap().nodes.len() * 5));
    let steady = steady.filter(|_| {
        invariant.is_some()
            && mutation.is_some()
            && count_values.is_some()
            && constant_packet.is_some()
    });
    let steady_block = steady.as_ref().map(|_| b.create_block());

    let owner_index = region.map.unwrap();
    let setup = b.func.dfg.num_insts();
    let words = Words::new(b, locals, code, slots)?;
    let packed =
        FrameState::registers(b, pointer, region.packed.as_ref().unwrap().packing_domain())
            .with_constant_packet(constant_packet);
    let endpoint_loads = usize::from(constant_packet.is_some() && bounds.is_some());
    let entry_work = (b.func.dfg.num_insts() - setup) as u64;
    // The existing closed-body envelope includes 1024 fixed units per
    // traversal. Keep the original setup bound; check all fixed templates below.
    if entry_work > 512 {
        return Err(backend("packed SSA entry exceeds closed setup envelope"));
    }
    let exit = b.create_block();
    let failed = b.create_block();
    b.append_block_param(failed, types::I64);
    b.set_cold_block(failed);
    let source_sites = closed
        .sites
        .iter()
        .enumerate()
        .filter_map(|(index, candidate)| (*candidate == Some(id)).then_some(index))
        .collect::<Vec<_>>();
    let mut emitted_sites = threaded.as_ref().map_or_else(
        || source_sites.clone(),
        |graph| graph.nodes.iter().map(|node| node.site).collect(),
    );
    let version_nodes = emitted_sites.len();
    let modes = if invariant.is_some() {
        4
    } else if bounds.is_some() {
        2
    } else {
        1
    };
    for _ in 1..modes {
        emitted_sites.extend_from_within(..version_nodes);
    }
    let blocks = emitted_sites
        .iter()
        .map(|_| b.create_block())
        .collect::<Vec<_>>();
    let original_target = |next: u32| {
        if next == region.tail.exit() {
            exit
        } else {
            blocks[source_sites.binary_search(&(next as usize)).unwrap()]
        }
    };
    let graph_target = |target: tag_threading::Target, mode: usize| match target {
        tag_threading::Target::Node(index) => blocks[index + mode * version_nodes],
        tag_threading::Target::Exit => exit,
    };
    let calls_allowed = invariant
        .as_ref()
        .map(|_| invariants::calls_allowed(b, &packed));
    let trip_end = if let Some(proof) = &bounds {
        let width = words.read(b, proof.width_word);
        let limit = words.read(b, proof.limit_word);
        let length = packed.read(b, core::mem::offset_of!(Frame, source_len));
        let nonzero = b.ins().icmp_imm(IntCC::UnsignedGreaterThan, width, 0);
        let fits = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, width, limit);
        let actual = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, limit, length);
        let valid = b.ins().band(nonzero, fits);
        let mut valid = b.ins().band(valid, actual);
        if mutation.is_some() {
            // Source index and private update tally must describe the same
            // completed prefix before this relation can cross any header.
            let index = words.read(b, proof.index_word);
            let updates = packed.read(b, core::mem::offset_of!(Frame, updates));
            let aligned = b.ins().icmp(IntCC::Equal, index, updates);
            // Other variants cannot enter this active recurrence according to
            // the certificate. Their unrelated tuple fields need no equality.
            let tag = words.read(b, threaded.as_ref().unwrap().carrier_word);
            let inactive = b
                .ins()
                .icmp_imm(IntCC::NotEqual, tag, i64::from(proof.active_tag));
            let aligned = b.ins().bor(aligned, inactive);
            valid = b.ins().band(valid, aligned);
        }
        // Wrapping arithmetic has no source effect on the declined branch.
        // In the selected branch, 1<=width<=limit proves this exact endpoint.
        let end = if let Some(packet) = constant_packet {
            b.ins()
                .load(types::I64, MemFlags::new().with_readonly(), packet, 16)
        } else {
            let end = b.ins().isub(limit, width);
            b.ins().iadd_imm(end, 1)
        };
        let selected_bounds = b.create_block();
        b.ins().brif(valid, selected_bounds, &[], blocks[0], &[]);
        b.switch_to_block(selected_bounds);
        // This receipt runs exactly once, only after the actual selector has
        // succeeded and before any original header debit or source action.
        let call_guard = calls_allowed
            .map(|v| b.ins().uextend(types::I64, v))
            .unwrap_or(zero);
        hooks.bound_call(
            b,
            function,
            context,
            site,
            14,
            [width, limit, call_guard],
            error,
        );
        if let Some(allowed) = calls_allowed {
            b.ins().brif(
                allowed,
                blocks[2 * version_nodes],
                &[],
                blocks[version_nodes],
                &[],
            );
        } else {
            b.ins().jump(blocks[version_nodes], &[]);
        }
        Some(end)
    } else {
        b.ins().jump(
            if threaded.is_some() {
                blocks[0]
            } else {
                original_target(site as u32)
            },
            &[],
        );
        None
    };
    let extra_entry_work = hooks.bound_shim_work() - entry_shim_work;
    let entry_template_work =
        entry_prefix_work + b.func.dfg.num_insts() - entry_template_start + extra_entry_work;
    let mut maximum_source_template_work = 0;
    let mut dispatch_template_work = 0;
    let mut folded_branches = Vec::new();
    let mut known_latches = Vec::new();
    let mut header_versions = 0;
    let mut certified_ranges = Vec::new();
    let mut lookup_substitutions = Vec::new();
    let mut call_substitutions = Vec::new();
    let mut rolling_substitutions = Vec::new();
    let mut steady_latches = Vec::new();
    let mut refreshes = Vec::new();
    let mut refresh_template_work = 0;
    let mut counter_headers = Vec::new();
    let mut counter_checks_removed = Vec::new();
    let mut nonempty_probes = Vec::new();
    let mut count_overflows_removed = Vec::new();

    for (version, &index) in emitted_sites.iter().enumerate() {
        let source_template_start = b.func.dfg.num_insts();
        let source_shim_work = hooks.bound_shim_work();
        let block = &code.blocks()[index];
        let current = blocks[version];
        let mode = version / version_nodes;
        let guarded = mode != 0;
        let calls_proved = mode >= 2;
        let mut refresh = None;
        let graph_node = version % version_nodes;
        let threaded_node = threaded.as_ref().map(|graph| &graph.nodes[graph_node]);
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
            if guarded
                && let Some(proof) = mutation
                && threaded_node.and_then(|node| node.header_tag) == Some(proof.active_tag)
            {
                // Entry equality and exactly one successful insertion per
                // index+1 recurrence prove this exact value, even before a
                // refill/error spill. Every physical map update remains.
                let index_value = words.read(b, proof.index_word);
                packed.write(b, core::mem::offset_of!(Frame, updates), index_value);
                counter_headers.push((version, index));
            }
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
            emit_aggregate(
                b,
                &words,
                layout,
                action,
                index,
                &facts,
                fail_at,
                report,
                report_region,
            )?;
        } else if let Some(span) = owner.span_site(index).map(|i| &owner.spans()[i]) {
            if span.range_site() as usize == index {
                let first = words.read(b, span.operands()[0].get() as usize);
                let last = words.read(b, span.operands()[1].get() as usize);
                let certified = guarded
                    && bounds
                        .as_ref()
                        .is_some_and(|proof| proof.ranges.contains(&graph_node));
                if certified {
                    certified_ranges.push(version);
                }
                let adjacent = mode == 3 && certified;
                if calls_proved {
                    call_substitutions.push((version, index, 10));
                }
                if adjacent {
                    rolling_substitutions.push((version, index));
                }
                packed_maps::select_state_proved(
                    b,
                    &packed,
                    first,
                    last,
                    hooks,
                    function,
                    context,
                    index,
                    fail_at,
                    packed_maps::SelectionProof {
                        bounds: certified,
                        calls: calls_proved,
                        adjacent,
                    },
                );
            }
        } else if owner.call_region(index) == Some(owner_index) {
            let Action::LibraryCall {
                template,
                operands,
                output,
            } = block.action()
            else {
                unreachable!()
            };
            let operation = match template.primitive() {
                P::MapContainsKey => 3,
                P::MapKeyAccess => 4,
                P::MapInsert => 5,
                _ => unreachable!(),
            };
            let replacement = if operation == 5 {
                words.read(b, operands[2].get() as usize)
            } else {
                zero
            };
            let token = words.read(b, operands[0].get() as usize);
            let lookup = lookups
                .as_ref()
                .map_or(packed_maps::LookupFact::Unknown, |facts| facts[graph_node]);
            let refreshed = if invariant.is_some() && guarded && operation == 5 {
                let block = b.create_block();
                b.append_block_param(block, types::I64);
                b.set_cold_block(block);
                refresh = Some((block, *output));
                Some(block)
            } else {
                None
            };
            let value = packed_maps::operation_state_proved(
                b,
                &packed,
                operation,
                replacement,
                token,
                hooks,
                function,
                context,
                index,
                fail_at,
                lookup,
                calls_proved,
                guarded
                    && mutation.is_some_and(|proof| {
                        proof
                            .single_insert_nodes
                            .as_ref()
                            .unwrap()
                            .contains(&graph_node)
                    }),
                mode == 3 && mutation.is_some(),
                refreshed,
            );
            if operation == 5
                && guarded
                && mutation.is_some_and(|proof| {
                    proof
                        .single_insert_nodes
                        .as_ref()
                        .unwrap()
                        .contains(&graph_node)
                })
            {
                counter_checks_removed.push((version, index));
            }
            if mode == 3
                && mutation.is_some()
                && matches!(
                    lookup,
                    packed_maps::LookupFact::Fresh | packed_maps::LookupFact::Unknown
                )
            {
                nonempty_probes.push((version, index));
            }
            if calls_proved {
                call_substitutions.push((version, index, operation));
            }
            if lookup != packed_maps::LookupFact::Unknown {
                lookup_substitutions.push((version, index, operation, lookup.name()));
            }
            words.write(b, output.get() as usize, value);
        } else {
            match block.action() {
                Action::Scalar(Scalar::None) => {}
                Action::Scalar(Scalar::Primitive {
                    op,
                    operands,
                    output,
                    ..
                }) => {
                    let args = operands
                        .iter()
                        .map(|v| words.read(b, v.get() as usize))
                        .collect::<Vec<_>>();
                    let value = if guarded
                        && *op == mech_engine::native_artifact::NativeScalarOp::U64Add
                        && count_values.is_some_and(|proof| {
                            proof
                                .count_values
                                .as_ref()
                                .unwrap()
                                .additions
                                .contains(&graph_node)
                        }) {
                        // Empty native seed + one bounded insertion per
                        // index+1 recurrence imply every stored value <= index.
                        // This source read+1 is <= next_index at a productive
                        // index. Unknown replacement rules keep checked adds.
                        count_overflows_removed.push((version, index));
                        b.ins().iadd(args[0], args[1])
                    } else if facts.arithmetic(index, *op, operands) {
                        use mech_engine::native_artifact::NativeScalarOp as Op;
                        note_edge_fact(true);
                        let value = match op {
                            Op::U64Add => b.ins().iadd(args[0], args[1]),
                            Op::U64Sub => b.ins().isub(args[0], args[1]),
                            _ => unreachable!("edge proof certifies only unsigned add/subtract"),
                        };
                        report.arithmetic(report_region, index, *op, [operands[0], operands[1]]);
                        value
                    } else {
                        primitive(b, *op, &args, fail_at)
                    };
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
            if node.folded_branch {
                folded_branches.push(index);
            }
            if node.known_latch {
                known_latches.push(index);
            }
            if guarded
                && bounds
                    .as_ref()
                    .is_some_and(|proof| proof.first == graph_node)
            {
                let proof = bounds.as_ref().unwrap();
                let index = words.read(b, proof.index_word);
                let stop =
                    b.ins()
                        .icmp(IntCC::UnsignedGreaterThanOrEqual, index, trip_end.unwrap());
                let tag_threading::Edge::Branch { yes, no } = node.edge else {
                    unreachable!()
                };
                b.ins().brif(
                    stop,
                    graph_target(yes, mode),
                    &[],
                    graph_target(no, mode),
                    &[],
                );
            } else if guarded
                && bounds
                    .as_ref()
                    .is_some_and(|proof| proof.later.contains(&graph_node))
            {
                let tag_threading::Edge::Branch { no, .. } = node.edge else {
                    unreachable!()
                };
                b.ins().jump(graph_target(no, mode), &[]);
            } else {
                match &node.edge {
                    tag_threading::Edge::Jump(target) => {
                        let next_mode = if calls_proved
                            && invariant
                                .as_ref()
                                .is_some_and(|p| p.steady_latches.contains(&graph_node))
                        {
                            steady_latches.push((version, index));
                            3
                        } else {
                            mode
                        };
                        let steady_target = steady.as_ref().filter(|call| next_mode == 3
                        && matches!(*target,tag_threading::Target::Node(node) if node==call.definition.active_node()));
                        b.ins().jump(
                            if steady_target.is_some() {
                                steady_block.unwrap()
                            } else {
                                graph_target(*target, next_mode)
                            },
                            &[],
                        );
                    }
                    tag_threading::Edge::Dispatch { cases, other } => {
                        // Dispatch follows the original header debit. Each arm
                        // enters a body version, so no traversal is charged twice.
                        let value = words.read(b, threaded.as_ref().unwrap().carrier_word);
                        let before = b.func.dfg.num_insts();
                        for &(ordinal, target) in cases {
                            let next = b.create_block();
                            let equal = b.ins().icmp_imm(IntCC::Equal, value, i64::from(ordinal));
                            b.ins()
                                .brif(equal, graph_target(target, mode), &[], next, &[]);
                            b.switch_to_block(next);
                        }
                        b.ins().jump(graph_target(*other, mode), &[]);
                        // At most eight comparisons/branches plus a jump. Count
                        // this on EVERY traversal, including unknown backedges.
                        let work = b.func.dfg.num_insts() - before;
                        if work > 32 {
                            return Err(backend(
                                "tag dispatcher exceeds existing per-visit envelope",
                            ));
                        }
                        dispatch_template_work = dispatch_template_work.max(work);
                    }
                    tag_threading::Edge::Branch { .. } | tag_threading::Edge::Reject => {
                        emit_source_terminator(
                            b,
                            &words,
                            code,
                            block.terminator(),
                            fail_at,
                            |next| {
                                let tag_threading::Edge::Branch { yes, no } = &node.edge else {
                                    unreachable!()
                                };
                                let source_yes = match block.terminator() {
                                    Term::Scalar(
                                        Branch::Branch { yes, .. }
                                        | Branch::LiteralBranch { yes, .. },
                                    ) => yes,
                                    _ => unreachable!(),
                                };
                                graph_target(if next == source_yes { *yes } else { *no }, mode)
                            },
                        );
                    }
                }
            }
        } else {
            emit_source_terminator(
                b,
                &words,
                code,
                block.terminator(),
                fail_at,
                original_target,
            );
        }
        if let Some((refreshed, output)) = refresh {
            b.switch_to_block(refreshed);
            let before = b.func.dfg.num_insts();
            let value = b.block_params(refreshed)[0];
            words.write(b, output.get() as usize, value);
            let allowed = invariants::calls_allowed(b, &packed);
            let tag_threading::Edge::Jump(target) = threaded_node.unwrap().edge else {
                unreachable!("proved insertion successor");
            };
            // Refresh to this operation's SUCCESSOR, never to the header:
            // no source action or funded traversal can be repeated or skipped.
            b.ins().brif(
                allowed,
                graph_target(target, if mode == 3 { 3 } else { 2 }),
                &[],
                graph_target(target, 1),
                &[],
            );
            refresh_template_work += b.func.dfg.num_insts() - before;
            refreshes.push((version, index));
        }
        b.switch_to_block(fail_at);
        let status = b.block_params(fail_at)[0];
        set_origin(b, frame, index as u64);
        b.ins().jump(failed, &[status.into()]);
        let work = b.func.dfg.num_insts() - source_template_start + hooks.bound_shim_work()
            - source_shim_work;
        let steady_source_work = if mode == 3 {
            steady
                .as_ref()
                .map_or(0, |call| call.definition.source_work)
        } else {
            0
        };
        // A steady continuation resumes ONLY version three. Checked/initial
        // modes cannot follow that native call and retain their old envelope.
        if hooks.bound_shim_work() > source_shim_work || (steady.is_some() && mode == 3) {
            if work + steady_source_work > super::super::program_tail_layout::OVERHEAD as usize {
                return Err(backend(format!(
                    "packed source template exceeds unchanged envelope: site={index}, mode={mode}, ordinary={work}, steady={steady_source_work}"
                )));
            }
            maximum_source_template_work = maximum_source_template_work.max(work);
        }
    }
    if let Some(graph) = &threaded {
        #[cfg(test)]
        TAG_THREAD_EMISSIONS.with(|counts| {
            let previous = counts.get();
            counts.set((
                previous.0 + emitted_sites.len(),
                previous.1 + folded_branches.len(),
                previous.2 + known_latches.len(),
            ));
        });
        report.tag_threading(
            site,
            graph.carrier_word,
            graph.variants.clone(),
            source_sites.len(),
            emitted_sites.len(),
            header_versions,
            folded_branches,
            known_latches,
        );
    }
    let extra_steady_work = if let Some(call) = &steady {
        b.switch_to_block(steady_block.unwrap());
        let caller_work = steady::emit_call(
            b,
            call,
            &words,
            &packed,
            locals,
            pointer,
            constant_packet.unwrap(),
            &blocks,
            version_nodes,
            exit,
            failed,
            region.tail.returned().get() as usize,
        )?;
        if let Some(record) = report
            .steady_loops
            .iter_mut()
            .find(|record| record.header == site)
        {
            record.caller_instructions = caller_work;
        }
        caller_work + call.definition.extra_fixed_work
    } else {
        0
    };
    let boundary_start = b.func.dfg.num_insts();
    b.switch_to_block(exit);
    // Tail validation confines source locals/carriers to this invocation and
    // permits only `returned` at its parent-scope exit. Do not make dead source
    // temporaries loop-carried merely to spill them into abandoned slots.
    let returned = region.tail.returned().get() as usize;
    let token = words.read(b, returned);
    b.ins()
        .store(MemFlags::new(), token, locals, offset(returned)?);
    spill(b, &packed, locals, region)?;
    b.ins().jump(source_exit, &[]);
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
    let fixed_template_work = entry_template_work
        + dispatch_template_work
        + boundary_work
        + refresh_template_work
        + extra_steady_work;
    if fixed_template_work > super::super::program_tail_layout::FIXED_OVERHEAD as usize {
        return Err(backend(
            "packed SSA templates exceed fixed per-traversal envelope",
        ));
    }
    if invariant.is_some() {
        invariants::note(call_substitutions.len(), rolling_substitutions.len());
        report
            .loop_invariants
            .push(super::super::iteration_compiler::InvariantLoop {
                header: site,
                version_stride: version_nodes,
                emitted_nodes: emitted_sites.len(),
                call_substitutions,
                rolling_substitutions,
                steady_latches,
                refreshes,
                fixed_template_work,
            });
    }
    if let Some(proof) = mutation {
        mutation_facts::note(
            counter_headers.len(),
            counter_checks_removed.len(),
            nonempty_probes.len(),
        );
        report
            .mutation_loops
            .push(super::super::iteration_compiler::MutationLoop {
                header: site,
                index_word: proof.index_word,
                active_tag: proof.active_tag,
                version_stride: version_nodes,
                emitted_nodes: emitted_sites.len(),
                alignment_guard: true,
                source_insert_nodes: proof
                    .single_insert_nodes
                    .as_ref()
                    .unwrap()
                    .iter()
                    .map(|&node| (node, threaded.as_ref().unwrap().nodes[node].site))
                    .collect(),
                counter_headers,
                counter_checks_removed,
                nonempty_probes,
            });
    }
    if let Some(proof) = count_values {
        let values = proof.count_values.as_ref().unwrap();
        let graph = threaded.as_ref().unwrap();
        count_bounds::note(count_overflows_removed.len());
        report
            .count_value_loops
            .push(super::super::iteration_compiler::CountValueLoop {
                header: site,
                index_word: proof.index_word,
                active_tag: proof.active_tag,
                version_stride: version_nodes,
                emitted_nodes: emitted_sites.len(),
                empty_seed_checked: true,
                source_read_nodes: values
                    .reads
                    .iter()
                    .map(|&node| (node, graph.nodes[node].site))
                    .collect(),
                source_add_nodes: values
                    .additions
                    .iter()
                    .map(|&node| (node, graph.nodes[node].site))
                    .collect(),
                overflow_checks_removed: count_overflows_removed,
            });
    }
    report.frame_effects(site, packed.effect_emissions());
    if hooks.bound_shim_calls() != 0 {
        report.preserving_call_regions.push(
            super::super::iteration_compiler::PreservingCallRegion {
                header: site,
                emitted_calls: hooks.bound_shim_calls(),
                maximum_source_template_work,
                extra_entry_work,
            },
        );
    }
    #[cfg(test)]
    LOOKUP_FACT_EMISSIONS.with(|counts| {
        let old = counts.get();
        counts.set((
            old.0 + usize::from(lookups.is_some()),
            old.1 + lookup_substitutions.len(),
        ));
    });
    report.lookup_facts(
        site,
        version_nodes,
        emitted_sites.len(),
        lookup_substitutions,
    );
    if let Some(proof) = &bounds {
        #[cfg(test)]
        LOOP_BOUNDS_EMISSIONS.with(|counts| {
            let old = counts.get();
            counts.set((old.0 + 1, old.1 + certified_ranges.len()));
        });
        report.guarded_loop(
            site,
            proof.index_word,
            proof.width_word,
            proof.limit_word,
            proof.active_tag,
            proof.guard_sites,
            proof.range_site,
            proof.recurrence_sites.clone(),
            emitted_sites.len(),
            certified_ranges,
            fixed_template_work,
        );
    }
    Ok((packed.constant_hash_loads(), endpoint_loads))
}

pub(super) fn emit_source_terminator(
    b: &mut FunctionBuilder<'_>,
    words: &Words,
    code: CodePlan<'_>,
    terminator: Term,
    fail_at: Block,
    target: impl Fn(u32) -> Block,
) {
    match terminator {
        Term::TailHeader { body, .. } | Term::Scalar(Branch::Jump(body)) => {
            b.ins().jump(target(body), &[]);
        }
        Term::Scalar(Branch::Branch { condition, yes, no }) => {
            let value = words.read(b, condition.get() as usize);
            b.ins().brif(value, target(yes), &[], target(no), &[]);
        }
        Term::Scalar(Branch::LiteralBranch {
            left,
            right,
            yes,
            no,
        }) => {
            let kind = code.kind(left).unwrap();
            let left = words.read(b, left.get() as usize);
            let right = words.read(b, right.get() as usize);
            let equal = if kind == NativeScalarKind::F64 {
                let left = b.ins().bitcast(types::F64, MemFlags::new(), left);
                let right = b.ins().bitcast(types::F64, MemFlags::new(), right);
                b.ins().fcmp(FloatCC::Equal, left, right)
            } else {
                b.ins().icmp(IntCC::Equal, left, right)
            };
            b.ins().brif(equal, target(yes), &[], target(no), &[]);
        }
        Term::Scalar(Branch::Reject) => {
            // Exhaustive source matches retain their fail-closed default
            // arm. Preserve its origin and unwind the owned frame/fuel.
            let status = b
                .ins()
                .iconst(types::I64, super::super::control_abi::STATUS_REJECT as i64);
            b.ins().jump(fail_at, &[status.into()]);
        }
        _ => unreachable!("checked packed SSA terminator"),
    }
}
