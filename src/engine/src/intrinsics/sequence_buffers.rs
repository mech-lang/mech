//! Capacity-retaining packed unsigned sequences shared by source and resident leaves.
use crate::*;
use mech_core::snapshot::{SequenceView, UnsignedMatrixBuffer};
use std::sync::LazyLock;

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
    WithCapacity,
    Reserve,
    Append,
    Clear,
}
#[cfg(feature = "semantic-compiler")]
pub(crate) const OPERATIONS: [Operation; 4] = [
    Operation::WithCapacity,
    Operation::Reserve,
    Operation::Append,
    Operation::Clear,
];
impl Operation {
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::WithCapacity => "sequence/with-capacity",
            Self::Reserve => "sequence/reserve",
            Self::Append => "sequence/append",
            Self::Clear => "sequence/clear",
        }
    }
    #[cfg(feature = "semantic-compiler")]
    pub(crate) const fn item(self) -> &'static str {
        match self {
            Self::WithCapacity => "with-capacity",
            Self::Reserve => "reserve",
            Self::Append => "append",
            Self::Clear => "clear",
        }
    }
    pub(crate) const fn arity(self) -> usize {
        if matches!(self, Self::Clear) { 1 } else { 2 }
    }
    pub(crate) fn contract(self) -> &'static OperationContractDeclaration {
        static WITH: LazyLock<OperationContractDeclaration> = LazyLock::new(|| {
            mech_core::maintained_operation_contract("sequence/with-capacity", 2, true).unwrap()
        });
        static RESERVE: LazyLock<OperationContractDeclaration> = LazyLock::new(|| {
            mech_core::maintained_operation_contract("sequence/reserve", 2, true).unwrap()
        });
        static APPEND: LazyLock<OperationContractDeclaration> = LazyLock::new(|| {
            mech_core::maintained_operation_contract("sequence/append", 2, true).unwrap()
        });
        static CLEAR: LazyLock<OperationContractDeclaration> = LazyLock::new(|| {
            mech_core::maintained_operation_contract("sequence/clear", 1, true).unwrap()
        });
        match self {
            Self::WithCapacity => &WITH,
            Self::Reserve => &RESERVE,
            Self::Append => &APPEND,
            Self::Clear => &CLEAR,
        }
    }
}

pub(crate) fn width(body: &SchemaBody) -> Option<usize> {
    match body {
        SchemaBody::UnsignedInteger(IntegerWidth::W8) => Some(1),
        SchemaBody::UnsignedInteger(IntegerWidth::W16) => Some(2),
        SchemaBody::UnsignedInteger(IntegerWidth::W32) => Some(4),
        SchemaBody::UnsignedInteger(IntegerWidth::W64) => Some(8),
        _ => None,
    }
}
pub(crate) fn element(body: &SchemaBody) -> Option<&SchemaBody> {
    let SchemaBody::Matrix {
        element,
        dimensions,
    } = body
    else {
        return None;
    };
    (dimensions.len() == 2
        && dimensions[0] == DimensionExpr::Constant(1)
        && width(element).is_some())
    .then_some(element)
}
fn input_element(body: &SchemaBody) -> Option<&SchemaBody> {
    match body {
        SchemaBody::Matrix {
            element,
            dimensions,
        } if dimensions.len() == 2 && width(element).is_some() => Some(element),
        _ => None,
    }
}
pub(crate) fn schemas_supported(
    operation: Operation,
    inputs: &[SchemaBody],
    output: &SchemaBody,
) -> bool {
    if inputs.len() != operation.arity() {
        return false;
    }
    let source = if operation == Operation::WithCapacity {
        width(&inputs[0]).map(|_| &inputs[0])
    } else {
        input_element(&inputs[0])
    };
    let Some(source) = source else {
        return false;
    };
    element(output) == Some(source)
        && match operation {
            Operation::WithCapacity | Operation::Reserve => {
                inputs[1] == SchemaBody::UnsignedInteger(IntegerWidth::W64)
            }
            Operation::Append => &inputs[1] == source,
            Operation::Clear => true,
        }
}

