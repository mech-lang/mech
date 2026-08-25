use std::collections::{BTreeMap, BTreeSet};

use mech_core::{
    AccessMode, AliasPolicy, ApplicationRequirement, BindingId, CellSlotId, ConstantId,
    DeliveryMode, ExternalInteraction, NodeId, OperationContractError, OperationContractId,
    OutputConstruction, PortDirection, ResolvedOperationContract, ResourceDelivery, ResourceIntent,
    SchemaId, validate_contract_schemas, validate_signal_bindings,
};

use super::{
    ArtifactBuildError, ArtifactSource, BindingDeclaration, InitializerReference,
    OperationReference, ProducerReference, ProgramArtifactDraft, SlotRole,
};

pub(super) fn validate(draft: &ProgramArtifactDraft) -> Result<(), ArtifactBuildError> {
    validate_dense_identities(draft)?;
    validate_contract_table(draft)?;
    validate_interfaces(draft)?;
    validate_slots(draft)?;
    validate_nodes_and_bindings(draft)?;
    validate_outputs_and_constraints(draft)?;
    validate_compute_regions(draft)?;
    validate_constants(draft)?;
    validate_combinational_graph(draft)
}

fn validate_contract_table(draft: &ProgramArtifactDraft) -> Result<(), ArtifactBuildError> {
    draft.contracts.validate_canonical_order()?;
    for contract in draft.contracts.iter() {
        validate_contract_schemas(contract, &draft.schemas)?;
    }
    Ok(())
}

fn canonical_name(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value != "."
        && value != ".."
        && !value.contains(['\0', '/', '\\'])
}

fn validate_operation(operation: &OperationReference) -> Result<(), ArtifactBuildError> {
    if operation.module_path.is_empty()
        || !canonical_name(&operation.operation_name)
        || operation
            .module_path
            .iter()
            .any(|segment| !canonical_name(segment))
    {
        return Err(ArtifactBuildError::InvalidOperationReference {
            operation: operation.clone(),
        });
    }
    Ok(())
}

fn expect_dense(
    identity: &'static str,
    expected: usize,
    found: u32,
) -> Result<(), ArtifactBuildError> {
    let expected = u32::try_from(expected).unwrap_or(u32::MAX);
    if found != expected {
        return Err(ArtifactBuildError::NonCanonicalIdentity {
            identity,
            expected,
            found,
        });
    }
    Ok(())
}

fn validate_dense_identities(draft: &ProgramArtifactDraft) -> Result<(), ArtifactBuildError> {
    for (index, input) in draft.inputs.iter().enumerate() {
        expect_dense("InputId", index, input.input.get())?;
    }
    for (index, slot) in draft.slots.iter().enumerate() {
        expect_dense("CellSlotId", index, slot.slot.get())?;
    }
    for (index, node) in draft.nodes.iter().enumerate() {
        expect_dense("NodeId", index, node.node.get())?;
    }
    for (index, binding) in draft.bindings.iter().enumerate() {
        expect_dense("BindingId", index, binding.id().get())?;
    }
    for (index, output) in draft.outputs.iter().enumerate() {
        expect_dense("OutputId", index, output.output.get())?;
    }
    for (index, constraint) in draft.constraints.iter().enumerate() {
        expect_dense("IntegrityConstraintId", index, constraint.constraint.get())?;
    }
    for (index, region) in draft.compute_regions.iter().enumerate() {
        expect_dense("ComputeRegionId", index, region.id.get())?;
    }
    Ok(())
}

fn validate_compute_regions(draft: &ProgramArtifactDraft) -> Result<(), ArtifactBuildError> {
    let mut names = BTreeSet::new();
    let mut assigned_nodes = BTreeSet::new();
    for region in &draft.compute_regions {
        if !canonical_name(&region.name) {
            return Err(ArtifactBuildError::InvalidComputeRegionName { region: region.id });
        }
        if !names.insert(region.name.as_ref()) {
            return Err(ArtifactBuildError::DuplicateComputeRegionName {
                name: region.name.clone(),
            });
        }
        if region.nodes.is_empty() {
            return Err(ArtifactBuildError::EmptyComputeRegion { region: region.id });
        }
        let mut previous = None;
        for node in &region.nodes {
            require_node(draft, *node)?;
            if previous.is_some_and(|previous| previous >= *node) {
                return Err(ArtifactBuildError::NonCanonicalComputeRegionNodes {
                    region: region.id,
                });
            }
            if !assigned_nodes.insert(*node) {
                return Err(ArtifactBuildError::DuplicateComputeRegionNode { node: *node });
            }
            previous = Some(*node);
        }
    }
    Ok(())
}

