use core::ops::Range;

use mech_core::{
    BindingId, CellSlotId, ConstantId, ConstantStore, InputId, IntegrityConstraintId,
    LegacySnapshotError, MechError, NodeId, OutputId, ProgramRevision, SchemaId, SchemaTable,
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
    pub inputs: Box<[ArtifactSource]>,
}

#[derive(Clone, Debug)]
pub struct ProgramArtifact {
    revision: ProgramRevision,
    schemas: SchemaTable,
    constants: ConstantStore,
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
            inputs: self.inputs,
            slots: self.slots,
            nodes: self.nodes,
            bindings: self.bindings,
            outputs: self.outputs,
            constraints: self.constraints,
        })
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

impl From<MechError> for ArtifactBuildError {
    fn from(error: MechError) -> Self {
        Self::CoreBytecode(error)
    }
}