/// Fixed initialized length and separately admitted physical capacity.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Geometry {
    pub(crate) length: usize,
    pub(crate) capacity: usize,
    pub(crate) width: usize,
}
impl Geometry {
    pub(crate) fn bytes(self) -> MResult<u64> {
        self.capacity
            .checked_mul(self.width)
            .and_then(|n| u64::try_from(n).ok())
            .ok_or_else(|| invalid("sequence buffer allocation exceeds address space"))
    }
    pub(crate) fn footprint(
        self,
        parameters: usize,
    ) -> MResult<mech_core::snapshot::ValueFootprint> {
        let metadata = (core::mem::size_of::<Value>() + core::mem::size_of::<ValueData>()) as u64;
        let retained = self
            .bytes()?
            .checked_add(metadata)
            .and_then(|n| n.checked_add((parameters as u64).checked_mul(8)?))
            .ok_or_else(|| invalid("sequence buffer footprint exceeds u64"))?;
        Ok(mech_core::snapshot::ValueFootprint {
            encoded_bytes: (self.length as u64)
                .checked_mul(self.width as u64)
                .ok_or_else(|| invalid("sequence buffer encoded length exceeds u64"))?,
            retained_bytes: retained,
            node_count: 3,
        })
    }
}

// The pinned allocation rule uses a 64-byte first allocation, doubling below
// 4 KiB and above 128 KiB, and a 3/2 step in between. Explicit reserve is exact.
pub(crate) fn append_capacity(capacity: usize, needed: usize, width: usize) -> MResult<usize> {
    if needed <= capacity {
        return Ok(capacity);
    }
    let growth = if capacity == 0 {
        Some(64 / width)
    } else if capacity < 4096 / width || capacity > 131072 / width {
        capacity.checked_mul(2)
    } else {
        capacity
            .checked_mul(3)
            .and_then(|n| n.checked_add(1))
            .map(|n| n / 2)
    };
    Ok(needed.max(growth.ok_or_else(|| invalid("sequence append capacity overflows"))?))
}
pub(crate) fn geometry(
    operation: Operation,
    source: Option<&Value>,
    source_width: usize,
    argument: u64,
) -> MResult<Geometry> {
    let (length, capacity) = match source {
        Some(source) => {
            let ValueData::Matrix(matrix) = source.data() else {
                return Err(invalid("sequence buffer requires a matrix"));
            };
            let schemas = source
                .schemas()
                .ok_or_else(|| invalid("sequence input schema missing"))?;
            let SchemaBody::Matrix { dimensions, .. } = schemas
                .get(source.schema())
                .ok_or_else(|| invalid("sequence input schema missing"))?
                .body()
            else {
                return Err(invalid("sequence buffer requires a matrix schema"));
            };
            if dimensions.len() != 2
                || source
                    .shape()
                    .resolve_dimension(&dimensions[0])
                    .map_err(|_| invalid("sequence input row extent"))?
                    != 1
            {
                return Err(invalid("sequence buffer requires exactly one row"));
            }
            (
                matrix.elements().len(),
                matrix
                    .unsigned_buffer_capacity()
                    .ok_or_else(|| invalid("sequence buffer requires packed unsigned elements"))?,
            )
        }
        None if operation == Operation::WithCapacity => (0, 0),
        None => return Err(invalid("sequence buffer source is missing")),
    };
    geometry_for_header(operation, length, capacity, source_width, argument)
}
fn geometry_for_header(
    operation: Operation,
    length: usize,
    capacity: usize,
    source_width: usize,
    argument: u64,
) -> MResult<Geometry> {
    let result = match operation {
        Operation::WithCapacity => Geometry {
            length: 0,
            capacity: usize::try_from(argument)
                .map_err(|_| invalid("sequence capacity exceeds address space"))?,
            width: source_width,
        },
        Operation::Reserve => Geometry {
            length,
            capacity: capacity.max(
                usize::try_from((length as u64).saturating_add(argument)).unwrap_or(usize::MAX),
            ),
            width: source_width,
        },
        Operation::Append => {
            let next = length
                .checked_add(1)
                .ok_or_else(|| invalid("sequence append length overflows"))?;
            Geometry {
                length: next,
                capacity: append_capacity(capacity, next, source_width)?,
                width: source_width,
            }
        }
        Operation::Clear => Geometry {
            length: 0,
            capacity,
            width: source_width,
        },
    };
    result.bytes()?;
    // Vec element allocations also require a layout no larger than isize::MAX.
    if result.bytes()? > isize::MAX as u64 {
        return Err(invalid("sequence buffer layout exceeds isize"));
    }
    Ok(result)
}
pub(crate) fn scalar_word(value: &ValueData) -> MResult<u64> {
    match value {
        ValueData::U8(x) => Ok(u64::from(*x)),
        ValueData::U16(x) => Ok(u64::from(*x)),
        ValueData::U32(x) => Ok(u64::from(*x)),
        ValueData::U64(x) => Ok(*x),
        _ => Err(invalid(
            "sequence buffer argument is not an unsigned scalar",
        )),
    }
}

