use crate::{ApplicationRequirement, EncodedConstant, LegacyValue, MResult, ValueKind};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, vec::Vec};

use super::{CompileConst, Register};

/// Canonical producer identity used when assigning bytecode registers.
///
/// Reactive cells retain their producer identity, typed views additionally
/// retain every annotation layer, and non-reactive values use the caller's
/// stable object identity. Mutable references are transparent.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum BytecodeRegisterIdentity {
    Cell(usize),
    Typed {
        inner: Box<BytecodeRegisterIdentity>,
        annotation: ValueKind,
    },
    Ephemeral(usize),
}

impl BytecodeRegisterIdentity {
    fn underlying_address(&self) -> usize {
        match self {
            Self::Cell(address) | Self::Ephemeral(address) => *address,
            Self::Typed { inner, .. } => inner.underlying_address(),
        }
    }
}

pub fn bytecode_register_identity(
    value: &LegacyValue,
    fallback: usize,
) -> BytecodeRegisterIdentity {
    match value {
        LegacyValue::MutableReference(reference) => {
            bytecode_register_identity(&reference.borrow(), reference.addr())
        }
        LegacyValue::Typed(value, annotation) => BytecodeRegisterIdentity::Typed {
            inner: Box::new(bytecode_register_identity(value, fallback)),
            annotation: annotation.clone(),
        },
        _ => value
            .reactive_root_cell_ids()
            .first()
            .map(|cell| BytecodeRegisterIdentity::Cell(cell.get() as usize))
            .unwrap_or(BytecodeRegisterIdentity::Ephemeral(fallback)),
    }
}

pub trait BytecodeCompilerContext {
    fn register_for_ptr_with_initialization_status(&mut self, pointer: usize) -> (Register, bool);

    /// Assign a register to a canonical logical value identity. Contexts that
    /// do not emit a bytecode register graph may retain pointer-only behavior;
    /// bytecode-producing contexts override this method with the complete key.
    fn register_for_identity_with_initialization_status(
        &mut self,
        identity: &BytecodeRegisterIdentity,
    ) -> (Register, bool) {
        self.register_for_ptr_with_initialization_status(identity.underlying_address())
    }

    fn register_for_value_with_initialization_status(
        &mut self,
        value: &LegacyValue,
        fallback: usize,
    ) -> (Register, bool) {
        self.register_for_identity_with_initialization_status(&bytecode_register_identity(
            value, fallback,
        ))
    }

    /// Records the exact semantic kind carried by a bytecode register.
    /// Contexts that do not build semantic compilation metadata may ignore it.
    fn record_register_kind(&mut self, _register: Register, _kind: ValueKind) -> MResult<()> {
        Ok(())
    }

    /// Records semantic metadata that is available only after a register's
    /// exact constant or composite template has been encoded.
    fn record_register_constant_metadata(
        &mut self,
        _register: Register,
        _constant: u32,
    ) -> MResult<()> {
        Ok(())
    }

    /// Preserve the semantic wrapper carried by a value while the compiler
    /// follows its legacy reference identity to the underlying register.
    /// Contexts that do not build semantic compilation metadata may ignore it.
    fn override_next_register_kind(&mut self, _kind: ValueKind) -> MResult<()> {
        Ok(())
    }

    fn record_register_constant_kind(
        &mut self,
        _register: Register,
        _constant: u32,
    ) -> MResult<()> {
        Ok(())
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

    fn emit_composite_pack(
        &mut self,
        destination: Register,
        template: u32,
        children: Vec<Register>,
    );

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

/// Resolves and, when necessary, initializes the register for one logical
/// value. Composite values are reconstructed from child registers so their
/// reactive topology is never replaced by a planning-time constant snapshot.
pub fn compile_value_register(
    value: &LegacyValue,
    fallback: usize,
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    if let LegacyValue::MutableReference(reference) = value {
        return compile_value_register(&reference.borrow(), reference.addr(), {
            context.override_next_register_kind(value.kind())?;
            context
        });
    }

    let (register, initialize) =
        context.register_for_value_with_initialization_status(value, fallback);
    context.record_register_kind(register, value.kind())?;
    if !initialize {
        return Ok(register);
    }

    if let Some(children) = crate::bytecode_composite_children(value) {
        let children = children
            .iter()
            .map(|child| compile_value_register(child, core::ptr::from_ref(child).addr(), context))
            .collect::<MResult<Vec<_>>>()?;
        let template = value.compile_const(context)?;
        context.record_register_constant_metadata(register, template)?;
        context.emit_composite_pack(register, template, children);
    } else {
        let constant = value.compile_const(context)?;
        context.record_register_constant_metadata(register, constant)?;
        context.emit_const_load(register, constant);
    }
    Ok(register)
}
