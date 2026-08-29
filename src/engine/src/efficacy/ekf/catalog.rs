//! Hidden ordinary-source compiler catalog for the frozen EKF efficacy path.

use std::sync::{Arc, LazyLock};

use mech_core::snapshot::F64Bits;
use mech_core::{
    AccessMode, AliasPolicy, BytecodeCompilerContext, CanonicalFunctionSpecializer,
    ChangeDetectionPolicy, DeliveryMode, DimensionExpr, ExternalInteraction, FunctionCatalog,
    FunctionCatalogBuilder, FunctionExport, FunctionExposure, FunctionInstance, FunctionInvocation,
    FunctionRuntimeType, FunctionStatePort, GuardFunctionSafety, InputPortLayout, InputPortPolicy,
    MResult, MechError, MechErrorKind, MechFunction, MechFunctionCompiler, MechFunctionFactory,
    MechFunctionImpl, OperationContractDeclaration, OutputConstruction, OutputPortPolicy,
    ReactiveNodeKind, Ref, Register, RuntimeFunctionContract, RuntimeFunctionSignature,
    RuntimeOutputAliasPolicy, SchemaBody, ShapeRule, SpecializationContext,
    SpecializationInvocation, SpecializedFunction, ValueCell, ValueData, ValueDataDraft,
    compile_runtime_produced_value_cell_register_with_seed, compile_value_cell_register,
    function_shape_contract_violation,
};
use nalgebra::{DMatrix, DVector};

use super::math::{self, EkfMathError};
use super::operation::{
    EkfKernel, EkfPredicate, FROZEN_EKF_OPERATIONS, FrozenEkfOperation, FrozenEkfValueShape,
    operation_spec,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum FrozenEkfOperationFailure {
    Arity {
        expected: usize,
        found: usize,
    },
    Shape {
        argument: usize,
        expected: FrozenEkfValueShape,
    },
    Math(EkfMathError),
    OutputShape,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FrozenEkfOperationError {
    pub operation: &'static str,
    pub reason: FrozenEkfOperationFailure,
}

impl MechErrorKind for FrozenEkfOperationError {
    fn name(&self) -> &str {
        "FrozenEkfOperationError"
    }

    fn message(&self) -> String {
        format!(
            "{} rejected frozen EKF data: {:?}",
            self.operation, self.reason
        )
    }
}

fn operation_error(operation: FrozenEkfOperation, reason: FrozenEkfOperationFailure) -> MechError {
    MechError::new(
        FrozenEkfOperationError {
            operation: operation_spec(operation).canonical_name,
            reason,
        },
        None,
    )
    .with_compiler_loc()
}

fn semantic_declaration(
    input_count: usize,
    change_detection: ChangeDetectionPolicy,
) -> OperationContractDeclaration {
    OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            (0..input_count)
                .map(|_| InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                })
                .collect(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            alias: AliasPolicy::NoAlias,
            change_detection,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    }
}

static KERNEL_1: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| semantic_declaration(1, ChangeDetectionPolicy::KernelReported));
static KERNEL_2: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| semantic_declaration(2, ChangeDetectionPolicy::KernelReported));
static KERNEL_3: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| semantic_declaration(3, ChangeDetectionPolicy::KernelReported));
static KERNEL_4: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| semantic_declaration(4, ChangeDetectionPolicy::KernelReported));
static PREDICATE_1: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| semantic_declaration(1, ChangeDetectionPolicy::ExactScalar));
static PREDICATE_2: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| semantic_declaration(2, ChangeDetectionPolicy::ExactScalar));

pub(crate) fn semantic_contract(
    operation: FrozenEkfOperation,
) -> &'static OperationContractDeclaration {
    let spec = operation_spec(operation);
    match (spec.change_detection, spec.inputs.len()) {
        (ChangeDetectionPolicy::KernelReported, 1) => &KERNEL_1,
        (ChangeDetectionPolicy::KernelReported, 2) => &KERNEL_2,
        (ChangeDetectionPolicy::KernelReported, 3) => &KERNEL_3,
        (ChangeDetectionPolicy::KernelReported, 4) => &KERNEL_4,
        (ChangeDetectionPolicy::ExactScalar, 1) => &PREDICATE_1,
        (ChangeDetectionPolicy::ExactScalar, 2) => &PREDICATE_2,
        _ => unreachable!("the frozen operation table has six contract forms"),
    }
}

