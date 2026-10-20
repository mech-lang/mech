//! Ordinary pure String line splitting and joining.
use crate::*;
use mech_core::snapshot::{SequenceView, prospective_string_value_footprint};
use std::sync::LazyLock;

#[path = "text_line_count.rs"]
mod text_line_count;
pub(crate) use text_line_count::count as line_count;

pub(crate) fn invalid(message: &'static str) -> MechError {
    MechError::new(
        GenericError {
            msg: message.into(),
        },
        None,
    )
    .with_compiler_loc()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Operation {
    Lines,
    Join,
}
impl Operation {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Lines => "text/lines",
            Self::Join => "text/join",
        }
    }
    pub(crate) fn arity(self) -> usize {
        if self == Self::Lines { 1 } else { 2 }
    }
    pub(crate) fn contract(self) -> &'static OperationContractDeclaration {
        match self {
            Self::Lines => &LINES,
            Self::Join => &JOIN,
        }
    }
}
static LINES: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| mech_core::maintained_operation_contract("text/lines", 1, true).unwrap());
static JOIN: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| mech_core::maintained_operation_contract("text/join", 2, false).unwrap());

pub(crate) fn is_string_matrix(body: &SchemaBody) -> bool {
    matches!(body, SchemaBody::Matrix {element, dimensions}
        if element.as_ref() == &SchemaBody::String && dimensions.len() == 2)
}

fn validate(operation: Operation, output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
    if inputs.len() != operation.arity() {
        return Err(invalid("text operation arity mismatch"));
    }
    let source = inputs[0].closed_schema_body()?;
    let target = output.closed_schema_body()?;
    let valid = match operation {
        Operation::Lines => {
            source == SchemaBody::String
                && is_string_matrix(&target)
                && matches!(&target, SchemaBody::Matrix {dimensions, ..} if dimensions[0] == DimensionExpr::Constant(1))
        }
        Operation::Join => {
            is_string_matrix(&source)
                && target == SchemaBody::String
                && inputs[1].closed_schema_body()? == SchemaBody::String
        }
    };
    if valid {
        Ok(())
    } else {
        Err(invalid(
            "text operation requires its declared String schemas",
        ))
    }
}
fn validate_lines(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
    validate(Operation::Lines, output, inputs)
}
fn validate_join(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
    validate(Operation::Join, output, inputs)
}

#[derive(Clone, Copy)]
struct Bounds {
    count: u64,
    payload: u64,
    input_bytes: u64,
}
fn overflow() -> MechError {
    invalid("text operation demand exceeds u64")
}
fn byte_bound(value: CurrentMemoryFootprint) -> MResult<u64> {
    value
        .encoded_bytes
        .max(value.payload_bytes)
        .checked_add(value.fixed_bytes)
        .ok_or_else(overflow)
}
fn bounds(operation: Operation, inputs: &[CurrentMemoryFootprint], count: u64) -> MResult<Bounds> {
    let source = byte_bound(inputs[0])?;
    match operation {
        Operation::Lines => Ok(Bounds {
            // A byte can end at most one line. This is an envelope, not an
            // observation of the input payload during planning.
            count: source.checked_add(1).ok_or_else(overflow)?,
            payload: source,
            input_bytes: source,
        }),
        Operation::Join => {
            let separator = byte_bound(inputs[1])?;
            Ok(Bounds {
                count: 1,
                payload: if count == 0 {
                    0
                } else {
                    source
                        .checked_add(separator.checked_mul(count - 1).ok_or_else(overflow)?)
                        .ok_or_else(overflow)?
                },
                input_bytes: source.checked_add(separator).ok_or_else(overflow)?,
            })
        }
    }
}