fn require_schema(
    draft: &ProgramArtifactDraft,
    schema: SchemaId,
) -> Result<(), ArtifactBuildError> {
    if draft.schemas.get(schema).is_none() {
        return Err(ArtifactBuildError::UnknownSchema { schema });
    }
    Ok(())
}

fn require_constant(
    draft: &ProgramArtifactDraft,
    constant: ConstantId,
) -> Result<(), ArtifactBuildError> {
    if draft.constants.get(constant).is_none() {
        return Err(ArtifactBuildError::UnknownConstant { constant });
    }
    Ok(())
}

fn require_slot(
    draft: &ProgramArtifactDraft,
    slot: CellSlotId,
) -> Result<&super::SlotDeclaration, ArtifactBuildError> {
    draft
        .slots
        .get(slot.get() as usize)
        .filter(|declaration| declaration.slot == slot)
        .ok_or(ArtifactBuildError::UnknownSlot { slot })
}

fn require_node(
    draft: &ProgramArtifactDraft,
    node: NodeId,
) -> Result<&super::NodeDeclaration, ArtifactBuildError> {
    draft
        .nodes
        .get(node.get() as usize)
        .filter(|declaration| declaration.node == node)
        .ok_or(ArtifactBuildError::UnknownNode { node })
}

fn require_contract(
    draft: &ProgramArtifactDraft,
    contract: OperationContractId,
) -> Result<&ResolvedOperationContract, ArtifactBuildError> {
    draft
        .contracts
        .get(contract)
        .ok_or(ArtifactBuildError::UnknownOperationContract { contract })
}

fn contract_input_schema(contract: &ResolvedOperationContract, ordinal: usize) -> Option<SchemaId> {
    match contract {
        ResolvedOperationContract::Declared(contract) => {
            contract.inputs.get(ordinal).map(|port| port.schema)
        }
        ResolvedOperationContract::LegacyOpaque(contract) => {
            contract.input_schemas.get(ordinal).copied()
        }
    }
}

fn contract_output_schema(
    contract: &ResolvedOperationContract,
    ordinal: usize,
) -> Option<SchemaId> {
    match contract {
        ResolvedOperationContract::Declared(contract) => {
            contract.outputs.get(ordinal).map(|port| port.schema)
        }
        ResolvedOperationContract::LegacyOpaque(contract) => {
            contract.output_schemas.get(ordinal).copied()
        }
    }
}

fn contract_port_counts(contract: &ResolvedOperationContract) -> (usize, usize) {
    match contract {
        ResolvedOperationContract::Declared(contract) => {
            (contract.inputs.len(), contract.outputs.len())
        }
        ResolvedOperationContract::LegacyOpaque(contract) => {
            (contract.input_schemas.len(), contract.output_schemas.len())
        }
    }
}

