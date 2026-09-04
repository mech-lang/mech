#[cfg(feature = "matrix")]
use super::matrix::*;
use super::*;
#[cfg(feature = "matrix")]
use mech_core::{
    DimensionExpr, SchemaBody, ValueCell, ValueData, function_shape_contract_violation,
};
use mech_core::{FunctionCatalogBuilder, MResult};

#[cfg(feature = "matrix")]
fn canonical_matrix_dimensions(cell: &ValueCell) -> MResult<Option<(usize, usize)>> {
    let SchemaBody::Matrix { dimensions, .. } = cell.closed_schema_body()? else {
        return Ok(None);
    };
    let [
        DimensionExpr::Constant(rows),
        DimensionExpr::Constant(columns),
    ] = dimensions.as_ref()
    else {
        unreachable!("closed canonical matrix dimensions are constant")
    };
    Ok(Some((*rows as usize, *columns as usize)))
}

#[cfg(feature = "matrix")]
fn matrix_element_count(cell: &ValueCell) -> MResult<Option<usize>> {
    Ok(canonical_matrix_dimensions(cell)?.map(|(rows, columns)| rows.saturating_mul(columns)))
}

#[cfg(feature = "matrix")]
fn assign_selector_cardinality(inputs: &[ValueCell], input_index: usize) -> MResult<usize> {
    let contract = "assign_slice";
    let selector = inputs.get(input_index).ok_or_else(|| {
        function_shape_contract_violation(contract, format!("missing selector input {input_index}"))
    })?;
    Ok(matrix_element_count(selector)?.unwrap_or(1))
}

#[cfg(feature = "matrix")]
fn assign_selector_is_matrix(inputs: &[ValueCell], input_index: usize) -> MResult<bool> {
    let selector = inputs.get(input_index).ok_or_else(|| {
        function_shape_contract_violation(
            "assign_slice",
            format!("missing selector input {input_index}"),
        )
    })?;
    Ok(canonical_matrix_dimensions(selector)?.is_some())
}

#[cfg(feature = "matrix")]
fn assign_logical_selected_offsets(
    inputs: &[ValueCell],
    input_index: usize,
) -> MResult<Vec<usize>> {
    let selector = inputs.get(input_index).ok_or_else(|| {
        function_shape_contract_violation(
            "assign_slice",
            format!("missing logical selector input {input_index}"),
        )
    })?;
    match selector.snapshot()?.data() {
        ValueData::Bool(selected) => Ok(if *selected { vec![0] } else { Vec::new() }),
        ValueData::Matrix(_) => selector
            .matrix_elements()?
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .filter_map(|(offset, value)| match value.snapshot() {
                Ok(snapshot) if matches!(snapshot.data(), ValueData::Bool(true)) => {
                    Some(Ok(offset))
                }
                Ok(_) => None,
                Err(error) => Some(Err(error)),
            })
            .collect(),
        _ => Ok(Vec::new()),
    }
}

#[cfg(feature = "matrix")]
fn assign_index_selected_offsets(inputs: &[ValueCell], input_index: usize) -> MResult<Vec<usize>> {
    let selector = inputs.get(input_index).ok_or_else(|| {
        function_shape_contract_violation(
            "assign_slice",
            format!("missing index selector input {input_index}"),
        )
    })?;
    match selector.snapshot()?.data() {
        ValueData::Index(index) => Ok(vec![(*index as usize).saturating_sub(1)]),
        ValueData::Matrix(_) => selector
            .matrix_elements()?
            .unwrap_or_default()
            .into_iter()
            .filter_map(|value| match value.snapshot() {
                Ok(snapshot) => match snapshot.data() {
                    ValueData::Index(index) => Some(Ok((*index as usize).saturating_sub(1))),
                    _ => None,
                },
                Err(error) => Some(Err(error)),
            })
            .collect(),
        _ => Ok(Vec::new()),
    }
}

#[cfg(feature = "matrix")]
fn selected_extent(offsets: &[usize]) -> usize {
    offsets
        .iter()
        .copied()
        .max()
        .map_or(0, |offset| offset.saturating_add(1))
}

#[cfg(feature = "matrix")]
fn validate_assign_source_matrix_layout(
    inputs: &[ValueCell],
    required_rows: usize,
    required_columns: usize,
    broadcast_rows: bool,
    broadcast_columns: bool,
) -> MResult<()> {
    let source = inputs.first().ok_or_else(|| {
        function_shape_contract_violation("assign_slice", "missing assignment source input 0")
    })?;
    let Some((rows, columns)) = canonical_matrix_dimensions(source)? else {
        return Ok(());
    };
    let rows_valid = rows >= required_rows || (broadcast_rows && rows == 1);
    let columns_valid = columns >= required_columns || (broadcast_columns && columns == 1);
    if !rows_valid || !columns_valid {
        return Err(function_shape_contract_violation(
            "assign_slice",
            format!(
                "matrix assignment source layout is {rows}x{columns}, selected layout requires at least {required_rows}x{required_columns}{}{}",
                if broadcast_rows {
                    " or one broadcast row"
                } else {
                    ""
                },
                if broadcast_columns {
                    " or one broadcast column"
                } else {
                    ""
                },
            ),
        ));
    }
    Ok(())
}

#[cfg(feature = "matrix")]
fn validate_assign_source_cardinality(inputs: &[ValueCell], required: usize) -> MResult<()> {
    let contract = "assign_slice";
    let source = inputs.first().ok_or_else(|| {
        function_shape_contract_violation(contract, "missing assignment source input 0")
    })?;
    let Some(actual) = matrix_element_count(source)? else {
        // Scalar sources broadcast across the selected cells.
        return Ok(());
    };
    if actual < required {
        return Err(function_shape_contract_violation(
            contract,
            format!(
                "matrix assignment source has {actual} elements, selected layout requires at least {required}"
            ),
        ));
    }
    Ok(())
}

#[cfg(feature = "matrix")]
#[derive(Clone, Copy)]
enum AssignIndexAxis {
    Linear,
    Row,
    Column,
}

