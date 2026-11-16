//! Stage the invariant part of an already admitted affine packed read.
use super::*;
use crate::native_artifact::program::codegen::traversal_facts::CapturedWindow;
use cranelift_codegen::ir::{Value, types};
use cranelift_frontend::Variable;

pub(in crate::native_artifact::program::codegen) struct ReadAddress {
    pub site: usize,
    pub array: usize,
    pub base: usize,
    pub induction: usize,
    home: Variable,
}

impl ReadAddress {
    pub fn new(b: &mut FunctionBuilder<'_>, site: usize, window: CapturedWindow) -> Self {
        Self {
            site,
            array: window.array,
            base: window.base,
            induction: window.induction,
            home: b.declare_var(types::I64),
        }
    }

    pub fn stage(&self, b: &mut FunctionBuilder<'_>, words: &Words, shadow: usize, proof: &Proof) {
        let pointer = words.read(b, shadow + proof.view(self.array));
        let base = words.read(b, self.base);
        let address = b.ins().iadd(pointer, base);
        b.def_var(self.home, address);
    }

    pub fn emit(&self, b: &mut FunctionBuilder<'_>, words: &Words) -> Value {
        let base = b.use_var(self.home);
        let at = words.read(b, self.induction);
        b.ins().iadd(base, at)
    }

    pub fn report(&self) -> (usize, usize, usize, usize) {
        (self.site, self.array, self.base, self.induction)
    }
}