fn validate_interfaces(draft: &ProgramArtifactDraft) -> Result<(), ArtifactBuildError> {
    let mut input_names = BTreeSet::new();
    for input in &draft.inputs {
        require_schema(draft, input.schema)?;
        if !canonical_name(&input.name) {
            return Err(ArtifactBuildError::InvalidInterfaceName {
                interface: "input",
                name: input.name.clone(),
            });
        }
        if !input_names.insert(input.name.clone()) {
            return Err(ArtifactBuildError::DuplicateInterfaceName {
                interface: "input",
                name: input.name.clone(),
            });
        }
    }
    let mut output_names = BTreeSet::new();
    let mut interactive_symbols = BTreeSet::new();
    for output in &draft.outputs {
        require_schema(draft, output.schema)?;
        if !canonical_name(&output.name) {
            return Err(ArtifactBuildError::InvalidInterfaceName {
                interface: "output",
                name: output.name.clone(),
            });
        }
        if !output_names.insert(output.name.clone()) {
            return Err(ArtifactBuildError::DuplicateInterfaceName {
                interface: "output",
                name: output.name.clone(),
            });
        }
        if let Some(binding) = &output.interactive_binding {
            if binding.output != output.output || binding.storage != output.source {
                return Err(ArtifactBuildError::InvalidInteractiveBinding {
                    name: binding.lexical_name.clone(),
                });
            }
            validate_source(draft, binding.artifact_source)?;
            let storage = require_slot(draft, binding.storage)?;
            let source_matches = match storage.producer {
                ProducerReference::Output { source, .. } => source == binding.artifact_source,
                _ => binding.artifact_source == ArtifactSource::Slot(binding.storage),
            };
            if !source_matches {
                return Err(ArtifactBuildError::InvalidInteractiveBinding {
                    name: binding.lexical_name.clone(),
                });
            }
            if !interactive_symbols.insert(binding.lexical_name.clone()) {
                return Err(ArtifactBuildError::DuplicateInterfaceName {
                    interface: "interactive output",
                    name: binding.lexical_name.clone(),
                });
            }
        }
    }
    Ok(())
}

fn validate_slots(draft: &ProgramArtifactDraft) -> Result<(), ArtifactBuildError> {
    let mut producers = BTreeSet::new();
    for slot in &draft.slots {
        require_schema(draft, slot.schema)?;
        if !producers.insert(producer_key(slot.producer)) {
            return Err(ArtifactBuildError::DuplicateProducer {
                producer: slot.producer,
            });
        }
        match (slot.role, slot.producer, slot.initializer) {
            (SlotRole::Input, ProducerReference::Input(_), None)
            | (SlotRole::State, ProducerReference::NodeOutput { .. }, None)
            | (
                SlotRole::State,
                ProducerReference::NodeOutput { .. },
                Some(InitializerReference::Constant(_)),
            )
            | (SlotRole::Derived, ProducerReference::NodeOutput { .. }, None)
            | (SlotRole::Output, ProducerReference::Output { .. }, None)
            | (
                SlotRole::Output,
                ProducerReference::Output { .. },
                Some(InitializerReference::Constant(_)),
            ) => {}
            _ => return Err(ArtifactBuildError::InvalidSlotRole { slot: slot.slot }),
        }
        if let Some(InitializerReference::Constant(constant)) = slot.initializer {
            require_constant(draft, constant)?;
            let value = draft
                .constants
                .get(constant)
                .expect("require_constant accepted the initializer");
            let schema_key = draft
                .schemas
                .entry(slot.schema)
                .expect("require_schema accepted the state slot schema")
                .key();
            if value.schema() != slot.schema || value.schema_key() != schema_key {
                return Err(ArtifactBuildError::InitializerSchemaMismatch {
                    slot: slot.slot,
                    constant,
                });
            }
        }
        match slot.producer {
            ProducerReference::Input(input) => {
                let Some(declaration) = draft.inputs.get(input.get() as usize) else {
                    return Err(ArtifactBuildError::UnknownInput { input });
                };
                if declaration.input != input
                    || declaration.slot != slot.slot
                    || declaration.schema != slot.schema
                {
                    return Err(ArtifactBuildError::ProducerBindingMismatch { slot: slot.slot });
                }
            }
            ProducerReference::NodeOutput { node, .. } => {
                require_node(draft, node)?;
            }
            ProducerReference::Output { output, source } => {
                let Some(declaration) = draft.outputs.get(output.get() as usize) else {
                    return Err(ArtifactBuildError::InvalidSlotRole { slot: slot.slot });
                };
                if declaration.output != output || declaration.source != slot.slot {
                    return Err(ArtifactBuildError::ProducerBindingMismatch { slot: slot.slot });
                }
                validate_source(draft, source)?;
                if source == ArtifactSource::Slot(slot.slot)
                    || source_schema(draft, source)? != slot.schema
                {
                    return Err(ArtifactBuildError::ProducerBindingMismatch { slot: slot.slot });
                }
            }
        }
    }
    for input in &draft.inputs {
        let slot = require_slot(draft, input.slot)?;
        if slot.role != SlotRole::Input
            || slot.schema != input.schema
            || slot.producer != ProducerReference::Input(input.input)
        {
            return Err(ArtifactBuildError::InterfaceSlotMismatch {
                interface: "input",
                slot: input.slot,
            });
        }
    }
    Ok(())
}

