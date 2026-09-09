use crate::intrinsics::*;
use nalgebra::{
    Dim,
    base::{Matrix as naMatrix, RawStorage, Storage, StorageMut},
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
#[cfg(feature = "logical_indexing")]
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
#[cfg(feature = "logical_indexing")]
declare_matrix_selection_contract!(
    PURE_TERNARY_LOGICAL_ROWS_LOGICAL_COLUMNS_CONTRACT,
    3,
    "logical-rows-logical-columns-output"
);
#[cfg(feature = "logical_indexing")]
declare_matrix_selection_contract!(
    PURE_TERNARY_LOGICAL_ROWS_EXPLICIT_COLUMNS_CONTRACT,
    3,
    "logical-rows-explicit-columns-output"
);
#[cfg(feature = "logical_indexing")]
declare_matrix_selection_contract!(
    PURE_TERNARY_EXPLICIT_ROWS_LOGICAL_COLUMNS_CONTRACT,
    3,
    "explicit-rows-logical-columns-output"
);
declare_matrix_selection_contract!(
    PURE_TERNARY_EXPLICIT_ROWS_EXPLICIT_COLUMNS_CONTRACT,
    3,
    "explicit-rows-explicit-columns-output"
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
    let (expected_rows, expected_cols) = matrix_access_expected_output_shape(output_value, inputs)?;
    let output_shape = matrix_descriptor(output_value)?
        .map(|descriptor| (descriptor.rows, descriptor.cols))
        .unwrap_or((1, 1));
    if require_exact_output_shape
        && (output_shape.0 != expected_rows || output_shape.1 != expected_cols)
    {
        return Err(function_shape_contract_violation(
            "matrix_access",
            format!(
                "output is {}x{}, selected indices require {expected_rows}x{expected_cols}",
                output_shape.0, output_shape.1,
            ),
        ));
    }
    Ok(())
}

fn matrix_access_expected_output_shape(
    output_value: &ValueCell,
    inputs: &[ValueCell],
) -> MResult<(usize, usize)> {
    let contract = "matrix_access";
    let source_value = inputs
        .first()
        .ok_or_else(|| function_shape_contract_violation(contract, "missing matrix input"))?;
    let source = matrix_descriptor(source_value)?.ok_or_else(|| {
        function_shape_contract_violation(contract, "input 0 must be matrix-backed")
    })?;
    let output = matrix_descriptor(output_value)?;
    match inputs.len() {
        2 => {
            let selector = inputs
                .get(1)
                .ok_or_else(|| function_shape_contract_violation(contract, "missing input 1"))?;
            let upper = matrix_access_binary_upper_bound(source, selector, output)?;
            let selection = matrix_access_selection(selector, upper, 1)?;
            matrix_access_binary_output_shape(source, selection, output)
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
            Ok((rows, cols))
        }
        found => {
            return Err(function_shape_contract_violation(
                contract,
                format!("expected 2 or 3 inputs including the source, found {found}"),
            ));
        }
    }
}

fn planned_matrix_access_output_shape(
    output: &ValueCell,
    inputs: &[ValueCell],
) -> MResult<Option<ShapeInstance>> {
    let has_logical_selector = inputs.iter().skip(1).any(|input| {
        matches!(
            input.closed_schema_body(),
            Ok(SchemaBody::Matrix { element, .. }) if matches!(element.as_ref(), SchemaBody::Bool)
        )
    });
    if !has_logical_selector {
        return Ok(None);
    }
    if matrix_descriptor(output)?.is_none() {
        return Ok(Some(output.shape().clone()));
    }
    let (rows, columns) = matrix_access_expected_output_shape(output, inputs)?;
    let schemas = output.snapshot()?.schemas().ok_or_else(|| {
        function_shape_contract_violation("matrix_access", "missing output schema table")
    })?;
    let schema = schemas.entry(output.schema()).ok_or_else(|| {
        function_shape_contract_violation("matrix_access", "missing output schema")
    })?;
    shape_for_resolved_extents(schema.schema(), &[rows as u64, columns as u64])
        .map(Some)
        .map_err(|error| MechError::new(error, None).with_compiler_loc())
}

fn planned_matrix_access_all_range_output_shape(
    output: &ValueCell,
    inputs: &[ValueCell],
) -> MResult<Option<ShapeInstance>> {
    let Some(selector) = inputs.get(1) else {
        return Err(function_shape_contract_violation(
            "matrix_access_all_range",
            "missing column selector",
        ));
    };
    if !matches!(
        selector.closed_schema_body(),
        Ok(SchemaBody::Matrix { element, .. }) if matches!(element.as_ref(), SchemaBody::Bool)
    ) {
        return Ok(None);
    }
    let source = matrix_descriptor(inputs.first().ok_or_else(|| {
        function_shape_contract_violation("matrix_access_all_range", "missing matrix input")
    })?)?
    .ok_or_else(|| {
        function_shape_contract_violation(
            "matrix_access_all_range",
            "input 0 must be matrix-backed",
        )
    })?;
    let columns = matrix_access_selection(selector, source.cols, 1)?.count(source.cols);
    let schemas = output.snapshot()?.schemas().ok_or_else(|| {
        function_shape_contract_violation("matrix_access_all_range", "missing output schema table")
    })?;
    let schema = schemas.entry(output.schema()).ok_or_else(|| {
        function_shape_contract_violation("matrix_access_all_range", "missing output schema")
    })?;
    shape_for_resolved_extents(schema.schema(), &[source.rows as u64, columns as u64])
        .map(Some)
        .map_err(|error| MechError::new(error, None).with_compiler_loc())
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

    fn u8_elements(value: &ValueCell) -> Vec<u8> {
        value
            .matrix_elements()
            .unwrap()
            .expect("u8 matrix elements")
            .iter()
            .map(|element| match element.snapshot().unwrap().data() {
                ValueData::U8(value) => *value,
                other => panic!("expected u8 matrix element, found {other:?}"),
            })
            .collect()
    }

    #[cfg(feature = "string")]
    fn string_elements(value: &ValueCell) -> Vec<String> {
        value
            .matrix_elements()
            .unwrap()
            .expect("String matrix elements")
            .iter()
            .map(|element| match element.snapshot().unwrap().data() {
                ValueData::String(value) => value.to_string(),
                other => panic!("expected String matrix element, found {other:?}"),
            })
            .collect()
    }

    fn replace_exact<T>(cell: &ValueCell, value: T)
    where
        T: CanonicalCellBacking,
    {
        cell.replace(&ValueCell::from_exact(value).unwrap().snapshot().unwrap())
            .unwrap();
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
        let source =
            ValueCell::from_exact(DMatrix::from_row_slice(2, 2, &[10_u8, 20, 30, 40])).unwrap();
        let ixes = ValueCell::from_exact(DVector::from_vec(vec![1_usize, 2])).unwrap();
        let out = ValueCell::from_exact(DVector::from_element(2, 0_u8)).unwrap();
        let invocation = FunctionInvocation::binary(out.clone(), source, ixes.clone());
        let function = crate::test_support::managed_factory_instance::<Access1DVDMD<u8>>(
            invocation,
            "test/access-index-vector",
        )
        .unwrap();

        function.instance().solve_result().unwrap();
        assert_eq!(u8_elements(&out), vec![10, 30]);

        replace_exact(&ixes, DVector::from_vec(vec![1_usize, 2, 3]));
        assert!(function.instance().solve_result().is_err());
        assert_eq!(u8_elements(&out), vec![10, 30]);
    }

    #[cfg(feature = "string")]
    #[test]
    fn typed_string_gather_uses_prospective_managed_admission() {
        let source = ValueCell::from_exact(DMatrix::from_row_slice(
            2,
            2,
            &[
                "first".to_owned(),
                "second".to_owned(),
                "third".to_owned(),
                "fourth".to_owned(),
            ],
        ))
        .unwrap();
        let ixes = ValueCell::from_exact(DVector::from_vec(vec![4_usize, 1, 4])).unwrap();
        let out = ValueCell::from_exact(DVector::from_element(3, String::new())).unwrap();
        let invocation = FunctionInvocation::binary(out.clone(), source, ixes);
        let function = crate::test_support::managed_factory_instance::<Access1DVDMD<String>>(
            invocation,
            "test/access-string-gather",
        )
        .unwrap();

        function.instance().solve_result().unwrap();
        assert_eq!(
            string_elements(&out),
            vec!["fourth".to_owned(), "first".to_owned(), "fourth".to_owned()]
        );
    }

    #[test]
    fn legacy_rectangle_factory_executes_through_live_managed_ports() {
        type Factory = Access2DRRVUU<u8, DMatrix<u8>, DMatrix<u8>, DVector<usize>, DVector<usize>>;
        let source =
            ValueCell::from_exact(DMatrix::from_row_slice(3, 2, &[10_u8, 11, 20, 21, 30, 31]))
                .unwrap();
        let rows = indices(vec![3, 1]);
        let columns = indices(vec![2, 1]);
        let out = ValueCell::from_exact(DMatrix::from_element(2, 2, 0_u8)).unwrap();
        let invocation =
            FunctionInvocation::ternary(out.clone(), source, rows.clone(), columns.clone());
        let function = crate::test_support::managed_factory_instance::<Factory>(
            invocation,
            "test/access-rectangle",
        )
        .unwrap();

        function.instance().solve_result().unwrap();
        assert_eq!(u8_elements(&out), vec![31, 30, 11, 10]);

        replace_exact(&rows, DVector::from_vec(vec![2_usize]));
        replace_exact(&columns, DVector::from_vec(vec![1_usize, 2]));
        assert!(function.instance().solve_result().is_err());
        assert_eq!(u8_elements(&out), vec![31, 30, 11, 10]);
    }

    #[test]
    fn legacy_all_rows_factory_executes_through_live_managed_ports() {
        type Factory = Access2DARV<u8, DMatrix<u8>, DMatrix<u8>, DVector<usize>>;
        let source =
            ValueCell::from_exact(DMatrix::from_row_slice(3, 2, &[10_u8, 11, 20, 21, 30, 31]))
                .unwrap();
        let columns = indices(vec![2, 1]);
        let out = ValueCell::from_exact(DMatrix::from_element(3, 2, 0_u8)).unwrap();
        let invocation = FunctionInvocation::binary(out.clone(), source, columns);
        let function = crate::test_support::managed_factory_instance::<Factory>(
            invocation,
            "test/access-columns",
        )
        .unwrap();

        function.instance().solve_result().unwrap();
        assert_eq!(u8_elements(&out), vec![11, 10, 21, 20, 31, 30]);
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
        let source = ValueCell::from_exact(DVector::from_vec(vec![10_u8, 20, 30])).unwrap();
        let ixes = ValueCell::from_exact(DVector::from_vec(vec![true, false, true])).unwrap();
        let out = ValueCell::from_exact(DVector::from_element(2, 0_u8)).unwrap();
        let invocation = FunctionInvocation::binary(out.clone(), source, ixes.clone());
        let function = crate::test_support::managed_factory_instance::<Access1DVDbVD<u8>>(
            invocation,
            "test/access-logical-vector",
        )
        .unwrap();

        function.instance().solve_result().unwrap();
        assert_eq!(u8_elements(&out), vec![10, 30]);

        replace_exact(&ixes, DVector::from_vec(vec![false, false, false]));
        function.instance().solve_result().unwrap();
        assert!(u8_elements(&out).is_empty());

        replace_exact(&ixes, DVector::from_vec(vec![false, true, false]));
        function.instance().solve_result().unwrap();
        assert_eq!(u8_elements(&out), vec![20]);
    }

    #[cfg(feature = "bool")]
    #[test]
    fn reactive_logical_matrix_selection_regrows_from_empty() {
        let source =
            ValueCell::from_exact(DMatrix::from_row_slice(3, 2, &[10_u8, 11, 20, 21, 30, 31]))
                .unwrap();
        let ixes = ValueCell::from_exact(DVector::from_vec(vec![true, false, true])).unwrap();
        let out = ValueCell::from_exact(DMatrix::from_element(2, 2, 0_u8)).unwrap();
        let invocation = FunctionInvocation::binary(out.clone(), source, ixes.clone());
        let function = crate::test_support::managed_factory_instance::<Access2DVDbAMD<u8>>(
            invocation,
            "test/access-logical-rows",
        )
        .unwrap();

        function.instance().solve_result().unwrap();
        assert_eq!(u8_elements(&out), vec![10, 11, 30, 31]);
        assert_eq!(matrix_descriptor(&out).unwrap().unwrap().rows, 2);

        replace_exact(&ixes, DVector::from_vec(vec![false, false, false]));
        function.instance().solve_result().unwrap();
        assert_eq!(matrix_descriptor(&out).unwrap().unwrap().rows, 0);

        replace_exact(&ixes, DVector::from_vec(vec![false, true, false]));
        function.instance().solve_result().unwrap();
        assert_eq!(u8_elements(&out), vec![20, 21]);
        assert_eq!(matrix_descriptor(&out).unwrap().unwrap().rows, 1);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ManagedMatrixAccessKernel {
    LinearScalar,
    LinearGather,
    Column,
    Row,
    Rows,
    Columns,
    Rectangle,
    ScalarRowColumns,
    RowsScalarColumn,
    ScalarCell,
}

macro_rules! managed_access_kernel {
    (access_1d) => {
        ManagedMatrixAccessKernel::LinearScalar
    };
    (access_1d_slice) => {
        ManagedMatrixAccessKernel::LinearGather
    };
    (access_1d_slice_bool_v) => {
        ManagedMatrixAccessKernel::LinearGather
    };
    (access_col) => {
        ManagedMatrixAccessKernel::Column
    };
    (access_row) => {
        ManagedMatrixAccessKernel::Row
    };
    (access_2d_slice_all) => {
        ManagedMatrixAccessKernel::Rows
    };
    (access_2d_slice_all_bool) => {
        ManagedMatrixAccessKernel::Rows
    };
    (assign_2d_all_range_v) => {
        ManagedMatrixAccessKernel::Columns
    };
    (assign_2d_all_range_vb) => {
        ManagedMatrixAccessKernel::Columns
    };
    (access_2d_range_range_vbb) => {
        ManagedMatrixAccessKernel::Rectangle
    };
    (access_2d_range_range_vbu) => {
        ManagedMatrixAccessKernel::Rectangle
    };
    (access_2d_range_range_vuu) => {
        ManagedMatrixAccessKernel::Rectangle
    };
    (access_2d_range_range_vub) => {
        ManagedMatrixAccessKernel::Rectangle
    };
    (access_2d) => {
        ManagedMatrixAccessKernel::ScalarCell
    };
    (access_2d_row_slice) => {
        ManagedMatrixAccessKernel::ScalarRowColumns
    };
    (access_2d_row_slice_bool) => {
        ManagedMatrixAccessKernel::ScalarRowColumns
    };
    (access_2d_col_slice) => {
        ManagedMatrixAccessKernel::RowsScalarColumn
    };
    (access_2d_col_slice_bool) => {
        ManagedMatrixAccessKernel::RowsScalarColumn
    };
}

macro_rules! managed_access_contract {
    (access_2d_range_range_vbb) => {
        PURE_TERNARY_LOGICAL_ROWS_LOGICAL_COLUMNS_CONTRACT
    };
    (access_2d_range_range_vbu) => {
        PURE_TERNARY_LOGICAL_ROWS_EXPLICIT_COLUMNS_CONTRACT
    };
    (access_2d_range_range_vuu) => {
        PURE_TERNARY_EXPLICIT_ROWS_EXPLICIT_COLUMNS_CONTRACT
    };
    (access_2d_range_range_vub) => {
        PURE_TERNARY_EXPLICIT_ROWS_LOGICAL_COLUMNS_CONTRACT
    };
}

trait ManagedAccessSelectorElement: mech_core::ManagedElement + FunctionPortBacking {
    const LOGICAL: bool;

    fn ordinal(self, upper: usize) -> MResult<usize>;
    fn selected(self) -> bool;
}

impl ManagedAccessSelectorElement for usize {
    const LOGICAL: bool = false;

    fn ordinal(self, upper: usize) -> MResult<usize> {
        if self == 0 || self > upper {
            return Err(function_shape_contract_violation(
                "matrix_access",
                format!("index {self} is outside 1..={upper}"),
            ));
        }
        Ok(self - 1)
    }

    fn selected(self) -> bool {
        true
    }
}

#[cfg(feature = "bool")]
impl ManagedAccessSelectorElement for bool {
    const LOGICAL: bool = true;

    fn ordinal(self, _upper: usize) -> MResult<usize> {
        Err(function_shape_contract_violation(
            "matrix_access",
            "logical selector cannot be converted to a positional index",
        ))
    }

    fn selected(self) -> bool {
        self
    }
}

trait ManagedAccessSelectorBacking {
    type Element: ManagedAccessSelectorElement;

    fn validate(port: FunctionInputPort<'_>) -> MResult<()> {
        let _ = port.try_managed_element::<Self::Element>()?;
        Ok(())
    }
}

impl ManagedAccessSelectorBacking for usize {
    type Element = usize;
}

impl<T, R, C, S> ManagedAccessSelectorBacking for naMatrix<T, R, C, S>
where
    T: ManagedAccessSelectorElement,
    R: Dim,
    C: Dim,
    S: RawStorage<T, R, C>,
{
    type Element = T;
}

fn validate_selector_view<S: ManagedAccessSelectorElement>(
    selector: &mech_core::ManagedValueView<'_, S>,
    upper: usize,
    require_scalar: bool,
) -> MResult<usize> {
    if require_scalar && selector.len() != 1 {
        return Err(function_shape_contract_violation(
            "matrix_access",
            format!("scalar selector contains {} elements", selector.len()),
        ));
    }
    if S::LOGICAL {
        if selector.len() != upper {
            return Err(function_shape_contract_violation(
                "matrix_access",
                format!(
                    "logical selector has {} elements, expected {upper}",
                    selector.len()
                ),
            ));
        }
        let mut selected = 0usize;
        for index in 0..selector.len() {
            if selector
                .get_column_major(index)
                .expect("validated selector geometry")
                .selected()
            {
                selected += 1;
            }
        }
        return Ok(selected);
    }
    for index in 0..selector.len() {
        selector
            .get_column_major(index)
            .expect("validated selector geometry")
            .ordinal(upper)?;
    }
    Ok(selector.len())
}

fn selected_position<S: ManagedAccessSelectorElement>(
    selector: &mech_core::ManagedValueView<'_, S>,
    ordinal: usize,
    upper: usize,
) -> MResult<usize> {
    if !S::LOGICAL {
        return selector
            .get_column_major(ordinal)
            .ok_or_else(|| {
                function_shape_contract_violation(
                    "matrix_access",
                    format!("selector ordinal {ordinal} is outside its live extent"),
                )
            })?
            .ordinal(upper);
    }
    let mut found = 0usize;
    for index in 0..selector.len() {
        if selector
            .get_column_major(index)
            .expect("validated selector geometry")
            .selected()
        {
            if found == ordinal {
                return Ok(index);
            }
            found += 1;
        }
    }
    Err(function_shape_contract_violation(
        "matrix_access",
        format!("logical selector has no selected ordinal {ordinal}"),
    ))
}

fn execute_fixed_binary_access<T, S>(
    source: mech_core::ManagedValueView<'_, T>,
    selector: mech_core::ManagedValueView<'_, S>,
    output: &mut mech_core::ManagedValueViewMut<'_, T>,
    kernel: ManagedMatrixAccessKernel,
) -> MResult<()>
where
    T: mech_core::ManagedElement,
    S: ManagedAccessSelectorElement,
{
    let (upper, scalar) = match kernel {
        ManagedMatrixAccessKernel::LinearScalar => (source.len(), true),
        ManagedMatrixAccessKernel::LinearGather => (source.len(), false),
        ManagedMatrixAccessKernel::Column => (source.columns(), true),
        ManagedMatrixAccessKernel::Row => (source.rows(), true),
        ManagedMatrixAccessKernel::Rows => (source.rows(), false),
        ManagedMatrixAccessKernel::Columns => (source.columns(), false),
        _ => {
            return Err(function_shape_contract_violation(
                "matrix_access",
                "binary factory selected an incompatible managed kernel",
            ));
        }
    };
    let selected = validate_selector_view(&selector, upper, scalar)?;
    match kernel {
        ManagedMatrixAccessKernel::LinearScalar => {
            if output.len() != 1 {
                return Err(function_shape_contract_violation(
                    "matrix_access",
                    "scalar selection requires one output element",
                ));
            }
            let source_index = selected_position(&selector, 0, source.len())?;
            output.try_fill_column_major(|_| {
                source.get_column_major(source_index).ok_or_else(|| {
                    function_shape_contract_violation(
                        "matrix_access",
                        "linear selector is outside the source",
                    )
                })
            })
        }
        ManagedMatrixAccessKernel::LinearGather => {
            if output.len() != selected {
                return Err(function_shape_contract_violation(
                    "matrix_access",
                    format!(
                        "output has {} elements, selector requires {selected}",
                        output.len()
                    ),
                ));
            }
            output.try_fill_column_major(|index| {
                let source_index = selected_position(&selector, index, source.len())?;
                source.get_column_major(source_index).ok_or_else(|| {
                    function_shape_contract_violation(
                        "matrix_access",
                        "linear selector is outside the source",
                    )
                })
            })
        }
        ManagedMatrixAccessKernel::Column => {
            if output.len() != source.rows() {
                return Err(function_shape_contract_violation(
                    "matrix_access",
                    "column selection output has the wrong extent",
                ));
            }
            let column = selected_position(&selector, 0, source.columns())?;
            output.try_fill_column_major(|row| {
                source.get(row, column).ok_or_else(|| {
                    function_shape_contract_violation(
                        "matrix_access",
                        "column selection is outside the source",
                    )
                })
            })
        }
        ManagedMatrixAccessKernel::Row => {
            if output.len() != source.columns() {
                return Err(function_shape_contract_violation(
                    "matrix_access",
                    "row selection output has the wrong extent",
                ));
            }
            let row = selected_position(&selector, 0, source.rows())?;
            output.try_fill_column_major(|column| {
                source.get(row, column).ok_or_else(|| {
                    function_shape_contract_violation(
                        "matrix_access",
                        "row selection is outside the source",
                    )
                })
            })
        }
        ManagedMatrixAccessKernel::Rows => {
            if output.rows() != selected || output.columns() != source.columns() {
                return Err(function_shape_contract_violation(
                    "matrix_access",
                    "row selection output geometry is inconsistent",
                ));
            }
            let output_rows = output.rows();
            output.try_fill_column_major(|index| {
                let output_row = index % output_rows;
                let column = index / output_rows;
                let source_row = selected_position(&selector, output_row, source.rows())?;
                source.get(source_row, column).ok_or_else(|| {
                    function_shape_contract_violation(
                        "matrix_access",
                        "row selection is outside the source",
                    )
                })
            })
        }
        ManagedMatrixAccessKernel::Columns => {
            if output.rows() != source.rows() || output.columns() != selected {
                return Err(function_shape_contract_violation(
                    "matrix_access",
                    "column selection output geometry is inconsistent",
                ));
            }
            let output_rows = output.rows();
            output.try_fill_column_major(|index| {
                let row = index % output_rows.max(1);
                let output_column = index / output_rows.max(1);
                let source_column = selected_position(&selector, output_column, source.columns())?;
                source.get(row, source_column).ok_or_else(|| {
                    function_shape_contract_violation(
                        "matrix_access",
                        "column selection is outside the source",
                    )
                })
            })
        }
        _ => unreachable!(),
    }
}

fn execute_fixed_ternary_access<T, R, C>(
    source: mech_core::ManagedValueView<'_, T>,
    rows: mech_core::ManagedValueView<'_, R>,
    columns: mech_core::ManagedValueView<'_, C>,
    output: &mut mech_core::ManagedValueViewMut<'_, T>,
    kernel: ManagedMatrixAccessKernel,
) -> MResult<()>
where
    T: mech_core::ManagedElement,
    R: ManagedAccessSelectorElement,
    C: ManagedAccessSelectorElement,
{
    let scalar_rows = matches!(
        kernel,
        ManagedMatrixAccessKernel::ScalarCell | ManagedMatrixAccessKernel::ScalarRowColumns
    );
    let scalar_columns = matches!(
        kernel,
        ManagedMatrixAccessKernel::ScalarCell | ManagedMatrixAccessKernel::RowsScalarColumn
    );
    let selected_rows = validate_selector_view(&rows, source.rows(), scalar_rows)?;
    let selected_columns = validate_selector_view(&columns, source.columns(), scalar_columns)?;
    if output.rows().saturating_mul(output.columns())
        != selected_rows.saturating_mul(selected_columns)
        || output.len() != selected_rows.saturating_mul(selected_columns)
    {
        return Err(function_shape_contract_violation(
            "matrix_access",
            "rectangular selection output geometry is inconsistent",
        ));
    }
    output.try_fill_column_major(|index| {
        let output_row = index % selected_rows.max(1);
        let output_column = index / selected_rows.max(1);
        let source_row = selected_position(&rows, output_row, source.rows())?;
        let source_column = selected_position(&columns, output_column, source.columns())?;
        source.get(source_row, source_column).ok_or_else(|| {
            function_shape_contract_violation(
                "matrix_access",
                "rectangular selector is outside the source",
            )
        })
    })
}

trait ManagedAccessElement:
    Debug
    + Clone
    + Sync
    + Send
    + PartialEq
    + 'static
    + ConstElem
    + FunctionRuntimeType
    + CanonicalMatrixElementBacking
{
    const MEMORY_CLASS: mech_core::ImplementationMemoryClass;

    fn validate_input(port: FunctionInputPort<'_>) -> MResult<()>;
    fn validate_output(port: FunctionOutputPort<'_>) -> MResult<()>;
    fn planned_binary_output_footprint(
        _source: &FunctionValueInput,
        _selector: &FunctionValueInput,
        _output: &FunctionValueOutput,
        _kernel: ManagedMatrixAccessKernel,
    ) -> MResult<Option<CurrentMemoryFootprint>> {
        Ok(None)
    }
    fn planned_ternary_output_footprint(
        _source: &FunctionValueInput,
        _rows: &FunctionValueInput,
        _columns: &FunctionValueInput,
        _output: &FunctionValueOutput,
    ) -> MResult<Option<CurrentMemoryFootprint>> {
        Ok(None)
    }
    fn planned_all_output_footprint(
        _source: &FunctionValueInput,
        _output: &FunctionValueOutput,
    ) -> MResult<Option<CurrentMemoryFootprint>> {
        Ok(None)
    }
    fn solve_binary<S: ManagedAccessSelectorElement>(
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        source: &FunctionValueInput,
        selector: &FunctionValueInput,
        output: &FunctionValueOutput,
        kernel: ManagedMatrixAccessKernel,
    ) -> MResult<()>;
    fn solve_ternary<R: ManagedAccessSelectorElement, C: ManagedAccessSelectorElement>(
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        source: &FunctionValueInput,
        rows: &FunctionValueInput,
        columns: &FunctionValueInput,
        output: &FunctionValueOutput,
        kernel: ManagedMatrixAccessKernel,
    ) -> MResult<()>;
    fn solve_all(
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        source: &FunctionValueInput,
        output: &FunctionValueOutput,
    ) -> MResult<()>;
}

macro_rules! impl_managed_fixed_access_element {
    ($($type:ty),+ $(,)?) => {$(
        impl ManagedAccessElement for $type {
            const MEMORY_CLASS: mech_core::ImplementationMemoryClass =
                mech_core::ImplementationMemoryClass::NoAdditionalScratch;

            fn validate_input(port: FunctionInputPort<'_>) -> MResult<()> {
                let _ = port.try_managed_element::<Self>()?;
                Ok(())
            }

            fn validate_output(port: FunctionOutputPort<'_>) -> MResult<()> {
                let _ = port.try_managed_element::<Self>()?;
                Ok(())
            }

            fn solve_binary<S: ManagedAccessSelectorElement>(
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                source: &FunctionValueInput,
                selector: &FunctionValueInput,
                output: &FunctionValueOutput,
                kernel: ManagedMatrixAccessKernel,
            ) -> MResult<()> {
                frame.with_binary_function_value_views::<Self, S, Self, _>(
                    source,
                    selector,
                    output,
                    |source, selector, output| {
                        execute_fixed_binary_access(source, selector, output, kernel)
                    },
                )
            }

            fn solve_ternary<R: ManagedAccessSelectorElement, C: ManagedAccessSelectorElement>(
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                source: &FunctionValueInput,
                rows: &FunctionValueInput,
                columns: &FunctionValueInput,
                output: &FunctionValueOutput,
                kernel: ManagedMatrixAccessKernel,
            ) -> MResult<()> {
                frame.with_ternary_function_value_views::<Self, R, C, Self, _>(
                    source,
                    rows,
                    columns,
                    output,
                    |source, rows, columns, output| {
                        execute_fixed_ternary_access(source, rows, columns, output, kernel)
                    },
                )
            }

            fn solve_all(
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                source: &FunctionValueInput,
                output: &FunctionValueOutput,
            ) -> MResult<()> {
                frame.with_unary_function_value_views::<Self, Self, _>(
                    source,
                    output,
                    |source, output| {
                        if source.len() != output.len() {
                            return Err(function_shape_contract_violation(
                                "matrix_access",
                                "all-elements output has the wrong extent",
                            ));
                        }
                        output.try_fill_column_major(|index| {
                            source.get_column_major(index).ok_or_else(|| {
                                function_shape_contract_violation(
                                    "matrix_access",
                                    "all-elements source geometry is inconsistent",
                                )
                            })
                        })
                    },
                )
            }
        }
    )+};
}

#[cfg(feature = "u8")]
impl_managed_fixed_access_element!(u8);
#[cfg(feature = "u16")]
impl_managed_fixed_access_element!(u16);
#[cfg(feature = "u32")]
impl_managed_fixed_access_element!(u32);
#[cfg(feature = "u64")]
impl_managed_fixed_access_element!(u64);
#[cfg(feature = "u128")]
impl_managed_fixed_access_element!(u128);
#[cfg(feature = "i8")]
impl_managed_fixed_access_element!(i8);
#[cfg(feature = "i16")]
impl_managed_fixed_access_element!(i16);
#[cfg(feature = "i32")]
impl_managed_fixed_access_element!(i32);
#[cfg(feature = "i64")]
impl_managed_fixed_access_element!(i64);
#[cfg(feature = "i128")]
impl_managed_fixed_access_element!(i128);
#[cfg(feature = "f32")]
impl_managed_fixed_access_element!(f32);
#[cfg(feature = "f64")]
impl_managed_fixed_access_element!(f64);
impl_managed_fixed_access_element!(usize);
#[cfg(feature = "bool")]
impl_managed_fixed_access_element!(bool);
#[cfg(feature = "complex")]
impl_managed_fixed_access_element!(C64);
#[cfg(feature = "rational")]
impl_managed_fixed_access_element!(R64);

#[cfg(feature = "string")]
impl ManagedAccessElement for String {
    const MEMORY_CLASS: mech_core::ImplementationMemoryClass =
        mech_core::ImplementationMemoryClass::CanonicalCloneInput { input: 0 };

    fn validate_input(port: FunctionInputPort<'_>) -> MResult<()> {
        let _ = port.try_managed_element::<Self>()?;
        Ok(())
    }

    fn validate_output(port: FunctionOutputPort<'_>) -> MResult<()> {
        let _ = port.try_managed_element::<Self>()?;
        Ok(())
    }

    fn planned_binary_output_footprint(
        source: &FunctionValueInput,
        selector: &FunctionValueInput,
        output: &FunctionValueOutput,
        kernel: ManagedMatrixAccessKernel,
    ) -> MResult<Option<CurrentMemoryFootprint>> {
        #[cfg(feature = "semantic-compiler")]
        {
            let access = string_binary_access(source, selector, output, kernel);
            return access
                .planned_output_footprints()
                .map(|footprints| footprints.and_then(|values| values.first().copied()));
        }
        #[cfg(not(feature = "semantic-compiler"))]
        {
            let _ = (source, selector, output, kernel);
            Ok(None)
        }
    }

    fn planned_ternary_output_footprint(
        source: &FunctionValueInput,
        rows: &FunctionValueInput,
        columns: &FunctionValueInput,
        output: &FunctionValueOutput,
    ) -> MResult<Option<CurrentMemoryFootprint>> {
        #[cfg(feature = "semantic-compiler")]
        {
            let access = string_ternary_access(source, rows, columns, output);
            return access
                .planned_output_footprints()
                .map(|footprints| footprints.and_then(|values| values.first().copied()));
        }
        #[cfg(not(feature = "semantic-compiler"))]
        {
            let _ = (source, rows, columns, output);
            Ok(None)
        }
    }

    fn planned_all_output_footprint(
        source: &FunctionValueInput,
        output: &FunctionValueOutput,
    ) -> MResult<Option<CurrentMemoryFootprint>> {
        #[cfg(feature = "semantic-compiler")]
        {
            let access = string_all_access(source, output);
            return access
                .planned_output_footprints()
                .map(|footprints| footprints.and_then(|values| values.first().copied()));
        }
        #[cfg(not(feature = "semantic-compiler"))]
        {
            let _ = (source, output);
            Ok(None)
        }
    }

    fn solve_binary<S: ManagedAccessSelectorElement>(
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        source: &FunctionValueInput,
        selector: &FunctionValueInput,
        output: &FunctionValueOutput,
        kernel: ManagedMatrixAccessKernel,
    ) -> MResult<()> {
        #[cfg(feature = "semantic-compiler")]
        return string_binary_access(source, selector, output, kernel).stage_managed(frame);
        #[cfg(not(feature = "semantic-compiler"))]
        {
            let _ = (frame, source, selector, output, kernel);
            Err(function_shape_contract_violation(
                "matrix_access",
                "String matrix access requires canonical runtime support",
            ))
        }
    }

    fn solve_ternary<R: ManagedAccessSelectorElement, C: ManagedAccessSelectorElement>(
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        source: &FunctionValueInput,
        rows: &FunctionValueInput,
        columns: &FunctionValueInput,
        output: &FunctionValueOutput,
        _kernel: ManagedMatrixAccessKernel,
    ) -> MResult<()> {
        #[cfg(feature = "semantic-compiler")]
        return string_ternary_access(source, rows, columns, output).stage_managed(frame);
        #[cfg(not(feature = "semantic-compiler"))]
        {
            let _ = (frame, source, rows, columns, output);
            Err(function_shape_contract_violation(
                "matrix_access",
                "String matrix access requires canonical runtime support",
            ))
        }
    }

    fn solve_all(
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        source: &FunctionValueInput,
        output: &FunctionValueOutput,
    ) -> MResult<()> {
        #[cfg(feature = "semantic-compiler")]
        return string_all_access(source, output).stage_managed(frame);
        #[cfg(not(feature = "semantic-compiler"))]
        {
            let _ = (frame, source, output);
            Err(function_shape_contract_violation(
                "matrix_access",
                "String matrix access requires canonical runtime support",
            ))
        }
    }
}

#[cfg(all(feature = "string", feature = "semantic-compiler"))]
fn string_binary_access(
    source: &FunctionValueInput,
    selector: &FunctionValueInput,
    output: &FunctionValueOutput,
    kernel: ManagedMatrixAccessKernel,
) -> super::CanonicalAccess {
    let selectors = match kernel {
        ManagedMatrixAccessKernel::Column => vec![
            crate::intrinsics::canonical_access::CanonicalAccessSelector::All,
            crate::intrinsics::canonical_access::CanonicalAccessSelector::Cell(
                selector.cell().clone(),
            ),
        ],
        ManagedMatrixAccessKernel::Row | ManagedMatrixAccessKernel::Rows => vec![
            crate::intrinsics::canonical_access::CanonicalAccessSelector::Cell(
                selector.cell().clone(),
            ),
            crate::intrinsics::canonical_access::CanonicalAccessSelector::All,
        ],
        _ => vec![
            crate::intrinsics::canonical_access::CanonicalAccessSelector::Cell(
                selector.cell().clone(),
            ),
        ],
    };
    super::CanonicalAccess::typed_matrix(source.cell().clone(), selectors, output.cell().clone())
}

#[cfg(all(feature = "string", feature = "semantic-compiler"))]
fn string_ternary_access(
    source: &FunctionValueInput,
    rows: &FunctionValueInput,
    columns: &FunctionValueInput,
    output: &FunctionValueOutput,
) -> super::CanonicalAccess {
    super::CanonicalAccess::typed_matrix(
        source.cell().clone(),
        vec![
            crate::intrinsics::canonical_access::CanonicalAccessSelector::Cell(rows.cell().clone()),
            crate::intrinsics::canonical_access::CanonicalAccessSelector::Cell(
                columns.cell().clone(),
            ),
        ],
        output.cell().clone(),
    )
}

#[cfg(all(feature = "string", feature = "semantic-compiler"))]
fn string_all_access(
    source: &FunctionValueInput,
    output: &FunctionValueOutput,
) -> super::CanonicalAccess {
    super::CanonicalAccess::typed_matrix(
        source.cell().clone(),
        vec![crate::intrinsics::canonical_access::CanonicalAccessSelector::All],
        output.cell().clone(),
    )
}

macro_rules! impl_access_fxn {
    ($struct_name:ident, $arg_type:ty, $ix_type:ty, $out_type:ty, $op:ident, $contract:ident) => {
        #[derive(Debug)]
        struct $struct_name<T> {
            source: FunctionValueInput,
            ixes: FunctionValueInput,
            out: FunctionValueOutput,
            invocation: FunctionInvocation,
            marker: core::marker::PhantomData<fn() -> T>,
        }
        impl<T> MechFunctionFactory for $struct_name<T>
        where
            T: ManagedAccessElement,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst,
            $arg_type: FunctionPortBacking,
            $ix_type: FunctionPortBacking + ManagedAccessSelectorBacking,
            $out_type: FunctionStateBacking,
        {
            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                T::MEMORY_CLASS
            }

            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <$out_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg_type as FunctionRuntimeType>::REPRESENTATION,
                <$ix_type as FunctionRuntimeType>::REPRESENTATION,
            );

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, source, ixes) = invocation.expect_binary()?;
                T::validate_input(source)?;
                <$ix_type as ManagedAccessSelectorBacking>::validate(ixes)?;
                T::validate_output(out)?;
                Ok(Box::new($struct_name {
                    source: source.value(),
                    ixes: ixes.value(),
                    out: out.value(),
                    invocation,
                    marker: core::marker::PhantomData::<fn() -> T>,
                }))
            }
        }
        impl<T> MechFunctionImpl for $struct_name<T>
        where
            T: ManagedAccessElement,
            $ix_type: ManagedAccessSelectorBacking,
            $out_type: FunctionStateBacking,
        {
            fn planned_output_shapes(&self) -> MResult<Option<Box<[ShapeInstance]>>> {
                Ok(planned_matrix_access_output_shape(
                    self.invocation.output_cell(),
                    self.invocation.input_cells(),
                )?
                .map(|shape| vec![shape].into_boxed_slice()))
            }

            fn planned_output_footprints(&self) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
                Ok(T::planned_binary_output_footprint(
                    &self.source,
                    &self.ixes,
                    &self.out,
                    managed_access_kernel!($op),
                )?
                .map(|footprint| vec![footprint].into_boxed_slice()))
            }

            fn solve_managed(
                &self,
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                _services: &mut dyn mech_core::MechExecutionServices,
            ) -> MResult<mech_core::ReactiveSolveStatus> {
                validate_matrix_access_contract(
                    self.invocation.output_cell(),
                    self.invocation.input_cells(),
                )?;
                T::solve_binary::<<$ix_type as ManagedAccessSelectorBacking>::Element>(
                    frame,
                    &self.source,
                    &self.ixes,
                    &self.out,
                    managed_access_kernel!($op),
                )?;
                Ok(mech_core::ReactiveSolveStatus::Changed)
            }
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_cell(self.out.cell()))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_cell(self.out.cell())]))
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
            T: ManagedAccessElement + CompileConst,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}>",
                    stringify!($struct_name),
                    <T as FunctionRuntimeType>::REPRESENTATION
                );
                let out = compile_value_cell_register(self.out.cell(), ctx)?;
                let source = compile_value_cell_register(self.source.cell(), ctx)?;
                let ixes = compile_value_cell_register(self.ixes.cell(), ctx)?;
                let function = ctx.function_id(&name)?;
                ctx.emit_binop(function, out, source, ixes);
                Ok(out)
            }
        }
    };
}

