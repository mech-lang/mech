#[cfg(all(feature = "no_std", not(feature = "std")))]
use alloc::string::String;
#[cfg(any(not(feature = "no_std"), feature = "std"))]
use std::string::String;

use crate::{FunctionMatrixDescriptor, MResult, MechError, MechErrorKind, ValueCell};
pub type RuntimeFunctionCanonicalValidator = fn(&ValueCell, &[ValueCell]) -> MResult<()>;

#[derive(Clone, Copy)]
enum CanonicalContractValidator {
    NoMatrix,
    SameShape,
    ElementwiseBroadcast,
    OutputMatchesInput(usize),
    MatrixProduct,
    Transpose,
    HorizontalConcatenation,
    VerticalConcatenation,
    LinearSolve,
    Custom(RuntimeFunctionCanonicalValidator),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RuntimeOutputAliasPolicy {
    DisallowInputAlias,
    AllowInputAlias,
}

#[derive(Clone, Copy)]
pub struct RuntimeFunctionContract {
    pub kind: &'static str,
    pub output_alias: RuntimeOutputAliasPolicy,
    canonical_validator: CanonicalContractValidator,
}

impl RuntimeFunctionContract {
    pub const fn no_matrix(output_alias: RuntimeOutputAliasPolicy) -> Self {
        Self::built_in(
            "no_matrix",
            output_alias,
            CanonicalContractValidator::NoMatrix,
        )
    }

    pub const fn same_shape(output_alias: RuntimeOutputAliasPolicy) -> Self {
        Self::built_in(
            "same_shape",
            output_alias,
            CanonicalContractValidator::SameShape,
        )
    }

    pub const fn elementwise_broadcast(output_alias: RuntimeOutputAliasPolicy) -> Self {
        Self::built_in(
            "elementwise_broadcast",
            output_alias,
            CanonicalContractValidator::ElementwiseBroadcast,
        )
    }

    pub const fn output_matches_input(
        input: usize,
        output_alias: RuntimeOutputAliasPolicy,
    ) -> Self {
        Self::built_in(
            "output_matches_input",
            output_alias,
            CanonicalContractValidator::OutputMatchesInput(input),
        )
    }

    pub const fn matrix_product(output_alias: RuntimeOutputAliasPolicy) -> Self {
        Self::built_in(
            "matrix_product",
            output_alias,
            CanonicalContractValidator::MatrixProduct,
        )
    }

    pub const fn transpose(output_alias: RuntimeOutputAliasPolicy) -> Self {
        Self::built_in(
            "transpose",
            output_alias,
            CanonicalContractValidator::Transpose,
        )
    }

    pub const fn horizontal_concatenation(output_alias: RuntimeOutputAliasPolicy) -> Self {
        Self::built_in(
            "horizontal_concatenation",
            output_alias,
            CanonicalContractValidator::HorizontalConcatenation,
        )
    }

    pub const fn vertical_concatenation(output_alias: RuntimeOutputAliasPolicy) -> Self {
        Self::built_in(
            "vertical_concatenation",
            output_alias,
            CanonicalContractValidator::VerticalConcatenation,
        )
    }

    pub const fn linear_solve(output_alias: RuntimeOutputAliasPolicy) -> Self {
        Self::built_in(
            "linear_solve",
            output_alias,
            CanonicalContractValidator::LinearSolve,
        )
    }

    pub const fn canonical_custom(
        kind: &'static str,
        output_alias: RuntimeOutputAliasPolicy,
        canonical_validator: RuntimeFunctionCanonicalValidator,
    ) -> Self {
        Self {
            kind,
            output_alias,
            canonical_validator: CanonicalContractValidator::Custom(canonical_validator),
        }
    }