/// Construct a fresh backing only for an actual allocation/copy path. The
/// caller owns all admission before entering; no capacity-sized zero fill runs.
pub(crate) fn copy_buffer(
    operation: Operation,
    source: Option<&Value>,
    geometry: Geometry,
    argument: u64,
) -> MResult<UnsignedMatrixBuffer> {
    macro_rules! copy {
        ($ty:ty, $variant:ident) => {{
            let mut words = Vec::<$ty>::new();
            words
                .try_reserve_exact(geometry.capacity)
                .map_err(|_| invalid("sequence buffer allocation failed"))?;
            if words.capacity() != geometry.capacity {
                return Err(invalid(
                    "sequence buffer allocation exceeds admitted capacity",
                ));
            }
            if operation != Operation::Clear {
                if let Some(source) = source {
                    let ValueData::Matrix(matrix) = source.data() else {
                        return Err(invalid("sequence source is not a matrix"));
                    };
                    let SequenceView::$variant(values) = matrix.elements() else {
                        return Err(invalid("sequence source kind changed"));
                    };
                    words.extend_from_slice(values);
                }
            }
            if operation == Operation::Append {
                words.push(
                    <$ty>::try_from(argument)
                        .map_err(|_| invalid("sequence element exceeds its width"))?,
                );
            }
            if words.len() != geometry.length {
                return Err(invalid("sequence constructed length changed"));
            }
            UnsignedMatrixBuffer::$variant(words)
        }};
    }
    Ok(match geometry.width {
        1 => copy!(u8, U8),
        2 => copy!(u16, U16),
        4 => copy!(u32, U32),
        8 => copy!(u64, U64),
        _ => return Err(invalid("sequence buffer width")),
    })
}