#[cfg(feature = "matrix")]
fn validate_assign_index(
    output: &ValueCell,
    inputs: &[ValueCell],
    input_index: usize,
    axis: AssignIndexAxis,
) -> MResult<()> {
    let contract = "assign_slice";
    let (rows, columns) = canonical_matrix_dimensions(output)?.ok_or_else(|| {
        function_shape_contract_violation(contract, "output must be matrix-backed")
    })?;
    let bound = match axis {
        AssignIndexAxis::Linear => rows.saturating_mul(columns),
        AssignIndexAxis::Row => rows,
        AssignIndexAxis::Column => columns,
    };
    let axis_name = match axis {
        AssignIndexAxis::Linear => "linear",
        AssignIndexAxis::Row => "row",
        AssignIndexAxis::Column => "column",
    };
    let input = inputs.get(input_index).ok_or_else(|| {
        function_shape_contract_violation(
            contract,
            format!("missing {axis_name} index input {input_index}"),
        )
    })?;
    let invalid = |index: usize| {
        function_shape_contract_violation(
            contract,
            format!("input {input_index} contains {axis_name} index {index}, expected 1..={bound}",),
        )
    };
    match input.snapshot()?.data() {
        ValueData::Index(index) => {
            let index = *index as usize;
            if index == 0 || index > bound {
                return Err(invalid(index));
            }
        }
        ValueData::Matrix(_) => {
            for value in input.matrix_elements()?.unwrap_or_default() {
                let snapshot = value.snapshot()?;
                let ValueData::Index(index) = snapshot.data() else {
                    continue;
                };
                let index = *index as usize;
                if index == 0 || index > bound {
                    return Err(invalid(index));
                }
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(feature = "matrix")]
fn validate_assign_logical_mask(
    output: &ValueCell,
    inputs: &[ValueCell],
    input_index: usize,
    axis: AssignIndexAxis,
) -> MResult<()> {
    let contract = "assign_slice";
    let (rows, columns) = canonical_matrix_dimensions(output)?.ok_or_else(|| {
        function_shape_contract_violation(contract, "output must be matrix-backed")
    })?;
    let (bound, axis_name) = match axis {
        AssignIndexAxis::Linear => (rows.saturating_mul(columns), "linear"),
        AssignIndexAxis::Row => (rows, "row"),
        AssignIndexAxis::Column => (columns, "column"),
    };
    let input = inputs.get(input_index).ok_or_else(|| {
        function_shape_contract_violation(
            contract,
            format!("missing {axis_name} logical-mask input {input_index}"),
        )
    })?;
    match input.closed_schema_body()? {
        SchemaBody::Bool => Ok(()),
        SchemaBody::Matrix {
            element,
            dimensions,
        } if *element == SchemaBody::Bool => {
            let actual = dimensions.iter().try_fold(1_usize, |len, dimension| {
                let DimensionExpr::Constant(dimension) = dimension else {
                    unreachable!("closed canonical matrix dimensions are constant")
                };
                Ok::<_, mech_core::MechError>(len.saturating_mul(*dimension as usize))
            })?;
            if actual != bound {
                return Err(function_shape_contract_violation(
                    contract,
                    format!(
                        "input {input_index} {axis_name} logical mask has {actual} elements, expected {bound}"
                    ),
                ));
            }
            Ok(())
        }
        found => Err(function_shape_contract_violation(
            contract,
            format!(
                "input {input_index} must be a bool or bool matrix {axis_name} logical mask, found {found:?}"
            ),
        )),
    }
}

#[cfg(feature = "matrix")]
fn validate_assign_whole(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
    let (rows, columns) = canonical_matrix_dimensions(output)?.ok_or_else(|| {
        function_shape_contract_violation("assign_slice", "output must be matrix-backed")
    })?;
    validate_assign_source_cardinality(inputs, rows.saturating_mul(columns))
}

#[cfg(feature = "matrix")]
fn validate_assign_linear(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
    validate_assign_index(output, inputs, 1, AssignIndexAxis::Linear)?;
    validate_assign_source_cardinality(inputs, assign_selector_cardinality(inputs, 1)?)
}

#[cfg(feature = "matrix")]
fn validate_assign_logical_linear(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
    validate_assign_logical_mask(output, inputs, 1, AssignIndexAxis::Linear)?;
    let required = if assign_selector_is_matrix(inputs, 1)? {
        selected_extent(&assign_logical_selected_offsets(inputs, 1)?)
    } else {
        // The scalar-bool vector kernel copies only the shared source/sink
        // prefix, so it cannot index beyond a shorter source.
        0
    };
    validate_assign_source_cardinality(inputs, required)
}

#[cfg(feature = "matrix")]
fn validate_assign_row_column(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
    validate_assign_index(output, inputs, 1, AssignIndexAxis::Row)?;
    validate_assign_index(output, inputs, 2, AssignIndexAxis::Column)?;
    let required = assign_selector_cardinality(inputs, 1)?
        .saturating_mul(assign_selector_cardinality(inputs, 2)?);
    validate_assign_source_cardinality(inputs, required)
}

#[cfg(feature = "matrix")]
fn validate_assign_row(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
    validate_assign_index(output, inputs, 1, AssignIndexAxis::Row)?;
    let (_, columns) = canonical_matrix_dimensions(output)?.ok_or_else(|| {
        function_shape_contract_violation("assign_slice", "output must be matrix-backed")
    })?;
    if assign_selector_is_matrix(inputs, 1)? {
        validate_assign_source_matrix_layout(
            inputs,
            assign_selector_cardinality(inputs, 1)?,
            columns,
            true,
            false,
        )
    } else {
        validate_assign_source_cardinality(inputs, columns)
    }
}

#[cfg(feature = "matrix")]
fn validate_assign_column(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
    validate_assign_index(output, inputs, 1, AssignIndexAxis::Column)?;
    let (rows, _) = canonical_matrix_dimensions(output)?.ok_or_else(|| {
        function_shape_contract_violation("assign_slice", "output must be matrix-backed")
    })?;
    if assign_selector_is_matrix(inputs, 1)? {
        validate_assign_source_matrix_layout(
            inputs,
            rows,
            assign_selector_cardinality(inputs, 1)?,
            false,
            true,
        )
    } else {
        validate_assign_source_cardinality(inputs, rows)
    }
}

#[cfg(feature = "matrix")]
fn validate_assign_logical_row(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
    validate_assign_logical_mask(output, inputs, 1, AssignIndexAxis::Row)?;
    let (_, columns) = canonical_matrix_dimensions(output)?.ok_or_else(|| {
        function_shape_contract_violation("assign_slice", "output must be matrix-backed")
    })?;
    validate_assign_source_matrix_layout(
        inputs,
        selected_extent(&assign_logical_selected_offsets(inputs, 1)?),
        columns,
        false,
        false,
    )
}

#[cfg(feature = "matrix")]
fn validate_assign_logical_column(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
    validate_assign_logical_mask(output, inputs, 1, AssignIndexAxis::Column)?;
    let (rows, _) = canonical_matrix_dimensions(output)?.ok_or_else(|| {
        function_shape_contract_violation("assign_slice", "output must be matrix-backed")
    })?;
    validate_assign_source_matrix_layout(
        inputs,
        rows,
        assign_logical_selected_offsets(inputs, 1)?.len(),
        false,
        true,
    )
}

#[cfg(feature = "matrix")]
fn validate_assign_logical_row_column(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
    validate_assign_logical_mask(output, inputs, 1, AssignIndexAxis::Row)?;
    validate_assign_index(output, inputs, 2, AssignIndexAxis::Column)?;
    let (rows, _) = canonical_matrix_dimensions(output)?.ok_or_else(|| {
        function_shape_contract_violation("assign_slice", "output must be matrix-backed")
    })?;
    let selected_rows = assign_logical_selected_offsets(inputs, 1)?;
    let required = if assign_selector_is_matrix(inputs, 2)? {
        let selected_columns = assign_index_selected_offsets(inputs, 2)?;
        selected_rows
            .iter()
            .flat_map(|row| {
                selected_columns
                    .iter()
                    .map(move |column| column.saturating_mul(rows).saturating_add(*row))
            })
            .max()
            .map_or(0, |offset| offset.saturating_add(1))
    } else {
        selected_extent(&selected_rows)
    };
    validate_assign_source_cardinality(inputs, required)
}

#[cfg(feature = "matrix")]
fn validate_assign_row_logical_column(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
    validate_assign_index(output, inputs, 1, AssignIndexAxis::Row)?;
    validate_assign_logical_mask(output, inputs, 2, AssignIndexAxis::Column)?;
    let (rows, _) = canonical_matrix_dimensions(output)?.ok_or_else(|| {
        function_shape_contract_violation("assign_slice", "output must be matrix-backed")
    })?;
    let selected_columns = assign_logical_selected_offsets(inputs, 2)?;
    let required = if assign_selector_is_matrix(inputs, 1)? {
        let selected_rows = assign_index_selected_offsets(inputs, 1)?;
        selected_rows
            .iter()
            .flat_map(|row| {
                selected_columns
                    .iter()
                    .map(move |column| column.saturating_mul(rows).saturating_add(*row))
            })
            .max()
            .map_or(0, |offset| offset.saturating_add(1))
    } else {
        selected_extent(&selected_columns)
    };
    validate_assign_source_cardinality(inputs, required)
}

#[cfg(feature = "matrix")]
fn validate_assign_logical_row_logical_column(
    output: &ValueCell,
    inputs: &[ValueCell],
) -> MResult<()> {
    validate_assign_logical_mask(output, inputs, 1, AssignIndexAxis::Row)?;
    validate_assign_logical_mask(output, inputs, 2, AssignIndexAxis::Column)?;
    let (_, columns) = canonical_matrix_dimensions(output)?.ok_or_else(|| {
        function_shape_contract_violation("assign_slice", "output must be matrix-backed")
    })?;
    let selected_rows = assign_logical_selected_offsets(inputs, 1)?;
    let selected_columns = assign_logical_selected_offsets(inputs, 2)?;
    let required = selected_rows
        .iter()
        .flat_map(|row| {
            selected_columns
                .iter()
                .map(move |column| row.saturating_mul(columns).saturating_add(*column))
        })
        .max()
        .map_or(0, |offset| offset.saturating_add(1));
    validate_assign_source_cardinality(inputs, required)
}

#[cfg(feature = "matrix")]
macro_rules! assign_layout_validator {
    (Whole) => {
        validate_assign_whole
    };
    (Linear) => {
        validate_assign_linear
    };
    (LogicalLinear) => {
        validate_assign_logical_linear
    };
    (Row) => {
        validate_assign_row
    };
    (LogicalRow) => {
        validate_assign_logical_row
    };
    (Column) => {
        validate_assign_column
    };
    (LogicalColumn) => {
        validate_assign_logical_column
    };
    (RowColumn) => {
        validate_assign_row_column
    };
    (LogicalRowColumn) => {
        validate_assign_logical_row_column
    };
    (RowLogicalColumn) => {
        validate_assign_row_logical_column
    };
    (LogicalRowLogicalColumn) => {
        validate_assign_logical_row_logical_column
    };
}

// Assignment's scalar factories deliberately keep the Rust type, emitted
// runtime spelling, Cargo value feature, and installer token independent.
// `r64` and `c64` are the two cases where those spellings differ.
macro_rules! for_each_assign_scalar_factory {
    ($callback:ident, $context:tt) => {
        $callback!($context; u8; u8; "u8"; "u8");
        $callback!($context; u16; u16; "u16"; "u16");
        $callback!($context; u32; u32; "u32"; "u32");
        $callback!($context; u64; u64; "u64"; "u64");
        $callback!($context; u128; u128; "u128"; "u128");
        $callback!($context; i8; i8; "i8"; "i8");
        $callback!($context; i16; i16; "i16"; "i16");
        $callback!($context; i32; i32; "i32"; "i32");
        $callback!($context; i64; i64; "i64"; "i64");
        $callback!($context; i128; i128; "i128"; "i128");
        $callback!($context; f32; f32; "f32"; "f32");
        $callback!($context; f64; f64; "f64"; "f64");
        $callback!($context; bool; bool; "bool"; "bool");
        $callback!($context; string; String; "string"; "string");
        $callback!($context; r64; R64; "r64"; "rational");
        $callback!($context; c64; C64; "c64"; "complex");
    };
}

macro_rules! declare_assign_scalar_factory {
    ($_context:tt; $installer_token:ident; $scalar:ty; $runtime_name:literal; $cargo_feature:literal) => {
        mech_core::paste::paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all(feature = "assign", feature = $cargo_feature),
                registration: [<register_assign_ $installer_token>],
                installer: [<install_assign_ $installer_token>],
                name: concat!("Assign<", $runtime_name, ">"),
                factory_type: Assign<$scalar>,
                contract: RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::AllowInputAlias),
                compiler_family: mech_core::RuntimeFamilyId::from_name(concat!("Assign<", $runtime_name, ">")),
                package: "mech-engine", crate_name: "mech_engine",
                installer_path: concat!("mech_engine::__mech_native::", stringify!([<install_assign_ $installer_token>])),
                extra_cargo_features: ["assign"],
            }
        }
    };
}

for_each_assign_scalar_factory!(declare_assign_scalar_factory, ());

mech_core::declare_native_runtime_factory! {
    cfg: feature = "assign",
    registration: register_assign_index,
    installer: install_assign_index,
    name: "Assign<index>",
    factory_type: Assign<usize>,
    contract: RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::AllowInputAlias),
    compiler_family: mech_core::RuntimeFamilyId::from_name("Assign<index>"),
    package: "mech-engine", crate_name: "mech_engine",
    installer_path: "mech_engine::__mech_native::install_assign_index",
    extra_cargo_features: ["assign"],
}

macro_rules! for_each_assign_value_matrix_shape {
    ($callback:ident, $context:tt) => {
        #[cfg(feature = "matrix1")]
        $callback!($context, Matrix1, "matrix1");
        #[cfg(feature = "matrix2")]
        $callback!($context, Matrix2, "matrix2");
        #[cfg(feature = "matrix2x3")]
        $callback!($context, Matrix2x3, "matrix2x3");
        #[cfg(feature = "matrix3x2")]
        $callback!($context, Matrix3x2, "matrix3x2");
        #[cfg(feature = "matrix3")]
        $callback!($context, Matrix3, "matrix3");
        #[cfg(feature = "matrix4")]
        $callback!($context, Matrix4, "matrix4");
        #[cfg(feature = "matrixd")]
        $callback!($context, DMatrix, "matrixd");
        #[cfg(feature = "vector2")]
        $callback!($context, Vector2, "vector2");
        #[cfg(feature = "vector3")]
        $callback!($context, Vector3, "vector3");
        #[cfg(feature = "vector4")]
        $callback!($context, Vector4, "vector4");
        #[cfg(feature = "vectord")]
        $callback!($context, DVector, "vectord");
        #[cfg(feature = "row_vector2")]
        $callback!($context, RowVector2, "row_vector2");
        #[cfg(feature = "row_vector3")]
        $callback!($context, RowVector3, "row_vector3");
        #[cfg(feature = "row_vector4")]
        $callback!($context, RowVector4, "row_vector4");
        #[cfg(feature = "row_vectord")]
        $callback!($context, RowDVector, "row_vectord");
    };
}

