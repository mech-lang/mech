use crate::{
    ApplicationRequirement, EncodedConstant, LegacyValue, MResult, MechError, MechErrorKind,
    ValueCell, ValueKind,
};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, vec::Vec};

use super::{CompileConst, Register, compile_annotated_constant};

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
    fn function_id(&mut self, canonical_name: &str) -> MResult<u64> {
        Ok(crate::hash_str(canonical_name))
    }

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

    /// Declares that an executable instruction, rather than a planning-time
    /// snapshot, is authoritative for this register's first value.
    ///
    /// A compiler may encounter a downstream declaration before the external
    /// producer that owns the same logical cell. Bytecode-producing contexts
    /// use this hook to discard that provisional initializer before emitting
    /// the producer instruction.
    fn record_runtime_produced_register(&mut self, _register: Register) -> MResult<()> {
        Ok(())
    }

    /// Records the source declaration's value before the first register turn.
    /// This remains distinct from the register's executable `ConstLoad`, whose
    /// current value may already reflect elaboration-time execution.
    fn record_state_initializer(&mut self, _register: Register, _constant: u32) -> MResult<()> {
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

    /// Records a declaration that is executable state but is not part of the
    /// artifact's public/root symbol namespace.
    fn define_local_symbol(
        &mut self,
        pointer: usize,
        register: Register,
        name: &str,
        mutable: bool,
    ) -> MResult<()> {
        self.define_symbol(pointer, register, name, mutable)
    }

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

    fn emit_declaration_binary(
        &mut self,
        function: u64,
        destination: Register,
        first: Register,
        second: Register,
    ) {
        self.emit_binop(function, destination, first, second);
    }

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

/// A compiler could not inspect the payload of an explicit value cell because
/// another owner held an incompatible borrow.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueCellCompilerBorrowConflict {
    pub phase: &'static str,
}

impl MechErrorKind for ValueCellCompilerBorrowConflict {
    fn name(&self) -> &str {
        "ValueCellCompilerBorrowConflict"
    }

    fn message(&self) -> String {
        format!(
            "value cell payload is already mutably borrowed during {}",
            self.phase,
        )
    }
}

fn value_cell_borrow_conflict(phase: &'static str) -> MechError {
    MechError::new(ValueCellCompilerBorrowConflict { phase }, None).with_compiler_loc()
}

/// Resolves and, when necessary, initializes the register owned by `cell`.
///
/// The cell is the outer register owner even when its current payload is a
/// reactive composite. Composite children continue to use their canonical
/// value identities so their live topology is retained.
pub fn compile_value_cell_register(
    cell: &ValueCell,
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    let reference = cell.legacy_ref();
    let value = cell
        .try_borrow()
        .map_err(|_| value_cell_borrow_conflict("value-cell register compilation"))?;
    compile_value_register_for_ptr(&value, reference.addr(), context)
}

/// Recovers the explicit cell carried by a legacy compiler ABI value.
///
/// This is a normalization bridge for compiler planning. Ordinary compiler
/// ownership should remain explicit after this boundary.
#[doc(hidden)]
pub fn compiler_value_cell_from_legacy(value: &LegacyValue) -> Option<ValueCell> {
    value
        .exact_ref_any()?
        .downcast_ref::<crate::Ref<LegacyValue>>()
        .cloned()
        .map(ValueCell::from_legacy_ref)
}

fn compile_annotation_layers(
    value: &LegacyValue,
    mut identity: BytecodeRegisterIdentity,
    mut register: Register,
    annotations: &[ValueKind],
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    for index in (0..annotations.len()).rev() {
        let annotation = &annotations[index];
        identity = BytecodeRegisterIdentity::Typed {
            inner: Box::new(identity),
            annotation: annotation.clone(),
        };
        let (typed_register, initialize) =
            context.register_for_identity_with_initialization_status(&identity);
        context.record_register_kind(typed_register, annotation.clone())?;
        if initialize {
            let template = compile_annotated_constant(value, &annotations[index..], context)?;
            context.record_register_constant_metadata(typed_register, template)?;
            context.emit_composite_pack(typed_register, template, vec![register]);
        }
        register = typed_register;
    }
    Ok(register)
}

