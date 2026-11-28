//! Allocation-free coexistence bounds for maintained shallow F64 arithmetic.
//! These bounds select optional metadata only. They never authorize a kernel,
//! replace its current facts, or turn a cache refusal into a source error.
use super::{CurrentCallBound, add, total};
use crate::native_artifact::{
    NativeCollectionPrimitive as Primitive, NativeProgramLibraryTemplate,
};
use crate::resident::budget;
use mech_core::{
    CurrentMemoryFootprint, ManagedMemoryBudget, Schema, SchemaBody, SchemaTable, Value, ValueData,
    ValueDataDraft,
    snapshot::{SequenceView, ValueFootprint},
};

fn f64(schema: &Schema) -> bool {
    match schema.body() {
        SchemaBody::FloatingPoint(mech_core::FloatWidth::W64) => {
            schema.dimension_parameters().is_empty()
        }
        SchemaBody::Matrix {
            element,
            dimensions,
        } => {
            dimensions.len() == 2
                && element.as_ref() == &SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)
        }
        _ => false,
    }
}

pub(super) fn supports(template: &NativeProgramLibraryTemplate) -> bool {
    let arity = match template.primitive() {
        Primitive::MatrixTranspose
        | Primitive::MatrixSqrt
        | Primitive::SumColumns
        | Primitive::SumRows => 1,
        Primitive::MatrixMatmul
        | Primitive::MatrixAdd
        | Primitive::MatrixSubtract
        | Primitive::MatrixMultiply
        | Primitive::MatrixDivide
        | Primitive::F64Power => 2,
        _ => return false,
    };
    template.inputs().len() == arity && template.inputs().iter().all(f64) && f64(template.output())
}

// Return only borrowed, already admitted geometry. Even malformed geometry is
// a cache miss; the original live binder retains its own rejection order.
fn dimensions(value: &Value, schema: &Schema) -> Option<[u64; 2]> {
    if value.shape().parameter_values().len() != schema.dimension_parameters().len() {
        return None;
    }
    match (schema.body(), value.data()) {
        (SchemaBody::FloatingPoint(mech_core::FloatWidth::W64), ValueData::F64(_)) => Some([1, 1]),
        (SchemaBody::Matrix { dimensions, .. }, ValueData::Matrix(matrix)) => {
            let SequenceView::F64(cells) = matrix.elements() else {
                return None;
            };
            let [row, column] = dimensions.as_ref() else {
                return None;
            };
            // The existing fixed per-port probe funds two atomic axis
            // lookups. Recursive dimension expressions retain ordinary
            // planning; this optional preview must not walk an unfunded tree.
            let axis = |expression: &mech_core::DimensionExpr| match expression {
                mech_core::DimensionExpr::Constant(_) | mech_core::DimensionExpr::Parameter(_) => {
                    value.shape().resolve_dimension(expression).ok()
                }
                _ => None,
            };
            let rows = axis(row)?;
            let columns = axis(column)?;
            (rows.checked_mul(columns)? == cells.len() as u64).then_some([rows, columns])
        }
        _ => None,
    }
}

