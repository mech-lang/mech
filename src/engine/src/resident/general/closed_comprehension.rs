//! Bounded lexical collection adapter for already-closed values. Ordinary
//! bodies go through the existing closed-operation owners, never a factory or
//! turn executor. Pattern projection/binding and result identity are shared
//! with resident collection execution.

use super::execution::comprehension_execution as collection;
use super::*;
use crate::{
    CollectionPattern as Pattern, ComprehensionDeclaration, ComprehensionKind,
    ComprehensionStep as Step, ComprehensionValue as LexicalValue,
};
use mech_core::snapshot::ValueFootprint;
use std::sync::Arc;

type Result<T> = core::result::Result<T, ResidentActivationError>;

#[derive(Default)]
pub(super) struct ClosedCollectionCache {
    // Only immutable preparation is shared. Lexical values, shape facts and
    // first-publication state are freshly supplied for every invocation.
    projections: std::sync::OnceLock<(
        Arc<mech_core::SchemaTable>,
        collection::StructuralProjectionTable,
    )>,
    frames: std::cell::RefCell<Vec<ClosedOperationFrame>>,
    pub(super) pattern_materializations: std::cell::Cell<u64>,
    pub(super) frame_constructions: std::cell::Cell<u64>,
}

struct ClosedOperationFrame {
    reference: OperationReference,
    contract: mech_core::OperationContractId,
    output: SchemaId,
    artifact: Arc<ProgramArtifact>,
}

#[derive(Clone)]
struct ClosedLexicalValue {
    snapshot: Arc<Value>,
    kind: ResidentValueKind,
}

impl core::ops::Deref for ClosedLexicalValue {
    type Target = Value;
    fn deref(&self) -> &Value {
        &self.snapshot
    }
}

impl AsRef<Value> for ClosedLexicalValue {
    fn as_ref(&self) -> &Value {
        &self.snapshot
    }
}

fn local_value(
    artifact: &ProgramArtifact,
    value: Value,
    control_output: bool,
) -> Result<ClosedLexicalValue> {
    let schema = artifact
        .schemas()
        .get(value.schema())
        .ok_or(ResidentActivationError::RegionSizeOverflow)?;
    let kind = if control_output
        || (matches!(schema.body(), SchemaBody::Matrix { .. })
            && schema
                .dimension_parameters()
                .iter()
                .any(|parameter| parameter.lifetime() == DimensionLifetime::Turn))
    {
        ResidentValueKind::Snapshot
    } else {
        schema_layout(
            artifact,
            value.schema(),
            value.shape(),
            schema.dimension_parameters().is_empty(),
            None,
            None,
        )?
        .0
    };
    Ok(ClosedLexicalValue {
        snapshot: Arc::new(value),
        kind,
    })
}

fn charge(analysis: &ClosedActivationAnalysisContext, bytes: u64, work: u64) -> Option<()> {
    analysis
        .construction
        .charge_compute_chunk(bytes, work, 1)
        .then_some(())
}

fn projection_context<'a>(
    artifact: &ProgramArtifact,
    analysis: &'a ClosedActivationAnalysisContext,
) -> Option<&'a (
    Arc<mech_core::SchemaTable>,
    collection::StructuralProjectionTable,
)> {
    if analysis.collections.projections.get().is_none() {
        let remaining = mech_core::RESIDENT_MAX_COMPARISON_WORK
            .saturating_sub(analysis.construction.comparison_work.get());
        let budget = SnapshotCanonicalizationBudget::new(remaining);
        let (retained, construction, nodes) = artifact
            .schemas()
            .component_closure_bounds_with_budget(&budget)?;
        let bytes = construction
            .checked_mul(4)?
            .checked_add(retained)?
            .checked_add(nodes.checked_mul(128)?)?;
        if !analysis.construction.charge_compute_with_comparison(
            bytes,
            nodes.checked_mul(4)?,
            budget.consumed(),
        ) {
            return None;
        }
        let (schemas, projections) =
            collection::structural_projection_schema_context(artifact.schemas()).ok()?;
        let _ = analysis
            .collections
            .projections
            .set((Arc::new(schemas), projections));
    }
    analysis.collections.projections.get()
}

fn value(
    artifact: &ProgramArtifact,
    reference: LexicalValue,
    inputs: &[ClosedLexicalValue],
    locals: &[ClosedLexicalValue],
    analysis: &ClosedActivationAnalysisContext,
) -> Option<ClosedLexicalValue> {
    match reference {
        LexicalValue::Constant(id) => {
            let value = artifact.constants().get(id)?;
            if !analysis
                .construction
                .charge_clone(value, artifact.schemas(), 1)
            {
                return None;
            }
            Some(ClosedLexicalValue {
                snapshot: Arc::new(value.clone()),
                kind: value_layout(artifact, value).ok()?.0,
            })
        }
        LexicalValue::Input(index) => inputs.get(index as usize).cloned(),
        LexicalValue::Local(index) => locals.get(index as usize).cloned(),
    }
}

pub(super) fn fold(
    artifact: &ProgramArtifact,
    node: NodeId,
    slot: CellSlotId,
    control: &ComprehensionDeclaration,
    facts: &ActivationFacts,
    analysis: &ClosedActivationAnalysisContext,
    depth: usize,
) -> Result<Option<Value>> {
    if depth >= 64 {
        return Ok(None);
    }
    let source_count = artifact.nodes()[node.get() as usize].input_bindings.len() as u64;
    if charge(
        analysis,
        source_count
            * (core::mem::size_of::<ClosedLexicalValue>() + core::mem::size_of::<ArtifactSource>())
                as u64,
        source_count,
    )
    .is_none()
    {
        return Ok(None);
    }
    let sources = node_inputs(artifact, node)?;
    let mut inputs = Vec::with_capacity(sources.len());
    for source in sources {
        let Some(input) = constant_comparison_operand_at_depth(
            artifact,
            node,
            source,
            facts,
            depth,
            &SnapshotCanonicalizationBudget::new(mech_core::RESIDENT_MAX_COMPARISON_WORK),
            analysis,
            true,
        )?
        else {
            return Ok(None);
        };
        if !analysis
            .construction
            .charge_clone(&input.value, artifact.schemas(), 1)
        {
            return Ok(None);
        }
        let kind = closed_operand_resident_kind(artifact, &input, facts)?;
        inputs.push(ClosedLexicalValue {
            snapshot: Arc::new(input.value.into_owned()),
            kind,
        });
    }
    fold_declaration(
        artifact,
        control,
        artifact.slots()[slot.get() as usize].schema,
        &inputs,
        analysis,
        depth,
    )
}

