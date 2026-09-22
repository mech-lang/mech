use mech_core::{
    ComputePlacement, ConstantId, ProgramRevision, SchemaId,
    canonical_application_requirement_bytes,
};
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

    fn control_value(&mut self, value: super::ControlValue) {
        match value {
            super::ControlValue::Constant(id) => {
                self.u8(0);
                self.u32(id.get());
            }
            super::ControlValue::Parameter { block, ordinal } => {
                self.u8(1);
                self.u32(block.0);
                self.u16(ordinal);
            }
            super::ControlValue::Local { block, node } => {
                self.u8(2);
                self.u32(block.0);
                self.u32(node);
            }
        }
    }

    fn control_block(&mut self, block: &super::ControlBlock) {
        self.u32(block.id.0);
        self.u64(block.parameters.len() as u64);
        for parameter in &block.parameters {
            self.u32(parameter.schema.get());
            match parameter.source {
                super::ControlParameterSource::Scrutinee => self.u8(0),
                super::ControlParameterSource::PatternBinding(local) => {
                    self.u8(2);
                    self.u32(local);
                }
                super::ControlParameterSource::Capture(index) => {
                    self.u8(1);
                    self.u16(index);
                }
            }
        }
        self.u64(block.operations.len() as u64);
        for operation in &block.operations {
            self.u32(operation.node);
            self.control_operation_body(&operation.body);
            self.u32(operation.schema.get());
            self.u64(operation.inputs.len() as u64);
            for input in &operation.inputs {
                self.control_value(*input);
            }
        }
        self.control_value(block.yield_value);
    }

    fn control_operation_body(&mut self, body: &super::ControlOperationBody) {
        match body {
            super::ControlOperationBody::Operation {
                operation,
                contract,
            } => {
                self.u8(0);
                self.operation(operation);
                self.u32(contract.get());
            }
            super::ControlOperationBody::Match(control) => {
                self.u8(1);
                self.match_declaration(control);
            }
            super::ControlOperationBody::Comprehension(control) => {
                self.u8(2);
                self.comprehension(control);
            }
            super::ControlOperationBody::Recur(ancestor) => {
                self.u8(3);
                self.u8(*ancestor);
            }
        }
    }

    fn match_declaration(&mut self, control: &super::MatchDeclaration) {
        self.u16(control.scrutinee);
        self.u8(u8::from(control.partial));
        self.u64(control.captures.len() as u64);
        for capture in &control.captures {
            self.u16(capture.input);
            self.u32(capture.schema.get());
        }
        self.u64(control.arms.len() as u64);
        for arm in &control.arms {
            match &arm.pattern {
                super::MatchPattern::Literal(constant) => {
                    self.u8(0);
                    self.u32(constant.get());
                }
                super::MatchPattern::Wildcard => self.u8(1),
                super::MatchPattern::Bind => self.u8(2),
                super::MatchPattern::Structural(pattern) => {
                    self.u8(3);
                    self.match_structural_pattern(pattern);
                }
            };
            match &arm.guard {
                None => self.u8(0),
                Some(guard) => {
                    self.u8(1);
                    self.control_block(guard);
                }
            }
            self.control_block(&arm.body);
        }
    }

    fn match_structural_pattern(
        &mut self,
        pattern: &super::CollectionPattern<mech_core::SchemaId, super::MatchPatternValue>,
    ) {
        match pattern {
            super::CollectionPattern::Wildcard => self.u8(0),
            super::CollectionPattern::Bind { local, schema } => {
                self.u8(1);
                self.u32(*local);
                self.u32(schema.get());
            }
            super::CollectionPattern::Equal(super::MatchPatternValue::Literal(constant)) => {
                self.u8(2);
                self.u8(0);
                self.u32(constant.get());
            }
            super::CollectionPattern::Equal(super::MatchPatternValue::Binding(local)) => {
                self.u8(2);
                self.u8(1);
                self.u32(*local);
            }
            super::CollectionPattern::Enum { ordinal, payload } => {
                self.u8(5);
                self.u32(*ordinal);
                match payload {
                    None => self.u8(0),
                    Some(payload) => {
                        self.u8(1);
                        self.match_structural_pattern(payload);
                    }
                }
            }
            super::CollectionPattern::Tuple(items) => {
                self.u8(3);
                self.u64(items.len() as u64);
                for item in items {
                    self.match_structural_pattern(item);
                }
            }
            super::CollectionPattern::Array {
                prefix,
                rest,
                suffix,
            } => {
                self.u8(4);
                self.u64(prefix.len() as u64);
                for item in prefix {
                    self.match_structural_pattern(item);
                }
                match rest {
                    None => self.u8(0),
                    Some(rest) => {
                        self.u8(1);
                        self.match_structural_pattern(rest);
                    }
                }
                self.u64(suffix.len() as u64);
                for item in suffix {
                    self.match_structural_pattern(item);
                }
            }
        }
    }

    fn comprehension_value(&mut self, value: super::ComprehensionValue) {
        match value {
            super::ComprehensionValue::Constant(id) => {
                self.u8(0);
                self.u32(id.get());
            }
            super::ComprehensionValue::Input(ordinal) => {
                self.u8(1);
                self.u16(ordinal);
            }
            super::ComprehensionValue::Local(local) => {
                self.u8(2);
                self.u32(local);
            }
        }
    }

    fn collection_pattern(&mut self, pattern: &super::CollectionPattern) {
        match pattern {
            super::CollectionPattern::Wildcard => self.u8(0),
            super::CollectionPattern::Bind { local, schema } => {
                self.u8(1);
                self.u32(*local);
                self.u32(schema.get());
            }
            super::CollectionPattern::Equal(value) => {
                self.u8(2);
                self.comprehension_value(*value);
            }
            super::CollectionPattern::Enum { ordinal, payload } => {
                self.u8(5);
                self.u32(*ordinal);
                match payload {
                    None => self.u8(0),
                    Some(payload) => {
                        self.u8(1);
                        self.collection_pattern(payload);
                    }
                }
            }
            super::CollectionPattern::Tuple(items) => {
                self.u8(3);
                self.u64(items.len() as u64);
                for item in items {
                    self.collection_pattern(item);
                }
            }
            super::CollectionPattern::Array {
                prefix,
                rest,
                suffix,
            } => {
                self.u8(4);
                self.u64(prefix.len() as u64);
                for item in prefix {
                    self.collection_pattern(item);
                }
                match rest {
                    None => self.u8(0),
                    Some(rest) => {
                        self.u8(1);
                        self.collection_pattern(rest);
                    }
                }
                self.u64(suffix.len() as u64);
                for item in suffix {
                    self.collection_pattern(item);
                }
            }
        }
    }

    fn comprehension(&mut self, control: &super::ComprehensionDeclaration) {
        self.u32(control.id.0);
        self.u8(match control.kind {
            super::ComprehensionKind::Matrix => 0,
            super::ComprehensionKind::Set => 1,
            super::ComprehensionKind::MatrixPreserveShape => 2,
        });
        self.u64(control.steps.len() as u64);
        for step in &control.steps {
            match step {
                super::ComprehensionStep::Generator { source, pattern } => {
                    self.u8(0);
                    self.comprehension_value(*source);
                    self.collection_pattern(pattern);
                }
                super::ComprehensionStep::Filter(value) => {
                    self.u8(1);
                    self.comprehension_value(*value);
                }
                super::ComprehensionStep::Operation(operation) => {
                    self.u8(2);
                    self.u32(operation.local);
                    self.control_operation_body(&operation.body);
                    self.u32(operation.schema.get());
                    self.u64(operation.inputs.len() as u64);
                    for value in &operation.inputs {
                        self.comprehension_value(*value);
                    }
                }
            }
        }
        self.comprehension_value(control.yield_value);
    }

    fn fsm_value(&mut self, value: &super::FsmValue) {
        match value {
            super::FsmValue::Input(input) => {
                self.u8(0);
                self.u16(*input);
            }
            super::FsmValue::Tuple(items) => {
                self.u8(1);
                self.u64(items.len() as u64);
                for item in items {
                    self.fsm_value(item);
                }
            }
            super::FsmValue::Array(items) => {
                self.u8(2);
                self.u64(items.len() as u64);
                for item in items {
                    self.fsm_value(item);
                }
            }
            super::FsmValue::AtomStruct { name, items } => {
                self.u8(3);
                self.string(name);
                self.u64(items.len() as u64);
                for item in items {
                    self.fsm_value(item);
                }
            }
            super::FsmValue::TupleStruct { name, items } => {
                self.u8(4);
                self.string(name);
                self.u64(items.len() as u64);
                for item in items {
                    self.fsm_value(item);
                }
            }
        }
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
            SlotRole::Output => 3,
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
            ProducerReference::Output { output, source } => {
                writer.u8(2);
                writer.u32(output.get());
                match source {
                    ArtifactSource::Constant(constant) => {
                        writer.u8(0);
                        writer.u32(constant.get());
                    }
                    ArtifactSource::Slot(slot) => {
                        writer.u8(1);
                        writer.u32(slot.get());
                    }
                }
            }
        }
        match slot.initializer {
            None => writer.u8(0),
            Some(InitializerReference::Constant(constant)) => {
                writer.u8(1);
                writer.u32(constant.get());
            }
            Some(InitializerReference::Activation(slot)) => {
                writer.u8(2);
                writer.u32(slot.get());
            }
        }
    }

    writer.u32(draft.nodes.len() as u32);
    for node in &draft.nodes {
        writer.u32(node.node.get());
        match &node.body {
            super::ExecutableNodeBody::Operation(operation) => {
                writer.u8(0);
                writer.operation(&operation.operation);
                writer.u32(operation.contract.get());
                match operation.requirement {
                    None => writer.u8(0),
                    Some(requirement) => {
                        writer.u8(1);
                        writer.u32(requirement.get());
                    }
                }
            }
            super::ExecutableNodeBody::Comprehension(control) => {
                writer.u8(2);
                writer.comprehension(control);
            }
            super::ExecutableNodeBody::Match(control) => {
                writer.u8(1);
                writer.match_declaration(control);
            }
            super::ExecutableNodeBody::Fsm(control) => {
                writer.u8(3);
                writer.string(&control.machine);
                writer.u64(control.arguments.len() as u64);
                for argument in &control.arguments {
                    match &argument.name {
                        None => writer.u8(0),
                        Some(name) => {
                            writer.u8(1);
                            writer.string(name);
                        }
                    }
                    writer.u16(argument.input);
                }
                writer.u64(control.stages.len() as u64);
                for stage in &control.stages {
                    writer.u8(match stage.kind {
                        super::FsmStageKind::State => 0,
                        super::FsmStageKind::Async => 1,
                        super::FsmStageKind::Output => 2,
                    });
                    writer.fsm_value(&stage.value);
                }
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
        writer.string(&constraint.name);
        writer.operation(&constraint.operation);
        writer.u32(constraint.contract.get());
        writer.u32(constraint.inputs.len() as u32);
        for source in &constraint.inputs {
            writer.source(*source);
        }
    }

    // Preserve revision identity for ordinary bytecode-v1 artifacts while
    // making explicit interactive query identity part of artifacts that use
    // the optional extension.
    let interactive_output_count = draft
        .outputs
        .iter()
        .filter(|output| output.interactive_binding.is_some())
        .count();
    if interactive_output_count != 0 {
        writer.bytes(b"interactive-output-symbols-v1");
        writer.u32(interactive_output_count as u32);
        for output in &draft.outputs {
            if let Some(binding) = &output.interactive_binding {
                writer.u32(output.output.get());
                writer.string(&binding.lexical_name);
                writer.source(binding.artifact_source);
                writer.u32(binding.storage.get());
                writer.u32(binding.output.get());
            }
        }
    }

    // Preserve revision identity for ordinary bytecode-v1 artifacts while
    // making the optional compute extension part of region-bearing identity.
    if !draft.compute_regions.is_empty() {
        writer.bytes(b"compute-regions-v1");
        writer.u32(draft.compute_regions.len() as u32);
        for region in &draft.compute_regions {
            writer.u32(region.id.get());
            writer.string(&region.name);
            writer.u8(match region.placement {
                ComputePlacement::Compute => 1,
                ComputePlacement::Cpu => 2,
                ComputePlacement::Gpu => 3,
            });
            writer.u32(region.nodes.len() as u32);
            for node in &region.nodes {
                writer.u32(node.get());
            }
        }
    }

    let mut hash = Sha256::new();
    hash.update(b"mech-program-v1\0");
    hash.update(writer.bytes);
    Ok(ProgramRevision::from_bytes(hash.finalize().into()))
}
