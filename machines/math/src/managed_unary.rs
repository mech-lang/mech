use mech_core::*;
use std::sync::LazyLock;

static PURE_UNARY_FULL_WRITE_EXACT_SCALAR: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| pure_unary_full_write(ChangeDetectionPolicy::ExactScalar));
static PURE_UNARY_FULL_WRITE_KERNEL_REPORTED: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| pure_unary_full_write(ChangeDetectionPolicy::KernelReported));

fn pure_unary_full_write(change_detection: ChangeDetectionPolicy) -> OperationContractDeclaration {
    OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![InputPortPolicy {
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            }]
            .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::SameAsInput { input: 0 },
            },
            alias: AliasPolicy::NoAlias,
            change_detection,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    }
}

pub(crate) fn unary_full_write_contract(
    output: FunctionValueRepresentation,
) -> &'static OperationContractDeclaration {
    match output {
        FunctionValueRepresentation::Matrix { .. } => &PURE_UNARY_FULL_WRITE_KERNEL_REPORTED,
        _ => &PURE_UNARY_FULL_WRITE_EXACT_SCALAR,
    }
}
