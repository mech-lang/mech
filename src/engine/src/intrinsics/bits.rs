//! Fixed U64 word operations. Shifts use the low six count bits; arithmetic
//! wraps modulo 2^64. Ordinary math operations retain checked overflow.
use crate::*;
#[cfg(feature = "semantic-compiler")]
use std::sync::Arc;
use std::sync::LazyLock;

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub(crate) enum Operation {
    And,
    Or,
    Xor,
    Not,
    ShiftLeft,
    ShiftRight,
    LeadingZeros,
    TrailingZeros,
    WrappingAdd,
    WrappingSub,
    WrappingMul,
}
pub(crate) const OPERATIONS: [Operation; 11] = [
    Operation::And,
    Operation::Or,
    Operation::Xor,
    Operation::Not,
    Operation::ShiftLeft,
    Operation::ShiftRight,
    Operation::LeadingZeros,
    Operation::TrailingZeros,
    Operation::WrappingAdd,
    Operation::WrappingSub,
    Operation::WrappingMul,
];
impl Operation {
    #[cfg(any(feature = "semantic-compiler", feature = "resident-artifact"))]
    pub(crate) const fn item(self) -> &'static str {
        match self {
            Self::And => "and",
            Self::Or => "or",
            Self::Xor => "xor",
            Self::Not => "not",
            Self::ShiftLeft => "shift-left-wrap",
            Self::ShiftRight => "shift-right-wrap",
            Self::LeadingZeros => "leading-zeros",
            Self::TrailingZeros => "trailing-zeros",
            Self::WrappingAdd => "wrapping-add",
            Self::WrappingSub => "wrapping-sub",
            Self::WrappingMul => "wrapping-mul",
        }
    }
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::And => "bits/and",
            Self::Or => "bits/or",
            Self::Xor => "bits/xor",
            Self::Not => "bits/not",
            Self::ShiftLeft => "bits/shift-left-wrap",
            Self::ShiftRight => "bits/shift-right-wrap",
            Self::LeadingZeros => "bits/leading-zeros",
            Self::TrailingZeros => "bits/trailing-zeros",
            Self::WrappingAdd => "bits/wrapping-add",
            Self::WrappingSub => "bits/wrapping-sub",
            Self::WrappingMul => "bits/wrapping-mul",
        }
    }
    pub(crate) const fn arity(self) -> usize {
        match self {
            Self::Not | Self::LeadingZeros | Self::TrailingZeros => 1,
            _ => 2,
        }
    }
    pub(crate) fn apply(self, lhs: u64, rhs: u64) -> u64 {
        match self {
            Self::And => lhs & rhs,
            Self::Or => lhs | rhs,
            Self::Xor => lhs ^ rhs,
            Self::Not => !lhs,
            Self::ShiftLeft => lhs.wrapping_shl((rhs & 63) as u32),
            Self::ShiftRight => lhs.wrapping_shr((rhs & 63) as u32),
            Self::LeadingZeros => u64::from(lhs.leading_zeros()),
            Self::TrailingZeros => u64::from(lhs.trailing_zeros()),
            Self::WrappingAdd => lhs.wrapping_add(rhs),
            Self::WrappingSub => lhs.wrapping_sub(rhs),
            Self::WrappingMul => lhs.wrapping_mul(rhs),
        }
    }
    pub(crate) fn contract(self) -> &'static OperationContractDeclaration {
        static UNARY: LazyLock<OperationContractDeclaration> = LazyLock::new(|| {
            mech_core::elementwise_operation_contract(1, ChangeDetectionPolicy::ExactScalar)
        });
        static BINARY: LazyLock<OperationContractDeclaration> = LazyLock::new(|| {
            mech_core::elementwise_operation_contract(2, ChangeDetectionPolicy::ExactScalar)
        });
        if self.arity() == 1 { &UNARY } else { &BINARY }
    }
}
fn invalid() -> MechError {
    function_shape_contract_violation("bits", "expected exact scalar U64 ports")
}
fn validate(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
    for cell in inputs.iter().chain([output]) {
        if cell.closed_schema_body()? != SchemaBody::UnsignedInteger(IntegerWidth::W64) {
            return Err(invalid());
        }
    }
    Ok(())
}

#[cfg(feature = "u64")]
#[derive(Debug)]
struct WordPorts {
    lhs: ManagedPort<u64>,
    rhs: Option<ManagedPort<u64>>,
    output: ManagedPort<u64>,
}

