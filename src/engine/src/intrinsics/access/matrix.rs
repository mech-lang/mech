use crate::intrinsics::*;
use nalgebra::{
    Dim,
    base::{Matrix as naMatrix, Storage, StorageMut},
};
use std::fmt::Debug;

macro_rules! optional_operation_contract {
    () => {
        None
    };
    ($contract:path) => {
        Some(&*$contract)
    };
}
use std::sync::LazyLock;

fn matrix_selection_contract(
    input_count: usize,
    _postcondition_name: &'static str,
) -> OperationContractDeclaration {
    OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                };
                input_count
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
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    }
}

macro_rules! declare_matrix_selection_contract {
    ($name:ident, $input_count:literal, $postcondition:literal) => {
        static $name: LazyLock<OperationContractDeclaration> =
            LazyLock::new(|| matrix_selection_contract($input_count, $postcondition));
    };
}

declare_matrix_selection_contract!(PURE_BINARY_SCALAR_INDEX_CONTRACT, 2, "scalar-index-output");
declare_matrix_selection_contract!(
    PURE_TERNARY_SCALAR_SCALAR_CONTRACT,
    3,
    "scalar-row-scalar-column-output"
);
declare_matrix_selection_contract!(
    PURE_BINARY_EXPLICIT_INDEX_CONTRACT,
    2,
    "explicit-index-vector-output"
);
#[cfg(feature = "logical_indexing")]
declare_matrix_selection_contract!(PURE_BINARY_LOGICAL_MASK_CONTRACT, 2, "logical-mask-output");
declare_matrix_selection_contract!(PURE_BINARY_ALL_ELEMENTS_CONTRACT, 2, "all-elements-output");
declare_matrix_selection_contract!(
    PURE_BINARY_ALL_ROWS_SCALAR_COLUMN_CONTRACT,
    2,
    "all-rows-scalar-column-output"
);
declare_matrix_selection_contract!(
    PURE_BINARY_SCALAR_ROW_ALL_COLUMNS_CONTRACT,
    2,
    "scalar-row-all-columns-output"
);
declare_matrix_selection_contract!(
    PURE_BINARY_EXPLICIT_ROWS_ALL_COLUMNS_CONTRACT,
    2,
    "explicit-rows-all-columns-output"
);
#[cfg(feature = "logical_indexing")]
declare_matrix_selection_contract!(
    PURE_BINARY_LOGICAL_ROWS_ALL_COLUMNS_CONTRACT,
    2,
    "logical-rows-all-columns-output"
);
declare_matrix_selection_contract!(
    PURE_BINARY_ALL_ROWS_EXPLICIT_COLUMNS_CONTRACT,
    2,
    "all-rows-explicit-columns-output"
);
declare_matrix_selection_contract!(
    PURE_BINARY_ALL_ROWS_LOGICAL_COLUMNS_CONTRACT,
    2,
    "all-rows-logical-columns-output"
);
declare_matrix_selection_contract!(
    PURE_TERNARY_SCALAR_ROW_EXPLICIT_COLUMNS_CONTRACT,
    3,
    "scalar-row-explicit-columns-output"
);
#[cfg(feature = "logical_indexing")]
declare_matrix_selection_contract!(
    PURE_TERNARY_SCALAR_ROW_LOGICAL_COLUMNS_CONTRACT,
    3,
    "scalar-row-logical-columns-output"
);
declare_matrix_selection_contract!(
    PURE_TERNARY_EXPLICIT_ROWS_SCALAR_COLUMN_CONTRACT,
    3,
    "explicit-rows-scalar-column-output"
);
#[cfg(feature = "logical_indexing")]
declare_matrix_selection_contract!(
    PURE_TERNARY_LOGICAL_ROWS_SCALAR_COLUMN_CONTRACT,
    3,
    "logical-rows-scalar-column-output"
);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MatrixAccessSelection {
    Scalar,
    All,
    Explicit(usize),
    Logical(usize),
}

impl MatrixAccessSelection {
    fn count(self, upper: usize) -> usize {
        match self {
            Self::Scalar => 1,
            Self::All => upper,
            Self::Explicit(count) | Self::Logical(count) => count,
        }
    }
}

fn matrix_access_selection(
    value: &ValueCell,
    upper: usize,
    input_index: usize,
) -> MResult<MatrixAccessSelection> {
    let contract = "matrix_access";
    let snapshot = match value.snapshot() {
        Ok(snapshot) => snapshot,
        Err(_) => return Ok(MatrixAccessSelection::All),
    };
    match snapshot.data() {
        ValueData::Index(value) => {
            let found = *value as usize;
            if found == 0 || found > upper {
                return Err(function_shape_contract_violation(
                    contract,
                    format!("input {input_index} index {found} is outside 1..={upper}"),
                ));
            }
            Ok(MatrixAccessSelection::Scalar)
        }
        ValueData::Matrix(_) => {
            let elements = value.matrix_elements()?.ok_or_else(|| {
                function_shape_contract_violation(
                    contract,
                    format!("input {input_index} matrix selector has no elements"),
                )
            })?;
            if elements.first().is_some_and(|element| {
                matches!(
                    element.snapshot().map(|value| value.data().clone()),
                    Ok(ValueData::Bool(_))
                )
            }) {
                if elements.len() != upper {
                    return Err(function_shape_contract_violation(
                        contract,
                        format!(
                            "input {input_index} logical mask has {} elements, expected {upper}",
                            elements.len(),
                        ),
                    ));
                }
                let mut selected = 0;
                for element in elements.iter() {
                    if matches!(element.snapshot()?.data(), ValueData::Bool(true)) {
                        selected += 1;
                    }
                }
                return Ok(MatrixAccessSelection::Logical(selected));
            }
            for element in elements.iter() {
                let snapshot = element.snapshot()?;
                let ValueData::Index(found) = snapshot.data() else {
                    return Err(function_shape_contract_violation(
                        contract,
                        format!("input {input_index} matrix selector must contain indices"),
                    ));
                };
                let found = *found as usize;
                if found == 0 || found > upper {
                    return Err(function_shape_contract_violation(
                        contract,
                        format!("input {input_index} index {found} is outside 1..={upper}"),
                    ));
                }
            }
            Ok(MatrixAccessSelection::Explicit(elements.len()))
        }
        _ => Err(function_shape_contract_violation(
            contract,
            format!("input {input_index} must be a scalar index, index vector, or logical mask"),
        )),
    }
}

fn matrix_descriptor(value: &ValueCell) -> MResult<Option<FunctionMatrixDescriptor>> {
    let FunctionValueRepresentation::Matrix { storage, .. } = value.representation() else {
        return Ok(None);
    };
    let SchemaBody::Matrix { dimensions, .. } = value.closed_schema_body()? else {
        return Ok(None);
    };
    let [
        DimensionExpr::Constant(rows),
        DimensionExpr::Constant(columns),
    ] = dimensions.as_ref()
    else {
        unreachable!("closed matrix dimensions are constant")
    };
    let representation = match storage {
        FunctionMatrixStoragePattern::Exact(representation) => representation,
        FunctionMatrixStoragePattern::AnyStorage => FunctionMatrixRepresentation::MatrixD,
    };
    Ok(Some(FunctionMatrixDescriptor {
        representation,
        rows: *rows as usize,
        cols: *columns as usize,
    }))
}

fn matrix_access_binary_output_shape(
    source: FunctionMatrixDescriptor,
    selection: MatrixAccessSelection,
    output: Option<FunctionMatrixDescriptor>,
) -> MResult<(usize, usize)> {
    let contract = "matrix_access";
    use FunctionMatrixRepresentation::*;

    match (
        selection,
        output.map(|descriptor| descriptor.representation),
    ) {
        (MatrixAccessSelection::Scalar, None) => Ok((1, 1)),
        (MatrixAccessSelection::Scalar, Some(VectorD)) => Ok((source.rows, 1)),
        (
            MatrixAccessSelection::Scalar,
            Some(RowVector2 | RowVector3 | RowVector4 | RowVectorD | Matrix1),
        ) => Ok((1, source.cols)),
        (MatrixAccessSelection::All, Some(VectorD)) => Ok((
            source.rows.checked_mul(source.cols).ok_or_else(|| {
                function_shape_contract_violation(contract, "source element count overflowed")
            })?,
            1,
        )),
        (
            MatrixAccessSelection::Explicit(count) | MatrixAccessSelection::Logical(count),
            Some(VectorD),
        ) => Ok((count, 1)),
        (
            MatrixAccessSelection::Explicit(count) | MatrixAccessSelection::Logical(count),
            Some(MatrixD),
        ) => Ok((count, source.cols)),
        (selection, output) => Err(function_shape_contract_violation(
            contract,
            format!(
                "selector {selection:?} is incompatible with binary output representation {output:?}"
            ),
        )),
    }
}

fn matrix_access_binary_upper_bound(
    source: FunctionMatrixDescriptor,
    selector: &ValueCell,
    output: Option<FunctionMatrixDescriptor>,
) -> MResult<usize> {
    use FunctionMatrixRepresentation::*;

    let snapshot = match selector.snapshot() {
        Ok(snapshot) => snapshot,
        Err(_) => {
            return source.rows.checked_mul(source.cols).ok_or_else(|| {
                function_shape_contract_violation(
                    "matrix_access",
                    "source element count overflowed",
                )
            });
        }
    };
    match (
        snapshot.data(),
        output.map(|descriptor| descriptor.representation),
    ) {
        (ValueData::Index(_), Some(VectorD)) => Ok(source.cols),
        (
            ValueData::Index(_),
            Some(RowVector2 | RowVector3 | RowVector4 | RowVectorD | Matrix1),
        )
        | (ValueData::Matrix(_), Some(MatrixD)) => Ok(source.rows),
        _ => source.rows.checked_mul(source.cols).ok_or_else(|| {
            function_shape_contract_violation("matrix_access", "source element count overflowed")
        }),
    }
}

fn validate_matrix_access_contract_impl(
    output_value: &ValueCell,
    inputs: &[ValueCell],
    require_exact_output_shape: bool,
) -> MResult<()> {
    let contract = "matrix_access";
    let source_value = inputs
        .first()
        .ok_or_else(|| function_shape_contract_violation(contract, "missing matrix input"))?;
    let source = matrix_descriptor(source_value)?.ok_or_else(|| {
        function_shape_contract_violation(contract, "input 0 must be matrix-backed")
    })?;
    let output = matrix_descriptor(output_value)?;
    let output_shape = output
        .map(|descriptor| (descriptor.rows, descriptor.cols))
        .unwrap_or((1, 1));
    let (expected_rows, expected_cols) = match inputs.len() {
        2 => {
            let selector = inputs
                .get(1)
                .ok_or_else(|| function_shape_contract_violation(contract, "missing input 1"))?;
            let upper = matrix_access_binary_upper_bound(source, selector, output)?;
            let selection = matrix_access_selection(selector, upper, 1)?;
            matrix_access_binary_output_shape(source, selection, output)?
        }
        3 => {
            let rows = matrix_access_selection(
                inputs.get(1).ok_or_else(|| {
                    function_shape_contract_violation(contract, "missing input 1")
                })?,
                source.rows,
                1,
            )?
            .count(source.rows);
            let cols = matrix_access_selection(
                inputs.get(2).ok_or_else(|| {
                    function_shape_contract_violation(contract, "missing input 2")
                })?,
                source.cols,
                2,
            )?
            .count(source.cols);
            (rows, cols)
        }
        found => {
            return Err(function_shape_contract_violation(
                contract,
                format!("expected 2 or 3 inputs including the source, found {found}"),
            ));
        }
    };
    if require_exact_output_shape
        && (output_shape.0 != expected_rows || output_shape.1 != expected_cols)
    {
        return Err(function_shape_contract_violation(
            contract,
            format!(
                "output is {}x{}, selected indices require {expected_rows}x{expected_cols}",
                output_shape.0, output_shape.1,
            ),
        ));
    }
    Ok(())
}

