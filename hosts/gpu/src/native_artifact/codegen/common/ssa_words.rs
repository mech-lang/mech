//! Shared SSA homes for admitted native scalar and aggregate word lanes.
//! Only explicit boundary synchronization touches the original local array.
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::codegen::control::plan::CodePlan;
use crate::native_artifact::compiler::offset;
use cranelift_codegen::ir::{InstBuilder, MemFlags, Value, types};
use cranelift_frontend::{FunctionBuilder, Variable};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(feature = "canonical-native-program")]
#[path = "ssa_direct_homes.rs"]
mod direct_homes;

#[derive(Clone, Copy)]
enum Home {
    Variable(Variable),
    Literal(Value),
    #[cfg(feature = "canonical-native-program")]
    Tracked(u32),
    #[cfg(feature = "early-region-emission")]
    Packet {
        address: Value,
        offset: i32,
    },
}

pub(in crate::native_artifact) struct Words {
    variables: BTreeMap<usize, (Home, MemFlags)>,
    literal_write: Cell<bool>,
    #[cfg(feature = "canonical-native-program")]
    tracked: RefCell<Vec<direct_homes::Tracked>>,
    #[cfg(feature = "canonical-native-program")]
    promote_tracked: bool,
    native_extent_slots: BTreeMap<usize, u64>,
    native_extent_values: RefCell<BTreeMap<Value, u64>>,
}

#[cfg(all(test, feature = "early-region-emission", feature = "jit"))]
#[path = "ssa_packet_homes_tests.rs"]
mod packet_tests;
#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "ssa_words_tests.rs"]
mod tests;
impl Words {
    /// Bind the exact initialized in/out ownership lanes certified by an early
    /// unit boundary. Its caller-owned packet is aligned, invocation-local and
    /// nonescaping. It is disjoint from the unit's ordinary output/call storage.
    #[cfg(feature = "early-region-emission")]
    pub(in crate::native_artifact) fn bind_packet_homes(
        &mut self,
        address: Value,
        homes: &BTreeMap<usize, usize>,
    ) -> Result<(), NativeCompileError> {
        for (&slot, &position) in homes {
            let Some((home @ Home::Variable(_), _)) = self.variables.get_mut(&slot) else {
                return Err(crate::native_artifact::compiler::backend(
                    "packet ownership home needs an ordinary source lane",
                ));
            };
            *home = Home::Packet {
                address,
                offset: offset(position)?,
            };
        }
        Ok(())
    }
    pub(in crate::native_artifact) fn new(
        b: &mut FunctionBuilder<'_>,
        locals: Value,
        code: CodePlan<'_>,
        slots: BTreeSet<usize>,
    ) -> Result<Self, NativeCompileError> {
        Self::new_with_regions(b, locals, code, slots, |_| MemFlags::new())
    }
    /// Initialize private SSA inputs without first publishing them to source
    /// homes. Ordinary source slots keep the same constant/local initialization.
    pub(in crate::native_artifact) fn new_with(
        b: &mut FunctionBuilder<'_>,
        locals: Value,
        code: CodePlan<'_>,
        slots: BTreeSet<usize>,
        initial: impl FnMut(
            &mut FunctionBuilder<'_>,
            usize,
        ) -> Result<Option<Value>, NativeCompileError>,
    ) -> Result<Self, NativeCompileError> {
        Self::build(b, locals, code, slots, initial, |_| MemFlags::new(), None)
    }
    /// A physical home keeps one alias region across entry, spills, and reloads.
    /// Tail/borrowed synchronization accesses these same private homes directly.
    pub(in crate::native_artifact) fn new_with_regions(
        b: &mut FunctionBuilder<'_>,
        locals: Value,
        code: CodePlan<'_>,
        slots: BTreeSet<usize>,
        flags: impl FnMut(usize) -> MemFlags,
    ) -> Result<Self, NativeCompileError> {
        Self::build(b, locals, code, slots, |_, _| Ok(None), flags, None)
    }
    pub(in crate::native_artifact) fn new_with_initial_regions(
        b: &mut FunctionBuilder<'_>,
        locals: Value,
        code: CodePlan<'_>,
        slots: BTreeSet<usize>,
        initial: impl FnMut(
            &mut FunctionBuilder<'_>,
            usize,
        ) -> Result<Option<Value>, NativeCompileError>,
        flags: impl FnMut(usize) -> MemFlags,
    ) -> Result<Self, NativeCompileError> {
        Self::build(b, locals, code, slots, initial, flags, None)
    }
    /// Current DEFLATE entries and selected related entries share one builder.
    #[cfg(feature = "canonical-native-program")]
    pub(in crate::native_artifact) fn new_with_regions_and_initial(
        b: &mut FunctionBuilder<'_>,
        locals: Value,
        code: CodePlan<'_>,
        slots: BTreeSet<usize>,
        initial: impl FnMut(
            &mut FunctionBuilder<'_>,
            usize,
        ) -> Result<Option<Value>, NativeCompileError>,
        flags: impl FnMut(usize) -> MemFlags,
    ) -> Result<Self, NativeCompileError> {
        Self::build(b, locals, code, slots, initial, flags, None)
    }
    /// Seed selected immutable literal lanes in the dominating entry block.
    /// The enclosing compiler discards this attempt if lowering writes any
    /// selected lane, or if the usual IR checks reject the result.
    #[cfg(feature = "canonical-native-program")]
    pub(in crate::native_artifact) fn new_with_entry_literals(
        b: &mut FunctionBuilder<'_>,
        locals: Value,
        code: CodePlan<'_>,
        slots: BTreeSet<usize>,
        initial: impl FnMut(
            &mut FunctionBuilder<'_>,
            usize,
        ) -> Result<Option<Value>, NativeCompileError>,
        literals: &BTreeSet<usize>,
    ) -> Result<Self, NativeCompileError> {
        Self::build(
            b,
            locals,
            code,
            slots,
            initial,
            |_| MemFlags::new(),
            Some(literals),
        )
    }
    #[cfg(feature = "canonical-native-program")]
    pub(in crate::native_artifact) fn attempted_literal_write(&self) -> bool {
        self.literal_write.get()
    }