fn fold_declaration(
    artifact: &ProgramArtifact,
    control: &ComprehensionDeclaration,
    output: SchemaId,
    inputs: &[ClosedLexicalValue],
    analysis: &ClosedActivationAnalysisContext,
    depth: usize,
) -> Result<Option<Value>> {
    if depth >= 64 {
        return Ok(None);
    }
    let Some((schemas, projections)) = projection_context(artifact, analysis) else {
        return Ok(None);
    };
    let Some(local_count) = control.steps.iter().try_fold(0usize, |count, step| {
        count.checked_add(match step {
            Step::Generator { pattern, .. } => crate::pattern_metrics(pattern)?.bindings,
            Step::Operation(_) => 1,
            Step::Filter(_) => 0,
        })
    }) else {
        return Ok(None);
    };
    if charge(
        analysis,
        (local_count as u64).saturating_mul(core::mem::size_of::<ClosedLexicalValue>() as u64),
        control.steps.len() as u64,
    )
    .is_none()
    {
        return Ok(None);
    }
    let mut locals = Vec::with_capacity(local_count);
    let mut values = Vec::new();
    let mut element_body = None;
    let mut footprint = ValueFootprint::zero();
    let mut finalization_work = 1u64;
    if !enumerate(
        artifact,
        control,
        0,
        inputs,
        &mut locals,
        &mut values,
        &mut element_body,
        &mut footprint,
        &mut finalization_work,
        schemas,
        projections,
        analysis,
        depth,
    )? {
        return Ok(None);
    }
    let Some(schema) = artifact.schemas().get(output) else {
        return Ok(None);
    };
    // Admit canonical output, the Vec-to-box overlap, shape resolution and set
    // sorting before any of them runs. Every yielded draft was admitted earlier.
    let count = values.len() as u64;
    if footprint.node_count > mech_core::RESIDENT_MAX_RETAINED_NODES {
        return Ok(None);
    }
    let sort = if control.kind == ComprehensionKind::Set {
        count.checked_mul(count.max(1).ilog2() as u64 + 1)
    } else {
        Some(0)
    };
    let Some(work) = sort.and_then(|sort| sort.checked_add(finalization_work)) else {
        return Ok(None);
    };
    let Some(bytes) = footprint
        .retained_bytes
        .checked_mul(2)
        .and_then(|bytes| {
            bytes.checked_add(count.checked_mul(core::mem::size_of::<ValueDataDraft>() as u64)?)
        })
        .and_then(|bytes| {
            bytes.checked_add(schema.clone_allocation_bound_bytes()?.checked_mul(4)?)
        })
    else {
        return Ok(None);
    };
    if !analysis
        .construction
        .charge_compute_with_comparison(bytes, work, work)
        || count > MAX_STATIC_SELECTOR_SOURCE_STEPS as u64
    {
        return Ok(None);
    }
    let (shape, data) = match control.kind {
        ComprehensionKind::Matrix | ComprehensionKind::MatrixPreserveShape => {
            let element = match element_body {
                Some(body) => body,
                None => {
                    let yield_schema = match control.yield_value {
                        LexicalValue::Constant(id) => {
                            artifact.constants().get(id).map(Value::schema)
                        }
                        LexicalValue::Input(index) => {
                            inputs.get(index as usize).map(|v| v.schema())
                        }
                        LexicalValue::Local(index) => {
                            comprehension::locals(control).get(index as usize).copied()
                        }
                    };
                    let Some(yield_schema) = yield_schema else {
                        return Ok(None);
                    };
                    let Ok(body) = collection::lower_bound_yield_body(yield_schema, &schemas)
                    else {
                        return Ok(None);
                    };
                    body
                }
            };
            let dimensions = if control.kind == ComprehensionKind::MatrixPreserveShape {
                let Some(source) = control.steps.iter().find_map(|step| match step {
                    Step::Generator { source, .. } => {
                        value(artifact, *source, inputs, &[], analysis)
                    }
                    _ => None,
                }) else {
                    return Ok(None);
                };
                let Ok(SchemaBody::Matrix { dimensions, .. }) = schemas
                    .get(source.schema())
                    .ok_or(ResidentActivationError::RegionSizeOverflow)?
                    .closed_body(source.shape())
                else {
                    return Ok(None);
                };
                if dimensions.iter().try_fold(1u64, |n, d| match d {
                    DimensionExpr::Constant(d) => n.checked_mul(*d),
                    _ => None,
                }) != Some(count)
                {
                    return Ok(None);
                }
                dimensions
            } else {
                vec![DimensionExpr::Constant(1), DimensionExpr::Constant(count)].into_boxed_slice()
            };
            let Ok(shape) = collection::completed_matrix_shape(schema, element, dimensions) else {
                return Ok(None);
            };
            (shape, ValueDataDraft::Matrix(values.into_boxed_slice()))
        }
        ComprehensionKind::Set => {
            let SchemaBody::Set { element, .. } = schema.body() else {
                return Ok(None);
            };
            if collection::normalize_collection_set(element, &mut values).is_err() {
                return Ok(None);
            }
            let data = ValueDataDraft::Set(values.into_boxed_slice());
            let Ok(shape) = mech_core::shape_for_value_data(schema, &data, &[], None) else {
                return Ok(None);
            };
            (shape, data)
        }
    };
    let budget = SnapshotCanonicalizationBudget::new(work.max(1));
    Ok(ValueDraft {
        schema: output,
        shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
        data,
    }
    .finalize(
        &SnapshotValidationContext::with_shared_schemas(&schemas)
            .with_canonicalization_budget(&budget),
    )
    .ok())
}