#[derive(Debug)]
struct WordRuntime {
    operation: Operation,
    inputs: Vec<ValueCell>,
    output: ValueCell,
    #[cfg(feature = "u64")]
    fixed: Option<WordPorts>,
}
impl MechFunctionImpl for WordRuntime {
    fn planned_output_footprints(&self) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
        Ok(Some(
            vec![self.output.current_memory_footprint()?].into_boxed_slice(),
        ))
    }
    fn solve_managed(
        &self,
        frame: &mut KernelMemoryFrame<'_>,
        _: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveSolveStatus> {
        #[cfg(feature = "u64")]
        if let Some(ports) = &self.fixed {
            if let Some(rhs) = &ports.rhs {
                frame.with_binary_port_views(&ports.lhs, rhs, &ports.output, |lhs, rhs, out| {
                    let lhs = lhs.get(0, 0).ok_or_else(invalid)?;
                    let rhs = rhs.get(0, 0).ok_or_else(invalid)?;
                    out.try_fill_column_major(|_| Ok(self.operation.apply(lhs, rhs)))
                })?;
            } else {
                frame.with_unary_port_views(&ports.lhs, &ports.output, |lhs, out| {
                    let lhs = lhs.get(0, 0).ok_or_else(invalid)?;
                    out.try_fill_column_major(|_| Ok(self.operation.apply(lhs, 0)))
                })?;
            }
            return Ok(ReactiveSolveStatus::Changed);
        }
        // Canonical scalar values use the same operation with an admitted
        // staged output; reads still go through the current call's leases.
        let read = |frame: &KernelMemoryFrame<'_>, index: usize| -> MResult<u64> {
            let input = &self.inputs[index];
            if input.requires_canonical_output_builder()? {
                let value = frame.snapshot_input_cell(input, index)?;
                let ValueData::U64(value) = value.data() else {
                    return Err(invalid());
                };
                Ok(*value)
            } else {
                frame.with_input_cell_value_view::<u64, _>(input, index, |view| {
                    view.get(0, 0).ok_or_else(invalid)
                })
            }
        };
        frame.with_admitted_canonical_output(
            &self.output,
            self.output.current_memory_footprint()?,
            |frame, construction| {
                let lhs = read(frame, 0)?;
                let rhs = if self.operation.arity() == 2 {
                    read(frame, 1)?
                } else {
                    0
                };
                let next = construction.try_rebuild_data_draft(
                    &self.output,
                    ValueDataDraft::U64(self.operation.apply(lhs, rhs)),
                )?;
                Ok(((), next))
            },
        )?;
        Ok(ReactiveSolveStatus::Changed)
    }
    fn semantic_operation_name(&self) -> Option<&str> {
        Some(self.operation.name())
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(self.operation.contract())
    }
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_cell(&self.output))
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_cell(&self.output)]))
    }
    fn to_string(&self) -> String {
        self.operation.name().into()
    }
}
#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for WordRuntime {
    fn compiler_owned_value_cells(&self) -> Vec<ValueCell> {
        self.inputs
            .iter()
            .cloned()
            .chain([self.output.clone()])
            .collect()
    }
    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let output = compile_value_cell_register(&self.output, context)?;
        let lhs = compile_value_cell_register(&self.inputs[0], context)?;
        let function = context.function_id(self.operation.name())?;
        if self.operation.arity() == 2 {
            let rhs = compile_value_cell_register(&self.inputs[1], context)?;
            context.emit_binop(function, output, lhs, rhs);
        } else {
            context.emit_unop(function, output, lhs);
        }
        Ok(output)
    }
}
struct Factory<const OP: usize>;
impl<const OP: usize> MechFunctionFactory for Factory<OP> {
    const SIGNATURE: RuntimeFunctionSignature = if OPERATIONS[OP].arity() == 1 {
        RuntimeFunctionSignature::unary(
            FunctionValueRepresentation::AnyValue,
            FunctionValueRepresentation::AnyValue,
        )
    } else {
        RuntimeFunctionSignature::binary(
            FunctionValueRepresentation::AnyValue,
            FunctionValueRepresentation::AnyValue,
            FunctionValueRepresentation::AnyValue,
        )
    };
    fn implementation_memory_class() -> ImplementationMemoryClass {
        ImplementationMemoryClass::CanonicalFinalize
    }
    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(OPERATIONS[OP].contract())
    }
    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let operation = OPERATIONS[OP];
        let inputs = invocation.input_cells().to_vec();
        let output = invocation.output_cell().clone();
        if inputs.len() != operation.arity() {
            return Err(invalid());
        }
        validate(&output, &inputs)?;
        #[cfg(feature = "u64")]
        let fixed = {
            // The call frame marshals canonical U64 inputs through the core
            // codec. Only the output's storage selects the publication path.
            if !output.requires_canonical_output_builder()? {
                let (out, lhs, rhs) = if operation.arity() == 1 {
                    let (out, lhs) = invocation.expect_unary()?;
                    (out.try_managed::<u64>()?, lhs.try_managed::<u64>()?, None)
                } else {
                    let (out, lhs, rhs) = invocation.expect_binary()?;
                    (
                        out.try_managed::<u64>()?,
                        lhs.try_managed::<u64>()?,
                        Some(rhs.try_managed::<u64>()?),
                    )
                };
                Some(WordPorts {
                    lhs,
                    rhs,
                    output: out,
                })
            } else {
                None
            }
        };
        Ok(Box::new(WordRuntime {
            operation,
            inputs,
            output,
            #[cfg(feature = "u64")]
            fixed,
        }))
    }
}
macro_rules! registration {
    ($register:ident, $install:ident, $index:expr, $name:literal) => {
        mech_core::declare_native_runtime_factory! {
            cfg: feature = "integer_bits", registration: $register, installer: $install,
            name: $name, factory_type: Factory<$index>,
            contract: RuntimeFunctionContract::canonical_custom($name, RuntimeOutputAliasPolicy::DisallowInputAlias, validate),
            operations: [OperationId::from_name($name)],
            package: "mech-engine", crate_name: "mech_engine",
            installer_path: concat!("mech_engine::__mech_native::", stringify!($install)),
            extra_cargo_features: ["integer_bits"],
        }
    }
}
registration!(register_and, install_bits_and, 0, "bits/and");
registration!(register_or, install_bits_or, 1, "bits/or");
registration!(register_xor, install_bits_xor, 2, "bits/xor");
registration!(register_not, install_bits_not, 3, "bits/not");
registration!(register_shl, install_bits_shl, 4, "bits/shift-left-wrap");
registration!(register_shr, install_bits_shr, 5, "bits/shift-right-wrap");
registration!(register_clz, install_bits_clz, 6, "bits/leading-zeros");
registration!(register_ctz, install_bits_ctz, 7, "bits/trailing-zeros");
registration!(register_add, install_bits_add, 8, "bits/wrapping-add");
registration!(register_sub, install_bits_sub, 9, "bits/wrapping-sub");
registration!(register_mul, install_bits_mul, 10, "bits/wrapping-mul");
pub(crate) fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    for install in [
        register_and,
        register_or,
        register_xor,
        register_not,
        register_shl,
        register_shr,
        register_clz,
        register_ctz,
        register_add,
        register_sub,
        register_mul,
    ] {
        install(builder)?;
    }
    Ok(())
}
#[cfg(feature = "semantic-compiler")]
struct SourceOperation(Operation);
#[cfg(feature = "semantic-compiler")]
impl CanonicalFunctionSpecializer for SourceOperation {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        if invocation.len() != self.0.arity() {
            return Err(invalid());
        }
        let inputs = invocation.inputs().iter().collect::<Vec<_>>();
        context.bind_resolved_runtime(
            RuntimeBindingSelector::Operation(OperationId::from_name(self.0.name())),
            ExecutionTarget::DirectRuntime,
            vec![Box::<[u64]>::default()].into_boxed_slice(),
            &inputs,
        )
    }
}
#[cfg(feature = "semantic-compiler")]
pub(crate) fn install_source(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    for operation in OPERATIONS {
        let name = operation.name();
        let id = builder.insert_canonical_specializer_with_contracts(
            name,
            mech_core::maintained_source_type_declaration(name)?,
            vec![operation.contract().clone()],
            Arc::new(SourceOperation(operation)),
        )?;
        builder.insert_export(FunctionExport {
            operation: id,
            canonical_name: name.into(),
            module: Some("bits".into()),
            item: Some(operation.item().into()),
            exposure: FunctionExposure::ModuleOnly,
        })?;
    }
    Ok(())
}
