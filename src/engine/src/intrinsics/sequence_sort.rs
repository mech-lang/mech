//! Ordinary lexicographic sorting of aligned rank-two sequences.
//!
//! Primary keys retain exact U64 identity. Secondary keys use the core
//! canonical String relation; original canonical ordinals break complete
//! ties. Returned positions are one-based and refer to canonical row-major
//! input order independently of either input's physical storage.
use crate::*;
use core::cmp::Ordering;
use mech_core::snapshot::SequenceView;
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

pub(crate) const NAME: &str = "sequence/sort-indices";
static CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| mech_core::maintained_operation_contract(NAME, 4, true).unwrap());
pub(crate) fn contract() -> &'static OperationContractDeclaration {
    &CONTRACT
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Direction {
    Ascending,
    Descending,
}
impl Direction {
    pub(crate) fn parse(value: &str) -> MResult<Self> {
        match value {
            "asc" => Ok(Self::Ascending),
            "desc" => Ok(Self::Descending),
            _ => Err(invalid("sort order must be exactly asc or desc")),
        }
    }
    fn apply(self, order: Ordering) -> Ordering {
        match self {
            Self::Ascending => order,
            Self::Descending => order.reverse(),
        }
    }
}

pub(crate) fn matrix_element(body: &SchemaBody, expected: &SchemaBody) -> bool {
    matches!(body, SchemaBody::Matrix { element, dimensions }
        if element.as_ref() == expected && dimensions.len() == 2)
}

pub(crate) fn canonical_u64s(value: &Value) -> MResult<&[u64]> {
    let ValueData::Matrix(matrix) = value.data() else {
        return Err(invalid("primary sort keys require a rank-two U64 matrix"));
    };
    let SequenceView::U64(values) = matrix.elements() else {
        return Err(invalid("primary sort keys require exact U64 elements"));
    };
    Ok(values)
}

pub(crate) fn canonical_strings(value: &Value) -> MResult<&[Box<str>]> {
    let ValueData::Matrix(matrix) = value.data() else {
        return Err(invalid(
            "secondary sort keys require a rank-two String matrix",
        ));
    };
    let SequenceView::String(values) = matrix.elements() else {
        return Err(invalid("secondary sort keys require String elements"));
    };
    Ok(values)
}

/// Work bound for the explicit binary heap below. Every comparison has two
/// scalar U64 comparisons at most, one canonical String comparison at most,
/// and one ordinal comparison. Each heap level makes at most two comparator
/// calls. The bound includes heap construction, extraction, metadata passes,
/// key population, and conversion to one-based result positions.
pub(crate) fn sorting_work(count: usize, maximum_key_bytes: usize) -> MResult<u64> {
    let n = u64::try_from(count).map_err(|_| invalid("sort length exceeds u64"))?;
    let bytes = u64::try_from(maximum_key_bytes)
        .map_err(|_| invalid("String key length exceeds u64"))?
        .max(1);
    let levels = u64::from(count.max(1).ilog2()) + 1;
    n.checked_mul(levels)
        .and_then(|value| value.checked_mul(8))
        .and_then(|value| value.checked_add(n.checked_mul(8)?))
        .and_then(|value| value.checked_mul(bytes.checked_add(4)?))
        .ok_or_else(|| invalid("sort comparison work exceeds u64"))
}

/// Allocation-free ordering over preadmitted index and canonical-key storage.
/// This function never constructs or clones String data while comparing keys.
pub(crate) fn sort_positions(
    positions: &mut [usize],
    primary: &[u64],
    primary_order: Direction,
    secondary: &[ValueData],
    secondary_order: Direction,
) -> MResult<()> {
    if positions.len() != primary.len()
        || primary.len() != secondary.len()
        || secondary
            .iter()
            .any(|key| !matches!(key, ValueData::String(_)))
    {
        return Err(invalid(
            "sort key sequences must have equal lengths and supported kinds",
        ));
    }
    let compare = |left: usize, right: usize| -> MResult<Ordering> {
        let first = mech_core::snapshot::compare_key_data(
            &SchemaBody::UnsignedInteger(IntegerWidth::W64),
            &ValueData::U64(primary[left]),
            &ValueData::U64(primary[right]),
        )
        .map_err(|_| invalid("primary sort key is not a canonical U64"))?;
        if !first.is_eq() {
            return Ok(primary_order.apply(first));
        }
        let second = mech_core::snapshot::compare_key_data(
            &SchemaBody::String,
            &secondary[left],
            &secondary[right],
        )
        .map_err(|_| invalid("secondary sort key is not canonical String"))?;
        Ok(secondary_order.apply(second).then_with(|| left.cmp(&right)))
    };
    fn sift_down(
        values: &mut [usize],
        mut root: usize,
        end: usize,
        compare: &impl Fn(usize, usize) -> MResult<Ordering>,
    ) -> MResult<()> {
        loop {
            let Some(mut child) = root.checked_mul(2).and_then(|value| value.checked_add(1)) else {
                return Ok(());
            };
            if child >= end {
                return Ok(());
            }
            if child + 1 < end && compare(values[child], values[child + 1])?.is_lt() {
                child += 1;
            }
            if !compare(values[root], values[child])?.is_lt() {
                return Ok(());
            }
            values.swap(root, child);
            root = child;
        }
    }
    for (ordinal, position) in positions.iter_mut().enumerate() {
        *position = ordinal;
    }
    for root in (0..positions.len() / 2).rev() {
        let end = positions.len();
        sift_down(positions, root, end, &compare)?;
    }
    for end in (1..positions.len()).rev() {
        positions.swap(0, end);
        sift_down(positions, 0, end, &compare)?;
    }
    Ok(())
}

