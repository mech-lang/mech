//! Hidden ordinary-source compiler catalog for the frozen EKF efficacy path.

use std::sync::{Arc, LazyLock};

use mech_core::{
    AccessMode, AliasPolicy, BuiltinScalarKind, BytecodeCompilerContext,
    CanonicalFunctionSpecializer, ChangeDetectionPolicy, DeliveryMode, DimensionExpr,
    ExternalInteraction, FunctionCatalog, FunctionCatalogBuilder, FunctionExport, FunctionExposure,
    FunctionInvocation, FunctionRuntimeType, FunctionStatePort, FunctionTypeDeclaration,
    GuardFunctionSafety, InputKindScheme, InputPortLayout, InputPortPolicy, KindExpr, KindScheme,
    MResult, MechError, MechErrorKind, MechFunction, MechFunctionCompiler, MechFunctionFactory,
    MechFunctionImpl, OperationContractDeclaration, OutputConstruction, OutputPortPolicy,
    ReactiveNodeKind, Register, RuntimeFunctionContract, RuntimeFunctionSignature,
    RuntimeOutputAliasPolicy, SchemaBody, ShapeRule, SpecializationContext,
    SpecializationInvocation, SpecializedFunction, ValueCell, ValueData,
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
static NEGATE: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| semantic_declaration(1, ChangeDetectionPolicy::ExactScalar));

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
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        let inputs = invocation
            .inputs()
            .iter()
            .map(|input| input.cell().cloned())
            .collect::<MResult<Vec<_>>>()?;
        validate_source_arguments(self.operation, &inputs)?;
        let semantic_inputs = invocation.inputs().iter().collect::<Vec<_>>();
        let output_shape = operation_spec(self.operation).output;
        let output_extents = match output_shape {
            FrozenEkfValueShape::F64 | FrozenEkfValueShape::Bool => Vec::new(),
            FrozenEkfValueShape::Vector(length) => vec![length as u64, 1],
            FrozenEkfValueShape::Matrix { rows, columns } => {
                vec![rows as u64, columns as u64]
            }
        };
        let descriptor = context.resolved_output_descriptor(
            0,
            output_extents.into_boxed_slice(),
            &semantic_inputs,
        )?;
        let output = allocate_output_for_descriptor(output_shape, &descriptor)?;
        let invocation = invocation_from_cells(output, inputs.into_boxed_slice());
        let function = instantiate(self.operation, invocation.clone())?;
        context.certify_instance(
            (function, invocation),
            mech_core::RuntimeFunctionId::from_name(operation_spec(self.operation).canonical_name),
            mech_core::ExecutionTarget::DirectRuntime,
            mech_core::ImplementationMemoryClass::NoAdditionalScratch,
        )
    }

    fn guard_safety(&self) -> GuardFunctionSafety {
        GuardFunctionSafety::PureStatic
    }
}

#[derive(Debug)]
pub(crate) struct FrozenEkfFunction {
    operation: FrozenEkfOperation,
    inputs: Box<[mech_core::ManagedPort<f64>]>,
    output: FrozenEkfOutputPort,
    invocation: FunctionInvocation,
}

#[derive(Debug)]
enum FrozenEkfOutputPort {
    F64(mech_core::ManagedPort<f64>),
    Bool(mech_core::ManagedPort<bool>),
}

impl FrozenEkfOutputPort {
    fn cell(&self) -> &ValueCell {
        match self {
            Self::F64(port) => port.cell(),
            Self::Bool(port) => port.cell(),
        }
    }
}