fn producer_key(producer: ProducerReference) -> (u8, u32, u16) {
    match producer {
        ProducerReference::Input(input) => (0, input.get(), 0),
        ProducerReference::NodeOutput {
            node,
            output_ordinal,
        } => (1, node.get(), output_ordinal),
        ProducerReference::Output { output, .. } => (2, output.get(), 0),
    }
}

fn checked_range(
    range: &core::ops::Range<u32>,
    len: usize,
    node: NodeId,
) -> Result<core::ops::Range<usize>, ArtifactBuildError> {
    let start = range.start as usize;
    let end = range.end as usize;
    if start > end || end > len {
        return Err(ArtifactBuildError::BindingRangeMismatch { node });
    }
    Ok(start..end)
}

fn validate_nodes_and_bindings(draft: &ProgramArtifactDraft) -> Result<(), ArtifactBuildError> {
    let mut cursor = 0usize;
    let mut bound_producers = BTreeSet::new();
    let mut state_writers = BTreeMap::<CellSlotId, Vec<(NodeId, u16)>>::new();
    for node in &draft.nodes {
        validate_operation(&node.operation)?;
        let inputs = checked_range(&node.input_bindings, draft.bindings.len(), node.node)?;
        let outputs = checked_range(&node.output_bindings, draft.bindings.len(), node.node)?;
        if inputs.start != cursor || inputs.end != outputs.start {
            return Err(ArtifactBuildError::BindingRangeMismatch { node: node.node });
        }
        cursor = outputs.end;

        let contract = require_contract(draft, node.contract)?;
        validate_node_requirement(draft, node, contract)?;
        validate_signal_bindings(contract)?;
        let (expected_inputs, expected_outputs) = contract_port_counts(contract);
        if inputs.len() != expected_inputs {
            return Err(OperationContractError::PortCountMismatch {
                direction: PortDirection::Input,
                expected: expected_inputs as u64,
                actual: inputs.len() as u64,
            }
            .into());
        }
        if outputs.len() != expected_outputs {
            return Err(OperationContractError::PortCountMismatch {
                direction: PortDirection::Output,
                expected: expected_outputs as u64,
                actual: outputs.len() as u64,
            }
            .into());
        }

        for (ordinal, binding) in draft.bindings[inputs.clone()].iter().enumerate() {
            validate_binding_identity(binding, node.node, ordinal)?;
            let BindingDeclaration::Input { source, .. } = binding else {
                return Err(ArtifactBuildError::BindingDirectionMismatch {
                    binding: binding.id(),
                });
            };
            validate_source(draft, *source)?;
            let expected = contract_input_schema(contract, ordinal)
                .expect("contract input count was validated");
            let actual = source_schema(draft, *source)?;
            if actual != expected {
                return Err(ArtifactBuildError::ContractInputSchemaMismatch {
                    contract: node.contract,
                    port: ordinal as u16,
                    expected,
                    actual,
                });
            }
        }
        for (ordinal, binding) in draft.bindings[outputs.clone()].iter().enumerate() {
            validate_binding_identity(binding, node.node, ordinal)?;
            let BindingDeclaration::Output { target, .. } = binding else {
                return Err(ArtifactBuildError::BindingDirectionMismatch {
                    binding: binding.id(),
                });
            };
            let slot = require_slot(draft, *target)?;
            let expected = contract_output_schema(contract, ordinal)
                .expect("contract output count was validated");
            if slot.schema != expected {
                return Err(ArtifactBuildError::ContractOutputSchemaMismatch {
                    contract: node.contract,
                    port: ordinal as u16,
                    expected,
                    actual: slot.schema,
                });
            }
            let writer = ProducerReference::NodeOutput {
                node: node.node,
                output_ordinal: ordinal as u16,
            };
            if slot.role == SlotRole::State {
                state_writers
                    .entry(*target)
                    .or_default()
                    .push((node.node, ordinal as u16));
            } else {
                if slot.producer != writer {
                    return Err(ArtifactBuildError::ProducerBindingMismatch { slot: *target });
                }
                if !bound_producers.insert(producer_key(slot.producer)) {
                    return Err(ArtifactBuildError::ProducerBindingMismatch { slot: *target });
                }
            }
        }
    }
    if cursor != draft.bindings.len() {
        let node = draft
            .nodes
            .last()
            .map(|node| node.node)
            .unwrap_or(NodeId(0));
        return Err(ArtifactBuildError::BindingRangeMismatch { node });
    }
    for slot in &draft.slots {
        if slot.role == SlotRole::State {
            let writers = state_writers
                .get(&slot.slot)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            validate_state_writer_chain(draft, slot.slot, writers)?;
            bound_producers.insert(producer_key(slot.producer));
        }
        if matches!(slot.producer, ProducerReference::NodeOutput { .. })
            && !bound_producers.contains(&producer_key(slot.producer))
        {
            return Err(ArtifactBuildError::MissingProducerBinding {
                slot: slot.slot,
                producer: slot.producer,
            });
        }
    }
    Ok(())
}

