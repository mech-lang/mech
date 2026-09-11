use mech_core::{
    AccessMode, AliasPolicy, ChangeDetectionPolicy, DeliveryMode, ExternalInteraction,
    FunctionCatalog, FunctionCatalogBuilder, InputPortLayout, InputPortPolicy, MResult,
    OperationContractDeclaration, OutputConstruction, OutputPortPolicy, ShapeRule,
};
use std::sync::Arc;

pub(crate) fn pure_test_operation_contract(input_count: usize) -> OperationContractDeclaration {
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

/// Installs the concrete factories owned by the engine's intrinsic fragment.
///
/// Standard distribution composition lives outside the engine; this narrow
/// installer exists so composition crates can include engine-owned operations.
pub fn install_intrinsic_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    crate::intrinsics::catalog::install_runtime(builder)
}

/// Installs the source specializers owned by the engine's intrinsic fragment.
#[cfg(feature = "source")]
pub fn install_intrinsic_source(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    crate::intrinsics::catalog::install_source(builder)
}

/// Returns a new empty catalog for a bare engine instance.
pub fn empty_function_catalog() -> Arc<FunctionCatalog> {
    Arc::new(FunctionCatalog::empty())
}

/// Builds the explicitly injected catalog used by source-behavior unit tests.
///
/// This deliberately contains no standard-machine implementations. The small
/// arithmetic and comparison specializers below are test doubles: they prove
/// engine behavior against caller-supplied operations without restoring a
/// distribution dependency or fallback.
pub(crate) fn function_catalog() -> Arc<FunctionCatalog> {
    let mut builder = FunctionCatalogBuilder::new();
    install_intrinsic_runtime(&mut builder)
        .expect("engine intrinsic runtime catalog must be valid");
    #[cfg(feature = "semantic-compiler")]
    crate::install_intrinsic_compiler_runtime(&mut builder)
        .expect("engine compiler runtime catalog must be valid");
    #[cfg(feature = "source")]
    install_intrinsic_source(&mut builder).expect("engine intrinsic source catalog must be valid");
    test_operations::install(&mut builder).expect("engine test catalog must be valid");
    Arc::new(
        builder
            .build()
            .expect("engine intrinsic catalog must be valid"),
    )
}

#[cfg(test)]
mod test_operations {
    #[cfg(all(feature = "f64", feature = "matrix", feature = "range_inclusive"))]
    use mech_core::snapshot::F64Bits;
    use mech_core::*;
    use std::sync::Arc;

    fn canonical_input(
        invocation: &SpecializationInvocation,
        index: usize,
        message: &str,
    ) -> MResult<ValueCell> {
        invocation
            .input(index)
            .ok_or_else(|| test_operation_error(message))?
            .cell()
            .cloned()
    }

    fn specialized_function(
        context: &SpecializationContext<'_>,
        implementation: Box<dyn MechFunction>,
        output: ValueCell,
        inputs: Vec<ValueCell>,
    ) -> SpecializedFunction {
        let invocation = match inputs.as_slice() {
            [] => FunctionInvocation::nullary(output),
            [input] => FunctionInvocation::unary(output, input.clone()),
            [first, second] => FunctionInvocation::binary(output, first.clone(), second.clone()),
            _ => FunctionInvocation::variadic(output, inputs.into_boxed_slice()),
        };
        context
            .certify_instance(
                (implementation, invocation),
                RuntimeFunctionId::from_name("TestSpecialized"),
                ExecutionTarget::DirectRuntime,
                mech_core::ImplementationMemoryClass::NoAdditionalScratch,
            )
            .expect("test implementation must retain its semantic operation descriptor")
    }

    fn pure_contract(input_count: usize) -> OperationContractDeclaration {
        super::pure_test_operation_contract(input_count)
    }

    fn state_rmw_contract() -> OperationContractDeclaration {
        OperationContractDeclaration {
            inputs: InputPortLayout::Fixed(
                vec![
                    InputPortPolicy {
                        access: AccessMode::Read,
                        delivery: DeliveryMode::Signal,
                    };
                    2
                ]
                .into_boxed_slice(),
            ),
            outputs: vec![OutputPortPolicy {
                access: AccessMode::ReadWrite,
                delivery: DeliveryMode::Signal,
                construction: OutputConstruction::ReadModifyWrite {
                    base_input: 0,
                    regions: RegionPolicy::WholeValue,
                },
                alias: AliasPolicy::MayAlias { input: 0 },
                change_detection: ChangeDetectionPolicy::KernelReported,
            }]
            .into_boxed_slice(),
            interaction: ExternalInteraction::Pure,
        }
    }