fn enumerate(
    artifact: &ProgramArtifact,
    control: &ComprehensionDeclaration,
    start: usize,
    inputs: &[ClosedLexicalValue],
    locals: &mut Vec<ClosedLexicalValue>,
    values: &mut Vec<ValueDataDraft>,
    element_body: &mut Option<SchemaBody>,
    footprint: &mut ValueFootprint,
    finalization_work: &mut u64,
    schemas: &Arc<mech_core::SchemaTable>,
    projections: &collection::StructuralProjectionTable,
    analysis: &ClosedActivationAnalysisContext,
    depth: usize,
) -> Result<bool> {
    for position in start..control.steps.len() {
        if charge(analysis, 0, 1).is_none() {
            return Ok(false);
        }
        match &control.steps[position] {
            Step::Generator { source, pattern } => {
                let Some(source) = value(artifact, *source, inputs, locals, analysis) else {
                    return Ok(false);
                };
                let count = match source.data() {
                    mech_core::ValueData::Matrix(v) => v.elements().len(),
                    mech_core::ValueData::Set(v) => v.elements().len(),
                    _ => return Ok(false),
                };
                let Some(metrics) = crate::pattern_metrics(pattern) else {
                    return Ok(false);
                };
                if metrics.depth >= 64 {
                    return Ok(false);
                }
                let Some(element_schema) = projections.matrix_element(source.schema()) else {
                    return Ok(false);
                };
                let Ok((element, shape_values)) = collection::generator_element(
                    source.schema(),
                    element_schema,
                    source.shape().parameter_values(),
                    schemas,
                ) else {
                    return Ok(false);
                };
                let retained_locals = locals.len();
                for ordinal in 0..count {
                    if charge(analysis, 0, metrics.nodes as u64 + 1).is_none() {
                        return Ok(false);
                    }
                    if !matches!(pattern, Pattern::Wildcard) {
                        // Every possible projection/binding clone is covered
                        // before the shared materializer copies the selected item.
                        let selected = match source.data() {
                            mech_core::ValueData::Matrix(matrix) => {
                                canonical_sequence_element_retained_footprint(
                                    &element,
                                    matrix.elements(),
                                    ordinal,
                                )
                            }
                            mech_core::ValueData::Set(set) => {
                                let Some(item) = set.elements().get(ordinal) else {
                                    return Ok(false);
                                };
                                mech_core::snapshot::canonical_data_retained_footprint(
                                    &element,
                                    item.data(),
                                )
                            }
                            _ => unreachable!(),
                        };
                        let Ok(selected) = selected else {
                            return Ok(false);
                        };
                        let copies = metrics.nodes as u64 * 4 + 4;
                        let Some(bytes) = selected
                            .retained_bytes
                            .checked_add(element.clone_allocation_bound_bytes().unwrap_or(u64::MAX))
                            .and_then(|n| n.checked_add(shape_values.len() as u64 * 8))
                            .and_then(|n| n.checked_mul(copies))
                        else {
                            return Ok(false);
                        };
                        if !analysis.construction.charge_compute_with_comparison(
                            bytes,
                            selected.node_count.saturating_mul(copies),
                            selected.node_count.saturating_mul(copies),
                        ) {
                            return Ok(false);
                        }
                        let budget = SnapshotCanonicalizationBudget::new(
                            mech_core::RESIDENT_MAX_COMPARISON_WORK
                                .saturating_sub(analysis.construction.comparison_work.get()),
                        );
                        let refs = [Some(source.as_ref().clone())];
                        analysis.collections.pattern_materializations.set(
                            analysis
                                .collections
                                .pattern_materializations
                                .get()
                                .saturating_add(1),
                        );
                        let Some(item) = collection::collection_item(
                            ResidentValueRef::Snapshot(&refs),
                            ResidentRegion {
                                kind: ResidentValueKind::Snapshot,
                                offset: 0,
                                len: 1,
                                shape: ResidentShape {
                                    rows: 1,
                                    columns: 1,
                                },
                            },
                            element_schema,
                            &element,
                            &shape_values,
                            ordinal,
                            &SnapshotValidationContext::with_shared_schemas(schemas)
                                .with_canonicalization_budget(&budget),
                        ) else {
                            return Ok(false);
                        };
                        let Some(matched) = match_pattern(
                            artifact,
                            pattern,
                            &item,
                            source.shape().parameter_values(),
                            inputs,
                            locals,
                            schemas,
                            projections,
                            &budget,
                            analysis,
                        )?
                        else {
                            return Ok(false);
                        };
                        if !analysis
                            .construction
                            .charge_comparison_work(budget.consumed())
                        {
                            return Ok(false);
                        }
                        if !matched {
                            locals.truncate(retained_locals);
                            continue;
                        }
                    }
                    if !enumerate(
                        artifact,
                        control,
                        position + 1,
                        inputs,
                        locals,
                        values,
                        element_body,
                        footprint,
                        finalization_work,
                        schemas,
                        projections,
                        analysis,
                        depth,
                    )? {
                        return Ok(false);
                    }
                    locals.truncate(retained_locals);
                }
                return Ok(true);
            }
            Step::Filter(reference) => match value(artifact, *reference, inputs, locals, analysis)
                .as_deref()
                .map(Value::data)
            {
                Some(mech_core::ValueData::Bool(true)) => {}
                Some(mech_core::ValueData::Bool(false)) => return Ok(true),
                _ => return Ok(false),
            },
            Step::Operation(operation) => {
                if operation.local as usize != locals.len() {
                    return Ok(false);
                }
                if charge(
                    analysis,
                    operation.inputs.len() as u64
                        * core::mem::size_of::<ClosedLexicalValue>() as u64,
                    operation.inputs.len() as u64,
                )
                .is_none()
                {
                    return Ok(false);
                }
                let Some(arguments) = operation
                    .inputs
                    .iter()
                    .map(|input| value(artifact, *input, inputs, locals, analysis))
                    .collect::<Option<Vec<_>>>()
                else {
                    return Ok(false);
                };
                let result = match &operation.body {
                    crate::ControlOperationBody::Operation {
                        operation: reference,
                        contract,
                    } => fold_operation(
                        artifact,
                        reference,
                        *contract,
                        operation.schema,
                        &arguments,
                        analysis,
                    )?,
                    crate::ControlOperationBody::Comprehension(control) => fold_declaration(
                        artifact,
                        control,
                        operation.schema,
                        &arguments,
                        analysis,
                        depth + 1,
                    )?,
                    crate::ControlOperationBody::Match(control) => fold_match(
                        artifact,
                        control,
                        operation.schema,
                        &arguments,
                        analysis,
                        depth + 1,
                    )?,
                    // Stateful/external and recursive controls cannot become
                    // activation-fixed values merely because captures are closed.
                    _ => None,
                };
                let Some(result) = result else {
                    return Ok(false);
                };
                locals.push(local_value(
                    artifact,
                    result,
                    matches!(
                        operation.body,
                        crate::ControlOperationBody::Comprehension(_)
                    ),
                )?);
            }
        }
    }
    let Some(yielded) = value(artifact, control.yield_value, inputs, locals, analysis) else {
        return Ok(false);
    };
    let Some(schema) = schemas.get(yielded.schema()) else {
        return Ok(false);
    };
    let Some(body_bytes) = schema.body().clone_allocation_bound_bytes() else {
        return Ok(false);
    };
    if charge(analysis, body_bytes * 4, 1).is_none()
        || !analysis.construction.charge_clone(&yielded, schemas, 3)
    {
        return Ok(false);
    }
    let Ok(body) = schema.closed_body(yielded.shape()) else {
        return Ok(false);
    };
    if element_body
        .as_ref()
        .is_some_and(|expected| expected != &body)
    {
        return Ok(false);
    }
    let mut meter = crate::resident::budget::ResidentBudgetMeter::for_closed_analysis(
        mech_core::RESIDENT_MAX_COMPUTE_WORK
            .saturating_sub(analysis.construction.compute_work.get()),
        mech_core::RESIDENT_MAX_COMPARISON_WORK
            .saturating_sub(analysis.construction.comparison_work.get()),
    );
    let Ok(item_footprint) = crate::resident::budget::measure_canonical_data_footprint(
        &mut meter,
        schema.body(),
        yielded.data(),
    ) else {
        return Ok(false);
    };
    let Ok(item_finalization) = crate::resident::budget::preflight_canonical_data_finalization(
        &mut meter,
        schema.body(),
        yielded.data(),
    ) else {
        return Ok(false);
    };
    if !analysis
        .construction
        .charge_comparison_work(meter.estimate().comparison_work())
    {
        return Ok(false);
    }
    if values.len() >= MAX_STATIC_SELECTOR_SOURCE_STEPS
        || charge(
            analysis,
            core::mem::size_of::<ValueDataDraft>() as u64 * 2,
            1,
        )
        .is_none()
        || !analysis.record_materialized_operand_values(1)
    {
        return Ok(false);
    }
    let Ok(data) = yielded.canonical_data_draft() else {
        return Ok(false);
    };
    values
        .try_reserve_exact(1)
        .map_err(|_| ResidentActivationError::RegionSizeOverflow)?;
    values.push(data);
    *footprint = footprint
        .checked_add(item_footprint)
        .map_err(|_| ResidentActivationError::RegionSizeOverflow)?;
    *finalization_work = finalization_work
        .checked_add(item_finalization)
        .ok_or(ResidentActivationError::RegionSizeOverflow)?;
    *element_body = Some(body);
    Ok(true)
}

