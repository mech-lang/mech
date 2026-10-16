//! Ordinary canonical String-count maps. No packed or alphabet-specific representation.
use crate::*;
#[cfg(feature = "resident-artifact")]
use mech_core::snapshot::ValueFootprint;
use mech_core::snapshot::{MapEntryDraft, SequenceView};
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Operation {
    Histogram,
    Keys,
    Values,
}
impl Operation {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Histogram => "map/histogram",
            Self::Keys => "map/keys",
            Self::Values => "map/values",
        }
    }
    pub(crate) fn memory(self) -> ImplementationMemoryClass {
        if self == Self::Histogram {
            ImplementationMemoryClass::CanonicalSortUnique
        } else {
            ImplementationMemoryClass::CanonicalFinalize
        }
    }
    pub(crate) fn contract(self) -> &'static OperationContractDeclaration {
        match self {
            Self::Histogram => &HISTOGRAM,
            Self::Keys => &KEYS,
            Self::Values => &VALUES,
        }
    }
}
static HISTOGRAM: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| mech_core::maintained_operation_contract("map/histogram", 1, false).unwrap());
static KEYS: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| mech_core::maintained_operation_contract("map/keys", 1, true).unwrap());
static VALUES: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| mech_core::maintained_operation_contract("map/values", 1, true).unwrap());

pub(crate) fn is_count_map(body: &SchemaBody) -> bool {
    matches!(body, SchemaBody::Map { key, value, .. }
        if key.as_ref() == &SchemaBody::String && value.as_ref() == &SchemaBody::UnsignedInteger(IntegerWidth::W64))
}
pub(crate) fn is_string_matrix(body: &SchemaBody) -> bool {
    matches!(body, SchemaBody::Matrix { element, dimensions }
        if element.as_ref() == &SchemaBody::String && dimensions.len() == 2)
}

/// Bottom-up merge sorting has a bounded number of levels. At each level a
/// comparison scans at most the bytes of the element it removes from a run.
/// Both index buffers must be admitted before calling this allocation-free helper.
pub(crate) fn sort_positions<'a>(
    order: &mut [usize],
    scratch: &mut [usize],
    key: impl Fn(usize) -> &'a str,
) {
    assert_eq!(order.len(), scratch.len());
    for (index, slot) in order.iter_mut().enumerate() {
        *slot = index;
    }
    let mut width = 1usize;
    while width < order.len() {
        let mut start = 0;
        while start < order.len() {
            let middle = start.saturating_add(width).min(order.len());
            let end = middle.saturating_add(width).min(order.len());
            let (mut left, mut right) = (start, middle);
            for output in &mut scratch[start..end] {
                if left < middle
                    && (right == end || key(order[left]).as_bytes() <= key(order[right]).as_bytes())
                {
                    *output = order[left];
                    left += 1;
                } else {
                    *output = order[right];
                    right += 1;
                }
            }
            start = end;
        }
        order.copy_from_slice(scratch);
        width = width.saturating_mul(2);
    }
}
pub(crate) fn run_end<'a>(order: &[usize], start: usize, key: impl Fn(usize) -> &'a str) -> usize {
    let mut end = start + 1;
    while end < order.len() && key(order[start]) == key(order[end]) {
        end += 1;
    }
    end
}
pub(crate) fn run_count<'a>(order: &[usize], key: impl Fn(usize) -> &'a str) -> usize {
    let mut count = 0;
    let mut position = 0;
    while position < order.len() {
        position = run_end(order, position, &key);
        count += 1;
    }
    count
}
pub(crate) fn count_run(start: usize, end: usize) -> MResult<u64> {
    (start..end).try_fold(0u64, |count, _| {
        count
            .checked_add(1)
            .ok_or_else(|| invalid("histogram count exceeds u64::MAX"))
    })
}

/// Exact prospective storage for this family's narrow canonical result kinds.
#[cfg(feature = "resident-artifact")]
pub(crate) fn prospective(
    operation: Operation,
    count: u64,
    payload: u64,
    shape_parameters: usize,
) -> MResult<ValueFootprint> {
    let checked = || invalid("map result footprint exceeds u64");
    if operation == Operation::Keys {
        return mech_core::snapshot::prospective_string_value_footprint(
            shape_parameters,
            true,
            count,
            payload,
        )
        .map_err(|_| checked());
    }
    let wrapper = (core::mem::size_of::<Value>() as u64)
        .checked_add(
            (shape_parameters as u64)
                .checked_mul(8)
                .ok_or_else(checked)?,
        )
        .ok_or_else(checked)?;
    let data = core::mem::size_of::<ValueData>() as u64;
    let (retained_bytes, encoded_bytes, node_count) = if operation == Operation::Histogram {
        let entry = (core::mem::size_of::<mech_core::snapshot::MapEntryValue>() as u64)
            .checked_add(data.checked_mul(2).ok_or_else(checked)?)
            .ok_or_else(checked)?;
        (
            wrapper
                .checked_add(data)
                .and_then(|v| v.checked_add(count.checked_mul(entry)?))
                .and_then(|v| v.checked_add(payload)),
            count
                .checked_mul(16)
                .and_then(|v| v.checked_add(payload))
                .and_then(|v| v.checked_add(8)),
            count.checked_mul(2).and_then(|v| v.checked_add(2)),
        )
    } else {
        (
            wrapper
                .checked_add(data)
                .and_then(|v| v.checked_add(count.checked_mul(8)?)),
            count.checked_mul(8),
            Some(3),
        )
    };
    Ok(ValueFootprint {
        retained_bytes: retained_bytes.ok_or_else(checked)?,
        encoded_bytes: encoded_bytes.ok_or_else(checked)?,
        node_count: node_count.ok_or_else(checked)?,
    })
}

