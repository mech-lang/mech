//! Provider-independent declarations shared by source producers and resident catalogs.
use super::*;
#[cfg(feature = "no_std")]
use alloc::{boxed::Box, string::ToString, vec, vec::Vec};

fn read() -> InputPortPolicy {
    InputPortPolicy {
        access: AccessMode::Read,
        delivery: DeliveryMode::Signal,
    }
}

fn declaration(
    inputs: InputPortLayout,
    construction: OutputConstruction,
    change_detection: ChangeDetectionPolicy,
) -> OperationContractDeclaration {
    OperationContractDeclaration {
        inputs,
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction,
            alias: AliasPolicy::NoAlias,
            change_detection,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    }
}

/// Operations whose semantics must be available independently of installed kernels.
/// `matrix_output` describes the resolved schema, never a runtime representation choice.
pub fn maintained_operation_contract(
    name: &str,
    input_count: usize,
    matrix_output: bool,
) -> Option<OperationContractDeclaration> {
    let fixed = || InputPortLayout::Fixed(vec![read(); input_count].into_boxed_slice());
    let full = |shape| {
        declaration(
            fixed(),
            OutputConstruction::FullWrite { shape },
            ChangeDetectionPolicy::KernelReported,
        )
    };
    match name {
        "matrix/horzcat"
        | "matrix/vertcat"
        | "matrix/comprehension"
        | "set/define"
        | "set/comprehension" => {
            let (module, contract_name, minimum) = match name {
                "matrix/horzcat" => (vec!["matrix", "concatenate"], "horizontal-output", 1),
                "matrix/vertcat" => (vec!["matrix", "concatenate"], "vertical-output", 1),
                "matrix/comprehension" => (vec!["matrix", "concatenate"], "horizontal-output", 0),
                "set/define" => (vec!["set"], "define-output", 0),
                _ => (vec!["set"], "comprehension-output", 0),
            };
            Some(declaration(
                InputPortLayout::Variadic {
                    prefix: Box::new([]),
                    repeated: read(),
                    min_repetitions: minimum,
                },
                OutputConstruction::Build {
                    postcondition: ShapeContractReference {
                        module_path: module
                            .into_iter()
                            .map(|part| part.to_string())
                            .collect::<Vec<_>>()
                            .into_boxed_slice(),
                        contract_name: contract_name.to_string(),
                    },
                },
                ChangeDetectionPolicy::KernelReported,
            ))
        }
        "option/some" => Some(full(ShapeRule::Declared)),
        "convert/kind" | "core/assign" => Some(full(ShapeRule::SameAsInput { input: 0 })),
        "matrix/matmul" | "matrix/multiply" | "matrix/dot" => Some(declaration(
            fixed(),
            OutputConstruction::FullWrite {
                shape: if matrix_output {
                    ShapeRule::MatrixProduct { lhs: 0, rhs: 1 }
                } else {
                    ShapeRule::Declared
                },
            },
            if matrix_output {
                ChangeDetectionPolicy::KernelReported
            } else {
                ChangeDetectionPolicy::ExactScalar
            },
        )),
        "matrix/solve" => Some(full(ShapeRule::SameAsInput { input: 1 })),
        "access/column" | "access/swizzle" | "access/scalar" | "access/range" | "access/rows"
        | "access/columns" | "access/rectangle" => Some(full(if input_count == 1 {
            ShapeRule::SameAsInput { input: 0 }
        } else {
            ShapeRule::Declared
        })),
        _ => None,
    }
}
