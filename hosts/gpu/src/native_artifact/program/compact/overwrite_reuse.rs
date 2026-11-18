//! Reuse a completed, allocation-free overwrite on a subsequent tail visit.
//! This is a compact-only proof. The managed metering contract is independent.
use super::{
    arrays::{self, Arrays},
    flow::Flow,
    guards::Guards,
    layout::{ARRAY_OWNER, Layout, LayoutKind},
    owner_flow::OwnerFlow,
    recipes::Recipe,
};
use crate::native_artifact::{
    NativeCompileError,
    codegen::common::ssa_words::Words,
    compact_runtime::{Counters, Heap},
    compiler::backend,
};
use cranelift_codegen::ir::{
    AbiParam, Block, FuncRef, InstBuilder, MemFlags, Value, condcodes::IntCC, types,
};
use cranelift_frontend::{FunctionBuilder, Variable};
use cranelift_module::{Linkage, Module};
use mech_engine::native_artifact::{
    NativeAggregateWord as Word, NativeCollectionPrimitive as P, NativeControlLoop as Loop,
    NativeIterationAction as Action, NativeIterationTerminator as Term,
    NativeIterationValueScope as Scope, NativeMatchAction as Scalar,
    NativeMatchTerminator as Branch, NativeMatchValue as V, NativeProgramArtifactPlan as Plan,
    NativeProgramFacts as Facts, NativeScalarAggregateAction as Aggregate, NativeTailLoop,
};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Lane {
    value: u32,
    offset: usize,
}
impl Lane {
    fn start(value: V) -> Self {
        Self {
            value: value.get(),
            offset: 0,
        }
    }
}

#[derive(Debug)]
struct Compare {
    current: u32,
    previous: u32,
    width: u8,
}

pub(super) struct Proof {
    pub initialize: usize,
    pub clear: usize,
    pub exit: usize,
    source: V,
    result: V,
    compares: Vec<Compare>,
    sites: usize,
}
impl Proof {
    pub fn summary(&self) -> serde_json::Value {
        serde_json::json!({
            "initialize":self.initialize, "clear":self.clear, "exit":self.exit,
            "source":self.source.get(), "result":self.result.get(), "source_sites":self.sites,
            "compares":self.compares.iter().map(|c| (c.current,c.previous,c.width)).collect::<Vec<_>>()
        })
    }
}

