use std::collections::BTreeMap;

use mech_core::{
    CellSlotId, DimensionExpr, OutputConstruction, ResolvedOperationContract, ResolvedRangeMode,
    Schema, SchemaBody, ShapeInstance, ShapeRule, Value, ValueData, canonical_positional_ordinal,
    canonical_value_range_size, shape_for_resolved_extents,
};
use mech_engine::{
    ArtifactSource, BindingDeclaration, InitializerReference, ProducerReference, ProgramArtifact,
};

use crate::{ConcatenationAxis, ElementwiseLowering, elementwise_lowering};

/// Resolves the current scalar/matrix dimensions carried by an artifact.
///
/// Semantic schemas retain their declared dynamic dimensions. Compute storage
/// is fixed for one compiled region, so target planning closes those schemas
/// from authoritative constant/state shapes and propagates the resulting
/// dimensions through the already-resolved elementwise operation graph.
pub fn resolve_compute_slot_dimensions(
    artifact: &ProgramArtifact,
) -> BTreeMap<CellSlotId, Box<[u64]>> {
    let mut resolved = BTreeMap::new();
    for slot in artifact.slots() {
        let Some(schema) = artifact.schemas().get(slot.schema) else {
            continue;
        };
        let dimensions = fixed_schema_dimensions(schema)
            .or_else(|| {
                artifact
                    .slot_shape_hint(slot.slot)
                    .and_then(|shape| closed_schema_dimensions(schema, shape))
            })
            .or_else(|| {
                let InitializerReference::Constant(constant) = slot.initializer?;
                let value = artifact.constants().get(constant)?;
                value_dimensions(artifact, value)
            });
        if let Some(dimensions) = dimensions {
            resolved.insert(slot.slot, dimensions);
        }
    }

    loop {
        let mut changed = false;
        for node in artifact.nodes() {
            let outputs = node
                .output_bindings
                .clone()
                .filter_map(|index| match artifact.bindings().get(index as usize) {
                    Some(BindingDeclaration::Output { target, .. }) => Some(*target),
                    _ => None,
                })
                .collect::<Vec<_>>();
            if outputs.is_empty() {
                continue;
            }
            if let Some(mode) = node.operation.resolved_range_mode() {
                let inputs = node
                    .input_bindings
                    .clone()
                    .filter_map(|index| match artifact.bindings().get(index as usize) {
                        Some(BindingDeclaration::Input { source, .. }) => {
                            static_range_endpoint(artifact, *source)
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                let (inclusive, incremented) = match mode {
                    ResolvedRangeMode::Exclusive => (false, false),
                    ResolvedRangeMode::ExclusiveIncrement => (false, true),
                    ResolvedRangeMode::Inclusive => (true, false),
                    ResolvedRangeMode::InclusiveIncrement => (true, true),
                };
                if let Ok(count) = canonical_value_range_size(&inputs, inclusive, incremented) {
                    for output in outputs {
                        let Some(schema) = artifact
                            .slots()
                            .get(output.get() as usize)
                            .and_then(|slot| artifact.schemas().get(slot.schema))
                        else {
                            continue;
                        };
                        let Some(dimensions) = dimensions_for_extents(schema, &[1, count as u64])
                        else {
                            continue;
                        };
                        changed |= resolved.insert(output, dimensions).is_none();
                    }
                }
                continue;
            }
            changed |= propagate_contract_dimensions(artifact, &mut resolved, node);
            let Some(lowering) = elementwise_lowering(&node.operation) else {
                continue;
            };
            let inputs = node
                .input_bindings
                .clone()
                .filter_map(|index| match artifact.bindings().get(index as usize) {
                    Some(BindingDeclaration::Input { source, .. }) => Some(*source),
                    _ => None,
                })
                .map(|source| source_dimensions(artifact, &resolved, source))
                .collect::<Option<Vec<_>>>();
            let Some(inputs) = inputs else {
                continue;
            };
            let Some(dimensions) = lowering_dimensions(lowering, &inputs) else {
                continue;
            };
            for output in outputs {
                let Some(schema) = artifact
                    .slots()
                    .get(output.get() as usize)
                    .and_then(|slot| artifact.schemas().get(slot.schema))
                else {
                    continue;
                };
                let Some(candidate) = dimensions_for_extents(schema, &dimensions) else {
                    continue;
                };
                changed |= resolved.insert(output, candidate).is_none();
            }
        }
        if !changed {
            break;
        }
    }
    resolved
}

fn propagate_contract_dimensions(
    artifact: &ProgramArtifact,
    resolved: &mut BTreeMap<CellSlotId, Box<[u64]>>,
    node: &mech_engine::NodeDeclaration,
) -> bool {
    let inputs = node
        .input_bindings
        .clone()
        .filter_map(|index| match artifact.bindings().get(index as usize) {
            Some(BindingDeclaration::Input { source, .. }) => Some(*source),
            _ => None,
        })
        .collect::<Vec<_>>();
    let outputs = node
        .output_bindings
        .clone()
        .filter_map(|index| match artifact.bindings().get(index as usize) {
            Some(BindingDeclaration::Output { target, .. }) => Some(*target),
            _ => None,
        })
        .collect::<Vec<_>>();
    let Some(ResolvedOperationContract::Declared(contract)) =
        artifact.contracts().get(node.contract)
    else {
        return false;
    };
    let mut changed = false;
    for (output, port) in outputs.into_iter().zip(contract.outputs.iter()) {
        let rule = match &port.construction {
            OutputConstruction::FullWrite { shape } | OutputConstruction::Replace { shape } => {
                Some(*shape)
            }
            OutputConstruction::ReadModifyWrite { base_input, .. } => {
                Some(ShapeRule::SameAsInput { input: *base_input })
            }
            OutputConstruction::Build { .. } => None,
        };
        let Some(rule) = rule else {
            continue;
        };
        changed |= propagate_shape_rule(artifact, resolved, &inputs, output, rule);
    }
    changed
}

fn propagate_shape_rule(
    artifact: &ProgramArtifact,
    resolved: &mut BTreeMap<CellSlotId, Box<[u64]>>,
    inputs: &[ArtifactSource],
    output: CellSlotId,
    rule: ShapeRule,
) -> bool {
    match rule {
        ShapeRule::Declared => false,
        ShapeRule::SameAsInput { input } => propagate_equal_dimensions(
            artifact,
            resolved,
            inputs.get(input as usize).copied(),
            output,
            false,
        ),
        ShapeRule::TransposeOf { input } => propagate_equal_dimensions(
            artifact,
            resolved,
            inputs.get(input as usize).copied(),
            output,
            true,
        ),
        ShapeRule::MatrixProduct { lhs, rhs } => {
            let Some(lhs) = inputs
                .get(lhs as usize)
                .and_then(|source| source_dimensions(artifact, resolved, *source))
            else {
                return false;
            };
            let Some(rhs) = inputs
                .get(rhs as usize)
                .and_then(|source| source_dimensions(artifact, resolved, *source))
            else {
                return false;
            };
            let ([rows, inner], [rhs_inner, columns]) = (lhs.as_ref(), rhs.as_ref()) else {
                return false;
            };
            if inner != rhs_inner {
                return false;
            }
            insert_slot_extents(artifact, resolved, output, &[*rows, *columns])
        }
    }
}

fn propagate_equal_dimensions(
    artifact: &ProgramArtifact,
    resolved: &mut BTreeMap<CellSlotId, Box<[u64]>>,
    input: Option<ArtifactSource>,
    output: CellSlotId,
    transpose: bool,
) -> bool {
    let Some(input) = input else {
        return false;
    };
    if let Some(mut dimensions) = source_dimensions(artifact, resolved, input) {
        if transpose && dimensions.len() == 2 {
            dimensions.swap(0, 1);
        }
        return insert_slot_extents(artifact, resolved, output, &dimensions);
    }
    let ArtifactSource::Slot(input_slot) = input else {
        return false;
    };
    let Some(mut dimensions) = resolved.get(&output).cloned() else {
        return false;
    };
    if transpose && dimensions.len() == 2 {
        dimensions.swap(0, 1);
    }
    insert_slot_extents(artifact, resolved, input_slot, &dimensions)
}

fn insert_slot_extents(
    artifact: &ProgramArtifact,
    resolved: &mut BTreeMap<CellSlotId, Box<[u64]>>,
    slot: CellSlotId,
    extents: &[u64],
) -> bool {
    if resolved.contains_key(&slot) {
        return false;
    }
    let Some(schema) = artifact
        .slots()
        .get(slot.get() as usize)
        .and_then(|slot| artifact.schemas().get(slot.schema))
    else {
        return false;
    };
    let Some(dimensions) = dimensions_for_extents(schema, extents) else {
        return false;
    };
    resolved.insert(slot, dimensions);
    true
}

fn static_range_endpoint(artifact: &ProgramArtifact, source: ArtifactSource) -> Option<ValueData> {
    const MAX_STATIC_ENDPOINT_STEPS: usize = 65_536;

    let mut source = source;
    let mut converted_to_index = false;
    for _ in 0..MAX_STATIC_ENDPOINT_STEPS {
        match source {
            ArtifactSource::Constant(constant) => {
                let value = artifact.constants().get(constant)?.data();
                return if converted_to_index {
                    canonical_positional_ordinal(value)
                        .ok()
                        .map(ValueData::Index)
                } else {
                    Some(value.clone())
                };
            }
            ArtifactSource::Slot(slot) => {
                let declaration = artifact.slots().get(slot.get() as usize)?;
                let ProducerReference::NodeOutput { node, .. } = declaration.producer else {
                    return None;
                };
                let node = artifact.nodes().get(node.get() as usize)?;
                if node.operation.canonical_name() != "access/index" {
                    return None;
                }
                let mut inputs = node.input_bindings.clone().filter_map(|binding| {
                    match artifact.bindings().get(binding as usize) {
                        Some(BindingDeclaration::Input { source, .. }) => Some(*source),
                        _ => None,
                    }
                });
                source = inputs.next()?;
                if inputs.next().is_some() {
                    return None;
                }
                converted_to_index = true;
            }
        }
    }
    None
}

fn source_dimensions(
    artifact: &ProgramArtifact,
    resolved: &BTreeMap<CellSlotId, Box<[u64]>>,
    source: ArtifactSource,
) -> Option<Box<[u64]>> {
    match source {
        ArtifactSource::Slot(slot) => resolved.get(&slot).cloned(),
        ArtifactSource::Constant(constant) => {
            value_dimensions(artifact, artifact.constants().get(constant)?)
        }
    }
}

fn value_dimensions(artifact: &ProgramArtifact, value: &Value) -> Option<Box<[u64]>> {
    let schema = artifact.schemas().get(value.schema())?;
    closed_schema_dimensions(schema, value.shape())
}

fn fixed_schema_dimensions(schema: &Schema) -> Option<Box<[u64]>> {
    match schema.body() {
        SchemaBody::FloatingPoint(_) => Some(Box::new([])),
        SchemaBody::Matrix { dimensions, .. } => dimensions
            .iter()
            .map(|dimension| match dimension {
                DimensionExpr::Constant(extent) => Some(*extent),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()
            .map(Vec::into_boxed_slice),
        _ => None,
    }
}

fn closed_schema_dimensions(schema: &Schema, shape: &ShapeInstance) -> Option<Box<[u64]>> {
    match schema.closed_body(shape).ok()? {
        SchemaBody::FloatingPoint(_) => Some(Box::new([])),
        SchemaBody::Matrix { dimensions, .. } => dimensions
            .iter()
            .map(|dimension| match dimension {
                DimensionExpr::Constant(extent) => Some(*extent),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()
            .map(Vec::into_boxed_slice),
        _ => None,
    }
}

fn dimensions_for_extents(schema: &Schema, extents: &[u64]) -> Option<Box<[u64]>> {
    let shape = shape_for_resolved_extents(schema, extents).ok()?;
    let dimensions = closed_schema_dimensions(schema, &shape)?;
    (dimensions.as_ref() == extents).then_some(dimensions)
}

fn lowering_dimensions(lowering: ElementwiseLowering, inputs: &[Box<[u64]>]) -> Option<Box<[u64]>> {
    match lowering {
        ElementwiseLowering::Apply(_) => {
            let mut rows = 1_u64;
            let mut columns = 1_u64;
            let mut matrix = false;
            for dimensions in inputs {
                match dimensions.as_ref() {
                    [] => {}
                    [candidate_rows, candidate_columns] => {
                        matrix = true;
                        rows = rows.max(*candidate_rows);
                        columns = columns.max(*candidate_columns);
                    }
                    _ => return None,
                }
            }
            Some(if matrix {
                vec![rows, columns].into_boxed_slice()
            } else {
                Box::new([])
            })
        }
        ElementwiseLowering::Concatenate(axis) => {
            let mut common = None;
            let mut varying = 0_u64;
            for dimensions in inputs {
                let (rows, columns) = match dimensions.as_ref() {
                    [] => (1, 1),
                    [rows, columns] => (*rows, *columns),
                    _ => return None,
                };
                let (candidate_common, candidate_varying) = match axis {
                    ConcatenationAxis::Horizontal => (rows, columns),
                    ConcatenationAxis::Vertical => (columns, rows),
                };
                if common.is_some_and(|common| common != candidate_common) {
                    return None;
                }
                common = Some(candidate_common);
                varying = varying.checked_add(candidate_varying)?;
            }
            let common = common?;
            Some(match axis {
                ConcatenationAxis::Horizontal => vec![common, varying].into_boxed_slice(),
                ConcatenationAxis::Vertical => vec![varying, common].into_boxed_slice(),
            })
        }
    }
}