#[cfg(feature = "matrix")]
macro_rules! declare_assign_value_matrix_shape {
    (
        ($scalar:ty; $scalar_token:ident; $runtime_name:literal; $scalar_feature:literal),
        $shape:ident,
        $shape_feature:literal
    ) => {
        mech_core::paste::paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all(
                    feature = "assign",
                    feature = $scalar_feature,
                    feature = $shape_feature
                ),
                registration: [<register_assign_value_ $scalar_token:lower _ $shape:lower>],
                installer: [<install_assign_value_ $scalar_token:lower _ $shape:lower>],
                name: concat!("Assign<", $runtime_name, stringify!($shape), ">"),
                factory_type: Assign<$shape<$scalar>>,
                contract: RuntimeFunctionContract::same_shape(
                    RuntimeOutputAliasPolicy::AllowInputAlias,
                ),
                compiler_family: mech_core::RuntimeFamilyId::from_name(concat!("Assign<", $runtime_name, stringify!($shape), ">")),
                package: "mech-engine", crate_name: "mech_engine",
                installer_path: concat!(
                    "mech_engine::__mech_native::",
                    stringify!([<install_assign_value_ $scalar_token:lower _ $shape:lower>]),
                ),
                extra_cargo_features: ["assign"],
            }
        }
    };
}

macro_rules! declare_assign_value_matrix_for_scalar {
    ($scalar:ty, $scalar_token:ident, $runtime_name:literal, $scalar_feature:literal) => {
        for_each_assign_value_matrix_shape!(
            declare_assign_value_matrix_shape,
            ($scalar; $scalar_token; $runtime_name; $scalar_feature)
        );
    };
}

declare_assign_value_matrix_for_scalar!(u8, u8, "u8", "u8");
declare_assign_value_matrix_for_scalar!(u16, u16, "u16", "u16");
declare_assign_value_matrix_for_scalar!(u32, u32, "u32", "u32");
declare_assign_value_matrix_for_scalar!(u64, u64, "u64", "u64");
declare_assign_value_matrix_for_scalar!(u128, u128, "u128", "u128");
declare_assign_value_matrix_for_scalar!(i8, i8, "i8", "i8");
declare_assign_value_matrix_for_scalar!(i16, i16, "i16", "i16");
declare_assign_value_matrix_for_scalar!(i32, i32, "i32", "i32");
declare_assign_value_matrix_for_scalar!(i64, i64, "i64", "i64");
declare_assign_value_matrix_for_scalar!(i128, i128, "i128", "i128");
declare_assign_value_matrix_for_scalar!(f32, f32, "f32", "f32");
declare_assign_value_matrix_for_scalar!(f64, f64, "f64", "f64");
declare_assign_value_matrix_for_scalar!(bool, bool, "bool", "bool");
declare_assign_value_matrix_for_scalar!(String, string, "string", "string");
declare_assign_value_matrix_for_scalar!(R64, r64, "rational", "r64");
declare_assign_value_matrix_for_scalar!(C64, c64, "complex", "c64");

#[cfg(all(
    any(feature = "native-plan", feature = "semantic-compiler"),
    feature = "matrix"
))]
macro_rules! register_assign_value_matrix_shape {
    (($builder:ident; $scalar_token:ident), $shape:ident, $_shape_feature:literal) => {
        mech_core::paste::paste! {
            [<register_assign_value_ $scalar_token:lower _ $shape:lower>]($builder)?;
        }
    };
}

#[cfg(all(
    any(feature = "native-plan", feature = "semantic-compiler"),
    feature = "matrix"
))]
macro_rules! register_assign_value_matrix_for_scalar {
    ($builder:ident, $scalar_token:ident, $scalar_feature:literal) => {
        #[cfg(feature = $scalar_feature)]
        for_each_assign_value_matrix_shape!(
            register_assign_value_matrix_shape,
            ($builder; $scalar_token)
        );
    };
}

#[cfg(all(feature = "native-link", feature = "matrix"))]
macro_rules! export_assign_value_matrix_shape {
    (($scalar_token:ident), $shape:ident, $_shape_feature:literal) => {
        mech_core::paste::paste! {
            pub use super::[<install_assign_value_ $scalar_token:lower _ $shape:lower>];
        }
    };
}

#[cfg(feature = "native-link")]
macro_rules! export_assign_value_matrix_for_scalar {
    ($scalar_token:ident, $scalar_feature:literal) => {
        #[cfg(feature = $scalar_feature)]
        for_each_assign_value_matrix_shape!(export_assign_value_matrix_shape, ($scalar_token));
    };
}

macro_rules! register_assign_scalar_factory {
    (($builder:ident); $installer_token:ident; $_scalar:ty; $_runtime_name:literal; $cargo_feature:literal) => {
        #[cfg(all(feature = "assign", feature = $cargo_feature))]
        mech_core::paste::paste! { [<register_assign_ $installer_token>]($builder)?; }
    };
}

#[cfg(feature = "native-link")]
macro_rules! export_assign_scalar_factory {
    ($_context:tt; $installer_token:ident; $_scalar:ty; $_runtime_name:literal; $cargo_feature:literal) => {
        #[cfg(all(feature = "assign", feature = $cargo_feature))]
        mech_core::paste::paste! { pub use super::[<install_assign_ $installer_token>]; }
    };
}

// All three consumers below are fed by the same concrete-factory traversal:
// declarations for native plans, direct runtime registrations, and hidden
// generated-application exports.  This deliberately replaces the historic
// aggregate assignment installer, whose single path could not describe an
// individual factory exactly.
#[cfg(feature = "matrix")]
macro_rules! declare_matrix_assign_factory {
    (
        ($_context:tt, $output_alias:expr, $layout:ident);
        $fxn_name:ident, $scalar:ident, $scalar_name:literal, $scalar_feature:literal,
        [$($shape:ident),+], [$($extra_feature:literal),*], $factory:ty
    ) => {
        mech_core::paste::paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all(feature = "assign", feature = "matrix", feature = $scalar_feature),
                registration: [<register_assign_ $fxn_name:lower _ $scalar:lower $( _ $shape:lower )*>],
                installer: [<install_assign_ $fxn_name:lower _ $scalar:lower $( _ $shape:lower )*>],
                name: concat!(stringify!($fxn_name), "<", $scalar_name, $(stringify!($shape)),*, ">"),
                factory_type: $factory,
                contract: RuntimeFunctionContract::canonical_custom(
                    "assign_slice",
                    $output_alias,
                    assign_layout_validator!($layout),
                ),
                compiler_family: mech_core::RuntimeFamilyId::from_name(concat!(stringify!($fxn_name), "<", $scalar_name, $(stringify!($shape)),*, ">")),
                package: "mech-engine", crate_name: "mech_engine",
                installer_path: concat!(
                    "mech_engine::__mech_native::",
                    stringify!([<install_assign_ $fxn_name:lower _ $scalar:lower $( _ $shape:lower )*>]),
                ),
                extra_cargo_features: ["assign"],
            }
        }
    };
}