    #[derive(Clone, Copy, Debug)]
    enum BinaryArithmetic {
        Add,
        Subtract,
        Multiply,
        Divide,
    }

    #[derive(Debug)]
    struct BinaryArithmeticFunction {
        operation: BinaryArithmetic,
        lhs: ManagedPort<f64>,
        rhs: ManagedPort<f64>,
        out: ManagedPort<f64>,
    }

    impl MechFunctionImpl for BinaryArithmeticFunction {
        fn solve_managed(
            &self,
            frame: &mut mech_core::KernelMemoryFrame<'_>,
            _services: &mut dyn mech_core::MechExecutionServices,
        ) -> MResult<mech_core::ReactiveSolveStatus> {
            frame.with_binary_port_views(&self.lhs, &self.rhs, &self.out, |lhs, rhs, out| {
                out.try_fill_column_major(|index| {
                    let lhs = lhs.get_column_major(index).ok_or_else(|| {
                        test_operation_error("binary arithmetic lhs geometry mismatch")
                    })?;
                    let rhs = rhs.get_column_major(index).ok_or_else(|| {
                        test_operation_error("binary arithmetic rhs geometry mismatch")
                    })?;
                    let result = match self.operation {
                        BinaryArithmetic::Add => lhs + rhs,
                        BinaryArithmetic::Subtract => lhs - rhs,
                        BinaryArithmetic::Multiply => lhs * rhs,
                        BinaryArithmetic::Divide => lhs / rhs,
                    };
                    Ok(result)
                })
            })?;
            Ok(mech_core::ReactiveSolveStatus::Changed)
        }

        fn primary_output_state_port(&self) -> Option<mech_core::FunctionStatePort<'_>> {
            Some(mech_core::FunctionStatePort::from_cell(self.out.cell()))
        }