pub(crate) struct FrozenEkfSpecializer {
    operation: FrozenEkfOperation,
}

impl CanonicalFunctionSpecializer for FrozenEkfSpecializer {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        _: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        let inputs = invocation
            .inputs()
            .iter()
            .map(|input| input.cell().cloned())
            .collect::<MResult<Vec<_>>>()?;
        validate_source_arguments(self.operation, &inputs)?;
        let output = allocate_output(operation_spec(self.operation).output)?;
        let function = FrozenEkfFunction {
            operation: self.operation,
            inputs: inputs.clone().into_boxed_slice(),
            output: output.clone(),
        };
        function.solve_result()?;
        Ok(SpecializedFunction::new(FunctionInstance::new(
            Box::new(function),
            invocation_from_cells(output, inputs.into_boxed_slice()),
        )))
    }

    fn guard_safety(&self) -> GuardFunctionSafety {
        GuardFunctionSafety::PureStatic
    }
}

#[derive(Debug)]
pub(crate) struct FrozenEkfFunction {
    operation: FrozenEkfOperation,
    inputs: Box<[ValueCell]>,
    output: ValueCell,
}

impl MechFunctionImpl for FrozenEkfFunction {
    fn solve_result(&self) -> MResult<()> {
        validate_operation_cells(self.operation, &self.output, &self.inputs)?;
        evaluate_into(self.operation, &self.inputs, &self.output)
    }

    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_cell(&self.output))
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(semantic_contract(self.operation))
    }

    fn reactive_node_kind(&self) -> ReactiveNodeKind {
        ReactiveNodeKind::Combinational
    }

    fn to_string(&self) -> String {
        operation_spec(self.operation).canonical_name.to_owned()
    }
}

impl MechFunctionCompiler for FrozenEkfFunction {
    fn compiler_owned_value_cells(&self) -> Vec<ValueCell> {
        std::iter::once(self.output.clone())
            .chain(self.inputs.iter().cloned())
            .collect()
    }

    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let zero = allocate_output(operation_spec(self.operation).output)?.snapshot()?;
        let destination =
            compile_runtime_produced_value_cell_register_with_seed(&self.output, &zero, context)?;
        let inputs = self
            .inputs
            .iter()
            .map(|argument| compile_value_cell_register(argument, context))
            .collect::<MResult<Vec<_>>>()?;
        let function = context.function_id(operation_spec(self.operation).canonical_name)?;
        match inputs.as_slice() {
            [a] => context.emit_unop(function, destination, *a),
            [a, b] => context.emit_binop(function, destination, *a, *b),
            [a, b, c] => context.emit_ternop(function, destination, *a, *b, *c),
            [a, b, c, d] => context.emit_quadop(function, destination, *a, *b, *c, *d),
            _ => unreachable!("frozen EKF operations have one to four inputs"),
        }
        Ok(destination)
    }
}

fn invocation_from_cells(output: ValueCell, inputs: Box<[ValueCell]>) -> FunctionInvocation {
    match inputs.into_vec() {
        inputs if inputs.len() == 1 => FunctionInvocation::unary(output, inputs[0].clone()),
        inputs if inputs.len() == 2 => {
            FunctionInvocation::binary(output, inputs[0].clone(), inputs[1].clone())
        }
        inputs if inputs.len() == 3 => FunctionInvocation::ternary(
            output,
            inputs[0].clone(),
            inputs[1].clone(),
            inputs[2].clone(),
        ),
        inputs if inputs.len() == 4 => FunctionInvocation::quaternary(
            output,
            inputs[0].clone(),
            inputs[1].clone(),
            inputs[2].clone(),
            inputs[3].clone(),
        ),
        inputs => FunctionInvocation::variadic(output, inputs.into_boxed_slice()),
    }
}