macro_rules! impl_access_all_fxn {
    ($struct_name:ident, $arg_type:ty, $out_type:ty, $contract:ident) => {
        #[derive(Debug)]
        struct $struct_name<T> {
            source: FunctionValueInput,
            out: FunctionValueOutput,
            invocation: FunctionInvocation,
            marker: core::marker::PhantomData<fn() -> T>,
        }

        impl<T> MechFunctionFactory for $struct_name<T>
        where
            T: ManagedAccessElement,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst,
            $arg_type: FunctionPortBacking,
            $out_type: FunctionStateBacking,
        {
            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                T::MEMORY_CLASS
            }

            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <$out_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg_type as FunctionRuntimeType>::REPRESENTATION,
                FunctionValueRepresentation::AnyValue,
            );

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, source, _all) = invocation.expect_binary()?;
                T::validate_input(source)?;
                T::validate_output(out)?;
                Ok(Box::new($struct_name {
                    source: source.value(),
                    out: out.value(),
                    invocation,
                    marker: core::marker::PhantomData::<fn() -> T>,
                }))
            }
        }

        impl<T> MechFunctionImpl for $struct_name<T>
        where
            T: ManagedAccessElement,
            $out_type: FunctionStateBacking,
        {
            fn planned_output_shapes(&self) -> MResult<Option<Box<[ShapeInstance]>>> {
                Ok(planned_matrix_access_output_shape(
                    self.invocation.output_cell(),
                    self.invocation.input_cells(),
                )?
                .map(|shape| vec![shape].into_boxed_slice()))
            }

            fn planned_output_footprints(&self) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
                Ok(T::planned_all_output_footprint(&self.source, &self.out)?
                    .map(|footprint| vec![footprint].into_boxed_slice()))
            }

            fn solve_managed(
                &self,
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                _services: &mut dyn mech_core::MechExecutionServices,
            ) -> MResult<mech_core::ReactiveSolveStatus> {
                validate_matrix_access_all_elements_contract(
                    self.invocation.output_cell(),
                    self.invocation.input_cells(),
                )?;
                T::solve_all(frame, &self.source, &self.out)?;
                Ok(mech_core::ReactiveSolveStatus::Changed)
            }

            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_cell(self.out.cell()))
            }

            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_cell(self.out.cell())]))
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
            T: ManagedAccessElement + CompileConst,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let out = compile_value_cell_register(self.out.cell(), ctx)?;
                let source = compile_value_cell_register(self.source.cell(), ctx)?;
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
            source: FunctionValueInput,
            ix1: FunctionValueInput,
            ix2: FunctionValueInput,
            out: FunctionValueOutput,
            invocation: FunctionInvocation,
            marker: core::marker::PhantomData<fn() -> T>,
        }
        impl<T> MechFunctionFactory for $struct_name<T>
        where
            T: ManagedAccessElement,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst,
            $arg_type: FunctionPortBacking,
            $ix1_type: FunctionPortBacking + ManagedAccessSelectorBacking,
            $ix2_type: FunctionPortBacking + ManagedAccessSelectorBacking,
            $out_type: FunctionStateBacking,
        {
            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                T::MEMORY_CLASS
            }

            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::ternary(
                <$out_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg_type as FunctionRuntimeType>::REPRESENTATION,
                <$ix1_type as FunctionRuntimeType>::REPRESENTATION,
                <$ix2_type as FunctionRuntimeType>::REPRESENTATION,
            );

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, source, ix1, ix2) = invocation.expect_ternary()?;
                T::validate_input(source)?;
                <$ix1_type as ManagedAccessSelectorBacking>::validate(ix1)?;
                <$ix2_type as ManagedAccessSelectorBacking>::validate(ix2)?;
                T::validate_output(out)?;
                Ok(Box::new($struct_name {
                    source: source.value(),
                    ix1: ix1.value(),
                    ix2: ix2.value(),
                    out: out.value(),
                    invocation,
                    marker: core::marker::PhantomData::<fn() -> T>,
                }))
            }
        }
        impl<T> MechFunctionImpl for $struct_name<T>
        where
            T: ManagedAccessElement,
            $ix1_type: ManagedAccessSelectorBacking,
            $ix2_type: ManagedAccessSelectorBacking,
            $out_type: FunctionStateBacking,
        {
            fn planned_output_shapes(&self) -> MResult<Option<Box<[ShapeInstance]>>> {
                Ok(planned_matrix_access_output_shape(
                    self.invocation.output_cell(),
                    self.invocation.input_cells(),
                )?
                .map(|shape| vec![shape].into_boxed_slice()))
            }

            fn planned_output_footprints(&self) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
                Ok(T::planned_ternary_output_footprint(
                    &self.source,
                    &self.ix1,
                    &self.ix2,
                    &self.out,
                )?
                .map(|footprint| vec![footprint].into_boxed_slice()))
            }

            fn solve_managed(
                &self,
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                _services: &mut dyn mech_core::MechExecutionServices,
            ) -> MResult<mech_core::ReactiveSolveStatus> {
                validate_matrix_access_contract(
                    self.invocation.output_cell(),
                    self.invocation.input_cells(),
                )?;
                T::solve_ternary::<
                    <$ix1_type as ManagedAccessSelectorBacking>::Element,
                    <$ix2_type as ManagedAccessSelectorBacking>::Element,
                >(
                    frame,
                    &self.source,
                    &self.ix1,
                    &self.ix2,
                    &self.out,
                    managed_access_kernel!($op),
                )?;
                Ok(mech_core::ReactiveSolveStatus::Changed)
            }
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_cell(self.out.cell()))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_cell(self.out.cell())]))
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
            T: ManagedAccessElement + CompileConst,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}>",
                    stringify!($struct_name),
                    <T as FunctionRuntimeType>::REPRESENTATION
                );
                let out = compile_value_cell_register(self.out.cell(), ctx)?;
                let source = compile_value_cell_register(self.source.cell(), ctx)?;
                let ix1 = compile_value_cell_register(self.ix1.cell(), ctx)?;
                let ix2 = compile_value_cell_register(self.ix2.cell(), ctx)?;
                let function = ctx.function_id(&name)?;
                ctx.emit_ternop(function, out, source, ix1, ix2);
                Ok(out)
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