fn match_pattern(
    artifact: &ProgramArtifact,
    pattern: &Pattern,
    item: &collection::PatternItem,
    shape_values: &[u64],
    inputs: &[ClosedLexicalValue],
    locals: &mut Vec<ClosedLexicalValue>,
    schemas: &Arc<mech_core::SchemaTable>,
    projections: &collection::StructuralProjectionTable,
    budget: &SnapshotCanonicalizationBudget,
    analysis: &ClosedActivationAnalysisContext,
) -> Result<Option<bool>> {
    if charge(analysis, 0, 1).is_none() {
        return Ok(None);
    }
    match pattern {
        Pattern::Wildcard => Ok(Some(true)),
        Pattern::Bind { local, schema } => {
            if *local as usize != locals.len() {
                return Ok(None);
            }
            let binding =
                match item
                    .clone()
                    .into_binding(*schema, shape_values, schemas, projections)
                {
                    Ok(Some(binding)) => binding,
                    Ok(None) => return Ok(Some(false)),
                    Err(_) => return Ok(None),
                };
            let Ok(value) = collection::finalize_pattern_binding(
                *schema,
                &binding.shape_values,
                binding.data,
                binding.schemas,
                binding.schema_index,
                schemas,
                budget,
            ) else {
                return Ok(None);
            };
            locals.push(local_value(artifact, value, false)?);
            Ok(Some(true))
        }
        Pattern::Equal(reference) => {
            let Some(peer) = value(artifact, *reference, inputs, locals, analysis) else {
                return Ok(None);
            };
            let refs = [Some(peer.as_ref().clone())];
            Ok(item
                .language_equals_with_budget(
                    ResidentValueRef::Snapshot(&refs),
                    ResidentRegion {
                        kind: ResidentValueKind::Snapshot,
                        offset: 0,
                        len: 1,
                        shape: ResidentShape {
                            rows: 1,
                            columns: 1,
                        },
                    },
                    peer.schema(),
                    shape_values,
                    budget,
                    schemas,
                    projections,
                )
                .ok()
                .flatten())
        }
        Pattern::Enum { ordinal, payload } => {
            let (actual, child) = match item.enum_variant(schemas) {
                Ok(Some(pair)) => pair,
                Ok(None) => return Ok(Some(false)),
                Err(_) => return Ok(None),
            };
            if actual != *ordinal {
                return Ok(Some(false));
            }
            match (payload, child) {
                (None, None) => Ok(Some(true)),
                (Some(pattern), Some(child)) => match_pattern(
                    artifact,
                    pattern,
                    &child,
                    shape_values,
                    inputs,
                    locals,
                    schemas,
                    projections,
                    budget,
                    analysis,
                ),
                _ => Ok(Some(false)),
            }
        }
        Pattern::Tuple(patterns) => {
            if item.structural_len(true) != Some(patterns.len()) {
                return Ok(Some(false));
            }
            for (index, pattern) in patterns.iter().enumerate() {
                let Some(child) = item.child(index, schemas, projections) else {
                    return Ok(Some(false));
                };
                match match_pattern(
                    artifact,
                    pattern,
                    &child,
                    shape_values,
                    inputs,
                    locals,
                    schemas,
                    projections,
                    budget,
                    analysis,
                )? {
                    Some(true) => {}
                    outcome => return Ok(outcome),
                }
            }
            Ok(Some(true))
        }
        Pattern::Array {
            prefix,
            rest,
            suffix,
        } => {
            let Some(count) = item.structural_len(false) else {
                return Ok(Some(false));
            };
            let required = prefix.len() + suffix.len();
            if count < required || (rest.is_none() && count != required) {
                return Ok(Some(false));
            }
            for (index, pattern) in prefix.iter().enumerate() {
                let Some(child) = item.child(index, schemas, projections) else {
                    return Ok(Some(false));
                };
                match match_pattern(
                    artifact,
                    pattern,
                    &child,
                    shape_values,
                    inputs,
                    locals,
                    schemas,
                    projections,
                    budget,
                    analysis,
                )? {
                    Some(true) => {}
                    outcome => return Ok(outcome),
                }
            }
            if let Some(rest) = rest {
                let Some(child) = item.middle(prefix.len(), suffix.len(), schemas, projections)
                else {
                    return Ok(Some(false));
                };
                match match_pattern(
                    artifact,
                    rest,
                    &child,
                    shape_values,
                    inputs,
                    locals,
                    schemas,
                    projections,
                    budget,
                    analysis,
                )? {
                    Some(true) => {}
                    outcome => return Ok(outcome),
                }
            }
            for (index, pattern) in suffix.iter().enumerate() {
                let Some(child) = item.child(count - suffix.len() + index, schemas, projections)
                else {
                    return Ok(Some(false));
                };
                match match_pattern(
                    artifact,
                    pattern,
                    &child,
                    shape_values,
                    inputs,
                    locals,
                    schemas,
                    projections,
                    budget,
                    analysis,
                )? {
                    Some(true) => {}
                    outcome => return Ok(outcome),
                }
            }
            Ok(Some(true))
        }
    }
}

