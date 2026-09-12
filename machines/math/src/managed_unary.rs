use mech_core::*;
use std::sync::LazyLock;

static PURE_UNARY_FULL_WRITE_EXACT_SCALAR: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| pure_unary_full_write(ChangeDetectionPolicy::ExactScalar));
static PURE_UNARY_FULL_WRITE_KERNEL_REPORTED: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| pure_unary_full_write(ChangeDetectionPolicy::KernelReported));

fn pure_unary_full_write(change_detection: ChangeDetectionPolicy) -> OperationContractDeclaration {
    mech_core::unary_math_operation_contract(change_detection)
}

pub(crate) fn unary_full_write_contract(
    output: FunctionValueRepresentation,
) -> &'static OperationContractDeclaration {
    match output {
        FunctionValueRepresentation::Matrix { .. } => &PURE_UNARY_FULL_WRITE_KERNEL_REPORTED,
        _ => &PURE_UNARY_FULL_WRITE_EXACT_SCALAR,
    }
}
