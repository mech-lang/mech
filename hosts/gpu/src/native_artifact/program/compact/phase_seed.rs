//! Relate a packed tally to a current byte histogram through the checked source CFG.
use super::{Output, Phase, function::Histogram};
use crate::native_artifact::program::codegen::cfg_cleanup::bounded_byte_histogram::Admission;
use crate::native_artifact::program::compact::{
    flow::Flow,
    layout::{ARRAY_OWNER, ARRAY_WORDS, Layout, LayoutKind},
    owner_flow::OwnerFlow,
    phase_bodies::define_body,
    phase_layout,
    recipes::Recipe,
};
use crate::native_artifact::program::prepare::{
    PreparedNativeProgram, readonly_numeric::Operation,
};
use crate::native_artifact::{
    NativeCompileError,
    compiler::{backend, compiled_stack_bytes},
};
use cranelift_module::{FuncId, Linkage, Module};
use mech_core::ValueData;
use mech_engine::native_artifact::{
    NativeAggregateWord, NativeCollectionPrimitive, NativeControlLoop, NativeIterationAction,
    NativeMatchAction, NativeMatchValue, NativeProgramArtifactPlan, NativeScalarAggregateAction,
    NativeScalarKind, NativeScalarOp, NativeTailLoop,
};
use std::collections::{BTreeMap, BTreeSet};

const LITERAL_BINS: usize = 256;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Word {
    value: u32,
    offset: usize,
}

impl Word {
    fn scalar(value: NativeMatchValue) -> Self {
        Self {
            value: value.get(),
            offset: 0,
        }
    }

    fn field(self, offset: usize) -> Self {
        Self {
            offset: self.offset + offset,
            ..self
        }
    }
}

struct Transport<'a> {
    plan: &'a NativeProgramArtifactPlan,
    flow: &'a Flow,
    layout: &'a Layout,
    recipes: &'a [Recipe],
    writes: Vec<Vec<usize>>,
}

impl<'a> Transport<'a> {
    fn new(
        plan: &'a NativeProgramArtifactPlan,
        flow: &'a Flow,
        layout: &'a Layout,
        recipes: &'a [Recipe],
    ) -> Self {
        let mut writes = vec![Vec::new(); plan.values().len()];
        for (site, values) in flow
            .defs
            .iter()
            .enumerate()
            .filter(|(site, _)| flow.reachable[*site])
        {
            for &value in values {
                writes[value as usize].push(site);
            }
        }
        Self {
            plan,
            flow,
            layout,
            recipes,
            writes,
        }
    }

    fn from_slot(&self, slot: usize) -> Option<Word> {
        self.layout
            .base
            .iter()
            .zip(&self.layout.types)
            .enumerate()
            .find_map(|(value, (&base, ty))| {
                let offset = slot.checked_sub(base)?;
                (offset < ty.words).then_some(Word {
                    value: value as u32,
                    offset,
                })
            })
    }

    fn input(&self, phase: &Phase, word: usize) -> Option<Word> {
        self.from_slot(*phase.inputs.get(word)?)
    }

    fn def(&self, word: Word) -> Option<usize> {
        let &[site] = self.writes[word.value as usize].as_slice() else {
            return None;
        };
        Some(site)
    }

    fn source(&self, site: usize, word: Word) -> Option<Word> {
        use NativeScalarAggregateAction::{Copy, Pack, Project};
        match &self.recipes[site] {
            Recipe::Aggregate(Copy { source, output }) if output.get() == word.value => {
                return Some(Word::scalar(*source).field(word.offset));
            }
            Recipe::Aggregate(Project {
                source,
                offset,
                guards,
                output,
            }) if output.get() == word.value && guards.is_empty() => {
                return Some(Word::scalar(*source).field(offset + word.offset));
            }
            Recipe::Aggregate(Pack { output, words }) if output.get() == word.value => {
                return match words.get(word.offset)? {
                    NativeAggregateWord::Field(source, offset) => {
                        Some(Word::scalar(*source).field(*offset))
                    }
                    NativeAggregateWord::Scalar(source) => Some(Word::scalar(*source)),
                    NativeAggregateWord::Constant(_) => None,
                };
            }
            _ => {}
        }
        let action = self.plan.blocks()[site].action();
        let (NativeIterationAction::Initialize { iteration }
        | NativeIterationAction::Latch { iteration }) = action
        else {
            return None;
        };
        let NativeControlLoop::Tail(tail) = &self.plan.iterations()[*iteration as usize] else {
            return None;
        };
        if tail.carrier().get() != word.value {
            return None;
        }
        let value = if matches!(action, NativeIterationAction::Initialize { .. }) {
            tail.seed()
        } else {
            tail.next()
        };
        Some(Word::scalar(value).field(word.offset))
    }