fn validate_matrix_access_contract(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
    let has_logical_selector = inputs.iter().skip(1).any(|input| {
        input
            .matrix_elements()
            .ok()
            .flatten()
            .is_some_and(|elements| {
                elements.first().is_some_and(|element| {
                    matches!(
                        element.snapshot().map(|value| value.data().clone()),
                        Ok(ValueData::Bool(_))
                    )
                })
            })
    });
    validate_matrix_access_contract_impl(output, inputs, !has_logical_selector)
}

fn validate_matrix_access_all_range_contract(
    output_value: &ValueCell,
    inputs: &[ValueCell],
) -> MResult<()> {
    let contract = "matrix_access_all_range";
    if inputs.len() != 2 {
        return Err(function_shape_contract_violation(
            contract,
            format!(
                "expected 2 inputs including the source, found {}",
                inputs.len(),
            ),
        ));
    }
    let source = matrix_descriptor(
        inputs
            .first()
            .ok_or_else(|| function_shape_contract_violation(contract, "missing matrix input"))?,
    )?
    .ok_or_else(|| function_shape_contract_violation(contract, "input 0 must be matrix-backed"))?;
    let columns = matrix_access_selection(
        inputs
            .get(1)
            .ok_or_else(|| function_shape_contract_violation(contract, "missing input 1"))?,
        source.cols,
        1,
    )?
    .count(source.cols);
    let output_shape = matrix_descriptor(output_value)?
        .map(|descriptor| (descriptor.rows, descriptor.cols))
        .unwrap_or((1, 1));
    if output_shape != (source.rows, columns) {
        return Err(function_shape_contract_violation(
            contract,
            format!(
                "output is {}x{}, selected columns require {}x{}",
                output_shape.0, output_shape.1, source.rows, columns,
            ),
        ));
    }
    Ok(())
}

fn validate_matrix_access_all_elements_contract(
    output: &ValueCell,
    inputs: &[ValueCell],
) -> MResult<()> {
    let source = inputs
        .first()
        .ok_or_else(|| function_shape_contract_violation("matrix_access", "missing input 0"))?;
    let source = matrix_descriptor(source)?.ok_or_else(|| {
        function_shape_contract_violation("matrix_access", "input 0 must be matrix-backed")
    })?;
    let expected = source.rows.saturating_mul(source.cols);
    let output_shape = matrix_descriptor(output)?
        .map(|descriptor| (descriptor.rows, descriptor.cols))
        .unwrap_or((1, 1));
    if output_shape != (expected, 1) {
        return Err(function_shape_contract_violation(
            "matrix_access",
            format!("output has shape {output_shape:?}, expected {expected}x1"),
        ));
    }
    Ok(())
}

#[cfg(all(test, feature = "u8", feature = "matrixd", feature = "vectord"))]
mod matrix_access_contract_tests {
    use super::*;

    fn matrix(rows: usize, cols: usize) -> ValueCell {
        ValueCell::from_exact_matrix_ref(
            Ref::new(DMatrix::<u8>::from_element(rows, cols, 0)),
            rows,
            cols,
        )
        .unwrap()
    }

    fn indices(values: Vec<usize>) -> ValueCell {
        let len = values.len();
        ValueCell::from_exact_matrix_ref(Ref::new(DVector::from_vec(values)), len, 1).unwrap()
    }

    fn vector(len: usize) -> ValueCell {
        ValueCell::from_exact_matrix_ref(Ref::new(DVector::<u8>::from_element(len, 0)), len, 1)
            .unwrap()
    }

    fn index(value: usize) -> ValueCell {
        ValueCell::from_exact(value).unwrap()
    }

    #[test]
    fn exact_contract_rejects_linear_output_with_wrong_selected_length() {
        let result =
            validate_matrix_access_contract(&vector(1), &[matrix(2, 2), indices(vec![1, 2, 3])]);

        assert!(result.is_err());
    }

    #[test]
    fn exact_contract_checks_scalar_column_against_column_count() {
        let result = validate_matrix_access_contract(&vector(2), &[matrix(2, 2), index(3)]);

        assert!(result.is_err());
    }

    #[test]
    fn exact_contract_rejects_two_dimensional_output_with_wrong_orientation() {
        let result = validate_matrix_access_contract(
            &matrix(1, 6),
            &[matrix(3, 3), indices(vec![1, 2]), indices(vec![1, 2, 3])],
        );

        assert!(result.is_err());
    }

    #[test]
    fn all_range_contract_rejects_selected_row_orientation() {
        let result = validate_matrix_access_all_range_contract(
            &matrix(2, 4),
            &[matrix(3, 4), indices(vec![1, 2])],
        );

        assert!(result.is_err());
    }

    #[test]
    fn reactive_numeric_selector_cannot_outgrow_fixed_output() {
        let source = Ref::new(DMatrix::from_row_slice(2, 2, &[10_u8, 20, 30, 40]));
        let ixes = Ref::new(DVector::from_vec(vec![1_usize, 2]));
        let out = Ref::new(DVector::from_element(2, 0_u8));
        let function = Access1DVDMD::<u8>::new_invocation(FunctionInvocation::binary(
            ValueCell::from_exact_matrix_ref(out.clone(), 2, 1).unwrap(),
            ValueCell::from_exact_matrix_ref(source, 2, 2).unwrap(),
            ValueCell::from_exact_matrix_ref(ixes.clone(), 2, 1).unwrap(),
        ))
        .unwrap();

        function.solve_result().unwrap();
        assert_eq!(out.borrow().as_slice(), &[10, 30]);

        *ixes.borrow_mut() = DVector::from_vec(vec![1_usize, 2, 3]);
        assert!(function.solve_result().is_err());
        assert_eq!(out.borrow().as_slice(), &[10, 30]);
    }

    #[cfg(feature = "bool")]
    #[test]
    fn exact_contract_rejects_logical_mask_with_wrong_axis_length() {
        let mask = ValueCell::from_exact_matrix_ref(Ref::new(DVector::from_vec(vec![true])), 1, 1)
            .unwrap();
        let result = validate_matrix_access_contract(
            &matrix(1, 2),
            &[matrix(2, 2), mask, indices(vec![1, 2])],
        );

        assert!(result.is_err());
    }

    #[cfg(feature = "bool")]
    #[test]
    fn reactive_logical_linear_selection_regrows_from_empty() {
        let source = Ref::new(DVector::from_vec(vec![10_u8, 20, 30]));
        let ixes = Ref::new(DVector::from_vec(vec![true, false, true]));
        let out = Ref::new(DVector::from_element(2, 0_u8));
        let function = Access1DVDbVD::<u8>::new_invocation(FunctionInvocation::binary(
            ValueCell::from_exact_matrix_ref(out.clone(), 2, 1).unwrap(),
            ValueCell::from_exact_matrix_ref(source, 3, 1).unwrap(),
            ValueCell::from_exact_matrix_ref(ixes.clone(), 3, 1).unwrap(),
        ))
        .unwrap();

        function.solve_result().unwrap();
        assert_eq!(out.borrow().as_slice(), &[10, 30]);

        *ixes.borrow_mut() = DVector::from_vec(vec![false, false, false]);
        function.solve_result().unwrap();
        assert!(out.borrow().is_empty());

        *ixes.borrow_mut() = DVector::from_vec(vec![false, true, false]);
        function.solve_result().unwrap();
        assert_eq!(out.borrow().as_slice(), &[20]);
    }

    #[cfg(feature = "bool")]
    #[test]
    fn reactive_logical_matrix_selection_regrows_from_empty() {
        let source = Ref::new(DMatrix::from_row_slice(3, 2, &[10_u8, 11, 20, 21, 30, 31]));
        let ixes = Ref::new(DVector::from_vec(vec![true, false, true]));
        let out = Ref::new(DMatrix::from_element(2, 2, 0_u8));
        let function = Access2DVDbAMD::<u8>::new_invocation(FunctionInvocation::binary(
            ValueCell::from_exact_matrix_ref(out.clone(), 2, 2).unwrap(),
            ValueCell::from_exact_matrix_ref(source, 3, 2).unwrap(),
            ValueCell::from_exact_matrix_ref(ixes.clone(), 3, 1).unwrap(),
        ))
        .unwrap();

        function.solve_result().unwrap();
        assert_eq!(
            *out.borrow(),
            DMatrix::from_row_slice(2, 2, &[10, 11, 30, 31])
        );

        *ixes.borrow_mut() = DVector::from_vec(vec![false, false, false]);
        function.solve_result().unwrap();
        assert_eq!(out.borrow().shape(), (0, 2));

        *ixes.borrow_mut() = DVector::from_vec(vec![false, true, false]);
        function.solve_result().unwrap();
        assert_eq!(*out.borrow(), DMatrix::from_row_slice(1, 2, &[20, 21]));
    }
}

macro_rules! access_1d {
    ($source:expr, $ix:expr, $out:expr) => {
        unsafe { *$out = (*$source).index(*$ix - 1).clone() }
    };
}

macro_rules! access_2d {
    ($source:expr, $ix1:expr, $ix2:expr, $out:expr) => {
        unsafe { *$out = (*$source).index((*$ix1 - 1, *$ix2 - 1)).clone() }
    };
}
macro_rules! access_1d_slice {
    ($source:expr, $ix:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$ix).len() {
                (&mut *$out)[i] = (*$source).index((&(*$ix))[i] - 1).clone();
            }
        }
    };
}

#[cfg(feature = "logical_indexing")]
macro_rules! access_1d_slice_bool_v {
    ($source:expr, $ix:expr, $out:expr) => {
        unsafe {
            let mut selected = Vec::new();
            for i in 0..(*$ix).len() {
                if (&(*$ix))[i] {
                    selected.push((*$source).index(i).clone());
                }
            }
            *$out = DVector::from_vec(selected);
        }
    };
}

#[cfg(feature = "logical_indexing")]
macro_rules! access_2d_row_slice_bool {
    ($source:expr, $ix1:expr, $ix2:expr, $out:expr) => {
        unsafe {
            let scalar_ix = &(*$ix1);
            let vec_ix = &(*$ix2);
            let mut selected = Vec::new();
            for i in 0..vec_ix.len() {
                if vec_ix[i] {
                    selected.push((*$source).index((scalar_ix - 1, i)).clone());
                }
            }
            *$out = RowDVector::from_row_slice(&selected);
        }
    };
}

#[cfg(feature = "logical_indexing")]
macro_rules! access_2d_col_slice_bool {
    ($source:expr, $ix1:expr, $ix2:expr, $out:expr) => {
        unsafe {
            let vec_ix = &(*$ix1);
            let scalar_ix = &(*$ix2);
            let mut selected = Vec::new();
            for i in 0..vec_ix.len() {
                if vec_ix[i] {
                    selected.push((*$source).index((i, scalar_ix - 1)).clone());
                }
            }
            *$out = DVector::from_vec(selected);
        }
    };
}

macro_rules! access_2d_slice_all {
    ($source:expr, $ix:expr, $out:expr) => {
        unsafe {
            let n_cols = (*$source).ncols();
            let n_rows = (*$ix).nrows();
            let mut out_ix = 0;
            for c in 0..n_cols {
                for r in 0..n_rows {
                    (&mut (*$out))[out_ix] = (*$source).index(((&(*$ix))[r] - 1, c)).clone();
                    out_ix += 1;
                }
            }
        }
    };
}

