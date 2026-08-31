//! Deterministic source-compiler graph lowering into `ProgramArtifact`.
//!
//! This is the semantic compiler product. Bytecode v1 encodes this artifact
//! and its decoder reconstructs and validates the same artifact; it does not
//! feed a separate compatibility graph back into this compiler.

use std::collections::BTreeMap;
#[cfg(feature = "semantic-compiler")]
use std::collections::BTreeSet;
#[cfg(feature = "semantic-compiler")]
use std::sync::LazyLock;

#[cfg(feature = "semantic-compiler")]
use mech_core::MResult;
use mech_core::{
    ApplicationRequirementId, BindingId, CellSlotId, ConstantId, ConstantStore, InputId,
    IntegrityConstraintId, NodeId, OperationContractDeclaration, OperationContractId,
    OperationContractTable, OutputId, SchemaId, SchemaTable,
};

#[cfg(feature = "semantic-compiler")]
use crate::{
    CompiledBytecode, CompiledInstructionRole, CompiledMatrixLiteralElement, CompiledNodeKind,
    CompiledSymbolDefinition,
};
#[cfg(feature = "semantic-compiler")]
use mech_core::{
    AccessMode, AliasPolicy, ApplicationRequirement, BytecodeInstruction, ChangeDetectionPolicy,
    ConstantHandle, ConstantStoreBuilder, DeliveryMode, DimensionExpr, ExternalInteraction,
    FunctionCatalog, InputPortLayout, InputPortPolicy, OperationContractError, OutputConstruction,
    OutputPortPolicy, Register, RuntimeType, SchemaBody, SchemaDraft, SchemaHandle,
    SchemaTableBuilder, ShapeRule, Value,
};