fn instantiate(
    operation: FrozenEkfOperation,
    invocation: FunctionInvocation,
) -> MResult<Box<dyn MechFunction>> {
    validate_operation_cells(
        operation,
        invocation.output_cell(),
        invocation.input_cells(),
    )?;
    Ok(Box::new(FrozenEkfFunction {
        operation,
        inputs: invocation.input_cells().to_vec().into_boxed_slice(),
        output: invocation.output_cell().clone(),
    }))
}

macro_rules! factory {
    ($factory:ident, $validator:ident, $operation:expr, unary, $output:ty, [$a:ty]) => {
        struct $factory;
        impl MechFunctionFactory for $factory {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::unary(
                <$output as FunctionRuntimeType>::REPRESENTATION,
                <$a as FunctionRuntimeType>::REPRESENTATION,
            );
            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                instantiate($operation, invocation)
            }
        }
        fn $validator(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
            validate_operation_cells($operation, output, inputs).map_err(|error| {
                function_shape_contract_violation(
                    operation_spec($operation).canonical_name,
                    error.simple_message(),
                )
            })
        }
    };
    ($factory:ident, $validator:ident, $operation:expr, binary, $output:ty, [$a:ty, $b:ty]) => {
        struct $factory;
        impl MechFunctionFactory for $factory {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <$output as FunctionRuntimeType>::REPRESENTATION,
                <$a as FunctionRuntimeType>::REPRESENTATION,
                <$b as FunctionRuntimeType>::REPRESENTATION,
            );
            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                instantiate($operation, invocation)
            }
        }
        fn $validator(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
            validate_operation_cells($operation, output, inputs).map_err(|error| {
                function_shape_contract_violation(
                    operation_spec($operation).canonical_name,
                    error.simple_message(),
                )
            })
        }
    };
    ($factory:ident, $validator:ident, $operation:expr, ternary, $output:ty, [$a:ty, $b:ty, $c:ty]) => {
        struct $factory;
        impl MechFunctionFactory for $factory {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::ternary(
                <$output as FunctionRuntimeType>::REPRESENTATION,
                <$a as FunctionRuntimeType>::REPRESENTATION,
                <$b as FunctionRuntimeType>::REPRESENTATION,
                <$c as FunctionRuntimeType>::REPRESENTATION,
            );
            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                instantiate($operation, invocation)
            }
        }
        fn $validator(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
            validate_operation_cells($operation, output, inputs).map_err(|error| {
                function_shape_contract_violation(
                    operation_spec($operation).canonical_name,
                    error.simple_message(),
                )
            })
        }
    };
    ($factory:ident, $validator:ident, $operation:expr, quaternary, $output:ty, [$a:ty, $b:ty, $c:ty, $d:ty]) => {
        struct $factory;
        impl MechFunctionFactory for $factory {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::quaternary(
                <$output as FunctionRuntimeType>::REPRESENTATION,
                <$a as FunctionRuntimeType>::REPRESENTATION,
                <$b as FunctionRuntimeType>::REPRESENTATION,
                <$c as FunctionRuntimeType>::REPRESENTATION,
                <$d as FunctionRuntimeType>::REPRESENTATION,
            );
            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                instantiate($operation, invocation)
            }
        }
        fn $validator(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
            validate_operation_cells($operation, output, inputs).map_err(|error| {
                function_shape_contract_violation(
                    operation_spec($operation).canonical_name,
                    error.simple_message(),
                )
            })
        }
    };
}

use EkfKernel::*;
use EkfPredicate::*;
use FrozenEkfOperation::{Kernel, Predicate};