/// Compiles a compiler-only typed view whose underlying identity is `cell`.
///
/// Annotation layers are ordered from outermost to innermost. This bridge is
/// intentionally hidden from ordinary API documentation; it exists so source
/// planning can preserve typed views without recreating MutableReference.
#[doc(hidden)]
pub fn compile_annotated_value_cell_register(
    cell: &ValueCell,
    annotations: &[ValueKind],
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    let reference = cell.legacy_ref();
    let value = cell
        .try_borrow()
        .map_err(|_| value_cell_borrow_conflict("annotated value-cell register compilation"))?
        .clone();
    let identity = BytecodeRegisterIdentity::Cell(reference.addr());
    let register = compile_value_register_for_ptr(&value, reference.addr(), context)?;
    compile_annotation_layers(&value, identity, register, annotations, context)
}

/// Compiles typed views of an immediate legacy value without reconstructing
/// legacy wrappers in engine planning.
#[doc(hidden)]
pub fn compile_annotated_value_register(
    value: &LegacyValue,
    annotations: &[ValueKind],
    fallback: usize,
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    let identity = bytecode_register_identity(value, fallback);
    let register = compile_value_register(value, fallback, context)?;
    compile_annotation_layers(value, identity, register, annotations, context)
}

/// Resolves the register owned by a cell whose payload will be supplied by an
/// executable instruction.
///
/// The current payload contributes semantic kind information only. No
/// planning-time `ConstLoad` or `CompositePack` is emitted, and a provisional
/// initializer for the same cell is discarded.
pub fn compile_runtime_produced_value_cell_register(
    cell: &ValueCell,
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    let reference = cell.legacy_ref();
    let value = cell
        .try_borrow()
        .map_err(|_| value_cell_borrow_conflict("runtime-produced value-cell compilation"))?;
    let (register, _) = context.register_for_ptr_with_initialization_status(reference.addr());
    context.record_register_kind(register, value.kind())?;
    context.record_runtime_produced_register(register)?;
    Ok(register)
}

/// Resolves and, when necessary, initializes the register for one logical
/// value. Composite values are reconstructed from child registers so their
/// reactive topology is never replaced by a planning-time constant snapshot.
pub fn compile_value_register(
    value: &LegacyValue,
    fallback: usize,
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    // MutableReference remains a supported legacy compiler ABI. New compiler
    // ownership paths should retain ValueCell and use
    // `compile_value_cell_register` instead of manufacturing this wrapper.
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

/// Resolves a value into the register owned by an explicit outer cell.
///
/// Generic declaration cells need to retain their own identity even when the
/// value inside the cell is a reactive composite. The composite's children
/// are still compiled by their canonical identities so `CompositePack`
/// retains the live topology instead of freezing the planning-time payload.
pub fn compile_value_register_for_ptr(
    value: &LegacyValue,
    pointer: usize,
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    let (register, initialize) = context.register_for_ptr_with_initialization_status(pointer);
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

/// Resolves the register for a value whose payload will be produced by an
/// executable instruction.
///
/// Unlike `compile_value_register`, this records the register's semantic kind
/// but deliberately emits no `ConstLoad` or `CompositePack`. The current value
/// may be used to establish compile-time schema/type information; its payload
/// is not part of the compiled program.
pub fn compile_runtime_produced_register(
    value: &LegacyValue,
    fallback: usize,
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    // MutableReference remains a supported legacy compiler ABI. New external
    // producers should call `compile_runtime_produced_value_cell_register`.
    if let LegacyValue::MutableReference(reference) = value {
        context.override_next_register_kind(value.kind())?;
        return compile_runtime_produced_register(&reference.borrow(), reference.addr(), context);
    }

    let (register, _) = context.register_for_value_with_initialization_status(value, fallback);
    context.record_register_kind(register, value.kind())?;
    context.record_runtime_produced_register(register)?;
    Ok(register)
}
