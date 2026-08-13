//! Deterministic source-compiler graph lowering into `ProgramArtifact`.
//!
//! This is the semantic compiler product. Bytecode v1 encodes this artifact
//! and its decoder reconstructs and validates the same artifact; it does not
//! feed a separate compatibility graph back into this compiler.

use std::collections::BTreeMap;

use mech_core::{
    ApplicationRequirementId, BindingId, CellSlotId, ConstantId, ConstantStore, InputId,
    IntegrityConstraintId, MResult, NodeId, OperationContractDeclaration, OperationContractId,
    OperationContractTable, OutputId, SchemaId, SchemaTable,
};

#[cfg(feature = "compiler")]
use mech_bytecode::{
    CompiledBytecode, CompiledInstructionRole, CompiledNodeKind, CompiledSymbolDefinition,
};
#[cfg(feature = "compiler")]
use mech_core::snapshot::SnapshotValidationContext;
#[cfg(feature = "compiler")]
use mech_core::{
    ApplicationRequirement, BytecodeInstruction, CanonicalNominalPath, ConstantHandle,
    ConstantStoreBuilder, DimensionEnvironmentBuilder, DimensionExpr, FunctionCatalog,
    InputPortLayout, KindId, LegacyEmptyPolicy, LegacyExtentRole, LegacyExtentSite,
    LegacyNominalResolution, LegacyReferencePolicy, LegacyResolvedExtent, LegacySemanticContext,
    LegacySnapshotContext, LegacyValue, NamedKindPathResolver, NominalKind, OperationContractError,
    OutputConstruction, SchemaBody, SchemaHandle, SchemaTableBuilder, SemanticModelError,
    ValueKind, schema_from_legacy_value_kind, snapshot_from_legacy,
};