factory!(
    TrigFactory,
    validate_trig,
    Kernel(TrigonometricState),
    unary,
    DVector<f64>,
    [DVector<f64>]
);
factory!(MotionFactory, validate_motion, Kernel(MotionJacobian), quaternary, DMatrix<f64>, [DVector<f64>, DVector<f64>, DVector<f64>, f64]);
factory!(ControlFactory, validate_control, Kernel(ControlJacobian), binary, DMatrix<f64>, [DVector<f64>, f64]);
factory!(PredictedStateFactory, validate_predicted_state, Kernel(PredictedState), quaternary, DVector<f64>, [DVector<f64>, DVector<f64>, DVector<f64>, f64]);
factory!(PredictedCovarianceFactory, validate_predicted_covariance, Kernel(PredictedCovariance), quaternary, DMatrix<f64>, [DMatrix<f64>, DMatrix<f64>, DMatrix<f64>, DMatrix<f64>]);
factory!(LandmarkFactory, validate_landmark, Kernel(LandmarkDeltaAndRange), binary, DVector<f64>, [DVector<f64>, DVector<f64>]);
factory!(MeasurementFactory, validate_measurement, Kernel(PredictedMeasurement), binary, DVector<f64>, [DVector<f64>, DVector<f64>]);
factory!(
    MeasurementJacobianFactory,
    validate_measurement_jacobian,
    Kernel(MeasurementJacobian),
    unary,
    DMatrix<f64>,
    [DVector<f64>]
);
factory!(InnovationCovarianceFactory, validate_innovation_covariance, Kernel(InnovationCovariance), ternary, DMatrix<f64>, [DMatrix<f64>, DMatrix<f64>, DMatrix<f64>]);
factory!(
    SolveFactory,
    validate_solve,
    Kernel(Solve2x2),
    unary,
    DMatrix<f64>,
    [DMatrix<f64>]
);
factory!(GainFactory, validate_gain, Kernel(KalmanGain), ternary, DMatrix<f64>, [DMatrix<f64>, DMatrix<f64>, DMatrix<f64>]);
factory!(InnovationFactory, validate_innovation, Kernel(Innovation), binary, DVector<f64>, [DVector<f64>, DVector<f64>]);
factory!(CorrectedStateFactory, validate_corrected_state, Kernel(CorrectedState), ternary, DVector<f64>, [DVector<f64>, DMatrix<f64>, DVector<f64>]);
factory!(JosephFactory, validate_joseph, Kernel(JosephCovarianceUpdate), quaternary, DMatrix<f64>, [DMatrix<f64>, DMatrix<f64>, DMatrix<f64>, DMatrix<f64>]);
factory!(
    SymmetrizationFactory,
    validate_symmetrization,
    Kernel(CovarianceSymmetrization),
    unary,
    DMatrix<f64>,
    [DMatrix<f64>]
);
factory!(FiniteFactory, validate_finite, Predicate(CandidateFinite), binary, bool, [DVector<f64>, DMatrix<f64>]);
factory!(
    PositiveFactory,
    validate_positive,
    Predicate(CovariancePositiveDiagonal),
    unary,
    bool,
    [DMatrix<f64>]
);
factory!(
    SymmetricFactory,
    validate_symmetric,
    Predicate(CovarianceSymmetric),
    unary,
    bool,
    [DMatrix<f64>]
);

macro_rules! register {
    ($builder:expr, $index:expr, $factory:ty, $validator:ident) => {{
        let spec = &FROZEN_EKF_OPERATIONS[$index];
        $builder.insert_runtime_factory_with_semantic_contract::<$factory>(
            spec.canonical_name,
            RuntimeFunctionContract::canonical_custom(
                spec.canonical_name,
                RuntimeOutputAliasPolicy::DisallowInputAlias,
                $validator,
            ),
            semantic_contract(spec.operation),
        )?;
    }};
}

pub(crate) fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    register!(builder, 0, TrigFactory, validate_trig);
    register!(builder, 1, MotionFactory, validate_motion);
    register!(builder, 2, ControlFactory, validate_control);
    register!(builder, 3, PredictedStateFactory, validate_predicted_state);
    register!(
        builder,
        4,
        PredictedCovarianceFactory,
        validate_predicted_covariance
    );
    register!(builder, 5, LandmarkFactory, validate_landmark);
    register!(builder, 6, MeasurementFactory, validate_measurement);
    register!(
        builder,
        7,
        MeasurementJacobianFactory,
        validate_measurement_jacobian
    );
    register!(
        builder,
        8,
        InnovationCovarianceFactory,
        validate_innovation_covariance
    );
    register!(builder, 9, SolveFactory, validate_solve);
    register!(builder, 10, GainFactory, validate_gain);
    register!(builder, 11, InnovationFactory, validate_innovation);
    register!(builder, 12, CorrectedStateFactory, validate_corrected_state);
    register!(builder, 13, JosephFactory, validate_joseph);
    register!(builder, 14, SymmetrizationFactory, validate_symmetrization);
    register!(builder, 15, FiniteFactory, validate_finite);
    register!(builder, 16, PositiveFactory, validate_positive);
    register!(builder, 17, SymmetricFactory, validate_symmetric);
    Ok(())
}