#[derive(Debug)]
struct BufferRuntime {
    operation: Operation,
    inputs: Vec<ValueCell>,
    output: ValueCell,
    width: usize,
}
impl BufferRuntime {
    fn new(operation: Operation, invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        validate(
            operation,
            invocation.output_cell(),
            invocation.input_cells(),
        )?;
        let body = invocation.output_cell().closed_schema_body()?;
        let width =
            width(element(&body).ok_or_else(|| invalid("sequence output must be a packed row"))?)
                .unwrap();
        Ok(Box::new(Self {
            operation,
            inputs: invocation.input_cells().to_vec(),
            output: invocation.output_cell().clone(),
            width,
        }))
    }
    fn geometry(&self) -> MResult<Geometry> {
        let argument = if self.operation.arity() == 2 {
            scalar_word(self.inputs[1].snapshot()?.data())?
        } else {
            0
        };
        if self.operation == Operation::WithCapacity {
            return geometry(self.operation, None, self.width, argument);
        }
        if self.inputs[0].requires_canonical_output_builder()? {
            // This clones only the frozen owner, without materializing elements.
            return geometry(
                self.operation,
                Some(&self.inputs[0].snapshot()?),
                self.width,
                argument,
            );
        }
        // Typed input cells have no spare snapshot capacity. Planning reads the
        // descriptor rather than allocating a canonical copy before admission.
        let extents = self.inputs[0]
            .resolved_descriptor()?
            .current_extents()
            .map_err(|_| invalid("sequence input geometry"))?;
        if extents.len() != 2 || extents[0] != 1 {
            return Err(invalid("sequence buffer requires exactly one row"));
        }
        let length = usize::try_from(
            *extents
                .get(1)
                .ok_or_else(|| invalid("sequence input is not a row"))?,
        )
        .map_err(|_| invalid("sequence row length exceeds address space"))?;
        geometry_for_header(self.operation, length, length, self.width, argument)
    }
    fn footprint(&self) -> MResult<CurrentMemoryFootprint> {
        let geometry = self.geometry()?;
        let footprint = geometry.footprint(self.output.shape().parameter_values().len())?;
        Ok(CurrentMemoryFootprint {
            logical_elements: geometry.length as u64,
            payload_bytes: footprint.retained_bytes,
            encoded_bytes: footprint.encoded_bytes,
            retained_nodes: footprint.node_count,
            schema_bytes: self
                .output
                .retained_schema_table()
                .clone_allocation_bound_bytes()
                .ok_or_else(|| invalid("sequence schema footprint exceeds u64"))?,
            shape_parameter_count: self.output.shape().parameter_values().len() as u64,
            ..CurrentMemoryFootprint::default()
        })
    }
}
fn validate(operation: Operation, output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
    let bodies = inputs
        .iter()
        .map(ValueCell::closed_schema_body)
        .collect::<MResult<Vec<_>>>()?;
    if !schemas_supported(operation, &bodies, &output.closed_schema_body()?)
        || !output.requires_canonical_output_builder()?
    {
        return Err(invalid("sequence buffer source/output schema mismatch"));
    }
    Ok(())
}
impl MechFunctionImpl for BufferRuntime {
    fn planned_output_shapes(&self) -> MResult<Option<Box<[ShapeInstance]>>> {
        let n = self.geometry()?.length;
        let table = self.output.retained_schema_table();
        let schema = table
            .get(self.output.schema())
            .ok_or_else(|| invalid("sequence output schema missing"))?;
        Ok(Some(
            vec![shape_for_resolved_extents(schema, &[1, n as u64])?].into_boxed_slice(),
        ))
    }
    fn planned_output_footprints(&self) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
        Ok(Some(vec![self.footprint()?].into_boxed_slice()))
    }
    fn planned_execution_demand(
        &self,
        request: &ExecutionDemandPlanningRequest<'_>,
    ) -> MResult<PlannedExecutionDemand> {
        let geometry = self.geometry()?;
        let mut bytes = geometry
            .bytes()?
            .checked_add(4096)
            .ok_or_else(|| invalid("sequence workspace exceeds u64"))?;
        let mut nodes = 8u64;
        for index in 0..self.inputs.len() {
            let input = request
                .input_footprint(index)
                .map_err(|_| invalid("sequence input footprint unavailable"))?;
            bytes = bytes
                .checked_add(
                    input
                        .payload_bytes
                        .checked_mul(3)
                        .ok_or_else(|| invalid("sequence input workspace overflow"))?,
                )
                .and_then(|n| n.checked_add(input.schema_bytes.checked_mul(3)?))
                .ok_or_else(|| invalid("sequence workspace overflow"))?;
            nodes = nodes
                .checked_add(
                    input
                        .retained_nodes
                        .checked_mul(3)
                        .ok_or_else(|| invalid("sequence nodes overflow"))?,
                )
                .ok_or_else(|| invalid("sequence nodes overflow"))?;
        }
        Ok(PlannedExecutionDemand {
            temporary_bytes: bytes,
            cloned_bytes: bytes,
            retained_nodes: nodes,
            work: WorkDemand {
                compute: bytes
                    .checked_mul(16)
                    .ok_or_else(|| invalid("sequence work overflow"))?,
                ..WorkDemand::default()
            },
        })
    }
    fn solve_managed(
        &self,
        frame: &mut KernelMemoryFrame<'_>,
        _: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveSolveStatus> {
        let footprint = self.footprint()?;
        frame.with_admitted_canonical_output(&self.output, footprint, |frame, construction| {
            let mut inputs = construction.try_vec_with_capacity(self.inputs.len())?;
            for (index, input) in self.inputs.iter().enumerate() {
                inputs.push(frame.snapshot_input_cell_with_construction(
                    input,
                    index,
                    construction,
                )?);
            }
            let source = (self.operation != Operation::WithCapacity).then(|| &inputs[0]);
            let argument = if inputs.len() == 2 {
                scalar_word(inputs[1].data())?
            } else {
                0
            };
            let geometry = geometry(self.operation, source, self.width, argument)?;
            let shape = shape_for_resolved_extents(
                self.output
                    .retained_schema_table()
                    .get(self.output.schema())
                    .unwrap(),
                &[1, geometry.length as u64],
            )?;
            let value = construction.try_build_canonical_candidate_with(|_| {
                let schemas =
                    std::sync::Arc::new(self.output.retained_schema_table().as_ref().clone());
                Value::from_unsigned_matrix_buffer(
                    self.output.schema(),
                    shape,
                    copy_buffer(self.operation, source, geometry, argument)?,
                    schemas,
                )
                .map_err(|_| invalid("sequence output construction rejected"))
            })?;
            Ok(((), value))
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
impl MechFunctionCompiler for BufferRuntime {
    fn compiler_owned_value_cells(&self) -> Vec<ValueCell> {
        self.inputs
            .iter()
            .cloned()
            .chain([self.output.clone()])
            .collect()
    }
    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let output = compile_value_cell_register(&self.output, context)?;
        let function = context.function_id(self.operation.name())?;
        let first = compile_value_cell_register(&self.inputs[0], context)?;
        if self.inputs.len() == 1 {
            context.emit_unop(function, output, first);
        } else {
            let second = compile_value_cell_register(&self.inputs[1], context)?;
            context.emit_binop(function, output, first, second);
        }
        Ok(output)
    }
}
// Arity stays fixed in each generated factory rather than being dispatched at runtime.
macro_rules! define_factory {
    ($factory:ident, $op:ident, $register:ident, $install:ident, $name:literal, $signature:expr, $validator:ident) => {
        fn $validator(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> { validate(Operation::$op, output, inputs) }
        struct $factory;
        impl MechFunctionFactory for $factory {
            const SIGNATURE: RuntimeFunctionSignature = $signature;
            fn implementation_memory_class() -> ImplementationMemoryClass { ImplementationMemoryClass::CanonicalFinalize }
            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> { BufferRuntime::new(Operation::$op, invocation) }
            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> { Some(Operation::$op.contract()) }
        }
        mech_core::declare_native_runtime_factory! {
            cfg: feature = "sequence_buffers", registration: $register, installer: $install,
            name: $name, factory_type: $factory,
            contract: RuntimeFunctionContract::canonical_custom($name, RuntimeOutputAliasPolicy::DisallowInputAlias, $validator),
            operations: [OperationId::from_name($name)], package: "mech-engine", crate_name: "mech_engine",
            installer_path: concat!("mech_engine::__mech_native::", stringify!($install)), extra_cargo_features: ["sequence_buffers"],
        }
    }
}
define_factory!(
    WithCapacityFactory,
    WithCapacity,
    register_with_capacity,
    install_sequence_with_capacity,
    "sequence/with-capacity",
    RuntimeFunctionSignature::binary(
        FunctionValueRepresentation::AnyValue,
        FunctionValueRepresentation::AnyValue,
        FunctionValueRepresentation::AnyValue
    ),
    validate_with_capacity
);
define_factory!(
    ReserveFactory,
    Reserve,
    register_reserve,
    install_sequence_reserve,
    "sequence/reserve",
    RuntimeFunctionSignature::binary(
        FunctionValueRepresentation::AnyValue,
        FunctionValueRepresentation::AnyValue,
        FunctionValueRepresentation::AnyValue
    ),
    validate_reserve
);
define_factory!(
    AppendFactory,
    Append,
    register_append,
    install_sequence_append,
    "sequence/append",
    RuntimeFunctionSignature::binary(
        FunctionValueRepresentation::AnyValue,
        FunctionValueRepresentation::AnyValue,
        FunctionValueRepresentation::AnyValue
    ),
    validate_append
);
define_factory!(
    ClearFactory,
    Clear,
    register_clear,
    install_sequence_clear,
    "sequence/clear",
    RuntimeFunctionSignature::unary(
        FunctionValueRepresentation::AnyValue,
        FunctionValueRepresentation::AnyValue
    ),
    validate_clear
);
pub(crate) fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    register_with_capacity(builder)?;
    register_reserve(builder)?;
    register_append(builder)?;
    register_clear(builder)
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
        let inputs = invocation.inputs().iter().collect::<Vec<_>>();
        let n = if self.0 == Operation::WithCapacity || self.0 == Operation::Clear {
            0
        } else {
            let extents = inputs[0]
                .cell()?
                .resolved_descriptor()?
                .current_extents()
                .map_err(|_| invalid("sequence row geometry"))?;
            let n = *extents
                .get(1)
                .ok_or_else(|| invalid("sequence row geometry"))?;
            if self.0 == Operation::Append {
                n.checked_add(1)
                    .ok_or_else(|| invalid("sequence length overflows"))?
            } else {
                n
            }
        };
        context.bind_resolved_runtime(
            RuntimeBindingSelector::Operation(OperationId::from_name(self.0.name())),
            ExecutionTarget::DirectRuntime,
            vec![vec![1, n].into_boxed_slice()].into_boxed_slice(),
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
            std::sync::Arc::new(SourceOperation(operation)),
        )?;
        builder.insert_export(FunctionExport {
            operation: id,
            canonical_name: name.into(),
            module: Some("sequence".into()),
            item: Some(operation.item().into()),
            exposure: FunctionExposure::ModuleOnly,
        })?;
    }
    Ok(())
}