#[cfg(feature = "native-link")]
#[cfg(feature = "matrix")]
macro_rules! export_matrix_assign_factory {
    (
        ($_context:tt, $_output_alias:expr, $_layout:ident);
        $fxn_name:ident, $scalar:ident, $scalar_name:literal, $scalar_feature:literal,
        [$($shape:ident),+], [$($extra_feature:literal),*], $factory:expr
    ) => {
        mech_core::paste::paste! {
            pub use super::[<install_assign_ $fxn_name:lower _ $scalar:lower $( _ $shape:lower )*>];
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! register_matrix_assign_factory {
    (
        ($builder:ident, $_output_alias:expr, $_layout:ident);
        $fxn_name:ident, $scalar:ident, $scalar_name:literal, $scalar_feature:literal,
        [$($shape:ident),+], [$($extra_feature:literal),*], $factory:expr
    ) => {
        mech_core::paste::paste! {
            [<register_assign_ $fxn_name:lower _ $scalar:lower $( _ $shape:lower )*>]($builder)?;
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_assign {
    ($emit:ident, $context:tt, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt) => {
        mech_core::paste::paste! {
            $emit!($context; $fxn_name, $scalar, $scalar_string, $scalar_string, [$row1, $row2, $row3], [], $fxn_name::<$scalar,$row1<$scalar>,$row2<$scalar>,$row3<usize>>);
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_assign_s {
    ($emit:ident, $context:tt, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt) => {
        mech_core::paste::paste! {
            $emit!($context; $fxn_name, $scalar, $scalar_string, $scalar_string, [$row1, $row2], [], $fxn_name::<$scalar,$row1<$scalar>,$row2<usize>>);
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_assign_srr {
    ($emit:ident, $context:tt, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt) => {
        mech_core::paste::paste! {
            $emit!($context; $fxn_name, $scalar, $scalar_string, $scalar_string, [$row1, $row2, $row3], [], $fxn_name::<$scalar,$row1<$scalar>,$row2<usize>,$row3<usize>>);
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_assign_srr_b {
    ($emit:ident, $context:tt, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt) => {
        mech_core::paste::paste! {
            $emit!($context; $fxn_name, $scalar, $scalar_string, $scalar_string, [$row1, $row2, $row3], ["bool"], $fxn_name::<$scalar,$row1<$scalar>,$row2<bool>,$row3<bool>>);
        }
    };
}

#[cfg(all(feature = "matrix", feature = "f64"))]
macro_rules! install_legacy_assign_srr_bu {
    ($emit:ident, $context:tt, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt) => {
        mech_core::paste::paste! {
            $emit!($context; $fxn_name, $scalar, $scalar_string, $scalar_string, [$row1, $row2, $row3], ["bool"], $fxn_name::<$scalar,$row1<$scalar>,$row2<bool>,$row3<usize>>);
        }
    };
}

#[cfg(all(feature = "matrix", feature = "f64"))]
macro_rules! install_legacy_assign_srr_ub {
    ($emit:ident, $context:tt, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt) => {
        mech_core::paste::paste! {
            $emit!($context; $fxn_name, $scalar, $scalar_string, $scalar_string, [$row1, $row2, $row3], ["bool"], $fxn_name::<$scalar,$row1<$scalar>,$row2<usize>,$row3<bool>>);
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_assign_srr_b2 {
    ($emit:ident, $context:tt, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt, $row4:tt) => {
        mech_core::paste::paste! {
            $emit!($context; $fxn_name, $scalar, $scalar_string, $scalar_string, [$row1, $row2, $row3, $row4], ["bool"], $fxn_name::<$scalar,$row1<$scalar>,$row2<$scalar>,$row3<bool>,$row4<bool>>);
        }
    };
}

#[cfg(all(feature = "matrix", feature = "f64"))]
macro_rules! install_legacy_assign_srr_bu2 {
    ($emit:ident, $context:tt, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt, $row4:tt) => {
        mech_core::paste::paste! {
            $emit!($context; $fxn_name, $scalar, $scalar_string, $scalar_string, [$row1, $row2, $row3, $row4], ["bool"], $fxn_name::<$scalar,$row1<$scalar>,$row2<$scalar>,$row3<bool>,$row4<usize>>);
        }
    };
}

#[cfg(all(feature = "matrix", feature = "f64"))]
macro_rules! install_legacy_assign_srr_ub2 {
    ($emit:ident, $context:tt, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt, $row4:tt) => {
        mech_core::paste::paste! {
            $emit!($context; $fxn_name, $scalar, $scalar_string, $scalar_string, [$row1, $row2, $row3, $row4], ["bool"], $fxn_name::<$scalar,$row1<$scalar>,$row2<$scalar>,$row3<usize>,$row4<bool>>);
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_assign_srr2 {
    ($emit:ident, $context:tt, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt, $row4:tt) => {
        mech_core::paste::paste! {
            $emit!($context; $fxn_name, $scalar, $scalar_string, $scalar_string, [$row1, $row2, $row3, $row4], [], $fxn_name::<$scalar,$row1<$scalar>,$row2<$scalar>,$row3<usize>,$row4<usize>>);
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_assign_s1 {
    ($emit:ident, $context:tt, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt) => {
        mech_core::paste::paste! {
            $emit!($context; $fxn_name, $scalar, $scalar_string, $scalar_string, [$row1], [], $fxn_name::<$scalar,$row1<$scalar>>);
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_assign_s2 {
    ($emit:ident, $context:tt, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt) => {
        mech_core::paste::paste! {
            $emit!($context; $fxn_name, $scalar, $scalar_string, $scalar_string, [$row1, $row2], [], $fxn_name::<$scalar,$row1<$scalar>,$row2<$scalar>>);
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_assign_b {
    ($emit:ident, $context:tt, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt) => {
        mech_core::paste::paste! {
            $emit!($context; $fxn_name, $scalar, $scalar_string, $scalar_string, [$row1, $row2, $row3], ["bool"], $fxn_name::<$scalar,$row1<$scalar>,$row2<$scalar>,$row3<bool>>);
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_assign_s_b {
    ($emit:ident, $context:tt, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt) => {
        mech_core::paste::paste! {
            $emit!($context; $fxn_name, $scalar, $scalar_string, $scalar_string, [$row1, $row2], ["bool"], $fxn_name::<$scalar,$row1<$scalar>,$row2<bool>>);
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_impl_assign_scalar_arms {
    ($emit:ident, $context:tt, $fxn_name:ident, $shape:tt, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(all(feature = $value_string))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape);
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_impl_assign_all_arms {
    ($emit:ident, $context:tt, $fxn_name:ident, $shape:tt, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(feature = $value_string)]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape);
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_impl_assign_scalar_scalar_arms {
    ($emit:ident, $context:tt, $fxn_name:ident, $shape:tt, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(feature = $value_string)]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape);
            #[cfg(all(feature = $value_string, feature = "matrixd", not(feature = "matrix1")))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name MD>], $value_kind, $value_string, $shape, DMatrix);
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_impl_set_range_arms {
    ($emit:ident, $context:tt, $fxn_name:ident, $shape:tt, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix4, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2x3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3x2, DVector);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector4, Vector4);
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_impl_set_range_all_arms {
    ($emit:ident, $context:tt, $fxn_name:ident, $shape:tt, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix4, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2x3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3x2, DVector);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector4, Vector4);
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_impl_assign_range_scalar_arms {
    ($emit:ident, $context:tt, $fxn_name:ident, $shape:tt, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, DMatrix);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix4, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2x3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3x2, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "vectord"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector);
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_impl_assign_scalar_range_arms {
    ($emit:ident, $context:tt, $fxn_name:ident, $shape:tt, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrixd", not(feature = "matrix1")))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, DMatrix);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix4, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2x3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3x2, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "vectord"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector);
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_impl_assign_range_range_arms {
    ($emit:ident, $context:tt, $fxn_name:ident, $shape:tt, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vector2"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vector3"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vector4"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vectord"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix1"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector2, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector3"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector2, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector4"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector2, Vector4);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vectord"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector2, DVector);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix1"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector3, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector2"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector3, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector4"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector3, Vector4);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vectord"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector3, DVector);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix1"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector4, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector2"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector4, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector3"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector4, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vectord"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector4, DVector);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrix1"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, DVector, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector2"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, DVector, Vector2);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector3"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, DVector, Vector3);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector4"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, DVector, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix1, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vector2"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector2, Matrix1, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "row_vector2"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector2, Matrix1, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vector3"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector3, Matrix1, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "row_vector3"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector3, Matrix1, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vector4"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector4, Matrix1, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vector2", feature = "matrix2"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2, Matrix1, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "row_vector4"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector4, Matrix1, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Matrix1, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "row_vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Matrix1, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "matrixd"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, Matrix1, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix1", feature = "vector2"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector2, Vector2, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix1", feature = "row_vector2"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector2, Vector2, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector4"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector4, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "row_vector4"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector4, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector3", feature = "matrix2x3"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2x3, Vector2, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector3", feature = "matrix3x2"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3x2, Vector2, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector3", feature = "vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector2, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector3", feature = "row_vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector2, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector4", feature = "vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector2, Vector4);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector4", feature = "row_vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector2, Vector4);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector4", feature = "matrixd"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, Vector2, Vector4);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vectord", feature = "vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector2, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vectord", feature = "row_vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector2, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vectord", feature = "matrixd"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, Vector2, DVector);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix1", feature = "vector3"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector3, Vector3, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix1", feature = "row_vector3"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector3, Vector3, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector2", feature = "matrix2x3"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2x3, Vector3, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector2", feature = "matrix3x2"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3x2, Vector3, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector2", feature = "vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector3, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector2", feature = "row_vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector3, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix3"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vectord", feature = "row_vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector3, DVector);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector4", feature = "vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector3, Vector4);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector4", feature = "row_vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector3, Vector4);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector4", feature = "matrixd"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, Vector3, Vector4);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vectord", feature = "vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector3, DVector);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vectord", feature = "matrixd"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, Vector3, DVector);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix1", feature = "vector4"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector4, Vector4, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix1", feature = "row_vector4"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector4, Vector4, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix1", feature = "matrix2"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2, Vector4, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector2", feature = "vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector4, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector2", feature = "row_vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector4, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector2", feature = "matrixd"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, Vector4, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector3", feature = "vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector4, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector3", feature = "row_vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector4, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector3", feature = "matrixd"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, Vector4, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "row_vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrixd"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, Vector4, DVector);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector4, DVector);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "row_vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector4, DVector);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix4"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix4, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrix1", feature = "vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrix1", feature = "row_vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrix1", feature = "matrixd"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector2", feature = "vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector, Vector2);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector2", feature = "row_vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector, Vector2);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector2", feature = "matrixd"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector, Vector2);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector3", feature = "vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector, Vector3);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector3", feature = "row_vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector, Vector3);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector3", feature = "matrixd"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector, Vector3);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector4", feature = "vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector4", feature = "row_vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector4", feature = "matrixd"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrixd"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "row_vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector, DVector);
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_impl_assign_all_range_arms {
    ($emit:ident, $context:tt, $fxn_name:ident, $shape:tt, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix4, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2x3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3x2, DVector);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector4, Vector4);
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_impl_assign_all_scalar_arms {
    ($emit:ident, $context:tt, $fxn_name:ident, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, Matrix2);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, Matrix3);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, Matrix4);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, Matrix2x3);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, Matrix3x2);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, DMatrix);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, RowDVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, RowVector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, RowVector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, RowVector4);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix2"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix2, RowVector2);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix2"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix3"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix3, RowVector3);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix3"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "matrix4"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix4, RowVector4);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix4"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix2x3"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix2x3, RowVector2);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix2x3"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix2x3, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix3x2"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix3x2, RowVector3);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix3x2"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix3x2, Vector3);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrixd"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "matrixd"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, DMatrix, RowDVector);
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_impl_assign_scalar_all_arms {
    ($emit:ident, $context:tt, $fxn_name:ident, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, Matrix2);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, Matrix3);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, Matrix4);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, Matrix2x3);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, Matrix3x2);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, DMatrix);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, RowDVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, RowVector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, RowVector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, RowVector4);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix2"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix2, RowVector2);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix2"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix3"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix3, RowVector3);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix3"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "matrix4"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix4, RowVector4);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix4"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix2x3"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix2x3, RowVector3);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix2x3"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix2x3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix3x2"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix3x2, RowVector2);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix3x2"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix3x2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrixd"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "matrixd"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, DMatrix, RowDVector);
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_impl_set_all_range_arms_b {
    ($emit:ident, $context:tt, $fxn_name:ident, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Matrix2, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Matrix3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Matrix4, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Matrix2x3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Matrix3x2, DVector);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, RowVector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, RowVector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, RowVector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, RowDVector, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Matrix1, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector2"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Matrix2, Matrix2, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Matrix3, Matrix3, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Matrix4, Matrix4, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Matrix2x3, Matrix2x3, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Matrix3x2, Matrix3x2, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, DMatrix, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, DVector, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, RowDVector, RowDVector, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Vector2, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Vector3, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Vector4, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, RowVector2, RowVector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, RowVector3, RowVector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, RowVector4, RowVector4, Vector4);
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_impl_set_range_all_arms_b {
    ($emit:ident, $context:tt, $fxn_name:ident, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector2"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Matrix2, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Matrix3, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Matrix4, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Matrix2x3, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Matrix3x2, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, RowVector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, RowVector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, RowVector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, RowDVector, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Matrix1, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector2"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Matrix2, Matrix2, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Matrix3, Matrix3, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Matrix4, Matrix4, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Matrix2x3, Matrix2x3, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Matrix3x2, Matrix3x2, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, DMatrix, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, DVector, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, RowDVector, RowDVector, DVector);
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_impl_set_range_arms_b {
    ($emit:ident, $context:tt, $fxn_name:ident, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Matrix2, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Matrix3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Matrix4, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Matrix2x3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Matrix3x2, DVector);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, RowVector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, RowVector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, RowVector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, RowDVector, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Matrix1, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Matrix2, Matrix2, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Matrix3, Matrix3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Matrix4, Matrix4, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Matrix2x3, Matrix2x3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Matrix3x2, Matrix3x2, DVector);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, DMatrix, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, DVector, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, RowDVector, RowDVector, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Vector2, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Vector3, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Vector4, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, RowVector2, RowVector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, RowVector3, RowVector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, RowVector4, RowVector4, Vector4);
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_impl_assign_scalar_arms_b {
    ($emit:ident, $context:tt, $fxn_name:ident, $shape:tt, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(feature = $value_string)]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, $shape);
            #[cfg(feature = $value_string)]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, $shape);
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_impl_assign_range_scalar_arms_b {
    ($emit:ident, $context:tt, $fxn_name:ident, $shape:tt, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, $shape, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, $shape, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, $shape, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, $shape, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, $shape, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix2, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix4, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix2x3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix3x2, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, RowVector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, RowVector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, RowVector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "vectord"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, RowDVector, DVector);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, DMatrix, DVector);
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_impl_assign_scalar_range_arms_b {
    ($emit:ident, $context:tt, $fxn_name:ident, $shape:tt, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, $shape, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, $shape, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, $shape, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, $shape, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, $shape, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix2, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix4, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix2x3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix3x2, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, RowVector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, RowVector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, RowVector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, RowDVector, DVector);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, DMatrix, DVector);
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_impl_assign_range_range_arms_b {
    ($emit:ident, $context:tt, $fxn_name:ident, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, Matrix1, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix1", feature = "row_vector2"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, RowVector2, Matrix1, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix1", feature = "row_vector3"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, RowVector3, Matrix1, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "matrix1", feature = "row_vector4"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, RowVector4, Matrix1, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "matrix1", feature = "row_vectord"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, RowDVector,Matrix1,DVector);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector2", feature = "matrix1"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, Vector2,Vector2,Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector3", feature = "matrix1"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, Vector3,Vector3,Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector4", feature = "matrix1"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, Vector4,Vector4,Matrix1);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vectord", feature = "matrix1"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, DVector,DVector,Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector2"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, Matrix2,Vector2,Vector2);
            #[cfg(all(feature = $value_string, feature = "matrix3", feature = "vector3"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, Matrix2,Matrix3,Vector3);
            #[cfg(all(feature = $value_string, feature = "matrix4", feature = "vector4"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, Matrix4,Vector4,Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "vector3", feature = "vector2"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, Matrix2x3,Vector2,Vector3);
            #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "vector2", feature = "vector3"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, Matrix3x2,Vector3,Vector2);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vector4", feature = "vector2"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, DMatrix, Vector4, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vector2", feature = "vector4"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, DMatrix, Vector2, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vector2"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, DMatrix, DVector, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vector3"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, DMatrix, DVector, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vector4"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, DMatrix, DVector, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vectord"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, DMatrix, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, Matrix1, Matrix1, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix1", feature = "row_vector2"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, RowVector2, RowVector2, Matrix1, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix1", feature = "row_vector3"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, RowVector3, RowVector3, Matrix1, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "matrix1", feature = "row_vector4"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, RowVector4, RowVector4, Matrix1, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "matrix1", feature = "row_vectord"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, RowDVector, RowDVector, Matrix1, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector2", feature = "matrix1"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, Vector2, Vector2, Vector2, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector3", feature = "matrix1"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, Vector3, Vector3, Vector3, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector4", feature = "matrix1"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, Vector4, Vector4, Vector4, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vectord", feature = "matrix1"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, DVector, DVector, DVector, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector2"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, Matrix2, Matrix2, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrix3", feature = "vector3"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, Matrix3, Matrix3, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrix4", feature = "vector4"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, Matrix4, Matrix4, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "vector3", feature = "vector2"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, Matrix2x3, Matrix2x3, Vector2, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "vector2", feature = "vector3"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, Matrix3x2, Matrix3x2, Vector3, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vector4", feature = "vector2"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, DMatrix, DMatrix, Vector4, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vector2", feature = "vector4"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, DMatrix, DMatrix, Vector2, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vector2"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, DMatrix, DMatrix, DVector, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vector3"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, DMatrix, DMatrix, DVector, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vector4"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, DMatrix, DMatrix, DVector, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vectord"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, DMatrix, DMatrix, DVector, DVector);
        }
    };
}

#[cfg(all(feature = "matrix", feature = "f64"))]
macro_rules! install_legacy_impl_assign_range_range_arms_bu {
    ($emit:ident, $context:tt, $fxn_name:ident, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vectord"))]
            install_legacy_assign_srr_bu!($emit, $context, [<$fxn_name BU>], $value_kind, $value_string, DMatrix, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vectord"))]
            install_legacy_assign_srr_bu2!($emit, $context, [<$fxn_name VBU>], $value_kind, $value_string, DMatrix, DMatrix, DVector, DVector);
        }
    };
}

#[cfg(all(feature = "matrix", feature = "f64"))]
macro_rules! install_legacy_impl_assign_range_range_arms_ub {
    ($emit:ident, $context:tt, $fxn_name:ident, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vectord"))]
            install_legacy_assign_srr_ub!($emit, $context, [<$fxn_name UB>], $value_kind, $value_string, DMatrix, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vectord"))]
            install_legacy_assign_srr_ub2!($emit, $context, [<$fxn_name VUB>], $value_kind, $value_string, DMatrix, DMatrix, DVector, DVector);
        }
    };
}

#[cfg(feature = "matrix")]
#[cfg(feature = "matrix")]
macro_rules! install_legacy_for_sink_shapes {
    ($emit:ident, $context:tt, $arm:ident, $fxn_name:ident, $value_kind:ident, $value_string:tt) => {
        #[cfg(feature = "row_vector2")]
        $arm!(
            $emit,
            $context,
            $fxn_name,
            RowVector2,
            $value_kind,
            $value_string
        );
        #[cfg(feature = "row_vector3")]
        $arm!(
            $emit,
            $context,
            $fxn_name,
            RowVector3,
            $value_kind,
            $value_string
        );
        #[cfg(feature = "row_vector4")]
        $arm!(
            $emit,
            $context,
            $fxn_name,
            RowVector4,
            $value_kind,
            $value_string
        );
        #[cfg(feature = "vector2")]
        $arm!(
            $emit,
            $context,
            $fxn_name,
            Vector2,
            $value_kind,
            $value_string
        );
        #[cfg(feature = "vector3")]
        $arm!(
            $emit,
            $context,
            $fxn_name,
            Vector3,
            $value_kind,
            $value_string
        );
        #[cfg(feature = "vector4")]
        $arm!(
            $emit,
            $context,
            $fxn_name,
            Vector4,
            $value_kind,
            $value_string
        );
        #[cfg(feature = "matrix1")]
        $arm!(
            $emit,
            $context,
            $fxn_name,
            Matrix1,
            $value_kind,
            $value_string
        );
        #[cfg(feature = "matrix2")]
        $arm!(
            $emit,
            $context,
            $fxn_name,
            Matrix2,
            $value_kind,
            $value_string
        );
        #[cfg(feature = "matrix3")]
        $arm!(
            $emit,
            $context,
            $fxn_name,
            Matrix3,
            $value_kind,
            $value_string
        );
        #[cfg(feature = "matrix4")]
        $arm!(
            $emit,
            $context,
            $fxn_name,
            Matrix4,
            $value_kind,
            $value_string
        );
        #[cfg(feature = "matrix2x3")]
        $arm!(
            $emit,
            $context,
            $fxn_name,
            Matrix2x3,
            $value_kind,
            $value_string
        );
        #[cfg(feature = "matrix3x2")]
        $arm!(
            $emit,
            $context,
            $fxn_name,
            Matrix3x2,
            $value_kind,
            $value_string
        );
        #[cfg(feature = "matrixd")]
        $arm!(
            $emit,
            $context,
            $fxn_name,
            DMatrix,
            $value_kind,
            $value_string
        );
        #[cfg(feature = "row_vectord")]
        $arm!(
            $emit,
            $context,
            $fxn_name,
            RowDVector,
            $value_kind,
            $value_string
        );
        #[cfg(feature = "vectord")]
        $arm!(
            $emit,
            $context,
            $fxn_name,
            DVector,
            $value_kind,
            $value_string
        );
    };
}

#[cfg(feature = "matrix")]
#[cfg(feature = "matrix")]
macro_rules! install_legacy_for_type {
    (
        $driver:ident,
        $emit:ident,
        $context:tt,
        $arm:ident,
        $fxn_name:ident,
        $value_kind:ident,
        $value_string:tt
    ) => {
        $driver!($emit, $context, $arm, $fxn_name, $value_kind, $value_string);
    };
}

#[cfg(feature = "matrix")]
#[cfg(feature = "matrix")]
macro_rules! install_legacy_for_all_types {
    ($driver:ident, $emit:ident, $context:tt, $arm:ident, $fxn_name:ident) => {
        install_legacy_for_type!($driver, $emit, $context, $arm, $fxn_name, u8, "u8");
        install_legacy_for_type!($driver, $emit, $context, $arm, $fxn_name, u16, "u16");
        install_legacy_for_type!($driver, $emit, $context, $arm, $fxn_name, u32, "u32");
        install_legacy_for_type!($driver, $emit, $context, $arm, $fxn_name, u64, "u64");
        install_legacy_for_type!($driver, $emit, $context, $arm, $fxn_name, u128, "u128");
        install_legacy_for_type!($driver, $emit, $context, $arm, $fxn_name, i8, "i8");
        install_legacy_for_type!($driver, $emit, $context, $arm, $fxn_name, i16, "i16");
        install_legacy_for_type!($driver, $emit, $context, $arm, $fxn_name, i32, "i32");
        install_legacy_for_type!($driver, $emit, $context, $arm, $fxn_name, i64, "i64");
        install_legacy_for_type!($driver, $emit, $context, $arm, $fxn_name, i128, "i128");
        install_legacy_for_type!($driver, $emit, $context, $arm, $fxn_name, f32, "f32");
        install_legacy_for_type!($driver, $emit, $context, $arm, $fxn_name, f64, "f64");
        install_legacy_for_type!($driver, $emit, $context, $arm, $fxn_name, R64, "rational");
        install_legacy_for_type!($driver, $emit, $context, $arm, $fxn_name, C64, "complex");
        install_legacy_for_type!($driver, $emit, $context, $arm, $fxn_name, bool, "bool");
        install_legacy_for_type!($driver, $emit, $context, $arm, $fxn_name, String, "string");
    };
}

#[cfg(feature = "matrix")]
#[cfg(feature = "matrix")]
macro_rules! install_legacy_direct {
    ($emit:ident, $context:tt, $arm:ident, $fxn_name:ident, $value_kind:ident, $value_string:tt) => {
        $arm!($emit, $context, $fxn_name, $value_kind, $value_string);
    };
}

#[cfg(feature = "matrix")]
#[cfg(feature = "matrix")]
macro_rules! for_each_matrix_assignment_factory {
    ($all_types:ident, $one_type:ident, $direct:ident, $emit:ident, $context:tt) => {
        $all_types!(
            install_legacy_for_sink_shapes,
            $emit,
            (
                $context,
                RuntimeOutputAliasPolicy::DisallowInputAlias,
                Linear
            ),
            install_legacy_impl_assign_scalar_arms,
            Assign1D
        );
        $all_types!(
            install_legacy_for_sink_shapes,
            $emit,
            (
                $context,
                RuntimeOutputAliasPolicy::DisallowInputAlias,
                LogicalLinear
            ),
            install_legacy_impl_assign_scalar_arms_b,
            Assign1D
        );
        $all_types!(
            install_legacy_for_sink_shapes,
            $emit,
            (
                $context,
                RuntimeOutputAliasPolicy::DisallowInputAlias,
                Linear
            ),
            install_legacy_impl_set_range_arms,
            Assign1DR
        );
        $all_types!(
            install_legacy_direct,
            $emit,
            (
                $context,
                RuntimeOutputAliasPolicy::DisallowInputAlias,
                LogicalLinear
            ),
            install_legacy_impl_set_range_arms_b,
            Assign1DR
        );
        $all_types!(
            install_legacy_for_sink_shapes,
            $emit,
            (
                $context,
                RuntimeOutputAliasPolicy::DisallowInputAlias,
                Whole
            ),
            install_legacy_impl_assign_all_arms,
            Set1DA
        );
        $all_types!(
            install_legacy_for_sink_shapes,
            $emit,
            (
                $context,
                RuntimeOutputAliasPolicy::DisallowInputAlias,
                RowColumn
            ),
            install_legacy_impl_assign_scalar_scalar_arms,
            Assign2DSS
        );
        $all_types!(
            install_legacy_direct,
            $emit,
            (
                $context,
                RuntimeOutputAliasPolicy::DisallowInputAlias,
                Column
            ),
            install_legacy_impl_assign_all_scalar_arms,
            Assign2DAS
        );
        $all_types!(
            install_legacy_direct,
            $emit,
            ($context, RuntimeOutputAliasPolicy::DisallowInputAlias, Row),
            install_legacy_impl_assign_scalar_all_arms,
            Assign2DSA
        );
        $all_types!(
            install_legacy_for_sink_shapes,
            $emit,
            (
                $context,
                RuntimeOutputAliasPolicy::DisallowInputAlias,
                RowColumn
            ),
            install_legacy_impl_assign_range_scalar_arms,
            Assign2DRS
        );
        $all_types!(
            install_legacy_for_sink_shapes,
            $emit,
            (
                $context,
                RuntimeOutputAliasPolicy::DisallowInputAlias,
                LogicalRowColumn
            ),
            install_legacy_impl_assign_range_scalar_arms_b,
            Assign2DRS
        );
        $all_types!(
            install_legacy_for_sink_shapes,
            $emit,
            (
                $context,
                RuntimeOutputAliasPolicy::DisallowInputAlias,
                RowColumn
            ),
            install_legacy_impl_assign_scalar_range_arms,
            Assign2DSR
        );
        $all_types!(
            install_legacy_for_sink_shapes,
            $emit,
            (
                $context,
                RuntimeOutputAliasPolicy::DisallowInputAlias,
                RowLogicalColumn
            ),
            install_legacy_impl_assign_scalar_range_arms_b,
            Assign2DSR
        );
        $all_types!(
            install_legacy_for_sink_shapes,
            $emit,
            (
                $context,
                RuntimeOutputAliasPolicy::DisallowInputAlias,
                RowColumn
            ),
            install_legacy_impl_assign_range_range_arms,
            Assign2DRR
        );
        $all_types!(
            install_legacy_direct,
            $emit,
            (
                $context,
                RuntimeOutputAliasPolicy::DisallowInputAlias,
                LogicalRowLogicalColumn
            ),
            install_legacy_impl_assign_range_range_arms_b,
            Assign2DRR
        );
        #[cfg(feature = "f64")]
        $direct!(
            $emit,
            (
                $context,
                RuntimeOutputAliasPolicy::DisallowInputAlias,
                LogicalRowColumn
            ),
            install_legacy_impl_assign_range_range_arms_bu,
            Assign2DRR,
            f64,
            "f64"
        );
        #[cfg(feature = "f64")]
        $direct!(
            $emit,
            (
                $context,
                RuntimeOutputAliasPolicy::DisallowInputAlias,
                RowLogicalColumn
            ),
            install_legacy_impl_assign_range_range_arms_ub,
            Assign2DRR,
            f64,
            "f64"
        );
        $all_types!(
            install_legacy_for_sink_shapes,
            $emit,
            (
                $context,
                RuntimeOutputAliasPolicy::DisallowInputAlias,
                Column
            ),
            install_legacy_impl_assign_all_range_arms,
            Set2DAR
        );
        $all_types!(
            install_legacy_direct,
            $emit,
            (
                $context,
                RuntimeOutputAliasPolicy::DisallowInputAlias,
                LogicalColumn
            ),
            install_legacy_impl_set_all_range_arms_b,
            Set2DAR
        );

        // Preserve the legacy omission of i128 from the non-boolean range/all path.
        $one_type!(
            install_legacy_for_sink_shapes,
            $emit,
            ($context, RuntimeOutputAliasPolicy::DisallowInputAlias, Row),
            install_legacy_impl_set_range_all_arms,
            Set2DRA,
            u8,
            "u8"
        );
        $one_type!(
            install_legacy_for_sink_shapes,
            $emit,
            ($context, RuntimeOutputAliasPolicy::DisallowInputAlias, Row),
            install_legacy_impl_set_range_all_arms,
            Set2DRA,
            u16,
            "u16"
        );
        $one_type!(
            install_legacy_for_sink_shapes,
            $emit,
            ($context, RuntimeOutputAliasPolicy::DisallowInputAlias, Row),
            install_legacy_impl_set_range_all_arms,
            Set2DRA,
            u32,
            "u32"
        );
        $one_type!(
            install_legacy_for_sink_shapes,
            $emit,
            ($context, RuntimeOutputAliasPolicy::DisallowInputAlias, Row),
            install_legacy_impl_set_range_all_arms,
            Set2DRA,
            u64,
            "u64"
        );
        $one_type!(
            install_legacy_for_sink_shapes,
            $emit,
            ($context, RuntimeOutputAliasPolicy::DisallowInputAlias, Row),
            install_legacy_impl_set_range_all_arms,
            Set2DRA,
            u128,
            "u128"
        );
        $one_type!(
            install_legacy_for_sink_shapes,
            $emit,
            ($context, RuntimeOutputAliasPolicy::DisallowInputAlias, Row),
            install_legacy_impl_set_range_all_arms,
            Set2DRA,
            i8,
            "i8"
        );
        $one_type!(
            install_legacy_for_sink_shapes,
            $emit,
            ($context, RuntimeOutputAliasPolicy::DisallowInputAlias, Row),
            install_legacy_impl_set_range_all_arms,
            Set2DRA,
            i16,
            "i16"
        );
        $one_type!(
            install_legacy_for_sink_shapes,
            $emit,
            ($context, RuntimeOutputAliasPolicy::DisallowInputAlias, Row),
            install_legacy_impl_set_range_all_arms,
            Set2DRA,
            i32,
            "i32"
        );
        $one_type!(
            install_legacy_for_sink_shapes,
            $emit,
            ($context, RuntimeOutputAliasPolicy::DisallowInputAlias, Row),
            install_legacy_impl_set_range_all_arms,
            Set2DRA,
            i64,
            "i64"
        );
        $one_type!(
            install_legacy_for_sink_shapes,
            $emit,
            ($context, RuntimeOutputAliasPolicy::DisallowInputAlias, Row),
            install_legacy_impl_set_range_all_arms,
            Set2DRA,
            f32,
            "f32"
        );
        $one_type!(
            install_legacy_for_sink_shapes,
            $emit,
            ($context, RuntimeOutputAliasPolicy::DisallowInputAlias, Row),
            install_legacy_impl_set_range_all_arms,
            Set2DRA,
            f64,
            "f64"
        );
        $one_type!(
            install_legacy_for_sink_shapes,
            $emit,
            ($context, RuntimeOutputAliasPolicy::DisallowInputAlias, Row),
            install_legacy_impl_set_range_all_arms,
            Set2DRA,
            R64,
            "rational"
        );
        $one_type!(
            install_legacy_for_sink_shapes,
            $emit,
            ($context, RuntimeOutputAliasPolicy::DisallowInputAlias, Row),
            install_legacy_impl_set_range_all_arms,
            Set2DRA,
            C64,
            "complex"
        );
        $one_type!(
            install_legacy_for_sink_shapes,
            $emit,
            ($context, RuntimeOutputAliasPolicy::DisallowInputAlias, Row),
            install_legacy_impl_set_range_all_arms,
            Set2DRA,
            bool,
            "bool"
        );
        $one_type!(
            install_legacy_for_sink_shapes,
            $emit,
            ($context, RuntimeOutputAliasPolicy::DisallowInputAlias, Row),
            install_legacy_impl_set_range_all_arms,
            Set2DRA,
            String,
            "string"
        );
        $all_types!(
            install_legacy_direct,
            $emit,
            (
                $context,
                RuntimeOutputAliasPolicy::DisallowInputAlias,
                LogicalRow
            ),
            install_legacy_impl_set_range_all_arms_b,
            Set2DRA
        );
    };
}

#[cfg(feature = "matrix")]
#[cfg(feature = "matrix")]
macro_rules! install_legacy_for_type_runtime {
    (
        $driver:ident,
        $emit:ident,
        ($builder:ident, $output_alias:expr, $layout:ident),
        $arm:ident,
        $fxn_name:ident,
        $value_kind:ident,
        $value_string:tt
    ) => {{
        #[cfg(feature = $value_string)]
        {
            #[inline(never)]
            fn install_type(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
                $driver!(
                    $emit,
                    (builder, $output_alias, $layout),
                    $arm,
                    $fxn_name,
                    $value_kind,
                    $value_string
                );
                Ok(())
            }

            install_type($builder)?;
        }
    }};
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_for_all_types_runtime {
    ($driver:ident, $emit:ident, ($builder:ident, $output_alias:expr, $layout:ident), $arm:ident, $fxn_name:ident) => {
        install_legacy_for_type_runtime!(
            $driver,
            $emit,
            ($builder, $output_alias, $layout),
            $arm,
            $fxn_name,
            u8,
            "u8"
        );
        install_legacy_for_type_runtime!(
            $driver,
            $emit,
            ($builder, $output_alias, $layout),
            $arm,
            $fxn_name,
            u16,
            "u16"
        );
        install_legacy_for_type_runtime!(
            $driver,
            $emit,
            ($builder, $output_alias, $layout),
            $arm,
            $fxn_name,
            u32,
            "u32"
        );
        install_legacy_for_type_runtime!(
            $driver,
            $emit,
            ($builder, $output_alias, $layout),
            $arm,
            $fxn_name,
            u64,
            "u64"
        );
        install_legacy_for_type_runtime!(
            $driver,
            $emit,
            ($builder, $output_alias, $layout),
            $arm,
            $fxn_name,
            u128,
            "u128"
        );
        install_legacy_for_type_runtime!(
            $driver,
            $emit,
            ($builder, $output_alias, $layout),
            $arm,
            $fxn_name,
            i8,
            "i8"
        );
        install_legacy_for_type_runtime!(
            $driver,
            $emit,
            ($builder, $output_alias, $layout),
            $arm,
            $fxn_name,
            i16,
            "i16"
        );
        install_legacy_for_type_runtime!(
            $driver,
            $emit,
            ($builder, $output_alias, $layout),
            $arm,
            $fxn_name,
            i32,
            "i32"
        );
        install_legacy_for_type_runtime!(
            $driver,
            $emit,
            ($builder, $output_alias, $layout),
            $arm,
            $fxn_name,
            i64,
            "i64"
        );
        install_legacy_for_type_runtime!(
            $driver,
            $emit,
            ($builder, $output_alias, $layout),
            $arm,
            $fxn_name,
            i128,
            "i128"
        );
        install_legacy_for_type_runtime!(
            $driver,
            $emit,
            ($builder, $output_alias, $layout),
            $arm,
            $fxn_name,
            f32,
            "f32"
        );
        install_legacy_for_type_runtime!(
            $driver,
            $emit,
            ($builder, $output_alias, $layout),
            $arm,
            $fxn_name,
            f64,
            "f64"
        );
        install_legacy_for_type_runtime!(
            $driver,
            $emit,
            ($builder, $output_alias, $layout),
            $arm,
            $fxn_name,
            R64,
            "rational"
        );
        install_legacy_for_type_runtime!(
            $driver,
            $emit,
            ($builder, $output_alias, $layout),
            $arm,
            $fxn_name,
            C64,
            "complex"
        );
        install_legacy_for_type_runtime!(
            $driver,
            $emit,
            ($builder, $output_alias, $layout),
            $arm,
            $fxn_name,
            bool,
            "bool"
        );
        install_legacy_for_type_runtime!(
            $driver,
            $emit,
            ($builder, $output_alias, $layout),
            $arm,
            $fxn_name,
            String,
            "string"
        );
    };
}

#[cfg(all(feature = "matrix", feature = "f64"))]
macro_rules! install_legacy_direct_runtime {
    ($emit:ident, ($builder:ident, $output_alias:expr, $layout:ident), $arm:ident, $fxn_name:ident, $value_kind:ident, $value_string:tt) => {{
        #[inline(never)]
        fn install_type(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
            $arm!(
                $emit,
                (builder, $output_alias, $layout),
                $fxn_name,
                $value_kind,
                $value_string
            );
            Ok(())
        }

        install_type($builder)?;
    }};
}

#[cfg(feature = "matrix")]
for_each_matrix_assignment_factory!(
    install_legacy_for_all_types,
    install_legacy_for_type,
    install_legacy_direct,
    declare_matrix_assign_factory,
    ()
);

#[cfg(feature = "matrix")]
fn install_matrix_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    for_each_matrix_assignment_factory!(
        install_legacy_for_all_types_runtime,
        install_legacy_for_type_runtime,
        install_legacy_direct_runtime,
        register_matrix_assign_factory,
        builder
    );
    Ok(())
}

/// Installs every enabled bytecode factory owned by stable assignment.
pub fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    for_each_assign_scalar_factory!(register_assign_scalar_factory, (builder));
    #[cfg(feature = "assign")]
    register_assign_index(builder)?;

    #[cfg(feature = "matrix")]
    install_matrix_runtime(builder)?;
    Ok(())
}

/// Installs whole-register matrix assignment factories emitted by the source
/// compiler. Runtime-only catalogs remain unchanged; compiler and native-plan
/// catalogs both close the exact storage-shaped bytecode surface.
#[cfg(all(
    any(feature = "native-plan", feature = "semantic-compiler"),
    feature = "matrix"
))]
fn install_compiled_matrix_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    register_assign_value_matrix_for_scalar!(builder, u8, "u8");
    register_assign_value_matrix_for_scalar!(builder, u16, "u16");
    register_assign_value_matrix_for_scalar!(builder, u32, "u32");
    register_assign_value_matrix_for_scalar!(builder, u64, "u64");
    register_assign_value_matrix_for_scalar!(builder, u128, "u128");
    register_assign_value_matrix_for_scalar!(builder, i8, "i8");
    register_assign_value_matrix_for_scalar!(builder, i16, "i16");
    register_assign_value_matrix_for_scalar!(builder, i32, "i32");
    register_assign_value_matrix_for_scalar!(builder, i64, "i64");
    register_assign_value_matrix_for_scalar!(builder, i128, "i128");
    register_assign_value_matrix_for_scalar!(builder, f32, "f32");
    register_assign_value_matrix_for_scalar!(builder, f64, "f64");
    register_assign_value_matrix_for_scalar!(builder, bool, "bool");
    register_assign_value_matrix_for_scalar!(builder, string, "string");
    register_assign_value_matrix_for_scalar!(builder, r64, "r64");
    register_assign_value_matrix_for_scalar!(builder, c64, "c64");
    Ok(())
}

#[cfg(feature = "semantic-compiler")]
pub(crate) fn install_compiler_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "matrix")]
    install_compiled_matrix_runtime(builder)?;
    let _ = builder;
    Ok(())
}

