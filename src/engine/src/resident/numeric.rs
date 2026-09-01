#[cfg(test)]
use crate::portable_index::PORTABLE_INDEX_MAX;
use crate::portable_index::ToPortableIndex;
use mech_core::snapshot::{
    F32Bits, F64Bits, SequenceView, SnapshotValidationContext, ValueDataDraft, ValueDraft,
    schema_data_language_eq, schema_data_partial_cmp,
};
use mech_core::{
    AccessMode, AliasPolicy, BoundResidentKernel, ChangeDetectionPolicy, DeliveryMode,
    ExternalInteraction, FunctionCatalogBuilder, MResult, OutputConstruction, RegionPolicy,
    ResidentKernelBindError, ResidentKernelBindRequest, ResidentKernelError, ResidentKernelInputs,
    ResidentShape, ResidentSnapshotOutput, ResidentValueKind, ResidentValueMut, ResidentValueRef,
    ResolvedOperationContract, SchemaBody, ShapeContractReference, ShapeRule, ValueData,
};

const MAX_MATRIX_SOLVE_WORK: usize = 16_777_216;

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
    register(builder, &["math"], "add", bind_add)?;
    register(builder, &["math"], "add-assign", bind_semantic_add_assign)?;
    register(
        builder,
        &["math", "add-assign"],
        "range-all",
        bind_add_indexed_rows,
    )?;
    register(
        builder,
        &["math", "sub-assign"],
        "range-all",
        bind_sub_indexed_rows,
    )?;
    register(builder, &["math"], "sub", bind_sub)?;
    register(builder, &["math"], "mul", bind_semantic_mul)?;
    register(builder, &["math"], "div", bind_div)?;
    register(builder, &["math"], "mod", bind_remainder)?;
    register(builder, &["math"], "neg", bind_negate)?;
    register(builder, &["math"], "pow", bind_pow)?;
    register(builder, &["math"], "abs", bind_abs)?;
    register(builder, &["math"], "copysign", bind_math_copysign)?;
    register(builder, &["math"], "fdim", bind_math_fdim)?;
    register(builder, &["math"], "floor", bind_floor)?;
    register(builder, &["math"], "fmod", bind_math_fmod)?;
    register(builder, &["math"], "nextafter", bind_math_nextafter)?;
    register(builder, &["math"], "remainder", bind_math_remainder)?;
    register(builder, &["math"], "sqrt", bind_sqrt)?;
    register(builder, &["math"], "atan2", bind_atan2)?;
    register(builder, &["math"], "cos", bind_cos)?;
    register(builder, &["math"], "sin", bind_sin)?;
    register(builder, &["math", "bessel"], "jn", bind_math_bessel_jn)?;
    register(builder, &["math", "bessel"], "yn", bind_math_bessel_yn)?;
    register(builder, &["logic"], "and", bind_semantic_bool_and)?;
    register(builder, &["logic"], "or", bind_semantic_bool_or)?;
    register(builder, &["logic"], "xor", bind_semantic_bool_xor)?;
    register(builder, &["logic"], "not", bind_bool_not)?;
    register(builder, &["compare"], "eq", bind_semantic_equal)?;
    register(builder, &["compare"], "neq", bind_semantic_not_equal)?;
    register(builder, &["compare"], "lt", bind_semantic_less)?;
    register(builder, &["compare"], "lte", bind_semantic_less_equal)?;
    register(builder, &["compare"], "gt", bind_semantic_greater)?;
    register(builder, &["compare"], "gte", bind_semantic_greater_equal)?;
    register(builder, &["compare"], "seq", bind_strict_equal)?;
    register(builder, &["compare"], "sneq", bind_strict_not_equal)?;
    register(builder, &["access"], "scalar", bind_semantic_scalar_access)?;
    register(builder, &["access"], "index", bind_scalar_index)?;
    register(builder, &["access"], "range", bind_semantic_range_access)?;
    register(builder, &["matrix"], "horzcat", bind_horizontal)?;
    register(builder, &["matrix"], "vertcat", bind_vertical)?;
    register(
        builder,
        &["matrix"],
        "comprehension",
        bind_matrix_comprehension,
    )?;
    register(builder, &["matrix"], "multiply", bind_matmul)?;
    register(builder, &["matrix"], "dot", bind_matrix_dot)?;
    register(builder, &["matrix"], "solve", bind_matrix_solve)?;
    register(builder, &["matrix"], "transpose", bind_semantic_transpose)?;
    register(builder, &["core"], "assign", bind_hold_state)?;
    register(
        builder,
        &["core", "assign"],
        "indexed-axis",
        bind_indexed_assign,
    )?;
    register(builder, &["range"], "exclusive", bind_range_exclusive)?;
    register(
        builder,
        &["range"],
        "exclusive-increment",
        bind_range_increment_exclusive,
    )?;
    register(builder, &["range"], "inclusive", bind_range_inclusive)?;
    register(
        builder,
        &["range"],
        "inclusive-increment",
        bind_range_increment_inclusive,
    )?;
    register(builder, &["combinatorics"], "n-choose-k", bind_n_choose_k)?;
    register(builder, &["stats", "sum"], "column", bind_sum_columns)?;
    register(builder, &["stats", "sum"], "row", bind_sum_rows)?;

    register(builder, &["ekf"], "trigonometric-state", bind_ekf_trig)?;
    register(builder, &["ekf"], "motion-jacobian", bind_ekf_motion)?;
    register(builder, &["ekf"], "control-jacobian", bind_ekf_control)?;
    register(
        builder,
        &["ekf"],
        "predicted-state",
        bind_ekf_predicted_state,
    )?;
    register(
        builder,
        &["ekf"],
        "predicted-covariance",
        bind_ekf_predicted_covariance,
    )?;
    register(
        builder,
        &["ekf"],
        "landmark-delta-and-range",
        bind_ekf_landmark,
    )?;
    register(
        builder,
        &["ekf"],
        "predicted-measurement",
        bind_ekf_measurement,
    )?;
    register(
        builder,
        &["ekf"],
        "measurement-jacobian",
        bind_ekf_measurement_jacobian,
    )?;
    register(
        builder,
        &["ekf"],
        "innovation-covariance",
        bind_ekf_innovation_covariance,
    )?;
    register(builder, &["ekf"], "solve-2x2", bind_ekf_solve)?;
    register(builder, &["ekf"], "kalman-gain", bind_ekf_gain)?;
    register(builder, &["ekf"], "innovation", bind_ekf_innovation)?;
    register(
        builder,
        &["ekf"],
        "corrected-state",
        bind_ekf_corrected_state,
    )?;
    register(
        builder,
        &["ekf"],
        "joseph-covariance-update",
        bind_ekf_joseph,
    )?;
    register(
        builder,
        &["ekf"],
        "covariance-symmetrization",
        bind_ekf_symmetrize,
    )?;
    register(builder, &["ekf"], "candidate-finite", bind_ekf_finite)?;
    register(
        builder,
        &["ekf"],
        "covariance-positive-diagonal",
        bind_ekf_positive_diagonal,
    )?;
    register(
        builder,
        &["ekf"],
        "covariance-symmetric",
        bind_ekf_symmetric,
    )?;
    Ok(())
}