impl MechFunctionImpl for FrozenEkfFunction {
    fn solve_managed(
        &self,
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        _services: &mut dyn mech_core::MechExecutionServices,
    ) -> MResult<mech_core::ReactiveSolveStatus> {
        match (&self.output, self.inputs.as_ref()) {
            (FrozenEkfOutputPort::F64(output), [first]) => {
                frame.with_unary_port_views(first, output, |first, output| {
                    evaluate_managed_unary(self.operation, first, output)
                })?
            }
            (FrozenEkfOutputPort::Bool(output), [first]) => {
                frame.with_unary_typed_port_views(first, output, |first, output| {
                    evaluate_managed_unary_bool(self.operation, first, output)
                })?
            }
            (FrozenEkfOutputPort::F64(output), [first, second]) => {
                frame.with_binary_port_views(first, second, output, |first, second, output| {
                    evaluate_managed_binary(self.operation, first, second, output)
                })?
            }
            (FrozenEkfOutputPort::Bool(output), [first, second]) => frame
                .with_binary_typed_port_views(first, second, output, |first, second, output| {
                    evaluate_managed_binary_bool(self.operation, first, second, output)
                })?,
            (FrozenEkfOutputPort::F64(output), [first, second, third]) => frame
                .with_ternary_typed_port_views(
                    first,
                    second,
                    third,
                    output,
                    |first, second, third, output| {
                        evaluate_managed_ternary(self.operation, first, second, third, output)
                    },
                )?,
            (FrozenEkfOutputPort::F64(output), [first, second, third, fourth]) => frame
                .with_quaternary_typed_port_views(
                    first,
                    second,
                    third,
                    fourth,
                    output,
                    |first, second, third, fourth, output| {
                        evaluate_managed_quaternary(
                            self.operation,
                            first,
                            second,
                            third,
                            fourth,
                            output,
                        )
                    },
                )?,
            _ => {
                return Err(operation_error(
                    self.operation,
                    FrozenEkfOperationFailure::Arity {
                        expected: operation_spec(self.operation).inputs.len(),
                        found: self.inputs.len(),
                    },
                ));
            }
        }
        Ok(mech_core::ReactiveSolveStatus::Changed)
    }

    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_cell(self.output.cell()))
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
        std::iter::once(self.invocation.output_cell().clone())
            .chain(self.invocation.input_cells().iter().cloned())
            .collect()
    }

    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let output = self.invocation.output_cell();
        let output_descriptor = output.resolved_descriptor()?;
        let zero_cell = allocate_output_for_descriptor(
            operation_spec(self.operation).output,
            &output_descriptor,
        )?;
        let zero = zero_cell.snapshot()?;
        let destination =
            compile_runtime_produced_value_cell_register_with_seed(output, &zero, context)?;
        let inputs = self
            .invocation
            .input_cells()
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
    let inputs = invocation
        .inputs()
        .map(|input| input.try_managed_element::<f64>())
        .collect::<MResult<Vec<_>>>()?
        .into_boxed_slice();
    let output = match operation_spec(operation).output {
        FrozenEkfValueShape::Bool => {
            FrozenEkfOutputPort::Bool(invocation.output().try_managed_element::<bool>()?)
        }
        FrozenEkfValueShape::F64
        | FrozenEkfValueShape::Vector(_)
        | FrozenEkfValueShape::Matrix { .. } => {
            FrozenEkfOutputPort::F64(invocation.output().try_managed_element::<f64>()?)
        }
    };
    Ok(Box::new(FrozenEkfFunction {
        operation,
        inputs,
        output,
        invocation,
    }))
}