        fn to_string(&self) -> String {
            format!("Test{:?}", self.operation)
        }
    }

    #[cfg(feature = "semantic-compiler")]
    impl MechFunctionCompiler for BinaryArithmeticFunction {
        fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }

    struct BinaryArithmeticSpecializer(BinaryArithmetic);

    impl CanonicalFunctionSpecializer for BinaryArithmeticSpecializer {
        fn specialize_invocation(
            &self,
            invocation: &SpecializationInvocation,
            context: &mut SpecializationContext<'_>,
        ) -> MResult<SpecializedFunction> {
            if invocation.len() != 2 {
                return Err(test_operation_error(
                    "binary arithmetic expects two arguments",
                ));
            }
            let lhs = canonical_input(invocation, 0, "binary arithmetic expects a lhs")?;
            let rhs = canonical_input(invocation, 1, "binary arithmetic expects a rhs")?;
            let output = ValueCell::from_exact(0.0_f64)?;
            let ports = FunctionInvocation::binary(output.clone(), lhs.clone(), rhs.clone());
            let function = BinaryArithmeticFunction {
                operation: self.0,
                lhs: ports.input(0).expect("lhs port").try_managed::<f64>()?,
                rhs: ports.input(1).expect("rhs port").try_managed::<f64>()?,
                out: ports.output().try_managed::<f64>()?,
            };
            Ok(specialized_function(
                context,
                Box::new(function),
                output,
                vec![lhs, rhs],
            ))
        }
    }

    #[derive(Debug)]
    struct NegateFunction {
        input: ManagedPort<f64>,
        out: ManagedPort<f64>,
    }

    impl MechFunctionImpl for NegateFunction {
        fn solve_managed(
            &self,
            frame: &mut mech_core::KernelMemoryFrame<'_>,
            _services: &mut dyn mech_core::MechExecutionServices,
        ) -> MResult<mech_core::ReactiveSolveStatus> {
            frame.with_unary_port_views(&self.input, &self.out, |input, output| {
                output.try_fill_column_major(|index| {
                    input
                        .get_column_major(index)
                        .map(|value| -value)
                        .ok_or_else(|| test_operation_error("negation input geometry mismatch"))
                })
            })?;
            Ok(mech_core::ReactiveSolveStatus::Changed)
        }

        fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
            Some(FunctionStatePort::from_cell(self.out.cell()))
        }

        fn to_string(&self) -> String {
            "TestNegate".to_string()
        }
    }

    #[cfg(feature = "semantic-compiler")]
    impl MechFunctionCompiler for NegateFunction {
        fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }

    struct NegateSpecializer;

    impl CanonicalFunctionSpecializer for NegateSpecializer {
        fn specialize_invocation(
            &self,
            invocation: &SpecializationInvocation,
            context: &mut SpecializationContext<'_>,
        ) -> MResult<SpecializedFunction> {
            if invocation.len() != 1 {
                return Err(test_operation_error("negation expects one argument"));
            }
            let input = canonical_input(invocation, 0, "negation expects an input")?;
            let output = ValueCell::from_exact(0.0_f64)?;
            let ports = FunctionInvocation::unary(output.clone(), input.clone());
            let function = NegateFunction {
                input: ports.input(0).unwrap().try_managed::<f64>()?,
                out: ports.output().try_managed::<f64>()?,
            };
            Ok(specialized_function(
                context,
                Box::new(function),
                output,
                vec![input],
            ))
        }
    }

    #[derive(Clone, Copy, Debug)]
    enum Comparison {
        Equal,
        NotEqual,
        Less,
        Greater,
        LessEqual,
        GreaterEqual,
    }

    #[derive(Debug)]
    struct ComparisonFunction {
        operation: Comparison,
        lhs: ValueCell,
        rhs: ValueCell,
        out: ManagedPort<bool>,
    }

    impl MechFunctionImpl for ComparisonFunction {
        fn solve_managed(
            &self,
            frame: &mut mech_core::KernelMemoryFrame<'_>,
            _services: &mut dyn mech_core::MechExecutionServices,
        ) -> MResult<mech_core::ReactiveSolveStatus> {
            let result = match self.operation {
                Comparison::Equal => canonical_values_equal(&self.lhs, &self.rhs)?,
                Comparison::NotEqual => !canonical_values_equal(&self.lhs, &self.rhs)?,
                Comparison::Less => numeric_pair(&self.lhs, &self.rhs, |a, b| a < b)?,
                Comparison::Greater => numeric_pair(&self.lhs, &self.rhs, |a, b| a > b)?,
                Comparison::LessEqual => numeric_pair(&self.lhs, &self.rhs, |a, b| a <= b)?,
                Comparison::GreaterEqual => numeric_pair(&self.lhs, &self.rhs, |a, b| a >= b)?,
            };
            frame.with_output_port_view(&self.out, |output| {
                output.try_fill_column_major(|_| Ok(result))
            })?;
            Ok(mech_core::ReactiveSolveStatus::Changed)
        }

        fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
            Some(FunctionStatePort::from_cell(self.out.cell()))
        }

        fn to_string(&self) -> String {
            format!("Test{:?}", self.operation)
        }
    }

    #[cfg(feature = "semantic-compiler")]
    impl MechFunctionCompiler for ComparisonFunction {
        fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }

    struct ComparisonSpecializer(Comparison);

    impl CanonicalFunctionSpecializer for ComparisonSpecializer {
        fn specialize_invocation(
            &self,
            invocation: &SpecializationInvocation,
            context: &mut SpecializationContext<'_>,
        ) -> MResult<SpecializedFunction> {
            if invocation.len() != 2 {
                return Err(test_operation_error("comparison expects two arguments"));
            }
            let lhs = canonical_input(invocation, 0, "comparison expects a lhs")?;
            let rhs = canonical_input(invocation, 1, "comparison expects a rhs")?;
            let output = ValueCell::from_exact(false)?;
            let ports = FunctionInvocation::binary(output.clone(), lhs.clone(), rhs.clone());
            let function = ComparisonFunction {
                operation: self.0,
                lhs: lhs.clone(),
                rhs: rhs.clone(),
                out: ports.output().try_managed::<bool>()?,
            };
            Ok(specialized_function(
                context,
                Box::new(function),
                output,
                vec![lhs, rhs],
            ))
        }

        fn guard_safety(&self) -> GuardFunctionSafety {
            GuardFunctionSafety::PureStatic
        }
    }

    #[derive(Clone, Copy, Debug)]
    enum BooleanOperation {
        And,
        Or,
        Xor,
    }

    #[derive(Debug)]
    struct BooleanFunction {
        operation: BooleanOperation,
        lhs: ManagedPort<bool>,
        rhs: ManagedPort<bool>,
        out: ManagedPort<bool>,
    }

    impl MechFunctionImpl for BooleanFunction {
        fn solve_managed(
            &self,
            frame: &mut mech_core::KernelMemoryFrame<'_>,
            _services: &mut dyn mech_core::MechExecutionServices,
        ) -> MResult<mech_core::ReactiveSolveStatus> {
            frame.with_binary_port_views(&self.lhs, &self.rhs, &self.out, |lhs, rhs, out| {
                out.try_fill_column_major(|index| {
                    let lhs = lhs
                        .get_column_major(index)
                        .ok_or_else(|| test_operation_error("boolean lhs geometry mismatch"))?;
                    let rhs = rhs
                        .get_column_major(index)
                        .ok_or_else(|| test_operation_error("boolean rhs geometry mismatch"))?;
                    Ok(match self.operation {
                        BooleanOperation::And => lhs && rhs,
                        BooleanOperation::Or => lhs || rhs,
                        BooleanOperation::Xor => lhs ^ rhs,
                    })
                })
            })?;
            Ok(mech_core::ReactiveSolveStatus::Changed)
        }

        fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
            Some(FunctionStatePort::from_cell(self.out.cell()))
        }

        fn to_string(&self) -> String {
            format!("Test{:?}", self.operation)
        }
    }

    #[cfg(feature = "semantic-compiler")]
    impl MechFunctionCompiler for BooleanFunction {
        fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }

    struct BooleanSpecializer(BooleanOperation);

    impl CanonicalFunctionSpecializer for BooleanSpecializer {
        fn specialize_invocation(
            &self,
            invocation: &SpecializationInvocation,
            context: &mut SpecializationContext<'_>,
        ) -> MResult<SpecializedFunction> {
            if invocation.len() != 2 {
                return Err(test_operation_error(
                    "boolean operation expects two arguments",
                ));
            }
            let lhs = canonical_input(invocation, 0, "boolean operation expects a lhs")?;
            let rhs = canonical_input(invocation, 1, "boolean operation expects a rhs")?;
            let output = ValueCell::from_exact(false)?;
            let ports = FunctionInvocation::binary(output.clone(), lhs.clone(), rhs.clone());
            let function = BooleanFunction {
                operation: self.0,
                lhs: ports.input(0).unwrap().try_managed::<bool>()?,
                rhs: ports.input(1).unwrap().try_managed::<bool>()?,
                out: ports.output().try_managed::<bool>()?,
            };
            Ok(specialized_function(
                context,
                Box::new(function),
                output,
                vec![lhs, rhs],
            ))
        }

        fn guard_safety(&self) -> GuardFunctionSafety {
            GuardFunctionSafety::PureStatic
        }
    }

    #[derive(Debug)]
    struct NotFunction {
        input: ManagedPort<bool>,
        out: ManagedPort<bool>,
    }

    impl MechFunctionImpl for NotFunction {
        fn solve_managed(
            &self,
            frame: &mut mech_core::KernelMemoryFrame<'_>,
            _services: &mut dyn mech_core::MechExecutionServices,
        ) -> MResult<mech_core::ReactiveSolveStatus> {
            frame.with_unary_port_views(&self.input, &self.out, |input, output| {
                output.try_fill_column_major(|index| {
                    input
                        .get_column_major(index)
                        .map(|value| !value)
                        .ok_or_else(|| test_operation_error("boolean not geometry mismatch"))
                })
            })?;
            Ok(mech_core::ReactiveSolveStatus::Changed)
        }

        fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
            Some(FunctionStatePort::from_cell(self.out.cell()))
        }

        fn to_string(&self) -> String {
            "TestNot".to_string()
        }
    }

    #[cfg(feature = "semantic-compiler")]
    impl MechFunctionCompiler for NotFunction {
        fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }

    struct NotSpecializer;

    impl CanonicalFunctionSpecializer for NotSpecializer {
        fn specialize_invocation(
            &self,
            invocation: &SpecializationInvocation,
            context: &mut SpecializationContext<'_>,
        ) -> MResult<SpecializedFunction> {
            if invocation.len() != 1 {
                return Err(test_operation_error("boolean not expects one argument"));
            }
            let input = canonical_input(invocation, 0, "boolean not expects an input")?;
            let output = ValueCell::from_exact(false)?;
            let ports = FunctionInvocation::unary(output.clone(), input.clone());
            let function = NotFunction {
                input: ports.input(0).unwrap().try_managed::<bool>()?,
                out: ports.output().try_managed::<bool>()?,
            };
            Ok(specialized_function(
                context,
                Box::new(function),
                output,
                vec![input],
            ))
        }

        fn guard_safety(&self) -> GuardFunctionSafety {
            GuardFunctionSafety::PureStatic
        }
    }

    #[derive(Debug)]
    struct AddAssignFunction {
        base: ManagedPort<f64>,
        sink: ManagedPort<f64>,
        source: ManagedPort<f64>,
    }

    impl MechFunctionImpl for AddAssignFunction {
        fn solve_managed(
            &self,
            frame: &mut mech_core::KernelMemoryFrame<'_>,
            _services: &mut dyn mech_core::MechExecutionServices,
        ) -> MResult<mech_core::ReactiveSolveStatus> {
            frame.with_binary_port_views(
                &self.base,
                &self.source,
                &self.sink,
                |base, source, sink| {
                    sink.try_fill_column_major(|index| {
                        Ok(base.get_column_major(index).unwrap()
                            + source.get_column_major(index).unwrap())
                    })
                },
            )?;
            Ok(mech_core::ReactiveSolveStatus::Changed)
        }

        fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
            Some(FunctionStatePort::from_cell(self.sink.cell()))
        }

        fn reactive_node_kind(&self) -> ReactiveNodeKind {
            ReactiveNodeKind::Register
        }

        fn to_string(&self) -> String {
            "TestAddAssign".to_string()
        }
    }

    #[cfg(feature = "semantic-compiler")]
    impl MechFunctionCompiler for AddAssignFunction {
        fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }

    struct AddAssignSpecializer;

    impl CanonicalFunctionSpecializer for AddAssignSpecializer {
        fn specialize_invocation(
            &self,
            invocation: &SpecializationInvocation,
            context: &mut SpecializationContext<'_>,
        ) -> MResult<SpecializedFunction> {
            if invocation.len() != 2 {
                return Err(test_operation_error("add assignment expects two arguments"));
            }
            let sink = canonical_input(invocation, 0, "add assignment expects a sink")?;
            let source = canonical_input(invocation, 1, "add assignment expects a source")?;
            let ports = FunctionInvocation::binary(sink.clone(), sink.clone(), source.clone());
            let function = AddAssignFunction {
                base: ports.input(0).unwrap().try_managed::<f64>()?,
                sink: ports.output().try_managed::<f64>()?,
                source: ports.input(1).unwrap().try_managed::<f64>()?,
            };
            Ok(specialized_function(
                context,
                Box::new(function),
                sink.clone(),
                vec![sink, source],
            ))
        }
    }

    #[cfg(all(feature = "f64", feature = "matrix", feature = "range_inclusive"))]
    #[derive(Debug)]
    struct InclusiveRangeFunction {
        start: ManagedPort<f64>,
        terminal: ManagedPort<f64>,
        output: ManagedPort<f64>,
    }

    #[cfg(all(feature = "f64", feature = "matrix", feature = "range_inclusive"))]
    impl MechFunctionImpl for InclusiveRangeFunction {
        fn solve_managed(
            &self,
            frame: &mut mech_core::KernelMemoryFrame<'_>,
            _services: &mut dyn mech_core::MechExecutionServices,
        ) -> MResult<mech_core::ReactiveSolveStatus> {
            frame.with_binary_port_views(
                &self.start,
                &self.terminal,
                &self.output,
                |start, terminal, output| {
                    let start = start.get_column_major(0).ok_or_else(|| {
                        test_operation_error("inclusive range start is unavailable")
                    })? as usize;
                    let terminal = terminal.get_column_major(0).ok_or_else(|| {
                        test_operation_error("inclusive range terminal is unavailable")
                    })? as usize;
                    let expected = terminal.saturating_sub(start).saturating_add(1);
                    if expected != output.len() {
                        return Err(test_operation_error(
                            "inclusive range output shape differs from its managed plan",
                        ));
                    }
                    output.try_fill_column_major(|index| Ok((start + index) as f64))
                },
            )?;
            Ok(mech_core::ReactiveSolveStatus::Changed)
        }

        fn initial_solve_policy(&self) -> InitialSolvePolicy {
            InitialSolvePolicy::PreserveSpecializedOutput
        }

        fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
            Some(FunctionStatePort::from_cell(self.output.cell()))
        }

        fn to_string(&self) -> String {
            "TestInclusiveRange".to_string()
        }
    }

    #[cfg(all(
        feature = "f64",
        feature = "matrix",
        feature = "range_inclusive",
        feature = "semantic-compiler"
    ))]
    impl MechFunctionCompiler for InclusiveRangeFunction {
        fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }

    #[cfg(all(feature = "f64", feature = "matrix", feature = "range_inclusive"))]
    struct InclusiveRangeSpecializer;

    #[cfg(all(feature = "f64", feature = "matrix", feature = "range_inclusive"))]
    impl CanonicalFunctionSpecializer for InclusiveRangeSpecializer {
        fn specialize_invocation(
            &self,
            invocation: &SpecializationInvocation,
            context: &mut SpecializationContext<'_>,
        ) -> MResult<SpecializedFunction> {
            if invocation.len() != 2 {
                return Err(test_operation_error(
                    "inclusive range expects two arguments",
                ));
            }
            let start = canonical_input(invocation, 0, "inclusive range expects a start")?;
            let terminal = canonical_input(invocation, 1, "inclusive range expects a terminal")?;
            let start_value = match start.snapshot()?.data() {
                ValueData::F64(value) => value.to_f64() as usize,
                _ => return Err(test_operation_error("inclusive range start must be F64")),
            };
            let terminal_value = match terminal.snapshot()?.data() {
                ValueData::F64(value) => value.to_f64() as usize,
                _ => return Err(test_operation_error("inclusive range terminal must be F64")),
            };
            let elements = (start_value..=terminal_value)
                .map(|value| ValueDataDraft::F64(F64Bits::from_f64(value as f64)))
                .collect::<Vec<_>>();
            let output = ValueCell::dynamic_matrix(
                SchemaBody::FloatingPoint(FloatWidth::W64),
                vec![elements.len() as u64, 1].into_boxed_slice(),
                elements.into_boxed_slice(),
            )?;
            let ports = FunctionInvocation::binary(output.clone(), start.clone(), terminal.clone());
            let function = Self::function(&ports)?;
            Ok(specialized_function(
                context,
                Box::new(function),
                output,
                vec![start, terminal],
            ))
        }
    }

    #[cfg(all(feature = "f64", feature = "matrix", feature = "range_inclusive"))]
    impl InclusiveRangeSpecializer {
        fn function(invocation: &FunctionInvocation) -> MResult<InclusiveRangeFunction> {
            Ok(InclusiveRangeFunction {
                start: invocation.input(0).unwrap().try_managed::<f64>()?,
                terminal: invocation.input(1).unwrap().try_managed::<f64>()?,
                output: invocation.output().try_managed_element::<f64>()?,
            })
        }
    }

    fn numeric_pair(
        lhs: &ValueCell,
        rhs: &ValueCell,
        compare: impl FnOnce(f64, f64) -> bool,
    ) -> MResult<bool> {
        let lhs = lhs.snapshot()?;
        let rhs = rhs.snapshot()?;
        let (ValueData::F64(lhs), ValueData::F64(rhs)) = (lhs.data(), rhs.data()) else {
            return Err(test_operation_error(
                "ordered comparison expects two F64 values",
            ));
        };
        Ok(compare(lhs.to_f64(), rhs.to_f64()))
    }

    fn canonical_values_equal(lhs: &ValueCell, rhs: &ValueCell) -> MResult<bool> {
        let lhs = lhs.snapshot()?;
        let rhs = rhs.snapshot()?;
        if lhs.schema_key() != rhs.schema_key() || lhs.shape() != rhs.shape() {
            return Ok(false);
        }
        let lhs_schemas = lhs
            .schemas()
            .ok_or_else(|| test_operation_error("lhs snapshot has no schema arena"))?;
        let rhs_schemas = rhs
            .schemas()
            .ok_or_else(|| test_operation_error("rhs snapshot has no schema arena"))?;
        let lhs = lhs
            .canonical_snapshot_bytes(lhs_schemas.as_ref())
            .map_err(|error| test_operation_error(&format!("invalid lhs snapshot: {error:?}")))?;
        let rhs = rhs
            .canonical_snapshot_bytes(rhs_schemas.as_ref())
            .map_err(|error| test_operation_error(&format!("invalid rhs snapshot: {error:?}")))?;
        Ok(lhs == rhs)
    }

    fn test_operation_error(message: &str) -> MechError {
        MechError::new(
            GenericError {
                msg: message.to_string(),
            },
            None,
        )
        .with_compiler_loc()
    }

    pub(super) fn install(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
        for (name, operation) in [
            ("math/add", BinaryArithmetic::Add),
            ("math/sub", BinaryArithmetic::Subtract),
            ("math/mul", BinaryArithmetic::Multiply),
            ("math/div", BinaryArithmetic::Divide),
        ] {
            builder.insert_canonical_intrinsic_specializer(
                name,
                pure_contract(2),
                Arc::new(BinaryArithmeticSpecializer(operation)),
            )?;
        }
        builder.insert_canonical_intrinsic_specializer(
            "math/neg",
            pure_contract(1),
            Arc::new(NegateSpecializer),
        )?;

        for (name, operation) in [
            ("compare/eq", Comparison::Equal),
            ("compare/neq", Comparison::NotEqual),
            ("compare/lt", Comparison::Less),
            ("compare/gt", Comparison::Greater),
            ("compare/lte", Comparison::LessEqual),
            ("compare/gte", Comparison::GreaterEqual),
        ] {
            builder.insert_canonical_intrinsic_specializer(
                name,
                pure_contract(2),
                Arc::new(ComparisonSpecializer(operation)),
            )?;
        }

        for (name, operation) in [
            ("logic/and", BooleanOperation::And),
            ("logic/or", BooleanOperation::Or),
            ("logic/xor", BooleanOperation::Xor),
        ] {
            builder.insert_canonical_intrinsic_specializer(
                name,
                pure_contract(2),
                Arc::new(BooleanSpecializer(operation)),
            )?;
        }
        builder.insert_canonical_intrinsic_specializer(
            "logic/not",
            pure_contract(1),
            Arc::new(NotSpecializer),
        )?;
        builder.insert_canonical_intrinsic_specializer(
            "math/add-assign",
            state_rmw_contract(),
            Arc::new(AddAssignSpecializer),
        )?;
        #[cfg(all(feature = "f64", feature = "matrix", feature = "range_inclusive"))]
        builder.insert_canonical_intrinsic_specializer(
            "range/inclusive",
            pure_contract(2),
            Arc::new(InclusiveRangeSpecializer),
        )?;
        Ok(())
    }
}

