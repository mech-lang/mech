#[cfg(test)]
use mech_core::PORTABLE_SELECTOR_INDEX_MAX as PORTABLE_INDEX_MAX;
use mech_core::snapshot::{
    F32Bits, F64Bits, SequenceView, SnapshotCanonicalizationBudget, SnapshotValidationContext,
    SnapshotValueError, ValueDataDraft, ValueDraft, ValueFootprint,
    canonical_sequence_element_retained_footprint, canonical_snapshot_data_draft, compare_key_data,
    schema_data_language_eq, schema_data_partial_cmp,
};
use mech_core::{
    AccessMode, AliasPolicy, BoundResidentKernel, ChangeDetectionPolicy, DeliveryMode,
    ExternalInteraction, FunctionCatalogBuilder, ImplementationMemoryClass, MResult,
    OutputConstruction, RegionPolicy, ResidentKernelBindError, ResidentKernelBindRequest,
    ResidentKernelError, ResidentKernelInputs, ResidentShape, ResidentSnapshotOutput,
    ResidentValueKind, ResidentValueMut, ResidentValueRef, ResolvedOperationContract,
    ResolvedSelectionMode, ResolvedSourceRouting, SchemaBody, SchemaId, ShapeContractReference,
    ShapeInstance, ShapeRule, ValueData,
};
use std::sync::Arc;

const MAX_MATRIX_SOLVE_WORK: usize = 16_777_216;

fn snapshot_clone_cost(
    meter: &mut super::budget::ResidentBudgetMeter,
    value: &mech_core::Value,
    schemas: &mech_core::SchemaTable,
) -> Result<(u64, u64), ResidentKernelError> {
    let footprint = super::budget::measure_canonical_value_footprint(meter, value, schemas)?;
    Ok((footprint.retained_bytes, footprint.node_count))
}

fn snapshot_lane_clone_footprint(
    meter: &mut super::budget::ResidentBudgetMeter,
    values: &[Option<mech_core::Value>],
    schemas: &mech_core::SchemaTable,
) -> Result<ValueFootprint, ResidentKernelError> {
    values
        .iter()
        .flatten()
        .try_fold(ValueFootprint::zero(), |total, value| {
            let footprint =
                super::budget::measure_canonical_value_footprint(meter, value, schemas)?;
            total
                .checked_add(footprint)
                .map_err(|_| ResidentKernelError::InvalidShape)
        })
}

fn snapshot_element_clone_footprint(
    meter: &mut super::budget::ResidentBudgetMeter,
    body: &SchemaBody,
    data: &ValueData,
    index: usize,
) -> Result<ValueFootprint, ResidentKernelError> {
    match (body, data) {
        (SchemaBody::Matrix { element, .. }, ValueData::Matrix(matrix)) => {
            let mut footprint = ValueFootprint::zero();
            selected_sequence_footprint(&mut footprint, meter, element, matrix.elements(), index)?;
            Ok(footprint)
        }
        (_, _) if index == 0 => super::budget::measure_canonical_data_footprint(meter, body, data),
        _ => Err(ResidentKernelError::InvalidShape),
    }
}

fn snapshot_element_comparison_work(
    meter: &mut super::budget::ResidentBudgetMeter,
    body: &SchemaBody,
    data: &ValueData,
    index: usize,
) -> Result<u64, ResidentKernelError> {
    match (body, data) {
        (SchemaBody::Matrix { element, .. }, ValueData::Matrix(matrix)) => {
            match matrix.elements() {
                SequenceView::Values(values) => {
                    let value = values.get(index).ok_or(ResidentKernelError::InvalidShape)?;
                    super::budget::measure_canonical_data_comparison_work(meter, element, value)
                }
                values => {
                    let footprint =
                        canonical_sequence_element_retained_footprint(element, values, index)
                            .map_err(|_| ResidentKernelError::InvalidInput)?;
                    Ok(footprint.encoded_bytes.max(footprint.node_count).max(1))
                }
            }
        }
        (_, _) if index == 0 => {
            super::budget::measure_canonical_data_comparison_work(meter, body, data)
        }
        _ => Err(ResidentKernelError::InvalidShape),
    }
}

fn snapshot_element_finalization_work(
    meter: &mut super::budget::ResidentBudgetMeter,
    body: &SchemaBody,
    data: &ValueData,
    index: usize,
) -> Result<u64, ResidentKernelError> {
    match (body, data) {
        (SchemaBody::Matrix { element, .. }, ValueData::Matrix(matrix)) => {
            match matrix.elements() {
                SequenceView::Values(values) => {
                    super::budget::preflight_canonical_data_finalization(
                        meter,
                        element,
                        values.get(index).ok_or(ResidentKernelError::InvalidShape)?,
                    )
                }
                values if index < values.len() => Ok(0),
                _ => Err(ResidentKernelError::InvalidShape),
            }
        }
        (_, _) if index == 0 => {
            super::budget::preflight_canonical_data_finalization(meter, body, data)
        }
        _ => Err(ResidentKernelError::InvalidShape),
    }
}

fn snapshot_string_element<'a>(
    body: &SchemaBody,
    data: &'a ValueData,
    index: usize,
) -> Result<&'a str, ResidentKernelError> {
    match (body, data) {
        (SchemaBody::Matrix { element, .. }, ValueData::Matrix(matrix))
            if element.as_ref() == &SchemaBody::String =>
        {
            let SequenceView::String(values) = matrix.elements() else {
                return Err(ResidentKernelError::InvalidInput);
            };
            values
                .get(index)
                .map(|value| value.as_ref())
                .ok_or(ResidentKernelError::InvalidShape)
        }
        (SchemaBody::String, ValueData::String(value)) if index == 0 => Ok(value.as_ref()),
        _ => Err(ResidentKernelError::InvalidShape),
    }
}

fn projected_snapshot_matrix_output(
    meter: &mut super::budget::ResidentBudgetMeter,
    current_schema: &SchemaBody,
    current_data: &ValueData,
    current_footprint: ValueFootprint,
    source_schema: &SchemaBody,
    source_data: &ValueData,
    output_len: usize,
    mut selected_source: impl FnMut(
        usize,
        &mut super::budget::ResidentBudgetMeter,
    ) -> Result<Option<usize>, ResidentKernelError>,
) -> Result<(ValueFootprint, u64), ResidentKernelError> {
    let (SchemaBody::Matrix { element, .. }, ValueData::Matrix(current_matrix)) =
        (current_schema, current_data)
    else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    if current_matrix.elements().len() != output_len {
        return Err(ResidentKernelError::InvalidShape);
    }
    let mut finalization_work =
        snapshot_data_finalization_work(meter, current_schema, current_data)?;
    match current_matrix.elements() {
        SequenceView::Values(current_values) => {
            let mut current_elements = ValueFootprint::zero();
            for value in current_values {
                current_elements = current_elements
                    .checked_add(super::budget::measure_canonical_data_footprint(
                        meter, element, value,
                    )?)
                    .map_err(|_| ResidentKernelError::InvalidShape)?;
            }
            let base = ValueFootprint {
                encoded_bytes: 0,
                retained_bytes: current_footprint
                    .retained_bytes
                    .checked_sub(current_elements.retained_bytes)
                    .ok_or(ResidentKernelError::InvalidShape)?,
                node_count: current_footprint
                    .node_count
                    .checked_sub(
                        current_elements
                            .node_count
                            .checked_add(1)
                            .ok_or(ResidentKernelError::InvalidShape)?,
                    )
                    .ok_or(ResidentKernelError::InvalidShape)?,
            };
            let mut final_elements = ValueFootprint::zero();
            for destination in 0..output_len {
                let selected = selected_source(destination, meter)?;
                let footprint = match selected {
                    Some(source_index) => {
                        finalization_work = finalization_work
                            .checked_add(snapshot_element_finalization_work(
                                meter,
                                source_schema,
                                source_data,
                                source_index,
                            )?)
                            .ok_or(ResidentKernelError::InvalidShape)?;
                        snapshot_element_clone_footprint(
                            meter,
                            source_schema,
                            source_data,
                            source_index,
                        )?
                    }
                    None => super::budget::measure_canonical_data_footprint(
                        meter,
                        element,
                        current_values
                            .get(destination)
                            .ok_or(ResidentKernelError::InvalidShape)?,
                    )?,
                };
                final_elements = final_elements
                    .checked_add(footprint)
                    .map_err(|_| ResidentKernelError::InvalidShape)?;
            }
            let sequence_node = ValueFootprint {
                encoded_bytes: 0,
                retained_bytes: 0,
                node_count: 1,
            };
            Ok((
                base.checked_add(sequence_node)
                    .and_then(|footprint| footprint.checked_add(final_elements))
                    .map_err(|_| ResidentKernelError::InvalidShape)?,
                finalization_work,
            ))
        }
        SequenceView::String(current_values) => {
            let container_bytes = super::budget::checked_u64(
                output_len
                    .checked_mul(core::mem::size_of::<Box<str>>())
                    .ok_or(ResidentKernelError::InvalidShape)?,
            )?;
            let current_payload = current_values.iter().try_fold(0u64, |total, value| {
                total
                    .checked_add(super::budget::checked_u64(value.len())?)
                    .ok_or(ResidentKernelError::InvalidShape)
            })?;
            let sequence_nodes = super::budget::checked_u64(
                output_len
                    .checked_add(1)
                    .ok_or(ResidentKernelError::InvalidShape)?,
            )?;
            let base_bytes = current_footprint
                .retained_bytes
                .checked_sub(container_bytes)
                .and_then(|bytes| bytes.checked_sub(current_payload))
                .ok_or(ResidentKernelError::InvalidShape)?;
            let base_nodes = current_footprint
                .node_count
                .checked_sub(sequence_nodes)
                .ok_or(ResidentKernelError::InvalidShape)?;
            let mut final_payload = 0u64;
            let mut encoded_bytes = 0u64;
            for (destination, current) in current_values.iter().enumerate() {
                let selected = selected_source(destination, meter)?;
                let next = match selected {
                    Some(source_index) => {
                        finalization_work = finalization_work
                            .checked_add(snapshot_element_finalization_work(
                                meter,
                                source_schema,
                                source_data,
                                source_index,
                            )?)
                            .ok_or(ResidentKernelError::InvalidShape)?;
                        snapshot_string_element(source_schema, source_data, source_index)?
                    }
                    None => current.as_ref(),
                };
                let bytes = super::budget::checked_u64(next.len())?;
                final_payload = final_payload
                    .checked_add(bytes)
                    .ok_or(ResidentKernelError::InvalidShape)?;
                encoded_bytes = encoded_bytes
                    .checked_add(
                        bytes
                            .checked_add(8)
                            .ok_or(ResidentKernelError::InvalidShape)?,
                    )
                    .ok_or(ResidentKernelError::InvalidShape)?;
            }
            Ok((
                ValueFootprint {
                    encoded_bytes,
                    retained_bytes: base_bytes
                        .checked_add(container_bytes)
                        .and_then(|bytes| bytes.checked_add(final_payload))
                        .ok_or(ResidentKernelError::InvalidShape)?,
                    node_count: base_nodes
                        .checked_add(sequence_nodes)
                        .ok_or(ResidentKernelError::InvalidShape)?,
                },
                finalization_work,
            ))
        }
        _ => {
            for destination in 0..output_len {
                if let Some(source_index) = selected_source(destination, meter)? {
                    finalization_work = finalization_work
                        .checked_add(snapshot_element_finalization_work(
                            meter,
                            source_schema,
                            source_data,
                            source_index,
                        )?)
                        .ok_or(ResidentKernelError::InvalidShape)?;
                }
            }
            Ok((current_footprint, finalization_work))
        }
    }
}

#[derive(Clone, Copy)]
#[repr(u64)]
enum SemanticComparison {
    Equal = 0,
    NotEqual = 1,
    Less = 2,
    LessEqual = 3,
    Greater = 4,
    GreaterEqual = 5,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u64)]
enum SemanticArithmetic {
    Add = 0,
    Subtract = 1,
    Multiply = 2,
    Divide = 3,
    Remainder = 4,
    Power = 5,
}

impl SemanticArithmetic {
    fn from_parameter(value: u64) -> Option<Self> {
        Some(match value {
            0 => Self::Add,
            1 => Self::Subtract,
            2 => Self::Multiply,
            3 => Self::Divide,
            4 => Self::Remainder,
            5 => Self::Power,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug)]
struct SnapshotAccessSelectorLayout {
    schema: SchemaId,
    shape: ShapeInstance,
    resident_shape: ResidentShape,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ResolvedAccessGeometry {
    logical_output_rows: usize,
    logical_output_columns: usize,
}

#[derive(Clone, Debug)]
struct SnapshotAccessPlan {
    selectors: Box<[SnapshotAccessSelectorLayout]>,
    matrix_mode: Option<ResolvedSelectionMode>,
    source_dimensions: Option<(usize, usize)>,
    output_dimensions: Option<(usize, usize)>,
    output_geometry: ResolvedAccessGeometry,
    aggregate_ordinal: Option<usize>,
    output_schema: SchemaId,
}

#[derive(Clone, Debug)]
struct SnapshotAggregateAssignPlan {
    source: SnapshotAccessSelectorLayout,
    selector: SnapshotAccessSelectorLayout,
    aggregate_ordinal: Option<usize>,
}

#[derive(Clone, Debug)]
struct MatrixSelectionAssignPlan {
    selectors: Box<[SnapshotAccessSelectorLayout]>,
    mode: ResolvedSelectionMode,
    rows: usize,
    columns: usize,
    source_rows: usize,
    source_columns: usize,
    source_routing: ResolvedSourceRouting,
}

impl SemanticComparison {
    fn from_parameter(value: u64) -> Option<Self> {
        Some(match value {
            0 => Self::Equal,
            1 => Self::NotEqual,
            2 => Self::Less,
            3 => Self::LessEqual,
            4 => Self::Greater,
            5 => Self::GreaterEqual,
            _ => return None,
        })
    }

    fn is_equality(self) -> bool {
        matches!(self, Self::Equal | Self::NotEqual)
    }
}

pub(crate) fn install(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    // ProgramArtifact nodes carry semantic identities. These factories select
    // the resident implementation from the resolved contract and layouts.
    register_no_additional_scratch(builder, &["math"], "add", bind_add)?;
    register_no_additional_scratch(builder, &["math"], "add-assign", bind_semantic_add_assign)?;
    register_no_additional_scratch(
        builder,
        &["math", "add-assign"],
        "range-all",
        bind_add_indexed_rows,
    )?;
    register_no_additional_scratch(
        builder,
        &["math", "sub-assign"],
        "range-all",
        bind_sub_indexed_rows,
    )?;
    register_no_additional_scratch(builder, &["math"], "sub", bind_sub)?;
    register_no_additional_scratch(builder, &["math"], "mul", bind_semantic_mul)?;
    register_no_additional_scratch(builder, &["math"], "div", bind_div)?;
    register_no_additional_scratch(builder, &["math"], "mod", bind_remainder)?;
    register_no_additional_scratch(builder, &["math"], "neg", bind_negate)?;
    register_no_additional_scratch(builder, &["math"], "pow", bind_pow)?;
    register_no_additional_scratch(builder, &["math"], "abs", bind_abs)?;
    register_no_additional_scratch(builder, &["math"], "acos", bind_math_acos)?;
    register_no_additional_scratch(builder, &["math"], "acosh", bind_math_acosh)?;
    register_no_additional_scratch(builder, &["math"], "acot", bind_math_acot)?;
    register_no_additional_scratch(builder, &["math"], "acsc", bind_math_acsc)?;
    register_no_additional_scratch(builder, &["math"], "asec", bind_math_asec)?;
    register_no_additional_scratch(builder, &["math"], "asin", bind_math_asin)?;
    register_no_additional_scratch(builder, &["math"], "asinh", bind_math_asinh)?;
    register_no_additional_scratch(builder, &["math"], "atan", bind_math_atan)?;
    register_no_additional_scratch(builder, &["math"], "copysign", bind_math_copysign)?;
    register_no_additional_scratch(builder, &["math"], "atanh", bind_math_atanh)?;
    register_no_additional_scratch(builder, &["math"], "cbrt", bind_math_cbrt)?;
    register_no_additional_scratch(builder, &["math"], "ceil", bind_math_ceil)?;
    register_no_additional_scratch(builder, &["math"], "cosh", bind_math_cosh)?;
    register_no_additional_scratch(builder, &["math"], "cot", bind_math_cot)?;
    register_no_additional_scratch(builder, &["math"], "csc", bind_math_csc)?;
    register_no_additional_scratch(builder, &["math"], "erf", bind_math_erf)?;
    register_no_additional_scratch(builder, &["math"], "erfc", bind_math_erfc)?;
    register_no_additional_scratch(builder, &["math"], "fdim", bind_math_fdim)?;
    register_no_additional_scratch(builder, &["math"], "floor", bind_floor)?;
    register_no_additional_scratch(builder, &["math"], "fmod", bind_math_fmod)?;
    register_no_additional_scratch(builder, &["math"], "lgamma", bind_math_lgamma)?;
    register_no_additional_scratch(builder, &["math"], "log", bind_math_log)?;
    register_no_additional_scratch(builder, &["math"], "log10", bind_math_log10)?;
    register_no_additional_scratch(builder, &["math"], "log1p", bind_math_log1p)?;
    register_no_additional_scratch(builder, &["math"], "log2", bind_math_log2)?;
    register_no_additional_scratch(builder, &["math"], "nextafter", bind_math_nextafter)?;
    register_no_additional_scratch(builder, &["math"], "remainder", bind_math_remainder)?;
    register_no_additional_scratch(builder, &["math"], "rint", bind_math_rint)?;
    register_no_additional_scratch(builder, &["math"], "round", bind_math_round)?;
    register_no_additional_scratch(builder, &["math"], "roundeven", bind_math_roundeven)?;
    register_no_additional_scratch(builder, &["math"], "sec", bind_math_sec)?;
    register_no_additional_scratch(builder, &["math"], "sqrt", bind_sqrt)?;
    register_no_additional_scratch(builder, &["math"], "sinh", bind_math_sinh)?;
    register_no_additional_scratch(builder, &["math"], "tan", bind_math_tan)?;
    register_no_additional_scratch(builder, &["math"], "tanh", bind_math_tanh)?;
    register_no_additional_scratch(builder, &["math"], "tgamma", bind_math_tgamma)?;
    register_no_additional_scratch(builder, &["math"], "trunc", bind_math_trunc)?;
    register_no_additional_scratch(builder, &["math"], "atan2", bind_atan2)?;
    register_no_additional_scratch(builder, &["math"], "cos", bind_cos)?;
    register_no_additional_scratch(builder, &["math"], "sin", bind_sin)?;
    register_no_additional_scratch(builder, &["math", "bessel"], "j0", bind_math_bessel_j0)?;
    register_no_additional_scratch(builder, &["math", "bessel"], "j1", bind_math_bessel_j1)?;
    register_no_additional_scratch(builder, &["math", "bessel"], "jn", bind_math_bessel_jn)?;
    register_no_additional_scratch(builder, &["math", "bessel"], "y0", bind_math_bessel_y0)?;
    register_no_additional_scratch(builder, &["math", "bessel"], "y1", bind_math_bessel_y1)?;
    register_no_additional_scratch(builder, &["math", "bessel"], "yn", bind_math_bessel_yn)?;
    register_no_additional_scratch(builder, &["logic"], "and", bind_semantic_bool_and)?;
    register_no_additional_scratch(builder, &["logic"], "or", bind_semantic_bool_or)?;
    register_no_additional_scratch(builder, &["logic"], "xor", bind_semantic_bool_xor)?;
    register_no_additional_scratch(builder, &["logic"], "not", bind_bool_not)?;
    register_no_additional_scratch(builder, &["compare"], "eq", bind_semantic_equal)?;
    register_no_additional_scratch(builder, &["compare"], "neq", bind_semantic_not_equal)?;
    register_no_additional_scratch(builder, &["compare"], "lt", bind_semantic_less)?;
    register_no_additional_scratch(builder, &["compare"], "lte", bind_semantic_less_equal)?;
    register_no_additional_scratch(builder, &["compare"], "gt", bind_semantic_greater)?;
    register_no_additional_scratch(builder, &["compare"], "gte", bind_semantic_greater_equal)?;
    register_no_additional_scratch(builder, &["compare"], "seq", bind_strict_equal)?;
    register_no_additional_scratch(builder, &["compare"], "sneq", bind_strict_not_equal)?;
    register_canonical_finalize(builder, &["access"], "scalar", bind_semantic_scalar_access)?;
    register_canonical_finalize(builder, &["access"], "rows", bind_semantic_rows_access)?;
    register_canonical_finalize(
        builder,
        &["access"],
        "columns",
        bind_semantic_columns_access,
    )?;
    register_no_additional_scratch(builder, &["access"], "index", bind_scalar_index)?;
    register_canonical_finalize(builder, &["access"], "range", bind_semantic_range_access)?;
    register_canonical_finalize(builder, &["matrix"], "horzcat", bind_horizontal)?;
    register_canonical_finalize(builder, &["matrix"], "vertcat", bind_vertical)?;
    register_canonical_finalize(
        builder,
        &["matrix"],
        "comprehension",
        bind_matrix_comprehension,
    )?;
    register_no_additional_scratch(builder, &["matrix"], "multiply", bind_matmul)?;
    register_no_additional_scratch(builder, &["matrix"], "dot", bind_matrix_dot)?;
    register_with_memory_class(
        builder,
        &["matrix"],
        "solve",
        ImplementationMemoryClass::MatrixSolve,
        bind_matrix_solve,
    )?;
    register_canonical_finalize(builder, &["matrix"], "transpose", bind_semantic_transpose)?;
    register_canonical_finalize(builder, &["core"], "assign", bind_hold_state)?;
    register_canonical_finalize(
        builder,
        &["core", "assign"],
        "whole-value",
        bind_whole_matrix_assign,
    )?;
    register_canonical_finalize(
        builder,
        &["core", "assign"],
        "indexed-axis",
        bind_indexed_assign,
    )?;
    register_canonical_finalize(
        builder,
        &["core", "assign"],
        "indexed-rows",
        bind_indexed_assign_rows,
    )?;
    register_canonical_finalize(
        builder,
        &["core", "assign"],
        "indexed-columns",
        bind_indexed_assign_columns,
    )?;
    register_canonical_finalize(
        builder,
        &["core", "assign"],
        "indexed-rectangle",
        bind_indexed_assign_rectangle,
    )?;
    register_canonical_finalize(
        builder,
        &["core", "assign"],
        "collection-entry",
        bind_collection_entry_assign,
    )?;
    register_canonical_finalize(
        builder,
        &["core", "assign"],
        "single-element",
        bind_single_element_assign,
    )?;
    register_no_additional_scratch(builder, &["range"], "exclusive", bind_range_exclusive)?;
    register_no_additional_scratch(
        builder,
        &["range"],
        "exclusive-increment",
        bind_range_increment_exclusive,
    )?;
    register_no_additional_scratch(builder, &["range"], "inclusive", bind_range_inclusive)?;
    register_no_additional_scratch(
        builder,
        &["range"],
        "inclusive-increment",
        bind_range_increment_inclusive,
    )?;
    register_no_additional_scratch(builder, &["combinatorics"], "n-choose-k", bind_n_choose_k)?;
    register_no_additional_scratch(builder, &["stats", "sum"], "column", bind_sum_columns)?;
    register_no_additional_scratch(builder, &["stats", "sum"], "row", bind_sum_rows)?;

    register_no_additional_scratch(builder, &["ekf"], "trigonometric-state", bind_ekf_trig)?;
    register_no_additional_scratch(builder, &["ekf"], "motion-jacobian", bind_ekf_motion)?;
    register_no_additional_scratch(builder, &["ekf"], "control-jacobian", bind_ekf_control)?;
    register_no_additional_scratch(
        builder,
        &["ekf"],
        "predicted-state",
        bind_ekf_predicted_state,
    )?;
    register_no_additional_scratch(
        builder,
        &["ekf"],
        "predicted-covariance",
        bind_ekf_predicted_covariance,
    )?;
    register_no_additional_scratch(
        builder,
        &["ekf"],
        "landmark-delta-and-range",
        bind_ekf_landmark,
    )?;
    register_no_additional_scratch(
        builder,
        &["ekf"],
        "predicted-measurement",
        bind_ekf_measurement,
    )?;
    register_no_additional_scratch(
        builder,
        &["ekf"],
        "measurement-jacobian",
        bind_ekf_measurement_jacobian,
    )?;
    register_no_additional_scratch(
        builder,
        &["ekf"],
        "innovation-covariance",
        bind_ekf_innovation_covariance,
    )?;
    register_no_additional_scratch(builder, &["ekf"], "solve-2x2", bind_ekf_solve)?;
    register_no_additional_scratch(builder, &["ekf"], "kalman-gain", bind_ekf_gain)?;
    register_no_additional_scratch(builder, &["ekf"], "innovation", bind_ekf_innovation)?;
    register_no_additional_scratch(
        builder,
        &["ekf"],
        "corrected-state",
        bind_ekf_corrected_state,
    )?;
    register_no_additional_scratch(
        builder,
        &["ekf"],
        "joseph-covariance-update",
        bind_ekf_joseph,
    )?;
    register_no_additional_scratch(
        builder,
        &["ekf"],
        "covariance-symmetrization",
        bind_ekf_symmetrize,
    )?;
    register_no_additional_scratch(builder, &["ekf"], "candidate-finite", bind_ekf_finite)?;
    register_no_additional_scratch(
        builder,
        &["ekf"],
        "covariance-positive-diagonal",
        bind_ekf_positive_diagonal,
    )?;
    register_no_additional_scratch(
        builder,
        &["ekf"],
        "covariance-symmetric",
        bind_ekf_symmetric,
    )?;
    Ok(())
}

fn register_no_additional_scratch(
    builder: &mut FunctionCatalogBuilder,
    module: &[&str],
    operation: &str,
    factory: mech_core::ResidentKernelFactory,
) -> MResult<()> {
    register_with_memory_class(
        builder,
        module,
        operation,
        ImplementationMemoryClass::NoAdditionalScratch,
        factory,
    )
}

fn register_canonical_finalize(
    builder: &mut FunctionCatalogBuilder,
    module: &[&str],
    operation: &str,
    factory: mech_core::ResidentKernelFactory,
) -> MResult<()> {
    register_with_memory_class(
        builder,
        module,
        operation,
        ImplementationMemoryClass::CanonicalFinalize,
        factory,
    )
}

fn register_with_memory_class(
    builder: &mut FunctionCatalogBuilder,
    module: &[&str],
    operation: &str,
    implementation_memory: ImplementationMemoryClass,
    factory: mech_core::ResidentKernelFactory,
) -> MResult<()> {
    builder.insert_resident_factory(
        module.iter().copied(),
        operation,
        implementation_memory,
        factory,
    )
}

fn bound(
    executor: mech_core::ResidentKernelExecutor,
    parameters: impl Into<Box<[u64]>>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    Ok(BoundResidentKernel::new(executor, parameters.into()))
}

fn validate_contract(
    request: &ResidentKernelBindRequest<'_>,
    input_count: usize,
    construction: OutputConstruction,
    alias: AliasPolicy,
    output_access: AccessMode,
    change_detection: ChangeDetectionPolicy,
) -> Result<(), ResidentKernelBindError> {
    let ResolvedOperationContract::Declared(contract) = request.contract else {
        return Err(ResidentKernelBindError::UnsupportedContract);
    };
    if contract.interaction != ExternalInteraction::Pure
        || contract.inputs.len() != input_count
        || request.inputs.len() != input_count
        || contract.outputs.len() != 1
        || contract
            .inputs
            .iter()
            .zip(request.inputs)
            .any(|(port, layout)| {
                port.schema != layout.schema_id
                    || port.access != AccessMode::Read
                    || port.delivery != DeliveryMode::Signal
            })
    {
        return Err(ResidentKernelBindError::UnsupportedContract);
    }
    let output = &contract.outputs[0];
    if output.schema != request.output.schema_id
        || output.access != output_access
        || output.delivery != DeliveryMode::Signal
        || output.construction != construction
        || output.alias != alias
        || output.change_detection != change_detection
    {
        return Err(ResidentKernelBindError::UnsupportedContract);
    }
    Ok(())
}

fn validate_full_write(
    request: &ResidentKernelBindRequest<'_>,
    input_count: usize,
    shape: ShapeRule,
    change_detection: ChangeDetectionPolicy,
) -> Result<(), ResidentKernelBindError> {
    validate_contract(
        request,
        input_count,
        OutputConstruction::FullWrite { shape },
        AliasPolicy::NoAlias,
        AccessMode::Write,
        change_detection,
    )
}

fn validate_build(
    request: &ResidentKernelBindRequest<'_>,
    input_count: usize,
    module_path: &[&str],
    contract_name: &str,
) -> Result<(), ResidentKernelBindError> {
    validate_contract(
        request,
        input_count,
        OutputConstruction::Build {
            postcondition: ShapeContractReference {
                module_path: module_path
                    .iter()
                    .map(|segment| (*segment).to_owned())
                    .collect(),
                contract_name: contract_name.to_owned(),
            },
        },
        AliasPolicy::NoAlias,
        AccessMode::Write,
        ChangeDetectionPolicy::KernelReported,
    )
}

fn validate_rmw(
    request: &ResidentKernelBindRequest<'_>,
    input_count: usize,
    regions: RegionPolicy,
) -> Result<(), ResidentKernelBindError> {
    validate_contract(
        request,
        input_count,
        OutputConstruction::ReadModifyWrite {
            base_input: 0,
            regions,
        },
        AliasPolicy::MayAlias { input: 0 },
        AccessMode::ReadWrite,
        ChangeDetectionPolicy::KernelReported,
    )
}

fn require_kind(
    request: &ResidentKernelBindRequest<'_>,
    input_kinds: &[ResidentValueKind],
    output_kind: ResidentValueKind,
) -> Result<(), ResidentKernelBindError> {
    if request.inputs.len() != input_kinds.len()
        || request
            .inputs
            .iter()
            .zip(input_kinds)
            .any(|(layout, kind)| layout.kind != *kind)
        || request.output.kind != output_kind
        || request.output.shape.len().is_none()
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    Ok(())
}

fn require_f64_lengths(
    request: &ResidentKernelBindRequest<'_>,
    input_lengths: &[usize],
    output_length: usize,
) -> Result<(), ResidentKernelBindError> {
    require_kind(
        request,
        &vec![ResidentValueKind::F64; input_lengths.len()],
        ResidentValueKind::F64,
    )?;
    if request
        .inputs
        .iter()
        .zip(input_lengths)
        .any(|(layout, len)| layout.shape.len() != Some(*len))
        || request.output.shape.len() != Some(output_length)
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    Ok(())
}

fn validate_f64_executor(
    request: &ResidentKernelBindRequest<'_>,
    input_lengths: &[usize],
    output_kind: ResidentValueKind,
    output_length: usize,
    change_detection: ChangeDetectionPolicy,
) -> Result<(), ResidentKernelBindError> {
    validate_full_write(
        request,
        input_lengths.len(),
        ShapeRule::Declared,
        change_detection,
    )?;
    require_kind(
        request,
        &vec![ResidentValueKind::F64; input_lengths.len()],
        output_kind,
    )?;
    if request
        .inputs
        .iter()
        .zip(input_lengths)
        .any(|(layout, len)| layout.shape.len() != Some(*len))
        || request.output.shape.len() != Some(output_length)
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    Ok(())
}

macro_rules! binder {
    ($binder:ident, $executor:ident, [$first:expr], $output_kind:expr, $output:expr, $change:expr) => {
        fn $binder(
            request: &ResidentKernelBindRequest<'_>,
        ) -> Result<BoundResidentKernel, ResidentKernelBindError> {
            validate_f64_executor(request, &[$first], $output_kind, $output, $change)?;
            Ok(BoundResidentKernel::new_f64_1($executor, Box::new([])))
        }
    };
    ($binder:ident, $executor:ident, [$first:expr, $second:expr], $output_kind:expr, $output:expr, $change:expr) => {
        fn $binder(
            request: &ResidentKernelBindRequest<'_>,
        ) -> Result<BoundResidentKernel, ResidentKernelBindError> {
            validate_f64_executor(request, &[$first, $second], $output_kind, $output, $change)?;
            Ok(BoundResidentKernel::new_f64_2($executor, Box::new([])))
        }
    };
    ($binder:ident, $executor:ident, [$first:expr, $second:expr, $third:expr], $output_kind:expr, $output:expr, $change:expr) => {
        fn $binder(
            request: &ResidentKernelBindRequest<'_>,
        ) -> Result<BoundResidentKernel, ResidentKernelBindError> {
            validate_f64_executor(
                request,
                &[$first, $second, $third],
                $output_kind,
                $output,
                $change,
            )?;
            Ok(BoundResidentKernel::new_f64_3($executor, Box::new([])))
        }
    };
    ($binder:ident, $executor:ident, [$first:expr, $second:expr, $third:expr, $fourth:expr], $output_kind:expr, $output:expr, $change:expr) => {
        fn $binder(
            request: &ResidentKernelBindRequest<'_>,
        ) -> Result<BoundResidentKernel, ResidentKernelBindError> {
            validate_f64_executor(
                request,
                &[$first, $second, $third, $fourth],
                $output_kind,
                $output,
                $change,
            )?;
            Ok(BoundResidentKernel::new_f64_4($executor, Box::new([])))
        }
    };
}

macro_rules! binder_f64_output {
    ($binder:ident, $executor:ident, [$first:expr], $output:expr, $change:expr) => {
        fn $binder(
            request: &ResidentKernelBindRequest<'_>,
        ) -> Result<BoundResidentKernel, ResidentKernelBindError> {
            validate_f64_executor(request, &[$first], ResidentValueKind::F64, $output, $change)?;
            Ok(BoundResidentKernel::new_f64_output_1(
                $executor,
                Box::new([]),
            ))
        }
    };
    ($binder:ident, $executor:ident, [$first:expr, $second:expr], $output:expr, $change:expr) => {
        fn $binder(
            request: &ResidentKernelBindRequest<'_>,
        ) -> Result<BoundResidentKernel, ResidentKernelBindError> {
            validate_f64_executor(
                request,
                &[$first, $second],
                ResidentValueKind::F64,
                $output,
                $change,
            )?;
            Ok(BoundResidentKernel::new_f64_output_2(
                $executor,
                Box::new([]),
            ))
        }
    };
    ($binder:ident, $executor:ident, [$first:expr, $second:expr, $third:expr], $output:expr, $change:expr) => {
        fn $binder(
            request: &ResidentKernelBindRequest<'_>,
        ) -> Result<BoundResidentKernel, ResidentKernelBindError> {
            validate_f64_executor(
                request,
                &[$first, $second, $third],
                ResidentValueKind::F64,
                $output,
                $change,
            )?;
            Ok(BoundResidentKernel::new_f64_output_3(
                $executor,
                Box::new([]),
            ))
        }
    };
    ($binder:ident, $executor:ident, [$first:expr, $second:expr, $third:expr, $fourth:expr], $output:expr, $change:expr) => {
        fn $binder(
            request: &ResidentKernelBindRequest<'_>,
        ) -> Result<BoundResidentKernel, ResidentKernelBindError> {
            validate_f64_executor(
                request,
                &[$first, $second, $third, $fourth],
                ResidentValueKind::F64,
                $output,
                $change,
            )?;
            Ok(BoundResidentKernel::new_f64_output_4(
                $executor,
                Box::new([]),
            ))
        }
    };
}

binder_f64_output!(
    bind_ekf_trig,
    ekf_trig,
    [3],
    2,
    ChangeDetectionPolicy::KernelReported
);
binder_f64_output!(
    bind_ekf_motion,
    ekf_motion,
    [3, 4, 2, 1],
    9,
    ChangeDetectionPolicy::KernelReported
);
binder_f64_output!(
    bind_ekf_control,
    ekf_control,
    [2, 1],
    6,
    ChangeDetectionPolicy::KernelReported
);
binder_f64_output!(
    bind_ekf_predicted_state,
    ekf_predicted_state,
    [3, 4, 2, 1],
    3,
    ChangeDetectionPolicy::KernelReported
);
binder_f64_output!(
    bind_ekf_predicted_covariance,
    ekf_predicted_covariance,
    [9, 9, 6, 4],
    9,
    ChangeDetectionPolicy::KernelReported
);
binder_f64_output!(
    bind_ekf_landmark,
    ekf_landmark,
    [3, 2],
    3,
    ChangeDetectionPolicy::KernelReported
);
binder_f64_output!(
    bind_ekf_measurement,
    ekf_measurement,
    [3, 3],
    2,
    ChangeDetectionPolicy::KernelReported
);
binder_f64_output!(
    bind_ekf_measurement_jacobian,
    ekf_measurement_jacobian,
    [3],
    6,
    ChangeDetectionPolicy::KernelReported
);
binder_f64_output!(
    bind_ekf_innovation_covariance,
    ekf_innovation_covariance,
    [9, 6, 4],
    4,
    ChangeDetectionPolicy::KernelReported
);
binder_f64_output!(
    bind_ekf_solve,
    ekf_solve,
    [4],
    4,
    ChangeDetectionPolicy::KernelReported
);
binder_f64_output!(
    bind_ekf_gain,
    ekf_gain,
    [9, 6, 4],
    6,
    ChangeDetectionPolicy::KernelReported
);
binder_f64_output!(
    bind_ekf_innovation,
    ekf_innovation,
    [4, 2],
    2,
    ChangeDetectionPolicy::KernelReported
);
binder_f64_output!(
    bind_ekf_corrected_state,
    ekf_corrected_state,
    [3, 6, 2],
    3,
    ChangeDetectionPolicy::KernelReported
);
binder_f64_output!(
    bind_ekf_joseph,
    ekf_joseph,
    [9, 6, 6, 4],
    9,
    ChangeDetectionPolicy::KernelReported
);
binder_f64_output!(
    bind_ekf_symmetrize,
    ekf_symmetrize,
    [9],
    9,
    ChangeDetectionPolicy::KernelReported
);
binder!(
    bind_ekf_finite,
    ekf_finite,
    [3, 9],
    ResidentValueKind::Bool,
    1,
    ChangeDetectionPolicy::ExactScalar
);
binder!(
    bind_ekf_positive_diagonal,
    ekf_positive_diagonal,
    [9],
    ResidentValueKind::Bool,
    1,
    ChangeDetectionPolicy::ExactScalar
);
binder!(
    bind_ekf_symmetric,
    ekf_symmetric,
    [9],
    ResidentValueKind::Bool,
    1,
    ChangeDetectionPolicy::ExactScalar
);

fn bind_hold_state(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_full_write(
        request,
        1,
        ShapeRule::SameAsInput { input: 0 },
        ChangeDetectionPolicy::KernelReported,
    )?;
    let [input] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let input_schema = request
        .schemas
        .get(input.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let output_schema = request
        .schemas
        .get(request.output.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let schema_compatible = input_schema.key() == output_schema.key()
        || matches!(
            (input_schema.body(), output_schema.body()),
            (
                SchemaBody::Matrix { element: input, .. },
                SchemaBody::Matrix { element: output, .. },
            ) if input == output
        );
    if !schema_compatible
        || input.kind != request.output.kind
        || input.shape != request.output.shape
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let kernel = bound(hold_state, Vec::<u64>::new().into_boxed_slice())?;
    if input.kind == ResidentValueKind::Snapshot {
        Ok(kernel.with_snapshot_schemas(request.schemas.clone()))
    } else {
        Ok(kernel)
    }
}

fn bind_negate(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let dense = || {
        let change = f64_output_change_detection(request)?;
        validate_full_write(request, 1, ShapeRule::SameAsInput { input: 0 }, change)?;
        require_kind(request, &[ResidentValueKind::F64], ResidentValueKind::F64)?;
        if request.inputs[0].shape != request.output.shape {
            return Err(ResidentKernelBindError::UnsupportedLayout);
        }
        bound(negate, Vec::<u64>::new().into_boxed_slice())
    };
    dense().or_else(|_| bind_snapshot_numeric_negate(request))
}

fn bind_unary_f64(
    request: &ResidentKernelBindRequest<'_>,
    executor: mech_core::ResidentKernelExecutor,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let change = f64_output_change_detection(request)?;
    validate_full_write(request, 1, ShapeRule::SameAsInput { input: 0 }, change)?;
    require_kind(request, &[ResidentValueKind::F64], ResidentValueKind::F64)?;
    if request.inputs[0].shape != request.output.shape {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    bound(executor, Vec::<u64>::new().into_boxed_slice())
}

fn bind_unary_f32_snapshot(
    request: &ResidentKernelBindRequest<'_>,
    executor: mech_core::ResidentKernelExecutor,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let [input] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let (input_element, input_shape) = comparison_logical_layout(request, input)?;
    let (output_element, output_shape) = comparison_logical_layout(request, &request.output)?;
    if input.kind != ResidentValueKind::Snapshot
        || request.output.kind != ResidentValueKind::Snapshot
        || input.shape != ResidentShape::SCALAR
        || request.output.shape != ResidentShape::SCALAR
        || input_element != &SchemaBody::FloatingPoint(mech_core::FloatWidth::W32)
        || input_element != output_element
        || input_shape != output_shape
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    validate_full_write(
        request,
        1,
        ShapeRule::SameAsInput { input: 0 },
        snapshot_output_change_detection(request)?,
    )?;
    Ok(bound(
        executor,
        vec![output_shape.rows as u64, output_shape.columns as u64].into_boxed_slice(),
    )?
    .with_snapshot_output(snapshot_output_metadata(request))
    .with_snapshot_schemas(request.schemas.clone()))
}

fn bind_unary_float(
    request: &ResidentKernelBindRequest<'_>,
    f64_executor: mech_core::ResidentKernelExecutor,
    f32_executor: mech_core::ResidentKernelExecutor,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_unary_f64(request, f64_executor)
        .or_else(|_| bind_unary_f32_snapshot(request, f32_executor))
}

macro_rules! float_unary_resident {
    ($binder:ident, $f64_executor:ident, $f32_executor:ident, $f64_operation:expr, $f32_operation:expr) => {
        fn $binder(
            request: &ResidentKernelBindRequest<'_>,
        ) -> Result<BoundResidentKernel, ResidentKernelBindError> {
            bind_unary_float(request, $f64_executor, $f32_executor)
        }

        fn $f64_executor(
            _kernel: &BoundResidentKernel,
            inputs: &dyn ResidentKernelInputs,
            output: ResidentValueMut<'_>,
        ) -> Result<bool, ResidentKernelError> {
            unary_f64(inputs, output, $f64_operation)
        }

        fn $f32_executor(
            kernel: &BoundResidentKernel,
            inputs: &dyn ResidentKernelInputs,
            output: ResidentValueMut<'_>,
        ) -> Result<bool, ResidentKernelError> {
            unary_f32_snapshot(kernel, inputs, output, $f32_operation)
        }
    };
}

float_unary_resident!(
    bind_math_acos,
    math_acos,
    math_acos_f32,
    libm::acos,
    libm::acosf
);
float_unary_resident!(
    bind_math_acosh,
    math_acosh,
    math_acosh_f32,
    libm::acosh,
    libm::acoshf
);
float_unary_resident!(
    bind_math_acot,
    math_acot,
    math_acot_f32,
    |value: f64| libm::atan(1.0 / value),
    |value: f32| libm::atanf(1.0 / value)
);
float_unary_resident!(
    bind_math_acsc,
    math_acsc,
    math_acsc_f32,
    |value: f64| libm::asin(1.0 / value),
    |value: f32| libm::asinf(1.0 / value)
);
float_unary_resident!(
    bind_math_asec,
    math_asec,
    math_asec_f32,
    |value: f64| libm::acos(1.0 / value),
    |value: f32| libm::acosf(1.0 / value)
);
float_unary_resident!(
    bind_math_asin,
    math_asin,
    math_asin_f32,
    libm::asin,
    libm::asinf
);
float_unary_resident!(
    bind_math_asinh,
    math_asinh,
    math_asinh_f32,
    libm::asinh,
    libm::asinhf
);
float_unary_resident!(
    bind_math_atan,
    math_atan,
    math_atan_f32,
    libm::atan,
    libm::atanf
);
float_unary_resident!(
    bind_math_atanh,
    math_atanh,
    math_atanh_f32,
    libm::atanh,
    libm::atanhf
);
float_unary_resident!(
    bind_math_cbrt,
    math_cbrt,
    math_cbrt_f32,
    libm::cbrt,
    libm::cbrtf
);
float_unary_resident!(
    bind_math_ceil,
    math_ceil,
    math_ceil_f32,
    libm::ceil,
    libm::ceilf
);
float_unary_resident!(
    bind_math_cosh,
    math_cosh,
    math_cosh_f32,
    libm::cosh,
    libm::coshf
);
float_unary_resident!(
    bind_math_cot,
    math_cot,
    math_cot_f32,
    |value: f64| 1.0 / libm::tan(value),
    |value: f32| 1.0 / libm::tanf(value)
);
float_unary_resident!(
    bind_math_csc,
    math_csc,
    math_csc_f32,
    |value: f64| 1.0 / libm::sin(value),
    |value: f32| 1.0 / libm::sinf(value)
);
float_unary_resident!(bind_math_erf, math_erf, math_erf_f32, libm::erf, libm::erff);
float_unary_resident!(
    bind_math_erfc,
    math_erfc,
    math_erfc_f32,
    libm::erfc,
    libm::erfcf
);
float_unary_resident!(
    bind_math_lgamma,
    math_lgamma,
    math_lgamma_f32,
    libm::lgamma,
    libm::lgammaf
);
float_unary_resident!(bind_math_log, math_log, math_log_f32, libm::log, libm::logf);
float_unary_resident!(
    bind_math_log10,
    math_log10,
    math_log10_f32,
    libm::log10,
    libm::log10f
);
float_unary_resident!(
    bind_math_log1p,
    math_log1p,
    math_log1p_f32,
    libm::log1p,
    libm::log1pf
);
float_unary_resident!(
    bind_math_log2,
    math_log2,
    math_log2_f32,
    libm::log2,
    libm::log2f
);
float_unary_resident!(
    bind_math_rint,
    math_rint,
    math_rint_f32,
    libm::rint,
    libm::rintf
);
float_unary_resident!(
    bind_math_round,
    math_round,
    math_round_f32,
    libm::round,
    libm::roundf
);
float_unary_resident!(
    bind_math_roundeven,
    math_roundeven,
    math_roundeven_f32,
    libm::roundeven,
    libm::roundevenf
);
float_unary_resident!(
    bind_math_sec,
    math_sec,
    math_sec_f32,
    |value: f64| 1.0 / libm::cos(value),
    |value: f32| 1.0 / libm::cosf(value)
);
float_unary_resident!(
    bind_math_sinh,
    math_sinh,
    math_sinh_f32,
    libm::sinh,
    libm::sinhf
);
float_unary_resident!(bind_math_tan, math_tan, math_tan_f32, libm::tan, libm::tanf);
float_unary_resident!(
    bind_math_tanh,
    math_tanh,
    math_tanh_f32,
    libm::tanh,
    libm::tanhf
);
float_unary_resident!(
    bind_math_tgamma,
    math_tgamma,
    math_tgamma_f32,
    libm::tgamma,
    libm::tgammaf
);
float_unary_resident!(
    bind_math_trunc,
    math_trunc,
    math_trunc_f32,
    libm::trunc,
    libm::truncf
);
float_unary_resident!(
    bind_math_bessel_j0,
    math_bessel_j0,
    math_bessel_j0_f32,
    libm::j0,
    libm::j0f
);
float_unary_resident!(
    bind_math_bessel_j1,
    math_bessel_j1,
    math_bessel_j1_f32,
    libm::j1,
    libm::j1f
);
float_unary_resident!(
    bind_math_bessel_y0,
    math_bessel_y0,
    math_bessel_y0_f32,
    libm::y0,
    libm::y0f
);
float_unary_resident!(
    bind_math_bessel_y1,
    math_bessel_y1,
    math_bessel_y1_f32,
    libm::y1,
    libm::y1f
);

fn bind_cos(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_unary_float(request, cosine, cosine_f32)
}

fn bind_abs(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_unary_float(request, absolute, absolute_f32)
        .or_else(|_| bind_snapshot_numeric_abs(request))
}

fn bind_floor(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_unary_float(request, floor, floor_f32)
}

fn bind_sqrt(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_unary_float(request, square_root, square_root_f32)
}

fn bind_sin(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_unary_float(request, sine, sine_f32)
}

fn bind_atan2(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_binary(request, atan2).or_else(|_| bind_binary_f32_snapshot(request, atan2_f32))
}

fn bind_math_copysign(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_binary(request, math_copysign)
        .or_else(|_| bind_binary_f32_snapshot(request, math_copysign_f32))
}

fn bind_math_fdim(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_binary(request, math_fdim).or_else(|_| bind_binary_f32_snapshot(request, math_fdim_f32))
}

fn bind_math_fmod(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_binary(request, math_fmod).or_else(|_| bind_binary_f32_snapshot(request, math_fmod_f32))
}

fn bind_math_nextafter(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_binary(request, math_nextafter)
        .or_else(|_| bind_binary_f32_snapshot(request, math_nextafter_f32))
}

fn bind_math_remainder(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_binary(request, math_remainder)
        .or_else(|_| bind_binary_f32_snapshot(request, math_remainder_f32))
}

fn bind_math_bessel_jn(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_binary(request, math_bessel_jn)
        .or_else(|_| bind_binary_f32_snapshot(request, math_bessel_jn_f32))
}

fn bind_math_bessel_yn(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_binary(request, math_bessel_yn)
        .or_else(|_| bind_binary_f32_snapshot(request, math_bessel_yn_f32))
}

fn bind_binary_f32_snapshot(
    request: &ResidentKernelBindRequest<'_>,
    executor: mech_core::ResidentKernelExecutor,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let output_schema = request
        .schemas
        .get(request.output.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let change = match output_schema.body() {
        SchemaBody::FloatingPoint(mech_core::FloatWidth::W32) => ChangeDetectionPolicy::ExactScalar,
        SchemaBody::Matrix { element, .. }
            if element.as_ref() == &SchemaBody::FloatingPoint(mech_core::FloatWidth::W32) =>
        {
            ChangeDetectionPolicy::KernelReported
        }
        _ => return Err(ResidentKernelBindError::UnsupportedLayout),
    };
    validate_full_write(request, 2, ShapeRule::Declared, change)?;
    require_kind(
        request,
        &[ResidentValueKind::Snapshot, ResidentValueKind::Snapshot],
        ResidentValueKind::Snapshot,
    )?;
    if request.output.shape != ResidentShape::SCALAR
        || request.inputs.iter().any(|input| {
            input.shape != ResidentShape::SCALAR || input.schema_key != request.output.schema_key
        })
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    Ok(bound(executor, Vec::<u64>::new())?
        .with_snapshot_output(ResidentSnapshotOutput {
            schema: request.output.schema_id,
            schema_key: request.output.schema_key,
            shape: request.output.shape_instance.clone(),
            exact_cardinality: None,
            maximum_cardinality: None,
        })
        .with_snapshot_schemas(request.schemas.clone()))
}

fn bind_sub(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_binary(request, subtract)
        .or_else(|_| bind_snapshot_numeric_binary(request, SemanticArithmetic::Subtract))
}

fn bind_add(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_binary(request, add)
        .or_else(|_| bind_snapshot_numeric_binary(request, SemanticArithmetic::Add))
}

fn bind_mul(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_binary(request, multiply)
}

fn bind_semantic_mul(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_mul(request)
        .or_else(|_| bind_snapshot_numeric_binary(request, SemanticArithmetic::Multiply))
        .or_else(|_| bind_mul_rows(request))
}

fn bind_div(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_binary(request, divide)
        .or_else(|_| bind_snapshot_numeric_binary(request, SemanticArithmetic::Divide))
}

fn bind_remainder(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_binary(request, remainder)
        .or_else(|_| bind_snapshot_numeric_binary(request, SemanticArithmetic::Remainder))
}

fn bind_f64_comparison(
    request: &ResidentKernelBindRequest<'_>,
    executor: mech_core::ResidentKernelExecutor,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_full_write(
        request,
        2,
        ShapeRule::Declared,
        ChangeDetectionPolicy::ExactScalar,
    )?;
    require_kind(
        request,
        &[ResidentValueKind::F64, ResidentValueKind::F64],
        ResidentValueKind::Bool,
    )?;
    if request
        .inputs
        .iter()
        .any(|input| input.shape != ResidentShape::SCALAR)
        || request.output.shape != ResidentShape::SCALAR
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    bound(executor, Vec::<u64>::new().into_boxed_slice())
}

fn bind_f64_equal(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_f64_comparison(request, f64_equal)
}

fn bind_semantic_equal(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_f64_equal(request)
        .or_else(|_| bind_f64_vector_equal(request))
        .or_else(|_| super::text::bind_string_equal(request))
        .or_else(|_| bind_dense_comparison(request, SemanticComparison::Equal))
        .or_else(|_| bind_snapshot_comparison(request, SemanticComparison::Equal))
        .or_else(|_| bind_snapshot_equality(request, snapshot_equal))
}

fn comparison_logical_layout<'a>(
    request: &'a ResidentKernelBindRequest<'_>,
    port: &'a mech_core::ResidentPortLayout,
) -> Result<(&'a SchemaBody, ResidentShape), ResidentKernelBindError> {
    let schema = request
        .schemas
        .get(port.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    match schema.body() {
        SchemaBody::Matrix {
            element,
            dimensions,
        } if dimensions.len() == 2 => {
            let rows = port
                .shape_instance
                .resolve_dimension(&dimensions[0])
                .map_err(|_| ResidentKernelBindError::UnsupportedLayout)?;
            let columns = port
                .shape_instance
                .resolve_dimension(&dimensions[1])
                .map_err(|_| ResidentKernelBindError::UnsupportedLayout)?;
            Ok((
                element,
                ResidentShape {
                    rows: u32::try_from(rows)
                        .map_err(|_| ResidentKernelBindError::UnsupportedLayout)?,
                    columns: u32::try_from(columns)
                        .map_err(|_| ResidentKernelBindError::UnsupportedLayout)?,
                },
            ))
        }
        body => Ok((body, ResidentShape::SCALAR)),
    }
}

fn validate_semantic_comparison_contract(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<ResidentShape, ResidentKernelBindError> {
    let (output_element, logical_output) = comparison_logical_layout(request, &request.output)?;
    if output_element != &SchemaBody::Bool
        || request.output.kind != ResidentValueKind::Bool
        || request.output.shape != logical_output
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let output_schema = request
        .schemas
        .get(request.output.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let change_detection = if matches!(output_schema.body(), SchemaBody::Bool) {
        ChangeDetectionPolicy::ExactScalar
    } else {
        ChangeDetectionPolicy::KernelReported
    };
    validate_full_write(request, 2, ShapeRule::Declared, change_detection)?;
    Ok(logical_output)
}

fn admit_dense_comparison_layout(output: ResidentShape) -> Result<(), ResidentKernelError> {
    let output_elements = output.len().ok_or(ResidentKernelError::InvalidShape)?;
    super::budget::PreparedKernel::new(
        (),
        super::budget::resident_cost! {
            comparison_work: output_elements,
            compute_work: output_elements,
            output_elements,
            output_bytes: output_elements,
            ..super::budget::KernelCostEstimate::default()
        },
    )
    .admit()?
    .into_plan();
    Ok(())
}

fn bind_dense_comparison(
    request: &ResidentKernelBindRequest<'_>,
    comparison: SemanticComparison,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let output = validate_semantic_comparison_contract(request)?;
    let [left, right] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    if left.kind != right.kind
        || !matches!(
            left.kind,
            ResidentValueKind::Bool | ResidentValueKind::Index | ResidentValueKind::String
        )
        || (!comparison.is_equality() && !matches!(left.kind, ResidentValueKind::Index))
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let (left_element, left_shape) = comparison_logical_layout(request, left)?;
    let (right_element, right_shape) = comparison_logical_layout(request, right)?;
    let kind_matches_schema = matches!(
        (left.kind, left_element),
        (ResidentValueKind::Bool, SchemaBody::Bool)
            | (ResidentValueKind::Index, SchemaBody::Index)
            | (ResidentValueKind::String, SchemaBody::String)
    );
    if left_element != right_element
        || !kind_matches_schema
        || left.shape != left_shape
        || right.shape != right_shape
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let left_mode = binary_broadcast_mode(left_shape, output)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let right_mode = binary_broadcast_mode(right_shape, output)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    bound(
        dense_comparison,
        vec![
            output.rows as u64,
            output.columns as u64,
            left_mode,
            right_mode,
            comparison as u64,
        ]
        .into_boxed_slice(),
    )
}

fn snapshot_comparison_element_supported(
    element: &SchemaBody,
    comparison: SemanticComparison,
) -> bool {
    if comparison.is_equality() {
        matches!(
            element,
            SchemaBody::Bool
                | SchemaBody::String
                | SchemaBody::UnsignedInteger(_)
                | SchemaBody::SignedInteger(_)
                | SchemaBody::FloatingPoint(_)
                | SchemaBody::Rational64
                | SchemaBody::Complex(_)
        )
    } else {
        matches!(
            element,
            SchemaBody::UnsignedInteger(_)
                | SchemaBody::SignedInteger(_)
                | SchemaBody::FloatingPoint(_)
                | SchemaBody::Rational64
                | SchemaBody::Complex(_)
        )
    }
}

fn bind_snapshot_comparison(
    request: &ResidentKernelBindRequest<'_>,
    comparison: SemanticComparison,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let output = validate_semantic_comparison_contract(request)?;
    let [left, right] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    if left.kind != ResidentValueKind::Snapshot
        || right.kind != ResidentValueKind::Snapshot
        || left.shape != ResidentShape::SCALAR
        || right.shape != ResidentShape::SCALAR
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let (left_element, left_shape) = comparison_logical_layout(request, left)?;
    let (right_element, right_shape) = comparison_logical_layout(request, right)?;
    if left_element != right_element
        || !snapshot_comparison_element_supported(left_element, comparison)
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let left_mode = binary_broadcast_mode(left_shape, output)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let right_mode = binary_broadcast_mode(right_shape, output)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    Ok(bound(
        snapshot_comparison,
        vec![
            output.rows as u64,
            output.columns as u64,
            left_mode,
            right_mode,
            comparison as u64,
        ]
        .into_boxed_slice(),
    )?
    .with_snapshot_schemas(request.schemas.clone()))
}

fn bind_snapshot_equality(
    request: &ResidentKernelBindRequest<'_>,
    executor: mech_core::ResidentKernelExecutor,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_full_write(
        request,
        2,
        ShapeRule::Declared,
        ChangeDetectionPolicy::ExactScalar,
    )?;
    require_kind(
        request,
        &[ResidentValueKind::Snapshot, ResidentValueKind::Snapshot],
        ResidentValueKind::Bool,
    )?;
    if request.output.shape != ResidentShape::SCALAR
        || request
            .inputs
            .iter()
            .any(|input| input.shape != ResidentShape::SCALAR)
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    Ok(bound(executor, Vec::<u64>::new())?.with_snapshot_schemas(request.schemas.clone()))
}

fn bind_semantic_not_equal(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_f64_not_equal(request)
        .or_else(|_| bind_f64_vector_not_equal(request))
        .or_else(|_| super::text::bind_string_not_equal(request))
        .or_else(|_| bind_dense_comparison(request, SemanticComparison::NotEqual))
        .or_else(|_| bind_snapshot_comparison(request, SemanticComparison::NotEqual))
        .or_else(|_| bind_snapshot_equality(request, snapshot_not_equal))
}

fn bind_f64_vector_equal(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_f64_vector_comparison(request, f64_vector_equal)
}

fn bind_f64_vector_not_equal(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_f64_vector_comparison(request, f64_vector_not_equal)
}

fn bind_f64_vector_comparison(
    request: &ResidentKernelBindRequest<'_>,
    executor: mech_core::ResidentKernelExecutor,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_full_write(
        request,
        2,
        ShapeRule::Declared,
        ChangeDetectionPolicy::KernelReported,
    )?;
    require_kind(
        request,
        &[ResidentValueKind::F64, ResidentValueKind::F64],
        ResidentValueKind::Bool,
    )?;
    request
        .output
        .shape
        .len()
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let modes = request
        .inputs
        .iter()
        .map(|input| binary_broadcast_mode(input.shape, request.output.shape))
        .collect::<Option<Vec<_>>>()
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    bound(
        executor,
        vec![
            request.output.shape.rows as u64,
            request.output.shape.columns as u64,
            modes[0],
            modes[1],
        ]
        .into_boxed_slice(),
    )
}

fn bind_f64_not_equal(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_f64_comparison(request, f64_not_equal)
}

fn bind_f64_less(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_f64_comparison(request, f64_less)
}

fn bind_semantic_less(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_f64_less(request)
        .or_else(|_| bind_f64_vector_comparison(request, f64_vector_less))
        .or_else(|_| bind_snapshot_comparison(request, SemanticComparison::Less))
}

fn bind_f64_less_equal(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_f64_comparison(request, f64_less_equal)
}

fn bind_semantic_less_equal(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_f64_less_equal(request)
        .or_else(|_| bind_f64_vector_comparison(request, f64_vector_less_equal))
        .or_else(|_| bind_snapshot_comparison(request, SemanticComparison::LessEqual))
}

fn bind_f64_greater(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_f64_comparison(request, f64_greater)
}

fn bind_semantic_greater(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_f64_greater(request)
        .or_else(|_| bind_f64_vector_comparison(request, f64_vector_greater))
        .or_else(|_| bind_snapshot_comparison(request, SemanticComparison::Greater))
}

fn bind_f64_greater_equal(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_f64_comparison(request, f64_greater_equal)
}

fn bind_semantic_greater_equal(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_f64_greater_equal(request)
        .or_else(|_| bind_f64_vector_comparison(request, f64_vector_greater_equal))
        .or_else(|_| bind_snapshot_comparison(request, SemanticComparison::GreaterEqual))
}

fn bind_bool_binary(
    request: &ResidentKernelBindRequest<'_>,
    executor: mech_core::ResidentKernelExecutor,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_full_write(
        request,
        2,
        ShapeRule::Declared,
        ChangeDetectionPolicy::ExactScalar,
    )?;
    require_kind(
        request,
        &[ResidentValueKind::Bool, ResidentValueKind::Bool],
        ResidentValueKind::Bool,
    )?;
    if request
        .inputs
        .iter()
        .any(|input| input.shape != ResidentShape::SCALAR)
        || request.output.shape != ResidentShape::SCALAR
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    bound(executor, Vec::<u64>::new().into_boxed_slice())
}

fn bind_bool_and(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_bool_binary(request, bool_and)
}

fn bind_semantic_bool_and(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_bool_and(request).or_else(|_| bind_bool_vector_and(request))
}

fn bind_bool_vector_and(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_bool_vector_binary(request, bool_vector_and)
}

fn bind_bool_vector_binary(
    request: &ResidentKernelBindRequest<'_>,
    executor: mech_core::ResidentKernelExecutor,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_full_write(
        request,
        2,
        ShapeRule::Declared,
        ChangeDetectionPolicy::KernelReported,
    )?;
    require_kind(
        request,
        &[ResidentValueKind::Bool, ResidentValueKind::Bool],
        ResidentValueKind::Bool,
    )?;
    request
        .output
        .shape
        .len()
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let modes = request
        .inputs
        .iter()
        .map(|input| binary_broadcast_mode(input.shape, request.output.shape))
        .collect::<Option<Vec<_>>>()
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    bound(
        executor,
        vec![
            request.output.shape.rows as u64,
            request.output.shape.columns as u64,
            modes[0],
            modes[1],
        ]
        .into_boxed_slice(),
    )
}

fn bind_bool_or(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_bool_binary(request, bool_or)
}

fn bind_semantic_bool_or(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_bool_or(request).or_else(|_| bind_bool_vector_binary(request, bool_vector_or))
}

fn bind_bool_xor(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_bool_binary(request, bool_xor)
}

fn bind_semantic_bool_xor(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_bool_xor(request).or_else(|_| bind_bool_vector_binary(request, bool_vector_xor))
}

fn bind_bool_not(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let [input] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let Some(input_schema) = request.schemas.get(input.schema_id) else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let Some(output_schema) = request.schemas.get(request.output.schema_id) else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    if input_schema.body() != output_schema.body() {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let scalar = match output_schema.body() {
        SchemaBody::Bool => true,
        SchemaBody::Matrix { element, .. } if element.as_ref() == &SchemaBody::Bool => false,
        _ => return Err(ResidentKernelBindError::UnsupportedLayout),
    };
    validate_full_write(
        request,
        1,
        ShapeRule::Declared,
        if scalar {
            ChangeDetectionPolicy::ExactScalar
        } else {
            ChangeDetectionPolicy::KernelReported
        },
    )?;
    require_kind(request, &[ResidentValueKind::Bool], ResidentValueKind::Bool)?;
    if request.inputs[0].shape != request.output.shape {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    bound(
        if scalar { bool_not } else { bool_vector_not },
        Vec::<u64>::new().into_boxed_slice(),
    )
}

fn bind_strict_comparison(
    request: &ResidentKernelBindRequest<'_>,
    executor: mech_core::ResidentKernelExecutor,
    mismatch_executor: mech_core::ResidentKernelExecutor,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_full_write(
        request,
        2,
        ShapeRule::Declared,
        ChangeDetectionPolicy::ExactScalar,
    )?;
    if request.output.kind != ResidentValueKind::Bool
        || request.output.shape != ResidentShape::SCALAR
        || request.inputs.len() != 2
        || request.inputs.iter().any(|input| {
            input.shape.len().is_none()
                || (input.kind == ResidentValueKind::Snapshot
                    && input.shape != ResidentShape::SCALAR)
        })
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    if !strict_inputs_share_identity(request) {
        return bound(mismatch_executor, Vec::<u64>::new().into_boxed_slice());
    }
    let kernel = bound(executor, Vec::<u64>::new().into_boxed_slice())?;
    if request
        .inputs
        .iter()
        .any(|input| input.kind == ResidentValueKind::Snapshot)
    {
        Ok(kernel.with_snapshot_schemas(request.schemas.clone()))
    } else {
        Ok(kernel)
    }
}

fn strict_inputs_share_identity(request: &ResidentKernelBindRequest<'_>) -> bool {
    let [left, right] = request.inputs else {
        return false;
    };
    if left.kind != right.kind || left.shape != right.shape {
        return false;
    }
    if left.schema_id == right.schema_id {
        return true;
    }
    matches!(
        (
            request.schemas.get(left.schema_id).map(|schema| schema.body()),
            request.schemas.get(right.schema_id).map(|schema| schema.body()),
        ),
        (
            Some(SchemaBody::Matrix { element: left, .. }),
            Some(SchemaBody::Matrix { element: right, .. }),
        ) if left == right
    )
}

fn bind_strict_equal(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_strict_comparison(request, strict_equal, strict_always_false)
}

fn bind_strict_not_equal(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_strict_comparison(request, strict_not_equal, strict_always_true)
}

fn bind_binary(
    request: &ResidentKernelBindRequest<'_>,
    executor: mech_core::ResidentKernelExecutor,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let change = f64_output_change_detection(request)?;
    validate_full_write(request, 2, ShapeRule::Declared, change)?;
    require_kind(
        request,
        &[ResidentValueKind::F64, ResidentValueKind::F64],
        ResidentValueKind::F64,
    )?;
    let rows = request.output.shape.rows;
    let columns = request.output.shape.columns;
    if request.output.shape.len().is_none() {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let modes = request
        .inputs
        .iter()
        .map(|input| binary_broadcast_mode(input.shape, request.output.shape))
        .collect::<Option<Vec<_>>>()
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    bound(
        executor,
        vec![rows as u64, columns as u64, modes[0], modes[1]].into_boxed_slice(),
    )
}

fn snapshot_arithmetic_element_supported(
    arithmetic: SemanticArithmetic,
    element: &SchemaBody,
) -> bool {
    use mech_core::{FloatWidth, IntegerWidth};
    match arithmetic {
        SemanticArithmetic::Add
        | SemanticArithmetic::Subtract
        | SemanticArithmetic::Multiply
        | SemanticArithmetic::Divide => {
            matches!(
                element,
                SchemaBody::UnsignedInteger(_)
                    | SchemaBody::SignedInteger(_)
                    | SchemaBody::FloatingPoint(FloatWidth::W32 | FloatWidth::W64)
            ) || (cfg!(feature = "r64") && matches!(element, SchemaBody::Rational64))
                || (cfg!(feature = "c64")
                    && matches!(element, SchemaBody::Complex(FloatWidth::W64)))
        }
        SemanticArithmetic::Remainder => matches!(
            element,
            SchemaBody::UnsignedInteger(_)
                | SchemaBody::SignedInteger(_)
                | SchemaBody::FloatingPoint(FloatWidth::W32 | FloatWidth::W64)
        ),
        SemanticArithmetic::Power => matches!(
            element,
            SchemaBody::UnsignedInteger(IntegerWidth::W8 | IntegerWidth::W16 | IntegerWidth::W32)
                | SchemaBody::FloatingPoint(FloatWidth::W32 | FloatWidth::W64)
        ),
    }
}

fn snapshot_negate_element_supported(element: &SchemaBody) -> bool {
    use mech_core::FloatWidth;
    matches!(
        element,
        SchemaBody::SignedInteger(_) | SchemaBody::FloatingPoint(FloatWidth::W32 | FloatWidth::W64)
    ) || (cfg!(feature = "r64") && matches!(element, SchemaBody::Rational64))
        || (cfg!(feature = "c64") && matches!(element, SchemaBody::Complex(FloatWidth::W64)))
}

fn snapshot_abs_element_supported(element: &SchemaBody) -> bool {
    use mech_core::FloatWidth;
    matches!(
        element,
        SchemaBody::UnsignedInteger(_)
            | SchemaBody::SignedInteger(_)
            | SchemaBody::FloatingPoint(FloatWidth::W32 | FloatWidth::W64)
    ) || (cfg!(feature = "r64") && matches!(element, SchemaBody::Rational64))
        || (cfg!(feature = "c64") && matches!(element, SchemaBody::Complex(FloatWidth::W64)))
}

fn snapshot_output_change_detection(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<ChangeDetectionPolicy, ResidentKernelBindError> {
    let output_schema = request
        .schemas
        .get(request.output.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    Ok(
        if matches!(output_schema.body(), SchemaBody::Matrix { .. }) {
            ChangeDetectionPolicy::KernelReported
        } else {
            ChangeDetectionPolicy::ExactScalar
        },
    )
}

fn snapshot_output_metadata(request: &ResidentKernelBindRequest<'_>) -> ResidentSnapshotOutput {
    ResidentSnapshotOutput {
        schema: request.output.schema_id,
        schema_key: request.output.schema_key,
        shape: request.output.shape_instance.clone(),
        exact_cardinality: None,
        maximum_cardinality: None,
    }
}

fn bind_snapshot_numeric_binary(
    request: &ResidentKernelBindRequest<'_>,
    arithmetic: SemanticArithmetic,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_full_write(
        request,
        2,
        ShapeRule::Declared,
        snapshot_output_change_detection(request)?,
    )?;
    let [left, right] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    if left.kind != ResidentValueKind::Snapshot
        || right.kind != ResidentValueKind::Snapshot
        || request.output.kind != ResidentValueKind::Snapshot
        || left.shape != ResidentShape::SCALAR
        || right.shape != ResidentShape::SCALAR
        || request.output.shape != ResidentShape::SCALAR
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let (left_element, left_shape) = comparison_logical_layout(request, left)?;
    let (right_element, right_shape) = comparison_logical_layout(request, right)?;
    let (output_element, output_shape) = comparison_logical_layout(request, &request.output)?;
    let rational_power = cfg!(feature = "r64")
        && arithmetic == SemanticArithmetic::Power
        && left_shape == ResidentShape::SCALAR
        && right_shape == ResidentShape::SCALAR
        && output_shape == ResidentShape::SCALAR
        && left_element == &SchemaBody::Rational64
        && right_element == &SchemaBody::SignedInteger(mech_core::IntegerWidth::W32)
        && output_element == &SchemaBody::Rational64;
    if !rational_power
        && (left_element != right_element
            || left_element != output_element
            || !snapshot_arithmetic_element_supported(arithmetic, output_element))
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let left_mode = binary_broadcast_mode(left_shape, output_shape)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let right_mode = binary_broadcast_mode(right_shape, output_shape)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    Ok(bound(
        snapshot_numeric_binary,
        vec![
            output_shape.rows as u64,
            output_shape.columns as u64,
            left_mode,
            right_mode,
            arithmetic as u64,
            u64::from(rational_power),
        ]
        .into_boxed_slice(),
    )?
    .with_snapshot_output(snapshot_output_metadata(request))
    .with_snapshot_schemas(request.schemas.clone()))
}

fn bind_snapshot_numeric_negate(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_full_write(
        request,
        1,
        ShapeRule::SameAsInput { input: 0 },
        snapshot_output_change_detection(request)?,
    )?;
    let [input] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    if input.kind != ResidentValueKind::Snapshot
        || request.output.kind != ResidentValueKind::Snapshot
        || input.shape != ResidentShape::SCALAR
        || request.output.shape != ResidentShape::SCALAR
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let (input_element, input_shape) = comparison_logical_layout(request, input)?;
    let (output_element, output_shape) = comparison_logical_layout(request, &request.output)?;
    if input_element != output_element
        || input_shape != output_shape
        || !snapshot_negate_element_supported(output_element)
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    Ok(bound(
        snapshot_numeric_negate,
        vec![output_shape.rows as u64, output_shape.columns as u64].into_boxed_slice(),
    )?
    .with_snapshot_output(snapshot_output_metadata(request))
    .with_snapshot_schemas(request.schemas.clone()))
}

fn bind_snapshot_numeric_abs(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_full_write(
        request,
        1,
        ShapeRule::SameAsInput { input: 0 },
        snapshot_output_change_detection(request)?,
    )?;
    let [input] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    if input.kind != ResidentValueKind::Snapshot
        || request.output.kind != ResidentValueKind::Snapshot
        || input.shape != ResidentShape::SCALAR
        || request.output.shape != ResidentShape::SCALAR
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let (input_element, input_shape) = comparison_logical_layout(request, input)?;
    let (output_element, output_shape) = comparison_logical_layout(request, &request.output)?;
    if input_element != output_element
        || input_shape != output_shape
        || !snapshot_abs_element_supported(output_element)
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    Ok(bound(
        snapshot_numeric_abs,
        vec![output_shape.rows as u64, output_shape.columns as u64].into_boxed_slice(),
    )?
    .with_snapshot_output(snapshot_output_metadata(request))
    .with_snapshot_schemas(request.schemas.clone()))
}

const BINARY_BROADCAST_SCALAR: u64 = 0;
const BINARY_BROADCAST_EXACT: u64 = 1;
const BINARY_BROADCAST_COLUMN: u64 = 2;
const BINARY_BROADCAST_ROW: u64 = 3;

fn binary_broadcast_mode(input: ResidentShape, output: ResidentShape) -> Option<u64> {
    if input == output {
        Some(BINARY_BROADCAST_EXACT)
    } else if input.len() == Some(1) {
        Some(BINARY_BROADCAST_SCALAR)
    } else if input.rows == output.rows && input.columns == 1 {
        Some(BINARY_BROADCAST_COLUMN)
    } else if input.rows == 1 && input.columns == output.columns {
        Some(BINARY_BROADCAST_ROW)
    } else {
        None
    }
}

fn f64_output_change_detection(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<ChangeDetectionPolicy, ResidentKernelBindError> {
    let Some(schema) = request.schemas.get(request.output.schema_id) else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    match schema.body() {
        SchemaBody::FloatingPoint(mech_core::FloatWidth::W64) => {
            Ok(ChangeDetectionPolicy::ExactScalar)
        }
        SchemaBody::Matrix { element, .. }
            if matches!(
                element.as_ref(),
                SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)
            ) =>
        {
            Ok(ChangeDetectionPolicy::KernelReported)
        }
        _ => Err(ResidentKernelBindError::UnsupportedLayout),
    }
}

fn bind_mul_rows(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_full_write(
        request,
        2,
        ShapeRule::Declared,
        ChangeDetectionPolicy::KernelReported,
    )?;
    require_kind(
        request,
        &[ResidentValueKind::F64, ResidentValueKind::F64],
        ResidentValueKind::F64,
    )?;
    let [matrix, vector] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    if matrix.shape != request.output.shape
        || vector.shape.len() != Some(request.output.shape.rows as usize)
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    bound(
        multiply_rows,
        vec![
            request.output.shape.rows as u64,
            request.output.shape.columns as u64,
        ]
        .into_boxed_slice(),
    )
}

fn bind_pow(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_binary(request, power)
        .or_else(|_| bind_snapshot_numeric_binary(request, SemanticArithmetic::Power))
}

fn bind_add_assign(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_rmw(request, 2, RegionPolicy::WholeValue)?;
    require_kind(
        request,
        &[ResidentValueKind::F64, ResidentValueKind::F64],
        ResidentValueKind::F64,
    )?;
    if request.inputs[0].shape != request.output.shape
        || request.inputs[1].shape != request.output.shape
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    bound(add_assign, Vec::<u64>::new().into_boxed_slice())
}

fn bind_semantic_add_assign(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_add_assign(request).or_else(|_| bind_add_indexed_rows(request))
}

fn bind_transpose(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_full_write(
        request,
        1,
        ShapeRule::TransposeOf { input: 0 },
        ChangeDetectionPolicy::KernelReported,
    )?;
    let [input] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    if matches!(
        input.kind,
        ResidentValueKind::Bool | ResidentValueKind::Index | ResidentValueKind::F64
    ) && input.kind == request.output.kind
        && request.output.shape.rows == input.shape.columns
        && request.output.shape.columns == input.shape.rows
    {
        return bound(
            transpose_dense,
            vec![input.shape.rows as u64, input.shape.columns as u64].into_boxed_slice(),
        );
    }
    let input_schema = request
        .schemas
        .get(input.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let output_schema = request
        .schemas
        .get(request.output.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let (
        SchemaBody::Matrix {
            element: input_element,
            ..
        },
        SchemaBody::Matrix {
            element: output_element,
            ..
        },
    ) = (input_schema.body(), output_schema.body())
    else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let (rows, columns) = declared_matrix_dimensions(request, input)?;
    let output_dimensions = declared_matrix_dimensions(request, &request.output)?;
    if input.kind != ResidentValueKind::Snapshot
        || request.output.kind != ResidentValueKind::Snapshot
        || input.shape != ResidentShape::SCALAR
        || request.output.shape != ResidentShape::SCALAR
        || input_element != output_element
        || !is_transpose_snapshot_schema(input_element)
        || output_dimensions != (columns, rows)
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    Ok(bound(
        transpose_snapshot,
        vec![rows as u64, columns as u64].into_boxed_slice(),
    )?
    .with_snapshot_output(ResidentSnapshotOutput {
        schema: request.output.schema_id,
        schema_key: request.output.schema_key,
        shape: request.output.shape_instance.clone(),
        exact_cardinality: None,
        maximum_cardinality: None,
    })
    .with_snapshot_schemas(request.schemas.clone()))
}

fn admit_dense_transpose_layout(
    kind: ResidentValueKind,
    shape: ResidentShape,
) -> Result<(), ResidentKernelError> {
    let elements = shape.len().ok_or(ResidentKernelError::InvalidShape)?;
    let element_bytes = match kind {
        ResidentValueKind::Bool => core::mem::size_of::<u8>(),
        ResidentValueKind::Index => core::mem::size_of::<u64>(),
        ResidentValueKind::F64 => core::mem::size_of::<f64>(),
        _ => return Err(ResidentKernelError::InvalidShape),
    };
    let output_bytes = elements
        .checked_mul(element_bytes)
        .ok_or(ResidentKernelError::InvalidShape)?;
    super::budget::PreparedKernel::new(
        (),
        super::budget::resident_cost! {
            compute_work: elements,
            output_elements: elements,
            output_bytes,
            ..super::budget::KernelCostEstimate::default()
        },
    )
    .admit()?
    .into_plan();
    Ok(())
}

fn is_transpose_snapshot_schema(body: &SchemaBody) -> bool {
    matches!(
        body,
        SchemaBody::UnsignedInteger(_)
            | SchemaBody::SignedInteger(_)
            | SchemaBody::FloatingPoint(mech_core::FloatWidth::W32)
            | SchemaBody::Complex(_)
            | SchemaBody::Rational64
    )
}

fn bind_semantic_transpose(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_transpose(request).or_else(|_| super::text::bind_string_transpose(request))
}

fn bind_sum_columns(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_full_write(
        request,
        1,
        ShapeRule::Declared,
        ChangeDetectionPolicy::KernelReported,
    )?;
    let [input] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    if input.kind == ResidentValueKind::F64 && request.output.kind == ResidentValueKind::F64 {
        if request.output.shape.len() != Some(input.shape.rows as usize) {
            return Err(ResidentKernelBindError::UnsupportedLayout);
        }
        return bound(
            sum_columns,
            vec![input.shape.rows as u64, input.shape.columns as u64].into_boxed_slice(),
        );
    }
    bind_snapshot_sum(request, true)
}

fn bind_sum_rows(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_full_write(
        request,
        1,
        ShapeRule::Declared,
        ChangeDetectionPolicy::KernelReported,
    )?;
    let [input] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    if input.kind == ResidentValueKind::F64 && request.output.kind == ResidentValueKind::F64 {
        if request.output.shape.len() != Some(input.shape.columns as usize) {
            return Err(ResidentKernelBindError::UnsupportedLayout);
        }
        return bound(
            sum_rows,
            vec![input.shape.rows as u64, input.shape.columns as u64].into_boxed_slice(),
        );
    }
    bind_snapshot_sum(request, false)
}

fn is_snapshot_sum_element(body: &SchemaBody) -> bool {
    match body {
        SchemaBody::UnsignedInteger(_)
        | SchemaBody::SignedInteger(_)
        | SchemaBody::FloatingPoint(mech_core::FloatWidth::W32 | mech_core::FloatWidth::W64) => {
            true
        }
        SchemaBody::Rational64 => cfg!(feature = "r64"),
        SchemaBody::Complex(mech_core::FloatWidth::W64) => cfg!(feature = "c64"),
        _ => false,
    }
}

fn bind_snapshot_sum(
    request: &ResidentKernelBindRequest<'_>,
    column: bool,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let [input] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    if input.kind != ResidentValueKind::Snapshot
        || request.output.kind != ResidentValueKind::Snapshot
        || input.shape != ResidentShape::SCALAR
        || request.output.shape != ResidentShape::SCALAR
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let input_schema = request
        .schemas
        .get(input.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let output_schema = request
        .schemas
        .get(request.output.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let (
        SchemaBody::Matrix {
            element: input_element,
            ..
        },
        SchemaBody::Matrix {
            element: output_element,
            ..
        },
    ) = (input_schema.body(), output_schema.body())
    else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let (rows, columns) = declared_matrix_dimensions(request, input)?;
    let output_dimensions = declared_matrix_dimensions(request, &request.output)?;
    let expected = if column { (rows, 1) } else { (1, columns) };
    if input_element != output_element
        || !is_snapshot_sum_element(input_element)
        || output_dimensions != expected
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    Ok(bound(
        sum_snapshot,
        vec![rows as u64, columns as u64, u64::from(column)].into_boxed_slice(),
    )?
    .with_snapshot_output(ResidentSnapshotOutput {
        schema: request.output.schema_id,
        schema_key: request.output.schema_key,
        shape: request.output.shape_instance.clone(),
        exact_cardinality: None,
        maximum_cardinality: None,
    })
    .with_snapshot_schemas(request.schemas.clone()))
}

fn bind_horizontal(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_build(
        request,
        request.inputs.len(),
        &["matrix", "concatenate"],
        "horizontal-output",
    )?;
    bind_matrix_constructor(request, true)
}

fn bind_matrix_comprehension(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_build(
        request,
        request.inputs.len(),
        &["matrix", "concatenate"],
        "horizontal-output",
    )?;
    bind_matrix_constructor(request, true)
}

fn constructor_input_dimensions(
    request: &ResidentKernelBindRequest<'_>,
    input: &mech_core::ResidentPortLayout,
    output_element: &SchemaBody,
) -> Result<(usize, usize), ResidentKernelBindError> {
    let schema = request
        .schemas
        .get(input.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    match schema.body() {
        body if body == output_element => Ok((1, 1)),
        SchemaBody::Matrix { element, .. } if element.as_ref() == output_element => {
            declared_matrix_dimensions(request, input)
        }
        _ => Err(ResidentKernelBindError::UnsupportedLayout),
    }
}

fn bind_matrix_constructor(
    request: &ResidentKernelBindRequest<'_>,
    horizontal: bool,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let output_schema = request
        .schemas
        .get(request.output.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let SchemaBody::Matrix {
        element: output_element,
        ..
    } = output_schema.body()
    else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let output_dimensions = declared_matrix_dimensions(request, &request.output)?;
    let dimensions = request
        .inputs
        .iter()
        .map(|input| constructor_input_dimensions(request, input, output_element))
        .collect::<Result<Vec<_>, _>>()?;
    let expected = if horizontal {
        let rows = dimensions
            .first()
            .map_or(output_dimensions.0, |shape| shape.0);
        if dimensions.iter().any(|shape| shape.0 != rows) {
            return Err(ResidentKernelBindError::UnsupportedLayout);
        }
        (
            rows,
            dimensions.iter().try_fold(0usize, |sum, shape| {
                sum.checked_add(shape.1)
                    .ok_or(ResidentKernelBindError::UnsupportedLayout)
            })?,
        )
    } else {
        let columns = dimensions
            .first()
            .map_or(output_dimensions.1, |shape| shape.1);
        if dimensions.iter().any(|shape| shape.1 != columns) {
            return Err(ResidentKernelBindError::UnsupportedLayout);
        }
        (
            dimensions.iter().try_fold(0usize, |sum, shape| {
                sum.checked_add(shape.0)
                    .ok_or(ResidentKernelBindError::UnsupportedLayout)
            })?,
            columns,
        )
    };
    if expected != output_dimensions {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let mut parameters = Vec::with_capacity(1 + dimensions.len() * 2);
    parameters.push(dimensions.len() as u64);
    for (rows, columns) in dimensions {
        parameters.push(rows as u64);
        parameters.push(columns as u64);
    }
    let dense_kind = matches!(
        request.output.kind,
        ResidentValueKind::Bool
            | ResidentValueKind::Index
            | ResidentValueKind::F64
            | ResidentValueKind::String
    );
    let dense_output_shape_matches = usize::try_from(request.output.shape.rows).ok()
        == Some(output_dimensions.0)
        && usize::try_from(request.output.shape.columns).ok() == Some(output_dimensions.1);
    if dense_kind
        && request.inputs.iter().all(|input| {
            input.kind == request.output.kind
                && input.shape.len().is_some()
                && input.kind != ResidentValueKind::Snapshot
        })
        && dense_output_shape_matches
    {
        return bound(
            if horizontal {
                concatenate_horizontal
            } else {
                concatenate_vertical
            },
            parameters.into_boxed_slice(),
        );
    }
    if request.output.kind != ResidentValueKind::Snapshot
        || request.output.shape != ResidentShape::SCALAR
        || request.inputs.iter().any(|input| {
            input.kind != ResidentValueKind::Snapshot || input.shape != ResidentShape::SCALAR
        })
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    Ok(bound(
        if horizontal {
            concatenate_horizontal_snapshot
        } else {
            concatenate_vertical_snapshot
        },
        parameters.into_boxed_slice(),
    )?
    .with_snapshot_output(ResidentSnapshotOutput {
        schema: request.output.schema_id,
        schema_key: request.output.schema_key,
        shape: request.output.shape_instance.clone(),
        exact_cardinality: None,
        maximum_cardinality: None,
    })
    .with_snapshot_schemas(request.schemas.clone()))
}

fn bind_indexed_assign(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_indexed_assign_with_region(request, RegionPolicy::IndexedAxis { axis: 0 })
}

fn bind_collection_entry_assign(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_indexed_assign_with_region(request, RegionPolicy::CollectionEntry)
}

fn bind_single_element_assign(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_indexed_assign_with_region(request, RegionPolicy::SingleElement)
}

fn scalar_layout_matches_schema_body(
    layout: &mech_core::ResidentPortLayout,
    body: &SchemaBody,
) -> bool {
    if layout.shape != ResidentShape::SCALAR {
        return false;
    }
    match body {
        SchemaBody::Bool => layout.kind == ResidentValueKind::Bool,
        SchemaBody::Index => layout.kind == ResidentValueKind::Index,
        SchemaBody::FloatingPoint(mech_core::FloatWidth::W64) => {
            layout.kind == ResidentValueKind::F64
        }
        SchemaBody::String => layout.kind == ResidentValueKind::String,
        _ => layout.kind == ResidentValueKind::Snapshot,
    }
}

fn is_positional_selector_schema(body: &SchemaBody) -> bool {
    mech_core::is_positional_selector_schema(body)
}

fn is_access_positional_selector_schema(body: &SchemaBody) -> bool {
    match body {
        SchemaBody::Bool => true,
        body if is_positional_selector_schema(body) => true,
        SchemaBody::Matrix { element, .. } => {
            matches!(element.as_ref(), SchemaBody::Bool)
                || is_positional_selector_schema(element.as_ref())
        }
        _ => false,
    }
}

fn is_logical_selector_schema(body: &SchemaBody) -> bool {
    matches!(body, SchemaBody::Bool)
        || matches!(body, SchemaBody::Matrix { element, .. } if element.as_ref() == &SchemaBody::Bool)
}

fn access_selector_layout_matches_schema(
    layout: &mech_core::ResidentPortLayout,
    body: &SchemaBody,
) -> bool {
    if layout.kind == ResidentValueKind::Snapshot {
        return layout.shape == ResidentShape::SCALAR;
    }
    match body {
        SchemaBody::Matrix { element, .. } => match element.as_ref() {
            SchemaBody::Bool => layout.kind == ResidentValueKind::Bool,
            SchemaBody::Index => layout.kind == ResidentValueKind::Index,
            SchemaBody::FloatingPoint(mech_core::FloatWidth::W64) => {
                layout.kind == ResidentValueKind::F64
            }
            SchemaBody::String => layout.kind == ResidentValueKind::String,
            _ => false,
        },
        _ => scalar_layout_matches_schema_body(layout, body),
    }
}

fn is_numeric_positional_selector_schema(body: &SchemaBody) -> bool {
    match body {
        body if is_positional_selector_schema(body) => true,
        SchemaBody::Matrix { element, .. } => is_positional_selector_schema(element.as_ref()),
        _ => false,
    }
}

pub(super) fn numeric_positional_selector_layout(
    request: &ResidentKernelBindRequest<'_>,
    layout: &mech_core::ResidentPortLayout,
) -> bool {
    request.schemas.get(layout.schema_id).is_some_and(|schema| {
        is_numeric_positional_selector_schema(schema.body())
            && access_selector_layout_matches_schema(layout, schema.body())
    })
}

fn positional_selector_layout(
    request: &ResidentKernelBindRequest<'_>,
    layout: &mech_core::ResidentPortLayout,
) -> bool {
    request.schemas.get(layout.schema_id).is_some_and(|schema| {
        is_access_positional_selector_schema(schema.body())
            && access_selector_layout_matches_schema(layout, schema.body())
    })
}

pub(super) fn declared_selector_cardinality(
    request: &ResidentKernelBindRequest<'_>,
    layout: &mech_core::ResidentPortLayout,
) -> Result<usize, ResidentKernelBindError> {
    let schema = request
        .schemas
        .get(layout.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    match schema.body() {
        SchemaBody::Matrix { .. } => {
            let (rows, columns) = declared_matrix_dimensions(request, layout)?;
            rows.checked_mul(columns)
                .ok_or(ResidentKernelBindError::UnsupportedLayout)
        }
        _ => layout
            .shape
            .len()
            .ok_or(ResidentKernelBindError::UnsupportedLayout),
    }
}

fn resident_shape_from_dimensions(
    rows: usize,
    columns: usize,
) -> Result<ResidentShape, ResidentKernelBindError> {
    Ok(ResidentShape {
        rows: u32::try_from(rows).map_err(|_| ResidentKernelBindError::UnsupportedLayout)?,
        columns: u32::try_from(columns).map_err(|_| ResidentKernelBindError::UnsupportedLayout)?,
    })
}

fn validate_snapshot_access_geometry(
    request: &ResidentKernelBindRequest<'_>,
    source_dimensions: Option<(usize, usize)>,
    output_dimensions: Option<(usize, usize)>,
    selection_mode: Option<ResolvedSelectionMode>,
    selector_layouts: &[mech_core::ResidentPortLayout],
    table_rows: Option<&mech_core::CardinalitySpec>,
) -> Result<ResolvedAccessGeometry, ResidentKernelBindError> {
    let selector = |index: usize| {
        let layout = selector_layouts
            .get(index)
            .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
        let schema = request
            .schemas
            .get(layout.schema_id)
            .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
        Ok((
            declared_selector_cardinality(request, layout)?,
            is_logical_selector_schema(schema.body()),
        ))
    };
    let geometry = if let Some(table_rows) = table_rows {
        if source_dimensions.is_some() || selection_mode.is_some() || selector_layouts.len() != 1 {
            return Err(ResidentKernelBindError::UnsupportedLayout);
        }
        let (output_rows, output_columns) =
            output_dimensions.ok_or(ResidentKernelBindError::UnsupportedLayout)?;
        if output_columns != 1 {
            return Err(ResidentKernelBindError::UnsupportedLayout);
        }
        let source_shape = &request
            .inputs
            .first()
            .ok_or(ResidentKernelBindError::UnsupportedLayout)?
            .shape_instance;
        match table_rows {
            mech_core::CardinalitySpec::Exact(rows) => {
                let rows = source_shape
                    .resolve_dimension(rows)
                    .ok()
                    .and_then(|value| usize::try_from(value).ok())
                    .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
                if output_rows != rows {
                    return Err(ResidentKernelBindError::UnsupportedLayout);
                }
            }
            mech_core::CardinalitySpec::Dynamic {
                upper_bound: Some(rows),
            } => {
                let maximum_rows = source_shape
                    .resolve_dimension(rows)
                    .ok()
                    .and_then(|value| usize::try_from(value).ok())
                    .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
                if output_rows > maximum_rows {
                    return Err(ResidentKernelBindError::UnsupportedLayout);
                }
            }
            mech_core::CardinalitySpec::Dynamic { upper_bound: None } => {}
        }
        ResolvedAccessGeometry {
            logical_output_rows: output_rows,
            logical_output_columns: 1,
        }
    } else if let (Some((source_rows, source_columns)), Some(mode)) =
        (source_dimensions, selection_mode)
    {
        let source_cardinality = source_rows
            .checked_mul(source_columns)
            .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
        let (first_count, first_logical) = selector(0)?;
        match mode {
            ResolvedSelectionMode::LinearScalar => {
                if output_dimensions.is_some() || first_logical || first_count != 1 {
                    return Err(ResidentKernelBindError::UnsupportedLayout);
                }
                ResolvedAccessGeometry {
                    logical_output_rows: 1,
                    logical_output_columns: 1,
                }
            }
            ResolvedSelectionMode::LinearGather => {
                let (output_rows, output_columns) =
                    output_dimensions.ok_or(ResidentKernelBindError::UnsupportedLayout)?;
                if output_columns != 1
                    || if first_logical {
                        first_count != source_cardinality || output_rows > first_count
                    } else {
                        output_rows != first_count
                    }
                {
                    return Err(ResidentKernelBindError::UnsupportedLayout);
                }
                ResolvedAccessGeometry {
                    logical_output_rows: output_rows,
                    logical_output_columns: 1,
                }
            }
            ResolvedSelectionMode::Rows => {
                let (output_rows, output_columns) =
                    output_dimensions.ok_or(ResidentKernelBindError::UnsupportedLayout)?;
                if output_columns != source_columns
                    || if first_logical {
                        first_count != source_rows || output_rows > first_count
                    } else {
                        output_rows != first_count
                    }
                {
                    return Err(ResidentKernelBindError::UnsupportedLayout);
                }
                ResolvedAccessGeometry {
                    logical_output_rows: output_rows,
                    logical_output_columns: source_columns,
                }
            }
            ResolvedSelectionMode::Columns => {
                let (output_rows, output_columns) =
                    output_dimensions.ok_or(ResidentKernelBindError::UnsupportedLayout)?;
                if output_rows != source_rows
                    || if first_logical {
                        first_count != source_columns || output_columns > first_count
                    } else {
                        output_columns != first_count
                    }
                {
                    return Err(ResidentKernelBindError::UnsupportedLayout);
                }
                ResolvedAccessGeometry {
                    logical_output_rows: source_rows,
                    logical_output_columns: output_columns,
                }
            }
            ResolvedSelectionMode::Rectangle => {
                let (second_count, second_logical) = selector(1)?;
                match output_dimensions {
                    None if !first_logical
                        && !second_logical
                        && first_count == 1
                        && second_count == 1 =>
                    {
                        ResolvedAccessGeometry {
                            logical_output_rows: 1,
                            logical_output_columns: 1,
                        }
                    }
                    Some((output_rows, output_columns)) => {
                        let rows_supported = if first_logical {
                            first_count == source_rows && output_rows <= first_count
                        } else {
                            output_rows == first_count
                        };
                        let columns_supported = if second_logical {
                            second_count == source_columns && output_columns <= second_count
                        } else {
                            output_columns == second_count
                        };
                        if !rows_supported || !columns_supported {
                            return Err(ResidentKernelBindError::UnsupportedLayout);
                        }
                        ResolvedAccessGeometry {
                            logical_output_rows: output_rows,
                            logical_output_columns: output_columns,
                        }
                    }
                    _ => return Err(ResidentKernelBindError::UnsupportedLayout),
                }
            }
            _ => return Err(ResidentKernelBindError::UnsupportedLayout),
        }
    } else {
        if source_dimensions.is_some() || selection_mode.is_some() || output_dimensions.is_some() {
            return Err(ResidentKernelBindError::UnsupportedLayout);
        }
        ResolvedAccessGeometry {
            logical_output_rows: 1,
            logical_output_columns: 1,
        }
    };
    let physical_shape_supported = if request.output.kind == ResidentValueKind::Snapshot {
        request.output.shape == ResidentShape::SCALAR
    } else {
        request.output.shape
            == resident_shape_from_dimensions(
                geometry.logical_output_rows,
                geometry.logical_output_columns,
            )?
    };
    if !physical_shape_supported {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    Ok(geometry)
}

fn bind_indexed_assign_with_region(
    request: &ResidentKernelBindRequest<'_>,
    regions: RegionPolicy,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_rmw(request, 3, regions)?;
    let [base, source, selector] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let Some(output_schema) = request.schemas.get(request.output.schema_id) else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let base_schema = request
        .schemas
        .get(base.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let source_schema = request
        .schemas
        .get(source.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let selector_schema = request
        .schemas
        .get(selector.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    if base_schema.body() != output_schema.body() {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let (aggregate_source_supported, aggregate_ordinal) = match output_schema.body() {
        SchemaBody::Tuple(elements) => {
            let homogeneous = elements
                .first()
                .is_some_and(|first| elements.iter().all(|element| element == first));
            let resolved = match selector.resolved_selector {
                Some(mech_core::ResidentResolvedSelector::Ordinal(ordinal)) => Some(ordinal),
                _ => None,
            };
            let supported = if homogeneous {
                elements
                    .first()
                    .is_some_and(|element| element == source_schema.body())
            } else {
                resolved
                    .and_then(|ordinal| elements.get(ordinal))
                    .is_some_and(|element| element == source_schema.body())
            };
            (supported, (!homogeneous).then_some(resolved).flatten())
        }
        SchemaBody::Record(fields) => {
            let homogeneous = fields
                .first()
                .is_some_and(|first| fields.iter().all(|field| field.schema == first.schema));
            let resolved = match selector.resolved_selector {
                Some(mech_core::ResidentResolvedSelector::Id(id)) => fields
                    .iter()
                    .position(|field| mech_core::hash_str(&field.name) == id),
                _ => None,
            };
            let supported = if homogeneous {
                fields
                    .first()
                    .is_some_and(|field| &field.schema == source_schema.body())
            } else {
                resolved
                    .and_then(|ordinal| fields.get(ordinal))
                    .is_some_and(|field| &field.schema == source_schema.body())
            };
            (supported, (!homogeneous).then_some(resolved).flatten())
        }
        SchemaBody::Map { value, .. } => (value.as_ref() == source_schema.body(), None),
        SchemaBody::Table { columns, .. } => {
            let source_element = match source_schema.body() {
                SchemaBody::Matrix { element, .. } => Some(element.as_ref()),
                _ => None,
            };
            let homogeneous = columns
                .first()
                .is_some_and(|first| columns.iter().all(|column| column.schema == first.schema));
            let resolved = match selector.resolved_selector {
                Some(mech_core::ResidentResolvedSelector::Id(id)) => columns
                    .iter()
                    .position(|column| mech_core::hash_str(&column.name) == id),
                _ => None,
            };
            let supported = if homogeneous {
                columns
                    .first()
                    .zip(source_element)
                    .is_some_and(|(column, element)| &column.schema == element)
            } else {
                resolved
                    .and_then(|ordinal| columns.get(ordinal))
                    .zip(source_element)
                    .is_some_and(|(column, element)| &column.schema == element)
            };
            (supported, (!homogeneous).then_some(resolved).flatten())
        }
        _ => (false, None),
    };
    if aggregate_source_supported {
        let aggregate_selector_supported = match output_schema.body() {
            SchemaBody::Tuple(_) => is_positional_selector_schema(selector_schema.body()),
            SchemaBody::Record(_) | SchemaBody::Table { .. } => {
                selector_schema.body() == &SchemaBody::Id
            }
            SchemaBody::Map { key, .. } => selector_schema.body() == key.as_ref(),
            _ => false,
        };
        if base.kind != ResidentValueKind::Snapshot
            || request.output.kind != ResidentValueKind::Snapshot
            || base.shape != ResidentShape::SCALAR
            || request.output.shape != ResidentShape::SCALAR
            || !aggregate_selector_supported
            || !scalar_layout_matches_schema_body(selector, selector_schema.body())
        {
            return Err(ResidentKernelBindError::UnsupportedLayout);
        }
        let plan = SnapshotAggregateAssignPlan {
            source: SnapshotAccessSelectorLayout {
                schema: source.schema_id,
                shape: source.shape_instance.clone(),
                resident_shape: source.shape,
            },
            selector: SnapshotAccessSelectorLayout {
                schema: selector.schema_id,
                shape: selector.shape_instance.clone(),
                resident_shape: selector.shape,
            },
            aggregate_ordinal,
        };
        return Ok(bound(
            indexed_assign_snapshot_aggregate,
            Vec::<u64>::new().into_boxed_slice(),
        )?
        .with_retained_state(Arc::new(plan))
        .with_snapshot_output(ResidentSnapshotOutput {
            schema: request.output.schema_id,
            schema_key: request.output.schema_key,
            shape: request.output.shape_instance.clone(),
            exact_cardinality: None,
            maximum_cardinality: None,
        })
        .with_snapshot_schemas(request.schemas.clone()));
    }
    let SchemaBody::Matrix { element, .. } = output_schema.body() else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    if !match source_schema.body() {
        body if body == element.as_ref() => true,
        SchemaBody::Matrix {
            element: source_element,
            ..
        } => source_element.as_ref() == element.as_ref(),
        _ => false,
    } {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    if base.kind == ResidentValueKind::Snapshot {
        let (rows, columns) = declared_matrix_dimensions(request, base)?;
        let output_len = rows
            .checked_mul(columns)
            .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
        let (source_rows, source_columns) = match request
            .schemas
            .get(source.schema_id)
            .ok_or(ResidentKernelBindError::UnsupportedLayout)?
            .body()
        {
            SchemaBody::Matrix { .. } => declared_matrix_dimensions(request, source)?,
            _ => (1, 1),
        };
        let source_len = source_rows
            .checked_mul(source_columns)
            .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
        let selector_len = declared_selector_cardinality(request, selector)?;
        let source_shape_supported = source_len == 1
            || source_len == output_len
            || if selector.kind == ResidentValueKind::Bool {
                source_len <= selector_len
            } else {
                source_len == selector_len
            };
        if !is_snapshot_index_assign_element(element)
            || source.kind != ResidentValueKind::Snapshot
            || request.output.kind != ResidentValueKind::Snapshot
            || base.shape != ResidentShape::SCALAR
            || source.shape != ResidentShape::SCALAR
            || request.output.shape != ResidentShape::SCALAR
            || !positional_selector_layout(request, selector)
            || (selector.kind == ResidentValueKind::Bool && selector_len != output_len)
            || !source_shape_supported
        {
            return Err(ResidentKernelBindError::UnsupportedLayout);
        }
        let source_routing = if source_len == 1 {
            ResolvedSourceRouting::ScalarBroadcast
        } else if selector.kind == ResidentValueKind::Bool {
            ResolvedSourceRouting::Positional
        } else if source_len == selector_len {
            ResolvedSourceRouting::CompactSelectionOrder
        } else {
            ResolvedSourceRouting::Positional
        };
        return Ok(bound(
            indexed_assign_snapshot,
            vec![
                rows as u64,
                columns as u64,
                source_rows as u64,
                source_columns as u64,
                source_routing as u64,
            ]
            .into_boxed_slice(),
        )?
        .with_snapshot_output(ResidentSnapshotOutput {
            schema: request.output.schema_id,
            schema_key: request.output.schema_key,
            shape: request.output.shape_instance.clone(),
            exact_cardinality: None,
            maximum_cardinality: None,
        })
        .with_snapshot_schemas(request.schemas.clone()));
    }
    let output_len = request
        .output
        .shape
        .len()
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let source_len = source
        .shape
        .len()
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let selector_len = declared_selector_cardinality(request, selector)?;
    let source_shape_supported = source_len == 1
        || source_len == output_len
        || if selector.kind == ResidentValueKind::Bool {
            // Logical selectors route a non-broadcast source by physical
            // position. A shorter source remains valid while every selected
            // position fits and is revalidated against the current mask.
            source_len <= selector_len
        } else {
            // Positional selectors route compact sources ordinally, so their
            // static cardinalities must agree exactly.
            source_len == selector_len
        };
    if base.kind != source.kind
        || base.kind != request.output.kind
        || base.shape != request.output.shape
        || !positional_selector_layout(request, selector)
        || (selector.kind == ResidentValueKind::Bool && selector_len != output_len)
        || !source_shape_supported
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let source_routing = if source_len == 1 {
        ResolvedSourceRouting::ScalarBroadcast
    } else if selector.kind == ResidentValueKind::Bool {
        ResolvedSourceRouting::Positional
    } else if source_len == selector_len {
        ResolvedSourceRouting::CompactSelectionOrder
    } else {
        ResolvedSourceRouting::Positional
    };
    let kernel = bound(
        indexed_assign,
        vec![output_len as u64, source_routing as u64].into_boxed_slice(),
    )?;
    if request.output.kind == ResidentValueKind::Snapshot {
        Ok(kernel.with_snapshot_schemas(request.schemas.clone()))
    } else {
        Ok(kernel)
    }
}

fn bind_indexed_assign_rows(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_matrix_selection_assign(
        request,
        RegionPolicy::IndexedAxis { axis: 0 },
        ResolvedSelectionMode::Rows,
    )
}

fn bind_indexed_assign_columns(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_matrix_selection_assign(
        request,
        RegionPolicy::IndexedAxis { axis: 1 },
        ResolvedSelectionMode::Columns,
    )
}

fn bind_indexed_assign_rectangle(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_matrix_selection_assign(
        request,
        RegionPolicy::RectangularRegion,
        ResolvedSelectionMode::Rectangle,
    )
}

fn bind_whole_matrix_assign(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_matrix_selection_assign(
        request,
        RegionPolicy::WholeValue,
        ResolvedSelectionMode::Whole,
    )
}

fn bind_matrix_selection_assign(
    request: &ResidentKernelBindRequest<'_>,
    regions: RegionPolicy,
    mode: ResolvedSelectionMode,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let selector_count = match mode {
        ResolvedSelectionMode::Whole => 0,
        ResolvedSelectionMode::Rectangle => 2,
        _ => 1,
    };
    validate_rmw(request, 2 + selector_count, regions)?;
    let base = request
        .inputs
        .first()
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let source = request
        .inputs
        .get(1)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let selectors = request
        .inputs
        .get(2..)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    if selectors.len() != selector_count {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let selector_cardinalities = selectors
        .iter()
        .map(|selector| declared_selector_cardinality(request, selector))
        .collect::<Result<Vec<_>, _>>()?;
    let base_schema = request
        .schemas
        .get(base.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let output_schema = request
        .schemas
        .get(request.output.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let source_schema = request
        .schemas
        .get(source.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let SchemaBody::Matrix { element, .. } = base_schema.body() else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    if output_schema.body() != base_schema.body() {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let (source_rows, source_columns) = match source_schema.body() {
        body if body == element.as_ref() => (1, 1),
        SchemaBody::Matrix {
            element: source_element,
            ..
        } if source_element == element => declared_matrix_dimensions(request, source)?,
        _ => return Err(ResidentKernelBindError::UnsupportedLayout),
    };
    let (rows, columns) = declared_matrix_dimensions(request, base)?;
    if declared_matrix_dimensions(request, &request.output)? != (rows, columns)
        || selectors
            .iter()
            .any(|selector| !positional_selector_layout(request, selector))
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let output_len = rows
        .checked_mul(columns)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let source_len = source_rows
        .checked_mul(source_columns)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let selector_capacity = match mode {
        ResolvedSelectionMode::Whole => Some(output_len),
        ResolvedSelectionMode::Rows => selector_cardinalities[0].checked_mul(columns),
        ResolvedSelectionMode::Columns => rows.checked_mul(selector_cardinalities[0]),
        ResolvedSelectionMode::Rectangle => {
            selector_cardinalities[0].checked_mul(selector_cardinalities[1])
        }
        ResolvedSelectionMode::LinearScalar
        | ResolvedSelectionMode::LinearGather
        | ResolvedSelectionMode::Field { .. }
        | ResolvedSelectionMode::TableColumn { .. }
        | ResolvedSelectionMode::MapKey => None,
    }
    .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let selector_axes_match = match mode {
        ResolvedSelectionMode::Whole => true,
        ResolvedSelectionMode::Rows => {
            selectors[0].kind != ResidentValueKind::Bool || selector_cardinalities[0] == rows
        }
        ResolvedSelectionMode::Columns => {
            selectors[0].kind != ResidentValueKind::Bool || selector_cardinalities[0] == columns
        }
        ResolvedSelectionMode::Rectangle => {
            (selectors[0].kind != ResidentValueKind::Bool || selector_cardinalities[0] == rows)
                && (selectors[1].kind != ResidentValueKind::Bool
                    || selector_cardinalities[1] == columns)
        }
        ResolvedSelectionMode::LinearScalar
        | ResolvedSelectionMode::LinearGather
        | ResolvedSelectionMode::Field { .. }
        | ResolvedSelectionMode::TableColumn { .. }
        | ResolvedSelectionMode::MapKey => false,
    };
    let compact_source_matches = if selectors
        .iter()
        .any(|selector| selector.kind == ResidentValueKind::Bool)
    {
        source_len <= selector_capacity
    } else {
        source_len == selector_capacity
    };
    if !selector_axes_match
        || (source_len != 1 && source_len != output_len && !compact_source_matches)
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let snapshot_layout = base.kind == ResidentValueKind::Snapshot
        && source.kind == ResidentValueKind::Snapshot
        && request.output.kind == ResidentValueKind::Snapshot
        && base.shape == ResidentShape::SCALAR
        && source.shape == ResidentShape::SCALAR
        && request.output.shape == ResidentShape::SCALAR;
    let dense_layout = matches!(
        base.kind,
        ResidentValueKind::Bool
            | ResidentValueKind::Index
            | ResidentValueKind::F64
            | ResidentValueKind::String
    ) && source.kind == base.kind
        && request.output.kind == base.kind
        && base.shape.len() == Some(output_len)
        && request.output.shape == base.shape
        && source.shape.len() == Some(source_len);
    if !snapshot_layout && !dense_layout {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let plan = MatrixSelectionAssignPlan {
        selectors: selectors
            .iter()
            .map(|selector| SnapshotAccessSelectorLayout {
                schema: selector.schema_id,
                shape: selector.shape_instance.clone(),
                resident_shape: selector.shape,
            })
            .collect::<Vec<_>>()
            .into_boxed_slice(),
        mode,
        rows,
        columns,
        source_rows,
        source_columns,
        source_routing: if source_len == 1 {
            ResolvedSourceRouting::ScalarBroadcast
        } else if source_len == output_len {
            ResolvedSourceRouting::Positional
        } else {
            ResolvedSourceRouting::CompactSelectionOrder
        },
    };
    let mut kernel = bound(
        indexed_assign_matrix_selection,
        Vec::<u64>::new().into_boxed_slice(),
    )?
    .with_retained_state(Arc::new(plan))
    .with_snapshot_schemas(request.schemas.clone());
    if snapshot_layout {
        kernel = kernel.with_snapshot_output(ResidentSnapshotOutput {
            schema: request.output.schema_id,
            schema_key: request.output.schema_key,
            shape: request.output.shape_instance.clone(),
            exact_cardinality: None,
            maximum_cardinality: None,
        });
    }
    Ok(kernel)
}

fn is_snapshot_index_assign_element(body: &SchemaBody) -> bool {
    matches!(
        body,
        SchemaBody::UnsignedInteger(_)
            | SchemaBody::SignedInteger(_)
            | SchemaBody::FloatingPoint(mech_core::FloatWidth::W32)
            | SchemaBody::Complex(_)
            | SchemaBody::Rational64
    )
}

fn bind_vertical(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_build(
        request,
        request.inputs.len(),
        &["matrix", "concatenate"],
        "vertical-output",
    )?;
    bind_matrix_constructor(request, false)
}

fn bind_range_inclusive(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_build(request, 2, &["range"], "inclusive-output")?;
    if require_kind(
        request,
        &[ResidentValueKind::F64, ResidentValueKind::F64],
        ResidentValueKind::F64,
    )
    .and_then(|_| require_f64_scalar_range_layout(request))
    .is_ok()
    {
        return bound(range_inclusive, Vec::<u64>::new().into_boxed_slice());
    }
    bind_snapshot_range(request, true, false)
}

fn bind_range_exclusive(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_build(request, 2, &["range"], "exclusive-output")?;
    if require_kind(
        request,
        &[ResidentValueKind::F64, ResidentValueKind::F64],
        ResidentValueKind::F64,
    )
    .and_then(|_| require_f64_scalar_range_layout(request))
    .is_ok()
    {
        return bound(range_exclusive, Vec::<u64>::new().into_boxed_slice());
    }
    bind_snapshot_range(request, false, false)
}

fn bind_range_increment_inclusive(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_build(request, 3, &["range"], "inclusive-increment-output")?;
    if require_kind(
        request,
        &[
            ResidentValueKind::F64,
            ResidentValueKind::F64,
            ResidentValueKind::F64,
        ],
        ResidentValueKind::F64,
    )
    .and_then(|_| require_f64_scalar_range_layout(request))
    .is_ok()
    {
        return bound(
            range_increment_inclusive,
            Vec::<u64>::new().into_boxed_slice(),
        );
    }
    bind_snapshot_range(request, true, true)
}

fn bind_range_increment_exclusive(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_build(request, 3, &["range"], "exclusive-increment-output")?;
    if require_kind(
        request,
        &[
            ResidentValueKind::F64,
            ResidentValueKind::F64,
            ResidentValueKind::F64,
        ],
        ResidentValueKind::F64,
    )
    .and_then(|_| require_f64_scalar_range_layout(request))
    .is_ok()
    {
        return bound(
            range_increment_exclusive,
            Vec::<u64>::new().into_boxed_slice(),
        );
    }
    bind_snapshot_range(request, false, true)
}

fn is_range_snapshot_element(body: &SchemaBody) -> bool {
    matches!(
        body,
        SchemaBody::UnsignedInteger(_)
            | SchemaBody::SignedInteger(_)
            | SchemaBody::FloatingPoint(mech_core::FloatWidth::W32 | mech_core::FloatWidth::W64)
    )
}

fn bind_snapshot_range(
    request: &ResidentKernelBindRequest<'_>,
    inclusive: bool,
    incremented: bool,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let expected_inputs = if incremented { 3 } else { 2 };
    if request.inputs.len() != expected_inputs
        || request.inputs.iter().any(|input| {
            input.kind != ResidentValueKind::Snapshot || input.shape != ResidentShape::SCALAR
        })
        || request.output.kind != ResidentValueKind::Snapshot
        || request.output.shape != ResidentShape::SCALAR
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let input_schema = request
        .schemas
        .get(request.inputs[0].schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    if !is_range_snapshot_element(input_schema.body())
        || request.inputs.iter().any(|input| {
            request
                .schemas
                .get(input.schema_id)
                .is_none_or(|schema| schema.body() != input_schema.body())
        })
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let output_schema = request
        .schemas
        .get(request.output.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let SchemaBody::Matrix { element, .. } = output_schema.body() else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let (rows, columns) = declared_matrix_dimensions(request, &request.output)?;
    if rows != 1 || columns == 0 || element.as_ref() != input_schema.body() {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    Ok(bound(
        range_snapshot,
        vec![u64::from(inclusive), u64::from(incremented), columns as u64].into_boxed_slice(),
    )?
    .with_snapshot_output(ResidentSnapshotOutput {
        schema: request.output.schema_id,
        schema_key: request.output.schema_key,
        shape: request.output.shape_instance.clone(),
        exact_cardinality: None,
        maximum_cardinality: None,
    })
    .with_snapshot_schemas(request.schemas.clone()))
}

fn require_f64_scalar_range_layout(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<(), ResidentKernelBindError> {
    let scalar = SchemaBody::FloatingPoint(mech_core::FloatWidth::W64);
    let inputs_are_scalars = request.inputs.iter().all(|input| {
        input.shape == ResidentShape::SCALAR
            && request
                .schemas
                .get(input.schema_id)
                .is_some_and(|schema| schema.body() == &scalar)
    });
    let output_is_matrix = request
        .schemas
        .get(request.output.schema_id)
        .is_some_and(|schema| {
            matches!(
                schema.body(),
                SchemaBody::Matrix { element, dimensions }
                    if dimensions.len() == 2 && element.as_ref() == &scalar
            )
        });
    if !inputs_are_scalars || !output_is_matrix || request.output.shape.rows != 1 {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    Ok(())
}

fn bind_n_choose_k(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    if matches!(
        request.contract,
        ResolvedOperationContract::Declared(contract)
            if matches!(
                contract.outputs.first().map(|output| &output.construction),
                Some(OutputConstruction::FullWrite { .. })
            )
    ) {
        validate_full_write(
            request,
            2,
            ShapeRule::Declared,
            ChangeDetectionPolicy::ExactScalar,
        )?;
        if require_f64_lengths(request, &[1, 1], 1).is_ok() {
            return bound(n_choose_k_scalar, Vec::<u64>::new().into_boxed_slice());
        }
        return bind_snapshot_n_choose_k_scalar(request);
    }

    validate_build(request, 2, &["combinatorics"], "n-choose-k-matrix-output")?;
    if require_kind(
        request,
        &[ResidentValueKind::F64, ResidentValueKind::F64],
        ResidentValueKind::F64,
    )
    .is_err()
    {
        return bind_snapshot_n_choose_k_matrix(request);
    }
    let Some(input_len) = request.inputs[0].shape.len() else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let k = request.output.shape.rows as usize;
    let combinations = request.output.shape.columns as usize;
    if request.inputs[1].shape != ResidentShape::SCALAR
        || k == 0
        || k > input_len
        || checked_combination_count(input_len, k) != Some(combinations)
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    bound(
        n_choose_k,
        vec![k as u64, combinations as u64].into_boxed_slice(),
    )
}

fn is_n_choose_k_snapshot_element(body: &SchemaBody) -> bool {
    match body {
        SchemaBody::UnsignedInteger(_)
        | SchemaBody::SignedInteger(_)
        | SchemaBody::FloatingPoint(mech_core::FloatWidth::W32 | mech_core::FloatWidth::W64) => {
            true
        }
        SchemaBody::Rational64 => cfg!(feature = "r64"),
        SchemaBody::Complex(mech_core::FloatWidth::W64) => cfg!(feature = "c64"),
        _ => false,
    }
}

fn bind_snapshot_n_choose_k_scalar(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let [n, k] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    if n.kind != ResidentValueKind::Snapshot
        || k.kind != ResidentValueKind::Snapshot
        || request.output.kind != ResidentValueKind::Snapshot
        || n.shape != ResidentShape::SCALAR
        || k.shape != ResidentShape::SCALAR
        || request.output.shape != ResidentShape::SCALAR
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let n_schema = request
        .schemas
        .get(n.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let k_schema = request
        .schemas
        .get(k.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let output_schema = request
        .schemas
        .get(request.output.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    if n_schema.body() != k_schema.body()
        || n_schema.body() != output_schema.body()
        || !is_n_choose_k_snapshot_element(n_schema.body())
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    Ok(bound(
        n_choose_k_scalar_snapshot,
        Vec::<u64>::new().into_boxed_slice(),
    )?
    .with_snapshot_output(ResidentSnapshotOutput {
        schema: request.output.schema_id,
        schema_key: request.output.schema_key,
        shape: request.output.shape_instance.clone(),
        exact_cardinality: None,
        maximum_cardinality: None,
    })
    .with_snapshot_schemas(request.schemas.clone()))
}

fn bind_snapshot_n_choose_k_matrix(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let [input, selection] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    if input.kind != ResidentValueKind::Snapshot
        || selection.kind != ResidentValueKind::Snapshot
        || request.output.kind != ResidentValueKind::Snapshot
        || input.shape != ResidentShape::SCALAR
        || selection.shape != ResidentShape::SCALAR
        || request.output.shape != ResidentShape::SCALAR
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let input_schema = request
        .schemas
        .get(input.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let selection_schema = request
        .schemas
        .get(selection.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let output_schema = request
        .schemas
        .get(request.output.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let (
        SchemaBody::Matrix {
            element: input_element,
            ..
        },
        SchemaBody::Matrix {
            element: output_element,
            ..
        },
    ) = (input_schema.body(), output_schema.body())
    else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let (input_rows, input_columns) = declared_matrix_dimensions(request, input)?;
    let input_len = input_rows
        .checked_mul(input_columns)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let (k, combinations) = declared_matrix_dimensions(request, &request.output)?;
    if k == 0
        || k > input_len
        || checked_combination_count(input_len, k) != Some(combinations)
        || input_element != output_element
        || selection_schema.body() != input_element.as_ref()
        || !is_n_choose_k_snapshot_element(input_element)
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    Ok(bound(
        n_choose_k_snapshot,
        vec![k as u64, combinations as u64].into_boxed_slice(),
    )?
    .with_snapshot_output(ResidentSnapshotOutput {
        schema: request.output.schema_id,
        schema_key: request.output.schema_key,
        shape: request.output.shape_instance.clone(),
        exact_cardinality: None,
        maximum_cardinality: None,
    })
    .with_snapshot_schemas(request.schemas.clone()))
}

pub(super) fn checked_combination_count(n: usize, k: usize) -> Option<usize> {
    if k > n {
        return None;
    }
    let k = k.min(n - k);
    let mut result = 1_u128;
    for divisor in 1..=k {
        result = result
            .checked_mul((n - k + divisor) as u128)?
            .checked_div(divisor as u128)?;
    }
    usize::try_from(result).ok()
}

fn bind_gather_1d(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_full_write(
        request,
        2,
        ShapeRule::Declared,
        ChangeDetectionPolicy::KernelReported,
    )?;
    let [source, selector] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let selected = declared_selector_cardinality(request, selector)?;
    let expected_output = resident_shape_from_dimensions(selected, 1)?;
    if source.kind != ResidentValueKind::F64
        || !numeric_positional_selector_layout(request, selector)
        || request.output.kind != ResidentValueKind::F64
        || request.output.shape != expected_output
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    bound(gather_1d, Vec::<u64>::new().into_boxed_slice())
}

fn bind_scalar_access_1d(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_full_write(
        request,
        2,
        ShapeRule::Declared,
        ChangeDetectionPolicy::KernelReported,
    )?;
    if request.inputs.len() != 2
        || request.inputs[0].kind != ResidentValueKind::F64
        || !numeric_positional_selector_layout(request, &request.inputs[1])
        || declared_selector_cardinality(request, &request.inputs[1])? != 1
        || request.output.kind != ResidentValueKind::F64
        || request.output.shape != ResidentShape::SCALAR
        || request.inputs[0].shape.len().is_none()
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    bound(scalar_access_1d, Vec::<u64>::new().into_boxed_slice())
}

fn bind_scalar_index(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_full_write(
        request,
        1,
        ShapeRule::Declared,
        ChangeDetectionPolicy::KernelReported,
    )?;
    if request.inputs.len() != 1
        || !numeric_positional_selector_layout(request, &request.inputs[0])
        || request.output.kind != ResidentValueKind::Index
        || request.output.shape.len()
            != Some(declared_selector_cardinality(request, &request.inputs[0])?)
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    bound(scalar_index, Vec::<u64>::new().into_boxed_slice())
}

fn bind_semantic_scalar_access(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_scalar_access_1d(request)
        .or_else(|_| bind_scalar_access_2d(request))
        .or_else(|_| bind_all_rows_column(request))
        .or_else(|_| bind_row_all_columns(request))
        .or_else(|_| super::text::bind_string_scalar_access(request))
        .or_else(|_| {
            let mode = if request.inputs.len() == 3 {
                ResolvedSelectionMode::Rectangle
            } else {
                ResolvedSelectionMode::LinearScalar
            };
            bind_snapshot_access_mode(request, Some(mode))
        })
}

fn bind_scalar_access_2d(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_full_write(
        request,
        3,
        ShapeRule::Declared,
        ChangeDetectionPolicy::KernelReported,
    )?;
    let [source, row, column] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    if source.kind != ResidentValueKind::F64
        || !numeric_positional_selector_layout(request, row)
        || !numeric_positional_selector_layout(request, column)
        || declared_selector_cardinality(request, row)? != 1
        || declared_selector_cardinality(request, column)? != 1
        || request.output.kind != ResidentValueKind::F64
        || request.output.shape != ResidentShape::SCALAR
        || source.shape.len().is_none()
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    bound(
        scalar_access_2d,
        vec![source.shape.rows as u64, source.shape.columns as u64].into_boxed_slice(),
    )
}

fn bind_semantic_range_access(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_gather_1d(request)
        .or_else(|_| bind_all_rows_columns(request))
        .or_else(|_| bind_rows_all_columns(request))
        .or_else(|_| super::text::bind_string_gather(request))
        .or_else(|_| {
            let mode = if request.inputs.len() == 3 {
                ResolvedSelectionMode::Rectangle
            } else {
                ResolvedSelectionMode::LinearGather
            };
            bind_snapshot_access_mode(request, Some(mode))
        })
}

fn bind_semantic_rows_access(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_row_all_columns(request)
        .or_else(|_| bind_rows_all_columns(request))
        .or_else(|_| bind_snapshot_access_mode(request, Some(ResolvedSelectionMode::Rows)))
}

fn bind_semantic_columns_access(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_all_rows_column(request)
        .or_else(|_| bind_all_rows_columns(request))
        .or_else(|_| bind_snapshot_access_mode(request, Some(ResolvedSelectionMode::Columns)))
}

#[cfg(test)]
fn bind_snapshot_access(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_snapshot_access_mode(request, None)
}

fn bind_snapshot_access_mode(
    request: &ResidentKernelBindRequest<'_>,
    explicit_matrix_mode: Option<ResolvedSelectionMode>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_full_write(
        request,
        request.inputs.len(),
        ShapeRule::Declared,
        ChangeDetectionPolicy::KernelReported,
    )?;
    if !(2..=3).contains(&request.inputs.len()) {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let source = &request.inputs[0];
    if source.kind != ResidentValueKind::Snapshot || source.shape != ResidentShape::SCALAR {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let source_schema = request
        .schemas
        .get(source.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let output_schema = request
        .schemas
        .get(request.output.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let selector_count = request.inputs.len() - 1;
    let selector_schemas = request.inputs[1..]
        .iter()
        .map(|selector| {
            request
                .schemas
                .get(selector.schema_id)
                .ok_or(ResidentKernelBindError::UnsupportedLayout)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let selector_layouts_valid = request.inputs[1..]
        .iter()
        .zip(&selector_schemas)
        .all(|(layout, schema)| access_selector_layout_matches_schema(layout, schema.body()));
    if !selector_layouts_valid {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let selector_schemas_valid = match source_schema.body() {
        SchemaBody::Tuple(_) => selector_schemas
            .first()
            .is_some_and(|schema| is_access_positional_selector_schema(schema.body())),
        SchemaBody::Record(_) | SchemaBody::Table { .. } => selector_schemas
            .first()
            .is_some_and(|schema| schema.body() == &SchemaBody::Id),
        SchemaBody::Map { key, .. } => selector_schemas
            .first()
            .is_some_and(|schema| schema.body() == key.as_ref()),
        SchemaBody::Matrix { .. } => selector_schemas
            .iter()
            .all(|schema| is_access_positional_selector_schema(schema.body())),
        _ => false,
    };
    if !selector_schemas_valid {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let table_rows = match source_schema.body() {
        SchemaBody::Table { rows, .. } => Some(rows),
        _ => None,
    };
    let (matrix_mode, source_dimensions, output_dimensions, aggregate_ordinal, output_supported) =
        match source_schema.body() {
            SchemaBody::Tuple(elements) if selector_count == 1 => {
                let homogeneous = elements
                    .first()
                    .is_some_and(|first| elements.iter().all(|element| element == first));
                let ordinal = match request.inputs[1].resolved_selector {
                    Some(mech_core::ResidentResolvedSelector::Ordinal(ordinal)) => Some(ordinal),
                    _ => None,
                };
                let selected_matches = ordinal
                    .and_then(|ordinal| elements.get(ordinal))
                    .is_some_and(|element| element == output_schema.body());
                (
                    None,
                    None,
                    None,
                    (!homogeneous).then_some(ordinal).flatten(),
                    if homogeneous {
                        elements
                            .first()
                            .is_some_and(|element| element == output_schema.body())
                    } else {
                        selected_matches
                    },
                )
            }
            SchemaBody::Record(fields) if selector_count == 1 => {
                let homogeneous = fields
                    .first()
                    .is_some_and(|first| fields.iter().all(|field| field.schema == first.schema));
                let ordinal = match request.inputs[1].resolved_selector {
                    Some(mech_core::ResidentResolvedSelector::Id(id)) => fields
                        .iter()
                        .position(|field| mech_core::hash_str(&field.name) == id),
                    _ => None,
                };
                let selected_matches = ordinal
                    .and_then(|ordinal| fields.get(ordinal))
                    .is_some_and(|field| &field.schema == output_schema.body());
                (
                    None,
                    None,
                    None,
                    (!homogeneous).then_some(ordinal).flatten(),
                    if homogeneous {
                        fields
                            .first()
                            .is_some_and(|field| &field.schema == output_schema.body())
                    } else {
                        selected_matches
                    },
                )
            }
            SchemaBody::Map { value, .. } if selector_count == 1 => (
                None,
                None,
                None,
                None,
                value.as_ref() == output_schema.body(),
            ),
            SchemaBody::Table { columns, .. } if selector_count == 1 => {
                let output_element = match output_schema.body() {
                    SchemaBody::Matrix { element, .. } => Some(element.as_ref()),
                    _ => None,
                };
                let homogeneous = columns.first().is_some_and(|first| {
                    columns.iter().all(|column| column.schema == first.schema)
                });
                let ordinal = match request.inputs[1].resolved_selector {
                    Some(mech_core::ResidentResolvedSelector::Id(id)) => columns
                        .iter()
                        .position(|column| mech_core::hash_str(&column.name) == id),
                    _ => None,
                };
                let selected_matches = ordinal
                    .and_then(|ordinal| columns.get(ordinal))
                    .zip(output_element)
                    .is_some_and(|(column, output)| &column.schema == output);
                let output_dimensions = declared_matrix_dimensions(request, &request.output).ok();
                (
                    None,
                    None,
                    output_dimensions,
                    (!homogeneous).then_some(ordinal).flatten(),
                    if homogeneous {
                        columns
                            .first()
                            .zip(output_element)
                            .is_some_and(|(column, output)| &column.schema == output)
                    } else {
                        selected_matches
                    },
                )
            }
            SchemaBody::Matrix { element, .. } if (1..=2).contains(&selector_count) => {
                let source_dimensions = declared_matrix_dimensions(request, source)?;
                let output_dimensions = match output_schema.body() {
                    body if body == element.as_ref() => None,
                    SchemaBody::Matrix {
                        element: output_element,
                        ..
                    } if output_element == element => {
                        Some(declared_matrix_dimensions(request, &request.output)?)
                    }
                    _ => return Err(ResidentKernelBindError::UnsupportedLayout),
                };
                let inferred_mode = if selector_count == 2 {
                    ResolvedSelectionMode::Rectangle
                } else if output_dimensions.is_none() {
                    ResolvedSelectionMode::LinearScalar
                } else {
                    ResolvedSelectionMode::LinearGather
                };
                if let Some(explicit) = explicit_matrix_mode {
                    let expected_selectors = match explicit {
                        ResolvedSelectionMode::Rectangle => 2,
                        _ => 1,
                    };
                    if selector_count != expected_selectors {
                        return Err(ResidentKernelBindError::UnsupportedLayout);
                    }
                }
                let mode = explicit_matrix_mode.unwrap_or(inferred_mode);
                (
                    Some(mode),
                    Some(source_dimensions),
                    output_dimensions,
                    None,
                    true,
                )
            }
            _ => return Err(ResidentKernelBindError::UnsupportedLayout),
        };
    if !output_supported {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let output_geometry = validate_snapshot_access_geometry(
        request,
        source_dimensions,
        output_dimensions,
        matrix_mode,
        &request.inputs[1..],
        table_rows,
    )?;
    let selectors = request.inputs[1..]
        .iter()
        .map(|selector| SnapshotAccessSelectorLayout {
            schema: selector.schema_id,
            shape: selector.shape_instance.clone(),
            resident_shape: selector.shape,
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let plan = SnapshotAccessPlan {
        selectors,
        matrix_mode,
        source_dimensions,
        output_dimensions,
        output_geometry,
        aggregate_ordinal,
        output_schema: request.output.schema_id,
    };
    Ok(
        bound(snapshot_access, Vec::<u64>::new().into_boxed_slice())?
            .with_retained_state(Arc::new(plan))
            .with_snapshot_output(ResidentSnapshotOutput {
                schema: request.output.schema_id,
                schema_key: request.output.schema_key,
                shape: request.output.shape_instance.clone(),
                exact_cardinality: None,
                maximum_cardinality: None,
            })
            .with_snapshot_schemas(request.schemas.clone()),
    )
}

fn bind_matmul(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_full_write(
        request,
        2,
        ShapeRule::MatrixProduct { lhs: 0, rhs: 1 },
        ChangeDetectionPolicy::KernelReported,
    )?;
    let [lhs, rhs] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    if lhs.kind == ResidentValueKind::F64
        && rhs.kind == ResidentValueKind::F64
        && request.output.kind == ResidentValueKind::F64
        && lhs.shape.columns == rhs.shape.rows
        && request.output.shape.rows == lhs.shape.rows
        && request.output.shape.columns == rhs.shape.columns
    {
        return bound(
            matrix_multiply,
            vec![
                lhs.shape.rows as u64,
                lhs.shape.columns as u64,
                rhs.shape.columns as u64,
            ]
            .into_boxed_slice(),
        );
    }
    let lhs_schema = request
        .schemas
        .get(lhs.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let rhs_schema = request
        .schemas
        .get(rhs.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let output_schema = request
        .schemas
        .get(request.output.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let (
        SchemaBody::Matrix {
            element: lhs_element,
            ..
        },
        SchemaBody::Matrix {
            element: rhs_element,
            ..
        },
        SchemaBody::Matrix {
            element: output_element,
            ..
        },
    ) = (lhs_schema.body(), rhs_schema.body(), output_schema.body())
    else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let (rows, inner) = declared_matrix_dimensions(request, lhs)?;
    let (rhs_rows, columns) = declared_matrix_dimensions(request, rhs)?;
    let output_dimensions = declared_matrix_dimensions(request, &request.output)?;
    if lhs.kind != ResidentValueKind::Snapshot
        || rhs.kind != ResidentValueKind::Snapshot
        || request.output.kind != ResidentValueKind::Snapshot
        || lhs.shape != ResidentShape::SCALAR
        || rhs.shape != ResidentShape::SCALAR
        || request.output.shape != ResidentShape::SCALAR
        || inner != rhs_rows
        || output_dimensions != (rows, columns)
        || lhs_element != rhs_element
        || lhs_element != output_element
        || !is_dot_numeric_schema(lhs_element)
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    Ok(bound(
        matrix_multiply_snapshot,
        vec![rows as u64, inner as u64, columns as u64].into_boxed_slice(),
    )?
    .with_snapshot_output(ResidentSnapshotOutput {
        schema: request.output.schema_id,
        schema_key: request.output.schema_key,
        shape: request.output.shape_instance.clone(),
        exact_cardinality: None,
        maximum_cardinality: None,
    })
    .with_snapshot_schemas(request.schemas.clone()))
}

fn dot_element_schema(body: &SchemaBody) -> &SchemaBody {
    match body {
        SchemaBody::Matrix { element, .. } => element,
        body => body,
    }
}

fn is_dot_numeric_schema(body: &SchemaBody) -> bool {
    matches!(
        body,
        SchemaBody::UnsignedInteger(_)
            | SchemaBody::SignedInteger(_)
            | SchemaBody::FloatingPoint(mech_core::FloatWidth::W32 | mech_core::FloatWidth::W64)
    )
}

fn bind_matrix_dot(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_full_write(
        request,
        2,
        ShapeRule::Declared,
        ChangeDetectionPolicy::ExactScalar,
    )?;
    let [left, right] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let left_schema = request
        .schemas
        .get(left.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let right_schema = request
        .schemas
        .get(right.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let output_schema = request
        .schemas
        .get(request.output.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let left_element = dot_element_schema(left_schema.body());
    let right_element = dot_element_schema(right_schema.body());
    if left_schema.body() != right_schema.body()
        || left_element != right_element
        || left_element != output_schema.body()
        || !is_dot_numeric_schema(left_element)
        || left.shape != right.shape
        || request.output.shape != ResidentShape::SCALAR
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    match (left.kind, right.kind, request.output.kind) {
        (ResidentValueKind::F64, ResidentValueKind::F64, ResidentValueKind::F64) => {
            bound(matrix_dot_f64, Vec::<u64>::new().into_boxed_slice())
        }
        (ResidentValueKind::Snapshot, ResidentValueKind::Snapshot, ResidentValueKind::Snapshot)
            if left.shape == ResidentShape::SCALAR =>
        {
            Ok(
                bound(matrix_dot_snapshot, Vec::<u64>::new().into_boxed_slice())?
                    .with_snapshot_output(ResidentSnapshotOutput {
                        schema: request.output.schema_id,
                        schema_key: request.output.schema_key,
                        shape: request.output.shape_instance.clone(),
                        exact_cardinality: None,
                        maximum_cardinality: None,
                    })
                    .with_snapshot_schemas(request.schemas.clone()),
            )
        }
        _ => Err(ResidentKernelBindError::UnsupportedLayout),
    }
}

fn declared_matrix_dimensions(
    request: &ResidentKernelBindRequest<'_>,
    layout: &mech_core::ResidentPortLayout,
) -> Result<(usize, usize), ResidentKernelBindError> {
    let schema = request
        .schemas
        .get(layout.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let SchemaBody::Matrix { dimensions, .. } = schema.body() else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let [rows, columns] = dimensions.as_ref() else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let rows = layout
        .shape_instance
        .resolve_dimension(rows)
        .ok()
        .and_then(|value| usize::try_from(value).ok())
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let columns = layout
        .shape_instance
        .resolve_dimension(columns)
        .ok()
        .and_then(|value| usize::try_from(value).ok())
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    Ok((rows, columns))
}

fn matrix_solve_work(rows: usize, right_columns: usize) -> Option<usize> {
    rows.checked_mul(rows).and_then(|square| {
        rows.checked_add(right_columns)
            .and_then(|width| square.checked_mul(width))
    })
}

fn admit_matrix_solve(
    rows: usize,
    right_columns: usize,
    element_bytes: usize,
    coefficient_copies: usize,
    right_copies: usize,
    output_container_bytes: usize,
    supplemental: super::budget::KernelCostEstimate,
) -> Result<(), ResidentKernelError> {
    let coefficient_count = rows
        .checked_mul(rows)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let output_count = rows
        .checked_mul(right_columns)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let coefficient_bytes = coefficient_count
        .checked_mul(element_bytes)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let output_bytes = output_count
        .checked_mul(element_bytes)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let temporary_bytes = coefficient_bytes
        .checked_mul(coefficient_copies)
        .and_then(|bytes| {
            output_bytes
                .checked_mul(right_copies)
                .and_then(|right| bytes.checked_add(right))
        })
        .and_then(|bytes| bytes.checked_add(output_container_bytes))
        .ok_or(ResidentKernelError::InvalidShape)?;
    super::budget::PreparedKernel::new(
        (),
        super::budget::resident_cost! {
            comparison_work: supplemental.comparison_work(),
            compute_work: super::budget::checked_u64(
                matrix_solve_work(rows, right_columns)
                    .ok_or(ResidentKernelError::InvalidShape)?,
            )?
                .checked_add(supplemental.compute_work())
                .ok_or(ResidentKernelError::InvalidShape)?,
            output_elements: output_count,
            output_bytes,
            temporary_bytes: super::budget::checked_u64(temporary_bytes)?
                .checked_add(supplemental.temporary_bytes())
                .ok_or(ResidentKernelError::InvalidShape)?,
            cloned_bytes: super::budget::checked_u64(coefficient_bytes)?
                .checked_add(super::budget::checked_u64(output_bytes)?)
                .and_then(|bytes| bytes.checked_add(supplemental.cloned_bytes()))
                .ok_or(ResidentKernelError::InvalidShape)?,
            retained_nodes: supplemental.retained_nodes(),
            ..super::budget::KernelCostEstimate::default()
        },
    )
    .admit()?
    .into_plan();
    Ok(())
}

fn bind_matrix_solve(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_full_write(
        request,
        2,
        ShapeRule::SameAsInput { input: 1 },
        ChangeDetectionPolicy::KernelReported,
    )?;
    let [coefficients, right] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let (rows, columns) = declared_matrix_dimensions(request, coefficients)?;
    let (right_rows, right_columns) = declared_matrix_dimensions(request, right)?;
    let (output_rows, output_columns) = declared_matrix_dimensions(request, &request.output)?;
    let coefficient_schema = request
        .schemas
        .get(coefficients.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let right_schema = request
        .schemas
        .get(right.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let output_schema = request
        .schemas
        .get(request.output.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let (
        SchemaBody::Matrix {
            element: coefficient_element,
            ..
        },
        SchemaBody::Matrix {
            element: right_element,
            ..
        },
        SchemaBody::Matrix {
            element: output_element,
            ..
        },
    ) = (
        coefficient_schema.body(),
        right_schema.body(),
        output_schema.body(),
    )
    else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    if rows != columns
        || right_rows != rows
        || (output_rows, output_columns) != (right_rows, right_columns)
        || coefficient_element != right_element
        || coefficient_element != output_element
        || !matches!(
            coefficient_element.as_ref(),
            SchemaBody::FloatingPoint(mech_core::FloatWidth::W32 | mech_core::FloatWidth::W64)
        )
        || matrix_solve_work(rows, right_columns).is_none_or(|work| work > MAX_MATRIX_SOLVE_WORK)
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let parameters = vec![rows as u64, right_columns as u64].into_boxed_slice();
    match (coefficients.kind, right.kind, request.output.kind) {
        (ResidentValueKind::F64, ResidentValueKind::F64, ResidentValueKind::F64) => {
            bound(matrix_solve_f64, parameters)
        }
        (ResidentValueKind::Snapshot, ResidentValueKind::Snapshot, ResidentValueKind::Snapshot)
            if coefficients.shape == ResidentShape::SCALAR
                && right.shape == ResidentShape::SCALAR
                && request.output.shape == ResidentShape::SCALAR =>
        {
            Ok(bound(matrix_solve_f32_snapshot, parameters)?
                .with_snapshot_output(ResidentSnapshotOutput {
                    schema: request.output.schema_id,
                    schema_key: request.output.schema_key,
                    shape: request.output.shape_instance.clone(),
                    exact_cardinality: None,
                    maximum_cardinality: None,
                })
                .with_snapshot_schemas(request.schemas.clone()))
        }
        _ => Err(ResidentKernelBindError::UnsupportedLayout),
    }
}

fn bind_all_rows_columns(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_selection_contract(request)?;
    let [source, _] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let selected = declared_selector_cardinality(request, &request.inputs[1])?;
    let expected = resident_shape_from_dimensions(source.shape.rows as usize, selected)?;
    if request.output.shape != expected {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    bound(
        all_rows_columns,
        vec![source.shape.rows as u64, source.shape.columns as u64].into_boxed_slice(),
    )
}

fn bind_all_rows_column(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    if request.inputs.get(1).is_none()
        || declared_selector_cardinality(request, &request.inputs[1])? != 1
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    bind_all_rows_columns(request).map(|_| {
        BoundResidentKernel::new(
            all_rows_column,
            vec![
                request.inputs[0].shape.rows as u64,
                request.inputs[0].shape.columns as u64,
            ]
            .into_boxed_slice(),
        )
    })
}

fn bind_row_all_columns(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_selection_contract(request)?;
    let [source, _] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let expected = resident_shape_from_dimensions(1, source.shape.columns as usize)?;
    if declared_selector_cardinality(request, &request.inputs[1])? != 1
        || request.output.shape != expected
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    bound(
        row_all_columns,
        vec![source.shape.rows as u64, source.shape.columns as u64].into_boxed_slice(),
    )
}

fn bind_rows_all_columns(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_selection_contract(request)?;
    let [source, _] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let selected = declared_selector_cardinality(request, &request.inputs[1])?;
    let expected = resident_shape_from_dimensions(selected, source.shape.columns as usize)?;
    if request.output.shape != expected {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    bound(
        rows_all_columns,
        vec![source.shape.rows as u64, source.shape.columns as u64].into_boxed_slice(),
    )
}

fn bind_add_indexed_rows(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_indexed_rows(request, add_indexed_rows)
}

fn bind_sub_indexed_rows(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_indexed_rows(request, sub_indexed_rows)
}

fn bind_indexed_rows(
    request: &ResidentKernelBindRequest<'_>,
    executor: mech_core::ResidentKernelExecutor,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_rmw(request, 3, RegionPolicy::IndexedAxis { axis: 0 })?;
    let [base, source, indices] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    if base.kind != ResidentValueKind::F64
        || source.kind != ResidentValueKind::F64
        || !numeric_positional_selector_layout(request, indices)
        || request.output.kind != ResidentValueKind::F64
        || base.shape != request.output.shape
        || source.shape.rows as usize != declared_selector_cardinality(request, indices)?
        || source.shape.columns != base.shape.columns
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    bound(
        executor,
        vec![
            base.shape.rows as u64,
            base.shape.columns as u64,
            source.shape.rows as u64,
            source.shape.columns as u64,
            declared_selector_cardinality(request, indices)? as u64,
        ]
        .into_boxed_slice(),
    )
}

fn validate_selection_contract(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<(), ResidentKernelBindError> {
    validate_full_write(
        request,
        2,
        ShapeRule::Declared,
        ChangeDetectionPolicy::KernelReported,
    )?;
    if request.inputs.len() != 2
        || request.inputs[0].kind != ResidentValueKind::F64
        || !numeric_positional_selector_layout(request, &request.inputs[1])
        || request.output.kind != ResidentValueKind::F64
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    Ok(())
}

fn input(
    inputs: &dyn ResidentKernelInputs,
    index: usize,
) -> Result<ResidentValueRef<'_>, ResidentKernelError> {
    inputs.get(index).ok_or(ResidentKernelError::InvalidInput)
}

fn f64_input(
    inputs: &dyn ResidentKernelInputs,
    index: usize,
) -> Result<&[f64], ResidentKernelError> {
    inputs.f64(index).ok_or(ResidentKernelError::InvalidInput)
}

fn f64_output(output: ResidentValueMut<'_>) -> Result<&mut [f64], ResidentKernelError> {
    let ResidentValueMut::F64(values) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    Ok(values)
}

fn index_at(
    inputs: &dyn ResidentKernelInputs,
    input_index: usize,
    ordinal: usize,
) -> Result<u64, ResidentKernelError> {
    let mut selected = None;
    let mut current = 0usize;
    selector_for_each_access_index(input(inputs, input_index)?, usize::MAX, |position| {
        if current == ordinal {
            selected = Some(position);
        }
        current = current
            .checked_add(1)
            .ok_or(ResidentKernelError::InvalidShape)?;
        Ok(())
    })?;
    selected
        .and_then(|position| position.checked_add(1))
        .and_then(|index| u64::try_from(index).ok())
        .ok_or(ResidentKernelError::InvalidInput)
}

#[derive(Clone, Copy)]
struct ValidatedIndices<'a> {
    selector: ResidentValueRef<'a>,
    upper: usize,
    len: usize,
}

impl<'a> ValidatedIndices<'a> {
    fn new(selector: ResidentValueRef<'a>, upper: usize) -> Result<Self, ResidentKernelError> {
        if matches!(
            selector,
            ResidentValueRef::Bool(_) | ResidentValueRef::String(_)
        ) {
            return Err(ResidentKernelError::InvalidShape);
        }
        let mut len = 0usize;
        selector_for_each_access_index(selector, upper, |_| {
            len = len
                .checked_add(1)
                .ok_or(ResidentKernelError::InvalidShape)?;
            Ok(())
        })?;
        Ok(Self {
            selector,
            upper,
            len,
        })
    }

    fn len(self) -> usize {
        self.len
    }

    fn try_for_each(
        self,
        mut visitor: impl FnMut(usize, u64) -> Result<(), ResidentKernelError>,
    ) -> Result<(), ResidentKernelError> {
        let mut ordinal = 0usize;
        selector_for_each_access_index(self.selector, self.upper, |position| {
            let index = position
                .checked_add(1)
                .and_then(|index| u64::try_from(index).ok())
                .ok_or(ResidentKernelError::InvalidShape)?;
            visitor(ordinal, index)?;
            ordinal = ordinal
                .checked_add(1)
                .ok_or(ResidentKernelError::InvalidShape)?;
            Ok(())
        })
    }

    fn try_for_each_position(
        self,
        mut visitor: impl FnMut(usize, usize) -> Result<(), ResidentKernelError>,
    ) -> Result<(), ResidentKernelError> {
        self.try_for_each(|ordinal, index| visitor(ordinal, index as usize - 1))
    }
}

#[derive(Clone, Copy)]
enum ValidatedPositions<'a> {
    Mask { values: &'a [u8], selected: usize },
    Indices(ValidatedIndices<'a>),
}

impl<'a> ValidatedPositions<'a> {
    fn new(selector: ResidentValueRef<'a>, output_len: usize) -> Result<Self, ResidentKernelError> {
        match selector {
            ResidentValueRef::Bool(values) if values.len() == output_len => {
                let mut selected = 0usize;
                for value in values {
                    if *value > 1 {
                        return Err(ResidentKernelError::InvalidInput);
                    }
                    selected = selected
                        .checked_add(usize::from(*value != 0))
                        .ok_or(ResidentKernelError::InvalidShape)?;
                }
                Ok(Self::Mask { values, selected })
            }
            ResidentValueRef::Bool(_) => Err(ResidentKernelError::InvalidShape),
            selector => ValidatedIndices::new(selector, output_len).map(Self::Indices),
        }
    }

    fn len(self) -> usize {
        match self {
            Self::Mask { selected, .. } => selected,
            Self::Indices(indices) => indices.len(),
        }
    }

    fn traversal_len(self) -> usize {
        match self {
            Self::Mask { values, .. } => values.len(),
            Self::Indices(indices) => indices.len(),
        }
    }

    fn maximum_position(self) -> Option<usize> {
        let mut maximum = None;
        let result = self.try_for_each(|_, position| {
            maximum = Some(maximum.map_or(position, |current: usize| current.max(position)));
            Ok::<(), ResidentKernelError>(())
        });
        debug_assert!(
            result.is_ok(),
            "validated selector replay must remain valid"
        );
        maximum
    }

    fn try_for_each(
        self,
        mut visitor: impl FnMut(usize, usize) -> Result<(), ResidentKernelError>,
    ) -> Result<(), ResidentKernelError> {
        match self {
            Self::Mask { values, .. } => {
                let mut ordinal = 0;
                for (position, selected) in values.iter().copied().enumerate() {
                    if selected != 0 {
                        visitor(ordinal, position)?;
                        ordinal += 1;
                    }
                }
                Ok(())
            }
            Self::Indices(indices) => indices.try_for_each_position(visitor),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct NormalizedWritePlan {
    last_source_by_destination: Box<[Option<usize>]>,
}

fn normalized_write_plan_cost(
    positions: ValidatedPositions<'_>,
    output_len: usize,
) -> Result<(usize, usize), ResidentKernelError> {
    let scan_work = positions
        .traversal_len()
        .checked_mul(output_len)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let compute_work = scan_work
        .checked_add(positions.traversal_len())
        .and_then(|work| {
            output_len
                .checked_mul(3)
                .and_then(|tail| work.checked_add(tail))
        })
        .ok_or(ResidentKernelError::InvalidShape)?;
    let index_bytes = output_len
        .checked_mul(core::mem::size_of::<Option<usize>>())
        .ok_or(ResidentKernelError::InvalidShape)?;
    Ok((compute_work, index_bytes))
}

fn last_source_for_destination(
    positions: ValidatedPositions<'_>,
    destination: usize,
    mut source_index: impl FnMut(usize, usize) -> usize,
) -> Result<Option<usize>, ResidentKernelError> {
    let mut last = None;
    positions.try_for_each(|ordinal, position| {
        if position == destination {
            last = Some(source_index(ordinal, position));
        }
        Ok(())
    })?;
    Ok(last)
}

fn materialize_normalized_last_write_sources(
    positions: ValidatedPositions<'_>,
    output_len: usize,
    mut source_index: impl FnMut(usize, usize) -> usize,
) -> Result<NormalizedWritePlan, ResidentKernelError> {
    let mut last_writes = vec![None; output_len];
    positions.try_for_each(|ordinal, position| {
        last_writes[position] = Some(source_index(ordinal, position));
        Ok(())
    })?;
    Ok(NormalizedWritePlan {
        last_source_by_destination: last_writes.into_boxed_slice(),
    })
}

fn replace_f64(output: &mut [f64], mut next: impl FnMut(usize) -> f64) -> bool {
    let mut changed = false;
    for (index, target) in output.iter_mut().enumerate() {
        let value = next(index);
        changed |= target.to_bits() != value.to_bits();
        *target = value;
    }
    changed
}

fn as_f64_array<const N: usize>(values: &[f64]) -> Result<&[f64; N], ResidentKernelError> {
    values
        .try_into()
        .map_err(|_| ResidentKernelError::InvalidShape)
}

fn f64_scalar(inputs: &dyn ResidentKernelInputs, index: usize) -> Result<f64, ResidentKernelError> {
    let [value] = f64_input(inputs, index)? else {
        return Err(ResidentKernelError::InvalidShape);
    };
    Ok(*value)
}

fn bool_scalar(
    inputs: &dyn ResidentKernelInputs,
    index: usize,
) -> Result<bool, ResidentKernelError> {
    let ResidentValueRef::Bool(values) = input(inputs, index)? else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let [value] = values else {
        return Err(ResidentKernelError::InvalidShape);
    };
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(ResidentKernelError::InvalidInput),
    }
}

fn write_f64_array<const N: usize>(
    output: &mut [f64],
    next: [f64; N],
) -> Result<bool, ResidentKernelError> {
    if output.len() != N {
        return Err(ResidentKernelError::InvalidShape);
    }
    let changed = output
        .iter()
        .zip(next)
        .any(|(left, right)| left.to_bits() != right.to_bits());
    output.copy_from_slice(&next);
    Ok(changed)
}

fn write_bool(output: ResidentValueMut<'_>, next: bool) -> Result<bool, ResidentKernelError> {
    let ResidentValueMut::Bool(output) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let [output] = output else {
        return Err(ResidentKernelError::InvalidShape);
    };
    let next = u8::from(next);
    let changed = *output != next;
    *output = next;
    Ok(changed)
}

fn semantic_comparison_result<T: PartialEq + PartialOrd>(
    comparison: SemanticComparison,
    left: &T,
    right: &T,
) -> bool {
    match comparison {
        SemanticComparison::Equal => left == right,
        SemanticComparison::NotEqual => left != right,
        SemanticComparison::Less => left < right,
        SemanticComparison::LessEqual => left <= right,
        SemanticComparison::Greater => left > right,
        SemanticComparison::GreaterEqual => left >= right,
    }
}

fn dense_comparison_slices<T: PartialEq + PartialOrd>(
    kernel: &BoundResidentKernel,
    left: &[T],
    right: &[T],
    output: &mut [u8],
) -> Result<bool, ResidentKernelError> {
    let [rows, columns, left_mode, right_mode, comparison] = kernel.parameters() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let rows = usize::try_from(*rows).map_err(|_| ResidentKernelError::InvalidShape)?;
    let columns = usize::try_from(*columns).map_err(|_| ResidentKernelError::InvalidShape)?;
    let comparison =
        SemanticComparison::from_parameter(*comparison).ok_or(ResidentKernelError::InvalidInput)?;
    if output.len()
        != rows
            .checked_mul(columns)
            .ok_or(ResidentKernelError::InvalidShape)?
    {
        return Err(ResidentKernelError::InvalidShape);
    }
    validate_binary_broadcast_len(left.len(), *left_mode, rows, columns)?;
    validate_binary_broadcast_len(right.len(), *right_mode, rows, columns)?;
    let mut changed = false;
    for (index, target) in output.iter_mut().enumerate() {
        let left = &left[binary_broadcast_index(*left_mode, index, rows)];
        let right = &right[binary_broadcast_index(*right_mode, index, rows)];
        let next = u8::from(semantic_comparison_result(comparison, left, right));
        changed |= *target != next;
        *target = next;
    }
    Ok(changed)
}

fn admit_dense_string_comparison(
    kernel: &BoundResidentKernel,
    left: &[String],
    right: &[String],
    output_len: usize,
) -> Result<(), ResidentKernelError> {
    let [rows, columns, left_mode, right_mode, _comparison] = kernel.parameters() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let rows = usize::try_from(*rows).map_err(|_| ResidentKernelError::InvalidShape)?;
    let columns = usize::try_from(*columns).map_err(|_| ResidentKernelError::InvalidShape)?;
    let expected = rows
        .checked_mul(columns)
        .ok_or(ResidentKernelError::InvalidShape)?;
    if output_len != expected {
        return Err(ResidentKernelError::InvalidShape);
    }
    validate_binary_broadcast_len(left.len(), *left_mode, rows, columns)?;
    validate_binary_broadcast_len(right.len(), *right_mode, rows, columns)?;

    // Reserve the complete byte-scan cost before the first comparison writes
    // an output lane. The incremental meter bounds this planning traversal
    // itself, while broadcast multiplicity charges scalar and row/column
    // values once for every comparison they participate in.
    let mut meter = super::budget::ResidentBudgetMeter::default();
    for index in 0..output_len {
        let left = &left[binary_broadcast_index(*left_mode, index, rows)];
        let right = &right[binary_broadcast_index(*right_mode, index, rows)];
        meter.charge_comparison_work(super::budget::checked_u64(
            left.len().max(right.len()).max(1),
        )?)?;
    }
    let mut cost = meter.estimate();
    cost.set_output_elements(super::budget::checked_u64(output_len)?);
    cost.set_output_bytes(super::budget::checked_u64(output_len)?);
    super::budget::PreparedKernel::new((), cost)
        .admit()?
        .into_plan();
    Ok(())
}

fn binary_broadcast_index(mode: u64, index: usize, rows: usize) -> usize {
    match mode {
        BINARY_BROADCAST_SCALAR => 0,
        BINARY_BROADCAST_EXACT => index,
        BINARY_BROADCAST_COLUMN => index % rows,
        BINARY_BROADCAST_ROW => index / rows,
        _ => unreachable!("validated binary broadcast mode"),
    }
}

fn dense_comparison(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let ResidentValueMut::Bool(output) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let [rows, columns, ..] = kernel.parameters() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    admit_dense_comparison_layout(ResidentShape {
        rows: u32::try_from(*rows).map_err(|_| ResidentKernelError::InvalidShape)?,
        columns: u32::try_from(*columns).map_err(|_| ResidentKernelError::InvalidShape)?,
    })?;
    match (inputs.get(0), inputs.get(1)) {
        (Some(ResidentValueRef::Bool(left)), Some(ResidentValueRef::Bool(right))) => {
            if left.iter().chain(right).any(|value| *value > 1) {
                return Err(ResidentKernelError::InvalidInput);
            }
            dense_comparison_slices(kernel, left, right, output)
        }
        (Some(ResidentValueRef::Index(left)), Some(ResidentValueRef::Index(right))) => {
            dense_comparison_slices(kernel, left, right, output)
        }
        (Some(ResidentValueRef::String(left)), Some(ResidentValueRef::String(right))) => {
            admit_dense_string_comparison(kernel, left, right, output.len())?;
            dense_comparison_slices(kernel, left, right, output)
        }
        _ => Err(ResidentKernelError::InvalidInput),
    }
}

enum SnapshotComparisonValues<'a> {
    Scalar(&'a ValueData),
    Matrix(Vec<ValueData>),
}

impl SnapshotComparisonValues<'_> {
    fn len(&self) -> usize {
        match self {
            Self::Scalar(_) => 1,
            Self::Matrix(values) => values.len(),
        }
    }

    fn get(&self, index: usize) -> Option<&ValueData> {
        match self {
            Self::Scalar(value) if index == 0 => Some(value),
            Self::Scalar(_) => None,
            Self::Matrix(values) => values.get(index),
        }
    }
}

fn snapshot_comparison(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let (
        Some(ResidentValueRef::Snapshot([Some(left)])),
        Some(ResidentValueRef::Snapshot([Some(right)])),
    ) = (inputs.get(0), inputs.get(1))
    else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let ResidentValueMut::Bool(output) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let [rows, columns, left_mode, right_mode, comparison] = kernel.parameters() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let rows = usize::try_from(*rows).map_err(|_| ResidentKernelError::InvalidShape)?;
    let columns = usize::try_from(*columns).map_err(|_| ResidentKernelError::InvalidShape)?;
    let output_len = rows
        .checked_mul(columns)
        .ok_or(ResidentKernelError::InvalidShape)?;
    if output.len() != output_len {
        return Err(ResidentKernelError::InvalidShape);
    }
    let comparison =
        SemanticComparison::from_parameter(*comparison).ok_or(ResidentKernelError::InvalidInput)?;
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let left_schema = left
        .validate_against(schemas)
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    let right_schema = right
        .validate_against(schemas)
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    let (left_element, left_count, left_matrix) = match (left_schema.body(), left.data()) {
        (SchemaBody::Matrix { element, .. }, ValueData::Matrix(matrix)) => (
            element.as_ref(),
            matrix.elements().len(),
            Some(matrix.elements()),
        ),
        (body, _) => (body, 1, None),
    };
    let (right_element, right_count, right_matrix) = match (right_schema.body(), right.data()) {
        (SchemaBody::Matrix { element, .. }, ValueData::Matrix(matrix)) => (
            element.as_ref(),
            matrix.elements().len(),
            Some(matrix.elements()),
        ),
        (body, _) => (body, 1, None),
    };
    if left_element != right_element
        || !snapshot_comparison_element_supported(left_element, comparison)
    {
        return Err(ResidentKernelError::InvalidInput);
    }
    validate_binary_broadcast_len(left_count, *left_mode, rows, columns)?;
    validate_binary_broadcast_len(right_count, *right_mode, rows, columns)?;

    let staged_elements = left_matrix
        .map_or(0, SequenceView::len)
        .checked_add(right_matrix.map_or(0, SequenceView::len))
        .ok_or(ResidentKernelError::InvalidShape)?;
    let mut footprint_meter = super::budget::ResidentBudgetMeter::default();
    let (left_bytes, left_nodes) = snapshot_clone_cost(&mut footprint_meter, left, schemas)?;
    let (right_bytes, right_nodes) = snapshot_clone_cost(&mut footprint_meter, right, schemas)?;
    let cloned_bytes = left_bytes
        .checked_add(right_bytes)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let retained_nodes = left_nodes
        .checked_add(right_nodes)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let comparison_index = |mode: u64, index: usize| {
        let row = index % rows;
        let column = index / rows;
        match mode {
            BINARY_BROADCAST_SCALAR => 0,
            BINARY_BROADCAST_EXACT => row * columns + column,
            BINARY_BROADCAST_COLUMN => row,
            BINARY_BROADCAST_ROW => column,
            _ => unreachable!("validated snapshot broadcast mode"),
        }
    };
    // Measure every comparison occurrence, including repeated scalar/row/
    // column broadcasts. Recursive planning is incrementally metered and the
    // actual comparison scan is charged once per output pair.
    for index in 0..output_len {
        let left_work = snapshot_element_comparison_work(
            &mut footprint_meter,
            left_schema.body(),
            left.data(),
            comparison_index(*left_mode, index),
        )?;
        let right_work = snapshot_element_comparison_work(
            &mut footprint_meter,
            right_schema.body(),
            right.data(),
            comparison_index(*right_mode, index),
        )?;
        footprint_meter.charge_comparison_work(left_work.max(right_work).max(1))?;
    }
    let footprint_work = footprint_meter.estimate();
    let container_bytes = super::budget::checked_u64(
        staged_elements
            .checked_mul(core::mem::size_of::<ValueData>())
            .ok_or(ResidentKernelError::InvalidShape)?,
    )?;
    super::budget::PreparedKernel::new(
        (),
        super::budget::resident_cost! {
            comparison_work: footprint_work.comparison_work(),
            compute_work: footprint_work.compute_work()
                .checked_add(super::budget::checked_u64(output_len)?)
                .ok_or(ResidentKernelError::InvalidShape)?,
            output_elements: output_len,
            output_bytes: output_len,
            temporary_bytes: cloned_bytes
                .checked_add(container_bytes)
                .ok_or(ResidentKernelError::InvalidShape)?,
            cloned_bytes,
            retained_nodes,
            ..super::budget::KernelCostEstimate::default()
        },
    )
    .admit()?
    .into_plan();

    let left_values = match left_matrix {
        Some(values) => SnapshotComparisonValues::Matrix(values.to_values()),
        None => SnapshotComparisonValues::Scalar(left.data()),
    };
    let right_values = match right_matrix {
        Some(values) => SnapshotComparisonValues::Matrix(values.to_values()),
        None => SnapshotComparisonValues::Scalar(right.data()),
    };
    debug_assert_eq!(left_values.len(), left_count);
    debug_assert_eq!(right_values.len(), right_count);
    let mut changed = false;
    for (index, target) in output.iter_mut().enumerate() {
        let left = left_values
            .get(comparison_index(*left_mode, index))
            .ok_or(ResidentKernelError::InvalidShape)?;
        let right = right_values
            .get(comparison_index(*right_mode, index))
            .ok_or(ResidentKernelError::InvalidShape)?;
        let equal = schema_data_language_eq(left_element, left, right);
        let ordered = || schema_data_partial_cmp(left_element, left, right);
        let next = match comparison {
            SemanticComparison::Equal => equal,
            SemanticComparison::NotEqual => !equal,
            SemanticComparison::Less => matches!(ordered(), Some(core::cmp::Ordering::Less)),
            SemanticComparison::LessEqual => matches!(
                ordered(),
                Some(core::cmp::Ordering::Less | core::cmp::Ordering::Equal)
            ),
            SemanticComparison::Greater => {
                matches!(ordered(), Some(core::cmp::Ordering::Greater))
            }
            SemanticComparison::GreaterEqual => matches!(
                ordered(),
                Some(core::cmp::Ordering::Greater | core::cmp::Ordering::Equal)
            ),
        };
        let next = u8::from(next);
        changed |= *target != next;
        *target = next;
    }
    Ok(changed)
}

fn hold_state(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 1 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let source = input(inputs, 0)?;
    if let ResidentValueRef::Bool(values) = source
        && values.iter().any(|value| *value > 1)
    {
        return Err(ResidentKernelError::InvalidInput);
    }
    match (source, output) {
        (ResidentValueRef::Bool(source), ResidentValueMut::Bool(target))
            if source.len() == target.len() =>
        {
            let changed = source != target;
            target.copy_from_slice(source);
            Ok(changed)
        }
        (ResidentValueRef::Index(source), ResidentValueMut::Index(target))
            if source.len() == target.len() =>
        {
            let changed = source != target;
            target.copy_from_slice(source);
            Ok(changed)
        }
        (ResidentValueRef::F64(source), ResidentValueMut::F64(target))
            if source.len() == target.len() =>
        {
            let changed = source
                .iter()
                .zip(target.iter())
                .any(|(left, right)| left.to_bits() != right.to_bits());
            target.copy_from_slice(source);
            Ok(changed)
        }
        (ResidentValueRef::String(source), ResidentValueMut::String(target))
            if source.len() == target.len() =>
        {
            let payload_bytes = source.iter().try_fold(0u64, |total, value| {
                total
                    .checked_add(super::budget::checked_u64(value.len())?)
                    .ok_or(ResidentKernelError::InvalidShape)
            })?;
            let container_bytes = super::budget::checked_u64(
                source
                    .len()
                    .checked_mul(core::mem::size_of::<String>())
                    .ok_or(ResidentKernelError::InvalidShape)?,
            )?;
            super::budget::PreparedKernel::new(
                (),
                super::budget::resident_cost! {
                    compute_work: source.len(),
                    output_elements: source.len(),
                    output_bytes: payload_bytes
                        .checked_add(container_bytes)
                        .ok_or(ResidentKernelError::InvalidShape)?,
                    temporary_bytes: payload_bytes,
                    container_bytes,
                    cloned_bytes: payload_bytes,
                    retained_nodes: source.len().saturating_add(1),
                    ..super::budget::KernelCostEstimate::default()
                },
            )
            .admit()?
            .into_plan();
            let changed = source != target;
            target.clone_from_slice(source);
            Ok(changed)
        }
        (ResidentValueRef::Snapshot(source), ResidentValueMut::Snapshot(target))
            if source.len() == target.len() =>
        {
            let schemas = kernel
                .snapshot_schemas()
                .ok_or(ResidentKernelError::InvalidInput)?;
            let mut footprint_meter = super::budget::ResidentBudgetMeter::default();
            let source_footprint =
                snapshot_lane_clone_footprint(&mut footprint_meter, source, schemas)?;
            let target_footprint =
                snapshot_lane_clone_footprint(&mut footprint_meter, target, schemas)
                    .map_err(|_| ResidentKernelError::InvalidOutput)?;
            let mut equality_work = source_footprint
                .encoded_bytes
                .max(source_footprint.node_count)
                .checked_add(
                    target_footprint
                        .encoded_bytes
                        .max(target_footprint.node_count),
                )
                .ok_or(ResidentKernelError::InvalidShape)?;
            for (source, target) in source.iter().zip(target.iter()) {
                let (Some(source), Some(target)) = (source, target) else {
                    continue;
                };
                if source.schema_key() == target.schema_key() {
                    let source_entry = schemas
                        .entry(source.schema())
                        .ok_or(ResidentKernelError::InvalidInput)?;
                    let target_entry = schemas
                        .entry(target.schema())
                        .ok_or(ResidentKernelError::InvalidOutput)?;
                    equality_work = equality_work
                        .checked_add(super::budget::checked_u64(
                            source_entry
                                .canonical_bytes()
                                .len()
                                .max(target_entry.canonical_bytes().len()),
                        )?)
                        .and_then(|work| {
                            work.checked_add(
                                super::budget::checked_u64(
                                    source
                                        .shape()
                                        .parameter_values()
                                        .len()
                                        .max(target.shape().parameter_values().len()),
                                )
                                .ok()?,
                            )
                        })
                        .ok_or(ResidentKernelError::InvalidShape)?;
                }
            }
            let container_bytes = super::budget::checked_u64(
                source
                    .len()
                    .checked_mul(core::mem::size_of::<Option<mech_core::Value>>())
                    .ok_or(ResidentKernelError::InvalidShape)?,
            )?;
            let measured = footprint_meter.estimate();
            super::budget::PreparedKernel::new(
                (),
                super::budget::resident_cost! {
                    comparison_work: measured.comparison_work()
                        .checked_add(equality_work)
                        .ok_or(ResidentKernelError::InvalidShape)?,
                    compute_work: measured.compute_work()
                        .checked_add(equality_work)
                        .and_then(|work| work.checked_add(super::budget::checked_u64(source.len()).ok()?))
                        .ok_or(ResidentKernelError::InvalidShape)?,
                    output_elements: source.len(),
                    output_bytes: source_footprint.retained_bytes
                        .checked_add(container_bytes)
                        .ok_or(ResidentKernelError::InvalidShape)?,
                    temporary_bytes: source_footprint.retained_bytes,
                    container_bytes,
                    cloned_bytes: source_footprint.retained_bytes,
                    retained_nodes: source_footprint.node_count
                        .checked_mul(2)
                        .and_then(|nodes| nodes.checked_add(target_footprint.node_count))
                        .ok_or(ResidentKernelError::InvalidShape)?,
                    ..super::budget::KernelCostEstimate::default()
                },
            )
            .admit()?
            .into_plan();
            let mut changed = false;
            for (source, target) in source.iter().zip(target.iter()) {
                let equal = match (source, target) {
                    (None, None) => true,
                    (Some(source), Some(target)) => source
                        .language_eq(schemas, target, schemas)
                        .map_err(|_| ResidentKernelError::InvalidInput)?,
                    _ => false,
                };
                changed |= !equal;
            }
            target.clone_from_slice(source);
            Ok(changed)
        }
        _ => Err(ResidentKernelError::InvalidShape),
    }
}

fn indexed_assign(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 2 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let [output_len, source_routing] = kernel.parameters() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let output_len = usize::try_from(*output_len).map_err(|_| ResidentKernelError::InvalidShape)?;
    let source_routing = ResolvedSourceRouting::from_parameter(*source_routing)
        .ok_or(ResidentKernelError::InvalidInput)?;
    if output.len() != output_len {
        return Err(ResidentKernelError::InvalidShape);
    }
    let positions = ValidatedPositions::new(input(inputs, 1)?, output_len)?;
    let source = input(inputs, 0)?;
    if let ResidentValueRef::Bool(values) = source
        && values.iter().any(|value| *value > 1)
    {
        return Err(ResidentKernelError::InvalidInput);
    }
    let source_index = |ordinal: usize, position: usize| match source_routing {
        ResolvedSourceRouting::ScalarBroadcast => 0,
        ResolvedSourceRouting::Positional => position,
        ResolvedSourceRouting::CompactSelectionOrder => ordinal,
    };
    let source_shape_valid = match source_routing {
        ResolvedSourceRouting::ScalarBroadcast => source.len() == 1,
        ResolvedSourceRouting::Positional => positions
            .maximum_position()
            .is_none_or(|position| position < source.len()),
        ResolvedSourceRouting::CompactSelectionOrder => source.len() == positions.len(),
    };
    if !source_shape_valid {
        return Err(ResidentKernelError::InvalidShape);
    }
    // The declared RMW contract permits the candidate to alias only the base
    // input consumed by the execution layer. The source and selector lanes are
    // immutable and cannot alias this output, so the validated plan is safe to
    // replay without staging heap copies.
    match (source, output) {
        (ResidentValueRef::Bool(source), ResidentValueMut::Bool(output)) => {
            if positions.len() == 0 {
                return Ok(false);
            }
            let mut changed = false;
            positions.try_for_each(|ordinal, position| {
                let next = source[source_index(ordinal, position)];
                changed |= output[position] != next;
                output[position] = next;
                Ok::<(), ResidentKernelError>(())
            })?;
            Ok(changed)
        }
        (ResidentValueRef::Index(source), ResidentValueMut::Index(output)) => {
            if positions.len() == 0 {
                return Ok(false);
            }
            let mut changed = false;
            positions.try_for_each(|ordinal, position| {
                let next = source[source_index(ordinal, position)];
                changed |= output[position] != next;
                output[position] = next;
                Ok::<(), ResidentKernelError>(())
            })?;
            Ok(changed)
        }
        (ResidentValueRef::F64(source), ResidentValueMut::F64(output)) => {
            if positions.len() == 0 {
                return Ok(false);
            }
            let mut changed = false;
            positions.try_for_each(|ordinal, position| {
                let next = source[source_index(ordinal, position)];
                changed |= output[position].to_bits() != next.to_bits();
                output[position] = next;
                Ok::<(), ResidentKernelError>(())
            })?;
            Ok(changed)
        }
        (ResidentValueRef::String(source), ResidentValueMut::String(output)) => {
            if positions.len() == 0 {
                return Ok(false);
            }
            let (compute_work, index_bytes) = normalized_write_plan_cost(positions, output_len)?;
            let mut output_payload = 0u64;
            let mut publication_work = 0u64;
            for (destination, current) in output.iter().enumerate() {
                let selected = last_source_for_destination(positions, destination, source_index)?;
                let next = selected
                    .map(|source_index| &source[source_index])
                    .unwrap_or(current);
                output_payload = output_payload
                    .checked_add(super::budget::checked_u64(next.len())?)
                    .ok_or(ResidentKernelError::InvalidShape)?;
                publication_work = publication_work
                    .checked_add(super::budget::checked_u64(current.len().max(next.len()))?)
                    .ok_or(ResidentKernelError::InvalidShape)?;
            }
            let container_bytes = super::budget::checked_u64(
                output_len
                    .checked_mul(core::mem::size_of::<String>())
                    .ok_or(ResidentKernelError::InvalidShape)?,
            )?;
            let plan_nodes = super::budget::checked_u64(output_len)?
                .checked_add(1)
                .ok_or(ResidentKernelError::InvalidShape)?;
            let admitted = super::budget::PreparedMutationPlan::new(
                (),
                super::budget::PublishedOutputFootprint {
                    elements: super::budget::checked_u64(output_len)?,
                    retained_bytes: output_payload
                        .checked_add(container_bytes)
                        .ok_or(ResidentKernelError::InvalidShape)?,
                    retained_nodes: super::budget::checked_u64(output_len)?
                        .checked_add(1)
                        .ok_or(ResidentKernelError::InvalidShape)?,
                },
                super::budget::MutationRetainedNodeFootprint {
                    current_persistent: super::budget::checked_u64(output_len)?
                        .checked_add(1)
                        .ok_or(ResidentKernelError::InvalidShape)?,
                    normalized_plan: plan_nodes,
                    temporary_draft: super::budget::checked_u64(output_len)?
                        .checked_add(1)
                        .ok_or(ResidentKernelError::InvalidShape)?,
                },
                super::budget::resident_cost! {
                    comparison_work: publication_work,
                    compute_work: super::budget::checked_u64(compute_work)?
                        .checked_add(publication_work)
                        .ok_or(ResidentKernelError::InvalidShape)?,
                    temporary_bytes: output_payload,
                    container_bytes,
                    cloned_bytes: output_payload,
                    index_bytes,
                    ..super::budget::KernelCostEstimate::default()
                },
            )?
            .admit()?;
            admitted.into_plan();
            let plan =
                materialize_normalized_last_write_sources(positions, output_len, source_index)?;
            let mut changed = false;
            let staged = output
                .iter()
                .zip(plan.last_source_by_destination)
                .map(|(current, selected)| {
                    let next = selected
                        .map(|source_index| source[source_index].clone())
                        .unwrap_or_else(|| current.clone());
                    changed |= current != &next;
                    next
                })
                .collect::<Vec<_>>();
            for (target, next) in output.iter_mut().zip(staged) {
                *target = next;
            }
            Ok(changed)
        }
        (ResidentValueRef::Snapshot(source), ResidentValueMut::Snapshot(output)) => {
            if positions.len() == 0 {
                return Ok(false);
            }
            let schemas = kernel
                .snapshot_schemas()
                .ok_or(ResidentKernelError::InvalidInput)?;
            let (compute_work, index_bytes) = normalized_write_plan_cost(positions, output_len)?;
            let mut meter = super::budget::ResidentBudgetMeter::default();
            let mut current_footprint = ValueFootprint::zero();
            for current in output.iter().flatten() {
                current_footprint = current_footprint
                    .checked_add(super::budget::measure_canonical_value_footprint(
                        &mut meter, current, schemas,
                    )?)
                    .map_err(|_| ResidentKernelError::InvalidShape)?;
            }
            let mut final_footprint = ValueFootprint::zero();
            let mut publication_equality_work = 0u64;
            for (destination, current) in output.iter().enumerate() {
                let selected = last_source_for_destination(positions, destination, source_index)?;
                let next = selected
                    .map(|source_index| &source[source_index])
                    .unwrap_or(current);
                if let Some(next) = next {
                    let next_footprint = super::budget::measure_canonical_value_footprint(
                        &mut meter, next, schemas,
                    )?;
                    final_footprint = final_footprint
                        .checked_add(next_footprint)
                        .map_err(|_| ResidentKernelError::InvalidShape)?;
                    if let Some(current) = current {
                        let current_footprint = super::budget::measure_canonical_value_footprint(
                            &mut meter, current, schemas,
                        )?;
                        publication_equality_work = publication_equality_work
                            .checked_add(super::budget::projected_language_equality_work(
                                schemas,
                                current,
                                current_footprint,
                                next.schema(),
                                next.shape().parameter_values().len(),
                                next_footprint,
                            )?)
                            .ok_or(ResidentKernelError::InvalidShape)?;
                    }
                }
            }
            let container_bytes = super::budget::checked_u64(
                output_len
                    .checked_mul(core::mem::size_of::<Option<mech_core::Value>>())
                    .ok_or(ResidentKernelError::InvalidShape)?,
            )?;
            let plan_nodes = super::budget::checked_u64(output_len)?
                .checked_add(1)
                .ok_or(ResidentKernelError::InvalidShape)?;
            let admitted = super::budget::PreparedMutationPlan::new(
                (),
                super::budget::PublishedOutputFootprint {
                    elements: super::budget::checked_u64(output_len)?,
                    retained_bytes: final_footprint
                        .retained_bytes
                        .checked_add(container_bytes)
                        .ok_or(ResidentKernelError::InvalidShape)?,
                    retained_nodes: final_footprint
                        .node_count
                        .checked_add(super::budget::checked_u64(output_len)?)
                        .and_then(|nodes| nodes.checked_add(1))
                        .ok_or(ResidentKernelError::InvalidShape)?,
                },
                super::budget::MutationRetainedNodeFootprint {
                    current_persistent: current_footprint
                        .node_count
                        .checked_add(super::budget::checked_u64(output_len)?)
                        .and_then(|nodes| nodes.checked_add(1))
                        .ok_or(ResidentKernelError::InvalidShape)?,
                    normalized_plan: plan_nodes,
                    temporary_draft: final_footprint
                        .node_count
                        .checked_add(super::budget::checked_u64(output_len)?)
                        .and_then(|nodes| nodes.checked_add(1))
                        .ok_or(ResidentKernelError::InvalidShape)?,
                },
                {
                    let measured = meter.estimate();
                    super::budget::resident_cost! {
                    comparison_work: measured.comparison_work()
                        .checked_add(publication_equality_work)
                        .ok_or(ResidentKernelError::InvalidShape)?,
                    compute_work: measured.compute_work()
                        .checked_add(super::budget::checked_u64(compute_work)?)
                        .and_then(|work| work.checked_add(publication_equality_work))
                        .ok_or(ResidentKernelError::InvalidShape)?,
                    temporary_bytes: final_footprint.retained_bytes,
                    container_bytes,
                    cloned_bytes: final_footprint.retained_bytes,
                    index_bytes,
                    ..super::budget::KernelCostEstimate::default()
                    }
                },
            )?
            .admit()?;
            let mut changed = false;
            admitted.into_plan();
            let plan =
                materialize_normalized_last_write_sources(positions, output_len, source_index)?;
            let staged = output
                .iter()
                .zip(plan.last_source_by_destination)
                .map(|(current, selected)| {
                    let next = selected
                        .map(|source_index| source[source_index].clone())
                        .unwrap_or_else(|| current.clone());
                    let equal = match (current, next.as_ref()) {
                        (None, None) => true,
                        (Some(current), Some(next)) => current
                            .language_eq(schemas, next, schemas)
                            .map_err(|_| ResidentKernelError::InvalidOutput)?,
                        _ => false,
                    };
                    changed |= !equal;
                    Ok(next)
                })
                .collect::<Result<Vec<_>, ResidentKernelError>>()?;
            for (target, next) in output.iter_mut().zip(staged) {
                *target = next;
            }
            Ok(changed)
        }
        _ => Err(ResidentKernelError::InvalidShape),
    }
}

fn indexed_assign_snapshot(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 2 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let [rows, columns, source_rows, source_columns, source_routing] = kernel.parameters() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let rows = usize::try_from(*rows).map_err(|_| ResidentKernelError::InvalidShape)?;
    let columns = usize::try_from(*columns).map_err(|_| ResidentKernelError::InvalidShape)?;
    let source_rows =
        usize::try_from(*source_rows).map_err(|_| ResidentKernelError::InvalidShape)?;
    let source_columns =
        usize::try_from(*source_columns).map_err(|_| ResidentKernelError::InvalidShape)?;
    let source_routing = ResolvedSourceRouting::from_parameter(*source_routing)
        .ok_or(ResidentKernelError::InvalidInput)?;
    let output_len = rows
        .checked_mul(columns)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let positions = ValidatedPositions::new(input(inputs, 1)?, output_len)?;
    let Some(ResidentValueRef::Snapshot([Some(source)])) = inputs.get(0) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let source_len = match source.data() {
        ValueData::Matrix(matrix) => matrix.elements().len(),
        _ => 1,
    };
    if source_rows.checked_mul(source_columns) != Some(source_len) {
        return Err(ResidentKernelError::InvalidShape);
    }
    let source_shape_valid = match source_routing {
        ResolvedSourceRouting::ScalarBroadcast => source_len == 1,
        ResolvedSourceRouting::Positional => positions
            .maximum_position()
            .is_none_or(|position| position < source_len),
        ResolvedSourceRouting::CompactSelectionOrder => source_len == positions.len(),
    };
    if !source_shape_valid {
        return Err(ResidentKernelError::InvalidShape);
    }
    let ResidentValueMut::Snapshot([target]) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let current = target.as_ref().ok_or(ResidentKernelError::InvalidOutput)?;
    let ValueData::Matrix(current_matrix) = current.data() else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    if current_matrix.elements().len() != output_len {
        return Err(ResidentKernelError::InvalidShape);
    }
    if positions.len() == 0 {
        return Ok(false);
    }
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let mut footprint_meter = super::budget::ResidentBudgetMeter::default();
    let current_footprint =
        super::budget::measure_canonical_value_footprint(&mut footprint_meter, current, schemas)
            .map_err(|_| ResidentKernelError::InvalidOutput)?;
    let source_footprint =
        super::budget::measure_canonical_value_footprint(&mut footprint_meter, source, schemas)?;
    let current_bytes = current_footprint.retained_bytes;
    let current_nodes = current_footprint.node_count;
    let source_bytes = source_footprint.retained_bytes;
    let source_nodes = source_footprint.node_count;
    let current_schema = current
        .validate_against(schemas)
        .map_err(|_| ResidentKernelError::InvalidOutput)?;
    let source_schema = source
        .validate_against(schemas)
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    let canonical_source_index = |position: usize| {
        let row = position % source_rows;
        let column = position / source_rows;
        row * source_columns + column
    };
    let mut execution_clone_footprint = ValueFootprint::zero();
    let (final_output_footprint, finalization_work) = projected_snapshot_matrix_output(
        &mut footprint_meter,
        current_schema.body(),
        current.data(),
        current_footprint,
        source_schema.body(),
        source.data(),
        output_len,
        |destination, meter| {
            meter.charge_compute_work(super::budget::checked_u64(positions.traversal_len())?)?;
            last_source_for_destination(positions, destination, |ordinal, position| {
                match source_routing {
                    ResolvedSourceRouting::ScalarBroadcast => 0,
                    ResolvedSourceRouting::Positional => canonical_source_index(position),
                    ResolvedSourceRouting::CompactSelectionOrder => ordinal,
                }
            })
        },
    )?;
    positions.try_for_each(|ordinal, position| {
        let source_index = match source_routing {
            ResolvedSourceRouting::ScalarBroadcast => 0,
            ResolvedSourceRouting::Positional => canonical_source_index(position),
            ResolvedSourceRouting::CompactSelectionOrder => ordinal,
        };
        execution_clone_footprint = execution_clone_footprint
            .checked_add(snapshot_element_clone_footprint(
                &mut footprint_meter,
                source_schema.body(),
                source.data(),
                source_index,
            )?)
            .map_err(|_| ResidentKernelError::InvalidShape)?;
        Ok::<(), ResidentKernelError>(())
    })?;
    let cloned_bytes = current_bytes
        .checked_add(source_bytes)
        .and_then(|bytes| bytes.checked_add(execution_clone_footprint.retained_bytes))
        .ok_or(ResidentKernelError::InvalidShape)?;
    let container_bytes = super::budget::checked_u64(
        output_len
            .checked_add(source_len)
            .and_then(|count| count.checked_mul(core::mem::size_of::<ValueDataDraft>()))
            .ok_or(ResidentKernelError::InvalidShape)?,
    )?;
    let footprint_work = footprint_meter.estimate();
    let metadata = kernel
        .snapshot_output()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let publication_equality_work = super::budget::projected_language_equality_work(
        schemas,
        current,
        current_footprint,
        metadata.schema,
        metadata.shape.parameter_values().len(),
        final_output_footprint,
    )?;
    let cost = super::budget::resident_cost! {
        comparison_work: footprint_work.comparison_work()
            .checked_add(publication_equality_work)
            .ok_or(ResidentKernelError::InvalidShape)?,
        compute_work: footprint_work
            .compute_work()
            .checked_add(super::budget::checked_u64(
                positions.len(),
            )?)
            .and_then(|work| work.checked_add(publication_equality_work))
            .ok_or(ResidentKernelError::InvalidShape)?,
        temporary_bytes: cloned_bytes
            .checked_mul(2)
            .and_then(|bytes| bytes.checked_add(container_bytes))
            .ok_or(ResidentKernelError::InvalidShape)?,
        cloned_bytes,
        ..super::budget::KernelCostEstimate::default()
    };
    let canonicalization_work_limit = super::budget::PreparedMutationPlan::new(
        finalization_work,
        super::budget::PublishedOutputFootprint {
            elements: super::budget::checked_u64(output_len)?,
            retained_bytes: final_output_footprint.retained_bytes,
            retained_nodes: final_output_footprint.node_count,
        },
        super::budget::MutationRetainedNodeFootprint {
            current_persistent: current_nodes
                .checked_add(source_nodes)
                .ok_or(ResidentKernelError::InvalidShape)?,
            normalized_plan: 0,
            temporary_draft: current_nodes
                .checked_add(source_nodes)
                .and_then(|nodes| nodes.checked_add(execution_clone_footprint.node_count))
                .ok_or(ResidentKernelError::InvalidShape)?,
        },
        cost,
    )?
    .admit()?
    .into_plan();
    let ValueDataDraft::Matrix(mut next) = current
        .canonical_data_draft()
        .map_err(|_| ResidentKernelError::InvalidOutput)?
    else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let source = match source
        .canonical_data_draft()
        .map_err(|_| ResidentKernelError::InvalidInput)?
    {
        ValueDataDraft::Matrix(elements) => elements.into_vec(),
        scalar => vec![scalar],
    };
    let canonical_index = |position: usize| {
        let row = position % rows;
        let column = position / rows;
        row * columns + column
    };
    positions.try_for_each(|ordinal, position| {
        let source_index = match source_routing {
            ResolvedSourceRouting::ScalarBroadcast => 0,
            ResolvedSourceRouting::Positional => canonical_source_index(position),
            ResolvedSourceRouting::CompactSelectionOrder => ordinal,
        };
        let destination = canonical_index(position);
        next[destination] = source[source_index].clone();
        Ok::<(), ResidentKernelError>(())
    })?;
    let next = finalize_snapshot_data_with_work_budget(
        kernel,
        ValueDataDraft::Matrix(next),
        Some(canonicalization_work_limit),
    )?;
    let changed = !current
        .language_eq(schemas, &next, schemas)
        .map_err(|_| ResidentKernelError::InvalidOutput)?;
    *target = Some(next);
    Ok(changed)
}

fn matrix_selection_coordinates(
    plan: &MatrixSelectionAssignPlan,
    selectors: &[mech_core::Value],
) -> Result<(Vec<usize>, Vec<usize>), ResidentKernelError> {
    match plan.mode {
        ResolvedSelectionMode::Whole => Ok(((0..plan.rows).collect(), (0..plan.columns).collect())),
        ResolvedSelectionMode::Rows => Ok((
            access_indices(&selectors[0], plan.rows)?,
            (0..plan.columns).collect(),
        )),
        ResolvedSelectionMode::Columns => Ok((
            (0..plan.rows).collect(),
            access_indices(&selectors[0], plan.columns)?,
        )),
        ResolvedSelectionMode::Rectangle => Ok((
            access_indices(&selectors[0], plan.rows)?,
            access_indices(&selectors[1], plan.columns)?,
        )),
        ResolvedSelectionMode::LinearScalar
        | ResolvedSelectionMode::LinearGather
        | ResolvedSelectionMode::Field { .. }
        | ResolvedSelectionMode::TableColumn { .. }
        | ResolvedSelectionMode::MapKey => Err(ResidentKernelError::InvalidInput),
    }
}

fn matrix_selection_for_each_coordinate(
    plan: &MatrixSelectionAssignPlan,
    inputs: &dyn ResidentKernelInputs,
    mut visit: impl FnMut(usize, usize, usize) -> Result<(), ResidentKernelError>,
) -> Result<usize, ResidentKernelError> {
    let mut ordinal = 0usize;
    let mut emit = |row: usize, column: usize| {
        let current = ordinal;
        ordinal = ordinal
            .checked_add(1)
            .ok_or(ResidentKernelError::InvalidShape)?;
        visit(current, row, column)
    };
    let selector = |index: usize| {
        inputs
            .get(index + 1)
            .ok_or(ResidentKernelError::InvalidInput)
    };
    match plan.mode {
        ResolvedSelectionMode::Whole => {
            for row in 0..plan.rows {
                for column in 0..plan.columns {
                    emit(row, column)?;
                }
            }
        }
        ResolvedSelectionMode::Rows => {
            selector_for_each_access_index(selector(0)?, plan.rows, |row| {
                for column in 0..plan.columns {
                    emit(row, column)?;
                }
                Ok(())
            })?;
        }
        ResolvedSelectionMode::Columns => {
            for row in 0..plan.rows {
                selector_for_each_access_index(selector(0)?, plan.columns, |column| {
                    emit(row, column)
                })?;
            }
        }
        ResolvedSelectionMode::Rectangle => {
            selector_for_each_access_index(selector(0)?, plan.rows, |row| {
                selector_for_each_access_index(selector(1)?, plan.columns, |column| {
                    emit(row, column)
                })
            })?;
        }
        ResolvedSelectionMode::LinearScalar
        | ResolvedSelectionMode::LinearGather
        | ResolvedSelectionMode::Field { .. }
        | ResolvedSelectionMode::TableColumn { .. }
        | ResolvedSelectionMode::MapKey => return Err(ResidentKernelError::InvalidInput),
    }
    Ok(ordinal)
}

fn matrix_selection_last_source(
    plan: &MatrixSelectionAssignPlan,
    inputs: &dyn ResidentKernelInputs,
    destination: usize,
    snapshot_layout: bool,
    meter: &mut super::budget::ResidentBudgetMeter,
) -> Result<Option<usize>, ResidentKernelError> {
    let mut selected = None;
    matrix_selection_for_each_coordinate(plan, inputs, |ordinal, row, column| {
        meter.charge_compute_work(1)?;
        let candidate = if snapshot_layout {
            row.checked_mul(plan.columns)
                .and_then(|offset| offset.checked_add(column))
        } else {
            column
                .checked_mul(plan.rows)
                .and_then(|offset| offset.checked_add(row))
        }
        .ok_or(ResidentKernelError::InvalidShape)?;
        if candidate == destination {
            selected = Some(match plan.source_routing {
                ResolvedSourceRouting::ScalarBroadcast => 0,
                ResolvedSourceRouting::Positional => candidate,
                ResolvedSourceRouting::CompactSelectionOrder if snapshot_layout => ordinal,
                ResolvedSourceRouting::CompactSelectionOrder => {
                    dense_compact_source_index(ordinal, plan)?
                }
            });
        }
        Ok(())
    })?;
    Ok(selected)
}

fn dense_compact_source_index(
    ordinal: usize,
    plan: &MatrixSelectionAssignPlan,
) -> Result<usize, ResidentKernelError> {
    if plan.source_columns == 0 {
        return Err(ResidentKernelError::InvalidShape);
    }
    let row = ordinal / plan.source_columns;
    let column = ordinal % plan.source_columns;
    column
        .checked_mul(plan.source_rows)
        .and_then(|offset| offset.checked_add(row))
        .ok_or(ResidentKernelError::InvalidShape)
}

fn assign_dense_matrix_selection<T: Clone>(
    source: &[T],
    target: &mut [T],
    selected_rows: &[usize],
    selected_columns: &[usize],
    plan: &MatrixSelectionAssignPlan,
    changed: impl Fn(&T, &T) -> bool,
) -> Result<bool, ResidentKernelError> {
    let output_len = plan
        .rows
        .checked_mul(plan.columns)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let source_len = plan
        .source_rows
        .checked_mul(plan.source_columns)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let write_count = selected_rows
        .len()
        .checked_mul(selected_columns.len())
        .ok_or(ResidentKernelError::InvalidShape)?;
    if target.len() != output_len
        || source.len() != source_len
        || match plan.source_routing {
            ResolvedSourceRouting::ScalarBroadcast => source_len != 1,
            ResolvedSourceRouting::Positional => source_len != output_len,
            ResolvedSourceRouting::CompactSelectionOrder => source_len != write_count,
        }
    {
        return Err(ResidentKernelError::InvalidShape);
    }
    let mut staged = target.to_vec();
    let mut ordinal = 0usize;
    for row in selected_rows {
        for column in selected_columns {
            let destination = column
                .checked_mul(plan.rows)
                .and_then(|offset| offset.checked_add(*row))
                .filter(|destination| *destination < output_len)
                .ok_or(ResidentKernelError::InvalidShape)?;
            let source_index = match plan.source_routing {
                ResolvedSourceRouting::ScalarBroadcast => 0,
                ResolvedSourceRouting::Positional => destination,
                ResolvedSourceRouting::CompactSelectionOrder => {
                    dense_compact_source_index(ordinal, plan)?
                }
            };
            staged[destination] = source
                .get(source_index)
                .ok_or(ResidentKernelError::InvalidShape)?
                .clone();
            ordinal = ordinal
                .checked_add(1)
                .ok_or(ResidentKernelError::InvalidShape)?;
        }
    }
    let output_changed = target
        .iter()
        .zip(&staged)
        .any(|(current, next)| changed(current, next));
    target.clone_from_slice(&staged);
    Ok(output_changed)
}

fn matrix_assignment_clone_amplification(
    meter: &mut super::budget::ResidentBudgetMeter,
    source: ResidentValueRef<'_>,
    schemas: &mech_core::SchemaTable,
    write_count: usize,
) -> Result<(u64, u64), ResidentKernelError> {
    if write_count == 0 {
        return Ok((0, 0));
    }
    let multiplicity = super::budget::checked_u64(write_count)?;
    match source {
        ResidentValueRef::String(values) => {
            let maximum = values.iter().map(String::len).max().unwrap_or(0);
            Ok((
                super::budget::checked_u64(maximum)?
                    .checked_mul(multiplicity)
                    .ok_or(ResidentKernelError::InvalidShape)?,
                multiplicity,
            ))
        }
        ResidentValueRef::Snapshot([Some(value)]) => {
            let schema = value
                .validate_against(schemas)
                .map_err(|_| ResidentKernelError::InvalidInput)?;
            let maximum = match (schema.body(), value.data()) {
                (SchemaBody::Matrix { element, .. }, ValueData::Matrix(matrix)) => {
                    let values = matrix.elements();
                    let mut maximum = ValueFootprint::zero();
                    for index in 0..values.len() {
                        let mut next = ValueFootprint::zero();
                        selected_sequence_footprint(&mut next, meter, element, values, index)?;
                        maximum.retained_bytes = maximum.retained_bytes.max(next.retained_bytes);
                        maximum.node_count = maximum.node_count.max(next.node_count);
                    }
                    maximum
                }
                (body, data) => super::budget::measure_canonical_data_footprint(meter, body, data)?,
            };
            let amplified = maximum
                .checked_multiply(multiplicity)
                .map_err(|_| ResidentKernelError::InvalidShape)?;
            Ok((amplified.retained_bytes, amplified.node_count))
        }
        ResidentValueRef::Snapshot(_) => Err(ResidentKernelError::InvalidInput),
        ResidentValueRef::Bool(_) | ResidentValueRef::Index(_) | ResidentValueRef::F64(_) => {
            Ok((0, 0))
        }
    }
}

fn indexed_assign_matrix_selection(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let plan = kernel
        .retained_state::<MatrixSelectionAssignPlan>()
        .ok_or(ResidentKernelError::InvalidInput)?;
    if inputs.len() != plan.selectors.len() + 1 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let output_len = plan
        .rows
        .checked_mul(plan.columns)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let source_len = plan
        .source_rows
        .checked_mul(plan.source_columns)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let mut footprint_meter = super::budget::ResidentBudgetMeter::default();
    let value_cost = |meter: &mut super::budget::ResidentBudgetMeter,
                      value: ResidentValueRef<'_>|
     -> Result<(u64, u64, usize, u64), ResidentKernelError> {
        match value {
            ResidentValueRef::Snapshot([Some(value)]) => {
                let elements = match value.data() {
                    ValueData::Matrix(matrix) => matrix.elements().len(),
                    _ => 1,
                };
                let (retained, nodes) = snapshot_clone_cost(meter, value, schemas)?;
                let containers = super::budget::checked_u64(
                    elements
                        .checked_mul(core::mem::size_of::<ValueDataDraft>())
                        .ok_or(ResidentKernelError::InvalidShape)?,
                )?;
                Ok((
                    retained,
                    retained
                        .checked_add(containers)
                        .ok_or(ResidentKernelError::InvalidShape)?,
                    elements,
                    nodes,
                ))
            }
            ResidentValueRef::Snapshot(_) => Err(ResidentKernelError::InvalidInput),
            ResidentValueRef::String(values) => {
                let payload = values
                    .iter()
                    .try_fold(0usize, |sum, value| sum.checked_add(value.len()))
                    .ok_or(ResidentKernelError::InvalidShape)?;
                let containers = values
                    .len()
                    .checked_mul(core::mem::size_of::<String>())
                    .ok_or(ResidentKernelError::InvalidShape)?;
                let payload = super::budget::checked_u64(payload)?;
                let containers = super::budget::checked_u64(containers)?;
                Ok((
                    payload
                        .checked_add(containers)
                        .ok_or(ResidentKernelError::InvalidShape)?,
                    payload
                        .checked_add(
                            containers
                                .checked_mul(2)
                                .ok_or(ResidentKernelError::InvalidShape)?,
                        )
                        .ok_or(ResidentKernelError::InvalidShape)?,
                    values.len(),
                    super::budget::checked_u64(values.len().saturating_add(1))?,
                ))
            }
            ResidentValueRef::Bool(values) => Ok((
                super::budget::checked_u64(values.len())?,
                super::budget::checked_u64(values.len())?,
                values.len(),
                super::budget::checked_u64(values.len())?,
            )),
            ResidentValueRef::Index(values) => {
                let bytes = values
                    .len()
                    .checked_mul(core::mem::size_of::<u64>())
                    .ok_or(ResidentKernelError::InvalidShape)?;
                let bytes = super::budget::checked_u64(bytes)?;
                Ok((
                    bytes,
                    bytes,
                    values.len(),
                    super::budget::checked_u64(values.len())?,
                ))
            }
            ResidentValueRef::F64(values) => {
                let bytes = values
                    .len()
                    .checked_mul(core::mem::size_of::<f64>())
                    .ok_or(ResidentKernelError::InvalidShape)?;
                let bytes = super::budget::checked_u64(bytes)?;
                Ok((
                    bytes,
                    bytes,
                    values.len(),
                    super::budget::checked_u64(values.len())?,
                ))
            }
        }
    };
    let source_ref = input(inputs, 0)?;
    if let ResidentValueRef::Bool(values) = source_ref
        && values.iter().any(|value| *value > 1)
    {
        return Err(ResidentKernelError::InvalidInput);
    }
    if let ResidentValueMut::Bool(values) = &output
        && values.iter().any(|value| *value > 1)
    {
        return Err(ResidentKernelError::InvalidOutput);
    }
    let actual_source_len = match source_ref {
        ResidentValueRef::Snapshot([Some(value)]) => match value.data() {
            ValueData::Matrix(matrix) => matrix.elements().len(),
            _ => 1,
        },
        ResidentValueRef::Snapshot(_) => return Err(ResidentKernelError::InvalidInput),
        source => source.len(),
    };
    let actual_output_len = match &output {
        ResidentValueMut::Snapshot([Some(value)]) => match value.data() {
            ValueData::Matrix(matrix) => matrix.elements().len(),
            _ => return Err(ResidentKernelError::InvalidOutput),
        },
        ResidentValueMut::Snapshot(_) => return Err(ResidentKernelError::InvalidOutput),
        ResidentValueMut::Bool(values) => values.len(),
        ResidentValueMut::Index(values) => values.len(),
        ResidentValueMut::F64(values) => values.len(),
        ResidentValueMut::String(values) => values.len(),
    };
    if actual_source_len != source_len || actual_output_len != output_len {
        return Err(ResidentKernelError::InvalidShape);
    }
    let selector = |index: usize| {
        inputs
            .get(index + 1)
            .ok_or(ResidentKernelError::InvalidInput)
    };
    let (selected_rows_count, selected_columns_count) = match plan.mode {
        ResolvedSelectionMode::Whole => (plan.rows, plan.columns),
        ResolvedSelectionMode::Rows => (
            selector_access_count(selector(0)?, plan.rows)?,
            plan.columns,
        ),
        ResolvedSelectionMode::Columns => (
            plan.rows,
            selector_access_count(selector(0)?, plan.columns)?,
        ),
        ResolvedSelectionMode::Rectangle => (
            selector_access_count(selector(0)?, plan.rows)?,
            selector_access_count(selector(1)?, plan.columns)?,
        ),
        ResolvedSelectionMode::LinearScalar
        | ResolvedSelectionMode::LinearGather
        | ResolvedSelectionMode::Field { .. }
        | ResolvedSelectionMode::TableColumn { .. }
        | ResolvedSelectionMode::MapKey => return Err(ResidentKernelError::InvalidInput),
    };
    let write_count = selected_rows_count
        .checked_mul(selected_columns_count)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let source_shape_valid = match plan.source_routing {
        ResolvedSourceRouting::ScalarBroadcast => source_len == 1,
        ResolvedSourceRouting::Positional => source_len == output_len,
        ResolvedSourceRouting::CompactSelectionOrder => source_len == write_count,
    };
    if !source_shape_valid {
        return Err(ResidentKernelError::InvalidShape);
    }
    if write_count == 0 {
        return Ok(false);
    }
    let (source_clone_bytes, source_materialization_bytes, source_nodes) = match source_ref {
        ResidentValueRef::Snapshot(_) => {
            let (cloned, materialized, _, nodes) = value_cost(&mut footprint_meter, source_ref)?;
            (cloned, materialized, nodes)
        }
        ResidentValueRef::Bool(values) => (0, 0, super::budget::checked_u64(values.len())?),
        ResidentValueRef::Index(values) => (0, 0, super::budget::checked_u64(values.len())?),
        ResidentValueRef::F64(values) => (0, 0, super::budget::checked_u64(values.len())?),
        ResidentValueRef::String(values) => (
            0,
            0,
            super::budget::checked_u64(values.len().saturating_add(1))?,
        ),
    };
    let (output_bytes, output_materialization_bytes, output_nodes) = match &output {
        ResidentValueMut::Snapshot([Some(value)]) => {
            let (retained, nodes) = snapshot_clone_cost(&mut footprint_meter, value, schemas)
                .map_err(|_| ResidentKernelError::InvalidOutput)?;
            let containers = super::budget::checked_u64(
                output_len
                    .checked_mul(core::mem::size_of::<ValueDataDraft>())
                    .ok_or(ResidentKernelError::InvalidShape)?,
            )?;
            (
                retained,
                retained
                    .checked_add(containers)
                    .ok_or(ResidentKernelError::InvalidShape)?,
                nodes,
            )
        }
        ResidentValueMut::Snapshot(_) => return Err(ResidentKernelError::InvalidOutput),
        ResidentValueMut::String(values) => {
            let payload = values
                .iter()
                .try_fold(0usize, |sum, value| sum.checked_add(value.len()))
                .ok_or(ResidentKernelError::InvalidShape)?;
            let containers = values
                .len()
                .checked_mul(core::mem::size_of::<String>())
                .ok_or(ResidentKernelError::InvalidShape)?;
            let payload = super::budget::checked_u64(payload)?;
            let containers = super::budget::checked_u64(containers)?;
            (
                payload
                    .checked_add(containers)
                    .ok_or(ResidentKernelError::InvalidShape)?,
                payload
                    .checked_add(containers)
                    .ok_or(ResidentKernelError::InvalidShape)?,
                super::budget::checked_u64(values.len().saturating_add(1))?,
            )
        }
        ResidentValueMut::Bool(values) => (
            super::budget::checked_u64(values.len())?,
            super::budget::checked_u64(values.len())?,
            super::budget::checked_u64(values.len())?,
        ),
        ResidentValueMut::Index(values) => {
            let bytes = values
                .len()
                .checked_mul(core::mem::size_of::<u64>())
                .ok_or(ResidentKernelError::InvalidShape)?;
            let bytes = super::budget::checked_u64(bytes)?;
            (bytes, bytes, super::budget::checked_u64(values.len())?)
        }
        ResidentValueMut::F64(values) => {
            let bytes = values
                .len()
                .checked_mul(core::mem::size_of::<f64>())
                .ok_or(ResidentKernelError::InvalidShape)?;
            let bytes = super::budget::checked_u64(bytes)?;
            (bytes, bytes, super::budget::checked_u64(values.len())?)
        }
    };
    let mut selector_clone_bytes = 0u64;
    let mut selector_materialization_bytes = 0u64;
    let mut selector_nodes = 0u64;
    for index in 0..plan.selectors.len() {
        let selector = inputs
            .get(index + 1)
            .ok_or(ResidentKernelError::InvalidInput)?;
        let (cloned, materialization, _, nodes) = value_cost(&mut footprint_meter, selector)?;
        selector_clone_bytes = selector_clone_bytes
            .checked_add(cloned)
            .ok_or(ResidentKernelError::InvalidShape)?;
        selector_materialization_bytes = selector_materialization_bytes
            .checked_add(materialization)
            .ok_or(ResidentKernelError::InvalidShape)?;
        selector_nodes = selector_nodes
            .checked_add(nodes)
            .ok_or(ResidentKernelError::InvalidShape)?;
    }
    let (assignment_clone_bytes, assignment_clone_nodes) = matrix_assignment_clone_amplification(
        &mut footprint_meter,
        source_ref,
        schemas,
        write_count,
    )?;
    let coordinate_bytes = super::budget::checked_u64(
        selected_rows_count
            .checked_add(selected_columns_count)
            .and_then(|count| count.checked_mul(core::mem::size_of::<usize>()))
            .ok_or(ResidentKernelError::InvalidShape)?,
    )?;
    let coordinate_nodes = super::budget::checked_u64(
        selected_rows_count
            .checked_add(selected_columns_count)
            .and_then(|count| count.checked_add(2))
            .ok_or(ResidentKernelError::InvalidShape)?,
    )?;
    let mut final_output_bytes = output_bytes;
    let mut final_output_nodes = output_nodes;
    let mut publication_comparison_work = super::budget::checked_u64(output_len)?;
    let mut additional_temporary_bytes = 0u64;
    let mut additional_cloned_bytes = 0u64;
    let snapshot_finalization_work = match (source_ref, &output) {
        (ResidentValueRef::String(source), ResidentValueMut::String(current)) => {
            let mut final_payload = 0u64;
            publication_comparison_work = 0;
            for (destination, current) in current.iter().enumerate() {
                let selected = matrix_selection_last_source(
                    plan,
                    inputs,
                    destination,
                    false,
                    &mut footprint_meter,
                )?;
                let next = match selected {
                    Some(source_index) => source
                        .get(source_index)
                        .ok_or(ResidentKernelError::InvalidShape)?,
                    None => current,
                };
                final_payload = final_payload
                    .checked_add(super::budget::checked_u64(next.len())?)
                    .ok_or(ResidentKernelError::InvalidShape)?;
                publication_comparison_work = publication_comparison_work
                    .checked_add(super::budget::checked_u64(current.len().max(next.len()))?)
                    .ok_or(ResidentKernelError::InvalidShape)?;
            }
            let container_bytes = super::budget::checked_u64(
                output_len
                    .checked_mul(core::mem::size_of::<String>())
                    .ok_or(ResidentKernelError::InvalidShape)?,
            )?;
            final_output_bytes = final_payload
                .checked_add(container_bytes)
                .ok_or(ResidentKernelError::InvalidShape)?;
            final_output_nodes = super::budget::checked_u64(
                output_len
                    .checked_add(1)
                    .ok_or(ResidentKernelError::InvalidShape)?,
            )?;
            // The staged final vector remains live while publication clones
            // its strings into the output resident.
            additional_temporary_bytes = final_output_bytes;
            additional_cloned_bytes = final_payload;
            None
        }
        (
            ResidentValueRef::Snapshot([Some(source)]),
            ResidentValueMut::Snapshot([Some(current)]),
        ) => {
            let current_schema = current
                .validate_against(schemas)
                .map_err(|_| ResidentKernelError::InvalidOutput)?;
            let source_schema = source
                .validate_against(schemas)
                .map_err(|_| ResidentKernelError::InvalidInput)?;
            let current_footprint = super::budget::measure_canonical_value_footprint(
                &mut footprint_meter,
                current,
                schemas,
            )
            .map_err(|_| ResidentKernelError::InvalidOutput)?;
            let (final_footprint, finalization_work) = projected_snapshot_matrix_output(
                &mut footprint_meter,
                current_schema.body(),
                current.data(),
                current_footprint,
                source_schema.body(),
                source.data(),
                output_len,
                |destination, meter| {
                    matrix_selection_last_source(plan, inputs, destination, true, meter)
                },
            )?;
            let metadata = kernel
                .snapshot_output()
                .ok_or(ResidentKernelError::InvalidOutput)?;
            publication_comparison_work = super::budget::projected_language_equality_work(
                schemas,
                current,
                current_footprint,
                metadata.schema,
                metadata.shape.parameter_values().len(),
                final_footprint,
            )?;
            final_output_bytes = final_footprint.retained_bytes;
            final_output_nodes = final_footprint.node_count;
            additional_temporary_bytes = final_footprint.retained_bytes;
            additional_cloned_bytes = final_footprint.retained_bytes;
            Some(finalization_work)
        }
        (ResidentValueRef::Bool(_), ResidentValueMut::Bool(_))
        | (ResidentValueRef::Index(_), ResidentValueMut::Index(_))
        | (ResidentValueRef::F64(_), ResidentValueMut::F64(_)) => None,
        _ => return Err(ResidentKernelError::InvalidInput),
    };
    let temporary_bytes = output_materialization_bytes
        .checked_add(source_materialization_bytes)
        .and_then(|bytes| bytes.checked_add(selector_materialization_bytes))
        .and_then(|bytes| bytes.checked_add(coordinate_bytes))
        .and_then(|bytes| bytes.checked_add(assignment_clone_bytes))
        .and_then(|bytes| bytes.checked_add(additional_temporary_bytes))
        .ok_or(ResidentKernelError::InvalidShape)?;
    let cloned_bytes = output_bytes
        .checked_add(source_clone_bytes)
        .and_then(|bytes| bytes.checked_add(selector_clone_bytes))
        .and_then(|bytes| bytes.checked_add(assignment_clone_bytes))
        .and_then(|bytes| bytes.checked_add(additional_cloned_bytes))
        .ok_or(ResidentKernelError::InvalidShape)?;
    let current_persistent_nodes = output_nodes
        .checked_add(source_nodes)
        .and_then(|nodes| nodes.checked_add(selector_nodes))
        .ok_or(ResidentKernelError::InvalidShape)?;
    let temporary_draft_nodes = output_nodes
        .checked_add(source_nodes)
        .and_then(|nodes| nodes.checked_add(selector_nodes))
        .and_then(|nodes| nodes.checked_add(assignment_clone_nodes))
        .and_then(|nodes| nodes.checked_add(final_output_nodes))
        .ok_or(ResidentKernelError::InvalidShape)?;
    let measured = footprint_meter.estimate();
    let snapshot_finalization_work = super::budget::PreparedMutationPlan::new(
        snapshot_finalization_work,
        super::budget::PublishedOutputFootprint {
            elements: super::budget::checked_u64(output_len)?,
            retained_bytes: final_output_bytes,
            retained_nodes: final_output_nodes,
        },
        super::budget::MutationRetainedNodeFootprint {
            current_persistent: current_persistent_nodes,
            normalized_plan: coordinate_nodes,
            temporary_draft: temporary_draft_nodes,
        },
        super::budget::resident_cost! {
            comparison_work: measured
                .comparison_work()
                .checked_add(publication_comparison_work)
                .ok_or(ResidentKernelError::InvalidShape)?,
            compute_work: measured
                .compute_work()
                .checked_add(super::budget::checked_u64(
                    output_len
                .checked_add(write_count)
                        .ok_or(ResidentKernelError::InvalidShape)?,
                )?)
                .and_then(|work| work.checked_add(publication_comparison_work))
                .ok_or(ResidentKernelError::InvalidShape)?,
            temporary_bytes,
            cloned_bytes,
            ..super::budget::KernelCostEstimate::default()
        },
    )?
    .admit()?
    .into_plan();
    let selectors = plan
        .selectors
        .iter()
        .enumerate()
        .map(|(index, layout)| {
            selector_value(
                schemas,
                layout,
                inputs
                    .get(index + 1)
                    .ok_or(ResidentKernelError::InvalidInput)?,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let (selected_rows, selected_columns) = matrix_selection_coordinates(plan, &selectors)?;
    let materialized_write_count = selected_rows
        .len()
        .checked_mul(selected_columns.len())
        .ok_or(ResidentKernelError::InvalidShape)?;
    if materialized_write_count != write_count {
        return Err(ResidentKernelError::InvalidShape);
    }

    match (source_ref, output) {
        (ResidentValueRef::Bool(source), ResidentValueMut::Bool(target)) => {
            if source.iter().any(|value| *value > 1) {
                return Err(ResidentKernelError::InvalidInput);
            }
            assign_dense_matrix_selection(
                source,
                target,
                &selected_rows,
                &selected_columns,
                plan,
                |left, right| left != right,
            )
        }
        (ResidentValueRef::Index(source), ResidentValueMut::Index(target)) => {
            assign_dense_matrix_selection(
                source,
                target,
                &selected_rows,
                &selected_columns,
                plan,
                |left, right| left != right,
            )
        }
        (ResidentValueRef::F64(source), ResidentValueMut::F64(target)) => {
            assign_dense_matrix_selection(
                source,
                target,
                &selected_rows,
                &selected_columns,
                plan,
                |left, right| left.to_bits() != right.to_bits(),
            )
        }
        (ResidentValueRef::String(source), ResidentValueMut::String(target)) => {
            assign_dense_matrix_selection(
                source,
                target,
                &selected_rows,
                &selected_columns,
                plan,
                |left, right| left != right,
            )
        }
        (ResidentValueRef::Snapshot([Some(source)]), ResidentValueMut::Snapshot([target])) => {
            let current = target.as_ref().ok_or(ResidentKernelError::InvalidOutput)?;
            let ValueDataDraft::Matrix(mut next) = current
                .canonical_data_draft()
                .map_err(|_| ResidentKernelError::InvalidOutput)?
            else {
                return Err(ResidentKernelError::InvalidOutput);
            };
            if next.len() != output_len {
                return Err(ResidentKernelError::InvalidShape);
            }
            let source = match source
                .canonical_data_draft()
                .map_err(|_| ResidentKernelError::InvalidInput)?
            {
                ValueDataDraft::Matrix(values) => values.into_vec(),
                value => vec![value],
            };
            if source.len() != source_len
                || match plan.source_routing {
                    ResolvedSourceRouting::ScalarBroadcast => source_len != 1,
                    ResolvedSourceRouting::Positional => source_len != output_len,
                    ResolvedSourceRouting::CompactSelectionOrder => source_len != write_count,
                }
            {
                return Err(ResidentKernelError::InvalidShape);
            }
            let mut ordinal = 0usize;
            for row in &selected_rows {
                for column in &selected_columns {
                    let destination = row
                        .checked_mul(plan.columns)
                        .and_then(|offset| offset.checked_add(*column))
                        .filter(|destination| *destination < output_len)
                        .ok_or(ResidentKernelError::InvalidShape)?;
                    let source_index = match plan.source_routing {
                        ResolvedSourceRouting::ScalarBroadcast => 0,
                        ResolvedSourceRouting::Positional => destination,
                        ResolvedSourceRouting::CompactSelectionOrder => ordinal,
                    };
                    next[destination] = source
                        .get(source_index)
                        .ok_or(ResidentKernelError::InvalidShape)?
                        .clone();
                    ordinal = ordinal
                        .checked_add(1)
                        .ok_or(ResidentKernelError::InvalidShape)?;
                }
            }
            let next = finalize_snapshot_data_with_work_budget(
                kernel,
                ValueDataDraft::Matrix(next),
                Some(snapshot_finalization_work.ok_or(ResidentKernelError::InvalidOutput)?),
            )?;
            let changed = !current
                .language_eq(schemas, &next, schemas)
                .map_err(|_| ResidentKernelError::InvalidOutput)?;
            *target = Some(next);
            Ok(changed)
        }
        _ => Err(ResidentKernelError::InvalidInput),
    }
}

fn aggregate_assignment_data(
    current: &mech_core::Value,
    current_schema: &SchemaBody,
    source: &mech_core::Value,
    selector: &mech_core::Value,
    schemas: &mech_core::SchemaTable,
    resolved_ordinal: Option<usize>,
) -> Result<ValueDataDraft, ResidentKernelError> {
    let source_schema = source
        .validate_against(schemas)
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    match current_schema {
        SchemaBody::Tuple(elements) => {
            let selected = access_indices(selector, elements.len())?;
            let [selected] = selected.as_slice() else {
                return Err(ResidentKernelError::InvalidShape);
            };
            if resolved_ordinal.is_some_and(|ordinal| ordinal != *selected) {
                return Err(ResidentKernelError::InvalidInput);
            }
            if source_schema.body() != &elements[*selected] {
                return Err(ResidentKernelError::InvalidInput);
            }
            let ValueDataDraft::Tuple(mut values) = current
                .canonical_data_draft()
                .map_err(|_| ResidentKernelError::InvalidOutput)?
            else {
                return Err(ResidentKernelError::InvalidOutput);
            };
            values[*selected] = source
                .canonical_data_draft()
                .map_err(|_| ResidentKernelError::InvalidInput)?;
            Ok(ValueDataDraft::Tuple(values))
        }
        SchemaBody::Record(fields) => {
            let ValueData::Id(_) = selector.data() else {
                return Err(ResidentKernelError::InvalidInput);
            };
            let selected = resolved_ordinal.ok_or(ResidentKernelError::InvalidInput)?;
            let field = fields
                .get(selected)
                .ok_or(ResidentKernelError::InvalidInput)?;
            if source_schema.body() != &field.schema {
                return Err(ResidentKernelError::InvalidInput);
            }
            let ValueDataDraft::Record(mut values) = current
                .canonical_data_draft()
                .map_err(|_| ResidentKernelError::InvalidOutput)?
            else {
                return Err(ResidentKernelError::InvalidOutput);
            };
            values[selected].value = source
                .canonical_data_draft()
                .map_err(|_| ResidentKernelError::InvalidInput)?;
            Ok(ValueDataDraft::Record(values))
        }
        SchemaBody::Map { value, .. } => {
            if source_schema.body() != value.as_ref() {
                return Err(ResidentKernelError::InvalidInput);
            }
            let ValueDataDraft::Map(mut entries) = current
                .canonical_data_draft()
                .map_err(|_| ResidentKernelError::InvalidOutput)?
            else {
                return Err(ResidentKernelError::InvalidOutput);
            };
            let replacement = source
                .canonical_data_draft()
                .map_err(|_| ResidentKernelError::InvalidInput)?;
            let selected = resolved_ordinal.ok_or(ResidentKernelError::InvalidInput)?;
            let value = entries
                .get_mut(selected)
                .and_then(|entry| entry.items.get_mut(1))
                .ok_or(ResidentKernelError::InvalidInput)?;
            *value = replacement;
            Ok(ValueDataDraft::Map(entries))
        }
        SchemaBody::Table { columns, .. } => {
            let ValueData::Id(_) = selector.data() else {
                return Err(ResidentKernelError::InvalidInput);
            };
            let selected = resolved_ordinal.ok_or(ResidentKernelError::InvalidInput)?;
            let column = columns
                .get(selected)
                .ok_or(ResidentKernelError::InvalidInput)?;
            let SchemaBody::Matrix { element, .. } = source_schema.body() else {
                return Err(ResidentKernelError::InvalidInput);
            };
            if element.as_ref() != &column.schema {
                return Err(ResidentKernelError::InvalidInput);
            }
            let ValueDataDraft::Matrix(source_values) = source
                .canonical_data_draft()
                .map_err(|_| ResidentKernelError::InvalidInput)?
            else {
                return Err(ResidentKernelError::InvalidInput);
            };
            let ValueDataDraft::Table(mut values) = current
                .canonical_data_draft()
                .map_err(|_| ResidentKernelError::InvalidOutput)?
            else {
                return Err(ResidentKernelError::InvalidOutput);
            };
            if source_values.len() != values[selected].values.len() {
                return Err(ResidentKernelError::InvalidShape);
            }
            values[selected].values = source_values;
            Ok(ValueDataDraft::Table(values))
        }
        _ => Err(ResidentKernelError::InvalidInput),
    }
}

fn indexed_assign_snapshot_aggregate(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let plan = kernel
        .retained_state::<SnapshotAggregateAssignPlan>()
        .ok_or(ResidentKernelError::InvalidInput)?;
    if inputs.len() != 2 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let ResidentValueMut::Snapshot([target]) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let current = target.as_ref().ok_or(ResidentKernelError::InvalidOutput)?;
    let current_schema = current
        .validate_against(schemas)
        .map_err(|_| ResidentKernelError::InvalidOutput)?;
    let mut footprint_meter = super::budget::ResidentBudgetMeter::default();
    let current_footprint =
        super::budget::measure_canonical_value_footprint(&mut footprint_meter, current, schemas)
            .map_err(|_| ResidentKernelError::InvalidOutput)?;
    let current_bytes = current_footprint.retained_bytes;
    let current_nodes = current_footprint.node_count;
    let source_cost = snapshot_selector_materialization_cost(
        schemas,
        &plan.source,
        input(inputs, 0)?,
        &mut footprint_meter,
    )?;
    let selector_cost = snapshot_selector_materialization_cost(
        schemas,
        &plan.selector,
        input(inputs, 1)?,
        &mut footprint_meter,
    )?;
    let execution_ordinal = match (current_schema.body(), current.data()) {
        (SchemaBody::Map { key, .. }, ValueData::Map(map)) => {
            Some(map_access_entry_for_selector_with_meter(
                map,
                key,
                input(inputs, 1)?,
                &mut footprint_meter,
            )?)
        }
        (SchemaBody::Record(fields), ValueData::Record(_)) => {
            let selected = named_schema_ordinal_with_meter(
                fields.iter().map(|candidate| candidate.name.as_str()),
                selector_id(input(inputs, 1)?)?,
                &mut footprint_meter,
            )?;
            if plan
                .aggregate_ordinal
                .is_some_and(|expected| expected != selected)
            {
                return Err(ResidentKernelError::InvalidInput);
            }
            Some(selected)
        }
        (SchemaBody::Table { columns, .. }, ValueData::Table(_)) => {
            let selected = named_schema_ordinal_with_meter(
                columns.iter().map(|candidate| candidate.name.as_str()),
                selector_id(input(inputs, 1)?)?,
                &mut footprint_meter,
            )?;
            if plan
                .aggregate_ordinal
                .is_some_and(|expected| expected != selected)
            {
                return Err(ResidentKernelError::InvalidInput);
            }
            Some(selected)
        }
        _ => plan.aggregate_ordinal,
    };
    let mut finalization_work = snapshot_data_finalization_work(
        &mut footprint_meter,
        current_schema.body(),
        current.data(),
    )?;
    if let Some(ResidentValueRef::Snapshot([Some(source)])) = inputs.get(0) {
        let source_schema = source
            .validate_against(schemas)
            .map_err(|_| ResidentKernelError::InvalidInput)?;
        finalization_work = finalization_work
            .checked_add(snapshot_data_finalization_work(
                &mut footprint_meter,
                source_schema.body(),
                source.data(),
            )?)
            .ok_or(ResidentKernelError::InvalidShape)?;
    }
    let materialized_bytes = source_cost
        .retained_bytes
        .checked_mul(2)
        .ok_or(ResidentKernelError::InvalidShape)?
        .checked_add(selector_cost.retained_bytes)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let output_bytes = current_bytes
        .checked_add(source_cost.retained_bytes)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let (final_output_nodes, node_phases) = snapshot_aggregate_assignment_node_phases(
        current_nodes,
        source_cost.retained_nodes,
        selector_cost.retained_nodes,
    )?;
    let measured_nodes = node_phases
        .current_persistent
        .checked_add(node_phases.normalized_plan)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let draft_bytes = final_output_nodes
        .checked_mul(super::budget::checked_u64(core::mem::size_of::<
            ValueDataDraft,
        >())?)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let output_metadata = kernel
        .snapshot_output()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let final_output_footprint = ValueFootprint {
        encoded_bytes: current_footprint
            .encoded_bytes
            .checked_add(source_cost.retained_bytes)
            .ok_or(ResidentKernelError::InvalidShape)?,
        retained_bytes: output_bytes,
        node_count: final_output_nodes,
    };
    let publication_equality_work = super::budget::projected_language_equality_work(
        schemas,
        current,
        current_footprint,
        output_metadata.schema,
        output_metadata.shape.parameter_values().len(),
        final_output_footprint,
    )?;
    let footprint_work = footprint_meter.estimate();
    let cost = super::budget::resident_cost! {
        comparison_work: footprint_work.comparison_work()
            .checked_add(publication_equality_work)
            .ok_or(ResidentKernelError::InvalidShape)?,
        compute_work: measured_nodes
            .checked_add(footprint_work.compute_work())
            .and_then(|work| work.checked_add(publication_equality_work))
            .ok_or(ResidentKernelError::InvalidShape)?,
        temporary_bytes: current_bytes
            .checked_add(materialized_bytes)
            .ok_or(ResidentKernelError::InvalidShape)?,
        cloned_bytes: output_bytes
            .checked_add(source_cost.cloned_bytes)
            .and_then(|bytes| bytes.checked_add(selector_cost.cloned_bytes))
            .ok_or(ResidentKernelError::InvalidShape)?,
        container_bytes: draft_bytes,
        ..super::budget::KernelCostEstimate::default()
    };
    let (execution_ordinal, canonicalization_work_limit) =
        super::budget::PreparedMutationPlan::new(
            (execution_ordinal, finalization_work),
            super::budget::PublishedOutputFootprint {
                elements: 1,
                retained_bytes: output_bytes,
                retained_nodes: final_output_nodes,
            },
            node_phases,
            cost,
        )?
        .admit()?
        .into_plan();
    let source = selector_value(schemas, &plan.source, input(inputs, 0)?)?;
    let selector = selector_value(schemas, &plan.selector, input(inputs, 1)?)?;
    let data = aggregate_assignment_data(
        current,
        current_schema.body(),
        &source,
        &selector,
        schemas,
        execution_ordinal,
    )?;
    let next =
        finalize_snapshot_data_with_work_budget(kernel, data, Some(canonicalization_work_limit))?;
    let changed = !current
        .language_eq(schemas, &next, schemas)
        .map_err(|_| ResidentKernelError::InvalidOutput)?;
    *target = Some(next);
    Ok(changed)
}

fn snapshot_aggregate_assignment_node_phases(
    current_nodes: u64,
    source_nodes: u64,
    selector_nodes: u64,
) -> Result<(u64, super::budget::MutationRetainedNodeFootprint), ResidentKernelError> {
    let normalized_plan = source_nodes
        .checked_add(selector_nodes)
        .ok_or(ResidentKernelError::InvalidShape)?;
    // Without materializing the replacement, the complete resulting tree is
    // conservatively bounded by the current tree plus the source tree. The
    // canonical aggregate draft and finalized value are separate admitted
    // populations while the borrowed current/source/selector values remain
    // live.
    let final_output_nodes = current_nodes
        .checked_add(source_nodes)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let current_persistent = current_nodes
        .checked_add(source_nodes)
        .and_then(|nodes| nodes.checked_add(selector_nodes))
        .ok_or(ResidentKernelError::InvalidShape)?;
    Ok((
        final_output_nodes,
        super::budget::MutationRetainedNodeFootprint {
            current_persistent,
            normalized_plan,
            temporary_draft: final_output_nodes,
        },
    ))
}

fn ekf_trig(
    _kernel: &BoundResidentKernel,
    state: &[f64],
    output: &mut [f64],
) -> Result<bool, ResidentKernelError> {
    write_f64_array(
        output,
        crate::efficacy::ekf::math::trigonometric_state(as_f64_array(state)?),
    )
}

fn ekf_motion(
    _kernel: &BoundResidentKernel,
    _state: &[f64],
    frame: &[f64],
    trig: &[f64],
    dt: &[f64],
    output: &mut [f64],
) -> Result<bool, ResidentKernelError> {
    write_f64_array(
        output,
        crate::efficacy::ekf::math::motion_jacobian(
            as_f64_array(frame)?,
            as_f64_array(trig)?,
            *as_f64_array::<1>(dt)?.first().unwrap(),
        ),
    )
}

fn ekf_control(
    _kernel: &BoundResidentKernel,
    trig: &[f64],
    dt: &[f64],
    output: &mut [f64],
) -> Result<bool, ResidentKernelError> {
    write_f64_array(
        output,
        crate::efficacy::ekf::math::control_jacobian(
            as_f64_array(trig)?,
            as_f64_array::<1>(dt)?[0],
        ),
    )
}

fn ekf_predicted_state(
    _kernel: &BoundResidentKernel,
    state: &[f64],
    frame: &[f64],
    trig: &[f64],
    dt: &[f64],
    output: &mut [f64],
) -> Result<bool, ResidentKernelError> {
    write_f64_array(
        output,
        crate::efficacy::ekf::math::predicted_state(
            as_f64_array(state)?,
            as_f64_array(frame)?,
            as_f64_array(trig)?,
            as_f64_array::<1>(dt)?[0],
        ),
    )
}

fn ekf_predicted_covariance(
    _kernel: &BoundResidentKernel,
    covariance: &[f64],
    motion: &[f64],
    control: &[f64],
    process: &[f64],
    output: &mut [f64],
) -> Result<bool, ResidentKernelError> {
    write_f64_array(
        output,
        crate::efficacy::ekf::math::predicted_covariance(
            as_f64_array(covariance)?,
            as_f64_array(motion)?,
            as_f64_array(control)?,
            as_f64_array(process)?,
        ),
    )
}

fn ekf_landmark(
    _kernel: &BoundResidentKernel,
    predicted: &[f64],
    landmark: &[f64],
    output: &mut [f64],
) -> Result<bool, ResidentKernelError> {
    let next = crate::efficacy::ekf::math::landmark_delta_and_range(
        as_f64_array(predicted)?,
        as_f64_array(landmark)?,
    )
    .map_err(|_| ResidentKernelError::Arithmetic)?;
    write_f64_array(output, next)
}

fn ekf_measurement(
    _kernel: &BoundResidentKernel,
    predicted: &[f64],
    delta: &[f64],
    output: &mut [f64],
) -> Result<bool, ResidentKernelError> {
    write_f64_array(
        output,
        crate::efficacy::ekf::math::predicted_measurement(
            as_f64_array(predicted)?,
            as_f64_array(delta)?,
        ),
    )
}

fn ekf_measurement_jacobian(
    _kernel: &BoundResidentKernel,
    delta: &[f64],
    output: &mut [f64],
) -> Result<bool, ResidentKernelError> {
    write_f64_array(
        output,
        crate::efficacy::ekf::math::measurement_jacobian(as_f64_array(delta)?),
    )
}

fn ekf_innovation_covariance(
    _kernel: &BoundResidentKernel,
    covariance: &[f64],
    jacobian: &[f64],
    noise: &[f64],
    output: &mut [f64],
) -> Result<bool, ResidentKernelError> {
    write_f64_array(
        output,
        crate::efficacy::ekf::math::innovation_covariance(
            as_f64_array(covariance)?,
            as_f64_array(jacobian)?,
            as_f64_array(noise)?,
        ),
    )
}

fn ekf_solve(
    _kernel: &BoundResidentKernel,
    covariance: &[f64],
    output: &mut [f64],
) -> Result<bool, ResidentKernelError> {
    let next = crate::efficacy::ekf::math::solve_2x2(as_f64_array(covariance)?)
        .map_err(|_| ResidentKernelError::Arithmetic)?;
    write_f64_array(output, next)
}

fn ekf_gain(
    _kernel: &BoundResidentKernel,
    covariance: &[f64],
    jacobian: &[f64],
    inverse: &[f64],
    output: &mut [f64],
) -> Result<bool, ResidentKernelError> {
    write_f64_array(
        output,
        crate::efficacy::ekf::math::kalman_gain(
            as_f64_array(covariance)?,
            as_f64_array(jacobian)?,
            as_f64_array(inverse)?,
        ),
    )
}

fn ekf_innovation(
    _kernel: &BoundResidentKernel,
    frame: &[f64],
    predicted: &[f64],
    output: &mut [f64],
) -> Result<bool, ResidentKernelError> {
    write_f64_array(
        output,
        crate::efficacy::ekf::math::innovation(as_f64_array(frame)?, as_f64_array(predicted)?),
    )
}

fn ekf_corrected_state(
    _kernel: &BoundResidentKernel,
    predicted: &[f64],
    gain: &[f64],
    innovation: &[f64],
    output: &mut [f64],
) -> Result<bool, ResidentKernelError> {
    write_f64_array(
        output,
        crate::efficacy::ekf::math::corrected_state(
            as_f64_array(predicted)?,
            as_f64_array(gain)?,
            as_f64_array(innovation)?,
        ),
    )
}

fn ekf_joseph(
    _kernel: &BoundResidentKernel,
    covariance: &[f64],
    jacobian: &[f64],
    gain: &[f64],
    noise: &[f64],
    output: &mut [f64],
) -> Result<bool, ResidentKernelError> {
    write_f64_array(
        output,
        crate::efficacy::ekf::math::joseph_covariance_update(
            as_f64_array(covariance)?,
            as_f64_array(jacobian)?,
            as_f64_array(gain)?,
            as_f64_array(noise)?,
        ),
    )
}

fn ekf_symmetrize(
    _kernel: &BoundResidentKernel,
    covariance: &[f64],
    output: &mut [f64],
) -> Result<bool, ResidentKernelError> {
    write_f64_array(
        output,
        crate::efficacy::ekf::math::covariance_symmetrization(as_f64_array(covariance)?),
    )
}

fn ekf_finite(
    _kernel: &BoundResidentKernel,
    state: &[f64],
    covariance: &[f64],
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    write_bool(
        output,
        crate::efficacy::ekf::math::candidate_finite(
            as_f64_array(state)?,
            as_f64_array(covariance)?,
        ),
    )
}

fn ekf_positive_diagonal(
    _kernel: &BoundResidentKernel,
    covariance: &[f64],
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    write_bool(
        output,
        crate::efficacy::ekf::math::covariance_positive_diagonal(as_f64_array(covariance)?),
    )
}

fn ekf_symmetric(
    _kernel: &BoundResidentKernel,
    covariance: &[f64],
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    write_bool(
        output,
        crate::efficacy::ekf::math::covariance_symmetric(as_f64_array(covariance)?),
    )
}

fn negate(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 1 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let input = f64_input(inputs, 0)?;
    let output = f64_output(output)?;
    if input.len() != output.len() {
        return Err(ResidentKernelError::InvalidShape);
    }
    Ok(replace_f64(output, |index| -input[index]))
}

fn unary_f64(
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
    operation: impl Fn(f64) -> f64,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 1 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let input = f64_input(inputs, 0)?;
    let output = f64_output(output)?;
    if input.len() != output.len() {
        return Err(ResidentKernelError::InvalidShape);
    }
    Ok(replace_f64(output, |index| operation(input[index])))
}

fn unary_f32_snapshot(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
    operation: impl Fn(f32) -> f32,
) -> Result<bool, ResidentKernelError> {
    let Some(ResidentValueRef::Snapshot([Some(input)])) = inputs.get(0) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let [rows, columns] = kernel.parameters() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let rows = usize::try_from(*rows).map_err(|_| ResidentKernelError::InvalidShape)?;
    let columns = usize::try_from(*columns).map_err(|_| ResidentKernelError::InvalidShape)?;
    let output_len = rows
        .checked_mul(columns)
        .ok_or(ResidentKernelError::InvalidShape)?;
    if f32_snapshot_len(input) != Some(output_len) {
        return Err(ResidentKernelError::InvalidShape);
    }
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    preflight_snapshot_arithmetic(
        kernel,
        schemas,
        &[input],
        &output,
        output_len,
        output_len,
        0,
    )?;
    let values = f32_snapshot_values(input)
        .ok_or(ResidentKernelError::InvalidInput)?
        .into_iter()
        .map(operation)
        .map(|value| ValueDataDraft::F32(F32Bits::from_f32(value)))
        .collect::<Vec<_>>();
    let output_schema = schemas
        .get(
            kernel
                .snapshot_output()
                .ok_or(ResidentKernelError::InvalidOutput)?
                .schema,
        )
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let data = if matches!(output_schema.body(), SchemaBody::Matrix { .. }) {
        ValueDataDraft::Matrix(values.into_boxed_slice())
    } else {
        let [value] = values.as_slice() else {
            return Err(ResidentKernelError::InvalidShape);
        };
        value.clone()
    };
    write_snapshot_data_with_work_budget(kernel, output, data, Some(0))
}

fn cosine(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    unary_f64(inputs, output, f64::cos)
}

fn cosine_f32(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    unary_f32_snapshot(kernel, inputs, output, libm::cosf)
}

fn absolute(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    unary_f64(inputs, output, f64::abs)
}

fn absolute_f32(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    unary_f32_snapshot(kernel, inputs, output, f32::abs)
}

fn floor(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    unary_f64(inputs, output, f64::floor)
}

fn floor_f32(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    unary_f32_snapshot(kernel, inputs, output, libm::floorf)
}

fn square_root(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    unary_f64(inputs, output, f64::sqrt)
}

fn square_root_f32(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    unary_f32_snapshot(kernel, inputs, output, libm::sqrtf)
}

fn sine(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    unary_f64(inputs, output, f64::sin)
}

fn sine_f32(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    unary_f32_snapshot(kernel, inputs, output, libm::sinf)
}

fn atan2(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    binary_f64(kernel, inputs, output, f64::atan2)
}

fn math_copysign(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    binary_f64(kernel, inputs, output, libm::copysign)
}

fn math_fdim(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    binary_f64(kernel, inputs, output, libm::fdim)
}

fn math_fmod(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    binary_f64(kernel, inputs, output, libm::fmod)
}

fn math_nextafter(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    binary_f64(kernel, inputs, output, libm::nextafter)
}

fn math_remainder(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    binary_f64(kernel, inputs, output, libm::remainder)
}

fn math_bessel_jn(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    binary_f64(kernel, inputs, output, |order, value| {
        libm::jn(order as i32, value)
    })
}

fn math_bessel_yn(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    binary_f64(kernel, inputs, output, |order, value| {
        libm::yn(order as i32, value)
    })
}

fn math_copysign_f32(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    binary_f32_snapshot(kernel, inputs, output, libm::copysignf)
}

fn atan2_f32(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    binary_f32_snapshot(kernel, inputs, output, libm::atan2f)
}

fn math_fdim_f32(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    binary_f32_snapshot(kernel, inputs, output, libm::fdimf)
}

fn math_fmod_f32(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    binary_f32_snapshot(kernel, inputs, output, libm::fmodf)
}

fn math_nextafter_f32(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    binary_f32_snapshot(kernel, inputs, output, libm::nextafterf)
}

fn math_remainder_f32(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    binary_f32_snapshot(kernel, inputs, output, libm::remainderf)
}

fn math_bessel_jn_f32(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    binary_f32_snapshot(kernel, inputs, output, |order, value| {
        libm::jnf(order as i32, value)
    })
}

fn math_bessel_yn_f32(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    binary_f32_snapshot(kernel, inputs, output, |order, value| {
        libm::ynf(order as i32, value)
    })
}

fn f32_snapshot_values(value: &mech_core::Value) -> Option<Vec<f32>> {
    match value.data() {
        ValueData::F32(value) => Some(vec![value.to_f32()]),
        ValueData::Matrix(matrix) => {
            let SequenceView::F32(values) = matrix.elements() else {
                return None;
            };
            Some(values.iter().map(|value| value.to_f32()).collect())
        }
        _ => None,
    }
}

fn f32_snapshot_len(value: &mech_core::Value) -> Option<usize> {
    match value.data() {
        ValueData::F32(_) => Some(1),
        ValueData::Matrix(matrix) => match matrix.elements() {
            SequenceView::F32(values) => Some(values.len()),
            _ => None,
        },
        _ => None,
    }
}

fn binary_f32_snapshot(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
    operation: impl Fn(f32, f32) -> f32,
) -> Result<bool, ResidentKernelError> {
    let (
        Some(ResidentValueRef::Snapshot([Some(left)])),
        Some(ResidentValueRef::Snapshot([Some(right)])),
    ) = (inputs.get(0), inputs.get(1))
    else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let left_len = f32_snapshot_len(left).ok_or(ResidentKernelError::InvalidInput)?;
    let right_len = f32_snapshot_len(right).ok_or(ResidentKernelError::InvalidInput)?;
    if left_len != right_len {
        return Err(ResidentKernelError::InvalidShape);
    }
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    preflight_snapshot_arithmetic(
        kernel,
        schemas,
        &[left, right],
        &output,
        left_len
            .checked_add(right_len)
            .ok_or(ResidentKernelError::InvalidShape)?,
        left_len,
        0,
    )?;
    let left = f32_snapshot_values(left).ok_or(ResidentKernelError::InvalidInput)?;
    let right = f32_snapshot_values(right).ok_or(ResidentKernelError::InvalidInput)?;
    debug_assert_eq!(left.len(), right.len());
    let values = left
        .into_iter()
        .zip(right)
        .map(|(left, right)| operation(left, right))
        .collect::<Vec<_>>();
    let metadata = kernel
        .snapshot_output()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let schema = schemas
        .get(metadata.schema)
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let data = match schema.body() {
        SchemaBody::FloatingPoint(mech_core::FloatWidth::W32) => {
            let [value] = values.as_slice() else {
                return Err(ResidentKernelError::InvalidShape);
            };
            ValueDataDraft::F32(F32Bits::from_f32(*value))
        }
        SchemaBody::Matrix { element, .. }
            if element.as_ref() == &SchemaBody::FloatingPoint(mech_core::FloatWidth::W32) =>
        {
            ValueDataDraft::Matrix(
                values
                    .into_iter()
                    .map(|value| ValueDataDraft::F32(F32Bits::from_f32(value)))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            )
        }
        _ => return Err(ResidentKernelError::InvalidOutput),
    };
    write_snapshot_data_with_work_budget(kernel, output, data, Some(0))
}

fn binary_f64(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
    operation: impl Fn(f64, f64) -> f64,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 2 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let left = f64_input(inputs, 0)?;
    let right = f64_input(inputs, 1)?;
    let output = f64_output(output)?;
    let parameters = kernel.parameters();
    if parameters.is_empty() {
        let output_len = output.len();
        let pick = |values: &[f64], index: usize| match values.len() {
            1 => Some(values[0]),
            len if len == output_len => Some(values[index]),
            _ => None,
        };
        if pick(left, 0).is_none() || pick(right, 0).is_none() {
            return Err(ResidentKernelError::InvalidShape);
        }
        return Ok(replace_f64(output, |index| {
            operation(pick(left, index).unwrap(), pick(right, index).unwrap())
        }));
    }
    let [rows, columns, left_mode, right_mode] = parameters else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let rows = *rows as usize;
    let columns = *columns as usize;
    let output_len = rows
        .checked_mul(columns)
        .ok_or(ResidentKernelError::InvalidShape)?;
    if output.len() != output_len {
        return Err(ResidentKernelError::InvalidShape);
    }
    validate_binary_broadcast_input(left, *left_mode, rows, columns)?;
    validate_binary_broadcast_input(right, *right_mode, rows, columns)?;
    Ok(replace_f64(output, |index| {
        operation(
            binary_broadcast_value(left, *left_mode, index, rows),
            binary_broadcast_value(right, *right_mode, index, rows),
        )
    }))
}

fn validate_binary_broadcast_input(
    values: &[f64],
    mode: u64,
    rows: usize,
    columns: usize,
) -> Result<(), ResidentKernelError> {
    validate_binary_broadcast_len(values.len(), mode, rows, columns)
}

fn validate_binary_broadcast_len(
    len: usize,
    mode: u64,
    rows: usize,
    columns: usize,
) -> Result<(), ResidentKernelError> {
    let expected = match mode {
        BINARY_BROADCAST_SCALAR => 1,
        BINARY_BROADCAST_EXACT => rows
            .checked_mul(columns)
            .ok_or(ResidentKernelError::InvalidShape)?,
        BINARY_BROADCAST_COLUMN => rows,
        BINARY_BROADCAST_ROW => columns,
        _ => return Err(ResidentKernelError::InvalidInput),
    };
    if len == expected {
        Ok(())
    } else {
        Err(ResidentKernelError::InvalidShape)
    }
}

fn binary_broadcast_value(values: &[f64], mode: u64, index: usize, rows: usize) -> f64 {
    match mode {
        BINARY_BROADCAST_SCALAR => values[0],
        BINARY_BROADCAST_EXACT => values[index],
        BINARY_BROADCAST_COLUMN => values[index % rows],
        BINARY_BROADCAST_ROW => values[index / rows],
        _ => unreachable!("validated binary broadcast mode"),
    }
}

fn subtract(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    binary_f64(kernel, inputs, output, |left, right| left - right)
}

fn add(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    binary_f64(kernel, inputs, output, |left, right| left + right)
}

fn multiply(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    binary_f64(kernel, inputs, output, |left, right| left * right)
}

fn divide(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    binary_f64(kernel, inputs, output, |left, right| left / right)
}

fn remainder(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    binary_f64(kernel, inputs, output, |left, right| left % right)
}

fn binary_f64_comparison(
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
    comparison: impl Fn(f64, f64) -> bool,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 2 {
        return Err(ResidentKernelError::InvalidInput);
    }
    write_bool(
        output,
        comparison(f64_scalar(inputs, 0)?, f64_scalar(inputs, 1)?),
    )
}

fn f64_equal(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    binary_f64_comparison(inputs, output, |left, right| left == right)
}

fn f64_vector_equal(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    f64_vector_comparison(kernel, inputs, output, |left, right| left == right)
}

fn f64_vector_not_equal(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    f64_vector_comparison(kernel, inputs, output, |left, right| left != right)
}

fn f64_vector_comparison(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
    comparison: impl Fn(f64, f64) -> bool,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 2 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let left = f64_input(inputs, 0)?;
    let right = f64_input(inputs, 1)?;
    let ResidentValueMut::Bool(output) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let [rows, columns, left_mode, right_mode] = kernel.parameters() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let rows = usize::try_from(*rows).map_err(|_| ResidentKernelError::InvalidShape)?;
    let columns = usize::try_from(*columns).map_err(|_| ResidentKernelError::InvalidShape)?;
    if output.len()
        != rows
            .checked_mul(columns)
            .ok_or(ResidentKernelError::InvalidShape)?
    {
        return Err(ResidentKernelError::InvalidShape);
    }
    validate_binary_broadcast_input(left, *left_mode, rows, columns)?;
    validate_binary_broadcast_input(right, *right_mode, rows, columns)?;
    let mut changed = false;
    for (index, target) in output.iter_mut().enumerate() {
        let next = u8::from(comparison(
            binary_broadcast_value(left, *left_mode, index, rows),
            binary_broadcast_value(right, *right_mode, index, rows),
        ));
        if *target != next {
            *target = next;
            changed = true;
        }
    }
    Ok(changed)
}

fn f64_vector_less(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    f64_vector_comparison(kernel, inputs, output, |left, right| left < right)
}

fn f64_vector_less_equal(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    f64_vector_comparison(kernel, inputs, output, |left, right| left <= right)
}

fn f64_vector_greater(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    f64_vector_comparison(kernel, inputs, output, |left, right| left > right)
}

fn f64_vector_greater_equal(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    f64_vector_comparison(kernel, inputs, output, |left, right| left >= right)
}

fn f64_not_equal(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    binary_f64_comparison(inputs, output, |left, right| left != right)
}

fn f64_less(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    binary_f64_comparison(inputs, output, |left, right| left < right)
}

fn f64_less_equal(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    binary_f64_comparison(inputs, output, |left, right| left <= right)
}

fn f64_greater(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    binary_f64_comparison(inputs, output, |left, right| left > right)
}

fn f64_greater_equal(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    binary_f64_comparison(inputs, output, |left, right| left >= right)
}

fn bool_and(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 2 {
        return Err(ResidentKernelError::InvalidInput);
    }
    write_bool(output, bool_scalar(inputs, 0)? && bool_scalar(inputs, 1)?)
}

fn bool_vector_and(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    bool_vector_binary(kernel, inputs, output, |left, right| left && right)
}

fn bool_vector_or(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    bool_vector_binary(kernel, inputs, output, |left, right| left || right)
}

fn bool_vector_xor(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    bool_vector_binary(kernel, inputs, output, |left, right| left ^ right)
}

fn bool_vector_binary(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
    operation: impl Fn(bool, bool) -> bool,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 2 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let Some(ResidentValueRef::Bool(left)) = inputs.get(0) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let Some(ResidentValueRef::Bool(right)) = inputs.get(1) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let ResidentValueMut::Bool(output) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    if left.iter().chain(right).any(|value| *value > 1) {
        return Err(ResidentKernelError::InvalidInput);
    }
    let [rows, columns, left_mode, right_mode] = kernel.parameters() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let rows = usize::try_from(*rows).map_err(|_| ResidentKernelError::InvalidShape)?;
    let columns = usize::try_from(*columns).map_err(|_| ResidentKernelError::InvalidShape)?;
    if output.len()
        != rows
            .checked_mul(columns)
            .ok_or(ResidentKernelError::InvalidShape)?
    {
        return Err(ResidentKernelError::InvalidShape);
    }
    validate_binary_broadcast_len(left.len(), *left_mode, rows, columns)?;
    validate_binary_broadcast_len(right.len(), *right_mode, rows, columns)?;
    let at = |values: &[u8], mode, index| {
        let index = match mode {
            BINARY_BROADCAST_SCALAR => 0,
            BINARY_BROADCAST_EXACT => index,
            BINARY_BROADCAST_COLUMN => index % rows,
            BINARY_BROADCAST_ROW => index / rows,
            _ => unreachable!("validated Boolean broadcast mode"),
        };
        values[index] != 0
    };
    let mut changed = false;
    for (index, target) in output.iter_mut().enumerate() {
        let next = u8::from(operation(
            at(left, *left_mode, index),
            at(right, *right_mode, index),
        ));
        if *target != next {
            *target = next;
            changed = true;
        }
    }
    Ok(changed)
}

fn bool_or(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 2 {
        return Err(ResidentKernelError::InvalidInput);
    }
    write_bool(output, bool_scalar(inputs, 0)? || bool_scalar(inputs, 1)?)
}

fn bool_xor(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 2 {
        return Err(ResidentKernelError::InvalidInput);
    }
    write_bool(output, bool_scalar(inputs, 0)? ^ bool_scalar(inputs, 1)?)
}

fn bool_not(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 1 {
        return Err(ResidentKernelError::InvalidInput);
    }
    write_bool(output, !bool_scalar(inputs, 0)?)
}

fn bool_vector_not(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let Some(ResidentValueRef::Bool(input)) = inputs.get(0) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let ResidentValueMut::Bool(output) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    if input.len() != output.len() {
        return Err(ResidentKernelError::InvalidShape);
    }
    if input.iter().any(|value| *value > 1) {
        return Err(ResidentKernelError::InvalidInput);
    }
    let changed = input
        .iter()
        .zip(output.iter())
        .any(|(input, output)| *output != u8::from(*input == 0));
    for (input, output) in input.iter().zip(output.iter_mut()) {
        *output = u8::from(*input == 0);
    }
    Ok(changed)
}

fn admit_snapshot_equality_work(
    kernel: &BoundResidentKernel,
    left: &mech_core::Value,
    right: &mech_core::Value,
    materializes_canonical_bytes: bool,
) -> Result<(), ResidentKernelError> {
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let schema_comparison_work = if left.schema_key() == right.schema_key() {
        let left_schema = schemas
            .entry(left.schema())
            .ok_or(ResidentKernelError::InvalidInput)?
            .canonical_bytes();
        let right_schema = schemas
            .entry(right.schema())
            .ok_or(ResidentKernelError::InvalidInput)?
            .canonical_bytes();
        super::budget::checked_u64(left_schema.len().max(right_schema.len()))?
    } else {
        0
    };
    let mut meter = super::budget::ResidentBudgetMeter::default();
    let left_footprint =
        super::budget::measure_canonical_value_footprint(&mut meter, left, schemas)?;
    let right_footprint =
        super::budget::measure_canonical_value_footprint(&mut meter, right, schemas)?;
    let encoded_bytes = left_footprint
        .encoded_bytes
        .checked_add(right_footprint.encoded_bytes)
        .ok_or(ResidentKernelError::InvalidShape)?;
    // The bounded borrowed traversal above is part of the operation, then the
    // equality implementation performs one more complete data comparison. A
    // snapshot equality also encodes both canonical payloads and retains both
    // buffers until their byte comparison completes.
    let data_equality_work = if materializes_canonical_bytes {
        encoded_bytes
            .checked_add(
                left_footprint
                    .encoded_bytes
                    .min(right_footprint.encoded_bytes),
            )
            .ok_or(ResidentKernelError::InvalidShape)?
    } else {
        encoded_bytes.max(
            left_footprint
                .node_count
                .checked_add(right_footprint.node_count)
                .ok_or(ResidentKernelError::InvalidShape)?,
        )
    };
    let equality_work = schema_comparison_work
        .checked_add(data_equality_work)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let measured = meter.estimate();
    super::budget::PreparedKernel::new(
        (),
        super::budget::resident_cost! {
            comparison_work: measured.comparison_work()
                .checked_add(equality_work)
                .ok_or(ResidentKernelError::InvalidShape)?,
            compute_work: measured.compute_work()
                .checked_add(equality_work)
                .ok_or(ResidentKernelError::InvalidShape)?,
            output_elements: 1,
            output_bytes: core::mem::size_of::<u8>(),
            temporary_bytes: if materializes_canonical_bytes { encoded_bytes } else { 0 },
            cloned_bytes: if materializes_canonical_bytes { encoded_bytes } else { 0 },
            retained_nodes: measured.retained_nodes(),
            ..super::budget::KernelCostEstimate::default()
        },
    )
    .admit()
    .map(super::budget::AdmittedKernel::into_plan)
}

fn strict_value_equal(
    kernel: &BoundResidentKernel,
    left: ResidentValueRef<'_>,
    right: ResidentValueRef<'_>,
) -> Result<bool, ResidentKernelError> {
    Ok(match (left, right) {
        (ResidentValueRef::Bool(left), ResidentValueRef::Bool(right)) => {
            if left.iter().chain(right).any(|value| *value > 1) {
                return Err(ResidentKernelError::InvalidInput);
            }
            left == right
        }
        (ResidentValueRef::Index(left), ResidentValueRef::Index(right)) => left == right,
        (ResidentValueRef::F64(left), ResidentValueRef::F64(right)) => left == right,
        (ResidentValueRef::String(left), ResidentValueRef::String(right)) => left == right,
        (ResidentValueRef::Snapshot([Some(left)]), ResidentValueRef::Snapshot([Some(right)])) => {
            admit_snapshot_equality_work(kernel, left, right, false)?;
            left.language_eq(
                kernel
                    .snapshot_schemas()
                    .ok_or(ResidentKernelError::InvalidInput)?,
                right,
                kernel
                    .snapshot_schemas()
                    .ok_or(ResidentKernelError::InvalidInput)?,
            )
            .map_err(|_| ResidentKernelError::InvalidInput)?
        }
        (ResidentValueRef::Snapshot(_), ResidentValueRef::Snapshot(_)) => {
            return Err(ResidentKernelError::InvalidInput);
        }
        _ => false,
    })
}

fn snapshot_value_equal(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 2 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let (ResidentValueRef::Snapshot([Some(left)]), ResidentValueRef::Snapshot([Some(right)])) =
        (input(inputs, 0)?, input(inputs, 1)?)
    else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidInput)?;
    admit_snapshot_equality_work(kernel, left, right, true)?;
    left.snapshot_eq(schemas, right, schemas)
        .map_err(|_| ResidentKernelError::InvalidInput)
}

fn snapshot_equal(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    write_bool(output, snapshot_value_equal(kernel, inputs)?)
}

fn snapshot_not_equal(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    write_bool(output, !snapshot_value_equal(kernel, inputs)?)
}

fn strict_equal(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 2 {
        return Err(ResidentKernelError::InvalidInput);
    }
    write_bool(
        output,
        strict_value_equal(kernel, input(inputs, 0)?, input(inputs, 1)?)?,
    )
}

fn strict_always_false(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 2 {
        return Err(ResidentKernelError::InvalidInput);
    }
    write_bool(output, false)
}

fn strict_always_true(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 2 {
        return Err(ResidentKernelError::InvalidInput);
    }
    write_bool(output, true)
}

fn strict_not_equal(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 2 {
        return Err(ResidentKernelError::InvalidInput);
    }
    write_bool(
        output,
        !strict_value_equal(kernel, input(inputs, 0)?, input(inputs, 1)?)?,
    )
}

fn power(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    binary_f64(kernel, inputs, output, f64::powf)
}

fn multiply_rows(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 2 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let matrix = f64_input(inputs, 0)?;
    let vector = f64_input(inputs, 1)?;
    let output = f64_output(output)?;
    let rows = kernel.parameters()[0] as usize;
    if matrix.len() != output.len() || vector.len() != rows {
        return Err(ResidentKernelError::InvalidShape);
    }
    Ok(replace_f64(output, |index| {
        matrix[index] * vector[index % rows]
    }))
}

fn add_assign(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 1 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let source = f64_input(inputs, 0)?;
    let output = f64_output(output)?;
    if source.len() != output.len() {
        return Err(ResidentKernelError::InvalidShape);
    }
    let mut changed = false;
    for (target, source) in output.iter_mut().zip(source) {
        let next = *target + *source;
        changed |= next.to_bits() != target.to_bits();
        *target = next;
    }
    Ok(changed)
}

fn transpose_dense(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 1 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let rows = kernel.parameters()[0] as usize;
    let columns = kernel.parameters()[1] as usize;
    let count = rows
        .checked_mul(columns)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let kind = match &output {
        ResidentValueMut::Bool(_) => ResidentValueKind::Bool,
        ResidentValueMut::Index(_) => ResidentValueKind::Index,
        ResidentValueMut::F64(_) => ResidentValueKind::F64,
        _ => return Err(ResidentKernelError::InvalidOutput),
    };
    admit_dense_transpose_layout(
        kind,
        ResidentShape {
            rows: u32::try_from(columns).map_err(|_| ResidentKernelError::InvalidShape)?,
            columns: u32::try_from(rows).map_err(|_| ResidentKernelError::InvalidShape)?,
        },
    )?;
    let source_index = |index: usize| {
        let output_row = index % columns;
        let output_column = index / columns;
        output_column + output_row * rows
    };
    match (input(inputs, 0)?, output) {
        (ResidentValueRef::Bool(input), ResidentValueMut::Bool(output))
            if input.len() == count && output.len() == count =>
        {
            if input.iter().any(|value| *value > 1) {
                return Err(ResidentKernelError::InvalidInput);
            }
            let mut changed = false;
            for (index, target) in output.iter_mut().enumerate() {
                let next = input[source_index(index)];
                changed |= *target != next;
                *target = next;
            }
            Ok(changed)
        }
        (ResidentValueRef::Index(input), ResidentValueMut::Index(output))
            if input.len() == count && output.len() == count =>
        {
            let mut changed = false;
            for (index, target) in output.iter_mut().enumerate() {
                let next = input[source_index(index)];
                changed |= *target != next;
                *target = next;
            }
            Ok(changed)
        }
        (ResidentValueRef::F64(input), ResidentValueMut::F64(output))
            if input.len() == count && output.len() == count =>
        {
            Ok(replace_f64(output, |index| input[source_index(index)]))
        }
        _ => Err(ResidentKernelError::InvalidShape),
    }
}

fn transpose_snapshot(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let Some(ResidentValueRef::Snapshot([Some(input)])) = inputs.get(0) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let [rows, columns] = kernel.parameters() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let rows = usize::try_from(*rows).map_err(|_| ResidentKernelError::InvalidShape)?;
    let columns = usize::try_from(*columns).map_err(|_| ResidentKernelError::InvalidShape)?;
    let count = rows
        .checked_mul(columns)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let ValueData::Matrix(matrix) = input.data() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    if matrix.elements().len() != count {
        return Err(ResidentKernelError::InvalidShape);
    }
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let ResidentValueMut::Snapshot([target]) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let mut footprint_meter = super::budget::ResidentBudgetMeter::default();
    let input_footprint =
        super::budget::measure_canonical_value_footprint(&mut footprint_meter, input, schemas)?;
    let input_bytes = input_footprint.retained_bytes;
    let retained_nodes = input_footprint.node_count;
    let input_schema = input
        .validate_against(schemas)
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    let finalization_work =
        snapshot_data_finalization_work(&mut footprint_meter, input_schema.body(), input.data())?;
    let prior_footprint = match target.as_ref() {
        Some(current) => Some(super::budget::measure_canonical_value_footprint(
            &mut footprint_meter,
            current,
            schemas,
        )?),
        None => None,
    };
    let metadata = kernel
        .snapshot_output()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let output_footprint = ValueFootprint {
        encoded_bytes: input_footprint.encoded_bytes,
        retained_bytes: input_bytes,
        node_count: retained_nodes,
    };
    let publication_work = match (target.as_ref(), prior_footprint) {
        (Some(current), Some(prior)) => super::budget::projected_language_equality_work(
            schemas,
            current,
            prior,
            metadata.schema,
            metadata.shape.parameter_values().len(),
            output_footprint,
        )?,
        _ => 0,
    };
    let container_bytes = super::budget::checked_u64(
        count
            .checked_mul(core::mem::size_of::<ValueDataDraft>())
            .ok_or(ResidentKernelError::InvalidShape)?,
    )?;
    let measured = footprint_meter.estimate();
    super::budget::PreparedKernel::new(
        (),
        super::budget::resident_cost! {
            comparison_work: measured.comparison_work()
                .checked_add(publication_work)
                .ok_or(ResidentKernelError::InvalidShape)?,
            compute_work: measured.compute_work()
                .checked_add(super::budget::checked_u64(count)?)
                .and_then(|work| work.checked_add(publication_work))
                .ok_or(ResidentKernelError::InvalidShape)?,
            output_elements: count,
            output_bytes: input_bytes,
            temporary_bytes: input_bytes
                .checked_mul(2)
                .and_then(|bytes| bytes.checked_add(container_bytes))
                .ok_or(ResidentKernelError::InvalidShape)?,
            cloned_bytes: input_bytes
                .checked_mul(2)
                .ok_or(ResidentKernelError::InvalidShape)?,
            retained_nodes: measured.retained_nodes()
                .checked_add(retained_nodes.checked_mul(2).ok_or(ResidentKernelError::InvalidShape)?)
                .ok_or(ResidentKernelError::InvalidShape)?,
            ..super::budget::KernelCostEstimate::default()
        },
    )
    .admit()?
    .into_plan();
    let ValueDataDraft::Matrix(elements) = input
        .canonical_data_draft()
        .map_err(|_| ResidentKernelError::InvalidInput)?
    else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let mut transposed = Vec::with_capacity(count);
    for output_row in 0..columns {
        for output_column in 0..rows {
            transposed.push(elements[output_column * columns + output_row].clone());
        }
    }
    write_snapshot_data_with_work_budget(
        kernel,
        ResidentValueMut::Snapshot(core::slice::from_mut(target)),
        ValueDataDraft::Matrix(transposed.into_boxed_slice()),
        Some(finalization_work),
    )
}

fn sum_columns(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 1 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let input = f64_input(inputs, 0)?;
    let output = f64_output(output)?;
    let rows = kernel.parameters()[0] as usize;
    let columns = kernel.parameters()[1] as usize;
    if input.len() != rows * columns || output.len() != rows {
        return Err(ResidentKernelError::InvalidShape);
    }
    Ok(replace_f64(output, |row| {
        (0..columns).map(|column| input[row + column * rows]).sum()
    }))
}

fn sum_rows(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 1 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let input = f64_input(inputs, 0)?;
    let output = f64_output(output)?;
    let rows = kernel.parameters()[0] as usize;
    let columns = kernel.parameters()[1] as usize;
    if input.len() != rows * columns || output.len() != columns {
        return Err(ResidentKernelError::InvalidShape);
    }
    Ok(replace_f64(output, |column| {
        (0..rows).map(|row| input[row + column * rows]).sum()
    }))
}

fn sum_snapshot(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let [rows, columns, column] = kernel.parameters() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let (rows, columns) = (
        usize::try_from(*rows).map_err(|_| ResidentKernelError::InvalidShape)?,
        usize::try_from(*columns).map_err(|_| ResidentKernelError::InvalidShape)?,
    );
    let column = match *column {
        0 => false,
        1 => true,
        _ => return Err(ResidentKernelError::InvalidInput),
    };
    let Some(ResidentValueRef::Snapshot([Some(input)])) = inputs.get(0) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let input_count = rows
        .checked_mul(columns)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let output_count = if column { rows } else { columns };
    if snapshot_numeric_element_count(input)? != input_count {
        return Err(ResidentKernelError::InvalidShape);
    }
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let schema = input
        .validate_against(schemas)
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    let SchemaBody::Matrix { element, .. } = schema.body() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    preflight_snapshot_arithmetic(
        kernel,
        schemas,
        &[input],
        &output,
        input_count,
        output_count,
        input_count,
    )?;
    let values = snapshot_numeric_elements(input)?;
    let mut sums = Vec::with_capacity(output_count);
    if column {
        for row in 0..rows {
            let mut sum = numeric_zero(element)?;
            for column in 0..columns {
                sum = numeric_add(sum, values[row * columns + column].clone())?;
            }
            sums.push(sum);
        }
    } else {
        for column in 0..columns {
            let mut sum = numeric_zero(element)?;
            for row in 0..rows {
                sum = numeric_add(sum, values[row * columns + column].clone())?;
            }
            sums.push(sum);
        }
    }
    write_snapshot_data_with_work_budget(
        kernel,
        output,
        ValueDataDraft::Matrix(sums.into_boxed_slice()),
        Some(0),
    )
}

fn admit_dense_concatenation(
    kind: ResidentValueKind,
    output_len: usize,
    string_payload_bytes: usize,
) -> Result<(), ResidentKernelError> {
    let element_bytes = match kind {
        ResidentValueKind::Bool => core::mem::size_of::<u8>(),
        ResidentValueKind::Index => core::mem::size_of::<u64>(),
        ResidentValueKind::F64 => core::mem::size_of::<f64>(),
        ResidentValueKind::String => core::mem::size_of::<String>(),
        ResidentValueKind::Snapshot => return Err(ResidentKernelError::InvalidOutput),
    };
    let resident_bytes = output_len
        .checked_mul(element_bytes)
        .and_then(|bytes| bytes.checked_add(string_payload_bytes))
        .ok_or(ResidentKernelError::InvalidShape)?;
    super::budget::PreparedKernel::new(
        (),
        super::budget::resident_cost! {
            compute_work: output_len
                .checked_mul(2)
                .ok_or(ResidentKernelError::InvalidShape)?,
            output_elements: output_len,
            output_bytes: resident_bytes,
            temporary_bytes: if kind == ResidentValueKind::String {
                string_payload_bytes
            } else {
                resident_bytes
            },
            cloned_bytes: string_payload_bytes,
            container_bytes: if kind == ResidentValueKind::String {
                output_len
                    .checked_mul(core::mem::size_of::<String>())
                    .ok_or(ResidentKernelError::InvalidShape)?
            } else {
                0
            },
            retained_nodes: output_len,
            ..super::budget::KernelCostEstimate::default()
        },
    )
    .admit()?
    .into_plan();
    Ok(())
}

fn concat_string_payload(inputs: &dyn ResidentKernelInputs) -> Result<usize, ResidentKernelError> {
    (0..inputs.len()).try_fold(0usize, |bytes, ordinal| {
        let ResidentValueRef::String(values) = input(inputs, ordinal)? else {
            return Err(ResidentKernelError::InvalidInput);
        };
        values.iter().try_fold(bytes, |bytes, value| {
            bytes
                .checked_add(value.len())
                .ok_or(ResidentKernelError::InvalidShape)
        })
    })
}

fn concatenate_horizontal(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if kernel.parameters().first().copied() != Some(inputs.len() as u64) {
        return Err(ResidentKernelError::InvalidInput);
    }
    let rows = kernel.parameters().get(1).copied().unwrap_or(1) as usize;
    let output_kind = output.kind();
    let output_len = output.len();
    let mut expected_output = 0usize;
    for ordinal in 0..inputs.len() {
        let source = input(inputs, ordinal)?;
        let input_rows = kernel.parameters()[1 + ordinal * 2] as usize;
        let input_columns = kernel.parameters()[2 + ordinal * 2] as usize;
        let len = input_rows
            .checked_mul(input_columns)
            .ok_or(ResidentKernelError::InvalidShape)?;
        if input_rows != rows || source.len() != len || source.kind() != output_kind {
            return Err(ResidentKernelError::InvalidShape);
        }
        expected_output = expected_output
            .checked_add(len)
            .ok_or(ResidentKernelError::InvalidShape)?;
    }
    if output_len != expected_output {
        return Err(ResidentKernelError::IncompleteOutput);
    }
    match output {
        ResidentValueMut::Bool(output) if output_kind == ResidentValueKind::Bool => {
            for ordinal in 0..inputs.len() {
                let ResidentValueRef::Bool(values) = input(inputs, ordinal)? else {
                    return Err(ResidentKernelError::InvalidInput);
                };
                if values.iter().any(|value| *value > 1) {
                    return Err(ResidentKernelError::InvalidInput);
                }
            }
            admit_dense_concatenation(output_kind, output_len, 0)?;
            let mut next = Vec::with_capacity(output_len);
            for ordinal in 0..inputs.len() {
                let ResidentValueRef::Bool(values) = input(inputs, ordinal)? else {
                    unreachable!("concatenation inputs were validated before admission")
                };
                next.extend_from_slice(values);
            }
            let changed = output != next;
            output.copy_from_slice(&next);
            Ok(changed)
        }
        ResidentValueMut::Index(output) if output_kind == ResidentValueKind::Index => {
            admit_dense_concatenation(output_kind, output_len, 0)?;
            let mut next = Vec::with_capacity(output_len);
            for ordinal in 0..inputs.len() {
                let ResidentValueRef::Index(values) = input(inputs, ordinal)? else {
                    return Err(ResidentKernelError::InvalidInput);
                };
                next.extend_from_slice(values);
            }
            let changed = output != next;
            output.copy_from_slice(&next);
            Ok(changed)
        }
        ResidentValueMut::F64(output) if output_kind == ResidentValueKind::F64 => {
            admit_dense_concatenation(output_kind, output_len, 0)?;
            let mut next = Vec::with_capacity(output_len);
            for ordinal in 0..inputs.len() {
                let ResidentValueRef::F64(values) = input(inputs, ordinal)? else {
                    return Err(ResidentKernelError::InvalidInput);
                };
                next.extend_from_slice(values);
            }
            let changed = output
                .iter()
                .zip(&next)
                .any(|(left, right)| left.to_bits() != right.to_bits());
            output.copy_from_slice(&next);
            Ok(changed)
        }
        ResidentValueMut::String(output) if output_kind == ResidentValueKind::String => {
            let payload_bytes = concat_string_payload(inputs)?;
            admit_dense_concatenation(output_kind, output_len, payload_bytes)?;
            let mut next = Vec::with_capacity(output_len);
            for ordinal in 0..inputs.len() {
                let ResidentValueRef::String(values) = input(inputs, ordinal)? else {
                    unreachable!("concatenation inputs were validated before admission")
                };
                next.extend(values.iter().cloned());
            }
            let changed = output != next;
            for (target, value) in output.iter_mut().zip(next) {
                *target = value;
            }
            Ok(changed)
        }
        _ => Err(ResidentKernelError::InvalidOutput),
    }
}

fn concatenate_vertical(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if kernel.parameters().first().copied() != Some(inputs.len() as u64) {
        return Err(ResidentKernelError::InvalidInput);
    }
    let columns = kernel.parameters().get(2).copied().unwrap_or(0) as usize;
    let output_kind = output.kind();
    let output_len = output.len();
    let mut output_rows = 0usize;
    for ordinal in 0..inputs.len() {
        let source = input(inputs, ordinal)?;
        let rows = kernel.parameters()[1 + ordinal * 2] as usize;
        let input_columns = kernel.parameters()[2 + ordinal * 2] as usize;
        let len = rows
            .checked_mul(columns)
            .ok_or(ResidentKernelError::InvalidShape)?;
        if input_columns != columns || source.len() != len || source.kind() != output_kind {
            return Err(ResidentKernelError::InvalidShape);
        }
        output_rows = output_rows
            .checked_add(rows)
            .ok_or(ResidentKernelError::InvalidShape)?;
    }
    if output_rows.checked_mul(columns) != Some(output_len) {
        return Err(ResidentKernelError::IncompleteOutput);
    }
    macro_rules! stage_vertical {
        ($variant:ident, $zero:expr) => {{
            let mut next = vec![$zero; output_len];
            let mut row_base = 0usize;
            for ordinal in 0..inputs.len() {
                let ResidentValueRef::$variant(values) = input(inputs, ordinal)? else {
                    return Err(ResidentKernelError::InvalidInput);
                };
                let rows = kernel.parameters()[1 + ordinal * 2] as usize;
                for column in 0..columns {
                    for row in 0..rows {
                        next[row_base + row + column * output_rows] =
                            values[row + column * rows].clone();
                    }
                }
                row_base += rows;
            }
            next
        }};
    }
    match output {
        ResidentValueMut::Bool(output) if output_kind == ResidentValueKind::Bool => {
            for ordinal in 0..inputs.len() {
                let ResidentValueRef::Bool(values) = input(inputs, ordinal)? else {
                    return Err(ResidentKernelError::InvalidInput);
                };
                if values.iter().any(|value| *value > 1) {
                    return Err(ResidentKernelError::InvalidInput);
                }
            }
            admit_dense_concatenation(output_kind, output_len, 0)?;
            let next = stage_vertical!(Bool, 0_u8);
            let changed = output != next;
            output.copy_from_slice(&next);
            Ok(changed)
        }
        ResidentValueMut::Index(output) if output_kind == ResidentValueKind::Index => {
            admit_dense_concatenation(output_kind, output_len, 0)?;
            let next = stage_vertical!(Index, 0_u64);
            let changed = output != next;
            output.copy_from_slice(&next);
            Ok(changed)
        }
        ResidentValueMut::F64(output) if output_kind == ResidentValueKind::F64 => {
            admit_dense_concatenation(output_kind, output_len, 0)?;
            let next = stage_vertical!(F64, 0.0_f64);
            let changed = output
                .iter()
                .zip(&next)
                .any(|(left, right)| left.to_bits() != right.to_bits());
            output.copy_from_slice(&next);
            Ok(changed)
        }
        ResidentValueMut::String(output) if output_kind == ResidentValueKind::String => {
            let payload_bytes = concat_string_payload(inputs)?;
            admit_dense_concatenation(output_kind, output_len, payload_bytes)?;
            let next = stage_vertical!(String, String::new());
            let changed = output != next;
            for (target, value) in output.iter_mut().zip(next) {
                *target = value;
            }
            Ok(changed)
        }
        _ => Err(ResidentKernelError::InvalidOutput),
    }
}

fn constructor_snapshot_inputs(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    previous: Option<&mech_core::Value>,
) -> Result<(Vec<Vec<ValueDataDraft>>, usize, u64), ResidentKernelError> {
    if kernel.parameters().first().copied() != Some(inputs.len() as u64) {
        return Err(ResidentKernelError::InvalidInput);
    }
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let mut retained_bytes = 0u64;
    let mut input_nodes = 0u64;
    let mut input_encoded_bytes = 0u64;
    let mut output_count = 0usize;
    let mut finalization_work = 0u64;
    let mut footprint_meter = super::budget::ResidentBudgetMeter::default();
    for ordinal in 0..inputs.len() {
        let Some(ResidentValueRef::Snapshot([Some(input)])) = inputs.get(ordinal) else {
            return Err(ResidentKernelError::InvalidInput);
        };
        let rows = kernel.parameters()[1 + ordinal * 2] as usize;
        let columns = kernel.parameters()[2 + ordinal * 2] as usize;
        let count = rows
            .checked_mul(columns)
            .ok_or(ResidentKernelError::InvalidShape)?;
        let actual = match input.data() {
            ValueData::Matrix(matrix) => matrix.elements().len(),
            _ => 1,
        };
        if actual != count {
            return Err(ResidentKernelError::InvalidShape);
        }
        output_count = output_count
            .checked_add(count)
            .ok_or(ResidentKernelError::InvalidShape)?;
        let footprint =
            super::budget::measure_canonical_value_footprint(&mut footprint_meter, input, schemas)?;
        retained_bytes = retained_bytes
            .checked_add(footprint.retained_bytes)
            .ok_or(ResidentKernelError::InvalidShape)?;
        input_nodes = input_nodes
            .checked_add(footprint.node_count)
            .ok_or(ResidentKernelError::InvalidShape)?;
        input_encoded_bytes = input_encoded_bytes
            .checked_add(footprint.encoded_bytes)
            .ok_or(ResidentKernelError::InvalidShape)?;
        let input_schema = input
            .validate_against(schemas)
            .map_err(|_| ResidentKernelError::InvalidInput)?;
        finalization_work = finalization_work
            .checked_add(snapshot_data_finalization_work(
                &mut footprint_meter,
                input_schema.body(),
                input.data(),
            )?)
            .ok_or(ResidentKernelError::InvalidShape)?;
    }
    let previous_footprint = match previous {
        Some(previous) => super::budget::measure_canonical_value_footprint(
            &mut footprint_meter,
            previous,
            schemas,
        )?,
        None => ValueFootprint::zero(),
    };
    let container_bytes = output_count
        .checked_mul(2)
        .and_then(|count| count.checked_mul(core::mem::size_of::<ValueDataDraft>()))
        .ok_or(ResidentKernelError::InvalidShape)?;
    let input_wrappers = super::budget::checked_u64(inputs.len())?;
    let staged_nodes = input_nodes
        .checked_sub(input_wrappers)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let result_nodes = staged_nodes
        .checked_add(1)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let final_nodes = result_nodes
        .checked_add(1)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let output_metadata = kernel
        .snapshot_output()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let final_output_footprint = ValueFootprint {
        encoded_bytes: input_encoded_bytes,
        retained_bytes,
        node_count: final_nodes,
    };
    let change_detection_work = match previous {
        Some(previous) => super::budget::projected_language_equality_work(
            schemas,
            previous,
            previous_footprint,
            output_metadata.schema,
            output_metadata.shape.parameter_values().len(),
            final_output_footprint,
        )?,
        None => 0,
    };
    let footprint_work = footprint_meter.estimate();
    let cost = super::budget::resident_cost! {
        comparison_work: footprint_work.comparison_work()
            .checked_add(change_detection_work)
            .ok_or(ResidentKernelError::InvalidShape)?,
        compute_work: super::budget::checked_u64(output_count)?
            .checked_add(footprint_work.compute_work())
            .and_then(|work| work.checked_add(change_detection_work))
            .ok_or(ResidentKernelError::InvalidShape)?,
        output_elements: output_count,
        output_bytes: retained_bytes,
        temporary_bytes: retained_bytes
            .checked_mul(2)
            .ok_or(ResidentKernelError::InvalidShape)?,
        cloned_bytes: retained_bytes
            .checked_mul(2)
            .ok_or(ResidentKernelError::InvalidShape)?,
        container_bytes,
        retained_nodes: super::budget::checked_cost_sum(&[
            footprint_work.retained_nodes(),
            staged_nodes,
            result_nodes,
            final_nodes,
        ])?,
        ..super::budget::KernelCostEstimate::default()
    };
    let (staged_capacity, canonicalization_work_limit) =
        super::budget::PreparedKernel::new((inputs.len(), finalization_work), cost)
            .admit()?
            .into_plan();
    let mut staged = Vec::with_capacity(staged_capacity);
    for ordinal in 0..inputs.len() {
        let Some(ResidentValueRef::Snapshot([Some(input)])) = inputs.get(ordinal) else {
            return Err(ResidentKernelError::InvalidInput);
        };
        staged.push(
            match input
                .canonical_data_draft()
                .map_err(|_| ResidentKernelError::InvalidInput)?
            {
                ValueDataDraft::Matrix(elements) => elements.into_vec(),
                scalar => vec![scalar],
            },
        );
    }
    Ok((staged, output_count, canonicalization_work_limit))
}

fn concatenate_horizontal_snapshot(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let ResidentValueMut::Snapshot([target]) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let (staged, output_count, canonicalization_work_limit) =
        constructor_snapshot_inputs(kernel, inputs, target.as_ref())?;
    let rows = kernel.parameters().get(1).copied().unwrap_or(0) as usize;
    let mut result = Vec::with_capacity(output_count);
    for row in 0..rows {
        for (ordinal, values) in staged.iter().enumerate() {
            let columns = kernel.parameters()[2 + ordinal * 2] as usize;
            let start = row
                .checked_mul(columns)
                .ok_or(ResidentKernelError::InvalidShape)?;
            result.extend_from_slice(&values[start..start + columns]);
        }
    }
    if result.len() != output_count {
        return Err(ResidentKernelError::IncompleteOutput);
    }
    write_snapshot_data_with_work_budget(
        kernel,
        ResidentValueMut::Snapshot(core::slice::from_mut(target)),
        ValueDataDraft::Matrix(result.into_boxed_slice()),
        Some(canonicalization_work_limit),
    )
}

fn concatenate_vertical_snapshot(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let ResidentValueMut::Snapshot([target]) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let (staged, output_count, canonicalization_work_limit) =
        constructor_snapshot_inputs(kernel, inputs, target.as_ref())?;
    let mut result = Vec::with_capacity(output_count);
    for values in staged {
        result.extend(values);
    }
    if result.len() != output_count {
        return Err(ResidentKernelError::IncompleteOutput);
    }
    write_snapshot_data_with_work_budget(
        kernel,
        ResidentValueMut::Snapshot(core::slice::from_mut(target)),
        ValueDataDraft::Matrix(result.into_boxed_slice()),
        Some(canonicalization_work_limit),
    )
}

fn admit_dense_range(output_len: usize) -> Result<(), ResidentKernelError> {
    let bytes = output_len
        .checked_mul(core::mem::size_of::<f64>())
        .ok_or(ResidentKernelError::InvalidShape)?;
    super::budget::PreparedKernel::new(
        (),
        super::budget::resident_cost! {
            compute_work: output_len,
            output_elements: output_len,
            output_bytes: bytes,
            ..super::budget::KernelCostEstimate::default()
        },
    )
    .admit()?
    .into_plan();
    Ok(())
}

fn range_inclusive(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 2 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let start = *f64_input(inputs, 0)?
        .first()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let end = *f64_input(inputs, 1)?
        .first()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let output = f64_output(output)?;
    let difference = end - start;
    if !start.is_finite()
        || difference < 0.0
        || !difference.is_finite()
        || difference >= usize::MAX as f64
    {
        return Err(ResidentKernelError::InvalidInput);
    }
    let expected_len = difference.floor() as usize + 1;
    if output.len() != expected_len {
        return Err(ResidentKernelError::InvalidShape);
    }
    admit_dense_range(output.len())?;
    Ok(replace_f64_range(output, start, 1.0))
}

fn range_exclusive(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 2 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let start = *f64_input(inputs, 0)?
        .first()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let end = *f64_input(inputs, 1)?
        .first()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let output = f64_output(output)?;
    let expected_len = exclusive_range_len(start, 1.0, end)?;
    if output.len() != expected_len {
        return Err(ResidentKernelError::InvalidShape);
    }
    admit_dense_range(output.len())?;
    Ok(replace_f64_range(output, start, 1.0))
}

fn range_increment_inclusive(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 3 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let start = *f64_input(inputs, 0)?
        .first()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let step = *f64_input(inputs, 1)?
        .first()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let end = *f64_input(inputs, 2)?
        .first()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let output = f64_output(output)?;
    let difference = end - start;
    if !start.is_finite()
        || !step.is_finite()
        || !end.is_finite()
        || step == 0.0
        || (difference > 0.0 && step < 0.0)
        || (difference < 0.0 && step > 0.0)
    {
        return Err(ResidentKernelError::InvalidInput);
    }
    let expected_len = if difference == 0.0 {
        1
    } else {
        let intervals = (difference / step).floor();
        if !intervals.is_finite() || intervals < 0.0 || intervals >= usize::MAX as f64 {
            return Err(ResidentKernelError::InvalidShape);
        }
        intervals as usize + 1
    };
    if output.len() != expected_len {
        return Err(ResidentKernelError::InvalidShape);
    }
    admit_dense_range(output.len())?;
    Ok(replace_f64_range(output, start, step))
}

fn range_increment_exclusive(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 3 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let start = *f64_input(inputs, 0)?
        .first()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let step = *f64_input(inputs, 1)?
        .first()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let end = *f64_input(inputs, 2)?
        .first()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let output = f64_output(output)?;
    let expected_len = exclusive_range_len(start, step, end)?;
    if output.len() != expected_len {
        return Err(ResidentKernelError::InvalidShape);
    }
    admit_dense_range(output.len())?;
    Ok(replace_f64_range(output, start, step))
}

fn replace_f64_range(output: &mut [f64], start: f64, step: f64) -> bool {
    let mut current = start;
    let last = output.len().saturating_sub(1);
    replace_f64(output, |index| {
        let value = current;
        if index < last {
            current += step;
        }
        value
    })
}

fn exclusive_range_len(start: f64, step: f64, end: f64) -> Result<usize, ResidentKernelError> {
    if !start.is_finite() || !step.is_finite() || !end.is_finite() || step == 0.0 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let difference = end - start;
    if difference == 0.0 || (difference > 0.0 && step < 0.0) || (difference < 0.0 && step > 0.0) {
        return Err(ResidentKernelError::InvalidInput);
    }
    let length = (difference / step).ceil();
    if !length.is_finite() || length <= 0.0 || length >= usize::MAX as f64 {
        return Err(ResidentKernelError::InvalidShape);
    }
    Ok(length as usize)
}

#[derive(Clone, Copy)]
enum SnapshotRangeNumber {
    Unsigned(u128),
    Signed(i128),
    Float(f64),
}

fn snapshot_range_number(data: &ValueData) -> Option<SnapshotRangeNumber> {
    Some(match data {
        ValueData::U8(value) => SnapshotRangeNumber::Unsigned(u128::from(*value)),
        ValueData::U16(value) => SnapshotRangeNumber::Unsigned(u128::from(*value)),
        ValueData::U32(value) => SnapshotRangeNumber::Unsigned(u128::from(*value)),
        ValueData::U64(value) => SnapshotRangeNumber::Unsigned(u128::from(*value)),
        ValueData::U128(value) => SnapshotRangeNumber::Unsigned(*value),
        ValueData::I8(value) => SnapshotRangeNumber::Signed(i128::from(*value)),
        ValueData::I16(value) => SnapshotRangeNumber::Signed(i128::from(*value)),
        ValueData::I32(value) => SnapshotRangeNumber::Signed(i128::from(*value)),
        ValueData::I64(value) => SnapshotRangeNumber::Signed(i128::from(*value)),
        ValueData::I128(value) => SnapshotRangeNumber::Signed(*value),
        ValueData::F32(value) => SnapshotRangeNumber::Float(f64::from(value.to_f32())),
        ValueData::F64(value) => SnapshotRangeNumber::Float(value.to_f64()),
        _ => return None,
    })
}

fn integer_snapshot_range_size(magnitude: u128, step: u128, inclusive: bool) -> Option<usize> {
    let size = if inclusive {
        magnitude.checked_div(step)?.checked_add(1)?
    } else {
        let quotient = magnitude.checked_div(step)?;
        quotient.checked_add(u128::from(magnitude % step != 0))?
    };
    usize::try_from(size).ok()
}

fn float_snapshot_range_size(from: f64, step: f64, to: f64, inclusive: bool) -> Option<usize> {
    if !from.is_finite() || !step.is_finite() || !to.is_finite() || step == 0.0 {
        return None;
    }
    let difference = to - from;
    let size = if difference == 0.0 {
        if inclusive { 1.0 } else { 0.0 }
    } else if (difference > 0.0 && step > 0.0) || (difference < 0.0 && step < 0.0) {
        let quotient = difference / step;
        if inclusive {
            quotient.floor() + 1.0
        } else {
            quotient.ceil()
        }
    } else {
        0.0
    };
    if !size.is_finite() || size < 0.0 || size >= usize::MAX as f64 {
        return None;
    }
    Some(size as usize)
}

fn snapshot_range_size(
    values: &[SnapshotRangeNumber],
    inclusive: bool,
    incremented: bool,
) -> Option<usize> {
    match (values, incremented) {
        (
            [
                SnapshotRangeNumber::Unsigned(from),
                SnapshotRangeNumber::Unsigned(to),
            ],
            false,
        ) => integer_snapshot_range_size(to.checked_sub(*from)?, 1, inclusive),
        (
            [
                SnapshotRangeNumber::Signed(from),
                SnapshotRangeNumber::Signed(to),
            ],
            false,
        ) => {
            if to < from {
                Some(0)
            } else {
                integer_snapshot_range_size(to.abs_diff(*from), 1, inclusive)
            }
        }
        (
            [
                SnapshotRangeNumber::Float(from),
                SnapshotRangeNumber::Float(to),
            ],
            false,
        ) => float_snapshot_range_size(*from, 1.0, *to, inclusive),
        (
            [
                SnapshotRangeNumber::Unsigned(from),
                SnapshotRangeNumber::Unsigned(step),
                SnapshotRangeNumber::Unsigned(to),
            ],
            true,
        ) => {
            if *step == 0 {
                None
            } else if to < from {
                Some(0)
            } else {
                integer_snapshot_range_size(to - from, *step, inclusive)
            }
        }
        (
            [
                SnapshotRangeNumber::Signed(from),
                SnapshotRangeNumber::Signed(step),
                SnapshotRangeNumber::Signed(to),
            ],
            true,
        ) => {
            if *step == 0 {
                return None;
            }
            if from == to {
                return Some(usize::from(inclusive));
            }
            if (to > from && *step < 0) || (to < from && *step > 0) {
                return Some(0);
            }
            integer_snapshot_range_size(to.abs_diff(*from), step.unsigned_abs(), inclusive)
        }
        (
            [
                SnapshotRangeNumber::Float(from),
                SnapshotRangeNumber::Float(step),
                SnapshotRangeNumber::Float(to),
            ],
            true,
        ) => float_snapshot_range_size(*from, *step, *to, inclusive),
        _ => None,
    }
}

pub(super) fn canonical_range_cardinality(
    values: &[&mech_core::Value],
    inclusive: bool,
    incremented: bool,
) -> Option<usize> {
    let numbers = values
        .iter()
        .map(|value| snapshot_range_number(value.data()))
        .collect::<Option<Vec<_>>>()?;
    snapshot_range_size(&numbers, inclusive, incremented)
}

fn range_snapshot(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let [inclusive, incremented, declared_count] = kernel.parameters() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let inclusive = match *inclusive {
        0 => false,
        1 => true,
        _ => return Err(ResidentKernelError::InvalidInput),
    };
    let incremented = match *incremented {
        0 => false,
        1 => true,
        _ => return Err(ResidentKernelError::InvalidInput),
    };
    let expected_inputs = if incremented { 3 } else { 2 };
    if inputs.len() != expected_inputs {
        return Err(ResidentKernelError::InvalidInput);
    }
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let value = |index| match inputs.get(index) {
        Some(ResidentValueRef::Snapshot([Some(value)])) => {
            value
                .validate_against(schemas)
                .map_err(|_| ResidentKernelError::InvalidInput)?;
            Ok(value)
        }
        _ => Err(ResidentKernelError::InvalidInput),
    };
    let first = value(0)?;
    let second = value(1)?;
    let third = if incremented { value(2)? } else { second };
    let values = [first, second, third];
    let values = &values[..expected_inputs];
    let numbers = [
        snapshot_range_number(first.data()).ok_or(ResidentKernelError::InvalidInput)?,
        snapshot_range_number(second.data()).ok_or(ResidentKernelError::InvalidInput)?,
        snapshot_range_number(third.data()).ok_or(ResidentKernelError::InvalidInput)?,
    ];
    let count = snapshot_range_size(&numbers[..expected_inputs], inclusive, incremented)
        .filter(|count| *count != 0)
        .ok_or(ResidentKernelError::InvalidInput)?;
    if u64::try_from(count).ok() != Some(*declared_count) {
        return Err(ResidentKernelError::InvalidShape);
    }
    preflight_snapshot_arithmetic(kernel, schemas, &values, &output, values.len(), count, 0)?;
    let schema = first
        .validate_against(schemas)
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    let mut current = first
        .canonical_data_draft()
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    let step = if incremented {
        second
            .canonical_data_draft()
            .map_err(|_| ResidentKernelError::InvalidInput)?
    } else {
        numeric_one(schema.body())?
    };
    let mut elements = Vec::with_capacity(count);
    for index in 0..count {
        elements.push(current.clone());
        if index + 1 < count {
            current = numeric_add(current, step.clone())?;
        }
    }
    write_snapshot_data_with_work_budget(
        kernel,
        output,
        ValueDataDraft::Matrix(elements.into_boxed_slice()),
        Some(0),
    )
}

fn n_choose_k(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 2 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let values = f64_input(inputs, 0)?;
    let requested = *f64_input(inputs, 1)?
        .first()
        .ok_or(ResidentKernelError::InvalidInput)?;
    if !requested.is_finite() || requested.fract() != 0.0 || requested < 0.0 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let k = requested as usize;
    let output = f64_output(output)?;
    let declared_k = kernel.parameters().first().copied().unwrap_or(0) as usize;
    let declared_combinations = kernel.parameters().get(1).copied().unwrap_or(0) as usize;
    let Some(combinations) = checked_combination_count(values.len(), k) else {
        return Err(ResidentKernelError::InvalidShape);
    };
    if k == 0
        || k != declared_k
        || combinations != declared_combinations
        || output.len() != k.checked_mul(combinations).unwrap_or(usize::MAX)
    {
        return Err(ResidentKernelError::InvalidShape);
    }
    let output_len = output.len();
    let output_bytes = output_len
        .checked_mul(core::mem::size_of::<f64>())
        .ok_or(ResidentKernelError::InvalidShape)?;
    let index_bytes = k
        .checked_mul(core::mem::size_of::<usize>())
        .ok_or(ResidentKernelError::InvalidShape)?;
    let compute_work = output_len
        .checked_add(
            combinations
                .checked_mul(k)
                .ok_or(ResidentKernelError::InvalidShape)?,
        )
        .ok_or(ResidentKernelError::InvalidShape)?;
    let (k, output_len) = super::budget::PreparedKernel::new(
        (k, output_len),
        super::budget::resident_cost! {
            compute_work,
            output_elements: output_len,
            output_bytes,
            temporary_bytes: output_bytes,
            index_bytes,
            ..super::budget::KernelCostEstimate::default()
        },
    )
    .admit()?
    .into_plan();
    let mut selected = vec![0usize; k];
    let mut next = vec![0.0_f64; output_len];
    let mut column = 0usize;
    fn visit(
        values: &[f64],
        selected: &mut [usize],
        depth: usize,
        start: usize,
        output: &mut [f64],
        column: &mut usize,
    ) {
        if depth == selected.len() {
            let rows = selected.len();
            for (row, index) in selected.iter().copied().enumerate() {
                output[row + *column * rows] = values[index];
            }
            *column += 1;
            return;
        }
        let remaining = selected.len() - depth;
        for index in start..=values.len() - remaining {
            selected[depth] = index;
            visit(values, selected, depth + 1, index + 1, output, column);
        }
    }
    visit(values, &mut selected, 0, 0, &mut next, &mut column);
    if column != combinations {
        return Err(ResidentKernelError::IncompleteOutput);
    }
    let changed = output
        .iter()
        .zip(&next)
        .any(|(left, right)| left.to_bits() != right.to_bits());
    output.copy_from_slice(&next);
    Ok(changed)
}

fn n_choose_k_scalar(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 2 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let [n] = f64_input(inputs, 0)? else {
        return Err(ResidentKernelError::InvalidShape);
    };
    let [k] = f64_input(inputs, 1)? else {
        return Err(ResidentKernelError::InvalidShape);
    };
    if !n.is_finite()
        || !k.is_finite()
        || *n < 0.0
        || *k < 0.0
        || n.fract() != 0.0
        || k.fract() != 0.0
        || *n > u128::MAX as f64
        || *k > u128::MAX as f64
    {
        return Err(ResidentKernelError::InvalidInput);
    }
    if *k > *n {
        return write_f64_array(f64_output(output)?, [0.0]);
    }
    let iterations = k.min(*n - *k);
    if iterations > 1_000_000.0 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let mut result = 1.0;
    let mut index = 0.0;
    while index < iterations {
        result = result * (*n - index) / (index + 1.0);
        index += 1.0;
    }
    write_f64_array(f64_output(output)?, [result])
}

fn greatest_common_divisor(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

fn checked_integer_n_choose_k(n: u128, k: u128) -> Option<u128> {
    if k > n {
        return Some(0);
    }
    let k = k.min(n - k);
    let mut result = 1_u128;
    for divisor in 1..=k {
        let mut numerator = n - k + divisor;
        let mut denominator = divisor;
        let numerator_gcd = greatest_common_divisor(numerator, denominator);
        numerator /= numerator_gcd;
        denominator /= numerator_gcd;
        let result_gcd = greatest_common_divisor(result, denominator);
        result /= result_gcd;
        denominator /= result_gcd;
        if denominator != 1 {
            return None;
        }
        result = result.checked_mul(numerator)?;
    }
    Some(result)
}

fn canonical_n_choose_k_selection(data: &ValueData) -> Option<(u128, Option<u128>)> {
    Some(match data {
        ValueData::U8(value) => (u128::from(*value), Some(u128::from(u8::MAX))),
        ValueData::U16(value) => (u128::from(*value), Some(u128::from(u16::MAX))),
        ValueData::U32(value) => (u128::from(*value), Some(u128::from(u32::MAX))),
        ValueData::U64(value) => (u128::from(*value), Some(u128::from(u64::MAX))),
        ValueData::U128(value) => (*value, Some(u128::MAX)),
        ValueData::I8(value) => (u128::try_from(*value).ok()?, Some(i8::MAX as u128)),
        ValueData::I16(value) => (u128::try_from(*value).ok()?, Some(i16::MAX as u128)),
        ValueData::I32(value) => (u128::try_from(*value).ok()?, Some(i32::MAX as u128)),
        ValueData::I64(value) => (u128::try_from(*value).ok()?, Some(i64::MAX as u128)),
        ValueData::I128(value) => (u128::try_from(*value).ok()?, Some(i128::MAX as u128)),
        ValueData::F32(value) => {
            let value = value.to_f32();
            if !value.is_finite() || value < 0.0 || value.fract() != 0.0 || value > u128::MAX as f32
            {
                return None;
            }
            (value as u128, None)
        }
        ValueData::F64(value) => {
            let value = value.to_f64();
            if !value.is_finite() || value < 0.0 || value.fract() != 0.0 || value > u128::MAX as f64
            {
                return None;
            }
            (value as u128, None)
        }
        ValueData::Rational64(value) if value.denominator() == 1 => (
            u128::try_from(value.numerator()).ok()?,
            Some(i64::MAX as u128),
        ),
        ValueData::Complex64(value) => {
            let real = value.real().to_f64();
            let imaginary = value.imaginary().to_f64();
            if !real.is_finite()
                || !imaginary.is_finite()
                || imaginary != 0.0
                || real < 0.0
                || real.fract() != 0.0
                || real > u128::MAX as f64
            {
                return None;
            }
            (real as u128, None)
        }
        _ => return None,
    })
}

pub(super) fn canonical_n_choose_k_cardinality(value: &mech_core::Value) -> Option<usize> {
    let (selection, _) = canonical_n_choose_k_selection(value.data())?;
    usize::try_from(selection).ok()
}

fn numeric_from_u128(
    body: &SchemaBody,
    value: u128,
) -> Result<ValueDataDraft, ResidentKernelError> {
    use mech_core::IntegerWidth;
    match body {
        SchemaBody::UnsignedInteger(IntegerWidth::W8) => u8::try_from(value)
            .map(ValueDataDraft::U8)
            .map_err(|_| ResidentKernelError::Arithmetic),
        SchemaBody::UnsignedInteger(IntegerWidth::W16) => u16::try_from(value)
            .map(ValueDataDraft::U16)
            .map_err(|_| ResidentKernelError::Arithmetic),
        SchemaBody::UnsignedInteger(IntegerWidth::W32) => u32::try_from(value)
            .map(ValueDataDraft::U32)
            .map_err(|_| ResidentKernelError::Arithmetic),
        SchemaBody::UnsignedInteger(IntegerWidth::W64) => u64::try_from(value)
            .map(ValueDataDraft::U64)
            .map_err(|_| ResidentKernelError::Arithmetic),
        SchemaBody::UnsignedInteger(IntegerWidth::W128) => Ok(ValueDataDraft::U128(value)),
        SchemaBody::SignedInteger(IntegerWidth::W8) => i8::try_from(value)
            .map(ValueDataDraft::I8)
            .map_err(|_| ResidentKernelError::Arithmetic),
        SchemaBody::SignedInteger(IntegerWidth::W16) => i16::try_from(value)
            .map(ValueDataDraft::I16)
            .map_err(|_| ResidentKernelError::Arithmetic),
        SchemaBody::SignedInteger(IntegerWidth::W32) => i32::try_from(value)
            .map(ValueDataDraft::I32)
            .map_err(|_| ResidentKernelError::Arithmetic),
        SchemaBody::SignedInteger(IntegerWidth::W64) => i64::try_from(value)
            .map(ValueDataDraft::I64)
            .map_err(|_| ResidentKernelError::Arithmetic),
        SchemaBody::SignedInteger(IntegerWidth::W128) => i128::try_from(value)
            .map(ValueDataDraft::I128)
            .map_err(|_| ResidentKernelError::Arithmetic),
        SchemaBody::FloatingPoint(mech_core::FloatWidth::W32) => {
            Ok(ValueDataDraft::F32(F32Bits::from_f32(value as f32)))
        }
        SchemaBody::FloatingPoint(mech_core::FloatWidth::W64) => {
            Ok(ValueDataDraft::F64(F64Bits::from_f64(value as f64)))
        }
        SchemaBody::Rational64 => Ok(ValueDataDraft::Rational64 {
            numerator: i64::try_from(value).map_err(|_| ResidentKernelError::Arithmetic)?,
            denominator: 1,
        }),
        SchemaBody::Complex(mech_core::FloatWidth::W64) => Ok(ValueDataDraft::Complex64(
            mech_core::snapshot::Complex64Bits::new(
                F64Bits::from_f64(value as f64),
                F64Bits::from_f64(0.0),
            ),
        )),
        _ => Err(ResidentKernelError::InvalidInput),
    }
}

fn n_choose_k_scalar_snapshot(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let (
        Some(ResidentValueRef::Snapshot([Some(n_value)])),
        Some(ResidentValueRef::Snapshot([Some(k_value)])),
    ) = (inputs.get(0), inputs.get(1))
    else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let n_schema = n_value
        .validate_against(schemas)
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    let k_schema = k_value
        .validate_against(schemas)
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    if n_schema.body() != k_schema.body() {
        return Err(ResidentKernelError::InvalidInput);
    }
    let (n, result_maximum) =
        canonical_n_choose_k_selection(n_value.data()).ok_or(ResidentKernelError::InvalidInput)?;
    let (k, _) =
        canonical_n_choose_k_selection(k_value.data()).ok_or(ResidentKernelError::InvalidInput)?;
    let steps = if k > n { 0 } else { k.min(n - k) };
    if steps > 1_000_000 {
        return Err(ResidentKernelError::InvalidShape);
    }
    preflight_snapshot_arithmetic(
        kernel,
        schemas,
        &[n_value, k_value],
        &output,
        2,
        1,
        usize::try_from(steps).map_err(|_| ResidentKernelError::InvalidShape)?,
    )?;
    let result = if k > n {
        numeric_zero(n_schema.body())?
    } else if let Some(maximum) = result_maximum {
        let result = checked_integer_n_choose_k(n, k)
            .filter(|result| *result <= maximum)
            .ok_or(ResidentKernelError::Arithmetic)?;
        numeric_from_u128(n_schema.body(), result)?
    } else {
        let mut result = numeric_one(n_schema.body())?;
        let n_draft = n_value
            .canonical_data_draft()
            .map_err(|_| ResidentKernelError::InvalidInput)?;
        for index in 0..steps {
            let numerator =
                numeric_subtract(n_draft.clone(), numeric_from_u128(n_schema.body(), index)?)?;
            let denominator = numeric_from_u128(
                n_schema.body(),
                index
                    .checked_add(1)
                    .ok_or(ResidentKernelError::Arithmetic)?,
            )?;
            result = numeric_divide(numeric_multiply(result, numerator)?, denominator)?;
        }
        result
    };
    write_snapshot_data_with_work_budget(kernel, output, result, Some(0))
}

fn n_choose_k_snapshot(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let [declared_k, declared_combinations] = kernel.parameters() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let (
        Some(ResidentValueRef::Snapshot([Some(values)])),
        Some(ResidentValueRef::Snapshot([Some(selection)])),
    ) = (inputs.get(0), inputs.get(1))
    else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let values_schema = values
        .validate_against(schemas)
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    selection
        .validate_against(schemas)
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    let SchemaBody::Matrix { element, .. } = values_schema.body() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let available = snapshot_numeric_element_count(values)?;
    let (requested, _) = canonical_n_choose_k_selection(selection.data())
        .ok_or(ResidentKernelError::InvalidInput)?;
    let requested = usize::try_from(requested).map_err(|_| ResidentKernelError::InvalidShape)?;
    let combinations =
        checked_combination_count(available, requested).ok_or(ResidentKernelError::InvalidShape)?;
    if requested == 0
        || requested > available
        || u64::try_from(requested).ok() != Some(*declared_k)
        || u64::try_from(combinations).ok() != Some(*declared_combinations)
    {
        return Err(ResidentKernelError::InvalidShape);
    }
    let output_count = requested
        .checked_mul(combinations)
        .ok_or(ResidentKernelError::InvalidShape)?;
    preflight_snapshot_arithmetic(
        kernel,
        schemas,
        &[values, selection],
        &output,
        available
            .checked_add(1)
            .ok_or(ResidentKernelError::InvalidShape)?,
        output_count,
        0,
    )?;
    let values = snapshot_numeric_elements(values)?;
    let mut selected = (0..requested).collect::<Vec<_>>();
    let mut next = vec![values[0].clone(); output_count];
    for column in 0..combinations {
        for (row, index) in selected.iter().copied().enumerate() {
            next[row * combinations + column] = values[index].clone();
        }
        if column + 1 < combinations && !advance_combination_indices(&mut selected, available) {
            return Err(ResidentKernelError::IncompleteOutput);
        }
    }
    if !is_n_choose_k_snapshot_element(element) {
        return Err(ResidentKernelError::IncompleteOutput);
    }
    write_snapshot_data_with_work_budget(
        kernel,
        output,
        ValueDataDraft::Matrix(next.into_boxed_slice()),
        Some(0),
    )
}

fn advance_combination_indices(selected: &mut [usize], available: usize) -> bool {
    let Some(pivot) = (0..selected.len())
        .rev()
        .find(|index| selected[*index] < available - selected.len() + *index)
    else {
        return false;
    };
    selected[pivot] += 1;
    for index in pivot + 1..selected.len() {
        selected[index] = selected[index - 1] + 1;
    }
    true
}

fn gather_1d(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 2 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let source_values = f64_input(inputs, 0)?;
    let output = f64_output(output)?;
    let indices = ValidatedIndices::new(input(inputs, 1)?, source_values.len())?;
    if output.len() != indices.len() {
        return Err(ResidentKernelError::InvalidShape);
    }
    let mut changed = false;
    indices.try_for_each_position(|ordinal, index| {
        let target = &mut output[ordinal];
        let next = source_values[index];
        changed |= target.to_bits() != next.to_bits();
        *target = next;
        Ok::<(), ResidentKernelError>(())
    })?;
    Ok(changed)
}

fn scalar_access_1d(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 2 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let source = f64_input(inputs, 0)?;
    let index = checked_one_based(index_at(inputs, 1, 0)?, source.len())?;
    let output = f64_output(output)?;
    let [target] = output else {
        return Err(ResidentKernelError::InvalidShape);
    };
    let next = source[index];
    let changed = target.to_bits() != next.to_bits();
    *target = next;
    Ok(changed)
}

fn portable_index_at(
    input: ResidentValueRef<'_>,
    ordinal: usize,
) -> Result<u64, ResidentKernelError> {
    match input {
        ResidentValueRef::F64(values) => values
            .get(ordinal)
            .copied()
            .map(F64Bits::from_f64)
            .map(ValueData::F64)
            .and_then(|value| mech_core::canonical_positional_ordinal(&value).ok()),
        ResidentValueRef::Index(values) => values
            .get(ordinal)
            .copied()
            .map(ValueData::Index)
            .and_then(|value| mech_core::canonical_positional_ordinal(&value).ok()),
        ResidentValueRef::Snapshot([value]) => {
            let value = value.as_ref().ok_or(ResidentKernelError::InvalidInput)?;
            match value.data() {
                ValueData::Matrix(matrix) => portable_sequence_index_at(matrix.elements(), ordinal),
                value if ordinal == 0 => portable_data_index(value),
                _ => None,
            }
        }
        ResidentValueRef::Bool(_) | ResidentValueRef::String(_) | ResidentValueRef::Snapshot(_) => {
            return Err(ResidentKernelError::InvalidInput);
        }
    }
    .ok_or(ResidentKernelError::InvalidInput)
}

fn portable_data_index(value: &ValueData) -> Option<u64> {
    mech_core::canonical_positional_ordinal(value).ok()
}

fn portable_sequence_index_at(sequence: SequenceView<'_>, ordinal: usize) -> Option<u64> {
    mech_core::canonical_positional_ordinal_at(sequence, ordinal).ok()
}

fn resident_portable_index_len(input: ResidentValueRef<'_>) -> Result<usize, ResidentKernelError> {
    match input {
        ResidentValueRef::Snapshot([Some(value)]) => Ok(match value.data() {
            ValueData::Matrix(matrix) => matrix.elements().len(),
            _ => 1,
        }),
        ResidentValueRef::Snapshot(_) => Err(ResidentKernelError::InvalidInput),
        value => Ok(value.len()),
    }
}

fn scalar_index(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 1 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let input = inputs.get(0).ok_or(ResidentKernelError::InvalidInput)?;
    let ResidentValueMut::Index(output) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let input_len = resident_portable_index_len(input)?;
    if input_len != output.len() {
        return Err(ResidentKernelError::InvalidShape);
    }
    for ordinal in 0..input_len {
        portable_index_at(input, ordinal)?;
    }
    let bytes = output
        .len()
        .checked_mul(core::mem::size_of::<u64>())
        .ok_or(ResidentKernelError::InvalidShape)?;
    let output_len = super::budget::PreparedKernel::new(
        input_len,
        super::budget::resident_cost! {
            compute_work: output
                .len()
                .checked_mul(2)
                .ok_or(ResidentKernelError::InvalidShape)?,
            output_elements: output.len(),
            output_bytes: bytes,
            temporary_bytes: bytes,
            ..super::budget::KernelCostEstimate::default()
        },
    )
    .admit()?
    .into_plan();
    let next = (0..output_len)
        .map(|ordinal| portable_index_at(input, ordinal))
        .collect::<Result<Vec<_>, _>>()?;
    let changed = output != next;
    output.copy_from_slice(&next);
    Ok(changed)
}

fn scalar_access_2d(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 3 || kernel.parameters().len() != 2 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let source = f64_input(inputs, 0)?;
    let rows =
        usize::try_from(kernel.parameters()[0]).map_err(|_| ResidentKernelError::InvalidShape)?;
    let columns =
        usize::try_from(kernel.parameters()[1]).map_err(|_| ResidentKernelError::InvalidShape)?;
    if rows.checked_mul(columns) != Some(source.len()) {
        return Err(ResidentKernelError::InvalidShape);
    }
    let row = checked_one_based(index_at(inputs, 1, 0)?, rows)?;
    let column = checked_one_based(index_at(inputs, 2, 0)?, columns)?;
    let index = row + column * rows;
    let output = f64_output(output)?;
    let [target] = output else {
        return Err(ResidentKernelError::InvalidShape);
    };
    let next = source[index];
    let changed = target.to_bits() != next.to_bits();
    *target = next;
    Ok(changed)
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct SelectorMaterializationCost {
    elements: usize,
    retained_bytes: u64,
    cloned_bytes: u64,
    retained_nodes: u64,
}

fn add_selector_cost(
    total: SelectorMaterializationCost,
    next: SelectorMaterializationCost,
) -> Result<SelectorMaterializationCost, ResidentKernelError> {
    Ok(SelectorMaterializationCost {
        elements: total
            .elements
            .checked_add(next.elements)
            .ok_or(ResidentKernelError::InvalidShape)?,
        retained_bytes: total
            .retained_bytes
            .checked_add(next.retained_bytes)
            .ok_or(ResidentKernelError::InvalidShape)?,
        cloned_bytes: total
            .cloned_bytes
            .checked_add(next.cloned_bytes)
            .ok_or(ResidentKernelError::InvalidShape)?,
        retained_nodes: total
            .retained_nodes
            .checked_add(next.retained_nodes)
            .ok_or(ResidentKernelError::InvalidShape)?,
    })
}

/// Validates one live selector and estimates both its draft and finalized
/// representation without allocating either one.
fn snapshot_selector_materialization_cost(
    schemas: &mech_core::SchemaTable,
    layout: &SnapshotAccessSelectorLayout,
    value: ResidentValueRef<'_>,
    meter: &mut super::budget::ResidentBudgetMeter,
) -> Result<SelectorMaterializationCost, ResidentKernelError> {
    if let ResidentValueRef::Snapshot([Some(value)]) = value {
        if value.schema() != layout.schema || value.shape() != &layout.shape {
            return Err(ResidentKernelError::InvalidInput);
        }
        value
            .validate_against(schemas)
            .map_err(|_| ResidentKernelError::InvalidInput)?;
        let footprint = super::budget::measure_canonical_value_footprint(meter, value, schemas)?;
        let elements = match value.data() {
            ValueData::Matrix(matrix) => matrix.elements().len(),
            _ => 1,
        };
        return Ok(SelectorMaterializationCost {
            elements,
            retained_bytes: footprint.retained_bytes,
            cloned_bytes: footprint.retained_bytes,
            retained_nodes: footprint.node_count,
        });
    }
    if matches!(value, ResidentValueRef::Snapshot(_)) {
        return Err(ResidentKernelError::InvalidInput);
    }
    if value.kind() != layout.resident_shape_kind(schemas)? {
        return Err(ResidentKernelError::InvalidInput);
    }
    let count = layout
        .resident_shape
        .len()
        .ok_or(ResidentKernelError::InvalidShape)?;
    if value.len() != count {
        return Err(ResidentKernelError::InvalidShape);
    }
    if let ResidentValueRef::Bool(values) = value
        && values.iter().any(|value| *value > 1)
    {
        return Err(ResidentKernelError::InvalidInput);
    }
    let payload_bytes = match value {
        ResidentValueRef::Bool(values) => values.len(),
        ResidentValueRef::Index(values) => values
            .len()
            .checked_mul(core::mem::size_of::<u64>())
            .ok_or(ResidentKernelError::InvalidShape)?,
        ResidentValueRef::F64(values) => values
            .len()
            .checked_mul(core::mem::size_of::<f64>())
            .ok_or(ResidentKernelError::InvalidShape)?,
        ResidentValueRef::String(values) => values.iter().try_fold(0usize, |bytes, value| {
            bytes
                .checked_add(value.len())
                .ok_or(ResidentKernelError::InvalidShape)
        })?,
        ResidentValueRef::Snapshot(_) => return Err(ResidentKernelError::InvalidInput),
    };
    let draft_containers = count
        .checked_mul(core::mem::size_of::<ValueDataDraft>())
        .ok_or(ResidentKernelError::InvalidShape)?;
    let final_containers = match value {
        ResidentValueRef::String(_) => count
            .checked_mul(core::mem::size_of::<Box<str>>())
            .ok_or(ResidentKernelError::InvalidShape)?,
        _ => payload_bytes,
    };
    let shape_bytes = layout
        .shape
        .parameter_values()
        .len()
        .checked_mul(core::mem::size_of::<u64>())
        .ok_or(ResidentKernelError::InvalidShape)?;
    let retained_bytes = core::mem::size_of::<mech_core::Value>()
        .checked_add(shape_bytes)
        .and_then(|bytes| bytes.checked_add(draft_containers))
        .and_then(|bytes| bytes.checked_add(final_containers))
        .and_then(|bytes| bytes.checked_add(payload_bytes))
        .ok_or(ResidentKernelError::InvalidShape)?;
    Ok(SelectorMaterializationCost {
        elements: count,
        retained_bytes: super::budget::checked_u64(retained_bytes)?,
        cloned_bytes: super::budget::checked_u64(payload_bytes)?,
        retained_nodes: super::budget::checked_u64(count.saturating_add(2))?,
    })
}

impl SnapshotAccessSelectorLayout {
    fn resident_shape_kind(
        &self,
        schemas: &mech_core::SchemaTable,
    ) -> Result<ResidentValueKind, ResidentKernelError> {
        let body = schemas
            .get(self.schema)
            .ok_or(ResidentKernelError::InvalidInput)?
            .body();
        let body = match body {
            SchemaBody::Matrix { element, .. } => element.as_ref(),
            body => body,
        };
        Ok(match body {
            SchemaBody::Bool => ResidentValueKind::Bool,
            SchemaBody::Index => ResidentValueKind::Index,
            SchemaBody::FloatingPoint(mech_core::FloatWidth::W64) => ResidentValueKind::F64,
            SchemaBody::String => ResidentValueKind::String,
            _ => ResidentValueKind::Snapshot,
        })
    }
}

fn sequence_for_each_access_index(
    values: SequenceView<'_>,
    upper: usize,
    mut visit: impl FnMut(usize) -> Result<(), ResidentKernelError>,
) -> Result<(), ResidentKernelError> {
    match values {
        SequenceView::Bool(values) => {
            if values.len() != upper {
                return Err(ResidentKernelError::InvalidShape);
            }
            for (index, selected) in values.iter().enumerate() {
                if *selected {
                    visit(index)?;
                }
            }
            Ok(())
        }
        values => {
            mech_core::visit_canonical_positional_sequence(values, upper, visit).map_err(|error| {
                match error {
                    mech_core::CanonicalSelectorVisitError::Selector(_) => {
                        ResidentKernelError::InvalidInput
                    }
                    mech_core::CanonicalSelectorVisitError::Visitor(error) => error,
                }
            })
        }
    }
}

pub(super) fn selector_for_each_access_index(
    value: ResidentValueRef<'_>,
    upper: usize,
    mut visit: impl FnMut(usize) -> Result<(), ResidentKernelError>,
) -> Result<(), ResidentKernelError> {
    match value {
        ResidentValueRef::Snapshot([Some(value)]) => match value.data() {
            ValueData::Matrix(matrix) => {
                sequence_for_each_access_index(matrix.elements(), upper, visit)
            }
            value => visit(access_index(value, upper)?),
        },
        ResidentValueRef::Bool(values) => {
            if values.len() != upper {
                return Err(ResidentKernelError::InvalidShape);
            }
            for (index, selected) in values.iter().enumerate() {
                match *selected {
                    0 => {}
                    1 => visit(index)?,
                    _ => return Err(ResidentKernelError::InvalidInput),
                }
            }
            Ok(())
        }
        ResidentValueRef::Index(values) => {
            for value in values {
                visit(access_index(&ValueData::Index(*value), upper)?)?;
            }
            Ok(())
        }
        ResidentValueRef::F64(values) => {
            for value in values {
                visit(access_index(
                    &ValueData::F64(F64Bits::from_f64(*value)),
                    upper,
                )?)?;
            }
            Ok(())
        }
        _ => Err(ResidentKernelError::InvalidInput),
    }
}

fn selector_access_count(
    value: ResidentValueRef<'_>,
    upper: usize,
) -> Result<usize, ResidentKernelError> {
    let mut count = 0usize;
    selector_for_each_access_index(value, upper, |_| {
        count = count
            .checked_add(1)
            .ok_or(ResidentKernelError::InvalidShape)?;
        Ok(())
    })?;
    Ok(count)
}

fn selector_single_access_index(
    value: ResidentValueRef<'_>,
    upper: usize,
) -> Result<usize, ResidentKernelError> {
    let mut selected = None;
    selector_for_each_access_index(value, upper, |index| {
        if selected.replace(index).is_some() {
            return Err(ResidentKernelError::InvalidShape);
        }
        Ok(())
    })?;
    selected.ok_or(ResidentKernelError::InvalidShape)
}

fn selector_id(value: ResidentValueRef<'_>) -> Result<u64, ResidentKernelError> {
    let ResidentValueRef::Snapshot([Some(value)]) = value else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let ValueData::Id(id) = value.data() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    Ok(*id)
}

fn map_access_entry_for_selector_with_meter(
    map: &mech_core::snapshot::MapValue,
    key_schema: &SchemaBody,
    selector: ResidentValueRef<'_>,
    meter: &mut super::budget::ResidentBudgetMeter,
) -> Result<usize, ResidentKernelError> {
    let compare = |candidate: &ValueData| -> Result<core::cmp::Ordering, ResidentKernelError> {
        let ordering = match selector {
            ResidentValueRef::Snapshot([Some(value)]) => {
                compare_key_data(key_schema, value.data(), candidate)
            }
            ResidentValueRef::Bool([value]) if *value <= 1 => {
                compare_key_data(key_schema, &ValueData::Bool(*value == 1), candidate)
            }
            ResidentValueRef::Index([value]) => {
                compare_key_data(key_schema, &ValueData::Index(*value), candidate)
            }
            ResidentValueRef::F64([value]) => compare_key_data(
                key_schema,
                &ValueData::F64(F64Bits::from_f64(*value)),
                candidate,
            ),
            ResidentValueRef::String([value]) => {
                let ValueData::String(candidate) = candidate else {
                    return Err(ResidentKernelError::InvalidInput);
                };
                return Ok(value.as_str().cmp(candidate.as_ref()));
            }
            _ => return Err(ResidentKernelError::InvalidInput),
        };
        ordering.map_err(|_| ResidentKernelError::InvalidInput)
    };
    // Plan the complete worst-case key walk before performing the first
    // comparison. A missing key and a key in the final entry must receive the
    // same admission decision, and an oversized map must fail while planning
    // rather than after partially executing the lookup.
    let candidate_work = match selector {
        ResidentValueRef::Snapshot([Some(value)]) => {
            let footprint =
                super::budget::measure_canonical_data_footprint(meter, key_schema, value.data())?;
            footprint.encoded_bytes.max(footprint.node_count).max(1)
        }
        ResidentValueRef::Bool([value]) if *value <= 1 => {
            meter.charge_comparison_work(1)?;
            meter.charge_retained_nodes(1)?;
            1
        }
        ResidentValueRef::Index([_]) | ResidentValueRef::F64([_]) => {
            meter.charge_comparison_work(8)?;
            meter.charge_retained_nodes(1)?;
            8
        }
        ResidentValueRef::String([value]) => {
            let work = super::budget::checked_u64(value.len())?
                .checked_add(8)
                .ok_or(ResidentKernelError::InvalidShape)?;
            meter.charge_comparison_work(work)?;
            meter.charge_retained_nodes(1)?;
            work
        }
        _ => return Err(ResidentKernelError::InvalidInput),
    };
    for entry in map.entries() {
        super::budget::charge_canonical_key_footprint(meter, key_schema, entry.key().data())?;
        meter.charge_comparison_work(candidate_work)?;
    }
    let cost = meter.estimate();
    super::budget::PreparedKernel::new((), cost)
        .admit()?
        .into_plan();
    let mut lower = 0usize;
    let mut upper = map.entries().len();
    while lower < upper {
        let middle = lower + (upper - lower) / 2;
        match compare(map.entries()[middle].key().data())? {
            core::cmp::Ordering::Less => upper = middle,
            core::cmp::Ordering::Greater => lower = middle + 1,
            core::cmp::Ordering::Equal => return Ok(middle),
        }
    }
    Err(ResidentKernelError::InvalidInput)
}

#[cfg(test)]
fn map_access_entry_for_selector(
    map: &mech_core::snapshot::MapValue,
    key_schema: &SchemaBody,
    selector: ResidentValueRef<'_>,
) -> Result<(usize, u64), ResidentKernelError> {
    let mut meter = super::budget::ResidentBudgetMeter::default();
    let ordinal = map_access_entry_for_selector_with_meter(map, key_schema, selector, &mut meter)?;
    Ok((ordinal, meter.estimate().comparison_work()))
}

fn selector_value(
    schemas: &mech_core::SchemaTable,
    layout: &SnapshotAccessSelectorLayout,
    value: ResidentValueRef<'_>,
) -> Result<mech_core::Value, ResidentKernelError> {
    if let ResidentValueRef::Snapshot([Some(value)]) = value {
        if value.schema() != layout.schema {
            return Err(ResidentKernelError::InvalidInput);
        }
        value
            .validate_against(schemas)
            .map_err(|_| ResidentKernelError::InvalidInput)?;
        return Ok(value.clone());
    }
    let schema = schemas
        .get(layout.schema)
        .ok_or(ResidentKernelError::InvalidInput)?;
    let matrix = matches!(schema.body(), SchemaBody::Matrix { .. });
    let rows = layout.resident_shape.rows as usize;
    let columns = layout.resident_shape.columns as usize;
    let count = rows
        .checked_mul(columns)
        .ok_or(ResidentKernelError::InvalidShape)?;
    if value.len() != count {
        return Err(ResidentKernelError::InvalidShape);
    }
    let canonical_indices =
        (0..rows).flat_map(|row| (0..columns).map(move |column| row + column * rows));
    let data = match value {
        ResidentValueRef::Bool(values) => {
            if values.iter().any(|value| *value > 1) {
                return Err(ResidentKernelError::InvalidInput);
            }
            let elements = canonical_indices
                .map(|index| ValueDataDraft::Bool(values[index] != 0))
                .collect::<Vec<_>>();
            if matrix {
                ValueDataDraft::Matrix(elements.into_boxed_slice())
            } else {
                elements
                    .into_iter()
                    .next()
                    .ok_or(ResidentKernelError::InvalidShape)?
            }
        }
        ResidentValueRef::Index(values) => {
            let elements = canonical_indices
                .map(|index| ValueDataDraft::Index(values[index]))
                .collect::<Vec<_>>();
            if matrix {
                ValueDataDraft::Matrix(elements.into_boxed_slice())
            } else {
                elements
                    .into_iter()
                    .next()
                    .ok_or(ResidentKernelError::InvalidShape)?
            }
        }
        ResidentValueRef::F64(values) => {
            let elements = canonical_indices
                .map(|index| ValueDataDraft::F64(F64Bits::from_f64(values[index])))
                .collect::<Vec<_>>();
            if matrix {
                ValueDataDraft::Matrix(elements.into_boxed_slice())
            } else {
                elements
                    .into_iter()
                    .next()
                    .ok_or(ResidentKernelError::InvalidShape)?
            }
        }
        ResidentValueRef::String(values) => {
            let elements = canonical_indices
                .map(|index| ValueDataDraft::String(values[index].clone()))
                .collect::<Vec<_>>();
            if matrix {
                ValueDataDraft::Matrix(elements.into_boxed_slice())
            } else {
                elements
                    .into_iter()
                    .next()
                    .ok_or(ResidentKernelError::InvalidShape)?
            }
        }
        ResidentValueRef::Snapshot(_) => return Err(ResidentKernelError::InvalidInput),
    };
    ValueDraft {
        schema: layout.schema,
        shape_values: layout.shape.parameter_values().to_vec().into_boxed_slice(),
        data,
    }
    .finalize(&SnapshotValidationContext::new(schemas))
    .map_err(|_| ResidentKernelError::InvalidInput)
}

fn access_index(data: &ValueData, upper: usize) -> Result<usize, ResidentKernelError> {
    let ordinal = mech_core::canonical_positional_ordinal(data)
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    checked_one_based(ordinal, upper)
}

fn access_indices(
    selector: &mech_core::Value,
    upper: usize,
) -> Result<Vec<usize>, ResidentKernelError> {
    match selector.data() {
        ValueData::Matrix(matrix) => match matrix.elements() {
            SequenceView::Bool(values) => {
                if values.len() != upper {
                    return Err(ResidentKernelError::InvalidShape);
                }
                Ok(values
                    .iter()
                    .enumerate()
                    .filter_map(|(index, selected)| selected.then_some(index))
                    .collect())
            }
            values => values
                .to_values()
                .iter()
                .map(|value| access_index(value, upper))
                .collect(),
        },
        value => Ok(vec![access_index(value, upper)?]),
    }
}

fn sequence_data_draft_at(
    schema: &SchemaBody,
    values: SequenceView<'_>,
    index: usize,
) -> Result<ValueDataDraft, ResidentKernelError> {
    macro_rules! draft {
        ($values:expr, $variant:ident) => {
            $values
                .get(index)
                .cloned()
                .map(ValueDataDraft::$variant)
                .ok_or(ResidentKernelError::InvalidShape)
        };
    }
    match values {
        SequenceView::U8(values) => draft!(values, U8),
        SequenceView::U16(values) => draft!(values, U16),
        SequenceView::U32(values) => draft!(values, U32),
        SequenceView::U64(values) => draft!(values, U64),
        SequenceView::U128(values) => draft!(values, U128),
        SequenceView::I8(values) => draft!(values, I8),
        SequenceView::I16(values) => draft!(values, I16),
        SequenceView::I32(values) => draft!(values, I32),
        SequenceView::I64(values) => draft!(values, I64),
        SequenceView::I128(values) => draft!(values, I128),
        SequenceView::F32(values) => draft!(values, F32),
        SequenceView::F64(values) => draft!(values, F64),
        SequenceView::Complex32(values) => draft!(values, Complex32),
        SequenceView::Complex64(values) => draft!(values, Complex64),
        SequenceView::Rational64(values) => values
            .get(index)
            .map(|value| ValueDataDraft::Rational64 {
                numerator: value.numerator(),
                denominator: value.denominator(),
            })
            .ok_or(ResidentKernelError::InvalidShape),
        SequenceView::Bool(values) => values
            .get(index)
            .copied()
            .map(ValueDataDraft::Bool)
            .ok_or(ResidentKernelError::InvalidShape),
        SequenceView::String(values) => values
            .get(index)
            .map(|value| ValueDataDraft::String(value.as_ref().to_owned()))
            .ok_or(ResidentKernelError::InvalidShape),
        SequenceView::Id(values) => draft!(values, Id),
        SequenceView::Index(values) => draft!(values, Index),
        SequenceView::Unit(count) if index < usize::try_from(count).unwrap_or(usize::MAX) => {
            Ok(ValueDataDraft::Atom)
        }
        SequenceView::Values(values) => canonical_snapshot_data_draft(
            schema,
            values.get(index).ok_or(ResidentKernelError::InvalidShape)?,
        )
        .map_err(|_| ResidentKernelError::InvalidInput),
        _ => Err(ResidentKernelError::InvalidShape),
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct SnapshotAccessOutputCost {
    footprint: ValueFootprint,
    count: usize,
    index_elements: usize,
    selected_ordinal: Option<usize>,
    finalization_work: u64,
}

fn named_schema_ordinal_with_meter<'a>(
    names: impl IntoIterator<Item = &'a str>,
    selected: u64,
    meter: &mut super::budget::ResidentBudgetMeter,
) -> Result<usize, ResidentKernelError> {
    for (ordinal, name) in names.into_iter().enumerate() {
        meter.charge_comparison_work(super::budget::checked_u64(name.len().max(1))?)?;
        if mech_core::hash_str(name) == selected {
            return Ok(ordinal);
        }
    }
    Err(ResidentKernelError::InvalidInput)
}

fn snapshot_data_finalization_work(
    meter: &mut super::budget::ResidentBudgetMeter,
    schema: &SchemaBody,
    data: &ValueData,
) -> Result<u64, ResidentKernelError> {
    super::budget::preflight_canonical_data_finalization(meter, schema, data)
}

fn checked_footprint_add(
    total: &mut ValueFootprint,
    next: ValueFootprint,
) -> Result<(), ResidentKernelError> {
    *total = total
        .checked_add(next)
        .map_err(|_| ResidentKernelError::InvalidShape)?;
    Ok(())
}

fn selected_sequence_footprint(
    total: &mut ValueFootprint,
    meter: &mut super::budget::ResidentBudgetMeter,
    schema: &SchemaBody,
    values: SequenceView<'_>,
    index: usize,
) -> Result<(), ResidentKernelError> {
    let footprint = match values {
        SequenceView::Values(values) => super::budget::measure_canonical_data_footprint(
            meter,
            schema,
            values.get(index).ok_or(ResidentKernelError::InvalidShape)?,
        )?,
        values => {
            let footprint = canonical_sequence_element_retained_footprint(schema, values, index)
                .map_err(|_| ResidentKernelError::InvalidInput)?;
            meter
                .charge_comparison_work(footprint.encoded_bytes.max(footprint.node_count).max(1))?;
            meter.charge_retained_nodes(footprint.node_count)?;
            footprint
        }
    };
    checked_footprint_add(total, footprint)
}

fn selected_sequence_footprint_with_finalization(
    total: &mut ValueFootprint,
    finalization_work: &mut u64,
    meter: &mut super::budget::ResidentBudgetMeter,
    schema: &SchemaBody,
    values: SequenceView<'_>,
    index: usize,
) -> Result<(), ResidentKernelError> {
    selected_sequence_footprint(total, meter, schema, values, index)?;
    if let SequenceView::Values(values) = values {
        let value = values.get(index).ok_or(ResidentKernelError::InvalidShape)?;
        *finalization_work = finalization_work
            .checked_add(snapshot_data_finalization_work(meter, schema, value)?)
            .ok_or(ResidentKernelError::InvalidShape)?;
    }
    Ok(())
}

fn snapshot_scalar_access_cost(
    meter: &mut super::budget::ResidentBudgetMeter,
    schema: &SchemaBody,
    data: &ValueData,
) -> Result<SnapshotAccessOutputCost, ResidentKernelError> {
    Ok(SnapshotAccessOutputCost {
        footprint: super::budget::measure_canonical_data_footprint(meter, schema, data)?,
        count: 1,
        index_elements: 1,
        selected_ordinal: None,
        finalization_work: snapshot_data_finalization_work(meter, schema, data)?,
    })
}

fn snapshot_access_output_cost(
    source: &mech_core::Value,
    source_schema: &SchemaBody,
    plan: &SnapshotAccessPlan,
    inputs: &dyn ResidentKernelInputs,
    meter: &mut super::budget::ResidentBudgetMeter,
) -> Result<SnapshotAccessOutputCost, ResidentKernelError> {
    let selector = |index: usize| {
        inputs
            .get(index + 1)
            .ok_or(ResidentKernelError::InvalidInput)
    };
    match (source_schema, source.data()) {
        (SchemaBody::Tuple(elements), ValueData::Tuple(values)) => {
            let live_ordinal = selector_single_access_index(selector(0)?, values.len())?;
            let index = plan.aggregate_ordinal.unwrap_or(live_ordinal);
            if index != live_ordinal {
                return Err(ResidentKernelError::InvalidInput);
            }
            let mut cost = snapshot_scalar_access_cost(
                meter,
                elements
                    .get(index)
                    .ok_or(ResidentKernelError::InvalidShape)?,
                values.get(index).ok_or(ResidentKernelError::InvalidShape)?,
            )?;
            cost.selected_ordinal = Some(index);
            Ok(cost)
        }
        (SchemaBody::Record(fields), ValueData::Record(record)) => {
            let field = selector_id(selector(0)?)?;
            let live_ordinal = named_schema_ordinal_with_meter(
                fields.iter().map(|candidate| candidate.name.as_str()),
                field,
                meter,
            )?;
            let index = plan.aggregate_ordinal.unwrap_or(live_ordinal);
            if index != live_ordinal {
                return Err(ResidentKernelError::InvalidInput);
            }
            let mut cost = snapshot_scalar_access_cost(
                meter,
                &fields[index].schema,
                record
                    .fields()
                    .get(index)
                    .ok_or(ResidentKernelError::InvalidShape)?,
            )?;
            cost.selected_ordinal = Some(index);
            Ok(cost)
        }
        (SchemaBody::Map { key, value, .. }, ValueData::Map(map)) => {
            let ordinal = map_access_entry_for_selector_with_meter(map, key, selector(0)?, meter)?;
            let entry = map
                .entries()
                .get(ordinal)
                .ok_or(ResidentKernelError::InvalidShape)?;
            let mut cost = snapshot_scalar_access_cost(meter, value, entry.value())?;
            cost.selected_ordinal = Some(ordinal);
            Ok(cost)
        }
        (SchemaBody::Table { columns, .. }, ValueData::Table(table)) => {
            let column = selector_id(selector(0)?)?;
            let live_ordinal = named_schema_ordinal_with_meter(
                columns.iter().map(|candidate| candidate.name.as_str()),
                column,
                meter,
            )?;
            let index = plan.aggregate_ordinal.unwrap_or(live_ordinal);
            if index != live_ordinal {
                return Err(ResidentKernelError::InvalidInput);
            }
            let values = table
                .column(index)
                .ok_or(ResidentKernelError::InvalidShape)?;
            let mut footprint = ValueFootprint::zero();
            let mut finalization_work = 0_u64;
            for row in 0..values.len() {
                selected_sequence_footprint_with_finalization(
                    &mut footprint,
                    &mut finalization_work,
                    meter,
                    &columns[index].schema,
                    values,
                    row,
                )?;
            }
            Ok(SnapshotAccessOutputCost {
                footprint,
                count: values.len(),
                index_elements: 0,
                selected_ordinal: Some(index),
                finalization_work,
            })
        }
        (SchemaBody::Matrix { element, .. }, ValueData::Matrix(matrix)) => {
            let (rows, columns) = plan
                .source_dimensions
                .ok_or(ResidentKernelError::InvalidInput)?;
            let elements = matrix.elements();
            let source_len = rows
                .checked_mul(columns)
                .ok_or(ResidentKernelError::InvalidShape)?;
            if elements.len() != source_len {
                return Err(ResidentKernelError::InvalidShape);
            }
            let expected_count = match plan.output_dimensions {
                Some((rows, columns)) => rows
                    .checked_mul(columns)
                    .ok_or(ResidentKernelError::InvalidShape)?,
                None => 1,
            };
            if u64::try_from(expected_count).map_or(true, |count| {
                count > super::budget::MAX_RESIDENT_OUTPUT_ELEMENTS
            }) {
                return Err(ResidentKernelError::InvalidShape);
            }
            let mut footprint = ValueFootprint::zero();
            let mut finalization_work = 0_u64;
            let (count, index_elements) = match plan.matrix_mode {
                Some(ResolvedSelectionMode::LinearScalar | ResolvedSelectionMode::LinearGather) => {
                    let mut count = 0usize;
                    selector_for_each_access_index(selector(0)?, source_len, |linear| {
                        let row = linear % rows;
                        let column = linear / rows;
                        selected_sequence_footprint_with_finalization(
                            &mut footprint,
                            &mut finalization_work,
                            meter,
                            element,
                            elements,
                            row * columns + column,
                        )?;
                        count = count
                            .checked_add(1)
                            .ok_or(ResidentKernelError::InvalidShape)?;
                        Ok(())
                    })?;
                    (count, count)
                }
                Some(ResolvedSelectionMode::Rows) => {
                    let selected_rows = selector_access_count(selector(0)?, rows)?;
                    let count = selected_rows
                        .checked_mul(columns)
                        .ok_or(ResidentKernelError::InvalidShape)?;
                    if count != expected_count {
                        return Err(ResidentKernelError::InvalidShape);
                    }
                    selector_for_each_access_index(selector(0)?, rows, |row| {
                        for column in 0..columns {
                            selected_sequence_footprint_with_finalization(
                                &mut footprint,
                                &mut finalization_work,
                                meter,
                                element,
                                elements,
                                row * columns + column,
                            )?;
                        }
                        Ok(())
                    })?;
                    (
                        count,
                        selected_rows
                            .checked_add(columns)
                            .ok_or(ResidentKernelError::InvalidShape)?,
                    )
                }
                Some(ResolvedSelectionMode::Columns) => {
                    let selected_columns = selector_access_count(selector(0)?, columns)?;
                    let count = rows
                        .checked_mul(selected_columns)
                        .ok_or(ResidentKernelError::InvalidShape)?;
                    if count != expected_count {
                        return Err(ResidentKernelError::InvalidShape);
                    }
                    for row in 0..rows {
                        selector_for_each_access_index(selector(0)?, columns, |column| {
                            selected_sequence_footprint_with_finalization(
                                &mut footprint,
                                &mut finalization_work,
                                meter,
                                element,
                                elements,
                                row * columns + column,
                            )
                        })?;
                    }
                    (
                        count,
                        rows.checked_add(selected_columns)
                            .ok_or(ResidentKernelError::InvalidShape)?,
                    )
                }
                Some(ResolvedSelectionMode::Rectangle) => {
                    let selected_rows = selector_access_count(selector(0)?, rows)?;
                    let selected_columns = selector_access_count(selector(1)?, columns)?;
                    let count = selected_rows
                        .checked_mul(selected_columns)
                        .ok_or(ResidentKernelError::InvalidShape)?;
                    if count != expected_count {
                        return Err(ResidentKernelError::InvalidShape);
                    }
                    selector_for_each_access_index(selector(0)?, rows, |row| {
                        selector_for_each_access_index(selector(1)?, columns, |column| {
                            selected_sequence_footprint_with_finalization(
                                &mut footprint,
                                &mut finalization_work,
                                meter,
                                element,
                                elements,
                                row * columns + column,
                            )
                        })
                    })?;
                    (
                        count,
                        selected_rows
                            .checked_add(selected_columns)
                            .ok_or(ResidentKernelError::InvalidShape)?,
                    )
                }
                _ => return Err(ResidentKernelError::InvalidInput),
            };
            if count != expected_count {
                return Err(ResidentKernelError::InvalidShape);
            }
            Ok(SnapshotAccessOutputCost {
                footprint,
                count,
                index_elements,
                selected_ordinal: None,
                finalization_work,
            })
        }
        _ => Err(ResidentKernelError::InvalidInput),
    }
}

fn snapshot_access_data(
    source: &mech_core::Value,
    source_schema: &SchemaBody,
    selectors: &[mech_core::Value],
    plan: &SnapshotAccessPlan,
    output_cost: &SnapshotAccessOutputCost,
    _schemas: &mech_core::SchemaTable,
) -> Result<ValueDataDraft, ResidentKernelError> {
    match source_schema {
        SchemaBody::Tuple(elements) => {
            let index = output_cost
                .selected_ordinal
                .ok_or(ResidentKernelError::InvalidInput)?;
            let ValueData::Tuple(values) = source.data() else {
                return Err(ResidentKernelError::InvalidInput);
            };
            canonical_snapshot_data_draft(
                elements
                    .get(index)
                    .ok_or(ResidentKernelError::InvalidShape)?,
                values.get(index).ok_or(ResidentKernelError::InvalidShape)?,
            )
            .map_err(|_| ResidentKernelError::InvalidInput)
        }
        SchemaBody::Record(fields) => {
            let index = output_cost
                .selected_ordinal
                .ok_or(ResidentKernelError::InvalidInput)?;
            let ValueData::Record(values) = source.data() else {
                return Err(ResidentKernelError::InvalidInput);
            };
            canonical_snapshot_data_draft(
                &fields[index].schema,
                values
                    .fields()
                    .get(index)
                    .ok_or(ResidentKernelError::InvalidShape)?,
            )
            .map_err(|_| ResidentKernelError::InvalidInput)
        }
        SchemaBody::Map { value, .. } => {
            let ValueData::Map(map) = source.data() else {
                return Err(ResidentKernelError::InvalidInput);
            };
            let ordinal = output_cost
                .selected_ordinal
                .ok_or(ResidentKernelError::InvalidInput)?;
            let entry = map
                .entries()
                .get(ordinal)
                .ok_or(ResidentKernelError::InvalidShape)?;
            canonical_snapshot_data_draft(value, entry.value())
                .map_err(|_| ResidentKernelError::InvalidInput)
        }
        SchemaBody::Table { columns, .. } => {
            let index = output_cost
                .selected_ordinal
                .ok_or(ResidentKernelError::InvalidInput)?;
            let ValueData::Table(table) = source.data() else {
                return Err(ResidentKernelError::InvalidInput);
            };
            let values = table
                .column(index)
                .ok_or(ResidentKernelError::InvalidShape)?;
            let element = &columns[index].schema;
            let output = (0..values.len())
                .map(|index| sequence_data_draft_at(element, values, index))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(ValueDataDraft::Matrix(output.into_boxed_slice()))
        }
        SchemaBody::Matrix { element, .. } => {
            let (rows, columns) = plan
                .source_dimensions
                .ok_or(ResidentKernelError::InvalidInput)?;
            let ValueData::Matrix(matrix) = source.data() else {
                return Err(ResidentKernelError::InvalidInput);
            };
            let elements = matrix.elements();
            let source_len = rows
                .checked_mul(columns)
                .ok_or(ResidentKernelError::InvalidShape)?;
            if elements.len() != source_len {
                return Err(ResidentKernelError::InvalidShape);
            }
            let (selected_rows, selected_columns, scalar) = match plan.matrix_mode {
                Some(ResolvedSelectionMode::Whole) => {
                    return Err(ResidentKernelError::InvalidInput);
                }
                Some(ResolvedSelectionMode::LinearScalar) => {
                    let selected = access_indices(&selectors[0], source_len)?;
                    let [linear] = selected.as_slice() else {
                        return Err(ResidentKernelError::InvalidShape);
                    };
                    let row = linear % rows;
                    let column = linear / rows;
                    return sequence_data_draft_at(element, elements, row * columns + column);
                }
                Some(ResolvedSelectionMode::LinearGather) => {
                    let selected = access_indices(&selectors[0], source_len)?;
                    let output_elements = plan
                        .output_dimensions
                        .and_then(|(rows, columns)| rows.checked_mul(columns))
                        .ok_or(ResidentKernelError::InvalidShape)?;
                    if selected.len() != output_elements {
                        return Err(ResidentKernelError::InvalidShape);
                    }
                    let values = selected
                        .iter()
                        .map(|linear| {
                            let row = linear % rows;
                            let column = linear / rows;
                            sequence_data_draft_at(element, elements, row * columns + column)
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    return Ok(ValueDataDraft::Matrix(values.into_boxed_slice()));
                }
                Some(ResolvedSelectionMode::Rows) => (
                    access_indices(&selectors[0], rows)?,
                    (0..columns).collect(),
                    false,
                ),
                Some(ResolvedSelectionMode::Columns) => (
                    (0..rows).collect(),
                    access_indices(&selectors[0], columns)?,
                    false,
                ),
                Some(ResolvedSelectionMode::Rectangle) => {
                    let selected_rows = access_indices(&selectors[0], rows)?;
                    let selected_columns = access_indices(&selectors[1], columns)?;
                    let scalar = plan.output_dimensions.is_none();
                    (selected_rows, selected_columns, scalar)
                }
                Some(
                    ResolvedSelectionMode::Field { .. }
                    | ResolvedSelectionMode::TableColumn { .. }
                    | ResolvedSelectionMode::MapKey,
                )
                | None => return Err(ResidentKernelError::InvalidInput),
            };
            if scalar {
                if selected_rows.len() != 1 || selected_columns.len() != 1 {
                    return Err(ResidentKernelError::InvalidShape);
                }
                return sequence_data_draft_at(
                    element,
                    elements,
                    selected_rows[0] * columns + selected_columns[0],
                );
            }
            if plan.output_dimensions != Some((selected_rows.len(), selected_columns.len())) {
                return Err(ResidentKernelError::InvalidShape);
            }
            let values = selected_rows
                .iter()
                .flat_map(|row| {
                    selected_columns.iter().map(|column| {
                        sequence_data_draft_at(element, elements, *row * columns + *column)
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(ValueDataDraft::Matrix(values.into_boxed_slice()))
        }
        _ => Err(ResidentKernelError::InvalidInput),
    }
}

fn write_access_output(
    kernel: &BoundResidentKernel,
    plan: &SnapshotAccessPlan,
    canonicalization_work_limit: u64,
    data: ValueDataDraft,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let next =
        finalize_snapshot_data_with_work_budget(kernel, data, Some(canonicalization_work_limit))?;
    if next.schema() != plan.output_schema {
        return Err(ResidentKernelError::InvalidOutput);
    }
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    if let ResidentValueMut::Snapshot([target]) = output {
        let changed = match target.as_ref() {
            Some(current) => !current
                .language_eq(schemas, &next, schemas)
                .map_err(|_| ResidentKernelError::InvalidOutput)?,
            None => true,
        };
        *target = Some(next);
        return Ok(changed);
    }
    let rows = plan.output_geometry.logical_output_rows;
    let columns = plan.output_geometry.logical_output_columns;
    let count = rows
        .checked_mul(columns)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let canonical_to_dense = |index: usize| {
        let row = index % rows;
        let column = index / rows;
        row * columns + column
    };
    match (next.data(), output) {
        (ValueData::Bool(value), ResidentValueMut::Bool([target])) => {
            let next = u8::from(*value);
            let changed = *target != next;
            *target = next;
            Ok(changed)
        }
        (ValueData::Index(value), ResidentValueMut::Index([target])) => {
            let changed = *target != *value;
            *target = *value;
            Ok(changed)
        }
        (ValueData::F64(value), ResidentValueMut::F64([target])) => {
            let next = value.to_f64();
            let changed = target.to_bits() != next.to_bits();
            *target = next;
            Ok(changed)
        }
        (ValueData::String(value), ResidentValueMut::String([target])) => {
            let changed = target.as_str() != value.as_ref();
            target.clear();
            target.push_str(value);
            Ok(changed)
        }
        (ValueData::Matrix(matrix), ResidentValueMut::Bool(target)) => {
            let SequenceView::Bool(values) = matrix.elements() else {
                return Err(ResidentKernelError::InvalidOutput);
            };
            if values.len() != count || target.len() != count {
                return Err(ResidentKernelError::InvalidShape);
            }
            let staged = (0..count)
                .map(|index| u8::from(values[canonical_to_dense(index)]))
                .collect::<Vec<_>>();
            let changed = target != staged;
            target.copy_from_slice(&staged);
            Ok(changed)
        }
        (ValueData::Matrix(matrix), ResidentValueMut::Index(target)) => {
            let SequenceView::Index(values) = matrix.elements() else {
                return Err(ResidentKernelError::InvalidOutput);
            };
            if values.len() != count || target.len() != count {
                return Err(ResidentKernelError::InvalidShape);
            }
            let staged = (0..count)
                .map(|index| values[canonical_to_dense(index)])
                .collect::<Vec<_>>();
            let changed = target != staged;
            target.copy_from_slice(&staged);
            Ok(changed)
        }
        (ValueData::Matrix(matrix), ResidentValueMut::F64(target)) => {
            let SequenceView::F64(values) = matrix.elements() else {
                return Err(ResidentKernelError::InvalidOutput);
            };
            if values.len() != count || target.len() != count {
                return Err(ResidentKernelError::InvalidShape);
            }
            let staged = (0..count)
                .map(|index| values[canonical_to_dense(index)].to_f64())
                .collect::<Vec<_>>();
            let changed = target
                .iter()
                .zip(&staged)
                .any(|(left, right)| left.to_bits() != right.to_bits());
            target.copy_from_slice(&staged);
            Ok(changed)
        }
        (ValueData::Matrix(matrix), ResidentValueMut::String(target)) => {
            let SequenceView::String(values) = matrix.elements() else {
                return Err(ResidentKernelError::InvalidOutput);
            };
            if values.len() != count || target.len() != count {
                return Err(ResidentKernelError::InvalidShape);
            }
            let staged = (0..count)
                .map(|index| values[canonical_to_dense(index)].as_ref().to_owned())
                .collect::<Vec<_>>();
            let changed = target != staged;
            target.clone_from_slice(&staged);
            Ok(changed)
        }
        _ => Err(ResidentKernelError::InvalidOutput),
    }
}

fn snapshot_access(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let plan = kernel
        .retained_state::<SnapshotAccessPlan>()
        .ok_or(ResidentKernelError::InvalidInput)?;
    if inputs.len() != plan.selectors.len() + 1 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let Some(ResidentValueRef::Snapshot([Some(source)])) = inputs.get(0) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let source_schema = source
        .validate_against(schemas)
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    let mut footprint_meter = super::budget::ResidentBudgetMeter::default();
    let source_footprint =
        super::budget::measure_canonical_value_footprint(&mut footprint_meter, source, schemas)?;
    let snapshot_output = matches!(&output, ResidentValueMut::Snapshot(_));
    let dense_string_current_bytes = match &output {
        ResidentValueMut::String(values) => values.iter().try_fold(0_u64, |total, value| {
            total
                .checked_add(super::budget::checked_u64(value.len())?)
                .ok_or(ResidentKernelError::InvalidShape)
        })?,
        _ => 0,
    };
    let (prior_output_footprint, prior_schema_comparison_work, prior_shape_comparison_work) =
        match &output {
            ResidentValueMut::Snapshot([Some(value)]) => {
                value
                    .validate_against(schemas)
                    .map_err(|_| ResidentKernelError::InvalidOutput)?;
                let footprint = super::budget::measure_canonical_value_footprint(
                    &mut footprint_meter,
                    value,
                    schemas,
                )?;
                let output_entry = schemas
                    .entry(plan.output_schema)
                    .ok_or(ResidentKernelError::InvalidOutput)?;
                let schema_work = if value.schema_key() == output_entry.key() {
                    let current_entry = schemas
                        .entry(value.schema())
                        .ok_or(ResidentKernelError::InvalidOutput)?;
                    super::budget::checked_u64(
                        current_entry
                            .canonical_bytes()
                            .len()
                            .max(output_entry.canonical_bytes().len()),
                    )?
                } else {
                    0
                };
                let output_shape = kernel
                    .snapshot_output()
                    .ok_or(ResidentKernelError::InvalidOutput)?
                    .shape
                    .parameter_values()
                    .len();
                let shape_work = super::budget::checked_u64(
                    value.shape().parameter_values().len().max(output_shape),
                )?;
                (Some(footprint), schema_work, shape_work)
            }
            ResidentValueMut::Snapshot([None]) => (None, 0, 0),
            ResidentValueMut::Snapshot(_) => return Err(ResidentKernelError::InvalidOutput),
            _ => (None, 0, 0),
        };
    let prior_output_nodes = match prior_output_footprint {
        Some(footprint) => footprint.node_count,
        None if snapshot_output => 0,
        None => super::budget::checked_u64(output.len())?,
    };
    let mut selector_cost = SelectorMaterializationCost::default();
    for index in 0..plan.selectors.len() {
        let selector = inputs
            .get(index + 1)
            .ok_or(ResidentKernelError::InvalidInput)?;
        selector_cost = add_selector_cost(
            selector_cost,
            snapshot_selector_materialization_cost(
                schemas,
                &plan.selectors[index],
                selector,
                &mut footprint_meter,
            )?,
        )?;
    }
    let output_cost = snapshot_access_output_cost(
        source,
        source_schema.body(),
        plan,
        inputs,
        &mut footprint_meter,
    )?;
    let footprint_work = footprint_meter.estimate();
    let publication_equality_work = match prior_output_footprint {
        Some(prior) => prior_schema_comparison_work
            .checked_add(prior_shape_comparison_work)
            .and_then(|work| work.checked_add(prior.encoded_bytes))
            .and_then(|work| work.checked_add(output_cost.footprint.encoded_bytes))
            .ok_or(ResidentKernelError::InvalidShape)?,
        None if snapshot_output => 0,
        None => super::budget::checked_u64(output_cost.count)?
            .checked_add(dense_string_current_bytes)
            .and_then(|work| {
                if dense_string_current_bytes == 0 {
                    Some(work)
                } else {
                    work.checked_add(output_cost.footprint.encoded_bytes)
                }
            })
            .ok_or(ResidentKernelError::InvalidShape)?,
    };
    let publication_work = output_cost
        .finalization_work
        .checked_add(publication_equality_work)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let published_output_footprint = if snapshot_output {
        let shape_parameters = kernel
            .snapshot_output()
            .ok_or(ResidentKernelError::InvalidOutput)?
            .shape
            .parameter_values()
            .len();
        super::budget::projected_canonical_value_footprint(output_cost.footprint, shape_parameters)?
    } else {
        output_cost.footprint
    };
    let output_data_retained_bytes = output_cost.footprint.retained_bytes;
    let output_retained_bytes = published_output_footprint.retained_bytes;
    let output_nodes = output_cost.footprint.node_count;
    let coordinate_bytes = output_cost
        .index_elements
        .checked_mul(core::mem::size_of::<usize>())
        .ok_or(ResidentKernelError::InvalidShape)?;
    let output_draft_bytes = output_cost
        .count
        .checked_mul(core::mem::size_of::<ValueDataDraft>())
        .ok_or(ResidentKernelError::InvalidShape)?;
    let selector_vector_bytes = plan
        .selectors
        .len()
        .checked_mul(core::mem::size_of::<mech_core::Value>())
        .ok_or(ResidentKernelError::InvalidShape)?;
    let dense_string_publication_copies = match &output {
        ResidentValueMut::String(_) if plan.output_dimensions.is_some() => 2,
        ResidentValueMut::String(_) => 1,
        _ => 0,
    };
    let dense_string_publication_bytes = output_data_retained_bytes
        .checked_mul(dense_string_publication_copies)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let cloned_bytes = selector_cost
        .cloned_bytes
        .checked_add(output_data_retained_bytes)
        .and_then(|bytes| bytes.checked_add(dense_string_publication_bytes))
        .ok_or(ResidentKernelError::InvalidShape)?;
    let staged_value_nodes = published_output_footprint.node_count;
    let final_output_nodes = if snapshot_output {
        published_output_footprint.node_count
    } else {
        super::budget::checked_u64(output_cost.count)?
    };
    let node_phases = snapshot_access_node_phases(
        source_footprint.node_count,
        prior_output_nodes,
        selector_cost.retained_nodes,
        output_nodes,
        staged_value_nodes,
    )?;
    let output_cost = super::budget::PreparedMutationPlan::new(
        output_cost,
        super::budget::PublishedOutputFootprint {
            elements: super::budget::checked_u64(output_cost.count)?,
            retained_bytes: output_retained_bytes,
            retained_nodes: final_output_nodes,
        },
        node_phases,
        super::budget::resident_cost! {
            comparison_work: footprint_work
                .comparison_work()
                .checked_add(publication_work)
                .ok_or(ResidentKernelError::InvalidShape)?,
            compute_work: super::budget::checked_u64(selector_cost.elements)?
                .checked_add(super::budget::checked_u64(output_cost.count)?)
                .and_then(|work| work.checked_add(footprint_work.compute_work()))
                .and_then(|work| work.checked_add(publication_work))
                .ok_or(ResidentKernelError::InvalidShape)?,
            temporary_bytes: output_retained_bytes
                .checked_mul(2)
                .ok_or(ResidentKernelError::InvalidShape)?,
            cloned_bytes,
            container_bytes: output_draft_bytes
                .checked_add(selector_vector_bytes)
                .ok_or(ResidentKernelError::InvalidShape)?,
            selector_bytes: selector_cost
                .retained_bytes
                .checked_add(footprint_work.temporary_bytes())
                .ok_or(ResidentKernelError::InvalidShape)?,
            index_bytes: coordinate_bytes,
            ..super::budget::KernelCostEstimate::default()
        },
    )?
    .admit()?
    .into_plan();
    let selectors = plan
        .selectors
        .iter()
        .enumerate()
        .map(|(index, layout)| {
            selector_value(
                schemas,
                layout,
                inputs
                    .get(index + 1)
                    .ok_or(ResidentKernelError::InvalidInput)?,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let data = snapshot_access_data(
        source,
        source_schema.body(),
        &selectors,
        plan,
        &output_cost,
        schemas,
    )?;
    write_access_output(kernel, plan, output_cost.finalization_work, data, output)
}

fn snapshot_access_node_phases(
    source_nodes: u64,
    prior_output_nodes: u64,
    selector_nodes: u64,
    output_draft_nodes: u64,
    staged_value_nodes: u64,
) -> Result<super::budget::MutationRetainedNodeFootprint, ResidentKernelError> {
    Ok(super::budget::MutationRetainedNodeFootprint {
        current_persistent: source_nodes
            .checked_add(prior_output_nodes)
            .and_then(|nodes| nodes.checked_add(selector_nodes))
            .ok_or(ResidentKernelError::InvalidShape)?,
        normalized_plan: selector_nodes,
        temporary_draft: output_draft_nodes
            .checked_add(staged_value_nodes)
            .ok_or(ResidentKernelError::InvalidShape)?,
    })
}

fn matrix_multiply(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 2 || kernel.parameters().len() != 3 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let lhs = f64_input(inputs, 0)?;
    let rhs = f64_input(inputs, 1)?;
    let output = f64_output(output)?;
    let rows = kernel.parameters()[0] as usize;
    let inner = kernel.parameters()[1] as usize;
    let columns = kernel.parameters()[2] as usize;
    let lhs_len = rows
        .checked_mul(inner)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let rhs_len = inner
        .checked_mul(columns)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let output_len = rows
        .checked_mul(columns)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let compute_work = output_len
        .checked_mul(inner)
        .and_then(|work| work.checked_mul(2))
        .ok_or(ResidentKernelError::InvalidShape)?;
    super::budget::PreparedKernel::new(
        (),
        super::budget::resident_cost! {
            compute_work,
            output_elements: output_len,
            output_bytes: output_len
                .checked_mul(core::mem::size_of::<f64>())
                .ok_or(ResidentKernelError::InvalidShape)?,
            ..super::budget::KernelCostEstimate::default()
        },
    )
    .admit()?
    .into_plan();
    if lhs.len() != lhs_len || rhs.len() != rhs_len || output.len() != output_len {
        return Err(ResidentKernelError::InvalidShape);
    }
    Ok(replace_f64(output, |target| {
        let row = target % rows;
        let column = target / rows;
        (0..inner)
            .map(|offset| lhs[row + offset * rows] * rhs[offset + column * inner])
            .sum()
    }))
}

fn matrix_multiply_snapshot(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let (
        Some(ResidentValueRef::Snapshot([Some(lhs)])),
        Some(ResidentValueRef::Snapshot([Some(rhs)])),
    ) = (inputs.get(0), inputs.get(1))
    else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let [rows, inner, columns] = kernel.parameters() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let rows = usize::try_from(*rows).map_err(|_| ResidentKernelError::InvalidShape)?;
    let inner = usize::try_from(*inner).map_err(|_| ResidentKernelError::InvalidShape)?;
    let columns = usize::try_from(*columns).map_err(|_| ResidentKernelError::InvalidShape)?;
    let lhs_len = rows
        .checked_mul(inner)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let rhs_len = inner
        .checked_mul(columns)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let output_len = rows
        .checked_mul(columns)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let (ValueData::Matrix(lhs_matrix), ValueData::Matrix(rhs_matrix)) = (lhs.data(), rhs.data())
    else {
        return Err(ResidentKernelError::InvalidInput);
    };
    if lhs_matrix.elements().len() != lhs_len || rhs_matrix.elements().len() != rhs_len {
        return Err(ResidentKernelError::InvalidShape);
    }
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let compute_work = output_len
        .checked_mul(inner)
        .and_then(|work| work.checked_mul(2))
        .ok_or(ResidentKernelError::InvalidShape)?;
    preflight_snapshot_arithmetic(
        kernel,
        schemas,
        &[lhs, rhs],
        &output,
        lhs_len
            .checked_add(rhs_len)
            .ok_or(ResidentKernelError::InvalidShape)?,
        output_len,
        compute_work.saturating_sub(output_len),
    )?;
    let ValueDataDraft::Matrix(lhs) = lhs
        .canonical_data_draft()
        .map_err(|_| ResidentKernelError::InvalidInput)?
    else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let ValueDataDraft::Matrix(rhs) = rhs
        .canonical_data_draft()
        .map_err(|_| ResidentKernelError::InvalidInput)?
    else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let metadata = kernel
        .snapshot_output()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let output_schema = schemas
        .get(metadata.schema)
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let SchemaBody::Matrix { element, .. } = output_schema.body() else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let mut result = Vec::with_capacity(output_len);
    for row in 0..rows {
        for column in 0..columns {
            let mut sum = numeric_zero(element)?;
            for offset in 0..inner {
                let product = numeric_multiply(
                    lhs[row * inner + offset].clone(),
                    rhs[offset * columns + column].clone(),
                )?;
                sum = numeric_add(sum, product)?;
            }
            result.push(sum);
        }
    }
    write_snapshot_data_with_work_budget(
        kernel,
        output,
        ValueDataDraft::Matrix(result.into_boxed_slice()),
        Some(0),
    )
}

fn matrix_dot_f64(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let left = f64_input(inputs, 0)?;
    let right = f64_input(inputs, 1)?;
    let output = f64_output(output)?;
    let [target] = output else {
        return Err(ResidentKernelError::InvalidShape);
    };
    if left.len() != right.len() {
        return Err(ResidentKernelError::InvalidShape);
    }
    let next = left
        .iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum::<f64>();
    let changed = target.to_bits() != next.to_bits();
    *target = next;
    Ok(changed)
}

fn snapshot_binary_broadcast_index(mode: u64, row: usize, column: usize, columns: usize) -> usize {
    match mode {
        BINARY_BROADCAST_SCALAR => 0,
        BINARY_BROADCAST_EXACT => row * columns + column,
        BINARY_BROADCAST_COLUMN => row,
        BINARY_BROADCAST_ROW => column,
        _ => unreachable!("validated snapshot arithmetic broadcast mode"),
    }
}

fn preflight_snapshot_arithmetic(
    kernel: &BoundResidentKernel,
    schemas: &mech_core::SchemaTable,
    inputs: &[&mech_core::Value],
    output: &ResidentValueMut<'_>,
    input_elements: usize,
    output_elements: usize,
    additional_compute_work: usize,
) -> Result<(), ResidentKernelError> {
    let mut footprint_meter = super::budget::ResidentBudgetMeter::default();
    let (cloned_bytes, _input_nodes) =
        inputs
            .iter()
            .try_fold((0u64, 0u64), |(bytes, nodes), value| {
                let (next_bytes, next_nodes) =
                    snapshot_clone_cost(&mut footprint_meter, value, schemas)?;
                Ok::<_, ResidentKernelError>((
                    bytes
                        .checked_add(next_bytes)
                        .ok_or(ResidentKernelError::InvalidShape)?,
                    nodes
                        .checked_add(next_nodes)
                        .ok_or(ResidentKernelError::InvalidShape)?,
                ))
            })?;
    let previous_footprint = match output {
        ResidentValueMut::Snapshot(values) => match values.first().and_then(|value| value.as_ref())
        {
            Some(previous) => Some(super::budget::measure_canonical_value_footprint(
                &mut footprint_meter,
                previous,
                schemas,
            )?),
            None => None,
        },
        _ => return Err(ResidentKernelError::InvalidOutput),
    };
    let draft_elements = input_elements
        .checked_add(output_elements)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let draft_bytes = super::budget::checked_u64(
        draft_elements
            .checked_mul(core::mem::size_of::<ValueDataDraft>())
            .ok_or(ResidentKernelError::InvalidShape)?,
    )?;
    let output_bytes = super::budget::checked_u64(
        output_elements
            .checked_mul(core::mem::size_of::<ValueDataDraft>())
            .ok_or(ResidentKernelError::InvalidShape)?,
    )?;
    let output_metadata = kernel
        .snapshot_output()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let output_nodes = super::budget::checked_u64(output_elements)?
        .checked_add(2)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let output_footprint = ValueFootprint {
        // Numeric scalar/matrix equality visits each element once. Container
        // tags add only constant work, covered by the two wrapper nodes.
        encoded_bytes: output_nodes,
        retained_bytes: output_bytes,
        node_count: output_nodes,
    };
    let publication_work = match (output, previous_footprint) {
        (ResidentValueMut::Snapshot(values), Some(previous_footprint)) => {
            let previous = values
                .first()
                .and_then(|value| value.as_ref())
                .ok_or(ResidentKernelError::InvalidOutput)?;
            super::budget::projected_language_equality_work(
                schemas,
                previous,
                previous_footprint,
                output_metadata.schema,
                output_metadata.shape.parameter_values().len(),
                output_footprint,
            )?
        }
        _ => 0,
    };
    let measured = footprint_meter.estimate();
    super::budget::PreparedKernel::new(
        (),
        super::budget::resident_cost! {
            comparison_work: measured.comparison_work()
                .checked_add(publication_work)
                .ok_or(ResidentKernelError::InvalidShape)?,
            compute_work: super::budget::checked_u64(output_elements)?
                .checked_add(super::budget::checked_u64(additional_compute_work)?)
                .and_then(|work| work.checked_add(measured.compute_work()))
                .and_then(|work| work.checked_add(publication_work))
                .ok_or(ResidentKernelError::InvalidShape)?,
            output_elements,
            output_bytes,
            temporary_bytes: cloned_bytes
                .checked_add(draft_bytes)
                .ok_or(ResidentKernelError::InvalidShape)?,
            cloned_bytes,
            retained_nodes: measured.retained_nodes()
                .checked_add(output_nodes.checked_mul(2).ok_or(ResidentKernelError::InvalidShape)?)
                .ok_or(ResidentKernelError::InvalidShape)?,
            ..super::budget::KernelCostEstimate::default()
        },
    )
    .admit()?
    .into_plan();
    Ok(())
}

fn snapshot_numeric_binary(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let (
        Some(ResidentValueRef::Snapshot([Some(left)])),
        Some(ResidentValueRef::Snapshot([Some(right)])),
    ) = (inputs.get(0), inputs.get(1))
    else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let [
        rows,
        columns,
        left_mode,
        right_mode,
        arithmetic,
        rational_power,
    ] = kernel.parameters()
    else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let rows = usize::try_from(*rows).map_err(|_| ResidentKernelError::InvalidShape)?;
    let columns = usize::try_from(*columns).map_err(|_| ResidentKernelError::InvalidShape)?;
    let output_len = rows
        .checked_mul(columns)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let arithmetic =
        SemanticArithmetic::from_parameter(*arithmetic).ok_or(ResidentKernelError::InvalidInput)?;
    let rational_power = match *rational_power {
        0 => false,
        1 => true,
        _ => return Err(ResidentKernelError::InvalidInput),
    };
    let left_len = snapshot_numeric_element_count(left)?;
    let right_len = snapshot_numeric_element_count(right)?;
    validate_binary_broadcast_len(left_len, *left_mode, rows, columns)?;
    validate_binary_broadcast_len(right_len, *right_mode, rows, columns)?;
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    left.validate_against(schemas)
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    right
        .validate_against(schemas)
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    preflight_snapshot_arithmetic(
        kernel,
        schemas,
        &[left, right],
        &output,
        left_len
            .checked_add(right_len)
            .ok_or(ResidentKernelError::InvalidShape)?,
        output_len,
        0,
    )?;
    let left = snapshot_numeric_elements(left)?;
    let right = snapshot_numeric_elements(right)?;
    let mut result = Vec::with_capacity(output_len);
    for row in 0..rows {
        for column in 0..columns {
            let left = left
                .get(snapshot_binary_broadcast_index(
                    *left_mode, row, column, columns,
                ))
                .ok_or(ResidentKernelError::InvalidShape)?
                .clone();
            let right = right
                .get(snapshot_binary_broadcast_index(
                    *right_mode,
                    row,
                    column,
                    columns,
                ))
                .ok_or(ResidentKernelError::InvalidShape)?
                .clone();
            result.push(if rational_power {
                numeric_rational_power(left, right)?
            } else {
                numeric_arithmetic(arithmetic, left, right)?
            });
        }
    }
    let output_schema = schemas
        .get(
            kernel
                .snapshot_output()
                .ok_or(ResidentKernelError::InvalidOutput)?
                .schema,
        )
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let data = if matches!(output_schema.body(), SchemaBody::Matrix { .. }) {
        ValueDataDraft::Matrix(result.into_boxed_slice())
    } else {
        let [value] = result.as_slice() else {
            return Err(ResidentKernelError::InvalidShape);
        };
        value.clone()
    };
    write_snapshot_data_with_work_budget(kernel, output, data, Some(0))
}

fn snapshot_numeric_negate(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let Some(ResidentValueRef::Snapshot([Some(input)])) = inputs.get(0) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let [rows, columns] = kernel.parameters() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let rows = usize::try_from(*rows).map_err(|_| ResidentKernelError::InvalidShape)?;
    let columns = usize::try_from(*columns).map_err(|_| ResidentKernelError::InvalidShape)?;
    let output_len = rows
        .checked_mul(columns)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let input_len = snapshot_numeric_element_count(input)?;
    if input_len != output_len {
        return Err(ResidentKernelError::InvalidShape);
    }
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    input
        .validate_against(schemas)
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    preflight_snapshot_arithmetic(kernel, schemas, &[input], &output, input_len, output_len, 0)?;
    let result = snapshot_numeric_elements(input)?
        .into_iter()
        .map(numeric_negate)
        .collect::<Result<Vec<_>, _>>()?;
    let output_schema = schemas
        .get(
            kernel
                .snapshot_output()
                .ok_or(ResidentKernelError::InvalidOutput)?
                .schema,
        )
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let data = if matches!(output_schema.body(), SchemaBody::Matrix { .. }) {
        ValueDataDraft::Matrix(result.into_boxed_slice())
    } else {
        let [value] = result.as_slice() else {
            return Err(ResidentKernelError::InvalidShape);
        };
        value.clone()
    };
    write_snapshot_data_with_work_budget(kernel, output, data, Some(0))
}

fn snapshot_numeric_abs(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let Some(ResidentValueRef::Snapshot([Some(input)])) = inputs.get(0) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let [rows, columns] = kernel.parameters() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let rows = usize::try_from(*rows).map_err(|_| ResidentKernelError::InvalidShape)?;
    let columns = usize::try_from(*columns).map_err(|_| ResidentKernelError::InvalidShape)?;
    let output_len = rows
        .checked_mul(columns)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let input_len = snapshot_numeric_element_count(input)?;
    if input_len != output_len {
        return Err(ResidentKernelError::InvalidShape);
    }
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    input
        .validate_against(schemas)
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    preflight_snapshot_arithmetic(kernel, schemas, &[input], &output, input_len, output_len, 0)?;
    let result = snapshot_numeric_elements(input)?
        .into_iter()
        .map(numeric_abs)
        .collect::<Result<Vec<_>, _>>()?;
    let output_schema = schemas
        .get(
            kernel
                .snapshot_output()
                .ok_or(ResidentKernelError::InvalidOutput)?
                .schema,
        )
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let data = if matches!(output_schema.body(), SchemaBody::Matrix { .. }) {
        ValueDataDraft::Matrix(result.into_boxed_slice())
    } else {
        let [value] = result.as_slice() else {
            return Err(ResidentKernelError::InvalidShape);
        };
        value.clone()
    };
    write_snapshot_data_with_work_budget(kernel, output, data, Some(0))
}

fn numeric_zero(body: &SchemaBody) -> Result<ValueDataDraft, ResidentKernelError> {
    use mech_core::IntegerWidth;
    match body {
        SchemaBody::UnsignedInteger(IntegerWidth::W8) => Ok(ValueDataDraft::U8(0)),
        SchemaBody::UnsignedInteger(IntegerWidth::W16) => Ok(ValueDataDraft::U16(0)),
        SchemaBody::UnsignedInteger(IntegerWidth::W32) => Ok(ValueDataDraft::U32(0)),
        SchemaBody::UnsignedInteger(IntegerWidth::W64) => Ok(ValueDataDraft::U64(0)),
        SchemaBody::UnsignedInteger(IntegerWidth::W128) => Ok(ValueDataDraft::U128(0)),
        SchemaBody::SignedInteger(IntegerWidth::W8) => Ok(ValueDataDraft::I8(0)),
        SchemaBody::SignedInteger(IntegerWidth::W16) => Ok(ValueDataDraft::I16(0)),
        SchemaBody::SignedInteger(IntegerWidth::W32) => Ok(ValueDataDraft::I32(0)),
        SchemaBody::SignedInteger(IntegerWidth::W64) => Ok(ValueDataDraft::I64(0)),
        SchemaBody::SignedInteger(IntegerWidth::W128) => Ok(ValueDataDraft::I128(0)),
        SchemaBody::FloatingPoint(mech_core::FloatWidth::W32) => {
            Ok(ValueDataDraft::F32(F32Bits::from_f32(0.0)))
        }
        SchemaBody::FloatingPoint(mech_core::FloatWidth::W64) => {
            Ok(ValueDataDraft::F64(F64Bits::from_f64(0.0)))
        }
        SchemaBody::Rational64 => Ok(ValueDataDraft::Rational64 {
            numerator: 0,
            denominator: 1,
        }),
        SchemaBody::Complex(mech_core::FloatWidth::W64) => Ok(ValueDataDraft::Complex64(
            mech_core::snapshot::Complex64Bits::new(F64Bits::from_f64(0.0), F64Bits::from_f64(0.0)),
        )),
        _ => Err(ResidentKernelError::InvalidInput),
    }
}

fn numeric_one(body: &SchemaBody) -> Result<ValueDataDraft, ResidentKernelError> {
    use mech_core::IntegerWidth;
    match body {
        SchemaBody::UnsignedInteger(IntegerWidth::W8) => Ok(ValueDataDraft::U8(1)),
        SchemaBody::UnsignedInteger(IntegerWidth::W16) => Ok(ValueDataDraft::U16(1)),
        SchemaBody::UnsignedInteger(IntegerWidth::W32) => Ok(ValueDataDraft::U32(1)),
        SchemaBody::UnsignedInteger(IntegerWidth::W64) => Ok(ValueDataDraft::U64(1)),
        SchemaBody::UnsignedInteger(IntegerWidth::W128) => Ok(ValueDataDraft::U128(1)),
        SchemaBody::SignedInteger(IntegerWidth::W8) => Ok(ValueDataDraft::I8(1)),
        SchemaBody::SignedInteger(IntegerWidth::W16) => Ok(ValueDataDraft::I16(1)),
        SchemaBody::SignedInteger(IntegerWidth::W32) => Ok(ValueDataDraft::I32(1)),
        SchemaBody::SignedInteger(IntegerWidth::W64) => Ok(ValueDataDraft::I64(1)),
        SchemaBody::SignedInteger(IntegerWidth::W128) => Ok(ValueDataDraft::I128(1)),
        SchemaBody::FloatingPoint(mech_core::FloatWidth::W32) => {
            Ok(ValueDataDraft::F32(F32Bits::from_f32(1.0)))
        }
        SchemaBody::FloatingPoint(mech_core::FloatWidth::W64) => {
            Ok(ValueDataDraft::F64(F64Bits::from_f64(1.0)))
        }
        SchemaBody::Rational64 => Ok(ValueDataDraft::Rational64 {
            numerator: 1,
            denominator: 1,
        }),
        SchemaBody::Complex(mech_core::FloatWidth::W64) => Ok(ValueDataDraft::Complex64(
            mech_core::snapshot::Complex64Bits::new(F64Bits::from_f64(1.0), F64Bits::from_f64(0.0)),
        )),
        _ => Err(ResidentKernelError::InvalidInput),
    }
}

macro_rules! checked_numeric_binary {
    ($left:expr, $right:expr, $method:ident) => {
        match ($left, $right) {
            (ValueDataDraft::U8(left), ValueDataDraft::U8(right)) => {
                left.$method(right).map(ValueDataDraft::U8)
            }
            (ValueDataDraft::U16(left), ValueDataDraft::U16(right)) => {
                left.$method(right).map(ValueDataDraft::U16)
            }
            (ValueDataDraft::U32(left), ValueDataDraft::U32(right)) => {
                left.$method(right).map(ValueDataDraft::U32)
            }
            (ValueDataDraft::U64(left), ValueDataDraft::U64(right)) => {
                left.$method(right).map(ValueDataDraft::U64)
            }
            (ValueDataDraft::U128(left), ValueDataDraft::U128(right)) => {
                left.$method(right).map(ValueDataDraft::U128)
            }
            (ValueDataDraft::I8(left), ValueDataDraft::I8(right)) => {
                left.$method(right).map(ValueDataDraft::I8)
            }
            (ValueDataDraft::I16(left), ValueDataDraft::I16(right)) => {
                left.$method(right).map(ValueDataDraft::I16)
            }
            (ValueDataDraft::I32(left), ValueDataDraft::I32(right)) => {
                left.$method(right).map(ValueDataDraft::I32)
            }
            (ValueDataDraft::I64(left), ValueDataDraft::I64(right)) => {
                left.$method(right).map(ValueDataDraft::I64)
            }
            (ValueDataDraft::I128(left), ValueDataDraft::I128(right)) => {
                left.$method(right).map(ValueDataDraft::I128)
            }
            _ => None,
        }
    };
}

#[cfg(feature = "r64")]
fn rational_from_draft(value: ValueDataDraft) -> Result<mech_core::R64, ResidentKernelError> {
    let ValueDataDraft::Rational64 {
        numerator,
        denominator,
    } = value
    else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let denominator = i64::try_from(denominator).map_err(|_| ResidentKernelError::Arithmetic)?;
    if denominator <= 0 {
        return Err(ResidentKernelError::InvalidInput);
    }
    Ok(mech_core::R64::new(numerator, denominator))
}

#[cfg(feature = "r64")]
fn rational_to_draft(value: mech_core::R64) -> Result<ValueDataDraft, ResidentKernelError> {
    let numerator = *value.numer();
    let denominator = u64::try_from(*value.denom()).map_err(|_| ResidentKernelError::Arithmetic)?;
    if denominator == 0 {
        return Err(ResidentKernelError::Arithmetic);
    }
    Ok(ValueDataDraft::Rational64 {
        numerator,
        denominator,
    })
}

#[cfg(feature = "c64")]
fn complex_from_draft(value: ValueDataDraft) -> Result<mech_core::C64, ResidentKernelError> {
    let ValueDataDraft::Complex64(value) = value else {
        return Err(ResidentKernelError::InvalidInput);
    };
    Ok(mech_core::C64::new(
        value.real().to_f64(),
        value.imaginary().to_f64(),
    ))
}

#[cfg(feature = "c64")]
fn complex_to_draft(value: mech_core::C64) -> ValueDataDraft {
    ValueDataDraft::Complex64(mech_core::snapshot::Complex64Bits::new(
        F64Bits::from_f64(value.0.re),
        F64Bits::from_f64(value.0.im),
    ))
}

fn numeric_multiply(
    left: ValueDataDraft,
    right: ValueDataDraft,
) -> Result<ValueDataDraft, ResidentKernelError> {
    match (left, right) {
        (ValueDataDraft::F32(left), ValueDataDraft::F32(right)) => Ok(ValueDataDraft::F32(
            F32Bits::from_f32(left.to_f32() * right.to_f32()),
        )),
        (ValueDataDraft::F64(left), ValueDataDraft::F64(right)) => Ok(ValueDataDraft::F64(
            F64Bits::from_f64(left.to_f64() * right.to_f64()),
        )),
        #[cfg(feature = "r64")]
        (left @ ValueDataDraft::Rational64 { .. }, right @ ValueDataDraft::Rational64 { .. }) => {
            let next = rational_from_draft(left)?
                .checked_mul(rational_from_draft(right)?)
                .ok_or(ResidentKernelError::Arithmetic)?;
            rational_to_draft(next)
        }
        #[cfg(feature = "c64")]
        (left @ ValueDataDraft::Complex64(_), right @ ValueDataDraft::Complex64(_)) => Ok(
            complex_to_draft(complex_from_draft(left)? * complex_from_draft(right)?),
        ),
        (left, right) => {
            checked_numeric_binary!(left, right, checked_mul).ok_or(ResidentKernelError::Arithmetic)
        }
    }
}

fn numeric_add(
    left: ValueDataDraft,
    right: ValueDataDraft,
) -> Result<ValueDataDraft, ResidentKernelError> {
    match (left, right) {
        (ValueDataDraft::F32(left), ValueDataDraft::F32(right)) => Ok(ValueDataDraft::F32(
            F32Bits::from_f32(left.to_f32() + right.to_f32()),
        )),
        (ValueDataDraft::F64(left), ValueDataDraft::F64(right)) => Ok(ValueDataDraft::F64(
            F64Bits::from_f64(left.to_f64() + right.to_f64()),
        )),
        #[cfg(feature = "r64")]
        (left @ ValueDataDraft::Rational64 { .. }, right @ ValueDataDraft::Rational64 { .. }) => {
            let next = rational_from_draft(left)?
                .checked_add(rational_from_draft(right)?)
                .ok_or(ResidentKernelError::Arithmetic)?;
            rational_to_draft(next)
        }
        #[cfg(feature = "c64")]
        (left @ ValueDataDraft::Complex64(_), right @ ValueDataDraft::Complex64(_)) => Ok(
            complex_to_draft(complex_from_draft(left)? + complex_from_draft(right)?),
        ),
        (left, right) => {
            checked_numeric_binary!(left, right, checked_add).ok_or(ResidentKernelError::Arithmetic)
        }
    }
}

fn numeric_subtract(
    left: ValueDataDraft,
    right: ValueDataDraft,
) -> Result<ValueDataDraft, ResidentKernelError> {
    match (left, right) {
        (ValueDataDraft::F32(left), ValueDataDraft::F32(right)) => Ok(ValueDataDraft::F32(
            F32Bits::from_f32(left.to_f32() - right.to_f32()),
        )),
        (ValueDataDraft::F64(left), ValueDataDraft::F64(right)) => Ok(ValueDataDraft::F64(
            F64Bits::from_f64(left.to_f64() - right.to_f64()),
        )),
        #[cfg(feature = "r64")]
        (left @ ValueDataDraft::Rational64 { .. }, right @ ValueDataDraft::Rational64 { .. }) => {
            let next = rational_from_draft(left)?
                .checked_sub(rational_from_draft(right)?)
                .ok_or(ResidentKernelError::Arithmetic)?;
            rational_to_draft(next)
        }
        #[cfg(feature = "c64")]
        (left @ ValueDataDraft::Complex64(_), right @ ValueDataDraft::Complex64(_)) => Ok(
            complex_to_draft(complex_from_draft(left)? - complex_from_draft(right)?),
        ),
        (left, right) => {
            checked_numeric_binary!(left, right, checked_sub).ok_or(ResidentKernelError::Arithmetic)
        }
    }
}

fn numeric_divide(
    left: ValueDataDraft,
    right: ValueDataDraft,
) -> Result<ValueDataDraft, ResidentKernelError> {
    match (left, right) {
        (ValueDataDraft::F32(left), ValueDataDraft::F32(right)) => Ok(ValueDataDraft::F32(
            F32Bits::from_f32(left.to_f32() / right.to_f32()),
        )),
        (ValueDataDraft::F64(left), ValueDataDraft::F64(right)) => Ok(ValueDataDraft::F64(
            F64Bits::from_f64(left.to_f64() / right.to_f64()),
        )),
        #[cfg(feature = "r64")]
        (left @ ValueDataDraft::Rational64 { .. }, right @ ValueDataDraft::Rational64 { .. }) => {
            let next = rational_from_draft(left)?
                .checked_div(rational_from_draft(right)?)
                .ok_or(ResidentKernelError::Arithmetic)?;
            rational_to_draft(next)
        }
        #[cfg(feature = "c64")]
        (left @ ValueDataDraft::Complex64(_), right @ ValueDataDraft::Complex64(_)) => Ok(
            complex_to_draft(complex_from_draft(left)? / complex_from_draft(right)?),
        ),
        (left, right) => {
            checked_numeric_binary!(left, right, checked_div).ok_or(ResidentKernelError::Arithmetic)
        }
    }
}

fn numeric_remainder(
    left: ValueDataDraft,
    right: ValueDataDraft,
) -> Result<ValueDataDraft, ResidentKernelError> {
    match (left, right) {
        (ValueDataDraft::F32(left), ValueDataDraft::F32(right)) => Ok(ValueDataDraft::F32(
            F32Bits::from_f32(left.to_f32() % right.to_f32()),
        )),
        (ValueDataDraft::F64(left), ValueDataDraft::F64(right)) => Ok(ValueDataDraft::F64(
            F64Bits::from_f64(left.to_f64() % right.to_f64()),
        )),
        (left, right) => {
            checked_numeric_binary!(left, right, checked_rem).ok_or(ResidentKernelError::Arithmetic)
        }
    }
}

fn numeric_power(
    left: ValueDataDraft,
    right: ValueDataDraft,
) -> Result<ValueDataDraft, ResidentKernelError> {
    match (left, right) {
        (ValueDataDraft::U8(left), ValueDataDraft::U8(right)) => left
            .checked_pow(u32::from(right))
            .map(ValueDataDraft::U8)
            .ok_or(ResidentKernelError::Arithmetic),
        (ValueDataDraft::U16(left), ValueDataDraft::U16(right)) => left
            .checked_pow(u32::from(right))
            .map(ValueDataDraft::U16)
            .ok_or(ResidentKernelError::Arithmetic),
        (ValueDataDraft::U32(left), ValueDataDraft::U32(right)) => left
            .checked_pow(right)
            .map(ValueDataDraft::U32)
            .ok_or(ResidentKernelError::Arithmetic),
        (ValueDataDraft::F32(left), ValueDataDraft::F32(right)) => Ok(ValueDataDraft::F32(
            F32Bits::from_f32(left.to_f32().powf(right.to_f32())),
        )),
        (ValueDataDraft::F64(left), ValueDataDraft::F64(right)) => Ok(ValueDataDraft::F64(
            F64Bits::from_f64(left.to_f64().powf(right.to_f64())),
        )),
        _ => Err(ResidentKernelError::InvalidInput),
    }
}

#[cfg(feature = "r64")]
fn numeric_rational_power_impl(
    left: ValueDataDraft,
    right: ValueDataDraft,
) -> Result<ValueDataDraft, ResidentKernelError> {
    let mut base = rational_from_draft(left)?;
    let ValueDataDraft::I32(exponent) = right else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let negative = exponent < 0;
    let mut exponent = exponent.unsigned_abs();
    let mut result = mech_core::R64::new(1, 1);
    while exponent != 0 {
        if exponent & 1 == 1 {
            result = result
                .checked_mul(base)
                .ok_or(ResidentKernelError::Arithmetic)?;
        }
        exponent >>= 1;
        if exponent != 0 {
            base = base
                .checked_mul(base)
                .ok_or(ResidentKernelError::Arithmetic)?;
        }
    }
    if negative {
        result = mech_core::R64::new(1, 1)
            .checked_div(result)
            .ok_or(ResidentKernelError::Arithmetic)?;
    }
    rational_to_draft(result)
}

fn numeric_rational_power(
    left: ValueDataDraft,
    right: ValueDataDraft,
) -> Result<ValueDataDraft, ResidentKernelError> {
    #[cfg(feature = "r64")]
    {
        return numeric_rational_power_impl(left, right);
    }
    #[cfg(not(feature = "r64"))]
    {
        let _ = (left, right);
        Err(ResidentKernelError::InvalidInput)
    }
}

fn numeric_arithmetic(
    arithmetic: SemanticArithmetic,
    left: ValueDataDraft,
    right: ValueDataDraft,
) -> Result<ValueDataDraft, ResidentKernelError> {
    match arithmetic {
        SemanticArithmetic::Add => numeric_add(left, right),
        SemanticArithmetic::Subtract => numeric_subtract(left, right),
        SemanticArithmetic::Multiply => numeric_multiply(left, right),
        SemanticArithmetic::Divide => numeric_divide(left, right),
        SemanticArithmetic::Remainder => numeric_remainder(left, right),
        SemanticArithmetic::Power => numeric_power(left, right),
    }
}

fn numeric_negate(value: ValueDataDraft) -> Result<ValueDataDraft, ResidentKernelError> {
    Ok(match value {
        ValueDataDraft::I8(value) => {
            ValueDataDraft::I8(value.checked_neg().ok_or(ResidentKernelError::Arithmetic)?)
        }
        ValueDataDraft::I16(value) => {
            ValueDataDraft::I16(value.checked_neg().ok_or(ResidentKernelError::Arithmetic)?)
        }
        ValueDataDraft::I32(value) => {
            ValueDataDraft::I32(value.checked_neg().ok_or(ResidentKernelError::Arithmetic)?)
        }
        ValueDataDraft::I64(value) => {
            ValueDataDraft::I64(value.checked_neg().ok_or(ResidentKernelError::Arithmetic)?)
        }
        ValueDataDraft::I128(value) => {
            ValueDataDraft::I128(value.checked_neg().ok_or(ResidentKernelError::Arithmetic)?)
        }
        ValueDataDraft::F32(value) => ValueDataDraft::F32(F32Bits::from_f32(-value.to_f32())),
        ValueDataDraft::F64(value) => ValueDataDraft::F64(F64Bits::from_f64(-value.to_f64())),
        #[cfg(feature = "r64")]
        value @ ValueDataDraft::Rational64 { .. } => rational_to_draft(
            rational_from_draft(value)?
                .checked_neg()
                .ok_or(ResidentKernelError::Arithmetic)?,
        )?,
        #[cfg(feature = "c64")]
        value @ ValueDataDraft::Complex64(_) => complex_to_draft(-complex_from_draft(value)?),
        _ => return Err(ResidentKernelError::InvalidInput),
    })
}

fn numeric_abs(value: ValueDataDraft) -> Result<ValueDataDraft, ResidentKernelError> {
    Ok(match value {
        value @ (ValueDataDraft::U8(_)
        | ValueDataDraft::U16(_)
        | ValueDataDraft::U32(_)
        | ValueDataDraft::U64(_)
        | ValueDataDraft::U128(_)) => value,
        ValueDataDraft::I8(value) => {
            ValueDataDraft::I8(value.checked_abs().ok_or(ResidentKernelError::Arithmetic)?)
        }
        ValueDataDraft::I16(value) => {
            ValueDataDraft::I16(value.checked_abs().ok_or(ResidentKernelError::Arithmetic)?)
        }
        ValueDataDraft::I32(value) => {
            ValueDataDraft::I32(value.checked_abs().ok_or(ResidentKernelError::Arithmetic)?)
        }
        ValueDataDraft::I64(value) => {
            ValueDataDraft::I64(value.checked_abs().ok_or(ResidentKernelError::Arithmetic)?)
        }
        ValueDataDraft::I128(value) => {
            ValueDataDraft::I128(value.checked_abs().ok_or(ResidentKernelError::Arithmetic)?)
        }
        ValueDataDraft::F32(value) => ValueDataDraft::F32(F32Bits::from_f32(value.to_f32().abs())),
        ValueDataDraft::F64(value) => ValueDataDraft::F64(F64Bits::from_f64(value.to_f64().abs())),
        #[cfg(feature = "r64")]
        value @ ValueDataDraft::Rational64 { .. } => {
            rational_to_draft(rational_from_draft(value)?.abs())?
        }
        #[cfg(feature = "c64")]
        value @ ValueDataDraft::Complex64(_) => complex_to_draft(complex_from_draft(value)?.abs()),
        _ => return Err(ResidentKernelError::InvalidInput),
    })
}

fn snapshot_numeric_elements(
    value: &mech_core::Value,
) -> Result<Vec<ValueDataDraft>, ResidentKernelError> {
    match value
        .canonical_data_draft()
        .map_err(|_| ResidentKernelError::InvalidInput)?
    {
        ValueDataDraft::Matrix(elements) => Ok(elements.into_vec()),
        scalar => Ok(vec![scalar]),
    }
}

fn snapshot_numeric_element_count(value: &mech_core::Value) -> Result<usize, ResidentKernelError> {
    match value.data() {
        ValueData::Matrix(elements) => Ok(elements.elements().len()),
        ValueData::U8(_)
        | ValueData::U16(_)
        | ValueData::U32(_)
        | ValueData::U64(_)
        | ValueData::U128(_)
        | ValueData::I8(_)
        | ValueData::I16(_)
        | ValueData::I32(_)
        | ValueData::I64(_)
        | ValueData::I128(_)
        | ValueData::F32(_)
        | ValueData::F64(_)
        | ValueData::Complex64(_)
        | ValueData::Rational64(_) => Ok(1),
        _ => Err(ResidentKernelError::InvalidInput),
    }
}

fn write_snapshot_data_with_work_budget(
    kernel: &BoundResidentKernel,
    output: ResidentValueMut<'_>,
    data: ValueDataDraft,
    canonicalization_work_limit: Option<u64>,
) -> Result<bool, ResidentKernelError> {
    let next = finalize_snapshot_data_with_work_budget(kernel, data, canonicalization_work_limit)?;
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let ResidentValueMut::Snapshot([target]) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let changed = match target.as_ref() {
        Some(current) => !current
            .language_eq(schemas, &next, schemas)
            .map_err(|_| ResidentKernelError::InvalidOutput)?,
        None => true,
    };
    *target = Some(next);
    Ok(changed)
}

fn finalize_snapshot_data_with_work_budget(
    kernel: &BoundResidentKernel,
    data: ValueDataDraft,
    canonicalization_work_limit: Option<u64>,
) -> Result<mech_core::Value, ResidentKernelError> {
    let metadata = kernel
        .snapshot_output()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let budget = canonicalization_work_limit.map(SnapshotCanonicalizationBudget::new);
    let mut context = SnapshotValidationContext::new(schemas);
    if let Some(budget) = budget.as_ref() {
        context = context.with_canonicalization_budget(budget);
    }
    let next = ValueDraft {
        schema: metadata.schema,
        shape_values: metadata
            .shape
            .parameter_values()
            .to_vec()
            .into_boxed_slice(),
        data,
    }
    .finalize(&context)
    .map_err(|error| match error {
        SnapshotValueError::CanonicalizationWorkLimitExceededV1 { .. } => {
            ResidentKernelError::InvalidShape
        }
        _ => ResidentKernelError::InvalidOutput,
    })?;
    if next.schema_key() != metadata.schema_key {
        return Err(ResidentKernelError::InvalidOutput);
    }
    Ok(next)
}

fn matrix_dot_snapshot(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let (
        Some(ResidentValueRef::Snapshot([Some(left)])),
        Some(ResidentValueRef::Snapshot([Some(right)])),
    ) = (inputs.get(0), inputs.get(1))
    else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let left_count = snapshot_numeric_element_count(left)?;
    let right_count = snapshot_numeric_element_count(right)?;
    if left_count != right_count {
        return Err(ResidentKernelError::InvalidShape);
    }
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    preflight_snapshot_arithmetic(
        kernel,
        schemas,
        &[left, right],
        &output,
        left_count
            .checked_add(right_count)
            .ok_or(ResidentKernelError::InvalidShape)?,
        1,
        left_count
            .checked_mul(2)
            .ok_or(ResidentKernelError::InvalidShape)?,
    )?;
    let left = snapshot_numeric_elements(left)?;
    let right = snapshot_numeric_elements(right)?;
    debug_assert_eq!(left.len(), right.len());
    let metadata = kernel
        .snapshot_output()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let output_schema = schemas
        .get(metadata.schema)
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let mut next = numeric_zero(output_schema.body())?;
    for (left, right) in left.into_iter().zip(right) {
        next = numeric_add(next, numeric_multiply(left, right)?)?;
    }
    write_snapshot_data_with_work_budget(kernel, output, next, Some(0))
}

trait ResidentSolveFloat:
    Copy
    + PartialEq
    + PartialOrd
    + std::ops::Sub<Output = Self>
    + std::ops::Mul<Output = Self>
    + std::ops::Div<Output = Self>
{
    fn zero() -> Self;
    fn abs(self) -> Self;
    fn is_finite(self) -> bool;
}

impl ResidentSolveFloat for f32 {
    fn zero() -> Self {
        0.0
    }

    fn abs(self) -> Self {
        self.abs()
    }

    fn is_finite(self) -> bool {
        self.is_finite()
    }
}

impl ResidentSolveFloat for f64 {
    fn zero() -> Self {
        0.0
    }

    fn abs(self) -> Self {
        self.abs()
    }

    fn is_finite(self) -> bool {
        self.is_finite()
    }
}

fn solve_dense<T: ResidentSolveFloat>(
    mut coefficients: Vec<T>,
    mut right: Vec<T>,
    rows: usize,
    right_columns: usize,
) -> Result<Vec<T>, ResidentKernelError> {
    if coefficients.len() != rows.saturating_mul(rows)
        || right.len() != rows.saturating_mul(right_columns)
    {
        return Err(ResidentKernelError::InvalidShape);
    }
    for pivot_column in 0..rows {
        let mut pivot_row = pivot_column;
        let mut pivot_abs = coefficients[pivot_row + pivot_column * rows].abs();
        for row in pivot_column + 1..rows {
            let candidate = coefficients[row + pivot_column * rows].abs();
            if candidate > pivot_abs {
                pivot_row = row;
                pivot_abs = candidate;
            }
        }
        if pivot_abs == T::zero() || !pivot_abs.is_finite() {
            return Err(ResidentKernelError::Arithmetic);
        }
        if pivot_row != pivot_column {
            for column in 0..rows {
                coefficients.swap(pivot_row + column * rows, pivot_column + column * rows);
            }
            for column in 0..right_columns {
                right.swap(pivot_row + column * rows, pivot_column + column * rows);
            }
        }
        let pivot = coefficients[pivot_column + pivot_column * rows];
        for column in pivot_column..rows {
            let index = pivot_column + column * rows;
            coefficients[index] = coefficients[index] / pivot;
        }
        for column in 0..right_columns {
            let index = pivot_column + column * rows;
            right[index] = right[index] / pivot;
        }
        for row in 0..rows {
            if row == pivot_column {
                continue;
            }
            let factor = coefficients[row + pivot_column * rows];
            if factor == T::zero() {
                continue;
            }
            for column in pivot_column..rows {
                let index = row + column * rows;
                let pivot_index = pivot_column + column * rows;
                coefficients[index] = coefficients[index] - factor * coefficients[pivot_index];
            }
            for column in 0..right_columns {
                let index = row + column * rows;
                let pivot_index = pivot_column + column * rows;
                right[index] = right[index] - factor * right[pivot_index];
            }
        }
    }
    if right.iter().any(|value| !value.is_finite()) {
        return Err(ResidentKernelError::Arithmetic);
    }
    Ok(right)
}

fn matrix_solve_f64(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let [rows, right_columns] = kernel.parameters() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let rows = *rows as usize;
    let right_columns = *right_columns as usize;
    admit_matrix_solve(
        rows,
        right_columns,
        core::mem::size_of::<f64>(),
        1,
        1,
        0,
        super::budget::KernelCostEstimate::default(),
    )?;
    let next = solve_dense(
        f64_input(inputs, 0)?.to_vec(),
        f64_input(inputs, 1)?.to_vec(),
        rows,
        right_columns,
    )?;
    let output = f64_output(output)?;
    if output.len() != next.len() {
        return Err(ResidentKernelError::InvalidShape);
    }
    let changed = output
        .iter()
        .zip(&next)
        .any(|(current, next)| current.to_bits() != next.to_bits());
    output.copy_from_slice(&next);
    Ok(changed)
}

fn matrix_solve_f32_snapshot(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let [rows, right_columns] = kernel.parameters() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let (
        Some(ResidentValueRef::Snapshot([Some(coefficients)])),
        Some(ResidentValueRef::Snapshot([Some(right)])),
    ) = (inputs.get(0), inputs.get(1))
    else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let rows = *rows as usize;
    let right_columns = *right_columns as usize;
    let output_count = rows
        .checked_mul(right_columns)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let output_container_bytes = output_count
        .checked_mul(core::mem::size_of::<ValueDataDraft>())
        .ok_or(ResidentKernelError::InvalidShape)?;
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    coefficients
        .validate_against(schemas)
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    right
        .validate_against(schemas)
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    let mut meter = super::budget::ResidentBudgetMeter::default();
    super::budget::measure_canonical_value_footprint(&mut meter, coefficients, schemas)?;
    super::budget::measure_canonical_value_footprint(&mut meter, right, schemas)?;
    let previous_footprint = match &output {
        ResidentValueMut::Snapshot(values) => match values.first().and_then(|value| value.as_ref())
        {
            Some(previous) => Some(super::budget::measure_canonical_value_footprint(
                &mut meter, previous, schemas,
            )?),
            None => None,
        },
        _ => return Err(ResidentKernelError::InvalidOutput),
    };
    let metadata = kernel
        .snapshot_output()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let output_nodes = super::budget::checked_u64(output_count)?
        .checked_add(2)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let output_footprint = ValueFootprint {
        encoded_bytes: output_nodes,
        retained_bytes: super::budget::checked_u64(
            output_count
                .checked_mul(core::mem::size_of::<f32>())
                .and_then(|bytes| bytes.checked_add(output_container_bytes))
                .ok_or(ResidentKernelError::InvalidShape)?,
        )?,
        node_count: output_nodes,
    };
    let publication_work = match (&output, previous_footprint) {
        (ResidentValueMut::Snapshot(values), Some(previous_footprint)) => {
            let previous = values
                .first()
                .and_then(|value| value.as_ref())
                .ok_or(ResidentKernelError::InvalidOutput)?;
            super::budget::projected_language_equality_work(
                schemas,
                previous,
                previous_footprint,
                metadata.schema,
                metadata.shape.parameter_values().len(),
                output_footprint,
            )?
        }
        _ => 0,
    };
    let mut supplemental = meter.estimate();
    supplemental.set_comparison_work(
        supplemental
            .comparison_work()
            .checked_add(publication_work)
            .ok_or(ResidentKernelError::InvalidShape)?,
    )?;
    supplemental.set_compute_work(
        supplemental
            .compute_work()
            .checked_add(publication_work)
            .ok_or(ResidentKernelError::InvalidShape)?,
    )?;
    supplemental.set_retained_nodes(
        supplemental
            .retained_nodes()
            .checked_add(
                output_nodes
                    .checked_mul(2)
                    .ok_or(ResidentKernelError::InvalidShape)?,
            )
            .ok_or(ResidentKernelError::InvalidShape)?,
    );
    admit_matrix_solve(
        rows,
        right_columns,
        core::mem::size_of::<f32>(),
        2,
        3,
        output_container_bytes,
        supplemental,
    )?;
    let canonical_coefficients =
        f32_snapshot_values(coefficients).ok_or(ResidentKernelError::InvalidInput)?;
    let canonical_right = f32_snapshot_values(right).ok_or(ResidentKernelError::InvalidInput)?;
    let to_column_major = |values: &[f32], columns: usize| {
        (0..columns)
            .flat_map(|column| (0..rows).map(move |row| values[row * columns + column]))
            .collect::<Vec<_>>()
    };
    let next = solve_dense(
        to_column_major(&canonical_coefficients, rows),
        to_column_major(&canonical_right, right_columns),
        rows,
        right_columns,
    )?;
    let canonical_next = (0..rows)
        .flat_map(|row| {
            let next = &next;
            (0..right_columns).map(move |column| next[row + column * rows])
        })
        .collect::<Vec<_>>();
    write_snapshot_data_with_work_budget(
        kernel,
        output,
        ValueDataDraft::Matrix(
            canonical_next
                .into_iter()
                .map(|value| ValueDataDraft::F32(F32Bits::from_f32(value)))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
        Some(0),
    )
}

fn all_rows_columns(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 2 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let source = f64_input(inputs, 0)?;
    let output = f64_output(output)?;
    let rows = kernel.parameters()[0] as usize;
    let source_columns = kernel.parameters()[1] as usize;
    let selected_columns = ValidatedIndices::new(input(inputs, 1)?, source_columns)?;
    if output.len()
        != rows
            .checked_mul(selected_columns.len())
            .ok_or(ResidentKernelError::InvalidShape)?
    {
        return Err(ResidentKernelError::InvalidShape);
    }
    let mut changed = false;
    selected_columns.try_for_each_position(|ordinal, column| {
        let source = &source[column * rows..(column + 1) * rows];
        let target = &mut output[ordinal * rows..(ordinal + 1) * rows];
        changed |= target
            .iter()
            .zip(source)
            .any(|(left, right)| left.to_bits() != right.to_bits());
        target.copy_from_slice(source);
        Ok::<(), ResidentKernelError>(())
    })?;
    Ok(changed)
}

fn all_rows_column(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    all_rows_columns(kernel, inputs, output)
}

fn row_all_columns(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 2 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let source = f64_input(inputs, 0)?;
    let output = f64_output(output)?;
    let rows = kernel.parameters()[0] as usize;
    let columns = kernel.parameters()[1] as usize;
    let row = checked_one_based(index_at(inputs, 1, 0)?, rows)?;
    if output.len() != columns {
        return Err(ResidentKernelError::InvalidShape);
    }
    Ok(replace_f64(output, |column| source[row + column * rows]))
}

fn rows_all_columns(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 2 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let source = f64_input(inputs, 0)?;
    let output = f64_output(output)?;
    let rows = kernel.parameters()[0] as usize;
    let columns = kernel.parameters()[1] as usize;
    let selected_rows = ValidatedIndices::new(input(inputs, 1)?, rows)?;
    if output.len()
        != selected_rows
            .len()
            .checked_mul(columns)
            .ok_or(ResidentKernelError::InvalidShape)?
    {
        return Err(ResidentKernelError::InvalidShape);
    }
    let mut changed = false;
    let mut target_index = 0;
    for column in 0..columns {
        selected_rows.try_for_each_position(|_, row| {
            let next = source[row + column * rows];
            changed |= output[target_index].to_bits() != next.to_bits();
            output[target_index] = next;
            target_index += 1;
            Ok::<(), ResidentKernelError>(())
        })?;
    }
    Ok(changed)
}

fn checked_one_based(index: u64, upper: usize) -> Result<usize, ResidentKernelError> {
    if index == 0 || index > upper as u64 {
        return Err(ResidentKernelError::IndexOutOfRange {
            index,
            upper_bound: upper as u64,
        });
    }
    Ok(index as usize - 1)
}

fn add_indexed_rows(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    indexed_rows(kernel, inputs, output, |target, source| target + source)
}

fn sub_indexed_rows(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    indexed_rows(kernel, inputs, output, |target, source| target - source)
}

fn indexed_rows(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
    operation: impl Fn(f64, f64) -> f64,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 2 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let source_values = f64_input(inputs, 0)?;
    let output = f64_output(output)?;
    let target_rows = kernel.parameters()[0] as usize;
    let columns = kernel.parameters()[1] as usize;
    let source_rows = kernel.parameters()[2] as usize;
    if output.len()
        != target_rows
            .checked_mul(columns)
            .ok_or(ResidentKernelError::InvalidShape)?
        || source_values.len()
            != source_rows
                .checked_mul(columns)
                .ok_or(ResidentKernelError::InvalidShape)?
    {
        return Err(ResidentKernelError::InvalidShape);
    }
    let rows = ValidatedIndices::new(input(inputs, 1)?, target_rows)?;
    if rows.len() != source_rows {
        return Err(ResidentKernelError::InvalidShape);
    }
    // As with indexed assignment, the RMW alias policy applies only to the
    // hidden base input; source_values remains immutable while this plan runs.
    let mut changed = false;
    rows.try_for_each_position(|occurrence, row| {
        for column in 0..columns {
            let target = row + column * target_rows;
            let source = occurrence + column * source_rows;
            let next = operation(output[target], source_values[source]);
            changed |= next.to_bits() != output[target].to_bits();
            output[target] = next;
        }
        Ok::<(), ResidentKernelError>(())
    })?;
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Inputs<'a>(&'a [ResidentValueRef<'a>]);

    impl ResidentKernelInputs for Inputs<'_> {
        fn len(&self) -> usize {
            self.0.len()
        }

        fn get(&self, index: usize) -> Option<ResidentValueRef<'_>> {
            self.0.get(index).copied()
        }
    }

    fn test_schema(body: SchemaBody) -> mech_core::Schema {
        mech_core::SchemaDraft {
            dimension_parameters: Box::new([]),
            body,
        }
        .finalize()
        .unwrap()
    }

    fn test_schema_table(
        bodies: impl IntoIterator<Item = SchemaBody>,
    ) -> (mech_core::SchemaTable, Vec<SchemaId>) {
        let mut builder = mech_core::SchemaTableBuilder::new();
        let handles = bodies
            .into_iter()
            .map(|body| builder.insert(test_schema(body)).unwrap())
            .collect::<Vec<_>>();
        let build = builder.finish().unwrap();
        let ids = handles
            .into_iter()
            .map(|handle| build.resolve(handle).unwrap())
            .collect::<Vec<_>>();
        let (schemas, _) = build.into_parts();
        (schemas, ids)
    }

    fn test_layout(
        schemas: &mech_core::SchemaTable,
        schema: SchemaId,
        kind: ResidentValueKind,
        shape: ResidentShape,
    ) -> mech_core::ResidentPortLayout {
        mech_core::ResidentPortLayout {
            schema_id: schema,
            schema_key: schemas.entry(schema).unwrap().key(),
            kind,
            shape,
            shape_instance: schemas
                .get(schema)
                .unwrap()
                .instantiate_shape(Box::new([]))
                .unwrap(),
            resolved_selector: None,
        }
    }

    #[test]
    fn dense_transpose_execution_admits_the_complete_output() {
        let maximum = super::super::budget::MAX_RESIDENT_OUTPUT_ELEMENTS as u32;
        for kind in [
            ResidentValueKind::Bool,
            ResidentValueKind::Index,
            ResidentValueKind::F64,
        ] {
            assert!(
                admit_dense_transpose_layout(
                    kind,
                    ResidentShape {
                        rows: 1,
                        columns: maximum,
                    },
                )
                .is_ok()
            );
            assert!(
                admit_dense_transpose_layout(
                    kind,
                    ResidentShape {
                        rows: 1,
                        columns: maximum + 1,
                    },
                )
                .is_err()
            );
        }
    }

    #[test]
    fn dense_comparison_execution_admits_the_complete_output() {
        let maximum = super::super::budget::MAX_RESIDENT_OUTPUT_ELEMENTS as u32;
        assert!(
            admit_dense_comparison_layout(ResidentShape {
                rows: 1,
                columns: maximum,
            })
            .is_ok()
        );
        assert!(
            admit_dense_comparison_layout(ResidentShape {
                rows: 1,
                columns: maximum + 1,
            })
            .is_err()
        );
    }

    #[test]
    fn dense_matrix_solve_binding_does_not_manufacture_an_execution_permit() {
        let columns = super::super::budget::MAX_RESIDENT_OUTPUT_ELEMENTS + 1;
        let coefficient_body = SchemaBody::Matrix {
            element: Box::new(SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)),
            dimensions: vec![
                mech_core::DimensionExpr::Constant(1),
                mech_core::DimensionExpr::Constant(1),
            ]
            .into_boxed_slice(),
        };
        let right_body = SchemaBody::Matrix {
            element: Box::new(SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)),
            dimensions: vec![
                mech_core::DimensionExpr::Constant(1),
                mech_core::DimensionExpr::Constant(columns),
            ]
            .into_boxed_slice(),
        };
        let (schemas, ids) = test_schema_table([coefficient_body, right_body]);
        let [coefficient_schema, right_schema] = ids.as_slice() else {
            unreachable!()
        };
        let contract = test_contract(
            &[*coefficient_schema, *right_schema],
            *right_schema,
            OutputConstruction::FullWrite {
                shape: ShapeRule::SameAsInput { input: 1 },
            },
            AccessMode::Write,
            AliasPolicy::NoAlias,
            ChangeDetectionPolicy::KernelReported,
        );
        let coefficient = test_layout(
            &schemas,
            *coefficient_schema,
            ResidentValueKind::F64,
            ResidentShape::SCALAR,
        );
        let right = test_layout(
            &schemas,
            *right_schema,
            ResidentValueKind::F64,
            ResidentShape {
                rows: 1,
                columns: u32::try_from(columns).unwrap(),
            },
        );

        assert!(
            bind_matrix_solve(&ResidentKernelBindRequest {
                contract: &contract,
                schemas: &schemas,
                inputs: &[coefficient, right.clone()],
                output: right,
            })
            .is_ok()
        );
    }

    #[test]
    fn snapshot_combinations_advance_without_recursion() {
        let mut selected = vec![0, 1];
        let mut combinations = vec![selected.clone()];
        while advance_combination_indices(&mut selected, 4) {
            combinations.push(selected.clone());
        }
        assert_eq!(
            combinations,
            vec![
                vec![0, 1],
                vec![0, 2],
                vec![0, 3],
                vec![1, 2],
                vec![1, 3],
                vec![2, 3],
            ]
        );

        let mut deep = (0..30_000).collect::<Vec<_>>();
        assert!(!advance_combination_indices(&mut deep, 30_000));
    }

    fn test_contract(
        inputs: &[SchemaId],
        output: SchemaId,
        construction: OutputConstruction,
        output_access: AccessMode,
        alias: AliasPolicy,
        change_detection: ChangeDetectionPolicy,
    ) -> ResolvedOperationContract {
        ResolvedOperationContract::Declared(mech_core::DeclaredOperationContract {
            inputs: inputs
                .iter()
                .copied()
                .map(|schema| mech_core::ResolvedInputPort {
                    schema,
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            outputs: vec![mech_core::ResolvedOutputPort {
                schema: output,
                access: output_access,
                delivery: DeliveryMode::Signal,
                construction,
                alias,
                change_detection,
            }]
            .into_boxed_slice(),
            interaction: ExternalInteraction::Pure,
        })
    }

    fn test_value(
        schemas: &mech_core::SchemaTable,
        schema: SchemaId,
        data: ValueDataDraft,
    ) -> mech_core::Value {
        ValueDraft {
            schema,
            shape_values: Box::new([]),
            data,
        }
        .finalize(&SnapshotValidationContext::new(schemas))
        .unwrap()
    }

    #[test]
    fn resident_unary_math_supports_abs_floor_and_sqrt_vectors() {
        let cases: &[(mech_core::ResidentKernelExecutor, &[f64], &[f64])] = &[
            (absolute, &[-2.5, 0.0, 3.25], &[2.5, 0.0, 3.25]),
            (floor, &[-1.2, 0.0, 3.9], &[-2.0, 0.0, 3.0]),
            (square_root, &[0.0, 4.0, 9.0], &[0.0, 2.0, 3.0]),
        ];
        for (executor, input, expected) in cases {
            let values = [ResidentValueRef::F64(input)];
            let kernel = BoundResidentKernel::new(*executor, Box::new([]));
            let mut output = [f64::NAN; 3];
            assert_eq!(
                kernel.execute(&Inputs(&values), ResidentValueMut::F64(&mut output)),
                Ok(true),
            );
            assert_eq!(&output, expected);
        }
    }

    #[test]
    fn resident_range_catalog_uses_only_canonical_operation_identities() {
        let mut builder = FunctionCatalogBuilder::new();
        install(&mut builder).unwrap();
        let catalog = builder.build().unwrap();
        for (module, operation) in [
            (["range"].as_slice(), "exclusive"),
            (["range"].as_slice(), "exclusive-increment"),
            (["range"].as_slice(), "inclusive"),
            (["range"].as_slice(), "inclusive-increment"),
        ] {
            let module = module
                .iter()
                .map(|segment| (*segment).to_owned())
                .collect::<Vec<_>>();
            assert!(
                catalog.resident_factory(&module, operation).is_some(),
                "missing canonical resident operation {}/{}",
                module.join("/"),
                operation,
            );
        }
    }

    #[test]
    fn statistical_reduction_resident_kernels_cover_both_axes() {
        let mut builder = FunctionCatalogBuilder::new();
        install(&mut builder).unwrap();
        let catalog = builder.build().unwrap();
        assert!(
            catalog
                .resident_factory(&["stats".to_owned(), "sum".to_owned()], "column")
                .is_some()
        );
        assert!(
            catalog
                .resident_factory(&["stats".to_owned(), "sum".to_owned()], "row")
                .is_some()
        );

        // Resident matrix storage is column-major: [1 2; 3 4] is [1, 3, 2, 4].
        let matrix = [1.0, 3.0, 2.0, 4.0];
        let inputs = [ResidentValueRef::F64(&matrix)];
        let columns = BoundResidentKernel::new(sum_columns, Box::new([2, 2]));
        let rows = BoundResidentKernel::new(sum_rows, Box::new([2, 2]));
        let mut column_output = [0.0; 2];
        let mut row_output = [0.0; 2];

        assert_eq!(
            columns.execute(&Inputs(&inputs), ResidentValueMut::F64(&mut column_output)),
            Ok(true)
        );
        assert_eq!(
            rows.execute(&Inputs(&inputs), ResidentValueMut::F64(&mut row_output)),
            Ok(true)
        );
        assert_eq!(column_output, [3.0, 7.0]);
        assert_eq!(row_output, [4.0, 6.0]);
    }

    #[test]
    fn binary_arithmetic_broadcasts_rows_columns_and_outer_vectors() {
        let matrix = [1.0, 4.0, 2.0, 5.0, 3.0, 6.0];
        let row = [10.0, 20.0, 30.0];
        let column = [10.0, 100.0];

        let row_inputs = [ResidentValueRef::F64(&matrix), ResidentValueRef::F64(&row)];
        let row_kernel = BoundResidentKernel::new(
            add,
            Box::new([2, 3, BINARY_BROADCAST_EXACT, BINARY_BROADCAST_ROW]),
        );
        let mut row_output = [0.0; 6];
        assert_eq!(
            row_kernel.execute(&Inputs(&row_inputs), ResidentValueMut::F64(&mut row_output),),
            Ok(true),
        );
        assert_eq!(row_output, [11.0, 14.0, 22.0, 25.0, 33.0, 36.0]);

        let column_inputs = [
            ResidentValueRef::F64(&column),
            ResidentValueRef::F64(&matrix),
        ];
        let column_kernel = BoundResidentKernel::new(
            subtract,
            Box::new([2, 3, BINARY_BROADCAST_COLUMN, BINARY_BROADCAST_EXACT]),
        );
        let mut column_output = [0.0; 6];
        assert_eq!(
            column_kernel.execute(
                &Inputs(&column_inputs),
                ResidentValueMut::F64(&mut column_output),
            ),
            Ok(true),
        );
        assert_eq!(column_output, [9.0, 96.0, 8.0, 95.0, 7.0, 94.0]);

        let outer_inputs = [ResidentValueRef::F64(&column), ResidentValueRef::F64(&row)];
        let outer_kernel = BoundResidentKernel::new(
            add,
            Box::new([2, 3, BINARY_BROADCAST_COLUMN, BINARY_BROADCAST_ROW]),
        );
        let mut outer_output = [0.0; 6];
        assert_eq!(
            outer_kernel.execute(
                &Inputs(&outer_inputs),
                ResidentValueMut::F64(&mut outer_output),
            ),
            Ok(true),
        );
        assert_eq!(outer_output, [20.0, 110.0, 30.0, 120.0, 40.0, 130.0]);

        let empty_inputs = [ResidentValueRef::F64(&[]), ResidentValueRef::F64(&row)];
        let empty_kernel = BoundResidentKernel::new(
            add,
            Box::new([0, 3, BINARY_BROADCAST_EXACT, BINARY_BROADCAST_ROW]),
        );
        let mut empty_output = [];
        assert_eq!(
            empty_kernel.execute(
                &Inputs(&empty_inputs),
                ResidentValueMut::F64(&mut empty_output),
            ),
            Ok(false),
        );
    }

    #[test]
    fn binary_broadcast_layout_selection_preserves_vector_orientation() {
        let output = ResidentShape {
            rows: 2,
            columns: 3,
        };
        assert_eq!(
            binary_broadcast_mode(
                ResidentShape {
                    rows: 2,
                    columns: 3,
                },
                output,
            ),
            Some(BINARY_BROADCAST_EXACT),
        );
        assert_eq!(
            binary_broadcast_mode(
                ResidentShape {
                    rows: 2,
                    columns: 1,
                },
                output,
            ),
            Some(BINARY_BROADCAST_COLUMN),
        );
        assert_eq!(
            binary_broadcast_mode(
                ResidentShape {
                    rows: 1,
                    columns: 3,
                },
                output,
            ),
            Some(BINARY_BROADCAST_ROW),
        );
        assert_eq!(
            binary_broadcast_mode(
                ResidentShape {
                    rows: 1,
                    columns: 2,
                },
                output,
            ),
            None,
        );
    }

    #[test]
    fn range_binders_require_scalar_f64_inputs_and_row_matrix_f64_outputs() {
        fn schema(body: SchemaBody) -> mech_core::Schema {
            mech_core::SchemaDraft {
                dimension_parameters: Box::new([]),
                body,
            }
            .finalize()
            .unwrap()
        }

        fn contract(
            inputs: &[mech_core::SchemaId],
            output: mech_core::SchemaId,
            postcondition: &str,
        ) -> ResolvedOperationContract {
            ResolvedOperationContract::Declared(mech_core::DeclaredOperationContract {
                inputs: inputs
                    .iter()
                    .map(|schema| mech_core::ResolvedInputPort {
                        schema: *schema,
                        access: AccessMode::Read,
                        delivery: DeliveryMode::Signal,
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
                outputs: vec![mech_core::ResolvedOutputPort {
                    schema: output,
                    access: AccessMode::Write,
                    delivery: DeliveryMode::Signal,
                    construction: OutputConstruction::Build {
                        postcondition: ShapeContractReference {
                            module_path: vec!["range".to_owned()].into_boxed_slice(),
                            contract_name: postcondition.to_owned(),
                        },
                    },
                    alias: AliasPolicy::NoAlias,
                    change_detection: ChangeDetectionPolicy::KernelReported,
                }]
                .into_boxed_slice(),
                interaction: ExternalInteraction::Pure,
            })
        }

        let f64_body = SchemaBody::FloatingPoint(mech_core::FloatWidth::W64);
        let matrix = |rows, columns| SchemaBody::Matrix {
            element: Box::new(f64_body.clone()),
            dimensions: vec![
                mech_core::DimensionExpr::Constant(rows),
                mech_core::DimensionExpr::Constant(columns),
            ]
            .into_boxed_slice(),
        };
        let mut builder = mech_core::SchemaTableBuilder::new();
        let scalar_handle = builder.insert(schema(f64_body.clone())).unwrap();
        let matrix_one_handle = builder.insert(schema(matrix(1, 1))).unwrap();
        let row_handle = builder.insert(schema(matrix(1, 4))).unwrap();
        let oversized_columns = super::super::budget::MAX_RESIDENT_OUTPUT_ELEMENTS + 1;
        let oversized_handle = builder
            .insert(schema(matrix(1, oversized_columns)))
            .unwrap();
        let non_row_handle = builder.insert(schema(matrix(2, 2))).unwrap();
        let build = builder.finish().unwrap();
        let scalar = build.resolve(scalar_handle).unwrap();
        let matrix_one = build.resolve(matrix_one_handle).unwrap();
        let row = build.resolve(row_handle).unwrap();
        let oversized = build.resolve(oversized_handle).unwrap();
        let non_row = build.resolve(non_row_handle).unwrap();
        let (schemas, _) = build.into_parts();
        let port = |schema_id, shape| mech_core::ResidentPortLayout {
            schema_id,
            schema_key: schemas.entry(schema_id).unwrap().key(),
            kind: ResidentValueKind::F64,
            shape,
            shape_instance: schemas
                .get(schema_id)
                .unwrap()
                .instantiate_shape(Box::new([]))
                .unwrap(),
            resolved_selector: None,
        };
        let row_shape = ResidentShape {
            rows: 1,
            columns: 4,
        };
        let non_row_shape = ResidentShape {
            rows: 2,
            columns: 2,
        };
        let oversized_shape = ResidentShape {
            rows: 1,
            columns: u32::try_from(oversized_columns).unwrap(),
        };

        let families: [(mech_core::ResidentKernelFactory, usize, &str); 4] = [
            (bind_range_exclusive, 2, "exclusive-output"),
            (
                bind_range_increment_exclusive,
                3,
                "exclusive-increment-output",
            ),
            (bind_range_inclusive, 2, "inclusive-output"),
            (
                bind_range_increment_inclusive,
                3,
                "inclusive-increment-output",
            ),
        ];
        for (binder, arity, postcondition) in families {
            let valid_inputs = vec![port(scalar, ResidentShape::SCALAR); arity];
            let valid_contract = contract(&vec![scalar; arity], row, postcondition);
            assert!(
                binder(&ResidentKernelBindRequest {
                    contract: &valid_contract,
                    schemas: &schemas,
                    inputs: &valid_inputs,
                    output: port(row, row_shape),
                })
                .is_ok(),
                "valid {postcondition} layout was rejected"
            );

            let invalid_inputs = vec![port(matrix_one, ResidentShape::SCALAR); arity];
            let invalid_contract = contract(&vec![matrix_one; arity], row, postcondition);
            assert!(
                matches!(
                    binder(&ResidentKernelBindRequest {
                        contract: &invalid_contract,
                        schemas: &schemas,
                        inputs: &invalid_inputs,
                        output: port(row, row_shape),
                    }),
                    Err(ResidentKernelBindError::UnsupportedLayout)
                ),
                "matrix schemas were accepted as scalar range inputs"
            );

            let invalid_contract = contract(&vec![scalar; arity], scalar, postcondition);
            assert!(
                matches!(
                    binder(&ResidentKernelBindRequest {
                        contract: &invalid_contract,
                        schemas: &schemas,
                        inputs: &valid_inputs,
                        output: port(scalar, row_shape),
                    }),
                    Err(ResidentKernelBindError::UnsupportedLayout)
                ),
                "a scalar schema was accepted as a range matrix output"
            );

            let invalid_contract = contract(&vec![scalar; arity], non_row, postcondition);
            assert!(
                matches!(
                    binder(&ResidentKernelBindRequest {
                        contract: &invalid_contract,
                        schemas: &schemas,
                        inputs: &valid_inputs,
                        output: port(non_row, non_row_shape),
                    }),
                    Err(ResidentKernelBindError::UnsupportedLayout)
                ),
                "a non-row matrix was accepted as a range output"
            );

            let oversized_contract = contract(&vec![scalar; arity], oversized, postcondition);
            assert!(
                binder(&ResidentKernelBindRequest {
                    contract: &oversized_contract,
                    schemas: &schemas,
                    inputs: &valid_inputs,
                    output: port(oversized, oversized_shape),
                })
                .is_ok(),
                "the semantic binder tried to manufacture an execution permit for {postcondition}"
            );
        }
    }

    fn indexed_kernel(
        executor: mech_core::ResidentKernelExecutor,
        rows: u64,
        columns: u64,
        source_rows: u64,
        index_count: u64,
    ) -> BoundResidentKernel {
        BoundResidentKernel::new(
            executor,
            [rows, columns, source_rows, columns, index_count]
                .into_iter()
                .collect::<Box<[_]>>(),
        )
    }

    #[test]
    fn duplicate_indexed_rows_accumulate_once_per_source_occurrence() {
        let kernel = indexed_kernel(add_indexed_rows, 2, 2, 3, 3);
        let source = [0.5, 1.0, 2.0, 5.0, 10.0, 20.0];
        let indices = [1_u64, 1, 2];
        let inputs = [
            ResidentValueRef::F64(&source),
            ResidentValueRef::Index(&indices),
        ];
        let mut target = [1.0, 2.0, 10.0, 20.0];

        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::F64(&mut target)),
            Ok(true)
        );
        assert_eq!(target, [2.5, 4.0, 25.0, 40.0]);
    }

    #[test]
    fn late_out_of_range_index_rejects_before_indexed_row_mutation() {
        let kernel = indexed_kernel(sub_indexed_rows, 2, 1, 2, 2);
        let source = [1.0, 2.0];
        let indices = [1_u64, 3];
        let inputs = [
            ResidentValueRef::F64(&source),
            ResidentValueRef::Index(&indices),
        ];
        let mut candidate = [10.0, 20.0];

        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::F64(&mut candidate)),
            Err(ResidentKernelError::IndexOutOfRange {
                index: 3,
                upper_bound: 2,
            })
        );
        assert_eq!(candidate, [10.0, 20.0]);
    }

    #[test]
    fn late_out_of_range_gather_index_rejects_before_output_mutation() {
        let kernel = BoundResidentKernel::new(gather_1d, Box::new([]));
        let source = [10.0, 20.0];
        let indices = [1_u64, 3];
        let inputs = [
            ResidentValueRef::F64(&source),
            ResidentValueRef::Index(&indices),
        ];
        let mut output = [90.0, 80.0];

        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::F64(&mut output)),
            Err(ResidentKernelError::IndexOutOfRange {
                index: 3,
                upper_bound: 2,
            })
        );
        assert_eq!(output, [90.0, 80.0]);
    }

    #[test]
    fn late_out_of_range_row_selector_rejects_before_output_mutation() {
        let kernel = BoundResidentKernel::new(rows_all_columns, Box::new([2, 2]));
        let source = [10.0, 20.0, 30.0, 40.0];
        let indices = [1_u64, 3];
        let inputs = [
            ResidentValueRef::F64(&source),
            ResidentValueRef::Index(&indices),
        ];
        let mut output = [90.0, 80.0, 70.0, 60.0];

        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::F64(&mut output)),
            Err(ResidentKernelError::IndexOutOfRange {
                index: 3,
                upper_bound: 2,
            })
        );
        assert_eq!(output, [90.0, 80.0, 70.0, 60.0]);
    }

    #[test]
    fn indexed_assignment_capability_supports_maps_but_rejects_nested_matrix_members() {
        fn schema(body: SchemaBody) -> mech_core::Schema {
            mech_core::SchemaDraft {
                dimension_parameters: Box::new([]),
                body,
            }
            .finalize()
            .unwrap()
        }
        let mut builder = mech_core::SchemaTableBuilder::new();
        let map = builder
            .insert(schema(SchemaBody::Map {
                key: Box::new(SchemaBody::Index),
                value: Box::new(SchemaBody::Tuple(vec![SchemaBody::Bool].into_boxed_slice())),
                cardinality: mech_core::CardinalitySpec::Dynamic { upper_bound: None },
            }))
            .unwrap();
        let tuple_body = SchemaBody::Tuple(vec![SchemaBody::Bool].into_boxed_slice());
        let tuple = builder.insert(schema(tuple_body.clone())).unwrap();
        let tuple_matrix = builder
            .insert(schema(SchemaBody::Matrix {
                element: Box::new(tuple_body),
                dimensions: vec![
                    mech_core::DimensionExpr::Constant(2),
                    mech_core::DimensionExpr::Constant(1),
                ]
                .into_boxed_slice(),
            }))
            .unwrap();
        let index = builder.insert(schema(SchemaBody::Index)).unwrap();
        let build = builder.finish().unwrap();
        let map = build.resolve(map).unwrap();
        let tuple = build.resolve(tuple).unwrap();
        let tuple_matrix = build.resolve(tuple_matrix).unwrap();
        let index = build.resolve(index).unwrap();
        let (schemas, _) = build.into_parts();
        let port = |schema_id, kind| mech_core::ResidentPortLayout {
            schema_id,
            schema_key: schemas.entry(schema_id).unwrap().key(),
            kind,
            shape: ResidentShape::SCALAR,
            shape_instance: schemas
                .get(schema_id)
                .unwrap()
                .instantiate_shape(Box::new([]))
                .unwrap(),
            resolved_selector: None,
        };
        let contract = ResolvedOperationContract::Declared(mech_core::DeclaredOperationContract {
            inputs: [map, tuple, index]
                .into_iter()
                .map(|schema| mech_core::ResolvedInputPort {
                    schema,
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            outputs: vec![mech_core::ResolvedOutputPort {
                schema: map,
                access: AccessMode::ReadWrite,
                delivery: DeliveryMode::Signal,
                construction: OutputConstruction::ReadModifyWrite {
                    base_input: 0,
                    regions: RegionPolicy::IndexedAxis { axis: 0 },
                },
                alias: AliasPolicy::MayAlias { input: 0 },
                change_detection: ChangeDetectionPolicy::KernelReported,
            }]
            .into_boxed_slice(),
            interaction: ExternalInteraction::Pure,
        });
        assert!(
            bind_indexed_assign(&ResidentKernelBindRequest {
                contract: &contract,
                schemas: &schemas,
                inputs: &[
                    port(map, ResidentValueKind::Snapshot),
                    port(tuple, ResidentValueKind::Snapshot),
                    port(index, ResidentValueKind::Index),
                ],
                output: port(map, ResidentValueKind::Snapshot),
            })
            .is_ok()
        );

        let matrix_contract =
            ResolvedOperationContract::Declared(mech_core::DeclaredOperationContract {
                inputs: [tuple_matrix, tuple, index]
                    .into_iter()
                    .map(|schema| mech_core::ResolvedInputPort {
                        schema,
                        access: AccessMode::Read,
                        delivery: DeliveryMode::Signal,
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
                outputs: vec![mech_core::ResolvedOutputPort {
                    schema: tuple_matrix,
                    access: AccessMode::ReadWrite,
                    delivery: DeliveryMode::Signal,
                    construction: OutputConstruction::ReadModifyWrite {
                        base_input: 0,
                        regions: RegionPolicy::IndexedAxis { axis: 0 },
                    },
                    alias: AliasPolicy::MayAlias { input: 0 },
                    change_detection: ChangeDetectionPolicy::KernelReported,
                }]
                .into_boxed_slice(),
                interaction: ExternalInteraction::Pure,
            });
        assert!(matches!(
            bind_indexed_assign(&ResidentKernelBindRequest {
                contract: &matrix_contract,
                schemas: &schemas,
                inputs: &[
                    port(tuple_matrix, ResidentValueKind::Snapshot),
                    port(tuple, ResidentValueKind::Snapshot),
                    port(index, ResidentValueKind::Index),
                ],
                output: port(tuple_matrix, ResidentValueKind::Snapshot),
            }),
            Err(ResidentKernelBindError::UnsupportedLayout)
        ));
    }

    #[test]
    fn boolean_mask_assignment_routes_vector_sources_by_physical_position() {
        let kernel = BoundResidentKernel::new(
            indexed_assign,
            Box::new([3, ResolvedSourceRouting::Positional as u64]),
        );
        let source = [10.0, 20.0];
        let selector = [0_u8, 1, 0];
        let inputs = [
            ResidentValueRef::F64(&source),
            ResidentValueRef::Bool(&selector),
        ];
        let mut output = [1.0, 2.0, 3.0];
        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::F64(&mut output)),
            Ok(true),
        );
        assert_eq!(output, [1.0, 20.0, 3.0]);

        let selector = [0_u8, 0, 1];
        let inputs = [
            ResidentValueRef::F64(&source),
            ResidentValueRef::Bool(&selector),
        ];
        let previous = output;
        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::F64(&mut output)),
            Err(ResidentKernelError::InvalidShape),
        );
        assert_eq!(output, previous);
    }

    #[test]
    fn indexed_assignment_uses_the_resolved_routing_when_lengths_are_ambiguous() {
        let source = [10.0, 20.0, 30.0];
        let selector = [2_u64, 1, 3];
        let inputs = [
            ResidentValueRef::F64(&source),
            ResidentValueRef::Index(&selector),
        ];

        let compact = BoundResidentKernel::new(
            indexed_assign,
            Box::new([3, ResolvedSourceRouting::CompactSelectionOrder as u64]),
        );
        let mut compact_output = [0.0; 3];
        compact
            .execute(&Inputs(&inputs), ResidentValueMut::F64(&mut compact_output))
            .unwrap();
        assert_eq!(compact_output, [20.0, 10.0, 30.0]);

        let positional = BoundResidentKernel::new(
            indexed_assign,
            Box::new([3, ResolvedSourceRouting::Positional as u64]),
        );
        let mut positional_output = [0.0; 3];
        positional
            .execute(
                &Inputs(&inputs),
                ResidentValueMut::F64(&mut positional_output),
            )
            .unwrap();
        assert_eq!(positional_output, [10.0, 20.0, 30.0]);
    }

    #[test]
    fn indexed_boolean_assignment_rejects_first_middle_and_last_invalid_values_atomically() {
        let selector = [1_u64, 2, 3];
        let kernel = BoundResidentKernel::new(
            indexed_assign,
            Box::new([3, ResolvedSourceRouting::CompactSelectionOrder as u64]),
        );

        for invalid in 0..3 {
            let mut source = [1_u8, 0, 1];
            source[invalid] = 2;
            let inputs = [
                ResidentValueRef::Bool(&source),
                ResidentValueRef::Index(&selector),
            ];
            let mut output = [0_u8, 1, 0];
            let previous = output;
            assert_eq!(
                kernel.execute(&Inputs(&inputs), ResidentValueMut::Bool(&mut output)),
                Err(ResidentKernelError::InvalidInput),
            );
            assert_eq!(output, previous);
        }
    }

    #[test]
    fn linear_string_broadcast_charges_every_clone_before_publication() {
        let selector = [1_u64, 2];
        let source = ["x".repeat(9 * 1024 * 1024)];
        let inputs = [
            ResidentValueRef::String(&source),
            ResidentValueRef::Index(&selector),
        ];
        let kernel = BoundResidentKernel::new(
            indexed_assign,
            Box::new([2, ResolvedSourceRouting::ScalarBroadcast as u64]),
        );
        let mut output = ["left".to_owned(), "right".to_owned()];
        let previous = output.clone();

        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::String(&mut output)),
            Err(ResidentKernelError::InvalidShape),
        );
        assert_eq!(output, previous);
    }

    #[test]
    fn duplicate_string_destinations_are_normalized_once_with_last_write_wins() {
        const COUNT: usize = 4_000;
        let selector = vec![1_u64; COUNT];
        let source = (0..COUNT)
            .map(|index| format!("value-{index}"))
            .collect::<Vec<_>>();
        let inputs = [
            ResidentValueRef::String(&source),
            ResidentValueRef::Index(&selector),
        ];
        let kernel = BoundResidentKernel::new(
            indexed_assign,
            Box::new([
                COUNT as u64,
                ResolvedSourceRouting::CompactSelectionOrder as u64,
            ]),
        );
        let mut output = (0..COUNT)
            .map(|index| format!("old-{index}"))
            .collect::<Vec<_>>();

        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::String(&mut output)),
            Ok(true),
        );
        assert_eq!(output[0], "value-3999");
        assert_eq!(output[1], "old-1");
        assert_eq!(output[COUNT - 1], format!("old-{}", COUNT - 1));
    }

    #[test]
    fn scalar_boolean_kernels_cover_the_retained_logic_prelude() {
        let binary_cases = [
            (bool_and as mech_core::ResidentKernelExecutor, [1, 0], false),
            (bool_or as mech_core::ResidentKernelExecutor, [1, 0], true),
            (bool_xor as mech_core::ResidentKernelExecutor, [1, 1], false),
        ];
        for (executor, values, expected) in binary_cases {
            let kernel = BoundResidentKernel::new(executor, Box::new([]));
            let inputs = [
                ResidentValueRef::Bool(&values[..1]),
                ResidentValueRef::Bool(&values[1..]),
            ];
            let mut output = [u8::from(!expected)];
            assert_eq!(
                kernel.execute(&Inputs(&inputs), ResidentValueMut::Bool(&mut output)),
                Ok(true)
            );
            assert_eq!(output, [u8::from(expected)]);
        }

        let input = [1];
        let inputs = [ResidentValueRef::Bool(&input)];
        let mut output = [1];
        let kernel = BoundResidentKernel::new(bool_not, Box::new([]));
        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::Bool(&mut output)),
            Ok(true)
        );
        assert_eq!(output, [0]);
    }

    #[test]
    fn scalar_boolean_kernels_reject_noncanonical_values() {
        let left = [2];
        let right = [1];
        let inputs = [
            ResidentValueRef::Bool(&left),
            ResidentValueRef::Bool(&right),
        ];
        let mut output = [0];
        let kernel = BoundResidentKernel::new(bool_and, Box::new([]));
        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::Bool(&mut output)),
            Err(ResidentKernelError::InvalidInput)
        );

        let matrix = [1, 2, 0, 1];
        let scalar = [1];
        let inputs = [
            ResidentValueRef::Bool(&matrix),
            ResidentValueRef::Bool(&scalar),
        ];
        let mut output = [0; 4];
        let kernel = BoundResidentKernel::new(
            bool_vector_or,
            Box::new([2, 2, BINARY_BROADCAST_EXACT, BINARY_BROADCAST_SCALAR]),
        );
        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::Bool(&mut output)),
            Err(ResidentKernelError::InvalidInput)
        );
    }

    #[test]
    fn boolean_matrix_kernels_share_broadcast_and_change_semantics() {
        let matrix = [1, 0, 0, 1, 1, 0];
        let row = [0, 1, 0];
        let inputs = [
            ResidentValueRef::Bool(&matrix),
            ResidentValueRef::Bool(&row),
        ];
        for (executor, expected) in [
            (
                bool_vector_and as mech_core::ResidentKernelExecutor,
                [0, 0, 0, 1, 0, 0],
            ),
            (
                bool_vector_or as mech_core::ResidentKernelExecutor,
                [1, 0, 1, 1, 1, 0],
            ),
            (
                bool_vector_xor as mech_core::ResidentKernelExecutor,
                [1, 0, 1, 0, 1, 0],
            ),
        ] {
            let kernel = BoundResidentKernel::new(
                executor,
                Box::new([2, 3, BINARY_BROADCAST_EXACT, BINARY_BROADCAST_ROW]),
            );
            let mut output = [0; 6];
            assert_eq!(
                kernel.execute(&Inputs(&inputs), ResidentValueMut::Bool(&mut output)),
                Ok(expected.iter().any(|value| *value != 0)),
            );
            assert_eq!(output, expected);
        }
    }

    #[test]
    fn boolean_matrix_not_preserves_shape_and_rejects_noncanonical_input_atomically() {
        let kernel = BoundResidentKernel::new(bool_vector_not, Box::new([]));
        let input = [1_u8, 0, 0, 1];
        let inputs = [ResidentValueRef::Bool(&input)];
        let mut output = [1_u8; 4];
        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::Bool(&mut output)),
            Ok(true),
        );
        assert_eq!(output, [0, 1, 1, 0]);

        let invalid = [1_u8, 2, 0, 1];
        let inputs = [ResidentValueRef::Bool(&invalid)];
        let previous = output;
        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::Bool(&mut output)),
            Err(ResidentKernelError::InvalidInput),
        );
        assert_eq!(output, previous);
    }

    #[test]
    fn one_by_one_boolean_matrix_not_uses_matrix_change_contract() {
        let mut builder = mech_core::SchemaTableBuilder::new();
        let handle = builder
            .insert(
                mech_core::SchemaDraft {
                    dimension_parameters: Box::new([]),
                    body: SchemaBody::Matrix {
                        element: Box::new(SchemaBody::Bool),
                        dimensions: vec![
                            mech_core::DimensionExpr::Constant(1),
                            mech_core::DimensionExpr::Constant(1),
                        ]
                        .into_boxed_slice(),
                    },
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let schema = build.resolve(handle).unwrap();
        let (schemas, _) = build.into_parts();
        let port = mech_core::ResidentPortLayout {
            schema_id: schema,
            schema_key: schemas.entry(schema).unwrap().key(),
            kind: ResidentValueKind::Bool,
            shape: ResidentShape::SCALAR,
            shape_instance: schemas
                .get(schema)
                .unwrap()
                .instantiate_shape(Box::new([]))
                .unwrap(),
            resolved_selector: None,
        };
        let contract = ResolvedOperationContract::Declared(mech_core::DeclaredOperationContract {
            inputs: vec![mech_core::ResolvedInputPort {
                schema,
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            }]
            .into_boxed_slice(),
            outputs: vec![mech_core::ResolvedOutputPort {
                schema,
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
        });
        let kernel = bind_bool_not(&ResidentKernelBindRequest {
            contract: &contract,
            schemas: &schemas,
            inputs: &[port.clone()],
            output: port,
        })
        .unwrap();
        let input = [1_u8];
        let inputs = [ResidentValueRef::Bool(&input)];
        let mut output = [1_u8];
        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::Bool(&mut output)),
            Ok(true),
        );
        assert_eq!(output, [0]);
    }

    #[test]
    fn dense_string_matrix_inequality_uses_declared_broadcast_layout() {
        let left = [
            "a".to_owned(),
            "b".to_owned(),
            "c".to_owned(),
            "b".to_owned(),
        ];
        let right = ["a".to_owned(), "b".to_owned()];
        let inputs = [
            ResidentValueRef::String(&left),
            ResidentValueRef::String(&right),
        ];
        let kernel = BoundResidentKernel::new(
            dense_comparison,
            Box::new([
                2,
                2,
                BINARY_BROADCAST_EXACT,
                BINARY_BROADCAST_ROW,
                SemanticComparison::NotEqual as u64,
            ]),
        );
        let mut output = [0_u8; 4];
        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::Bool(&mut output)),
            Ok(true),
        );
        assert_eq!(output, [0, 1, 1, 0]);
    }

    #[test]
    fn snapshot_u64_matrix_ordering_broadcasts_into_column_major_output() {
        let mut builder = mech_core::SchemaTableBuilder::new();
        let scalar_handle = builder
            .insert(
                mech_core::SchemaDraft {
                    dimension_parameters: Box::new([]),
                    body: SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let matrix_handle = builder
            .insert(
                mech_core::SchemaDraft {
                    dimension_parameters: Box::new([]),
                    body: SchemaBody::Matrix {
                        element: Box::new(SchemaBody::UnsignedInteger(
                            mech_core::IntegerWidth::W64,
                        )),
                        dimensions: vec![
                            mech_core::DimensionExpr::Constant(2),
                            mech_core::DimensionExpr::Constant(2),
                        ]
                        .into_boxed_slice(),
                    },
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let output_handle = builder
            .insert(
                mech_core::SchemaDraft {
                    dimension_parameters: Box::new([]),
                    body: SchemaBody::Matrix {
                        element: Box::new(SchemaBody::Bool),
                        dimensions: vec![
                            mech_core::DimensionExpr::Constant(2),
                            mech_core::DimensionExpr::Constant(2),
                        ]
                        .into_boxed_slice(),
                    },
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let scalar_schema = build.resolve(scalar_handle).unwrap();
        let matrix_schema = build.resolve(matrix_handle).unwrap();
        let output_schema = build.resolve(output_handle).unwrap();
        let (schemas, _) = build.into_parts();
        let matrix = ValueDraft {
            schema: matrix_schema,
            shape_values: Box::new([]),
            data: ValueDataDraft::Matrix(
                [1_u64, 2, 3, 4]
                    .into_iter()
                    .map(ValueDataDraft::U64)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
        }
        .finalize(&SnapshotValidationContext::new(&schemas))
        .unwrap();
        let scalar = ValueDraft {
            schema: scalar_schema,
            shape_values: Box::new([]),
            data: ValueDataDraft::U64(2),
        }
        .finalize(&SnapshotValidationContext::new(&schemas))
        .unwrap();
        let left = [Some(matrix)];
        let right = [Some(scalar)];
        let inputs = [
            ResidentValueRef::Snapshot(&left),
            ResidentValueRef::Snapshot(&right),
        ];
        let port = |schema_id, kind, shape| mech_core::ResidentPortLayout {
            schema_id,
            schema_key: schemas.entry(schema_id).unwrap().key(),
            kind,
            shape,
            shape_instance: schemas
                .get(schema_id)
                .unwrap()
                .instantiate_shape(Box::new([]))
                .unwrap(),
            resolved_selector: None,
        };
        let contract = ResolvedOperationContract::Declared(mech_core::DeclaredOperationContract {
            inputs: [matrix_schema, scalar_schema]
                .into_iter()
                .map(|schema| mech_core::ResolvedInputPort {
                    schema,
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            outputs: vec![mech_core::ResolvedOutputPort {
                schema: output_schema,
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
        });
        let output_shape = ResidentShape {
            rows: 2,
            columns: 2,
        };
        let kernel = bind_semantic_greater(&ResidentKernelBindRequest {
            contract: &contract,
            schemas: &schemas,
            inputs: &[
                port(
                    matrix_schema,
                    ResidentValueKind::Snapshot,
                    ResidentShape::SCALAR,
                ),
                port(
                    scalar_schema,
                    ResidentValueKind::Snapshot,
                    ResidentShape::SCALAR,
                ),
            ],
            output: port(output_schema, ResidentValueKind::Bool, output_shape),
        })
        .unwrap();
        let mut output = [0_u8; 4];
        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::Bool(&mut output)),
            Ok(true),
        );
        assert_eq!(output, [0, 1, 0, 1]);
    }

    #[test]
    fn scalar_f64_comparison_kernels_cover_the_retained_relations() {
        let left = [2.0];
        let right = [2.0];
        let inputs = [ResidentValueRef::F64(&left), ResidentValueRef::F64(&right)];
        let cases = [
            (f64_equal as mech_core::ResidentKernelExecutor, true),
            (f64_not_equal as mech_core::ResidentKernelExecutor, false),
            (f64_less as mech_core::ResidentKernelExecutor, false),
            (f64_less_equal as mech_core::ResidentKernelExecutor, true),
            (f64_greater as mech_core::ResidentKernelExecutor, false),
            (f64_greater_equal as mech_core::ResidentKernelExecutor, true),
        ];
        for (executor, expected) in cases {
            let kernel = BoundResidentKernel::new(executor, Box::new([]));
            let mut output = [u8::from(!expected)];
            assert_eq!(
                kernel.execute(&Inputs(&inputs), ResidentValueMut::Bool(&mut output)),
                Ok(true)
            );
            assert_eq!(output, [u8::from(expected)]);
        }
    }

    #[test]
    fn f64_matrix_comparisons_share_declared_broadcast_layouts() {
        let matrix = [1.0, 4.0, 2.0, 5.0, 3.0, 6.0];
        let row = [1.0, 4.0, 3.0];
        let inputs = [ResidentValueRef::F64(&matrix), ResidentValueRef::F64(&row)];
        for (executor, expected) in [
            (
                f64_vector_equal as mech_core::ResidentKernelExecutor,
                [1, 0, 0, 0, 1, 0],
            ),
            (
                f64_vector_not_equal as mech_core::ResidentKernelExecutor,
                [0, 1, 1, 1, 0, 1],
            ),
            (
                f64_vector_less as mech_core::ResidentKernelExecutor,
                [0, 0, 1, 0, 0, 0],
            ),
            (
                f64_vector_less_equal as mech_core::ResidentKernelExecutor,
                [1, 0, 1, 0, 1, 0],
            ),
            (
                f64_vector_greater as mech_core::ResidentKernelExecutor,
                [0, 1, 0, 1, 0, 1],
            ),
            (
                f64_vector_greater_equal as mech_core::ResidentKernelExecutor,
                [1, 1, 0, 1, 1, 1],
            ),
        ] {
            let kernel = BoundResidentKernel::new(
                executor,
                Box::new([2, 3, BINARY_BROADCAST_EXACT, BINARY_BROADCAST_ROW]),
            );
            let mut output = [0; 6];
            assert_eq!(
                kernel.execute(&Inputs(&inputs), ResidentValueMut::Bool(&mut output)),
                Ok(true),
            );
            assert_eq!(output, expected);
        }
    }

    #[test]
    fn all_f64_matrix_comparison_binders_admit_the_same_broadcast_layouts() {
        fn schema(body: SchemaBody) -> mech_core::Schema {
            mech_core::SchemaDraft {
                dimension_parameters: Box::new([]),
                body,
            }
            .finalize()
            .unwrap()
        }
        let matrix = |element, rows, columns| SchemaBody::Matrix {
            element: Box::new(element),
            dimensions: vec![
                mech_core::DimensionExpr::Constant(rows),
                mech_core::DimensionExpr::Constant(columns),
            ]
            .into_boxed_slice(),
        };
        let mut builder = mech_core::SchemaTableBuilder::new();
        let lhs = builder
            .insert(schema(matrix(
                SchemaBody::FloatingPoint(mech_core::FloatWidth::W64),
                2,
                3,
            )))
            .unwrap();
        let rhs = builder
            .insert(schema(matrix(
                SchemaBody::FloatingPoint(mech_core::FloatWidth::W64),
                1,
                3,
            )))
            .unwrap();
        let output = builder
            .insert(schema(matrix(SchemaBody::Bool, 2, 3)))
            .unwrap();
        let build = builder.finish().unwrap();
        let lhs = build.resolve(lhs).unwrap();
        let rhs = build.resolve(rhs).unwrap();
        let output = build.resolve(output).unwrap();
        let (schemas, _) = build.into_parts();
        let port = |schema_id, kind, shape| mech_core::ResidentPortLayout {
            schema_id,
            schema_key: schemas.entry(schema_id).unwrap().key(),
            kind,
            shape,
            shape_instance: schemas
                .get(schema_id)
                .unwrap()
                .instantiate_shape(Box::new([]))
                .unwrap(),
            resolved_selector: None,
        };
        let inputs = [
            port(
                lhs,
                ResidentValueKind::F64,
                ResidentShape {
                    rows: 2,
                    columns: 3,
                },
            ),
            port(
                rhs,
                ResidentValueKind::F64,
                ResidentShape {
                    rows: 1,
                    columns: 3,
                },
            ),
        ];
        let output_port = port(
            output,
            ResidentValueKind::Bool,
            ResidentShape {
                rows: 2,
                columns: 3,
            },
        );
        let contract = ResolvedOperationContract::Declared(mech_core::DeclaredOperationContract {
            inputs: [lhs, rhs]
                .into_iter()
                .map(|schema| mech_core::ResolvedInputPort {
                    schema,
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            outputs: vec![mech_core::ResolvedOutputPort {
                schema: output,
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
        });
        for binder in [
            bind_semantic_equal as mech_core::ResidentKernelFactory,
            bind_semantic_not_equal as mech_core::ResidentKernelFactory,
            bind_semantic_less as mech_core::ResidentKernelFactory,
            bind_semantic_less_equal as mech_core::ResidentKernelFactory,
            bind_semantic_greater as mech_core::ResidentKernelFactory,
            bind_semantic_greater_equal as mech_core::ResidentKernelFactory,
        ] {
            assert!(
                binder(&ResidentKernelBindRequest {
                    contract: &contract,
                    schemas: &schemas,
                    inputs: &inputs,
                    output: output_port.clone(),
                })
                .is_ok()
            );
        }
    }

    #[test]
    fn strict_comparison_covers_equal_unequal_and_cross_kind_values() {
        let left = [2.0, 3.0];
        let equal = [2.0, 3.0];
        let unequal = [2.0, 4.0];
        let text = ["2.0".to_owned(), "3.0".to_owned()];
        let cases = [
            (
                strict_equal as mech_core::ResidentKernelExecutor,
                ResidentValueRef::F64(&equal),
                true,
            ),
            (
                strict_not_equal as mech_core::ResidentKernelExecutor,
                ResidentValueRef::F64(&unequal),
                true,
            ),
            (
                strict_equal as mech_core::ResidentKernelExecutor,
                ResidentValueRef::String(&text),
                false,
            ),
        ];
        for (executor, right, expected) in cases {
            let inputs = [ResidentValueRef::F64(&left), right];
            let kernel = BoundResidentKernel::new(executor, Box::new([]));
            let mut output = [u8::from(!expected)];
            assert_eq!(
                kernel.execute(&Inputs(&inputs), ResidentValueMut::Bool(&mut output)),
                Ok(true)
            );
            assert_eq!(output, [u8::from(expected)]);
        }
    }

    #[test]
    fn hold_state_rejects_same_shape_values_with_different_schemas() {
        fn schema(body: SchemaBody) -> mech_core::Schema {
            mech_core::SchemaDraft {
                dimension_parameters: Box::new([]),
                body,
            }
            .finalize()
            .unwrap()
        }

        let mut builder = mech_core::SchemaTableBuilder::new();
        let scalar = builder
            .insert(schema(SchemaBody::FloatingPoint(
                mech_core::FloatWidth::W64,
            )))
            .unwrap();
        let matrix = builder
            .insert(schema(SchemaBody::Matrix {
                element: Box::new(SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)),
                dimensions: vec![
                    mech_core::DimensionExpr::Constant(1),
                    mech_core::DimensionExpr::Constant(1),
                ]
                .into_boxed_slice(),
            }))
            .unwrap();
        let build = builder.finish().unwrap();
        let scalar = build.resolve(scalar).unwrap();
        let matrix = build.resolve(matrix).unwrap();
        let (schemas, _) = build.into_parts();
        let input = mech_core::ResidentPortLayout {
            schema_id: scalar,
            schema_key: schemas.entry(scalar).unwrap().key(),
            kind: ResidentValueKind::F64,
            shape: ResidentShape::SCALAR,
            shape_instance: schemas
                .get(scalar)
                .unwrap()
                .instantiate_shape(Box::new([]))
                .unwrap(),
            resolved_selector: None,
        };
        let output = mech_core::ResidentPortLayout {
            schema_id: matrix,
            schema_key: schemas.entry(matrix).unwrap().key(),
            kind: ResidentValueKind::F64,
            shape: ResidentShape::SCALAR,
            shape_instance: schemas
                .get(matrix)
                .unwrap()
                .instantiate_shape(Box::new([]))
                .unwrap(),
            resolved_selector: None,
        };
        let contract = ResolvedOperationContract::Declared(mech_core::DeclaredOperationContract {
            inputs: vec![mech_core::ResolvedInputPort {
                schema: scalar,
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            }]
            .into_boxed_slice(),
            outputs: vec![mech_core::ResolvedOutputPort {
                schema: matrix,
                access: AccessMode::Write,
                delivery: DeliveryMode::Signal,
                construction: OutputConstruction::FullWrite {
                    shape: ShapeRule::SameAsInput { input: 0 },
                },
                alias: AliasPolicy::NoAlias,
                change_detection: ChangeDetectionPolicy::KernelReported,
            }]
            .into_boxed_slice(),
            interaction: ExternalInteraction::Pure,
        });

        assert!(matches!(
            bind_hold_state(&ResidentKernelBindRequest {
                contract: &contract,
                schemas: &schemas,
                inputs: &[input],
                output,
            }),
            Err(ResidentKernelBindError::UnsupportedLayout)
        ));
    }

    #[test]
    fn f64_change_detection_uses_schema_identity_for_one_by_one_matrices() {
        fn request_for(
            body: SchemaBody,
        ) -> (
            mech_core::SchemaTable,
            mech_core::SchemaId,
            ResolvedOperationContract,
        ) {
            let mut builder = mech_core::SchemaTableBuilder::new();
            let provisional = builder
                .insert(
                    mech_core::SchemaDraft {
                        dimension_parameters: Box::new([]),
                        body,
                    }
                    .finalize()
                    .unwrap(),
                )
                .unwrap();
            let build = builder.finish().unwrap();
            let schema = build.resolve(provisional).unwrap();
            let (schemas, _) = build.into_parts();
            (
                schemas,
                schema,
                ResolvedOperationContract::Declared(mech_core::DeclaredOperationContract {
                    inputs: Box::new([]),
                    outputs: Box::new([]),
                    interaction: mech_core::ExternalInteraction::Pure,
                }),
            )
        }

        let cases = [
            (
                SchemaBody::FloatingPoint(mech_core::FloatWidth::W64),
                ChangeDetectionPolicy::ExactScalar,
            ),
            (
                SchemaBody::Matrix {
                    element: Box::new(SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)),
                    dimensions: vec![
                        mech_core::DimensionExpr::Constant(1),
                        mech_core::DimensionExpr::Constant(1),
                    ]
                    .into_boxed_slice(),
                },
                ChangeDetectionPolicy::KernelReported,
            ),
        ];
        for (body, expected) in cases {
            let (schemas, schema_id, contract) = request_for(body);
            let output = mech_core::ResidentPortLayout {
                schema_id,
                schema_key: schemas.entry(schema_id).unwrap().key(),
                kind: ResidentValueKind::F64,
                shape: ResidentShape::SCALAR,
                shape_instance: schemas
                    .get(schema_id)
                    .unwrap()
                    .instantiate_shape(Box::new([]))
                    .unwrap(),
                resolved_selector: None,
            };
            let request = ResidentKernelBindRequest {
                contract: &contract,
                schemas: &schemas,
                inputs: &[],
                output,
            };
            assert_eq!(f64_output_change_detection(&request), Ok(expected));
        }
    }

    #[test]
    fn inclusive_range_rejects_reactive_cardinality_changes_before_writing() {
        let start = [1.0];
        let end = [5.0];
        let inputs = [ResidentValueRef::F64(&start), ResidentValueRef::F64(&end)];
        let mut output = [11.0, 12.0, 13.0];
        let previous = output;
        let kernel = BoundResidentKernel::new(range_inclusive, Box::new([]));

        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::F64(&mut output)),
            Err(ResidentKernelError::InvalidShape)
        );
        assert_eq!(output, previous);
    }

    #[test]
    fn inclusive_range_rejects_reversed_endpoints_before_writing() {
        let start = [5.0];
        let end = [1.0];
        let inputs = [ResidentValueRef::F64(&start), ResidentValueRef::F64(&end)];
        let mut output = [11.0, 12.0, 13.0, 14.0, 15.0];
        let previous = output;
        let kernel = BoundResidentKernel::new(range_inclusive, Box::new([]));

        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::F64(&mut output)),
            Err(ResidentKernelError::InvalidInput)
        );
        assert_eq!(output, previous);
    }

    #[test]
    fn exclusive_range_fills_the_declared_resident_output() {
        let start = [1.0];
        let end = [5.0];
        let inputs = [ResidentValueRef::F64(&start), ResidentValueRef::F64(&end)];
        let mut output = [0.0; 4];
        let kernel = BoundResidentKernel::new(range_exclusive, Box::new([]));

        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::F64(&mut output)),
            Ok(true)
        );
        assert_eq!(output, [1.0, 2.0, 3.0, 4.0]);

        let mut wrong_shape = [11.0; 3];
        let previous = wrong_shape;
        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::F64(&mut wrong_shape)),
            Err(ResidentKernelError::InvalidShape)
        );
        assert_eq!(wrong_shape, previous);
    }

    #[test]
    fn exclusive_increment_range_stops_before_the_terminal_value() {
        let start = [1.0];
        let step = [2.0];
        let end = [6.0];
        let inputs = [
            ResidentValueRef::F64(&start),
            ResidentValueRef::F64(&step),
            ResidentValueRef::F64(&end),
        ];
        let mut output = [0.0; 3];
        let kernel = BoundResidentKernel::new(range_increment_exclusive, Box::new([]));

        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::F64(&mut output)),
            Ok(true)
        );
        assert_eq!(output, [1.0, 3.0, 5.0]);
    }

    #[test]
    fn resident_increment_ranges_match_machine_repeated_addition_bits() {
        let start = [0.0_f64];
        let step = [0.1_f64];
        let end = [1.0_f64];
        let inputs = [
            ResidentValueRef::F64(&start),
            ResidentValueRef::F64(&step),
            ResidentValueRef::F64(&end),
        ];
        let mut expected = Vec::with_capacity(11);
        let mut current = start[0];
        for _ in 0..11 {
            expected.push(current);
            current += step[0];
        }
        assert_ne!(
            expected[6].to_bits(),
            (start[0] + step[0] * 6.0).to_bits(),
            "test values must distinguish recurrence from index multiplication"
        );

        let mut exclusive = vec![0.0; 10];
        let kernel = BoundResidentKernel::new(range_increment_exclusive, Box::new([]));
        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::F64(&mut exclusive)),
            Ok(true)
        );
        assert_eq!(
            exclusive
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>(),
            expected[..10]
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>()
        );

        let mut inclusive = vec![0.0; 11];
        let kernel = BoundResidentKernel::new(range_increment_inclusive, Box::new([]));
        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::F64(&mut inclusive)),
            Ok(true)
        );
        assert_eq!(
            inclusive
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>(),
            expected
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn n_choose_k_accepts_every_declared_selection_size() {
        let values = [1.0, 2.0, 3.0];
        for (k, combinations, expected) in [
            (1_u64, 3_u64, vec![1.0, 2.0, 3.0]),
            (3_u64, 1_u64, vec![1.0, 2.0, 3.0]),
        ] {
            let selection = [k as f64];
            let inputs = [
                ResidentValueRef::F64(&values),
                ResidentValueRef::F64(&selection),
            ];
            let mut output = vec![0.0; (k * combinations) as usize];
            let kernel =
                BoundResidentKernel::new(n_choose_k, vec![k, combinations].into_boxed_slice());
            assert_eq!(
                kernel.execute(&Inputs(&inputs), ResidentValueMut::F64(&mut output)),
                Ok(true)
            );
            assert_eq!(output, expected);
        }
    }

    #[test]
    fn inclusive_increment_range_fills_the_declared_resident_output() {
        let start = [1.0];
        let step = [1.0];
        let end = [4.0];
        let inputs = [
            ResidentValueRef::F64(&start),
            ResidentValueRef::F64(&step),
            ResidentValueRef::F64(&end),
        ];
        let mut output = [0.0; 4];
        let kernel = BoundResidentKernel::new(range_increment_inclusive, Box::new([]));

        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::F64(&mut output)),
            Ok(true)
        );
        assert_eq!(output, [1.0, 2.0, 3.0, 4.0]);

        let mut wrong_shape = [0.0; 3];
        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::F64(&mut wrong_shape)),
            Err(ResidentKernelError::InvalidShape)
        );
    }

    #[test]
    fn matrix_dot_and_solve_residents_use_column_major_layouts() {
        let left = [1.0, 2.0, 3.0];
        let right = [4.0, 5.0, 6.0];
        let inputs = [ResidentValueRef::F64(&left), ResidentValueRef::F64(&right)];
        let mut dot = [0.0];
        let kernel = BoundResidentKernel::new(matrix_dot_f64, Box::new([]));
        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::F64(&mut dot)),
            Ok(true)
        );
        assert_eq!(dot, [32.0]);

        let coefficients = [4.0, 2.0, 1.0, 3.0];
        let right = [9.0, 8.0];
        let inputs = [
            ResidentValueRef::F64(&coefficients),
            ResidentValueRef::F64(&right),
        ];
        let mut solution = [0.0; 2];
        let kernel = BoundResidentKernel::new(matrix_solve_f64, vec![2, 1].into_boxed_slice());
        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::F64(&mut solution)),
            Ok(true)
        );
        assert!((solution[0] - 1.9).abs() < 1.0e-12);
        assert!((solution[1] - 1.4).abs() < 1.0e-12);

        let singular_coefficients = [1.0, 2.0, 2.0, 4.0];
        let singular_right = [1.0, 2.0];
        let singular_inputs = [
            ResidentValueRef::F64(&singular_coefficients),
            ResidentValueRef::F64(&singular_right),
        ];
        let mut unchanged = [7.0, 8.0];
        assert_eq!(
            kernel.execute(
                &Inputs(&singular_inputs),
                ResidentValueMut::F64(&mut unchanged),
            ),
            Err(ResidentKernelError::Arithmetic)
        );
        assert_eq!(unchanged, [7.0, 8.0]);
        assert!(matrix_solve_work(255, 1).is_some_and(|work| work <= MAX_MATRIX_SOLVE_WORK));
        assert!(matrix_solve_work(256, 1).is_some_and(|work| work > MAX_MATRIX_SOLVE_WORK));
        assert_eq!(matrix_solve_work(usize::MAX, 1), None);

        let coefficients = [1.0];
        let oversized_right = vec![1.0; 1_000_000];
        let oversized_inputs = [
            ResidentValueRef::F64(&coefficients),
            ResidentValueRef::F64(&oversized_right),
        ];
        let mut oversized_output = vec![0.0; oversized_right.len()];
        let oversized = BoundResidentKernel::new(
            matrix_solve_f64,
            vec![1, oversized_right.len() as u64].into_boxed_slice(),
        );
        assert_eq!(
            oversized.execute(
                &Inputs(&oversized_inputs),
                ResidentValueMut::F64(&mut oversized_output),
            ),
            Err(ResidentKernelError::InvalidShape),
        );

        assert_eq!(
            numeric_add(
                ValueDataDraft::U8(1),
                numeric_multiply(ValueDataDraft::U8(2), ValueDataDraft::U8(3)).unwrap(),
            ),
            Ok(ValueDataDraft::U8(7))
        );
    }

    #[test]
    fn scalar_access_and_matrix_product_use_column_major_layouts() {
        let source = [1.0, 2.0, 3.0, 4.0];
        let index = [3_u64];
        let inputs = [
            ResidentValueRef::F64(&source),
            ResidentValueRef::Index(&index),
        ];
        let mut selected = [0.0];
        let access = BoundResidentKernel::new(scalar_access_1d, Box::new([]));
        assert_eq!(
            access.execute(&Inputs(&inputs), ResidentValueMut::F64(&mut selected)),
            Ok(true)
        );
        assert_eq!(selected, [3.0]);

        let row = [2_u64];
        let column = [1_u64];
        let inputs = [
            ResidentValueRef::F64(&source),
            ResidentValueRef::Index(&row),
            ResidentValueRef::Index(&column),
        ];
        let access = BoundResidentKernel::new(scalar_access_2d, vec![2, 2].into_boxed_slice());
        assert_eq!(
            access.execute(&Inputs(&inputs), ResidentValueMut::F64(&mut selected)),
            Ok(true)
        );
        assert_eq!(selected, [2.0]);

        let lhs = [1.0, 3.0, 2.0, 4.0];
        let rhs = [5.0, 7.0, 6.0, 8.0];
        let inputs = [ResidentValueRef::F64(&lhs), ResidentValueRef::F64(&rhs)];
        let mut product = [0.0; 4];
        let matmul = BoundResidentKernel::new(matrix_multiply, vec![2, 2, 2].into_boxed_slice());
        assert_eq!(
            matmul.execute(&Inputs(&inputs), ResidentValueMut::F64(&mut product)),
            Ok(true)
        );
        assert_eq!(product, [19.0, 43.0, 22.0, 50.0]);
    }

    #[test]
    fn scalar_index_matches_existing_f64_index_conversion() {
        let input = [2.0];
        let inputs = [ResidentValueRef::F64(&input)];
        let mut output = [0_u64];
        let conversion = BoundResidentKernel::new(scalar_index, Box::new([]));
        assert_eq!(
            conversion.execute(&Inputs(&inputs), ResidentValueMut::Index(&mut output)),
            Ok(true)
        );
        assert_eq!(output, [2]);

        let fractional = [1.5];
        let inputs = [ResidentValueRef::F64(&fractional)];
        assert_eq!(
            conversion.execute(&Inputs(&inputs), ResidentValueMut::Index(&mut output)),
            Ok(true)
        );
        assert_eq!(output, [1]);

        let maximum = [PORTABLE_INDEX_MAX as f64];
        let inputs = [ResidentValueRef::F64(&maximum)];
        assert_eq!(
            conversion.execute(&Inputs(&inputs), ResidentValueMut::Index(&mut output)),
            Ok(true)
        );
        assert_eq!(output, [PORTABLE_INDEX_MAX]);

        for rejected in [
            0.0,
            0.5,
            -1.0,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            PORTABLE_INDEX_MAX as f64 + 1.0,
        ] {
            let input = [rejected];
            let inputs = [ResidentValueRef::F64(&input)];
            assert_eq!(
                conversion.execute(&Inputs(&inputs), ResidentValueMut::Index(&mut output)),
                Err(ResidentKernelError::InvalidInput)
            );
        }

        let input = [PORTABLE_INDEX_MAX + 1];
        let inputs = [ResidentValueRef::Index(&input)];
        assert_eq!(
            conversion.execute(&Inputs(&inputs), ResidentValueMut::Index(&mut output)),
            Err(ResidentKernelError::InvalidInput)
        );
    }

    #[test]
    fn indexed_assignment_binding_rejects_undersized_positional_sources() {
        let f64_body = SchemaBody::FloatingPoint(mech_core::FloatWidth::W64);
        let matrix = |columns| SchemaBody::Matrix {
            element: Box::new(f64_body.clone()),
            dimensions: vec![
                mech_core::DimensionExpr::Constant(1),
                mech_core::DimensionExpr::Constant(columns),
            ]
            .into_boxed_slice(),
        };
        let index_matrix = SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: vec![
                mech_core::DimensionExpr::Constant(1),
                mech_core::DimensionExpr::Constant(3),
            ]
            .into_boxed_slice(),
        };
        let (schemas, ids) = test_schema_table([matrix(4), matrix(2), index_matrix]);
        let [base, source, selector] = ids.as_slice() else {
            unreachable!()
        };
        let contract = test_contract(
            &[*base, *source, *selector],
            *base,
            OutputConstruction::ReadModifyWrite {
                base_input: 0,
                regions: RegionPolicy::IndexedAxis { axis: 0 },
            },
            AccessMode::ReadWrite,
            AliasPolicy::MayAlias { input: 0 },
            ChangeDetectionPolicy::KernelReported,
        );
        assert!(matches!(
            bind_indexed_assign(&ResidentKernelBindRequest {
                contract: &contract,
                schemas: &schemas,
                inputs: &[
                    test_layout(
                        &schemas,
                        *base,
                        ResidentValueKind::F64,
                        ResidentShape {
                            rows: 1,
                            columns: 4,
                        },
                    ),
                    test_layout(
                        &schemas,
                        *source,
                        ResidentValueKind::F64,
                        ResidentShape {
                            rows: 1,
                            columns: 2,
                        },
                    ),
                    test_layout(
                        &schemas,
                        *selector,
                        ResidentValueKind::Index,
                        ResidentShape {
                            rows: 1,
                            columns: 3,
                        },
                    ),
                ],
                output: test_layout(
                    &schemas,
                    *base,
                    ResidentValueKind::F64,
                    ResidentShape {
                        rows: 1,
                        columns: 4,
                    },
                ),
            }),
            Err(ResidentKernelBindError::UnsupportedLayout),
        ));
    }

    #[test]
    fn matrix_assignment_rejects_changed_mask_extent_before_mutation() {
        let bool_selector = SchemaBody::Matrix {
            element: Box::new(SchemaBody::Bool),
            dimensions: vec![
                mech_core::DimensionExpr::Constant(1),
                mech_core::DimensionExpr::Constant(3),
            ]
            .into_boxed_slice(),
        };
        let (schemas, ids) = test_schema_table([bool_selector]);
        let [selector_schema] = ids.as_slice() else {
            unreachable!()
        };
        let selector_shape = schemas
            .get(*selector_schema)
            .unwrap()
            .instantiate_shape(Box::new([]))
            .unwrap();
        let plan = MatrixSelectionAssignPlan {
            selectors: vec![SnapshotAccessSelectorLayout {
                schema: *selector_schema,
                shape: selector_shape,
                resident_shape: ResidentShape {
                    rows: 1,
                    columns: 3,
                },
            }]
            .into_boxed_slice(),
            mode: ResolvedSelectionMode::Columns,
            rows: 2,
            columns: 3,
            source_rows: 1,
            source_columns: 1,
            source_routing: ResolvedSourceRouting::ScalarBroadcast,
        };
        let kernel = BoundResidentKernel::new(indexed_assign_matrix_selection, Box::new([]))
            .with_retained_state(Arc::new(plan))
            .with_snapshot_schemas(schemas);
        let source = [9.0];
        let changed_extent = [1_u8, 1, 1, 1];
        let inputs = [
            ResidentValueRef::F64(&source),
            ResidentValueRef::Bool(&changed_extent),
        ];
        let mut output = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let previous = output;
        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::F64(&mut output)),
            Err(ResidentKernelError::InvalidShape),
        );
        assert_eq!(output, previous);
    }

    #[test]
    fn string_matrix_scalar_broadcast_charges_every_materialized_clone() {
        let (schemas, _) = test_schema_table([SchemaBody::String]);
        let plan = MatrixSelectionAssignPlan {
            selectors: Box::new([]),
            mode: ResolvedSelectionMode::Whole,
            rows: 1,
            columns: 2,
            source_rows: 1,
            source_columns: 1,
            source_routing: ResolvedSourceRouting::ScalarBroadcast,
        };
        let kernel = BoundResidentKernel::new(indexed_assign_matrix_selection, Box::new([]))
            .with_retained_state(Arc::new(plan))
            .with_snapshot_schemas(schemas);
        let source = ["x".repeat(9 * 1024 * 1024)];
        let inputs = [ResidentValueRef::String(&source)];
        let mut output = ["left".to_owned(), "right".to_owned()];
        let previous = output.clone();
        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::String(&mut output)),
            Err(ResidentKernelError::InvalidShape),
        );
        assert_eq!(output, previous);
    }

    #[test]
    fn partial_string_matrix_assignment_admits_the_complete_published_value() {
        let (schemas, ids) = test_schema_table([SchemaBody::String, SchemaBody::Index]);
        let selector_schema = ids[1];
        let selector_shape = schemas
            .get(selector_schema)
            .unwrap()
            .instantiate_shape(Box::new([]))
            .unwrap();
        let plan = MatrixSelectionAssignPlan {
            selectors: vec![SnapshotAccessSelectorLayout {
                schema: selector_schema,
                shape: selector_shape,
                resident_shape: ResidentShape::SCALAR,
            }]
            .into_boxed_slice(),
            mode: ResolvedSelectionMode::Columns,
            rows: 1,
            columns: 3,
            source_rows: 1,
            source_columns: 1,
            source_routing: ResolvedSourceRouting::ScalarBroadcast,
        };
        let kernel = BoundResidentKernel::new(indexed_assign_matrix_selection, Box::new([]))
            .with_retained_state(Arc::new(plan))
            .with_snapshot_schemas(schemas);
        let source = ["replacement".to_owned()];
        let selected_column = [1_u64];
        let inputs = [
            ResidentValueRef::String(&source),
            ResidentValueRef::Index(&selected_column),
        ];
        let mut output = [
            "a".repeat(6 * 1024 * 1024),
            "b".repeat(6 * 1024 * 1024),
            "c".repeat(6 * 1024 * 1024),
        ];
        let previous = output.clone();
        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::String(&mut output)),
            Err(ResidentKernelError::InvalidShape),
        );
        assert_eq!(output, previous);
    }

    #[test]
    fn empty_string_matrix_selection_returns_before_output_sized_planning() {
        let (schemas, ids) = test_schema_table([SchemaBody::String, SchemaBody::Index]);
        let selector_schema = ids[1];
        let selector_shape = schemas
            .get(selector_schema)
            .unwrap()
            .instantiate_shape(Box::new([]))
            .unwrap();
        let plan = MatrixSelectionAssignPlan {
            selectors: vec![SnapshotAccessSelectorLayout {
                schema: selector_schema,
                shape: selector_shape,
                resident_shape: ResidentShape::SCALAR,
            }]
            .into_boxed_slice(),
            mode: ResolvedSelectionMode::Columns,
            rows: 1,
            columns: 2,
            source_rows: 1,
            source_columns: 1,
            source_routing: ResolvedSourceRouting::ScalarBroadcast,
        };
        let kernel = BoundResidentKernel::new(indexed_assign_matrix_selection, Box::new([]))
            .with_retained_state(Arc::new(plan))
            .with_snapshot_schemas(schemas);
        let source = ["replacement".to_owned()];
        let selected_columns = Vec::<u64>::new();
        let inputs = [
            ResidentValueRef::String(&source),
            ResidentValueRef::Index(&selected_columns),
        ];
        let mut output = ["a".repeat(9 * 1024 * 1024), "b".repeat(9 * 1024 * 1024)];
        let previous = output.clone();
        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::String(&mut output)),
            Ok(false),
        );
        assert_eq!(output, previous);
    }

    #[test]
    fn snapshot_u64_matrix_assignment_is_schema_aware_and_atomic() {
        let u64_body = SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64);
        let matrix_body = SchemaBody::Matrix {
            element: Box::new(u64_body.clone()),
            dimensions: vec![
                mech_core::DimensionExpr::Constant(2),
                mech_core::DimensionExpr::Constant(2),
            ]
            .into_boxed_slice(),
        };
        let (schemas, ids) = test_schema_table([u64_body, matrix_body, SchemaBody::Index]);
        let [scalar, matrix, selector] = ids.as_slice() else {
            unreachable!()
        };
        let contract = test_contract(
            &[*matrix, *scalar, *selector],
            *matrix,
            OutputConstruction::ReadModifyWrite {
                base_input: 0,
                regions: RegionPolicy::IndexedAxis { axis: 0 },
            },
            AccessMode::ReadWrite,
            AliasPolicy::MayAlias { input: 0 },
            ChangeDetectionPolicy::KernelReported,
        );
        let kernel = bind_indexed_assign(&ResidentKernelBindRequest {
            contract: &contract,
            schemas: &schemas,
            inputs: &[
                test_layout(
                    &schemas,
                    *matrix,
                    ResidentValueKind::Snapshot,
                    ResidentShape::SCALAR,
                ),
                test_layout(
                    &schemas,
                    *scalar,
                    ResidentValueKind::Snapshot,
                    ResidentShape::SCALAR,
                ),
                test_layout(
                    &schemas,
                    *selector,
                    ResidentValueKind::Index,
                    ResidentShape::SCALAR,
                ),
            ],
            output: test_layout(
                &schemas,
                *matrix,
                ResidentValueKind::Snapshot,
                ResidentShape::SCALAR,
            ),
        })
        .unwrap();
        let current = test_value(
            &schemas,
            *matrix,
            ValueDataDraft::Matrix(
                [1, 2, 3, 4]
                    .into_iter()
                    .map(ValueDataDraft::U64)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
        );
        let replacement = test_value(&schemas, *scalar, ValueDataDraft::U64(9));
        let source = [Some(replacement)];
        let selector = [2_u64];
        let inputs = [
            ResidentValueRef::Snapshot(&source),
            ResidentValueRef::Index(&selector),
        ];
        let mut output = [Some(current)];
        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::Snapshot(&mut output)),
            Ok(true),
        );
        let ValueData::Matrix(values) = output[0].as_ref().unwrap().data() else {
            panic!("assignment output must remain a matrix")
        };
        assert!(matches!(
            values.elements().to_values().as_slice(),
            [
                ValueData::U64(1),
                ValueData::U64(2),
                ValueData::U64(9),
                ValueData::U64(4)
            ]
        ));
    }

    #[test]
    fn snapshot_whole_matrix_selection_carries_finalization_into_publication() {
        let scalar_body = SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64);
        let matrix_body = SchemaBody::Matrix {
            element: Box::new(scalar_body.clone()),
            dimensions: vec![
                mech_core::DimensionExpr::Constant(2),
                mech_core::DimensionExpr::Constant(2),
            ]
            .into_boxed_slice(),
        };
        let (schemas, ids) = test_schema_table([scalar_body, matrix_body]);
        let [scalar_schema, matrix_schema] = ids.as_slice() else {
            unreachable!()
        };
        let plan = MatrixSelectionAssignPlan {
            selectors: Box::new([]),
            mode: ResolvedSelectionMode::Whole,
            rows: 2,
            columns: 2,
            source_rows: 1,
            source_columns: 1,
            source_routing: ResolvedSourceRouting::ScalarBroadcast,
        };
        let kernel = BoundResidentKernel::new(indexed_assign_matrix_selection, Box::new([]))
            .with_retained_state(Arc::new(plan))
            .with_snapshot_output(ResidentSnapshotOutput {
                schema: *matrix_schema,
                schema_key: schemas.entry(*matrix_schema).unwrap().key(),
                shape: schemas
                    .get(*matrix_schema)
                    .unwrap()
                    .instantiate_shape(Box::new([]))
                    .unwrap(),
                exact_cardinality: None,
                maximum_cardinality: None,
            })
            .with_snapshot_schemas(schemas.clone());
        let source = [Some(test_value(
            &schemas,
            *scalar_schema,
            ValueDataDraft::U64(9),
        ))];
        let current = test_value(
            &schemas,
            *matrix_schema,
            ValueDataDraft::Matrix(
                [1, 2, 3, 4]
                    .into_iter()
                    .map(ValueDataDraft::U64)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
        );
        let inputs = [ResidentValueRef::Snapshot(&source)];
        let mut output = [Some(current)];
        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::Snapshot(&mut output)),
            Ok(true),
        );
        let ValueData::Matrix(values) = output[0].as_ref().unwrap().data() else {
            panic!("assignment output must remain a matrix")
        };
        assert!(matches!(
            values.elements().to_values().as_slice(),
            [
                ValueData::U64(9),
                ValueData::U64(9),
                ValueData::U64(9),
                ValueData::U64(9)
            ]
        ));
    }

    #[test]
    fn snapshot_nested_matrix_selection_plans_recursive_final_storage() {
        let scalar_body = SchemaBody::Tuple(vec![SchemaBody::Bool].into_boxed_slice());
        let matrix_body = SchemaBody::Matrix {
            element: Box::new(scalar_body.clone()),
            dimensions: vec![
                mech_core::DimensionExpr::Constant(1),
                mech_core::DimensionExpr::Constant(2),
            ]
            .into_boxed_slice(),
        };
        let (schemas, ids) = test_schema_table([scalar_body, matrix_body]);
        let [scalar_schema, matrix_schema] = ids.as_slice() else {
            unreachable!()
        };
        let plan = MatrixSelectionAssignPlan {
            selectors: Box::new([]),
            mode: ResolvedSelectionMode::Whole,
            rows: 1,
            columns: 2,
            source_rows: 1,
            source_columns: 1,
            source_routing: ResolvedSourceRouting::ScalarBroadcast,
        };
        let kernel = BoundResidentKernel::new(indexed_assign_matrix_selection, Box::new([]))
            .with_retained_state(Arc::new(plan))
            .with_snapshot_output(ResidentSnapshotOutput {
                schema: *matrix_schema,
                schema_key: schemas.entry(*matrix_schema).unwrap().key(),
                shape: schemas
                    .get(*matrix_schema)
                    .unwrap()
                    .instantiate_shape(Box::new([]))
                    .unwrap(),
                exact_cardinality: None,
                maximum_cardinality: None,
            })
            .with_snapshot_schemas(schemas.clone());
        let tuple =
            |value| ValueDataDraft::Tuple(vec![ValueDataDraft::Bool(value)].into_boxed_slice());
        let source = [Some(test_value(&schemas, *scalar_schema, tuple(true)))];
        let current = test_value(
            &schemas,
            *matrix_schema,
            ValueDataDraft::Matrix(vec![tuple(false), tuple(false)].into_boxed_slice()),
        );
        let inputs = [ResidentValueRef::Snapshot(&source)];
        let mut output = [Some(current)];
        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::Snapshot(&mut output)),
            Ok(true),
        );
        let ValueData::Matrix(values) = output[0].as_ref().unwrap().data() else {
            panic!("assignment output must remain a matrix")
        };
        assert!(matches!(
            values.elements().to_values().as_slice(),
            [ValueData::Tuple(_), ValueData::Tuple(_)]
        ));
    }

    #[test]
    fn snapshot_indexed_assignment_admits_all_recursive_node_phases() {
        const TUPLE_WIDTH: usize = 15_000;
        let tuple_body = SchemaBody::Tuple(
            std::iter::repeat_n(SchemaBody::Bool, TUPLE_WIDTH)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        );
        let matrix_body = SchemaBody::Matrix {
            element: Box::new(tuple_body.clone()),
            dimensions: vec![
                mech_core::DimensionExpr::Constant(1),
                mech_core::DimensionExpr::Constant(2),
            ]
            .into_boxed_slice(),
        };
        let (schemas, ids) = test_schema_table([tuple_body, matrix_body]);
        let [tuple_schema, matrix_schema] = ids.as_slice() else {
            unreachable!()
        };
        let tuple = || {
            ValueDataDraft::Tuple(
                std::iter::repeat_n(ValueDataDraft::Bool(true), TUPLE_WIDTH)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            )
        };
        let source = [Some(test_value(&schemas, *tuple_schema, tuple()))];
        let original = test_value(
            &schemas,
            *matrix_schema,
            ValueDataDraft::Matrix(vec![tuple(), tuple()].into_boxed_slice()),
        );
        let selector = [1_u64];
        let inputs = [
            ResidentValueRef::Snapshot(&source),
            ResidentValueRef::Index(&selector),
        ];
        let kernel = BoundResidentKernel::new(
            indexed_assign_snapshot,
            vec![1, 2, 1, 1, ResolvedSourceRouting::ScalarBroadcast as u64].into_boxed_slice(),
        )
        .with_snapshot_output(ResidentSnapshotOutput {
            schema: *matrix_schema,
            schema_key: schemas.entry(*matrix_schema).unwrap().key(),
            shape: schemas
                .get(*matrix_schema)
                .unwrap()
                .instantiate_shape(Box::new([]))
                .unwrap(),
            exact_cardinality: None,
            maximum_cardinality: None,
        })
        .with_snapshot_schemas(schemas.clone());
        let mut output = [Some(original.clone())];

        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::Snapshot(&mut output)),
            Err(ResidentKernelError::InvalidShape),
        );
        assert!(
            output[0]
                .as_ref()
                .unwrap()
                .language_eq(&schemas, &original, &schemas)
                .unwrap()
        );
    }

    #[test]
    fn snapshot_aggregate_assignment_executes_records_maps_and_tables_atomically() {
        let u64_body = SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64);
        let f64_body = SchemaBody::FloatingPoint(mech_core::FloatWidth::W64);
        let record_body = SchemaBody::Record(
            vec![mech_core::SchemaField {
                name: "number".to_owned(),
                schema: u64_body.clone(),
            }]
            .into_boxed_slice(),
        );
        let map_body = SchemaBody::Map {
            key: Box::new(f64_body.clone()),
            value: Box::new(u64_body.clone()),
            cardinality: mech_core::CardinalitySpec::Dynamic { upper_bound: None },
        };
        let column_body = SchemaBody::Matrix {
            element: Box::new(u64_body.clone()),
            dimensions: vec![
                mech_core::DimensionExpr::Constant(2),
                mech_core::DimensionExpr::Constant(1),
            ]
            .into_boxed_slice(),
        };
        let table_body = SchemaBody::Table {
            columns: vec![mech_core::SchemaField {
                name: "number".to_owned(),
                schema: u64_body.clone(),
            }]
            .into_boxed_slice(),
            rows: mech_core::CardinalitySpec::Exact(mech_core::DimensionExpr::Constant(2)),
        };
        let (schemas, ids) = test_schema_table([
            u64_body,
            f64_body,
            SchemaBody::Id,
            record_body,
            map_body,
            column_body,
            table_body,
        ]);
        let [
            u64_schema,
            f64_schema,
            id_schema,
            record,
            map,
            column,
            table,
        ] = ids.as_slice()
        else {
            unreachable!()
        };
        let bind = |base, source, selector, selector_kind| {
            let contract = test_contract(
                &[base, source, selector],
                base,
                OutputConstruction::ReadModifyWrite {
                    base_input: 0,
                    regions: RegionPolicy::IndexedAxis { axis: 0 },
                },
                AccessMode::ReadWrite,
                AliasPolicy::MayAlias { input: 0 },
                ChangeDetectionPolicy::KernelReported,
            );
            bind_indexed_assign(&ResidentKernelBindRequest {
                contract: &contract,
                schemas: &schemas,
                inputs: &[
                    test_layout(
                        &schemas,
                        base,
                        ResidentValueKind::Snapshot,
                        ResidentShape::SCALAR,
                    ),
                    test_layout(
                        &schemas,
                        source,
                        ResidentValueKind::Snapshot,
                        ResidentShape::SCALAR,
                    ),
                    test_layout(&schemas, selector, selector_kind, ResidentShape::SCALAR),
                ],
                output: test_layout(
                    &schemas,
                    base,
                    ResidentValueKind::Snapshot,
                    ResidentShape::SCALAR,
                ),
            })
            .unwrap()
        };

        let replacement = [Some(test_value(
            &schemas,
            *u64_schema,
            ValueDataDraft::U64(9),
        ))];

        let record_kernel = bind(
            *record,
            *u64_schema,
            *id_schema,
            ResidentValueKind::Snapshot,
        );
        let record_selector = [Some(test_value(
            &schemas,
            *id_schema,
            ValueDataDraft::Id(mech_core::hash_str("number")),
        ))];
        let record_inputs = [
            ResidentValueRef::Snapshot(&replacement),
            ResidentValueRef::Snapshot(&record_selector),
        ];
        let mut record_output = [Some(test_value(
            &schemas,
            *record,
            ValueDataDraft::Record(
                vec![mech_core::snapshot::NamedValueDraft {
                    name: "number".to_owned(),
                    value: ValueDataDraft::U64(1),
                }]
                .into_boxed_slice(),
            ),
        ))];
        assert_eq!(
            record_kernel.execute(
                &Inputs(&record_inputs),
                ResidentValueMut::Snapshot(&mut record_output),
            ),
            Ok(true),
        );
        let ValueData::Record(record_value) = record_output[0].as_ref().unwrap().data() else {
            panic!("record assignment must preserve the record")
        };
        assert!(matches!(&record_value.fields()[0], ValueData::U64(9)));

        let map_kernel = bind(*map, *u64_schema, *f64_schema, ResidentValueKind::F64);
        let make_map_value = || {
            test_value(
                &schemas,
                *map,
                ValueDataDraft::Map(
                    vec![mech_core::snapshot::MapEntryDraft {
                        items: vec![
                            ValueDataDraft::F64(F64Bits::from_f64(-0.0)),
                            ValueDataDraft::U64(1),
                        ]
                        .into_boxed_slice(),
                    }]
                    .into_boxed_slice(),
                ),
            )
        };
        let positive_zero = [0.0];
        let map_inputs = [
            ResidentValueRef::Snapshot(&replacement),
            ResidentValueRef::F64(&positive_zero),
        ];
        let mut map_output = [Some(make_map_value())];
        assert_eq!(
            map_kernel.execute(
                &Inputs(&map_inputs),
                ResidentValueMut::Snapshot(&mut map_output),
            ),
            Ok(true),
        );
        let ValueData::Map(map_value) = map_output[0].as_ref().unwrap().data() else {
            panic!("map assignment must preserve the map")
        };
        assert!(matches!(map_value.entries()[0].value(), ValueData::U64(9)));

        let missing_key = [3.0];
        let missing_inputs = [
            ResidentValueRef::Snapshot(&replacement),
            ResidentValueRef::F64(&missing_key),
        ];
        let original_map = make_map_value();
        let mut rejected_output = [Some(original_map.clone())];
        assert_eq!(
            map_kernel.execute(
                &Inputs(&missing_inputs),
                ResidentValueMut::Snapshot(&mut rejected_output),
            ),
            Err(ResidentKernelError::InvalidInput),
        );
        assert!(
            rejected_output[0]
                .as_ref()
                .unwrap()
                .language_eq(&schemas, &original_map, &schemas)
                .unwrap()
        );

        let table_kernel = bind(*table, *column, *id_schema, ResidentValueKind::Snapshot);
        let source_column = [Some(test_value(
            &schemas,
            *column,
            ValueDataDraft::Matrix(
                vec![ValueDataDraft::U64(7), ValueDataDraft::U64(8)].into_boxed_slice(),
            ),
        ))];
        let table_inputs = [
            ResidentValueRef::Snapshot(&source_column),
            ResidentValueRef::Snapshot(&record_selector),
        ];
        let mut table_output = [Some(test_value(
            &schemas,
            *table,
            ValueDataDraft::Table(
                vec![mech_core::snapshot::TableColumnDraft {
                    name: "number".to_owned(),
                    values: vec![ValueDataDraft::U64(1), ValueDataDraft::U64(2)].into_boxed_slice(),
                }]
                .into_boxed_slice(),
            ),
        ))];
        assert_eq!(
            table_kernel.execute(
                &Inputs(&table_inputs),
                ResidentValueMut::Snapshot(&mut table_output),
            ),
            Ok(true),
        );
        let ValueData::Table(table_value) = table_output[0].as_ref().unwrap().data() else {
            panic!("table assignment must preserve the table")
        };
        assert!(matches!(
            table_value.column(0).unwrap().to_values().as_slice(),
            [ValueData::U64(7), ValueData::U64(8)]
        ));
    }

    #[test]
    fn snapshot_aggregate_assignment_admits_both_source_copies_before_cloning() {
        let record_body = SchemaBody::Record(
            vec![mech_core::SchemaField {
                name: "text".to_owned(),
                schema: SchemaBody::String,
            }]
            .into_boxed_slice(),
        );
        let (schemas, ids) = test_schema_table([SchemaBody::String, SchemaBody::Id, record_body]);
        let [string_schema, id_schema, record_schema] = ids.as_slice() else {
            unreachable!()
        };
        let contract = test_contract(
            &[*record_schema, *string_schema, *id_schema],
            *record_schema,
            OutputConstruction::ReadModifyWrite {
                base_input: 0,
                regions: RegionPolicy::IndexedAxis { axis: 0 },
            },
            AccessMode::ReadWrite,
            AliasPolicy::MayAlias { input: 0 },
            ChangeDetectionPolicy::KernelReported,
        );
        let kernel = bind_indexed_assign(&ResidentKernelBindRequest {
            contract: &contract,
            schemas: &schemas,
            inputs: &[
                test_layout(
                    &schemas,
                    *record_schema,
                    ResidentValueKind::Snapshot,
                    ResidentShape::SCALAR,
                ),
                test_layout(
                    &schemas,
                    *string_schema,
                    ResidentValueKind::Snapshot,
                    ResidentShape::SCALAR,
                ),
                test_layout(
                    &schemas,
                    *id_schema,
                    ResidentValueKind::Snapshot,
                    ResidentShape::SCALAR,
                ),
            ],
            output: test_layout(
                &schemas,
                *record_schema,
                ResidentValueKind::Snapshot,
                ResidentShape::SCALAR,
            ),
        })
        .unwrap();
        let source = [Some(test_value(
            &schemas,
            *string_schema,
            ValueDataDraft::String("x".repeat(9 * 1024 * 1024)),
        ))];
        let selector = [Some(test_value(
            &schemas,
            *id_schema,
            ValueDataDraft::Id(mech_core::hash_str("text")),
        ))];
        let inputs = [
            ResidentValueRef::Snapshot(&source),
            ResidentValueRef::Snapshot(&selector),
        ];
        let current = test_value(
            &schemas,
            *record_schema,
            ValueDataDraft::Record(
                vec![mech_core::snapshot::NamedValueDraft {
                    name: "text".to_owned(),
                    value: ValueDataDraft::String("unchanged".to_owned()),
                }]
                .into_boxed_slice(),
            ),
        );
        let mut output = [Some(current.clone())];

        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::Snapshot(&mut output)),
            Err(ResidentKernelError::InvalidShape),
        );
        assert!(
            output[0]
                .as_ref()
                .unwrap()
                .language_eq(&schemas, &current, &schemas)
                .unwrap()
        );
    }

    #[test]
    fn map_aggregate_assignment_uses_admitted_append_fast_finalization() {
        let u64_body = SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64);
        let map_body = SchemaBody::Map {
            key: Box::new(u64_body.clone()),
            value: Box::new(u64_body.clone()),
            cardinality: mech_core::CardinalitySpec::Dynamic { upper_bound: None },
        };
        let (schemas, ids) = test_schema_table([u64_body, map_body]);
        let [u64_schema, map_schema] = ids.as_slice() else {
            unreachable!()
        };
        let contract = test_contract(
            &[*map_schema, *u64_schema, *u64_schema],
            *map_schema,
            OutputConstruction::ReadModifyWrite {
                base_input: 0,
                regions: RegionPolicy::IndexedAxis { axis: 0 },
            },
            AccessMode::ReadWrite,
            AliasPolicy::MayAlias { input: 0 },
            ChangeDetectionPolicy::KernelReported,
        );
        let kernel = bind_indexed_assign(&ResidentKernelBindRequest {
            contract: &contract,
            schemas: &schemas,
            inputs: &[
                test_layout(
                    &schemas,
                    *map_schema,
                    ResidentValueKind::Snapshot,
                    ResidentShape::SCALAR,
                ),
                test_layout(
                    &schemas,
                    *u64_schema,
                    ResidentValueKind::Snapshot,
                    ResidentShape::SCALAR,
                ),
                test_layout(
                    &schemas,
                    *u64_schema,
                    ResidentValueKind::Snapshot,
                    ResidentShape::SCALAR,
                ),
            ],
            output: test_layout(
                &schemas,
                *map_schema,
                ResidentValueKind::Snapshot,
                ResidentShape::SCALAR,
            ),
        })
        .unwrap();
        let current = test_value(
            &schemas,
            *map_schema,
            ValueDataDraft::Map(
                (0_u64..400)
                    .map(|key| mech_core::snapshot::MapEntryDraft {
                        items: vec![ValueDataDraft::U64(key), ValueDataDraft::U64(0)]
                            .into_boxed_slice(),
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
        );
        let source = [Some(test_value(
            &schemas,
            *u64_schema,
            ValueDataDraft::U64(9),
        ))];
        let selector = [Some(test_value(
            &schemas,
            *u64_schema,
            ValueDataDraft::U64(399),
        ))];
        let inputs = [
            ResidentValueRef::Snapshot(&source),
            ResidentValueRef::Snapshot(&selector),
        ];
        let mut output = [Some(current.clone())];

        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::Snapshot(&mut output)),
            Ok(true),
        );
        let ValueData::Map(output) = output[0].as_ref().unwrap().data() else {
            unreachable!()
        };
        assert_eq!(output.entries().len(), 400);
        assert!(matches!(output.entries()[399].value(), ValueData::U64(9)));
    }

    #[test]
    fn snapshot_access_covers_numeric_matrices_records_maps_and_tables() {
        let u64_body = SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64);
        let matrix_body = SchemaBody::Matrix {
            element: Box::new(u64_body.clone()),
            dimensions: vec![
                mech_core::DimensionExpr::Constant(2),
                mech_core::DimensionExpr::Constant(2),
            ]
            .into_boxed_slice(),
        };
        let record_body = SchemaBody::Record(
            vec![mech_core::SchemaField {
                name: "number".to_owned(),
                schema: u64_body.clone(),
            }]
            .into_boxed_slice(),
        );
        let map_body = SchemaBody::Map {
            key: Box::new(SchemaBody::String),
            value: Box::new(u64_body.clone()),
            cardinality: mech_core::CardinalitySpec::Dynamic { upper_bound: None },
        };
        let column_body = SchemaBody::Matrix {
            element: Box::new(u64_body.clone()),
            dimensions: vec![
                mech_core::DimensionExpr::Constant(2),
                mech_core::DimensionExpr::Constant(1),
            ]
            .into_boxed_slice(),
        };
        let table_body = SchemaBody::Table {
            columns: vec![mech_core::SchemaField {
                name: "number".to_owned(),
                schema: u64_body.clone(),
            }]
            .into_boxed_slice(),
            rows: mech_core::CardinalitySpec::Exact(mech_core::DimensionExpr::Constant(2)),
        };
        let (schemas, ids) = test_schema_table([
            u64_body,
            matrix_body,
            SchemaBody::Index,
            record_body,
            SchemaBody::Id,
            map_body,
            SchemaBody::String,
            column_body,
            table_body,
        ]);
        let [
            u64_schema,
            matrix,
            index_schema,
            record,
            id_schema,
            map,
            string_schema,
            column,
            table,
        ] = ids.as_slice()
        else {
            unreachable!()
        };
        let access_contract = |source, selector, output| {
            test_contract(
                &[source, selector],
                output,
                OutputConstruction::FullWrite {
                    shape: ShapeRule::Declared,
                },
                AccessMode::Write,
                AliasPolicy::NoAlias,
                ChangeDetectionPolicy::KernelReported,
            )
        };
        let invalid_map_contract = access_contract(*map, *index_schema, *u64_schema);
        assert!(matches!(
            bind_snapshot_access(&ResidentKernelBindRequest {
                contract: &invalid_map_contract,
                schemas: &schemas,
                inputs: &[
                    test_layout(
                        &schemas,
                        *map,
                        ResidentValueKind::Snapshot,
                        ResidentShape::SCALAR,
                    ),
                    test_layout(
                        &schemas,
                        *index_schema,
                        ResidentValueKind::Index,
                        ResidentShape::SCALAR,
                    ),
                ],
                output: test_layout(
                    &schemas,
                    *u64_schema,
                    ResidentValueKind::Snapshot,
                    ResidentShape::SCALAR,
                ),
            }),
            Err(ResidentKernelBindError::UnsupportedLayout),
        ));
        let run_scalar = |source_schema,
                          selector_schema,
                          selector_kind,
                          source_value: mech_core::Value,
                          selector_ref: ResidentValueRef<'_>| {
            let contract = access_contract(source_schema, selector_schema, *u64_schema);
            let kernel = bind_snapshot_access(&ResidentKernelBindRequest {
                contract: &contract,
                schemas: &schemas,
                inputs: &[
                    test_layout(
                        &schemas,
                        source_schema,
                        ResidentValueKind::Snapshot,
                        ResidentShape::SCALAR,
                    ),
                    test_layout(
                        &schemas,
                        selector_schema,
                        selector_kind,
                        ResidentShape::SCALAR,
                    ),
                ],
                output: test_layout(
                    &schemas,
                    *u64_schema,
                    ResidentValueKind::Snapshot,
                    ResidentShape::SCALAR,
                ),
            })
            .unwrap();
            let source = [Some(source_value)];
            let inputs = [ResidentValueRef::Snapshot(&source), selector_ref];
            let mut output = [None];
            kernel
                .execute(&Inputs(&inputs), ResidentValueMut::Snapshot(&mut output))
                .unwrap_or_else(|error| {
                    panic!("snapshot access for schema {source_schema:?} failed: {error:?}")
                });
            match output[0].as_ref().unwrap().data() {
                ValueData::U64(value) => *value,
                _ => panic!("access output must be u64"),
            }
        };

        let matrix_value = test_value(
            &schemas,
            *matrix,
            ValueDataDraft::Matrix(
                [1, 2, 3, 4]
                    .into_iter()
                    .map(ValueDataDraft::U64)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
        );
        let linear = [2_u64];
        assert_eq!(
            run_scalar(
                *matrix,
                *index_schema,
                ResidentValueKind::Index,
                matrix_value,
                ResidentValueRef::Index(&linear),
            ),
            3,
        );

        let record_value = test_value(
            &schemas,
            *record,
            ValueDataDraft::Record(
                vec![mech_core::snapshot::NamedValueDraft {
                    name: "number".to_owned(),
                    value: ValueDataDraft::U64(7),
                }]
                .into_boxed_slice(),
            ),
        );
        let record_selector = [Some(test_value(
            &schemas,
            *id_schema,
            ValueDataDraft::Id(mech_core::hash_str("number")),
        ))];
        assert_eq!(
            run_scalar(
                *record,
                *id_schema,
                ResidentValueKind::Snapshot,
                record_value,
                ResidentValueRef::Snapshot(&record_selector),
            ),
            7,
        );

        let map_value = test_value(
            &schemas,
            *map,
            ValueDataDraft::Map(
                vec![mech_core::snapshot::MapEntryDraft {
                    items: vec![
                        ValueDataDraft::String("key".to_owned()),
                        ValueDataDraft::U64(11),
                    ]
                    .into_boxed_slice(),
                }]
                .into_boxed_slice(),
            ),
        );
        let key = ["key".to_owned()];
        assert_eq!(
            run_scalar(
                *map,
                *string_schema,
                ResidentValueKind::String,
                map_value,
                ResidentValueRef::String(&key),
            ),
            11,
        );

        let contract = access_contract(*table, *id_schema, *column);
        let kernel = bind_snapshot_access(&ResidentKernelBindRequest {
            contract: &contract,
            schemas: &schemas,
            inputs: &[
                test_layout(
                    &schemas,
                    *table,
                    ResidentValueKind::Snapshot,
                    ResidentShape::SCALAR,
                ),
                test_layout(
                    &schemas,
                    *id_schema,
                    ResidentValueKind::Snapshot,
                    ResidentShape::SCALAR,
                ),
            ],
            output: test_layout(
                &schemas,
                *column,
                ResidentValueKind::Snapshot,
                ResidentShape::SCALAR,
            ),
        })
        .unwrap();
        let table_value = [Some(test_value(
            &schemas,
            *table,
            ValueDataDraft::Table(
                vec![mech_core::snapshot::TableColumnDraft {
                    name: "number".to_owned(),
                    values: vec![ValueDataDraft::U64(5), ValueDataDraft::U64(6)].into_boxed_slice(),
                }]
                .into_boxed_slice(),
            ),
        ))];
        let column_selector = [Some(test_value(
            &schemas,
            *id_schema,
            ValueDataDraft::Id(mech_core::hash_str("number")),
        ))];
        let inputs = [
            ResidentValueRef::Snapshot(&table_value),
            ResidentValueRef::Snapshot(&column_selector),
        ];
        let mut output = [None];
        kernel
            .execute(&Inputs(&inputs), ResidentValueMut::Snapshot(&mut output))
            .unwrap();
        let ValueData::Matrix(values) = output[0].as_ref().unwrap().data() else {
            panic!("table access must produce a column matrix")
        };
        assert!(matches!(
            values.elements().to_values().as_slice(),
            [ValueData::U64(5), ValueData::U64(6)]
        ));
    }

    #[test]
    fn map_access_plan_charges_every_canonical_key_examined() {
        let string_body = SchemaBody::String;
        let u64_body = SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64);
        let map_body = SchemaBody::Map {
            key: Box::new(string_body.clone()),
            value: Box::new(u64_body.clone()),
            cardinality: mech_core::CardinalitySpec::Dynamic { upper_bound: None },
        };
        let (schemas, ids) = test_schema_table([string_body.clone(), u64_body, map_body]);
        let [_, _, map_schema] = ids.as_slice() else {
            unreachable!()
        };
        let map = test_value(
            &schemas,
            *map_schema,
            ValueDataDraft::Map(
                ["alpha", "beta", "gamma"]
                    .into_iter()
                    .enumerate()
                    .map(|(index, key)| mech_core::snapshot::MapEntryDraft {
                        items: vec![
                            ValueDataDraft::String(key.to_owned()),
                            ValueDataDraft::U64(index as u64),
                        ]
                        .into_boxed_slice(),
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
        );
        let ValueData::Map(map) = map.data() else {
            unreachable!()
        };
        let selector = ["gamma".to_owned()];
        let (ordinal, comparison_work) =
            map_access_entry_for_selector(map, &string_body, ResidentValueRef::String(&selector))
                .unwrap();
        assert_eq!(ordinal, 2);
        let expected_work = map
            .entries()
            .iter()
            .map(|entry| {
                mech_core::snapshot::canonical_data_retained_footprint(
                    &string_body,
                    entry.key().data(),
                )
                .unwrap()
            })
            .map(|footprint| footprint.encoded_bytes.max(footprint.node_count).max(1))
            .sum::<u64>()
            + (selector[0].len() as u64 + 8) * (map.entries().len() as u64 + 1);
        assert_eq!(comparison_work, expected_work);
    }

    #[test]
    fn record_name_lookup_is_incrementally_metered_before_materialization() {
        let field_name = "x".repeat(
            usize::try_from(super::super::budget::MAX_RESIDENT_COMPARISON_WORK).unwrap() + 1,
        );
        let u64_body = SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64);
        let record_body = SchemaBody::Record(
            vec![mech_core::SchemaField {
                name: field_name.clone(),
                schema: u64_body.clone(),
            }]
            .into_boxed_slice(),
        );
        let (schemas, ids) = test_schema_table([u64_body, record_body, SchemaBody::Id]);
        let [u64_schema, record_schema, id_schema] = ids.as_slice() else {
            unreachable!()
        };
        let contract = test_contract(
            &[*record_schema, *id_schema],
            *u64_schema,
            OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            AccessMode::Write,
            AliasPolicy::NoAlias,
            ChangeDetectionPolicy::KernelReported,
        );
        let kernel = bind_snapshot_access(&ResidentKernelBindRequest {
            contract: &contract,
            schemas: &schemas,
            inputs: &[
                test_layout(
                    &schemas,
                    *record_schema,
                    ResidentValueKind::Snapshot,
                    ResidentShape::SCALAR,
                ),
                test_layout(
                    &schemas,
                    *id_schema,
                    ResidentValueKind::Snapshot,
                    ResidentShape::SCALAR,
                ),
            ],
            output: test_layout(
                &schemas,
                *u64_schema,
                ResidentValueKind::Snapshot,
                ResidentShape::SCALAR,
            ),
        })
        .unwrap();
        let record = [Some(test_value(
            &schemas,
            *record_schema,
            ValueDataDraft::Record(
                vec![mech_core::snapshot::NamedValueDraft {
                    name: field_name,
                    value: ValueDataDraft::U64(7),
                }]
                .into_boxed_slice(),
            ),
        ))];
        let selector = [Some(test_value(
            &schemas,
            *id_schema,
            ValueDataDraft::Id(mech_core::hash_str("missing")),
        ))];
        let inputs = [
            ResidentValueRef::Snapshot(&record),
            ResidentValueRef::Snapshot(&selector),
        ];
        let previous = test_value(&schemas, *u64_schema, ValueDataDraft::U64(99));
        let mut output = [Some(previous.clone())];

        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::Snapshot(&mut output)),
            Err(ResidentKernelError::InvalidShape),
        );
        assert!(
            output[0]
                .as_ref()
                .unwrap()
                .language_eq(&schemas, &previous, &schemas)
                .unwrap()
        );
    }

    #[test]
    fn dense_access_and_assignment_accept_typed_fractional_selectors() {
        let matrix = |element, rows, columns| SchemaBody::Matrix {
            element: Box::new(element),
            dimensions: vec![
                mech_core::DimensionExpr::Constant(rows),
                mech_core::DimensionExpr::Constant(columns),
            ]
            .into_boxed_slice(),
        };
        let f64_body = SchemaBody::FloatingPoint(mech_core::FloatWidth::W64);
        let u64_body = SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64);
        let f32_body = SchemaBody::FloatingPoint(mech_core::FloatWidth::W32);
        let (schemas, ids) = test_schema_table([
            f64_body.clone(),
            matrix(f64_body.clone(), 1, 3),
            matrix(f64_body.clone(), 2, 1),
            matrix(u64_body, 1, 2),
            matrix(f32_body, 1, 2),
            matrix(f64_body.clone(), 2, 3),
            matrix(f64_body.clone(), 2, 2),
        ]);
        let [
            f64_schema,
            source_schema,
            output_schema,
            u64_selector_schema,
            f32_selector_schema,
            matrix_2x3_schema,
            matrix_2x2_schema,
        ] = ids.as_slice()
        else {
            unreachable!()
        };
        let source_layout = test_layout(
            &schemas,
            *source_schema,
            ResidentValueKind::F64,
            ResidentShape {
                rows: 1,
                columns: 3,
            },
        );
        let output_layout = test_layout(
            &schemas,
            *output_schema,
            ResidentValueKind::F64,
            ResidentShape {
                rows: 2,
                columns: 1,
            },
        );
        let gather_contract = test_contract(
            &[*source_schema, *u64_selector_schema],
            *output_schema,
            OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            AccessMode::Write,
            AliasPolicy::NoAlias,
            ChangeDetectionPolicy::KernelReported,
        );
        let u64_selector_layout = test_layout(
            &schemas,
            *u64_selector_schema,
            ResidentValueKind::Snapshot,
            ResidentShape::SCALAR,
        );
        let kernel = bind_gather_1d(&ResidentKernelBindRequest {
            contract: &gather_contract,
            schemas: &schemas,
            inputs: &[source_layout.clone(), u64_selector_layout],
            output: output_layout.clone(),
        })
        .unwrap();
        let u64_selector = [Some(test_value(
            &schemas,
            *u64_selector_schema,
            ValueDataDraft::Matrix(
                vec![ValueDataDraft::U64(3), ValueDataDraft::U64(1)].into_boxed_slice(),
            ),
        ))];
        let source = [10.0, 20.0, 30.0];
        let inputs = [
            ResidentValueRef::F64(&source),
            ResidentValueRef::Snapshot(&u64_selector),
        ];
        let mut output = [0.0; 2];
        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::F64(&mut output)),
            Ok(true),
        );
        assert_eq!(output, [30.0, 10.0]);
        let changed_selector = [Some(test_value(
            &schemas,
            *u64_selector_schema,
            ValueDataDraft::Matrix(
                vec![ValueDataDraft::U64(2), ValueDataDraft::U64(2)].into_boxed_slice(),
            ),
        ))];
        let changed_inputs = [
            ResidentValueRef::F64(&source),
            ResidentValueRef::Snapshot(&changed_selector),
        ];
        assert_eq!(
            kernel.execute(&Inputs(&changed_inputs), ResidentValueMut::F64(&mut output),),
            Ok(true),
        );
        assert_eq!(output, [20.0, 20.0]);

        let fractional_contract = test_contract(
            &[*source_schema, *f32_selector_schema],
            *output_schema,
            OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            AccessMode::Write,
            AliasPolicy::NoAlias,
            ChangeDetectionPolicy::KernelReported,
        );
        let fractional_layout = test_layout(
            &schemas,
            *f32_selector_schema,
            ResidentValueKind::Snapshot,
            ResidentShape::SCALAR,
        );
        let kernel = bind_gather_1d(&ResidentKernelBindRequest {
            contract: &fractional_contract,
            schemas: &schemas,
            inputs: &[source_layout.clone(), fractional_layout.clone()],
            output: output_layout,
        })
        .unwrap();
        let fractional_selector = [Some(test_value(
            &schemas,
            *f32_selector_schema,
            ValueDataDraft::Matrix(
                vec![
                    ValueDataDraft::F32(F32Bits::from_f32(2.9)),
                    ValueDataDraft::F32(F32Bits::from_f32(1.1)),
                ]
                .into_boxed_slice(),
            ),
        ))];
        let inputs = [
            ResidentValueRef::F64(&source),
            ResidentValueRef::Snapshot(&fractional_selector),
        ];
        let mut output = [0.0; 2];
        kernel
            .execute(&Inputs(&inputs), ResidentValueMut::F64(&mut output))
            .unwrap();
        assert_eq!(output, [20.0, 10.0]);

        let assignment_contract = test_contract(
            &[*source_schema, *f64_schema, *u64_selector_schema],
            *source_schema,
            OutputConstruction::ReadModifyWrite {
                base_input: 0,
                regions: RegionPolicy::IndexedAxis { axis: 0 },
            },
            AccessMode::ReadWrite,
            AliasPolicy::MayAlias { input: 0 },
            ChangeDetectionPolicy::KernelReported,
        );
        let kernel = bind_indexed_assign_with_region(
            &ResidentKernelBindRequest {
                contract: &assignment_contract,
                schemas: &schemas,
                inputs: &[
                    source_layout.clone(),
                    test_layout(
                        &schemas,
                        *f64_schema,
                        ResidentValueKind::F64,
                        ResidentShape::SCALAR,
                    ),
                    test_layout(
                        &schemas,
                        *u64_selector_schema,
                        ResidentValueKind::Snapshot,
                        ResidentShape::SCALAR,
                    ),
                ],
                output: source_layout,
            },
            RegionPolicy::IndexedAxis { axis: 0 },
        )
        .unwrap();
        let replacement = [9.0];
        let inputs = [
            ResidentValueRef::F64(&replacement),
            ResidentValueRef::Snapshot(&u64_selector),
        ];
        let mut output = [10.0, 20.0, 30.0];
        kernel
            .execute(&Inputs(&inputs), ResidentValueMut::F64(&mut output))
            .unwrap();
        assert_eq!(output, [9.0, 20.0, 9.0]);

        let matrix_2x3_layout = test_layout(
            &schemas,
            *matrix_2x3_schema,
            ResidentValueKind::F64,
            ResidentShape {
                rows: 2,
                columns: 3,
            },
        );
        let matrix_2x2_layout = test_layout(
            &schemas,
            *matrix_2x2_schema,
            ResidentValueKind::F64,
            ResidentShape {
                rows: 2,
                columns: 2,
            },
        );
        let row_contract = test_contract(
            &[*matrix_2x3_schema, *f32_selector_schema],
            *matrix_2x3_schema,
            OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            AccessMode::Write,
            AliasPolicy::NoAlias,
            ChangeDetectionPolicy::KernelReported,
        );
        let kernel = bind_rows_all_columns(&ResidentKernelBindRequest {
            contract: &row_contract,
            schemas: &schemas,
            inputs: &[matrix_2x3_layout.clone(), fractional_layout.clone()],
            output: matrix_2x3_layout.clone(),
        })
        .unwrap();
        let source = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let inputs = [
            ResidentValueRef::F64(&source),
            ResidentValueRef::Snapshot(&fractional_selector),
        ];
        let mut output = [0.0; 6];
        kernel
            .execute(&Inputs(&inputs), ResidentValueMut::F64(&mut output))
            .unwrap();
        assert_eq!(output, [2.0, 1.0, 4.0, 3.0, 6.0, 5.0]);

        let column_contract = test_contract(
            &[*matrix_2x3_schema, *u64_selector_schema],
            *matrix_2x2_schema,
            OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            AccessMode::Write,
            AliasPolicy::NoAlias,
            ChangeDetectionPolicy::KernelReported,
        );
        let kernel = bind_all_rows_columns(&ResidentKernelBindRequest {
            contract: &column_contract,
            schemas: &schemas,
            inputs: &[
                matrix_2x3_layout.clone(),
                test_layout(
                    &schemas,
                    *u64_selector_schema,
                    ResidentValueKind::Snapshot,
                    ResidentShape::SCALAR,
                ),
            ],
            output: matrix_2x2_layout,
        })
        .unwrap();
        let inputs = [
            ResidentValueRef::F64(&source),
            ResidentValueRef::Snapshot(&u64_selector),
        ];
        let mut output = [0.0; 4];
        kernel
            .execute(&Inputs(&inputs), ResidentValueMut::F64(&mut output))
            .unwrap();
        assert_eq!(output, [5.0, 6.0, 1.0, 2.0]);

        let rectangle_contract = test_contract(
            &[
                *matrix_2x3_schema,
                *f64_schema,
                *f32_selector_schema,
                *u64_selector_schema,
            ],
            *matrix_2x3_schema,
            OutputConstruction::ReadModifyWrite {
                base_input: 0,
                regions: RegionPolicy::RectangularRegion,
            },
            AccessMode::ReadWrite,
            AliasPolicy::MayAlias { input: 0 },
            ChangeDetectionPolicy::KernelReported,
        );
        let kernel = bind_indexed_assign_rectangle(&ResidentKernelBindRequest {
            contract: &rectangle_contract,
            schemas: &schemas,
            inputs: &[
                matrix_2x3_layout.clone(),
                test_layout(
                    &schemas,
                    *f64_schema,
                    ResidentValueKind::F64,
                    ResidentShape::SCALAR,
                ),
                fractional_layout,
                test_layout(
                    &schemas,
                    *u64_selector_schema,
                    ResidentValueKind::Snapshot,
                    ResidentShape::SCALAR,
                ),
            ],
            output: matrix_2x3_layout,
        })
        .unwrap();
        let replacement = [9.0];
        let inputs = [
            ResidentValueRef::F64(&replacement),
            ResidentValueRef::Snapshot(&fractional_selector),
            ResidentValueRef::Snapshot(&u64_selector),
        ];
        let mut output = source;
        kernel
            .execute(&Inputs(&inputs), ResidentValueMut::F64(&mut output))
            .unwrap();
        assert_eq!(output, [9.0, 9.0, 3.0, 4.0, 9.0, 9.0]);
    }

    #[test]
    fn access_binders_reject_same_cardinality_wrong_output_dimensions() {
        let matrix = |element, rows, columns| SchemaBody::Matrix {
            element: Box::new(element),
            dimensions: vec![
                mech_core::DimensionExpr::Constant(rows),
                mech_core::DimensionExpr::Constant(columns),
            ]
            .into_boxed_slice(),
        };
        let f64_body = SchemaBody::FloatingPoint(mech_core::FloatWidth::W64);
        let u64_body = SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64);
        let table_body = SchemaBody::Table {
            columns: vec![mech_core::SchemaField {
                name: "number".to_owned(),
                schema: u64_body.clone(),
            }]
            .into_boxed_slice(),
            rows: mech_core::CardinalitySpec::Exact(mech_core::DimensionExpr::Constant(2)),
        };
        let (schemas, ids) = test_schema_table([
            u64_body.clone(),
            SchemaBody::Id,
            matrix(u64_body.clone(), 2, 1),
            matrix(f64_body.clone(), 1, 3),
            matrix(f64_body.clone(), 2, 1),
            matrix(f64_body.clone(), 2, 3),
            matrix(f64_body.clone(), 2, 2),
            matrix(u64_body.clone(), 2, 3),
            matrix(u64_body.clone(), 3, 2),
            matrix(u64_body.clone(), 2, 2),
            matrix(u64_body.clone(), 1, 4),
            matrix(u64_body.clone(), 1, 2),
            matrix(u64_body.clone(), 3, 1),
            table_body,
        ]);
        let [
            u64_schema,
            id_schema,
            selector_2x1,
            dense_1x3,
            dense_2x1,
            dense_2x3,
            dense_2x2,
            snapshot_2x3,
            snapshot_3x2,
            snapshot_2x2,
            snapshot_1x4,
            snapshot_1x2,
            snapshot_3x1,
            table,
        ] = ids.as_slice()
        else {
            unreachable!()
        };
        let contract = |source, selectors: &[mech_core::SchemaId], output| {
            let mut inputs = vec![source];
            inputs.extend_from_slice(selectors);
            test_contract(
                &inputs,
                output,
                OutputConstruction::FullWrite {
                    shape: ShapeRule::Declared,
                },
                AccessMode::Write,
                AliasPolicy::NoAlias,
                ChangeDetectionPolicy::KernelReported,
            )
        };
        let dense_layout = |schema, rows, columns| {
            test_layout(
                &schemas,
                schema,
                ResidentValueKind::F64,
                ResidentShape { rows, columns },
            )
        };
        let snapshot_layout = |schema| {
            test_layout(
                &schemas,
                schema,
                ResidentValueKind::Snapshot,
                ResidentShape::SCALAR,
            )
        };
        let selector_layout = snapshot_layout(*selector_2x1);

        let gather_contract = contract(*dense_1x3, &[*selector_2x1], *dense_2x1);
        assert!(
            bind_gather_1d(&ResidentKernelBindRequest {
                contract: &gather_contract,
                schemas: &schemas,
                inputs: &[dense_layout(*dense_1x3, 1, 3), selector_layout.clone()],
                output: dense_layout(*dense_2x1, 2, 1),
            })
            .is_ok()
        );
        assert!(matches!(
            bind_gather_1d(&ResidentKernelBindRequest {
                contract: &gather_contract,
                schemas: &schemas,
                inputs: &[dense_layout(*dense_1x3, 1, 3), selector_layout.clone()],
                output: dense_layout(*dense_2x1, 1, 2),
            }),
            Err(ResidentKernelBindError::UnsupportedLayout)
        ));

        let row_contract = contract(*dense_2x3, &[*selector_2x1], *dense_2x3);
        assert!(
            bind_rows_all_columns(&ResidentKernelBindRequest {
                contract: &row_contract,
                schemas: &schemas,
                inputs: &[dense_layout(*dense_2x3, 2, 3), selector_layout.clone()],
                output: dense_layout(*dense_2x3, 2, 3),
            })
            .is_ok()
        );
        assert!(matches!(
            bind_rows_all_columns(&ResidentKernelBindRequest {
                contract: &row_contract,
                schemas: &schemas,
                inputs: &[dense_layout(*dense_2x3, 2, 3), selector_layout.clone()],
                output: dense_layout(*dense_2x3, 3, 2),
            }),
            Err(ResidentKernelBindError::UnsupportedLayout)
        ));

        let column_contract = contract(*dense_2x3, &[*selector_2x1], *dense_2x2);
        assert!(
            bind_all_rows_columns(&ResidentKernelBindRequest {
                contract: &column_contract,
                schemas: &schemas,
                inputs: &[dense_layout(*dense_2x3, 2, 3), selector_layout.clone()],
                output: dense_layout(*dense_2x2, 2, 2),
            })
            .is_ok()
        );
        assert!(matches!(
            bind_all_rows_columns(&ResidentKernelBindRequest {
                contract: &column_contract,
                schemas: &schemas,
                inputs: &[dense_layout(*dense_2x3, 2, 3), selector_layout.clone()],
                output: dense_layout(*dense_2x2, 1, 4),
            }),
            Err(ResidentKernelBindError::UnsupportedLayout)
        ));

        let scalar_row_contract = contract(*dense_2x3, &[*u64_schema], *dense_1x3);
        assert!(matches!(
            bind_row_all_columns(&ResidentKernelBindRequest {
                contract: &scalar_row_contract,
                schemas: &schemas,
                inputs: &[dense_layout(*dense_2x3, 2, 3), snapshot_layout(*u64_schema),],
                output: dense_layout(*dense_1x3, 3, 1),
            }),
            Err(ResidentKernelBindError::UnsupportedLayout)
        ));

        let snapshot_cases = [
            (
                ResolvedSelectionMode::LinearGather,
                vec![*selector_2x1],
                *selector_2x1,
                *snapshot_1x2,
            ),
            (
                ResolvedSelectionMode::Rows,
                vec![*selector_2x1],
                *snapshot_2x3,
                *snapshot_3x2,
            ),
            (
                ResolvedSelectionMode::Columns,
                vec![*selector_2x1],
                *snapshot_2x2,
                *snapshot_1x4,
            ),
            (
                ResolvedSelectionMode::Rectangle,
                vec![*selector_2x1, *selector_2x1],
                *snapshot_2x2,
                *snapshot_1x4,
            ),
        ];
        for (mode, selectors, valid_output, invalid_output) in snapshot_cases {
            let mut inputs = vec![snapshot_layout(*snapshot_2x3)];
            inputs.extend(selectors.iter().map(|schema| snapshot_layout(*schema)));
            let valid_contract = contract(*snapshot_2x3, &selectors, valid_output);
            assert!(
                bind_snapshot_access_mode(
                    &ResidentKernelBindRequest {
                        contract: &valid_contract,
                        schemas: &schemas,
                        inputs: &inputs,
                        output: snapshot_layout(valid_output),
                    },
                    Some(mode),
                )
                .is_ok()
            );
            let invalid_contract = contract(*snapshot_2x3, &selectors, invalid_output);
            assert!(matches!(
                bind_snapshot_access_mode(
                    &ResidentKernelBindRequest {
                        contract: &invalid_contract,
                        schemas: &schemas,
                        inputs: &inputs,
                        output: snapshot_layout(invalid_output),
                    },
                    Some(mode),
                ),
                Err(ResidentKernelBindError::UnsupportedLayout)
            ));
        }

        let valid_table_contract = contract(*table, &[*id_schema], *selector_2x1);
        assert!(
            bind_snapshot_access(&ResidentKernelBindRequest {
                contract: &valid_table_contract,
                schemas: &schemas,
                inputs: &[snapshot_layout(*table), snapshot_layout(*id_schema)],
                output: snapshot_layout(*selector_2x1),
            })
            .is_ok()
        );
        for invalid_output in [*snapshot_1x2, *snapshot_3x1] {
            let table_contract = contract(*table, &[*id_schema], invalid_output);
            assert!(matches!(
                bind_snapshot_access(&ResidentKernelBindRequest {
                    contract: &table_contract,
                    schemas: &schemas,
                    inputs: &[snapshot_layout(*table), snapshot_layout(*id_schema)],
                    output: snapshot_layout(invalid_output),
                }),
                Err(ResidentKernelBindError::UnsupportedLayout)
            ));
        }
    }

    #[test]
    fn snapshot_access_geometry_enforces_masks_physical_shape_and_table_bounds() {
        let matrix = |element, rows, columns| SchemaBody::Matrix {
            element: Box::new(element),
            dimensions: vec![
                mech_core::DimensionExpr::Constant(rows),
                mech_core::DimensionExpr::Constant(columns),
            ]
            .into_boxed_slice(),
        };
        let f64_body = SchemaBody::FloatingPoint(mech_core::FloatWidth::W64);
        let u64_body = SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64);
        let exact_f64_table = SchemaBody::Table {
            columns: vec![mech_core::SchemaField {
                name: "number".to_owned(),
                schema: f64_body.clone(),
            }]
            .into_boxed_slice(),
            rows: mech_core::CardinalitySpec::Exact(mech_core::DimensionExpr::Constant(2)),
        };
        let bounded_u64_table = SchemaBody::Table {
            columns: vec![mech_core::SchemaField {
                name: "number".to_owned(),
                schema: u64_body.clone(),
            }]
            .into_boxed_slice(),
            rows: mech_core::CardinalitySpec::Dynamic {
                upper_bound: Some(mech_core::DimensionExpr::Constant(5)),
            },
        };
        let (schemas, ids) = test_schema_table([
            SchemaBody::Id,
            matrix(SchemaBody::Bool, 2, 1),
            matrix(SchemaBody::Bool, 3, 1),
            matrix(SchemaBody::Bool, 5, 1),
            matrix(SchemaBody::Bool, 6, 1),
            matrix(u64_body.clone(), 2, 1),
            matrix(u64_body.clone(), 1, 1),
            matrix(u64_body.clone(), 1, 2),
            matrix(u64_body.clone(), 1, 3),
            matrix(u64_body.clone(), 2, 2),
            matrix(u64_body.clone(), 2, 3),
            matrix(u64_body.clone(), 5, 1),
            matrix(u64_body, 6, 1),
            matrix(f64_body.clone(), 2, 1),
            matrix(f64_body, 2, 3),
            exact_f64_table,
            bounded_u64_table,
        ]);
        let [
            id_schema,
            bool_2x1,
            bool_3x1,
            bool_5x1,
            bool_6x1,
            u64_2x1,
            u64_1x1,
            u64_1x2,
            u64_1x3,
            u64_2x2,
            u64_2x3,
            u64_5x1,
            u64_6x1,
            f64_2x1,
            f64_2x3,
            f64_table,
            bounded_table,
        ] = ids.as_slice()
        else {
            unreachable!()
        };
        let contract = |source, selectors: &[mech_core::SchemaId], output| {
            let mut inputs = vec![source];
            inputs.extend_from_slice(selectors);
            test_contract(
                &inputs,
                output,
                OutputConstruction::FullWrite {
                    shape: ShapeRule::Declared,
                },
                AccessMode::Write,
                AliasPolicy::NoAlias,
                ChangeDetectionPolicy::KernelReported,
            )
        };
        let snapshot_layout = |schema| {
            test_layout(
                &schemas,
                schema,
                ResidentValueKind::Snapshot,
                ResidentShape::SCALAR,
            )
        };
        let dense_layout = |schema, rows, columns| {
            test_layout(
                &schemas,
                schema,
                ResidentValueKind::F64,
                ResidentShape { rows, columns },
            )
        };
        let bind_matrix =
            |source, selectors: &[mech_core::SchemaId], output, output_layout, mode| {
                let contract = contract(source, selectors, output);
                let mut inputs = vec![snapshot_layout(source)];
                inputs.extend(selectors.iter().map(|schema| snapshot_layout(*schema)));
                bind_snapshot_access_mode(
                    &ResidentKernelBindRequest {
                        contract: &contract,
                        schemas: &schemas,
                        inputs: &inputs,
                        output: output_layout,
                    },
                    Some(mode),
                )
            };

        assert!(matches!(
            bind_matrix(
                *u64_2x3,
                &[*bool_3x1],
                *u64_1x3,
                snapshot_layout(*u64_1x3),
                ResolvedSelectionMode::Rows,
            ),
            Err(ResidentKernelBindError::UnsupportedLayout)
        ));
        assert!(matches!(
            bind_matrix(
                *u64_2x2,
                &[*bool_3x1],
                *u64_2x1,
                snapshot_layout(*u64_2x1),
                ResolvedSelectionMode::Columns,
            ),
            Err(ResidentKernelBindError::UnsupportedLayout)
        ));
        assert!(matches!(
            bind_matrix(
                *u64_2x3,
                &[*bool_5x1],
                *u64_1x1,
                snapshot_layout(*u64_1x1),
                ResolvedSelectionMode::LinearGather,
            ),
            Err(ResidentKernelBindError::UnsupportedLayout)
        ));
        assert!(matches!(
            bind_matrix(
                *u64_2x3,
                &[*bool_2x1, *bool_2x1],
                *u64_1x1,
                snapshot_layout(*u64_1x1),
                ResolvedSelectionMode::Rectangle,
            ),
            Err(ResidentKernelBindError::UnsupportedLayout)
        ));

        assert!(
            bind_matrix(
                *u64_2x3,
                &[*bool_2x1],
                *u64_1x3,
                snapshot_layout(*u64_1x3),
                ResolvedSelectionMode::Rows,
            )
            .is_ok()
        );
        assert!(
            bind_matrix(
                *u64_2x2,
                &[*bool_2x1],
                *u64_2x1,
                snapshot_layout(*u64_2x1),
                ResolvedSelectionMode::Columns,
            )
            .is_ok()
        );
        assert!(
            bind_matrix(
                *u64_2x3,
                &[*bool_6x1],
                *u64_2x1,
                snapshot_layout(*u64_2x1),
                ResolvedSelectionMode::LinearGather,
            )
            .is_ok()
        );
        assert!(
            bind_matrix(
                *u64_2x3,
                &[*bool_2x1, *bool_3x1],
                *u64_1x2,
                snapshot_layout(*u64_1x2),
                ResolvedSelectionMode::Rectangle,
            )
            .is_ok()
        );

        assert!(
            bind_matrix(
                *f64_2x3,
                &[*u64_2x1],
                *f64_2x3,
                dense_layout(*f64_2x3, 2, 3),
                ResolvedSelectionMode::Rows,
            )
            .is_ok()
        );
        assert!(matches!(
            bind_matrix(
                *f64_2x3,
                &[*u64_2x1],
                *f64_2x3,
                dense_layout(*f64_2x3, 3, 2),
                ResolvedSelectionMode::Rows,
            ),
            Err(ResidentKernelBindError::UnsupportedLayout)
        ));

        let bind_table = |table, output, output_layout| {
            let contract = contract(table, &[*id_schema], output);
            bind_snapshot_access(&ResidentKernelBindRequest {
                contract: &contract,
                schemas: &schemas,
                inputs: &[snapshot_layout(table), snapshot_layout(*id_schema)],
                output: output_layout,
            })
        };
        assert!(bind_table(*f64_table, *f64_2x1, dense_layout(*f64_2x1, 2, 1)).is_ok());
        assert!(matches!(
            bind_table(*f64_table, *f64_2x1, dense_layout(*f64_2x1, 1, 2)),
            Err(ResidentKernelBindError::UnsupportedLayout)
        ));
        assert!(bind_table(*bounded_table, *u64_5x1, snapshot_layout(*u64_5x1)).is_ok());
        assert!(matches!(
            bind_table(*bounded_table, *u64_6x1, snapshot_layout(*u64_6x1)),
            Err(ResidentKernelBindError::UnsupportedLayout)
        ));
    }

    #[test]
    fn missing_map_key_is_admitted_before_the_first_comparison() {
        let string_body = SchemaBody::String;
        let u64_body = SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64);
        let map_body = SchemaBody::Map {
            key: Box::new(string_body.clone()),
            value: Box::new(u64_body.clone()),
            cardinality: mech_core::CardinalitySpec::Dynamic { upper_bound: None },
        };
        let (schemas, ids) = test_schema_table([string_body.clone(), u64_body, map_body]);
        let [_, _, map_schema] = ids.as_slice() else {
            unreachable!()
        };
        let map = test_value(
            &schemas,
            *map_schema,
            ValueDataDraft::Map(
                vec![mech_core::snapshot::MapEntryDraft {
                    items: vec![
                        ValueDataDraft::String("x".repeat(
                            super::super::budget::MAX_RESIDENT_TEMPORARY_BYTES as usize + 1,
                        )),
                        ValueDataDraft::U64(1),
                    ]
                    .into_boxed_slice(),
                }]
                .into_boxed_slice(),
            ),
        );
        let ValueData::Map(map) = map.data() else {
            unreachable!()
        };
        let missing = ["missing".to_owned()];
        assert_eq!(
            map_access_entry_for_selector(map, &string_body, ResidentValueRef::String(&missing),),
            Err(ResidentKernelError::InvalidShape),
        );
    }

    #[test]
    fn late_map_key_stops_at_the_incremental_comparison_limit() {
        let string_body = SchemaBody::String;
        let u64_body = SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64);
        let map_body = SchemaBody::Map {
            key: Box::new(string_body.clone()),
            value: Box::new(u64_body.clone()),
            cardinality: mech_core::CardinalitySpec::Dynamic { upper_bound: None },
        };
        let (schemas, ids) = test_schema_table([string_body.clone(), u64_body, map_body]);
        let [_, _, map_schema] = ids.as_slice() else {
            unreachable!()
        };
        let entries = (0..5_000)
            .map(|index| mech_core::snapshot::MapEntryDraft {
                items: vec![
                    ValueDataDraft::String(format!("key-{index:05}")),
                    ValueDataDraft::U64(index),
                ]
                .into_boxed_slice(),
            })
            .collect::<Vec<_>>();
        let map = test_value(
            &schemas,
            *map_schema,
            ValueDataDraft::Map(entries.into_boxed_slice()),
        );
        let ValueData::Map(map) = map.data() else {
            unreachable!()
        };
        let late = ["key-04999".to_owned()];
        assert_eq!(
            map_access_entry_for_selector(map, &string_body, ResidentValueRef::String(&late)),
            Err(ResidentKernelError::InvalidShape),
        );
    }

    #[test]
    fn nested_map_key_stops_while_measuring_the_recursive_key() {
        let bool_body = SchemaBody::Bool;
        let key_body = SchemaBody::Tuple(
            vec![bool_body; super::super::budget::MAX_RESIDENT_RETAINED_NODES as usize + 1]
                .into_boxed_slice(),
        );
        let u64_body = SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64);
        let map_body = SchemaBody::Map {
            key: Box::new(key_body.clone()),
            value: Box::new(u64_body.clone()),
            cardinality: mech_core::CardinalitySpec::Dynamic { upper_bound: None },
        };
        let (schemas, ids) = test_schema_table([key_body.clone(), u64_body, map_body]);
        let [key_schema, _, map_schema] = ids.as_slice() else {
            unreachable!()
        };
        let key = (0..super::super::budget::MAX_RESIDENT_RETAINED_NODES as usize + 1)
            .map(|_| ValueDataDraft::Bool(false))
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let map = test_value(
            &schemas,
            *map_schema,
            ValueDataDraft::Map(
                vec![mech_core::snapshot::MapEntryDraft {
                    items: vec![ValueDataDraft::Tuple(key.clone()), ValueDataDraft::U64(1)]
                        .into_boxed_slice(),
                }]
                .into_boxed_slice(),
            ),
        );
        let selector = [Some(test_value(
            &schemas,
            *key_schema,
            ValueDataDraft::Tuple(key),
        ))];
        let ValueData::Map(map) = map.data() else {
            unreachable!()
        };
        assert_eq!(
            map_access_entry_for_selector(map, &key_body, ResidentValueRef::Snapshot(&selector),),
            Err(ResidentKernelError::InvalidShape),
        );
    }

    #[test]
    fn heterogeneous_snapshot_access_requires_a_resolved_aggregate_selector() {
        let u64_body = SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64);
        let tuple_body =
            SchemaBody::Tuple(vec![u64_body.clone(), SchemaBody::String].into_boxed_slice());
        let record_body = SchemaBody::Record(
            vec![
                mech_core::SchemaField {
                    name: "number".to_owned(),
                    schema: u64_body.clone(),
                },
                mech_core::SchemaField {
                    name: "name".to_owned(),
                    schema: SchemaBody::String,
                },
            ]
            .into_boxed_slice(),
        );
        let table_body = SchemaBody::Table {
            columns: vec![
                mech_core::SchemaField {
                    name: "number".to_owned(),
                    schema: u64_body.clone(),
                },
                mech_core::SchemaField {
                    name: "name".to_owned(),
                    schema: SchemaBody::String,
                },
            ]
            .into_boxed_slice(),
            rows: mech_core::CardinalitySpec::Exact(mech_core::DimensionExpr::Constant(2)),
        };
        let column_body = SchemaBody::Matrix {
            element: Box::new(u64_body.clone()),
            dimensions: vec![
                mech_core::DimensionExpr::Constant(2),
                mech_core::DimensionExpr::Constant(1),
            ]
            .into_boxed_slice(),
        };
        let (schemas, ids) = test_schema_table([
            u64_body,
            SchemaBody::String,
            SchemaBody::Index,
            SchemaBody::Id,
            tuple_body,
            record_body,
            table_body,
            column_body,
        ]);
        let [u64_schema, _string, index, id, tuple, record, table, column] = ids.as_slice() else {
            unreachable!()
        };
        let contract = |source, selector, output| {
            test_contract(
                &[source, selector],
                output,
                OutputConstruction::FullWrite {
                    shape: ShapeRule::Declared,
                },
                AccessMode::Write,
                AliasPolicy::NoAlias,
                ChangeDetectionPolicy::KernelReported,
            )
        };
        let bind = |source,
                    selector_schema,
                    selector: Option<mech_core::ResidentResolvedSelector>,
                    output| {
            let contract = contract(source, selector_schema, output);
            let mut selector_layout = test_layout(
                &schemas,
                selector_schema,
                if selector_schema == *index {
                    ResidentValueKind::Index
                } else {
                    ResidentValueKind::Snapshot
                },
                ResidentShape::SCALAR,
            );
            selector_layout.resolved_selector = selector;
            bind_snapshot_access(&ResidentKernelBindRequest {
                contract: &contract,
                schemas: &schemas,
                inputs: &[
                    test_layout(
                        &schemas,
                        source,
                        ResidentValueKind::Snapshot,
                        ResidentShape::SCALAR,
                    ),
                    selector_layout,
                ],
                output: test_layout(
                    &schemas,
                    output,
                    ResidentValueKind::Snapshot,
                    ResidentShape::SCALAR,
                ),
            })
        };

        assert!(matches!(
            bind(*tuple, *index, None, *u64_schema),
            Err(ResidentKernelBindError::UnsupportedLayout)
        ));
        let tuple_kernel = bind(
            *tuple,
            *index,
            Some(mech_core::ResidentResolvedSelector::Ordinal(0)),
            *u64_schema,
        )
        .unwrap();
        assert_eq!(
            tuple_kernel
                .retained_state::<SnapshotAccessPlan>()
                .unwrap()
                .aggregate_ordinal,
            Some(0)
        );
        assert!(matches!(
            bind(
                *tuple,
                *index,
                Some(mech_core::ResidentResolvedSelector::Ordinal(1)),
                *u64_schema,
            ),
            Err(ResidentKernelBindError::UnsupportedLayout)
        ));

        assert!(matches!(
            bind(*record, *id, None, *u64_schema),
            Err(ResidentKernelBindError::UnsupportedLayout)
        ));
        assert!(
            bind(
                *record,
                *id,
                Some(mech_core::ResidentResolvedSelector::Id(
                    mech_core::hash_str("number")
                )),
                *u64_schema,
            )
            .is_ok()
        );

        assert!(matches!(
            bind(*table, *id, None, *column),
            Err(ResidentKernelBindError::UnsupportedLayout)
        ));
        assert!(
            bind(
                *table,
                *id,
                Some(mech_core::ResidentResolvedSelector::Id(
                    mech_core::hash_str("number")
                )),
                *column,
            )
            .is_ok()
        );
    }

    #[test]
    fn heterogeneous_snapshot_assignment_requires_a_resolved_aggregate_selector() {
        let u64_body = SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64);
        let tuple_body =
            SchemaBody::Tuple(vec![u64_body.clone(), SchemaBody::String].into_boxed_slice());
        let record_body = SchemaBody::Record(
            vec![
                mech_core::SchemaField {
                    name: "number".to_owned(),
                    schema: u64_body.clone(),
                },
                mech_core::SchemaField {
                    name: "name".to_owned(),
                    schema: SchemaBody::String,
                },
            ]
            .into_boxed_slice(),
        );
        let table_body = SchemaBody::Table {
            columns: vec![
                mech_core::SchemaField {
                    name: "number".to_owned(),
                    schema: u64_body.clone(),
                },
                mech_core::SchemaField {
                    name: "name".to_owned(),
                    schema: SchemaBody::String,
                },
            ]
            .into_boxed_slice(),
            rows: mech_core::CardinalitySpec::Exact(mech_core::DimensionExpr::Constant(2)),
        };
        let column_body = SchemaBody::Matrix {
            element: Box::new(u64_body.clone()),
            dimensions: vec![
                mech_core::DimensionExpr::Constant(2),
                mech_core::DimensionExpr::Constant(1),
            ]
            .into_boxed_slice(),
        };
        let (schemas, ids) = test_schema_table([
            u64_body,
            SchemaBody::Index,
            SchemaBody::Id,
            tuple_body,
            record_body,
            table_body,
            column_body,
        ]);
        let [u64_schema, index, id, tuple, record, table, column] = ids.as_slice() else {
            unreachable!()
        };
        let bind = |base,
                    source,
                    selector_schema,
                    selector: Option<mech_core::ResidentResolvedSelector>| {
            let contract = test_contract(
                &[base, source, selector_schema],
                base,
                OutputConstruction::ReadModifyWrite {
                    base_input: 0,
                    regions: RegionPolicy::IndexedAxis { axis: 0 },
                },
                AccessMode::ReadWrite,
                AliasPolicy::MayAlias { input: 0 },
                ChangeDetectionPolicy::KernelReported,
            );
            let mut selector_layout = test_layout(
                &schemas,
                selector_schema,
                if selector_schema == *index {
                    ResidentValueKind::Index
                } else {
                    ResidentValueKind::Snapshot
                },
                ResidentShape::SCALAR,
            );
            selector_layout.resolved_selector = selector;
            bind_indexed_assign(&ResidentKernelBindRequest {
                contract: &contract,
                schemas: &schemas,
                inputs: &[
                    test_layout(
                        &schemas,
                        base,
                        ResidentValueKind::Snapshot,
                        ResidentShape::SCALAR,
                    ),
                    test_layout(
                        &schemas,
                        source,
                        ResidentValueKind::Snapshot,
                        ResidentShape::SCALAR,
                    ),
                    selector_layout,
                ],
                output: test_layout(
                    &schemas,
                    base,
                    ResidentValueKind::Snapshot,
                    ResidentShape::SCALAR,
                ),
            })
        };

        assert!(matches!(
            bind(*tuple, *u64_schema, *index, None),
            Err(ResidentKernelBindError::UnsupportedLayout)
        ));
        assert!(
            bind(
                *tuple,
                *u64_schema,
                *index,
                Some(mech_core::ResidentResolvedSelector::Ordinal(0)),
            )
            .is_ok()
        );
        assert!(matches!(
            bind(
                *tuple,
                *u64_schema,
                *index,
                Some(mech_core::ResidentResolvedSelector::Ordinal(1)),
            ),
            Err(ResidentKernelBindError::UnsupportedLayout)
        ));

        for (base, source) in [(*record, *u64_schema), (*table, *column)] {
            assert!(matches!(
                bind(base, source, *id, None),
                Err(ResidentKernelBindError::UnsupportedLayout)
            ));
            let kernel = bind(
                base,
                source,
                *id,
                Some(mech_core::ResidentResolvedSelector::Id(
                    mech_core::hash_str("number"),
                )),
            )
            .unwrap();
            assert_eq!(
                kernel
                    .retained_state::<SnapshotAggregateAssignPlan>()
                    .unwrap()
                    .aggregate_ordinal,
                Some(0)
            );
        }

        let record_kernel = bind(
            *record,
            *u64_schema,
            *id,
            Some(mech_core::ResidentResolvedSelector::Id(
                mech_core::hash_str("number"),
            )),
        )
        .unwrap();
        let source = [Some(test_value(
            &schemas,
            *u64_schema,
            ValueDataDraft::U64(9),
        ))];
        let changed_selector = [Some(test_value(
            &schemas,
            *id,
            ValueDataDraft::Id(mech_core::hash_str("name")),
        ))];
        let current = test_value(
            &schemas,
            *record,
            ValueDataDraft::Record(
                vec![
                    mech_core::snapshot::NamedValueDraft {
                        name: "number".to_owned(),
                        value: ValueDataDraft::U64(1),
                    },
                    mech_core::snapshot::NamedValueDraft {
                        name: "name".to_owned(),
                        value: ValueDataDraft::String("original".to_owned()),
                    },
                ]
                .into_boxed_slice(),
            ),
        );
        let inputs = [
            ResidentValueRef::Snapshot(&source),
            ResidentValueRef::Snapshot(&changed_selector),
        ];
        let mut output = [Some(current.clone())];
        assert_eq!(
            record_kernel.execute(&Inputs(&inputs), ResidentValueMut::Snapshot(&mut output)),
            Err(ResidentKernelError::InvalidInput),
        );
        assert!(
            output[0]
                .as_ref()
                .unwrap()
                .language_eq(&schemas, &current, &schemas)
                .unwrap()
        );
    }

    #[test]
    fn snapshot_access_counts_duplicate_aggregate_payloads_before_cloning() {
        let source_body = SchemaBody::Matrix {
            element: Box::new(SchemaBody::String),
            dimensions: vec![
                mech_core::DimensionExpr::Constant(1),
                mech_core::DimensionExpr::Constant(1),
            ]
            .into_boxed_slice(),
        };
        let selector_body = SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: vec![
                mech_core::DimensionExpr::Constant(1),
                mech_core::DimensionExpr::Constant(20),
            ]
            .into_boxed_slice(),
        };
        let output_body = SchemaBody::Matrix {
            element: Box::new(SchemaBody::String),
            dimensions: vec![
                mech_core::DimensionExpr::Constant(20),
                mech_core::DimensionExpr::Constant(1),
            ]
            .into_boxed_slice(),
        };
        let (schemas, ids) = test_schema_table([source_body, selector_body, output_body]);
        let [source_schema, selector_schema, output_schema] = ids.as_slice() else {
            unreachable!()
        };
        let contract = test_contract(
            &[*source_schema, *selector_schema],
            *output_schema,
            OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            AccessMode::Write,
            AliasPolicy::NoAlias,
            ChangeDetectionPolicy::KernelReported,
        );
        let kernel = bind_snapshot_access(&ResidentKernelBindRequest {
            contract: &contract,
            schemas: &schemas,
            inputs: &[
                test_layout(
                    &schemas,
                    *source_schema,
                    ResidentValueKind::Snapshot,
                    ResidentShape::SCALAR,
                ),
                test_layout(
                    &schemas,
                    *selector_schema,
                    ResidentValueKind::Index,
                    ResidentShape {
                        rows: 1,
                        columns: 20,
                    },
                ),
            ],
            output: test_layout(
                &schemas,
                *output_schema,
                ResidentValueKind::Snapshot,
                ResidentShape::SCALAR,
            ),
        })
        .unwrap();
        let source = [Some(test_value(
            &schemas,
            *source_schema,
            ValueDataDraft::Matrix(
                vec![ValueDataDraft::String("x".repeat(1024 * 1024))].into_boxed_slice(),
            ),
        ))];
        let selector = [1_u64; 20];
        let inputs = [
            ResidentValueRef::Snapshot(&source),
            ResidentValueRef::Index(&selector),
        ];
        let mut output = [None];
        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::Snapshot(&mut output)),
            Err(ResidentKernelError::InvalidShape),
        );
        assert!(output[0].is_none());
    }

    #[test]
    fn snapshot_aggregate_assignment_admits_draft_and_final_node_populations() {
        let (final_nodes, phases) =
            snapshot_aggregate_assignment_node_phases(40_000, 1, 1).unwrap();
        assert_eq!(final_nodes, 40_001);
        assert_eq!(phases.current_persistent, 40_002);
        assert_eq!(phases.normalized_plan, 2);
        assert_eq!(phases.temporary_draft, 40_001);
        assert_eq!(
            super::super::budget::PreparedMutationPlan::new(
                (),
                super::super::budget::PublishedOutputFootprint {
                    elements: 1,
                    retained_bytes: 0,
                    retained_nodes: final_nodes,
                },
                phases,
                super::super::budget::KernelCostEstimate::default(),
            )
            .unwrap()
            .admit()
            .unwrap_err(),
            ResidentKernelError::InvalidShape,
        );
    }

    #[test]
    fn snapshot_access_admits_source_prior_output_and_both_staged_populations() {
        let phases = snapshot_access_node_phases(40_000, 30_000, 1, 30_000, 30_001).unwrap();
        assert_eq!(phases.current_persistent, 70_001);
        assert_eq!(phases.normalized_plan, 1);
        assert_eq!(phases.temporary_draft, 60_001);
        assert_eq!(
            super::super::budget::PreparedMutationPlan::new(
                (),
                super::super::budget::PublishedOutputFootprint {
                    elements: 1,
                    retained_bytes: 0,
                    retained_nodes: 30_001,
                },
                phases,
                super::super::budget::KernelCostEstimate::default(),
            )
            .unwrap()
            .admit()
            .unwrap_err(),
            ResidentKernelError::InvalidShape,
        );
    }

    #[test]
    fn snapshot_access_publication_includes_the_value_wrapper() {
        let data = ValueFootprint {
            encoded_bytes: 1,
            retained_bytes: super::super::budget::MAX_RESIDENT_OUTPUT_BYTES
                - core::mem::size_of::<mech_core::Value>() as u64
                + 1,
            node_count: 1,
        };
        let published = super::super::budget::projected_canonical_value_footprint(data, 0)
            .expect("projected output footprint");
        assert!(
            published.retained_bytes > super::super::budget::MAX_RESIDENT_OUTPUT_BYTES,
            "the complete published Value must exceed the limit even when its data alone fits",
        );
        assert_eq!(
            super::super::budget::PreparedMutationPlan::new(
                (),
                super::super::budget::PublishedOutputFootprint {
                    elements: 1,
                    retained_bytes: published.retained_bytes,
                    retained_nodes: published.node_count,
                },
                super::super::budget::MutationRetainedNodeFootprint::default(),
                super::super::budget::KernelCostEstimate::default(),
            )
            .unwrap()
            .admit()
            .unwrap_err(),
            ResidentKernelError::InvalidShape,
        );
    }

    #[test]
    fn snapshot_access_reserves_nested_set_finalization_before_cloning() {
        let set_body = SchemaBody::Set {
            element: Box::new(SchemaBody::String),
            cardinality: mech_core::CardinalitySpec::Exact(mech_core::DimensionExpr::Constant(50)),
        };
        let tuple_body = SchemaBody::Tuple(vec![set_body.clone()].into_boxed_slice());
        let (schemas, ids) = test_schema_table([set_body, tuple_body, SchemaBody::Index]);
        let [set_schema, tuple_schema, index_schema] = ids.as_slice() else {
            unreachable!()
        };
        let contract = test_contract(
            &[*tuple_schema, *index_schema],
            *set_schema,
            OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            AccessMode::Write,
            AliasPolicy::NoAlias,
            ChangeDetectionPolicy::KernelReported,
        );
        let kernel = bind_snapshot_access(&ResidentKernelBindRequest {
            contract: &contract,
            schemas: &schemas,
            inputs: &[
                test_layout(
                    &schemas,
                    *tuple_schema,
                    ResidentValueKind::Snapshot,
                    ResidentShape::SCALAR,
                ),
                test_layout(
                    &schemas,
                    *index_schema,
                    ResidentValueKind::Index,
                    ResidentShape::SCALAR,
                ),
            ],
            output: test_layout(
                &schemas,
                *set_schema,
                ResidentValueKind::Snapshot,
                ResidentShape::SCALAR,
            ),
        })
        .unwrap();
        let source = [Some(test_value(
            &schemas,
            *tuple_schema,
            ValueDataDraft::Tuple(
                vec![ValueDataDraft::Set(
                    (0..50)
                        .map(|index| {
                            ValueDataDraft::String(format!("{}-{index:04}", "x".repeat(500)))
                        })
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                )]
                .into_boxed_slice(),
            ),
        ))];
        let selector = [1_u64];
        let inputs = [
            ResidentValueRef::Snapshot(&source),
            ResidentValueRef::Index(&selector),
        ];
        let mut output = [None];

        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::Snapshot(&mut output)),
            Err(ResidentKernelError::InvalidShape),
        );
        assert!(output[0].is_none());
    }

    #[test]
    fn snapshot_access_reserves_snapshot_change_detection_before_cloning() {
        let tuple_body = SchemaBody::Tuple(vec![SchemaBody::String].into_boxed_slice());
        let (schemas, ids) = test_schema_table([SchemaBody::String, tuple_body, SchemaBody::Index]);
        let [string_schema, tuple_schema, index_schema] = ids.as_slice() else {
            unreachable!()
        };
        let contract = test_contract(
            &[*tuple_schema, *index_schema],
            *string_schema,
            OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            AccessMode::Write,
            AliasPolicy::NoAlias,
            ChangeDetectionPolicy::KernelReported,
        );
        let kernel = bind_snapshot_access(&ResidentKernelBindRequest {
            contract: &contract,
            schemas: &schemas,
            inputs: &[
                test_layout(
                    &schemas,
                    *tuple_schema,
                    ResidentValueKind::Snapshot,
                    ResidentShape::SCALAR,
                ),
                test_layout(
                    &schemas,
                    *index_schema,
                    ResidentValueKind::Index,
                    ResidentShape::SCALAR,
                ),
            ],
            output: test_layout(
                &schemas,
                *string_schema,
                ResidentValueKind::Snapshot,
                ResidentShape::SCALAR,
            ),
        })
        .unwrap();
        let text = "x".repeat(18 * 1024);
        let source = [Some(test_value(
            &schemas,
            *tuple_schema,
            ValueDataDraft::Tuple(vec![ValueDataDraft::String(text.clone())].into_boxed_slice()),
        ))];
        let selector = [1_u64];
        let inputs = [
            ResidentValueRef::Snapshot(&source),
            ResidentValueRef::Index(&selector),
        ];
        let current_text = text.clone();
        let current = test_value(&schemas, *string_schema, ValueDataDraft::String(text));
        let mut output = [Some(current.clone())];

        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::Snapshot(&mut output)),
            Err(ResidentKernelError::InvalidShape),
        );
        assert!(
            output[0]
                .as_ref()
                .unwrap()
                .language_eq(&schemas, &current, &schemas)
                .unwrap()
        );

        let dense_kernel = bind_snapshot_access(&ResidentKernelBindRequest {
            contract: &contract,
            schemas: &schemas,
            inputs: &[
                test_layout(
                    &schemas,
                    *tuple_schema,
                    ResidentValueKind::Snapshot,
                    ResidentShape::SCALAR,
                ),
                test_layout(
                    &schemas,
                    *index_schema,
                    ResidentValueKind::Index,
                    ResidentShape::SCALAR,
                ),
            ],
            output: test_layout(
                &schemas,
                *string_schema,
                ResidentValueKind::String,
                ResidentShape::SCALAR,
            ),
        })
        .unwrap();
        let mut dense_output = [current_text.clone()];
        assert_eq!(
            dense_kernel.execute(
                &Inputs(&inputs),
                ResidentValueMut::String(&mut dense_output),
            ),
            Err(ResidentKernelError::InvalidShape),
        );
        assert_eq!(dense_output, [current_text]);
    }

    #[test]
    fn admitted_matrix_families_execute_dense_and_snapshot_layouts() {
        let u64_body = SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64);
        let u64_matrix = |rows, columns| SchemaBody::Matrix {
            element: Box::new(u64_body.clone()),
            dimensions: vec![
                mech_core::DimensionExpr::Constant(rows),
                mech_core::DimensionExpr::Constant(columns),
            ]
            .into_boxed_slice(),
        };
        let bool_matrix = |rows, columns| SchemaBody::Matrix {
            element: Box::new(SchemaBody::Bool),
            dimensions: vec![
                mech_core::DimensionExpr::Constant(rows),
                mech_core::DimensionExpr::Constant(columns),
            ]
            .into_boxed_slice(),
        };
        let (schemas, ids) = test_schema_table([
            u64_body.clone(),
            u64_matrix(2, 2),
            bool_matrix(2, 3),
            bool_matrix(3, 2),
            SchemaBody::Bool,
            bool_matrix(1, 2),
            u64_matrix(1, 2),
        ]);
        let [
            u64_scalar,
            u64_2x2,
            bool_2x3,
            bool_3x2,
            bool_scalar,
            bool_1x2,
            u64_1x2,
        ] = ids.as_slice()
        else {
            unreachable!()
        };

        let transpose_contract = test_contract(
            &[*bool_2x3],
            *bool_3x2,
            OutputConstruction::FullWrite {
                shape: ShapeRule::TransposeOf { input: 0 },
            },
            AccessMode::Write,
            AliasPolicy::NoAlias,
            ChangeDetectionPolicy::KernelReported,
        );
        let transpose = bind_semantic_transpose(&ResidentKernelBindRequest {
            contract: &transpose_contract,
            schemas: &schemas,
            inputs: &[test_layout(
                &schemas,
                *bool_2x3,
                ResidentValueKind::Bool,
                ResidentShape {
                    rows: 2,
                    columns: 3,
                },
            )],
            output: test_layout(
                &schemas,
                *bool_3x2,
                ResidentValueKind::Bool,
                ResidentShape {
                    rows: 3,
                    columns: 2,
                },
            ),
        })
        .unwrap();
        let source = [1_u8, 0, 0, 1, 1, 0];
        let inputs = [ResidentValueRef::Bool(&source)];
        let mut output = [0_u8; 6];
        transpose
            .execute(&Inputs(&inputs), ResidentValueMut::Bool(&mut output))
            .unwrap();
        assert_eq!(output, [1, 0, 1, 0, 1, 0]);

        let matmul_contract = test_contract(
            &[*u64_2x2, *u64_2x2],
            *u64_2x2,
            OutputConstruction::FullWrite {
                shape: ShapeRule::MatrixProduct { lhs: 0, rhs: 1 },
            },
            AccessMode::Write,
            AliasPolicy::NoAlias,
            ChangeDetectionPolicy::KernelReported,
        );
        let matmul = bind_matmul(&ResidentKernelBindRequest {
            contract: &matmul_contract,
            schemas: &schemas,
            inputs: &[
                test_layout(
                    &schemas,
                    *u64_2x2,
                    ResidentValueKind::Snapshot,
                    ResidentShape::SCALAR,
                ),
                test_layout(
                    &schemas,
                    *u64_2x2,
                    ResidentValueKind::Snapshot,
                    ResidentShape::SCALAR,
                ),
            ],
            output: test_layout(
                &schemas,
                *u64_2x2,
                ResidentValueKind::Snapshot,
                ResidentShape::SCALAR,
            ),
        })
        .unwrap();
        let matrix = |values: [u64; 4]| {
            test_value(
                &schemas,
                *u64_2x2,
                ValueDataDraft::Matrix(
                    values
                        .into_iter()
                        .map(ValueDataDraft::U64)
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                ),
            )
        };
        let left = [Some(matrix([1, 2, 3, 4]))];
        let right = [Some(matrix([5, 6, 7, 8]))];
        let inputs = [
            ResidentValueRef::Snapshot(&left),
            ResidentValueRef::Snapshot(&right),
        ];
        let mut product = [None];
        matmul
            .execute(&Inputs(&inputs), ResidentValueMut::Snapshot(&mut product))
            .unwrap();
        let ValueData::Matrix(values) = product[0].as_ref().unwrap().data() else {
            panic!("matrix product must remain a matrix")
        };
        assert!(matches!(
            values.elements().to_values().as_slice(),
            [
                ValueData::U64(19),
                ValueData::U64(22),
                ValueData::U64(43),
                ValueData::U64(50)
            ]
        ));

        let horizontal_contract = test_contract(
            &[*bool_scalar, *bool_scalar],
            *bool_1x2,
            OutputConstruction::Build {
                postcondition: ShapeContractReference {
                    module_path: vec!["matrix".to_owned(), "concatenate".to_owned()]
                        .into_boxed_slice(),
                    contract_name: "horizontal-output".to_owned(),
                },
            },
            AccessMode::Write,
            AliasPolicy::NoAlias,
            ChangeDetectionPolicy::KernelReported,
        );
        let horizontal = bind_horizontal(&ResidentKernelBindRequest {
            contract: &horizontal_contract,
            schemas: &schemas,
            inputs: &[
                test_layout(
                    &schemas,
                    *bool_scalar,
                    ResidentValueKind::Bool,
                    ResidentShape::SCALAR,
                ),
                test_layout(
                    &schemas,
                    *bool_scalar,
                    ResidentValueKind::Bool,
                    ResidentShape::SCALAR,
                ),
            ],
            output: test_layout(
                &schemas,
                *bool_1x2,
                ResidentValueKind::Bool,
                ResidentShape {
                    rows: 1,
                    columns: 2,
                },
            ),
        })
        .unwrap();
        let left = [1_u8];
        let right = [0_u8];
        let inputs = [
            ResidentValueRef::Bool(&left),
            ResidentValueRef::Bool(&right),
        ];
        let mut joined = [0_u8; 2];
        horizontal
            .execute(&Inputs(&inputs), ResidentValueMut::Bool(&mut joined))
            .unwrap();
        assert_eq!(joined, [1, 0]);

        let snapshot_contract = test_contract(
            &[*u64_scalar, *u64_scalar],
            *u64_1x2,
            OutputConstruction::Build {
                postcondition: ShapeContractReference {
                    module_path: vec!["matrix".to_owned(), "concatenate".to_owned()]
                        .into_boxed_slice(),
                    contract_name: "horizontal-output".to_owned(),
                },
            },
            AccessMode::Write,
            AliasPolicy::NoAlias,
            ChangeDetectionPolicy::KernelReported,
        );
        let snapshot_horizontal = bind_horizontal(&ResidentKernelBindRequest {
            contract: &snapshot_contract,
            schemas: &schemas,
            inputs: &[
                test_layout(
                    &schemas,
                    *u64_scalar,
                    ResidentValueKind::Snapshot,
                    ResidentShape::SCALAR,
                ),
                test_layout(
                    &schemas,
                    *u64_scalar,
                    ResidentValueKind::Snapshot,
                    ResidentShape::SCALAR,
                ),
            ],
            output: test_layout(
                &schemas,
                *u64_1x2,
                ResidentValueKind::Snapshot,
                ResidentShape::SCALAR,
            ),
        })
        .unwrap();
        let left = [Some(test_value(
            &schemas,
            *u64_scalar,
            ValueDataDraft::U64(2),
        ))];
        let right = [Some(test_value(
            &schemas,
            *u64_scalar,
            ValueDataDraft::U64(3),
        ))];
        let inputs = [
            ResidentValueRef::Snapshot(&left),
            ResidentValueRef::Snapshot(&right),
        ];
        let mut joined = [None];
        snapshot_horizontal
            .execute(&Inputs(&inputs), ResidentValueMut::Snapshot(&mut joined))
            .unwrap();
        let ValueData::Matrix(values) = joined[0].as_ref().unwrap().data() else {
            panic!("snapshot concatenation must produce a matrix")
        };
        assert!(matches!(
            values.elements().to_values().as_slice(),
            [ValueData::U64(2), ValueData::U64(3)]
        ));
    }

    #[test]
    fn snapshot_equality_rejects_large_canonical_material_before_publication() {
        let table_body = SchemaBody::Table {
            columns: vec![mech_core::SchemaField {
                name: "text".to_owned(),
                schema: SchemaBody::String,
            }]
            .into_boxed_slice(),
            rows: mech_core::CardinalitySpec::Exact(mech_core::DimensionExpr::Constant(1)),
        };
        let (schemas, ids) = test_schema_table([table_body, SchemaBody::Bool]);
        let [table, boolean] = ids.as_slice() else {
            unreachable!()
        };
        let contract = test_contract(
            &[*table, *table],
            *boolean,
            OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            AccessMode::Write,
            AliasPolicy::NoAlias,
            ChangeDetectionPolicy::ExactScalar,
        );
        let table_value = || {
            test_value(
                &schemas,
                *table,
                ValueDataDraft::Table(
                    vec![mech_core::snapshot::TableColumnDraft {
                        name: "text".to_owned(),
                        values: vec![ValueDataDraft::String("x".repeat(70_000))].into_boxed_slice(),
                    }]
                    .into_boxed_slice(),
                ),
            )
        };
        let left = [Some(table_value())];
        let right = [Some(table_value())];
        let inputs = [
            ResidentValueRef::Snapshot(&left),
            ResidentValueRef::Snapshot(&right),
        ];
        for executor in [
            snapshot_equal as mech_core::ResidentKernelExecutor,
            snapshot_not_equal as mech_core::ResidentKernelExecutor,
        ] {
            let kernel = bind_snapshot_equality(
                &ResidentKernelBindRequest {
                    contract: &contract,
                    schemas: &schemas,
                    inputs: &[
                        test_layout(
                            &schemas,
                            *table,
                            ResidentValueKind::Snapshot,
                            ResidentShape::SCALAR,
                        ),
                        test_layout(
                            &schemas,
                            *table,
                            ResidentValueKind::Snapshot,
                            ResidentShape::SCALAR,
                        ),
                    ],
                    output: test_layout(
                        &schemas,
                        *boolean,
                        ResidentValueKind::Bool,
                        ResidentShape::SCALAR,
                    ),
                },
                executor,
            )
            .unwrap();
            let mut output = [1_u8];
            assert_eq!(
                kernel.execute(&Inputs(&inputs), ResidentValueMut::Bool(&mut output)),
                Err(ResidentKernelError::InvalidShape),
            );
            assert_eq!(output, [1]);
        }

        let enum_body = SchemaBody::Enum {
            key: mech_core::NominalKey::from_bytes([42; 32]),
            variants: (0..512)
                .map(|index| mech_core::EnumVariantSchema {
                    name: format!("{index:04}-{}", "schema".repeat(48)),
                    payload: None,
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        };
        let (schemas, ids) = test_schema_table([enum_body, SchemaBody::Bool]);
        let [enumeration, boolean] = ids.as_slice() else {
            unreachable!()
        };
        assert!(
            schemas.entry(*enumeration).unwrap().canonical_bytes().len()
                > super::super::budget::MAX_RESIDENT_COMPARISON_WORK as usize
        );
        let contract = test_contract(
            &[*enumeration, *enumeration],
            *boolean,
            OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            AccessMode::Write,
            AliasPolicy::NoAlias,
            ChangeDetectionPolicy::ExactScalar,
        );
        let value = || {
            test_value(
                &schemas,
                *enumeration,
                ValueDataDraft::Enum(mech_core::snapshot::EnumDraft {
                    ordinal: 0,
                    payload: None,
                }),
            )
        };
        let left = [Some(value())];
        let right = [Some(value())];
        let inputs = [
            ResidentValueRef::Snapshot(&left),
            ResidentValueRef::Snapshot(&right),
        ];
        let ports = [
            test_layout(
                &schemas,
                *enumeration,
                ResidentValueKind::Snapshot,
                ResidentShape::SCALAR,
            ),
            test_layout(
                &schemas,
                *enumeration,
                ResidentValueKind::Snapshot,
                ResidentShape::SCALAR,
            ),
        ];
        let output_port = test_layout(
            &schemas,
            *boolean,
            ResidentValueKind::Bool,
            ResidentShape::SCALAR,
        );
        for kernel in [
            bind_snapshot_equality(
                &ResidentKernelBindRequest {
                    contract: &contract,
                    schemas: &schemas,
                    inputs: &ports,
                    output: output_port.clone(),
                },
                snapshot_equal,
            )
            .unwrap(),
            bind_strict_equal(&ResidentKernelBindRequest {
                contract: &contract,
                schemas: &schemas,
                inputs: &ports,
                output: output_port.clone(),
            })
            .unwrap(),
        ] {
            let mut output = [1_u8];
            assert_eq!(
                kernel.execute(&Inputs(&inputs), ResidentValueMut::Bool(&mut output)),
                Err(ResidentKernelError::InvalidShape),
            );
            assert_eq!(output, [1]);
        }
    }

    #[test]
    fn snapshot_concatenation_admits_recursive_inputs_drafts_and_output_together() {
        const TUPLE_ELEMENTS: usize = 10_000;
        let tuple_body = SchemaBody::Tuple(
            std::iter::repeat_n(SchemaBody::Bool, TUPLE_ELEMENTS)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        );
        let matrix_body = SchemaBody::Matrix {
            element: Box::new(tuple_body.clone()),
            dimensions: vec![
                mech_core::DimensionExpr::Constant(1),
                mech_core::DimensionExpr::Constant(2),
            ]
            .into_boxed_slice(),
        };
        let (schemas, ids) = test_schema_table([tuple_body, matrix_body]);
        let [tuple, matrix] = ids.as_slice() else {
            unreachable!()
        };
        let contract = test_contract(
            &[*tuple, *tuple],
            *matrix,
            OutputConstruction::Build {
                postcondition: ShapeContractReference {
                    module_path: vec!["matrix".to_owned(), "concatenate".to_owned()]
                        .into_boxed_slice(),
                    contract_name: "horizontal-output".to_owned(),
                },
            },
            AccessMode::Write,
            AliasPolicy::NoAlias,
            ChangeDetectionPolicy::KernelReported,
        );
        let kernel = bind_horizontal(&ResidentKernelBindRequest {
            contract: &contract,
            schemas: &schemas,
            inputs: &[
                test_layout(
                    &schemas,
                    *tuple,
                    ResidentValueKind::Snapshot,
                    ResidentShape::SCALAR,
                ),
                test_layout(
                    &schemas,
                    *tuple,
                    ResidentValueKind::Snapshot,
                    ResidentShape::SCALAR,
                ),
            ],
            output: test_layout(
                &schemas,
                *matrix,
                ResidentValueKind::Snapshot,
                ResidentShape::SCALAR,
            ),
        })
        .unwrap();
        let tuple_draft = || {
            ValueDataDraft::Tuple(
                std::iter::repeat_n(ValueDataDraft::Bool(true), TUPLE_ELEMENTS)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            )
        };
        let element = || test_value(&schemas, *tuple, tuple_draft());
        let left = [Some(element())];
        let right = [Some(element())];
        let previous = test_value(
            &schemas,
            *matrix,
            ValueDataDraft::Matrix(vec![tuple_draft(), tuple_draft()].into_boxed_slice()),
        );
        let inputs = [
            ResidentValueRef::Snapshot(&left),
            ResidentValueRef::Snapshot(&right),
        ];
        let mut output = [Some(previous.clone())];

        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::Snapshot(&mut output),),
            Err(ResidentKernelError::InvalidShape),
        );
        assert!(
            output[0]
                .as_ref()
                .unwrap()
                .language_eq(&schemas, &previous, &schemas)
                .unwrap()
        );
    }

    #[test]
    fn snapshot_string_broadcast_comparison_charges_every_pair() {
        let (schemas, ids) = test_schema_table([SchemaBody::String]);
        let [string_schema] = ids.as_slice() else {
            unreachable!()
        };
        let left = [Some(test_value(
            &schemas,
            *string_schema,
            ValueDataDraft::String("x".repeat(1_024)),
        ))];
        let right = [Some(test_value(
            &schemas,
            *string_schema,
            ValueDataDraft::String("x".repeat(1_024)),
        ))];
        let inputs = [
            ResidentValueRef::Snapshot(&left),
            ResidentValueRef::Snapshot(&right),
        ];
        let kernel = BoundResidentKernel::new(
            snapshot_comparison,
            vec![
                100,
                100,
                BINARY_BROADCAST_SCALAR,
                BINARY_BROADCAST_SCALAR,
                SemanticComparison::Equal as u64,
            ]
            .into_boxed_slice(),
        )
        .with_snapshot_schemas(schemas);
        let mut output = vec![9_u8; 10_000];

        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::Bool(&mut output)),
            Err(ResidentKernelError::InvalidShape),
        );
        assert!(output.iter().all(|value| *value == 9));
    }

    #[test]
    fn dense_string_broadcast_comparison_charges_every_pair_before_writing() {
        let left = ["x".repeat(1_024)];
        let right = ["x".repeat(1_024)];
        let inputs = [
            ResidentValueRef::String(&left),
            ResidentValueRef::String(&right),
        ];
        let kernel = BoundResidentKernel::new(
            dense_comparison,
            vec![
                100,
                100,
                BINARY_BROADCAST_SCALAR,
                BINARY_BROADCAST_SCALAR,
                SemanticComparison::Equal as u64,
            ]
            .into_boxed_slice(),
        );
        let mut output = vec![9_u8; 10_000];

        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::Bool(&mut output)),
            Err(ResidentKernelError::InvalidShape),
        );
        assert!(output.iter().all(|value| *value == 9));
    }

    #[test]
    fn snapshot_hold_arithmetic_and_transpose_admit_prior_output_work() {
        let string_body = SchemaBody::String;
        let u8_matrix_body = SchemaBody::Matrix {
            element: Box::new(SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W8)),
            dimensions: vec![
                mech_core::DimensionExpr::Constant(200),
                mech_core::DimensionExpr::Constant(200),
            ]
            .into_boxed_slice(),
        };
        let (schemas, ids) = test_schema_table([string_body, u8_matrix_body]);
        let [string_schema, matrix_schema] = ids.as_slice() else {
            unreachable!()
        };

        let large_string = || {
            test_value(
                &schemas,
                *string_schema,
                ValueDataDraft::String("equal".repeat(4_000)),
            )
        };
        let source = [Some(large_string())];
        let original = large_string();
        let mut output = [Some(original.clone())];
        let hold = BoundResidentKernel::new(hold_state, Box::new([]))
            .with_snapshot_schemas(schemas.clone());
        assert_eq!(
            hold.execute(
                &Inputs(&[ResidentValueRef::Snapshot(&source)]),
                ResidentValueMut::Snapshot(&mut output),
            ),
            Err(ResidentKernelError::InvalidShape),
        );
        assert!(
            output[0]
                .as_ref()
                .unwrap()
                .language_eq(&schemas, &original, &schemas)
                .unwrap()
        );

        let matrix_value = || {
            test_value(
                &schemas,
                *matrix_schema,
                ValueDataDraft::Matrix(
                    std::iter::repeat_n(ValueDataDraft::U8(1), 40_000)
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                ),
            )
        };
        let source_value = matrix_value();
        let source = [Some(source_value)];
        for (executor, parameters) in [
            (
                snapshot_numeric_negate as mech_core::ResidentKernelExecutor,
                vec![200, 200],
            ),
            (transpose_snapshot, vec![200, 200]),
        ] {
            let original = matrix_value();
            let mut output = [Some(original.clone())];
            let kernel = BoundResidentKernel::new(executor, parameters.into_boxed_slice())
                .with_snapshot_output(ResidentSnapshotOutput {
                    schema: *matrix_schema,
                    schema_key: schemas.entry(*matrix_schema).unwrap().key(),
                    shape: schemas
                        .get(*matrix_schema)
                        .unwrap()
                        .instantiate_shape(Box::new([]))
                        .unwrap(),
                    exact_cardinality: None,
                    maximum_cardinality: None,
                })
                .with_snapshot_schemas(schemas.clone());
            assert_eq!(
                kernel.execute(
                    &Inputs(&[ResidentValueRef::Snapshot(&source)]),
                    ResidentValueMut::Snapshot(&mut output),
                ),
                Err(ResidentKernelError::InvalidShape),
            );
            assert!(
                output[0]
                    .as_ref()
                    .unwrap()
                    .language_eq(&schemas, &original, &schemas)
                    .unwrap()
            );
        }
    }

    #[test]
    fn snapshot_math_and_dot_reject_staging_cost_before_output_mutation() {
        let f32_matrix_body = SchemaBody::Matrix {
            element: Box::new(SchemaBody::FloatingPoint(mech_core::FloatWidth::W32)),
            dimensions: vec![
                mech_core::DimensionExpr::Constant(65_537),
                mech_core::DimensionExpr::Constant(1),
            ]
            .into_boxed_slice(),
        };
        let u64_body = SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64);
        let u64_matrix_body = SchemaBody::Matrix {
            element: Box::new(u64_body.clone()),
            dimensions: vec![
                mech_core::DimensionExpr::Constant(200_000),
                mech_core::DimensionExpr::Constant(1),
            ]
            .into_boxed_slice(),
        };
        let (schemas, ids) = test_schema_table([f32_matrix_body, u64_body, u64_matrix_body]);
        let [f32_matrix, u64_scalar, u64_matrix] = ids.as_slice() else {
            unreachable!()
        };
        let snapshot_kernel = |executor, output_schema| {
            BoundResidentKernel::new(executor, Box::new([]))
                .with_snapshot_output(ResidentSnapshotOutput {
                    schema: output_schema,
                    schema_key: schemas.entry(output_schema).unwrap().key(),
                    shape: schemas
                        .get(output_schema)
                        .unwrap()
                        .instantiate_shape(Box::new([]))
                        .unwrap(),
                    exact_cardinality: None,
                    maximum_cardinality: None,
                })
                .with_snapshot_schemas(schemas.clone())
        };
        let f32_value = test_value(
            &schemas,
            *f32_matrix,
            ValueDataDraft::Matrix(
                (0..65_537)
                    .map(|_| ValueDataDraft::F32(F32Bits::from_f32(1.0)))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
        );
        let left = [Some(f32_value.clone())];
        let right = [Some(f32_value)];
        let inputs = [
            ResidentValueRef::Snapshot(&left),
            ResidentValueRef::Snapshot(&right),
        ];
        let mut output = [None];
        assert_eq!(
            snapshot_kernel(math_copysign_f32, *f32_matrix)
                .execute(&Inputs(&inputs), ResidentValueMut::Snapshot(&mut output),),
            Err(ResidentKernelError::InvalidShape),
        );
        assert!(output[0].is_none());

        let u64_value = test_value(
            &schemas,
            *u64_matrix,
            ValueDataDraft::Matrix(
                (0..200_000)
                    .map(|_| ValueDataDraft::U64(1))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
        );
        let left = [Some(u64_value.clone())];
        let right = [Some(u64_value)];
        let inputs = [
            ResidentValueRef::Snapshot(&left),
            ResidentValueRef::Snapshot(&right),
        ];
        let mut output = [None];
        assert_eq!(
            snapshot_kernel(matrix_dot_snapshot, *u64_scalar)
                .execute(&Inputs(&inputs), ResidentValueMut::Snapshot(&mut output),),
            Err(ResidentKernelError::InvalidShape),
        );
        assert!(output[0].is_none());
    }

    #[test]
    fn dense_concatenation_first_middle_and_last_failures_are_atomic() {
        for invalid in 0..4 {
            let mut left = [1_u8, 0];
            let mut right = [0_u8, 1];
            if invalid < 2 {
                left[invalid] = 2;
            } else {
                right[invalid - 2] = 2;
            }
            let inputs = [
                ResidentValueRef::Bool(&left),
                ResidentValueRef::Bool(&right),
            ];

            let horizontal = BoundResidentKernel::new(
                concatenate_horizontal,
                vec![2, 1, 2, 1, 2].into_boxed_slice(),
            );
            let mut output = [9_u8; 4];
            assert_eq!(
                horizontal.execute(&Inputs(&inputs), ResidentValueMut::Bool(&mut output)),
                Err(ResidentKernelError::InvalidInput)
            );
            assert_eq!(output, [9; 4]);

            let vertical = BoundResidentKernel::new(
                concatenate_vertical,
                vec![2, 2, 1, 2, 1].into_boxed_slice(),
            );
            assert_eq!(
                vertical.execute(&Inputs(&inputs), ResidentValueMut::Bool(&mut output)),
                Err(ResidentKernelError::InvalidInput)
            );
            assert_eq!(output, [9; 4]);
        }
    }

    #[test]
    fn scalar_index_first_middle_and_last_failures_are_atomic() {
        let kernel = BoundResidentKernel::new(scalar_index, Box::new([]));
        for invalid in 0..3 {
            let mut values = [1.0_f64, 2.0, 3.0];
            values[invalid] = f64::NAN;
            let inputs = [ResidentValueRef::F64(&values)];
            let mut output = [9_u64; 3];
            assert_eq!(
                kernel.execute(&Inputs(&inputs), ResidentValueMut::Index(&mut output)),
                Err(ResidentKernelError::InvalidInput)
            );
            assert_eq!(output, [9; 3]);
        }
    }

    #[test]
    fn dense_range_and_combinatorics_respect_shared_output_admission() {
        let range = BoundResidentKernel::new(range_inclusive, Box::new([]));
        let start = [1.0_f64];
        let end = [65_537.0_f64];
        let range_inputs = [ResidentValueRef::F64(&start), ResidentValueRef::F64(&end)];
        let mut range_output = vec![7.0_f64; 65_537];
        assert_eq!(
            range.execute(
                &Inputs(&range_inputs),
                ResidentValueMut::F64(&mut range_output),
            ),
            Err(ResidentKernelError::InvalidShape)
        );
        assert!(range_output.iter().all(|value| *value == 7.0));

        let combinations = checked_combination_count(18, 9).unwrap();
        let choose =
            BoundResidentKernel::new(n_choose_k, vec![9, combinations as u64].into_boxed_slice());
        let values = (0..18).map(|value| value as f64).collect::<Vec<_>>();
        let selection = [9.0_f64];
        let choose_inputs = [
            ResidentValueRef::F64(&values),
            ResidentValueRef::F64(&selection),
        ];
        let mut choose_output = vec![7.0_f64; 9 * combinations];
        assert_eq!(
            choose.execute(
                &Inputs(&choose_inputs),
                ResidentValueMut::F64(&mut choose_output),
            ),
            Err(ResidentKernelError::InvalidShape)
        );
        assert!(choose_output.iter().all(|value| *value == 7.0));
    }
}