pub(super) fn current(
    template: &NativeProgramLibraryTemplate,
    schemas: &SchemaTable,
    inputs: &[Option<Value>],
    output: &Option<Value>,
    memory: &ManagedMemoryBudget,
) -> Option<CurrentCallBound> {
    if !supports(template) || inputs.len() != template.inputs().len() {
        return None;
    }
    let mut source_bytes = 0;
    let mut source_nodes = 0;
    let mut elements = 0;
    let mut first = [1, 1];
    let mut last = [1, 1];
    let mut widest = [1, 1];
    for (index, (input, schema)) in inputs.iter().zip(template.inputs()).enumerate() {
        let input = input.as_ref()?;
        let dims = dimensions(input, schema)?;
        if index == 0 {
            first = dims;
        }
        last = dims;
        widest = [widest[0].max(dims[0]), widest[1].max(dims[1])];
        elements = add(elements, dims[0].checked_mul(dims[1])?)?;
        source_bytes = add(source_bytes, input.memory_budget_retained_bytes(memory)?)?;
        source_nodes = add(source_nodes, input.memory_budget_retained_nodes(memory)?)?;
    }
    // Every legal result in these selected families has this population (or a
    // smaller one for empty-axis broadcasting). Shape compatibility is still
    // validated by the maintained binder and kernel, never by this estimate.
    let output_elements = match template.primitive() {
        Primitive::MatrixTranspose | Primitive::MatrixSqrt => first[0].checked_mul(first[1])?,
        Primitive::SumColumns => first[0],
        Primitive::SumRows => first[1],
        Primitive::MatrixMatmul => first[0].checked_mul(last[1])?,
        _ => widest[0].checked_mul(widest[1])?,
    };
    let matrix = matches!(template.output().body(), SchemaBody::Matrix { .. });
    let output_elements = if matrix { output_elements } else { 1 };
    let shape_parameters = template.output().dimension_parameters().len();
    let canonical_data = ValueFootprint {
        encoded_bytes: output_elements.checked_mul(8)?,
        retained_bytes: add(
            core::mem::size_of::<ValueData>() as u64,
            if matrix {
                output_elements.checked_mul(8)?
            } else {
                0
            },
        )?,
        node_count: if matrix { 2 } else { 1 },
    };
    let published =
        budget::projected_canonical_value_footprint(canonical_data, shape_parameters).ok()?;
    // Numeric preflight deliberately keeps one ValueDataDraft per candidate
    // cell in its output envelope, even when final canonical storage is packed.
    // Bound BOTH that declared envelope and the actual published snapshot.
    let draft_width = core::mem::size_of::<ValueDataDraft>() as u64;
    let declared_output = output_elements.checked_mul(draft_width)?;
    let published_bytes = published
        .retained_bytes
        .max(declared_output)
        .max(source_bytes);
    let mut current_bytes = source_bytes;
    let mut current_nodes = source_nodes;
    let mut old_bytes = 0;
    if let Some(old) = output {
        dimensions(old, template.output())?;
        old_bytes = old.memory_budget_retained_bytes(memory)?;
        current_bytes = add(current_bytes, old_bytes)?;
        current_nodes = add(current_nodes, old.memory_budget_retained_nodes(memory)?)?;
    }
    let prospective = CurrentMemoryFootprint {
        logical_elements: output_elements,
        payload_bytes: published_bytes,
        encoded_bytes: published_bytes,
        retained_nodes: published.node_count,
        shape_parameter_count: shape_parameters as u64,
        schema_bytes: add(schemas.clone_allocation_bound_bytes()?, old_bytes)?,
        ..CurrentMemoryFootprint::default()
    };
    let handle = core::mem::size_of::<Option<Value>>() as u64;
    let slots = (inputs.len() as u64).checked_add(2)?.checked_mul(handle)?;
    let draft = mech_core::canonical_snapshot_draft_bytes(prospective)
        .ok()?
        .max(add(published_bytes, handle)?);
    let finalization = mech_core::canonical_snapshot_finalization_bytes(prospective).ok()?;
    // Covers transpose's two source copies and result draft, and arithmetic's
    // retained operands plus all input/result draft cells. Summing both is a
    // conservative coexistence proof across those maintained mutation phases.
    let kernel = total([
        source_bytes.checked_mul(2)?,
        add(elements, output_elements)?.checked_mul(draft_width)?,
        published_bytes.checked_mul(2)?,
    ])?;
    let temporary_bytes = total([
        current_bytes,
        slots,
        published_bytes.checked_mul(2)?,
        draft,
        finalization,
        kernel,
        Value::memory_budget_claim_metadata_bytes(),
    ])?;
    let retained_nodes = total([
        current_nodes,
        source_nodes.checked_mul(2)?,
        published.node_count.checked_mul(4)?,
        elements,
        output_elements,
    ])?;
    Some(CurrentCallBound {
        temporary_bytes,
        retained_nodes,
    })
}