fn fold_match(
    artifact: &ProgramArtifact,
    control: &crate::MatchDeclaration,
    output: SchemaId,
    inputs: &[ClosedLexicalValue],
    analysis: &ClosedActivationAnalysisContext,
    depth: usize,
) -> Result<Option<Value>> {
    if depth >= 64 || control.contains_suspend() {
        return Ok(None);
    }
    let Some(scrutinee) = inputs.get(control.scrutinee as usize) else {
        return Ok(None);
    };
    let Some((schemas, projections)) = projection_context(artifact, analysis) else {
        return Ok(None);
    };
    for arm in &control.arms {
        if charge(analysis, 0, 1).is_none() {
            return Ok(None);
        }
        let mut bindings = Vec::new();
        let matches = match &arm.pattern {
            crate::MatchPattern::Wildcard | crate::MatchPattern::Bind => Some(true),
            crate::MatchPattern::Literal(id) => {
                let Some(peer) =
                    value(artifact, LexicalValue::Constant(*id), inputs, &[], analysis)
                else {
                    return Ok(None);
                };
                let Some(schema) = schemas.get(scrutinee.schema()) else {
                    return Ok(None);
                };
                if !crate::is_control_scalar_schema(schema) {
                    return Ok(None);
                }
                let remaining = mech_core::RESIDENT_MAX_COMPARISON_WORK
                    .saturating_sub(analysis.construction.comparison_work.get());
                let mut meter = crate::resident::budget::ResidentBudgetMeter::for_closed_analysis(
                    mech_core::RESIDENT_MAX_COMPUTE_WORK
                        .saturating_sub(analysis.construction.compute_work.get()),
                    remaining,
                );
                let Ok(left) = crate::resident::budget::measure_canonical_value_footprint(
                    &mut meter, scrutinee, schemas,
                ) else {
                    return Ok(None);
                };
                let Ok(right) = crate::resident::budget::measure_canonical_value_footprint(
                    &mut meter, &peer, schemas,
                ) else {
                    return Ok(None);
                };
                let Some(work) = left
                    .encoded_bytes
                    .checked_add(right.encoded_bytes)
                    .and_then(|work| work.checked_add(meter.estimate().comparison_work()))
                else {
                    return Ok(None);
                };
                if !analysis.construction.charge_comparison_work(work) {
                    return Ok(None);
                }
                scrutinee.language_eq(schemas, &peer, schemas).ok()
            }
            crate::MatchPattern::Structural(pattern) => {
                let Some(metrics) = crate::pattern_metrics(pattern) else {
                    return Ok(None);
                };
                if metrics.depth >= 64
                    || charge(
                        analysis,
                        (metrics.nodes as u64).saturating_mul(256).saturating_add(
                            metrics.bindings as u64
                                * core::mem::size_of::<ClosedLexicalValue>() as u64,
                        ),
                        (metrics.nodes as u64).saturating_mul(metrics.nodes as u64),
                    )
                    .is_none()
                {
                    return Ok(None);
                }
                bindings.reserve_exact(metrics.bindings);
                let pattern = pattern.map(&|schema| *schema, &|reference| match reference {
                    crate::MatchPatternValue::Literal(id) => LexicalValue::Constant(*id),
                    crate::MatchPatternValue::Binding(local) => LexicalValue::Local(*local),
                    crate::MatchPatternValue::Input(input) => LexicalValue::Input(*input),
                });
                let target = pattern.map(
                    &|schema| comprehension::ActivatedPatternBinding {
                        region: ResidentRegion {
                            kind: ResidentValueKind::Snapshot,
                            offset: 0,
                            len: 1,
                            shape: ResidentShape {
                                rows: 1,
                                columns: 1,
                            },
                        },
                        schema: *schema,
                    },
                    &|reference| ActivatedPatternValue {
                        location: ResidentReadLocation::Constant(ResidentRegion {
                            kind: ResidentValueKind::Snapshot,
                            offset: 0,
                            len: 1,
                            shape: ResidentShape {
                                rows: 1,
                                columns: 1,
                            },
                        }),
                        schema: match reference {
                            LexicalValue::Constant(id) => {
                                artifact.constants().get(*id).map(Value::schema)
                            }
                            LexicalValue::Input(index) => {
                                inputs.get(*index as usize).map(|value| value.schema())
                            }
                            // Repeated bindings are already declared by this pattern.
                            LexicalValue::Local(local) => {
                                let mut selected = None;
                                pattern.bindings(&mut |index, schema| {
                                    if index == *local {
                                        selected = Some(*schema);
                                    }
                                });
                                selected
                            }
                        }
                        .unwrap_or(scrutinee.schema()),
                    },
                );
                let Ok(dynamic_depth) = collection::pattern_dynamic_target_depth(&target, schemas)
                else {
                    return Ok(None);
                };
                let region = ResidentRegion {
                    kind: ResidentValueKind::Snapshot,
                    offset: 0,
                    len: 1,
                    shape: ResidentShape {
                        rows: 1,
                        columns: 1,
                    },
                };
                let refs = [Some(scrutinee.as_ref().clone())];
                let meter = crate::resident::budget::ResidentBudgetMeter::for_closed_analysis(
                    mech_core::RESIDENT_MAX_COMPUTE_WORK
                        .saturating_sub(analysis.construction.compute_work.get()),
                    mech_core::RESIDENT_MAX_COMPARISON_WORK
                        .saturating_sub(analysis.construction.comparison_work.get()),
                );
                let prepared = collection::prepare_pattern_item_materialization(
                    ResidentValueRef::Snapshot(&refs),
                    region,
                    scrutinee.schema(),
                    false,
                    metrics.nodes as u64,
                    metrics.bindings as u64,
                    metrics.equalities as u64,
                    (metrics.bindings + metrics.equalities) as u64,
                    metrics.depth as u64,
                    dynamic_depth,
                    schemas,
                    meter,
                    Some(&mut |cost| {
                        if analysis.construction.charge_compute_with_comparison(
                            cost.temporary_bytes(),
                            cost.compute_work(),
                            cost.comparison_work(),
                        ) {
                            Ok(())
                        } else {
                            Err(ResidentKernelError::InvalidShape)
                        }
                    }),
                );
                let Ok(prepared) = prepared else {
                    return Ok(None);
                };
                if prepared
                    .prepare_for_analysis(|cost| {
                        if analysis.construction.charge_compute_with_comparison(
                            cost.temporary_bytes(),
                            cost.compute_work(),
                            cost.comparison_work(),
                        ) {
                            Ok(())
                        } else {
                            Err(ResidentKernelError::InvalidShape)
                        }
                    })
                    .is_err()
                {
                    return Ok(None);
                }
                analysis.collections.pattern_materializations.set(
                    analysis
                        .collections
                        .pattern_materializations
                        .get()
                        .saturating_add(1),
                );
                let mut materialization_meter =
                    crate::resident::budget::ResidentBudgetMeter::for_closed_analysis(
                        mech_core::RESIDENT_MAX_COMPUTE_WORK
                            .saturating_sub(analysis.construction.compute_work.get()),
                        mech_core::RESIDENT_MAX_COMPARISON_WORK
                            .saturating_sub(analysis.construction.comparison_work.get()),
                    );
                let Some(item) = collection::resident_pattern_item_with_admission(
                    ResidentValueRef::Snapshot(&refs),
                    region,
                    scrutinee.schema(),
                    scrutinee.shape().parameter_values(),
                    schemas,
                    false,
                    &mut materialization_meter,
                    &mut Some(&mut |cost| {
                        if analysis.construction.charge_compute_with_comparison(
                            cost.temporary_bytes(),
                            cost.compute_work(),
                            cost.comparison_work(),
                        ) {
                            Ok(())
                        } else {
                            Err(ResidentKernelError::InvalidShape)
                        }
                    }),
                ) else {
                    return Ok(None);
                };
                let budget = SnapshotCanonicalizationBudget::new(
                    mech_core::RESIDENT_MAX_COMPARISON_WORK
                        .saturating_sub(analysis.construction.comparison_work.get()),
                );
                let matched = match_pattern(
                    artifact,
                    &pattern,
                    &item,
                    scrutinee.shape().parameter_values(),
                    inputs,
                    &mut bindings,
                    schemas,
                    projections,
                    &budget,
                    analysis,
                )?;
                if !analysis
                    .construction
                    .charge_comparison_work(budget.consumed())
                {
                    return Ok(None);
                }
                matched
            }
        };
        match matches {
            None => return Ok(None),
            Some(false) => continue,
            Some(true) => {}
        }
        if let Some(guard) = &arm.guard {
            let Some(guard) =
                fold_block(artifact, control, guard, inputs, &bindings, analysis, depth)?
            else {
                return Ok(None);
            };
            match guard.data() {
                mech_core::ValueData::Bool(false) => continue,
                mech_core::ValueData::Bool(true) => {}
                _ => return Ok(None),
            }
        }
        let Some(result) = fold_block(
            artifact, control, &arm.body, inputs, &bindings, analysis, depth,
        )?
        else {
            return Ok(None);
        };
        if result.schema() != output || !analysis.construction.charge_clone(&result, schemas, 2) {
            return Ok(None);
        }
        // Match publication retains the selected block's complete value, not
        // an extent-derived replacement shape witness.
        return Ok(Some(result.as_ref().clone()));
    }
    Ok(None)
}