#[cfg(feature = "logical_indexing")]
macro_rules! access_2d_slice_all_bool {
    ($source:expr, $ix:expr, $out:expr) => {
        unsafe {
            let vec_ix = &(*$ix);
            let rows = vec_ix.iter().filter(|selected| **selected).count();
            let cols = (*$source).ncols();
            let mut selected = Vec::with_capacity(rows.saturating_mul(cols));
            for k in 0..cols {
                for i in 0..vec_ix.len() {
                    if vec_ix[i] {
                        selected.push((*$source).index((i, k)).clone());
                    }
                }
            }
            *$out = DMatrix::from_column_slice(rows, cols, &selected);
        }
    };
}

macro_rules! access_2d_row_slice {
    ($source:expr, $ix1:expr, $ix2:expr, $out:expr) => {
        unsafe {
            let ix1 = &(*$ix1);
            let ix2 = &(*$ix2);
            let out_cols = ix2.nrows();
            let mut out_ix = 0;
            for c in 0..out_cols {
                (&mut (*$out))[out_ix] = (*$source).index((ix1 - 1, ix2[c] - 1)).clone();
                out_ix += 1;
            }
        }
    };
}

macro_rules! access_2d_col_slice {
    ($source:expr, $ix1:expr, $ix2:expr, $out:expr) => {
        unsafe {
            let ix1 = &(*$ix1);
            let ix2 = &(*$ix2);
            let out_rows = ix1.nrows();
            let mut out_ix = 0;
            for c in 0..out_rows {
                (&mut (*$out))[out_ix] = (*$source).index((ix1[c] - 1, ix2 - 1)).clone();
                out_ix += 1;
            }
        }
    };
}

macro_rules! access_col {
    ($source:expr, $ix:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$source).nrows() {
                (&mut (*$out))[i] = (*$source).index((i, *$ix - 1)).clone();
            }
        }
    };
}

macro_rules! access_row {
    ($source:expr, $ix:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$source).ncols() {
                (&mut (*$out))[i] = (*$source).index((*$ix - 1, i)).clone();
            }
        }
    };
}

macro_rules! access_1d_all {
    ($source:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$source).len() {
                (&mut (*$out))[i] = (*$source).index(i).clone();
            }
        }
    };
}

macro_rules! solve_access_1d {
    (access_1d_all, $source:expr, $indexes:expr, $out:expr) => {{
        // `IndexAll` participates in validation and bytecode identity but carries
        // no data for the copy kernel itself.
        access_1d_all!($source, $out)
    }};
    ($operation:ident, $source:expr, $indexes:expr, $out:expr) => {{
        let indexes = $indexes.as_ptr();
        $operation!($source, indexes, $out)
    }};
}

macro_rules! impl_access_fxn {
    ($struct_name:ident, $arg_type:ty, $ix_type:ty, $out_type:ty, $op:ident, $contract:ident) => {
        #[derive(Debug)]
        struct $struct_name<T> {
            source: Ref<$arg_type>,
            ixes: Ref<$ix_type>,
            out: Ref<$out_type>,
            invocation: FunctionInvocation,
        }
        impl<T> MechFunctionFactory for $struct_name<T>
        where
            T: Debug
                + Clone
                + Sync
                + Send
                + PartialEq
                + 'static
                + ConstElem
                + FunctionRuntimeType
                + CanonicalMatrixElementBacking,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst + CanonicalMatrixElementBacking,
            $arg_type: FunctionPortBacking,
            $ix_type: FunctionPortBacking,
            $out_type: FunctionStateBacking,
        {
            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
            }

            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <$out_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg_type as FunctionRuntimeType>::REPRESENTATION,
                <$ix_type as FunctionRuntimeType>::REPRESENTATION,
            );

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, source, ixes) = invocation.expect_binary()?;
                let source: Ref<$arg_type> = source.try_ref()?;
                let ixes: Ref<$ix_type> = ixes.try_ref()?;
                let out: Ref<$out_type> = out.try_ref()?;
                Ok(Box::new($struct_name {
                    source,
                    ixes,
                    out,
                    invocation,
                }))
            }
        }
        impl<T> MechFunctionImpl for $struct_name<T>
        where
            T: Debug + Clone + Sync + Send + PartialEq + 'static,
            $out_type: FunctionStateBacking,
        {
            fn solve_result(&self) -> MResult<()> {
                validate_matrix_access_contract(
                    self.invocation.output_cell(),
                    self.invocation.input_cells(),
                )?;
                let source_ptr = self.source.as_ptr();
                let out_ptr = self.out.as_mut_ptr();
                solve_access_1d!($op, source_ptr, self.ixes, out_ptr);
                Ok(())
            }
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_ref(&self.out))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_ref(&self.out)]))
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                Some(&$contract)
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl<T> MechFunctionCompiler for $struct_name<T>
        where
            T: CompileConst + ConstElem + FunctionRuntimeType + CanonicalMatrixElementBacking,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}>",
                    stringify!($struct_name),
                    <T as FunctionRuntimeType>::REPRESENTATION
                );
                compile_binop!(name, self.out, self.source, self.ixes, ctx);
            }
        }
    };
}

macro_rules! impl_access_all_fxn {
    ($struct_name:ident, $arg_type:ty, $out_type:ty, $contract:ident) => {
        #[derive(Debug)]
        struct $struct_name<T> {
            source: Ref<$arg_type>,
            out: Ref<$out_type>,
            invocation: FunctionInvocation,
        }

        impl<T> MechFunctionFactory for $struct_name<T>
        where
            T: Debug
                + Clone
                + Sync
                + Send
                + PartialEq
                + 'static
                + ConstElem
                + FunctionRuntimeType
                + CanonicalMatrixElementBacking,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst + CanonicalMatrixElementBacking,
            $arg_type: FunctionPortBacking,
            $out_type: FunctionStateBacking,
        {
            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
            }

            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <$out_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg_type as FunctionRuntimeType>::REPRESENTATION,
                FunctionValueRepresentation::AnyValue,
            );

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, source, _all) = invocation.expect_binary()?;
                let source: Ref<$arg_type> = source.try_ref()?;
                let out: Ref<$out_type> = out.try_ref()?;
                Ok(Box::new($struct_name {
                    source,
                    out,
                    invocation,
                }))
            }
        }

        impl<T> MechFunctionImpl for $struct_name<T>
        where
            T: Debug + Clone + Sync + Send + PartialEq + 'static,
            $out_type: FunctionStateBacking,
        {
            fn solve_result(&self) -> MResult<()> {
                validate_matrix_access_all_elements_contract(
                    self.invocation.output_cell(),
                    self.invocation.input_cells(),
                )?;
                access_1d_all!(self.source.as_ptr(), self.out.as_mut_ptr());
                Ok(())
            }

            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_ref(&self.out))
            }

            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_ref(&self.out)]))
            }

            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                Some(&$contract)
            }

            fn to_string(&self) -> String {
                format!("{self:#?}")
            }
        }

        #[cfg(feature = "semantic-compiler")]
        impl<T> MechFunctionCompiler for $struct_name<T>
        where
            T: CompileConst + ConstElem + FunctionRuntimeType + CanonicalMatrixElementBacking,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let out = compile_register_brrw!(self.out, ctx);
                let source = compile_register_brrw!(self.source, ctx);
                let all = self
                    .invocation
                    .input(1)
                    .expect("all-selection input")
                    .value()
                    .compile_register(ctx)?;
                let function = ctx.function_id(&format!(
                    "{}<{}>",
                    stringify!($struct_name),
                    <T as FunctionRuntimeType>::REPRESENTATION
                ))?;
                ctx.emit_binop(function, out, source, all);
                Ok(out)
            }
        }
    };
}

macro_rules! impl_access_fxn2 {
    ($struct_name:ident, $arg_type:ty, $ix1_type:ty, $ix2_type:ty, $out_type:ty, $op:ident, $contract:ident) => {
        #[derive(Debug)]
        struct $struct_name<T> {
            source: Ref<$arg_type>,
            ix1: Ref<$ix1_type>,
            ix2: Ref<$ix2_type>,
            out: Ref<$out_type>,
            invocation: FunctionInvocation,
        }
        impl<T> MechFunctionFactory for $struct_name<T>
        where
            T: Debug
                + Clone
                + Sync
                + Send
                + PartialEq
                + 'static
                + ConstElem
                + FunctionRuntimeType
                + CanonicalMatrixElementBacking,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst + CanonicalMatrixElementBacking,
            $arg_type: FunctionPortBacking,
            $ix1_type: FunctionPortBacking,
            $ix2_type: FunctionPortBacking,
            $out_type: FunctionStateBacking,
        {
            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
            }

            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::ternary(
                <$out_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg_type as FunctionRuntimeType>::REPRESENTATION,
                <$ix1_type as FunctionRuntimeType>::REPRESENTATION,
                <$ix2_type as FunctionRuntimeType>::REPRESENTATION,
            );

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, source, ix1, ix2) = invocation.expect_ternary()?;
                let source: Ref<$arg_type> = source.try_ref()?;
                let ix1: Ref<$ix1_type> = ix1.try_ref()?;
                let ix2: Ref<$ix2_type> = ix2.try_ref()?;
                let out: Ref<$out_type> = out.try_ref()?;
                Ok(Box::new($struct_name {
                    source,
                    ix1,
                    ix2,
                    out,
                    invocation,
                }))
            }
        }
        impl<T> MechFunctionImpl for $struct_name<T>
        where
            T: Debug + Clone + Sync + Send + PartialEq + 'static,
            $out_type: FunctionStateBacking,
        {
            fn solve_result(&self) -> MResult<()> {
                validate_matrix_access_contract(
                    self.invocation.output_cell(),
                    self.invocation.input_cells(),
                )?;
                let source_ptr = self.source.as_ptr();
                let ix1_ptr = self.ix1.as_ptr();
                let ix2_ptr = self.ix2.as_ptr();
                let out_ptr = self.out.as_mut_ptr();
                $op!(source_ptr, ix1_ptr, ix2_ptr, out_ptr);
                Ok(())
            }
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_ref(&self.out))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_ref(&self.out)]))
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                Some(&$contract)
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl<T> MechFunctionCompiler for $struct_name<T>
        where
            T: CompileConst + ConstElem + FunctionRuntimeType + CanonicalMatrixElementBacking,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}>",
                    stringify!($struct_name),
                    <T as FunctionRuntimeType>::REPRESENTATION
                );
                compile_ternop!(name, self.out, self.source, self.ix1, self.ix2, ctx);
            }
        }
    };
}

macro_rules! impl_access_fxn_shape {
    ($name:ident, $ix_type:ty, $out_type:ty, $fxn:ident, $contract:ident) => {
        paste! {
          #[cfg(feature = "matrix1")]
          impl_access_fxn!([<$name M1>],   Matrix1<T>,    $ix_type, $out_type, $fxn, $contract);
          impl_access_fxn_shape_without_matrix1!($name, $ix_type, $out_type, $fxn, $contract);
        }
    };
}

