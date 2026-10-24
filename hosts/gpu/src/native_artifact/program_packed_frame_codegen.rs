//! Register carriers for one invocation-owned packed table. Only slow calls
//! expose the frame to Rust. The maintained private callback's exact effects
//! permit narrow synchronization; unknown operations and exits retain it all.
use super::super::super::{program_hooks::Hooks, program_packed_map::Frame, program_key_domain::Domain};
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, types};
use cranelift_frontend::{FunctionBuilder, Variable};
use std::cell::Cell;

// A changed private layout must revisit this effect proof. In particular, a
// new validation read or runtime write must not silently inherit a zero mask.
const WORDS: usize = 19;
const _: () = assert!(core::mem::size_of::<Frame>() == WORDS * 8);
macro_rules! fields {
    ($($field:ident),* $(,)?) => { 0u64 $(| (1u64 << (core::mem::offset_of!(Frame, $field) / 8)))* };
}
const IMMUTABLE: u64 = fields!(source_ptr, source_len, width, low_mask, high_mask);
const MUTABLE: u64 = ((1u64 << WORDS) - 1) & !IMMUTABLE;
// Enforced by write(), not inferred from today's callers. Runtime-owned words
// remain current in memory between callbacks because generated code cannot
// change them; therefore a helper needs stores only for reads in this set.
const GENERATED: u64 = fields!(fuel, updates, key_low, key_high, entry_pointer, entry_known, previous_start);
const VALIDATION_READS: u64 = fields!(slots, controls, capacity, len, fuel, entry_known, width);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Effects { reads: u64, writes: u64 }
impl Effects { fn stores(self) -> u64 { self.reads & GENERATED } }
fn effects(operation: u64) -> Option<Effects> {
    // This table belongs ONLY to the function/context pair installed by
    // program_map_closed_runtime::begin. It is not an arbitrary host-call
    // annotation. Validation still reads all its original fields, including
    // the runtime-owned words already current in memory.
    Some(match operation {
        0 => Effects { reads: VALIDATION_READS, writes: fields!(fuel, entry_known, entry_pointer) },
        7 => Effects {
            reads: VALIDATION_READS | fields!(key_low, key_high, entry_pointer, updates),
            writes: fields!(slots, controls, capacity, len, updates, entry_pointer,
                range_call_allowed, read_call_allowed, write_call_allowed),
        },
        6 | 8 | 9 => Effects { reads: 0, writes: 0 },
        10..=12 => Effects { reads: VALIDATION_READS, writes: 0 },
        // Legacy ordinary-map operations and future private operations have
        // no effect proof. Preserve full synchronization for those calls.
        _ => return None,
    })
}

#[cfg(test)]
thread_local! {
    static EFFECTS_DISABLED: Cell<bool> = const { Cell::new(false) };
    static EMISSIONS: Cell<(usize, usize, usize)> = const { Cell::new((0, 0, 0)) };
}
#[cfg(test)]
pub(in crate::native_artifact) fn without_frame_effects<T>(f: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) { EFFECTS_DISABLED.with(|disabled| disabled.set(self.0)); }
    }
    let _restore = Restore(EFFECTS_DISABLED.with(|disabled| disabled.replace(true)));
    f()
}
#[cfg(test)]
pub(in crate::native_artifact) fn take_frame_effect_emissions() -> (usize, usize, usize) {
    EMISSIONS.with(|counts| counts.replace((0, 0, 0)))
}
pub(in crate::native_artifact) fn effects_enabled() -> bool {
    #[cfg(test)]
    { !EFFECTS_DISABLED.with(Cell::get) }
    #[cfg(not(test))]
    { true }
}