#[cfg(feature = "native-plan")]
pub(crate) fn install_native_plan(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "matrix")]
    install_compiled_matrix_runtime(builder)?;
    let _ = builder;
    Ok(())
}

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    for_each_assign_scalar_factory!(export_assign_scalar_factory, ());
    export_assign_value_matrix_for_scalar!(u8, "u8");
    export_assign_value_matrix_for_scalar!(u16, "u16");
    export_assign_value_matrix_for_scalar!(u32, "u32");
    export_assign_value_matrix_for_scalar!(u64, "u64");
    export_assign_value_matrix_for_scalar!(u128, "u128");
    export_assign_value_matrix_for_scalar!(i8, "i8");
    export_assign_value_matrix_for_scalar!(i16, "i16");
    export_assign_value_matrix_for_scalar!(i32, "i32");
    export_assign_value_matrix_for_scalar!(i64, "i64");
    export_assign_value_matrix_for_scalar!(i128, "i128");
    export_assign_value_matrix_for_scalar!(f32, "f32");
    export_assign_value_matrix_for_scalar!(f64, "f64");
    export_assign_value_matrix_for_scalar!(bool, "bool");
    export_assign_value_matrix_for_scalar!(string, "string");
    export_assign_value_matrix_for_scalar!(r64, "r64");
    export_assign_value_matrix_for_scalar!(c64, "c64");

    #[cfg(feature = "assign")]
    pub use super::install_assign_index;
    #[cfg(feature = "matrix")]
    for_each_matrix_assignment_factory!(
        install_legacy_for_all_types,
        install_legacy_for_type,
        install_legacy_direct,
        export_matrix_assign_factory,
        ()
    );
}

