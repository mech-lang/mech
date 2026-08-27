#[cfg(feature = "matrix")]
use crate::BytecodeValidationError;
use crate::{
    ApplicationRequirement, EncodedConstant, LegacyValue, MResult, MechError, MechErrorKind,
    ValueCell, ValueKind,
};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, vec::Vec};

#[cfg(feature = "matrix")]
use super::CompiledMatrixLiteralElement;
#[cfg(feature = "matrix")]
use super::constants::compile_kind_constant;
use super::{CompileConst, CompiledMatrixLiteral, Register, compile_annotated_constant};

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
    let kind = value.kind();
    context.record_register_kind(register, kind.clone())?;
    // Observing a value whose register is owned by an executable producer is
    // not a second source construction. In particular, do not manufacture a
    // matrix-literal sidecar for a runtime-produced generic matrix.
    if !initialize && context.register_is_runtime_produced(register) {
        return Ok(register);
    }
    #[cfg(feature = "matrix")]
    if let Some(matrix) = value
        .exact_matrix_any()
        .and_then(|matrix| matrix.downcast_ref::<crate::matrix::Matrix<LegacyValue>>())
    {
        return compile_matrix_literal_register(matrix, kind, register, initialize, context);
    }
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
    let kind = value.kind();
    context.record_register_kind(register, kind.clone())?;
    // An executable producer supersedes the planning-time payload stored in
    // this cell, so observing that payload is not matrix-literal registration.
    if !initialize && context.register_is_runtime_produced(register) {
        return Ok(register);
    }
    #[cfg(feature = "matrix")]
    if let Some(matrix) = value
        .exact_matrix_any()
        .and_then(|matrix| matrix.downcast_ref::<crate::matrix::Matrix<LegacyValue>>())
    {
        return compile_matrix_literal_register(matrix, kind, register, initialize, context);
    }
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

#[cfg(feature = "matrix")]
fn compile_matrix_literal_register(
    matrix: &crate::matrix::Matrix<LegacyValue>,
    output_kind: ValueKind,
    register: Register,
    initialize: bool,
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    let Some((_, declared_dimensions)) = output_kind.matrix_parts() else {
        return compiler_invariant(format!(
            "generic matrix literal register {register} does not have a matrix output kind",
        ));
    };
    let rows = matrix.rows();
    let columns = matrix.cols();
    if declared_dimensions != [rows, columns] {
        return compiler_invariant(format!(
            "generic matrix literal register {register} declares dimensions {:?}, found {rows}x{columns}",
            declared_dimensions,
        ));
    }
    let rows_u32 = u32::try_from(rows).map_err(|_| {
        compiler_invariant::<()>(format!(
            "generic matrix literal row count {rows} exceeds u32",
        ))
        .unwrap_err()
    })?;
    let columns_u32 = u32::try_from(columns).map_err(|_| {
        compiler_invariant::<()>(format!(
            "generic matrix literal column count {columns} exceeds u32",
        ))
        .unwrap_err()
    })?;

    let mut values = Vec::with_capacity(rows.saturating_mul(columns));
    for row in 0..rows {
        for column in 0..columns {
            values.push(matrix.index2d(row + 1, column + 1));
        }
    }

    let mut elements = Vec::with_capacity(values.len());
    let mut children = Vec::with_capacity(values.len());
    for value in &values {
        let child = compile_value_register(value, core::ptr::from_ref(value).addr(), context)?;
        children.push(child);
        elements.push(if value.is_legacy_empty() {
            CompiledMatrixLiteralElement::Empty { register: child }
        } else {
            CompiledMatrixLiteralElement::Value { register: child }
        });
    }

    let literal =
        CompiledMatrixLiteral::new(register, rows_u32, columns_u32, elements.into_boxed_slice())?;
    context.record_matrix_literal(literal)?;

    if initialize {
        let template = compile_kind_constant(&output_kind, context)?;
        context.record_register_constant_metadata(register, template)?;
        context.emit_composite_pack(register, template, children);
    }
    Ok(register)
}

#[cfg(feature = "matrix")]
fn compiler_invariant<T>(reason: impl Into<String>) -> MResult<T> {
    Err(MechError::new(
        BytecodeValidationError {
            reason: reason.into(),
        },
        None,
    )
    .with_compiler_loc())
}

#[cfg(all(test, feature = "matrix", feature = "f64"))]
mod tests {
    use super::*;
    use crate::{
        BytecodeInstruction, CompileCtx, CompiledBytecode, Ref, decode_encoded_constants,
        matrix::Matrix,
    };

    fn f64_value(value: f64) -> LegacyValue {
        LegacyValue::F64(Ref::new(value))
    }

    fn compiled_matrix(values: Vec<LegacyValue>, rows: usize, columns: usize) -> CompiledBytecode {
        let value = LegacyValue::MatrixValue(Matrix::from_vec(values, rows, columns));
        let mut context = CompileCtx::new();
        let output =
            compile_value_register(&value, core::ptr::from_ref(&value).addr(), &mut context)
                .unwrap();
        context.finish_program(output).unwrap()
    }