pub(in crate::native_artifact) struct FrameState {
    pointer: Value,
    words: Option<Vec<Variable>>,
    effects_enabled: bool,
    dirty_only: bool,
    // Constant-size actual emission evidence, aggregated by private operation.
    // It does not claim that a runtime path executed or authorize any access.
    emissions: Cell<[u64; 14]>,
    pub(super) domain: Domain,
}
impl FrameState {
    pub(super) fn memory(pointer: Value, domain: Domain) -> Self {
        Self { pointer, words: None, effects_enabled: false, dirty_only: false, emissions: Cell::new([0; 14]), domain }
    }
    pub(in crate::native_artifact) fn registers(b: &mut FunctionBuilder<'_>, pointer: Value, domain: Domain) -> Self {
        assert_eq!(core::mem::size_of::<Frame>() % 8, 0);
        let words = (0..core::mem::size_of::<Frame>() / 8).map(|word| {
            let variable = b.declare_var(types::I64);
            let value = b.ins().load(types::I64, MemFlags::new(), pointer, (word * 8) as i32);
            b.def_var(variable, value);
            variable
        }).collect();
        let effects_enabled = effects_enabled();
        let dirty_only = super::super::packed_loop_codegen::invariant_loops_enabled();
        Self { pointer, words: Some(words), effects_enabled, dirty_only, emissions: Cell::new([0; 14]), domain }
    }
    pub(in crate::native_artifact) fn read(&self, b: &mut FunctionBuilder<'_>, offset: usize) -> Value {
        if let Some(words) = &self.words { b.use_var(words[offset / 8]) }
        else { b.ins().load(types::I64, MemFlags::new(), self.pointer, offset as i32) }
    }
    pub(in crate::native_artifact) fn write(&self, b: &mut FunctionBuilder<'_>, offset: usize, value: Value) {
        assert!(offset % 8 == 0 && offset / 8 < WORDS && GENERATED & (1 << (offset / 8)) != 0,
            "generated packed-frame write requires an explicit effect proof");
        if let Some(words) = &self.words { b.def_var(words[offset / 8], value); }
        else { b.ins().store(MemFlags::new(), value, self.pointer, offset as i32); }
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
                if mask & (1 << word) == 0 { continue; }
                let value = b.use_var(variable);
                b.ins().store(MemFlags::new(), value, self.pointer, (word * 8) as i32);
            }
        }
    }
    fn reload_mask(&self, b: &mut FunctionBuilder<'_>, mask: u64) {
        if let Some(words) = &self.words {
            for (word, &variable) in words.iter().enumerate() {
                if mask & (1 << word) == 0 { continue; }
                let value = b.ins().load(types::I64, MemFlags::new(), self.pointer, (word * 8) as i32);
                b.def_var(variable, value);
            }
        }
    }
    pub(in crate::native_artifact) fn call(&self, b: &mut FunctionBuilder<'_>, hooks: &Hooks,
        function: Value, context: Value, site: usize, operation: u64, args: [Value; 3], error: Block)
    {
        let selected = self.effects_enabled.then(|| effects(operation)).flatten();
        let (stores, loads) = selected.map_or((MUTABLE, MUTABLE), |effects| (effects.stores(), effects.writes));
        self.spill_mask(b, stores);
        hooks.bound_call(b, function, context, site, operation, args, error);
        // The helper status branch remains before every reload. Insertion is
        // failure-atomic; an error retains the same SSA values as before this
        // optimization and reaches the unchanged full terminal spill. Success
        // reloads ALL written words, including cap flags on non-growing inserts.
        self.reload_mask(b, loads);
        if selected.is_some() {
            let mut counts = self.emissions.get();
            counts[operation as usize] += 1;
            self.emissions.set(counts);
            #[cfg(test)]
            EMISSIONS.with(|counts| {
                let (calls, before, after) = counts.get();
                counts.set((calls + 1, before + stores.count_ones() as usize, after + loads.count_ones() as usize));
            });
        }
    }
    pub(in crate::native_artifact) fn effect_emissions(&self) -> Vec<(u64, u64, u64, u64)> {
        self.emissions.get().into_iter().enumerate().filter_map(|(op, count)| {
            if count == 0 { return None; }
            let effects = effects(op as u64).expect("only proved operations receive emission receipts");
            Some((op as u64, count, effects.stores(), effects.writes))
        }).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_codegen::ir::{AbiParam, Function, Opcode};
    use cranelift_frontend::FunctionBuilderContext;
    use cranelift_module::default_libcall_names;

    fn memory_instructions(function: &Function) -> (usize, usize) {
        let mut counts = (0, 0);
        for block in function.layout.blocks() {
            for inst in function.layout.block_insts(block) {
                match function.dfg.insts[inst].opcode() {
                    Opcode::Store => counts.0 += 1,
                    Opcode::Load => counts.1 += 1,
                    _ => {}
                }
            }
        }
        counts
    }

    #[test]
    fn actual_call_emission_keeps_unknown_ops_and_terminal_spills_conservative() {
        let module = cranelift_jit::JITModule::new(cranelift_jit::JITBuilder::with_flags(
            &[], default_libcall_names()).unwrap());
        for op in [0, 1, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, u64::MAX] {
            // All three paths use the real call emitter, including its status
            // branch. This test inspects IR; the end-to-end fixtures execute it.
            for mode in 0..3 {
                let mut function = Function::new();
                function.signature.params = vec![AbiParam::new(types::I64)];
                function.signature.returns = vec![AbiParam::new(types::I64)];
                let mut context = FunctionBuilderContext::new();
                let mut b = FunctionBuilder::new(&mut function, &mut context);
                let entry = b.create_block();
                b.append_block_params_for_function_params(entry);
                let error = b.create_block();
                b.append_block_param(error, types::I64);
                b.switch_to_block(entry);
                let pointer = b.block_params(entry)[0];
                let hooks = Hooks::load(true, &mut b, &module, pointer, pointer, error).unwrap().unwrap();
                let state = match mode {
                    0 => FrameState::registers(&mut b, pointer, Domain::ascii()),
                    1 => without_frame_effects(|| FrameState::registers(&mut b, pointer, Domain::ascii())),
                    _ => FrameState::memory(pointer, Domain::ascii()),
                };
                let zero = b.ins().iconst(types::I64, 0);
                let before = memory_instructions(b.func);
                state.call(&mut b, &hooks, pointer, pointer, 17, op, [zero; 3], error);
                let after = memory_instructions(b.func);
                let expected = match (mode, effects(op)) {
                    (0, Some(effect)) => (effect.stores().count_ones() as usize, effect.writes.count_ones() as usize),
                    (0 | 1, _) => (14, 14),
                    _ => (0, 0),
                };
                assert_eq!((after.0 - before.0, after.1 - before.1), expected, "op={op}, mode={mode}");
                assert_eq!(!state.effect_emissions().is_empty(), mode == 0 && effects(op).is_some());
                state.spill(&mut b);
                let final_spill = memory_instructions(b.func);
                assert_eq!((final_spill.0 - after.0, final_spill.1 - after.1),
                    (if mode == 2 { 0 } else { 7 }, 0), "all generated exit/error state synchronized");
                b.ins().return_(&[zero]);
                b.switch_to_block(error);
                let status = b.block_params(error)[0];
                b.ins().return_(&[status]);
                b.seal_all_blocks();
                b.finalize();
            }
        }
    }

    #[test]
    fn private_effect_masks_cover_validation_growth_and_all_generated_writes() {
        assert_eq!((MUTABLE.count_ones(), GENERATED.count_ones()), (14, 7));
        assert_eq!(IMMUTABLE & GENERATED, 0);
        let refill = effects(0).unwrap();
        assert_eq!((refill.stores().count_ones(), refill.writes.count_ones()), (2, 3));
        assert_eq!(refill.stores(), fields!(fuel, entry_known));
        assert_eq!(refill.writes, fields!(fuel, entry_known, entry_pointer));
        let insert = effects(7).unwrap();
        assert_eq!((insert.stores().count_ones(), insert.writes.count_ones()), (6, 9));
        assert_eq!(insert.stores(), GENERATED & !fields!(previous_start));
        assert_eq!(insert.writes, fields!(slots, controls, capacity, len, updates, entry_pointer,
            range_call_allowed, read_call_allowed, write_call_allowed));
        for op in [0, 7, 10, 11, 12] {
            let effect = effects(op).unwrap();
            assert_eq!(effect.reads & VALIDATION_READS, VALIDATION_READS);
            assert_eq!(effect.writes & IMMUTABLE, 0);
        }
        for op in [6, 8, 9] { assert_eq!(effects(op), Some(Effects { reads: 0, writes: 0 })); }
        for op in [1, 2, 3, 4, 5, 13, 14, u64::MAX] { assert_eq!(effects(op), None); }
    }
}