macro_rules! factory {
    ($factory:ident, $validator:ident, $operation:expr, unary, $output:ty, [$a:ty]) => {
        struct $factory;
        impl MechFunctionFactory for $factory {
            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
            }

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
            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
            }

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
            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
            }

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
            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
            }

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
            mech_core::OperationId::from_name(spec.canonical_name),
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
        let scheme = KindScheme::new(
            Box::new([]),
            Box::new([]),
            InputKindScheme::Fixed(
                spec.inputs
                    .iter()
                    .copied()
                    .map(frozen_ekf_kind)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
            vec![frozen_ekf_kind(spec.output)].into_boxed_slice(),
            Box::new([]),
        )?;
        let operation = builder.insert_canonical_specializer(
            spec.canonical_name,
            FunctionTypeDeclaration::from_schemes(vec![scheme]),
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
    let negate = builder.insert_canonical_specializer_with_contract(
        "math/neg",
        mech_core::maintained_source_type_declaration("math/neg")?,
        NEGATE.clone(),
        Arc::new(FrozenF64NegateSpecializer),
    )?;
    builder.insert_export(FunctionExport {
        operation: negate,
        canonical_name: "math/neg".to_owned(),
        module: None,
        item: None,
        exposure: FunctionExposure::Prelude,
    })?;
    Ok(())
}

fn frozen_ekf_kind(shape: FrozenEkfValueShape) -> KindExpr {
    let f64_kind = BuiltinScalarKind::F64.kind_expr();
    match shape {
        FrozenEkfValueShape::F64 => f64_kind,
        FrozenEkfValueShape::Bool => BuiltinScalarKind::Bool.kind_expr(),
        FrozenEkfValueShape::Vector(rows) => KindExpr::Matrix {
            element: Box::new(f64_kind),
            dimensions: vec![
                DimensionExpr::Constant(rows as u64),
                DimensionExpr::Constant(1),
            ]
            .into_boxed_slice(),
        },
        FrozenEkfValueShape::Matrix { rows, columns } => KindExpr::Matrix {
            element: Box::new(f64_kind),
            dimensions: vec![
                DimensionExpr::Constant(rows as u64),
                DimensionExpr::Constant(columns as u64),
            ]
            .into_boxed_slice(),
        },
    }
}

struct FrozenF64NegateSpecializer;

impl CanonicalFunctionSpecializer for FrozenF64NegateSpecializer {
    fn specialize_invocation(
        &self,
        arguments: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
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
        context.certify_instance(
            (
                Box::new(FrozenF64NegateFunction {
                    output: output.clone(),
                }),
                FunctionInvocation::unary(output, input),
            ),
            mech_core::RuntimeFunctionId::from_name("FrozenF64Negate"),
            mech_core::ExecutionTarget::DirectRuntime,
            mech_core::ImplementationMemoryClass::NoAdditionalScratch,
        )
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
    fn solve_managed(
        &self,
        _frame: &mut mech_core::KernelMemoryFrame<'_>,
        _services: &mut dyn mech_core::MechExecutionServices,
    ) -> MResult<mech_core::ReactiveSolveStatus> {
        // The specializer has already produced the immutable folded value.
        // Reporting a change would require a newly initialized transaction
        // stage and would incorrectly replace that value during registration.
        Ok(mech_core::ReactiveSolveStatus::Unchanged)
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
                return ValueCell::from_exact(nalgebra::Vector2::<f64>::zeros());
            }
            #[cfg(feature = "vector3")]
            if length == 3 {
                return ValueCell::from_exact(nalgebra::Vector3::<f64>::zeros());
            }
            #[cfg(feature = "vector4")]
            if length == 4 {
                return ValueCell::from_exact(nalgebra::Vector4::<f64>::zeros());
            }
            ValueCell::from_exact(DVector::<f64>::zeros(length))
        }
        FrozenEkfValueShape::Matrix { rows, columns } => {
            #[cfg(feature = "matrix2")]
            if (rows, columns) == (2, 2) {
                return ValueCell::from_exact(nalgebra::Matrix2::<f64>::zeros());
            }
            #[cfg(feature = "matrix3")]
            if (rows, columns) == (3, 3) {
                return ValueCell::from_exact(nalgebra::Matrix3::<f64>::zeros());
            }
            #[cfg(feature = "matrix2x3")]
            if (rows, columns) == (2, 3) {
                return ValueCell::from_exact(nalgebra::Matrix2x3::<f64>::zeros());
            }
            #[cfg(feature = "matrix3x2")]
            if (rows, columns) == (3, 2) {
                return ValueCell::from_exact(nalgebra::Matrix3x2::<f64>::zeros());
            }
            ValueCell::from_exact(DMatrix::<f64>::zeros(rows, columns))
        }
        FrozenEkfValueShape::F64 => ValueCell::from_exact(0.0_f64),
    }
}

fn allocate_output_for_descriptor(
    shape: FrozenEkfValueShape,
    descriptor: &mech_core::ResolvedValueDescriptor,
) -> MResult<ValueCell> {
    let backing = allocate_output(shape)?;
    ValueCell::allocate_for_descriptor(descriptor, backing.representation())
}

fn shape_dimensions(shape: FrozenEkfValueShape) -> Option<(usize, usize)> {
    match shape {
        FrozenEkfValueShape::Vector(rows) => Some((rows, 1)),
        FrozenEkfValueShape::Matrix { rows, columns } => Some((rows, columns)),
        FrozenEkfValueShape::F64 | FrozenEkfValueShape::Bool => None,
    }
}

fn managed_array<const N: usize>(
    operation: FrozenEkfOperation,
    argument: usize,
    value: &mech_core::ManagedValueView<'_, f64>,
) -> MResult<[f64; N]> {
    let expected = operation_spec(operation).inputs[argument];
    let Some((rows, columns)) = shape_dimensions(expected) else {
        return Err(operation_error(
            operation,
            FrozenEkfOperationFailure::Shape { argument, expected },
        ));
    };
    if value.rows() != rows || value.columns() != columns || value.len() != N {
        return Err(operation_error(
            operation,
            FrozenEkfOperationFailure::Shape { argument, expected },
        ));
    }
    let mut result = [0.0; N];
    for (index, slot) in result.iter_mut().enumerate() {
        *slot = value.get_column_major(index).ok_or_else(|| {
            operation_error(
                operation,
                FrozenEkfOperationFailure::Shape { argument, expected },
            )
        })?;
    }
    Ok(result)
}

