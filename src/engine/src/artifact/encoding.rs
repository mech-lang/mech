use mech_core::{ConstantId, ProgramRevision, SchemaId, canonical_application_requirement_bytes};
use sha2::{Digest, Sha256};

use super::{
    ArtifactBuildError, ArtifactSource, BindingDeclaration, InitializerReference,
    OperationReference, ProducerReference, ProgramArtifactDraft, SlotRole,
};

struct CanonicalArtifactWriter {
    bytes: Vec<u8>,
}

impl CanonicalArtifactWriter {
    fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn bytes(&mut self, value: &[u8]) {
        self.u64(value.len() as u64);
        self.bytes.extend_from_slice(value);
    }

    fn string(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }

    fn operation(&mut self, operation: &OperationReference) {
        self.u32(operation.module_path.len() as u32);
        for segment in &operation.module_path {
            self.string(segment);
        }
        self.string(&operation.operation_name);
    }

    fn source(&mut self, source: ArtifactSource) {
        match source {
            ArtifactSource::Constant(constant) => {
                self.u8(0);
                self.u32(constant.get());
            }
            ArtifactSource::Slot(slot) => {
                self.u8(1);
                self.u32(slot.get());
            }
        }
    }
}

pub(super) fn program_revision(
    draft: &ProgramArtifactDraft,
) -> Result<ProgramRevision, ArtifactBuildError> {
    let mut writer = CanonicalArtifactWriter::new();
    writer.u32(draft.schemas.len() as u32);
    for raw in 0..draft.schemas.len() {
        let entry = draft
            .schemas
            .entry(SchemaId::new(raw as u32))
            .expect("dense SchemaTable");
        writer.bytes(entry.key().as_bytes());
        writer.bytes(entry.canonical_bytes());
    }

    writer.u32(draft.constants.len() as u32);
    for raw in 0..draft.constants.len() {
        let entry = draft
            .constants
            .entry(ConstantId::new(raw as u32))
            .expect("dense ConstantStore");
        let value = entry.value();
        writer.bytes(entry.hash().as_bytes());
        writer.bytes(value.schema_key().as_bytes());
        writer.bytes(&value.shape().canonical_bytes());
        writer.bytes(&value.canonical_payload_bytes(&draft.schemas)?);
    }

    writer.bytes(&draft.contracts.canonical_bytes()?);

    writer.u32(draft.requirements.len() as u32);
    for (_, requirement) in draft.requirements.iter() {
        writer.bytes(&canonical_application_requirement_bytes(requirement)?);
    }

    writer.u32(draft.inputs.len() as u32);
    for input in &draft.inputs {
        writer.u32(input.input.get());
        writer.string(&input.name);
        writer.u32(input.slot.get());
        writer.u32(input.schema.get());
    }

    writer.u32(draft.slots.len() as u32);
    for slot in &draft.slots {
        writer.u32(slot.slot.get());
        writer.u32(slot.schema.get());
        writer.u8(match slot.role {
            SlotRole::Input => 0,
            SlotRole::State => 1,
            SlotRole::Derived => 2,
        });
        match slot.producer {
            ProducerReference::Input(input) => {
                writer.u8(0);
                writer.u32(input.get());
            }
            ProducerReference::NodeOutput {
                node,
                output_ordinal,
            } => {
                writer.u8(1);
                writer.u32(node.get());
                writer.u16(output_ordinal);
            }
        }
        match slot.initializer {
            None => writer.u8(0),
            Some(InitializerReference::Constant(constant)) => {
                writer.u8(1);
                writer.u32(constant.get());
            }
        }
    }

    writer.u32(draft.nodes.len() as u32);
    for node in &draft.nodes {
        writer.u32(node.node.get());
        writer.operation(&node.operation);
        writer.u32(node.contract.get());
        match node.requirement {
            None => writer.u8(0),
            Some(requirement) => {
                writer.u8(1);
                writer.u32(requirement.get());
            }
        }
        writer.u32(node.input_bindings.start);
        writer.u32(node.input_bindings.end);
        writer.u32(node.output_bindings.start);
        writer.u32(node.output_bindings.end);
    }

    writer.u32(draft.bindings.len() as u32);
    for binding in &draft.bindings {
        match binding {
            BindingDeclaration::Input {
                id,
                node,
                port_ordinal,
                source,
            } => {
                writer.u8(0);
                writer.u32(id.get());
                writer.u32(node.get());
                writer.u16(*port_ordinal);
                writer.source(*source);
            }
            BindingDeclaration::Output {
                id,
                node,
                port_ordinal,
                target,
            } => {
                writer.u8(1);
                writer.u32(id.get());
                writer.u32(node.get());
                writer.u16(*port_ordinal);
                writer.u32(target.get());
            }
        }
    }

    writer.u32(draft.outputs.len() as u32);
    for output in &draft.outputs {
        writer.u32(output.output.get());
        writer.string(&output.name);
        writer.u32(output.source.get());
        writer.u32(output.schema.get());
    }

    writer.u32(draft.constraints.len() as u32);
    for constraint in &draft.constraints {
        writer.u32(constraint.constraint.get());
        writer.operation(&constraint.operation);
        writer.u32(constraint.contract.get());
        writer.u32(constraint.inputs.len() as u32);
        for source in &constraint.inputs {
            writer.source(*source);
        }
    }

    let mut hash = Sha256::new();
    hash.update(b"mech-program-v1\0");
    hash.update(writer.bytes);
    Ok(ProgramRevision::from_bytes(hash.finalize().into()))
}