    const fn built_in(
        kind: &'static str,
        output_alias: RuntimeOutputAliasPolicy,
        canonical_validator: CanonicalContractValidator,
    ) -> Self {
        Self {
            kind,
            output_alias,
            canonical_validator,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionShapeContractViolation {
    pub contract: &'static str,
    pub reason: String,
}

impl MechErrorKind for FunctionShapeContractViolation {
    fn name(&self) -> &str {
        "FunctionShapeContractViolation"
    }

    fn message(&self) -> String {
        format!(
            "function shape contract {} failed: {}",
            self.contract, self.reason
        )
    }
}

fn invalid(contract: &'static str, reason: impl Into<String>) -> MechError {
    MechError::new(
        FunctionShapeContractViolation {
            contract,
            reason: reason.into(),
        },
        None,
    )
    .with_compiler_loc()
}

/// Creates the structured error returned by owner-local runtime shape
/// validators.
pub fn function_shape_contract_violation(
    contract: &'static str,
    reason: impl Into<String>,
) -> MechError {
    invalid(contract, reason)
}

fn same_dimensions(left: FunctionMatrixDescriptor, right: FunctionMatrixDescriptor) -> bool {
    left.rows == right.rows && left.cols == right.cols
}

pub(crate) fn validate_canonical_shapes(
    contract: RuntimeFunctionContract,
    output_cell: &ValueCell,
    input_cells: &[ValueCell],
    output: Option<FunctionMatrixDescriptor>,
    inputs: &[Option<FunctionMatrixDescriptor>],
) -> MResult<()> {
    match contract.canonical_validator {
        CanonicalContractValidator::NoMatrix => {
            if output.is_some() {
                return Err(invalid("no_matrix", "output is matrix-backed"));
            }
            if let Some(index) = inputs.iter().position(Option::is_some) {
                return Err(invalid(
                    "no_matrix",
                    format!("input {index} is matrix-backed"),
                ));
            }
        }
        CanonicalContractValidator::SameShape => {
            let mut expected = output;
            let mut found_matrix_input = false;
            for (index, found) in inputs.iter().copied().enumerate() {
                if let Some(found) = found {
                    found_matrix_input = true;
                    if let Some(expected) = expected {
                        if !same_dimensions(expected, found) {
                            return Err(invalid(
                                "same_shape",
                                format!(
                                    "input {index} is {}x{}, expected {}x{}",
                                    found.rows, found.cols, expected.rows, expected.cols,
                                ),
                            ));
                        }
                    } else {
                        expected = Some(found);
                    }
                }
            }
            if found_matrix_input && output.is_none() {
                return Err(invalid(
                    "same_shape",
                    "matrix-backed inputs require a matrix-backed output",
                ));
            }
        }
        CanonicalContractValidator::ElementwiseBroadcast => {
            let mut expected = None::<FunctionMatrixDescriptor>;
            for found in inputs.iter().copied().flatten() {
                expected = Some(match expected {
                    None => found,
                    Some(current) => {
                        let axis = |left: usize, right: usize| {
                            if left == right {
                                Some(left)
                            } else if left == 1 {
                                Some(right)
                            } else if right == 1 {
                                Some(left)
                            } else {
                                None
                            }
                        };
                        FunctionMatrixDescriptor {
                            representation: current.representation,
                            rows: axis(current.rows, found.rows).ok_or_else(|| {
                                invalid(
                                    "elementwise_broadcast",
                                    format!(
                                        "row extents {} and {} cannot broadcast",
                                        current.rows, found.rows,
                                    ),
                                )
                            })?,
                            cols: axis(current.cols, found.cols).ok_or_else(|| {
                                invalid(
                                    "elementwise_broadcast",
                                    format!(
                                        "column extents {} and {} cannot broadcast",
                                        current.cols, found.cols,
                                    ),
                                )
                            })?,
                        }
                    }
                });
            }
            match (output, expected) {
                (Some(output), Some(expected)) if same_dimensions(output, expected) => {}
                (Some(output), Some(expected)) => {
                    return Err(invalid(
                        "elementwise_broadcast",
                        format!(
                            "output is {}x{}, expected {}x{}",
                            output.rows, output.cols, expected.rows, expected.cols,
                        ),
                    ));
                }
                (None, Some(_)) => {
                    return Err(invalid(
                        "elementwise_broadcast",
                        "matrix-backed inputs require a matrix-backed output",
                    ));
                }
                (Some(_), None) => {
                    return Err(invalid(
                        "elementwise_broadcast",
                        "scalar inputs require a scalar output",
                    ));
                }
                (None, None) => {}
            }
        }
        CanonicalContractValidator::OutputMatchesInput(index) => {
            if index > 3 {
                return Err(invalid(
                    "output_matches_input",
                    format!("selected input exceeds {} available inputs", inputs.len()),
                ));
            }
            let output = output
                .ok_or_else(|| invalid("output_matches_input", "output is not matrix-backed"))?;
            let input = inputs.get(index).copied().flatten().ok_or_else(|| {
                invalid(
                    "output_matches_input",
                    format!("input {index} is not matrix-backed"),
                )
            })?;
            if !same_dimensions(output, input) {
                return Err(invalid(
                    "output_matches_input",
                    format!(
                        "output is {}x{}, input {index} is {}x{}",
                        output.rows, output.cols, input.rows, input.cols,
                    ),
                ));
            }
        }
        CanonicalContractValidator::MatrixProduct => {
            let (output, lhs, rhs) = required_canonical_binary(output, inputs, "matrix_product")?;
            if lhs.cols != rhs.rows || output.rows != lhs.rows || output.cols != rhs.cols {
                return Err(invalid(
                    "matrix_product",
                    format!(
                        "output {}x{}, lhs {}x{}, rhs {}x{}",
                        output.rows, output.cols, lhs.rows, lhs.cols, rhs.rows, rhs.cols,
                    ),
                ));
            }
        }
        CanonicalContractValidator::Transpose => {
            let output = required_canonical_output(output, "transpose")?;
            let input = required_canonical_input(inputs, 0, "transpose")?;
            if output.rows != input.cols || output.cols != input.rows {
                return Err(invalid(
                    "transpose",
                    format!(
                        "output {}x{} is not the transpose of input {}x{}",
                        output.rows, output.cols, input.rows, input.cols,
                    ),
                ));
            }
        }
        CanonicalContractValidator::HorizontalConcatenation => {
            validate_canonical_concatenation(output, inputs, true)?;
        }
        CanonicalContractValidator::VerticalConcatenation => {
            validate_canonical_concatenation(output, inputs, false)?;
        }
        CanonicalContractValidator::LinearSolve => {
            let (output, a, b) = required_canonical_binary(output, inputs, "linear_solve")?;
            if a.rows != a.cols
                || b.rows != a.rows
                || output.rows != a.cols
                || output.cols != b.cols
            {
                return Err(invalid(
                    "linear_solve",
                    format!(
                        "output {}x{}, A {}x{}, B {}x{}",
                        output.rows, output.cols, a.rows, a.cols, b.rows, b.cols,
                    ),
                ));
            }
        }
        CanonicalContractValidator::Custom(validator) => {
            validator(output_cell, input_cells)?;
        }
    }
    Ok(())
}

fn required_canonical_output(
    output: Option<FunctionMatrixDescriptor>,
    contract: &'static str,
) -> MResult<FunctionMatrixDescriptor> {
    output.ok_or_else(|| invalid(contract, "output is not matrix-backed"))
}

fn required_canonical_input(
    inputs: &[Option<FunctionMatrixDescriptor>],
    index: usize,
    contract: &'static str,
) -> MResult<FunctionMatrixDescriptor> {
    inputs
        .get(index)
        .copied()
        .flatten()
        .ok_or_else(|| invalid(contract, format!("input {index} is not matrix-backed")))
}

fn required_canonical_binary(
    output: Option<FunctionMatrixDescriptor>,
    inputs: &[Option<FunctionMatrixDescriptor>],
    contract: &'static str,
) -> MResult<(
    FunctionMatrixDescriptor,
    FunctionMatrixDescriptor,
    FunctionMatrixDescriptor,
)> {
    Ok((
        required_canonical_output(output, contract)?,
        required_canonical_input(inputs, 0, contract)?,
        required_canonical_input(inputs, 1, contract)?,
    ))
}

fn validate_canonical_concatenation(
    output: Option<FunctionMatrixDescriptor>,
    inputs: &[Option<FunctionMatrixDescriptor>],
    horizontal: bool,
) -> MResult<()> {
    let contract = if horizontal {
        "horizontal_concatenation"
    } else {
        "vertical_concatenation"
    };
    let output = required_canonical_output(output, contract)?;
    let logical = |index: usize| {
        inputs
            .get(index)
            .copied()
            .flatten()
            .map(|value| (value.rows, value.cols))
            .unwrap_or((1, 1))
    };
    let first = logical(0);
    let mut extent = 0usize;
    for index in 0..inputs.len() {
        let current = logical(index);
        let compatible = if horizontal {
            current.0 == first.0
        } else {
            current.1 == first.1
        };
        if !compatible {
            return Err(invalid(
                contract,
                format!("input {index} has incompatible dimensions"),
            ));
        }
        extent = extent
            .checked_add(if horizontal { current.1 } else { current.0 })
            .ok_or_else(|| invalid(contract, "input dimension sum overflowed usize"))?;
    }
    let expected = if horizontal {
        (first.0, extent)
    } else {
        (extent, first.1)
    };
    if (output.rows, output.cols) != expected {
        return Err(invalid(
            contract,
            format!(
                "output is {}x{}, expected {}x{}",
                output.rows, output.cols, expected.0, expected.1
            ),
        ));
    }
    Ok(())
}