fn managed_scalar(
    operation: FrozenEkfOperation,
    argument: usize,
    value: &mech_core::ManagedValueView<'_, f64>,
) -> MResult<f64> {
    if operation_spec(operation).inputs[argument] != FrozenEkfValueShape::F64 || value.len() != 1 {
        return Err(operation_error(
            operation,
            FrozenEkfOperationFailure::Shape {
                argument,
                expected: FrozenEkfValueShape::F64,
            },
        ));
    }
    value.get(0, 0).ok_or_else(|| {
        operation_error(
            operation,
            FrozenEkfOperationFailure::Shape {
                argument,
                expected: FrozenEkfValueShape::F64,
            },
        )
    })
}

fn publish_managed_array<const N: usize>(
    operation: FrozenEkfOperation,
    output: &mut mech_core::ManagedValueViewMut<'_, f64>,
    value: [f64; N],
) -> MResult<()> {
    let expected = operation_spec(operation).output;
    let Some((rows, columns)) = shape_dimensions(expected) else {
        return Err(operation_error(
            operation,
            FrozenEkfOperationFailure::OutputShape,
        ));
    };
    if output.rows() != rows || output.columns() != columns || output.len() != N {
        return Err(operation_error(
            operation,
            FrozenEkfOperationFailure::OutputShape,
        ));
    }
    output.try_fill_column_major(|index| Ok(value[index]))
}

fn publish_managed_bool(
    operation: FrozenEkfOperation,
    output: &mut mech_core::ManagedValueViewMut<'_, bool>,
    value: bool,
) -> MResult<()> {
    if operation_spec(operation).output != FrozenEkfValueShape::Bool || output.len() != 1 {
        return Err(operation_error(
            operation,
            FrozenEkfOperationFailure::OutputShape,
        ));
    }
    output.try_fill_column_major(|_| Ok(value))
}

fn evaluate_managed_unary(
    operation: FrozenEkfOperation,
    input: mech_core::ManagedValueView<'_, f64>,
    output: &mut mech_core::ManagedValueViewMut<'_, f64>,
) -> MResult<()> {
    let next = match operation {
        Kernel(TrigonometricState) => {
            return publish_managed_array(
                operation,
                output,
                math::trigonometric_state(&managed_array(operation, 0, &input)?),
            );
        }
        Kernel(MeasurementJacobian) => {
            return publish_managed_array(
                operation,
                output,
                math::measurement_jacobian(&managed_array(operation, 0, &input)?),
            );
        }
        Kernel(Solve2x2) => math::solve_2x2(&managed_array(operation, 0, &input)?)
            .map_err(|error| operation_error(operation, FrozenEkfOperationFailure::Math(error)))?,
        Kernel(CovarianceSymmetrization) => {
            return publish_managed_array(
                operation,
                output,
                math::covariance_symmetrization(&managed_array(operation, 0, &input)?),
            );
        }
        _ => {
            return Err(operation_error(
                operation,
                FrozenEkfOperationFailure::Arity {
                    expected: operation_spec(operation).inputs.len(),
                    found: 1,
                },
            ));
        }
    };
    publish_managed_array(operation, output, next)
}

fn evaluate_managed_unary_bool(
    operation: FrozenEkfOperation,
    input: mech_core::ManagedValueView<'_, f64>,
    output: &mut mech_core::ManagedValueViewMut<'_, bool>,
) -> MResult<()> {
    let next = match operation {
        Predicate(CovariancePositiveDiagonal) => {
            math::covariance_positive_diagonal(&managed_array(operation, 0, &input)?)
        }
        Predicate(CovarianceSymmetric) => {
            math::covariance_symmetric(&managed_array(operation, 0, &input)?)
        }
        _ => {
            return Err(operation_error(
                operation,
                FrozenEkfOperationFailure::OutputShape,
            ));
        }
    };
    publish_managed_bool(operation, output, next)
}

fn evaluate_managed_binary(
    operation: FrozenEkfOperation,
    first: mech_core::ManagedValueView<'_, f64>,
    second: mech_core::ManagedValueView<'_, f64>,
    output: &mut mech_core::ManagedValueViewMut<'_, f64>,
) -> MResult<()> {
    match operation {
        Kernel(ControlJacobian) => publish_managed_array(
            operation,
            output,
            math::control_jacobian(
                &managed_array(operation, 0, &first)?,
                managed_scalar(operation, 1, &second)?,
            ),
        ),
        Kernel(LandmarkDeltaAndRange) => publish_managed_array(
            operation,
            output,
            math::landmark_delta_and_range(
                &managed_array(operation, 0, &first)?,
                &managed_array(operation, 1, &second)?,
            )
            .map_err(|error| operation_error(operation, FrozenEkfOperationFailure::Math(error)))?,
        ),
        Kernel(PredictedMeasurement) => publish_managed_array(
            operation,
            output,
            math::predicted_measurement(
                &managed_array(operation, 0, &first)?,
                &managed_array(operation, 1, &second)?,
            ),
        ),
        Kernel(Innovation) => publish_managed_array(
            operation,
            output,
            math::innovation(
                &managed_array(operation, 0, &first)?,
                &managed_array(operation, 1, &second)?,
            ),
        ),
        _ => Err(operation_error(
            operation,
            FrozenEkfOperationFailure::Arity {
                expected: operation_spec(operation).inputs.len(),
                found: 2,
            },
        )),
    }
}