pub(crate) fn index_footprint(
    count: u64,
    shape_parameters: usize,
) -> MResult<mech_core::snapshot::ValueFootprint> {
    let shape = u64::try_from(shape_parameters)
        .ok()
        .and_then(|count| count.checked_mul(8))
        .ok_or_else(|| invalid("sort result shape metadata exceeds u64"))?;
    let payload = count
        .checked_mul(8)
        .ok_or_else(|| invalid("sort result exceeds u64"))?;
    Ok(mech_core::snapshot::ValueFootprint {
        encoded_bytes: payload,
        retained_bytes: (core::mem::size_of::<Value>() as u64)
            .checked_add(core::mem::size_of::<ValueData>() as u64)
            .and_then(|bytes| bytes.checked_add(shape))
            .and_then(|bytes| bytes.checked_add(payload))
            .ok_or_else(|| invalid("sort result footprint exceeds u64"))?,
        node_count: 3,
    })
}

fn element_count(cell: &ValueCell) -> MResult<u64> {
    let descriptor = cell.resolved_descriptor()?;
    let extents = descriptor.current_extents()?;
    let [rows, columns] = extents.as_ref() else {
        return Err(invalid("sort keys require rank-two matrices"));
    };
    rows.checked_mul(*columns)
        .ok_or_else(|| invalid("sort length exceeds u64"))
}

fn validate(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
    let [primary, first_order, secondary, second_order] = inputs else {
        return Err(invalid("sequence/sort-indices requires four inputs"));
    };
    let primary_schema = primary.closed_schema_body()?;
    let secondary_schema = secondary.closed_schema_body()?;
    let output_schema = output.closed_schema_body()?;
    if !matrix_element(
        &primary_schema,
        &SchemaBody::UnsignedInteger(IntegerWidth::W64),
    ) || !matrix_element(&secondary_schema, &SchemaBody::String)
        || first_order.closed_schema_body()? != SchemaBody::String
        || second_order.closed_schema_body()? != SchemaBody::String
        || !matches!(&output_schema, SchemaBody::Matrix { element, dimensions }
            if element.as_ref() == &SchemaBody::Index && dimensions.len() == 2
                && dimensions[0] == DimensionExpr::Constant(1))
        || !output.requires_canonical_output_builder()?
    {
        return Err(invalid(
            "sort-indices requires U64/String matrices, String orders and a canonical Index row result",
        ));
    }
    if element_count(primary)? != element_count(secondary)? {
        return Err(invalid("sort key sequences must have equal lengths"));
    }
    Ok(())
}

#[derive(Debug)]
struct SortIndicesRuntime {
    inputs: [ValueCell; 4],
    output: ValueCell,
}

impl SortIndicesRuntime {
    fn new(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        validate(invocation.output_cell(), invocation.input_cells())?;
        let inputs: [ValueCell; 4] = invocation
            .input_cells()
            .to_vec()
            .try_into()
            .map_err(|_| invalid("sort-indices requires four inputs"))?;
        Ok(Box::new(Self {
            inputs,
            output: invocation.output_cell().clone(),
        }))
    }

    fn footprint_for_count(&self, n: u64) -> MResult<CurrentMemoryFootprint> {
        let shape_parameters = self.output.shape().parameter_values().len();
        let footprint = index_footprint(n, shape_parameters)?;
        Ok(CurrentMemoryFootprint {
            logical_elements: n,
            payload_bytes: footprint.retained_bytes,
            encoded_bytes: footprint.encoded_bytes,
            retained_nodes: footprint.node_count,
            schema_bytes: self
                .output
                .retained_schema_table()
                .clone_allocation_bound_bytes()
                .ok_or_else(|| invalid("sort output schema allocation exceeds u64"))?,
            shape_parameter_count: shape_parameters as u64,
            ..CurrentMemoryFootprint::default()
        })
    }

