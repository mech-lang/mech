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

use super::snapshot::data_draft;
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
    initializer: Option<u32>,
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
enum WireNodeBody {
    Operation {
        operation: u32,
        contract: u32,
        requirement: Option<u32>,
    },
    BooleanMatch {
        scrutinee: u16,
        captures: Box<[(u16, u32)]>,
        arms: Box<[WireMatchArm]>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct WireMatchArm {
    pattern: u8,
    guard: Option<WireControlBlock>,
    body: WireControlBlock,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct WireControlBlock {
    id: u32,
    parameters: Box<[(Option<u16>, u32)]>,
    operations: Box<[WireControlOperation]>,
    yield_value: WireControlValue,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct WireControlOperation {
    node: u32,
    operation: u32,
    contract: u32,
    inputs: Box<[WireControlValue]>,
    schema: u32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
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
                InitializerReference::Constant(constant) => constant.get(),
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
            revision: 2,
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
    if graph.revision != 2 {
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
                    initializer: slot
                        .initializer
                        .map(|constant| InitializerReference::Constant(ConstantId::new(constant))),
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
    constant: ConstantId,
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
        data: data_draft(value.data(), schema.body()).ok_or(ArtifactBytecodeError::Artifact(
            ArtifactBuildError::UnknownConstant { constant },
        ))?,
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
                        super::ControlParameterSource::Scrutinee => None,
                        super::ControlParameterSource::Capture(index) => Some(index),
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
                operation: operations[&operation.operation],
                contract: operation.contract.get(),
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
        super::ExecutableNodeBody::BooleanMatch(control) => WireNodeBody::BooleanMatch {
            scrutinee: control.scrutinee,
            captures: control
                .captures
                .iter()
                .map(|capture| (capture.input, capture.schema.get()))
                .collect(),
            arms: control
                .arms
                .iter()
                .map(|arm| WireMatchArm {
                    pattern: match arm.pattern {
                        super::BooleanPattern::Literal(false) => 0,
                        super::BooleanPattern::Literal(true) => 1,
                        super::BooleanPattern::Wildcard => 2,
                        super::BooleanPattern::Bind => 3,
                    },
                    guard: arm
                        .guard
                        .as_ref()
                        .map(|block| wire_control_block(block, operations)),
                    body: wire_control_block(&arm.body, operations),
                })
                .collect(),
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
            .map(|(capture, schema)| super::ControlParameter {
                source: capture.map_or(
                    super::ControlParameterSource::Scrutinee,
                    super::ControlParameterSource::Capture,
                ),
                schema: SchemaId::new(schema),
            })
            .collect(),
        operations: block
            .operations
            .into_iter()
            .map(|node| {
                Ok(super::ControlOperation {
                    node: node.node,
                    operation: operation(node.operation)?,
                    contract: OperationContractId::new(node.contract),
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
        WireNodeBody::BooleanMatch {
            scrutinee,
            captures,
            arms,
        } => super::ExecutableNodeBody::BooleanMatch(super::BooleanMatchDeclaration {
            scrutinee,
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
                    Ok(super::BooleanMatchArm {
                        pattern: match arm.pattern {
                            0 => super::BooleanPattern::Literal(false),
                            1 => super::BooleanPattern::Literal(true),
                            2 => super::BooleanPattern::Wildcard,
                            3 => super::BooleanPattern::Bind,
                            tag => {
                                return Err(ArtifactBytecodeError::InvalidWireTag {
                                    section: "Boolean pattern",
                                    tag,
                                });
                            }
                        },
                        guard: arm
                            .guard
                            .map(|block| control_block_from_wire(block, operation))
                            .transpose()?,
                        body: control_block_from_wire(arm.body, operation)?,
                    })
                })
                .collect::<Result<Box<[_]>, ArtifactBytecodeError>>()?,
        }),
    })
}

fn node_operation_references(body: &super::ExecutableNodeBody) -> Vec<OperationReference> {
    match body {
        super::ExecutableNodeBody::Operation(operation) => vec![operation.operation.clone()],
        super::ExecutableNodeBody::BooleanMatch(control) => control
            .arms
            .iter()
            .flat_map(|arm| arm.guard.iter().chain(core::iter::once(&arm.body)))
            .flat_map(|block| {
                block
                    .operations
                    .iter()
                    .map(|operation| operation.operation.clone())
            })
            .collect(),
    }
}

fn wire_operation_ids(body: &WireNodeBody) -> Vec<u32> {
    match body {
        WireNodeBody::Operation { operation, .. } => vec![*operation],
        WireNodeBody::BooleanMatch { arms, .. } => arms
            .iter()
            .flat_map(|arm| arm.guard.iter().chain(core::iter::once(&arm.body)))
            .flat_map(|block| block.operations.iter().map(|operation| operation.operation))
            .collect(),
    }
}

/// First pass visits tokens without constructing graph arrays. The typed decode
/// can allocate only after aggregate control counts and nesting are admitted.
fn preflight_control_graph(
    bytes: &[u8],
    limits: &ArtifactDecodeLimits,
) -> Result<(), ArtifactBytecodeError> {
    #[derive(Clone, Copy)]
    enum Field {
        Other,
        Nodes,
        Requirements,
        Arms,
        Blocks,
        Operations,
        Operands,
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
        depth: u8,
    }
    impl Scan<'_> {
        fn charge<E: serde::de::Error>(&mut self) -> Result<(), E> {
            let (count, limit) = match self.field {
                Field::Other => return Ok(()),
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
                Field::Operands => (&mut self.counts.operands, self.limits.max_control_operands),
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
        fn deserialize<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
            if self.depth > 32 {
                return Err(D::Error::custom("control graph nesting limit"));
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
        fn visit_str<E: serde::de::Error>(self, _: &str) -> Result<(), E> {
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
                    field: Field::Other,
                    depth: self.depth + 1,
                })?
                .is_some()
            {
                self.charge::<A::Error>()?;
            }
            Ok(())
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(mut self, mut map: A) -> Result<(), A::Error> {
            while let Some(key) = map.next_key::<String>()? {
                if key == "id" {
                    self.field = Field::Blocks;
                    self.charge::<A::Error>()?;
                }
                let field = match key.as_str() {
                    "nodes" => Field::Nodes,
                    "requirements" => Field::Requirements,
                    "arms" => Field::Arms,
                    "operations" => Field::Operations,
                    "inputs" | "parameters" | "captures" => Field::Operands,
                    _ => Field::Other,
                };
                map.next_value_seed(Scan {
                    counts: self.counts,
                    limits: self.limits,
                    field,
                    depth: self.depth + 1,
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
    }
    .deserialize(&mut decoder)?;
    decoder.end()?;
    Ok(())
}
