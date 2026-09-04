use core::ops::Range;

use mech_core::{
    AccessMode, ApplicationRequirementId, BindingId, CellSlotId, ComputePlacement, ComputeRegionId,
    ConstantId, ConstantStore, DeclaredOperationContract, DeliveryMode, ExternalInteraction,
    InputId, IntegrityConstraintId, MechError, NodeId, OperationContractDeclaration,
    OperationContractError, OperationContractId, OperationContractTable,
    OperationContractTableBuilder, OutputId, PortDirection, ProgramRevision, ResolvedInputPort,
    ResolvedOperationContract, ResolvedOutputPort, ResolvedRangeMode, ResolvedReductionMode,
    ResolvedSelectionMode, SchemaId, SchemaTable, SemanticModelError, SnapshotValueError,
    validate_declaration,
};

use super::CompilerIrError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComputeRegionDeclaration {
    pub id: ComputeRegionId,
    pub name: Box<str>,
    pub placement: ComputePlacement,
    pub nodes: Box<[NodeId]>,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OperationReference {
    pub module_path: Box<[String]>,
    pub operation_name: String,
}

impl OperationReference {
    pub fn canonical_name(&self) -> String {
        if self.module_path.is_empty() {
            return self.operation_name.clone();
        }
        format!("{}/{}", self.module_path.join("/"), self.operation_name)
    }

    fn module_is(&self, expected: &[&str]) -> bool {
        self.module_path.len() == expected.len()
            && self
                .module_path
                .iter()
                .zip(expected)
                .all(|(actual, expected)| actual == expected)
    }

    /// Returns the semantic selection encoded by this canonical operation
    /// identity. The result depends only on artifact data, never on shapes.
    pub fn resolved_selection_mode(&self, selector_count: usize) -> Option<ResolvedSelectionMode> {
        match self.operation_name.as_str() {
            "scalar" if self.module_is(&["access"]) && selector_count == 1 => {
                Some(ResolvedSelectionMode::LinearScalar)
            }
            "range" if self.module_is(&["access"]) && selector_count == 1 => {
                Some(ResolvedSelectionMode::LinearGather)
            }
            "scalar" | "range" if self.module_is(&["access"]) && selector_count == 2 => {
                Some(ResolvedSelectionMode::Rectangle)
            }
            "index" if self.module_is(&["access"]) && selector_count == 0 => {
                Some(ResolvedSelectionMode::LinearScalar)
            }
            "rows" if self.module_is(&["access"]) => Some(ResolvedSelectionMode::Rows),
            "columns" if self.module_is(&["access"]) => Some(ResolvedSelectionMode::Columns),
            "rectangle" if self.module_is(&["access"]) => Some(ResolvedSelectionMode::Rectangle),
            "assign" if self.module_is(&["core"]) => Some(ResolvedSelectionMode::Whole),
            "range-all"
                if self.module_is(&["math", "add-assign"])
                    || self.module_is(&["math", "sub-assign"]) =>
            {
                Some(ResolvedSelectionMode::Rows)
            }
            _ => None,
        }
    }

    pub fn resolved_range_mode(&self) -> Option<ResolvedRangeMode> {
        if !self.module_is(&["range"]) {
            return None;
        }
        match self.operation_name.as_str() {
            "exclusive" => Some(ResolvedRangeMode::Exclusive),
            "exclusive-increment" => Some(ResolvedRangeMode::ExclusiveIncrement),
            "inclusive" => Some(ResolvedRangeMode::Inclusive),
            "inclusive-increment" => Some(ResolvedRangeMode::InclusiveIncrement),
            _ => None,
        }
    }

    pub fn resolved_reduction_mode(&self) -> Option<ResolvedReductionMode> {
        if !self.module_is(&["stats", "sum"]) {
            return None;
        }
        match self.operation_name.as_str() {
            "row" => Some(ResolvedReductionMode::Rows),
            "column" => Some(ResolvedReductionMode::Columns),
            _ => None,
        }
    }
}

#[cfg(test)]
mod operation_mode_tests {
    use super::*;

    fn operation(module_path: &[&str], operation_name: &str) -> OperationReference {
        OperationReference {
            module_path: module_path
                .iter()
                .map(|segment| (*segment).to_owned())
                .collect(),
            operation_name: operation_name.to_owned(),
        }
    }

    #[test]
    fn semantic_modes_are_resolved_from_artifact_identity_and_selector_arity() {
        let scalar = operation(&["access"], "scalar");
        assert_eq!(
            scalar.resolved_selection_mode(1),
            Some(ResolvedSelectionMode::LinearScalar)
        );
        assert_eq!(
            operation(&["access"], "range").resolved_selection_mode(1),
            Some(ResolvedSelectionMode::LinearGather)
        );
        assert_eq!(
            scalar.resolved_selection_mode(2),
            Some(ResolvedSelectionMode::Rectangle)
        );
        assert_eq!(
            operation(&["access"], "rows").resolved_selection_mode(1),
            Some(ResolvedSelectionMode::Rows)
        );
        assert_eq!(
            operation(&["access"], "columns").resolved_selection_mode(1),
            Some(ResolvedSelectionMode::Columns)
        );
        assert_eq!(
            operation(&["range"], "inclusive-increment").resolved_range_mode(),
            Some(ResolvedRangeMode::InclusiveIncrement)
        );
        assert_eq!(
            operation(&["stats", "sum"], "row").resolved_reduction_mode(),
            Some(ResolvedReductionMode::Rows)
        );
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SlotRole {
    Input,
    State,
    Derived,
    Output,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProducerReference {
    Input(InputId),
    NodeOutput {
        node: NodeId,
        output_ordinal: u16,
    },
    Output {
        output: OutputId,
        source: ArtifactSource,
    },
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

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
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
    pub requirement: Option<ApplicationRequirementId>,
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
    /// Explicit interactive identity and storage mapping. Ordinary artifact
    /// interfaces leave this unset, even when their canonical name happens to
    /// resemble the compiler's collision-safe transport encoding.
    pub interactive_binding: Option<InteractiveSymbolBinding>,
    pub source: CellSlotId,
    pub schema: SchemaId,
}

/// First-class identity for a lexical symbol exported to an interactive host.
///
/// `lexical_name` is never constrained by artifact-interface syntax.
/// `artifact_source` identifies the semantic producer, while `storage` is the
/// live resident cell used to inspect it. Several lexical names may therefore
/// share the same source and storage without sharing an interface name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InteractiveSymbolBinding {
    pub lexical_name: String,
    pub artifact_source: ArtifactSource,
    pub storage: CellSlotId,
    pub output: OutputId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntegrityConstraintDeclaration {
    pub constraint: IntegrityConstraintId,
    pub name: String,
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
    requirements: super::ApplicationRequirementTable,
    inputs: Box<[InputDeclaration]>,
    slots: Box<[SlotDeclaration]>,
    nodes: Box<[NodeDeclaration]>,
    bindings: Box<[BindingDeclaration]>,
    outputs: Box<[OutputDeclaration]>,
    constraints: Box<[IntegrityConstraintDeclaration]>,
    compute_regions: Box<[ComputeRegionDeclaration]>,
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

    pub const fn requirements(&self) -> &super::ApplicationRequirementTable {
        &self.requirements
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

    pub fn interactive_symbol_bindings(&self) -> impl Iterator<Item = &InteractiveSymbolBinding> {
        self.outputs
            .iter()
            .filter_map(|output| output.interactive_binding.as_ref())
    }

    pub const fn constraints(&self) -> &[IntegrityConstraintDeclaration] {
        &self.constraints
    }

    pub const fn compute_regions(&self) -> &[ComputeRegionDeclaration] {
        &self.compute_regions
    }

    #[cfg(feature = "semantic-compiler")]
    pub(crate) fn with_compute_regions(
        self,
        compute_regions: Box<[ComputeRegionDeclaration]>,
    ) -> Result<Self, ArtifactBuildError> {
        ProgramArtifactDraft {
            schemas: self.schemas,
            constants: self.constants,
            contracts: self.contracts,
            requirements: self.requirements,
            inputs: self.inputs,
            slots: self.slots,
            nodes: self.nodes,
            bindings: self.bindings,
            outputs: self.outputs,
            constraints: self.constraints,
            compute_regions,
        }
        .finalize()
    }
}

#[derive(Clone, Debug)]
pub struct ProgramArtifactDraft {
    pub schemas: SchemaTable,
    pub constants: ConstantStore,
    pub contracts: OperationContractTable,
    pub requirements: super::ApplicationRequirementTable,
    pub inputs: Box<[InputDeclaration]>,
    pub slots: Box<[SlotDeclaration]>,
    pub nodes: Box<[NodeDeclaration]>,
    pub bindings: Box<[BindingDeclaration]>,
    pub outputs: Box<[OutputDeclaration]>,
    pub constraints: Box<[IntegrityConstraintDeclaration]>,
    pub compute_regions: Box<[ComputeRegionDeclaration]>,
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
            requirements: self.requirements,
            inputs: self.inputs,
            slots: self.slots,
            nodes: self.nodes,
            bindings: self.bindings,
            outputs: self.outputs,
            constraints: self.constraints,
            compute_regions: self.compute_regions,
        })
    }
}

impl ProgramArtifactDraft {
    pub(super) fn attach_contracts(
        mut self,
        declarations: &[&OperationContractDeclaration],
    ) -> Result<Self, ArtifactBuildError> {
        if declarations.len() != self.nodes.len() {
            return Err(ArtifactBuildError::CompiledMetadataLengthMismatch {
                table: "node_contracts",
                expected: self.nodes.len(),
                actual: declarations.len(),
            });
        }
        let mut builder = OperationContractTableBuilder::new();
        let mut node_handles = Vec::with_capacity(self.nodes.len());
        for (node, declaration) in self.nodes.iter().zip(declarations) {
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
            validate_declaration(declaration)?;
            let policies = declaration.inputs.resolve(inputs.len())?;
            if declaration.outputs.len() != outputs.len() {
                return Err(OperationContractError::PortCountMismatch {
                    direction: PortDirection::Output,
                    expected: declaration.outputs.len() as u64,
                    actual: outputs.len() as u64,
                }
                .into());
            }
            let contract = ResolvedOperationContract::Declared(DeclaredOperationContract {
                inputs: inputs
                    .into_iter()
                    .zip(policies)
                    .map(|(schema, policy)| ResolvedInputPort {
                        schema,
                        access: policy.access,
                        delivery: policy.delivery,
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
                outputs: outputs
                    .into_iter()
                    .zip(declaration.outputs.iter())
                    .map(|(schema, policy)| ResolvedOutputPort {
                        schema,
                        access: policy.access,
                        delivery: policy.delivery,
                        construction: policy.construction.clone(),
                        alias: policy.alias,
                        change_detection: policy.change_detection,
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
                interaction: declaration.interaction.clone(),
            });
            node_handles.push(builder.insert(contract)?);
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
    CompiledTypeBindingMismatch {
        instruction: u32,
        reason: String,
    },
    CompiledRegisterDescriptorMismatch {
        register: u32,
        reason: String,
    },
    MissingOperationContract {
        node: NodeId,
        operation: OperationReference,
    },
    MatrixLiteralMetadataMismatch {
        output: u32,
        reason: &'static str,
    },
    UnresolvedEmptyRegister {
        register: u32,
    },
    InvalidDeclarationMarker {
        instruction: u32,
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
    MissingSemanticOperation {
        instruction: u32,
        implementation: String,
    },
    UnknownApplicationRequirement {
        requirement: u32,
    },
    ApplicationRequirementKindMismatch {
        requirement: u32,
        expected: &'static str,
    },
    NonCanonicalApplicationRequirementTable,
    MissingApplicationRequirement {
        node: NodeId,
    },
    UnexpectedApplicationRequirement {
        node: NodeId,
    },
    ApplicationRequirementInteractionMismatch {
        node: NodeId,
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
    DeclaredSourceNodeLoweringUnsupported {
        source_node: u32,
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
    InvalidInteractiveBinding {
        name: String,
    },
    InvalidComputeRegionName {
        region: ComputeRegionId,
    },
    DuplicateComputeRegionName {
        name: Box<str>,
    },
    EmptyComputeRegion {
        region: ComputeRegionId,
    },
    NonCanonicalComputeRegionNodes {
        region: ComputeRegionId,
    },
    DuplicateComputeRegionNode {
        node: NodeId,
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
    InvalidStateWriterChain {
        slot: CellSlotId,
        reason: &'static str,
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
    CoreBytecode(MechError),
    OperationContract(OperationContractError),
    CompilerIr(CompilerIrError),
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

impl From<CompilerIrError> for ArtifactBuildError {
    fn from(error: CompilerIrError) -> Self {
        Self::CompilerIr(error)
    }
}