fn register(
    builder: &mut FunctionCatalogBuilder,
    module: &[&str],
    operation: &str,
    factory: mech_core::ResidentKernelFactory,
) -> MResult<()> {
    builder.insert_resident_factory(module.iter().copied(), operation, factory)
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
    if input.schema_id != request.output.schema_id
        || input.schema_key != request.output.schema_key
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
    let change = f64_output_change_detection(request)?;
    validate_full_write(request, 1, ShapeRule::SameAsInput { input: 0 }, change)?;
    require_kind(request, &[ResidentValueKind::F64], ResidentValueKind::F64)?;
    if request.inputs[0].shape != request.output.shape {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    bound(negate, Vec::<u64>::new().into_boxed_slice())
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

fn bind_cos(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_unary_f64(request, cosine)
}

fn bind_abs(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_unary_f64(request, absolute)
}

fn bind_floor(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_unary_f64(request, floor)
}

fn bind_sqrt(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_unary_f64(request, square_root)
}

fn bind_sin(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_unary_f64(request, sine)
}

fn bind_atan2(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_binary(request, atan2)
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
}

fn bind_add(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_binary(request, add)
}

fn bind_mul(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_binary(request, multiply)
}

fn bind_semantic_mul(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_mul(request).or_else(|_| bind_mul_rows(request))
}

fn bind_div(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_binary(request, divide)
}

fn bind_remainder(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_binary(request, remainder)
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
    if !strict_inputs_share_identity(&request.inputs[0], &request.inputs[1]) {
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

fn strict_inputs_share_identity(
    left: &mech_core::ResidentPortLayout,
    right: &mech_core::ResidentPortLayout,
) -> bool {
    left.schema_id == right.schema_id && left.kind == right.kind && left.shape == right.shape
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
    require_kind(request, &[ResidentValueKind::F64], ResidentValueKind::F64)?;
    let [input] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    if request.output.shape.rows != input.shape.columns
        || request.output.shape.columns != input.shape.rows
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    bound(
        transpose,
        vec![input.shape.rows as u64, input.shape.columns as u64].into_boxed_slice(),
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
    require_kind(request, &[ResidentValueKind::F64], ResidentValueKind::F64)?;
    let [input] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    if request.output.shape.len() != Some(input.shape.rows as usize) {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    bound(
        sum_columns,
        vec![input.shape.rows as u64, input.shape.columns as u64].into_boxed_slice(),
    )
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
    require_kind(request, &[ResidentValueKind::F64], ResidentValueKind::F64)?;
    let [input] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    if request.output.shape.len() != Some(input.shape.columns as usize) {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    bound(
        sum_rows,
        vec![input.shape.rows as u64, input.shape.columns as u64].into_boxed_slice(),
    )
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
    if request.inputs.is_empty()
        || request
            .inputs
            .iter()
            .any(|input| input.kind != ResidentValueKind::F64)
        || request.output.kind != ResidentValueKind::F64
        || request
            .inputs
            .iter()
            .any(|input| input.shape.rows != request.output.shape.rows)
        || request
            .inputs
            .iter()
            .map(|input| input.shape.columns)
            .sum::<u32>()
            != request.output.shape.columns
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let mut parameters = Vec::with_capacity(1 + request.inputs.len() * 2);
    parameters.push(request.inputs.len() as u64);
    for input in request.inputs {
        parameters.push(input.shape.rows as u64);
        parameters.push(input.shape.columns as u64);
    }
    bound(concatenate_horizontal, parameters.into_boxed_slice())
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
    if (request.output.shape.len() == Some(0)
        || (request.inputs.len() == 1
            && request.inputs[0].kind == ResidentValueKind::Snapshot
            && request.inputs[0].shape == ResidentShape::SCALAR
            && request.output.kind == ResidentValueKind::Snapshot
            && request.output.shape == ResidentShape::SCALAR))
        && request.inputs.iter().all(|input| {
            (input.shape.len() == Some(0)
                || (input.kind == ResidentValueKind::Snapshot
                    && input.shape == ResidentShape::SCALAR))
                && input.kind == request.output.kind
        })
    {
        let kernel = bound(
            retain_empty_matrix_comprehension,
            Vec::<u64>::new().into_boxed_slice(),
        )?;
        return if request.output.kind == ResidentValueKind::Snapshot {
            Ok(kernel.with_snapshot_schemas(request.schemas.clone()))
        } else {
            Ok(kernel)
        };
    }
    if request.inputs.is_empty() {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    bind_horizontal(request)
}

fn bind_indexed_assign(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_rmw(request, 3, RegionPolicy::IndexedAxis { axis: 0 })?;
    let [base, source, selector] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let Some(output_schema) = request.schemas.get(request.output.schema_id) else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let SchemaBody::Matrix { element, .. } = output_schema.body() else {
        // Snapshot-backed records, maps, tables, tuples, and aggregate-valued
        // matrices require their own semantic assignment capability. Treating
        // their one snapshot lane as a one-element dense matrix would replace
        // the aggregate itself instead of the selected member.
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    if base.kind == ResidentValueKind::Snapshot {
        // A matrix whose element type lacks a dense resident lane is stored as
        // one whole-value snapshot. Indexed assignment cannot address members
        // of that lane without an explicit schema-aware aggregate capability.
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    if request
        .schemas
        .get(base.schema_id)
        .map(mech_core::Schema::body)
        != Some(output_schema.body())
        || !request
            .schemas
            .get(source.schema_id)
            .is_some_and(|schema| match schema.body() {
                body if body == element.as_ref() => true,
                SchemaBody::Matrix {
                    element: source_element,
                    ..
                } => source_element.as_ref() == element.as_ref(),
                _ => false,
            })
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
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
    let selector_len = selector
        .shape
        .len()
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    if base.kind != source.kind
        || base.kind != request.output.kind
        || base.shape != request.output.shape
        || !matches!(
            selector.kind,
            ResidentValueKind::Bool | ResidentValueKind::Index | ResidentValueKind::F64
        )
        || (selector.kind == ResidentValueKind::Bool && selector_len != output_len)
        || (source_len != 1 && source_len != output_len && source_len > selector_len)
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let kernel = bound(indexed_assign, vec![output_len as u64].into_boxed_slice())?;
    if request.output.kind == ResidentValueKind::Snapshot {
        Ok(kernel.with_snapshot_schemas(request.schemas.clone()))
    } else {
        Ok(kernel)
    }
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
    if request.inputs.is_empty()
        || request
            .inputs
            .iter()
            .any(|input| input.kind != ResidentValueKind::F64)
        || request.output.kind != ResidentValueKind::F64
        || request
            .inputs
            .iter()
            .any(|input| input.shape.columns != request.output.shape.columns)
        || request
            .inputs
            .iter()
            .map(|input| input.shape.rows)
            .sum::<u32>()
            != request.output.shape.rows
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let mut parameters = Vec::with_capacity(1 + request.inputs.len() * 2);
    parameters.push(request.inputs.len() as u64);
    for input in request.inputs {
        parameters.push(input.shape.rows as u64);
        parameters.push(input.shape.columns as u64);
    }
    bound(concatenate_vertical, parameters.into_boxed_slice())
}

fn bind_range_inclusive(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_build(request, 2, &["range"], "inclusive-output")?;
    require_kind(
        request,
        &[ResidentValueKind::F64, ResidentValueKind::F64],
        ResidentValueKind::F64,
    )?;
    require_f64_scalar_range_layout(request)?;
    bound(range_inclusive, Vec::<u64>::new().into_boxed_slice())
}

fn bind_range_exclusive(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_build(request, 2, &["range"], "exclusive-output")?;
    require_kind(
        request,
        &[ResidentValueKind::F64, ResidentValueKind::F64],
        ResidentValueKind::F64,
    )?;
    require_f64_scalar_range_layout(request)?;
    bound(range_exclusive, Vec::<u64>::new().into_boxed_slice())
}

fn bind_range_increment_inclusive(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_build(request, 3, &["range"], "inclusive-increment-output")?;
    require_kind(
        request,
        &[
            ResidentValueKind::F64,
            ResidentValueKind::F64,
            ResidentValueKind::F64,
        ],
        ResidentValueKind::F64,
    )?;
    require_f64_scalar_range_layout(request)?;
    bound(
        range_increment_inclusive,
        Vec::<u64>::new().into_boxed_slice(),
    )
}

fn bind_range_increment_exclusive(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_build(request, 3, &["range"], "exclusive-increment-output")?;
    require_kind(
        request,
        &[
            ResidentValueKind::F64,
            ResidentValueKind::F64,
            ResidentValueKind::F64,
        ],
        ResidentValueKind::F64,
    )?;
    require_f64_scalar_range_layout(request)?;
    bound(
        range_increment_exclusive,
        Vec::<u64>::new().into_boxed_slice(),
    )
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
        require_f64_lengths(request, &[1, 1], 1)?;
        return bound(n_choose_k_scalar, Vec::<u64>::new().into_boxed_slice());
    }

    validate_build(request, 2, &["combinatorics"], "n-choose-k-matrix-output")?;
    require_kind(
        request,
        &[ResidentValueKind::F64, ResidentValueKind::F64],
        ResidentValueKind::F64,
    )?;
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

fn checked_combination_count(n: usize, k: usize) -> Option<usize> {
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
    if request.inputs.len() != 2
        || request.inputs[0].kind != ResidentValueKind::F64
        || !matches!(
            request.inputs[1].kind,
            ResidentValueKind::F64 | ResidentValueKind::Index
        )
        || request.output.kind != ResidentValueKind::F64
        || request.output.shape.len() != request.inputs[1].shape.len()
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
        || !matches!(
            request.inputs[1].kind,
            ResidentValueKind::F64 | ResidentValueKind::Index
        )
        || request.inputs[1].shape != ResidentShape::SCALAR
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
        || !matches!(
            request.inputs[0].kind,
            ResidentValueKind::F64 | ResidentValueKind::Index | ResidentValueKind::Snapshot
        )
        || request.output.kind != ResidentValueKind::Index
        || request.output.shape.len() != request.inputs[0].shape.len()
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
        || !matches!(row.kind, ResidentValueKind::F64 | ResidentValueKind::Index)
        || !matches!(
            column.kind,
            ResidentValueKind::F64 | ResidentValueKind::Index
        )
        || row.shape != ResidentShape::SCALAR
        || column.shape != ResidentShape::SCALAR
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
    require_kind(
        request,
        &[ResidentValueKind::F64, ResidentValueKind::F64],
        ResidentValueKind::F64,
    )?;
    let [lhs, rhs] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    if lhs.shape.columns != rhs.shape.rows
        || request.output.shape.rows != lhs.shape.rows
        || request.output.shape.columns != rhs.shape.columns
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    bound(
        matrix_multiply,
        vec![
            lhs.shape.rows as u64,
            lhs.shape.columns as u64,
            rhs.shape.columns as u64,
        ]
        .into_boxed_slice(),
    )
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
    let selected = request.inputs[1]
        .shape
        .len()
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    if request.output.shape.len() != Some(source.shape.rows as usize * selected) {
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
    if request.inputs.get(1).map(|input| input.shape) != Some(ResidentShape::SCALAR) {
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
    if request.inputs[1].shape != ResidentShape::SCALAR
        || request.output.shape.len() != Some(source.shape.columns as usize)
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
    let selected = request.inputs[1]
        .shape
        .len()
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    if request.output.shape.len() != Some(selected * source.shape.columns as usize) {
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
        || !matches!(
            indices.kind,
            ResidentValueKind::F64 | ResidentValueKind::Index
        )
        || request.output.kind != ResidentValueKind::F64
        || base.shape != request.output.shape
        || source.shape.rows as usize
            != indices
                .shape
                .len()
                .ok_or(ResidentKernelBindError::UnsupportedLayout)?
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
            indices.shape.len().unwrap_or(0) as u64,
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
        || !matches!(
            request.inputs[1].kind,
            ResidentValueKind::F64 | ResidentValueKind::Index
        )
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
    match input(inputs, input_index)? {
        ResidentValueRef::Index(values) => values
            .get(ordinal)
            .copied()
            .ok_or(ResidentKernelError::InvalidInput),
        ResidentValueRef::F64(values) => {
            let value = *values
                .get(ordinal)
                .ok_or(ResidentKernelError::InvalidInput)?;
            if !value.is_finite() || value < 0.0 || value.fract() != 0.0 {
                return Err(ResidentKernelError::InvalidInput);
            }
            Ok(value as u64)
        }
        ResidentValueRef::Bool(_) | ResidentValueRef::String(_) | ResidentValueRef::Snapshot(_) => {
            Err(ResidentKernelError::InvalidInput)
        }
    }
}

#[derive(Clone, Copy)]
enum ValidatedIndices<'a> {
    Index(&'a [u64]),
    F64(&'a [f64]),
}

impl<'a> ValidatedIndices<'a> {
    fn new(selector: ResidentValueRef<'a>, upper: usize) -> Result<Self, ResidentKernelError> {
        let indices = match selector {
            ResidentValueRef::Index(values) => Self::Index(values),
            ResidentValueRef::F64(values) => {
                if values
                    .iter()
                    .any(|value| !value.is_finite() || *value < 0.0 || value.fract() != 0.0)
                {
                    return Err(ResidentKernelError::InvalidInput);
                }
                Self::F64(values)
            }
            _ => return Err(ResidentKernelError::InvalidShape),
        };
        indices.try_for_each(|_, index| checked_one_based(index, upper).map(|_| ()))?;
        Ok(indices)
    }

    fn len(self) -> usize {
        match self {
            Self::Index(values) => values.len(),
            Self::F64(values) => values.len(),
        }
    }

    fn try_for_each<E>(
        self,
        mut visitor: impl FnMut(usize, u64) -> Result<(), E>,
    ) -> Result<(), E> {
        match self {
            Self::Index(values) => {
                for (ordinal, index) in values.iter().copied().enumerate() {
                    visitor(ordinal, index)?;
                }
            }
            Self::F64(values) => {
                for (ordinal, index) in values.iter().copied().enumerate() {
                    visitor(ordinal, index as u64)?;
                }
            }
        }
        Ok(())
    }

    fn try_for_each_position<E>(
        self,
        mut visitor: impl FnMut(usize, usize) -> Result<(), E>,
    ) -> Result<(), E> {
        self.try_for_each(|ordinal, index| visitor(ordinal, index as usize - 1))
    }
}

#[derive(Clone, Copy)]
enum ValidatedPositions<'a> {
    Mask(&'a [u8]),
    Indices(ValidatedIndices<'a>),
}

impl<'a> ValidatedPositions<'a> {
    fn new(selector: ResidentValueRef<'a>, output_len: usize) -> Result<Self, ResidentKernelError> {
        match selector {
            ResidentValueRef::Bool(values) if values.len() == output_len => {
                if values.iter().any(|value| *value > 1) {
                    return Err(ResidentKernelError::InvalidInput);
                }
                Ok(Self::Mask(values))
            }
            ResidentValueRef::Bool(_) => Err(ResidentKernelError::InvalidShape),
            selector => ValidatedIndices::new(selector, output_len).map(Self::Indices),
        }
    }

    fn len(self) -> usize {
        match self {
            Self::Mask(values) => values.iter().filter(|value| **value != 0).count(),
            Self::Indices(indices) => indices.len(),
        }
    }

    fn is_mask(self) -> bool {
        matches!(self, Self::Mask(_))
    }

    fn maximum_position(self) -> Option<usize> {
        let mut maximum = None;
        let result = self.try_for_each(|_, position| {
            maximum = Some(maximum.map_or(position, |current: usize| current.max(position)));
            Ok::<(), core::convert::Infallible>(())
        });
        match result {
            Ok(()) => maximum,
            Err(error) => match error {},
        }
    }

    fn try_for_each<E>(
        self,
        mut visitor: impl FnMut(usize, usize) -> Result<(), E>,
    ) -> Result<(), E> {
        match self {
            Self::Mask(values) => {
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
    let cloned_bytes = left
        .canonical_payload_len(schemas)
        .map_err(|_| ResidentKernelError::InvalidInput)?
        .checked_add(
            right
                .canonical_payload_len(schemas)
                .map_err(|_| ResidentKernelError::InvalidInput)?,
        )
        .ok_or(ResidentKernelError::InvalidShape)?;
    let container_bytes = staged_elements
        .checked_mul(core::mem::size_of::<ValueData>())
        .ok_or(ResidentKernelError::InvalidShape)?;
    super::budget::KernelCostEstimate {
        comparison_work: output_len,
        compute_work: output_len,
        temporary_bytes: cloned_bytes
            .checked_add(container_bytes)
            .ok_or(ResidentKernelError::InvalidShape)?,
        cloned_bytes,
        ..super::budget::KernelCostEstimate::default()
    }
    .admit()?;

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
    let canonical_index = |mode: u64, index: usize| {
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
    let mut changed = false;
    for (index, target) in output.iter_mut().enumerate() {
        let left = left_values
            .get(canonical_index(*left_mode, index))
            .ok_or(ResidentKernelError::InvalidShape)?;
        let right = right_values
            .get(canonical_index(*right_mode, index))
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
    match (input(inputs, 0)?, output) {
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

fn retain_empty_matrix_comprehension(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() == 1 && input(inputs, 0)?.is_empty() && output.len() == 0 {
        return Ok(false);
    }
    if inputs.len() == 1 {
        let (ResidentValueRef::Snapshot([source]), ResidentValueMut::Snapshot([target])) =
            (input(inputs, 0)?, output)
        else {
            return Err(ResidentKernelError::InvalidShape);
        };
        let schemas = kernel
            .snapshot_schemas()
            .ok_or(ResidentKernelError::InvalidInput)?;
        let equal = match (source, target.as_ref()) {
            (None, None) => true,
            (Some(source), Some(target)) => source
                .language_eq(schemas, target, schemas)
                .map_err(|_| ResidentKernelError::InvalidInput)?,
            _ => false,
        };
        target.clone_from(source);
        return Ok(!equal);
    }
    for index in 0..inputs.len() {
        if !input(inputs, index)?.is_empty() {
            return Err(ResidentKernelError::InvalidShape);
        }
    }
    match output {
        ResidentValueMut::Bool(values) if values.is_empty() => Ok(false),
        ResidentValueMut::Index(values) if values.is_empty() => Ok(false),
        ResidentValueMut::F64(values) if values.is_empty() => Ok(false),
        ResidentValueMut::String(values) if values.is_empty() => Ok(false),
        ResidentValueMut::Snapshot(values) if values.is_empty() => Ok(false),
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
    let output_len = kernel.parameters()[0] as usize;
    if output.len() != output_len {
        return Err(ResidentKernelError::InvalidShape);
    }
    let positions = ValidatedPositions::new(input(inputs, 1)?, output_len)?;
    let source = input(inputs, 0)?;
    let mask_routing = positions.is_mask();
    let source_index = |ordinal: usize, position: usize| {
        if source.len() == 1 {
            0
        } else if mask_routing || source.len() == output_len {
            position
        } else {
            ordinal
        }
    };
    let source_shape_valid = if source.len() == 1 {
        true
    } else if mask_routing {
        positions
            .maximum_position()
            .is_none_or(|position| position < source.len())
    } else {
        source.len() == output_len || source.len() == positions.len()
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
            let mut changed = false;
            positions.try_for_each(|ordinal, position| {
                let next = &source[source_index(ordinal, position)];
                changed |= output[position] != *next;
                output[position].clone_from(next);
                Ok::<(), ResidentKernelError>(())
            })?;
            Ok(changed)
        }
        (ResidentValueRef::Snapshot(source), ResidentValueMut::Snapshot(output)) => {
            let schemas = kernel
                .snapshot_schemas()
                .ok_or(ResidentKernelError::InvalidInput)?;
            let mut changed = false;
            positions.try_for_each(|ordinal, position| {
                let next = &source[source_index(ordinal, position)];
                let equal = match (next, &output[position]) {
                    (None, None) => true,
                    (Some(next), Some(current)) => next
                        .language_eq(schemas, current, schemas)
                        .map_err(|_| ResidentKernelError::InvalidInput)?,
                    _ => false,
                };
                changed |= !equal;
                Ok(())
            })?;
            positions.try_for_each(|ordinal, position| {
                let next = &source[source_index(ordinal, position)];
                output[position].clone_from(next);
                Ok::<(), ResidentKernelError>(())
            })?;
            Ok(changed)
        }
        _ => Err(ResidentKernelError::InvalidShape),
    }
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

fn cosine(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    unary_f64(inputs, output, f64::cos)
}

fn absolute(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    unary_f64(inputs, output, f64::abs)
}

fn floor(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    unary_f64(inputs, output, f64::floor)
}

fn square_root(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    unary_f64(inputs, output, f64::sqrt)
}

fn sine(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    unary_f64(inputs, output, f64::sin)
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
    let left = f32_snapshot_values(left).ok_or(ResidentKernelError::InvalidInput)?;
    let right = f32_snapshot_values(right).ok_or(ResidentKernelError::InvalidInput)?;
    if left.len() != right.len() {
        return Err(ResidentKernelError::InvalidShape);
    }
    let values = left
        .into_iter()
        .zip(right)
        .map(|(left, right)| operation(left, right))
        .collect::<Vec<_>>();
    let metadata = kernel
        .snapshot_output()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let schemas = kernel
        .snapshot_schemas()
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
    let next = ValueDraft {
        schema: metadata.schema,
        shape_values: metadata
            .shape
            .parameter_values()
            .to_vec()
            .into_boxed_slice(),
        data,
    }
    .finalize(&SnapshotValidationContext::new(schemas))
    .map_err(|_| ResidentKernelError::InvalidOutput)?;
    if next.schema_key() != metadata.schema_key {
        return Err(ResidentKernelError::InvalidOutput);
    }
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

fn transpose(
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
    if input.len() != rows * columns || output.len() != input.len() {
        return Err(ResidentKernelError::InvalidShape);
    }
    Ok(replace_f64(output, |index| {
        let output_row = index % columns;
        let output_column = index / columns;
        input[output_column + output_row * rows]
    }))
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

fn concatenate_horizontal(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let output = f64_output(output)?;
    if kernel.parameters().first().copied() != Some(inputs.len() as u64) {
        return Err(ResidentKernelError::InvalidInput);
    }
    let rows = kernel.parameters().get(1).copied().unwrap_or(1) as usize;
    let mut cursor = 0usize;
    let mut changed = false;
    for ordinal in 0..inputs.len() {
        let input = f64_input(inputs, ordinal)?;
        let input_rows = kernel.parameters()[1 + ordinal * 2] as usize;
        let input_columns = kernel.parameters()[2 + ordinal * 2] as usize;
        if input_rows != rows || input.len() != input_rows * input_columns {
            return Err(ResidentKernelError::InvalidShape);
        }
        let target = &mut output[cursor..cursor + input.len()];
        changed |= target
            .iter()
            .zip(input)
            .any(|(left, right)| left.to_bits() != right.to_bits());
        target.copy_from_slice(input);
        cursor += input.len();
    }
    if cursor != output.len() {
        return Err(ResidentKernelError::IncompleteOutput);
    }
    Ok(changed)
}

fn concatenate_vertical(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let output = f64_output(output)?;
    let columns = kernel.parameters().get(2).copied().unwrap_or(1) as usize;
    let output_rows = output.len() / columns;
    let mut changed = false;
    let mut row_base = 0usize;
    for ordinal in 0..inputs.len() {
        let input = f64_input(inputs, ordinal)?;
        let rows = kernel.parameters()[1 + ordinal * 2] as usize;
        let input_columns = kernel.parameters()[2 + ordinal * 2] as usize;
        if input_columns != columns || input.len() != rows * columns {
            return Err(ResidentKernelError::InvalidShape);
        }
        for column in 0..columns {
            for row in 0..rows {
                let target = row_base + row + column * output_rows;
                let source = row + column * rows;
                changed |= output[target].to_bits() != input[source].to_bits();
                output[target] = input[source];
            }
        }
        row_base += rows;
    }
    if row_base != output_rows {
        return Err(ResidentKernelError::IncompleteOutput);
    }
    Ok(changed)
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
    let mut selected = vec![0usize; k];
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
    let previous = output.to_vec();
    visit(values, &mut selected, 0, 0, output, &mut column);
    if column != combinations {
        return Err(ResidentKernelError::IncompleteOutput);
    }
    Ok(previous != output)
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
    let indices_len = input(inputs, 1)?.len();
    if output.len() != indices_len {
        return Err(ResidentKernelError::InvalidShape);
    }
    let indices = ValidatedIndices::new(input(inputs, 1)?, source_values.len())?;
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
    if input.len() != output.len() {
        return Err(ResidentKernelError::InvalidShape);
    }
    let mut changed = false;
    for (ordinal, target) in output.iter_mut().enumerate() {
        let next = match input {
            ResidentValueRef::F64(values) => values[ordinal].to_portable_index(),
            ResidentValueRef::Index(values) => values[ordinal].to_portable_index(),
            ResidentValueRef::Snapshot(values) => {
                let value = values[ordinal]
                    .as_ref()
                    .ok_or(ResidentKernelError::InvalidInput)?;
                match value.data() {
                    ValueData::U8(value) => value.to_portable_index(),
                    ValueData::U16(value) => value.to_portable_index(),
                    ValueData::U32(value) => value.to_portable_index(),
                    ValueData::U64(value) => value.to_portable_index(),
                    ValueData::U128(value) => value.to_portable_index(),
                    ValueData::I8(value) => value.to_portable_index(),
                    ValueData::I16(value) => value.to_portable_index(),
                    ValueData::I32(value) => value.to_portable_index(),
                    ValueData::I64(value) => value.to_portable_index(),
                    ValueData::I128(value) => value.to_portable_index(),
                    ValueData::F32(value) => value.to_f32().to_portable_index(),
                    ValueData::F64(value) => value.to_f64().to_portable_index(),
                    ValueData::Index(value) => value.to_portable_index(),
                    _ => return Err(ResidentKernelError::InvalidInput),
                }
            }
            ResidentValueRef::Bool(_) | ResidentValueRef::String(_) => {
                return Err(ResidentKernelError::InvalidInput);
            }
        }
        .ok_or(ResidentKernelError::InvalidInput)?;
        changed |= *target != next;
        *target = next;
    }
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
    if lhs.len() != rows * inner || rhs.len() != inner * columns || output.len() != rows * columns {
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
        (left, right) => {
            checked_numeric_binary!(left, right, checked_add).ok_or(ResidentKernelError::Arithmetic)
        }
    }
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

fn write_snapshot_data(
    kernel: &BoundResidentKernel,
    output: ResidentValueMut<'_>,
    data: ValueDataDraft,
) -> Result<bool, ResidentKernelError> {
    let metadata = kernel
        .snapshot_output()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let next = ValueDraft {
        schema: metadata.schema,
        shape_values: metadata
            .shape
            .parameter_values()
            .to_vec()
            .into_boxed_slice(),
        data,
    }
    .finalize(&SnapshotValidationContext::new(schemas))
    .map_err(|_| ResidentKernelError::InvalidOutput)?;
    if next.schema_key() != metadata.schema_key {
        return Err(ResidentKernelError::InvalidOutput);
    }
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
    let left = snapshot_numeric_elements(left)?;
    let right = snapshot_numeric_elements(right)?;
    if left.len() != right.len() {
        return Err(ResidentKernelError::InvalidShape);
    }
    let metadata = kernel
        .snapshot_output()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let output_schema = schemas
        .get(metadata.schema)
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let mut next = numeric_zero(output_schema.body())?;
    for (left, right) in left.into_iter().zip(right) {
        next = numeric_add(next, numeric_multiply(left, right)?)?;
    }
    write_snapshot_data(kernel, output, next)
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
    write_snapshot_data(
        kernel,
        output,
        ValueDataDraft::Matrix(
            canonical_next
                .into_iter()
                .map(|value| ValueDataDraft::F32(F32Bits::from_f32(value)))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
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
    let selected_columns = input(inputs, 1)?.len();
    if output.len()
        != rows
            .checked_mul(selected_columns)
            .ok_or(ResidentKernelError::InvalidShape)?
    {
        return Err(ResidentKernelError::InvalidShape);
    }
    let selected_columns = ValidatedIndices::new(input(inputs, 1)?, source_columns)?;
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
    let selected_rows = input(inputs, 1)?.len();
    if output.len()
        != selected_rows
            .checked_mul(columns)
            .ok_or(ResidentKernelError::InvalidShape)?
    {
        return Err(ResidentKernelError::InvalidShape);
    }
    let selected_rows = ValidatedIndices::new(input(inputs, 1)?, rows)?;
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
    let index_count = input(inputs, 1)?.len();
    if index_count != source_rows {
        return Err(ResidentKernelError::InvalidShape);
    }
    let rows = ValidatedIndices::new(input(inputs, 1)?, target_rows)?;
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
        let non_row_handle = builder.insert(schema(matrix(2, 2))).unwrap();
        let build = builder.finish().unwrap();
        let scalar = build.resolve(scalar_handle).unwrap();
        let matrix_one = build.resolve(matrix_one_handle).unwrap();
        let row = build.resolve(row_handle).unwrap();
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
        };
        let row_shape = ResidentShape {
            rows: 1,
            columns: 4,
        };
        let non_row_shape = ResidentShape {
            rows: 2,
            columns: 2,
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
    fn indexed_assignment_capability_rejects_snapshot_aggregate_lanes() {
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
        assert!(matches!(
            bind_indexed_assign(&ResidentKernelBindRequest {
                contract: &contract,
                schemas: &schemas,
                inputs: &[
                    port(map, ResidentValueKind::Snapshot),
                    port(tuple, ResidentValueKind::Snapshot),
                    port(index, ResidentValueKind::Index),
                ],
                output: port(map, ResidentValueKind::Snapshot),
            }),
            Err(ResidentKernelBindError::UnsupportedLayout)
        ));

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
        let kernel = BoundResidentKernel::new(indexed_assign, Box::new([3]));
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
    fn strict_identity_distinguishes_scalar_from_one_by_one_matrix() {
        let shape_instance = || {
            mech_core::SchemaDraft {
                dimension_parameters: Box::new([]),
                body: SchemaBody::FloatingPoint(mech_core::FloatWidth::W64),
            }
            .finalize()
            .unwrap()
            .instantiate_shape(Box::new([]))
            .unwrap()
        };
        let scalar = mech_core::ResidentPortLayout {
            schema_id: mech_core::SchemaId::new(1),
            schema_key: mech_core::SchemaKey::from_bytes([1; 32]),
            kind: ResidentValueKind::F64,
            shape: ResidentShape::SCALAR,
            shape_instance: shape_instance(),
        };
        let matrix = mech_core::ResidentPortLayout {
            schema_id: mech_core::SchemaId::new(2),
            schema_key: mech_core::SchemaKey::from_bytes([2; 32]),
            kind: ResidentValueKind::F64,
            shape: ResidentShape::SCALAR,
            shape_instance: shape_instance(),
        };
        assert!(!strict_inputs_share_identity(&scalar, &matrix));
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

        for rejected in [-1.0, f64::NAN, PORTABLE_INDEX_MAX as f64 + 1.0] {
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
}