fn validate(operation: Operation, output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
    let [input] = inputs else {
        return Err(invalid("map operation requires one input"));
    };
    let source = input.closed_schema_body()?;
    let target = output.closed_schema_body()?;
    let supported = match operation {
        Operation::Histogram => is_string_matrix(&source) && is_count_map(&target),
        Operation::Keys | Operation::Values => {
            is_count_map(&source)
                && matches!(&target, SchemaBody::Matrix { element, dimensions }
            if dimensions.len() == 2 && dimensions[0] == DimensionExpr::Constant(1)
                && element.as_ref() == &if operation == Operation::Keys { SchemaBody::String } else { SchemaBody::UnsignedInteger(IntegerWidth::W64) })
        }
    };
    if !supported || !output.requires_canonical_output_builder()? {
        return Err(invalid(
            "map operation requires its canonical supported input and output schemas",
        ));
    }
    Ok(())
}
fn validate_histogram(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
    validate(Operation::Histogram, output, inputs)
}
fn validate_keys(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
    validate(Operation::Keys, output, inputs)
}
fn validate_values(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
    validate(Operation::Values, output, inputs)
}

#[derive(Debug)]
struct MapRuntime {
    operation: Operation,
    input: ValueCell,
    output: ValueCell,
}
impl MapRuntime {
    fn new(operation: Operation, invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        validate(
            operation,
            invocation.output_cell(),
            invocation.input_cells(),
        )?;
        Ok(Box::new(Self {
            operation,
            input: invocation.input_cells()[0].clone(),
            output: invocation.output_cell().clone(),
        }))
    }
    fn footprint(&self) -> MResult<CurrentMemoryFootprint> {
        // The common aggregate witness includes input reconstruction, draft
        // headers, Map entries and two temporary index arrays without observing
        // or executing an input. Duplicate input keys are an upper bound.
        let input = self.input.current_memory_footprint()?;
        let mut footprint = self
            .output
            .prospective_aggregate_memory_footprint([(&self.input, 6)])?;
        footprint.logical_elements = if self.operation == Operation::Histogram {
            input.logical_elements
        } else {
            self.input.current_map_entry_count()?
        };
        Ok(footprint)
    }
    fn construct(
        &self,
        construction: &mut FrozenSnapshotConstruction,
        source: &Value,
    ) -> MResult<Value> {
        if self.operation == Operation::Histogram {
            let ValueData::Matrix(matrix) = source.data() else {
                return Err(invalid("histogram requires a String matrix"));
            };
            let SequenceView::String(strings) = matrix.elements() else {
                return Err(invalid("histogram requires String elements"));
            };
            let mut order = construction.try_vec_with_capacity(strings.len())?;
            let mut scratch = construction.try_vec_with_capacity(strings.len())?;
            order.resize(strings.len(), 0usize);
            scratch.resize(strings.len(), 0usize);
            let key = |index: usize| -> &str { strings[index].as_ref() };
            sort_positions(&mut order, &mut scratch, key);
            let unique = run_count(&order, key);
            let mut start = 0;
            let entries = construction.try_boxed_slice_with(unique, |construction, _| {
                let end = run_end(&order, start, key);
                let count = count_run(start, end)?;
                let mut text = Some(construction.try_concatenate_string(key(order[start]), "")?);
                start = end;
                Ok(MapEntryDraft {
                    items: construction.try_boxed_slice_with(2, |_, index| {
                        Ok(if index == 0 {
                            ValueDataDraft::String(text.take().expect("one key field"))
                        } else {
                            ValueDataDraft::U64(count)
                        })
                    })?,
                })
            })?;
            return construction.try_rebuild_data_draft(&self.output, ValueDataDraft::Map(entries));
        }
        let ValueData::Map(map) = source.data() else {
            return Err(invalid("projection requires a String/U64 map"));
        };
        let entries = map.entries();
        let values = construction.try_boxed_slice_with(entries.len(), |construction, index| {
            let entry = &entries[index];
            match self.operation {
                Operation::Keys => {
                    let ValueData::String(text) = entry.key().data() else {
                        return Err(invalid("map key must be String"));
                    };
                    Ok(ValueDataDraft::String(
                        construction.try_concatenate_string(text, "")?,
                    ))
                }
                Operation::Values => {
                    let ValueData::U64(value) = entry.value() else {
                        return Err(invalid("map count must be U64"));
                    };
                    Ok(ValueDataDraft::U64(*value))
                }
                Operation::Histogram => unreachable!(),
            }
        })?;
        let count =
            u64::try_from(entries.len()).map_err(|_| invalid("map cardinality exceeds u64"))?;
        let dimensions = construction
            .try_boxed_slice_with(2, |_, index| Ok(if index == 0 { 1 } else { count }))?;
        construction.try_rebuild_matrix_drafts(&self.output, dimensions, values)
    }
}
impl MechFunctionImpl for MapRuntime {
    fn planned_output_shapes(&self) -> MResult<Option<Box<[ShapeInstance]>>> {
        if self.operation == Operation::Histogram {
            return Ok(None);
        }
        let count = self.input.current_map_entry_count()?;
        let schemas = self.output.retained_schema_table();
        let schema = schemas
            .get(self.output.schema())
            .ok_or_else(|| invalid("map projection output has no retained schema"))?;
        Ok(Some(
            vec![shape_for_resolved_extents(schema, &[1, count])?].into_boxed_slice(),
        ))
    }