fn evaluate_managed_binary_bool(
    operation: FrozenEkfOperation,
    first: mech_core::ManagedValueView<'_, f64>,
    second: mech_core::ManagedValueView<'_, f64>,
    output: &mut mech_core::ManagedValueViewMut<'_, bool>,
) -> MResult<()> {
    let Predicate(CandidateFinite) = operation else {
        return Err(operation_error(
            operation,
            FrozenEkfOperationFailure::OutputShape,
        ));
    };
    publish_managed_bool(
        operation,
        output,
        math::candidate_finite(
            &managed_array(operation, 0, &first)?,
            &managed_array(operation, 1, &second)?,
        ),
    )
}

fn evaluate_managed_ternary(
    operation: FrozenEkfOperation,
    first: mech_core::ManagedValueView<'_, f64>,
    second: mech_core::ManagedValueView<'_, f64>,
    third: mech_core::ManagedValueView<'_, f64>,
    output: &mut mech_core::ManagedValueViewMut<'_, f64>,
) -> MResult<()> {
    match operation {
        Kernel(InnovationCovariance) => publish_managed_array(
            operation,
            output,
            math::innovation_covariance(
                &managed_array(operation, 0, &first)?,
                &managed_array(operation, 1, &second)?,
                &managed_array(operation, 2, &third)?,
            ),
        ),
        Kernel(KalmanGain) => publish_managed_array(
            operation,
            output,
            math::kalman_gain(
                &managed_array(operation, 0, &first)?,
                &managed_array(operation, 1, &second)?,
                &managed_array(operation, 2, &third)?,
            ),
        ),
        Kernel(CorrectedState) => publish_managed_array(
            operation,
            output,
            math::corrected_state(
                &managed_array(operation, 0, &first)?,
                &managed_array(operation, 1, &second)?,
                &managed_array(operation, 2, &third)?,
            ),
        ),
        _ => Err(operation_error(
            operation,
            FrozenEkfOperationFailure::Arity {
                expected: operation_spec(operation).inputs.len(),
                found: 3,
            },
        )),
    }
}

fn evaluate_managed_quaternary(
    operation: FrozenEkfOperation,
    first: mech_core::ManagedValueView<'_, f64>,
    second: mech_core::ManagedValueView<'_, f64>,
    third: mech_core::ManagedValueView<'_, f64>,
    fourth: mech_core::ManagedValueView<'_, f64>,
    output: &mut mech_core::ManagedValueViewMut<'_, f64>,
) -> MResult<()> {
    match operation {
        Kernel(MotionJacobian) => publish_managed_array(
            operation,
            output,
            math::motion_jacobian(
                &managed_array(operation, 1, &second)?,
                &managed_array(operation, 2, &third)?,
                managed_scalar(operation, 3, &fourth)?,
            ),
        ),
        Kernel(PredictedState) => publish_managed_array(
            operation,
            output,
            math::predicted_state(
                &managed_array(operation, 0, &first)?,
                &managed_array(operation, 1, &second)?,
                &managed_array(operation, 2, &third)?,
                managed_scalar(operation, 3, &fourth)?,
            ),
        ),
        Kernel(PredictedCovariance) => publish_managed_array(
            operation,
            output,
            math::predicted_covariance(
                &managed_array(operation, 0, &first)?,
                &managed_array(operation, 1, &second)?,
                &managed_array(operation, 2, &third)?,
                &managed_array(operation, 3, &fourth)?,
            ),
        ),
        Kernel(JosephCovarianceUpdate) => publish_managed_array(
            operation,
            output,
            math::joseph_covariance_update(
                &managed_array(operation, 0, &first)?,
                &managed_array(operation, 1, &second)?,
                &managed_array(operation, 2, &third)?,
                &managed_array(operation, 3, &fourth)?,
            ),
        ),
        _ => Err(operation_error(
            operation,
            FrozenEkfOperationFailure::Arity {
                expected: operation_spec(operation).inputs.len(),
                found: 4,
            },
        )),
    }
}