fn fold_block(
    artifact: &ProgramArtifact,
    control: &crate::MatchDeclaration,
    block: &crate::ControlBlock,
    captures: &[ClosedLexicalValue],
    bindings: &[ClosedLexicalValue],
    analysis: &ClosedActivationAnalysisContext,
    depth: usize,
) -> Result<Option<ClosedLexicalValue>> {
    if charge(
        analysis,
        (block.parameters.len() + block.operations.len()) as u64
            * core::mem::size_of::<ClosedLexicalValue>() as u64,
        block.parameters.len() as u64 + block.operations.len() as u64,
    )
    .is_none()
    {
        return Ok(None);
    }
    let parameters = block
        .parameters
        .iter()
        .map(|parameter| {
            let selected = match parameter.source {
                crate::ControlParameterSource::Scrutinee => {
                    captures.get(control.scrutinee as usize)
                }
                crate::ControlParameterSource::PatternBinding(local) => {
                    bindings.get(local as usize)
                }
                crate::ControlParameterSource::Capture(index) => control
                    .captures
                    .get(index as usize)
                    .and_then(|capture| captures.get(capture.input as usize)),
            }?;
            (parameter.schema == selected.schema()).then(|| selected.clone())
        })
        .collect::<Option<Vec<_>>>();
    let Some(parameters) = parameters else {
        return Ok(None);
    };
    let mut locals = Vec::with_capacity(block.operations.len());
    let resolve = |reference, locals: &[ClosedLexicalValue]| match reference {
        crate::ControlValue::Constant(id) => {
            value(artifact, LexicalValue::Constant(id), &[], &[], analysis)
        }
        crate::ControlValue::Parameter { block: id, ordinal } if id == block.id => {
            parameters.get(ordinal as usize).cloned()
        }
        crate::ControlValue::Local { block: id, node } if id == block.id => {
            locals.get(node as usize).cloned()
        }
        _ => None,
    };
    for operation in &block.operations {
        if operation.node as usize != locals.len()
            || charge(
                analysis,
                operation.inputs.len() as u64 * core::mem::size_of::<ClosedLexicalValue>() as u64,
                operation.inputs.len() as u64 + 1,
            )
            .is_none()
        {
            return Ok(None);
        }
        let Some(arguments) = operation
            .inputs
            .iter()
            .map(|input| resolve(*input, &locals))
            .collect::<Option<Vec<_>>>()
        else {
            return Ok(None);
        };
        let result = match &operation.body {
            crate::ControlOperationBody::Operation {
                operation: reference,
                contract,
            } => fold_operation(
                artifact,
                reference,
                *contract,
                operation.schema,
                &arguments,
                analysis,
            )?,
            crate::ControlOperationBody::Comprehension(control) => fold_declaration(
                artifact,
                control,
                operation.schema,
                &arguments,
                analysis,
                depth + 1,
            )?,
            crate::ControlOperationBody::Match(control) => fold_match(
                artifact,
                control,
                operation.schema,
                &arguments,
                analysis,
                depth + 1,
            )?,
            crate::ControlOperationBody::Recur(_)
            | crate::ControlOperationBody::Suspend
            | crate::ControlOperationBody::Publish => None,
        };
        let Some(result) = result else {
            return Ok(None);
        };
        locals.push(local_value(
            artifact,
            result,
            matches!(
                operation.body,
                crate::ControlOperationBody::Comprehension(_)
            ),
        )?);
    }
    Ok(resolve(block.yield_value, &locals))
}

