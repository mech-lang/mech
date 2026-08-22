//! Hidden ordinary-source compiler catalog for the frozen EKF efficacy path.

use std::sync::{Arc, LazyLock};

use mech_core::{
    AccessMode, AliasPolicy, BytecodeCompilerContext, ChangeDetectionPolicy, CompileConst,
    DeliveryMode, ExternalInteraction, FunctionArgs, FunctionCatalog, FunctionCatalogBuilder,
    FunctionExport, FunctionExposure, FunctionRuntimeType, FunctionSpecializer,
    GuardFunctionSafety, InputPortLayout, InputPortPolicy, LegacyValue, MResult, MechError,
    MechErrorKind, MechFunction, MechFunctionCompiler, MechFunctionFactory, MechFunctionImpl,
    OperationContractDeclaration, OutputConstruction, OutputPortPolicy, ReactiveNodeKind, Ref,
    Register, RuntimeFunctionContract, RuntimeFunctionSignature, RuntimeOutputAliasPolicy,
    ShapeRule, compile_runtime_produced_register, compile_value_register,
    function_shape_contract_violation,
};
use nalgebra::{DMatrix, DVector};

use crate::Matrix;

use super::math::{self, EkfMathError};
use super::operation::{
    EkfKernel, EkfPredicate, FROZEN_EKF_OPERATIONS, FrozenEkfOperation, FrozenEkfOperationSpec,
    FrozenEkfValueShape, operation_spec,
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

impl FunctionSpecializer for FrozenEkfSpecializer {
    fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
        validate_source_arguments(self.operation, arguments)?;
        let function = FrozenEkfFunction {
            operation: self.operation,
            arguments: arguments.to_vec().into_boxed_slice(),
            compile_arguments: frozen_compile_arguments(arguments),
            output: allocate_output(operation_spec(self.operation).output),
        };
        function.solve_result()?;
        Ok(Box::new(function))
    }

    fn guard_safety(&self) -> GuardFunctionSafety {
        GuardFunctionSafety::PureStatic
    }
}

#[derive(Debug)]
pub(crate) struct FrozenEkfFunction {
    operation: FrozenEkfOperation,
    arguments: Box<[LegacyValue]>,
    compile_arguments: Box<[LegacyValue]>,
    output: LegacyValue,
}

impl MechFunctionImpl for FrozenEkfFunction {
    fn solve_result(&self) -> MResult<()> {
        evaluate_into(self.operation, &self.arguments, &self.output)
    }

    fn out(&self) -> LegacyValue {
        self.output.clone()
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(semantic_contract(self.operation))
    }

    fn reactive_node_kind(&self) -> ReactiveNodeKind {
        ReactiveNodeKind::Combinational
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
    }

    fn to_string(&self) -> String {
        operation_spec(self.operation).canonical_name.to_owned()
    }
}