    fn init_input(
        &self,
        tail: &NativeTailLoop,
        carrier: Word,
        accept: impl Fn(Word) -> bool,
    ) -> Option<Word> {
        if carrier.value != tail.carrier().get() {
            return None;
        }
        let mut at = Word::scalar(tail.seed()).field(carrier.offset);
        let mut visited = BTreeSet::new();
        while !accept(at) {
            if !visited.insert(at) {
                return None;
            }
            at = self.source(self.def(at)?, at)?;
        }
        Some(at)
    }

    fn array(&self, word: Word, bytes: u8) -> bool {
        word.offset == 0 && self.layout.ty(word.value).element().is_ok_and(|element| {
            matches!(element.kind, LayoutKind::Scalar { bytes: width, signed: false, float: false } if width == bytes)
        })
    }

    fn constant(&self, word: Word, depth: usize) -> Option<u64> {
        if depth == 0 || word.offset != 0 {
            return None;
        }
        if self.writes[word.value as usize].is_empty() {
            let value = self
                .plan
                .constants()
                .iter()
                .find(|c| c.value().get() == word.value)?;
            return match value.constant().data() {
                ValueData::U64(n) => Some(*n),
                ValueData::U32(n) => Some((*n).into()),
                _ => None,
            };
        }
        let site = self.def(word)?;
        if let Some(source) = self.source(site, word) {
            return self.constant(source, depth - 1);
        }
        let NativeIterationAction::Scalar(NativeMatchAction::Primitive {
            op: NativeScalarOp::U64WrappingAdd,
            operands,
            output,
        }) = self.plan.blocks()[site].action()
        else {
            return None;
        };
        let &[left, right] = operands.as_ref() else {
            return None;
        };
        (output.get() == word.value).then_some(())?;
        Some(
            self.constant(Word::scalar(left), depth - 1)?
                .wrapping_add(self.constant(Word::scalar(right), depth - 1)?),
        )
    }

    fn reaches(&self, start: &[usize], target: usize, stop: impl Fn(usize) -> bool) -> bool {
        let mut visited = vec![false; self.flow.successors.len()];
        let mut pending = start.to_vec();
        while let Some(site) = pending.pop() {
            if site == target {
                return true;
            }
            if !core::mem::replace(&mut visited[site], true) && !stop(site) {
                pending.extend(&self.flow.successors[site]);
            }
        }
        false
    }

    fn dominates(&self, site: usize, target: usize) -> bool {
        self.flow.reachable[site]
            && self.flow.reachable[target]
            && !self.reaches(&[self.flow.entry], target, |at| at == site)
    }

    fn refreshes(&self, start: usize, through: usize, target: usize) -> bool {
        !self.reaches(&self.flow.successors[start], target, |at| at == through)
    }

    // Follow every reachable carrier definition. Cyclic forwarding needs a
    // real incoming origin, and each read must have been assigned since the
    // latest execution of that origin. Site dominance alone permits stale
    // copies when an outer loop executes the same definition again.
    fn origin(&self, word: Word, stop: Option<Word>) -> Option<(Word, Vec<(Word, usize)>)> {
        let mut pending = vec![word];
        let mut visited = BTreeSet::new();
        let mut reads = Vec::new();
        let mut root = None;
        while let Some(at) = pending.pop() {
            if !visited.insert(at) {
                continue;
            }
            let defs = &self.writes[at.value as usize];
            if stop != Some(at) && !defs.is_empty() {
                let sources = defs
                    .iter()
                    .map(|&site| self.source(site, at).map(|source| (source, site)))
                    .collect::<Option<Vec<_>>>();
                if let Some(sources) = sources {
                    pending.extend(sources.iter().map(|&(source, _)| source));
                    reads.extend(sources);
                    continue;
                }
                if defs.len() != 1 {
                    return None;
                }
            }
            if root.is_some_and(|prior| prior != at) {
                return None;
            }
            root = Some(at);
        }
        Some((root?, reads))
    }