#[cfg(feature = "logical_indexing")]
impl_range_range_fxn_v!(Access2DRRVBB, access_2d_range_range_vbb, bool, bool);
#[cfg(feature = "logical_indexing")]
impl_range_range_fxn_v!(Access2DRRVBU, access_2d_range_range_vbu, bool, usize);
impl_range_range_fxn_v!(Access2DRRVUU, access_2d_range_range_vuu, usize, usize);
#[cfg(feature = "logical_indexing")]
impl_range_range_fxn_v!(Access2DRRVUB, access_2d_range_range_vub, usize, bool);

impl_all_fxn_v!(
    Access2DARV,
    assign_2d_all_range_v,
    usize,
    PURE_BINARY_ALL_ROWS_EXPLICIT_COLUMNS_CONTRACT
);
#[cfg(feature = "logical_indexing")]
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
    source: ManagedIndexInput,
    output: mech_core::ManagedPort<usize>,
}

#[cfg(any(feature = "subscript_formula", feature = "subscript_range"))]
macro_rules! managed_index_inputs {
    (
        $base_variant:ident: $base_type:ty, $base_schema:pat => $base_value:expr;
        $($feature:literal => $variant:ident: $type:ty, $schema:pat => $value:expr);+ $(;)?
    ) => {
        #[derive(Debug)]
        enum ManagedIndexInput {
            $base_variant(mech_core::ManagedPort<$base_type>),
            $(#[cfg(feature = $feature)] $variant(mech_core::ManagedPort<$type>)),+
        }

        impl ManagedIndexInput {
            fn bind(source: mech_core::FunctionInputPort<'_>, schema: &SchemaBody) -> MResult<Self> {
                match schema {
                    $base_schema => Ok(Self::$base_variant(source.try_managed_element::<$base_type>()?)),
                    $(#[cfg(feature = $feature)] $schema => Ok(Self::$variant(source.try_managed_element::<$type>()?)),)+
                    _ => Err(index_conversion_error()),
                }
            }

            #[cfg(feature = "semantic-compiler")]
            fn cell(&self) -> &ValueCell {
                match self {
                    Self::$base_variant(port) => port.cell(),
                    $(#[cfg(feature = $feature)] Self::$variant(port) => port.cell()),+
                }
            }

            fn convert(&self, frame: &mut mech_core::KernelMemoryFrame<'_>, output: &mech_core::ManagedPort<usize>) -> MResult<()> {
                match self {
                    Self::$base_variant(source) => {
                        frame.with_unary_typed_port_views(source, output, |source, output| {
                            if source.len() != output.len() {
                                return Err(index_conversion_error());
                            }
                            output.try_fill_column_major(|index| {
                                let value = source.get(index / source.columns(), index % source.columns())
                                    .ok_or_else(index_conversion_error)?;
                                let ordinal = mech_core::canonical_positional_ordinal(&($base_value)(value))
                                    .map_err(|_| index_conversion_error())?;
                                usize::try_from(ordinal).map_err(|_| index_conversion_error())
                            })
                        })
                    },
                    $(#[cfg(feature = $feature)] Self::$variant(source) => {
                        frame.with_unary_typed_port_views(source, output, |source, output| {
                            if source.len() != output.len() {
                                return Err(index_conversion_error());
                            }
                            // Canonical selector matrices flatten in row-major
                            // order even when their live arena is column-major.
                            // Conversion uses the core semantic authority and
                            // writes only the transaction's unpublished stage.
                            output.try_fill_column_major(|index| {
                                let value = source.get(index / source.columns(), index % source.columns())
                                    .ok_or_else(index_conversion_error)?;
                                let ordinal = mech_core::canonical_positional_ordinal(&($value)(value))
                                    .map_err(|_| index_conversion_error())?;
                                usize::try_from(ordinal).map_err(|_| index_conversion_error())
                            })
                        })
                    }),+
                }
            }
        }
    };
}

#[cfg(any(feature = "subscript_formula", feature = "subscript_range"))]
managed_index_inputs!(
    Index: usize, SchemaBody::Index => |value| ValueData::Index(value as u64);
    "u8" => U8: u8, SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W8) => ValueData::U8;
    "u16" => U16: u16, SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W16) => ValueData::U16;
    "u32" => U32: u32, SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W32) => ValueData::U32;
    "u64" => U64: u64, SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64) => ValueData::U64;
    "u128" => U128: u128, SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W128) => ValueData::U128;
    "i8" => I8: i8, SchemaBody::SignedInteger(mech_core::IntegerWidth::W8) => ValueData::I8;
    "i16" => I16: i16, SchemaBody::SignedInteger(mech_core::IntegerWidth::W16) => ValueData::I16;
    "i32" => I32: i32, SchemaBody::SignedInteger(mech_core::IntegerWidth::W32) => ValueData::I32;
    "i64" => I64: i64, SchemaBody::SignedInteger(mech_core::IntegerWidth::W64) => ValueData::I64;
    "i128" => I128: i128, SchemaBody::SignedInteger(mech_core::IntegerWidth::W128) => ValueData::I128;
    "f32" => F32: f32, SchemaBody::FloatingPoint(mech_core::FloatWidth::W32) => |value| ValueData::F32(mech_core::snapshot::F32Bits::from_f32(value));
    "f64" => F64: f64, SchemaBody::FloatingPoint(mech_core::FloatWidth::W64) => |value| ValueData::F64(mech_core::snapshot::F64Bits::from_f64(value));
);