    fn register_f64(compiled: &CompiledBytecode, register: Register) -> f64 {
        let constant = compiled
            .program
            .instructions
            .iter()
            .find_map(|instruction| match instruction {
                BytecodeInstruction::ConstLoad { dst, constant } if *dst == register => {
                    Some(*constant)
                }
                _ => None,
            })
            .unwrap();
        let values = decode_encoded_constants(&compiled.program.constants).unwrap();
        let LegacyValue::F64(value) = &values[constant as usize] else {
            panic!("matrix element register must contain an f64 constant");
        };
        *value.borrow()
    }

    #[test]
    fn generic_matrix_compilation_records_empty_and_repeated_elements() {
        let empty = compiled_matrix(Vec::new(), 0, 0);
        let descriptor = empty.matrix_literals.get(&empty.return_register).unwrap();
        assert_eq!((descriptor.rows, descriptor.columns), (0, 0));
        assert!(descriptor.elements.is_empty());

        let shared = f64_value(1.0);
        let compiled = compiled_matrix(vec![shared.clone(), LegacyValue::Empty, shared], 1, 3);
        let descriptor = compiled
            .matrix_literals
            .get(&compiled.return_register)
            .unwrap();
        assert!(matches!(
            descriptor.elements[1],
            CompiledMatrixLiteralElement::Empty { .. }
        ));
        assert_eq!(
            descriptor.elements[0].register(),
            descriptor.elements[2].register()
        );
    }

    #[test]
    fn generic_matrix_compilation_uses_canonical_row_major_order_and_kind_template() {
        // Matrix storage is column-major; the logical value is
        // [1, 2, 3]
        // [4, 5, 6].
        let compiled = compiled_matrix(
            [1.0, 4.0, 2.0, 5.0, 3.0, 6.0]
                .into_iter()
                .map(f64_value)
                .collect(),
            2,
            3,
        );
        let descriptor = compiled
            .matrix_literals
            .get(&compiled.return_register)
            .unwrap();
        let values = descriptor
            .elements
            .iter()
            .map(|element| register_f64(&compiled, element.register()))
            .collect::<Vec<_>>();
        assert_eq!(values, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);

        let templates = decode_encoded_constants(&compiled.program.constants).unwrap();
        let template = compiled
            .program
            .instructions
            .iter()
            .find_map(|instruction| match instruction {
                BytecodeInstruction::CompositePack {
                    dst,
                    template,
                    children,
                } if *dst == compiled.return_register => {
                    assert_eq!(children.len(), 6);
                    Some(*template)
                }
                _ => None,
            })
            .expect("generic matrix compilation must emit one CompositePack");
        assert_eq!(
            templates[template as usize],
            LegacyValue::Kind(ValueKind::Matrix(Box::new(ValueKind::F64), vec![2, 3]))
        );
        assert_eq!(
            compiled
                .program
                .instructions
                .iter()
                .filter(|instruction| matches!(
                    instruction,
                    BytecodeInstruction::CompositePack { .. }
                ))
                .count(),
            1
        );
    }

    #[test]
    fn nonempty_generic_matrix_is_not_an_ordinary_constant() {
        let value = LegacyValue::MatrixValue(Matrix::from_vec(vec![f64_value(1.0)], 1, 1));
        let mut context = CompileCtx::new();
        assert!(value.compile_const(&mut context).is_err());
    }

    #[test]
    fn reused_matrix_register_rejects_a_conflicting_descriptor_without_reemitting() {
        let left = f64_value(1.0);
        let right = f64_value(2.0);
        let first =
            LegacyValue::MatrixValue(Matrix::from_vec(vec![left.clone(), right.clone()], 1, 2));
        let conflicting = LegacyValue::MatrixValue(Matrix::from_vec(vec![right, left], 1, 2));
        let mut context = CompileCtx::new();
        let pointer = 0x4d41_5452_4958usize;
        let output = compile_value_register_for_ptr(&first, pointer, &mut context).unwrap();
        let error = compile_value_register_for_ptr(&conflicting, pointer, &mut context)
            .expect_err("reusing a matrix output must validate its complete descriptor");
        assert!(error.kind_message().contains("conflicting descriptors"));

        let compiled = context.finish_program(output).unwrap();
        assert_eq!(
            compiled
                .program
                .instructions
                .iter()
                .filter(|instruction| matches!(
                    instruction,
                    BytecodeInstruction::CompositePack { dst, .. } if *dst == output
                ))
                .count(),
            1,
            "descriptor validation must not emit another construction instruction",
        );
    }

    #[test]
    fn runtime_produced_generic_matrix_can_be_observed_without_a_literal_sidecar() {
        let value = LegacyValue::MatrixValue(Matrix::from_vec(Vec::new(), 0, 0));
        let pointer = core::ptr::from_ref(&value).addr();
        let mut context = CompileCtx::new();
        let produced = compile_runtime_produced_register(&value, pointer, &mut context).unwrap();
        let observed = compile_value_register(&value, pointer, &mut context).unwrap();
        let compiled = context.finish_program(observed).unwrap();

        assert_eq!(observed, produced);
        assert!(!compiled.matrix_literals.contains_key(&observed));
        assert!(
            compiled
                .program
                .instructions
                .iter()
                .all(|instruction| !matches!(
                    instruction,
                    BytecodeInstruction::ConstLoad { dst, .. }
                        | BytecodeInstruction::CompositePack { dst, .. }
                        if *dst == observed
                ))
        );
    }
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