    fn current(&self, word: Word, site: usize, expected: Word) -> Option<()> {
        let (root, mut reads) = self.origin(word, Some(expected))?;
        if root != expected {
            return None;
        }
        if let Some(anchor) = self.def(root) {
            if !self.dominates(anchor, site) {
                return None;
            }
            reads.push((word, site));
            for (word, read) in reads {
                if word.value == root.value {
                    continue;
                }
                let writes = &self.writes[word.value as usize];
                if self.reaches(&self.flow.successors[anchor], read, |at| {
                    at == anchor || writes.contains(&at)
                }) {
                    return None;
                }
            }
        } else if !self.writes[root.value as usize].is_empty()
            || !(self
                .plan
                .inputs()
                .iter()
                .any(|p| p.value().get() == root.value)
                || self
                    .plan
                    .constants()
                    .iter()
                    .any(|c| c.value().get() == root.value))
        {
            return None;
        }
        Some(())
    }

    fn view(&self, word: Word, site: usize, expected: Word) -> Option<()> {
        for lane in 0..ARRAY_WORDS {
            self.current(word.field(lane), site, expected.field(lane))?;
        }
        Some(())
    }
}

struct ByteHistogram<'a> {
    phase: &'a Phase,
    tail: &'a NativeTailLoop,
    raw: Word,
    begin: Word,
    length: Word,
    result: Word,
    count: u64,
}

impl<'a> ByteHistogram<'a> {
    fn prove(
        transport: &Transport<'_>,
        prepared: &'a PreparedNativeProgram,
        phase: &'a Phase,
        admission: Admission,
    ) -> Option<Self> {
        let region = prepared
            .metadata()
            .closed_tails
            .regions
            .iter()
            .find(|region| region.tail.header() as usize == phase.header)?;
        let tail = &region.tail;
        let bins = *phase.inputs.get(admission.bin_word)?;
        let array = phase
            .arrays
            .iter()
            .position(|&(base, writable)| base == bins && writable)?;
        let numeric = region.numeric.as_ref()?;
        let mut writes = numeric
            .actions
            .iter()
            .filter_map(|(_, operation)| match operation {
                Operation::Write { array, .. } => Some(*array),
                _ => None,
            });
        if numeric.result.direct_array()? != array
            || writes.next() != Some(array)
            || writes.next().is_some()
            || numeric
                .actions
                .iter()
                .any(|(_, operation)| matches!(operation, Operation::Fallback) || operation.grows())
        {
            return None;
        }
        let at = phase.header;
        let raw = transport.input(phase, admission.input_word)?;
        let captured_raw = transport.init_input(tail, raw, |w| transport.array(w, 1))?;
        transport.view(raw, at, captured_raw)?;
        let scalar = |w: Word| {
            w.offset == 0
                && transport.plan.values()[w.value as usize].scalar_kind()
                    == Some(NativeScalarKind::U64)
        };
        let captured_scalar = |index| {
            let current = transport.input(phase, index)?;
            let root = transport.init_input(tail, current, scalar)?;
            transport.current(current, at, root)?;
            Some(root)
        };
        let begin = captured_scalar(admission.begin_word)?;
        let length = captured_scalar(admission.end_word)?;
        let cursor =
            transport.init_input(tail, transport.input(phase, admission.cursor_word)?, scalar)?;
        if transport.constant(cursor, 8)? != 0 {
            return None;
        }
        let zeros =
            transport.init_input(tail, transport.from_slot(bins)?, |w| transport.array(w, 4))?;
        let initialized = tail.initialize() as usize;
        let zero_site = transport.def(zeros)?;
        let NativeIterationAction::CollectionFinish { collection } =
            transport.plan.blocks()[zero_site].action()
        else {
            return None;
        };
        let uniform = prepared
            .metadata()
            .uniform_word_collections
            .iter()
            .find(|p| p.collection() == *collection)?;
        let start = transport.constant(Word::scalar(uniform.start()), 8)?;
        let limit = transport.constant(Word::scalar(uniform.limit()), 8)?;
        let count = limit
            .checked_sub(start)?
            .checked_add(u64::from(uniform.inclusive() && !uniform.filtered()))?;
        if uniform.kind() != NativeScalarKind::U32
            || count < LITERAL_BINS as u64
            || transport.constant(Word::scalar(uniform.word()), 8)? != 0
            || !transport.dominates(zero_site, initialized)
            || !transport.refreshes(phase.exit, zero_site, initialized)
        {
            return None;
        }
        // Check the incoming seed packet in this loop visit. Its writable
        // array and cursor legitimately change on the histogram backedge.
        let at_init = |word| {
            let word = transport.input(phase, word)?;
            Some(Word::scalar(tail.seed()).field(word.offset))
        };
        transport.view(at_init(admission.bin_word)?, initialized, zeros)?;
        transport.current(at_init(admission.cursor_word)?, initialized, cursor)?;
        let result = Word::scalar(tail.result());
        if !transport.array(result, 4)
            || transport.def(result)? != phase.exit
            || transport.source(phase.exit, result)? != Word::scalar(tail.returned())
            || !transport.dominates(at, phase.exit)
        {
            return None;
        }
        Some(Self {
            phase,
            tail,
            raw: captured_raw,
            begin,
            length,
            result,
            count,
        })
    }