#[cfg(all(
    test,
    feature = "matrix",
    feature = "matrixd",
    feature = "vectord",
    feature = "logical_indexing",
    feature = "u8"
))]
mod tests {
    use super::*;
    use mech_core::{FunctionInvocation, Ref, RuntimeFunctionId};
    use nalgebra::{DMatrix, DVector};

    fn output() -> ValueCell {
        ValueCell::from_exact_matrix_ref(Ref::new(DMatrix::<u8>::zeros(2, 3)), 2, 3).unwrap()
    }

    fn source() -> ValueCell {
        ValueCell::from_exact(7_u8).unwrap()
    }

    fn vector_source(values: &[u8]) -> ValueCell {
        ValueCell::from_exact_matrix_ref(
            Ref::new(DVector::from_column_slice(values)),
            values.len(),
            1,
        )
        .unwrap()
    }

    fn matrix_source(rows: usize, columns: usize, values: &[u8]) -> ValueCell {
        ValueCell::from_exact_matrix_ref(
            Ref::new(DMatrix::from_row_slice(rows, columns, values)),
            rows,
            columns,
        )
        .unwrap()
    }

    fn indices(values: &[usize]) -> ValueCell {
        ValueCell::from_exact_matrix_ref(
            Ref::new(DVector::from_column_slice(values)),
            values.len(),
            1,
        )
        .unwrap()
    }