fn validate_node_requirement(
    draft: &ProgramArtifactDraft,
    node: &super::NodeDeclaration,
    contract: &ResolvedOperationContract,
) -> Result<(), ArtifactBuildError> {
    let requirement = match node.requirement {
        Some(id) => Some(draft.requirements.get(id).ok_or(
            ArtifactBuildError::UnknownApplicationRequirement {
                requirement: id.get(),
            },
        )?),
        None => None,
    };
    let ResolvedOperationContract::Declared(contract) = contract else {
        // Host calls remain valid generic artifact data until their explicit
        // resident contract is declared. D3 resident admission rejects them.
        return Ok(());
    };
    match (&contract.interaction, requirement) {
        (ExternalInteraction::Pure, None) => Ok(()),
        (ExternalInteraction::Pure, Some(_)) => {
            Err(ArtifactBuildError::UnexpectedApplicationRequirement { node: node.node })
        }
        (ExternalInteraction::Observation(_), Some(ApplicationRequirement::Resource(request)))
            if request.intent == ResourceIntent::Read
                && request.delivery == ResourceDelivery::Live =>
        {
            Ok(())
        }
        (ExternalInteraction::Effect(_), Some(ApplicationRequirement::Resource(request)))
            if matches!(
                request.intent,
                ResourceIntent::Assign | ResourceIntent::Send
            ) && request.delivery == ResourceDelivery::Snapshot
                && contract.outputs.is_empty() =>
        {
            Ok(())
        }
        (
            ExternalInteraction::TransactionalExternal(_),
            Some(ApplicationRequirement::Resource(request)),
        ) if matches!(
            request.intent,
            ResourceIntent::Assign | ResourceIntent::Send
        ) && request.delivery == ResourceDelivery::Snapshot =>
        {
            Ok(())
        }
        (
            ExternalInteraction::Observation(_)
            | ExternalInteraction::Effect(_)
            | ExternalInteraction::TransactionalExternal(_),
            None,
        ) => Err(ArtifactBuildError::MissingApplicationRequirement { node: node.node }),
        _ => Err(ArtifactBuildError::ApplicationRequirementInteractionMismatch { node: node.node }),
    }
}

