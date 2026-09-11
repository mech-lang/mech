use crate::BytecodeValidationError;
use crate::{
    ApplicationRequirement, EncodedConstant, MResult, MechError, MechErrorKind, ValueCell,
};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, vec::Vec};

#[cfg(feature = "matrix")]
use super::CompiledMatrixLiteralElement;
use super::{CompiledMatrixLiteral, Register};

/// Canonical producer identity used when assigning bytecode registers.
///
/// Reactive cells retain their producer identity, typed views additionally
/// retain every annotation layer, and non-reactive values use the caller's
/// stable object identity. Mutable references are transparent.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum BytecodeRegisterIdentity {
    Cell(usize),
    /// Compiler-control absence is a register value in its own right. It is
    /// distinct from canonical unit and from `Option(None)` and therefore
    /// must not be reconstructed through an empty-value convention.
    Absent,
    Typed {
        inner: Box<BytecodeRegisterIdentity>,
        annotation: crate::SchemaKey,
    },
    Ephemeral(usize),
}

impl BytecodeRegisterIdentity {
    fn underlying_address(&self) -> usize {
        match self {
            Self::Cell(address) | Self::Ephemeral(address) => *address,
            Self::Absent => usize::MAX,
            Self::Typed { inner, .. } => inner.underlying_address(),
        }
    }
}

pub trait BytecodeCompilerContext {
    fn function_id(&mut self, canonical_name: &str) -> MResult<u64> {
        Ok(crate::hash_str(canonical_name))
    }

    fn register_for_ptr_with_initialization_status(&mut self, pointer: usize) -> (Register, bool);

    /// Retains a canonical cell for the lifetime of this compilation so its
    /// process-local storage identity cannot be recycled for another cell.
    fn retain_canonical_cell(&mut self, _cell: &ValueCell) -> MResult<()> {
        Ok(())
    }

    /// Assign a register to a canonical logical value identity. Contexts that
    /// do not emit a bytecode register graph may retain pointer-only behavior;
    /// bytecode-producing contexts override this method with the complete key.
    fn register_for_identity_with_initialization_status(
        &mut self,
        identity: &BytecodeRegisterIdentity,
    ) -> (Register, bool) {
        self.register_for_ptr_with_initialization_status(identity.underlying_address())
    }

    /// Records the canonical schema carried by a register owned by a
    /// [`ValueCell`]. Artifact construction prefers this authority over the
    /// lossy runtime-type sidecar.
    fn record_register_schema(
        &mut self,
        _register: Register,
        _schema: crate::SchemaBody,
    ) -> MResult<()> {
        Ok(())
    }