    fn index(value: usize) -> ValueCell {
        ValueCell::from_exact(value).unwrap()
    }

    fn logical_mask(values: &[bool]) -> ValueCell {
        ValueCell::from_exact_matrix_ref(
            Ref::new(DVector::from_column_slice(values)),
            values.len(),
            1,
        )
        .unwrap()
    }

    #[test]
    fn two_dimensional_assignment_indices_use_their_exact_axes() {
        validate_assign_row_column(&output(), &[source(), indices(&[1, 2]), index(3)]).unwrap();

        let row_error = validate_assign_row_column(&output(), &[source(), indices(&[3]), index(1)])
            .unwrap_err();
        assert!(row_error.kind_message().contains("row index 3"));

        let column_error =
            validate_assign_row_column(&output(), &[source(), index(1), index(4)]).unwrap_err();
        assert!(column_error.kind_message().contains("column index 4"));
    }

    #[test]
    fn logical_masks_are_validated_against_their_declared_layout() {
        validate_assign_logical_linear(
            &output(),
            &[
                source(),
                logical_mask(&[true, false, true, false, true, false]),
            ],
        )
        .unwrap();
        let error =
            validate_assign_logical_row(&output(), &[source(), logical_mask(&[true, false, true])])
                .unwrap_err();
        assert!(
            error
                .kind_message()
                .contains("row logical mask has 3 elements, expected 2")
        );
    }