fn validate_state_writer_chain(
    draft: &ProgramArtifactDraft,
    slot_id: CellSlotId,
    writers: &[(NodeId, u16)],
) -> Result<(), ArtifactBuildError> {
    let slot = require_slot(draft, slot_id)?;
    let Some(&(final_node, final_output)) = writers.last() else {
        return Err(ArtifactBuildError::MissingProducerBinding {
            slot: slot_id,
            producer: slot.producer,
        });
    };
    if slot.producer
        != (ProducerReference::NodeOutput {
            node: final_node,
            output_ordinal: final_output,
        })
    {
        return Err(ArtifactBuildError::ProducerBindingMismatch { slot: slot_id });
    }

    enum WriterForm {
        FullWrite,
        ReadModifyWrite,
    }
    let mut form = None;
    for &(node_id, output_ordinal) in writers {
        let node = require_node(draft, node_id)?;
        let contract = match require_contract(draft, node.contract)? {
            ResolvedOperationContract::Declared(contract) => contract,
            ResolvedOperationContract::LegacyOpaque(_) if writers.len() == 1 => return Ok(()),
            ResolvedOperationContract::LegacyOpaque(_) => {
                return Err(ArtifactBuildError::InvalidStateWriterChain {
                    slot: slot_id,
                    reason: "multi-writer state chains require declared operation contracts",
                });
            }
        };
        let output = contract.outputs.get(output_ordinal as usize).ok_or(
            ArtifactBuildError::InvalidStateWriterChain {
                slot: slot_id,
                reason: "state writer output is absent from its contract",
            },
        )?;
        let current = match (&output.construction, output.access, output.alias) {
            (OutputConstruction::FullWrite { .. }, AccessMode::Write, AliasPolicy::NoAlias) => {
                WriterForm::FullWrite
            }
            (
                OutputConstruction::ReadModifyWrite { base_input, .. },
                AccessMode::ReadWrite,
                AliasPolicy::MayAlias { input },
            ) if *base_input == input => {
                let inputs = checked_range(&node.input_bindings, draft.bindings.len(), node.node)?;
                let binding = draft
                    .bindings
                    .get(inputs.start + *base_input as usize)
                    .ok_or(ArtifactBuildError::InvalidStateWriterChain {
                        slot: slot_id,
                        reason: "state writer base input is out of range",
                    })?;
                if !matches!(
                    binding,
                    BindingDeclaration::Input {
                        source: ArtifactSource::Slot(base),
                        ..
                    } if *base == slot_id
                ) {
                    return Err(ArtifactBuildError::InvalidStateWriterChain {
                        slot: slot_id,
                        reason: "read-modify-write base input must resolve to the same state slot",
                    });
                }
                WriterForm::ReadModifyWrite
            }
            _ => {
                return Err(ArtifactBuildError::InvalidStateWriterChain {
                    slot: slot_id,
                    reason: "state writer access, construction, and alias policies disagree",
                });
            }
        };
        match (&form, &current) {
            (None, _) => form = Some(current),
            (Some(WriterForm::FullWrite), WriterForm::FullWrite)
            | (Some(WriterForm::FullWrite), WriterForm::ReadModifyWrite)
            | (Some(WriterForm::ReadModifyWrite), WriterForm::FullWrite) => {
                return Err(ArtifactBuildError::InvalidStateWriterChain {
                    slot: slot_id,
                    reason: "a state slot must use one full writer or an ordered RMW chain",
                });
            }
            (Some(WriterForm::ReadModifyWrite), WriterForm::ReadModifyWrite) => {}
        }
    }
    Ok(())
}

fn validate_binding_identity(
    binding: &BindingDeclaration,
    node: NodeId,
    ordinal: usize,
) -> Result<(), ArtifactBuildError> {
    if binding.node() != node {
        return Err(ArtifactBuildError::BindingNodeMismatch {
            binding: binding.id(),
        });
    }
    let expected =
        u16::try_from(ordinal).map_err(|_| ArtifactBuildError::ArtifactIdentityExhausted {
            identity: "binding port ordinal",
        })?;
    if binding.port_ordinal() != expected {
        return Err(ArtifactBuildError::BindingPortMismatch {
            binding: binding.id(),
            expected,
            found: binding.port_ordinal(),
        });
    }
    Ok(())
}

fn validate_source(
    draft: &ProgramArtifactDraft,
    source: ArtifactSource,
) -> Result<(), ArtifactBuildError> {
    match source {
        ArtifactSource::Constant(constant) => require_constant(draft, constant),
        ArtifactSource::Slot(slot) => require_slot(draft, slot).map(|_| ()),
    }
}

fn source_schema(
    draft: &ProgramArtifactDraft,
    source: ArtifactSource,
) -> Result<SchemaId, ArtifactBuildError> {
    match source {
        ArtifactSource::Constant(constant) => draft
            .constants
            .get(constant)
            .map(|value| value.schema())
            .ok_or(ArtifactBuildError::UnknownConstant { constant }),
        ArtifactSource::Slot(slot) => require_slot(draft, slot).map(|slot| slot.schema),
    }
}

