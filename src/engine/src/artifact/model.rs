use core::ops::Range;

use mech_core::{
    AccessMode, BindingId, CellSlotId, ConstantId, ConstantStore, DeclaredOperationContract,
    DeliveryMode, ExternalInteraction, InputId, IntegrityConstraintId,
    LegacyOpaqueOperationContract, LegacySnapshotError, MechError, NodeId, OperationContractError,
    OperationContractId, OperationContractTable, OperationContractTableBuilder, OutputId,
    ProgramRevision, ResolvedInputPort, ResolvedOperationContract, SchemaId, SchemaTable,
    SemanticModelError, SnapshotValueError,
};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OperationReference {
    pub module_path: Box<[String]>,
    pub operation_name: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SlotRole {
    Input,
    State,
    Derived,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProducerReference {
    Input(InputId),
    NodeOutput { node: NodeId, output_ordinal: u16 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InitializerReference {
    Constant(ConstantId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SlotDeclaration {
    pub slot: CellSlotId,
    pub schema: SchemaId,
    pub role: SlotRole,
    pub producer: ProducerReference,
    pub initializer: Option<InitializerReference>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArtifactSource {
    Constant(ConstantId),
    Slot(CellSlotId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BindingDeclaration {
    Input {
        id: BindingId,
        node: NodeId,
        port_ordinal: u16,
        source: ArtifactSource,
    },
    Output {
        id: BindingId,
        node: NodeId,
        port_ordinal: u16,
        target: CellSlotId,
    },
}

impl BindingDeclaration {
    pub const fn id(&self) -> BindingId {
        match self {
            Self::Input { id, .. } | Self::Output { id, .. } => *id,
        }
    }

    pub const fn node(&self) -> NodeId {
        match self {
            Self::Input { node, .. } | Self::Output { node, .. } => *node,
        }
    }

    pub const fn port_ordinal(&self) -> u16 {
        match self {
            Self::Input { port_ordinal, .. } | Self::Output { port_ordinal, .. } => *port_ordinal,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeDeclaration {
    pub node: NodeId,
    pub operation: OperationReference,
    pub contract: OperationContractId,
    pub input_bindings: Range<u32>,
    pub output_bindings: Range<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InputDeclaration {
    pub input: InputId,
    pub name: String,
    pub slot: CellSlotId,
    pub schema: SchemaId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputDeclaration {
    pub output: OutputId,
    pub name: String,
    pub source: CellSlotId,
    pub schema: SchemaId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntegrityConstraintDeclaration {
    pub constraint: IntegrityConstraintId,
    pub operation: OperationReference,
    pub contract: OperationContractId,
    pub inputs: Box<[ArtifactSource]>,
}

#[derive(Clone, Debug)]
pub struct ProgramArtifact {
    revision: ProgramRevision,
    schemas: SchemaTable,
    constants: ConstantStore,
    contracts: OperationContractTable,
    inputs: Box<[InputDeclaration]>,
    slots: Box<[SlotDeclaration]>,
    nodes: Box<[NodeDeclaration]>,
    bindings: Box<[BindingDeclaration]>,
    outputs: Box<[OutputDeclaration]>,
    constraints: Box<[IntegrityConstraintDeclaration]>,
}

impl ProgramArtifact {
    pub const fn revision(&self) -> ProgramRevision {
        self.revision
    }

    pub const fn schemas(&self) -> &SchemaTable {
        &self.schemas
    }

    pub const fn constants(&self) -> &ConstantStore {
        &self.constants
    }

    pub const fn contracts(&self) -> &OperationContractTable {
        &self.contracts
    }

    pub const fn inputs(&self) -> &[InputDeclaration] {
        &self.inputs
    }

    pub const fn slots(&self) -> &[SlotDeclaration] {
        &self.slots
    }

    pub const fn nodes(&self) -> &[NodeDeclaration] {
        &self.nodes
    }

    pub const fn bindings(&self) -> &[BindingDeclaration] {
        &self.bindings
    }

    pub const fn outputs(&self) -> &[OutputDeclaration] {
        &self.outputs
    }

    pub const fn constraints(&self) -> &[IntegrityConstraintDeclaration] {
        &self.constraints
    }
}

#[derive(Clone, Debug)]
pub struct ProgramArtifactDraft {
    pub schemas: SchemaTable,
    pub constants: ConstantStore,
    pub contracts: OperationContractTable,
    pub inputs: Box<[InputDeclaration]>,
    pub slots: Box<[SlotDeclaration]>,
    pub nodes: Box<[NodeDeclaration]>,
    pub bindings: Box<[BindingDeclaration]>,
    pub outputs: Box<[OutputDeclaration]>,
    pub constraints: Box<[IntegrityConstraintDeclaration]>,
}

impl ProgramArtifactDraft {
    pub fn finalize(self) -> Result<ProgramArtifact, ArtifactBuildError> {
        super::validation::validate(&self)?;
        let revision = super::encoding::program_revision(&self)?;
        Ok(ProgramArtifact {
            revision,
            schemas: self.schemas,
            constants: self.constants,
            contracts: self.contracts,
            inputs: self.inputs,
            slots: self.slots,
            nodes: self.nodes,
            bindings: self.bindings,
            outputs: self.outputs,
            constraints: self.constraints,
        })
    }
}

impl ProgramArtifactDraft {
    pub(super) fn attach_legacy_contracts(mut self) -> Result<Self, ArtifactBuildError> {
        let mut builder = OperationContractTableBuilder::new();
        let mut node_handles = Vec::with_capacity(self.nodes.len());
        for node in &self.nodes {
            let inputs = binding_range(&self, &node.input_bindings, node.node)?
                .iter()
                .map(|binding| match binding {
                    BindingDeclaration::Input { source, .. } => source_schema(&self, *source),
                    BindingDeclaration::Output { id, .. } => {
                        Err(ArtifactBuildError::BindingDirectionMismatch { binding: *id })
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;
            let outputs = binding_range(&self, &node.output_bindings, node.node)?
                .iter()
                .map(|binding| match binding {
                    BindingDeclaration::Output { target, .. } => self
                        .slots
                        .get(target.get() as usize)
                        .filter(|slot| slot.slot == *target)
                        .map(|slot| slot.schema)
                        .ok_or(ArtifactBuildError::UnknownSlot { slot: *target }),
                    BindingDeclaration::Input { id, .. } => {
                        Err(ArtifactBuildError::BindingDirectionMismatch { binding: *id })
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;
            node_handles.push(builder.insert(ResolvedOperationContract::LegacyOpaque(
                LegacyOpaqueOperationContract {
                    input_schemas: inputs.into_boxed_slice(),
                    output_schemas: outputs.into_boxed_slice(),
                },
            ))?);
        }

        let mut constraint_handles = Vec::with_capacity(self.constraints.len());
        for constraint in &self.constraints {
            let inputs = constraint
                .inputs
                .iter()
                .map(|source| {
                    Ok(ResolvedInputPort {
                        schema: source_schema(&self, *source)?,
                        access: AccessMode::Read,
                        delivery: DeliveryMode::Signal,
                    })
                })
                .collect::<Result<Vec<_>, ArtifactBuildError>>()?;
            constraint_handles.push(builder.insert(ResolvedOperationContract::Declared(
                DeclaredOperationContract {
                    inputs: inputs.into_boxed_slice(),
                    outputs: Box::new([]),
                    interaction: ExternalInteraction::Pure,
                },
            ))?);
        }

        let build = builder.finish()?;
        for (node, handle) in self.nodes.iter_mut().zip(node_handles) {
            node.contract = build.resolve(handle)?;
        }
        for (constraint, handle) in self.constraints.iter_mut().zip(constraint_handles) {
            constraint.contract = build.resolve(handle)?;
        }
        self.contracts = build.table;
        Ok(self)
    }
}

fn binding_range<'a>(
    draft: &'a ProgramArtifactDraft,
    range: &Range<u32>,
    node: NodeId,
) -> Result<&'a [BindingDeclaration], ArtifactBuildError> {
    draft
        .bindings
        .get(range.start as usize..range.end as usize)
        .ok_or(ArtifactBuildError::BindingRangeMismatch { node })
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
        ArtifactSource::Slot(slot) => draft
            .slots
            .get(slot.get() as usize)
            .filter(|declaration| declaration.slot == slot)
            .map(|declaration| declaration.schema)
            .ok_or(ArtifactBuildError::UnknownSlot { slot }),
    }
}

#[derive(Debug)]
pub enum ArtifactBuildError {
    CompiledMetadataLengthMismatch {
        table: &'static str,
        expected: usize,
        actual: usize,
    },
    MissingInstructionRole {
        instruction: u32,
    },
    UnexpectedInstructionRole {
        instruction: u32,
        role: &'static str,
    },
    MissingRegisterKind {
        instruction: u32,
        register: u32,
    },
    MissingRegisterSource {
        instruction: u32,
        register: u32,
        role: &'static str,
    },
    UnknownRuntimeFunction {
        function: u64,
    },
    UnknownApplicationRequirement {
        requirement: u32,
    },
    ApplicationRequirementKindMismatch {
        requirement: u32,
        expected: &'static str,
    },
    InvalidCompiledOperationName {
        namespace: &'static str,
        name: String,
    },
    MissingInputInterfaceName {
        register: u32,
    },
    AmbiguousRegisterRole {
        register: u32,
    },
    MissingRegisterCollectionCardinality {
        register: u32,
    },
    CompiledReturnCount {
        found: usize,
    },
    NonTerminalCompiledReturn {
        instruction: u32,
    },
    CompiledReturnRegisterMismatch {
        instruction: u32,
        expected: u32,
        found: u32,
    },
    IntegrityConstraintMetadataMismatch {
        constraint: u32,
        marker_register: Option<u32>,
        declared_register: Option<u32>,
    },
    IntegrityConstraintSchemaMismatch {
        constraint: u32,
        schema: SchemaId,
    },
    InvalidOperationReference {
        operation: OperationReference,
    },
    NonCanonicalIdentity {
        identity: &'static str,
        expected: u32,
        found: u32,
    },
    DuplicateInterfaceName {
        interface: &'static str,
        name: String,
    },
    InvalidInterfaceName {
        interface: &'static str,
        name: String,
    },
    UnknownSchema {
        schema: SchemaId,
    },
    UnknownOperationContract {
        contract: OperationContractId,
    },
    ContractInputSchemaMismatch {
        contract: OperationContractId,
        port: u16,
        expected: SchemaId,
        actual: SchemaId,
    },
    ContractOutputSchemaMismatch {
        contract: OperationContractId,
        port: u16,
        expected: SchemaId,
        actual: SchemaId,
    },
    IntegrityConstraintContractInvalid {
        constraint: IntegrityConstraintId,
    },
    UnknownConstant {
        constant: ConstantId,
    },
    InitializerSchemaMismatch {
        slot: CellSlotId,
        constant: ConstantId,
    },
    UnknownInput {
        input: InputId,
    },
    UnknownNode {
        node: NodeId,
    },
    UnknownSlot {
        slot: CellSlotId,
    },
    InvalidSlotRole {
        slot: CellSlotId,
    },
    DuplicateProducer {
        producer: ProducerReference,
    },
    ProducerBindingMismatch {
        slot: CellSlotId,
    },
    MissingProducerBinding {
        slot: CellSlotId,
        producer: ProducerReference,
    },
    BindingRangeMismatch {
        node: NodeId,
    },
    BindingDirectionMismatch {
        binding: BindingId,
    },
    BindingNodeMismatch {
        binding: BindingId,
    },
    BindingPortMismatch {
        binding: BindingId,
        expected: u16,
        found: u16,
    },
    InterfaceSlotMismatch {
        interface: &'static str,
        slot: CellSlotId,
    },
    ArtifactIdentityExhausted {
        identity: &'static str,
    },
    SourceGraphReferenceOutOfRange {
        reference: &'static str,
        index: u32,
    },
    CombinationalCycle,
    Snapshot(SnapshotValueError),
    Semantic(SemanticModelError),
    LegacySnapshot(LegacySnapshotError),
    CoreBytecode(MechError),
    OperationContract(OperationContractError),
}

impl From<SnapshotValueError> for ArtifactBuildError {
    fn from(error: SnapshotValueError) -> Self {
        Self::Snapshot(error)
    }
}

impl From<SemanticModelError> for ArtifactBuildError {
    fn from(error: SemanticModelError) -> Self {
        Self::Semantic(error)
    }
}

impl From<LegacySnapshotError> for ArtifactBuildError {
    fn from(error: LegacySnapshotError) -> Self {
        Self::LegacySnapshot(error)
    }
}

impl From<OperationContractError> for ArtifactBuildError {
    fn from(error: OperationContractError) -> Self {
        Self::OperationContract(error)
    }
}

impl From<MechError> for ArtifactBuildError {
    fn from(error: MechError) -> Self {
        Self::CoreBytecode(error)
    }
}