struct Analysis<'a> {
    plan: &'a Plan,
    layout: &'a Layout,
    flow: &'a Flow,
    actions: &'a [Recipe],
    facts: Facts<'a>,
    guards: &'a Guards,
    owners: &'a OwnerFlow,
    consuming: &'a [bool],
}
impl Analysis<'_> {
    fn stable_fact(&self, value: u32, iteration: u32) -> bool {
        matches!(self.facts.definition_sites(value), Ok([definition])
            if definition.iteration() == Some(iteration))
    }

    fn movement(&self, site: usize, lane: Lane) -> Option<Lane> {
        let source = |value: V, offset| Lane {
            value: value.get(),
            offset,
        };
        match self.actions.get(site)? {
            Recipe::Aggregate(Aggregate::Copy {
                source: from,
                output,
            }) if output.get() == lane.value => Some(source(*from, lane.offset)),
            Recipe::Aggregate(Aggregate::Project {
                source: from,
                output,
                offset,
                guards,
            }) if output.get() == lane.value && guards.is_empty() => {
                Some(source(*from, offset.checked_add(lane.offset)?))
            }
            Recipe::Aggregate(Aggregate::Pack { output, words }) if output.get() == lane.value => {
                match *words.get(lane.offset)? {
                    Word::Scalar(from) => Some(source(from, 0)),
                    Word::Field(from, offset) => Some(source(from, offset)),
                    Word::Constant(_) => None,
                }
            }
            Recipe::None => match self.plan.blocks()[site].action() {
                action @ (Action::Initialize { iteration } | Action::Latch { iteration }) => {
                    let Loop::Tail(tail) = &self.plan.iterations()[*iteration as usize] else {
                        return None;
                    };
                    (tail.carrier().get() == lane.value).then(|| {
                        source(
                            if matches!(action, Action::Initialize { .. }) {
                                tail.seed()
                            } else {
                                tail.next()
                            },
                            lane.offset,
                        )
                    })
                }
                _ => None,
            },
            _ => None,
        }
    }

    // All definitions in the same outer visit. Discard a mutually exclusive
    // definition only with a single-definition predicate in that same visit.
    fn latch_sources(
        &self,
        value: Lane,
        tail: &NativeTailLoop,
        iteration: u32,
        boundary: V,
    ) -> Option<BTreeSet<Lane>> {
        let mut sources = BTreeSet::new();
        let mut pending = vec![value];
        let mut seen = BTreeSet::new();
        while let Some(lane) = pending.pop() {
            if lane.value == boundary.get() {
                sources.insert(lane);
                continue;
            }
            if !seen.insert(lane) {
                continue;
            }
            let scope = self.plan.value_scopes()[lane.value as usize];
            if lane.value == tail.carrier().get()
                || scope
                    != (Scope::Immutable {
                        iteration: Some(iteration),
                    })
            {
                sources.insert(lane);
                continue;
            }
            for definition in self.facts.definition_sites(lane.value).ok()? {
                let site = definition.site()? as usize;
                if !self.flow.reachable[site]
                    || self
                        .guards
                        .contradictory_at(site, tail.latch() as usize, |v| {
                            self.stable_fact(v, iteration)
                        })
                {
                    continue;
                }
                if let Some(parent) = self.movement(site, lane) {
                    pending.push(parent);
                } else {
                    sources.insert(lane);
                }
            }
        }
        (!sources.is_empty()).then_some(sources)
    }

    // Follow pure array transport and each append back to its cleared storage.
    // Current-visit validation independently requires an initialized seed.
    fn rebuilt_from(&self, value: V, storage: V, region: &BTreeSet<usize>) -> Option<()> {
        let storage = Lane::start(storage);
        let mut pending = vec![Lane::start(value)];
        let mut seen = BTreeSet::new();
        let mut witnessed_clear = false;
        while let Some(lane) = pending.pop() {
            if lane == storage {
                witnessed_clear = true;
                continue;
            }
            if !seen.insert(lane) {
                continue;
            }
            let definitions = self.facts.definition_sites(lane.value).ok()?;
            if definitions.is_empty() {
                return None;
            }
            for definition in definitions {
                let at = definition.site()? as usize;
                if !self.flow.reachable[at] {
                    continue;
                }
                if !region.contains(&at) {
                    return None;
                }
                match &self.actions[at] {
                    Recipe::Buffer {
                        operation: P::SequenceAppend,
                        operands,
                        output,
                    } if lane == Lane::start(*output) && self.consuming[at] => {
                        pending.push(Lane::start(operands[0]))
                    }
                    _ => pending.push(self.movement(at, lane)?),
                }
            }
        }
        witnessed_clear.then_some(())
    }

    fn within(&self, start: usize, end: usize, iteration: u32) -> Option<BTreeSet<usize>> {
        let mut visited = BTreeSet::new();
        let mut pending = vec![start];
        let mut reached_exit = false;
        while let Some(site) = pending.pop() {
            if site == end {
                reached_exit = true;
                continue;
            }
            if !visited.insert(site) {
                continue;
            }
            let block = &self.plan.blocks()[site];
            if !self
                .facts
                .lexical_ancestor(Some(iteration), block.iteration())
            {
                return None;
            }
            match (&self.actions[site], block.action(), block.terminator()) {
                (Recipe::Aggregate(_) | Recipe::Length { .. } | Recipe::Read { .. }, _, _) => {}
                (
                    Recipe::Buffer {
                        operation: P::SequenceClear,
                        ..
                    },
                    _,
                    _,
                ) if site == start => {}
                (
                    Recipe::Buffer {
                        operation: P::SequenceAppend,
                        ..
                    },
                    _,
                    _,
                ) if self.consuming[site] => {}
                (Recipe::None, Action::Scalar(Scalar::None | Scalar::Primitive { .. }), _) => {}
                (
                    Recipe::None,
                    Action::Initialize { iteration: nested } | Action::Latch { iteration: nested },
                    _,
                ) if *nested != iteration
                    && matches!(&self.plan.iterations()[*nested as usize], Loop::Tail(_)) => {}
                _ => return None,
            }
            if self.flow.successors[site].is_empty()
                && !matches!(
                    block.terminator(),
                    Term::Scalar(Branch::Reject | Branch::IntegrityFailure)
                )
            {
                return None;
            }
            pending.extend(&self.flow.successors[site]);
        }
        if !reached_exit {
            return None;
        }
        // The new edge mirrors one successful traversal. Interior values
        // cannot enter or escape via an unrelated control path.
        for &site in &visited {
            if site != start
                && self.facts.predecessors(site as u32).ok()?.iter().any(|at| {
                    self.flow.reachable[*at as usize] && !visited.contains(&(*at as usize))
                })
            {
                return None;
            }
        }
        Some(visited)
    }

    // A total or checked deterministic scalar expression which already
    // succeeded, with invariant operands in the same enclosing visit.
    fn fixed_expression(
        &self,
        value: u32,
        tail: &NativeTailLoop,
        iteration: u32,
        result: V,
        pending: &mut BTreeSet<u32>,
    ) -> bool {
        if let Scope::Immutable { iteration: scope } = self.plan.value_scopes()[value as usize]
            && scope != Some(iteration)
            && self.facts.lexical_ancestor(scope, Some(iteration))
        {
            return true;
        }
        if let Some(offset) = self.project_home(value, tail, iteration) {
            let field = Lane {
                value: tail.next().get(),
                offset,
            };
            return self.latch_sources(field, tail, iteration, result)
                == Some(BTreeSet::from([Lane {
                    value: tail.carrier().get(),
                    offset,
                }]));
        }
        if !pending.insert(value) {
            return false;
        }
        let valid = (|| {
            let [definition] = self.facts.definition_sites(value).ok()? else {
                return None;
            };
            let site = definition.site()? as usize;
            let Action::Scalar(Scalar::Primitive { operands, .. }) =
                self.plan.blocks()[site].action()
            else {
                return None;
            };
            Some(
                operands
                    .iter()
                    .all(|arg| self.fixed_expression(arg.get(), tail, iteration, result, pending)),
            )
        })()
        .unwrap_or(false);
        pending.remove(&value);
        valid
    }

    fn project_home(&self, value: u32, tail: &NativeTailLoop, iteration: u32) -> Option<usize> {
        let [def] = self.facts.definition_sites(value).ok()? else {
            return None;
        };
        if def.iteration() != Some(iteration) {
            return None;
        }
        let lane = self.movement(def.site()? as usize, Lane { value, offset: 0 })?;
        (lane.value == tail.carrier().get()).then_some(lane.offset)
    }

    fn build(
        &self,
        clear: usize,
        source: V,
        emptied: V,
        result: V,
        exit: usize,
        tail: &NativeTailLoop,
        iteration: u32,
    ) -> Result<Proof, &'static str> {
        if !self.facts.strictly_dominates(clear as u32, tail.latch())? {
            return Err("overwrite does not execute on every continuing visit");
        }
        let scratch = self.layout.ty(source.get());
        if scratch != self.layout.ty(result.get())
            || scratch != self.layout.ty(emptied.get())
            || !matches!(scratch.kind, LayoutKind::Array { .. })
            || self
                .owners
                .live_at_entry(exit, self.layout.slot(source.get()) + ARRAY_OWNER)
            || !self
                .owners
                .live_at_entry(exit, self.layout.slot(result.get()) + ARRAY_OWNER)
            || !self.consuming[clear]
        {
            return Err("region geometry or output liveness");
        }
        let scratch_field = self
            .project_home(source.get(), tail, iteration)
            .ok_or("scratch carrier projection")?;
        let next_lane = |offset| Lane {
            value: tail.next().get(),
            offset,
        };
        if self
            .latch_sources(next_lane(scratch_field), tail, iteration, result)
            .ok_or("scratch latch provenance")?
            != BTreeSet::from([Lane::start(result)])
        {
            return Err("scratch is not the result on every latch");
        }

        let region = self
            .within(clear, exit, iteration)
            .ok_or("region is not a checked overwrite")?;
        let scratch_slots = self.layout.slot(source.get());
        let result_slots = self.layout.slot(result.get());
        let appended = scratch
            .element()
            .ok()
            .and_then(|element| element.packed_bytes);
        if !matches!(appended, Some(1 | 2 | 4 | 8))
            || (scratch_slots..scratch_slots + scratch.words)
                .any(|slot| (result_slots..result_slots + scratch.words).contains(&slot))
        {
            return Err("incompatible array homes");
        }

        if self.plan.blocks()[exit].iteration() != Some(iteration)
            || self.flow.defs[clear].contains(&source.get())
            || self.flow.successors[clear].iter().any(|&next| {
                self.owners
                    .live_at_entry(next, self.layout.slot(source.get()) + ARRAY_OWNER)
            })
        {
            return Err("source storage remains live after the clear");
        }
        if self
            .facts
            .predecessors(exit as u32)?
            .iter()
            .any(|i| self.flow.reachable[*i as usize] && !region.contains(&(*i as usize)))
        {
            return Err("external path into exit");
        }
        self.rebuilt_from(result, emptied, &region)
            .ok_or("result provenance")?;
        let mut definitions = BTreeSet::new();
        let mut reads = BTreeSet::new();
        for &site in &region {
            if let Recipe::Buffer {
                operation: P::SequenceAppend,
                operands,
                ..
            } = &self.actions[site]
            {
                self.rebuilt_from(operands[0], emptied, &region)
                    .ok_or("append provenance")?;
            }
            if site != clear && self.flow.uses[site].contains(&source.get()) {
                return Err("old contents used");
            }
            if site != clear && self.flow.defs[site].contains(&source.get()) {
                return Err("scratch projection is redefined");
            }
            definitions.extend(self.flow.defs[site].iter().copied());
            reads.extend(self.flow.uses[site].iter().copied());
        }
        if !definitions.contains(&result.get())
            || self.flow.uses.iter().enumerate().any(|(site, uses)| {
                !region.contains(&site)
                    && uses
                        .iter()
                        .any(|v| *v != result.get() && definitions.contains(v))
            })
        {
            return Err("extra region publication");
        }

        let mut compares = Vec::new();
        let carrier = self.layout.ty(tail.carrier().get());
        let leaves = carrier.leaves();
        for &value in reads.difference(&definitions) {
            if value == source.get() {
                continue;
            }
            let Scope::Immutable { iteration: scope } = self.plan.value_scopes()[value as usize]
            else {
                return Err("external input is not immutable");
            };
            // Captures of a genuinely enclosing visit are not changed by
            // the current latch. Local projections are checked field-wise.
            if scope != Some(iteration) && self.facts.lexical_ancestor(scope, Some(iteration)) {
                continue;
            }
            if self.fixed_expression(value, tail, iteration, result, &mut BTreeSet::new()) {
                continue;
            }
            let current_field = self
                .project_home(value, tail, iteration)
                .ok_or("external input is not projected")?;
            let ty = self.layout.ty(value);
            let LayoutKind::Array { element } = &ty.kind else {
                return Err("modified nonarray field");
            };
            let LayoutKind::Scalar {
                bytes,
                signed: false,
                float: false,
            } = element.kind
            else {
                return Err("modified noninteger field");
            };
            let previous = leaves
                .iter()
                .filter(|(_, field)| field == ty)
                .find_map(|&(offset, _)| {
                    if self.latch_sources(next_lane(offset), tail, iteration, result)?
                        != BTreeSet::from([Lane {
                            value: tail.carrier().get(),
                            offset: current_field,
                        }])
                    {
                        return None;
                    }
                    (0..clear)
                        .rev()
                        .take_while(|&site| self.plan.blocks()[site].iteration() == Some(iteration))
                        .find_map(|site| match &self.actions[site] {
                            Recipe::Aggregate(Aggregate::Project {
                                source,
                                offset: at,
                                output,
                                guards,
                            }) if *source == tail.carrier()
                                && *at == offset
                                && guards.is_empty()
                                && self.layout.ty(output.get()) == ty
                                && self.owners.live_at_entry(
                                    clear,
                                    self.layout.slot(output.get()) + ARRAY_OWNER,
                                )
                                && self
                                    .facts
                                    .strictly_dominates(site as u32, clear as u32)
                                    .ok()
                                    == Some(true) =>
                            {
                                Some(output.get())
                            }
                            _ => None,
                        })
                })
                .ok_or("prior input is not available on this edge")?;
            if !self
                .owners
                .live_at_entry(clear, self.layout.slot(value) + ARRAY_OWNER)
            {
                return Err("current input has no owner on this edge");
            }
            compares.push(Compare {
                current: value,
                previous,
                width: bytes,
            });
        }
        Ok(Proof {
            initialize: tail.initialize() as usize,
            clear,
            exit,
            source,
            result,
            compares,
            sites: region.len(),
        })
    }
}

