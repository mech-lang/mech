//! Structural binding for genuine counted application. The carrier is the
//! owned private carrier region; body locals have separate fixed arena lanes.
//! Runtime shape and payload remain owned by canonical snapshots where needed.
use super::*;

pub(super) const CARRIER_LOCAL: u32 = u32::MAX;

type LocalDefinition = (bool, bool, u32, u32, SchemaId);

pub(super) fn all_local_definitions(control: &crate::IterationDeclaration) -> Vec<LocalDefinition> {
    // This is a private layout identity for an implicit canonical parameter,
    // outside the artifact's bounded dense operation-local identity range.
    let mut output = vec![(
        false,
        true,
        control.body.id.0,
        CARRIER_LOCAL,
        control.carrier,
    )];
    for operation in &control.body.operations {
        output.push((
            false,
            false,
            control.body.id.0,
            operation.node,
            operation.schema,
        ));
        match &operation.body {
            crate::ControlOperationBody::Operation { .. } => {}
            crate::ControlOperationBody::Match(nested) => {
                output.extend(comprehension::all_match_local_definitions(nested))
            }
            crate::ControlOperationBody::Comprehension(nested) => {
                output.extend(comprehension::all_local_definitions(nested))
            }
            crate::ControlOperationBody::Iteration(nested) => {
                output.extend(all_local_definitions(nested))
            }
            crate::ControlOperationBody::Recur(_)
            | crate::ControlOperationBody::Suspend
            | crate::ControlOperationBody::Publish => {
                unreachable!("validated pure iteration body")
            }
        }
    }
    output
}

pub(super) fn prepare(
    artifact: &ProgramArtifact,
    owner: NodeId,
    budget_node: NodeId,
    control: &crate::IterationDeclaration,
    inputs: &[ArtifactSource],
    input_reads: &[ResidentReadLocation],
    output_slot: CellSlotId,
    layout: &LayoutBuild,
) -> Result<ActivatedIterationNode, ResidentActivationError> {
    let invalid = || ResidentActivationError::UnsupportedControlLayout { node: owner };
    let output = layout
        .slots
        .get(output_slot.get() as usize)
        .ok_or_else(invalid)?;
    if output.schema != control.carrier {
        return Err(invalid());
    }
    let carrier_slot = layout
        .control_carriers
        .get(&(owner, control.body.id.0))
        .ok_or_else(invalid)?
        .0;
    let carrier = &layout.slots[carrier_slot.get() as usize];
    if carrier.schema != control.carrier || carrier.storage != ResidentStorageClass::Scratch {
        return Err(invalid());
    }
    let seed = *inputs.get(control.seed as usize).ok_or_else(invalid)?;
    let seed_layout = match seed {
        ArtifactSource::Constant(id) => {
            let value = artifact.constants().get(id).ok_or_else(invalid)?;
            let region = layout.constant_regions[id.get() as usize];
            ResidentPortLayout {
                kind: region.kind,
                shape: region.shape,
                schema_id: value.schema(),
                schema_key: value.schema_key(),
                shape_instance: value.shape().clone(),
                activation_fixed_shape: true,
                resolved_selector: None,
            }
        }
        ArtifactSource::Slot(slot) => slot_port_layout(&layout.slots[slot.get() as usize]),
    };
    let mut locals = Vec::new();
    for (pattern, implicit, block, local, _) in all_local_definitions(control) {
        let slot = if implicit {
            layout.control_carriers[&(owner, block)].0
        } else if pattern {
            layout.match_bindings[&(owner, block, local)].0
        } else {
            layout.control_locals[&(owner, block, local)].0
        };
        let region = layout.slots[slot.get() as usize].region;
        if !locals.contains(&region) {
            locals.push(region);
        }
    }
    Ok(ActivatedIterationNode {
        artifact_node: owner,
        budget_node,
        count: control.count,
        seed: *input_reads.get(control.seed as usize).ok_or_else(invalid)?,
        seed_layout,
        write: ResidentWriteLocation {
            slot: output_slot,
            storage: output.storage,
            region: output.region,
        },
        carrier: ResidentWriteLocation {
            slot: carrier_slot,
            storage: carrier.storage,
            region: carrier.region,
        },
        body: None,
        locals: locals.into_boxed_slice(),
    })
}

pub(super) fn bind_body(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    owner: NodeId,
    control: &crate::IterationDeclaration,
    inputs: &[ArtifactSource],
    input_reads: &[ResidentReadLocation],
    output_slot: CellSlotId,
    layout: &LayoutBuild,
    steps: &mut Vec<ActivatedTurnStep>,
    reads: &mut Vec<ResidentReadLocation>,
    calls: &mut Vec<(
        NodeId,
        crate::memory_planner::CallSiteMemoryTemplate,
        mech_core::CallMemoryPlan,
    )>,
    recursive_roots: &[ActivatedNodeIndex],
) -> Result<ActivatedControlBlock, ResidentActivationError> {
    let _output = &layout.slots[output_slot.get() as usize];
    let carrier_slot = layout
        .control_carriers
        .get(&(owner, control.body.id.0))
        .ok_or(ResidentActivationError::UnsupportedControlLayout { node: owner })?
        .0;
    let carrier = &layout.slots[carrier_slot.get() as usize];
    let scope = ControlBlockInputScope::Iteration {
        captures: &control.captures,
        carrier_slot,
    };
    // Treat the owned current carrier as a retained lexical binding. The
    // common block accounting excludes it when a child's own input contract
    // already accounts for the same region and retains it for unrelated steps.
    bind_control_block(
        artifact,
        catalog,
        owner,
        control.body.id.0,
        &scope,
        &control.body,
        inputs,
        input_reads,
        &[carrier.region],
        &BTreeMap::new(),
        layout,
        steps,
        reads,
        calls,
        recursive_roots,
    )
}