fn fold_operation(
    artifact: &ProgramArtifact,
    reference: &OperationReference,
    contract: mech_core::OperationContractId,
    output: SchemaId,
    inputs: &[ClosedLexicalValue],
    analysis: &ClosedActivationAnalysisContext,
) -> Result<Option<Value>> {
    use crate::{
        ExecutableNodeBody, InputDeclaration, NodeDeclaration, OperationNodeBody,
        ProgramArtifactDraft, SlotDeclaration,
    };
    use mech_core::{BindingId, ConstantStoreBuilder, OperationContractTableBuilder};
    let Some(disposition) = closed_producer_disposition(reference) else {
        return Ok(None);
    };
    if disposition == ClosedProducerDisposition::DeferredStatefulMutation {
        return Ok(None);
    }
    let Some(mech_core::ResolvedOperationContract::Declared(declaration)) =
        artifact.contracts().get(contract)
    else {
        return Ok(None);
    };
    if declaration.interaction != ExternalInteraction::Pure
        || declaration.inputs.len() != inputs.len()
        || declaration.outputs.len() != 1
        || declaration.outputs[0].schema != output
        || declaration
            .inputs
            .iter()
            .zip(inputs)
            .any(|(port, value)| port.schema != value.schema())
    {
        return Ok(None);
    }
    // A one-operation metadata frame lets the existing fold owners consume
    // lexical values without assigning them artifact constants or executing a
    // runtime. Captures are proved closed above and seeded into this frame's
    // private value cache. Only immutable validated metadata is retained; no
    // frame is loaded or executed as a program.
    let cached_count = analysis.collections.frames.borrow().len() as u64;
    if charge(analysis, 0, cached_count + inputs.len() as u64 + 1).is_none() {
        return Ok(None);
    }
    let cached = analysis
        .collections
        .frames
        .borrow()
        .iter()
        .find(|entry| {
            entry.reference == *reference && entry.contract == contract && entry.output == output
        })
        .map(|entry| entry.artifact.clone());
    let frame = if let Some(frame) = cached {
        frame
    } else {
        let Some(schema_bytes) = artifact.schemas().clone_allocation_bound_bytes() else {
            return Ok(None);
        };
        let encoded = artifact
            .schemas()
            .entries()
            .try_fold(0u64, |n, e| n.checked_add(e.canonical_bytes().len() as u64));
        let ports = declaration.inputs.len() as u64 + declaration.outputs.len() as u64;
        let names = declaration.outputs.iter().try_fold(0u64, |n, port| {
            if let OutputConstruction::Build { postcondition } = &port.construction {
                n.checked_add(postcondition.contract_name.len() as u64)?
                    .checked_add(
                        postcondition
                            .module_path
                            .iter()
                            .map(|s| s.len() as u64 + core::mem::size_of::<String>() as u64)
                            .sum::<u64>(),
                    )
            } else {
                Some(n)
            }
        });
        let Some(bytes) = encoded
            .and_then(|n| n.checked_mul(4))
            .and_then(|n| n.checked_add(schema_bytes.checked_mul(2)?))
            .and_then(|n| n.checked_add(ports.checked_mul(512)?))
            .and_then(|n| n.checked_add(names?.checked_mul(4)?))
        else {
            return Ok(None);
        };
        if charge(analysis, bytes, bytes).is_none() {
            return Ok(None);
        }
        analysis.collections.frame_constructions.set(
            analysis
                .collections
                .frame_constructions
                .get()
                .saturating_add(1),
        );
        let mut builder = OperationContractTableBuilder::new();
        let Ok(handle) = builder.insert(mech_core::ResolvedOperationContract::Declared(
            declaration.clone(),
        )) else {
            return Ok(None);
        };
        let Ok(build) = builder.finish() else {
            return Ok(None);
        };
        let frame_contract = build
            .resolve(handle)
            .map_err(|_| ResidentActivationError::RegionSizeOverflow)?;
        let contracts = build.table;
        let mut slots = vec![SlotDeclaration {
            slot: CellSlotId::new(0),
            schema: output,
            role: SlotRole::Derived,
            producer: ProducerReference::NodeOutput {
                node: NodeId::new(0),
                output_ordinal: 0,
            },
            initializer: None,
        }];
        let mut captures = Vec::new();
        let mut bindings = Vec::new();
        for (index, value) in inputs.iter().enumerate() {
            let slot = CellSlotId::new(index as u32 + 1);
            slots.push(SlotDeclaration {
                slot,
                schema: value.schema(),
                role: SlotRole::Input,
                producer: ProducerReference::Input(InputId::new(index as u32)),
                initializer: None,
            });
            captures.push(InputDeclaration {
                input: InputId::new(index as u32),
                name: format!("capture{index}"),
                slot,
                schema: value.schema(),
            });
            bindings.push(BindingDeclaration::Input {
                id: BindingId::new(index as u32),
                node: NodeId::new(0),
                port_ordinal: index as u16,
                source: ArtifactSource::Slot(slot),
            });
        }
        let end = bindings.len() as u32;
        bindings.push(BindingDeclaration::Output {
            id: BindingId::new(end),
            node: NodeId::new(0),
            port_ordinal: 0,
            target: CellSlotId::new(0),
        });
        let schemas = artifact.schemas().clone();
        let constants = ConstantStoreBuilder::new(&schemas)
            .finish()
            .map_err(|_| ResidentActivationError::RegionSizeOverflow)?
            .store;
        let Ok(frame) = (ProgramArtifactDraft {
            schemas,
            constants,
            contracts,
            requirements: crate::ApplicationRequirementTable::empty(),
            inputs: captures.into_boxed_slice(),
            slots: slots.into_boxed_slice(),
            nodes: vec![NodeDeclaration {
                node: NodeId::new(0),
                body: ExecutableNodeBody::Operation(OperationNodeBody {
                    operation: reference.clone(),
                    contract: frame_contract,
                    requirement: None,
                }),
                input_bindings: 0..end,
                output_bindings: end..end + 1,
            }]
            .into_boxed_slice(),
            bindings: bindings.into_boxed_slice(),
            outputs: Box::new([]),
            constraints: Box::new([]),
            compute_regions: Box::new([]),
        })
        .finalize() else {
            return Ok(None);
        };
        let frame = Arc::new(frame);
        // Admission above includes the descriptor and Vec growth overlap.
        analysis
            .collections
            .frames
            .borrow_mut()
            .push(ClosedOperationFrame {
                reference: reference.clone(),
                contract,
                output,
                artifact: frame.clone(),
            });
        frame
    };
    let frame_bytes = inputs.iter().try_fold(2048u64, |bytes, input| {
        bytes
            .checked_add(512)?
            .checked_add(input.shape().parameter_values().len() as u64 * 8)
    });
    let Some(frame_bytes) = frame_bytes else {
        return Ok(None);
    };
    if charge(analysis, frame_bytes, inputs.len() as u64 + 1).is_none() {
        return Ok(None);
    }
    let mut facts = ActivationFacts::default();
    let frame_analysis = ClosedActivationAnalysisContext {
        construction: analysis.construction.clone(),
        collections: analysis.collections.clone(),
        ..Default::default()
    };
    for (index, value) in inputs.iter().enumerate() {
        let slot = CellSlotId::new(index as u32 + 1);
        frame_analysis
            .values
            .borrow_mut()
            .insert(ArtifactSource::Slot(slot), value.snapshot.clone());
        facts.slot_shapes.insert(slot, value.shape().clone());
        if value.kind == ResidentValueKind::Snapshot {
            facts.resident_snapshot_slots.insert(slot);
        }
    }
    if frame.schemas().get(output).is_some_and(|schema| {
        matches!(schema.body(), SchemaBody::Matrix { .. })
            && schema
                .dimension_parameters()
                .iter()
                .any(|parameter| parameter.lifetime() == DimensionLifetime::Turn)
    }) {
        facts.resident_snapshot_slots.insert(CellSlotId::new(0));
    }
    let schedule = build_activation_schedule(&frame, &[NodeClass::Activation])?;
    let facts = complete_activation_shape_facts_with_analysis(
        &frame,
        &facts,
        &[NodeClass::Activation],
        &schedule,
        &frame_analysis,
    )?;
    if disposition == ClosedProducerDisposition::ExactMask {
        let mask = if reference.module_path.as_ref() == ["compare"] {
            closed_comparison_mask(&frame, NodeId::new(0), &facts, &frame_analysis)?
        } else {
            let sources = node_inputs(&frame, NodeId::new(0))?;
            for source in &sources {
                if let Some(mask) = closed_boolean_producer_mask(
                    &frame,
                    NodeId::new(0),
                    *source,
                    &facts,
                    &frame_analysis,
                )? {
                    frame_analysis.logical_masks.insert(*source, mask);
                }
            }
            match sources.as_slice() {
                [source] => frame_analysis
                    .logical_masks
                    .get(source)
                    .and_then(|mask| mask.negated(&frame_analysis)),
                [left, right] => frame_analysis
                    .logical_masks
                    .get(left)
                    .zip(frame_analysis.logical_masks.get(right))
                    .and_then(|(left, right)| {
                        left.binary(&right, &reference.operation_name, &frame_analysis)
                    }),
                _ => None,
            }
        };
        let Some(mask) = mask else {
            return Ok(None);
        };
        return closed_logical_mask_value(
            &frame,
            ArtifactSource::Slot(CellSlotId::new(0)),
            &mask,
            &facts,
            &SnapshotCanonicalizationBudget::new(mech_core::RESIDENT_MAX_COMPARISON_WORK),
            &frame_analysis,
            false,
        );
    }
    Ok(constant_comparison_operand(
        &frame,
        NodeId::new(0),
        ArtifactSource::Slot(CellSlotId::new(0)),
        &facts,
        &frame_analysis,
        true,
    )?
    .map(|value| value.value.into_owned()))
}