#[cfg(any(feature = "subscript_formula", feature = "subscript_range"))]
fn index_conversion_error() -> MechError {
    MechError::new(
        CannotConvertToTypeError {
            target_type: "portable index",
        },
        None,
    )
    .with_compiler_loc()
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
        validate_canonical_index_conversion(invocation.output_cell(), invocation.input_cells())?;
        let schema = invocation.input_cells()[0].closed_schema_body()?;
        let element = match &schema {
            SchemaBody::Matrix { element, .. } => element.as_ref(),
            scalar => scalar,
        };
        let (output, source) = invocation.expect_unary()?;
        Ok(Box::new(Self {
            source: ManagedIndexInput::bind(source, element)?,
            output: output.try_managed_element::<usize>()?,
        }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_UNARY_INDEX_CONVERSION_CONTRACT)
    }
}

#[cfg(any(feature = "subscript_formula", feature = "subscript_range"))]
impl MechFunctionImpl for CanonicalIndexConversion {
    fn solve_managed(
        &self,
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        _services: &mut dyn mech_core::MechExecutionServices,
    ) -> MResult<mech_core::ReactiveSolveStatus> {
        self.source.convert(frame, &self.output)?;
        Ok(mech_core::ReactiveSolveStatus::Changed)
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
        let output = mech_core::compile_value_cell_register(self.output.cell(), context)?;
        let source = mech_core::compile_value_cell_register(self.source.cell(), context)?;
        context.emit_unop(hash_str("access/index"), output, source);
        Ok(output)
    }
}