pub(super) fn derive(
    plan: &Plan,
    layout: &Layout,
    flow: &Flow,
    actions: &[Recipe],
    guards: &Guards,
    owners: &OwnerFlow,
    consuming: &[bool],
) -> Vec<Proof> {
    let analysis = Analysis {
        plan,
        layout,
        flow,
        actions,
        facts: plan.facts(),
        guards,
        owners,
        consuming,
    };
    let mut proofs = Vec::new();
    for (clear, action) in actions.iter().enumerate() {
        let Recipe::Buffer {
            operation: P::SequenceClear,
            operands,
            output,
        } = action
        else {
            continue;
        };
        let Some(iteration) = plan.blocks()[clear].iteration() else {
            continue;
        };
        let Loop::Tail(tail) = &plan.iterations()[iteration as usize] else {
            continue;
        };
        let Some(&source) = operands.first() else {
            continue;
        };
        for child in plan.iterations() {
            let Loop::Tail(child) = child else { continue };
            if child.parent() != Some(iteration) || child.initialize() as usize <= clear {
                continue;
            }
            let [exit] = flow.successors[child.exit() as usize].as_slice() else {
                continue;
            };
            let result = analysis.build(
                clear,
                source,
                *output,
                child.result(),
                *exit,
                tail,
                iteration,
            );
            if std::env::var_os("MECH_INSPECT_OVERWRITE_PROOF").is_some() {
                eprintln!(
                    "whole overwrite {clear} -> {exit} {}",
                    result.as_ref().err().unwrap_or(&"eligible")
                );
            }
            if let Ok(proof) = result {
                // Each pilot owns one overwrite region in an outer invocation.
                if !proofs
                    .iter()
                    .any(|other: &Proof| other.initialize == proof.initialize)
                {
                    proofs.push(proof);
                }
                break;
            }
        }
    }
    proofs
}