    fn at_tally(&self, transport: &Transport<'_>, phase: &Phase, end_word: usize) -> Option<()> {
        let at = phase.header;
        let initialized = self.tail.initialize() as usize;
        transport.current(transport.input(phase, end_word)?, at, self.length)?;
        if !transport.dominates(self.phase.exit, at)
            || !transport.refreshes(initialized, self.phase.exit, at)
        {
            return None;
        }
        for reference in [self.raw, self.begin, self.length] {
            transport.current(reference, at, reference)?;
            if let Some(site) = transport.def(reference)
                && !transport.refreshes(site, initialized, at)
            {
                return None;
            }
        }
        Some(())
    }

    fn seed_at(&self, transport: &Transport<'_>, candidate: Word, at: usize) -> Option<()> {
        let (root, _) = transport.origin(candidate, Some(self.result))?;
        if root != self.result {
            let store = transport.def(root)?;
            let Recipe::Buffer {
                operation: NativeCollectionPrimitive::MatrixIndexedReplace,
                operands,
                output,
            } = &transport.recipes[store]
            else {
                return None;
            };
            let &[array, _, index] = operands.as_ref() else {
                return None;
            };
            // Matrix indices are one-based. The update must leave the entire
            // literal prefix unchanged, including when source storage is moved.
            if output.get() != root.value
                || root.offset != 0
                || transport.constant(Word::scalar(index), 8)? <= LITERAL_BINS as u64
                || !transport.refreshes(self.phase.exit, store, at)
            {
                return None;
            }
            transport.view(Word::scalar(array), store, self.result)?;
        }
        transport.view(candidate, at, root)
    }
}

#[derive(Clone, Copy, serde::Serialize)]
pub(in crate::native_artifact::program::compact) struct SeedProof {
    producer: usize,
    consumer: usize,
    pub(super) raw_view: usize,
    pub(super) begin: usize,
    pub(super) seed_view: usize,
    packed_end_word: usize,
    #[serde(skip)]
    count: u64,
}