fn validate_outputs_and_constraints(
    draft: &ProgramArtifactDraft,
) -> Result<(), ArtifactBuildError> {
    for output in &draft.outputs {
        let slot = require_slot(draft, output.source)?;
        if slot.schema != output.schema || !matches!(slot.role, SlotRole::State | SlotRole::Output)
        {
            return Err(ArtifactBuildError::InterfaceSlotMismatch {
                interface: "output",
                slot: output.source,
            });
        }
    }
    for constraint in &draft.constraints {
        validate_operation(&constraint.operation)?;
        let contract = require_contract(draft, constraint.contract)?;
        let ResolvedOperationContract::Declared(contract) = contract else {
            return Err(ArtifactBuildError::IntegrityConstraintContractInvalid {
                constraint: constraint.constraint,
            });
        };
        if contract.interaction != ExternalInteraction::Pure
            || !contract.outputs.is_empty()
            || contract.inputs.len() != constraint.inputs.len()
        {
            return Err(ArtifactBuildError::IntegrityConstraintContractInvalid {
                constraint: constraint.constraint,
            });
        }
        for (ordinal, source) in constraint.inputs.iter().enumerate() {
            validate_source(draft, *source)?;
            let port = &contract.inputs[ordinal];
            if port.access != AccessMode::Read
                || port.delivery != DeliveryMode::Signal
                || port.schema != source_schema(draft, *source)?
            {
                return Err(ArtifactBuildError::IntegrityConstraintContractInvalid {
                    constraint: constraint.constraint,
                });
            }
        }
    }
    Ok(())
}

fn validate_constants(draft: &ProgramArtifactDraft) -> Result<(), ArtifactBuildError> {
    for raw in 0..draft.constants.len() {
        let constant = ConstantId::new(raw as u32);
        let value = draft.constants.get(constant).expect("dense ConstantStore");
        value.validate_against(&draft.schemas)?;
    }
    Ok(())
}

fn validate_combinational_graph(draft: &ProgramArtifactDraft) -> Result<(), ArtifactBuildError> {
    let mut incoming = vec![0usize; draft.nodes.len()];
    let mut downstream = vec![Vec::<usize>::new(); draft.nodes.len()];
    let mut latest_state_writer = BTreeMap::<CellSlotId, usize>::new();
    for node in &draft.nodes {
        let range = node.input_bindings.start as usize..node.input_bindings.end as usize;
        for binding in &draft.bindings[range] {
            let BindingDeclaration::Input {
                source: ArtifactSource::Slot(slot),
                ..
            } = binding
            else {
                continue;
            };
            let slot = require_slot(draft, *slot)?;
            let from = if slot.role == SlotRole::State {
                latest_state_writer.get(&slot.slot).copied()
            } else {
                match slot.producer {
                    ProducerReference::NodeOutput { node: producer, .. } => {
                        Some(producer.get() as usize)
                    }
                    ProducerReference::Input(_) => None,
                    ProducerReference::Output { .. } => None,
                }
            };
            let Some(from) = from else { continue };
            let to = node.node.get() as usize;
            if !downstream[from].contains(&to) {
                downstream[from].push(to);
                incoming[to] += 1;
            }
        }
        let outputs = node.output_bindings.start as usize..node.output_bindings.end as usize;
        for binding in &draft.bindings[outputs] {
            let BindingDeclaration::Output { target, .. } = binding else {
                continue;
            };
            if require_slot(draft, *target)?.role == SlotRole::State {
                latest_state_writer.insert(*target, node.node.get() as usize);
            }
        }
    }

    let mut ready = incoming
        .iter()
        .enumerate()
        .filter_map(|(node, count)| (*count == 0).then_some(node))
        .collect::<Vec<_>>();
    let mut visited = 0usize;
    while let Some(node) = ready.pop() {
        visited += 1;
        for next in &downstream[node] {
            incoming[*next] -= 1;
            if incoming[*next] == 0 {
                ready.push(*next);
            }
        }
    }
    if visited != draft.nodes.len() {
        return Err(ArtifactBuildError::CombinationalCycle);
    }
    Ok(())
}
