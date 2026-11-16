//! Compiler-local dirty masks for physical scalar homes. A cleared bit means
//! these exact homes were zeroed since the last resumable helper. The mask is
//! never serialized, passed to the runtime, or used as execution authority.
use crate::native_artifact::NativeCompileError;
use cranelift_codegen::ir::{InstBuilder, types};
use cranelift_frontend::{FunctionBuilder, Variable};
use std::collections::BTreeMap;

struct Mask {
    variable: Variable,
    initial: u64,
}

pub(super) struct ScalarHomes {
    masks: Vec<Mask>,
    nodes: BTreeMap<usize, (usize, u64)>,
}

impl ScalarHomes {
    pub(super) fn new(b: &mut FunctionBuilder<'_>, nodes: impl IntoIterator<Item = usize>) -> Self {
        let nodes = nodes
            .into_iter()
            .enumerate()
            .map(|(ordinal, node)| (node, (ordinal / 64, 1u64 << (ordinal % 64))))
            .collect::<BTreeMap<_, _>>();
        let mut initial = Vec::<u64>::new();
        for &(word, bit) in nodes.values() {
            if word >= initial.len() {
                initial.resize(word + 1, 0);
            }
            initial[word] |= bit;
        }
        let masks = initial
            .into_iter()
            .map(|initial| {
                let variable = b.declare_var(types::I64);
                // Entry is conservatively dirty, including reentry after an
                // ordinary continuation. No zero-home admission assumption.
                let value = b.ins().iconst(types::I64, initial as i64);
                b.def_var(variable, value);
                Mask { variable, initial }
            })
            .collect();
        Self { masks, nodes }
    }

    pub(super) fn word_count(&self) -> usize {
        self.masks.len()
    }

    /// A first change may reify active carriers and refresh the bank pointer.
    /// Conservatively forget every clean bit, including inactive siblings and
    /// any physical aliases, rather than relying on callback implementation.
    pub(super) fn materialized(&self, b: &mut FunctionBuilder<'_>) {
        for mask in &self.masks {
            let value = b.ins().iconst(types::I64, mask.initial as i64);
            b.def_var(mask.variable, value);
        }
    }

    pub(super) fn retire(
        &self,
        b: &mut FunctionBuilder<'_>,
        node: usize,
        clear: impl FnOnce(&mut FunctionBuilder<'_>) -> Result<(), NativeCompileError>,
    ) -> Result<(), NativeCompileError> {
        let Some(&(word, bit)) = self.nodes.get(&node) else {
            return clear(b);
        };
        let mask = &self.masks[word];
        let before = b.use_var(mask.variable);
        let pending = b.ins().band_imm(before, bit as i64);
        let dirty = b.create_block();
        let done = b.create_block();
        b.set_cold_block(dirty);
        b.ins().brif(pending, dirty, &[], done, &[]);
        b.switch_to_block(dirty);
        clear(b)?;
        let after = b.ins().band_imm(before, (!bit) as i64);
        b.def_var(mask.variable, after);
        b.ins().jump(done, &[]);
        b.switch_to_block(done);
        Ok(())
    }
}

#[cfg(test)]
#[path = "numeric_retirement_tests.rs"]
mod tests;