    #[test]
    fn missing_declared_selector_is_a_structured_contract_error() {
        let error = validate_assign_row(&output(), &[source()]).unwrap_err();
        assert_eq!(error.kind_name(), "FunctionShapeContractViolation");
        assert!(error.kind_message().contains("missing row index input 1"));
    }

    #[test]
    fn assignment_source_cardinality_tracks_selected_cells_only() {
        let selected = indices(&[1, 2, 3]);
        let error =
            validate_assign_linear(&output(), &[vector_source(&[10, 20]), selected.clone()])
                .unwrap_err();
        assert_eq!(error.kind_name(), "FunctionShapeContractViolation");
        assert!(
            error
                .kind_message()
                .contains("source has 2 elements, selected layout requires at least 3")
        );

        validate_assign_linear(&output(), &[vector_source(&[10, 20, 30]), selected]).unwrap();

        // Scalar sources broadcast, so selector cardinality is not bounded by the
        // sink cardinality when every selected index is valid.
        validate_assign_linear(&output(), &[source(), indices(&[1, 1, 2, 2, 3, 3, 6, 6])]).unwrap();

        validate_assign_logical_linear(
            &output(),
            &[
                vector_source(&[9]),
                logical_mask(&[true, false, false, false, false, false]),
            ],
        )
        .unwrap();
        let error = validate_assign_logical_linear(
            &output(),
            &[
                vector_source(&[9]),
                logical_mask(&[false, true, false, false, false, false]),
            ],
        )
        .unwrap_err();
        assert!(error.kind_message().contains("requires at least 2"));
    }

    #[test]
    fn row_selection_requires_complete_source_rows_or_one_broadcast_row() {
        let selected_rows = indices(&[1, 2]);
        let error = validate_assign_row(
            &output(),
            &[matrix_source(2, 2, &[1, 2, 3, 4]), selected_rows.clone()],
        )
        .unwrap_err();
        assert!(error.kind_message().contains("requires at least 2x3"));

        validate_assign_row(
            &output(),
            &[
                matrix_source(2, 3, &[1, 2, 3, 4, 5, 6]),
                selected_rows.clone(),
            ],
        )
        .unwrap();
        validate_assign_row(&output(), &[matrix_source(1, 3, &[1, 2, 3]), selected_rows]).unwrap();
    }

    #[test]
    fn installed_factories_enforce_every_assignment_index_layout() {
        let mut builder = FunctionCatalogBuilder::new();
        install_runtime(&mut builder).unwrap();
        let catalog = builder.build().unwrap();

        let entry = |name: &str| {
            catalog
                .runtime_entry(RuntimeFunctionId::from_name(name))
                .unwrap_or_else(|| panic!("installed assignment factory {name}"))
        };

        let aliased_vector = vector_source(&[1, 2, 3]);
        let error = entry("Assign1DRV<u8DVectorDVectorDVector>")
            .validate_invocation(&FunctionInvocation::binary(
                aliased_vector.clone(),
                aliased_vector,
                indices(&[1, 2, 3]),
            ))
            .unwrap_err();
        assert!(error.kind_message().contains("alias"));

        entry("Set1DAS<u8DMatrix>")
            .validate_invocation(&FunctionInvocation::unary(output(), source()))
            .unwrap();

        let linear = entry("Assign1DS<u8DMatrix>");
        linear
            .validate_invocation(&FunctionInvocation::binary(output(), source(), index(6)))
            .unwrap();
        let error = linear
            .validate_invocation(&FunctionInvocation::binary(output(), source(), index(7)))
            .unwrap_err();
        assert!(error.kind_message().contains("linear index 7"));

        let logical_linear = entry("Assign1DRB<u8DMatrixDVector>");
        logical_linear
            .validate_invocation(&FunctionInvocation::binary(
                output(),
                source(),
                logical_mask(&[true, false, true, false, true, false]),
            ))
            .unwrap();
        let error = logical_linear
            .validate_invocation(&FunctionInvocation::binary(
                output(),
                source(),
                logical_mask(&[true, false, true, false, true]),
            ))
            .unwrap_err();
        assert!(error.kind_message().contains("expected 6"));

        let logical_row = entry("Set2DRAB<u8DMatrixDVector>");
        logical_row
            .validate_invocation(&FunctionInvocation::binary(
                output(),
                source(),
                logical_mask(&[true, false]),
            ))
            .unwrap();
        let error = logical_row
            .validate_invocation(&FunctionInvocation::binary(
                output(),
                source(),
                logical_mask(&[true, false, true]),
            ))
            .unwrap_err();
        assert!(error.kind_message().contains("expected 2"));

        let logical_column = entry("Set2DARB<u8DMatrixDVector>");
        logical_column
            .validate_invocation(&FunctionInvocation::binary(
                output(),
                source(),
                logical_mask(&[true, false, true]),
            ))
            .unwrap();
        let error = logical_column
            .validate_invocation(&FunctionInvocation::binary(
                output(),
                source(),
                logical_mask(&[true, false]),
            ))
            .unwrap_err();
        assert!(error.kind_message().contains("expected 3"));

        let row = entry("Assign2DSAS<u8DMatrix>");
        row.validate_invocation(&FunctionInvocation::binary(output(), source(), index(2)))
            .unwrap();
        let error = row
            .validate_invocation(&FunctionInvocation::binary(output(), source(), index(3)))
            .unwrap_err();
        assert!(error.kind_message().contains("row index 3"));

        let column = entry("Assign2DASS<u8DMatrix>");
        column
            .validate_invocation(&FunctionInvocation::binary(output(), source(), index(3)))
            .unwrap();
        let error = column
            .validate_invocation(&FunctionInvocation::binary(output(), source(), index(4)))
            .unwrap_err();
        assert!(error.kind_message().contains("column index 4"));

        let logical_row_column = entry("Assign2DRSB<u8DMatrixDVector>");
        logical_row_column
            .validate_invocation(&FunctionInvocation::ternary(
                output(),
                source(),
                logical_mask(&[true, false]),
                index(3),
            ))
            .unwrap();
        let error = logical_row_column
            .validate_invocation(&FunctionInvocation::ternary(
                output(),
                source(),
                logical_mask(&[true, false, true]),
                index(3),
            ))
            .unwrap_err();
        assert!(error.kind_message().contains("expected 2"));

        let row_logical_column = entry("Assign2DSRB<u8DMatrixDVector>");
        row_logical_column
            .validate_invocation(&FunctionInvocation::ternary(
                output(),
                source(),
                index(2),
                logical_mask(&[true, false, true]),
            ))
            .unwrap();
        let error = row_logical_column
            .validate_invocation(&FunctionInvocation::ternary(
                output(),
                source(),
                index(2),
                logical_mask(&[true, false]),
            ))
            .unwrap_err();
        assert!(error.kind_message().contains("expected 3"));

        let logical_row_logical_column = entry("Assign2DRRBB<u8DMatrixDVectorDVector>");
        logical_row_logical_column
            .validate_invocation(&FunctionInvocation::ternary(
                output(),
                source(),
                logical_mask(&[true, false]),
                logical_mask(&[true, false, true]),
            ))
            .unwrap();
        let error = logical_row_logical_column
            .validate_invocation(&FunctionInvocation::ternary(
                output(),
                source(),
                logical_mask(&[true, false]),
                logical_mask(&[true, false]),
            ))
            .unwrap_err();
        assert!(error.kind_message().contains("expected 3"));

        let row_column = entry("Assign2DSSS<u8DMatrix>");
        row_column
            .validate_invocation(&FunctionInvocation::ternary(
                output(),
                source(),
                index(2),
                index(3),
            ))
            .unwrap();
        let error = row_column
            .validate_invocation(&FunctionInvocation::ternary(
                output(),
                source(),
                index(2),
                index(4),
            ))
            .unwrap_err();
        assert!(error.kind_message().contains("column index 4"));

        let range = entry("Assign2DRSS<u8DMatrixDVector>");
        range
            .validate_invocation(&FunctionInvocation::ternary(
                output(),
                source(),
                indices(&[1, 2]),
                index(3),
            ))
            .unwrap();
        let error = range
            .validate_invocation(&FunctionInvocation::ternary(
                output(),
                source(),
                indices(&[1, 3]),
                index(3),
            ))
            .unwrap_err();
        assert!(error.kind_message().contains("row index 3"));
    }
}