    fn construct(
        &self,
        frame: &KernelMemoryFrame<'_>,
        construction: &mut FrozenSnapshotConstruction,
    ) -> MResult<Value> {
        let primary =
            frame.snapshot_input_cell_with_construction(&self.inputs[0], 0, construction)?;
        let first_order =
            frame.snapshot_input_cell_with_construction(&self.inputs[1], 1, construction)?;
        let secondary =
            frame.snapshot_input_cell_with_construction(&self.inputs[2], 2, construction)?;
        let second_order =
            frame.snapshot_input_cell_with_construction(&self.inputs[3], 3, construction)?;
        let primary = canonical_u64s(&primary)?;
        let strings = canonical_strings(&secondary)?;
        let (ValueData::String(first_order), ValueData::String(second_order)) =
            (first_order.data(), second_order.data())
        else {
            return Err(invalid("sort directions require scalar String values"));
        };
        let first_order = Direction::parse(first_order)?;
        let second_order = Direction::parse(second_order)?;
        if primary.len() != strings.len() {
            return Err(invalid("sort key sequences must have equal lengths"));
        }
        let keys = construction.try_boxed_slice_with(strings.len(), |construction, index| {
            Ok(ValueData::String(
                construction
                    .try_concatenate_string(&strings[index], "")?
                    .into_boxed_str(),
            ))
        })?;
        let mut positions = construction.try_vec_with_capacity(primary.len())?;
        positions.resize(primary.len(), 0usize);
        sort_positions(&mut positions, primary, first_order, &keys, second_order)?;
        let values = construction.try_boxed_slice_with(positions.len(), |_, index| {
            let one_based = u64::try_from(positions[index])
                .ok()
                .and_then(|value| value.checked_add(1))
                .ok_or_else(|| invalid("sort position exceeds one-based Index range"))?;
            Ok(ValueDataDraft::Index(one_based))
        })?;
        let n = u64::try_from(positions.len()).map_err(|_| invalid("sort length exceeds u64"))?;
        let dimensions =
            construction.try_boxed_slice_with(2, |_, index| Ok(if index == 0 { 1 } else { n }))?;
        construction.try_rebuild_matrix_drafts(&self.output, dimensions, values)
    }
}

impl MechFunctionImpl for SortIndicesRuntime {
    fn planned_output_shapes(&self) -> MResult<Option<Box<[ShapeInstance]>>> {
        let n = element_count(&self.inputs[0])?;
        if n != element_count(&self.inputs[2])? {
            return Err(invalid("sort key sequences must have equal lengths"));
        }
        let schemas = self.output.retained_schema_table();
        let schema = schemas
            .get(self.output.schema())
            .ok_or_else(|| invalid("sort output schema is unavailable"))?;
        Ok(Some(
            vec![shape_for_resolved_extents(schema, &[1, n])?].into_boxed_slice(),
        ))
    }