#[derive(Debug)]
struct TextRuntime {
    operation: Operation,
    inputs: Vec<ValueCell>,
    output: ValueCell,
}
impl TextRuntime {
    fn new(operation: Operation, invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        validate(
            operation,
            invocation.output_cell(),
            invocation.input_cells(),
        )?;
        Ok(Box::new(Self {
            operation,
            inputs: invocation.input_cells().to_vec(),
            output: invocation.output_cell().clone(),
        }))
    }
    fn planned_bounds(
        &self,
        input: &ResolvedValueDescriptor,
        footprints: &[CurrentMemoryFootprint],
    ) -> MResult<Bounds> {
        let count = if self.operation == Operation::Join {
            let SchemaBody::Matrix { dimensions, .. } = input.schema().body() else {
                return Err(invalid("text/join expects a matrix descriptor"));
            };
            dimensions.iter().try_fold(1u64, |count, dimension| {
                count
                    .checked_mul(
                        input
                            .shape()
                            .resolve_dimension(dimension)
                            .map_err(MechError::from)?,
                    )
                    .ok_or_else(overflow)
            })?
        } else {
            1
        };
        bounds(self.operation, footprints, count)
    }
    fn footprint(&self, bounds: Bounds) -> MResult<CurrentMemoryFootprint> {
        let parameters = self.output.shape().parameter_values().len();
        let value = prospective_string_value_footprint(
            parameters,
            self.operation == Operation::Lines,
            bounds.count,
            bounds.payload,
        )
        .map_err(|_| overflow())?;
        Ok(CurrentMemoryFootprint {
            logical_elements: bounds.count,
            payload_bytes: value.retained_bytes,
            encoded_bytes: value.encoded_bytes,
            retained_nodes: value.node_count,
            shape_parameter_count: parameters as u64,
            schema_bytes: self
                .output
                .retained_schema_table()
                .clone_allocation_bound_bytes()
                .ok_or_else(overflow)?,
            ..CurrentMemoryFootprint::default()
        })
    }
    fn construct(
        &self,
        construction: &mut FrozenSnapshotConstruction,
        inputs: &[Value],
    ) -> MResult<Value> {
        if self.operation == Operation::Lines {
            let ValueData::String(text) = inputs[0].data() else {
                return Err(invalid("text/lines expects String"));
            };
            let count = line_count(text);
            let mut lines = text.lines();
            let values = construction.try_boxed_slice_with(count, |construction, _| {
                Ok(ValueDataDraft::String(
                    construction
                        .try_concatenate_string(lines.next().expect("counted lines"), "")?,
                ))
            })?;
            let dimensions = construction
                .try_boxed_slice_with(2, |_, axis| Ok(if axis == 0 { 1 } else { count as u64 }))?;
            return construction.try_rebuild_matrix_drafts(&self.output, dimensions, values);
        }
        let ValueData::Matrix(matrix) = inputs[0].data() else {
            return Err(invalid("text/join expects a String matrix"));
        };
        let SequenceView::String(strings) = matrix.elements() else {
            return Err(invalid("text/join expects String elements"));
        };
        let ValueData::String(separator) = inputs[1].data() else {
            return Err(invalid("text/join separator must be String"));
        };
        let bytes = strings
            .iter()
            .try_fold(0usize, |bytes, text| {
                bytes.checked_add(text.len()).ok_or_else(overflow)
            })?
            .checked_add(
                separator
                    .len()
                    .checked_mul(strings.len().saturating_sub(1))
                    .ok_or_else(overflow)?,
            )
            .ok_or_else(overflow)?;
        let mut joined = construction.try_vec_with_capacity(bytes)?;
        for (index, text) in strings.iter().enumerate() {
            if index != 0 {
                joined.extend_from_slice(separator.as_bytes());
            }
            joined.extend_from_slice(text.as_bytes());
        }
        let joined =
            String::from_utf8(joined).map_err(|_| invalid("text/join produced invalid UTF-8"))?;
        construction.try_rebuild_data_draft(&self.output, ValueDataDraft::String(joined))
    }
}
impl MechFunctionImpl for TextRuntime {
    fn planned_output_footprints_with_request(
        &self,
        request: &ExecutionDemandPlanningRequest<'_>,
    ) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
        let inputs = [
            request
                .input_footprint(0)
                .map_err(|error| MechError::new(error, None).with_compiler_loc())?,
            if self.operation == Operation::Join {
                request
                    .input_footprint(1)
                    .map_err(|error| MechError::new(error, None).with_compiler_loc())?
            } else {
                CurrentMemoryFootprint::default()
            },
        ];
        Ok(Some(
            vec![self.footprint(self.planned_bounds(&request.inputs[0], &inputs)?)?]
                .into_boxed_slice(),
        ))
    }
    fn planned_execution_demand(
        &self,
        request: &ExecutionDemandPlanningRequest<'_>,
    ) -> MResult<PlannedExecutionDemand> {
        let inputs = [
            request
                .input_footprint(0)
                .map_err(|error| MechError::new(error, None).with_compiler_loc())?,
            if self.operation == Operation::Join {
                request
                    .input_footprint(1)
                    .map_err(|error| MechError::new(error, None).with_compiler_loc())?
            } else {
                CurrentMemoryFootprint::default()
            },
        ];
        let bounds = self.planned_bounds(&request.inputs[0], &inputs)?;
        let footprint = self.footprint(bounds)?;
        let scratch = footprint
            .payload_bytes
            .checked_mul(4)
            .and_then(|bytes| bytes.checked_add(bounds.input_bytes.checked_mul(4)?))
            .and_then(|bytes| bytes.checked_add(footprint.schema_bytes.checked_mul(4)?))
            .ok_or_else(overflow)?;
        let compute = bounds
            .payload
            .checked_add(bounds.input_bytes)
            .and_then(|work| work.checked_add(bounds.count))
            .and_then(|work| work.checked_add(footprint.schema_bytes))
            .and_then(|work| work.checked_mul(16))
            .ok_or_else(overflow)?;
        Ok(PlannedExecutionDemand {
            temporary_bytes: scratch,
            cloned_bytes: scratch,
            retained_nodes: footprint
                .retained_nodes
                .checked_mul(4)
                .ok_or_else(overflow)?,
            work: WorkDemand {
                compute,
                ..WorkDemand::default()
            },
        })
    }
    fn solve_managed(
        &self,
        frame: &mut KernelMemoryFrame<'_>,
        _: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveSolveStatus> {
        // Reuse the exact frozen input witnesses admitted by this frame. The
        // implementation keeps no cross-call planning cache or ambient views.
        let descriptor = frame.input_cell_descriptor(&self.inputs[0], 0)?;
        let footprints = [
            frame.input_cell_footprint(&self.inputs[0], 0)?,
            if self.operation == Operation::Join {
                frame.input_cell_footprint(&self.inputs[1], 1)?
            } else {
                CurrentMemoryFootprint::default()
            },
        ];
        let footprint = self.footprint(self.planned_bounds(descriptor, &footprints)?)?;
        frame.with_admitted_canonical_output(&self.output, footprint, |frame, construction| {
            let source =
                frame.snapshot_input_cell_with_construction(&self.inputs[0], 0, construction)?;
            let next = if self.operation == Operation::Lines {
                self.construct(construction, std::slice::from_ref(&source))?
            } else {
                let separator = frame.snapshot_input_cell_with_construction(
                    &self.inputs[1],
                    1,
                    construction,
                )?;
                self.construct(construction, &[source, separator])?
            };
            Ok(((), next))
        })?;
        Ok(ReactiveSolveStatus::Changed)
    }
    fn semantic_operation_name(&self) -> Option<&str> {
        Some(self.operation.name())
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(self.operation.contract())
    }
    fn to_string(&self) -> String {
        self.operation.name().into()
    }
}
#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for TextRuntime {
    fn compiler_owned_value_cells(&self) -> Vec<ValueCell> {
        self.inputs
            .iter()
            .cloned()
            .chain(std::iter::once(self.output.clone()))
            .collect()
    }
    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let output = compile_value_cell_register(&self.output, context)?;
        let first = compile_value_cell_register(&self.inputs[0], context)?;
        let function = context.function_id(self.operation.name())?;
        if self.operation == Operation::Lines {
            context.emit_unop(function, output, first);
        } else {
            let separator = compile_value_cell_register(&self.inputs[1], context)?;
            context.emit_binop(function, output, first, separator);
        }
        Ok(output)
    }
}
macro_rules! factory {
    ($name:ident, $operation:ident, $signature:expr) => {
        struct $name;
        impl MechFunctionFactory for $name {
            const SIGNATURE: RuntimeFunctionSignature = $signature;
            fn implementation_memory_class() -> ImplementationMemoryClass {
                ImplementationMemoryClass::CanonicalFinalize
            }
            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                TextRuntime::new(Operation::$operation, invocation)
            }
            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                Some(Operation::$operation.contract())
            }
        }
    };
}
factory!(
    LinesRuntime,
    Lines,
    RuntimeFunctionSignature::unary(
        FunctionValueRepresentation::AnyValue,
        FunctionValueRepresentation::AnyValue
    )
);
factory!(
    JoinRuntime,
    Join,
    RuntimeFunctionSignature::binary(
        FunctionValueRepresentation::AnyValue,
        FunctionValueRepresentation::AnyValue,
        FunctionValueRepresentation::AnyValue
    )
);
macro_rules! registration {
    ($register:ident, $install:ident, $factory:ty, $name:literal, $validator:ident) => {
        mech_core::declare_native_runtime_factory! {
            cfg: feature = "pure_text", registration: $register, installer: $install,
            name: $name, factory_type: $factory,
            contract: RuntimeFunctionContract::canonical_custom($name, RuntimeOutputAliasPolicy::DisallowInputAlias, $validator),
            operations: [OperationId::from_name($name)],
            package: "mech-engine", crate_name: "mech_engine",
            installer_path: concat!("mech_engine::__mech_native::", stringify!($install)),
            extra_cargo_features: ["pure_text"],
        }
    };
}
registration!(
    register_lines,
    install_text_lines,
    LinesRuntime,
    "text/lines",
    validate_lines
);
registration!(
    register_join,
    install_text_join,
    JoinRuntime,
    "text/join",
    validate_join
);
pub(crate) fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    register_lines(builder)?;
    register_join(builder)
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
            return Err(invalid("text operation arity mismatch"));
        }
        let inputs = invocation.inputs().iter().collect::<Vec<_>>();
        // The line count is discovered during admitted execution. No payload
        // evaluation is needed to seed the Turn-scoped lower-bound shape.
        let shape = if self.0 == Operation::Lines {
            vec![1, 0].into_boxed_slice()
        } else {
            Box::new([])
        };
        context.bind_resolved_runtime(
            RuntimeBindingSelector::Operation(OperationId::from_name(self.0.name())),
            ExecutionTarget::DirectRuntime,
            vec![shape].into_boxed_slice(),
            &inputs,
        )
    }
}
#[cfg(feature = "semantic-compiler")]
pub(crate) fn install_source(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    for operation in [Operation::Lines, Operation::Join] {
        let name = operation.name();
        let id = builder.insert_canonical_specializer_with_contracts(
            name,
            mech_core::maintained_source_type_declaration(name)?,
            vec![operation.contract().clone()],
            std::sync::Arc::new(SourceOperation(operation)),
        )?;
        builder.insert_export(FunctionExport {
            operation: id,
            canonical_name: name.into(),
            module: Some("text".into()),
            item: Some(name.split_once('/').unwrap().1.into()),
            exposure: FunctionExposure::ModuleOnly,
        })?;
    }
    Ok(())
}