pub(crate) fn install_source(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    for spec in FROZEN_EKF_OPERATIONS {
        let operation = builder.insert_canonical_specializer(
            spec.canonical_name,
            Arc::new(FrozenEkfSpecializer {
                operation: spec.operation,
            }),
        )?;
        builder.insert_export(FunctionExport {
            operation,
            canonical_name: spec.canonical_name.to_owned(),
            module: Some("ekf".to_owned()),
            item: Some(spec.module_item.to_owned()),
            exposure: FunctionExposure::ModuleOnly,
        })?;
    }
    let negate =
        builder.insert_canonical_specializer("math/neg", Arc::new(FrozenF64NegateSpecializer))?;
    builder.insert_export(FunctionExport {
        operation: negate,
        canonical_name: "math/neg".to_owned(),
        module: None,
        item: None,
        exposure: FunctionExposure::Prelude,
    })?;
    Ok(())
}

struct FrozenF64NegateSpecializer;

impl CanonicalFunctionSpecializer for FrozenF64NegateSpecializer {
    fn specialize_invocation(
        &self,
        arguments: &SpecializationInvocation,
        _: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        if arguments.len() != 1 {
            return Err(MechError::new(
                mech_core::IncorrectNumberOfArguments {
                    expected: 1,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let input = arguments
            .input(0)
            .expect("unary negation input")
            .cell()?
            .clone();
        let snapshot = input.snapshot()?;
        let ValueData::F64(value) = snapshot.data() else {
            return Err(function_shape_contract_violation(
                "math/neg",
                "frozen source literal negation requires f64",
            ));
        };
        let output = ValueCell::from_exact(-value.to_f64())?;
        Ok(SpecializedFunction::new(FunctionInstance::new(
            Box::new(FrozenF64NegateFunction {
                output: output.clone(),
            }),
            FunctionInvocation::unary(output, input),
        )))
    }

    fn guard_safety(&self) -> GuardFunctionSafety {
        GuardFunctionSafety::PureStatic
    }
}

#[derive(Debug)]
struct FrozenF64NegateFunction {
    output: ValueCell,
}

impl MechFunctionImpl for FrozenF64NegateFunction {
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }

    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_cell(&self.output))
    }

    fn to_string(&self) -> String {
        "math/neg".to_owned()
    }
}

impl MechFunctionCompiler for FrozenF64NegateFunction {
    fn compiler_owned_value_cells(&self) -> Vec<ValueCell> {
        vec![self.output.clone()]
    }

    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        compile_value_cell_register(&self.output, context)
    }
}

#[doc(hidden)]
pub fn frozen_ekf_compiler_catalog() -> MResult<Arc<FunctionCatalog>> {
    let mut builder = FunctionCatalogBuilder::new();
    crate::intrinsics::catalog::install_runtime(&mut builder)?;
    crate::intrinsics::assign::install_frozen_ekf_state_runtime(&mut builder)?;
    crate::intrinsics::catalog::install_source(&mut builder)?;
    install_runtime(&mut builder)?;
    install_source(&mut builder)?;
    crate::function::install_intrinsic_resident(&mut builder)?;
    Ok(Arc::new(builder.build()?))
}

fn value_shape(value: &ValueCell) -> MResult<Option<FrozenEkfValueShape>> {
    match value.snapshot()?.data() {
        ValueData::F64(_) => Ok(Some(FrozenEkfValueShape::F64)),
        ValueData::Bool(_) => Ok(Some(FrozenEkfValueShape::Bool)),
        ValueData::Matrix(_) => {
            let SchemaBody::Matrix { dimensions, .. } = value.closed_schema_body()? else {
                return Ok(None);
            };
            let [
                DimensionExpr::Constant(rows),
                DimensionExpr::Constant(columns),
            ] = dimensions.as_ref()
            else {
                unreachable!("closed EKF matrix dimensions are constant")
            };
            let rows = *rows as usize;
            let columns = *columns as usize;
            if columns == 1 && rows > 1 {
                Ok(Some(FrozenEkfValueShape::Vector(rows)))
            } else {
                Ok(Some(FrozenEkfValueShape::Matrix { rows, columns }))
            }
        }
        _ => Ok(None),
    }
}

