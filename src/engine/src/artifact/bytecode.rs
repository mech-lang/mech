//! Bytecode-v1 representation of the deterministic semantic program artifact.

use std::collections::{BTreeMap, BTreeSet};

use core::fmt;
use mech_core::snapshot::{SnapshotCanonicalizationBudget, SnapshotValidationContext};
use mech_core::{
    ApplicationRequirement, ApplicationRequirementId, BindingId, BytecodeArtifactSections,
    BytecodeProgram, CellSlotId, ComputePlacement, ConstantId, ConstantStore, ConstantStoreBuilder,
    DimensionParameterDeclaration, DimensionParameterId, DimensionParameterOrigin,
    ExecutionHostFunctionRequest, ExecutionResourceRequest, InputId, IntegrityConstraintId,
    MechError, NodeId, OperationContractId, OperationContractTable, OutputId, ParsedProgram,
    ResourceDelivery, ResourceIntent, SchemaDraft, SchemaId, SchemaTable, SchemaTableBuilder,
    SemanticModelError, SnapshotValueError, Value, ValueDraft, write_bytecode_with_artifact,
};
use serde::{
    Deserialize, Serialize,
    de::{DeserializeOwned, DeserializeSeed, Error as _, IgnoredAny, SeqAccess, Visitor},
};

use super::{
    ApplicationRequirementTable, ArtifactBuildError, ArtifactSource, BindingDeclaration,
    ComputeRegionDeclaration, InitializerReference, InputDeclaration,
    IntegrityConstraintDeclaration, InteractiveSymbolBinding, NodeDeclaration, OperationReference,
    OutputDeclaration, ProducerReference, ProgramArtifact, ProgramArtifactDraft, SlotDeclaration,
    SlotRole,
};

const DEFAULT_MAX_ARTIFACT_SECTION_BYTES: usize = 16_777_216;
const DEFAULT_MAX_ARTIFACT_BYTES: usize = 67_108_864;
const DEFAULT_MAX_CONSTANT_CANONICALIZATION_WORK: u64 = 65_536;
const WIRE_GRAPH_REVISION: u32 = 10;

#[derive(Clone, Copy, Debug)]
pub struct ArtifactDecodeLimits {
    pub max_section_bytes: usize,
    pub max_total_bytes: usize,
    pub max_schemas: usize,
    pub max_constants: usize,
    pub max_inputs: usize,
    pub max_slots: usize,
    pub max_nodes: usize,
    pub max_requirements: usize,
    pub max_bindings: usize,
    pub max_outputs: usize,
    pub max_constraints: usize,
    pub max_operations: usize,
    pub max_contracts: usize,
    pub max_compute_regions: usize,
    pub max_control_arms: usize,
    pub max_control_blocks: usize,
    pub max_control_operations: usize,
    pub max_control_operands: usize,
    pub max_constant_canonicalization_work: u64,
}

impl Default for ArtifactDecodeLimits {
    fn default() -> Self {
        Self {
            max_section_bytes: DEFAULT_MAX_ARTIFACT_SECTION_BYTES,
            max_total_bytes: DEFAULT_MAX_ARTIFACT_BYTES,
            max_schemas: 100_000,
            max_constants: 1_000_000,
            max_inputs: 1_000_000,
            max_slots: 1_000_000,
            max_nodes: 1_000_000,
            max_requirements: 100_000,
            max_bindings: 1_000_000,
            max_outputs: 1_000_000,
            max_constraints: 1_000_000,
            max_operations: 1_000_000,
            max_contracts: 100_000,
            max_compute_regions: 100_000,
            max_control_arms: super::MAX_CONTROL_ARMS,
            max_control_blocks: super::MAX_CONTROL_BLOCKS,
            max_control_operations: super::MAX_CONTROL_OPERATIONS,
            max_control_operands: super::MAX_CONTROL_OPERANDS,
            max_constant_canonicalization_work: DEFAULT_MAX_CONSTANT_CANONICALIZATION_WORK,
        }
    }
}

#[derive(Debug)]
pub enum ArtifactBytecodeError {
    CoreBytecode(MechError),
    Json(serde_json::Error),
    Semantic(SemanticModelError),
    Snapshot(SnapshotValueError),
    Artifact(ArtifactBuildError),
    MissingArtifactSections,
    SectionByteLimit {
        section: &'static str,
        limit: usize,
    },
    AggregateByteLimit {
        limit: usize,
    },
    SectionItemLimit {
        section: &'static str,
        limit: usize,
        actual: usize,
    },
    NonCanonicalSchemaId {
        expected: u32,
        found: u32,
    },
    NonCanonicalConstantId {
        expected: u32,
        found: u32,
    },
    UnknownOperation {
        operation: u32,
    },
    NonCanonicalOperationTable,
    RequirementTableMismatch,
    InvalidWireTag {
        section: &'static str,
        tag: u8,
    },
}

