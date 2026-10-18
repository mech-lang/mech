//! Direct managed String selection. Planning observes metadata only; grapheme
//! boundaries and selector values are read through admitted call-frame leases.
use super::{CanonicalAccess, CanonicalAccessSelector};
use crate::text_index::PreparedGraphemeIndex;
use crate::*;
use mech_core::snapshot::{F32Bits, F64Bits, SequenceView, prospective_string_value_footprint};

fn invalid(message: impl Into<String>) -> MechError {
    MechError::new(
        GenericError {
            msg: message.into(),
        },
        None,
    )
    .with_compiler_loc()
}

fn overflow() -> MechError {
    invalid("String selection demand exceeds the target range")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Numeric {
    Index,
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,
    F32,
    F64,
}

fn numeric(body: &SchemaBody) -> MResult<Numeric> {
    if let SchemaBody::IntegerInterval(interval) = body {
        return numeric(&interval.base_body());
    }
    Ok(match body {
        SchemaBody::Index => Numeric::Index,
        SchemaBody::UnsignedInteger(IntegerWidth::W8) => Numeric::U8,
        SchemaBody::UnsignedInteger(IntegerWidth::W16) => Numeric::U16,
        SchemaBody::UnsignedInteger(IntegerWidth::W32) => Numeric::U32,
        SchemaBody::UnsignedInteger(IntegerWidth::W64) => Numeric::U64,
        SchemaBody::UnsignedInteger(IntegerWidth::W128) => Numeric::U128,
        SchemaBody::SignedInteger(IntegerWidth::W8) => Numeric::I8,
        SchemaBody::SignedInteger(IntegerWidth::W16) => Numeric::I16,
        SchemaBody::SignedInteger(IntegerWidth::W32) => Numeric::I32,
        SchemaBody::SignedInteger(IntegerWidth::W64) => Numeric::I64,
        SchemaBody::SignedInteger(IntegerWidth::W128) => Numeric::I128,
        SchemaBody::FloatingPoint(FloatWidth::W32) => Numeric::F32,
        SchemaBody::FloatingPoint(FloatWidth::W64) => Numeric::F64,
        _ => {
            return Err(invalid(
                "invalid positional access selector: UnsupportedSchema",
            ));
        }
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Selection {
    All,
    Numeric(Numeric),
    Bitmap,
}

fn selector_schema(body: &SchemaBody) -> MResult<Selection> {
    match body {
        SchemaBody::Matrix {
            element,
            dimensions,
        } if dimensions.len() == 2 => {
            if element.as_ref() == &SchemaBody::Bool {
                Ok(Selection::Bitmap)
            } else {
                numeric(element).map(Selection::Numeric)
            }
        }
        _ => numeric(body).map(Selection::Numeric),
    }
}

/// A type-correct seed, not a precomputed observation of selector contents.
pub(super) fn empty_seed(selectors: &[CanonicalAccessSelector]) -> MResult<ValueCell> {
    match selectors {
        [CanonicalAccessSelector::All] => {}
        [CanonicalAccessSelector::Cell(cell)] => {
            selector_schema(&cell.closed_schema_body()?)?;
        }
        _ => return Err(invalid("String selection requires exactly one selector")),
    }
    ValueCell::from_schema_data(SchemaBody::String, ValueDataDraft::String(String::new()))
}

#[derive(Clone, Copy, Debug)]
struct Bounds {
    source_bytes: u64,
    selector_count: u64,
    selection: Selection,
    payload: u64,
}

fn bounds(
    source: &ResolvedValueDescriptor,
    source_footprint: CurrentMemoryFootprint,
    selector: Option<&ResolvedValueDescriptor>,
) -> MResult<Bounds> {
    if source.schema().body() != &SchemaBody::String {
        return Err(invalid(
            "String selection requires a String input descriptor",
        ));
    }
    // The current Known canonical String witness includes every UTF-8 byte.
    // Neither this bound nor selector cardinality inspects the input payload.
    let source_bytes = source_footprint
        .payload_bytes
        .max(source_footprint.encoded_bytes);
    let (selection, selector_count) = match selector {
        None => (Selection::All, 0),
        Some(selector) => {
            let selection = selector_schema(selector.schema().body())?;
            let count = match selector.schema().body() {
                SchemaBody::Matrix { dimensions, .. } => {
                    dimensions.iter().try_fold(1u64, |count, dimension| {
                        count
                            .checked_mul(
                                selector
                                    .shape()
                                    .resolve_dimension(dimension)
                                    .map_err(MechError::from)?,
                            )
                            .ok_or_else(overflow)
                    })?
                }
                _ => 1,
            };
            (selection, count)
        }
    };
    let payload = if matches!(selection, Selection::Numeric(_)) {
        source_bytes
            .checked_mul(selector_count)
            .ok_or_else(overflow)?
    } else {
        source_bytes
    };
    Ok(Bounds {
        source_bytes,
        selector_count,
        selection,
        payload,
    })
}

impl CanonicalAccess {
    pub(super) fn is_managed_string_selection(&self) -> bool {
        self.source.representation() == FunctionValueRepresentation::String
            && self.selectors.len() == 1
    }

    fn string_request_bounds(
        &self,
        request: &ExecutionDemandPlanningRequest<'_>,
    ) -> MResult<Bounds> {
        let source = request
            .inputs
            .first()
            .ok_or_else(|| invalid("missing String input descriptor"))?;
        let source_footprint = request
            .input_footprint(0)
            .map_err(|error| MechError::new(error, None).with_compiler_loc())?;
        let selector = match self.selectors.first() {
            Some(CanonicalAccessSelector::All) if request.inputs.len() == 1 => None,
            Some(CanonicalAccessSelector::Cell(_)) if request.inputs.len() == 2 => {
                Some(&request.inputs[1])
            }
            _ => return Err(invalid("String selection input arity mismatch")),
        };
        bounds(source, source_footprint, selector)
    }

    fn string_footprint(&self, bounds: Bounds) -> MResult<CurrentMemoryFootprint> {
        if self.output.representation() != FunctionValueRepresentation::String {
            return Err(invalid("String selection requires a scalar String output"));
        }
        let parameters = self.output.shape().parameter_values().len();
        let value = prospective_string_value_footprint(parameters, false, 1, bounds.payload)
            .map_err(|_| overflow())?;
        Ok(CurrentMemoryFootprint {
            logical_elements: 1,
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

    pub(super) fn managed_string_output_footprints(
        &self,
        request: &ExecutionDemandPlanningRequest<'_>,
    ) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
        Ok(Some(
            vec![self.string_footprint(self.string_request_bounds(request)?)?].into_boxed_slice(),
        ))
    }

    pub(super) fn managed_string_execution_demand(
        &self,
        request: &ExecutionDemandPlanningRequest<'_>,
    ) -> MResult<PlannedExecutionDemand> {
        let bounds = self.string_request_bounds(request)?;
        let footprint = self.string_footprint(bounds)?;
        let offsets = if bounds.selection == Selection::All {
            0
        } else {
            bounds
                .source_bytes
                .checked_add(1)
                .and_then(|count| count.checked_mul(core::mem::size_of::<usize>() as u64))
                .ok_or_else(overflow)?
        };
        // Offset backing coexists with the staged UTF-8 buffer, final String,
        // draft/frozen wrappers and schema construction. Canonical inputs share
        // their immutable roots; dense numeric selectors are borrowed in place.
        let scratch = offsets
            .checked_add(core::mem::size_of::<PreparedGraphemeIndex>() as u64)
            .and_then(|bytes| bytes.checked_add(footprint.payload_bytes.checked_mul(4)?))
            .and_then(|bytes| bytes.checked_add(footprint.schema_bytes.checked_mul(4)?))
            .ok_or_else(overflow)?;
        // One segmentation pass, two complete selector visits, payload copy,
        // UTF-8 validation and canonical finalization are all bounded here.
        let compute = bounds
            .source_bytes
            .checked_add(bounds.selector_count)
            .and_then(|work| work.checked_add(bounds.payload))
            .and_then(|work| work.checked_add(footprint.schema_bytes))
            .and_then(|work| work.checked_mul(16))
            .ok_or_else(overflow)?;
        Ok(PlannedExecutionDemand {
            temporary_bytes: scratch,
            cloned_bytes: footprint
                .payload_bytes
                .checked_mul(4)
                .and_then(|bytes| bytes.checked_add(footprint.schema_bytes.checked_mul(4)?))
                .ok_or_else(overflow)?,
            retained_nodes: footprint
                .retained_nodes
                .checked_mul(4)
                .and_then(|nodes| nodes.checked_add(2))
                .ok_or_else(overflow)?,
            work: WorkDemand {
                compute,
                ..WorkDemand::default()
            },
        })
    }

    pub(super) fn stage_managed_string(&self, frame: &mut KernelMemoryFrame<'_>) -> MResult<()> {
        let source_descriptor = frame.input_cell_descriptor(&self.source, 0)?;
        let source_footprint = frame.input_cell_footprint(&self.source, 0)?;
        let selector_descriptor = match &self.selectors[0] {
            CanonicalAccessSelector::All => None,
            CanonicalAccessSelector::Cell(cell) => Some(frame.input_cell_descriptor(cell, 1)?),
        };
        let bounds = bounds(source_descriptor, source_footprint, selector_descriptor)?;
        let footprint = self.string_footprint(bounds)?;
        frame.with_admitted_canonical_output(&self.output, footprint, |frame, construction| {
            let source =
                frame.snapshot_input_cell_with_construction(&self.source, 0, construction)?;
            let ValueData::String(text) = source.data() else {
                return Err(invalid("String selection input contains non-String data"));
            };
            if u64::try_from(text.len()).map_err(|_| overflow())? > bounds.source_bytes {
                return Err(invalid(
                    "String selection input exceeds its admitted footprint",
                ));
            }
            if bounds.selection == Selection::All {
                let joined = construction.try_concatenate_string(text, "")?;
                let next = construction
                    .try_rebuild_data_draft(&self.output, ValueDataDraft::String(joined))?;
                return Ok(((), next));
            }
            let index = prepared_index(text, construction)?;
            // Check every selector and exact selected size before copying any
            // output. The second pass borrows the same immutable frame inputs.
            let mut bytes = 0usize;
            self.visit_string_selection(frame, bounds, index.len(), |ordinal| {
                let range = index
                    .byte_range(ordinal)
                    .ok_or_else(|| invalid("String index is outside its admitted boundaries"))?;
                bytes = bytes.checked_add(range.len()).ok_or_else(overflow)?;
                Ok(())
            })?;
            if u64::try_from(bytes).map_err(|_| overflow())? > bounds.payload {
                return Err(invalid(
                    "String selection output exceeds its admitted footprint",
                ));
            }
            let mut selected = construction.try_vec_with_capacity::<u8>(bytes)?;
            self.visit_string_selection(frame, bounds, index.len(), |ordinal| {
                let range = index
                    .byte_range(ordinal)
                    .ok_or_else(|| invalid("String index is outside its admitted boundaries"))?;
                selected.extend_from_slice(&text.as_bytes()[range]);
                Ok(())
            })?;
            let selected = String::from_utf8(selected)
                .map_err(|_| invalid("String selection produced invalid UTF-8"))?;
            let next = construction
                .try_rebuild_data_draft(&self.output, ValueDataDraft::String(selected))?;
            Ok(((), next))
        })?;
        Ok(())
    }

    fn visit_string_selection(
        &self,
        frame: &KernelMemoryFrame<'_>,
        bounds: Bounds,
        upper: usize,
        mut visit: impl FnMut(usize) -> MResult<()>,
    ) -> MResult<()> {
        let CanonicalAccessSelector::Cell(selector) = &self.selectors[0] else {
            return Err(invalid("unexpected All selector in indexed String access"));
        };
        // This physical predicate observes no contents. Both branches retain
        // the exact semantic input lease; it is not a cache/identity shortcut.
        if selector.requires_canonical_output_builder()? {
            let selector = frame.snapshot_input_cell(selector, 1)?;
            return visit_snapshot(&selector, bounds, upper, visit);
        }
        if bounds.selection == Selection::Bitmap {
            #[cfg(feature = "bool")]
            return frame.with_input_cell_value_view::<bool, _>(selector, 1, |values| {
                check_count(values.len(), bounds.selector_count)?;
                check_bitmap_length(values.len(), upper)?;
                for ordinal in 0..values.len() {
                    if values
                        .get(ordinal / values.columns(), ordinal % values.columns())
                        .ok_or_else(|| invalid("logical selector has invalid geometry"))?
                    {
                        visit(ordinal)?;
                    }
                }
                Ok(())
            });
            #[cfg(not(feature = "bool"))]
            return Err(invalid(
                "dense logical String selection requires Bool support",
            ));
        }
        macro_rules! dense {
            ($type:ty, $convert:expr) => {
                visit_dense::<$type>(frame, selector, bounds, upper, $convert, &mut visit)
            };
        }
        match bounds.selection {
            Selection::Numeric(Numeric::Index) => {
                dense!(usize, |value| ValueData::Index(value as u64))
            }
            Selection::Numeric(Numeric::U8) => dense!(u8, ValueData::U8),
            Selection::Numeric(Numeric::U16) => dense!(u16, ValueData::U16),
            Selection::Numeric(Numeric::U32) => dense!(u32, ValueData::U32),
            Selection::Numeric(Numeric::U64) => dense!(u64, ValueData::U64),
            Selection::Numeric(Numeric::U128) => dense!(u128, ValueData::U128),
            Selection::Numeric(Numeric::I8) => dense!(i8, ValueData::I8),
            Selection::Numeric(Numeric::I16) => dense!(i16, ValueData::I16),
            Selection::Numeric(Numeric::I32) => dense!(i32, ValueData::I32),
            Selection::Numeric(Numeric::I64) => dense!(i64, ValueData::I64),
            Selection::Numeric(Numeric::I128) => dense!(i128, ValueData::I128),
            Selection::Numeric(Numeric::F32) => {
                dense!(f32, |value| ValueData::F32(F32Bits::from_f32(value)))
            }
            Selection::Numeric(Numeric::F64) => {
                dense!(f64, |value| ValueData::F64(F64Bits::from_f64(value)))
            }
            _ => Err(invalid("invalid managed String selector")),
        }
    }
}

fn check_count(actual: usize, expected: u64) -> MResult<()> {
    if u64::try_from(actual).map_err(|_| overflow())? != expected {
        return Err(invalid(
            "String selector no longer matches its admitted descriptor",
        ));
    }
    Ok(())
}

fn check_bitmap_length(length: usize, upper: usize) -> MResult<()> {
    if length != upper {
        return Err(invalid(format!(
            "logical selector length {length} does not match extent {upper}"
        )));
    }
    Ok(())
}

fn positional(one_based: Result<u64, CanonicalSelectorError>, upper: usize) -> MResult<usize> {
    let one_based = one_based
        .map_err(|error| invalid(format!("invalid positional access selector: {error:?}")))?;
    usize::try_from(one_based)
        .ok()
        .and_then(|index| index.checked_sub(1))
        .filter(|index| *index < upper)
        .ok_or_else(|| invalid(format!("access index {one_based} is outside 1..={upper}")))
}

fn visit_dense<T: ManagedElement + Copy>(
    frame: &KernelMemoryFrame<'_>,
    selector: &ValueCell,
    bounds: Bounds,
    upper: usize,
    convert: impl Fn(T) -> ValueData,
    visit: &mut impl FnMut(usize) -> MResult<()>,
) -> MResult<()> {
    frame.with_input_cell_value_view::<T, _>(selector, 1, |values| {
        check_count(values.len(), bounds.selector_count)?;
        // Canonical selector matrices enumerate row-major, even though the
        // realized fixed-width storage may be column-major or strided.
        for ordinal in 0..values.len() {
            let value = values
                .get(ordinal / values.columns(), ordinal % values.columns())
                .ok_or_else(|| invalid("numeric selector has invalid geometry"))?;
            visit(positional(
                canonical_positional_ordinal(&convert(value)),
                upper,
            )?)?;
        }
        Ok(())
    })
}

fn visit_snapshot(
    value: &Value,
    bounds: Bounds,
    upper: usize,
    mut visit: impl FnMut(usize) -> MResult<()>,
) -> MResult<()> {
    match value.data() {
        ValueData::Matrix(matrix) => {
            let values = matrix.elements();
            check_count(values.len(), bounds.selector_count)?;
            if bounds.selection == Selection::Bitmap {
                let SequenceView::Bool(values) = values else {
                    return Err(invalid(
                        "logical selector mixes boolean and non-boolean values",
                    ));
                };
                check_bitmap_length(values.len(), upper)?;
                for (ordinal, selected) in values.iter().enumerate() {
                    if *selected {
                        visit(ordinal)?;
                    }
                }
            } else {
                for ordinal in 0..values.len() {
                    visit(positional(
                        canonical_positional_ordinal_at(values, ordinal),
                        upper,
                    )?)?;
                }
            }
        }
        data if matches!(bounds.selection, Selection::Numeric(_)) => {
            check_count(1, bounds.selector_count)?;
            visit(positional(canonical_positional_ordinal(data), upper)?)?;
        }
        _ => return Err(invalid("String selector has invalid canonical data")),
    }
    Ok(())
}

fn prepared_index(
    text: &str,
    construction: &mut FrozenSnapshotConstruction,
) -> MResult<PreparedGraphemeIndex> {
    #[cfg(test)]
    INDEX_BUILDS.with(|builds| builds.set(builds.get() + 1));
    PreparedGraphemeIndex::build_with(
        text,
        |capacity| {
            construction
                .try_vec_with_capacity(capacity)
                .map_err(MechError::from)
        },
        overflow,
    )
}

#[cfg(test)]
std::thread_local! { static INDEX_BUILDS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }

#[cfg(test)]
mod tests {
    use super::super::canonical_aggregate_access_tests::{assert_string, managed_scalar_access};
    use super::*;
    use std::rc::Rc;

    fn text(text: &str) -> ValueCell {
        ValueCell::from_schema_data(SchemaBody::String, ValueDataDraft::String(text.into()))
            .unwrap()
    }

    fn ordinal(value: u64) -> ValueCell {
        ValueCell::from_schema_data(
            SchemaBody::UnsignedInteger(IntegerWidth::W64),
            ValueDataDraft::U64(value),
        )
        .unwrap()
    }

    fn matrix(
        element: SchemaBody,
        rows: usize,
        columns: usize,
        values: impl IntoIterator<Item = ValueDataDraft>,
    ) -> ValueCell {
        ValueCell::from_schema_data(
            SchemaBody::Matrix {
                element: Box::new(element),
                dimensions: vec![
                    DimensionExpr::Constant(rows as u64),
                    DimensionExpr::Constant(columns as u64),
                ]
                .into_boxed_slice(),
            },
            ValueDataDraft::Matrix(values.into_iter().collect()),
        )
        .unwrap()
    }

    fn access(source: ValueCell, selector: CanonicalAccessSelector) -> CanonicalAccess {
        CanonicalAccess {
            source,
            output: empty_seed(std::slice::from_ref(&selector)).unwrap(),
            selectors: vec![selector],
            name: "StringAccessScalar",
        }
    }

    fn specialized(
        source: ValueCell,
        selector: CanonicalAccessSelector,
    ) -> MResult<SpecializedFunction> {
        let input = match selector {
            CanonicalAccessSelector::All => SpecializationInput::MatrixAllSelection,
            CanonicalAccessSelector::Cell(cell) => SpecializationInput::Cell(cell),
        };
        let invocation = SpecializationInvocation::new(
            vec![SpecializationInput::Cell(source), input].into_boxed_slice(),
        );
        let context = SpecializationContext::for_syntax_directed_invocation(
            &invocation,
            None,
            ResolvedOperationDescriptor::from_name(
                "access/scalar",
                super::super::PURE_CANONICAL_ACCESS_BINARY_CONTRACT.clone(),
            )?,
        )?;
        super::super::canonical_access(&invocation, &context, "StringAccessScalar")
    }

    #[test]
    fn specialization_and_planning_do_not_observe_selector_contents_or_segment_text() {
        INDEX_BUILDS.with(|builds| builds.set(0));
        let source = text("e\u{301}🧑‍🚀");
        let selector = ordinal(0);
        let function = specialized(
            source.clone(),
            CanonicalAccessSelector::Cell(selector.clone()),
        )
        .unwrap();
        assert_string(function.output(), "");
        assert_eq!(INDEX_BUILDS.with(|builds| builds.get()), 0);
        let implementation = access(
            source.clone(),
            CanonicalAccessSelector::Cell(selector.clone()),
        );
        assert!(
            implementation
                .planned_output_footprints()
                .unwrap()
                .is_none()
        );
        let inputs = vec![
            source.resolved_descriptor().unwrap(),
            selector.resolved_descriptor().unwrap(),
        ];
        let witnesses = vec![
            MemoryFootprintWitness::Known(source.current_memory_footprint().unwrap()),
            MemoryFootprintWitness::Known(selector.current_memory_footprint().unwrap()),
        ];
        let request = ExecutionDemandPlanningRequest {
            inputs: &inputs,
            input_witnesses: &witnesses,
        };
        assert!(
            implementation
                .planned_output_footprints_with_request(&request)
                .unwrap()
                .is_some()
        );
        assert!(
            implementation
                .planned_execution_demand(&request)
                .unwrap()
                .temporary_bytes
                > 0
        );
        assert_eq!(INDEX_BUILDS.with(|builds| builds.get()), 0);
        let version = function.output().published_version();
        assert!(function.instance().solve_result().is_err());
        assert_string(function.output(), "");
        assert_eq!(function.output().published_version(), version);
        selector.replace(&ordinal(2).snapshot().unwrap()).unwrap();
        function.instance().solve_result().unwrap();
        assert_string(function.output(), "🧑‍🚀");
    }

    #[test]
    fn equal_byte_replacements_rebuild_the_current_grapheme_index_and_fail_atomically() {
        let source = text("éA");
        let function = managed_scalar_access(source.clone(), ordinal(2));
        INDEX_BUILDS.with(|builds| builds.set(0));
        for (next, expected) in [("éA", "A"), ("Aé", "é"), ("XYZ", "Y")] {
            source.replace(&text(next).snapshot().unwrap()).unwrap();
            function.instance().solve_result().unwrap();
            assert_string(function.output(), expected);
        }
        assert_eq!(INDEX_BUILDS.with(|builds| builds.get()), 3);
        let version = function.output().published_version();
        source
            .replace(&text("e\u{301}").snapshot().unwrap())
            .unwrap();
        assert!(function.instance().solve_result().is_err());
        assert_string(function.output(), "Y");
        assert_eq!(function.output().published_version(), version);
        source.replace(&text("éA").snapshot().unwrap()).unwrap();
        function.instance().solve_result().unwrap();
        assert_string(function.output(), "A");
    }

    #[test]
    fn canonical_numeric_and_bitmap_matrices_keep_row_major_selector_order() {
        let source = text("ABCDEF");
        let gather = matrix(
            SchemaBody::UnsignedInteger(IntegerWidth::W64),
            2,
            3,
            [6, 1, 4, 2, 6, 3].map(ValueDataDraft::U64),
        );
        let function = managed_scalar_access(source.clone(), gather);
        function.instance().solve_result().unwrap();
        assert_string(function.output(), "FADBFC");
        let bitmap = matrix(
            SchemaBody::Bool,
            2,
            3,
            [true, false, true, false, true, false].map(ValueDataDraft::Bool),
        );
        let function = managed_scalar_access(source, bitmap);
        function.instance().solve_result().unwrap();
        assert_string(function.output(), "ACE");
    }

    #[test]
    fn all_and_empty_numeric_selection_preserve_raw_string_bytes() {
        for contents in ["", "e\u{301}🧑‍🚀\r\n🇺🇸\0"] {
            let function = specialized(text(contents), CanonicalAccessSelector::All).unwrap();
            assert_string(function.output(), "");
            INDEX_BUILDS.with(|builds| builds.set(0));
            function.instance().solve_result().unwrap();
            assert_string(function.output(), contents);
            assert_eq!(INDEX_BUILDS.with(|builds| builds.get()), 0);
            let empty = matrix(SchemaBody::UnsignedInteger(IntegerWidth::W64), 1, 0, []);
            let function = managed_scalar_access(text(contents), empty);
            function.instance().solve_result().unwrap();
            assert_string(function.output(), "");
        }
    }

    #[test]
    fn bitmap_cardinality_and_nonfinite_selectors_fail_after_admission_without_publication() {
        let source = text("éA");
        let bitmap = matrix(
            SchemaBody::Bool,
            1,
            2,
            [true, false].map(ValueDataDraft::Bool),
        );
        let function = managed_scalar_access(source.clone(), bitmap);
        function.instance().solve_result().unwrap();
        assert_string(function.output(), "é");
        let version = function.output().published_version();
        source.replace(&text("XYZ").snapshot().unwrap()).unwrap();
        assert!(function.instance().solve_result().is_err());
        assert_string(function.output(), "é");
        assert_eq!(function.output().published_version(), version);
        for invalid in [f64::NAN, f64::INFINITY, -1.0, 0.0, 4_294_967_296.0] {
            let selector = ValueCell::from_schema_data(
                SchemaBody::FloatingPoint(FloatWidth::W64),
                ValueDataDraft::F64(F64Bits::from_f64(invalid)),
            )
            .unwrap();
            let function =
                specialized(text("ABC"), CanonicalAccessSelector::Cell(selector)).unwrap();
            assert_string(function.output(), "");
            assert!(function.instance().solve_result().is_err());
            assert_string(function.output(), "");
        }
        let selector = ValueCell::from_schema_data(
            SchemaBody::FloatingPoint(FloatWidth::W64),
            ValueDataDraft::F64(F64Bits::from_f64(2.9)),
        )
        .unwrap();
        let function = managed_scalar_access(text("ABC"), selector);
        function.instance().solve_result().unwrap();
        assert_string(function.output(), "B");
    }

    #[test]
    fn empty_bitmap_still_checks_the_actual_grapheme_extent() {
        let bitmap = matrix(SchemaBody::Bool, 1, 0, []);
        let function =
            specialized(text(""), CanonicalAccessSelector::Cell(bitmap.clone())).unwrap();
        function.instance().solve_result().unwrap();
        assert_string(function.output(), "");
        let function = specialized(text("A"), CanonicalAccessSelector::Cell(bitmap)).unwrap();
        let version = function.output().published_version();
        assert!(function.instance().solve_result().is_err());
        assert_eq!(function.output().published_version(), version);
        assert_string(function.output(), "");
    }

    #[cfg(all(feature = "matrixd", feature = "f64", feature = "bool"))]
    #[test]
    fn dense_numeric_and_bitmap_selectors_use_leased_geometry_without_canonical_reconstruction() {
        let dense = ValueCell::from_exact(nalgebra::DMatrix::from_row_slice(
            2,
            3,
            &[6.0f64, 1.0, 4.0, 2.0, 6.0, 3.0],
        ))
        .unwrap();
        assert!(!dense.requires_canonical_output_builder().unwrap());
        let function = managed_scalar_access(text("ABCDEF"), dense);
        function.instance().solve_result().unwrap();
        assert_string(function.output(), "FADBFC");
        let mask = ValueCell::from_exact(nalgebra::DMatrix::from_row_slice(
            2,
            3,
            &[true, false, true, false, true, false],
        ))
        .unwrap();
        assert!(!mask.requires_canonical_output_builder().unwrap());
        let function = managed_scalar_access(text("ABCDEF"), mask);
        function.instance().solve_result().unwrap();
        assert_string(function.output(), "ACE");
    }

    #[test]
    fn finite_work_denial_precedes_segmentation_and_retains_prior_publication_then_recovers() {
        let source = text("éA");
        let selector = ordinal(2);
        let function = managed_scalar_access(source.clone(), selector.clone());
        let output = function.output().clone();
        let mut template = function.memory_plan().clone();
        drop(function);
        output.memory_domain().unwrap().collect_retired().unwrap();
        template.target.limits.max_compute_work = Some(65_536);
        let plan = replan_call_memory(&template, &template.bound_call).unwrap();
        let instance = FunctionInstance::new(
            Box::new(CanonicalAccess {
                source: source.clone(),
                selectors: vec![CanonicalAccessSelector::Cell(selector.clone())],
                output: output.clone(),
                name: "StringAccessScalar",
            }),
            FunctionInvocation::binary(output.clone(), source.clone(), selector),
            Rc::new(plan),
        )
        .unwrap();
        instance.solve_result().unwrap();
        assert_string(&output, "A");
        let version = output.published_version();
        INDEX_BUILDS.with(|builds| builds.set(0));
        source
            .replace(&text(&"A".repeat(4096)).snapshot().unwrap())
            .unwrap();
        assert!(instance.solve_result().is_err());
        assert_eq!(INDEX_BUILDS.with(|builds| builds.get()), 0);
        assert_eq!(output.published_version(), version);
        assert_string(&output, "A");
        source.replace(&text("Aé").snapshot().unwrap()).unwrap();
        instance.solve_result().unwrap();
        assert_eq!(INDEX_BUILDS.with(|builds| builds.get()), 1);
        assert_string(&output, "é");
    }
}