pub(super) struct State<'a> {
    pub proof: &'a Proof,
    ready: Variable,
    armed: Variable,
    requests: Variable,
    compare: Option<FuncRef>,
}

fn requests(b: &mut FunctionBuilder<'_>, heap: Value) -> Value {
    const OFFSET: i32 = (std::mem::offset_of!(Heap<'static>, counters)
        + std::mem::offset_of!(Counters, payload_requests)) as i32;
    b.ins().load(types::I64, MemFlags::new(), heap, OFFSET)
}

impl<'a> State<'a> {
    pub(super) fn declare<M: Module>(
        module: &mut M,
        b: &mut FunctionBuilder<'_>,
        proof: &'a Proof,
        zero: Value,
    ) -> Result<Self, NativeCompileError> {
        let ready = b.declare_var(types::I8);
        let armed = b.declare_var(types::I8);
        let allocation_start = b.declare_var(types::I64);
        let false_ = b.ins().iconst(types::I8, 0);
        b.def_var(ready, false_);
        b.def_var(armed, false_);
        b.def_var(allocation_start, zero);
        let compare = if proof.compares.is_empty() {
            None
        } else {
            let mut signature = module.make_signature();
            signature.params = vec![AbiParam::new(types::I64); 3];
            signature.returns = vec![AbiParam::new(types::I32)];
            let id = module
                .declare_function("memcmp", Linkage::Import, &signature)
                .map_err(backend)?;
            Some(module.declare_func_in_func(id, b.func))
        };
        Ok(Self {
            proof,
            ready,
            armed,
            requests: allocation_start,
            compare,
        })
    }

    pub(super) fn reset(&self, b: &mut FunctionBuilder<'_>) {
        let no = b.ins().iconst(types::I8, 0);
        b.def_var(self.ready, no);
    }

    pub(super) fn record_success(&self, b: &mut FunctionBuilder<'_>, heap: Value) {
        let old = b.use_var(self.requests);
        let current = requests(b, heap);
        let equal = b.ins().icmp(IntCC::Equal, current, old);
        let armed = b.use_var(self.armed);
        let allowed = b.ins().band(armed, equal);
        b.def_var(self.ready, allowed);
    }

    pub(super) fn emit(
        &self,
        b: &mut FunctionBuilder<'_>,
        words: &Words,
        layout: &Layout,
        arrays: &Arrays,
        owners: &OwnerFlow,
        destination: Block,
        error: Block,
        observer: Option<FuncRef>,
    ) {
        let from = layout.slot(self.proof.source.get());
        let to = layout.slot(self.proof.result.get());
        let view = arrays::read(words, b, layout, self.proof.source);
        let count = requests(b, arrays.heap);
        b.def_var(self.requests, count);
        let prior = b.use_var(self.ready);
        let inspect = b.create_block();
        let select = b.create_block();
        let fallback = b.create_block();
        let no = b.ins().iconst(types::I8, 0);
        b.append_block_param(select, types::I8);

        let row = b.ins().icmp_imm(IntCC::Equal, view[3], 1);
        let columns = b.ins().icmp(IntCC::Equal, view[1], view[4]);
        let capacity = b
            .ins()
            .icmp(IntCC::UnsignedLessThanOrEqual, view[1], view[2]);
        let owned = b.ins().icmp_imm(IntCC::NotEqual, view[5], 0);
        let geometry = b.ins().band(row, columns);
        let backing = b.ins().band(capacity, owned);
        let present = b.ins().band(geometry, backing);
        b.ins().brif(present, inspect, &[], select, &[no.into()]);
        b.switch_to_block(inspect);
        let refs = b.ins().load(types::I64, MemFlags::new(), view[5], 0);
        let private = b.ins().icmp_imm(IntCC::Equal, refs, 1);
        b.ins().jump(select, &[private.into()]);
        b.switch_to_block(select);
        let admitted = b.block_params(select)[0];
        b.def_var(self.armed, admitted);
        let previous = b.ins().band(prior, admitted);
        let compare = b.create_block();
        b.ins().brif(previous, compare, &[], fallback, &[]);
        b.switch_to_block(compare);

        for table in &self.proof.compares {
            let a = layout.slot(table.current);
            let z = layout.slot(table.previous);
            let check = b.create_block();
            let physical = b.create_block();
            let next = b.create_block();
            for lane in [1, 3, 4] {
                let a_word = words.read(b, a + lane);
                let z_word = words.read(b, z + lane);
                let same = b.ins().icmp(IntCC::Equal, a_word, z_word);
                let good = b.create_block();
                b.ins().brif(same, good, &[], fallback, &[]);
                b.switch_to_block(good);
            }
            let length = words.read(b, a + 1);
            let p = words.read(b, a);
            let q = words.read(b, z);
            let empty = b.ins().icmp_imm(IntCC::Equal, length, 0);
            let identical = b.ins().icmp(IntCC::Equal, p, q);
            let equal = b.ins().bor(empty, identical);
            b.ins().brif(equal, next, &[], check, &[]);
            b.switch_to_block(check);
            let width = b.ins().iconst(types::I64, i64::from(table.width));
            let (bytes, overflow) = b.ins().umul_overflow(length, width);
            b.ins().brif(overflow, fallback, &[], physical, &[]);
            b.switch_to_block(physical);
            let call = b.ins().call(
                self.compare.expect("declared exact comparison"),
                &[p, q, bytes],
            );
            let comparison = b.inst_results(call)[0];
            let same = b.ins().icmp_imm(IntCC::Equal, comparison, 0);
            b.ins().brif(same, next, &[], fallback, &[]);
            b.switch_to_block(next);
        }
        // Publish the exact current owned view. All six words were captured
        // before touching either home. The same owner moves once.
        if let Some(observer) = observer {
            let at = b.ins().iconst(types::I64, self.proof.clear as i64);
            let length = view[1];
            b.ins().call(observer, &[arrays.heap, at, length]);
        }
        arrays::write(words, b, to, &view);
        let zero = b.ins().iconst(types::I64, 0);
        words.write(b, from + ARRAY_OWNER, zero);
        let retire = owners.retire_bypass(self.proof.clear, self.proof.exit, &[to + ARRAY_OWNER]);
        arrays.retire(b, words, &retire, error);
        b.ins().jump(destination, &[]);
        b.switch_to_block(fallback);
    }
}