pub(in crate::native_artifact::program::compact) fn prove_seeded_histograms(
    prepared: &PreparedNativeProgram,
    layout: &Layout,
    flow: &Flow,
    recipes: &[Recipe],
    owners: &OwnerFlow,
    phases: &[&Phase],
) -> Vec<SeedProof> {
    let transport = Transport::new(prepared.plan(), flow, layout, recipes);
    let producers = phases
        .iter()
        .filter_map(|phase| match phase.histogram {
            Some(Histogram::Bytes(admission)) => {
                ByteHistogram::prove(&transport, prepared, phase, admission)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let mut result = Vec::new();
    for phase in phases {
        let Some(Histogram::Packed(admission)) = phase.histogram else {
            continue;
        };
        let Some(end_word) = admission.end_word else {
            continue;
        };
        if admission.minimum_bins != LITERAL_BINS as u64 {
            continue;
        }
        for producer in &producers {
            let raw = layout.slot(producer.raw.value);
            if !owners.live_at_entry(phase.header, raw + ARRAY_OWNER)
                || producer.at_tally(&transport, phase, end_word).is_none()
            {
                continue;
            }
            let seed = layout.types.iter().enumerate().find_map(|(value, _)| {
                let candidate = Word {
                    value: value as u32,
                    offset: 0,
                };
                let slot = layout.slot(candidate.value);
                if !transport.array(candidate, 4)
                    || !owners.live_at_entry(phase.header, slot + ARRAY_OWNER)
                {
                    return None;
                }
                producer.seed_at(&transport, candidate, phase.header)?;
                Some(slot)
            });
            if let Some(seed_view) = seed {
                result.push(SeedProof {
                    producer: producer.phase.header,
                    consumer: phase.header,
                    raw_view: raw,
                    begin: layout.slot(producer.begin.value),
                    seed_view,
                    packed_end_word: end_word,
                    count: producer.count,
                });
                break;
            }
        }
    }
    result
}

pub(in crate::native_artifact::program::compact) struct Route {
    pub(super) target: FuncId,
    pub(super) seed: SeedProof,
    pub summary: serde_json::Value,
}

pub(in crate::native_artifact::program::compact) fn define_seeded_histograms<M: Module>(
    module: &mut M,
    phases: &[&Phase],
    proofs: &[SeedProof],
) -> Result<BTreeMap<usize, Route>, NativeCompileError> {
    let mut routes = BTreeMap::new();
    for &seed in proofs {
        let Some(phase) = phases.iter().copied().find(|p| p.header == seed.consumer) else {
            continue;
        };
        let Some(body) = &phase.seeded else { continue };
        if phase.vector.is_some()
            || phase.no_growth.is_some()
            || phase.private.is_some()
            || phase
                .outputs
                .iter()
                .any(|(_, value)| matches!(value, Output::Stack(_)))
        {
            continue;
        }
        let name = format!("mech_compact_numeric_{}_seeded", phase.header);
        let mut context = module.make_context();
        context.func = body.function();
        let fallback = module.declare_func_in_func(phase.function, &mut context.func);
        if body.emit(&mut context.func, fallback, seed.count).is_none() {
            continue;
        }
        context.compute_cfg();
        context.compute_domtree();
        context
            .eliminate_unreachable_code(module.isa())
            .map_err(backend)?;
        context.verify(module.isa()).map_err(backend)?;
        let inspection = std::env::var_os("MECH_INSPECT_PHASE_REGISTER_PROFILE")
            .map(std::path::PathBuf::from)
            .filter(|path| path.file_stem().is_some_and(|stem| stem == name.as_str()));
        if let Some(path) = &inspection {
            std::fs::write(
                path.with_extension("clif"),
                context.func.display().to_string(),
            )
            .map_err(backend)?;
            context.set_disasm(true);
        }
        phase_layout::apply_register_weights(&mut context, &name).map_err(backend)?;
        let id = module
            .declare_function(&name, Linkage::Local, &context.func.signature)
            .map_err(backend)?;
        let (target, _) = define_body(module, id, &name, &mut context, None)?;
        let compiled = context
            .compiled_code()
            .ok_or_else(|| backend("seeded tally lacks code"))?;
        if let Some(path) = inspection {
            let vcode = compiled
                .vcode
                .as_ref()
                .ok_or_else(|| backend("seeded tally lacks VCode"))?;
            std::fs::write(path.with_extension("vcode"), vcode).map_err(backend)?;
        }
        let entry_name = format!("{name}_entry");
        let mut entry = module.make_context();
        entry.func.signature = context.func.signature.clone();
        let attempt = module.declare_func_in_func(target, &mut entry.func);
        let fallback = module.declare_func_in_func(phase.function, &mut entry.func);
        body.emit_entry(&mut entry.func, attempt, fallback);
        entry.verify(module.isa()).map_err(backend)?;
        let entry_id = module
            .declare_function(&entry_name, Linkage::Local, &entry.func.signature)
            .map_err(backend)?;
        let (target, _) = define_body(module, entry_id, &entry_name, &mut entry, None)?;
        let adapter = entry
            .compiled_code()
            .ok_or_else(|| backend("seeded tally entry lacks code"))?;
        let summary = serde_json::json!({
            "symbol": name, "proof": seed, "bin_count": seed.count,
            "code_bytes": compiled.code_info().total_size,
            "stack_bytes": compiled_stack_bytes(&compiled.buffer)?,
            "entry": {"symbol": entry_name, "code_bytes": adapter.code_info().total_size,
                      "stack_bytes": compiled_stack_bytes(&adapter.buffer)?},
        });
        routes.insert(
            phase.header,
            Route {
                target,
                seed,
                summary,
            },
        );
    }
    Ok(routes)
}