    fn planned_output_footprints(&self) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
        Ok(Some(vec![self.footprint()?].into_boxed_slice()))
    }
    fn solve_managed(
        &self,
        frame: &mut KernelMemoryFrame<'_>,
        _: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveSolveStatus> {
        frame.with_admitted_canonical_output(
            &self.output,
            self.footprint()?,
            |frame, construction| {
                let source =
                    frame.snapshot_input_cell_with_construction(&self.input, 0, construction)?;
                Ok(((), self.construct(construction, &source)?))
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
    fn to_string(&self) -> String {
        self.operation.name().into()
    }
}
#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for MapRuntime {
    fn compiler_owned_value_cells(&self) -> Vec<ValueCell> {
        vec![self.input.clone(), self.output.clone()]
    }
    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let output = compile_value_cell_register(&self.output, context)?;
        let source = compile_value_cell_register(&self.input, context)?;
        let function = context.function_id(self.operation.name())?;
        context.emit_unop(function, output, source);
        Ok(output)
    }
}
macro_rules! factory {
    ($name:ident, $operation:ident) => {
        struct $name;
        impl MechFunctionFactory for $name {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::unary(
                FunctionValueRepresentation::AnyValue,
                FunctionValueRepresentation::AnyValue,
            );
            fn implementation_memory_class() -> ImplementationMemoryClass {
                Operation::$operation.memory()
            }
            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                MapRuntime::new(Operation::$operation, invocation)
            }
            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                Some(Operation::$operation.contract())
            }
        }
    };
}
factory!(HistogramRuntime, Histogram);
factory!(KeysRuntime, Keys);
factory!(ValuesRuntime, Values);
macro_rules! registration {
    ($register:ident, $install:ident, $factory:ty, $name:literal, $validator:ident) => {
        mech_core::declare_native_runtime_factory! {
            cfg: feature = "map_ops", registration: $register, installer: $install,
            name: $name, factory_type: $factory,
            contract: RuntimeFunctionContract::canonical_custom($name, RuntimeOutputAliasPolicy::DisallowInputAlias, $validator),
            operations: [OperationId::from_name($name)],
            package: "mech-engine", crate_name: "mech_engine",
            installer_path: concat!("mech_engine::__mech_native::", stringify!($install)),
            extra_cargo_features: ["map_ops"],
        }
    }
}
registration!(
    register_histogram,
    install_map_histogram,
    HistogramRuntime,
    "map/histogram",
    validate_histogram
);
registration!(
    register_keys,
    install_map_keys,
    KeysRuntime,
    "map/keys",
    validate_keys
);
registration!(
    register_values,
    install_map_values,
    ValuesRuntime,
    "map/values",
    validate_values
);
pub(crate) fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    register_histogram(builder)?;
    register_keys(builder)?;
    register_values(builder)
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
        let [input] = invocation.inputs() else {
            return Err(invalid("map operation requires one input"));
        };
        let extents = if self.0 == Operation::Histogram {
            Box::new([]) as Box<[u64]>
        } else {
            vec![1, input.cell()?.current_map_entry_count()?].into_boxed_slice()
        };
        context.bind_resolved_runtime(
            RuntimeBindingSelector::Operation(OperationId::from_name(self.0.name())),
            ExecutionTarget::DirectRuntime,
            vec![extents].into_boxed_slice(),
            &[input],
        )
    }
}
#[cfg(feature = "semantic-compiler")]
pub(crate) fn install_source(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    for operation in [Operation::Histogram, Operation::Keys, Operation::Values] {
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
            module: Some("map".into()),
            item: Some(name.split_once('/').unwrap().1.into()),
            exposure: FunctionExposure::ModuleOnly,
        })?;
    }
    Ok(())
}
