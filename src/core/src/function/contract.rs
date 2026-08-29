#[cfg(all(feature = "no_std", not(feature = "std")))]
use alloc::string::String;
#[cfg(any(not(feature = "no_std"), feature = "std"))]
use std::string::String;

use crate::{
    FunctionArgs, FunctionArgumentRole, FunctionMatrixDescriptor, MResult, MechError,
    MechErrorKind, ValueCell,
};

pub type RuntimeFunctionShapeValidator = fn(&FunctionArgs) -> MResult<()>;
pub type RuntimeFunctionCanonicalValidator = fn(&ValueCell, &[ValueCell]) -> MResult<()>;

#[derive(Clone, Copy)]
enum CanonicalContractValidator {
    NoMatrix,
    SameShape,
    OutputMatchesInput(usize),
    MatrixProduct,
    Transpose,
    HorizontalConcatenation,
    VerticalConcatenation,
    LinearSolve,
    Custom(RuntimeFunctionCanonicalValidator),
    Unsupported,
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
    pub validate_shapes: RuntimeFunctionShapeValidator,
    canonical_validator: CanonicalContractValidator,
}

impl RuntimeFunctionContract {
    pub const fn no_matrix(output_alias: RuntimeOutputAliasPolicy) -> Self {
        Self::built_in(
            "no_matrix",
            output_alias,
            validate_no_matrix_shapes,
            CanonicalContractValidator::NoMatrix,
        )
    }

    pub const fn same_shape(output_alias: RuntimeOutputAliasPolicy) -> Self {
        Self::built_in(
            "same_shape",
            output_alias,
            validate_same_shapes,
            CanonicalContractValidator::SameShape,
        )
    }

    pub const fn output_matches_input(
        input: usize,
        output_alias: RuntimeOutputAliasPolicy,
    ) -> Self {
        let validator = match input {
            0 => validate_output_matches_input_0 as RuntimeFunctionShapeValidator,
            1 => validate_output_matches_input_1,
            2 => validate_output_matches_input_2,
            3 => validate_output_matches_input_3,
            _ => validate_output_matches_unavailable_input,
        };
        Self::built_in(
            "output_matches_input",
            output_alias,
            validator,
            CanonicalContractValidator::OutputMatchesInput(input),
        )
    }

    pub const fn matrix_product(output_alias: RuntimeOutputAliasPolicy) -> Self {
        Self::built_in(
            "matrix_product",
            output_alias,
            validate_matrix_product,
            CanonicalContractValidator::MatrixProduct,
        )
    }

    pub const fn transpose(output_alias: RuntimeOutputAliasPolicy) -> Self {
        Self::built_in(
            "transpose",
            output_alias,
            validate_transpose,
            CanonicalContractValidator::Transpose,
        )
    }

    pub const fn horizontal_concatenation(output_alias: RuntimeOutputAliasPolicy) -> Self {
        Self::built_in(
            "horizontal_concatenation",
            output_alias,
            validate_horizontal_concatenation,
            CanonicalContractValidator::HorizontalConcatenation,
        )
    }

    pub const fn vertical_concatenation(output_alias: RuntimeOutputAliasPolicy) -> Self {
        Self::built_in(
            "vertical_concatenation",
            output_alias,
            validate_vertical_concatenation,
            CanonicalContractValidator::VerticalConcatenation,
        )
    }

    pub const fn linear_solve(output_alias: RuntimeOutputAliasPolicy) -> Self {
        Self::built_in(
            "linear_solve",
            output_alias,
            validate_linear_solve,
            CanonicalContractValidator::LinearSolve,
        )
    }

    pub const fn custom(
        kind: &'static str,
        output_alias: RuntimeOutputAliasPolicy,
        validator: RuntimeFunctionShapeValidator,
    ) -> Self {
        Self {
            kind,
            output_alias,
            validate_shapes: validator,
            canonical_validator: CanonicalContractValidator::Unsupported,
        }
    }

    pub const fn custom_with_canonical(
        kind: &'static str,
        output_alias: RuntimeOutputAliasPolicy,
        validator: RuntimeFunctionShapeValidator,
        canonical_validator: RuntimeFunctionCanonicalValidator,
    ) -> Self {
        Self {
            kind,
            output_alias,
            validate_shapes: validator,
            canonical_validator: CanonicalContractValidator::Custom(canonical_validator),
        }
    }