#[cfg(feature = "semantic-compiler")]
use super::ComputeRegionDeclaration;
use super::{
    ApplicationRequirementTable, ArtifactBuildError, ArtifactSource, BindingDeclaration,
    CompilerIrError, ExpressionIR, InitializerReference, InputDeclaration,
    IntegrityConstraintDeclaration, InteractiveSymbolBinding, MatrixLiteralIR, NodeDeclaration,
    OperationReference, OutputDeclaration, ProducerReference, ProgramArtifact,
    ProgramArtifactDraft, SlotDeclaration, SlotRole,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceValue {
    Constant(ConstantId),
    Input(u32),
    State(u32),
    NodeOutput { node: u32, output_ordinal: u16 },
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SourceMatrixLiteral {
    rows: u64,
    columns: u64,
    // `None` is the compiler-only empty expression; `Some` retains a source
    // identity until artifact slots have been allocated.
    elements: Box<[Option<SourceValue>]>,
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

#[cfg(feature = "semantic-compiler")]
static COMPILER_STATE_HOLD_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![InputPortPolicy {
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            }]
            .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::SameAsInput { input: 0 },
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    });

#[cfg(feature = "semantic-compiler")]
fn matrix_literal_contract(element_count: usize) -> OperationContractDeclaration {
    OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            (0..element_count)
                .map(|_| InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::AlwaysChanged,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceOutput {
    pub name: String,
    pub interactive_symbol: Option<String>,
    pub source: SourceValue,
    pub schema: SchemaId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceIntegrityConstraint {
    pub name: String,
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
#[cfg(feature = "semantic-compiler")]
pub trait ExternalRequirementContractResolver {
    fn resolve_external_contract(
        &self,
        requirement: &ApplicationRequirement,
    ) -> MResult<Option<&'static OperationContractDeclaration>>;
}

#[cfg(feature = "semantic-compiler")]
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

/// Compiles a semantic source graph with declarations already selected by the
/// specializing compiler. The declaration slice is parallel to `graph.nodes`.
pub fn compile_source_program_with_contracts(
    graph: &SourceProgram,
    context: &mut ArtifactBuildContext<'_>,
    node_contracts: &[&OperationContractDeclaration],
) -> Result<ProgramArtifact, ArtifactBuildError> {
    compile_source_program_with_metadata(graph, context, node_contracts, &[])
}

fn compile_source_program_with_metadata(
    graph: &SourceProgram,
    context: &mut ArtifactBuildContext<'_>,
    node_contracts: &[&OperationContractDeclaration],
    node_matrix_literals: &[Option<SourceMatrixLiteral>],
) -> Result<ProgramArtifact, ArtifactBuildError> {
    if !node_matrix_literals.is_empty() && node_matrix_literals.len() != graph.nodes.len() {
        return Err(ArtifactBuildError::CompiledMetadataLengthMismatch {
            table: "node_matrix_literals",
            expected: graph.nodes.len(),
            actual: node_matrix_literals.len(),
        });
    }
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
        let matrix_ir = node_matrix_literals
            .get(node_index)
            .and_then(Option::as_ref)
            .map(|literal| {
                resolve_source_matrix_literal(literal, &input_slots, &state_slots, &output_slots)
            })
            .transpose()?;
        let resolved_inputs = if let Some(matrix_ir) = &matrix_ir {
            if matrix_ir.elements.len() != declaration.inputs.len() {
                return Err(ArtifactBuildError::CompiledMetadataLengthMismatch {
                    table: "matrix_literal_elements",
                    expected: declaration.inputs.len(),
                    actual: matrix_ir.elements.len(),
                });
            }
            matrix_ir
                .elements
                .iter()
                .enumerate()
                .map(|(index, expression)| match expression {
                    ExpressionIR::Constant(constant) => Ok(ArtifactSource::Constant(*constant)),
                    ExpressionIR::Slot(slot) => Ok(ArtifactSource::Slot(*slot)),
                    ExpressionIR::Empty => {
                        Err(CompilerIrError::DynamicEmptyMatrixLiteralUnsupported { index }.into())
                    }
                    ExpressionIR::MatrixLiteral(_) | ExpressionIR::Selection(_) => {
                        Err(CompilerIrError::UnresolvedMatrixLiteralElement { index }.into())
                    }
                })
                .collect::<Result<Vec<_>, ArtifactBuildError>>()?
        } else {
            declaration
                .inputs
                .iter()
                .map(|source| resolve_source(*source, &input_slots, &state_slots, &output_slots))
                .collect::<Result<Vec<_>, ArtifactBuildError>>()?
        };
        for (ordinal, source) in resolved_inputs.into_iter().enumerate() {
            let id = BindingId(checked_u32(bindings.len(), "BindingId")?);
            bindings.push(BindingDeclaration::Input {
                id,
                node,
                port_ordinal: checked_u16(ordinal, "input port ordinal")?,
                source,
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

    // Output aliases are identified by their semantic source, not by the
    // source's storage class. Constants, transient slots, and input slots all
    // publish through one materialized output slot; state/output slots are
    // already persistent and can be addressed directly.
    let mut published_sources = BTreeMap::<ArtifactSource, CellSlotId>::new();
    let outputs = graph
        .outputs
        .iter()
        .enumerate()
        .map(|(index, output)| {
            let output_id = OutputId(checked_u32(index, "OutputId")?);
            let artifact_source =
                resolve_source(output.source, &input_slots, &state_slots, &output_slots)?;
            let persistent = match artifact_source {
                ArtifactSource::Slot(source) => matches!(
                    slots[source.get() as usize].role,
                    SlotRole::State | SlotRole::Output
                )
                .then_some(source),
                ArtifactSource::Constant(_) => None,
            };
            let source = if let Some(source) = persistent {
                source
            } else if let Some(target) = published_sources.get(&artifact_source).copied() {
                target
            } else {
                let target = CellSlotId(next_slot);
                next_slot = next_slot.checked_add(1).ok_or(
                    ArtifactBuildError::ArtifactIdentityExhausted {
                        identity: "CellSlotId",
                    },
                )?;
                slots.push(SlotDeclaration {
                    slot: target,
                    schema: output.schema,
                    role: SlotRole::Output,
                    producer: ProducerReference::Output {
                        output: output_id,
                        source: artifact_source,
                    },
                    initializer: published_output_initializer(graph, output.source)
                        .map(InitializerReference::Constant),
                });
                published_sources.insert(artifact_source, target);
                target
            };
            Ok(OutputDeclaration {
                output: output_id,
                name: output.name.clone(),
                interactive_binding: output.interactive_symbol.clone().map(|lexical_name| {
                    InteractiveSymbolBinding {
                        lexical_name,
                        artifact_source,
                        storage: source,
                        output: output_id,
                    }
                }),
                source,
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
                name: constraint.name.clone(),
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
        compute_regions: Box::new([]),
    };
    draft.attach_contracts(node_contracts)?.finalize()
}

#[cfg(feature = "semantic-compiler")]
#[derive(Clone, Copy)]
struct RegisterSemantic {
    source: SourceValue,
    schema: SchemaId,
}

#[cfg(feature = "semantic-compiler")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RegisterMatrixLiteralElement {
    Empty,
    Constant(ConstantId),
    Register(Register),
}

#[cfg(feature = "semantic-compiler")]
#[derive(Clone, Debug, Eq, PartialEq)]
struct RegisterMatrixLiteral {
    rows: u64,
    columns: u64,
    elements: Box<[RegisterMatrixLiteralElement]>,
}

#[cfg(feature = "semantic-compiler")]
impl RegisterMatrixLiteral {
    fn constant_ir(&self) -> Option<MatrixLiteralIR> {
        let elements = self
            .elements
            .iter()
            .map(|element| match element {
                RegisterMatrixLiteralElement::Empty => Some(ExpressionIR::Empty),
                RegisterMatrixLiteralElement::Constant(constant) => {
                    Some(ExpressionIR::Constant(*constant))
                }
                RegisterMatrixLiteralElement::Register(_) => None,
            })
            .collect::<Option<Vec<_>>>()?;
        Some(MatrixLiteralIR {
            rows: self.rows,
            columns: self.columns,
            elements: elements.into_boxed_slice(),
        })
    }

    fn contains_empty(&self) -> bool {
        self.elements
            .iter()
            .any(|element| matches!(element, RegisterMatrixLiteralElement::Empty))
    }
}

#[cfg(feature = "semantic-compiler")]
fn source_matrix_literal_from_registers(
    literal: &RegisterMatrixLiteral,
    registers: &[Option<RegisterSemantic>],
    instruction: u32,
) -> Result<SourceMatrixLiteral, ArtifactBuildError> {
    let elements = literal
        .elements
        .iter()
        .map(|element| match element {
            RegisterMatrixLiteralElement::Empty => Ok(None),
            RegisterMatrixLiteralElement::Constant(constant) => {
                Ok(Some(SourceValue::Constant(*constant)))
            }
            RegisterMatrixLiteralElement::Register(input_register) => {
                register(registers, *input_register)?
                    .map(|semantic| Some(semantic.source))
                    .ok_or(ArtifactBuildError::MissingRegisterSource {
                        instruction,
                        register: *input_register,
                        role: "matrix literal input",
                    })
            }
        })
        .collect::<Result<Vec<_>, ArtifactBuildError>>()?;
    Ok(SourceMatrixLiteral {
        rows: literal.rows,
        columns: literal.columns,
        elements: elements.into_boxed_slice(),
    })
}

/// Adapts the actual executable compiler product into C3's durable semantic
/// graph. Execution still consumes the existing bytecode/plan; this product is
/// emitted alongside it for bytecode-v1 persistence and later activation.
#[cfg(feature = "semantic-compiler")]
pub fn compile_executable_program_artifact(
    compiled: &CompiledBytecode,
    catalog: &FunctionCatalog,
) -> Result<ProgramArtifact, ArtifactBuildError> {
    compile_executable_program_artifact_with_outputs_and_external_inputs(
        compiled,
        &[],
        catalog,
        &BTreeSet::new(),
    )
}

/// Compiles a source product while publishing every explicitly requested root
/// result. The executable bytecode stream has one return instruction, while
/// bytecode v1's authoritative artifact sections retain all root outputs.
#[cfg(feature = "semantic-compiler")]
pub fn compile_executable_program_artifact_with_outputs(
    compiled: &CompiledBytecode,
    published_outputs: &[Register],
    catalog: &FunctionCatalog,
) -> Result<ProgramArtifact, ArtifactBuildError> {
    compile_executable_program_artifact_with_outputs_and_external_inputs(
        compiled,
        published_outputs,
        catalog,
        &BTreeSet::new(),
    )
}

/// Compiles an executable source product while treating the named immutable
/// declarations as activation inputs. This is used only when an enclosing
/// compute boundary explicitly supplies those values; ordinary source
/// literals remain artifact constants.
#[cfg(feature = "semantic-compiler")]
pub fn compile_executable_program_artifact_with_outputs_and_external_inputs(
    compiled: &CompiledBytecode,
    published_outputs: &[Register],
    catalog: &FunctionCatalog,
    external_input_names: &BTreeSet<String>,
) -> Result<ProgramArtifact, ArtifactBuildError> {
    compile_executable_program_artifact_from_semantics(
        compiled,
        published_outputs,
        &[],
        catalog,
        external_input_names,
    )
}

/// Namespace used to carry an interactive lexical symbol through the durable
/// artifact interface without weakening canonical interface-name validation.
pub const INTERACTIVE_SYMBOL_OUTPUT_PREFIX: &str = "mech-repl-symbol-";

/// Encode an arbitrary UTF-8 lexical symbol as one canonical artifact output
/// name. Encoding every interactive name (rather than only invalid names)
/// makes the mapping injective and keeps query identity independent from the
/// artifact interface grammar.
pub fn encode_interactive_symbol_output_name(name: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut encoded =
        String::with_capacity(INTERACTIVE_SYMBOL_OUTPUT_PREFIX.len() + name.len() * 2);
    encoded.push_str(INTERACTIVE_SYMBOL_OUTPUT_PREFIX);
    for byte in name.bytes() {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

/// Decode an interactive artifact output name back to its lexical query name.
pub fn decode_interactive_symbol_output_name(name: &str) -> Option<String> {
    let encoded = name.strip_prefix(INTERACTIVE_SYMBOL_OUTPUT_PREFIX)?;
    if encoded.len() % 2 != 0 {
        return None;
    }
    let bytes = encoded
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = (pair[0] as char).to_digit(16)? as u8;
            let low = (pair[1] as char).to_digit(16)? as u8;
            Some((high << 4) | low)
        })
        .collect::<Option<Vec<_>>>()?;
    String::from_utf8(bytes).ok()
}

/// Compiles explicitly named source outputs. Multiple names may intentionally
/// address the same register; the artifact retains each alias while resident
/// activation continues to share the underlying cell slot.
#[cfg(feature = "semantic-compiler")]
pub fn compile_executable_program_artifact_with_named_outputs_and_external_inputs(
    compiled: &CompiledBytecode,
    published_outputs: &[Register],
    published_output_names: &[Option<String>],
    catalog: &FunctionCatalog,
    external_input_names: &BTreeSet<String>,
) -> Result<ProgramArtifact, ArtifactBuildError> {
    validate_compiled_metadata_length(
        "published_output_names",
        published_outputs.len(),
        published_output_names.len(),
    )?;
    compile_executable_program_artifact_from_semantics(
        compiled,
        published_outputs,
        published_output_names,
        catalog,
        external_input_names,
    )
}

#[cfg(feature = "semantic-compiler")]
fn matrix_literal_mismatch(output: Register, reason: &'static str) -> ArtifactBuildError {
    ArtifactBuildError::MatrixLiteralMetadataMismatch { output, reason }
}

#[cfg(feature = "semantic-compiler")]
fn validate_compiled_matrix_literals(
    compiled: &CompiledBytecode,
) -> Result<BTreeMap<Register, usize>, ArtifactBuildError> {
    let mut instructions = BTreeMap::new();
    for (key, literal) in &compiled.matrix_literals {
        if *key != literal.output {
            return Err(matrix_literal_mismatch(
                *key,
                "sidecar key does not match output",
            ));
        }
        let output = literal.output;
        if output >= compiled.program.register_count {
            return Err(matrix_literal_mismatch(
                output,
                "output register is out of range",
            ));
        }
        if literal
            .elements
            .iter()
            .any(|element| element.register() >= compiled.program.register_count)
        {
            return Err(matrix_literal_mismatch(
                output,
                "element register is out of range",
            ));
        }

        let writers = compiled
            .program
            .instructions
            .iter()
            .enumerate()
            .filter(|(index, instruction)| {
                instruction_destination(instruction) == Some(output)
                    && !matches!(
                        compiled.instruction_roles[*index],
                        Some(CompiledInstructionRole::DeclarationMarker)
                    )
            })
            .collect::<Vec<_>>();
        let [
            (
                instruction_index,
                BytecodeInstruction::CompositePack {
                    template, children, ..
                },
            ),
        ] = writers.as_slice()
        else {
            return Err(matrix_literal_mismatch(
                output,
                "output must have exactly one CompositePack writer",
            ));
        };
        let descriptor_children = literal
            .elements
            .iter()
            .map(|element| element.register())
            .collect::<Vec<_>>();
        if children != &descriptor_children {
            return Err(matrix_literal_mismatch(
                output,
                "CompositePack children do not match the sidecar",
            ));
        }
        let Some(template_constant) = compiled.program.constants.get(*template as usize) else {
            return Err(matrix_literal_mismatch(
                output,
                "CompositePack template constant is out of range",
            ));
        };
        let RuntimeType::Kind(mech_core::BytecodeKind::Matrix(
            template_element,
            template_dimensions,
        )) = &template_constant.runtime_type
        else {
            return Err(matrix_literal_mismatch(
                output,
                "CompositePack template is not a matrix kind",
            ));
        };
        let [template_rows, template_columns] = template_dimensions.as_slice() else {
            return Err(matrix_literal_mismatch(
                output,
                "CompositePack matrix template is not rank two",
            ));
        };
        let Some(Some(SchemaBody::Matrix {
            element: output_element,
            dimensions: output_dimensions,
        })) = compiled.register_schemas.get(output as usize)
        else {
            return Err(matrix_literal_mismatch(
                output,
                "output register schema is not a matrix",
            ));
        };
        let [
            DimensionExpr::Constant(output_rows),
            DimensionExpr::Constant(output_columns),
        ] = output_dimensions.as_ref()
        else {
            return Err(matrix_literal_mismatch(
                output,
                "output register matrix schema does not have concrete rank-two dimensions",
            ));
        };
        let expected_template_element = mech_core::bytecode_kind_from_schema(output_element)?;
        let empty_extent = literal.rows == 0 || literal.columns == 0;
        if !(empty_extent && matches!(template_element.as_ref(), mech_core::BytecodeKind::Any))
            && template_element.as_ref() != &expected_template_element
        {
            return Err(matrix_literal_mismatch(
                output,
                "output register schema does not match the matrix template",
            ));
        }
        let literal_rows = usize::try_from(literal.rows).map_err(|_| {
            matrix_literal_mismatch(output, "matrix literal row count exceeds usize")
        })?;
        let literal_columns = usize::try_from(literal.columns).map_err(|_| {
            matrix_literal_mismatch(output, "matrix literal column count exceeds usize")
        })?;
        if [*template_rows, *template_columns] != [literal_rows, literal_columns]
            || [*output_rows as usize, *output_columns as usize] != [literal_rows, literal_columns]
        {
            return Err(matrix_literal_mismatch(
                output,
                "matrix kind dimensions do not match the sidecar",
            ));
        }
        for element in &literal.elements {
            let register = element.register();
            let constant = compiled
                .program
                .instructions
                .iter()
                .find_map(|instruction| match instruction {
                    BytecodeInstruction::ConstLoad { dst, constant } if *dst == register => {
                        Some(*constant)
                    }
                    _ => None,
                });
            match element {
                CompiledMatrixLiteralElement::Empty { .. } => {
                    if !constant
                        .and_then(|constant| compiled.program.constants.get(constant as usize))
                        .is_some_and(|constant| constant.runtime_type == RuntimeType::Empty)
                    {
                        return Err(matrix_literal_mismatch(
                            output,
                            "empty element does not reference an Empty constant",
                        ));
                    }
                }
                CompiledMatrixLiteralElement::Value { .. } => {
                    if constant
                        .and_then(|constant| compiled.program.constants.get(constant as usize))
                        .is_some_and(|constant| constant.runtime_type == RuntimeType::Empty)
                    {
                        return Err(matrix_literal_mismatch(
                            output,
                            "value element references an Empty constant",
                        ));
                    }
                }
            }
        }
        instructions.insert(output, *instruction_index);
    }

    let empty_registers = compiled
        .matrix_literals
        .values()
        .flat_map(|literal| literal.elements.iter())
        .filter_map(|element| match element {
            CompiledMatrixLiteralElement::Empty { register } => Some(*register),
            CompiledMatrixLiteralElement::Value { .. } => None,
        })
        .collect::<BTreeSet<_>>();
    for register in empty_registers {
        for (instruction_index, instruction) in compiled.program.instructions.iter().enumerate() {
            match instruction {
                BytecodeInstruction::ConstLoad { dst, .. } if *dst == register => {}
                BytecodeInstruction::CompositePack { dst, children, .. } => {
                    for (index, child) in children.iter().enumerate() {
                        if *child != register {
                            continue;
                        }
                        let allowed = compiled
                            .matrix_literals
                            .get(dst)
                            .and_then(|literal| literal.elements.get(index))
                            .is_some_and(|element| {
                                matches!(
                                    element,
                                    CompiledMatrixLiteralElement::Empty { register: empty }
                                        if *empty == register
                                )
                            });
                        if !allowed {
                            return Err(ArtifactBuildError::UnresolvedEmptyRegister { register });
                        }
                    }
                }
                _ if instruction_input_registers(instruction).contains(&register) => {
                    let _ = instruction_index;
                    return Err(ArtifactBuildError::UnresolvedEmptyRegister { register });
                }
                _ => {}
            }
        }
    }
    Ok(instructions)
}

#[cfg(feature = "semantic-compiler")]
fn instruction_input_registers(instruction: &BytecodeInstruction) -> Vec<Register> {
    match instruction {
        BytecodeInstruction::ConstLoad { .. }
        | BytecodeInstruction::RuntimeNullary { .. }
        | BytecodeInstruction::ResourceRead { .. } => Vec::new(),
        BytecodeInstruction::CompositePack { children, .. }
        | BytecodeInstruction::RuntimeVariadic {
            arguments: children,
            ..
        }
        | BytecodeInstruction::HostCall {
            arguments: children,
            ..
        } => children.clone(),
        BytecodeInstruction::RuntimeUnary { src, .. }
        | BytecodeInstruction::ResourceWrite { src, .. }
        | BytecodeInstruction::ResourceSend { src, .. }
        | BytecodeInstruction::Return { src } => vec![*src],
        BytecodeInstruction::RuntimeBinary { lhs, rhs, .. } => vec![*lhs, *rhs],
        BytecodeInstruction::RuntimeTernary { a, b, c, .. } => vec![*a, *b, *c],
        BytecodeInstruction::RuntimeQuaternary { a, b, c, d, .. } => vec![*a, *b, *c, *d],
    }
}

#[cfg(feature = "semantic-compiler")]
fn constant_for_register(
    register: Register,
    register_schemas: &[Option<SchemaId>],
    constants: &BTreeMap<(u32, SchemaId), ConstantId>,
    instructions: &[BytecodeInstruction],
) -> Option<ConstantId> {
    let schema = register_schemas.get(register as usize).copied().flatten()?;
    let encoded = instructions
        .iter()
        .find_map(|instruction| match instruction {
            BytecodeInstruction::ConstLoad { dst, constant } if *dst == register => Some(*constant),
            BytecodeInstruction::CompositePack { dst, template, .. } if *dst == register => {
                Some(*template)
            }
            _ => None,
        })?;
    constants.get(&(encoded, schema)).copied()
}

#[cfg(feature = "semantic-compiler")]
fn extend_constant_store_with_matrix_literals(
    schemas: &SchemaTable,
    constants: &ConstantStore,
    matrices: BTreeMap<Register, Value>,
) -> Result<
    (
        ConstantStore,
        BTreeMap<ConstantId, ConstantId>,
        BTreeMap<Register, ConstantId>,
    ),
    ArtifactBuildError,
> {
    let mut builder = ConstantStoreBuilder::new(schemas);
    let mut existing = BTreeMap::new();
    for raw in 0..constants.len() {
        let old = ConstantId::new(checked_u32(raw, "ConstantId")?);
        let value = constants
            .get(old)
            .ok_or(ArtifactBuildError::UnknownConstant { constant: old })?;
        existing.insert(old, builder.insert(value.clone())?);
    }
    let mut matrix_handles = BTreeMap::new();
    for (register, value) in matrices {
        matrix_handles.insert(register, builder.insert(value)?);
    }
    let build = builder.finish()?;
    let existing = existing
        .into_iter()
        .map(|(old, handle)| Ok((old, build.resolve(handle)?)))
        .collect::<Result<BTreeMap<_, _>, ArtifactBuildError>>()?;
    let matrices = matrix_handles
        .into_iter()
        .map(|(register, handle)| Ok((register, build.resolve(handle)?)))
        .collect::<Result<BTreeMap<_, _>, ArtifactBuildError>>()?;
    let (store, _) = build.into_parts();
    Ok((store, existing, matrices))
}

#[cfg(feature = "semantic-compiler")]
fn compile_executable_program_artifact_from_semantics(
    compiled: &CompiledBytecode,
    published_outputs: &[Register],
    published_output_names: &[Option<String>],
    catalog: &FunctionCatalog,
    external_input_names: &BTreeSet<String>,
) -> Result<ProgramArtifact, ArtifactBuildError> {
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
        "instruction_operations",
        compiled.program.instructions.len(),
        compiled.instruction_operations.len(),
    )?;
    validate_compiled_metadata_length(
        "instruction_source_nodes",
        compiled.program.instructions.len(),
        compiled.instruction_source_nodes.len(),
    )?;
    validate_compiled_metadata_length(
        "register_schemas",
        compiled.program.register_count as usize,
        compiled.register_schemas.len(),
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

    let canonical_constants = mech_core::decode_encoded_constants(&compiled.program.constants)?;
    let matrix_literal_instructions = validate_compiled_matrix_literals(compiled)?;

    struct PendingRegisterSchema {
        handle: SchemaHandle,
        contains_reference: bool,
    }

    let mut schema_builder = SchemaTableBuilder::new();
    let mut pending_register_schemas = Vec::with_capacity(compiled.register_schemas.len());
    for (register, body) in compiled.register_schemas.iter().enumerate() {
        if let Some(body) = body.clone() {
            let schema = SchemaDraft {
                dimension_parameters: Box::new([]),
                body,
            }
            .finalize()?;
            pending_register_schemas.push(Some(PendingRegisterSchema {
                handle: schema_builder.insert(schema)?,
                contains_reference: false,
            }));
            continue;
        }
        let register = checked_u32(register, "bytecode register")?;
        if compiled.absent_registers.contains(&register) {
            pending_register_schemas.push(None);
            continue;
        }
        pending_register_schemas.push(None);
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
        let has_constant_seed = compiled.program.instructions.iter().any(|instruction| {
            matches!(
                instruction,
                BytecodeInstruction::ConstLoad { dst, .. }
                    | BytecodeInstruction::CompositePack { dst, .. }
                    if *dst == register
            )
        });
        let explicitly_external = definitions.iter().any(|definition| {
            definition.root_visible && external_input_names.contains(&definition.name)
        });
        register_constant_roles[register_index] = Some(if has_mutable {
            CompilerConstantRole::StateInitializer
        } else if explicitly_external
            || (pending_schema.contains_reference
                && has_immutable
                && !computed_registers[register_index]
                && !has_constant_seed)
        {
            CompilerConstantRole::ExternalInput
        } else {
            CompilerConstantRole::Snapshot
        });
    }

    let mut pending_constants = BTreeSet::<(u32, SchemaId)>::new();
    for instruction in &compiled.program.instructions {
        let (register, constant) = match instruction {
            BytecodeInstruction::ConstLoad { dst, constant } => (*dst, *constant),
            BytecodeInstruction::CompositePack { dst, template, .. }
                if !compiled.matrix_literals.contains_key(dst) =>
            {
                (*dst, *template)
            }
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
        if role == CompilerConstantRole::ExternalInput {
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
        canonical_constants.get(constant as usize).ok_or(
            ArtifactBuildError::SourceGraphReferenceOutOfRange {
                reference: "bytecode constant",
                index: constant,
            },
        )?;
        pending_constants.insert((constant, schema));
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
        canonical_constants.get(*constant as usize).ok_or(
            ArtifactBuildError::SourceGraphReferenceOutOfRange {
                reference: "state initializer bytecode constant",
                index: *constant,
            },
        )?;
        pending_constants.insert((*constant, schema));
    }

    let mut constant_builder = ConstantStoreBuilder::new(&schemas);
    let mut constant_handles = BTreeMap::<(u32, SchemaId), ConstantHandle>::new();
    for (constant, schema) in pending_constants {
        let value = canonical_constants[constant as usize].rebind(
            schema,
            &schemas
                .get(schema)
                .expect("registered artifact schema remains present")
                .instantiate_shape(Box::new([]))?,
            &schemas,
        )?;
        constant_handles.insert((constant, schema), constant_builder.insert(value)?);
    }
    let constant_build = constant_builder.finish()?;
    let mut constants = constant_handles
        .into_iter()
        .map(|(key, handle)| Ok((key, constant_build.resolve(handle)?)))
        .collect::<Result<BTreeMap<_, _>, ArtifactBuildError>>()?;
    let (base_constant_store, _) = constant_build.into_parts();

    let mut register_matrix_literals = BTreeMap::<Register, RegisterMatrixLiteral>::new();
    let mut folded_matrix_values = BTreeMap::<Register, Value>::new();
    for literal in compiled.matrix_literals.values() {
        let elements = literal
            .elements
            .iter()
            .map(|element| match element {
                CompiledMatrixLiteralElement::Empty { .. } => RegisterMatrixLiteralElement::Empty,
                CompiledMatrixLiteralElement::Value { register }
                    if register_constant_roles[*register as usize]
                        == Some(CompilerConstantRole::Snapshot) =>
                {
                    constant_for_register(
                        *register,
                        &register_schemas,
                        &constants,
                        &compiled.program.instructions,
                    )
                    .map(RegisterMatrixLiteralElement::Constant)
                    .unwrap_or(RegisterMatrixLiteralElement::Register(*register))
                }
                CompiledMatrixLiteralElement::Value { register } => {
                    RegisterMatrixLiteralElement::Register(*register)
                }
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let source_literal = RegisterMatrixLiteral {
            rows: u64::from(literal.rows),
            columns: u64::from(literal.columns),
            elements,
        };
        let instruction = matrix_literal_instructions[&literal.output];
        if let Some(ir) = source_literal.constant_ir()
            && compiled.instruction_source_nodes[instruction].is_none()
        {
            let schema = register_schemas[literal.output as usize].ok_or(
                ArtifactBuildError::MissingRegisterKind {
                    instruction: checked_u32(instruction, "instruction")?,
                    register: literal.output,
                },
            )?;
            folded_matrix_values.insert(
                literal.output,
                ir.resolve_constant(schema, &schemas, &base_constant_store)?,
            );
        }
        register_matrix_literals.insert(literal.output, source_literal);
    }

    let (constant_store, constant_remap, folded_matrix_constants) =
        extend_constant_store_with_matrix_literals(
            &schemas,
            &base_constant_store,
            folded_matrix_values,
        )?;
    for constant in constants.values_mut() {
        *constant = constant_remap[constant];
    }
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

    let mut mutable_declaration_instructions = vec![None; compiled.program.register_count as usize];
    for (instruction_index, (instruction, role)) in compiled
        .program
        .instructions
        .iter()
        .zip(&compiled.instruction_roles)
        .enumerate()
    {
        if *role != Some(CompiledInstructionRole::DeclarationMarker) {
            continue;
        }
        let Some(register) = instruction_destination(instruction) else {
            continue;
        };
        if definitions_by_register[register as usize]
            .iter()
            .any(|definition| definition.mutable)
        {
            mutable_declaration_instructions[register as usize].get_or_insert(instruction_index);
        }
    }
    let assigned_state_registers = compiled
        .program
        .instructions
        .iter()
        .zip(&compiled.instruction_roles)
        .enumerate()
        .filter_map(|(instruction_index, (instruction, role))| {
            let register = matches!(role, Some(CompiledInstructionRole::Node(_)))
                .then(|| instruction_destination(instruction))
                .flatten()?;
            mutable_declaration_instructions[register as usize]
                .is_some_and(|declaration| instruction_index > declaration)
                .then_some(register)
        })
        .collect::<std::collections::BTreeSet<_>>();
    let mut inferred_state_initializers = vec![None; compiled.program.register_count as usize];
    for instruction in &compiled.program.instructions {
        let (register, constant) = match instruction {
            BytecodeInstruction::ConstLoad { dst, constant } => (*dst, *constant),
            BytecodeInstruction::CompositePack { dst, template, .. } => (*dst, *template),
            _ => continue,
        };
        if register_constant_roles[register as usize]
            != Some(CompilerConstantRole::StateInitializer)
        {
            continue;
        }
        let Some(schema) = register_schemas[register as usize] else {
            continue;
        };
        inferred_state_initializers[register as usize] = folded_matrix_constants
            .get(&register)
            .copied()
            .or_else(|| constants.get(&(constant, schema)).copied());
    }

    let mut register_state_indexes = vec![None::<u32>; compiled.program.register_count as usize];
    let mut pending_states = definitions_by_register
        .iter()
        .enumerate()
        .filter_map(|(register, definitions)| {
            mutable_declaration_instructions[register]?;
            definitions
                .iter()
                .find(|definition| definition.mutable)
                .map(|definition| (definition.ordinal, register))
        })
        .collect::<Vec<_>>();
    pending_states.sort_by_key(|(ordinal, _)| *ordinal);
    let mut states = Vec::<SourceState>::with_capacity(pending_states.len());
    for (_, register) in pending_states {
        let state = checked_u32(states.len(), "state")?;
        let schema = register_schemas[register].ok_or(ArtifactBuildError::MissingRegisterKind {
            instruction: 0,
            register: register as u32,
        })?;
        let initializer = explicit_state_initializers[register]
            .or(inferred_state_initializers[register])
            .ok_or(ArtifactBuildError::MissingRegisterSource {
                instruction: 0,
                register: register as u32,
                role: "state initializer",
            })?;
        register_state_indexes[register] = Some(state);
        states.push(SourceState {
            schema,
            initializer: Some(initializer),
            producer_node: u32::MAX,
            producer_output_ordinal: 0,
        });
    }

    let mut registers = vec![None::<RegisterSemantic>; compiled.program.register_count as usize];
    let mut nodes = Vec::<SourceNode>::new();
    let mut source_node_origins = Vec::<Option<u32>>::new();
    let mut node_contracts = Vec::<Option<OperationContractDeclaration>>::new();
    let mut node_matrix_literals = Vec::<Option<SourceMatrixLiteral>>::new();
    let mut lowered_declared_source_nodes = std::collections::BTreeSet::new();

    // A mutable declaration remains semantic state even when source contains
    // no later assignment. Its initializer seeds the resident state slot and
    // this pure self-copy node preserves that slot across accepted turns. A
    // declaration with a real writer continues to use that writer as its
    // producer instead.
    for (register, state) in register_state_indexes.iter().enumerate() {
        let Some(state) = *state else {
            continue;
        };
        if assigned_state_registers.contains(&(register as u32)) {
            continue;
        }
        let node = checked_u32(nodes.len(), "NodeId")?;
        states[state as usize].producer_node = node;
        nodes.push(SourceNode {
            operation: OperationReference {
                module_path: vec!["core".to_owned()].into_boxed_slice(),
                operation_name: "assign".to_owned(),
            },
            requirement: None,
            inputs: vec![SourceValue::State(state)].into_boxed_slice(),
            outputs: vec![SourceNodeOutput::State(state)].into_boxed_slice(),
        });
        node_contracts.push(Some((*COMPILER_STATE_HOLD_CONTRACT).clone()));
        node_matrix_literals.push(None);
    }

    for (instruction_index, instruction) in compiled.program.instructions.iter().enumerate() {
        let instruction_id = checked_u32(instruction_index, "instruction")?;
        let role = compiled.instruction_roles[instruction_index];
        if let BytecodeInstruction::CompositePack { dst, .. } = instruction
            && let Some(literal) = compiled.matrix_literals.get(dst)
        {
            let kind = match role {
                Some(CompiledInstructionRole::Node(kind)) => kind,
                Some(role) => {
                    return Err(ArtifactBuildError::UnexpectedInstructionRole {
                        instruction: instruction_id,
                        role: instruction_role_name(role),
                    });
                }
                None => {
                    return Err(ArtifactBuildError::MissingInstructionRole {
                        instruction: instruction_id,
                    });
                }
            };
            if kind != CompiledNodeKind::Combinational {
                return Err(matrix_literal_mismatch(
                    *dst,
                    "matrix literal CompositePack is not combinational",
                ));
            }
            let schema =
                register_schemas[*dst as usize].ok_or(ArtifactBuildError::MissingRegisterKind {
                    instruction: instruction_id,
                    register: *dst,
                })?;
            if register_constant_roles[*dst as usize] == Some(CompilerConstantRole::ExternalInput) {
                let input = input_indexes[*dst as usize].ok_or(
                    ArtifactBuildError::MissingRegisterSource {
                        instruction: instruction_id,
                        register: *dst,
                        role: "external input",
                    },
                )?;
                set_register(
                    &mut registers,
                    *dst,
                    Some(RegisterSemantic {
                        source: SourceValue::Input(input),
                        schema,
                    }),
                )?;
                continue;
            }
            if let Some(constant) = folded_matrix_constants.get(dst).copied() {
                let source = register_state_indexes[*dst as usize]
                    .map(SourceValue::State)
                    .unwrap_or(SourceValue::Constant(constant));
                set_register(
                    &mut registers,
                    *dst,
                    Some(RegisterSemantic { source, schema }),
                )?;
                continue;
            }

            let register_literal = &register_matrix_literals[dst];
            if register_literal.contains_empty() {
                let index = register_literal
                    .elements
                    .iter()
                    .position(|element| matches!(element, RegisterMatrixLiteralElement::Empty))
                    .expect("compiled matrix literals contain no nested expressions");
                return Err(
                    super::CompilerIrError::DynamicEmptyMatrixLiteralUnsupported { index }.into(),
                );
            }
            let node_index = checked_u32(nodes.len(), "NodeId")?;
            let state_index = if register_state_indexes[*dst as usize].is_some()
                && mutable_declaration_instructions[*dst as usize]
                    .is_some_and(|marker| instruction_index > marker)
            {
                let state = register_state_indexes[*dst as usize].ok_or(
                    ArtifactBuildError::MissingRegisterSource {
                        instruction: instruction_id,
                        register: *dst,
                        role: "state declaration",
                    },
                )?;
                states[state as usize].producer_node = node_index;
                Some(state)
            } else {
                None
            };
            let source_literal =
                source_matrix_literal_from_registers(register_literal, &registers, instruction_id)?;
            let inputs = source_literal
                .elements
                .iter()
                .filter_map(|element| *element)
                .collect::<Vec<_>>();
            if let Some(source_node) = compiled.instruction_source_nodes[instruction_index]
                && !lowered_declared_source_nodes.insert(source_node)
            {
                return Err(ArtifactBuildError::DeclaredSourceNodeLoweringUnsupported {
                    source_node,
                });
            }
            nodes.push(SourceNode {
                operation: OperationReference {
                    module_path: vec!["matrix".to_owned()].into_boxed_slice(),
                    operation_name: "literal".to_owned(),
                },
                requirement: None,
                inputs: inputs.into_boxed_slice(),
                outputs: match state_index {
                    Some(state) => vec![SourceNodeOutput::State(state)],
                    None => vec![SourceNodeOutput::Derived { schema }],
                }
                .into_boxed_slice(),
            });
            source_node_origins.push(compiled.instruction_source_nodes[instruction_index]);
            node_contracts.push(Some(matrix_literal_contract(literal.elements.len())));
            node_matrix_literals.push(Some(source_literal));
            let source = state_index
                .map(SourceValue::State)
                .unwrap_or(SourceValue::NodeOutput {
                    node: node_index,
                    output_ordinal: 0,
                });
            set_register(
                &mut registers,
                *dst,
                Some(RegisterSemantic { source, schema }),
            )?;
            continue;
        }
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
                    Some(CompilerConstantRole::StateInitializer) => {
                        match register_state_indexes.get(*dst as usize).copied().flatten() {
                            Some(state) => schema.map(|schema| RegisterSemantic {
                                source: SourceValue::State(state),
                                schema,
                            }),
                            None => schema.and_then(|schema| {
                                constants
                                    .get(&(*constant, schema))
                                    .copied()
                                    .map(|constant| RegisterSemantic {
                                        source: SourceValue::Constant(constant),
                                        schema,
                                    })
                            }),
                        }
                    }
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
                let semantics = instruction_semantics(
                    instruction_id,
                    instruction,
                    compiled.instruction_operations[instruction_id as usize].as_deref(),
                    &compiled.program.requirements,
                )?
                .ok_or(ArtifactBuildError::UnexpectedInstructionRole {
                    instruction: instruction_id,
                    role: "node",
                })?;
                let dst = semantics.destination;
                let schema = register_schemas.get(dst as usize).copied().flatten();
                let pseudo_destination = compiled.absent_registers.contains(&dst);
                if schema.is_none() && !pseudo_destination {
                    return Err(ArtifactBuildError::MissingRegisterKind {
                        instruction: instruction_id,
                        register: dst,
                    });
                }
                if register_constant_roles[dst as usize]
                    == Some(CompilerConstantRole::ExternalInput)
                {
                    let input = input_indexes[dst as usize].ok_or(
                        ArtifactBuildError::MissingRegisterSource {
                            instruction: instruction_id,
                            register: dst,
                            role: "external input",
                        },
                    )?;
                    set_register(
                        &mut registers,
                        dst,
                        Some(RegisterSemantic {
                            source: SourceValue::Input(input),
                            schema: schema.expect("external input has a validated schema"),
                        }),
                    )?;
                    continue;
                }
                let prior = register(&registers, dst)?;
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
                if matches!(instruction, BytecodeInstruction::CompositePack { .. })
                    && register_state_indexes
                        .get(dst as usize)
                        .copied()
                        .flatten()
                        .is_some()
                {
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
                // A one-element matrix row/column is represented by the
                // legacy intrinsic as a nullary concatenate whose output is
                // the very same reference as its already-compiled operand.
                // It is an identity wrapper, not a new producer node. Keep
                // the prior semantic source so `[a; b]` lowers to the actual
                // vertical concatenation inputs instead of two source-less
                // horizontal nodes.
                if kind == CompiledNodeKind::Combinational
                    && is_literal_constructor_operation(&semantics.operation)
                    && matches!(instruction, BytecodeInstruction::RuntimeNullary { .. })
                    && prior.is_some()
                {
                    continue;
                }
                if kind == CompiledNodeKind::Combinational
                    && is_literal_constructor_operation(&semantics.operation)
                    && (register_state_indexes[dst as usize].is_some()
                        || prior
                            .is_some_and(|value| matches!(value.source, SourceValue::Constant(_))))
                {
                    if register_state_indexes[dst as usize].is_some() {
                        continue;
                    }
                    let constructor_inputs = semantic_input_registers(&semantics, declaration)?
                        .iter()
                        .map(|input| {
                            register(&registers, *input)?
                                .map(|value| value.source)
                                .ok_or(ArtifactBuildError::MissingRegisterSource {
                                    instruction: instruction_id,
                                    register: *input,
                                    role: "literal constructor input",
                                })
                        })
                        .collect::<Result<Vec<_>, ArtifactBuildError>>()?;
                    if constructor_inputs
                        .iter()
                        .all(|source| matches!(source, SourceValue::Constant(_)))
                    {
                        continue;
                    }
                }
                let node_index = checked_u32(nodes.len(), "NodeId")?;
                let state_index = if register_state_indexes[dst as usize].is_some()
                    && mutable_declaration_instructions[dst as usize]
                        .is_some_and(|marker| instruction_index > marker)
                {
                    let _schema = schema.ok_or(ArtifactBuildError::MissingRegisterKind {
                        instruction: instruction_id,
                        register: dst,
                    })?;
                    let state = register_state_indexes[dst as usize].ok_or(
                        ArtifactBuildError::MissingRegisterSource {
                            instruction: instruction_id,
                            register: dst,
                            role: "state declaration",
                        },
                    )?;
                    let declaration = &mut states[state as usize];
                    declaration.producer_node = node_index;
                    Some(state)
                } else {
                    None
                };
                // Specialized plan nodes can expose a contract directly, but
                // catalog-installed runtime functions also carry authoritative
                // semantic metadata. Preserve that declaration when the
                // specialized function uses the trait's default `None`.
                let mut semantic_inputs = semantic_input_registers(&semantics, declaration)?
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
                    .collect::<Result<Vec<_>, ArtifactBuildError>>()?;
                if let Some(template) = semantics.template_constant {
                    let schema = schema.ok_or(ArtifactBuildError::MissingRegisterKind {
                        instruction: instruction_id,
                        register: dst,
                    })?;
                    let constant = constants.get(&(template, schema)).copied().ok_or(
                        ArtifactBuildError::SourceGraphReferenceOutOfRange {
                            reference: "composite template constant",
                            index: template,
                        },
                    )?;
                    semantic_inputs.insert(0, SourceValue::Constant(constant));
                }
                let semantic_inputs = semantic_inputs.into_boxed_slice();
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
                let exposes_output =
                    declaration.is_none_or(|declaration| !declaration.outputs.is_empty());
                nodes.push(SourceNode {
                    operation: semantics.operation,
                    requirement: semantics.requirement,
                    inputs: semantic_inputs,
                    outputs: match (exposes_output, state_index, schema) {
                        (false, _, _) => Vec::new(),
                        (true, Some(state), _) => vec![SourceNodeOutput::State(state)],
                        (true, None, Some(schema)) => vec![SourceNodeOutput::Derived { schema }],
                        (true, None, None) => Vec::new(),
                    }
                    .into_boxed_slice(),
                });
                source_node_origins.push(compiled.instruction_source_nodes[instruction_index]);
                node_contracts.push(declaration.cloned());
                node_matrix_literals.push(None);
                if schema.is_none() {
                    set_register(&mut registers, dst, None)?;
                    continue;
                }
                // Effect nodes such as resource writes deliberately expose no
                // graph output, but their bytecode destination still owns the
                // canonical unit seed returned by the source statement. Keep
                // that prior source available when the statement itself is a
                // published program result without fabricating an effect-node
                // output binding.
                if !exposes_output {
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
    let output_registers = if published_outputs.is_empty() {
        vec![compiled.return_register]
    } else {
        published_outputs.to_vec()
    };
    let encoded_output_names = published_output_names
        .iter()
        .map(|name| name.as_deref().map(encode_interactive_symbol_output_name))
        .collect::<Vec<_>>();
    let explicit_output_names = encoded_output_names
        .iter()
        .filter_map(Clone::clone)
        .collect::<BTreeSet<_>>();
    let mut output_names = BTreeMap::<String, usize>::new();
    let mut outputs = Vec::with_capacity(output_registers.len());
    for (ordinal, output_register) in output_registers.into_iter().enumerate() {
        let Some(returned) = register(&registers, output_register)? else {
            if compiled.absent_registers.contains(&output_register) {
                continue;
            }
            return Err(ArtifactBuildError::MissingRegisterSource {
                instruction: return_instruction,
                register: output_register,
                role: "published root output",
            });
        };
        let explicit_name = encoded_output_names.get(ordinal).and_then(Clone::clone);
        let mut base_name = explicit_name.clone().unwrap_or_else(|| {
            definitions_by_register[output_register as usize]
                .iter()
                .max_by_key(|definition| definition.ordinal)
                .map(|definition| definition.name.clone())
                .unwrap_or_else(|| {
                    if ordinal == 0 {
                        "result".to_owned()
                    } else {
                        format!("result-{}", ordinal + 1)
                    }
                })
        });
        if explicit_name.is_none() && explicit_output_names.contains(&base_name) {
            let mut suffix = 2;
            loop {
                let candidate = format!("{base_name}-{suffix}");
                if !explicit_output_names.contains(&candidate)
                    && !output_names.contains_key(&candidate)
                {
                    base_name = candidate;
                    break;
                }
                suffix += 1;
            }
        }
        let occurrence = output_names.entry(base_name.clone()).or_default();
        *occurrence += 1;
        let name = if *occurrence == 1 {
            base_name
        } else {
            format!("{base_name}-{}", *occurrence)
        };
        outputs.push(SourceOutput {
            name,
            interactive_symbol: published_output_names.get(ordinal).and_then(Clone::clone),
            source: returned.source,
            schema: returned.schema,
        });
    }

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
            name: constraint.name.clone(),
            operation: OperationReference {
                module_path: vec!["integrity".to_owned()].into_boxed_slice(),
                operation_name: "assert".to_owned(),
            },
            inputs: vec![semantic.source].into_boxed_slice(),
        });
    }

    let (inputs, mut nodes, mut outputs, mut constraints) = prune_unused_inputs(
        inputs,
        nodes,
        outputs,
        constraints,
        &mut node_matrix_literals,
    )?;
    let constant_store = prune_unused_constants(
        &schemas,
        &constant_store,
        &mut states,
        &mut nodes,
        &mut outputs,
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
    let node_contract_refs = node_contracts
        .iter()
        .enumerate()
        .map(|(node, declaration)| {
            declaration
                .as_ref()
                .ok_or(ArtifactBuildError::MissingOperationContract {
                    node: NodeId::new(node as u32),
                    operation: source.nodes[node].operation.clone(),
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let artifact = compile_source_program_with_metadata(
        &source,
        &mut ArtifactBuildContext::new(&schemas, &constant_store),
        &node_contract_refs,
        &node_matrix_literals,
    )?;
    let compute_regions = compiled
        .compute_regions
        .iter()
        .enumerate()
        .map(|(region_index, region)| ComputeRegionDeclaration {
            id: mech_core::ComputeRegionId::new(region_index as u32),
            name: region.name.clone().into_boxed_str(),
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
    artifact.with_compute_regions(compute_regions)
}

#[cfg(feature = "semantic-compiler")]
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
            matches!(
                request.intent,
                mech_core::ResourceIntent::Assign | mech_core::ResourceIntent::Send
            ) && request.delivery == mech_core::ResourceDelivery::Snapshot
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

#[cfg(feature = "semantic-compiler")]
fn prune_unused_constants(
    schemas: &SchemaTable,
    constants: &ConstantStore,
    states: &mut [SourceState],
    nodes: &mut [SourceNode],
    outputs: &mut [SourceOutput],
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
    for output in outputs.iter() {
        if let SourceValue::Constant(constant) = output.source {
            used.insert(constant);
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
    for output in outputs {
        remap_source(&mut output.source);
    }
    for constraint in constraints {
        for source in &mut constraint.inputs {
            remap_source(source);
        }
    }
    Ok(constants)
}

#[cfg(feature = "semantic-compiler")]
fn is_literal_constructor_operation(operation: &OperationReference) -> bool {
    (operation.module_path.as_ref() == ["matrix"]
        && matches!(operation.operation_name.as_str(), "horzcat" | "vertcat"))
        || (operation.module_path.as_ref() == ["set"] && operation.operation_name == "define")
}

#[cfg(feature = "semantic-compiler")]
fn prune_unused_inputs(
    inputs: Vec<SourceInput>,
    mut nodes: Vec<SourceNode>,
    mut outputs: Vec<SourceOutput>,
    mut constraints: Vec<SourceIntegrityConstraint>,
    node_matrix_literals: &mut [Option<SourceMatrixLiteral>],
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
        note(output.source);
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
    for literal in node_matrix_literals.iter_mut().flatten() {
        for element in &mut literal.elements {
            if let Some(source) = element {
                remap_value(source);
            }
        }
    }
    for output in &mut outputs {
        remap_value(&mut output.source);
    }
    for constraint in &mut constraints {
        for source in &mut constraint.inputs {
            remap_value(source);
        }
    }
    Ok((retained, nodes, outputs, constraints))
}

#[cfg(feature = "semantic-compiler")]
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

#[cfg(feature = "semantic-compiler")]
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

#[cfg(feature = "semantic-compiler")]
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

#[cfg(feature = "semantic-compiler")]
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
                    instruction_semantics(
                        instruction_id,
                        instruction,
                        compiled.instruction_operations[index].as_deref(),
                        &compiled.program.requirements,
                    )?;
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

#[cfg(feature = "semantic-compiler")]
fn instruction_role_name(role: CompiledInstructionRole) -> &'static str {
    match role {
        CompiledInstructionRole::Node(_) => "node",
        CompiledInstructionRole::IntegrityMarker => "integrity marker",
        CompiledInstructionRole::DeclarationMarker => "declaration marker",
    }
}

#[cfg(feature = "semantic-compiler")]
struct CompiledInstructionSemantics {
    destination: u32,
    inputs: Vec<u32>,
    template_constant: Option<u32>,
    operation: OperationReference,
    requirement: Option<ApplicationRequirementId>,
}

/// Some executable instructions use their destination as the logical base of
/// a read/modify/write operation without repeating it in the operand list.
/// The semantic artifact exposes that dependency without changing bytecode.
#[cfg(feature = "semantic-compiler")]
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

#[cfg(feature = "semantic-compiler")]
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

#[cfg(feature = "semantic-compiler")]
fn is_variable_definition_instruction(
    instruction: &BytecodeInstruction,
    catalog: &FunctionCatalog,
) -> bool {
    instruction
        .runtime_function()
        .and_then(|function| catalog.runtime_entry_by_raw(function))
        .is_some_and(|entry| entry.name.starts_with("VariableDefine"))
}

#[cfg(feature = "semantic-compiler")]
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

#[cfg(feature = "semantic-compiler")]
fn semantic_operation_reference(
    canonical_name: &str,
) -> Result<OperationReference, ArtifactBuildError> {
    let canonical_name = match canonical_name {
        "matrix/matmul" => "matrix/multiply",
        "assign" => "core/assign",
        name => name,
    };
    operation_reference_from_name("core", canonical_name)
}

#[cfg(feature = "semantic-compiler")]
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

#[cfg(feature = "semantic-compiler")]
fn instruction_semantics(
    instruction_id: u32,
    instruction: &BytecodeInstruction,
    semantic_operation: Option<&str>,
    requirements: &[ApplicationRequirement],
) -> Result<Option<CompiledInstructionSemantics>, ArtifactBuildError> {
    let runtime = |function: u64| {
        semantic_operation
            .ok_or_else(|| ArtifactBuildError::MissingSemanticOperation {
                instruction: instruction_id,
                implementation: format!("0x{function:016x}"),
            })
            .and_then(semantic_operation_reference)
    };
    let semantics = match instruction {
        BytecodeInstruction::ConstLoad { .. } | BytecodeInstruction::Return { .. } => {
            return Ok(None);
        }
        BytecodeInstruction::CompositePack {
            dst,
            template,
            children,
        } => CompiledInstructionSemantics {
            destination: *dst,
            inputs: children.clone(),
            template_constant: Some(*template),
            operation: OperationReference {
                module_path: vec!["core".to_owned()].into_boxed_slice(),
                operation_name: "composite-pack".to_owned(),
            },
            requirement: None,
        },
        BytecodeInstruction::RuntimeNullary { function, dst } => CompiledInstructionSemantics {
            destination: *dst,
            inputs: Vec::new(),
            template_constant: None,
            operation: runtime(*function)?,
            requirement: None,
        },
        BytecodeInstruction::RuntimeUnary { function, dst, src } => CompiledInstructionSemantics {
            destination: *dst,
            inputs: vec![*src],
            template_constant: None,
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
            template_constant: None,
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
            template_constant: None,
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
            template_constant: None,
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
            template_constant: None,
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
                template_constant: None,
                operation: operation_reference_from_name("host", &request.name)?,
                requirement: Some(ApplicationRequirementId::new(*requirement)),
            }
        }
        BytecodeInstruction::ResourceRead { requirement, dst } => CompiledInstructionSemantics {
            destination: *dst,
            inputs: Vec::new(),
            template_constant: None,
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
            template_constant: None,
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
            template_constant: None,
            operation: resource_operation_reference(*requirement, requirements, "send")?,
            requirement: Some(ApplicationRequirementId::new(*requirement)),
        },
    };
    Ok(Some(semantics))
}

fn resolve_source_matrix_literal(
    literal: &SourceMatrixLiteral,
    inputs: &[CellSlotId],
    states: &[CellSlotId],
    outputs: &BTreeMap<(u32, u16), CellSlotId>,
) -> Result<MatrixLiteralIR, ArtifactBuildError> {
    let elements = literal
        .elements
        .iter()
        .map(|element| match element {
            None => Ok(ExpressionIR::Empty),
            Some(source) => Ok(match resolve_source(*source, inputs, states, outputs)? {
                ArtifactSource::Constant(constant) => ExpressionIR::Constant(constant),
                ArtifactSource::Slot(slot) => ExpressionIR::Slot(slot),
            }),
        })
        .collect::<Result<Vec<_>, ArtifactBuildError>>()?;
    Ok(MatrixLiteralIR {
        rows: literal.rows,
        columns: literal.columns,
        elements: elements.into_boxed_slice(),
    })
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

fn published_output_initializer(graph: &SourceProgram, source: SourceValue) -> Option<ConstantId> {
    match source {
        SourceValue::Constant(constant) => Some(constant),
        SourceValue::NodeOutput {
            node,
            output_ordinal: 0,
        } => {
            let node = graph.nodes.get(node as usize)?;
            (node.operation.module_path.as_ref() == ["core"]
                && node.operation.operation_name == "composite-pack")
                .then(|| match node.inputs.first() {
                    Some(SourceValue::Constant(template)) => Some(*template),
                    _ => None,
                })
                .flatten()
        }
        SourceValue::Input(_) | SourceValue::State(_) | SourceValue::NodeOutput { .. } => None,
    }
}

fn checked_u32(value: usize, identity: &'static str) -> Result<u32, ArtifactBuildError> {
    u32::try_from(value).map_err(|_| ArtifactBuildError::ArtifactIdentityExhausted { identity })
}

fn checked_u16(value: usize, identity: &'static str) -> Result<u16, ArtifactBuildError> {
    u16::try_from(value).map_err(|_| ArtifactBuildError::ArtifactIdentityExhausted { identity })
}

#[cfg(all(test, feature = "semantic-compiler"))]
mod tests {
    use super::*;

    #[test]
    fn matrix_ir_slots_use_resolved_artifact_identity_not_register_numbers() {
        let mut registers = vec![None; 10];
        registers[9] = Some(RegisterSemantic {
            source: SourceValue::Input(0),
            schema: SchemaId::new(0),
        });
        let register_literal = RegisterMatrixLiteral {
            rows: 1,
            columns: 1,
            elements: vec![RegisterMatrixLiteralElement::Register(9)].into_boxed_slice(),
        };

        let source_literal =
            source_matrix_literal_from_registers(&register_literal, &registers, 0).unwrap();
        let ir = resolve_source_matrix_literal(
            &source_literal,
            &[CellSlotId::new(0)],
            &[],
            &BTreeMap::new(),
        )
        .unwrap();

        assert_eq!(
            ir.elements.as_ref(),
            [ExpressionIR::Slot(CellSlotId::new(0))]
        );
        assert_ne!(
            ir.elements.as_ref(),
            [ExpressionIR::Slot(CellSlotId::new(9))]
        );
    }

    #[test]
    fn published_reactive_composite_retains_its_canonical_template_initializer() {
        let template = ConstantId::new(3);
        let graph = SourceProgram {
            nodes: vec![SourceNode {
                operation: OperationReference {
                    module_path: vec!["core".to_owned()].into_boxed_slice(),
                    operation_name: "composite-pack".to_owned(),
                },
                requirement: None,
                inputs: vec![SourceValue::Constant(template), SourceValue::Input(0)]
                    .into_boxed_slice(),
                outputs: vec![SourceNodeOutput::Derived {
                    schema: SchemaId::new(0),
                }]
                .into_boxed_slice(),
            }]
            .into_boxed_slice(),
            ..SourceProgram::default()
        };

        assert_eq!(
            published_output_initializer(
                &graph,
                SourceValue::NodeOutput {
                    node: 0,
                    output_ordinal: 0,
                },
            ),
            Some(template),
        );
        assert_eq!(
            published_output_initializer(
                &graph,
                SourceValue::NodeOutput {
                    node: 0,
                    output_ordinal: 1,
                },
            ),
            None,
        );
    }
}