    fn planned_output_footprints(&self) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
        Ok(Some(
            vec![self.footprint_for_count(element_count(&self.inputs[0])?)?].into_boxed_slice(),
        ))
    }

    fn planned_output_footprints_with_request(
        &self,
        request: &ExecutionDemandPlanningRequest<'_>,
    ) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
        let n = request
            .input_footprint(0)
            .map_err(|error| MechError::new(error, None).with_compiler_loc())?
            .logical_elements;
        if n != request
            .input_footprint(2)
            .map_err(|error| MechError::new(error, None).with_compiler_loc())?
            .logical_elements
        {
            return Err(invalid("sort key sequences must have equal lengths"));
        }
        Ok(Some(vec![self.footprint_for_count(n)?].into_boxed_slice()))
    }

    fn planned_execution_demand(
        &self,
        request: &ExecutionDemandPlanningRequest<'_>,
    ) -> MResult<PlannedExecutionDemand> {
        let primary = request
            .input_footprint(0)
            .map_err(|error| MechError::new(error, None).with_compiler_loc())?;
        let secondary = request
            .input_footprint(2)
            .map_err(|error| MechError::new(error, None).with_compiler_loc())?;
        let n = primary.logical_elements;
        if n != secondary.logical_elements {
            return Err(invalid("sort key sequences must have equal lengths"));
        }
        // The canonical encoded footprint bounds every individual UTF-8 key.
        // This uses the call planner's already resolved metadata; it never
        // walks String payloads just to discover a work allowance.
        let work = sorting_work(
            usize::try_from(n).map_err(|_| invalid("sort length exceeds usize"))?,
            usize::try_from(secondary.encoded_bytes)
                .map_err(|_| invalid("sort key work exceeds usize"))?,
        )?;
        let mut scratch = 0u64;
        let mut cloned = 0u64;
        let mut nodes = n
            .checked_add(4)
            .ok_or_else(|| invalid("sort node count exceeds u64"))?;
        let mut setup_work = 0u64;
        for index in 0..4 {
            let input = request
                .input_footprint(index)
                .map_err(|error| MechError::new(error, None).with_compiler_loc())?;
            let payload = input
                .payload_bytes
                .checked_add(input.fixed_bytes)
                .ok_or_else(|| invalid("sort input payload exceeds u64"))?;
            // Input reconstruction, its draft/frozen overlap, schema owner,
            // and the secondary canonical-key population coexist in the call.
            let input_scratch = payload
                .checked_mul(3)
                .and_then(|bytes| bytes.checked_add(input.encoded_bytes.checked_mul(2)?))
                .and_then(|bytes| bytes.checked_add(input.schema_bytes.checked_mul(3)?))
                .and_then(|bytes| {
                    bytes.checked_add(
                        input
                            .logical_elements
                            .checked_mul(core::mem::size_of::<ValueDataDraft>() as u64)?,
                    )
                })
                .ok_or_else(|| invalid("sort reconstruction workspace exceeds u64"))?;
            scratch = scratch
                .checked_add(input_scratch)
                .ok_or_else(|| invalid("sort scratch exceeds u64"))?;
            cloned = cloned
                .checked_add(
                    payload
                        .checked_mul(2)
                        .ok_or_else(|| invalid("sort clone count exceeds u64"))?,
                )
                .ok_or_else(|| invalid("sort clone count exceeds u64"))?;
            nodes = nodes
                .checked_add(
                    input
                        .retained_nodes
                        .checked_mul(3)
                        .ok_or_else(|| invalid("sort node count exceeds u64"))?,
                )
                .ok_or_else(|| invalid("sort node count exceeds u64"))?;
            setup_work = setup_work
                .checked_add(input.encoded_bytes)
                .and_then(|value| value.checked_add(payload))
                .and_then(|value| value.checked_add(input.logical_elements))
                .ok_or_else(|| invalid("sort setup work exceeds u64"))?;
        }
        scratch = scratch
            .checked_add(
                n.checked_mul(
                    (core::mem::size_of::<usize>() + core::mem::size_of::<ValueData>()) as u64,
                )
                .ok_or_else(|| invalid("sort index workspace exceeds u64"))?,
            )
            .ok_or_else(|| invalid("sort scratch exceeds u64"))?;
        Ok(PlannedExecutionDemand {
            temporary_bytes: scratch,
            cloned_bytes: cloned,
            retained_nodes: nodes,
            work: WorkDemand {
                comparison: work,
                compute: work
                    .checked_add(setup_work)
                    .ok_or_else(|| invalid("sort work exceeds u64"))?,
                ..WorkDemand::default()
            },
        })
    }

    fn solve_managed(
        &self,
        frame: &mut KernelMemoryFrame<'_>,
        _: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveSolveStatus> {
        let descriptor = frame.input_cell_descriptor(&self.inputs[0], 0)?;
        let extents = descriptor.current_extents()?;
        let [rows, columns] = extents.as_ref() else {
            return Err(invalid("sort keys require rank-two matrices"));
        };
        let count = rows
            .checked_mul(*columns)
            .ok_or_else(|| invalid("sort length exceeds u64"))?;
        frame.with_admitted_canonical_output(
            &self.output,
            self.footprint_for_count(count)?,
            |frame, construction| Ok(((), self.construct(frame, construction)?)),
        )?;
        Ok(ReactiveSolveStatus::Changed)
    }
    fn semantic_operation_name(&self) -> Option<&str> {
        Some(NAME)
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(contract())
    }
    fn to_string(&self) -> String {
        NAME.into()
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for SortIndicesRuntime {
    fn compiler_owned_value_cells(&self) -> Vec<ValueCell> {
        self.inputs
            .iter()
            .cloned()
            .chain([self.output.clone()])
            .collect()
    }
    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let output = compile_value_cell_register(&self.output, context)?;
        let inputs = self
            .inputs
            .iter()
            .map(|cell| compile_value_cell_register(cell, context))
            .collect::<MResult<Vec<_>>>()?;
        let function = context.function_id(NAME)?;
        context.emit_quadop(function, output, inputs[0], inputs[1], inputs[2], inputs[3]);
        Ok(output)
    }
}

struct SortIndicesFactory;
impl MechFunctionFactory for SortIndicesFactory {
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::quaternary(
        FunctionValueRepresentation::AnyValue,
        FunctionValueRepresentation::AnyValue,
        FunctionValueRepresentation::String,
        FunctionValueRepresentation::AnyValue,
        FunctionValueRepresentation::String,
    );
    fn implementation_memory_class() -> ImplementationMemoryClass {
        ImplementationMemoryClass::CanonicalFinalize
    }
    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        SortIndicesRuntime::new(invocation)
    }
    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(contract())
    }
}