#[cfg(all(test, feature = "full_source"))]
mod tests {
    use super::*;

    #[test]
    fn pattern_budget_exhaustion_is_not_an_empty_collection_nonmatch() {
        let artifact =
            super::super::shape_fact_tests::closed_collection_source("[x | x <- [true false]]\n");
        let schema = artifact
            .schemas()
            .entries()
            .enumerate()
            .find(|(_, entry)| entry.schema().body() == &SchemaBody::Bool)
            .map(|(index, _)| SchemaId::new(index as u32))
            .unwrap();
        let constant = (0..artifact.constants().len())
            .map(|index| mech_core::ConstantId::new(index as u32))
            .find(|id| {
                artifact
                    .constants()
                    .get(*id)
                    .is_some_and(|value| value.schema() == schema)
            })
            .unwrap();
        let analysis = ClosedActivationAnalysisContext::default();
        let (schemas, projections) = projection_context(&artifact, &analysis).unwrap();
        let item = collection::PatternItem::new(ValueDataDraft::Bool(true));
        analysis
            .construction
            .compute_work
            .set(mech_core::RESIDENT_MAX_COMPUTE_WORK);
        for pattern in [
            Pattern::Bind { local: 0, schema },
            Pattern::Equal(LexicalValue::Constant(constant)),
        ] {
            let mut locals = Vec::new();
            let outcome = match_pattern(
                &artifact,
                &pattern,
                &item,
                &[],
                &[],
                &mut locals,
                schemas,
                projections,
                &SnapshotCanonicalizationBudget::new(mech_core::RESIDENT_MAX_COMPARISON_WORK),
                &analysis,
            )
            .unwrap();
            assert_eq!(
                outcome, None,
                "a refusal must not skip the generator as a nonmatch"
            );
            assert!(locals.is_empty());
        }
    }

    #[test]
    fn lexical_operations_share_metadata_but_not_iteration_values() {
        let artifact =
            super::super::shape_fact_tests::closed_collection_source("[x + 10 | x <- [1 2 3 4]]\n");
        let node = artifact
            .nodes()
            .iter()
            .find(|node| matches!(node.body, crate::ExecutableNodeBody::Comprehension(_)))
            .unwrap();
        let slot = node_output_slot(&artifact, node.node).unwrap();
        let analysis = ClosedActivationAnalysisContext::default();
        let folded = constant_comparison_operand(
            &artifact,
            node.node,
            ArtifactSource::Slot(slot),
            &ActivationFacts::default(),
            &analysis,
            true,
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            folded.value.canonical_data_draft().unwrap(),
            ValueDataDraft::Matrix(
                [11.0, 12.0, 13.0, 14.0]
                    .into_iter()
                    .map(|n| ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(n)))
                    .collect(),
            )
        );
        assert_eq!(analysis.collections.frame_constructions.get(), 2);
        assert_eq!(analysis.collections.pattern_materializations.get(), 4);
        assert!(analysis.construction.compute_work.get() > 4);
    }
}