macro_rules! impl_access_fxn_shape_without_matrix1 {
    ($name:ident, $ix_type:ty, $out_type:ty, $fxn:ident, $contract:ident) => {
        paste! {
          #[cfg(feature = "matrix2")]
          impl_access_fxn!([<$name M2>],   Matrix2<T>,    $ix_type, $out_type, $fxn, $contract);
          #[cfg(feature = "matrix3")]
          impl_access_fxn!([<$name M3>],   Matrix3<T>,    $ix_type, $out_type, $fxn, $contract);
          #[cfg(feature = "matrix4")]
          impl_access_fxn!([<$name M4>],   Matrix4<T>,    $ix_type, $out_type, $fxn, $contract);
          #[cfg(feature = "matrix2x3")]
          impl_access_fxn!([<$name M2x3>], Matrix2x3<T>,  $ix_type, $out_type, $fxn, $contract);
          #[cfg(feature = "matrix3x2")]
          impl_access_fxn!([<$name M3x2>], Matrix3x2<T>,  $ix_type, $out_type, $fxn, $contract);
          #[cfg(feature = "matrixd")]
          impl_access_fxn!([<$name MD>],   DMatrix<T>,    $ix_type, $out_type, $fxn, $contract);
          #[cfg(feature = "vector2")]
          impl_access_fxn!([<$name V2>],   Vector2<T>,    $ix_type, $out_type, $fxn, $contract);
          #[cfg(feature = "vector3")]
          impl_access_fxn!([<$name V3>],   Vector3<T>,    $ix_type, $out_type, $fxn, $contract);
          #[cfg(feature = "vector4")]
          impl_access_fxn!([<$name V4>],   Vector4<T>,    $ix_type, $out_type, $fxn, $contract);
          #[cfg(feature = "vectord")]
          impl_access_fxn!([<$name VD>],   DVector<T>,    $ix_type, $out_type, $fxn, $contract);
          #[cfg(feature = "row_vector2")]
          impl_access_fxn!([<$name R2>],   RowVector2<T>, $ix_type, $out_type, $fxn, $contract);
          #[cfg(feature = "row_vector3")]
          impl_access_fxn!([<$name R3>],   RowVector3<T>, $ix_type, $out_type, $fxn, $contract);
          #[cfg(feature = "row_vector4")]
          impl_access_fxn!([<$name R4>],   RowVector4<T>, $ix_type, $out_type, $fxn, $contract);
          #[cfg(feature = "row_vectord")]
          impl_access_fxn!([<$name RD>],   RowDVector<T>, $ix_type, $out_type, $fxn, $contract);
        }
    };
}

macro_rules! impl_access_all_fxn_shape_without_matrix1 {
    ($name:ident, $out_type:ty, $contract:ident) => {
        paste! {
          #[cfg(feature = "matrix2")]
          impl_access_all_fxn!([<$name M2>],   Matrix2<T>,    $out_type, $contract);
          #[cfg(feature = "matrix3")]
          impl_access_all_fxn!([<$name M3>],   Matrix3<T>,    $out_type, $contract);
          #[cfg(feature = "matrix4")]
          impl_access_all_fxn!([<$name M4>],   Matrix4<T>,    $out_type, $contract);
          #[cfg(feature = "matrix2x3")]
          impl_access_all_fxn!([<$name M2x3>], Matrix2x3<T>,  $out_type, $contract);
          #[cfg(feature = "matrix3x2")]
          impl_access_all_fxn!([<$name M3x2>], Matrix3x2<T>,  $out_type, $contract);
          #[cfg(feature = "matrixd")]
          impl_access_all_fxn!([<$name MD>],   DMatrix<T>,    $out_type, $contract);
          #[cfg(feature = "vector2")]
          impl_access_all_fxn!([<$name V2>],   Vector2<T>,    $out_type, $contract);
          #[cfg(feature = "vector3")]
          impl_access_all_fxn!([<$name V3>],   Vector3<T>,    $out_type, $contract);
          #[cfg(feature = "vector4")]
          impl_access_all_fxn!([<$name V4>],   Vector4<T>,    $out_type, $contract);
          #[cfg(feature = "vectord")]
          impl_access_all_fxn!([<$name VD>],   DVector<T>,    $out_type, $contract);
          #[cfg(feature = "row_vector2")]
          impl_access_all_fxn!([<$name R2>],   RowVector2<T>, $out_type, $contract);
          #[cfg(feature = "row_vector3")]
          impl_access_all_fxn!([<$name R3>],   RowVector3<T>, $out_type, $contract);
          #[cfg(feature = "row_vector4")]
          impl_access_all_fxn!([<$name R4>],   RowVector4<T>, $out_type, $contract);
          #[cfg(feature = "row_vectord")]
          impl_access_all_fxn!([<$name RD>],   RowDVector<T>, $out_type, $contract);
        }
    };
}

macro_rules! impl_access_fxn_shape2 {
    ($name:ident, $ix1_type:ty, $ix2_type:ty, $out_type:ty, $fxn:ident, $contract:ident) => {
        paste! {
          #[cfg(feature = "matrix2")]
          impl_access_fxn2!([<$name M2>],   Matrix2<T>,    $ix1_type, $ix2_type, $out_type, $fxn, $contract);
          #[cfg(feature = "matrix3")]
          impl_access_fxn2!([<$name M3>],   Matrix3<T>,    $ix1_type, $ix2_type, $out_type, $fxn, $contract);
          #[cfg(feature = "matrix4")]
          impl_access_fxn2!([<$name M4>],   Matrix4<T>,    $ix1_type, $ix2_type, $out_type, $fxn, $contract);
          #[cfg(feature = "matrix2x3")]
          impl_access_fxn2!([<$name M2x3>], Matrix2x3<T>,  $ix1_type, $ix2_type, $out_type, $fxn, $contract);
          #[cfg(feature = "matrix3x2")]
          impl_access_fxn2!([<$name M3x2>], Matrix3x2<T>,  $ix1_type, $ix2_type, $out_type, $fxn, $contract);
          #[cfg(feature = "matrixd")]
          impl_access_fxn2!([<$name MD>],   DMatrix<T>,    $ix1_type, $ix2_type, $out_type, $fxn, $contract);
        }
    };
}

macro_rules! impl_access_fxn_matrix_shape {
    ($name:ident, $ix_type:ty, $out_type:ty, $fxn:ident, $contract:ident) => {
        paste! {
          #[cfg(feature = "matrix2")]
          impl_access_fxn!([<$name M2>],   Matrix2<T>,    $ix_type, $out_type, $fxn, $contract);
          #[cfg(feature = "matrix3")]
          impl_access_fxn!([<$name M3>],   Matrix3<T>,    $ix_type, $out_type, $fxn, $contract);
          #[cfg(feature = "matrix4")]
          impl_access_fxn!([<$name M4>],   Matrix4<T>,    $ix_type, $out_type, $fxn, $contract);
          #[cfg(feature = "matrix2x3")]
          impl_access_fxn!([<$name M2x3>], Matrix2x3<T>,  $ix_type, $out_type, $fxn, $contract);
          #[cfg(feature = "matrix3x2")]
          impl_access_fxn!([<$name M3x2>], Matrix3x2<T>,  $ix_type, $out_type, $fxn, $contract);
          #[cfg(feature = "matrixd")]
          impl_access_fxn!([<$name MD>],   DMatrix<T>,    $ix_type, $out_type, $fxn, $contract);
        }
    };
}

// x[1]
impl_access_fxn_shape!(
    Access1DS,
    usize,
    T,
    access_1d,
    PURE_BINARY_SCALAR_INDEX_CONTRACT
);

// x[1,2]
impl_access_fxn_shape2!(
    Access2DSS,
    usize,
    usize,
    T,
    access_2d,
    PURE_TERNARY_SCALAR_SCALAR_CONTRACT
);
#[cfg(feature = "vector2")]
impl_access_fxn2!(
    Access2DSSV2,
    Vector2<T>,
    usize,
    usize,
    T,
    access_2d,
    PURE_TERNARY_SCALAR_SCALAR_CONTRACT
);
#[cfg(feature = "vector3")]
impl_access_fxn2!(
    Access2DSSV3,
    Vector3<T>,
    usize,
    usize,
    T,
    access_2d,
    PURE_TERNARY_SCALAR_SCALAR_CONTRACT
);
#[cfg(feature = "vector4")]
impl_access_fxn2!(
    Access2DSSV4,
    Vector4<T>,
    usize,
    usize,
    T,
    access_2d,
    PURE_TERNARY_SCALAR_SCALAR_CONTRACT
);
#[cfg(feature = "vectord")]
impl_access_fxn2!(
    Access2DSSVD,
    DVector<T>,
    usize,
    usize,
    T,
    access_2d,
    PURE_TERNARY_SCALAR_SCALAR_CONTRACT
);
#[cfg(feature = "row_vector2")]
impl_access_fxn2!(
    Access2DSSR2,
    RowVector2<T>,
    usize,
    usize,
    T,
    access_2d,
    PURE_TERNARY_SCALAR_SCALAR_CONTRACT
);
#[cfg(feature = "row_vector3")]
impl_access_fxn2!(
    Access2DSSR3,
    RowVector3<T>,
    usize,
    usize,
    T,
    access_2d,
    PURE_TERNARY_SCALAR_SCALAR_CONTRACT
);
#[cfg(feature = "row_vector4")]
impl_access_fxn2!(
    Access2DSSR4,
    RowVector4<T>,
    usize,
    usize,
    T,
    access_2d,
    PURE_TERNARY_SCALAR_SCALAR_CONTRACT
);
#[cfg(feature = "row_vectord")]
impl_access_fxn2!(
    Access2DSSRD,
    RowDVector<T>,
    usize,
    usize,
    T,
    access_2d,
    PURE_TERNARY_SCALAR_SCALAR_CONTRACT
);

// x[1..3]
impl_access_fxn_shape!(
    Access1DVD,
    DVector<usize>,
    DVector<T>,
    access_1d_slice,
    PURE_BINARY_EXPLICIT_INDEX_CONTRACT
);
#[cfg(feature = "logical_indexing")]
impl_access_fxn_shape!(
    Access1DVDb,
    DVector<bool>,
    DVector<T>,
    access_1d_slice_bool_v,
    PURE_BINARY_LOGICAL_MASK_CONTRACT
);

// x[:]
impl_access_all_fxn_shape_without_matrix1!(
    Access1DA,
    DVector<T>,
    PURE_BINARY_ALL_ELEMENTS_CONTRACT
);

// x[:,1]
impl_access_fxn_matrix_shape!(
    Access2DAS,
    usize,
    DVector<T>,
    access_col,
    PURE_BINARY_ALL_ROWS_SCALAR_COLUMN_CONTRACT
);

// x[1,:]
#[cfg(feature = "matrix1")]
impl_access_fxn!(
    Access2DSAM1,
    Matrix1<T>,
    usize,
    Matrix1<T>,
    access_row,
    PURE_BINARY_SCALAR_ROW_ALL_COLUMNS_CONTRACT
);
#[cfg(all(feature = "matrix2", feature = "row_vector2"))]
impl_access_fxn!(
    Access2DSAM2,
    Matrix2<T>,
    usize,
    RowVector2<T>,
    access_row,
    PURE_BINARY_SCALAR_ROW_ALL_COLUMNS_CONTRACT
);
#[cfg(all(feature = "matrix3", feature = "row_vector3"))]
impl_access_fxn!(
    Access2DSAM3,
    Matrix3<T>,
    usize,
    RowVector3<T>,
    access_row,
    PURE_BINARY_SCALAR_ROW_ALL_COLUMNS_CONTRACT
);
#[cfg(all(feature = "matrix4", feature = "row_vector4"))]
impl_access_fxn!(
    Access2DSAM4,
    Matrix4<T>,
    usize,
    RowVector4<T>,
    access_row,
    PURE_BINARY_SCALAR_ROW_ALL_COLUMNS_CONTRACT
);
#[cfg(all(feature = "matrix2x3", feature = "row_vector3"))]
impl_access_fxn!(
    Access2DSAM2x3,
    Matrix2x3<T>,
    usize,
    RowVector3<T>,
    access_row,
    PURE_BINARY_SCALAR_ROW_ALL_COLUMNS_CONTRACT
);
#[cfg(all(feature = "matrix3x2", feature = "row_vector2"))]
impl_access_fxn!(
    Access2DSAM3x2,
    Matrix3x2<T>,
    usize,
    RowVector2<T>,
    access_row,
    PURE_BINARY_SCALAR_ROW_ALL_COLUMNS_CONTRACT
);
#[cfg(all(feature = "matrixd", feature = "row_vectord"))]
impl_access_fxn!(
    Access2DSAMD,
    DMatrix<T>,
    usize,
    RowDVector<T>,
    access_row,
    PURE_BINARY_SCALAR_ROW_ALL_COLUMNS_CONTRACT
);

