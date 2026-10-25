//! Shared SSA homes for admitted native scalar and aggregate word lanes.
//! Only explicit boundary synchronization touches the original local array.
use super::super::{NativeCompileError, compiler::offset, iteration_code_plan::CodePlan};
use cranelift_codegen::ir::{InstBuilder, MemFlags, Value, types};
use cranelift_frontend::{FunctionBuilder, Variable};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Words {
    variables: BTreeMap<usize, Variable>,
}
impl Words {
    pub(super) fn new(
        b: &mut FunctionBuilder<'_>,
        locals: Value,
        code: CodePlan<'_>,
        slots: BTreeSet<usize>,
    ) -> Result<Self, NativeCompileError> {
        let constants = (0..code.constant_count())
            .map(|index| code.constant(index))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .filter_map(|(value, word)| word.map(|word| (value.get() as usize, word)))
            .collect::<BTreeMap<_, _>>();
        let mut variables = BTreeMap::new();
        for slot in slots {
            let variable = b.declare_var(types::I64);
            let value = if let Some(&word) = constants.get(&slot) {
                b.ins().iconst(types::I64, word as i64)
            } else {
                b.ins()
                    .load(types::I64, MemFlags::new(), locals, offset(slot)?)
            };
            b.def_var(variable, value);
            variables.insert(slot, variable);
        }
        Ok(Self { variables })
    }
    pub(super) fn read(&self, b: &mut FunctionBuilder<'_>, slot: usize) -> Value {
        b.use_var(self.variables[&slot])
    }
    pub(super) fn write(&self, b: &mut FunctionBuilder<'_>, slot: usize, value: Value) {
        b.def_var(self.variables[&slot], value);
    }
    pub(super) fn spill_all(
        &self,
        b: &mut FunctionBuilder<'_>,
        locals: Value,
    ) -> Result<(), NativeCompileError> {
        self.spill_live(b, locals, &self.variables.keys().copied().collect())
    }
    pub(super) fn spill_live(
        &self,
        b: &mut FunctionBuilder<'_>,
        locals: Value,
        live: &BTreeSet<usize>,
    ) -> Result<(), NativeCompileError> {
        for &slot in live {
            let value = self.read(b, slot);
            b.ins().store(MemFlags::new(), value, locals, offset(slot)?);
        }
        Ok(())
    }
    pub(super) fn reload_live(
        &self,
        b: &mut FunctionBuilder<'_>,
        locals: Value,
        live: &BTreeSet<usize>,
    ) -> Result<(), NativeCompileError> {
        for &slot in live {
            let value = b
                .ins()
                .load(types::I64, MemFlags::new(), locals, offset(slot)?);
            self.write(b, slot, value);
        }
        Ok(())
    }
}