use super::{
    ApplicationRequirementTable, ArtifactBuildError, ArtifactComputeRegion, ArtifactSource,
    BindingDeclaration, InitializerReference, InputDeclaration, IntegrityConstraintDeclaration,
    NodeDeclaration, OperationReference, OutputDeclaration, ProducerReference, ProgramArtifact,
    ProgramArtifactDraft, SlotDeclaration, SlotRole,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceValue {
    Constant(ConstantId),
    Input(u32),
    State(u32),
    NodeOutput { node: u32, output_ordinal: u16 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceSlot {
    Input(u32),
    State(u32),
    NodeOutput { node: u32, output_ordinal: u16 },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceInput {
    pub name: String,
    pub schema: SchemaId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceState {
    pub schema: SchemaId,
    pub initializer: Option<ConstantId>,
    pub producer_node: u32,
    pub producer_output_ordinal: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceNodeOutput {
    State(u32),
    Derived { schema: SchemaId },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceNode {
    pub operation: OperationReference,
    pub requirement: Option<ApplicationRequirementId>,
    pub inputs: Box<[SourceValue]>,
    pub outputs: Box<[SourceNodeOutput]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceOutput {
    pub name: String,
    pub source: SourceSlot,
    pub schema: SchemaId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceIntegrityConstraint {
    pub operation: OperationReference,
    pub inputs: Box<[SourceValue]>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SourceProgram {
    pub requirements: ApplicationRequirementTable,
    pub inputs: Box<[SourceInput]>,
    pub states: Box<[SourceState]>,
    pub nodes: Box<[SourceNode]>,
    pub outputs: Box<[SourceOutput]>,
    pub constraints: Box<[SourceIntegrityConstraint]>,
}

pub struct ArtifactBuildContext<'a> {
    schemas: &'a SchemaTable,
    constants: &'a ConstantStore,
}

/// Resolves the exact provider-independent semantic contract that an external
/// requirement contributes to an immutable compiler artifact.
///
/// The source interpreter can discover resource requests without knowing a
/// host provider. D3 supplies this resolver only at the explicit resident
/// compilation boundary; ordinary compilation retains its existing behavior.
#[cfg(feature = "compiler")]
pub trait ExternalRequirementContractResolver {
    fn resolve_external_contract(
        &self,
        requirement: &ApplicationRequirement,
    ) -> MResult<Option<&'static OperationContractDeclaration>>;
}

#[cfg(feature = "compiler")]
pub fn resolve_compiled_external_contracts(
    compiled: &mut CompiledBytecode,
    resolver: &dyn ExternalRequirementContractResolver,
) -> MResult<()> {
    for (instruction, contract) in compiled
        .program
        .instructions
        .iter()
        .zip(&mut compiled.instruction_contracts)
    {
        let requirement = match instruction {
            BytecodeInstruction::HostCall { requirement, .. }
            | BytecodeInstruction::ResourceRead { requirement, .. }
            | BytecodeInstruction::ResourceWrite { requirement, .. }
            | BytecodeInstruction::ResourceSend { requirement, .. } => *requirement,
            _ => continue,
        };
        let requirement = compiled
            .program
            .requirements
            .get(requirement as usize)
            .ok_or_else(|| {
                mech_core::MechError::new(
                    mech_core::GenericError {
                        msg: "compiled external requirement is out of range".to_owned(),
                    },
                    None,
                )
            })?;
        if let Some(resolved) = resolver.resolve_external_contract(requirement)? {
            *contract = Some(resolved);
        }
    }
    Ok(())
}

impl<'a> ArtifactBuildContext<'a> {
    pub const fn new(schemas: &'a SchemaTable, constants: &'a ConstantStore) -> Self {
        Self { schemas, constants }
    }

    pub const fn schemas(&self) -> &'a SchemaTable {
        self.schemas
    }

    pub const fn constants(&self) -> &'a ConstantStore {
        self.constants
    }
}

pub fn compile_source_program(
    graph: &SourceProgram,
    context: &mut ArtifactBuildContext<'_>,
) -> Result<ProgramArtifact, ArtifactBuildError> {
    compile_source_program_with_contracts(graph, context, &[])
}

/// Compiles a semantic source graph with declarations already selected by the
/// specializing compiler. The declaration slice is parallel to `graph.nodes`.
pub fn compile_source_program_with_contracts(
    graph: &SourceProgram,
    context: &mut ArtifactBuildContext<'_>,
    node_contracts: &[Option<&'static OperationContractDeclaration>],
) -> Result<ProgramArtifact, ArtifactBuildError> {
    let input_count = checked_u32(graph.inputs.len(), "InputId")?;
    let state_count = checked_u32(graph.states.len(), "CellSlotId")?;
    let mut next_slot = input_count.checked_add(state_count).ok_or(
        ArtifactBuildError::ArtifactIdentityExhausted {
            identity: "CellSlotId",
        },
    )?;

    let input_slots = (0..input_count).map(CellSlotId).collect::<Box<[_]>>();
    let state_slots = (0..state_count)
        .map(|state| CellSlotId(input_count + state))
        .collect::<Box<[_]>>();
    let mut output_slots = BTreeMap::<(u32, u16), CellSlotId>::new();
    for (node, declaration) in graph.nodes.iter().enumerate() {
        let node = checked_u32(node, "NodeId")?;
        for (ordinal, output) in declaration.outputs.iter().enumerate() {
            let ordinal = checked_u16(ordinal, "node output ordinal")?;
            let slot = match output {
                SourceNodeOutput::State(state) => *state_slots.get(*state as usize).ok_or(
                    ArtifactBuildError::SourceGraphReferenceOutOfRange {
                        reference: "state",
                        index: *state,
                    },
                )?,
                SourceNodeOutput::Derived { .. } => {
                    let slot = CellSlotId(next_slot);
                    next_slot = next_slot.checked_add(1).ok_or(
                        ArtifactBuildError::ArtifactIdentityExhausted {
                            identity: "CellSlotId",
                        },
                    )?;
                    slot
                }
            };
            output_slots.insert((node, ordinal), slot);
        }
    }

    let inputs = graph
        .inputs
        .iter()
        .enumerate()
        .map(|(index, input)| {
            let raw = checked_u32(index, "InputId")?;
            Ok(InputDeclaration {
                input: InputId(raw),
                name: input.name.clone(),
                slot: input_slots[index],
                schema: input.schema,
            })
        })
        .collect::<Result<Vec<_>, ArtifactBuildError>>()?;

    let mut slots = Vec::with_capacity(next_slot as usize);
    for (index, input) in inputs.iter().enumerate() {
        slots.push(SlotDeclaration {
            slot: input_slots[index],
            schema: input.schema,
            role: SlotRole::Input,
            producer: ProducerReference::Input(input.input),
            initializer: None,
        });
    }
    for (index, state) in graph.states.iter().enumerate() {
        slots.push(SlotDeclaration {
            slot: state_slots[index],
            schema: state.schema,
            role: SlotRole::State,
            producer: ProducerReference::NodeOutput {
                node: NodeId(state.producer_node),
                output_ordinal: state.producer_output_ordinal,
            },
            initializer: state.initializer.map(InitializerReference::Constant),
        });
    }
    for (node, declaration) in graph.nodes.iter().enumerate() {
        let raw_node = checked_u32(node, "NodeId")?;
        for (ordinal, output) in declaration.outputs.iter().enumerate() {
            let ordinal = checked_u16(ordinal, "node output ordinal")?;
            let SourceNodeOutput::Derived { schema } = output else {
                continue;
            };
            slots.push(SlotDeclaration {
                slot: output_slots[&(raw_node, ordinal)],
                schema: *schema,
                role: SlotRole::Derived,
                producer: ProducerReference::NodeOutput {
                    node: NodeId(raw_node),
                    output_ordinal: ordinal,
                },
                initializer: None,
            });
        }
    }
    slots.sort_by_key(|slot| slot.slot.get());

    let mut bindings = Vec::new();
    let mut nodes = Vec::with_capacity(graph.nodes.len());
    for (node_index, declaration) in graph.nodes.iter().enumerate() {
        let node = NodeId(checked_u32(node_index, "NodeId")?);
        let input_start = checked_u32(bindings.len(), "BindingId")?;
        for (ordinal, source) in declaration.inputs.iter().enumerate() {
            let id = BindingId(checked_u32(bindings.len(), "BindingId")?);
            bindings.push(BindingDeclaration::Input {
                id,
                node,
                port_ordinal: checked_u16(ordinal, "input port ordinal")?,
                source: resolve_source(*source, &input_slots, &state_slots, &output_slots)?,
            });
        }
        let input_end = checked_u32(bindings.len(), "BindingId")?;
        let output_start = input_end;
        for (ordinal, _) in declaration.outputs.iter().enumerate() {
            let ordinal = checked_u16(ordinal, "output port ordinal")?;
            let id = BindingId(checked_u32(bindings.len(), "BindingId")?);
            let target = *output_slots.get(&(node.get(), ordinal)).ok_or(
                ArtifactBuildError::SourceGraphReferenceOutOfRange {
                    reference: "node output",
                    index: node.get(),
                },
            )?;
            bindings.push(BindingDeclaration::Output {
                id,
                node,
                port_ordinal: ordinal,
                target,
            });
        }
        let output_end = checked_u32(bindings.len(), "BindingId")?;
        nodes.push(NodeDeclaration {
            node,
            operation: declaration.operation.clone(),
            contract: OperationContractId::new(0),
            requirement: declaration.requirement,
            input_bindings: input_start..input_end,
            output_bindings: output_start..output_end,
        });
    }

    let outputs = graph
        .outputs
        .iter()
        .enumerate()
        .map(|(index, output)| {
            Ok(OutputDeclaration {
                output: OutputId(checked_u32(index, "OutputId")?),
                name: output.name.clone(),
                source: resolve_slot_source(
                    output.source,
                    &input_slots,
                    &state_slots,
                    &output_slots,
                )?,
                schema: output.schema,
            })
        })
        .collect::<Result<Vec<_>, ArtifactBuildError>>()?;

    let constraints = graph
        .constraints
        .iter()
        .enumerate()
        .map(|(index, constraint)| {
            Ok(IntegrityConstraintDeclaration {
                constraint: IntegrityConstraintId(checked_u32(index, "IntegrityConstraintId")?),
                operation: constraint.operation.clone(),
                contract: OperationContractId::new(0),
                inputs: constraint
                    .inputs
                    .iter()
                    .map(|source| {
                        resolve_source(*source, &input_slots, &state_slots, &output_slots)
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .into_boxed_slice(),
            })
        })
        .collect::<Result<Vec<_>, ArtifactBuildError>>()?;

    let draft = ProgramArtifactDraft {
        schemas: context.schemas.clone(),
        constants: context.constants.clone(),
        contracts: OperationContractTable::empty(),
        requirements: graph.requirements.clone(),
        inputs: inputs.into_boxed_slice(),
        slots: slots.into_boxed_slice(),
        nodes: nodes.into_boxed_slice(),
        bindings: bindings.into_boxed_slice(),
        outputs: outputs.into_boxed_slice(),
        constraints: constraints.into_boxed_slice(),
    };
    if node_contracts.is_empty() {
        draft.attach_legacy_contracts()?.finalize()
    } else {
        draft.attach_contracts(node_contracts)?.finalize()
    }
}

#[cfg(feature = "compiler")]
#[derive(Clone, Copy)]
struct RegisterSemantic {
    source: SourceValue,
    schema: SchemaId,
}

#[cfg(feature = "compiler")]
#[derive(Clone, Debug, Default)]
struct CompilerLegacyContext {
    extent: u64,
}

#[cfg(feature = "compiler")]
impl CompilerLegacyContext {
    fn for_kind(
        kind: &ValueKind,
        collection_cardinality: Option<usize>,
        register: u32,
    ) -> Result<Self, ArtifactBuildError> {
        let extent = match kind {
            ValueKind::Matrix(_, dimensions) => dimensions.last().copied().unwrap_or(0),
            ValueKind::Table(_, rows) => *rows,
            ValueKind::Set(_, _) | ValueKind::Map(_, _) => collection_cardinality
                .ok_or(ArtifactBuildError::MissingRegisterCollectionCardinality { register })?,
            _ => 0,
        };
        Ok(Self {
            extent: extent as u64,
        })
    }
}

#[cfg(feature = "compiler")]
impl NamedKindPathResolver for CompilerLegacyContext {
    fn canonical_path(&self, _id: KindId) -> Option<&CanonicalNominalPath> {
        None
    }
}

#[cfg(feature = "compiler")]
impl LegacySemanticContext for CompilerLegacyContext {
    fn resolve_named_kind(&mut self, legacy_id: u64) -> Result<KindId, SemanticModelError> {
        Err(SemanticModelError::LegacyNamedKindUnresolved { legacy_id })
    }

    fn resolve_nominal(
        &mut self,
        nominal_kind: NominalKind,
        legacy_id: u64,
        legacy_name: &str,
    ) -> Result<LegacyNominalResolution, SemanticModelError> {
        Err(SemanticModelError::LegacyNominalUnresolved {
            kind: nominal_kind,
            legacy_id,
            legacy_name: legacy_name.to_owned(),
        })
    }

    fn resolve_unspecified_extent(
        &mut self,
        site: &LegacyExtentSite,
        _dimensions: &mut DimensionEnvironmentBuilder,
    ) -> Result<LegacyResolvedExtent, SemanticModelError> {
        let value = DimensionExpr::Constant(self.extent);
        Ok(match site.role {
            LegacyExtentRole::MatrixDimensions => {
                LegacyResolvedExtent::Dimensions(vec![value].into_boxed_slice())
            }
            LegacyExtentRole::TableRows
            | LegacyExtentRole::SetCardinality
            | LegacyExtentRole::MapCardinality => LegacyResolvedExtent::Cardinality(value),
        })
    }
}

/// Adapts the actual executable compiler product into C3's durable semantic
/// graph. Execution still consumes the existing bytecode/plan; this product is
/// emitted alongside it for bytecode-v1 persistence and later activation.
#[cfg(feature = "compiler")]
pub fn compile_executable_program_artifact(
    compiled: &CompiledBytecode,
    catalog: &FunctionCatalog,
) -> Result<ProgramArtifact, ArtifactBuildError> {
    compile_executable_program_artifact_product(compiled, catalog).map(|(artifact, _)| artifact)
}

#[cfg(feature = "compiler")]
pub fn compile_executable_program_artifact_product(
    compiled: &CompiledBytecode,
    catalog: &FunctionCatalog,
) -> Result<(ProgramArtifact, Box<[ArtifactComputeRegion]>), ArtifactBuildError> {
    validate_compiled_metadata_length(
        "instruction_roles",
        compiled.program.instructions.len(),
        compiled.instruction_roles.len(),
    )?;
    validate_compiled_metadata_length(
        "instruction_contracts",
        compiled.program.instructions.len(),
        compiled.instruction_contracts.len(),
    )?;
    validate_compiled_metadata_length(
        "instruction_source_nodes",
        compiled.program.instructions.len(),
        compiled.instruction_source_nodes.len(),
    )?;
    validate_compiled_metadata_length(
        "register_kinds",
        compiled.program.register_count as usize,
        compiled.register_kinds.len(),
    )?;
    validate_compiled_metadata_length(
        "register_collection_cardinalities",
        compiled.program.register_count as usize,
        compiled.register_collection_cardinalities.len(),
    )?;
    validate_compiled_metadata_length(
        "register_state_initializers",
        compiled.program.register_count as usize,
        compiled.register_state_initializers.len(),
    )?;
    validate_compiled_instruction_roles(compiled, catalog)?;

    let legacy_constants = mech_core::decode_encoded_constants(&compiled.program.constants)?;

    struct PendingRegisterSchema {
        handle: SchemaHandle,
        semantic: CompilerLegacyContext,
        contains_reference: bool,
    }

    let mut schema_builder = SchemaTableBuilder::new();
    let mut pending_register_schemas = Vec::with_capacity(compiled.register_kinds.len());
    for (register, kind) in compiled.register_kinds.iter().enumerate() {
        let Some(kind) = kind else {
            pending_register_schemas.push(None);
            continue;
        };
        if is_compiler_pseudo_kind(kind) {
            pending_register_schemas.push(None);
            continue;
        }
        let (kind, contains_reference) = normalize_legacy_binding_kind(kind.clone());
        let register = checked_u32(register, "bytecode register")?;
        let mut semantic = CompilerLegacyContext::for_kind(
            &kind,
            compiled.register_collection_cardinalities[register as usize],
            register,
        )?;
        let schema = schema_from_legacy_value_kind(&kind, &mut semantic)?;
        pending_register_schemas.push(Some(PendingRegisterSchema {
            handle: schema_builder.insert(schema)?,
            semantic,
            contains_reference,
        }));
    }
    let schema_build = schema_builder.finish()?;
    let register_schemas = pending_register_schemas
        .iter()
        .map(|entry| {
            entry
                .as_ref()
                .map(|entry| schema_build.resolve(entry.handle))
                .transpose()
        })
        .collect::<Result<Vec<_>, _>>()?;
    let (schemas, _) = schema_build.into_parts();

    let mut definitions_by_register =
        vec![Vec::<&CompiledSymbolDefinition>::new(); compiled.program.register_count as usize];
    for definition in &compiled.symbol_definitions {
        definitions_by_register
            .get_mut(definition.register as usize)
            .ok_or(ArtifactBuildError::SourceGraphReferenceOutOfRange {
                reference: "symbol register",
                index: definition.register,
            })?
            .push(definition);
    }
    for definitions in &mut definitions_by_register {
        definitions.sort_by_key(|definition| definition.ordinal);
    }
    let mut computed_registers = vec![false; compiled.program.register_count as usize];
    for (instruction, role) in compiled
        .program
        .instructions
        .iter()
        .zip(&compiled.instruction_roles)
    {
        let Some(CompiledInstructionRole::Node(_)) = role else {
            continue;
        };
        let Some(destination) = instruction_destination(instruction) else {
            continue;
        };
        if !is_variable_definition_instruction(instruction, catalog) {
            let computed = computed_registers.get_mut(destination as usize).ok_or(
                ArtifactBuildError::SourceGraphReferenceOutOfRange {
                    reference: "bytecode register",
                    index: destination,
                },
            )?;
            *computed = true;
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum CompilerConstantRole {
        Snapshot,
        ExternalInput,
        StateInitializer,
    }

    let mut register_constant_roles = vec![None; compiled.program.register_count as usize];
    for (register_index, pending_schema) in pending_register_schemas.iter().enumerate() {
        let Some(pending_schema) = pending_schema else {
            continue;
        };
        let register = checked_u32(register_index, "bytecode register")?;
        let definitions = &definitions_by_register[register_index];
        let first_mutable = definitions
            .iter()
            .find(|definition| definition.mutable)
            .map(|definition| definition.ordinal);
        let has_mutable = first_mutable.is_some();
        let has_immutable = definitions.iter().any(|definition| !definition.mutable);
        let immutable_precedes_state = first_mutable.is_some_and(|mutable_ordinal| {
            definitions
                .iter()
                .any(|definition| !definition.mutable && definition.ordinal < mutable_ordinal)
        });
        if has_mutable && has_immutable && immutable_precedes_state {
            return Err(ArtifactBuildError::AmbiguousRegisterRole { register });
        }
        register_constant_roles[register_index] = Some(if has_mutable {
            CompilerConstantRole::StateInitializer
        } else if pending_schema.contains_reference
            && has_immutable
            && !computed_registers[register_index]
        {
            CompilerConstantRole::ExternalInput
        } else {
            CompilerConstantRole::Snapshot
        });
    }

    let mut pending_constants = BTreeMap::<(u32, SchemaId), CompilerLegacyContext>::new();
    for instruction in &compiled.program.instructions {
        let (register, constant, initializer_only) = match instruction {
            BytecodeInstruction::ConstLoad { dst, constant } => (*dst, *constant, false),
            BytecodeInstruction::CompositePack { dst, template, .. } => (*dst, *template, true),
            _ => continue,
        };
        let register_index = register as usize;
        let Some(role) = register_constant_roles
            .get(register_index)
            .copied()
            .flatten()
        else {
            continue;
        };
        if role == CompilerConstantRole::ExternalInput
            || (initializer_only && role != CompilerConstantRole::StateInitializer)
        {
            continue;
        }
        if role == CompilerConstantRole::StateInitializer
            && compiled.register_state_initializers[register_index].is_some()
        {
            // The declaration snapshot is authoritative. The executable
            // register seed may already contain post-elaboration state and is
            // neither an artifact constant nor part of ProgramRevision.
            continue;
        }
        let schema = register_schemas
            .get(register_index)
            .copied()
            .flatten()
            .ok_or(ArtifactBuildError::MissingRegisterKind {
                instruction: 0,
                register,
            })?;
        let semantic = pending_register_schemas[register_index]
            .as_ref()
            .expect("schema and pending schema are parallel")
            .semantic
            .clone();
        legacy_constants.get(constant as usize).ok_or(
            ArtifactBuildError::SourceGraphReferenceOutOfRange {
                reference: "bytecode constant",
                index: constant,
            },
        )?;
        pending_constants
            .entry((constant, schema))
            .or_insert(semantic);
    }
    for (register, constant) in compiled.register_state_initializers.iter().enumerate() {
        let Some(constant) = constant else {
            continue;
        };
        let schema = register_schemas.get(register).copied().flatten().ok_or(
            ArtifactBuildError::MissingRegisterKind {
                instruction: 0,
                register: register as u32,
            },
        )?;
        let semantic = pending_register_schemas[register]
            .as_ref()
            .expect("schema and pending schema are parallel")
            .semantic
            .clone();
        legacy_constants.get(*constant as usize).ok_or(
            ArtifactBuildError::SourceGraphReferenceOutOfRange {
                reference: "state initializer bytecode constant",
                index: *constant,
            },
        )?;
        pending_constants
            .entry((*constant, schema))
            .or_insert(semantic);
    }

    let mut constant_builder = ConstantStoreBuilder::new(&schemas);
    let mut constant_handles = BTreeMap::<(u32, SchemaId), ConstantHandle>::new();
    for ((constant, schema), mut semantic) in pending_constants {
        let value = &legacy_constants[constant as usize];
        let named = semantic.clone();
        let validation = SnapshotValidationContext::with_named_kinds(&schemas, &named);
        let mut snapshot_context = LegacySnapshotContext::new(
            &mut semantic,
            LegacyEmptyPolicy::Reject,
            LegacyReferencePolicy::SnapshotCurrentValue,
        );
        let value = snapshot_from_legacy(
            value,
            schema,
            Box::new([]),
            &validation,
            &mut snapshot_context,
        )?;
        constant_handles.insert((constant, schema), constant_builder.insert(value)?);
    }
    let constant_build = constant_builder.finish()?;
    let constants = constant_handles
        .into_iter()
        .map(|(key, handle)| Ok((key, constant_build.resolve(handle)?)))
        .collect::<Result<BTreeMap<_, _>, ArtifactBuildError>>()?;
    let (constant_store, _) = constant_build.into_parts();
    let explicit_state_initializers = compiled
        .register_state_initializers
        .iter()
        .enumerate()
        .map(|(register, constant)| {
            let Some(constant) = constant else {
                return Ok(None);
            };
            let schema =
                register_schemas[register].ok_or(ArtifactBuildError::MissingRegisterKind {
                    instruction: 0,
                    register: register as u32,
                })?;
            constants
                .get(&(*constant, schema))
                .copied()
                .map(Some)
                .ok_or(ArtifactBuildError::SourceGraphReferenceOutOfRange {
                    reference: "state initializer constant",
                    index: *constant,
                })
        })
        .collect::<Result<Vec<_>, ArtifactBuildError>>()?;

    let mut inputs = Vec::<SourceInput>::new();
    let mut input_indexes = vec![None::<u32>; compiled.program.register_count as usize];
    let mut pending_inputs = Vec::<(u32, u32, String, SchemaId)>::new();
    for (register, role) in register_constant_roles.iter().enumerate() {
        if *role != Some(CompilerConstantRole::ExternalInput) {
            continue;
        }
        let register = checked_u32(register, "bytecode register")?;
        let definition = definitions_by_register[register as usize]
            .iter()
            .copied()
            .find(|definition| !definition.mutable)
            .ok_or(ArtifactBuildError::MissingInputInterfaceName { register })?;
        let schema =
            register_schemas[register as usize].ok_or(ArtifactBuildError::MissingRegisterKind {
                instruction: 0,
                register,
            })?;
        pending_inputs.push((
            definition.ordinal,
            register,
            definition.name.clone(),
            schema,
        ));
    }
    pending_inputs.sort_by_key(|(ordinal, ..)| *ordinal);
    for (_, register, name, schema) in pending_inputs {
        let input = checked_u32(inputs.len(), "InputId")?;
        inputs.push(SourceInput { name, schema });
        input_indexes[register as usize] = Some(input);
    }

    let mut state_initializers = explicit_state_initializers;
    for instruction in &compiled.program.instructions {
        let (register, constant) = match instruction {
            BytecodeInstruction::ConstLoad { dst, constant } => (*dst, *constant),
            BytecodeInstruction::CompositePack { dst, template, .. } => (*dst, *template),
            _ => continue,
        };
        if register_constant_roles
            .get(register as usize)
            .copied()
            .flatten()
            != Some(CompilerConstantRole::StateInitializer)
        {
            continue;
        }
        let schema =
            register_schemas[register as usize].ok_or(ArtifactBuildError::MissingRegisterKind {
                instruction: 0,
                register,
            })?;
        state_initializers[register as usize] =
            Some(constants.get(&(constant, schema)).copied().ok_or(
                ArtifactBuildError::SourceGraphReferenceOutOfRange {
                    reference: "state initializer constant",
                    index: constant,
                },
            )?);
    }

    let mut states = Vec::<SourceState>::new();
    let mut initial_state_by_register = vec![None::<u32>; compiled.program.register_count as usize];
    let mut state_indexes_by_instruction = vec![None::<u32>; compiled.program.instructions.len()];
    let mut lowered_node_count = 0_u32;
    for (instruction_index, instruction) in compiled.program.instructions.iter().enumerate() {
        let Some(CompiledInstructionRole::Node(kind)) =
            compiled.instruction_roles[instruction_index]
        else {
            continue;
        };
        let semantics =
            instruction_semantics(instruction, catalog, &compiled.program.requirements)?.ok_or(
                ArtifactBuildError::UnexpectedInstructionRole {
                    instruction: checked_u32(instruction_index, "instruction")?,
                    role: "node",
                },
            )?;
        if is_variable_definition_instruction(instruction, catalog) {
            continue;
        }
        if kind == CompiledNodeKind::Register {
            let register = semantics.destination;
            let state = checked_u32(states.len(), "state")?;
            let schema = register_schemas[register as usize].ok_or(
                ArtifactBuildError::MissingRegisterKind {
                    instruction: checked_u32(instruction_index, "instruction")?,
                    register,
                },
            )?;
            states.push(SourceState {
                schema,
                initializer: state_initializers[register as usize],
                producer_node: lowered_node_count,
                producer_output_ordinal: 0,
            });
            initial_state_by_register[register as usize].get_or_insert(state);
            state_indexes_by_instruction[instruction_index] = Some(state);
        }
        lowered_node_count = lowered_node_count.checked_add(1).ok_or(
            ArtifactBuildError::SourceGraphReferenceOutOfRange {
                reference: "NodeId",
                index: u32::MAX,
            },
        )?;
    }

    let mut registers = vec![None::<RegisterSemantic>; compiled.program.register_count as usize];
    let mut nodes = Vec::<SourceNode>::new();
    let mut source_node_origins = Vec::<Option<u32>>::new();
    let mut node_contracts = Vec::<Option<&'static OperationContractDeclaration>>::new();
    let mut lowered_declared_source_nodes = std::collections::BTreeSet::new();

    for (instruction_index, instruction) in compiled.program.instructions.iter().enumerate() {
        let instruction_id = checked_u32(instruction_index, "instruction")?;
        let role = compiled.instruction_roles[instruction_index];
        match instruction {
            BytecodeInstruction::ConstLoad { dst, constant } => {
                if let Some(role) = role {
                    return Err(ArtifactBuildError::UnexpectedInstructionRole {
                        instruction: instruction_id,
                        role: instruction_role_name(role),
                    });
                }
                let schema = register_schemas.get(*dst as usize).copied().flatten();
                let semantic = match register_constant_roles
                    .get(*dst as usize)
                    .copied()
                    .flatten()
                {
                    Some(CompilerConstantRole::ExternalInput) => input_indexes
                        .get(*dst as usize)
                        .copied()
                        .flatten()
                        .zip(schema)
                        .map(|(input, schema)| RegisterSemantic {
                            source: SourceValue::Input(input),
                            schema,
                        }),
                    Some(CompilerConstantRole::StateInitializer) => schema.and_then(|schema| {
                        initial_state_by_register
                            .get(*dst as usize)
                            .copied()
                            .flatten()
                            .map(|state| RegisterSemantic {
                                source: SourceValue::State(state),
                                schema,
                            })
                            .or_else(|| {
                                constants
                                    .get(&(*constant, schema))
                                    .copied()
                                    .map(|constant| RegisterSemantic {
                                        source: SourceValue::Constant(constant),
                                        schema,
                                    })
                            })
                    }),
                    Some(CompilerConstantRole::Snapshot) => schema.and_then(|schema| {
                        constants
                            .get(&(*constant, schema))
                            .copied()
                            .map(|constant| RegisterSemantic {
                                source: SourceValue::Constant(constant),
                                schema,
                            })
                    }),
                    None => None,
                };
                set_register(&mut registers, *dst, semantic)?;
            }
            BytecodeInstruction::Return { .. } => {
                if let Some(role) = role {
                    return Err(ArtifactBuildError::UnexpectedInstructionRole {
                        instruction: instruction_id,
                        role: instruction_role_name(role),
                    });
                }
            }
            instruction => {
                let kind = match role {
                    Some(CompiledInstructionRole::Node(kind)) => kind,
                    Some(CompiledInstructionRole::IntegrityMarker) => {
                        if !matches!(instruction, BytecodeInstruction::RuntimeVariadic { .. }) {
                            return Err(ArtifactBuildError::UnexpectedInstructionRole {
                                instruction: instruction_id,
                                role: "integrity marker",
                            });
                        }
                        continue;
                    }
                    Some(CompiledInstructionRole::DeclarationMarker) => {
                        if !matches!(instruction, BytecodeInstruction::RuntimeBinary { .. })
                            || !is_variable_definition_instruction(instruction, catalog)
                        {
                            return Err(ArtifactBuildError::InvalidDeclarationMarker {
                                instruction: instruction_id,
                            });
                        }
                        continue;
                    }
                    None => {
                        return Err(ArtifactBuildError::MissingInstructionRole {
                            instruction: instruction_id,
                        });
                    }
                };
                let semantics =
                    instruction_semantics(instruction, catalog, &compiled.program.requirements)?
                        .ok_or(ArtifactBuildError::UnexpectedInstructionRole {
                            instruction: instruction_id,
                            role: "node",
                        })?;
                let dst = semantics.destination;
                let prior = register(&registers, dst)?;
                let schema = register_schemas.get(dst as usize).copied().flatten();
                let pseudo_destination = compiled
                    .register_kinds
                    .get(dst as usize)
                    .and_then(Option::as_ref)
                    .is_some_and(is_compiler_pseudo_kind);
                if schema.is_none() && !pseudo_destination {
                    return Err(ArtifactBuildError::MissingRegisterKind {
                        instruction: instruction_id,
                        register: dst,
                    });
                }
                if is_variable_definition_instruction(instruction, catalog) {
                    if prior.is_none() && !pseudo_destination {
                        return Err(ArtifactBuildError::MissingRegisterSource {
                            instruction: instruction_id,
                            register: dst,
                            role: "variable definition",
                        });
                    }
                    continue;
                }
                let declaration = compiled.instruction_contracts[instruction_index].or_else(|| {
                    instruction
                        .runtime_function()
                        .and_then(|function| catalog.runtime_entry_by_raw(function))
                        .and_then(|entry| entry.semantic_contract())
                });
                let declaration = declaration.filter(|declaration| {
                    semantics.requirement.is_none_or(|requirement| {
                        compiled
                            .program
                            .requirements
                            .get(requirement.get() as usize)
                            .is_some_and(|requirement| {
                                external_declaration_matches_requirement(declaration, requirement)
                            })
                    })
                });
                let node_index = checked_u32(nodes.len(), "NodeId")?;
                let state_index = if kind == CompiledNodeKind::Register {
                    state_indexes_by_instruction[instruction_index]
                } else {
                    None
                };
                // Specialized plan nodes can expose a contract directly, but
                // catalog-installed runtime functions also carry authoritative
                // semantic metadata. Preserve that declaration when the
                // specialized function uses the trait's default `None`.
                let semantic_inputs = semantic_input_registers(&semantics, declaration)?
                    .iter()
                    .map(|input| {
                        if *input == dst && state_index.is_some() {
                            Ok(SourceValue::State(state_index.unwrap()))
                        } else {
                            register(&registers, *input)?
                                .map(|value| value.source)
                                .ok_or(ArtifactBuildError::MissingRegisterSource {
                                    instruction: instruction_id,
                                    register: *input,
                                    role: "input",
                                })
                        }
                    })
                    .collect::<Result<Vec<_>, ArtifactBuildError>>()?
                    .into_boxed_slice();
                if declaration.is_some() {
                    if let Some(source_node) = compiled.instruction_source_nodes[instruction_index]
                    {
                        if !lowered_declared_source_nodes.insert(source_node) {
                            return Err(
                                ArtifactBuildError::DeclaredSourceNodeLoweringUnsupported {
                                    source_node,
                                },
                            );
                        }
                    }
                }
                nodes.push(SourceNode {
                    operation: semantics.operation,
                    requirement: semantics.requirement,
                    inputs: semantic_inputs,
                    outputs: match (state_index, schema) {
                        (Some(state), _) => vec![SourceNodeOutput::State(state)],
                        (None, Some(schema)) => vec![SourceNodeOutput::Derived { schema }],
                        (None, None) => Vec::new(),
                    }
                    .into_boxed_slice(),
                });
                source_node_origins.push(compiled.instruction_source_nodes[instruction_index]);
                node_contracts.push(declaration);
                if schema.is_none() {
                    set_register(&mut registers, dst, None)?;
                    continue;
                }
                let source = match state_index {
                    Some(state) => SourceValue::State(state),
                    None if prior
                        .is_some_and(|value| matches!(value.source, SourceValue::State(_))) =>
                    {
                        prior.expect("checked prior state source").source
                    }
                    None => SourceValue::NodeOutput {
                        node: node_index,
                        output_ordinal: 0,
                    },
                };
                set_register(
                    &mut registers,
                    dst,
                    Some(RegisterSemantic {
                        source,
                        schema: schema.expect("checked semantic destination schema"),
                    }),
                )?;
            }
        }
    }

    let return_instruction = compiled
        .program
        .instructions
        .iter()
        .position(|instruction| matches!(instruction, BytecodeInstruction::Return { .. }))
        .and_then(|index| u32::try_from(index).ok())
        .unwrap_or(u32::MAX);
    let outputs = match register(&registers, compiled.return_register)? {
        Some(returned) => {
            let source = match returned.source {
                SourceValue::Constant(constant) => {
                    let node = checked_u32(nodes.len(), "NodeId")?;
                    nodes.push(SourceNode {
                        operation: OperationReference {
                            module_path: vec!["core".to_owned()].into_boxed_slice(),
                            operation_name: "literal".to_owned(),
                        },
                        requirement: None,
                        inputs: vec![SourceValue::Constant(constant)].into_boxed_slice(),
                        outputs: vec![SourceNodeOutput::Derived {
                            schema: returned.schema,
                        }]
                        .into_boxed_slice(),
                    });
                    source_node_origins.push(None);
                    node_contracts.push(None);
                    SourceSlot::NodeOutput {
                        node,
                        output_ordinal: 0,
                    }
                }
                SourceValue::Input(input) => SourceSlot::Input(input),
                SourceValue::State(state) => SourceSlot::State(state),
                SourceValue::NodeOutput {
                    node,
                    output_ordinal,
                } => SourceSlot::NodeOutput {
                    node,
                    output_ordinal,
                },
            };
            let output_name = definitions_by_register[compiled.return_register as usize]
                .iter()
                .max_by_key(|definition| definition.ordinal)
                .map(|definition| definition.name.clone())
                .unwrap_or_else(|| "result".to_owned());
            vec![SourceOutput {
                name: output_name,
                source,
                schema: returned.schema,
            }]
        }
        None if compiled
            .register_kinds
            .get(compiled.return_register as usize)
            .and_then(Option::as_ref)
            .is_some_and(is_compiler_pseudo_kind) =>
        {
            Vec::new()
        }
        None => {
            return Err(ArtifactBuildError::MissingRegisterSource {
                instruction: return_instruction,
                register: compiled.return_register,
                role: "return",
            });
        }
    };

    let mut constraints = Vec::with_capacity(compiled.integrity_constraints.len());
    for (constraint_index, constraint) in compiled.integrity_constraints.iter().enumerate() {
        let constraint_id = checked_u32(constraint_index, "IntegrityConstraintId")?;
        let semantic = register(&registers, constraint.result_register)?.ok_or(
            ArtifactBuildError::MissingRegisterSource {
                instruction: return_instruction,
                register: constraint.result_register,
                role: "integrity constraint",
            },
        )?;
        let schema = schemas
            .get(semantic.schema)
            .ok_or(ArtifactBuildError::UnknownSchema {
                schema: semantic.schema,
            })?;
        if !matches!(schema.body(), SchemaBody::Bool) {
            return Err(ArtifactBuildError::IntegrityConstraintSchemaMismatch {
                constraint: constraint_id,
                schema: semantic.schema,
            });
        }
        constraints.push(SourceIntegrityConstraint {
            operation: OperationReference {
                module_path: vec!["integrity".to_owned()].into_boxed_slice(),
                operation_name: "assert".to_owned(),
            },
            inputs: vec![semantic.source].into_boxed_slice(),
        });
    }

    let (inputs, mut nodes, outputs, mut constraints) =
        prune_unused_inputs(inputs, nodes, outputs, constraints)?;
    let constant_store = prune_unused_constants(
        &schemas,
        &constant_store,
        &mut states,
        &mut nodes,
        &mut constraints,
    )?;

    let source = SourceProgram {
        requirements: ApplicationRequirementTable::from_canonical_entries(
            compiled.program.requirements.clone(),
        )?,
        inputs: inputs.into_boxed_slice(),
        states: states.into_boxed_slice(),
        nodes: nodes.into_boxed_slice(),
        outputs: outputs.into_boxed_slice(),
        constraints: constraints.into_boxed_slice(),
    };
    let artifact = compile_source_program_with_contracts(
        &source,
        &mut ArtifactBuildContext::new(&schemas, &constant_store),
        &node_contracts,
    )?;
    let compute_regions = compiled
        .compute_regions
        .iter()
        .map(|region| ArtifactComputeRegion {
            name: region.name.clone(),
            placement: region.placement,
            nodes: source_node_origins
                .iter()
                .enumerate()
                .filter_map(|(node, source_node)| {
                    source_node
                        .filter(|source_node| region.source_nodes.contains(source_node))
                        .map(|_| artifact.nodes()[node].node)
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    Ok((artifact, compute_regions))
}

#[cfg(feature = "compiler")]
fn external_declaration_matches_requirement(
    declaration: &OperationContractDeclaration,
    requirement: &ApplicationRequirement,
) -> bool {
    match (requirement, &declaration.interaction) {
        (ApplicationRequirement::HostFunction(_), _) => true,
        (
            ApplicationRequirement::Resource(request),
            mech_core::ExternalInteraction::Observation(_),
        ) => {
            request.intent == mech_core::ResourceIntent::Read
                && request.delivery == mech_core::ResourceDelivery::Live
        }
        (ApplicationRequirement::Resource(request), mech_core::ExternalInteraction::Effect(_)) => {
            request.intent == mech_core::ResourceIntent::Send
                && request.delivery == mech_core::ResourceDelivery::Snapshot
        }
        (
            ApplicationRequirement::Resource(request),
            mech_core::ExternalInteraction::TransactionalExternal(_),
        ) => {
            matches!(
                request.intent,
                mech_core::ResourceIntent::Assign | mech_core::ResourceIntent::Send
            ) && request.delivery == mech_core::ResourceDelivery::Snapshot
        }
        _ => false,
    }
}

#[cfg(feature = "compiler")]
fn prune_unused_constants(
    schemas: &SchemaTable,
    constants: &ConstantStore,
    states: &mut [SourceState],
    nodes: &mut [SourceNode],
    constraints: &mut [SourceIntegrityConstraint],
) -> Result<ConstantStore, ArtifactBuildError> {
    let mut used = std::collections::BTreeSet::<ConstantId>::new();
    for state in states.iter() {
        if let Some(constant) = state.initializer {
            used.insert(constant);
        }
    }
    for node in nodes.iter() {
        for source in &node.inputs {
            if let SourceValue::Constant(constant) = source {
                used.insert(*constant);
            }
        }
    }
    for constraint in constraints.iter() {
        for source in &constraint.inputs {
            if let SourceValue::Constant(constant) = source {
                used.insert(*constant);
            }
        }
    }

    let mut builder = ConstantStoreBuilder::new(schemas);
    let mut remap = std::collections::BTreeMap::<ConstantId, ConstantHandle>::new();
    for constant in used {
        let value = constants
            .get(constant)
            .ok_or(ArtifactBuildError::UnknownConstant { constant })?;
        remap.insert(constant, builder.insert(value.clone())?);
    }
    let build = builder.finish()?;
    let remap = remap
        .into_iter()
        .map(|(old, handle)| Ok((old, build.resolve(handle)?)))
        .collect::<Result<std::collections::BTreeMap<_, _>, ArtifactBuildError>>()?;
    let (constants, _) = build.into_parts();
    let remap_source = |source: &mut SourceValue| {
        if let SourceValue::Constant(constant) = source {
            *constant = remap[constant];
        }
    };
    for state in states {
        if let Some(constant) = &mut state.initializer {
            *constant = remap[constant];
        }
    }
    for node in nodes {
        for source in &mut node.inputs {
            remap_source(source);
        }
    }
    for constraint in constraints {
        for source in &mut constraint.inputs {
            remap_source(source);
        }
    }
    Ok(constants)
}

#[cfg(feature = "compiler")]
fn prune_unused_inputs(
    inputs: Vec<SourceInput>,
    mut nodes: Vec<SourceNode>,
    mut outputs: Vec<SourceOutput>,
    mut constraints: Vec<SourceIntegrityConstraint>,
) -> Result<
    (
        Vec<SourceInput>,
        Vec<SourceNode>,
        Vec<SourceOutput>,
        Vec<SourceIntegrityConstraint>,
    ),
    ArtifactBuildError,
> {
    let mut used = vec![false; inputs.len()];
    let mut note = |source: SourceValue| {
        if let SourceValue::Input(input) = source {
            if let Some(used) = used.get_mut(input as usize) {
                *used = true;
            }
        }
    };
    for node in &nodes {
        for source in &node.inputs {
            note(*source);
        }
    }
    for output in &outputs {
        if let SourceSlot::Input(input) = output.source {
            note(SourceValue::Input(input));
        }
    }
    for constraint in &constraints {
        for source in &constraint.inputs {
            note(*source);
        }
    }
    let mut remap = vec![None; inputs.len()];
    let mut retained = Vec::new();
    for (old, input) in inputs.into_iter().enumerate() {
        if used[old] {
            let next = checked_u32(retained.len(), "InputId")?;
            remap[old] = Some(next);
            retained.push(input);
        }
    }
    let remap_value = |source: &mut SourceValue| {
        if let SourceValue::Input(input) = source {
            *input = remap[*input as usize].expect("used inputs have a remapped identity");
        }
    };
    for node in &mut nodes {
        for source in &mut node.inputs {
            remap_value(source);
        }
    }
    for output in &mut outputs {
        if let SourceSlot::Input(input) = &mut output.source {
            *input = remap[*input as usize].expect("used inputs have a remapped identity");
        }
    }
    for constraint in &mut constraints {
        for source in &mut constraint.inputs {
            remap_value(source);
        }
    }
    Ok((retained, nodes, outputs, constraints))
}

#[cfg(feature = "compiler")]
fn normalize_legacy_binding_kind(kind: ValueKind) -> (ValueKind, bool) {
    match kind {
        ValueKind::Reference(referenced) => {
            let (kind, _) = normalize_legacy_binding_kind(*referenced);
            (kind, true)
        }
        ValueKind::Matrix(element, dimensions) => {
            let (element, contains_reference) = normalize_legacy_binding_kind(*element);
            (
                ValueKind::Matrix(Box::new(element), dimensions),
                contains_reference,
            )
        }
        ValueKind::Record(fields) => {
            let mut contains_reference = false;
            let fields = fields
                .into_iter()
                .map(|(name, kind)| {
                    let (kind, nested_reference) = normalize_legacy_binding_kind(kind);
                    contains_reference |= nested_reference;
                    (name, kind)
                })
                .collect();
            (ValueKind::Record(fields), contains_reference)
        }
        ValueKind::Map(key, value) => {
            let (key, key_reference) = normalize_legacy_binding_kind(*key);
            let (value, value_reference) = normalize_legacy_binding_kind(*value);
            (
                ValueKind::Map(Box::new(key), Box::new(value)),
                key_reference || value_reference,
            )
        }
        ValueKind::Table(columns, rows) => {
            let mut contains_reference = false;
            let columns = columns
                .into_iter()
                .map(|(name, kind)| {
                    let (kind, nested_reference) = normalize_legacy_binding_kind(kind);
                    contains_reference |= nested_reference;
                    (name, kind)
                })
                .collect();
            (ValueKind::Table(columns, rows), contains_reference)
        }
        ValueKind::Tuple(elements) => {
            let mut contains_reference = false;
            let elements = elements
                .into_iter()
                .map(|kind| {
                    let (kind, nested_reference) = normalize_legacy_binding_kind(kind);
                    contains_reference |= nested_reference;
                    kind
                })
                .collect();
            (ValueKind::Tuple(elements), contains_reference)
        }
        ValueKind::Set(element, maximum_len) => {
            let (element, contains_reference) = normalize_legacy_binding_kind(*element);
            (
                ValueKind::Set(Box::new(element), maximum_len),
                contains_reference,
            )
        }
        ValueKind::Option(element) => {
            let (element, contains_reference) = normalize_legacy_binding_kind(*element);
            (ValueKind::Option(Box::new(element)), contains_reference)
        }
        ValueKind::Kind(reified) => {
            let (reified, contains_reference) = normalize_legacy_binding_kind(*reified);
            (ValueKind::Kind(Box::new(reified)), contains_reference)
        }
        kind => (kind, false),
    }
}

#[cfg(feature = "compiler")]
fn is_compiler_pseudo_kind(kind: &ValueKind) -> bool {
    match kind {
        ValueKind::Empty | ValueKind::Any | ValueKind::None => true,
        ValueKind::Reference(referenced) => is_compiler_pseudo_kind(referenced),
        _ => false,
    }
}

#[cfg(feature = "compiler")]
fn register(
    registers: &[Option<RegisterSemantic>],
    register: u32,
) -> Result<Option<RegisterSemantic>, ArtifactBuildError> {
    registers.get(register as usize).copied().ok_or(
        ArtifactBuildError::SourceGraphReferenceOutOfRange {
            reference: "bytecode register",
            index: register,
        },
    )
}

#[cfg(feature = "compiler")]
fn set_register(
    registers: &mut [Option<RegisterSemantic>],
    register: u32,
    value: Option<RegisterSemantic>,
) -> Result<(), ArtifactBuildError> {
    let target = registers.get_mut(register as usize).ok_or(
        ArtifactBuildError::SourceGraphReferenceOutOfRange {
            reference: "bytecode register",
            index: register,
        },
    )?;
    *target = value;
    Ok(())
}

#[cfg(feature = "compiler")]
fn validate_compiled_metadata_length(
    table: &'static str,
    expected: usize,
    actual: usize,
) -> Result<(), ArtifactBuildError> {
    if expected != actual {
        return Err(ArtifactBuildError::CompiledMetadataLengthMismatch {
            table,
            expected,
            actual,
        });
    }
    Ok(())
}

#[cfg(feature = "compiler")]
fn validate_compiled_instruction_roles(
    compiled: &CompiledBytecode,
    catalog: &FunctionCatalog,
) -> Result<(), ArtifactBuildError> {
    let returns = compiled
        .program
        .instructions
        .iter()
        .enumerate()
        .filter_map(|(index, instruction)| match instruction {
            BytecodeInstruction::Return { src } => Some((index, *src)),
            _ => None,
        })
        .collect::<Vec<_>>();
    if returns.len() != 1 {
        return Err(ArtifactBuildError::CompiledReturnCount {
            found: returns.len(),
        });
    }
    let (return_index, return_register) = returns[0];
    if return_index + 1 != compiled.program.instructions.len() {
        return Err(ArtifactBuildError::NonTerminalCompiledReturn {
            instruction: checked_u32(return_index, "instruction")?,
        });
    }
    if return_register != compiled.return_register {
        return Err(ArtifactBuildError::CompiledReturnRegisterMismatch {
            instruction: checked_u32(return_index, "instruction")?,
            expected: compiled.return_register,
            found: return_register,
        });
    }

    let marker_registers = compiled
        .program
        .instructions
        .iter()
        .zip(&compiled.instruction_roles)
        .filter_map(|(instruction, role)| match (instruction, role) {
            (
                BytecodeInstruction::RuntimeVariadic { arguments, .. },
                Some(CompiledInstructionRole::IntegrityMarker),
            ) => Some(arguments.first().copied()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let metadata_len = marker_registers
        .len()
        .max(compiled.integrity_constraints.len());
    for index in 0..metadata_len {
        let marker_register = marker_registers.get(index).copied().flatten();
        let declared_register = compiled
            .integrity_constraints
            .get(index)
            .map(|constraint| constraint.result_register);
        if marker_registers.get(index).is_none()
            || compiled.integrity_constraints.get(index).is_none()
            || marker_register != declared_register
        {
            return Err(ArtifactBuildError::IntegrityConstraintMetadataMismatch {
                constraint: checked_u32(index, "IntegrityConstraintId")?,
                marker_register,
                declared_register,
            });
        }
    }

    for (index, (instruction, role)) in compiled
        .program
        .instructions
        .iter()
        .zip(&compiled.instruction_roles)
        .enumerate()
    {
        let instruction_id = checked_u32(index, "instruction")?;
        match instruction {
            BytecodeInstruction::ConstLoad { .. } | BytecodeInstruction::Return { .. } => {
                if let Some(role) = role {
                    return Err(ArtifactBuildError::UnexpectedInstructionRole {
                        instruction: instruction_id,
                        role: instruction_role_name(*role),
                    });
                }
            }
            BytecodeInstruction::RuntimeVariadic { .. }
                if *role == Some(CompiledInstructionRole::IntegrityMarker) => {}
            BytecodeInstruction::RuntimeBinary { .. }
                if *role == Some(CompiledInstructionRole::DeclarationMarker)
                    && is_variable_definition_instruction(instruction, catalog) => {}
            _ => match role {
                Some(CompiledInstructionRole::Node(_)) => {
                    instruction_semantics(instruction, catalog, &compiled.program.requirements)?;
                }
                Some(CompiledInstructionRole::IntegrityMarker) => {
                    return Err(ArtifactBuildError::UnexpectedInstructionRole {
                        instruction: instruction_id,
                        role: "integrity marker",
                    });
                }
                Some(CompiledInstructionRole::DeclarationMarker) => {
                    return Err(ArtifactBuildError::InvalidDeclarationMarker {
                        instruction: instruction_id,
                    });
                }
                None => {
                    return Err(ArtifactBuildError::MissingInstructionRole {
                        instruction: instruction_id,
                    });
                }
            },
        }
    }
    Ok(())
}

#[cfg(feature = "compiler")]
fn instruction_role_name(role: CompiledInstructionRole) -> &'static str {
    match role {
        CompiledInstructionRole::Node(_) => "node",
        CompiledInstructionRole::IntegrityMarker => "integrity marker",
        CompiledInstructionRole::DeclarationMarker => "declaration marker",
    }
}

#[cfg(feature = "compiler")]
struct CompiledInstructionSemantics {
    destination: u32,
    inputs: Vec<u32>,
    operation: OperationReference,
    requirement: Option<ApplicationRequirementId>,
}

/// Some executable instructions use their destination as the logical base of
/// a read/modify/write operation without repeating it in the operand list.
/// The semantic artifact exposes that dependency without changing bytecode.
#[cfg(feature = "compiler")]
fn semantic_input_registers(
    semantics: &CompiledInstructionSemantics,
    declaration: Option<&OperationContractDeclaration>,
) -> Result<Vec<u32>, ArtifactBuildError> {
    let mut inputs = semantics.inputs.clone();
    let Some(declaration) = declaration else {
        return Ok(inputs);
    };
    let InputPortLayout::Fixed(policies) = &declaration.inputs else {
        return Ok(inputs);
    };
    if policies.len() != inputs.len() + 1 {
        return Ok(inputs);
    }
    let base_input = declaration.outputs.iter().find_map(|output| {
        if let OutputConstruction::ReadModifyWrite { base_input, .. } = output.construction {
            Some(base_input)
        } else {
            None
        }
    });
    let Some(base_input) = base_input else {
        return Ok(inputs);
    };
    if base_input as usize > inputs.len() {
        return Err(OperationContractError::InputOrdinalOutOfRange {
            field: "ReadModifyWrite.base_input",
            input: base_input,
            inputs: policies.len() as u32,
        }
        .into());
    }
    inputs.insert(base_input as usize, semantics.destination);
    Ok(inputs)
}

#[cfg(feature = "compiler")]
fn instruction_destination(instruction: &BytecodeInstruction) -> Option<u32> {
    match instruction {
        BytecodeInstruction::ConstLoad { dst, .. }
        | BytecodeInstruction::CompositePack { dst, .. }
        | BytecodeInstruction::RuntimeNullary { dst, .. }
        | BytecodeInstruction::RuntimeUnary { dst, .. }
        | BytecodeInstruction::RuntimeBinary { dst, .. }
        | BytecodeInstruction::RuntimeTernary { dst, .. }
        | BytecodeInstruction::RuntimeQuaternary { dst, .. }
        | BytecodeInstruction::RuntimeVariadic { dst, .. }
        | BytecodeInstruction::HostCall { dst, .. }
        | BytecodeInstruction::ResourceRead { dst, .. }
        | BytecodeInstruction::ResourceWrite { dst, .. }
        | BytecodeInstruction::ResourceSend { dst, .. } => Some(*dst),
        BytecodeInstruction::Return { .. } => None,
    }
}

#[cfg(feature = "compiler")]
fn is_variable_definition_instruction(
    instruction: &BytecodeInstruction,
    catalog: &FunctionCatalog,
) -> bool {
    instruction
        .runtime_function()
        .and_then(|function| catalog.runtime_entry_by_raw(function))
        .is_some_and(|entry| entry.name.starts_with("VariableDefine"))
}

#[cfg(feature = "compiler")]
fn operation_reference_from_name(
    default_namespace: &'static str,
    canonical_name: &str,
) -> Result<OperationReference, ArtifactBuildError> {
    let invalid = || ArtifactBuildError::InvalidCompiledOperationName {
        namespace: default_namespace,
        name: canonical_name.to_owned(),
    };
    if canonical_name.is_empty()
        || canonical_name.starts_with('/')
        || canonical_name.ends_with('/')
        || canonical_name.contains('\0')
        || canonical_name.contains('\\')
    {
        return Err(invalid());
    }
    let segments = canonical_name.split('/').collect::<Vec<_>>();
    if segments
        .iter()
        .any(|segment| segment.is_empty() || *segment == "." || *segment == "..")
    {
        return Err(invalid());
    }
    let operation_name = segments.last().expect("validated nonempty operation name");
    let module_path = if segments.len() == 1 {
        vec![default_namespace.to_owned()]
    } else {
        segments[..segments.len() - 1]
            .iter()
            .map(|segment| (*segment).to_owned())
            .collect()
    };
    Ok(OperationReference {
        module_path: module_path.into_boxed_slice(),
        operation_name: (*operation_name).to_owned(),
    })
}

#[cfg(feature = "compiler")]
fn resource_operation_reference(
    requirement: u32,
    requirements: &[ApplicationRequirement],
    direction: &'static str,
) -> Result<OperationReference, ArtifactBuildError> {
    let requirement_value = requirements
        .get(requirement as usize)
        .ok_or(ArtifactBuildError::UnknownApplicationRequirement { requirement })?;
    let ApplicationRequirement::Resource(request) = requirement_value else {
        return Err(ArtifactBuildError::ApplicationRequirementKindMismatch {
            requirement,
            expected: "resource",
        });
    };
    let parsed = operation_reference_from_name("resource", &request.operation)?;
    let mut module_path = vec!["resource".to_owned(), direction.to_owned()];
    if parsed.module_path.as_ref() != ["resource"] {
        module_path.extend(parsed.module_path.iter().cloned());
    }
    Ok(OperationReference {
        module_path: module_path.into_boxed_slice(),
        operation_name: parsed.operation_name,
    })
}

#[cfg(feature = "compiler")]
fn instruction_semantics(
    instruction: &BytecodeInstruction,
    catalog: &FunctionCatalog,
    requirements: &[ApplicationRequirement],
) -> Result<Option<CompiledInstructionSemantics>, ArtifactBuildError> {
    let runtime = |function: u64| {
        let entry = catalog
            .runtime_entry_by_raw(function)
            .ok_or(ArtifactBuildError::UnknownRuntimeFunction { function })?;
        operation_reference_from_name("runtime", &entry.name)
    };
    let semantics = match instruction {
        BytecodeInstruction::ConstLoad { .. } | BytecodeInstruction::Return { .. } => {
            return Ok(None);
        }
        BytecodeInstruction::CompositePack { dst, children, .. } => CompiledInstructionSemantics {
            destination: *dst,
            inputs: children.clone(),
            operation: OperationReference {
                module_path: vec!["core".to_owned()].into_boxed_slice(),
                operation_name: "composite-pack".to_owned(),
            },
            requirement: None,
        },
        BytecodeInstruction::RuntimeNullary { function, dst } => CompiledInstructionSemantics {
            destination: *dst,
            inputs: Vec::new(),
            operation: runtime(*function)?,
            requirement: None,
        },
        BytecodeInstruction::RuntimeUnary { function, dst, src } => CompiledInstructionSemantics {
            destination: *dst,
            inputs: vec![*src],
            operation: runtime(*function)?,
            requirement: None,
        },
        BytecodeInstruction::RuntimeBinary {
            function,
            dst,
            lhs,
            rhs,
        } => CompiledInstructionSemantics {
            destination: *dst,
            inputs: vec![*lhs, *rhs],
            operation: runtime(*function)?,
            requirement: None,
        },
        BytecodeInstruction::RuntimeTernary {
            function,
            dst,
            a,
            b,
            c,
        } => CompiledInstructionSemantics {
            destination: *dst,
            inputs: vec![*a, *b, *c],
            operation: runtime(*function)?,
            requirement: None,
        },
        BytecodeInstruction::RuntimeQuaternary {
            function,
            dst,
            a,
            b,
            c,
            d,
        } => CompiledInstructionSemantics {
            destination: *dst,
            inputs: vec![*a, *b, *c, *d],
            operation: runtime(*function)?,
            requirement: None,
        },
        BytecodeInstruction::RuntimeVariadic {
            function,
            dst,
            arguments,
        } => CompiledInstructionSemantics {
            destination: *dst,
            inputs: arguments.clone(),
            operation: runtime(*function)?,
            requirement: None,
        },
        BytecodeInstruction::HostCall {
            requirement,
            dst,
            arguments,
        } => {
            let requirement_value = requirements.get(*requirement as usize).ok_or(
                ArtifactBuildError::UnknownApplicationRequirement {
                    requirement: *requirement,
                },
            )?;
            let ApplicationRequirement::HostFunction(request) = requirement_value else {
                return Err(ArtifactBuildError::ApplicationRequirementKindMismatch {
                    requirement: *requirement,
                    expected: "host function",
                });
            };
            CompiledInstructionSemantics {
                destination: *dst,
                inputs: arguments.clone(),
                operation: operation_reference_from_name("host", &request.name)?,
                requirement: Some(ApplicationRequirementId::new(*requirement)),
            }
        }
        BytecodeInstruction::ResourceRead { requirement, dst } => CompiledInstructionSemantics {
            destination: *dst,
            inputs: Vec::new(),
            operation: resource_operation_reference(*requirement, requirements, "read")?,
            requirement: Some(ApplicationRequirementId::new(*requirement)),
        },
        BytecodeInstruction::ResourceWrite {
            requirement,
            dst,
            src,
        } => CompiledInstructionSemantics {
            destination: *dst,
            inputs: vec![*src],
            operation: resource_operation_reference(*requirement, requirements, "write")?,
            requirement: Some(ApplicationRequirementId::new(*requirement)),
        },
        BytecodeInstruction::ResourceSend {
            requirement,
            dst,
            src,
        } => CompiledInstructionSemantics {
            destination: *dst,
            inputs: vec![*src],
            operation: resource_operation_reference(*requirement, requirements, "send")?,
            requirement: Some(ApplicationRequirementId::new(*requirement)),
        },
    };
    Ok(Some(semantics))
}

fn resolve_source(
    source: SourceValue,
    inputs: &[CellSlotId],
    states: &[CellSlotId],
    outputs: &BTreeMap<(u32, u16), CellSlotId>,
) -> Result<ArtifactSource, ArtifactBuildError> {
    Ok(match source {
        SourceValue::Constant(constant) => ArtifactSource::Constant(constant),
        SourceValue::Input(index) => ArtifactSource::Slot(*inputs.get(index as usize).ok_or(
            ArtifactBuildError::SourceGraphReferenceOutOfRange {
                reference: "input",
                index,
            },
        )?),
        SourceValue::State(index) => ArtifactSource::Slot(*states.get(index as usize).ok_or(
            ArtifactBuildError::SourceGraphReferenceOutOfRange {
                reference: "state",
                index,
            },
        )?),
        SourceValue::NodeOutput {
            node,
            output_ordinal,
        } => ArtifactSource::Slot(*outputs.get(&(node, output_ordinal)).ok_or(
            ArtifactBuildError::SourceGraphReferenceOutOfRange {
                reference: "node output",
                index: node,
            },
        )?),
    })
}

fn resolve_slot_source(
    source: SourceSlot,
    inputs: &[CellSlotId],
    states: &[CellSlotId],
    outputs: &BTreeMap<(u32, u16), CellSlotId>,
) -> Result<CellSlotId, ArtifactBuildError> {
    match source {
        SourceSlot::Input(index) => inputs.get(index as usize).copied().ok_or(
            ArtifactBuildError::SourceGraphReferenceOutOfRange {
                reference: "input",
                index,
            },
        ),
        SourceSlot::State(index) => states.get(index as usize).copied().ok_or(
            ArtifactBuildError::SourceGraphReferenceOutOfRange {
                reference: "state",
                index,
            },
        ),
        SourceSlot::NodeOutput {
            node,
            output_ordinal,
        } => outputs.get(&(node, output_ordinal)).copied().ok_or(
            ArtifactBuildError::SourceGraphReferenceOutOfRange {
                reference: "node output",
                index: node,
            },
        ),
    }
}

fn checked_u32(value: usize, identity: &'static str) -> Result<u32, ArtifactBuildError> {
    u32::try_from(value).map_err(|_| ArtifactBuildError::ArtifactIdentityExhausted { identity })
}

fn checked_u16(value: usize, identity: &'static str) -> Result<u16, ArtifactBuildError> {
    u16::try_from(value).map_err(|_| ArtifactBuildError::ArtifactIdentityExhausted { identity })
}