fn validate_source_arguments(
    operation: FrozenEkfOperation,
    arguments: &[ValueCell],
) -> MResult<()> {
    let spec = operation_spec(operation);
    if arguments.len() != spec.inputs.len() {
        return Err(operation_error(
            operation,
            FrozenEkfOperationFailure::Arity {
                expected: spec.inputs.len(),
                found: arguments.len(),
            },
        ));
    }
    for (index, (argument, expected)) in arguments.iter().zip(spec.inputs).enumerate() {
        if value_shape(argument)? != Some(*expected) {
            return Err(operation_error(
                operation,
                FrozenEkfOperationFailure::Shape {
                    argument: index,
                    expected: *expected,
                },
            ));
        }
    }
    Ok(())
}

fn validate_operation_cells(
    operation: FrozenEkfOperation,
    output: &ValueCell,
    inputs: &[ValueCell],
) -> MResult<()> {
    validate_source_arguments(operation, inputs)?;
    if value_shape(output)? != Some(operation_spec(operation).output) {
        return Err(operation_error(
            operation,
            FrozenEkfOperationFailure::OutputShape,
        ));
    }
    Ok(())
}

fn allocate_output(shape: FrozenEkfValueShape) -> MResult<ValueCell> {
    match shape {
        FrozenEkfValueShape::Bool => ValueCell::from_exact(false),
        FrozenEkfValueShape::Vector(length) => {
            #[cfg(feature = "vector2")]
            if length == 2 {
                return ValueCell::from_exact_matrix_ref(
                    Ref::new(nalgebra::Vector2::<f64>::zeros()),
                    2,
                    1,
                );
            }
            #[cfg(feature = "vector3")]
            if length == 3 {
                return ValueCell::from_exact_matrix_ref(
                    Ref::new(nalgebra::Vector3::<f64>::zeros()),
                    3,
                    1,
                );
            }
            #[cfg(feature = "vector4")]
            if length == 4 {
                return ValueCell::from_exact_matrix_ref(
                    Ref::new(nalgebra::Vector4::<f64>::zeros()),
                    4,
                    1,
                );
            }
            ValueCell::from_exact_matrix_ref(Ref::new(DVector::<f64>::zeros(length)), length, 1)
        }
        FrozenEkfValueShape::Matrix { rows, columns } => {
            #[cfg(feature = "matrix2")]
            if (rows, columns) == (2, 2) {
                return ValueCell::from_exact_matrix_ref(
                    Ref::new(nalgebra::Matrix2::<f64>::zeros()),
                    rows,
                    columns,
                );
            }
            #[cfg(feature = "matrix3")]
            if (rows, columns) == (3, 3) {
                return ValueCell::from_exact_matrix_ref(
                    Ref::new(nalgebra::Matrix3::<f64>::zeros()),
                    rows,
                    columns,
                );
            }
            #[cfg(feature = "matrix2x3")]
            if (rows, columns) == (2, 3) {
                return ValueCell::from_exact_matrix_ref(
                    Ref::new(nalgebra::Matrix2x3::<f64>::zeros()),
                    rows,
                    columns,
                );
            }
            #[cfg(feature = "matrix3x2")]
            if (rows, columns) == (3, 2) {
                return ValueCell::from_exact_matrix_ref(
                    Ref::new(nalgebra::Matrix3x2::<f64>::zeros()),
                    rows,
                    columns,
                );
            }
            ValueCell::from_exact_matrix_ref(
                Ref::new(DMatrix::<f64>::zeros(rows, columns)),
                rows,
                columns,
            )
        }
        FrozenEkfValueShape::F64 => ValueCell::from_exact(0.0_f64),
    }
}

