use crate::{ApplicationRequirement, EncodedConstant, MResult, ValueKind};

#[cfg(feature = "no_std")]
use alloc::vec::Vec;

use super::Register;

pub trait BytecodeCompilerContext {
    fn register_for_ptr_with_initialization_status(&mut self, pointer: usize) -> (Register, bool);

    /// Resolve a typed view of a reactive cell without collapsing it onto the
    /// unannotated cell. Compiler contexts that only capture constants may use
    /// the underlying pointer identity; bytecode-producing contexts override
    /// this to include the annotation in their register key.
    fn register_for_typed_ptr_with_initialization_status(
        &mut self,
        pointer: usize,
        _annotation: &ValueKind,
    ) -> (Register, bool) {
        self.register_for_ptr_with_initialization_status(pointer)
    }

    fn intern_constant(&mut self, constant: EncodedConstant) -> MResult<u32>;

    fn define_symbol(
        &mut self,
        pointer: usize,
        register: Register,
        name: &str,
        mutable: bool,
    ) -> MResult<()>;

    fn intern_requirement(&mut self, requirement: ApplicationRequirement) -> MResult<u32>;

    fn emit_const_load(&mut self, destination: Register, constant: u32);

    fn emit_nullop(&mut self, function: u64, destination: Register);

    fn emit_unop(&mut self, function: u64, destination: Register, source: Register);

    fn emit_binop(&mut self, function: u64, destination: Register, lhs: Register, rhs: Register);

    fn emit_ternop(
        &mut self,
        function: u64,
        destination: Register,
        a: Register,
        b: Register,
        c: Register,
    );

    fn emit_quadop(
        &mut self,
        function: u64,
        destination: Register,
        a: Register,
        b: Register,
        c: Register,
        d: Register,
    );

    fn emit_varop(&mut self, function: u64, destination: Register, arguments: Vec<Register>);

    fn emit_host_call(&mut self, requirement: u32, destination: Register, arguments: Vec<Register>);

    fn emit_resource_read(&mut self, requirement: u32, destination: Register);

    fn emit_resource_write(&mut self, requirement: u32, destination: Register, source: Register);

    fn emit_resource_send(&mut self, requirement: u32, destination: Register, source: Register);
}