    /// Records the complete semantic certificate carried by a canonical
    /// register. Bytecode bytes remain unchanged; artifact construction uses
    /// this dense sidecar to prevent schema-only reinterpretation.
    fn record_register_type_descriptor(
        &mut self,
        _register: Register,
        _descriptor: crate::ResolvedValueDescriptor,
    ) -> MResult<()> {
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

    /// Marks compiler-control absence. This is not canonical unit and has no
    /// schema, but remains a real bytecode register for source construction.
    fn record_absent_register(&mut self, _register: Register) -> MResult<()> {
        Ok(())
    }

    fn record_register_constant_schema(
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

    /// Reports whether an executable producer already owns this register.
    /// Such values may be observed through the compiler API without being
    /// reclassified as source matrix-literal construction.
    fn register_is_runtime_produced(&self, _register: Register) -> bool {
        false
    }

    /// Records deterministic, in-memory construction metadata for a generic
    /// matrix literal. Contexts that do not build semantic artifacts may
    /// ignore it.
    fn record_matrix_literal(&mut self, _literal: CompiledMatrixLiteral) -> MResult<()> {
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

fn record_resolved_register(
    cell: &ValueCell,
    register: Register,
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<()> {
    let descriptor = cell.resolved_descriptor()?;
    context.record_register_type_descriptor(register, descriptor.clone())?;
    context.record_register_schema(register, descriptor.schema().body().clone())
}

fn value_cell_register_identity(cell: &ValueCell) -> BytecodeRegisterIdentity {
    BytecodeRegisterIdentity::Typed {
        inner: Box::new(BytecodeRegisterIdentity::Cell(cell.compiler_identity())),
        annotation: cell.schema_key(),
    }
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
    if let Some(children) = cell.compiler_composite_children() {
        return compile_value_cell_composite_register(cell, children, context);
    }
    context.retain_canonical_cell(cell)?;
    let (register, initialize) = context
        .register_for_identity_with_initialization_status(&value_cell_register_identity(cell));
    record_resolved_register(cell, register, context)?;
    if initialize {
        let value = compiler_value_cell_snapshot(cell, "value-cell register compilation")?;
        let encoded = crate::encode_canonical_constant(&value, cell.representation())?;
        let constant = context.intern_constant(encoded)?;
        context.record_register_constant_metadata(register, constant)?;
        context.emit_const_load(register, constant);
    }
    Ok(register)
}

/// Reserves the register owned by `cell` with the exact declaration-time
/// canonical value that must seed mutable state.
///
/// Source planning executes before bytecode emission, so the live cell may
/// already contain a later reactive value. Keeping the declaration snapshot
/// explicit prevents that live value from replacing the state initializer.
pub fn compile_value_cell_initializer_register(
    cell: &ValueCell,
    initial: &crate::Value,
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    context.retain_canonical_cell(cell)?;
    let encoded = crate::encode_canonical_constant(initial, cell.representation())?;
    let (register, initialize) = context
        .register_for_identity_with_initialization_status(&value_cell_register_identity(cell));
    record_resolved_register(cell, register, context)?;
    let constant = context.intern_constant(encoded)?;
    if initialize {
        context.record_register_constant_metadata(register, constant)?;
        context.emit_const_load(register, constant);
    }
    context.record_state_initializer(register, constant)?;
    Ok(register)
}

/// Compiles a canonical composite cell from its live canonical child cells.
///
/// The encoded value is used only as the structural template. Child registers
/// remain the executable payload authority, preserving reactive topology
/// without reconstructing an erased aggregate.
pub fn compile_value_cell_composite_register(
    cell: &ValueCell,
    children: &[ValueCell],
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    context.retain_canonical_cell(cell)?;
    let value = compiler_value_cell_snapshot(cell, "composite value-cell compilation")?;
    let encoded = crate::encode_canonical_composite_template(&value, cell.representation())?;
    let (register, initialize) = context
        .register_for_identity_with_initialization_status(&value_cell_register_identity(cell));
    record_resolved_register(cell, register, context)?;
    if !initialize {
        return Ok(register);
    }
    let children = children
        .iter()
        .map(|child| compile_value_cell_register(child, context))
        .collect::<MResult<Vec<_>>>()?;
    let template = context.intern_constant(encoded)?;
    context.record_register_constant_metadata(register, template)?;
    context.emit_composite_pack(register, template, children);
    Ok(register)
}

/// Compiles a generic canonical matrix literal from live element cells and
/// explicit source-absence positions. Bytecode v1 represents this as a
/// `CompositePack` plus a matrix-literal sidecar rather than attempting to
/// encode heterogeneous or optional matrix elements as one flat constant.
#[cfg(feature = "matrix")]
pub fn compile_value_cell_matrix_literal_register(
    cell: &ValueCell,
    rows: u32,
    columns: u32,
    elements: &[Option<ValueCell>],
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    context.retain_canonical_cell(cell)?;
    let _ = compiler_value_cell_snapshot(cell, "matrix value-cell compilation")?;
    let (register, initialize) = context
        .register_for_identity_with_initialization_status(&value_cell_register_identity(cell));
    let schema = cell.closed_schema_body()?;
    record_resolved_register(cell, register, context)?;
    if !initialize {
        return Ok(register);
    }
    let crate::SchemaBody::Matrix { element, .. } = schema else {
        return compiler_invariant("canonical matrix compiler received a non-matrix schema");
    };
    let mut descriptors = Vec::with_capacity(elements.len());
    let mut children = Vec::with_capacity(elements.len());
    for element in elements {
        let (child, descriptor) = match element {
            Some(element) => {
                let child = compile_value_cell_register(element, context)?;
                (
                    child,
                    CompiledMatrixLiteralElement::Value { register: child },
                )
            }
            None => {
                let child = compile_absent_register(context)?;
                (
                    child,
                    CompiledMatrixLiteralElement::Empty { register: child },
                )
            }
        };
        children.push(child);
        descriptors.push(descriptor);
    }
    context.record_matrix_literal(CompiledMatrixLiteral::new(
        register,
        rows,
        columns,
        descriptors.into_boxed_slice(),
    )?)?;
    let template = context.intern_constant(EncodedConstant {
        runtime_type: crate::RuntimeType::Kind(crate::BytecodeKind::Matrix(
            Box::new(if rows == 0 || columns == 0 {
                crate::BytecodeKind::Any
            } else {
                bytecode_kind_from_schema(&element)?
            }),
            vec![rows as usize, columns as usize],
        )),
        alignment: 1,
        bytes: Vec::new(),
    })?;
    context.record_register_constant_metadata(register, template)?;
    context.emit_composite_pack(register, template, children);
    Ok(register)
}

pub fn bytecode_kind_from_schema(schema: &crate::SchemaBody) -> MResult<crate::BytecodeKind> {
    use crate::{BytecodeKind, FloatWidth, IntegerWidth, SchemaBody};

    Ok(match schema {
        SchemaBody::Bool => BytecodeKind::Scalar(crate::hash_str("bool")),
        SchemaBody::UnsignedInteger(IntegerWidth::W8) => {
            BytecodeKind::Scalar(crate::hash_str("u8"))
        }
        SchemaBody::UnsignedInteger(IntegerWidth::W16) => {
            BytecodeKind::Scalar(crate::hash_str("u16"))
        }
        SchemaBody::UnsignedInteger(IntegerWidth::W32) => {
            BytecodeKind::Scalar(crate::hash_str("u32"))
        }
        SchemaBody::UnsignedInteger(IntegerWidth::W64) => {
            BytecodeKind::Scalar(crate::hash_str("u64"))
        }
        SchemaBody::UnsignedInteger(IntegerWidth::W128) => {
            BytecodeKind::Scalar(crate::hash_str("u128"))
        }
        SchemaBody::SignedInteger(IntegerWidth::W8) => BytecodeKind::Scalar(crate::hash_str("i8")),
        SchemaBody::SignedInteger(IntegerWidth::W16) => {
            BytecodeKind::Scalar(crate::hash_str("i16"))
        }
        SchemaBody::SignedInteger(IntegerWidth::W32) => {
            BytecodeKind::Scalar(crate::hash_str("i32"))
        }
        SchemaBody::SignedInteger(IntegerWidth::W64) => {
            BytecodeKind::Scalar(crate::hash_str("i64"))
        }
        SchemaBody::SignedInteger(IntegerWidth::W128) => {
            BytecodeKind::Scalar(crate::hash_str("i128"))
        }
        SchemaBody::FloatingPoint(FloatWidth::W32) => BytecodeKind::Scalar(crate::hash_str("f32")),
        SchemaBody::FloatingPoint(FloatWidth::W64) => BytecodeKind::Scalar(crate::hash_str("f64")),
        SchemaBody::Complex(FloatWidth::W64) => BytecodeKind::Scalar(crate::hash_str("complex")),
        SchemaBody::Rational64 => BytecodeKind::Scalar(crate::hash_str("rational")),
        SchemaBody::String => BytecodeKind::Scalar(crate::hash_str("string")),
        SchemaBody::Id => BytecodeKind::Id,
        SchemaBody::Index => BytecodeKind::Index,
        SchemaBody::Option(inner) => {
            BytecodeKind::Option(Box::new(bytecode_kind_from_schema(inner)?))
        }
        SchemaBody::Tuple(elements) if elements.is_empty() => BytecodeKind::Empty,
        _ => {
            return compiler_invariant(format!(
                "canonical matrix element schema {schema:?} has no bytecode-v1 kind"
            ));
        }
    })
}

fn compiler_value_cell_snapshot(cell: &ValueCell, phase: &'static str) -> MResult<crate::Value> {
    cell.snapshot().map_err(|error| {
        if error.kind_as::<crate::ValueCellBorrowConflict>().is_some() {
            value_cell_borrow_conflict(phase)
        } else {
            error
        }
    })
}

/// Compiles the source-control absence marker without conflating it with
/// canonical unit or option absence.
pub fn compile_absent_register(context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
    let (register, initialize) =
        context.register_for_identity_with_initialization_status(&BytecodeRegisterIdentity::Absent);
    context.record_absent_register(register)?;
    if initialize {
        let constant = context.intern_constant(EncodedConstant {
            runtime_type: crate::RuntimeType::Empty,
            alignment: 1,
            bytes: Vec::new(),
        })?;
        context.record_register_constant_metadata(register, constant)?;
        context.emit_const_load(register, constant);
    }
    Ok(register)
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
    context.retain_canonical_cell(cell)?;
    let _ = compiler_value_cell_snapshot(cell, "runtime-produced value-cell compilation")?;
    let (register, _) = context
        .register_for_identity_with_initialization_status(&value_cell_register_identity(cell));
    record_resolved_register(cell, register, context)?;
    context.record_runtime_produced_register(register)?;
    Ok(register)
}

/// Resolves a runtime-produced canonical cell and emits an explicit seed used
/// only by bytecode formats that require destination storage initialization.
/// The executable producer remains authoritative for the register's value.
pub fn compile_runtime_produced_value_cell_register_with_seed(
    cell: &ValueCell,
    seed: &crate::Value,
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    if seed.schema_key() != cell.schema_key() {
        return compiler_invariant("runtime-produced register seed has the wrong schema");
    }
    let register = compile_runtime_produced_value_cell_register(cell, context)?;
    let encoded = crate::encode_canonical_exact_backing(seed, cell.representation())?;
    let constant = context.intern_constant(encoded)?;
    context.record_register_constant_metadata(register, constant)?;
    context.emit_const_load(register, constant);
    Ok(register)
}

fn compiler_invariant<T>(reason: impl Into<String>) -> MResult<T> {
    Err(MechError::new(
        BytecodeValidationError {
            reason: reason.into(),
        },
        None,
    )
    .with_compiler_loc())
}