#[cfg(all(
    feature = "semantic-compiler",
    any(feature = "subscript_formula", feature = "subscript_range")
))]
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
    let instance = (
        CanonicalIndexConversion::new_invocation(invocation.clone())?,
        invocation,
    );
    let specialized = SpecializedFunction::syntax_directed(
        instance,
        ResolvedOperationDescriptor::from_name(
            "access/index",
            PURE_UNARY_INDEX_CONVERSION_CONTRACT.clone(),
        )?,
        RuntimeFunctionId::from_name("access/index"),
        ExecutionTarget::DirectRuntime,
        mech_core::ImplementationMemoryClass::NoAdditionalScratch,
    )?;
    if !execution.plan().activation_registration_active() {
        specialized.instance().solve_result()?;
    }
    execution.plan().register_specialized(specialized)?;
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
    let instance = (
        CanonicalIndexConversion::new_invocation(invocation.clone())?,
        invocation,
    );
    let specialized = SpecializedFunction::syntax_directed(
        instance,
        ResolvedOperationDescriptor::from_name(
            "access/index",
            PURE_UNARY_INDEX_CONVERSION_CONTRACT.clone(),
        )?,
        RuntimeFunctionId::from_name("access/index"),
        ExecutionTarget::DirectRuntime,
        mech_core::ImplementationMemoryClass::NoAdditionalScratch,
    )?;
    if !execution.plan().activation_registration_active() {
        specialized.instance().solve_result()?;
    }
    execution.plan().register_specialized(specialized)?;
    Ok(output)
}