// x[1..3,:]
impl_access_fxn_matrix_shape!(
    Access2DVDA,
    DVector<usize>,
    DMatrix<T>,
    access_2d_slice_all,
    PURE_BINARY_EXPLICIT_ROWS_ALL_COLUMNS_CONTRACT
);
#[cfg(feature = "logical_indexing")]
impl_access_fxn_matrix_shape!(
    Access2DVDbA,
    DVector<bool>,
    DMatrix<T>,
    access_2d_slice_all_bool,
    PURE_BINARY_LOGICAL_ROWS_ALL_COLUMNS_CONTRACT
);

// x[2,1..3]
impl_access_fxn_shape2!(
    Access2DSVD,
    usize,
    DVector<usize>,
    RowDVector<T>,
    access_2d_row_slice,
    PURE_TERNARY_SCALAR_ROW_EXPLICIT_COLUMNS_CONTRACT
);
#[cfg(feature = "logical_indexing")]
impl_access_fxn_shape2!(
    Access2DSVDb,
    usize,
    DVector<bool>,
    RowDVector<T>,
    access_2d_row_slice_bool,
    PURE_TERNARY_SCALAR_ROW_LOGICAL_COLUMNS_CONTRACT
);

// x[1..3,2]
impl_access_fxn_shape2!(
    Access2DVDS,
    DVector<usize>,
    usize,
    DVector<T>,
    access_2d_col_slice,
    PURE_TERNARY_EXPLICIT_ROWS_SCALAR_COLUMN_CONTRACT
);
#[cfg(feature = "logical_indexing")]
impl_access_fxn_shape2!(
    Access2DVDbS,
    DVector<bool>,
    usize,
    DVector<T>,
    access_2d_col_slice_bool,
    PURE_TERNARY_LOGICAL_ROWS_SCALAR_COLUMN_CONTRACT
);

macro_rules! access_2d_range_range_vbb {
    ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
        let mut sink_rix = 0;
        let mut sink_cix = 0;
        for r in 0..($ix1).len() {
            if ($ix1)[r] {
                for c in 0..($ix2).len() {
                    if ($ix2)[c] {
                        ($sink)[(sink_rix, sink_cix)] = ($source)[(r, c)].clone();
                        sink_cix += 1;
                    }
                }
                sink_cix = 0;
                sink_rix += 1;
            }
        }
    };
}

macro_rules! access_2d_range_range_vuu {
    ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
        let mut sink_rix = 0;
        let mut sink_cix = 0;
        for r in 0..($ix1).len() {
            let row = ($ix1)[r] - 1;
            for c in 0..($ix2).len() {
                let col = ($ix2)[c] - 1;
                ($sink)[(sink_rix, sink_cix)] = ($source)[(row, col)].clone();
                sink_cix += 1;
            }
            sink_cix = 0;
            sink_rix += 1;
        }
    };
}

macro_rules! access_2d_range_range_vub {
    ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
        let mut sink_rix = 0;
        let mut sink_cix = 0;
        for r in 0..($ix1).len() {
            let row = ($ix1)[r] - 1;
            for c in 0..($ix2).len() {
                if ($ix2)[c] {
                    ($sink)[(sink_rix, sink_cix)] = ($source)[(row, c)].clone();
                    sink_cix += 1;
                }
            }
            sink_cix = 0;
            sink_rix += 1;
        }
    };
}

macro_rules! access_2d_range_range_vbu {
    ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
        let mut sink_rix = 0;
        let mut sink_cix = 0;
        for r in 0..($ix1).len() {
            if ($ix1)[r] {
                for c in 0..($ix2).len() {
                    let col = ($ix2)[c] - 1;
                    ($sink)[(sink_rix, sink_cix)] = ($source)[(r, col)].clone();
                    sink_cix += 1;
                }
                sink_cix = 0;
                sink_rix += 1;
            }
        }
    };
}

impl_range_range_fxn_v!(Access2DRRVBB, access_2d_range_range_vbb, bool, bool);
impl_range_range_fxn_v!(Access2DRRVBU, access_2d_range_range_vbu, bool, usize);
impl_range_range_fxn_v!(Access2DRRVUU, access_2d_range_range_vuu, usize, usize);
impl_range_range_fxn_v!(Access2DRRVUB, access_2d_range_range_vub, usize, bool);

macro_rules! assign_2d_all_range_v {
    ($source:expr, $ix:expr, $sink:expr) => {{
        let mut sink_col_ix = 0;
        for i in 0..(*$ix).len() {
            let col_ix = $ix[i] - 1;
            let mut sink_col = ($sink).column_mut(sink_col_ix);
            let src_col = ($source).column(col_ix);
            for (dst, src) in sink_col.iter_mut().zip(src_col.iter()) {
                *dst = src.clone();
            }
            sink_col_ix += 1;
        }
    }};
}

macro_rules! assign_2d_all_range_vb {
    ($source:expr, $ix:expr, $sink:expr) => {{
        let mut sink_col_ix = 0;
        for i in 0..(*$source).ncols() {
            if $ix[i] {
                let mut sink_col = ($sink).column_mut(sink_col_ix);
                let src_col = ($source).column(i);
                for (dst, src) in sink_col.iter_mut().zip(src_col.iter()) {
                    *dst = src.clone();
                }
                sink_col_ix += 1;
            }
        }
    }};
}

impl_all_fxn_v!(
    Access2DARV,
    assign_2d_all_range_v,
    usize,
    PURE_BINARY_ALL_ROWS_EXPLICIT_COLUMNS_CONTRACT
);
impl_all_fxn_v!(
    Access2DARVB,
    assign_2d_all_range_vb,
    bool,
    PURE_BINARY_ALL_ROWS_LOGICAL_COLUMNS_CONTRACT
);

// Runtime catalog -----------------------------------------------------------

// Keep the scalar list in one place so the explicit catalog follows the same
// feature and legacy-name quirks as the source dispatch macros above. In
// particular, C64/R64 use c64/r64 in the one-type factory names, but the
// older multi-shape factories use complex/rational.
macro_rules! for_each_access_scalar {
    ($callback:ident, ($($args:tt)*)) => {
        #[cfg(feature = "bool")]
        $callback!($($args)*; bool, "bool", "bool");
        #[cfg(feature = "i8")]
        $callback!($($args)*; i8, "i8", "i8");
        #[cfg(feature = "i16")]
        $callback!($($args)*; i16, "i16", "i16");
        #[cfg(feature = "i32")]
        $callback!($($args)*; i32, "i32", "i32");
        #[cfg(feature = "i64")]
        $callback!($($args)*; i64, "i64", "i64");
        #[cfg(feature = "i128")]
        $callback!($($args)*; i128, "i128", "i128");
        #[cfg(feature = "u8")]
        $callback!($($args)*; u8, "u8", "u8");
        #[cfg(feature = "u16")]
        $callback!($($args)*; u16, "u16", "u16");
        #[cfg(feature = "u32")]
        $callback!($($args)*; u32, "u32", "u32");
        #[cfg(feature = "u64")]
        $callback!($($args)*; u64, "u64", "u64");
        #[cfg(feature = "u128")]
        $callback!($($args)*; u128, "u128", "u128");
        #[cfg(feature = "f32")]
        $callback!($($args)*; f32, "f32", "f32");
        #[cfg(feature = "f64")]
        $callback!($($args)*; f64, "f64", "f64");
        #[cfg(feature = "string")]
        $callback!($($args)*; String, "string", "string");
        #[cfg(feature = "complex")]
        $callback!($($args)*; C64, "c64", "complex");
        #[cfg(feature = "rational")]
        $callback!($($args)*; R64, "r64", "rational");
    };
}

// The access catalog deliberately derives its native declaration and runtime
// registration from the same scalar traversal.  Keeping the shape and scalar
// features beside the concrete implementation prevents a native build from
// silently selecting a broader profile than the factory it installs.
macro_rules! declare_access_typed_scalar {
    (
        $factory:ident,
        [$($feature:literal),+ $(,)?];
        $scalar:ident,
        $runtime_name:literal,
        $cargo_scalar:literal
    ) => {
        paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all(
                    feature = "access",
                    feature = $cargo_scalar,
                    $(feature = $feature),+
                ),
                registration: [<register_ $factory:snake _ $scalar:lower>],
                installer: [<install_ $factory:snake _ $scalar:lower>],
                name: concat!(stringify!($factory), "<", $runtime_name, ">"),
                factory_type: $factory<$scalar>,
                contract: RuntimeFunctionContract::canonical_custom(
                    "matrix_access",
                    RuntimeOutputAliasPolicy::DisallowInputAlias,
                    validate_matrix_access_contract,
                ),
                compiler_family: mech_core::RuntimeFamilyId::from_name(concat!(stringify!($factory), "<", $runtime_name, ">")),
                package: "mech-engine",
                crate_name: "mech_engine",
                installer_path: concat!(
                    "mech_engine::__mech_native::install_",
                    stringify!([<$factory:snake>]),
                    "_",
                    stringify!([<$scalar:lower>]),
                ),
                extra_cargo_features: ["access"],
            }
        }
    };
}

macro_rules! declare_access_typed_family {
    ($factory:ident, [$($feature:literal),+ $(,)?]) => {
        for_each_access_scalar!(declare_access_typed_scalar, ($factory, [$($feature),+]));
    };
}

macro_rules! install_access_typed_scalar {
    ($builder:expr, $factory:ident; $scalar:ident, $runtime_name:literal, $cargo_scalar:literal) => {
        paste! {
            crate::intrinsics::access::matrix::native_declarations::[<register_ $factory:snake _ $scalar:lower>]($builder)?;
        }
    };
}

macro_rules! install_access_typed_scalars {
    ($builder:expr, $factory:ident) => {{
        #[inline(never)]
        fn install(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
            for_each_access_scalar!(install_access_typed_scalar, (builder, $factory));
            Ok(())
        }

        install($builder)?;
    }};
}

macro_rules! install_access_shape {
    ($builder:expr, $feature:literal, $family:ident, $shape:ident) => {
        #[cfg(feature = $feature)]
        paste! {
            install_access_typed_scalars!($builder, [<$family $shape>]);
        }
    };
}

macro_rules! declare_access_shape {
    ($family:ident, $shape:ident, $feature:literal) => {
        paste! {
            declare_access_typed_family!([<$family $shape>], [$feature]);
        }
    };
}