#[cfg(all(test, feature = "semantic-compiler"))]
mod tests {
    use super::*;
    use crate::Interpreter;
    #[cfg(feature = "program")]
    use crate::{CompilerPlanningConfig, CompilerPlanningProgram, ExtensionFunctionId};
    #[cfg(all(
        feature = "program",
        feature = "source",
        feature = "formulas",
        feature = "math_add",
        feature = "f64",
        feature = "semantic-compiler"
    ))]
    use mech_core::{BytecodeCompilerContext, MechFunctionCompiler, Register};
    #[cfg(all(
        feature = "program",
        feature = "source",
        feature = "formulas",
        feature = "math_add",
        feature = "f64"
    ))]
    use mech_core::{
        CanonicalFunctionSpecializer, ExecutionTarget, FunctionExport, FunctionExposure,
        FunctionInvocation, ManagedPort, MechFunctionImpl, RuntimeFunctionId,
        SpecializationContext, SpecializationInvocation, SpecializedFunction, ValueCell, ValueData,
    };

    #[cfg(all(
        feature = "program",
        feature = "source",
        feature = "formulas",
        feature = "math_add",
        feature = "f64"
    ))]
    struct TestAddFunction {
        lhs: ManagedPort<f64>,
        rhs: ManagedPort<f64>,
        out: ManagedPort<f64>,
    }

    #[cfg(all(
        feature = "program",
        feature = "source",
        feature = "formulas",
        feature = "math_add",
        feature = "f64"
    ))]
    impl MechFunctionImpl for TestAddFunction {
        fn solve_managed(
            &self,
            frame: &mut mech_core::KernelMemoryFrame<'_>,
            _services: &mut dyn mech_core::MechExecutionServices,
        ) -> MResult<mech_core::ReactiveSolveStatus> {
            frame.with_binary_port_views(&self.lhs, &self.rhs, &self.out, |lhs, rhs, out| {
                out.try_fill_column_major(|index| {
                    Ok(lhs.get_column_major(index).unwrap() + rhs.get_column_major(index).unwrap())
                })
            })?;
            Ok(mech_core::ReactiveSolveStatus::Changed)
        }

        fn primary_output_state_port(&self) -> Option<mech_core::FunctionStatePort<'_>> {
            Some(mech_core::FunctionStatePort::from_cell(self.out.cell()))
        }

        fn to_string(&self) -> String {
            String::from("TestAddFunction")
        }
    }

    #[cfg(all(
        feature = "program",
        feature = "source",
        feature = "formulas",
        feature = "math_add",
        feature = "f64",
        feature = "semantic-compiler"
    ))]
    impl MechFunctionCompiler for TestAddFunction {
        fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }

    #[cfg(all(
        feature = "program",
        feature = "source",
        feature = "formulas",
        feature = "math_add",
        feature = "f64"
    ))]
    struct TestAddSpecializer;

    #[cfg(all(
        feature = "program",
        feature = "source",
        feature = "formulas",
        feature = "math_add",
        feature = "f64"
    ))]
    impl CanonicalFunctionSpecializer for TestAddSpecializer {
        fn specialize_invocation(
            &self,
            invocation: &SpecializationInvocation,
            context: &mut SpecializationContext<'_>,
        ) -> MResult<SpecializedFunction> {
            let [lhs, rhs] = invocation.inputs() else {
                panic!("test math/add expects two f64 arguments");
            };
            let lhs = lhs.cell()?.clone();
            let rhs = rhs.cell()?.clone();
            let output = ValueCell::from_exact(0.0_f64)?;
            let ports = FunctionInvocation::binary(output.clone(), lhs.clone(), rhs.clone());
            let function = TestAddFunction {
                lhs: ports.input(0).unwrap().try_managed::<f64>()?,
                rhs: ports.input(1).unwrap().try_managed::<f64>()?,
                out: ports.output().try_managed::<f64>()?,
            };
            context.certify_instance(
                (
                    Box::new(function),
                    FunctionInvocation::binary(output, lhs, rhs),
                ),
                RuntimeFunctionId::from_name("TestAddFunction"),
                ExecutionTarget::DirectRuntime,
                mech_core::ImplementationMemoryClass::NoAdditionalScratch,
            )
        }
    }

    #[test]
    fn empty_catalog_has_no_function_surface() {
        let catalog = empty_function_catalog();

        assert_eq!(catalog.runtime_factory_count(), 0);
        assert_eq!(catalog.specializer_count(), 0);
        assert_eq!(catalog.intrinsic_specializer_count(), 0);
        assert_eq!(catalog.all_exports().len(), 0);
    }

    #[test]
    fn empty_catalog_is_not_cached() {
        let first = empty_function_catalog();
        let second = empty_function_catalog();

        assert!(!Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn interpreter_new_has_an_empty_catalog() {
        let interpreter = Interpreter::new(41, 100);
        let catalog = interpreter.function_catalog();

        assert_eq!(catalog.runtime_factory_count(), 0);
        assert_eq!(catalog.specializer_count(), 0);
        assert_eq!(catalog.intrinsic_specializer_count(), 0);
        assert_eq!(catalog.all_exports().len(), 0);
    }

    #[cfg(feature = "program")]
    #[test]
    fn program_new_has_an_empty_catalog() {
        let program = CompilerPlanningProgram::new(CompilerPlanningConfig::default());
        let catalog = program.function_catalog();

        assert_eq!(catalog.runtime_factory_count(), 0);
        assert_eq!(catalog.specializer_count(), 0);
        assert_eq!(catalog.intrinsic_specializer_count(), 0);
        assert_eq!(catalog.all_exports().len(), 0);
    }

    #[cfg(feature = "program")]
    #[test]
    fn bare_programs_have_independent_catalogs_and_environments() {
        let first = CompilerPlanningProgram::new(CompilerPlanningConfig::default());
        let second = CompilerPlanningProgram::new(CompilerPlanningConfig::default());
        let extension = ExtensionFunctionId::from_name("host/first-only");

        assert!(!Arc::ptr_eq(
            first.function_catalog(),
            second.function_catalog()
        ));

        first
            .interpreter
            .state
            .borrow_mut()
            .function_environment
            .bind_extension("host/first-only", "first-only", extension)
            .unwrap();

        assert_eq!(
            first
                .interpreter
                .state
                .borrow()
                .function_environment
                .resolve_name("first-only"),
            Some(crate::FunctionBinding::Extension(extension)),
        );
        assert_eq!(
            second
                .interpreter
                .state
                .borrow()
                .function_environment
                .resolve_name("first-only"),
            None,
        );
    }

    #[cfg(all(
        feature = "program",
        feature = "source",
        feature = "formulas",
        feature = "math_add",
        feature = "f64"
    ))]
    #[test]
    fn bare_program_cannot_execute_math_add() {
        let mut program = CompilerPlanningProgram::new(CompilerPlanningConfig::default());

        let error = program.plan_source_for_test("1.0 + 2.0").unwrap_err();

        assert_eq!(error.kind_name(), "FunctionOperationUnavailable");
        assert!(error.kind_message().contains("math/add"));
    }

    #[cfg(all(feature = "program", feature = "source"))]
    #[test]
    fn bare_program_does_not_know_standard_modules() {
        let mut program = CompilerPlanningProgram::new(CompilerPlanningConfig::default());

        assert!(!program.function_catalog().has_module("math"));
        let error = program.plan_source_for_test("+> math").unwrap_err();

        assert_eq!(error.kind_name(), "MissingFunction");
    }

    #[cfg(all(
        feature = "program",
        feature = "source",
        feature = "formulas",
        feature = "math_add",
        feature = "f64"
    ))]
    #[test]
    fn supplied_custom_catalog_executes_math_add() {
        let mut builder = FunctionCatalogBuilder::new();
        let operation = builder
            .insert_canonical_specializer_with_contract(
                "math/add",
                mech_core::maintained_source_type_declaration("math/add").unwrap(),
                super::pure_test_operation_contract(2),
                Arc::new(TestAddSpecializer),
            )
            .unwrap();
        builder
            .insert_export(FunctionExport {
                operation,
                canonical_name: String::from("math/add"),
                module: None,
                item: None,
                exposure: FunctionExposure::Prelude,
            })
            .unwrap();
        let catalog = Arc::new(builder.build().unwrap());
        let mut program = CompilerPlanningProgram::with_function_catalog(
            CompilerPlanningConfig::default(),
            catalog,
        );

        let output = program.plan_source_for_test("1.0 + 2.0").unwrap().unwrap();
        let output = output.snapshot().unwrap();
        let ValueData::F64(output) = output.data() else {
            panic!("test math/add output must remain F64");
        };
        assert_eq!(output.to_f64(), 3.0);
    }
}