mech_core::declare_native_runtime_factory! {
    cfg: feature = "sequence_sort", registration: register_sort_indices, installer: install_sequence_sort_indices,
    name: "sequence/sort-indices", factory_type: SortIndicesFactory,
    contract: RuntimeFunctionContract::canonical_custom("sequence/sort-indices", RuntimeOutputAliasPolicy::DisallowInputAlias, validate),
    operations: [OperationId::from_name("sequence/sort-indices")],
    package: "mech-engine", crate_name: "mech_engine",
    installer_path: "mech_engine::__mech_native::install_sequence_sort_indices",
    extra_cargo_features: ["sequence_sort"],
}

pub(crate) fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    register_sort_indices(builder)
}

#[cfg(feature = "semantic-compiler")]
struct SourceOperation;
#[cfg(feature = "semantic-compiler")]
impl CanonicalFunctionSpecializer for SourceOperation {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        let [primary, first, secondary, second] = invocation.inputs() else {
            return Err(invalid("sort-indices requires four inputs"));
        };
        let n = element_count(primary.cell()?)?;
        if n != element_count(secondary.cell()?)? {
            return Err(invalid("sort key sequences must have equal lengths"));
        }
        context.bind_resolved_runtime(
            RuntimeBindingSelector::Operation(OperationId::from_name(NAME)),
            ExecutionTarget::DirectRuntime,
            vec![vec![1, n].into_boxed_slice()].into_boxed_slice(),
            &[primary, first, secondary, second],
        )
    }
}
#[cfg(feature = "semantic-compiler")]
pub(crate) fn install_source(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    let id = builder.insert_canonical_specializer_with_contracts(
        NAME,
        mech_core::maintained_source_type_declaration(NAME)?,
        vec![contract().clone()],
        std::sync::Arc::new(SourceOperation),
    )?;
    builder.insert_export(FunctionExport {
        operation: id,
        canonical_name: NAME.into(),
        module: Some("sequence".into()),
        item: Some("sort-indices".into()),
        exposure: FunctionExposure::ModuleOnly,
    })
}

#[cfg(test)]
mod ordering_tests {
    use super::*;
    fn indices(
        primary: &[u64],
        secondary: &[&str],
        first: Direction,
        second: Direction,
    ) -> Vec<usize> {
        let secondary = secondary
            .iter()
            .map(|value| ValueData::String((*value).into()))
            .collect::<Vec<_>>();
        let mut result = vec![0; primary.len()];
        sort_positions(&mut result, primary, first, &secondary, second).unwrap();
        result.into_iter().map(|value| value + 1).collect()
    }
    #[test]
    fn exact_u64_primary_and_string_secondary_preserve_ordinal_ties() {
        assert_eq!(
            indices(
                &[
                    u64::MAX,
                    9_007_199_254_740_992,
                    9_007_199_254_740_993,
                    u64::MAX,
                    u64::MAX,
                    0
                ],
                &["é", "x", "x", "A", "A", ""],
                Direction::Descending,
                Direction::Ascending,
            ),
            [4, 5, 1, 3, 2, 6]
        );
    }
    #[test]
    fn directions_are_independent_and_empty_is_valid() {
        assert_eq!(
            indices(
                &[1, 2, 1, 2],
                &["a", "b", "z", "a"],
                Direction::Ascending,
                Direction::Descending
            ),
            [3, 1, 2, 4]
        );
        assert!(indices(&[], &[], Direction::Descending, Direction::Ascending).is_empty());
        for invalid in ["", "ascending", "DESC", "desc\n", " asc"] {
            assert!(Direction::parse(invalid).is_err());
        }
    }
    #[test]
    fn canonical_utf8_keys_remain_distinct_without_normalization() {
        assert_eq!(
            indices(
                &[1, 1, 1, 1],
                &["é", "e\u{301}", "🦀", "A"],
                Direction::Ascending,
                Direction::Ascending
            ),
            [4, 2, 1, 3]
        );
    }
}