impl MechFunctionCompiler for FrozenEkfFunction {
    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let destination =
            compile_runtime_produced_register(&self.output, self.output.addr(), context)?;
        let output_seed = match operation_spec(self.operation).output {
            FrozenEkfValueShape::F64 => LegacyValue::F64(Ref::new(0.0)),
            FrozenEkfValueShape::Bool => LegacyValue::Bool(Ref::new(false)),
            FrozenEkfValueShape::Vector(rows) => {
                LegacyValue::MatrixF64(Matrix::DVector(Ref::new(DVector::zeros(rows))))
            }
            FrozenEkfValueShape::Matrix { rows, columns } => {
                LegacyValue::MatrixF64(Matrix::DMatrix(Ref::new(DMatrix::zeros(rows, columns))))
            }
        };
        let output_seed = output_seed.compile_const(context)?;
        context.record_register_constant_metadata(destination, output_seed)?;
        context.emit_const_load(destination, output_seed);
        let inputs = self
            .compile_arguments
            .iter()
            .map(|argument| compile_value_register(argument, argument.addr(), context))
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

fn frozen_compile_argument(value: &LegacyValue) -> LegacyValue {
    match value {
        LegacyValue::Typed(value, _) => frozen_compile_argument(value),
        LegacyValue::MutableReference(value) => frozen_compile_argument(&value.borrow()),
        LegacyValue::F64(value) => LegacyValue::F64(Ref::new(*value.borrow())),
        _ => value.clone(),
    }
}

fn frozen_compile_arguments(arguments: &[LegacyValue]) -> Box<[LegacyValue]> {
    arguments
        .iter()
        .map(frozen_compile_argument)
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

fn instantiate(
    operation: FrozenEkfOperation,
    arguments: FunctionArgs,
) -> MResult<Box<dyn MechFunction>> {
    let input_values = arguments.input_values();
    Ok(Box::new(FrozenEkfFunction {
        operation,
        compile_arguments: frozen_compile_arguments(&input_values),
        arguments: input_values.into_boxed_slice(),
        output: arguments.output_value().clone(),
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
            fn new(arguments: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
                instantiate($operation, arguments)
            }
        }
        fn $validator(arguments: &FunctionArgs) -> MResult<()> {
            validate_runtime_shapes($operation, arguments)
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
            fn new(arguments: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
                instantiate($operation, arguments)
            }
        }
        fn $validator(arguments: &FunctionArgs) -> MResult<()> {
            validate_runtime_shapes($operation, arguments)
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
            fn new(arguments: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
                instantiate($operation, arguments)
            }
        }
        fn $validator(arguments: &FunctionArgs) -> MResult<()> {
            validate_runtime_shapes($operation, arguments)
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
            fn new(arguments: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
                instantiate($operation, arguments)
            }
        }
        fn $validator(arguments: &FunctionArgs) -> MResult<()> {
            validate_runtime_shapes($operation, arguments)
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

fn runtime_registration(
    operation: FrozenEkfOperation,
) -> (
    RuntimeFunctionSignature,
    fn(FunctionArgs) -> MResult<Box<dyn MechFunction>>,
    fn(&FunctionArgs) -> MResult<()>,
) {
    macro_rules! entry {
        ($factory:ty, $validator:ident) => {
            (<$factory>::SIGNATURE, <$factory>::new, $validator)
        };
    }
    match operation {
        Kernel(TrigonometricState) => entry!(TrigFactory, validate_trig),
        Kernel(MotionJacobian) => entry!(MotionFactory, validate_motion),
        Kernel(ControlJacobian) => entry!(ControlFactory, validate_control),
        Kernel(PredictedState) => entry!(PredictedStateFactory, validate_predicted_state),
        Kernel(PredictedCovariance) => {
            entry!(PredictedCovarianceFactory, validate_predicted_covariance)
        }
        Kernel(LandmarkDeltaAndRange) => entry!(LandmarkFactory, validate_landmark),
        Kernel(PredictedMeasurement) => entry!(MeasurementFactory, validate_measurement),
        Kernel(MeasurementJacobian) => {
            entry!(MeasurementJacobianFactory, validate_measurement_jacobian)
        }
        Kernel(InnovationCovariance) => {
            entry!(InnovationCovarianceFactory, validate_innovation_covariance)
        }
        Kernel(Solve2x2) => entry!(SolveFactory, validate_solve),
        Kernel(KalmanGain) => entry!(GainFactory, validate_gain),
        Kernel(Innovation) => entry!(InnovationFactory, validate_innovation),
        Kernel(CorrectedState) => entry!(CorrectedStateFactory, validate_corrected_state),
        Kernel(JosephCovarianceUpdate) => entry!(JosephFactory, validate_joseph),
        Kernel(CovarianceSymmetrization) => entry!(SymmetrizationFactory, validate_symmetrization),
        Predicate(CandidateFinite) => entry!(FiniteFactory, validate_finite),
        Predicate(CovariancePositiveDiagonal) => entry!(PositiveFactory, validate_positive),
        Predicate(CovarianceSymmetric) => entry!(SymmetricFactory, validate_symmetric),
    }
}

// Builder registration requires factory types at compile time, so keep the
// ordered calls derived from the single operation table and assert alignment.
macro_rules! register {
    ($builder:expr, $index:expr, $factory:ty, $validator:ident) => {{
        let spec = &FROZEN_EKF_OPERATIONS[$index];
        let (signature, _, validator) = runtime_registration(spec.operation);
        debug_assert_eq!(signature, <$factory>::SIGNATURE);
        debug_assert_eq!(validator as usize, $validator as usize);
        $builder.insert_runtime_factory_with_semantic_contract::<$factory>(
            spec.canonical_name,
            RuntimeFunctionContract::custom(
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

pub(crate) fn install_source_operations(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    for spec in FROZEN_EKF_OPERATIONS {
        let operation = builder.insert_specializer(
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
    Ok(())
}

pub(crate) fn install_source(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_source_operations(builder)?;
    let negate = builder.insert_specializer("math/neg", Arc::new(FrozenF64NegateSpecializer))?;
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

impl FunctionSpecializer for FrozenF64NegateSpecializer {
    fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
        let [argument] = arguments else {
            return Err(MechError::new(
                mech_core::IncorrectNumberOfArguments {
                    expected: 1,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        };
        let value = match argument {
            LegacyValue::F64(value) => -*value.borrow(),
            LegacyValue::Typed(value, _) => match value.as_ref() {
                LegacyValue::F64(value) => -*value.borrow(),
                _ => {
                    return Err(function_shape_contract_violation(
                        "math/neg",
                        "frozen source literal negation requires f64",
                    ));
                }
            },
            _ => {
                return Err(function_shape_contract_violation(
                    "math/neg",
                    "frozen source literal negation requires f64",
                ));
            }
        };
        Ok(Box::new(FrozenF64NegateFunction {
            output: Ref::new(value),
        }))
    }

    fn guard_safety(&self) -> GuardFunctionSafety {
        GuardFunctionSafety::PureStatic
    }
}

#[derive(Debug)]
struct FrozenF64NegateFunction {
    output: Ref<f64>,
}

impl MechFunctionImpl for FrozenF64NegateFunction {
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }

    fn out(&self) -> LegacyValue {
        LegacyValue::F64(self.output.clone())
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
    }

    fn to_string(&self) -> String {
        "math/neg".to_owned()
    }
}

impl MechFunctionCompiler for FrozenF64NegateFunction {
    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let output = LegacyValue::F64(self.output.clone());
        compile_value_register(&output, self.output.addr(), context)
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

fn value_shape(value: &LegacyValue) -> Option<FrozenEkfValueShape> {
    match value {
        LegacyValue::Typed(value, _) => value_shape(value),
        LegacyValue::MutableReference(value) => value_shape(&value.borrow()),
        LegacyValue::F64(_) => Some(FrozenEkfValueShape::F64),
        LegacyValue::Bool(_) => Some(FrozenEkfValueShape::Bool),
        LegacyValue::MatrixF64(value) => {
            let shape = value.shape();
            match shape.as_slice() {
                [rows, 1] if *rows > 1 => Some(FrozenEkfValueShape::Vector(*rows)),
                [rows, columns] => Some(FrozenEkfValueShape::Matrix {
                    rows: *rows,
                    columns: *columns,
                }),
                _ => None,
            }
        }
        _ => None,
    }
}

fn validate_source_arguments(
    operation: FrozenEkfOperation,
    arguments: &[LegacyValue],
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
        if value_shape(argument) != Some(*expected) {
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

fn validate_runtime_shapes(operation: FrozenEkfOperation, args: &FunctionArgs) -> MResult<()> {
    let spec = operation_spec(operation);
    if args.input_count() != spec.inputs.len() {
        return Err(function_shape_contract_violation(
            spec.canonical_name,
            format!(
                "expected {} inputs, found {}",
                spec.inputs.len(),
                args.input_count()
            ),
        ));
    }
    if value_shape(args.output_value()) != Some(spec.output) {
        return Err(function_shape_contract_violation(
            spec.canonical_name,
            format!("output requires {:?}", spec.output),
        ));
    }
    for (index, expected) in spec.inputs.iter().enumerate() {
        let value = args.input_value(index).expect("arity checked");
        if value_shape(value) != Some(*expected) {
            return Err(function_shape_contract_violation(
                spec.canonical_name,
                format!("input {index} requires {expected:?}"),
            ));
        }
    }
    Ok(())
}

fn allocate_output(shape: FrozenEkfValueShape) -> LegacyValue {
    match shape {
        FrozenEkfValueShape::Bool => LegacyValue::Bool(Ref::new(false)),
        FrozenEkfValueShape::Vector(length) => {
            #[cfg(feature = "vector2")]
            if length == 2 {
                return LegacyValue::MatrixF64(Matrix::Vector2(Ref::new(
                    nalgebra::Vector2::zeros(),
                )));
            }
            #[cfg(feature = "vector3")]
            if length == 3 {
                return LegacyValue::MatrixF64(Matrix::Vector3(Ref::new(
                    nalgebra::Vector3::zeros(),
                )));
            }
            #[cfg(feature = "vector4")]
            if length == 4 {
                return LegacyValue::MatrixF64(Matrix::Vector4(Ref::new(
                    nalgebra::Vector4::zeros(),
                )));
            }
            LegacyValue::MatrixF64(Matrix::DVector(Ref::new(DVector::zeros(length))))
        }
        FrozenEkfValueShape::Matrix { rows, columns } => {
            #[cfg(feature = "matrix2")]
            if (rows, columns) == (2, 2) {
                return LegacyValue::MatrixF64(Matrix::Matrix2(Ref::new(
                    nalgebra::Matrix2::zeros(),
                )));
            }
            #[cfg(feature = "matrix3")]
            if (rows, columns) == (3, 3) {
                return LegacyValue::MatrixF64(Matrix::Matrix3(Ref::new(
                    nalgebra::Matrix3::zeros(),
                )));
            }
            #[cfg(feature = "matrix2x3")]
            if (rows, columns) == (2, 3) {
                return LegacyValue::MatrixF64(Matrix::Matrix2x3(Ref::new(
                    nalgebra::Matrix2x3::zeros(),
                )));
            }
            #[cfg(feature = "matrix3x2")]
            if (rows, columns) == (3, 2) {
                return LegacyValue::MatrixF64(Matrix::Matrix3x2(Ref::new(
                    nalgebra::Matrix3x2::zeros(),
                )));
            }
            LegacyValue::MatrixF64(Matrix::DMatrix(Ref::new(DMatrix::zeros(rows, columns))))
        }
        FrozenEkfValueShape::F64 => LegacyValue::F64(Ref::new(0.0)),
    }
}

fn read_array<const N: usize>(
    operation: FrozenEkfOperation,
    argument: usize,
    value: &LegacyValue,
) -> MResult<[f64; N]> {
    let mut result = [0.0; N];
    match value {
        LegacyValue::Typed(value, _) => return read_array(operation, argument, value),
        LegacyValue::MutableReference(value) => {
            return read_array(operation, argument, &value.borrow());
        }
        LegacyValue::MatrixF64(value) => {
            let value = value.as_vec();
            if value.len() != N {
                return Err(operation_error(
                    operation,
                    FrozenEkfOperationFailure::Shape {
                        argument,
                        expected: operation_spec(operation).inputs[argument],
                    },
                ));
            }
            result.copy_from_slice(&value);
        }
        _ => {
            return Err(operation_error(
                operation,
                FrozenEkfOperationFailure::Shape {
                    argument,
                    expected: operation_spec(operation).inputs[argument],
                },
            ));
        }
    }
    Ok(result)
}

fn read_scalar(
    operation: FrozenEkfOperation,
    argument: usize,
    value: &LegacyValue,
) -> MResult<f64> {
    match value {
        LegacyValue::Typed(value, _) => read_scalar(operation, argument, value),
        LegacyValue::MutableReference(value) => read_scalar(operation, argument, &value.borrow()),
        LegacyValue::F64(value) => Ok(*value.borrow()),
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
    output: &LegacyValue,
    value: [f64; N],
) -> MResult<()> {
    macro_rules! write_fixed {
        ($output:expr) => {{
            let mut output = $output.borrow_mut();
            if output.len() != N {
                return Err(operation_error(
                    operation,
                    FrozenEkfOperationFailure::OutputShape,
                ));
            }
            output.as_mut_slice().copy_from_slice(&value);
            Ok(())
        }};
    }
    match output {
        #[cfg(feature = "vector2")]
        LegacyValue::MatrixF64(Matrix::Vector2(output)) => write_fixed!(output),
        #[cfg(feature = "vector3")]
        LegacyValue::MatrixF64(Matrix::Vector3(output)) => write_fixed!(output),
        #[cfg(feature = "vector4")]
        LegacyValue::MatrixF64(Matrix::Vector4(output)) => write_fixed!(output),
        #[cfg(feature = "matrix2")]
        LegacyValue::MatrixF64(Matrix::Matrix2(output)) => write_fixed!(output),
        #[cfg(feature = "matrix3")]
        LegacyValue::MatrixF64(Matrix::Matrix3(output)) => write_fixed!(output),
        #[cfg(feature = "matrix2x3")]
        LegacyValue::MatrixF64(Matrix::Matrix2x3(output)) => write_fixed!(output),
        #[cfg(feature = "matrix3x2")]
        LegacyValue::MatrixF64(Matrix::Matrix3x2(output)) => write_fixed!(output),
        LegacyValue::MatrixF64(Matrix::DVector(output)) if output.borrow().len() == N => {
            output.borrow_mut().as_mut_slice().copy_from_slice(&value);
            Ok(())
        }
        LegacyValue::MatrixF64(Matrix::DMatrix(output)) if output.borrow().len() == N => {
            output.borrow_mut().as_mut_slice().copy_from_slice(&value);
            Ok(())
        }
        _ => Err(operation_error(
            operation,
            FrozenEkfOperationFailure::OutputShape,
        )),
    }
}

fn write_bool(operation: FrozenEkfOperation, output: &LegacyValue, value: bool) -> MResult<()> {
    match output {
        LegacyValue::Bool(output) => {
            *output.borrow_mut() = value;
            Ok(())
        }
        _ => Err(operation_error(
            operation,
            FrozenEkfOperationFailure::OutputShape,
        )),
    }
}

fn evaluate_into(
    operation: FrozenEkfOperation,
    inputs: &[LegacyValue],
    output: &LegacyValue,
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