macro_rules! for_each_access_shape {
    ($callback:ident, ($family:ident)) => {
        $callback!($family, M1, "matrix1");
        $callback!($family, M2, "matrix2");
        $callback!($family, M3, "matrix3");
        $callback!($family, M4, "matrix4");
        $callback!($family, M2x3, "matrix2x3");
        $callback!($family, M3x2, "matrix3x2");
        $callback!($family, MD, "matrixd");
        $callback!($family, V2, "vector2");
        $callback!($family, V3, "vector3");
        $callback!($family, V4, "vector4");
        $callback!($family, VD, "vectord");
        $callback!($family, R2, "row_vector2");
        $callback!($family, R3, "row_vector3");
        $callback!($family, R4, "row_vector4");
        $callback!($family, RD, "row_vectord");
    };
}

macro_rules! for_each_access_shape_without_matrix1 {
    ($callback:ident, ($family:ident)) => {
        $callback!($family, M2, "matrix2");
        $callback!($family, M3, "matrix3");
        $callback!($family, M4, "matrix4");
        $callback!($family, M2x3, "matrix2x3");
        $callback!($family, M3x2, "matrix3x2");
        $callback!($family, MD, "matrixd");
        $callback!($family, V2, "vector2");
        $callback!($family, V3, "vector3");
        $callback!($family, V4, "vector4");
        $callback!($family, VD, "vectord");
        $callback!($family, R2, "row_vector2");
        $callback!($family, R3, "row_vector3");
        $callback!($family, R4, "row_vector4");
        $callback!($family, RD, "row_vectord");
    };
}

macro_rules! for_each_access_matrix_shape {
    ($callback:ident, ($family:ident)) => {
        $callback!($family, M2, "matrix2");
        $callback!($family, M3, "matrix3");
        $callback!($family, M4, "matrix4");
        $callback!($family, M2x3, "matrix2x3");
        $callback!($family, M3x2, "matrix3x2");
        $callback!($family, MD, "matrixd");
    };
}

macro_rules! install_access_all_shapes {
    ($builder:expr, $family:ident) => {
        install_access_shape!($builder, "matrix1", $family, M1);
        install_access_shape!($builder, "matrix2", $family, M2);
        install_access_shape!($builder, "matrix3", $family, M3);
        install_access_shape!($builder, "matrix4", $family, M4);
        install_access_shape!($builder, "matrix2x3", $family, M2x3);
        install_access_shape!($builder, "matrix3x2", $family, M3x2);
        install_access_shape!($builder, "matrixd", $family, MD);
        install_access_shape!($builder, "vector2", $family, V2);
        install_access_shape!($builder, "vector3", $family, V3);
        install_access_shape!($builder, "vector4", $family, V4);
        install_access_shape!($builder, "vectord", $family, VD);
        install_access_shape!($builder, "row_vector2", $family, R2);
        install_access_shape!($builder, "row_vector3", $family, R3);
        install_access_shape!($builder, "row_vector4", $family, R4);
        install_access_shape!($builder, "row_vectord", $family, RD);
    };
}

macro_rules! install_access_shapes_without_matrix1 {
    ($builder:expr, $family:ident) => {
        install_access_shape!($builder, "matrix2", $family, M2);
        install_access_shape!($builder, "matrix3", $family, M3);
        install_access_shape!($builder, "matrix4", $family, M4);
        install_access_shape!($builder, "matrix2x3", $family, M2x3);
        install_access_shape!($builder, "matrix3x2", $family, M3x2);
        install_access_shape!($builder, "matrixd", $family, MD);
        install_access_shape!($builder, "vector2", $family, V2);
        install_access_shape!($builder, "vector3", $family, V3);
        install_access_shape!($builder, "vector4", $family, V4);
        install_access_shape!($builder, "vectord", $family, VD);
        install_access_shape!($builder, "row_vector2", $family, R2);
        install_access_shape!($builder, "row_vector3", $family, R3);
        install_access_shape!($builder, "row_vector4", $family, R4);
        install_access_shape!($builder, "row_vectord", $family, RD);
    };
}

macro_rules! install_access_matrix_shapes {
    ($builder:expr, $family:ident) => {
        install_access_shape!($builder, "matrix2", $family, M2);
        install_access_shape!($builder, "matrix3", $family, M3);
        install_access_shape!($builder, "matrix4", $family, M4);
        install_access_shape!($builder, "matrix2x3", $family, M2x3);
        install_access_shape!($builder, "matrix3x2", $family, M3x2);
        install_access_shape!($builder, "matrixd", $family, MD);
    };
}

macro_rules! declare_access_range_range_scalar {
    (
        $factory:ident,
        $output:ident,
        $input:ident,
        $ix1:ident,
        $ix1_scalar:ident,
        $ix2:ident,
        $ix2_scalar:ident,
        [$($feature:literal),+ $(,)?];
        $scalar:ident,
        $runtime_name:literal,
        $cargo_scalar:literal
    ) => {
        paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all(
                    feature = "access",
                    feature = $cargo_scalar,
                    $(feature = $feature),+
                ),
                registration: [<register_ $factory:snake _ $output:snake _ $input:snake _ $ix1:snake _ $ix2:snake _ $scalar:lower>],
                installer: [<install_ $factory:snake _ $output:snake _ $input:snake _ $ix1:snake _ $ix2:snake _ $scalar:lower>],
                name: concat!(
                    stringify!($factory),
                    "<",
                    $cargo_scalar,
                    stringify!($output),
                    stringify!($input),
                    stringify!($ix1),
                    stringify!($ix2),
                    ">"
                ),
                factory_type: $factory<
                    $scalar,
                    $output<$scalar>,
                    $input<$scalar>,
                    $ix1<$ix1_scalar>,
                    $ix2<$ix2_scalar>,
                >,
                contract: RuntimeFunctionContract::canonical_custom(
                    "matrix_access",
                    RuntimeOutputAliasPolicy::DisallowInputAlias,
                    validate_matrix_access_contract,
                ),
                compiler_family: mech_core::RuntimeFamilyId::from_name(concat!(stringify!($factory), "<", $cargo_scalar, stringify!($output), stringify!($input), stringify!($ix1), stringify!($ix2), ">")),
                package: "mech-engine",
                crate_name: "mech_engine",
                installer_path: concat!(
                    "mech_engine::__mech_native::install_",
                    stringify!([<$factory:snake _ $output:snake _ $input:snake _ $ix1:snake _ $ix2:snake _ $scalar:lower>]),
                ),
                extra_cargo_features: ["access"],
            }
        }
    };
}

macro_rules! declare_access_range_range_family {
    (
        $factory:ident,
        $output:ident,
        $input:ident,
        $ix1:ident,
        $ix1_scalar:ident,
        $ix2:ident,
        $ix2_scalar:ident,
        [$($feature:literal),+ $(,)?]
    ) => {
        for_each_access_scalar!(
            declare_access_range_range_scalar,
            (
                $factory,
                $output,
                $input,
                $ix1,
                $ix1_scalar,
                $ix2,
                $ix2_scalar,
                [$($feature),+]
            )
        );
    };
}

macro_rules! declare_access_all_range_scalar {
    (
        $factory:ident,
        $output:ident,
        $input:ident,
        $ix:ident,
        $ix_scalar:ident,
        [$($feature:literal),+ $(,)?];
        $scalar:ident,
        $runtime_name:literal,
        $cargo_scalar:literal
    ) => {
        paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all(
                    feature = "access",
                    feature = $cargo_scalar,
                    $(feature = $feature),+
                ),
                registration: [<register_ $factory:snake _ $output:snake _ $input:snake _ $ix:snake _ $scalar:lower>],
                installer: [<install_ $factory:snake _ $output:snake _ $input:snake _ $ix:snake _ $scalar:lower>],
                name: concat!(
                    stringify!($factory),
                    "<",
                    $cargo_scalar,
                    stringify!($output),
                    stringify!($input),
                    stringify!($ix),
                    ">"
                ),
                factory_type: $factory<
                    $scalar,
                    $output<$scalar>,
                    $input<$scalar>,
                    $ix<$ix_scalar>,
                >,
                contract: RuntimeFunctionContract::canonical_custom(
                    "matrix_access_all_range",
                    RuntimeOutputAliasPolicy::DisallowInputAlias,
                    validate_matrix_access_all_range_contract,
                ),
                compiler_family: mech_core::RuntimeFamilyId::from_name(concat!(stringify!($factory), "<", $cargo_scalar, stringify!($output), stringify!($input), stringify!($ix), ">")),
                package: "mech-engine",
                crate_name: "mech_engine",
                installer_path: concat!(
                    "mech_engine::__mech_native::install_",
                    stringify!([<$factory:snake _ $output:snake _ $input:snake _ $ix:snake _ $scalar:lower>]),
                ),
                extra_cargo_features: ["access"],
            }
        }
    };
}

macro_rules! declare_access_all_range_family {
    (
        $factory:ident,
        $output:ident,
        $input:ident,
        $ix:ident,
        $ix_scalar:ident,
        [$($feature:literal),+ $(,)?]
    ) => {
        for_each_access_scalar!(
            declare_access_all_range_scalar,
            ($factory, $output, $input, $ix, $ix_scalar, [$($feature),+])
        );
    };
}

macro_rules! install_access_range_range_scalar {
    (
        $builder:expr,
        $factory:ident,
        $output:ident,
        $input:ident,
        $ix1:ident,
        $ix1_scalar:ident,
        $ix2:ident,
        $ix2_scalar:ident;
        $scalar:ident,
        $runtime_name:literal,
        $assign_name:literal
    ) => {
        paste! {
            crate::intrinsics::access::matrix::native_declarations::[<register_ $factory:snake _ $output:snake _ $input:snake _ $ix1:snake _ $ix2:snake _ $scalar:lower>]($builder)?;
        }
    };
}

macro_rules! install_access_all_range_scalar {
    (
        $builder:expr,
        $factory:ident,
        $output:ident,
        $input:ident,
        $ix:ident,
        $ix_scalar:ident;
        $scalar:ident,
        $runtime_name:literal,
        $assign_name:literal
    ) => {
        paste! {
            crate::intrinsics::access::matrix::native_declarations::[<register_ $factory:snake _ $output:snake _ $input:snake _ $ix:snake _ $scalar:lower>]($builder)?;
        }
    };
}

