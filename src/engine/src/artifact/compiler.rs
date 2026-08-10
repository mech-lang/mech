//! Deterministic source-compiler graph lowering into `ProgramArtifact`.
//!
//! This is the semantic compiler product. Bytecode v1 encodes this artifact
//! and its decoder reconstructs and validates the same artifact; it does not
//! feed a separate compatibility graph back into this compiler.

use std::collections::BTreeMap;

use mech_core::{
    BindingId, CellSlotId, ConstantId, ConstantStore, InputId, IntegrityConstraintId, NodeId,
    OperationContractId, OperationContractTable, OutputId, SchemaId, SchemaTable,
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
    ConstantStoreBuilder, DimensionEnvironmentBuilder, DimensionExpr, FunctionCatalog, KindId,
    LegacyEmptyPolicy, LegacyExtentRole, LegacyExtentSite, LegacyNominalResolution,
    LegacyReferencePolicy, LegacyResolvedExtent, LegacySemanticContext, LegacySnapshotContext,
    LegacyValue, NamedKindPathResolver, NominalKind, SchemaBody, SchemaHandle, SchemaTableBuilder,
    SemanticModelError, ValueKind, schema_from_legacy_value_kind, snapshot_from_legacy,
    write_bytecode,
};

use super::{
    ArtifactBuildError, ArtifactSource, BindingDeclaration, InitializerReference, InputDeclaration,
    IntegrityConstraintDeclaration, NodeDeclaration, OperationReference, OutputDeclaration,
    ProducerReference, ProgramArtifact, ProgramArtifactDraft, SlotDeclaration, SlotRole,
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

    ProgramArtifactDraft {
        schemas: context.schemas.clone(),
        constants: context.constants.clone(),
        contracts: OperationContractTable::empty(),
        inputs: inputs.into_boxed_slice(),
        slots: slots.into_boxed_slice(),
        nodes: nodes.into_boxed_slice(),
        bindings: bindings.into_boxed_slice(),
        outputs: outputs.into_boxed_slice(),
        constraints: constraints.into_boxed_slice(),
    }
    .attach_legacy_contracts()?
    .finalize()
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
    validate_compiled_metadata_length(
        "instruction_roles",
        compiled.program.instructions.len(),
        compiled.instruction_roles.len(),
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
    validate_compiled_instruction_roles(compiled, catalog)?;

    let legacy_bytes = write_bytecode(&compiled.program)?;
    let legacy_constants =
        mech_core::ParsedProgram::from_bytes(&legacy_bytes)?.decode_constants()?;

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

    let mut registers = vec![None::<RegisterSemantic>; compiled.program.register_count as usize];
    let mut state_initializers = vec![None::<ConstantId>; compiled.program.register_count as usize];
    let mut states = Vec::<SourceState>::new();
    let mut nodes = Vec::<SourceNode>::new();

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
                    Some(
                        CompilerConstantRole::Snapshot | CompilerConstantRole::StateInitializer,
                    ) => schema.and_then(|schema| {
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
                if register_constant_roles
                    .get(*dst as usize)
                    .copied()
                    .flatten()
                    == Some(CompilerConstantRole::StateInitializer)
                {
                    state_initializers[*dst as usize] =
                        semantic.and_then(|value| match value.source {
                            SourceValue::Constant(constant) => Some(constant),
                            _ => None,
                        });
                }
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
                if let BytecodeInstruction::CompositePack { template, .. } = instruction {
                    if register_constant_roles.get(dst as usize).copied().flatten()
                        == Some(CompilerConstantRole::StateInitializer)
                    {
                        let schema = schema.expect("non-pseudo composite destination has schema");
                        let initializer = constants.get(&(*template, schema)).copied().ok_or(
                            ArtifactBuildError::SourceGraphReferenceOutOfRange {
                                reference: "state initializer constant",
                                index: *template,
                            },
                        )?;
                        state_initializers[dst as usize] = Some(initializer);
                    }
                }
                let node_index = checked_u32(nodes.len(), "NodeId")?;
                let state_index = if kind == CompiledNodeKind::Register {
                    let schema = schema.ok_or(ArtifactBuildError::MissingRegisterKind {
                        instruction: instruction_id,
                        register: dst,
                    })?;
                    let state = checked_u32(states.len(), "state")?;
                    states.push(SourceState {
                        schema,
                        initializer: state_initializers[dst as usize].or_else(|| {
                            prior.and_then(|value| match value.source {
                                SourceValue::Constant(constant) => Some(constant),
                                _ => None,
                            })
                        }),
                        producer_node: node_index,
                        producer_output_ordinal: 0,
                    });
                    Some(state)
                } else {
                    None
                };
                let semantic_inputs = semantics
                    .inputs
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
                nodes.push(SourceNode {
                    operation: semantics.operation,
                    inputs: semantic_inputs,
                    outputs: match (state_index, schema) {
                        (Some(state), _) => vec![SourceNodeOutput::State(state)],
                        (None, Some(schema)) => vec![SourceNodeOutput::Derived { schema }],
                        (None, None) => Vec::new(),
                    }
                    .into_boxed_slice(),
                });
                if schema.is_none() {
                    set_register(&mut registers, dst, None)?;
                    continue;
                }
                let preserves_destination =
                    is_variable_definition_instruction(instruction, catalog);
                let source = match state_index {
                    Some(state) => SourceValue::State(state),
                    None if preserves_destination => {
                        prior
                            .ok_or(ArtifactBuildError::MissingRegisterSource {
                                instruction: instruction_id,
                                register: dst,
                                role: "preserved destination",
                            })?
                            .source
                    }
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
                        inputs: vec![SourceValue::Constant(constant)].into_boxed_slice(),
                        outputs: vec![SourceNodeOutput::Derived {
                            schema: returned.schema,
                        }]
                        .into_boxed_slice(),
                    });
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

    compile_source_program(
        &SourceProgram {
            inputs: inputs.into_boxed_slice(),
            states: states.into_boxed_slice(),
            nodes: nodes.into_boxed_slice(),
            outputs: outputs.into_boxed_slice(),
            constraints: constraints.into_boxed_slice(),
        },
        &mut ArtifactBuildContext::new(&schemas, &constant_store),
    )
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
    }
}

#[cfg(feature = "compiler")]
struct CompiledInstructionSemantics {
    destination: u32,
    inputs: Vec<u32>,
    operation: OperationReference,
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
        },
        BytecodeInstruction::RuntimeNullary { function, dst } => CompiledInstructionSemantics {
            destination: *dst,
            inputs: Vec::new(),
            operation: runtime(*function)?,
        },
        BytecodeInstruction::RuntimeUnary { function, dst, src } => CompiledInstructionSemantics {
            destination: *dst,
            inputs: vec![*src],
            operation: runtime(*function)?,
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
        },
        BytecodeInstruction::RuntimeVariadic {
            function,
            dst,
            arguments,
        } => CompiledInstructionSemantics {
            destination: *dst,
            inputs: arguments.clone(),
            operation: runtime(*function)?,
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
            }
        }
        BytecodeInstruction::ResourceRead { requirement, dst } => CompiledInstructionSemantics {
            destination: *dst,
            inputs: Vec::new(),
            operation: resource_operation_reference(*requirement, requirements, "read")?,
        },
        BytecodeInstruction::ResourceWrite {
            requirement,
            dst,
            src,
        } => CompiledInstructionSemantics {
            destination: *dst,
            inputs: vec![*src],
            operation: resource_operation_reference(*requirement, requirements, "write")?,
        },
        BytecodeInstruction::ResourceSend {
            requirement,
            dst,
            src,
        } => CompiledInstructionSemantics {
            destination: *dst,
            inputs: vec![*src],
            operation: resource_operation_reference(*requirement, requirements, "send")?,
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