fn shape_dimensions(shape: FrozenEkfValueShape) -> Option<(usize, usize)> {
    match shape {
        FrozenEkfValueShape::Vector(rows) => Some((rows, 1)),
        FrozenEkfValueShape::Matrix { rows, columns } => Some((rows, columns)),
        FrozenEkfValueShape::F64 | FrozenEkfValueShape::Bool => None,
    }
}

fn read_array<const N: usize>(
    operation: FrozenEkfOperation,
    argument: usize,
    value: &ValueCell,
) -> MResult<[f64; N]> {
    let expected = operation_spec(operation).inputs[argument];
    if value_shape(value)? != Some(expected) {
        return Err(operation_error(
            operation,
            FrozenEkfOperationFailure::Shape { argument, expected },
        ));
    }
    let Some((rows, columns)) = shape_dimensions(expected) else {
        unreachable!("array EKF input has matrix dimensions")
    };
    let elements = value.matrix_elements()?.ok_or_else(|| {
        operation_error(
            operation,
            FrozenEkfOperationFailure::Shape { argument, expected },
        )
    })?;
    if elements.len() != N {
        return Err(operation_error(
            operation,
            FrozenEkfOperationFailure::Shape { argument, expected },
        ));
    }
    let mut row_major = Vec::with_capacity(N);
    for element in elements {
        let snapshot = element.snapshot()?;
        let ValueData::F64(value) = snapshot.data() else {
            return Err(operation_error(
                operation,
                FrozenEkfOperationFailure::Shape { argument, expected },
            ));
        };
        row_major.push(value.to_f64());
    }
    let mut result = [0.0; N];
    for column in 0..columns {
        for row in 0..rows {
            result[column * rows + row] = row_major[row * columns + column];
        }
    }
    Ok(result)
}

fn read_scalar(operation: FrozenEkfOperation, argument: usize, value: &ValueCell) -> MResult<f64> {
    match value.snapshot()?.data() {
        ValueData::F64(value) => Ok(value.to_f64()),
        _ => Err(operation_error(
            operation,
            FrozenEkfOperationFailure::Shape {
                argument,
                expected: FrozenEkfValueShape::F64,
            },
        )),
    }
}

fn write_array<const N: usize>(
    operation: FrozenEkfOperation,
    output: &ValueCell,
    value: [f64; N],
) -> MResult<()> {
    let expected = operation_spec(operation).output;
    let Some((rows, columns)) = shape_dimensions(expected) else {
        return Err(operation_error(
            operation,
            FrozenEkfOperationFailure::OutputShape,
        ));
    };
    if rows.saturating_mul(columns) != N || value_shape(output)? != Some(expected) {
        return Err(operation_error(
            operation,
            FrozenEkfOperationFailure::OutputShape,
        ));
    }
    let mut row_major = Vec::with_capacity(N);
    for row in 0..rows {
        for column in 0..columns {
            row_major.push(ValueDataDraft::F64(F64Bits::from_f64(
                value[column * rows + row],
            )));
        }
    }
    let next = output.rebuild_matrix_drafts(
        vec![rows as u64, columns as u64].into_boxed_slice(),
        row_major.into_boxed_slice(),
    )?;
    output.replace(&next)
}

fn write_bool(operation: FrozenEkfOperation, output: &ValueCell, value: bool) -> MResult<()> {
    if value_shape(output)? != Some(FrozenEkfValueShape::Bool) {
        return Err(operation_error(
            operation,
            FrozenEkfOperationFailure::OutputShape,
        ));
    }
    let next = output.rebuild_data_draft(ValueDataDraft::Bool(value))?;
    output.replace(&next)
}

