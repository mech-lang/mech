//! Optional cache headroom for two maintained shallow publication families.
//!
//! These are upper bounds for deciding whether optional metadata may coexist
//! with the current call. They are never measured facts, kernel permits, or
//! publication authority. The ordinary binder/planner/kernel still run.
use crate::native_artifact::{
    NativeCollectionPrimitive as Primitive, NativeProgramLibraryTemplate,
};
use crate::resident::budget;
use mech_core::{
    CurrentMemoryFootprint, IntegerWidth, ManagedMemoryBudget, Schema, SchemaBody, SchemaTable,
    Value, ValueData, ValueDataDraft, snapshot::SequenceView,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct CurrentCallBound {
    /// Complete current payload/old-publication and candidate coexistence.
    /// Fixed binding, invocation, cache and scope-owner metadata is separate.
    pub temporary_bytes: u64,
    pub retained_nodes: u64,
}

fn string_matrix(schema: &Schema) -> bool {
    matches!(schema.body(), SchemaBody::Matrix { element, dimensions }
        if element.as_ref() == &SchemaBody::String && dimensions.len() == 2)
}

fn scalar_u64(schema: &Schema) -> bool {
    schema.body() == &SchemaBody::UnsignedInteger(IntegerWidth::W64)
        && schema.dimension_parameters().is_empty()
}

/// Unsupported families keep the existing complete-ceiling capacity proof.
/// A supported family whose current proof fails must instead decline/evict
/// optional metadata, retaining the ordinary call's own rejection order.
pub(super) fn supports_template(template: &NativeProgramLibraryTemplate) -> bool {
    match (template.primitive(), template.inputs()) {
        (Primitive::SequenceLength, [source]) => {
            string_matrix(source) && scalar_u64(template.output())
        }
        (Primitive::MatrixScalarAccess, [source, selector]) => {
            string_matrix(source)
                && scalar_u64(selector)
                && template.output().body() == &SchemaBody::String
                && template.output().dimension_parameters().is_empty()
        }
        _ => false,
    }
}

fn add(left: u64, right: u64) -> Option<u64> {
    left.checked_add(right)
}

fn total(values: impl IntoIterator<Item = u64>) -> Option<u64> {
    values.into_iter().try_fold(0, add)
}

/// Allocation-free current-call capacity proof. No error is emitted here:
/// malformed selectors, wrong owners and arithmetic overflow all return None.
/// In particular a selector failure must not precede the maintained call's
/// current-facts/work checks merely because optional caching was considered.
pub(super) fn current_call_bound(
    template: &NativeProgramLibraryTemplate,
    schemas: &SchemaTable,
    inputs: &[Option<Value>],
    output: &Option<Value>,
    memory: &ManagedMemoryBudget,
) -> Option<CurrentCallBound> {
    if !supports_template(template) || inputs.len() != template.inputs().len() {
        return None;
    }
    let source = inputs.first()?.as_ref()?;
    if source.shape().parameter_values().len() != template.inputs()[0].dimension_parameters().len()
    {
        return None;
    }
    let ValueData::Matrix(matrix) = source.data() else {
        return None;
    };
    let SequenceView::String(strings) = matrix.elements() else {
        return None;
    };
    let source_bytes = source.memory_budget_retained_bytes(memory)?;
    let source_nodes = source.memory_budget_retained_nodes(memory)?;
    let mut current_bytes = source_bytes;
    let mut current_nodes = source_nodes;
    let mut selector_bytes = 0;
    let mut selector_nodes = 0;
    let selecting = template.primitive() == Primitive::MatrixScalarAccess;
    if selecting {
        let selector = inputs.get(1)?.as_ref()?;
        if !selector.shape().parameter_values().is_empty() {
            return None;
        }
        let ValueData::U64(index) = selector.data() else {
            return None;
        };
        let index = usize::try_from(index.checked_sub(1)?).ok()?;
        if index >= strings.len() {
            return None;
        }
        selector_bytes = selector.memory_budget_retained_bytes(memory)?;
        selector_nodes = selector.memory_budget_retained_nodes(memory)?;
        current_bytes = add(current_bytes, selector_bytes)?;
        current_nodes = add(current_nodes, selector_nodes)?;
    }
    let mut old_bytes = 0;
    if let Some(old) = output {
        if !old.shape().parameter_values().is_empty()
            || !matches!(
                (selecting, old.data()),
                (true, ValueData::String(_)) | (false, ValueData::U64(_))
            )
        {
            return None;
        }
        old_bytes = old.memory_budget_retained_bytes(memory)?;
        current_bytes = add(current_bytes, old_bytes)?;
        current_nodes = add(current_nodes, old.memory_budget_retained_nodes(memory)?)?;
    }

    // A selected String's bytes are at most the already admitted complete
    // source. This deliberately avoids a cell/payload walk and also covers
    // column-major positional selection on every rank-two matrix shape.
    let published = if selecting {
        mech_core::snapshot::prospective_string_value_footprint(0, false, 1, source_bytes).ok()?
    } else {
        budget::projected_canonical_value_footprint(
            mech_core::snapshot::ValueFootprint {
                encoded_bytes: 8,
                retained_bytes: core::mem::size_of::<ValueData>() as u64,
                node_count: 1,
            },
            0,
        )
        .ok()?
    };
    let published_bytes = published.retained_bytes;
    let published_nodes = published.node_count;
    let handle_bytes = core::mem::size_of::<Option<Value>>() as u64;

    // apply_observed_turn_demand conservatively uses retained candidate bytes
    // as encoded bytes until publication. Its finalizer also keeps the old
    // output's schema witness. The same-account old-root bound covers that
    // witness even when its arena has unrelated retained schemas. The actual
    // maintained output arena is separately included through the precomputed
    // O(1) SchemaTable allocation witness.
    let prospective = CurrentMemoryFootprint {
        logical_elements: 1,
        payload_bytes: published_bytes,
        encoded_bytes: published_bytes,
        retained_nodes: published_nodes,
        schema_bytes: add(schemas.clone_allocation_bound_bytes()?, old_bytes)?,
        ..CurrentMemoryFootprint::default()
    };
    let draft = mech_core::canonical_snapshot_draft_bytes(prospective).ok()?;
    let finalization = mech_core::canonical_snapshot_finalization_bytes(prospective).ok()?;

    // CanonicalFinalize has one output and one transaction candidate, one
    // max(value,draft) workspace, and one finalization workspace (Core derive).
    // Explicit native Option<Value> slots cover all input/output/stage handles;
    // admitted roots already cover their actual canonical payloads and owners.
    let slots = (inputs.len() as u64)
        .checked_add(2)?
        .checked_mul(handle_bytes)?;
    let output_and_stage = published_bytes.checked_mul(2)?;
    let draft = draft.max(add(published_bytes, handle_bytes)?);

    // SAME maintained kernel envelopes:
    // - pure_text::publish_u64: one published Value, no selector/container work.
    // - numeric::snapshot_access_from_source: 2*published, one ValueDataDraft,
    //   one selector Value, selector's retained bytes, and one usize coordinate.
    //   This is linear scalar access to packed Strings: no dense source,
    //   aggregate closure, component expansion, or selector matrix exists.
    let kernel = if selecting {
        total([
            published_bytes.checked_mul(2)?,
            core::mem::size_of::<ValueDataDraft>() as u64,
            core::mem::size_of::<Value>() as u64,
            selector_bytes,
            core::mem::size_of::<usize>() as u64,
        ])?
    } else {
        published_bytes
    };
    let temporary_bytes = total([
        current_bytes,
        slots,
        output_and_stage,
        draft,
        finalization,
        kernel,
        // The maintained Snapshot payload scope adds this import metadata
        // before constructing/transferring the independently owned result.
        Value::memory_budget_claim_metadata_bytes(),
    ])?;

    // Core output+transaction populations and the exact maintained mutation
    // phases are both covered. For String scalar access the latter contains
    // current source/prior/selector, normalized selector, one draft String,
    // one staged Value, and one final Value (see snapshot_access_node_phases).
    let canonical_nodes = add(current_nodes, published_nodes.checked_mul(2)?)?;
    let kernel_nodes = if selecting {
        total([
            current_nodes,
            selector_nodes,
            1,               // one selected String draft node
            published_nodes, // staged Value
            published_nodes, // final Value
        ])?
    } else {
        add(current_nodes, published_nodes)?
    };
    Some(CurrentCallBound {
        temporary_bytes,
        retained_nodes: canonical_nodes.max(kernel_nodes),
    })
}