    /// Declares a value-dependent contract whose canonical validator is the
    /// sole production authority. Compatibility invocations are first bound
    /// to canonical cells by the legacy adapter and therefore follow this
    /// same validator instead of a parallel legacy implementation.
    pub const fn canonical_custom(
        kind: &'static str,
        output_alias: RuntimeOutputAliasPolicy,
        canonical_validator: RuntimeFunctionCanonicalValidator,
    ) -> Self {
        Self {
            kind,
            output_alias,
            validate_shapes: reject_direct_legacy_custom_validation,
            canonical_validator: CanonicalContractValidator::Custom(canonical_validator),
        }
    }

    const fn built_in(
        kind: &'static str,
        output_alias: RuntimeOutputAliasPolicy,
        validator: RuntimeFunctionShapeValidator,
        canonical_validator: CanonicalContractValidator,
    ) -> Self {
        Self {
            kind,
            output_alias,
            validate_shapes: validator,
            canonical_validator,
        }
    }
}

fn reject_direct_legacy_custom_validation(_args: &FunctionArgs) -> MResult<()> {
    Err(invalid(
        "canonical_custom",
        "custom contracts require canonical invocation cells",
    ))
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

fn output(args: &FunctionArgs) -> MResult<Option<FunctionMatrixDescriptor>> {
    args.output_value()
        .function_matrix_descriptor(FunctionArgumentRole::Output)
}

fn input(args: &FunctionArgs, index: usize) -> MResult<Option<FunctionMatrixDescriptor>> {
    args.input_value(index)
        .ok_or_else(|| invalid("argument", format!("missing input {index}")))?
        .function_matrix_descriptor(FunctionArgumentRole::Input(index))
}

fn required_output(
    args: &FunctionArgs,
    contract: &'static str,
) -> MResult<FunctionMatrixDescriptor> {
    output(args)?.ok_or_else(|| invalid(contract, "output is not matrix-backed"))
}

fn required_input(
    args: &FunctionArgs,
    index: usize,
    contract: &'static str,
) -> MResult<FunctionMatrixDescriptor> {
    input(args, index)?
        .ok_or_else(|| invalid(contract, format!("input {index} is not matrix-backed")))
}

fn logical_input_dimensions(
    args: &FunctionArgs,
    index: usize,
    contract: &'static str,
) -> MResult<(usize, usize)> {
    if args.input_value(index).is_none() {
        return Err(invalid(contract, format!("missing input {index}")));
    }
    Ok(input(args, index)?
        .map(|descriptor| (descriptor.rows, descriptor.cols))
        .unwrap_or((1, 1)))
}

fn same_dimensions(left: FunctionMatrixDescriptor, right: FunctionMatrixDescriptor) -> bool {
    left.rows == right.rows && left.cols == right.cols
}

fn validate_no_matrix_shapes(args: &FunctionArgs) -> MResult<()> {
    if output(args)?.is_some() {
        return Err(invalid("no_matrix", "output is matrix-backed"));
    }
    for index in 0..args.input_count() {
        if input(args, index)?.is_some() {
            return Err(invalid(
                "no_matrix",
                format!("input {index} is matrix-backed"),
            ));
        }
    }
    Ok(())
}

fn validate_same_shapes(args: &FunctionArgs) -> MResult<()> {
    let output = output(args)?;
    let mut expected = output;
    let mut found_matrix_input = false;
    for index in 0..args.input_count() {
        if let Some(found) = input(args, index)? {
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
    Ok(())
}

fn validate_output_matches_input(args: &FunctionArgs, index: usize) -> MResult<()> {
    let output = required_output(args, "output_matches_input")?;
    let input = required_input(args, index, "output_matches_input")?;
    if !same_dimensions(output, input) {
        return Err(invalid(
            "output_matches_input",
            format!(
                "output is {}x{}, input {index} is {}x{}",
                output.rows, output.cols, input.rows, input.cols,
            ),
        ));
    }
    Ok(())
}

fn validate_output_matches_input_0(args: &FunctionArgs) -> MResult<()> {
    validate_output_matches_input(args, 0)
}
fn validate_output_matches_input_1(args: &FunctionArgs) -> MResult<()> {
    validate_output_matches_input(args, 1)
}
fn validate_output_matches_input_2(args: &FunctionArgs) -> MResult<()> {
    validate_output_matches_input(args, 2)
}
fn validate_output_matches_input_3(args: &FunctionArgs) -> MResult<()> {
    validate_output_matches_input(args, 3)
}
fn validate_output_matches_unavailable_input(args: &FunctionArgs) -> MResult<()> {
    Err(invalid(
        "output_matches_input",
        format!(
            "selected input exceeds {} available inputs",
            args.input_count()
        ),
    ))
}

fn validate_matrix_product(args: &FunctionArgs) -> MResult<()> {
    let output = required_output(args, "matrix_product")?;
    let lhs = required_input(args, 0, "matrix_product")?;
    let rhs = required_input(args, 1, "matrix_product")?;
    if lhs.cols != rhs.rows || output.rows != lhs.rows || output.cols != rhs.cols {
        return Err(invalid(
            "matrix_product",
            format!(
                "output {}x{}, lhs {}x{}, rhs {}x{}",
                output.rows, output.cols, lhs.rows, lhs.cols, rhs.rows, rhs.cols,
            ),
        ));
    }
    Ok(())
}

fn validate_transpose(args: &FunctionArgs) -> MResult<()> {
    let output = required_output(args, "transpose")?;
    let input = required_input(args, 0, "transpose")?;
    if output.rows != input.cols || output.cols != input.rows {
        return Err(invalid(
            "transpose",
            format!(
                "output {}x{} is not the transpose of input {}x{}",
                output.rows, output.cols, input.rows, input.cols,
            ),
        ));
    }
    Ok(())
}

fn validate_horizontal_concatenation(args: &FunctionArgs) -> MResult<()> {
    let output = required_output(args, "horizontal_concatenation")?;
    let (first_rows, _) = logical_input_dimensions(args, 0, "horizontal_concatenation")?;
    let mut cols = 0usize;
    for index in 0..args.input_count() {
        let (rows, input_cols) = logical_input_dimensions(args, index, "horizontal_concatenation")?;
        if rows != first_rows {
            return Err(invalid(
                "horizontal_concatenation",
                format!("input {index} has {rows} rows, expected {first_rows}"),
            ));
        }
        cols = cols.checked_add(input_cols).ok_or_else(|| {
            invalid(
                "horizontal_concatenation",
                "input column sum overflowed usize",
            )
        })?;
    }
    if output.rows != first_rows || output.cols != cols {
        return Err(invalid(
            "horizontal_concatenation",
            format!(
                "output is {}x{}, expected {}x{}",
                output.rows, output.cols, first_rows, cols
            ),
        ));
    }
    Ok(())
}

fn validate_vertical_concatenation(args: &FunctionArgs) -> MResult<()> {
    let output = required_output(args, "vertical_concatenation")?;
    let (_, first_cols) = logical_input_dimensions(args, 0, "vertical_concatenation")?;
    let mut rows = 0usize;
    for index in 0..args.input_count() {
        let (input_rows, cols) = logical_input_dimensions(args, index, "vertical_concatenation")?;
        if cols != first_cols {
            return Err(invalid(
                "vertical_concatenation",
                format!("input {index} has {cols} columns, expected {first_cols}"),
            ));
        }
        rows = rows
            .checked_add(input_rows)
            .ok_or_else(|| invalid("vertical_concatenation", "input row sum overflowed usize"))?;
    }
    if output.rows != rows || output.cols != first_cols {
        return Err(invalid(
            "vertical_concatenation",
            format!(
                "output is {}x{}, expected {}x{}",
                output.rows, output.cols, rows, first_cols
            ),
        ));
    }
    Ok(())
}

fn validate_linear_solve(args: &FunctionArgs) -> MResult<()> {
    let output = required_output(args, "linear_solve")?;
    let a = required_input(args, 0, "linear_solve")?;
    let b = required_input(args, 1, "linear_solve")?;
    if a.rows != a.cols || b.rows != a.rows || output.rows != a.cols || output.cols != b.cols {
        return Err(invalid(
            "linear_solve",
            format!(
                "output {}x{}, A {}x{}, B {}x{}",
                output.rows, output.cols, a.rows, a.cols, b.rows, b.cols,
            ),
        ));
    }
    Ok(())
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
        CanonicalContractValidator::Unsupported => {
            return Err(invalid(
                contract.kind,
                "custom contract has no canonical validator",
            ));
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

#[cfg(all(
    test,
    feature = "f64",
    feature = "matrix",
    feature = "matrix2",
    feature = "matrixd"
))]
mod tests {
    use super::*;
    use crate::structures::Matrix;
    use crate::{LegacyValue, Ref};
    use nalgebra::{DMatrix, Matrix2};

    fn dynamic(rows: usize, cols: usize) -> LegacyValue {
        LegacyValue::MatrixF64(Matrix::DMatrix(Ref::new(DMatrix::zeros(rows, cols))))
    }

    fn fixed2() -> LegacyValue {
        LegacyValue::MatrixF64(Matrix::Matrix2(Ref::new(Matrix2::zeros())))
    }

    fn disallow(contract: RuntimeFunctionContract, args: FunctionArgs) -> crate::MechError {
        args.validate_contract(contract).unwrap_err()
    }

    fn legacy_only_custom_validator(_args: &FunctionArgs) -> MResult<()> {
        Ok(())
    }

    #[test]
    fn unregistered_custom_canonical_validator_fails_closed() {
        let contract = RuntimeFunctionContract::custom(
            "legacy_only_custom",
            RuntimeOutputAliasPolicy::DisallowInputAlias,
            legacy_only_custom_validator,
        );
        let invocation = crate::FunctionInvocation::unary(
            ValueCell::from_exact(0.0_f64).unwrap(),
            ValueCell::from_exact(1.0_f64).unwrap(),
        );
        let error = invocation.validate_contract(contract).unwrap_err();
        assert_eq!(error.kind_name(), "FunctionShapeContractViolation");
        assert!(error.kind_message().contains("no canonical validator"));
    }

    #[test]
    fn same_shape_rejects_dynamic_input_and_output_mismatches_without_mutation() {
        let contract =
            RuntimeFunctionContract::same_shape(RuntimeOutputAliasPolicy::DisallowInputAlias);
        let output = dynamic(2, 2);
        let lhs = dynamic(2, 2);
        let rhs = dynamic(3, 3);
        let before = output.clone();
        let error = disallow(contract, FunctionArgs::Binary(output.clone(), lhs, rhs));
        assert_eq!(error.kind_name(), "FunctionShapeContractViolation");
        assert_eq!(output, before);

        let error = disallow(
            contract,
            FunctionArgs::Binary(dynamic(3, 3), dynamic(2, 2), dynamic(2, 2)),
        );
        assert_eq!(error.kind_name(), "FunctionShapeContractViolation");
    }

    #[test]
    fn matrix_outputs_cannot_alias_either_input_for_dynamic_or_fixed_storage() {
        let contract =
            RuntimeFunctionContract::same_shape(RuntimeOutputAliasPolicy::DisallowInputAlias);
        let lhs = dynamic(2, 2);
        let error = disallow(
            contract,
            FunctionArgs::Binary(lhs.clone(), lhs, dynamic(2, 2)),
        );
        assert_eq!(error.kind_name(), "FunctionArgumentAliasViolation");

        let rhs = dynamic(2, 2);
        let error = disallow(
            contract,
            FunctionArgs::Binary(rhs.clone(), dynamic(2, 2), rhs),
        );
        assert_eq!(error.kind_name(), "FunctionArgumentAliasViolation");

        let lhs = fixed2();
        let error = disallow(contract, FunctionArgs::Binary(lhs.clone(), lhs, fixed2()));
        assert_eq!(error.kind_name(), "FunctionArgumentAliasViolation");
    }

    #[test]
    fn matrix_product_and_linear_solve_validate_every_dimension_relation() {
        let product =
            RuntimeFunctionContract::matrix_product(RuntimeOutputAliasPolicy::DisallowInputAlias);
        assert_eq!(
            disallow(
                product,
                FunctionArgs::Binary(dynamic(2, 2), dynamic(2, 3), dynamic(4, 2)),
            )
            .kind_name(),
            "FunctionShapeContractViolation",
        );
        assert_eq!(
            disallow(
                product,
                FunctionArgs::Binary(dynamic(2, 4), dynamic(2, 3), dynamic(3, 2)),
            )
            .kind_name(),
            "FunctionShapeContractViolation",
        );

        let solve =
            RuntimeFunctionContract::linear_solve(RuntimeOutputAliasPolicy::DisallowInputAlias);
        assert_eq!(
            disallow(
                solve,
                FunctionArgs::Binary(dynamic(3, 1), dynamic(2, 3), dynamic(2, 1)),
            )
            .kind_name(),
            "FunctionShapeContractViolation",
        );
        assert_eq!(
            disallow(
                solve,
                FunctionArgs::Binary(dynamic(2, 1), dynamic(2, 2), dynamic(3, 1)),
            )
            .kind_name(),
            "FunctionShapeContractViolation",
        );
    }
}