macro_rules! install_access_dynamic_for_shape {
    ($builder:expr, $shape:ident) => {
        #[cfg(all(feature = "matrixd", feature = "vectord"))]
        for_each_access_scalar!(
            install_access_range_range_scalar,
            (
                $builder,
                Access2DRRVUU,
                DMatrix,
                $shape,
                DVector,
                usize,
                DVector,
                usize
            )
        );

        // The legacy bool/bool match arm required all three dynamic output
        // shapes even though it registered each output independently.
        #[cfg(all(
            feature = "matrixd",
            feature = "vectord",
            feature = "row_vectord",
            feature = "logical_indexing"
        ))]
        {
            for_each_access_scalar!(
                install_access_range_range_scalar,
                (
                    $builder,
                    Access2DRRVBB,
                    DMatrix,
                    $shape,
                    DVector,
                    bool,
                    DVector,
                    bool
                )
            );
            for_each_access_scalar!(
                install_access_range_range_scalar,
                (
                    $builder,
                    Access2DRRVBB,
                    DVector,
                    $shape,
                    DVector,
                    bool,
                    DVector,
                    bool
                )
            );
            for_each_access_scalar!(
                install_access_range_range_scalar,
                (
                    $builder,
                    Access2DRRVBB,
                    RowDVector,
                    $shape,
                    DVector,
                    bool,
                    DVector,
                    bool
                )
            );
        }

        #[cfg(all(feature = "matrixd", feature = "vectord", feature = "logical_indexing"))]
        for_each_access_scalar!(
            install_access_range_range_scalar,
            (
                $builder,
                Access2DRRVUB,
                DMatrix,
                $shape,
                DVector,
                usize,
                DVector,
                bool
            )
        );
        #[cfg(all(feature = "vectord", feature = "logical_indexing"))]
        for_each_access_scalar!(
            install_access_range_range_scalar,
            (
                $builder,
                Access2DRRVUB,
                DVector,
                $shape,
                DVector,
                usize,
                DVector,
                bool
            )
        );
        #[cfg(all(
            feature = "vectord",
            feature = "row_vectord",
            feature = "logical_indexing"
        ))]
        for_each_access_scalar!(
            install_access_range_range_scalar,
            (
                $builder,
                Access2DRRVUB,
                RowDVector,
                $shape,
                DVector,
                usize,
                DVector,
                bool
            )
        );

        #[cfg(all(feature = "matrixd", feature = "vectord", feature = "logical_indexing"))]
        for_each_access_scalar!(
            install_access_range_range_scalar,
            (
                $builder,
                Access2DRRVBU,
                DMatrix,
                $shape,
                DVector,
                bool,
                DVector,
                usize
            )
        );
        #[cfg(all(feature = "vectord", feature = "logical_indexing"))]
        for_each_access_scalar!(
            install_access_range_range_scalar,
            (
                $builder,
                Access2DRRVBU,
                DVector,
                $shape,
                DVector,
                bool,
                DVector,
                usize
            )
        );
        #[cfg(all(
            feature = "vectord",
            feature = "row_vectord",
            feature = "logical_indexing"
        ))]
        for_each_access_scalar!(
            install_access_range_range_scalar,
            (
                $builder,
                Access2DRRVBU,
                RowDVector,
                $shape,
                DVector,
                bool,
                DVector,
                usize
            )
        );

        #[cfg(all(feature = "row_vectord", feature = "vectord"))]
        for_each_access_scalar!(
            install_access_all_range_scalar,
            ($builder, Access2DARV, RowDVector, $shape, DVector, usize)
        );
        #[cfg(all(feature = "matrixd", feature = "vectord"))]
        for_each_access_scalar!(
            install_access_all_range_scalar,
            ($builder, Access2DARV, DMatrix, $shape, DVector, usize)
        );

        // This row-vector bool case intentionally lacked logical_indexing in
        // the legacy registration; preserve that source-visible quirk.
        #[cfg(all(feature = "row_vectord", feature = "vectord"))]
        for_each_access_scalar!(
            install_access_all_range_scalar,
            ($builder, Access2DARVB, RowDVector, $shape, DVector, bool)
        );
        #[cfg(all(feature = "matrixd", feature = "vectord", feature = "logical_indexing"))]
        {
            for_each_access_scalar!(
                install_access_all_range_scalar,
                ($builder, Access2DARVB, DVector, $shape, DVector, bool)
            );
            for_each_access_scalar!(
                install_access_all_range_scalar,
                ($builder, Access2DARVB, DMatrix, $shape, DVector, bool)
            );
        }
    };
}

macro_rules! install_access_dynamic_shape {
    ($builder:expr, $feature:literal, $shape:ident) => {{
        #[cfg(feature = $feature)]
        {
            #[inline(never)]
            fn install(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
                install_access_dynamic_for_shape!(builder, $shape);
                Ok(())
            }

            install($builder)?;
        }
    }};
}

macro_rules! declare_access_dynamic_for_shape {
    ($shape:ident, $shape_feature:literal) => {
        declare_access_range_range_family!(
            Access2DRRVUU,
            DMatrix,
            $shape,
            DVector,
            usize,
            DVector,
            usize,
            ["matrixd", "vectord", $shape_feature]
        );

        declare_access_range_range_family!(
            Access2DRRVBB,
            DMatrix,
            $shape,
            DVector,
            bool,
            DVector,
            bool,
            [
                "bool",
                "matrixd",
                "vectord",
                "row_vectord",
                "logical_indexing",
                $shape_feature
            ]
        );
        declare_access_range_range_family!(
            Access2DRRVBB,
            DVector,
            $shape,
            DVector,
            bool,
            DVector,
            bool,
            [
                "bool",
                "matrixd",
                "vectord",
                "row_vectord",
                "logical_indexing",
                $shape_feature
            ]
        );
        declare_access_range_range_family!(
            Access2DRRVBB,
            RowDVector,
            $shape,
            DVector,
            bool,
            DVector,
            bool,
            [
                "bool",
                "matrixd",
                "vectord",
                "row_vectord",
                "logical_indexing",
                $shape_feature
            ]
        );

        declare_access_range_range_family!(
            Access2DRRVUB,
            DMatrix,
            $shape,
            DVector,
            usize,
            DVector,
            bool,
            [
                "bool",
                "matrixd",
                "vectord",
                "logical_indexing",
                $shape_feature
            ]
        );
        declare_access_range_range_family!(
            Access2DRRVUB,
            DVector,
            $shape,
            DVector,
            usize,
            DVector,
            bool,
            ["bool", "vectord", "logical_indexing", $shape_feature]
        );
        declare_access_range_range_family!(
            Access2DRRVUB,
            RowDVector,
            $shape,
            DVector,
            usize,
            DVector,
            bool,
            [
                "bool",
                "vectord",
                "row_vectord",
                "logical_indexing",
                $shape_feature
            ]
        );

        declare_access_range_range_family!(
            Access2DRRVBU,
            DMatrix,
            $shape,
            DVector,
            bool,
            DVector,
            usize,
            [
                "bool",
                "matrixd",
                "vectord",
                "logical_indexing",
                $shape_feature
            ]
        );
        declare_access_range_range_family!(
            Access2DRRVBU,
            DVector,
            $shape,
            DVector,
            bool,
            DVector,
            usize,
            ["bool", "vectord", "logical_indexing", $shape_feature]
        );
        declare_access_range_range_family!(
            Access2DRRVBU,
            RowDVector,
            $shape,
            DVector,
            bool,
            DVector,
            usize,
            [
                "bool",
                "vectord",
                "row_vectord",
                "logical_indexing",
                $shape_feature
            ]
        );

        declare_access_all_range_family!(
            Access2DARV,
            RowDVector,
            $shape,
            DVector,
            usize,
            ["row_vectord", "vectord", $shape_feature]
        );
        declare_access_all_range_family!(
            Access2DARV,
            DMatrix,
            $shape,
            DVector,
            usize,
            ["matrixd", "vectord", $shape_feature]
        );

        declare_access_all_range_family!(
            Access2DARVB,
            RowDVector,
            $shape,
            DVector,
            bool,
            ["bool", "row_vectord", "vectord", $shape_feature]
        );
        declare_access_all_range_family!(
            Access2DARVB,
            DVector,
            $shape,
            DVector,
            bool,
            [
                "bool",
                "matrixd",
                "vectord",
                "logical_indexing",
                $shape_feature
            ]
        );
        declare_access_all_range_family!(
            Access2DARVB,
            DMatrix,
            $shape,
            DVector,
            bool,
            [
                "bool",
                "matrixd",
                "vectord",
                "logical_indexing",
                $shape_feature
            ]
        );
    };
}

pub(crate) mod native_declarations {
    use super::*;

    mech_core::declare_native_runtime_factory! {
        cfg: any(feature = "subscript_formula", feature = "subscript_range"),
        registration: register_canonical_index_conversion,
        installer: install_canonical_index_conversion,
        name: "access/index",
        factory_type: super::CanonicalIndexConversion,
        contract: RuntimeFunctionContract::canonical_custom(
            "canonical_index_conversion",
            RuntimeOutputAliasPolicy::DisallowInputAlias,
            super::validate_canonical_index_conversion,
        ),
        operations: [OperationId::from_name("access/index")],
        package: "mech-engine", crate_name: "mech_engine",
        installer_path: "mech_engine::__mech_native::install_canonical_index_conversion",
        extra_cargo_features: ["access", "subscript_formula", "subscript_range"],
    }

    for_each_access_shape!(declare_access_shape, (Access1DS));
    for_each_access_shape_without_matrix1!(declare_access_shape, (Access2DSS));
    for_each_access_shape!(declare_access_shape, (Access1DVD));
    for_each_access_shape_without_matrix1!(declare_access_shape, (Access1DA));

    #[cfg(feature = "logical_indexing")]
    for_each_access_shape!(declare_access_shape, (Access1DVDb));

    for_each_access_matrix_shape!(declare_access_shape, (Access2DAS));
    for_each_access_matrix_shape!(declare_access_shape, (Access2DVDA));
    for_each_access_matrix_shape!(declare_access_shape, (Access2DVDS));
    for_each_access_matrix_shape!(declare_access_shape, (Access2DSVD));

    #[cfg(feature = "logical_indexing")]
    for_each_access_matrix_shape!(declare_access_shape, (Access2DVDbA));
    #[cfg(feature = "logical_indexing")]
    for_each_access_matrix_shape!(declare_access_shape, (Access2DVDbS));
    #[cfg(feature = "logical_indexing")]
    for_each_access_matrix_shape!(declare_access_shape, (Access2DSVDb));

    declare_access_typed_family!(Access2DSAM1, ["matrix1"]);
    declare_access_typed_family!(Access2DSAM2, ["matrix2", "row_vector2"]);
    declare_access_typed_family!(Access2DSAM3, ["matrix3", "row_vector3"]);
    declare_access_typed_family!(Access2DSAM4, ["matrix4", "row_vector4"]);
    declare_access_typed_family!(Access2DSAM2x3, ["matrix2x3", "row_vector3"]);
    declare_access_typed_family!(Access2DSAM3x2, ["matrix3x2", "row_vector2"]);
    declare_access_typed_family!(Access2DSAMD, ["matrixd", "row_vectord"]);

    declare_access_dynamic_for_shape!(Matrix1, "matrix1");
    declare_access_dynamic_for_shape!(Matrix2, "matrix2");
    declare_access_dynamic_for_shape!(Matrix3, "matrix3");
    declare_access_dynamic_for_shape!(Matrix4, "matrix4");
    declare_access_dynamic_for_shape!(Matrix2x3, "matrix2x3");
    declare_access_dynamic_for_shape!(Matrix3x2, "matrix3x2");
    declare_access_dynamic_for_shape!(DMatrix, "matrixd");
    declare_access_dynamic_for_shape!(Vector2, "vector2");
    declare_access_dynamic_for_shape!(Vector3, "vector3");
    declare_access_dynamic_for_shape!(Vector4, "vector4");
    declare_access_dynamic_for_shape!(DVector, "vectord");
    declare_access_dynamic_for_shape!(RowVector2, "row_vector2");
    declare_access_dynamic_for_shape!(RowVector3, "row_vector3");
    declare_access_dynamic_for_shape!(RowVector4, "row_vector4");
    declare_access_dynamic_for_shape!(RowDVector, "row_vectord");

    // The retained n-body source uses a fixed three-column selector against
    // its dynamic body table. Keep that exact all-feature representation in
    // the runtime catalog rather than changing global shape preference.
    declare_access_all_range_scalar!(
        Access2DARV,
        DMatrix,
        DMatrix,
        Vector2,
        usize,
        ["matrixd", "vector2"];
        f64,
        "f64",
        "f64"
    );
    declare_access_all_range_scalar!(
        Access2DARV,
        DMatrix,
        DMatrix,
        Vector3,
        usize,
        ["matrixd", "vector3"];
        f64,
        "f64",
        "f64"
    );
}

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    pub use super::native_declarations::*;
}