impl From<MechError> for ArtifactBytecodeError {
    fn from(error: MechError) -> Self {
        Self::CoreBytecode(error)
    }
}
impl From<serde_json::Error> for ArtifactBytecodeError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}
impl From<SemanticModelError> for ArtifactBytecodeError {
    fn from(error: SemanticModelError) -> Self {
        Self::Semantic(error)
    }
}
impl From<SnapshotValueError> for ArtifactBytecodeError {
    fn from(error: SnapshotValueError) -> Self {
        Self::Snapshot(error)
    }
}
impl From<ArtifactBuildError> for ArtifactBytecodeError {
    fn from(error: ArtifactBuildError) -> Self {
        Self::Artifact(error)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct WireInput {
    input: u32,
    name: String,
    slot: u32,
    schema: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct WireSlot {
    slot: u32,
    schema: u32,
    role: u8,
    initializer: Option<WireSource>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum WireProducer {
    Input(u32),
    NodeOutput { node: u32, output_ordinal: u16 },
    Output { output: u32, source: WireSource },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct WireNode {
    node: u32,
    body: WireNodeBody,
    input_start: u32,
    input_end: u32,
    output_start: u32,
    output_end: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum WireNodeBody {
    Operation {
        operation: u32,
        contract: u32,
        requirement: Option<u32>,
    },
    Comprehension(WireComprehensionDeclaration),
    Match(WireMatchDeclaration),
    Fsm {
        machine: String,
        arguments: Box<[(Option<String>, u16)]>,
        stages: Box<[WireFsmStage]>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireMatchDeclaration {
    scrutinee: u16,
    partial: bool,
    captures: Box<[(u16, u32)]>,
    arms: Box<[WireMatchArm]>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireFsmStage {
    kind: u8,
    value: WireFsmValue,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum WireFsmValue {
    Input(u16),
    Tuple(Box<[WireFsmValue]>),
    Array(Box<[WireFsmValue]>),
    AtomStruct {
        name: String,
        items: Box<[WireFsmValue]>,
    },
    TupleStruct {
        name: String,
        items: Box<[WireFsmValue]>,
    },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
enum WireComprehensionValue {
    Constant(u32),
    Input(u16),
    Local(u32),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum WireCollectionPattern {
    Wildcard,
    Bind {
        local: u32,
        schema: u32,
    },
    Equal(WireComprehensionValue),
    Enum {
        ordinal: u32,
        payload: Option<Box<WireCollectionPattern>>,
    },
    Tuple(Box<[WireCollectionPattern]>),
    Array {
        prefix: Box<[WireCollectionPattern]>,
        rest: Option<Box<WireCollectionPattern>>,
        suffix: Box<[WireCollectionPattern]>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum WireComprehensionStep {
    Generator {
        source: WireComprehensionValue,
        pattern: WireCollectionPattern,
    },
    Filter(WireComprehensionValue),
    Operation {
        local: u32,
        body: WireControlOperationBody,
        inputs: Box<[WireComprehensionValue]>,
        schema: u32,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireComprehensionDeclaration {
    id: u32,
    kind: u8,
    steps: Box<[WireComprehensionStep]>,
    yield_value: WireComprehensionValue,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireMatchArm {
    pattern: WirePattern,
    guard: Option<WireControlBlock>,
    body: WireControlBlock,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum WirePattern {
    Literal(u32),
    Wildcard,
    Bind,
    Structural(WireStructuralMatchPattern),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum WireMatchPatternValue {
    Literal(u32),
    Binding(u32),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum WireStructuralMatchPattern {
    Wildcard,
    Bind {
        local: u32,
        schema: u32,
    },
    Equal(WireMatchPatternValue),
    Enum {
        ordinal: u32,
        payload: Option<Box<WireStructuralMatchPattern>>,
    },
    Tuple(Box<[WireStructuralMatchPattern]>),
    Array {
        prefix: Box<[WireStructuralMatchPattern]>,
        rest: Option<Box<WireStructuralMatchPattern>>,
        suffix: Box<[WireStructuralMatchPattern]>,
    },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
enum WireControlParameterSource {
    Scrutinee,
    PatternBinding(u32),
    Capture(u16),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireControlBlock {
    id: u32,
    parameters: Box<[(WireControlParameterSource, u32)]>,
    operations: Box<[WireControlOperation]>,
    yield_value: WireControlValue,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireControlOperation {
    node: u32,
    body: WireControlOperationBody,
    inputs: Box<[WireControlValue]>,
    schema: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum WireControlOperationBody {
    Operation { operation: u32, contract: u32 },
    Match(WireMatchDeclaration),
    Comprehension(WireComprehensionDeclaration),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum WireControlValue {
    Constant(u32),
    Parameter { block: u32, ordinal: u16 },
    Local { block: u32, node: u32 },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct WireGraph {
    revision: u32,
    requirements: Box<[WireRequirement]>,
    nodes: Box<[WireNode]>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct WireRequirement {
    kind: u8,
    host_name: Option<String>,
    base_uri: Option<String>,
    path: Option<String>,
    context_name: Option<String>,
    operation: Option<String>,
    intent: Option<u8>,
    delivery: Option<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum WireSource {
    Constant(u32),
    Slot(u32),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum WireBinding {
    Input {
        id: u32,
        node: u32,
        port_ordinal: u16,
        source: WireSource,
    },
    Output {
        id: u32,
        node: u32,
        port_ordinal: u16,
        target: u32,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct WireOutput {
    output: u32,
    name: String,
    source: u32,
    schema: u32,
    // Keep optional extensions at the end so ordinary JSON output declarations
    // retain their canonical field order; the default admits older artifacts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    interactive_binding: Option<WireInteractiveSymbolBinding>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct WireInteractiveSymbolBinding {
    lexical_name: String,
    artifact_source: WireSource,
    storage: u32,
    output: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct WireConstraint {
    constraint: u32,
    #[serde(default)]
    name: String,
    operation: u32,
    contract: u32,
    inputs: Box<[WireSource]>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct WireOperation {
    module_path: Box<[String]>,
    operation_name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct WireComputeRegion {
    name: String,
    placement: u8,
    nodes: Box<[u32]>,
}

pub fn encode_program_artifact_bytecode_v1(
    artifact: &ProgramArtifact,
) -> Result<Vec<u8>, ArtifactBytecodeError> {
    let sections = encode_program_artifact_sections(artifact)?;
    let program = BytecodeProgram {
        register_count: 0,
        constants: Vec::new(),
        symbols: BTreeMap::new(),
        mutable_symbols: BTreeSet::new(),
        instructions: Vec::new(),
        dictionary: BTreeMap::new(),
        requirements: artifact
            .requirements()
            .iter()
            .map(|(_, requirement)| requirement.clone())
            .collect(),
    };
    Ok(write_bytecode_with_artifact(&program, &sections)?)
}

pub fn encode_program_artifact_sections(
    artifact: &ProgramArtifact,
) -> Result<BytecodeArtifactSections, ArtifactBytecodeError> {
    let schemas = schema_drafts(artifact.schemas());
    let constants = constant_drafts(artifact.constants(), artifact.schemas())?;
    let (operations, operation_ids) = operation_table(artifact);
    let slots = artifact
        .slots()
        .iter()
        .map(|slot| WireSlot {
            slot: slot.slot.get(),
            schema: slot.schema.get(),
            role: match slot.role {
                SlotRole::Input => 1,
                SlotRole::State => 2,
                SlotRole::Derived => 3,
                SlotRole::Output => 4,
            },
            initializer: slot.initializer.map(|initializer| match initializer {
                InitializerReference::Constant(constant) => WireSource::Constant(constant.get()),
                InitializerReference::Activation(slot) => WireSource::Slot(slot.get()),
            }),
        })
        .collect::<Vec<_>>();
    let producers = artifact
        .slots()
        .iter()
        .map(|slot| match slot.producer {
            ProducerReference::Input(input) => WireProducer::Input(input.get()),
            ProducerReference::NodeOutput {
                node,
                output_ordinal,
            } => WireProducer::NodeOutput {
                node: node.get(),
                output_ordinal,
            },
            ProducerReference::Output { output, source } => WireProducer::Output {
                output: output.get(),
                source: source_to_wire(source),
            },
        })
        .collect::<Vec<_>>();
    Ok(BytecodeArtifactSections {
        schemas: encode(&schemas)?,
        constants: encode(&constants)?,
        inputs: encode(
            &artifact
                .inputs()
                .iter()
                .map(|input| WireInput {
                    input: input.input.get(),
                    name: input.name.clone(),
                    slot: input.slot.get(),
                    schema: input.schema.get(),
                })
                .collect::<Vec<_>>(),
        )?,
        slots: encode(&slots)?,
        producers: encode(&producers)?,
        nodes: encode(&WireGraph {
            revision: WIRE_GRAPH_REVISION,
            requirements: artifact
                .requirements()
                .iter()
                .map(|(_, requirement)| wire_requirement(requirement))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            nodes: artifact
                .nodes()
                .iter()
                .map(|node| WireNode {
                    node: node.node.get(),
                    body: wire_node_body(&node.body, &operation_ids),
                    input_start: node.input_bindings.start,
                    input_end: node.input_bindings.end,
                    output_start: node.output_bindings.start,
                    output_end: node.output_bindings.end,
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        })?,
        bindings: encode(
            &artifact
                .bindings()
                .iter()
                .map(wire_binding)
                .collect::<Vec<_>>(),
        )?,
        outputs: encode(
            &artifact
                .outputs()
                .iter()
                .map(|output| WireOutput {
                    output: output.output.get(),
                    name: output.name.clone(),
                    interactive_binding: output.interactive_binding.as_ref().map(|binding| {
                        WireInteractiveSymbolBinding {
                            lexical_name: binding.lexical_name.clone(),
                            artifact_source: wire_source(binding.artifact_source),
                            storage: binding.storage.get(),
                            output: binding.output.get(),
                        }
                    }),
                    source: output.source.get(),
                    schema: output.schema.get(),
                })
                .collect::<Vec<_>>(),
        )?,
        integrity_constraints: encode(
            &artifact
                .constraints()
                .iter()
                .map(|constraint| WireConstraint {
                    constraint: constraint.constraint.get(),
                    name: constraint.name.clone(),
                    operation: operation_ids[&constraint.operation],
                    contract: constraint.contract.get(),
                    inputs: constraint.inputs.iter().copied().map(wire_source).collect(),
                })
                .collect::<Vec<_>>(),
        )?,
        operations: encode(&operations)?,
        operation_contracts: artifact
            .contracts()
            .canonical_bytes()
            .map_err(ArtifactBuildError::from)?
            .into_vec(),
        compute_regions: if artifact.compute_regions().is_empty() {
            Vec::new()
        } else {
            encode(
                &artifact
                    .compute_regions()
                    .iter()
                    .map(|region| WireComputeRegion {
                        name: region.name.to_string(),
                        placement: match region.placement {
                            ComputePlacement::Compute => 1,
                            ComputePlacement::Cpu => 2,
                            ComputePlacement::Gpu => 3,
                        },
                        nodes: region.nodes.iter().map(|node| node.get()).collect(),
                    })
                    .collect::<Vec<_>>(),
            )?
        },
    })
}

pub fn decode_program_artifact_bytecode_v1(
    bytes: &[u8],
) -> Result<ProgramArtifact, ArtifactBytecodeError> {
    let parsed = ParsedProgram::from_bytes(bytes)?;
    decode_program_artifact_sections_with_requirements(
        &parsed.artifact,
        Some(parsed.requirements),
        ArtifactDecodeLimits::default(),
    )
}

pub fn decode_program_artifact_sections(
    sections: &BytecodeArtifactSections,
) -> Result<ProgramArtifact, ArtifactBytecodeError> {
    decode_program_artifact_sections_with_limits(sections, ArtifactDecodeLimits::default())
}

pub fn decode_program_artifact_sections_with_limits(
    sections: &BytecodeArtifactSections,
    limits: ArtifactDecodeLimits,
) -> Result<ProgramArtifact, ArtifactBytecodeError> {
    decode_program_artifact_sections_with_requirements(sections, None, limits)
}

fn decode_program_artifact_sections_with_requirements(
    sections: &BytecodeArtifactSections,
    requirements: Option<Vec<ApplicationRequirement>>,
    limits: ArtifactDecodeLimits,
) -> Result<ProgramArtifact, ArtifactBytecodeError> {
    decode_program_artifact_sections_owned(sections, requirements, limits)
}

fn decode_program_artifact_sections_owned(
    sections: &BytecodeArtifactSections,
    requirements: Option<Vec<ApplicationRequirement>>,
    limits: ArtifactDecodeLimits,
) -> Result<ProgramArtifact, ArtifactBytecodeError> {
    if sections.is_empty() {
        return Err(ArtifactBytecodeError::MissingArtifactSections);
    }
    validate_section_bytes(sections, limits)?;
    let schema_drafts: Vec<SchemaDraft> =
        decode_vec("schemas", &sections.schemas, limits.max_schemas)?;
    let schemas = finalize_schemas(schema_drafts)?;
    let value_drafts: Vec<ValueDraft> =
        decode_vec("constants", &sections.constants, limits.max_constants)?;
    let constants = finalize_constants(
        value_drafts,
        &schemas,
        limits.max_constant_canonicalization_work,
    )?;
    let contract_count = sections
        .operation_contracts
        .get(..4)
        .map(|bytes| u32::from_le_bytes(bytes.try_into().unwrap()) as usize)
        .ok_or(ArtifactBytecodeError::InvalidWireTag {
            section: "operation contracts",
            tag: 0,
        })?;
    if contract_count > limits.max_contracts {
        return Err(ArtifactBytecodeError::SectionItemLimit {
            section: "operation contracts",
            limit: limits.max_contracts,
            actual: contract_count,
        });
    }
    let contracts = OperationContractTable::from_canonical_bytes(&sections.operation_contracts)
        .map_err(ArtifactBuildError::from)?;
    let operations: Vec<WireOperation> =
        decode_vec("operations", &sections.operations, limits.max_operations)?;
    let inputs: Vec<WireInput> = decode_vec("inputs", &sections.inputs, limits.max_inputs)?;
    let slots: Vec<WireSlot> = decode_vec("slots", &sections.slots, limits.max_slots)?;
    let producers: Vec<WireProducer> =
        decode_vec("producers", &sections.producers, limits.max_slots)?;
    if slots.len() != producers.len() {
        return Err(ArtifactBytecodeError::InvalidWireTag {
            section: "producers",
            tag: 0,
        });
    }
    preflight_control_graph(&sections.nodes, &limits)?;
    let graph: WireGraph = serde_json::from_slice(&sections.nodes)?;
    if graph.revision != WIRE_GRAPH_REVISION {
        return Err(ArtifactBytecodeError::InvalidWireTag {
            section: "graph revision",
            tag: graph.revision.min(255) as u8,
        });
    }
    if graph.nodes.len() > limits.max_nodes {
        return Err(ArtifactBytecodeError::SectionItemLimit {
            section: "nodes",
            limit: limits.max_nodes,
            actual: graph.nodes.len(),
        });
    }
    if graph.requirements.len() > limits.max_requirements {
        return Err(ArtifactBytecodeError::SectionItemLimit {
            section: "application requirements",
            limit: limits.max_requirements,
            actual: graph.requirements.len(),
        });
    }
    let embedded_requirements = graph
        .requirements
        .into_vec()
        .into_iter()
        .map(requirement_from_wire)
        .collect::<Result<Vec<_>, _>>()?;
    if let Some(requirements) = requirements {
        if requirements != embedded_requirements {
            return Err(ArtifactBytecodeError::RequirementTableMismatch);
        }
    }
    let requirements = embedded_requirements;
    let nodes = graph.nodes.into_vec();
    let bindings: Vec<WireBinding> =
        decode_vec("bindings", &sections.bindings, limits.max_bindings)?;
    let outputs: Vec<WireOutput> = decode_vec("outputs", &sections.outputs, limits.max_outputs)?;
    let constraints: Vec<WireConstraint> = decode_vec(
        "integrity constraints",
        &sections.integrity_constraints,
        limits.max_constraints,
    )?;
    validate_operation_table(&operations, &nodes, &constraints)?;
    let operation = |id: u32| -> Result<OperationReference, ArtifactBytecodeError> {
        operations
            .get(id as usize)
            .map(|operation| OperationReference {
                module_path: operation.module_path.clone(),
                operation_name: operation.operation_name.clone(),
            })
            .ok_or(ArtifactBytecodeError::UnknownOperation { operation: id })
    };
    let mut draft = ProgramArtifactDraft {
        schemas,
        constants,
        contracts,
        requirements: ApplicationRequirementTable::from_canonical_entries(requirements)?,
        inputs: inputs
            .into_iter()
            .map(|input| InputDeclaration {
                input: InputId(input.input),
                name: input.name,
                slot: CellSlotId(input.slot),
                schema: SchemaId::new(input.schema),
            })
            .collect::<Vec<_>>()
            .into_boxed_slice(),
        slots: slots
            .into_iter()
            .zip(producers)
            .map(|(slot, producer)| {
                Ok(SlotDeclaration {
                    slot: CellSlotId(slot.slot),
                    schema: SchemaId::new(slot.schema),
                    role: match slot.role {
                        1 => SlotRole::Input,
                        2 => SlotRole::State,
                        3 => SlotRole::Derived,
                        4 => SlotRole::Output,
                        tag => {
                            return Err(ArtifactBytecodeError::InvalidWireTag {
                                section: "slots",
                                tag,
                            });
                        }
                    },
                    producer: match producer {
                        WireProducer::Input(input) => ProducerReference::Input(InputId(input)),
                        WireProducer::NodeOutput {
                            node,
                            output_ordinal,
                        } => ProducerReference::NodeOutput {
                            node: NodeId(node),
                            output_ordinal,
                        },
                        WireProducer::Output { output, source } => ProducerReference::Output {
                            output: OutputId(output),
                            source: source_from_wire(source),
                        },
                    },
                    initializer: slot.initializer.map(|source| match source {
                        WireSource::Constant(constant) => {
                            InitializerReference::Constant(ConstantId::new(constant))
                        }
                        WireSource::Slot(slot) => {
                            InitializerReference::Activation(CellSlotId(slot))
                        }
                    }),
                })
            })
            .collect::<Result<Vec<_>, ArtifactBytecodeError>>()?
            .into_boxed_slice(),
        nodes: nodes
            .into_iter()
            .map(|node| {
                Ok(NodeDeclaration {
                    node: NodeId(node.node),
                    body: node_body_from_wire(node.body, &operation)?,
                    input_bindings: node.input_start..node.input_end,
                    output_bindings: node.output_start..node.output_end,
                })
            })
            .collect::<Result<Vec<_>, ArtifactBytecodeError>>()?
            .into_boxed_slice(),
        bindings: bindings
            .into_iter()
            .map(binding_from_wire)
            .collect::<Vec<_>>()
            .into_boxed_slice(),
        outputs: outputs
            .into_iter()
            .map(|output| OutputDeclaration {
                output: OutputId(output.output),
                name: output.name,
                interactive_binding: output.interactive_binding.map(|binding| {
                    InteractiveSymbolBinding {
                        lexical_name: binding.lexical_name,
                        artifact_source: source_from_wire(binding.artifact_source),
                        storage: CellSlotId(binding.storage),
                        output: OutputId(binding.output),
                    }
                }),
                source: CellSlotId(output.source),
                schema: SchemaId::new(output.schema),
            })
            .collect::<Vec<_>>()
            .into_boxed_slice(),
        constraints: constraints
            .into_iter()
            .map(|constraint| {
                Ok(IntegrityConstraintDeclaration {
                    constraint: IntegrityConstraintId(constraint.constraint),
                    name: constraint.name,
                    operation: operation(constraint.operation)?,
                    contract: OperationContractId::new(constraint.contract),
                    inputs: constraint
                        .inputs
                        .into_vec()
                        .into_iter()
                        .map(source_from_wire)
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                })
            })
            .collect::<Result<Vec<_>, ArtifactBytecodeError>>()?
            .into_boxed_slice(),
        compute_regions: Box::new([]),
    };

    let wire_regions: Vec<WireComputeRegion> = if sections.compute_regions.is_empty() {
        Vec::new()
    } else {
        decode_vec(
            "compute regions",
            &sections.compute_regions,
            limits.max_compute_regions,
        )?
    };
    let mut names = BTreeSet::new();
    let mut assigned_nodes = BTreeSet::new();
    let mut compute_regions = Vec::with_capacity(wire_regions.len());
    for (region_index, region) in wire_regions.into_iter().enumerate() {
        if region.name.is_empty() || !names.insert(region.name.clone()) {
            return Err(ArtifactBytecodeError::InvalidWireTag {
                section: "compute regions",
                tag: 0,
            });
        }
        let placement = match region.placement {
            1 => ComputePlacement::Compute,
            2 => ComputePlacement::Cpu,
            3 => ComputePlacement::Gpu,
            tag => {
                return Err(ArtifactBytecodeError::InvalidWireTag {
                    section: "compute regions",
                    tag,
                });
            }
        };
        let mut previous = None;
        let nodes = region
            .nodes
            .into_vec()
            .into_iter()
            .map(|node| {
                if node as usize >= draft.nodes.len()
                    || previous.is_some_and(|previous| previous >= node)
                    || !assigned_nodes.insert(node)
                {
                    return Err(ArtifactBytecodeError::InvalidWireTag {
                        section: "compute regions",
                        tag: 0,
                    });
                }
                previous = Some(node);
                Ok(NodeId(node))
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_boxed_slice();
        compute_regions.push(ComputeRegionDeclaration {
            id: mech_core::ComputeRegionId::new(region_index as u32),
            name: region.name.into_boxed_str(),
            placement,
            nodes,
        });
    }
    draft.compute_regions = compute_regions.into_boxed_slice();
    draft.finalize().map_err(ArtifactBytecodeError::from)
}

fn validate_operation_table(
    operations: &[WireOperation],
    nodes: &[WireNode],
    constraints: &[WireConstraint],
) -> Result<(), ArtifactBytecodeError> {
    let mut canonical = nodes
        .iter()
        .flat_map(|node| wire_operation_ids(&node.body))
        .chain(constraints.iter().map(|constraint| constraint.operation))
        .map(|id| {
            operations
                .get(id as usize)
                .map(|operation| OperationReference {
                    module_path: operation.module_path.clone(),
                    operation_name: operation.operation_name.clone(),
                })
                .ok_or(ArtifactBytecodeError::UnknownOperation { operation: id })
        })
        .collect::<Result<Vec<_>, _>>()?;
    canonical.sort();
    canonical.dedup();
    let encoded = operations
        .iter()
        .map(|operation| OperationReference {
            module_path: operation.module_path.clone(),
            operation_name: operation.operation_name.clone(),
        })
        .collect::<Vec<_>>();
    if encoded != canonical {
        return Err(ArtifactBytecodeError::NonCanonicalOperationTable);
    }
    Ok(())
}

fn artifact_section_bytes(sections: &BytecodeArtifactSections) -> [(&'static str, &[u8]); 12] {
    [
        ("schemas", &sections.schemas),
        ("constants", &sections.constants),
        ("inputs", &sections.inputs),
        ("slots", &sections.slots),
        ("producers", &sections.producers),
        ("nodes", &sections.nodes),
        ("bindings", &sections.bindings),
        ("outputs", &sections.outputs),
        ("integrity constraints", &sections.integrity_constraints),
        ("operations", &sections.operations),
        ("operation contracts", &sections.operation_contracts),
        ("compute regions", &sections.compute_regions),
    ]
}

fn validate_section_bytes(
    sections: &BytecodeArtifactSections,
    limits: ArtifactDecodeLimits,
) -> Result<(), ArtifactBytecodeError> {
    let mut total = 0usize;
    for (section, bytes) in artifact_section_bytes(sections) {
        if bytes.len() > limits.max_section_bytes {
            return Err(ArtifactBytecodeError::SectionByteLimit {
                section,
                limit: limits.max_section_bytes,
            });
        }
        total =
            total
                .checked_add(bytes.len())
                .ok_or(ArtifactBytecodeError::AggregateByteLimit {
                    limit: limits.max_total_bytes,
                })?;
        if total > limits.max_total_bytes {
            return Err(ArtifactBytecodeError::AggregateByteLimit {
                limit: limits.max_total_bytes,
            });
        }
    }
    Ok(())
}

fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, ArtifactBytecodeError> {
    Ok(serde_json::to_vec(value)?)
}

fn decode_vec<T: DeserializeOwned>(
    section: &'static str,
    bytes: &[u8],
    limit: usize,
) -> Result<Vec<T>, ArtifactBytecodeError> {
    // JSON has no trusted encoded-capacity prefix: allocations grow only with
    // bytes already admitted by the section and aggregate byte limits above.
    // Count with `IgnoredAny` before constructing the typed Vec so the item
    // limit is also enforced before any element allocation.
    let mut counter = serde_json::Deserializer::from_slice(bytes);
    if let Err(error) = (CountSequence { limit }).deserialize(&mut counter) {
        if error.to_string().contains(ITEM_LIMIT_SENTINEL) {
            return Err(ArtifactBytecodeError::SectionItemLimit {
                section,
                limit,
                actual: limit.saturating_add(1),
            });
        }
        return Err(error.into());
    }
    counter.end()?;
    let value: Vec<T> = serde_json::from_slice(bytes)?;
    debug_assert!(value.len() <= limit);
    Ok(value)
}

const ITEM_LIMIT_SENTINEL: &str = "artifact-section-item-limit";

struct CountSequence {
    limit: usize,
}

impl<'de> DeserializeSeed<'de> for CountSequence {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

impl<'de> Visitor<'de> for CountSequence {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an artifact section array")
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut count = 0usize;
        while sequence.next_element::<IgnoredAny>()?.is_some() {
            if count >= self.limit {
                return Err(A::Error::custom(ITEM_LIMIT_SENTINEL));
            }
            count += 1;
        }
        Ok(())
    }
}

fn operation_table(
    artifact: &ProgramArtifact,
) -> (Vec<WireOperation>, BTreeMap<OperationReference, u32>) {
    let mut references = artifact
        .nodes()
        .iter()
        .flat_map(|node| node_operation_references(&node.body))
        .chain(
            artifact
                .constraints()
                .iter()
                .map(|constraint| constraint.operation.clone()),
        )
        .collect::<Vec<_>>();
    references.sort();
    references.dedup();
    let ids = references
        .iter()
        .enumerate()
        .map(|(id, operation)| {
            (
                operation.clone(),
                u32::try_from(id).expect("validated artifact operation count"),
            )
        })
        .collect();
    let operations = references
        .into_iter()
        .map(|operation| WireOperation {
            module_path: operation.module_path,
            operation_name: operation.operation_name,
        })
        .collect();
    (operations, ids)
}

fn wire_source(source: ArtifactSource) -> WireSource {
    match source {
        ArtifactSource::Constant(constant) => WireSource::Constant(constant.get()),
        ArtifactSource::Slot(slot) => WireSource::Slot(slot.get()),
    }
}

fn wire_requirement(requirement: &ApplicationRequirement) -> WireRequirement {
    match requirement {
        ApplicationRequirement::HostFunction(request) => WireRequirement {
            kind: 0,
            host_name: Some(request.name.clone()),
            base_uri: None,
            path: None,
            context_name: None,
            operation: None,
            intent: None,
            delivery: None,
        },
        ApplicationRequirement::Resource(request) => WireRequirement {
            kind: 1,
            host_name: None,
            base_uri: Some(request.base_uri.clone()),
            path: Some(request.path.clone()),
            context_name: Some(request.context_name.clone()),
            operation: Some(request.operation.clone()),
            intent: Some(request.intent as u8),
            delivery: Some(request.delivery as u8),
        },
    }
}

fn requirement_from_wire(
    requirement: WireRequirement,
) -> Result<ApplicationRequirement, ArtifactBytecodeError> {
    match requirement {
        WireRequirement {
            kind: 0,
            host_name: Some(name),
            base_uri: None,
            path: None,
            context_name: None,
            operation: None,
            intent: None,
            delivery: None,
        } => Ok(ApplicationRequirement::HostFunction(
            ExecutionHostFunctionRequest { name },
        )),
        WireRequirement {
            kind: 1,
            host_name: None,
            base_uri: Some(base_uri),
            path: Some(path),
            context_name: Some(context_name),
            operation: Some(operation),
            intent: Some(intent),
            delivery: Some(delivery),
        } => {
            let intent = match intent {
                1 => ResourceIntent::Read,
                2 => ResourceIntent::Assign,
                3 => ResourceIntent::Send,
                tag => {
                    return Err(ArtifactBytecodeError::InvalidWireTag {
                        section: "application requirements",
                        tag,
                    });
                }
            };
            let delivery = match delivery {
                0 => ResourceDelivery::Snapshot,
                1 => ResourceDelivery::Live,
                tag => {
                    return Err(ArtifactBytecodeError::InvalidWireTag {
                        section: "application requirements",
                        tag,
                    });
                }
            };
            Ok(ApplicationRequirement::Resource(ExecutionResourceRequest {
                base_uri,
                path,
                context_name,
                operation,
                intent,
                delivery,
            }))
        }
        other => Err(ArtifactBytecodeError::InvalidWireTag {
            section: "application requirements",
            tag: other.kind,
        }),
    }
}

fn source_from_wire(source: WireSource) -> ArtifactSource {
    match source {
        WireSource::Constant(constant) => ArtifactSource::Constant(ConstantId::new(constant)),
        WireSource::Slot(slot) => ArtifactSource::Slot(CellSlotId(slot)),
    }
}

fn source_to_wire(source: ArtifactSource) -> WireSource {
    match source {
        ArtifactSource::Constant(constant) => WireSource::Constant(constant.get()),
        ArtifactSource::Slot(slot) => WireSource::Slot(slot.get()),
    }
}

fn wire_binding(binding: &BindingDeclaration) -> WireBinding {
    match binding {
        BindingDeclaration::Input {
            id,
            node,
            port_ordinal,
            source,
        } => WireBinding::Input {
            id: id.get(),
            node: node.get(),
            port_ordinal: *port_ordinal,
            source: wire_source(*source),
        },
        BindingDeclaration::Output {
            id,
            node,
            port_ordinal,
            target,
        } => WireBinding::Output {
            id: id.get(),
            node: node.get(),
            port_ordinal: *port_ordinal,
            target: target.get(),
        },
    }
}

fn binding_from_wire(binding: WireBinding) -> BindingDeclaration {
    match binding {
        WireBinding::Input {
            id,
            node,
            port_ordinal,
            source,
        } => BindingDeclaration::Input {
            id: BindingId(id),
            node: NodeId(node),
            port_ordinal,
            source: source_from_wire(source),
        },
        WireBinding::Output {
            id,
            node,
            port_ordinal,
            target,
        } => BindingDeclaration::Output {
            id: BindingId(id),
            node: NodeId(node),
            port_ordinal,
            target: CellSlotId(target),
        },
    }
}

fn schema_drafts(table: &SchemaTable) -> Vec<SchemaDraft> {
    (0..table.len())
        .map(|index| {
            let schema = table.get(SchemaId::new(index as u32)).unwrap();
            SchemaDraft {
                dimension_parameters: schema
                    .dimension_parameters()
                    .iter()
                    .enumerate()
                    .map(|(id, parameter)| DimensionParameterDeclaration {
                        id: DimensionParameterId::new(id as u32),
                        origin: DimensionParameterOrigin::Explicit,
                        lifetime: parameter.lifetime(),
                        lower_bound: parameter.lower_bound().clone(),
                        upper_bound: parameter.upper_bound().cloned(),
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
                body: schema.body().clone(),
            }
        })
        .collect()
}

fn finalize_schemas(drafts: Vec<SchemaDraft>) -> Result<SchemaTable, ArtifactBytecodeError> {
    let mut builder = SchemaTableBuilder::new();
    let mut handles = Vec::with_capacity(drafts.len());
    for draft in drafts {
        handles.push(builder.insert(draft.finalize()?)?);
    }
    let build = builder.finish()?;
    for (expected, handle) in handles.into_iter().enumerate() {
        let found = build.resolve(handle)?.get();
        if found != expected as u32 {
            return Err(ArtifactBytecodeError::NonCanonicalSchemaId {
                expected: expected as u32,
                found,
            });
        }
    }
    Ok(build.into_parts().0)
}

fn constant_drafts(
    constants: &ConstantStore,
    schemas: &SchemaTable,
) -> Result<Vec<ValueDraft>, ArtifactBytecodeError> {
    (0..constants.len())
        .map(|index| {
            let id = ConstantId::new(index as u32);
            value_draft(id, constants.get(id).unwrap(), schemas)
        })
        .collect()
}

fn finalize_constants(
    drafts: Vec<ValueDraft>,
    schemas: &SchemaTable,
    canonicalization_work_limit: u64,
) -> Result<ConstantStore, ArtifactBytecodeError> {
    // One shared allowance covers every recursively nested set/map constant
    // in the artifact. Untrusted bytecode cannot restart the normalization
    // budget for each constant or defer an insertion-shift failure until
    // after an unbounded amount of decode-time work.
    let budget = SnapshotCanonicalizationBudget::new(canonicalization_work_limit);
    let validation = SnapshotValidationContext::new(schemas).with_canonicalization_budget(&budget);
    let mut builder = ConstantStoreBuilder::new(schemas);
    let mut handles = Vec::with_capacity(drafts.len());
    for draft in drafts {
        handles.push(builder.insert(draft.finalize(&validation)?)?);
    }
    let build = builder.finish()?;
    for (expected, handle) in handles.into_iter().enumerate() {
        let found = build.resolve(handle)?.get();
        if found != expected as u32 {
            return Err(ArtifactBytecodeError::NonCanonicalConstantId {
                expected: expected as u32,
                found,
            });
        }
    }
    Ok(build.into_parts().0)
}

fn value_draft(
    _constant: ConstantId,
    value: &Value,
    schemas: &SchemaTable,
) -> Result<ValueDraft, ArtifactBytecodeError> {
    let schema = schemas
        .get(value.schema())
        .ok_or(ArtifactBytecodeError::Artifact(
            ArtifactBuildError::UnknownSchema {
                schema: value.schema(),
            },
        ))?;
    Ok(ValueDraft {
        schema: value.schema(),
        shape_values: value.shape().parameter_values().to_vec().into_boxed_slice(),
        data: mech_core::snapshot::canonical_snapshot_data_draft_in(
            schema.body(),
            value.data(),
            schemas,
        )?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_core::snapshot::{MapEntryDraft, ValueDataDraft};
    use mech_core::{CardinalitySpec, IntegerWidth, SchemaBody};

    fn schema_table(body: SchemaBody) -> (SchemaTable, SchemaId) {
        let mut builder = SchemaTableBuilder::new();
        let handle = builder
            .insert(
                SchemaDraft {
                    dimension_parameters: Box::new([]),
                    body,
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let schema = build.resolve(handle).unwrap();
        let (schemas, _) = build.into_parts();
        (schemas, schema)
    }

    fn draft(schema: SchemaId, data: ValueDataDraft) -> ValueDraft {
        ValueDraft {
            schema,
            shape_values: Box::new([]),
            data,
        }
    }

    fn assert_work_limit(error: ArtifactBytecodeError, limit: u64) {
        assert!(matches!(
            error,
            ArtifactBytecodeError::Snapshot(
                SnapshotValueError::CanonicalizationWorkLimitExceededV1 { limit: found }
            ) if found == limit
        ));
    }

    #[test]
    fn dynamic_constant_projection_rebinds_foreign_nominal_schema_ids() {
        use mech_core::snapshot::CompositeSnapshotConstructor;
        use std::sync::Arc;
        let atom = |name: &str| {
            SchemaBody::Atom(mech_core::NominalKey::from_path(
                mech_core::NominalKind::Atom,
                &mech_core::CanonicalNominalPath::new(vec![name.to_owned()]).unwrap(),
            ))
        };
        let tuple = SchemaBody::Tuple(vec![SchemaBody::Dynamic].into_boxed_slice());
        let arena = |bodies: Vec<SchemaBody>| {
            let mut builder = SchemaTableBuilder::new();
            for body in bodies {
                builder
                    .insert(
                        SchemaDraft {
                            body,
                            dimension_parameters: Box::new([]),
                        }
                        .finalize()
                        .unwrap(),
                    )
                    .unwrap();
            }
            Arc::new(builder.finish().unwrap().into_parts().0)
        };
        let target = arena(vec![
            atom("ready"),
            atom("other"),
            atom("third"),
            tuple.clone(),
        ]);
        let outer = |child: Value, schemas: &Arc<SchemaTable>| {
            let child_id = schemas.find_by_key(child.schema_key()).unwrap();
            let tuple_schema = SchemaDraft {
                body: tuple.clone(),
                dimension_parameters: Box::new([]),
            }
            .finalize()
            .unwrap();
            let id = schemas.find_by_key(tuple_schema.key()).unwrap();
            CompositeSnapshotConstructor::bind(
                id,
                schemas
                    .get(id)
                    .unwrap()
                    .instantiate_shape(Box::new([]))
                    .unwrap(),
                &[(child_id, child.shape().clone())],
                Arc::clone(schemas),
            )
            .unwrap()
            .construct(vec![child].into_boxed_slice(), None)
            .unwrap()
        };
        let mut shifted = 0;
        for name in ["ready", "other", "third"] {
            let (foreign, id) = schema_table(atom(name));
            let child = draft(id, ValueDataDraft::Atom)
                .finalize(&SnapshotValidationContext::new(&foreign))
                .unwrap();
            if target.find_by_key(child.schema_key()).unwrap() != child.schema() {
                shifted += 1;
            }
            let foreign = arena(vec![atom(name), tuple.clone()]);
            let foreign_id = foreign.find_by_key(child.schema_key()).unwrap();
            let nested_child = child.rebind(foreign_id, child.shape(), &foreign).unwrap();
            for original in [
                outer(child, &target),
                outer(outer(nested_child, &foreign), &target),
            ] {
                let expected = original.canonical_snapshot_bytes(&target).unwrap();
                let missing = arena(vec![tuple.clone()]);
                assert!(
                    mech_core::snapshot::canonical_snapshot_data_draft_in(
                        &tuple,
                        original.data(),
                        &missing,
                    )
                    .is_err(),
                    "missing dynamic schema keys must fail closed"
                );
                let mut builder = ConstantStoreBuilder::new(&target);
                builder.insert(original).unwrap();
                let constants = builder.finish().unwrap().into_parts().0;
                let bytes = encode(&constant_drafts(&constants, &target).unwrap()).unwrap();
                let drafts = decode_vec("constants", &bytes, 1).unwrap();
                let decoded =
                    finalize_constants(drafts, &target, DEFAULT_MAX_CONSTANT_CANONICALIZATION_WORK)
                        .unwrap();
                assert_eq!(
                    decoded
                        .get(ConstantId::new(0))
                        .unwrap()
                        .canonical_snapshot_bytes(&target)
                        .unwrap(),
                    expected
                );
            }
        }
        assert!(
            shifted >= 2,
            "exercise compatible nominal ordinals from different arenas"
        );
    }

    #[test]
    fn artifact_constant_finalization_has_one_fail_closed_budget() {
        let cardinality = CardinalitySpec::Dynamic { upper_bound: None };
        let (set_schemas, set_schema) = schema_table(SchemaBody::Set {
            element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
            cardinality: cardinality.clone(),
        });
        let descending_set = ValueDataDraft::Set(
            (0_u64..1_024)
                .rev()
                .map(ValueDataDraft::U64)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        );
        assert_work_limit(
            finalize_constants(
                vec![draft(set_schema, descending_set)],
                &set_schemas,
                DEFAULT_MAX_CONSTANT_CANONICALIZATION_WORK,
            )
            .unwrap_err(),
            DEFAULT_MAX_CONSTANT_CANONICALIZATION_WORK,
        );

        let (map_schemas, map_schema) = schema_table(SchemaBody::Map {
            key: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
            value: Box::new(SchemaBody::Bool),
            cardinality,
        });
        let descending_map = ValueDataDraft::Map(
            (0_u64..1_024)
                .rev()
                .map(|key| MapEntryDraft {
                    items: vec![ValueDataDraft::U64(key), ValueDataDraft::Bool(true)]
                        .into_boxed_slice(),
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        );
        assert_work_limit(
            finalize_constants(
                vec![draft(map_schema, descending_map)],
                &map_schemas,
                DEFAULT_MAX_CONSTANT_CANONICALIZATION_WORK,
            )
            .unwrap_err(),
            DEFAULT_MAX_CONSTANT_CANONICALIZATION_WORK,
        );

        // The allowance belongs to the artifact, not to each constant. Four
        // ascending keys consume three comparisons; two constants cannot each
        // restart a five-unit decode allowance.
        let ascending = |start| {
            ValueDataDraft::Set(
                (start..start + 4)
                    .map(ValueDataDraft::U64)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            )
        };
        assert_work_limit(
            finalize_constants(
                vec![
                    draft(set_schema, ascending(0)),
                    draft(set_schema, ascending(4)),
                ],
                &set_schemas,
                5,
            )
            .unwrap_err(),
            5,
        );
    }
}

fn wire_control_value(value: super::ControlValue) -> WireControlValue {
    match value {
        super::ControlValue::Constant(id) => WireControlValue::Constant(id.get()),
        super::ControlValue::Parameter { block, ordinal } => WireControlValue::Parameter {
            block: block.0,
            ordinal,
        },
        super::ControlValue::Local { block, node } => WireControlValue::Local {
            block: block.0,
            node,
        },
    }
}

fn control_value_from_wire(value: WireControlValue) -> super::ControlValue {
    match value {
        WireControlValue::Constant(id) => super::ControlValue::Constant(ConstantId::new(id)),
        WireControlValue::Parameter { block, ordinal } => super::ControlValue::Parameter {
            block: super::ControlBlockId(block),
            ordinal,
        },
        WireControlValue::Local { block, node } => super::ControlValue::Local {
            block: super::ControlBlockId(block),
            node,
        },
    }
}

fn wire_control_block(
    block: &super::ControlBlock,
    operations: &BTreeMap<OperationReference, u32>,
) -> WireControlBlock {
    WireControlBlock {
        id: block.id.0,
        parameters: block
            .parameters
            .iter()
            .map(|parameter| {
                (
                    match parameter.source {
                        super::ControlParameterSource::Scrutinee => {
                            WireControlParameterSource::Scrutinee
                        }
                        super::ControlParameterSource::PatternBinding(local) => {
                            WireControlParameterSource::PatternBinding(local)
                        }
                        super::ControlParameterSource::Capture(index) => {
                            WireControlParameterSource::Capture(index)
                        }
                    },
                    parameter.schema.get(),
                )
            })
            .collect(),
        operations: block
            .operations
            .iter()
            .map(|operation| WireControlOperation {
                node: operation.node,
                body: wire_control_operation_body(&operation.body, operations),
                schema: operation.schema.get(),
                inputs: operation
                    .inputs
                    .iter()
                    .copied()
                    .map(wire_control_value)
                    .collect(),
            })
            .collect(),
        yield_value: wire_control_value(block.yield_value),
    }
}

fn wire_control_operation_body(
    body: &super::ControlOperationBody,
    operations: &BTreeMap<OperationReference, u32>,
) -> WireControlOperationBody {
    match body {
        super::ControlOperationBody::Operation {
            operation,
            contract,
        } => WireControlOperationBody::Operation {
            operation: operations[operation],
            contract: contract.get(),
        },
        super::ControlOperationBody::Match(control) => {
            WireControlOperationBody::Match(wire_match(control, operations))
        }
        super::ControlOperationBody::Comprehension(control) => {
            WireControlOperationBody::Comprehension(wire_comprehension(control, operations))
        }
    }
}

fn wire_comprehension(
    control: &super::ComprehensionDeclaration,
    operations: &BTreeMap<OperationReference, u32>,
) -> WireComprehensionDeclaration {
    WireComprehensionDeclaration {
        id: control.id.0,
        kind: match control.kind {
            super::ComprehensionKind::Matrix => 0,
            super::ComprehensionKind::Set => 1,
        },
        steps: control
            .steps
            .iter()
            .map(|step| match step {
                super::ComprehensionStep::Generator { source, pattern } => {
                    WireComprehensionStep::Generator {
                        source: wire_comprehension_value(*source),
                        pattern: wire_collection_pattern(pattern),
                    }
                }
                super::ComprehensionStep::Filter(value) => {
                    WireComprehensionStep::Filter(wire_comprehension_value(*value))
                }
                super::ComprehensionStep::Operation(operation) => {
                    WireComprehensionStep::Operation {
                        local: operation.local,
                        body: wire_control_operation_body(&operation.body, operations),
                        inputs: operation
                            .inputs
                            .iter()
                            .copied()
                            .map(wire_comprehension_value)
                            .collect(),
                        schema: operation.schema.get(),
                    }
                }
            })
            .collect(),
        yield_value: wire_comprehension_value(control.yield_value),
    }
}

fn wire_comprehension_value(value: super::ComprehensionValue) -> WireComprehensionValue {
    match value {
        super::ComprehensionValue::Constant(id) => WireComprehensionValue::Constant(id.get()),
        super::ComprehensionValue::Input(ordinal) => WireComprehensionValue::Input(ordinal),
        super::ComprehensionValue::Local(local) => WireComprehensionValue::Local(local),
    }
}
fn comprehension_value_from_wire(value: WireComprehensionValue) -> super::ComprehensionValue {
    match value {
        WireComprehensionValue::Constant(id) => {
            super::ComprehensionValue::Constant(ConstantId::new(id))
        }
        WireComprehensionValue::Input(ordinal) => super::ComprehensionValue::Input(ordinal),
        WireComprehensionValue::Local(local) => super::ComprehensionValue::Local(local),
    }
}
fn wire_collection_pattern(pattern: &super::CollectionPattern) -> WireCollectionPattern {
    match pattern {
        super::CollectionPattern::Wildcard => WireCollectionPattern::Wildcard,
        super::CollectionPattern::Bind { local, schema } => WireCollectionPattern::Bind {
            local: *local,
            schema: schema.get(),
        },
        super::CollectionPattern::Equal(value) => {
            WireCollectionPattern::Equal(wire_comprehension_value(*value))
        }
        super::CollectionPattern::Enum { ordinal, payload } => WireCollectionPattern::Enum {
            ordinal: *ordinal,
            payload: payload
                .as_deref()
                .map(wire_collection_pattern)
                .map(Box::new),
        },
        super::CollectionPattern::Tuple(items) => {
            WireCollectionPattern::Tuple(items.iter().map(wire_collection_pattern).collect())
        }
        super::CollectionPattern::Array {
            prefix,
            rest,
            suffix,
        } => WireCollectionPattern::Array {
            prefix: prefix.iter().map(wire_collection_pattern).collect(),
            rest: rest
                .as_ref()
                .map(|rest| Box::new(wire_collection_pattern(rest))),
            suffix: suffix.iter().map(wire_collection_pattern).collect(),
        },
    }
}
fn collection_pattern_from_wire(pattern: WireCollectionPattern) -> super::CollectionPattern {
    match pattern {
        WireCollectionPattern::Wildcard => super::CollectionPattern::Wildcard,
        WireCollectionPattern::Bind { local, schema } => super::CollectionPattern::Bind {
            local,
            schema: SchemaId::new(schema),
        },
        WireCollectionPattern::Equal(value) => {
            super::CollectionPattern::Equal(comprehension_value_from_wire(value))
        }
        WireCollectionPattern::Enum { ordinal, payload } => super::CollectionPattern::Enum {
            ordinal,
            payload: payload.map(|payload| Box::new(collection_pattern_from_wire(*payload))),
        },
        WireCollectionPattern::Tuple(items) => super::CollectionPattern::Tuple(
            items
                .into_iter()
                .map(collection_pattern_from_wire)
                .collect(),
        ),
        WireCollectionPattern::Array {
            prefix,
            rest,
            suffix,
        } => super::CollectionPattern::Array {
            prefix: prefix
                .into_iter()
                .map(collection_pattern_from_wire)
                .collect(),
            rest: rest.map(|rest| Box::new(collection_pattern_from_wire(*rest))),
            suffix: suffix
                .into_iter()
                .map(collection_pattern_from_wire)
                .collect(),
        },
    }
}

fn wire_structural_match_pattern(
    pattern: &super::CollectionPattern<SchemaId, super::MatchPatternValue>,
) -> WireStructuralMatchPattern {
    match pattern {
        super::CollectionPattern::Wildcard => WireStructuralMatchPattern::Wildcard,
        super::CollectionPattern::Bind { local, schema } => WireStructuralMatchPattern::Bind {
            local: *local,
            schema: schema.get(),
        },
        super::CollectionPattern::Equal(super::MatchPatternValue::Literal(constant)) => {
            WireStructuralMatchPattern::Equal(WireMatchPatternValue::Literal(constant.get()))
        }
        super::CollectionPattern::Equal(super::MatchPatternValue::Binding(local)) => {
            WireStructuralMatchPattern::Equal(WireMatchPatternValue::Binding(*local))
        }
        super::CollectionPattern::Enum { ordinal, payload } => WireStructuralMatchPattern::Enum {
            ordinal: *ordinal,
            payload: payload
                .as_deref()
                .map(wire_structural_match_pattern)
                .map(Box::new),
        },
        super::CollectionPattern::Tuple(items) => WireStructuralMatchPattern::Tuple(
            items.iter().map(wire_structural_match_pattern).collect(),
        ),
        super::CollectionPattern::Array {
            prefix,
            rest,
            suffix,
        } => WireStructuralMatchPattern::Array {
            prefix: prefix.iter().map(wire_structural_match_pattern).collect(),
            rest: rest
                .as_deref()
                .map(wire_structural_match_pattern)
                .map(Box::new),
            suffix: suffix.iter().map(wire_structural_match_pattern).collect(),
        },
    }
}

fn structural_match_pattern_from_wire(
    pattern: WireStructuralMatchPattern,
) -> super::CollectionPattern<SchemaId, super::MatchPatternValue> {
    match pattern {
        WireStructuralMatchPattern::Wildcard => super::CollectionPattern::Wildcard,
        WireStructuralMatchPattern::Bind { local, schema } => super::CollectionPattern::Bind {
            local,
            schema: SchemaId::new(schema),
        },
        WireStructuralMatchPattern::Equal(WireMatchPatternValue::Literal(constant)) => {
            super::CollectionPattern::Equal(super::MatchPatternValue::Literal(ConstantId::new(
                constant,
            )))
        }
        WireStructuralMatchPattern::Equal(WireMatchPatternValue::Binding(local)) => {
            super::CollectionPattern::Equal(super::MatchPatternValue::Binding(local))
        }
        WireStructuralMatchPattern::Enum { ordinal, payload } => super::CollectionPattern::Enum {
            ordinal,
            payload: payload.map(|payload| Box::new(structural_match_pattern_from_wire(*payload))),
        },
        WireStructuralMatchPattern::Tuple(items) => super::CollectionPattern::Tuple(
            items
                .into_iter()
                .map(structural_match_pattern_from_wire)
                .collect(),
        ),
        WireStructuralMatchPattern::Array {
            prefix,
            rest,
            suffix,
        } => super::CollectionPattern::Array {
            prefix: prefix
                .into_iter()
                .map(structural_match_pattern_from_wire)
                .collect(),
            rest: rest.map(|rest| Box::new(structural_match_pattern_from_wire(*rest))),
            suffix: suffix
                .into_iter()
                .map(structural_match_pattern_from_wire)
                .collect(),
        },
    }
}

fn wire_node_body(
    body: &super::ExecutableNodeBody,
    operations: &BTreeMap<OperationReference, u32>,
) -> WireNodeBody {
    match body {
        super::ExecutableNodeBody::Operation(operation) => WireNodeBody::Operation {
            operation: operations[&operation.operation],
            contract: operation.contract.get(),
            requirement: operation.requirement.map(ApplicationRequirementId::get),
        },
        super::ExecutableNodeBody::Comprehension(control) => {
            WireNodeBody::Comprehension(wire_comprehension(control, operations))
        }
        super::ExecutableNodeBody::Match(control) => {
            WireNodeBody::Match(wire_match(control, operations))
        }
        super::ExecutableNodeBody::Fsm(control) => WireNodeBody::Fsm {
            machine: control.machine.clone(),
            arguments: control
                .arguments
                .iter()
                .map(|argument| (argument.name.clone(), argument.input))
                .collect(),
            stages: control
                .stages
                .iter()
                .map(|stage| WireFsmStage {
                    kind: match stage.kind {
                        super::FsmStageKind::State => 0,
                        super::FsmStageKind::Async => 1,
                        super::FsmStageKind::Output => 2,
                    },
                    value: wire_fsm_value(&stage.value),
                })
                .collect(),
        },
    }
}

fn wire_match(
    control: &super::MatchDeclaration,
    operations: &BTreeMap<OperationReference, u32>,
) -> WireMatchDeclaration {
    WireMatchDeclaration {
        scrutinee: control.scrutinee,
        partial: control.partial,
        captures: control
            .captures
            .iter()
            .map(|capture| (capture.input, capture.schema.get()))
            .collect(),
        arms: control
            .arms
            .iter()
            .map(|arm| WireMatchArm {
                pattern: match &arm.pattern {
                    super::MatchPattern::Literal(constant) => WirePattern::Literal(constant.get()),
                    super::MatchPattern::Wildcard => WirePattern::Wildcard,
                    super::MatchPattern::Bind => WirePattern::Bind,
                    super::MatchPattern::Structural(pattern) => {
                        WirePattern::Structural(wire_structural_match_pattern(pattern))
                    }
                },
                guard: arm
                    .guard
                    .as_ref()
                    .map(|block| wire_control_block(block, operations)),
                body: wire_control_block(&arm.body, operations),
            })
            .collect(),
    }
}

fn wire_fsm_value(value: &super::FsmValue) -> WireFsmValue {
    match value {
        super::FsmValue::Input(input) => WireFsmValue::Input(*input),
        super::FsmValue::Tuple(items) => {
            WireFsmValue::Tuple(items.iter().map(wire_fsm_value).collect())
        }
        super::FsmValue::Array(items) => {
            WireFsmValue::Array(items.iter().map(wire_fsm_value).collect())
        }
        super::FsmValue::AtomStruct { name, items } => WireFsmValue::AtomStruct {
            name: name.clone(),
            items: items.iter().map(wire_fsm_value).collect(),
        },
        super::FsmValue::TupleStruct { name, items } => WireFsmValue::TupleStruct {
            name: name.clone(),
            items: items.iter().map(wire_fsm_value).collect(),
        },
    }
}

fn fsm_value_from_wire(value: WireFsmValue) -> super::FsmValue {
    match value {
        WireFsmValue::Input(input) => super::FsmValue::Input(input),
        WireFsmValue::Tuple(items) => {
            super::FsmValue::Tuple(items.into_iter().map(fsm_value_from_wire).collect())
        }
        WireFsmValue::Array(items) => {
            super::FsmValue::Array(items.into_iter().map(fsm_value_from_wire).collect())
        }
        WireFsmValue::AtomStruct { name, items } => super::FsmValue::AtomStruct {
            name,
            items: items.into_iter().map(fsm_value_from_wire).collect(),
        },
        WireFsmValue::TupleStruct { name, items } => super::FsmValue::TupleStruct {
            name,
            items: items.into_iter().map(fsm_value_from_wire).collect(),
        },
    }
}

fn control_block_from_wire(
    block: WireControlBlock,
    operation: &impl Fn(u32) -> Result<OperationReference, ArtifactBytecodeError>,
) -> Result<super::ControlBlock, ArtifactBytecodeError> {
    Ok(super::ControlBlock {
        id: super::ControlBlockId(block.id),
        parameters: block
            .parameters
            .into_iter()
            .map(|(source, schema)| super::ControlParameter {
                source: match source {
                    WireControlParameterSource::Scrutinee => {
                        super::ControlParameterSource::Scrutinee
                    }
                    WireControlParameterSource::PatternBinding(local) => {
                        super::ControlParameterSource::PatternBinding(local)
                    }
                    WireControlParameterSource::Capture(index) => {
                        super::ControlParameterSource::Capture(index)
                    }
                },
                schema: SchemaId::new(schema),
            })
            .collect(),
        operations: block
            .operations
            .into_iter()
            .map(|node| {
                Ok(super::ControlOperation {
                    node: node.node,
                    body: control_operation_body_from_wire(node.body, operation)?,
                    schema: SchemaId::new(node.schema),
                    inputs: node
                        .inputs
                        .into_iter()
                        .map(control_value_from_wire)
                        .collect(),
                })
            })
            .collect::<Result<Box<[_]>, ArtifactBytecodeError>>()?,
        yield_value: control_value_from_wire(block.yield_value),
    })
}

fn control_operation_body_from_wire(
    body: WireControlOperationBody,
    operation: &impl Fn(u32) -> Result<OperationReference, ArtifactBytecodeError>,
) -> Result<super::ControlOperationBody, ArtifactBytecodeError> {
    Ok(match body {
        WireControlOperationBody::Operation {
            operation: reference,
            contract,
        } => super::ControlOperationBody::Operation {
            operation: operation(reference)?,
            contract: OperationContractId::new(contract),
        },
        WireControlOperationBody::Match(control) => {
            super::ControlOperationBody::Match(match_from_wire(control, operation)?)
        }
        WireControlOperationBody::Comprehension(control) => {
            super::ControlOperationBody::Comprehension(comprehension_from_wire(control, operation)?)
        }
    })
}

fn comprehension_from_wire(
    control: WireComprehensionDeclaration,
    operation: &impl Fn(u32) -> Result<OperationReference, ArtifactBytecodeError>,
) -> Result<super::ComprehensionDeclaration, ArtifactBytecodeError> {
    Ok(super::ComprehensionDeclaration {
        id: super::ControlBlockId(control.id),
        kind: match control.kind {
            0 => super::ComprehensionKind::Matrix,
            1 => super::ComprehensionKind::Set,
            tag => {
                return Err(ArtifactBytecodeError::InvalidWireTag {
                    section: "comprehension kind",
                    tag,
                });
            }
        },
        steps: control
            .steps
            .into_iter()
            .map(|step| {
                Ok(match step {
                    WireComprehensionStep::Generator { source, pattern } => {
                        super::ComprehensionStep::Generator {
                            source: comprehension_value_from_wire(source),
                            pattern: collection_pattern_from_wire(pattern),
                        }
                    }
                    WireComprehensionStep::Filter(value) => {
                        super::ComprehensionStep::Filter(comprehension_value_from_wire(value))
                    }
                    WireComprehensionStep::Operation {
                        local,
                        body,
                        inputs,
                        schema,
                    } => super::ComprehensionStep::Operation(super::ComprehensionOperation {
                        local,
                        body: control_operation_body_from_wire(body, operation)?,
                        inputs: inputs
                            .into_iter()
                            .map(comprehension_value_from_wire)
                            .collect(),
                        schema: SchemaId::new(schema),
                    }),
                })
            })
            .collect::<Result<Box<[_]>, ArtifactBytecodeError>>()?,
        yield_value: comprehension_value_from_wire(control.yield_value),
    })
}

fn node_body_from_wire(
    body: WireNodeBody,
    operation: &impl Fn(u32) -> Result<OperationReference, ArtifactBytecodeError>,
) -> Result<super::ExecutableNodeBody, ArtifactBytecodeError> {
    Ok(match body {
        WireNodeBody::Operation {
            operation: id,
            contract,
            requirement,
        } => super::ExecutableNodeBody::Operation(super::OperationNodeBody {
            operation: operation(id)?,
            contract: OperationContractId::new(contract),
            requirement: requirement.map(ApplicationRequirementId::new),
        }),
        WireNodeBody::Comprehension(control) => {
            super::ExecutableNodeBody::Comprehension(comprehension_from_wire(control, operation)?)
        }
        WireNodeBody::Match(control) => {
            super::ExecutableNodeBody::Match(match_from_wire(control, operation)?)
        }
        WireNodeBody::Fsm {
            machine,
            arguments,
            stages,
        } => super::ExecutableNodeBody::Fsm(super::FsmDeclaration {
            machine,
            arguments: arguments
                .into_iter()
                .map(|(name, input)| super::FsmArgument { name, input })
                .collect(),
            stages: stages
                .into_iter()
                .map(|stage| {
                    Ok(super::FsmStage {
                        kind: match stage.kind {
                            0 => super::FsmStageKind::State,
                            1 => super::FsmStageKind::Async,
                            2 => super::FsmStageKind::Output,
                            tag => {
                                return Err(ArtifactBytecodeError::InvalidWireTag {
                                    section: "FSM stage kind",
                                    tag,
                                });
                            }
                        },
                        value: fsm_value_from_wire(stage.value),
                    })
                })
                .collect::<Result<Box<[_]>, ArtifactBytecodeError>>()?,
        }),
    })
}

fn match_from_wire(
    control: WireMatchDeclaration,
    operation: &impl Fn(u32) -> Result<OperationReference, ArtifactBytecodeError>,
) -> Result<super::MatchDeclaration, ArtifactBytecodeError> {
    let WireMatchDeclaration {
        scrutinee,
        partial,
        captures,
        arms,
    } = control;
    Ok(super::MatchDeclaration {
        scrutinee,
        partial,
        captures: captures
            .into_iter()
            .map(|(input, schema)| super::ControlCapture {
                input,
                schema: SchemaId::new(schema),
            })
            .collect(),
        arms: arms
            .into_iter()
            .map(|arm| {
                Ok(super::ControlMatchArm {
                    pattern: match arm.pattern {
                        WirePattern::Literal(constant) => {
                            super::MatchPattern::Literal(ConstantId::new(constant))
                        }
                        WirePattern::Wildcard => super::MatchPattern::Wildcard,
                        WirePattern::Bind => super::MatchPattern::Bind,
                        WirePattern::Structural(pattern) => super::MatchPattern::Structural(
                            structural_match_pattern_from_wire(pattern),
                        ),
                    },
                    guard: arm
                        .guard
                        .map(|block| control_block_from_wire(block, operation))
                        .transpose()?,
                    body: control_block_from_wire(arm.body, operation)?,
                })
            })
            .collect::<Result<Box<[_]>, ArtifactBytecodeError>>()?,
    })
}

fn node_operation_references(body: &super::ExecutableNodeBody) -> Vec<OperationReference> {
    match body {
        super::ExecutableNodeBody::Operation(operation) => vec![operation.operation.clone()],
        super::ExecutableNodeBody::Comprehension(control) => {
            comprehension_operation_references(control)
        }
        super::ExecutableNodeBody::Match(control) => match_operation_references(control),
        super::ExecutableNodeBody::Fsm(_) => Vec::new(),
    }
}

fn control_body_operation_references(
    body: &super::ControlOperationBody,
) -> Vec<OperationReference> {
    match body {
        super::ControlOperationBody::Operation { operation, .. } => vec![operation.clone()],
        super::ControlOperationBody::Match(control) => match_operation_references(control),
        super::ControlOperationBody::Comprehension(control) => {
            comprehension_operation_references(control)
        }
    }
}

fn match_operation_references(control: &super::MatchDeclaration) -> Vec<OperationReference> {
    control
        .arms
        .iter()
        .flat_map(|arm| arm.guard.iter().chain(core::iter::once(&arm.body)))
        .flat_map(|block| {
            block
                .operations
                .iter()
                .flat_map(|operation| control_body_operation_references(&operation.body))
        })
        .collect()
}

fn comprehension_operation_references(
    control: &super::ComprehensionDeclaration,
) -> Vec<OperationReference> {
    control
        .operations()
        .flat_map(|operation| control_body_operation_references(&operation.body))
        .collect()
}

fn wire_operation_ids(body: &WireNodeBody) -> Vec<u32> {
    match body {
        WireNodeBody::Operation { operation, .. } => vec![*operation],
        WireNodeBody::Comprehension(control) => wire_comprehension_operation_ids(control),
        WireNodeBody::Match(control) => wire_match_operation_ids(control),
        WireNodeBody::Fsm { .. } => Vec::new(),
    }
}

fn wire_control_body_operation_ids(body: &WireControlOperationBody) -> Vec<u32> {
    match body {
        WireControlOperationBody::Operation { operation, .. } => vec![*operation],
        WireControlOperationBody::Match(control) => wire_match_operation_ids(control),
        WireControlOperationBody::Comprehension(control) => {
            wire_comprehension_operation_ids(control)
        }
    }
}

fn wire_comprehension_operation_ids(control: &WireComprehensionDeclaration) -> Vec<u32> {
    control
        .steps
        .iter()
        .flat_map(|step| match step {
            WireComprehensionStep::Operation { body, .. } => wire_control_body_operation_ids(body),
            _ => Vec::new(),
        })
        .collect()
}

fn wire_match_operation_ids(control: &WireMatchDeclaration) -> Vec<u32> {
    control
        .arms
        .iter()
        .flat_map(|arm| arm.guard.iter().chain(core::iter::once(&arm.body)))
        .flat_map(|block| {
            block
                .operations
                .iter()
                .flat_map(|operation| wire_control_body_operation_ids(&operation.body))
        })
        .collect()
}

/// First pass visits tokens without constructing graph arrays. The typed decode
/// can allocate only after aggregate control counts and nesting are admitted.
fn preflight_control_graph(
    bytes: &[u8],
    limits: &ArtifactDecodeLimits,
) -> Result<(), ArtifactBytecodeError> {
    // A structured FSM value adds an enum object, a struct object, and an
    // items array at each admitted semantic layer. The remaining allowance
    // covers the graph, node, FSM body, stage, and leaf containers.
    const MAX_CONTROL_GRAPH_WIRE_DEPTH: usize = super::fsm::MAX_FSM_VALUE_DEPTH * 3 + 16;

    #[derive(Clone, Copy)]
    enum Field {
        Other,
        Nodes,
        Requirements,
        Arms,
        Blocks,
        Operations,
        Operands,
        Generator,
        Pattern,
        PatternChildren,
        SingleOperand,
    }
    struct Counts {
        nodes: usize,
        requirements: usize,
        arms: usize,
        blocks: usize,
        operations: usize,
        operands: usize,
    }
    struct Scan<'a> {
        counts: &'a mut Counts,
        limits: &'a ArtifactDecodeLimits,
        field: Field,
        depth: usize,
        control_depth: usize,
    }
    impl Scan<'_> {
        fn charge<E: serde::de::Error>(&mut self) -> Result<(), E> {
            let (count, limit) = match self.field {
                Field::Other | Field::Generator | Field::PatternChildren => return Ok(()),
                Field::Nodes => (&mut self.counts.nodes, self.limits.max_nodes),
                Field::Requirements => {
                    (&mut self.counts.requirements, self.limits.max_requirements)
                }
                Field::Arms => (&mut self.counts.arms, self.limits.max_control_arms),
                Field::Blocks => (&mut self.counts.blocks, self.limits.max_control_blocks),
                Field::Operations => (
                    &mut self.counts.operations,
                    self.limits.max_control_operations,
                ),
                Field::Operands | Field::Pattern | Field::SingleOperand => {
                    (&mut self.counts.operands, self.limits.max_control_operands)
                }
            };
            *count = count
                .checked_add(1)
                .ok_or_else(|| E::custom("control count overflow"))?;
            if *count > limit {
                return Err(E::custom("control graph item limit"));
            }
            Ok(())
        }
    }
    impl<'de> DeserializeSeed<'de> for Scan<'_> {
        type Value = ();
        fn deserialize<D: serde::Deserializer<'de>>(
            mut self,
            deserializer: D,
        ) -> Result<(), D::Error> {
            // Keep the complete declared control depth below serde_json's own
            // recursion bound while admitting every valid FSM value depth.
            if self.depth > MAX_CONTROL_GRAPH_WIRE_DEPTH
                || self.control_depth > super::MAX_CONTROL_DEPTH
            {
                return Err(D::Error::custom("control graph nesting limit"));
            }
            if matches!(self.field, Field::SingleOperand) {
                self.charge::<D::Error>()?;
            }
            deserializer.deserialize_any(self)
        }
    }
    impl<'de> Visitor<'de> for Scan<'_> {
        type Value = ();
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("bounded control graph")
        }
        fn visit_bool<E: serde::de::Error>(self, _: bool) -> Result<(), E> {
            Ok(())
        }
        fn visit_i64<E: serde::de::Error>(self, _: i64) -> Result<(), E> {
            Ok(())
        }
        fn visit_u64<E: serde::de::Error>(self, _: u64) -> Result<(), E> {
            Ok(())
        }
        fn visit_f64<E: serde::de::Error>(self, _: f64) -> Result<(), E> {
            Ok(())
        }
        fn visit_str<E: serde::de::Error>(mut self, _: &str) -> Result<(), E> {
            if matches!(self.field, Field::Pattern) {
                self.charge::<E>()?;
            }
            Ok(())
        }
        fn visit_unit<E: serde::de::Error>(self) -> Result<(), E> {
            Ok(())
        }
        fn visit_seq<A: SeqAccess<'de>>(mut self, mut seq: A) -> Result<(), A::Error> {
            while seq
                .next_element_seed(Scan {
                    counts: self.counts,
                    limits: self.limits,
                    field: if matches!(self.field, Field::PatternChildren) {
                        Field::Pattern
                    } else {
                        Field::Other
                    },
                    depth: self.depth + 1,
                    control_depth: self.control_depth,
                })?
                .is_some()
            {
                self.charge::<A::Error>()?;
            }
            Ok(())
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(mut self, mut map: A) -> Result<(), A::Error> {
            if matches!(self.field, Field::Pattern) {
                self.charge::<A::Error>()?;
            }
            while let Some(key) = map.next_key::<String>()? {
                if key == "id" {
                    Scan {
                        counts: self.counts,
                        limits: self.limits,
                        field: Field::Blocks,
                        depth: self.depth,
                        control_depth: self.control_depth,
                    }
                    .charge::<A::Error>()?;
                }
                let field = match key.as_str() {
                    "nodes" => Field::Nodes,
                    "requirements" => Field::Requirements,
                    "arms" => Field::Arms,
                    "operations" | "steps" | "stages" => Field::Operations,
                    "inputs" | "parameters" | "captures" | "arguments" => Field::Operands,
                    "Generator" => Field::Generator,
                    "pattern" if matches!(self.field, Field::Generator) => Field::Pattern,
                    "Structural" => Field::Pattern,
                    "rest" => Field::Pattern,
                    "value" => Field::Pattern,
                    "Tuple" | "Array" | "items" | "prefix" | "suffix" => Field::PatternChildren,
                    "Filter" => Field::SingleOperand,
                    _ => Field::Other,
                };
                map.next_value_seed(Scan {
                    counts: self.counts,
                    limits: self.limits,
                    field,
                    depth: self.depth + 1,
                    control_depth: self.control_depth
                        + usize::from(matches!(key.as_str(), "Match" | "Comprehension")),
                })?;
            }
            Ok(())
        }
    }
    let mut counts = Counts {
        nodes: 0,
        requirements: 0,
        arms: 0,
        blocks: 0,
        operations: 0,
        operands: 0,
    };
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    Scan {
        counts: &mut counts,
        limits,
        field: Field::Other,
        depth: 0,
        control_depth: 0,
    }
    .deserialize(&mut decoder)?;
    decoder.end()?;
    Ok(())
}

#[cfg(test)]
mod control_preflight_tests {
    use super::{ArtifactDecodeLimits, WireComprehensionStep, preflight_control_graph};

    #[test]
    fn unknown_fields_cannot_hide_generator_patterns_from_admission() {
        // An ignored ID must not change the context of the following pattern.
        let bytes = br#"{"Generator":{"id":0,"source":{"Input":0},"pattern":{"Tuple":["Wildcard","Wildcard"]}}}"#;
        let limits = ArtifactDecodeLimits {
            max_control_operands: 2,
            ..ArtifactDecodeLimits::default()
        };
        assert!(preflight_control_graph(bytes, &limits).is_err());
        assert!(preflight_control_graph(bytes, &ArtifactDecodeLimits::default()).is_ok());
        assert!(serde_json::from_slice::<WireComprehensionStep>(bytes).is_err());
    }

    #[test]
    fn nested_comprehension_tags_are_bounded_before_typed_allocation() {
        let nested = |depth| {
            let mut value = "0".to_owned();
            for _ in 0..depth {
                value = format!(r#"{{"Comprehension":{value}}}"#);
            }
            value
        };
        assert!(
            preflight_control_graph(
                nested(super::super::MAX_CONTROL_DEPTH).as_bytes(),
                &ArtifactDecodeLimits::default(),
            )
            .is_ok()
        );
        assert!(
            preflight_control_graph(
                nested(super::super::MAX_CONTROL_DEPTH + 1).as_bytes(),
                &ArtifactDecodeLimits::default(),
            )
            .is_err()
        );
    }
}
