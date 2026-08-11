use mech_core::{
    AccessMode, AliasPolicy, BoundResidentKernel, ChangeDetectionPolicy, DeliveryMode,
    ExternalInteraction, FunctionCatalogBuilder, MResult, OutputConstruction, RegionPolicy,
    ResidentKernelBindError, ResidentKernelBindRequest, ResidentKernelError, ResidentKernelInputs,
    ResidentShape, ResidentValueKind, ResidentValueMut, ResidentValueRef,
    ResolvedOperationContract, ShapeContractReference, ShapeRule,
};

pub(crate) fn install(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    let runtime = ["runtime"];
    register(builder, &runtime, "Access1DVDVD<f64>", bind_gather_1d)?;
    register(
        builder,
        &runtime,
        "Access2DARV<f64DMatrixDMatrixDVector>",
        bind_all_rows_columns,
    )?;
    register(builder, &runtime, "Access2DASMD<f64>", bind_all_rows_column)?;
    register(builder, &runtime, "Access2DSAMD<f64>", bind_row_all_columns)?;
    register(
        builder,
        &runtime,
        "Access2DVDAMD<f64>",
        bind_rows_all_columns,
    )?;
    register(
        builder,
        &runtime,
        "AddAssign2DRAV<f64DMatrixDMatrixDVector>",
        bind_add_indexed_rows,
    )?;
    register(builder, &runtime, "AddAssignVV<[f64]:0,0>", bind_add_assign)?;
    register(
        builder,
        &runtime,
        "HorizontalConcatenateNArgs<f64>",
        bind_horizontal,
    )?;
    register(
        builder,
        &runtime,
        "HorizontalConcatenateRDN<f64>",
        bind_horizontal,
    )?;
    register(
        builder,
        &runtime,
        "HorizontalConcatenateS1D<f64>",
        bind_horizontal,
    )?;
    register(builder, &runtime, "MulMDS<f64>", bind_mul)?;
    register(builder, &runtime, "MulMDVD<f64>", bind_mul_rows)?;
    register(builder, &runtime, "MulSS<f64>", bind_mul)?;
    register(builder, &runtime, "MulSVD<f64>", bind_mul)?;
    register(builder, &runtime, "NChooseKMatrix<f64>", bind_n_choose_k)?;
    register(builder, &runtime, "NegateS<f64>", bind_negate)?;
    register(builder, &runtime, "PowMDS<f64>", bind_pow)?;
    register(builder, &runtime, "PowSS<f64>", bind_pow)?;
    register(builder, &runtime, "PowVDS<f64>", bind_pow)?;
    register(
        builder,
        &runtime,
        "RangeInclusiveScalar<f64RowDVector>",
        bind_range_inclusive,
    )?;
    register(builder, &runtime, "StatsSumColumnMD<f64>", bind_sum_columns)?;
    register(
        builder,
        &runtime,
        "SubAssign2DRAV<f64DMatrixDMatrixDVector>",
        bind_sub_indexed_rows,
    )?;
    register(builder, &runtime, "SubMDMD<f64>", bind_sub)?;
    register(builder, &runtime, "TransposeMD<f64>", bind_transpose)?;
    register(builder, &runtime, "TransposeRD<f64>", bind_transpose)?;
    register(
        builder,
        &runtime,
        "VerticalConcatenateVDN<f64>",
        bind_vertical,
    )?;
    register(builder, &runtime, "Assign<f64DVector>", bind_assign)?;
    register(builder, &runtime, "Assign<f64DMatrix>", bind_assign)?;

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

fn bind_assign(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_full_write(
        request,
        1,
        ShapeRule::SameAsInput { input: 0 },
        ChangeDetectionPolicy::KernelReported,
    )?;
    require_kind(request, &[ResidentValueKind::F64], ResidentValueKind::F64)?;
    if request.inputs[0].shape != request.output.shape {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    bound(assign, Vec::<u64>::new().into_boxed_slice())
}

fn bind_negate(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let change = if request.output.shape == ResidentShape::SCALAR {
        ChangeDetectionPolicy::ExactScalar
    } else {
        ChangeDetectionPolicy::KernelReported
    };
    validate_full_write(request, 1, ShapeRule::SameAsInput { input: 0 }, change)?;
    require_kind(request, &[ResidentValueKind::F64], ResidentValueKind::F64)?;
    if request.inputs[0].shape != request.output.shape {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    bound(negate, Vec::<u64>::new().into_boxed_slice())
}

fn bind_sub(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_binary(request, subtract)
}

fn bind_mul(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_binary(request, multiply)
}

fn bind_binary(
    request: &ResidentKernelBindRequest<'_>,
    executor: mech_core::ResidentKernelExecutor,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let change = if request.output.shape == ResidentShape::SCALAR {
        ChangeDetectionPolicy::ExactScalar
    } else {
        ChangeDetectionPolicy::KernelReported
    };
    validate_full_write(request, 2, ShapeRule::Declared, change)?;
    require_kind(
        request,
        &[ResidentValueKind::F64, ResidentValueKind::F64],
        ResidentValueKind::F64,
    )?;
    let output_len = request
        .output
        .shape
        .len()
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    if request.inputs.iter().any(|input| {
        input
            .shape
            .len()
            .is_none_or(|len| len != 1 && len != output_len)
    }) {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    bound(executor, Vec::<u64>::new().into_boxed_slice())
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
    if request
        .inputs
        .iter()
        .any(|input| input.shape != ResidentShape::SCALAR)
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    bound(range_inclusive, Vec::<u64>::new().into_boxed_slice())
}

fn bind_n_choose_k(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_build(request, 2, &["combinatorics"], "n-choose-k-matrix-output")?;
    require_kind(
        request,
        &[ResidentValueKind::F64, ResidentValueKind::F64],
        ResidentValueKind::F64,
    )?;
    if request.inputs[1].shape != ResidentShape::SCALAR || request.output.shape.rows != 2 {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    bound(n_choose_k, Vec::<u64>::new().into_boxed_slice())
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
        ResidentValueRef::Bool(_) => Err(ResidentKernelError::InvalidInput),
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

fn f64_array<const N: usize>(
    inputs: &dyn ResidentKernelInputs,
    index: usize,
) -> Result<&[f64; N], ResidentKernelError> {
    f64_input(inputs, index)?
        .try_into()
        .map_err(|_| ResidentKernelError::InvalidShape)
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

fn assign(
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
    let changed = output
        .iter()
        .zip(source)
        .any(|(left, right)| left.to_bits() != right.to_bits());
    output.copy_from_slice(source);
    Ok(changed)
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

fn binary_f64(
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
    let output_len = output.len();
    let pick = |values: &[f64], index: usize| match values.len() {
        1 => Some(values[0]),
        len if len == output_len => Some(values[index]),
        _ => None,
    };
    if pick(left, 0).is_none() || pick(right, 0).is_none() {
        return Err(ResidentKernelError::InvalidShape);
    }
    Ok(replace_f64(output, |index| {
        operation(pick(left, index).unwrap(), pick(right, index).unwrap())
    }))
}

fn subtract(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    binary_f64(inputs, output, |left, right| left - right)
}

fn multiply(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    binary_f64(inputs, output, |left, right| left * right)
}

fn power(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    binary_f64(inputs, output, f64::powf)
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
    let step = if end >= start { 1.0 } else { -1.0 };
    Ok(replace_f64(output, |index| start + step * index as f64))
}

fn n_choose_k(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 2 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let values = f64_input(inputs, 0)?;
    let k = *f64_input(inputs, 1)?
        .first()
        .ok_or(ResidentKernelError::InvalidInput)? as usize;
    let output = f64_output(output)?;
    if k == 0 || output.len() % k != 0 {
        return Err(ResidentKernelError::InvalidShape);
    }
    let combinations = output.len() / k;
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
    let mut changed = false;
    for (ordinal, target) in output.iter_mut().enumerate() {
        let index = checked_one_based(index_at(inputs, 1, ordinal)?, source_values.len())?;
        let next = source_values[index];
        changed |= target.to_bits() != next.to_bits();
        *target = next;
    }
    Ok(changed)
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
    if output.len() != rows * selected_columns {
        return Err(ResidentKernelError::InvalidShape);
    }
    let mut changed = false;
    for ordinal in 0..selected_columns {
        let column = checked_one_based(index_at(inputs, 1, ordinal)?, source_columns)?;
        let source = &source[column * rows..(column + 1) * rows];
        let target = &mut output[ordinal * rows..(ordinal + 1) * rows];
        changed |= target
            .iter()
            .zip(source)
            .any(|(left, right)| left.to_bits() != right.to_bits());
        target.copy_from_slice(source);
    }
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
    if output.len() != selected_rows * columns {
        return Err(ResidentKernelError::InvalidShape);
    }
    let mut changed = false;
    let mut target_index = 0;
    for column in 0..columns {
        for ordinal in 0..selected_rows {
            let row = checked_one_based(index_at(inputs, 1, ordinal)?, rows)?;
            let next = source[row + column * rows];
            changed |= output[target_index].to_bits() != next.to_bits();
            output[target_index] = next;
            target_index += 1;
        }
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
    if output.len() != target_rows * columns || source_values.len() != source_rows * columns {
        return Err(ResidentKernelError::InvalidShape);
    }
    let mut changed = false;
    let index_count = input(inputs, 1)?.len();
    for occurrence in 0..index_count {
        let row = checked_one_based(index_at(inputs, 1, occurrence)?, target_rows)?;
        for column in 0..columns {
            let target = row + column * target_rows;
            let source = occurrence + column * source_rows;
            let next = operation(output[target], source_values[source]);
            changed |= next.to_bits() != output[target].to_bits();
            output[target] = next;
        }
    }
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
    fn out_of_range_index_is_a_structured_kernel_rejection() {
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
        assert_eq!(candidate, [9.0, 20.0]);
    }
}
