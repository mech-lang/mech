use mech_core::*;
use std::sync::LazyLock;

static PURE_BINARY_FULL_WRITE_EXACT_SCALAR: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| pure_binary_full_write(ChangeDetectionPolicy::ExactScalar));
static PURE_BINARY_FULL_WRITE_KERNEL_REPORTED: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| pure_binary_full_write(ChangeDetectionPolicy::KernelReported));
fn pure_binary_full_write(change_detection: ChangeDetectionPolicy) -> OperationContractDeclaration {
    OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
            ]
            .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            alias: AliasPolicy::NoAlias,
            change_detection,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    }
}

pub(crate) fn arithmetic_full_write_contract(
    output: FunctionValueRepresentation,
) -> &'static OperationContractDeclaration {
    match output {
        FunctionValueRepresentation::Matrix { .. } => &PURE_BINARY_FULL_WRITE_KERNEL_REPORTED,
        _ => &PURE_BINARY_FULL_WRITE_EXACT_SCALAR,
    }
}

pub(crate) fn managed_broadcast_element<T: ManagedElement>(
    input: &ManagedValueView<'_, T>,
    row: usize,
    column: usize,
    output_rows: usize,
    output_columns: usize,
) -> MResult<T> {
    let coordinate = match (input.rows(), input.columns()) {
        (1, 1) => Some((0, 0)),
        (rows, columns) if rows == output_rows && columns == output_columns => Some((row, column)),
        (1, columns) if columns == output_columns => Some((0, column)),
        (rows, 1) if rows == output_rows => Some((row, 0)),
        _ => None,
    };
    coordinate
        .and_then(|(row, column)| input.get(row, column))
        .ok_or_else(|| {
            MechError::from(MemoryRuntimeError::InvalidLayout {
                object: None,
                size: input.len() as u64,
                alignment: core::mem::align_of::<T>() as u32,
                reason: "managed arithmetic broadcast geometry is incompatible",
            })
        })
}