fn evaluate_into(
    operation: FrozenEkfOperation,
    inputs: &[ValueCell],
    output: &ValueCell,
) -> MResult<()> {
    match operation {
        Kernel(TrigonometricState) => write_array(
            operation,
            output,
            math::trigonometric_state(&read_array(operation, 0, &inputs[0])?),
        ),
        Kernel(MotionJacobian) => write_array(
            operation,
            output,
            math::motion_jacobian(
                &read_array(operation, 1, &inputs[1])?,
                &read_array(operation, 2, &inputs[2])?,
                read_scalar(operation, 3, &inputs[3])?,
            ),
        ),
        Kernel(ControlJacobian) => write_array(
            operation,
            output,
            math::control_jacobian(
                &read_array(operation, 0, &inputs[0])?,
                read_scalar(operation, 1, &inputs[1])?,
            ),
        ),
        Kernel(PredictedState) => write_array(
            operation,
            output,
            math::predicted_state(
                &read_array(operation, 0, &inputs[0])?,
                &read_array(operation, 1, &inputs[1])?,
                &read_array(operation, 2, &inputs[2])?,
                read_scalar(operation, 3, &inputs[3])?,
            ),
        ),
        Kernel(PredictedCovariance) => write_array(
            operation,
            output,
            math::predicted_covariance(
                &read_array(operation, 0, &inputs[0])?,
                &read_array(operation, 1, &inputs[1])?,
                &read_array(operation, 2, &inputs[2])?,
                &read_array(operation, 3, &inputs[3])?,
            ),
        ),
        Kernel(LandmarkDeltaAndRange) => write_array(
            operation,
            output,
            math::landmark_delta_and_range(
                &read_array(operation, 0, &inputs[0])?,
                &read_array(operation, 1, &inputs[1])?,
            )
            .map_err(|error| operation_error(operation, FrozenEkfOperationFailure::Math(error)))?,
        ),
        Kernel(PredictedMeasurement) => write_array(
            operation,
            output,
            math::predicted_measurement(
                &read_array(operation, 0, &inputs[0])?,
                &read_array(operation, 1, &inputs[1])?,
            ),
        ),
        Kernel(MeasurementJacobian) => write_array(
            operation,
            output,
            math::measurement_jacobian(&read_array(operation, 0, &inputs[0])?),
        ),
        Kernel(InnovationCovariance) => write_array(
            operation,
            output,
            math::innovation_covariance(
                &read_array(operation, 0, &inputs[0])?,
                &read_array(operation, 1, &inputs[1])?,
                &read_array(operation, 2, &inputs[2])?,
            ),
        ),
        Kernel(Solve2x2) => write_array(
            operation,
            output,
            math::solve_2x2(&read_array(operation, 0, &inputs[0])?).map_err(|error| {
                operation_error(operation, FrozenEkfOperationFailure::Math(error))
            })?,
        ),
        Kernel(KalmanGain) => write_array(
            operation,
            output,
            math::kalman_gain(
                &read_array(operation, 0, &inputs[0])?,
                &read_array(operation, 1, &inputs[1])?,
                &read_array(operation, 2, &inputs[2])?,
            ),
        ),
        Kernel(Innovation) => write_array(
            operation,
            output,
            math::innovation(
                &read_array(operation, 0, &inputs[0])?,
                &read_array(operation, 1, &inputs[1])?,
            ),
        ),
        Kernel(CorrectedState) => write_array(
            operation,
            output,
            math::corrected_state(
                &read_array(operation, 0, &inputs[0])?,
                &read_array(operation, 1, &inputs[1])?,
                &read_array(operation, 2, &inputs[2])?,
            ),
        ),
        Kernel(JosephCovarianceUpdate) => write_array(
            operation,
            output,
            math::joseph_covariance_update(
                &read_array(operation, 0, &inputs[0])?,
                &read_array(operation, 1, &inputs[1])?,
                &read_array(operation, 2, &inputs[2])?,
                &read_array(operation, 3, &inputs[3])?,
            ),
        ),
        Kernel(CovarianceSymmetrization) => write_array(
            operation,
            output,
            math::covariance_symmetrization(&read_array(operation, 0, &inputs[0])?),
        ),
        Predicate(CandidateFinite) => write_bool(
            operation,
            output,
            math::candidate_finite(
                &read_array(operation, 0, &inputs[0])?,
                &read_array(operation, 1, &inputs[1])?,
            ),
        ),
        Predicate(CovariancePositiveDiagonal) => write_bool(
            operation,
            output,
            math::covariance_positive_diagonal(&read_array(operation, 0, &inputs[0])?),
        ),
        Predicate(CovarianceSymmetric) => write_bool(
            operation,
            output,
            math::covariance_symmetric(&read_array(operation, 0, &inputs[0])?),
        ),
    }
}
