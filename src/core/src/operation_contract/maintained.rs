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

/// The maintained elementwise contract shared by semantic producers and
/// concrete provider declarations. Providers retain their established output
/// change policy; this function defines the common port and shape semantics.
pub fn elementwise_operation_contract(
    input_count: usize,
    change_detection: ChangeDetectionPolicy,
) -> OperationContractDeclaration {
    declaration(
        InputPortLayout::Fixed(vec![read(); input_count].into_boxed_slice()),
        OutputConstruction::FullWrite {
            shape: ShapeRule::Declared,
        },
        change_detection,
    )
}

/// Shape-preserving mathematical unary operations use the input's geometry.
pub fn unary_math_operation_contract(
    change_detection: ChangeDetectionPolicy,
) -> OperationContractDeclaration {
    declaration(
        InputPortLayout::Fixed(vec![read()].into_boxed_slice()),
        OutputConstruction::FullWrite {
            shape: ShapeRule::SameAsInput { input: 0 },
        },
        change_detection,
    )
}

/// Operations whose semantics must be available independently of installed kernels.
/// `matrix_output` describes the resolved schema, never a runtime representation choice.
pub fn maintained_operation_contract(
    name: &str,
    input_count: usize,
    matrix_output: bool,
) -> Option<OperationContractDeclaration> {
    let change_detection = if matrix_output {
        ChangeDetectionPolicy::KernelReported
    } else {
        ChangeDetectionPolicy::ExactScalar
    };
    if let Some(operation) = crate::maintained_math_operation(name) {
        if operation.input_count() != input_count {
            return None;
        }
        return Some(if input_count == 1 {
            unary_math_operation_contract(change_detection)
        } else {
            elementwise_operation_contract(input_count, change_detection)
        });
    }
    let fixed = || InputPortLayout::Fixed(vec![read(); input_count].into_boxed_slice());
    let full = |shape| {
        declaration(
            fixed(),
            OutputConstruction::FullWrite { shape },
            ChangeDetectionPolicy::KernelReported,
        )
    };
    match name {
        "compare/neq" | "compare/eq" | "compare/sneq" | "compare/seq" | "compare/gt"
        | "compare/lt" | "compare/gte" | "compare/lte" | "compare/min" | "compare/max"
        | "logic/or" | "logic/and" | "logic/xor" | "string/concat"
            if input_count == 2 =>
        {
            Some(elementwise_operation_contract(2, change_detection))
        }
        "table/join"
        | "table/left-outer-join"
        | "table/right-outer-join"
        | "table/full-outer-join"
        | "table/left-semi-join"
        | "table/left-anti-join"
            if input_count == 2 =>
        {
            Some(full(ShapeRule::Declared))
        }
        "set/union"
        | "set/intersection"
        | "set/difference"
        | "set/symmetric-difference"
        | "set/cartesian-product"
            if input_count == 2 =>
        {
            Some(declaration(
                fixed(),
                OutputConstruction::FullWrite {
                    shape: ShapeRule::Declared,
                },
                ChangeDetectionPolicy::AlwaysChanged,
            ))
        }
        "logic/not" if input_count == 1 => {
            Some(elementwise_operation_contract(1, change_detection))
        }
        "range/inclusive" | "range/exclusive" if input_count == 2 => {
            Some(range_operation_contract(name, input_count))
        }
        "range/inclusive-increment" | "range/exclusive-increment" if input_count == 3 => {
            Some(range_operation_contract(name, input_count))
        }
        "matrix/transpose" if input_count == 1 => Some(full(ShapeRule::TransposeOf { input: 0 })),
        "matrix/literal" => Some(declaration(
            fixed(),
            OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            ChangeDetectionPolicy::AlwaysChanged,
        )),
        "core/composite-pack" => Some(declaration(
            InputPortLayout::Variadic {
                prefix: Box::new([]),
                repeated: read(),
                min_repetitions: 0,
            },
            OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            ChangeDetectionPolicy::KernelReported,
        )),
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
        "access/range" => Some(full(ShapeRule::Declared)),
        "access/column" | "access/swizzle" | "access/scalar" | "access/rows" | "access/columns"
        | "access/rectangle" => Some(full(if input_count == 1 {
            ShapeRule::SameAsInput { input: 0 }
        } else {
            ShapeRule::Declared
        })),
        _ => None,
    }
}

fn range_operation_contract(name: &str, input_count: usize) -> OperationContractDeclaration {
    // Called only by the four explicit maintained range entries above.
    let contract_name = match name {
        "range/inclusive" => "inclusive-output",
        "range/exclusive" => "exclusive-output",
        "range/inclusive-increment" => "inclusive-increment-output",
        "range/exclusive-increment" => "exclusive-increment-output",
        _ => unreachable!("maintained range declaration"),
    };
    declaration(
        InputPortLayout::Fixed(vec![read(); input_count].into_boxed_slice()),
        OutputConstruction::Build {
            postcondition: ShapeContractReference {
                module_path: vec!["range".to_string()].into_boxed_slice(),
                contract_name: contract_name.to_string(),
            },
        },
        ChangeDetectionPolicy::KernelReported,
    )
}