    fn build(
        b: &mut FunctionBuilder<'_>,
        locals: Value,
        code: CodePlan<'_>,
        slots: BTreeSet<usize>,
        mut initial: impl FnMut(
            &mut FunctionBuilder<'_>,
            usize,
        ) -> Result<Option<Value>, NativeCompileError>,
        mut flags: impl FnMut(usize) -> MemFlags,
        literals: Option<&BTreeSet<usize>>,
    ) -> Result<Self, NativeCompileError> {
        let constants = (0..code.constant_count())
            .map(|index| code.constant(index))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .filter_map(|(value, word)| word.map(|word| (value.get() as usize, word)))
            .collect::<BTreeMap<_, _>>();
        let mut variables = BTreeMap::new();
        for slot in slots {
            let is_literal = literals.is_some_and(|literals| literals.contains(&slot));
            let variable = (!is_literal).then(|| b.declare_var(types::I64));
            let flags = flags(slot);
            let value = if let Some(value) = initial(b, slot)? {
                value
            } else if let Some(&word) = constants.get(&slot) {
                b.ins().iconst(types::I64, word as i64)
            } else {
                b.ins().load(types::I64, flags, locals, offset(slot)?)
            };
            let home = if let Some(variable) = variable {
                b.def_var(variable, value);
                Home::Variable(variable)
            } else {
                Home::Literal(value)
            };
            variables.insert(slot, (home, flags));
        }
        Ok(Self {
            variables,
            literal_write: Cell::new(false),
            #[cfg(feature = "canonical-native-program")]
            tracked: RefCell::new(Vec::new()),
            #[cfg(feature = "canonical-native-program")]
            promote_tracked: false,
            native_extent_slots: BTreeMap::new(),
            native_extent_values: RefCell::new(BTreeMap::new()),
        })
    }
    pub(in crate::native_artifact) fn read(
        &self,
        b: &mut FunctionBuilder<'_>,
        slot: usize,
    ) -> Value {
        let value = match self.variables[&slot].0 {
            Home::Variable(variable) => b.use_var(variable),
            Home::Literal(value) => value,
            #[cfg(feature = "canonical-native-program")]
            Home::Tracked(index) => {
                self.tracked.borrow_mut()[index as usize].read(b, self.promote_tracked)
            }
            #[cfg(feature = "early-region-emission")]
            Home::Packet { address, offset } => b.ins().load(
                types::I64,
                MemFlags::new().with_aligned().with_notrap(),
                address,
                offset,
            ),
        };
        if let Some(&upper) = self.native_extent_slots.get(&slot) {
            self.native_extent_values
                .borrow_mut()
                .entry(value)
                .and_modify(|old| *old = (*old).min(upper))
                .or_insert(upper);
        }
        value
    }
    /// Only the numeric emitter's genuine packed slice length/capacity lanes
    /// qualify. Shape axes and arbitrary words loaded from arrays do not.
    #[cfg(feature = "canonical-native-program")]
    pub(in crate::native_artifact) fn bound_native_extent(&mut self, slot: usize, upper: u64) {
        assert!(self.variables.contains_key(&slot));
        self.native_extent_slots.insert(slot, upper);
    }
    #[cfg(feature = "canonical-native-program")]
    pub(in crate::native_artifact) fn native_extent_bounds(&self) -> Vec<(Value, u64)> {
        self.native_extent_values
            .borrow()
            .iter()
            .map(|(&v, &upper)| (v, upper))
            .collect()
    }
    pub(in crate::native_artifact) fn write(
        &self,
        b: &mut FunctionBuilder<'_>,
        slot: usize,
        value: Value,
    ) {
        match self.variables[&slot].0 {
            Home::Variable(variable) => b.def_var(variable, value),
            Home::Literal(_) => self.literal_write.set(true),
            #[cfg(feature = "canonical-native-program")]
            Home::Tracked(index) => self.tracked.borrow_mut()[index as usize].write(b, value),
            #[cfg(feature = "early-region-emission")]
            Home::Packet { address, offset } => {
                b.ins().store(
                    MemFlags::new().with_aligned().with_notrap(),
                    value,
                    address,
                    offset,
                );
            }
        }
    }
    pub(in crate::native_artifact) fn spill_all(
        &self,
        b: &mut FunctionBuilder<'_>,
        locals: Value,
    ) -> Result<(), NativeCompileError> {
        self.spill_live(b, locals, &self.variables.keys().copied().collect())
    }
    pub(in crate::native_artifact) fn spill_live(
        &self,
        b: &mut FunctionBuilder<'_>,
        locals: Value,
        live: &BTreeSet<usize>,
    ) -> Result<(), NativeCompileError> {
        for &slot in live {
            let value = self.read(b, slot);
            b.ins()
                .store(self.variables[&slot].1, value, locals, offset(slot)?);
        }
        Ok(())
    }
    pub(in crate::native_artifact) fn reload_live(
        &self,
        b: &mut FunctionBuilder<'_>,
        locals: Value,
        live: &BTreeSet<usize>,
    ) -> Result<(), NativeCompileError> {
        for &slot in live {
            let value = b
                .ins()
                .load(types::I64, self.variables[&slot].1, locals, offset(slot)?);
            self.write(b, slot, value);
        }
        Ok(())
    }
}
