//! Register carriers for one invocation-owned packed table. Only slow calls
//! expose the frame to Rust. The maintained private callback's exact effects
//! permit narrow synchronization; unknown operations and exits retain it all.
use crate::native_artifact::codegen::options::OptimizationPass;
use crate::native_artifact::program::codegen::hooks::Hooks;
#[cfg(feature = "canonical-native-program-runtime")]
use crate::native_artifact::program::protocol::OwnerOperation;
use crate::native_artifact::program::runtime::maps::key_domain::Domain;
use crate::native_artifact::program::runtime::maps::packed::Frame;
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, types};
use cranelift_frontend::{FunctionBuilder, Variable};
use std::cell::Cell;

// A changed private layout must revisit this effect proof. In particular, a
// new validation read or runtime write must not silently inherit a zero mask.
const WORDS: usize = 20;
const _: () = assert!(core::mem::size_of::<Frame>() == WORDS * 8);
macro_rules! fields {
    ($($field:ident),* $(,)?) => { 0u64 $(| (1u64 << (core::mem::offset_of!(Frame, $field) / 8)))* };
}
const IMMUTABLE: u64 = fields!(
    source_ptr,
    source_len,
    width,
    low_mask,
    high_mask,
    lookup_cache
);
const MUTABLE: u64 = ((1u64 << WORDS) - 1) & !IMMUTABLE;
// Enforced by write(), not inferred from today's callers. Runtime-owned words
// remain current in memory between callbacks because generated code cannot
// change them; therefore a helper needs stores only for reads in this set.
const GENERATED: u64 = fields!(
    fuel,
    updates,
    key_low,
    key_high,
    entry_pointer,
    entry_known,
    previous_start
);
const VALIDATION_READS: u64 = fields!(
    slots,
    controls,
    capacity,
    len,
    fuel,
    entry_known,
    width,
    lookup_cache
);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Effects {
    reads: u64,
    writes: u64,
}
impl Effects {
    fn stores(self) -> u64 {
        self.reads & GENERATED
    }
}
fn effects(operation: OwnerOperation) -> Option<Effects> {
    // This table belongs ONLY to the function/context pair installed by
    // program_map_closed_runtime::begin. It is not an arbitrary host-call
    // annotation. Validation still reads all its original fields, including
    // the runtime-owned words already current in memory.
    Some(match operation {
        OwnerOperation::Refill => Effects {
            reads: VALIDATION_READS,
            writes: fields!(fuel, entry_known, entry_pointer),
        },
        OwnerOperation::InsertMissingPacked => Effects {
            reads: VALIDATION_READS | fields!(key_low, key_high, entry_pointer, updates),
            writes: fields!(
                slots,
                controls,
                capacity,
                len,
                updates,
                entry_pointer,
                range_call_allowed,
                read_call_allowed,
                write_call_allowed
            ),
        },
        OwnerOperation::PrepareAndSelectRange
        | OwnerOperation::MissingKeyFailure
        | OwnerOperation::InvalidPackedOperation => Effects {
            reads: 0,
            writes: 0,
        },
        OwnerOperation::RangeCallDenied
        | OwnerOperation::ReadCallDenied
        | OwnerOperation::WriteCallDenied => Effects {
            reads: VALIDATION_READS,
            writes: 0,
        },
        // Legacy ordinary-map operations and future private operations have
        // no effect proof. Preserve full synchronization for those calls.
        _ => return None,
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::native_artifact) enum LookupMode {
    Dynamic,
    Hash,
    Direct,
}

pub(in crate::native_artifact) struct FrameState {
    options: crate::native_artifact::codegen::options::OptimizationOptions,
    pointer: Value,
    words: Option<Vec<Variable>>,
    effects_enabled: bool,
    dirty_only: bool,
    constant_packet: Option<Value>,
    constant_values: Option<[Value; 2]>,
    constant_hash_loads: Cell<usize>,
    specialized_width: Option<u64>,
    lookup_mode: LookupMode,
    // Callee-local SSA only, never part of the private runtime frame ABI.
    // The valid bit is cleared at pointer changes, helper calls and reloads.
    candidate_value: Option<(Variable, Variable)>,
    candidate_value_loads: Cell<usize>,
    paired_candidate_loads: Cell<usize>,
    // Constant-size actual emission evidence, aggregated by private operation.
    // It does not claim that a runtime path executed or authorize any access.
    emissions: Cell<[u64; 14]>,
    pub(in crate::native_artifact::program::codegen::packed_map) domain: Domain,
}
impl FrameState {
    pub(in crate::native_artifact::program::codegen::packed_map) fn memory(
        pointer: Value,
        domain: Domain,
    ) -> Self {
        Self {
            pointer,
            options: crate::native_artifact::codegen::options::OptimizationOptions::default(),
            words: None,
            effects_enabled: false,
            dirty_only: false,
            constant_packet: None,
            constant_values: None,
            constant_hash_loads: Cell::new(0),
            specialized_width: None,
            lookup_mode: LookupMode::Dynamic,
            candidate_value: None,
            candidate_value_loads: Cell::new(0),
            paired_candidate_loads: Cell::new(0),
            emissions: Cell::new([0; 14]),
            domain,
        }
    }
    pub(in crate::native_artifact) fn registers(
        b: &mut FunctionBuilder<'_>,
        pointer: Value,
        domain: Domain,
        options: crate::native_artifact::codegen::options::OptimizationOptions,
    ) -> Self {
        assert_eq!(core::mem::size_of::<Frame>() % 8, 0);
        let words = (0..core::mem::size_of::<Frame>() / 8)
            .map(|word| {
                let variable = b.declare_var(types::I64);
                let value = b
                    .ins()
                    .load(types::I64, MemFlags::new(), pointer, (word * 8) as i32);
                b.def_var(variable, value);
                variable
            })
            .collect();
        let effects_enabled = options.enabled(OptimizationPass::FrameEffects);
        let dirty_only = options.enabled(OptimizationPass::LoopInvariants);
        Self {
            pointer,
            options,
            words: Some(words),
            effects_enabled,
            dirty_only,
            constant_packet: None,
            constant_values: None,
            constant_hash_loads: Cell::new(0),
            specialized_width: None,
            lookup_mode: LookupMode::Dynamic,
            candidate_value: None,
            candidate_value_loads: Cell::new(0),
            paired_candidate_loads: Cell::new(0),
            emissions: Cell::new([0; 14]),
            domain,
        }
    }
    pub(in crate::native_artifact) fn with_constant_packet(
        mut self,
        packet: Option<Value>,
    ) -> Self {
        self.constant_packet = packet;
        self
    }
    pub(in crate::native_artifact) fn with_constant_values(mut self, values: [Value; 2]) -> Self {
        self.constant_values = Some(values);
        self
    }
    /// Only the helper-free source function may use this binding, after its
    /// entry switch compares the actual constructor-owned immutable width.
    /// Frame masks are constructor-derived and never writable by generated
    /// code or source helpers. A generic arm retains the ordinary values.
    pub(in crate::native_artifact) fn with_specialized_width(mut self, width: Option<u64>) -> Self {
        assert!(width.is_none_or(|width| {
            (1..=crate::native_artifact::program::runtime::maps::packed::MAX_WIDTH).contains(&width)
        }));
        self.specialized_width = width;
        self
    }
    /// Direct is confined to a separate private function: its caller tests the
    /// actual owned cache pointer and width, and its entry selects that width.
    pub(in crate::native_artifact) fn with_lookup_mode(mut self, mode: LookupMode) -> Self {
        assert!(
            mode != LookupMode::Direct
                || self.specialized_width.is_some_and(|width| {
                    crate::native_artifact::program::runtime::maps::packed::lookup_cache_entries(
                        self.domain,
                        width,
                    )
                    .is_some()
                })
        );
        self.lookup_mode = mode;
        self
    }
    /// Only a separately emitted helper-free hashed loop selects this cache.
    /// An occupied candidate's initialized payload is loaded with its key;
    /// full equality must succeed before that value becomes a source result.
    pub(in crate::native_artifact) fn with_candidate_value_loads(
        mut self,
        b: &mut FunctionBuilder<'_>,
    ) -> Self {
        if !self.options.enabled(OptimizationPass::CandidateValues) {
            return self;
        }
        if self.lookup_mode == LookupMode::Hash && self.words.is_some() {
            let value = b.declare_var(types::I64);
            let valid = b.declare_var(types::I8);
            let zero = b.ins().iconst(types::I64, 0);
            let no = b.ins().iconst(types::I8, 0);
            b.def_var(value, zero);
            b.def_var(valid, no);
            self.candidate_value = Some((value, valid));
        }
        self
    }
    pub(in crate::native_artifact::program::codegen::packed_map) fn has_candidate_values(
        &self,
    ) -> bool {
        self.candidate_value.is_some()
    }
    pub(in crate::native_artifact::program::codegen::packed_map) fn paired_candidates_enabled(
        &self,
    ) -> bool {
        if !self.options.enabled(OptimizationPass::PairedCandidates) {
            return false;
        }
        self.has_candidate_values()
    }
    pub(in crate::native_artifact::program::codegen::packed_map) fn note_paired_candidate_load(
        &self,
    ) {
        assert!(self.has_candidate_values());
        self.paired_candidate_loads
            .set(self.paired_candidate_loads.get() + 1);
    }
    pub(in crate::native_artifact) fn paired_candidate_loads(&self) -> usize {
        self.paired_candidate_loads.get()
    }
    pub(in crate::native_artifact::program::codegen::packed_map) fn note_candidate_value_load(
        &self,
    ) {
        assert!(self.has_candidate_values());
        self.candidate_value_loads
            .set(self.candidate_value_loads.get() + 1);
    }
    pub(in crate::native_artifact) fn candidate_value_loads(&self) -> usize {
        self.candidate_value_loads.get()
    }
    pub(in crate::native_artifact::program::codegen::packed_map) fn remember_candidate_value(
        &self,
        b: &mut FunctionBuilder<'_>,
        loaded: Value,
    ) {
        if let Some((value, valid)) = self.candidate_value {
            let yes = b.ins().iconst(types::I8, 1);
            b.def_var(value, loaded);
            b.def_var(valid, yes);
        }
    }
    pub(in crate::native_artifact::program::codegen::packed_map) fn cached_candidate_value(
        &self,
        b: &mut FunctionBuilder<'_>,
    ) -> Option<(Value, Value)> {
        self.candidate_value
            .map(|(value, valid)| (b.use_var(value), b.use_var(valid)))
    }
    pub(in crate::native_artifact::program::codegen::packed_map) fn invalidate_candidate_value(
        &self,
        b: &mut FunctionBuilder<'_>,
    ) {
        if let Some((_, valid)) = self.candidate_value {
            let no = b.ins().iconst(types::I8, 0);
            b.def_var(valid, no);
        }
    }
    pub(in crate::native_artifact::program::codegen::packed_map) fn direct_lookup(&self) -> bool {
        self.lookup_mode == LookupMode::Direct
    }
    pub(in crate::native_artifact::program::codegen::packed_map) fn hash_constant(
        &self,
        b: &mut FunctionBuilder<'_>,
        ordinal: usize,
    ) -> Option<Value> {
        assert!(ordinal < 2);
        if let Some(values) = self.constant_values {
            return Some(values[ordinal]);
        }
        let pointer = self.constant_packet?;
        self.constant_hash_loads
            .set(self.constant_hash_loads.get() + 1);
        // The root initialized this private packet before entering the callee.
        // Its pointer never reaches a helper or aliases any writable source.
        Some(b.ins().load(
            types::I64,
            MemFlags::new().with_readonly(),
            pointer,
            (ordinal * 8) as i32,
        ))
    }
    pub(in crate::native_artifact) fn constant_hash_loads(&self) -> usize {
        self.constant_hash_loads.get()
    }
    pub(in crate::native_artifact::program::codegen::packed_map) fn may_use_lookup_cache(
        &self,
    ) -> bool {
        self.lookup_mode != LookupMode::Hash
            && self.specialized_width.is_none_or(|width| {
                crate::native_artifact::program::runtime::maps::packed::lookup_cache_entries(
                    self.domain,
                    width,
                )
                .is_some()
            })
    }
    /// A cold continuation can recover immutable construction metadata from
    /// the actual pinned frame instead of keeping an entry-time SSA recipe
    /// live across all successful iterations. These are ordinary frame loads,
    /// with the same conservative memory flags as other private-frame reads.
    pub(in crate::native_artifact) fn snapshot_immutable(
        &self,
        b: &mut FunctionBuilder<'_>,
        offset: usize,
    ) -> Value {
        assert!(offset % 8 == 0 && offset / 8 < WORDS && IMMUTABLE & (1 << (offset / 8)) != 0);
        b.ins()
            .load(types::I64, MemFlags::new(), self.pointer, offset as i32)
    }
    pub(in crate::native_artifact) fn read(
        &self,
        b: &mut FunctionBuilder<'_>,
        offset: usize,
    ) -> Value {
        if let Some(width) = self.specialized_width {
            let bits = width * u64::from(self.domain.bits());
            let mask = (1u128 << bits) - 1;
            let constant = match offset {
                v if v == core::mem::offset_of!(Frame, width) => Some(width),
                v if v == core::mem::offset_of!(Frame, low_mask) => Some(mask as u64),
                v if v == core::mem::offset_of!(Frame, high_mask) => Some((mask >> 64) as u64),
                _ => None,
            };
            if let Some(value) = constant {
                return b.ins().iconst(types::I64, value as i64);
            }
        }
        if let Some(words) = &self.words {
            b.use_var(words[offset / 8])
        } else {
            b.ins()
                .load(types::I64, MemFlags::new(), self.pointer, offset as i32)
        }
    }
    pub(in crate::native_artifact) fn write(
        &self,
        b: &mut FunctionBuilder<'_>,
        offset: usize,
        value: Value,
    ) {
        assert!(
            offset % 8 == 0 && offset / 8 < WORDS && GENERATED & (1 << (offset / 8)) != 0,
            "generated packed-frame write requires an explicit effect proof"
        );
        if offset == core::mem::offset_of!(Frame, entry_pointer) {
            self.invalidate_candidate_value(b);
        }
        if let Some(words) = &self.words {
            b.def_var(words[offset / 8], value);
        } else {
            b.ins()
                .store(MemFlags::new(), value, self.pointer, offset as i32);
        }
    }
    pub(in crate::native_artifact) fn spill(&self, b: &mut FunctionBuilder<'_>) {
        // write() enforces that only GENERATED words can become dirty here.
        // Runtime-owned words are already current in memory; callbacks reload
        // every written SSA word before success and no generated write aliases
        // that storage. Retire only the actual generated state at exit/error.
        self.spill_mask(b, if self.dirty_only { GENERATED } else { MUTABLE });
    }
    fn spill_mask(&self, b: &mut FunctionBuilder<'_>, mask: u64) {
        if let Some(words) = &self.words {
            for (word, &variable) in words.iter().enumerate() {
                if mask & (1 << word) == 0 {
                    continue;
                }
                let value = b.use_var(variable);
                b.ins()
                    .store(MemFlags::new(), value, self.pointer, (word * 8) as i32);
            }
        }
    }
    fn reload_mask(&self, b: &mut FunctionBuilder<'_>, mask: u64) {
        if let Some(words) = &self.words {
            for (word, &variable) in words.iter().enumerate() {
                if mask & (1 << word) == 0 {
                    continue;
                }
                let value =
                    b.ins()
                        .load(types::I64, MemFlags::new(), self.pointer, (word * 8) as i32);
                b.def_var(variable, value);
            }
        }
    }
    pub(in crate::native_artifact) fn reload_generated(&self, b: &mut FunctionBuilder<'_>) {
        self.invalidate_candidate_value(b);
        self.reload_mask(b, GENERATED);
    }
    pub(in crate::native_artifact) fn call(
        &self,
        b: &mut FunctionBuilder<'_>,
        hooks: &Hooks,
        function: Value,
        context: Value,
        site: usize,
        operation: OwnerOperation,
        args: [Value; 3],
        error: Block,
    ) {
        let selected = self.effects_enabled.then(|| effects(operation)).flatten();
        let (stores, loads) = selected.map_or((MUTABLE, MUTABLE), |effects| {
            (effects.stores(), effects.writes)
        });
        self.spill_mask(b, stores);
        self.invalidate_candidate_value(b);
        hooks.bound_call(b, function, context, site, operation, args, error);
        // The helper status branch remains before every reload. Insertion is
        // failure-atomic; an error retains the same SSA values as before this
        // optimization and reaches the unchanged full terminal spill. Success
        // reloads ALL written words, including cap flags on non-growing inserts.
        self.reload_mask(b, loads);
        if selected.is_some() {
            let mut counts = self.emissions.get();
            counts[operation.raw() as usize] += 1;
            self.emissions.set(counts);
        }
    }
    pub(in crate::native_artifact) fn effect_emissions(&self) -> Vec<(u64, u64, u64, u64)> {
        self.emissions
            .get()
            .into_iter()
            .enumerate()
            .filter_map(|(op, count)| {
                if count == 0 {
                    return None;
                }
                let effects = effects(OwnerOperation::decode(op as u64))
                    .expect("only proved operations receive emission receipts");
                Some((op as u64, count, effects.stores(), effects.writes))
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "tests.rs"]
pub(in crate::native_artifact::program::codegen::packed_map::frame_state) mod tests;
