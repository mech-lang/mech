//! Register carriers for one invocation-owned packed table. Only slow calls
//! expose the frame to Rust; every such boundary synchronizes mutable words.
use super::super::super::{program_hooks::Hooks, program_packed_map::Frame};
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, types};
use cranelift_frontend::{FunctionBuilder, Variable};

pub(in crate::native_artifact) struct FrameState {
    pointer: Value,
    words: Option<Vec<Variable>>,
}
impl FrameState {
    pub(super) fn memory(pointer: Value) -> Self { Self { pointer, words: None } }
    pub(in crate::native_artifact) fn registers(b: &mut FunctionBuilder<'_>, pointer: Value) -> Self {
        assert_eq!(core::mem::size_of::<Frame>() % 8, 0);
        let words = (0..core::mem::size_of::<Frame>() / 8).map(|word| {
            let variable = b.declare_var(types::I64);
            let value = b.ins().load(types::I64, MemFlags::new(), pointer, (word * 8) as i32);
            b.def_var(variable, value);
            variable
        }).collect();
        Self { pointer, words: Some(words) }
    }
    pub(in crate::native_artifact) fn read(&self, b: &mut FunctionBuilder<'_>, offset: usize) -> Value {
        if let Some(words) = &self.words { b.use_var(words[offset / 8]) }
        else { b.ins().load(types::I64, MemFlags::new(), self.pointer, offset as i32) }
    }
    pub(in crate::native_artifact) fn write(&self, b: &mut FunctionBuilder<'_>, offset: usize, value: Value) {
        if let Some(words) = &self.words { b.def_var(words[offset / 8], value); }
        else { b.ins().store(MemFlags::new(), value, self.pointer, offset as i32); }
    }
    // These fields are initialized once while the immutable source is pinned.
    // New frame fields are conservatively synchronized until proved invariant.
    fn immutable(offset: usize) -> bool {
        [core::mem::offset_of!(Frame, source_ptr), core::mem::offset_of!(Frame, source_len),
            core::mem::offset_of!(Frame, width), core::mem::offset_of!(Frame, low_mask),
            core::mem::offset_of!(Frame, high_mask)].contains(&offset)
    }
    pub(in crate::native_artifact) fn spill(&self, b: &mut FunctionBuilder<'_>) {
        if let Some(words) = &self.words {
            for (word, &variable) in words.iter().enumerate() {
                if Self::immutable(word * 8) { continue; }
                let value = b.use_var(variable);
                b.ins().store(MemFlags::new(), value, self.pointer, (word * 8) as i32);
            }
        }
    }
    fn reload(&self, b: &mut FunctionBuilder<'_>) {
        if let Some(words) = &self.words {
            for (word, &variable) in words.iter().enumerate() {
                if Self::immutable(word * 8) { continue; }
                let value = b.ins().load(types::I64, MemFlags::new(), self.pointer, (word * 8) as i32);
                b.def_var(variable, value);
            }
        }
    }
    pub(in crate::native_artifact) fn call(&self, b: &mut FunctionBuilder<'_>, hooks: &Hooks,
        function: Value, context: Value, site: usize, operation: u64, args: [Value; 3], error: Block)
    {
        self.spill(b);
        hooks.bound_call(b, function, context, site, operation, args, error);
        // Successful insertion can relocate slots, change cap flags and install
        // a cached entry. Refill changes fuel. Never retain pre-call versions.
        self.reload(b);
    }
}