pub(super) fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(any(feature = "subscript_formula", feature = "subscript_range"))]
    native_declarations::register_canonical_index_conversion(builder)?;

    install_access_all_shapes!(builder, Access1DS);
    install_access_shapes_without_matrix1!(builder, Access2DSS);
    install_access_all_shapes!(builder, Access1DVD);
    install_access_shapes_without_matrix1!(builder, Access1DA);

    #[cfg(feature = "logical_indexing")]
    install_access_all_shapes!(builder, Access1DVDb);

    install_access_matrix_shapes!(builder, Access2DAS);
    install_access_matrix_shapes!(builder, Access2DVDA);
    install_access_matrix_shapes!(builder, Access2DVDS);
    install_access_matrix_shapes!(builder, Access2DSVD);

    #[cfg(feature = "logical_indexing")]
    {
        install_access_matrix_shapes!(builder, Access2DVDbA);
        install_access_matrix_shapes!(builder, Access2DVDbS);
        install_access_matrix_shapes!(builder, Access2DSVDb);
    }

    #[cfg(feature = "matrix1")]
    install_access_typed_scalars!(builder, Access2DSAM1);
    #[cfg(all(feature = "matrix2", feature = "row_vector2"))]
    install_access_typed_scalars!(builder, Access2DSAM2);
    #[cfg(all(feature = "matrix3", feature = "row_vector3"))]
    install_access_typed_scalars!(builder, Access2DSAM3);
    #[cfg(all(feature = "matrix4", feature = "row_vector4"))]
    install_access_typed_scalars!(builder, Access2DSAM4);
    #[cfg(all(feature = "matrix2x3", feature = "row_vector3"))]
    install_access_typed_scalars!(builder, Access2DSAM2x3);
    #[cfg(all(feature = "matrix3x2", feature = "row_vector2"))]
    install_access_typed_scalars!(builder, Access2DSAM3x2);
    #[cfg(all(feature = "matrixd", feature = "row_vectord"))]
    install_access_typed_scalars!(builder, Access2DSAMD);

    install_access_dynamic_shape!(builder, "matrix1", Matrix1);
    install_access_dynamic_shape!(builder, "matrix2", Matrix2);
    install_access_dynamic_shape!(builder, "matrix3", Matrix3);
    install_access_dynamic_shape!(builder, "matrix4", Matrix4);
    install_access_dynamic_shape!(builder, "matrix2x3", Matrix2x3);
    install_access_dynamic_shape!(builder, "matrix3x2", Matrix3x2);
    install_access_dynamic_shape!(builder, "matrixd", DMatrix);
    install_access_dynamic_shape!(builder, "vector2", Vector2);
    install_access_dynamic_shape!(builder, "vector3", Vector3);
    install_access_dynamic_shape!(builder, "vector4", Vector4);
    install_access_dynamic_shape!(builder, "vectord", DVector);
    install_access_dynamic_shape!(builder, "row_vector2", RowVector2);
    install_access_dynamic_shape!(builder, "row_vector3", RowVector3);
    install_access_dynamic_shape!(builder, "row_vector4", RowVector4);
    install_access_dynamic_shape!(builder, "row_vectord", RowDVector);

    #[cfg(all(feature = "f64", feature = "matrixd", feature = "vector3"))]
    install_access_all_range_scalar!(
        builder,
        Access2DARV,
        DMatrix,
        DMatrix,
        Vector3,
        usize;
        f64,
        "f64",
        "f64"
    );

    #[cfg(all(feature = "f64", feature = "matrixd", feature = "vector2"))]
    install_access_all_range_scalar!(
        builder,
        Access2DARV,
        DMatrix,
        DMatrix,
        Vector2,
        usize;
        f64,
        "f64",
        "f64"
    );

    Ok(())
}

#[cfg(any(feature = "subscript_formula", feature = "subscript_range"))]
declare_matrix_selection_contract!(PURE_UNARY_INDEX_CONVERSION_CONTRACT, 1, "scalar-index");

#[derive(Debug)]
#[cfg(any(feature = "subscript_formula", feature = "subscript_range"))]
struct CanonicalIndexConversion {
    source: FunctionValueInput,
    output: FunctionValueOutput,
}

#[cfg(any(feature = "subscript_formula", feature = "subscript_range"))]
impl MechFunctionFactory for CanonicalIndexConversion {
    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::unary(
        FunctionValueRepresentation::AnyValue,
        FunctionValueRepresentation::AnyValue,
    );

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (output, source) = invocation.expect_unary()?;
        Ok(Box::new(Self {
            source: source.value(),
            output: output.value(),
        }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_UNARY_INDEX_CONVERSION_CONTRACT)
    }
}

#[cfg(any(feature = "subscript_formula", feature = "subscript_range"))]
impl MechFunctionImpl for CanonicalIndexConversion {
    fn solve_result(&self) -> MResult<()> {
        if let Some(elements) = self.source.cell().matrix_elements()? {
            let elements = elements
                .iter()
                .map(canonical_portable_index)
                .map(|value| value.map(|value| ValueDataDraft::Index(value as u64)))
                .collect::<MResult<Vec<_>>>()?;
            let next = self.output.cell().rebuild_matrix_drafts(
                vec![elements.len() as u64, 1].into_boxed_slice(),
                elements.into_boxed_slice(),
            )?;
            return self.output.replace(&next);
        }
        let index = canonical_portable_index(self.source.cell())?;
        self.output
            .replace(&ValueCell::from_exact(index)?.snapshot()?)
    }

    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_cell(self.output.cell()))
    }

    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_cell(self.output.cell())]))
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_UNARY_INDEX_CONVERSION_CONTRACT)
    }

    fn semantic_operation_name(&self) -> Option<&str> {
        Some("access/index")
    }

    fn to_string(&self) -> String {
        "CanonicalIndexConversion".to_owned()
    }
}

#[cfg(all(
    feature = "semantic-compiler",
    any(feature = "subscript_formula", feature = "subscript_range")
))]
impl MechFunctionCompiler for CanonicalIndexConversion {
    fn compiler_owned_value_cells(&self) -> Vec<ValueCell> {
        vec![self.source.cell().clone(), self.output.cell().clone()]
    }

    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let output = self.output.compile_register(context)?;
        let source = self.source.compile_register(context)?;
        context.emit_unop(hash_str("access/index"), output, source);
        Ok(output)
    }
}

#[cfg(any(feature = "subscript_formula", feature = "subscript_range"))]
fn canonical_portable_index(value: &ValueCell) -> MResult<usize> {
    let snapshot = value.snapshot()?;
    let value = mech_core::canonical_positional_ordinal(snapshot.data()).map_err(|_| {
        MechError::new(
            CannotConvertToTypeError {
                target_type: "portable index",
            },
            None,
        )
        .with_compiler_loc()
    })?;
    Ok(value as usize)
}

#[cfg(any(feature = "subscript_formula", feature = "subscript_range"))]
fn validate_canonical_index_conversion(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
    let [source] = inputs else {
        return Err(function_shape_contract_violation(
            "canonical_index_conversion",
            format!("expected one source input, found {}", inputs.len()),
        ));
    };
    let source_schema = source.closed_schema_body()?;
    let output_schema = output.closed_schema_body()?;
    let valid = match (&source_schema, &output_schema) {
        (source, SchemaBody::Index) => is_positional_selector_schema(source),
        (
            SchemaBody::Matrix {
                element: source_element,
                ..
            },
            SchemaBody::Matrix {
                element: output_element,
                ..
            },
        ) => {
            let source_cardinality = source
                .resolved_descriptor()?
                .current_extents()
                .map_err(MechError::from)?
                .into_iter()
                .try_fold(1_u64, u64::checked_mul);
            let output_cardinality = output
                .resolved_descriptor()?
                .current_extents()
                .map_err(MechError::from)?
                .into_iter()
                .try_fold(1_u64, u64::checked_mul);
            is_positional_selector_schema(source_element)
                && output_element.as_ref() == &SchemaBody::Index
                && source_cardinality == output_cardinality
        }
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(function_shape_contract_violation(
            "canonical_index_conversion",
            "source and output do not form a canonical positional-index conversion",
        ))
    }
}

/// Converts a canonical scalar selector into a live canonical index cell.
/// Boolean selectors and already-indexed cells remain unchanged.
#[cfg(all(feature = "subscript_formula", feature = "semantic-compiler"))]
pub(crate) fn canonical_reactive_scalar_index(
    value: ValueCell,
    execution: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    if matches!(
        value.representation(),
        FunctionValueRepresentation::Bool | FunctionValueRepresentation::Index
    ) {
        return Ok(value);
    }
    let output = ValueCell::from_exact(canonical_portable_index(&value)?)?;
    let invocation = FunctionInvocation::unary(output.clone(), value);
    let instance = FunctionInstance::new(
        CanonicalIndexConversion::new_invocation(invocation.clone())?,
        invocation,
    );
    if !execution.plan().activation_registration_active() {
        instance.solve_result()?;
    }
    execution
        .plan()
        .register_specialized(SpecializedFunction::syntax_directed(
            instance,
            ResolvedOperationDescriptor::from_name(
                "access/index",
                PURE_UNARY_INDEX_CONVERSION_CONTRACT.clone(),
            )?,
            RuntimeFunctionId::from_name("access/index"),
            ExecutionTarget::DirectRuntime,
        )?)?;
    Ok(output)
}

#[cfg(all(feature = "subscript_range", feature = "semantic-compiler"))]
fn canonical_matrix_dimensions(value: &ValueCell) -> MResult<(usize, usize)> {
    let SchemaBody::Matrix { dimensions, .. } = value.closed_schema_body()? else {
        return Err(MechError::new(
            CannotConvertToTypeError {
                target_type: "portable index matrix",
            },
            None,
        )
        .with_compiler_loc());
    };
    let [
        DimensionExpr::Constant(rows),
        DimensionExpr::Constant(columns),
    ] = dimensions.as_ref()
    else {
        return Err(MechError::new(
            CannotConvertToTypeError {
                target_type: "closed matrix dimensions",
            },
            None,
        )
        .with_compiler_loc());
    };
    Ok((*rows as usize, *columns as usize))
}

#[cfg(all(feature = "subscript_range", feature = "semantic-compiler"))]
pub(crate) fn canonical_reactive_index_matrix(
    value: ValueCell,
    execution: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    let (rows, columns) = canonical_matrix_dimensions(&value)?;
    let elements = value
        .matrix_elements()?
        .ok_or_else(|| {
            MechError::new(
                CannotConvertToTypeError {
                    target_type: "portable index matrix",
                },
                None,
            )
            .with_compiler_loc()
        })?
        .iter()
        .map(canonical_portable_index)
        .map(|value| value.map(|value| ValueDataDraft::Index(value as u64)))
        .collect::<MResult<Vec<_>>>()?;
    let output = ValueCell::dynamic_matrix(
        SchemaBody::Index,
        vec![rows.saturating_mul(columns) as u64, 1].into_boxed_slice(),
        elements.into_boxed_slice(),
    )?;
    let invocation = FunctionInvocation::unary(output.clone(), value);
    let instance = FunctionInstance::new(
        CanonicalIndexConversion::new_invocation(invocation.clone())?,
        invocation,
    );
    if !execution.plan().activation_registration_active() {
        instance.solve_result()?;
    }
    execution
        .plan()
        .register_specialized(SpecializedFunction::syntax_directed(
            instance,
            ResolvedOperationDescriptor::from_name(
                "access/index",
                PURE_UNARY_INDEX_CONVERSION_CONTRACT.clone(),
            )?,
            RuntimeFunctionId::from_name("access/index"),
            ExecutionTarget::DirectRuntime,
        )?)?;
    Ok(output)
}
