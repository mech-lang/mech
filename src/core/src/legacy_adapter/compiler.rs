//! Bytecode compiler compatibility for the retired universal value ABI.

use crate::{
    BytecodeCompilerContext, BytecodeRegisterIdentity, CompileConst, LegacyValue, MResult, Ref,
    Register, ValueCell, ValueKind,
};

#[cfg(feature = "matrix")]
use crate::{CompiledMatrixLiteral, CompiledMatrixLiteralElement, MechError, matrix::Matrix};

#[cfg(feature = "matrix")]
fn compiler_invariant<T>(reason: impl Into<String>) -> MResult<T> {
    Err(MechError::new(
        crate::BytecodeValidationError {
            reason: reason.into(),
        },
        None,
    )
    .with_compiler_loc())
}

fn record_legacy_register_schema(
    register: Register,
    value: &LegacyValue,
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<()> {
    if value.is_legacy_empty() {
        return context.record_absent_register(register);
    }
    context.record_register_schema(
        register,
        ValueCell::new(value.clone()).closed_schema_body()?,
    )
}

pub fn bytecode_register_identity(
    value: &LegacyValue,
    fallback: usize,
) -> MResult<BytecodeRegisterIdentity> {
    Ok(match value {
        LegacyValue::MutableReference(reference) => {
            bytecode_register_identity(&reference.borrow(), reference.addr())?
        }
        LegacyValue::Typed(value, annotation) => BytecodeRegisterIdentity::Typed {
            inner: Box::new(bytecode_register_identity(value, fallback)?),
            annotation: super::kind::schema_from_legacy_value_kind(
                annotation,
                &mut super::value::InferredLegacySemanticContext,
            )?
            .key(),
        },
        _ => value
            .reactive_root_cell_ids()
            .first()
            .map(|cell| BytecodeRegisterIdentity::Cell(cell.get() as usize))
            .unwrap_or(BytecodeRegisterIdentity::Ephemeral(fallback)),
    })
}

#[doc(hidden)]
pub fn compiler_value_cell_from_legacy(value: &LegacyValue) -> Option<ValueCell> {
    value
        .exact_ref_any()?
        .downcast_ref::<Ref<LegacyValue>>()
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
        let schema = super::kind::schema_from_legacy_value_kind(
            annotation,
            &mut super::value::InferredLegacySemanticContext,
        )?;
        identity = BytecodeRegisterIdentity::Typed {
            inner: Box::new(identity),
            annotation: schema.key(),
        };
        let (typed_register, initialize) =
            context.register_for_identity_with_initialization_status(&identity);
        context.record_register_schema(typed_register, schema.body().clone())?;
        if initialize {
            let template = crate::program::compiler::compile_annotated_constant(
                value,
                &annotations[index..],
                context,
            )?;
            context.record_register_constant_metadata(typed_register, template)?;
            context.emit_composite_pack(typed_register, template, vec![register]);
        }
        register = typed_register;
    }
    Ok(register)
}

#[doc(hidden)]
pub fn compile_annotated_value_register(
    value: &LegacyValue,
    annotations: &[ValueKind],
    fallback: usize,
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    let identity = bytecode_register_identity(value, fallback)?;
    let register = compile_value_register(value, fallback, context)?;
    compile_annotation_layers(value, identity, register, annotations, context)
}

pub fn compile_value_register(
    value: &LegacyValue,
    fallback: usize,
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    if let LegacyValue::MutableReference(reference) = value {
        return compile_value_register(&reference.borrow(), reference.addr(), context);
    }

    let identity = bytecode_register_identity(value, fallback)?;
    let (register, initialize) =
        context.register_for_identity_with_initialization_status(&identity);
    record_legacy_register_schema(register, value, context)?;
    if !initialize && context.register_is_runtime_produced(register) {
        return Ok(register);
    }
    #[cfg(feature = "matrix")]
    if let Some(matrix) = value
        .exact_matrix_any()
        .and_then(|matrix| matrix.downcast_ref::<Matrix<LegacyValue>>())
    {
        return compile_matrix_literal_register(
            matrix,
            value.kind(),
            register,
            initialize,
            context,
        );
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

pub fn compile_value_register_for_ptr(
    value: &LegacyValue,
    pointer: usize,
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    let (register, initialize) = context.register_for_ptr_with_initialization_status(pointer);
    record_legacy_register_schema(register, value, context)?;
    if !initialize && context.register_is_runtime_produced(register) {
        return Ok(register);
    }
    #[cfg(feature = "matrix")]
    if let Some(matrix) = value
        .exact_matrix_any()
        .and_then(|matrix| matrix.downcast_ref::<Matrix<LegacyValue>>())
    {
        return compile_matrix_literal_register(
            matrix,
            value.kind(),
            register,
            initialize,
            context,
        );
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
    matrix: &Matrix<LegacyValue>,
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
    let rows_u32 = u32::try_from(rows)
        .map_err(|_| compiler_invariant::<()>("generic matrix rows exceed u32").unwrap_err())?;
    let columns_u32 = u32::try_from(columns)
        .map_err(|_| compiler_invariant::<()>("generic matrix columns exceed u32").unwrap_err())?;

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
    context.record_matrix_literal(CompiledMatrixLiteral::new(
        register,
        rows_u32,
        columns_u32,
        elements.into_boxed_slice(),
    )?)?;
    if initialize {
        let template = crate::program::compiler::compile_kind_constant(&output_kind, context)?;
        context.record_register_constant_metadata(register, template)?;
        context.emit_composite_pack(register, template, children);
    }
    Ok(register)
}

pub fn compile_runtime_produced_register(
    value: &LegacyValue,
    fallback: usize,
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    if let LegacyValue::MutableReference(reference) = value {
        return compile_runtime_produced_register(&reference.borrow(), reference.addr(), context);
    }
    let identity = bytecode_register_identity(value, fallback)?;
    let (register, _) = context.register_for_identity_with_initialization_status(&identity);
    record_legacy_register_schema(register, value, context)?;
    context.record_runtime_produced_register(register)?;
    Ok(register)
}
