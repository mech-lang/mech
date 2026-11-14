//! The maintained scalar word binder; native certification follows the exact
//! validated U64 contract and cannot be supplied by a public operation name.
use super::native_semantics::{self, NativeScalarOp};
use crate::intrinsics::bits::{OPERATIONS, Operation};
use mech_core::*;

pub(crate) fn install(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    macro_rules! install {
        ($index:literal) => {
            builder.insert_resident_factory(
                ["bits"],
                OPERATIONS[$index].item(),
                ImplementationMemoryClass::CanonicalFinalize,
                |request| bind(request, OPERATIONS[$index]),
            )?;
        };
    }
    install!(0);
    install!(1);
    install!(2);
    install!(3);
    install!(4);
    install!(5);
    install!(6);
    install!(7);
    install!(8);
    install!(9);
    install!(10);
    Ok(())
}
fn bind(
    request: &ResidentKernelBindRequest<'_>,
    operation: Operation,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    super::numeric::validate_full_write(
        request,
        operation.arity(),
        ShapeRule::Declared,
        ChangeDetectionPolicy::ExactScalar,
    )?;
    for port in request.inputs.iter().chain([&request.output]) {
        let Some(schema) = request.schemas.get(port.schema_id) else {
            return Err(ResidentKernelBindError::UnsupportedLayout);
        };
        if port.kind != ResidentValueKind::Snapshot
            || port.shape != ResidentShape::SCALAR
            || !port.shape_instance.parameter_values().is_empty()
            || schema.body() != &SchemaBody::UnsignedInteger(IntegerWidth::W64)
            || schema.key() != port.schema_key
            || !schema.dimension_parameters().is_empty()
        {
            return Err(ResidentKernelBindError::UnsupportedLayout);
        }
    }
    let kernel = BoundResidentKernel::new(execute, Box::new([operation as u64]))
        .with_snapshot_output(ResidentSnapshotOutput {
            schema: request.output.schema_id,
            schema_key: request.output.schema_key,
            shape: request.output.shape_instance.clone(),
            exact_cardinality: None,
            maximum_cardinality: None,
        })
        .with_snapshot_schemas(request.schemas.clone());
    let semantics = match operation {
        Operation::And => NativeScalarOp::U64BitAnd,
        Operation::Or => NativeScalarOp::U64BitOr,
        Operation::Xor => NativeScalarOp::U64BitXor,
        Operation::Not => NativeScalarOp::U64BitNot,
        Operation::ShiftLeft => NativeScalarOp::U64ShlWrap,
        Operation::ShiftRight => NativeScalarOp::U64ShrWrap,
        Operation::LeadingZeros => NativeScalarOp::U64Clz,
        Operation::TrailingZeros => NativeScalarOp::U64Ctz,
        Operation::WrappingAdd => NativeScalarOp::U64WrappingAdd,
        Operation::WrappingSub => NativeScalarOp::U64WrappingSub,
        Operation::WrappingMul => NativeScalarOp::U64WrappingMul,
    };
    Ok(native_semantics::certify(request, kernel, semantics))
}
fn execute(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let operation = kernel
        .parameters()
        .first()
        .and_then(|index| usize::try_from(*index).ok())
        .and_then(|index| OPERATIONS.get(index))
        .ok_or(ResidentKernelError::InvalidInput)?;
    let read = |index| {
        let Some(ResidentValueRef::Snapshot([Some(value)])) = inputs.get(index) else {
            return Err(ResidentKernelError::InvalidInput);
        };
        let ValueData::U64(value) = value.data() else {
            return Err(ResidentKernelError::InvalidInput);
        };
        Ok(*value)
    };
    let lhs = read(0)?;
    let rhs = if operation.arity() == 2 { read(1)? } else { 0 };
    super::numeric::write_snapshot_data_with_work_budget(
        kernel,
        output,
        ValueDataDraft::U64(operation.apply(lhs, rhs)),
        Some(0),
    )
}
