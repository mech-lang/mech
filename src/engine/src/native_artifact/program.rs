//! Mixed canonical values in the existing native control lowering.
//! This is a constructor-private compiler view of ProgramArtifact, not another
//! serialized program, interpreter, scheduler or execution authority.
use super::{
    NativeArtifactLoweringError, NativeControlLoop, NativeIterationBlock,
    NativeIterationValueScope, NativeMatchValue, NativeProgramCollection, NativeScalarKind,
    NativeScalarType,
};
use mech_core::{
    ApplicationRequirementId, CellSlotId, ExecutionResourceRequest, InputId, NodeId, OutputId,
    ProgramRevision, Schema, SchemaId, SchemaKey, Value,
};

/// Exact symbolic type. Managed dimensions are deliberately not replaced by
/// the physical one-handle transport shape; a call must bind its current shape.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct NativeProgramType {
    pub(crate) schema: SchemaId,
    pub(crate) definition: Schema,
    pub(crate) scalar: Option<NativeScalarType>,
}
impl NativeProgramType {
    pub const fn schema(&self) -> SchemaId {
        self.schema
    }
    pub fn schema_key(&self) -> SchemaKey {
        self.definition.key()
    }
    pub fn definition(&self) -> &Schema {
        &self.definition
    }
    pub fn scalar(&self) -> Option<&NativeScalarType> {
        self.scalar.as_ref()
    }
    pub fn scalar_kind(&self) -> Option<NativeScalarKind> {
        self.scalar().map(|ty| ty.kind())
    }
    pub(crate) fn into_scalar(self) -> Result<NativeScalarType, NativeArtifactLoweringError> {
        self.scalar
            .ok_or(NativeArtifactLoweringError::UnsupportedSchema {
                schema: self.schema,
            })
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct NativeProgramInput {
    pub(crate) input: InputId,
    pub(crate) slot: CellSlotId,
    pub(crate) name: String,
    pub(crate) ty: NativeProgramType,
    pub(crate) value: NativeMatchValue,
}
impl NativeProgramInput {
    pub const fn input(&self) -> InputId {
        self.input
    }
    pub const fn slot(&self) -> CellSlotId {
        self.slot
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn ty(&self) -> &NativeProgramType {
        &self.ty
    }
    pub const fn value(&self) -> NativeMatchValue {
        self.value
    }
}

/// An original root observation captured by the authorized host BEFORE native
/// execution. This is transport metadata, never a native I/O callback or grant.
#[derive(Clone, Debug, serde::Serialize)]
pub struct NativeProgramObservation {
    pub(crate) node: NodeId,
    pub(crate) requirement: ApplicationRequirementId,
    pub(crate) request: ExecutionResourceRequest,
    pub(crate) slot: CellSlotId,
    pub(crate) ty: NativeProgramType,
    pub(crate) value: NativeMatchValue,
    pub(crate) ordinal: u32,
}
impl NativeProgramObservation {
    pub const fn node(&self) -> NodeId {
        self.node
    }
    pub const fn requirement(&self) -> ApplicationRequirementId {
        self.requirement
    }
    pub fn request(&self) -> &ExecutionResourceRequest {
        &self.request
    }
    pub const fn slot(&self) -> CellSlotId {
        self.slot
    }
    pub fn ty(&self) -> &NativeProgramType {
        &self.ty
    }
    pub const fn value(&self) -> NativeMatchValue {
        self.value
    }
    pub const fn ordinal(&self) -> u32 {
        self.ordinal
    }
}

/// An immutable payload selected at the original root Send site. Completion
/// transports it to the authorized host; emitted computation performs no I/O.
#[derive(Clone, Debug, serde::Serialize)]
pub struct NativeProgramEffect {
    pub(crate) node: NodeId,
    pub(crate) requirement: ApplicationRequirementId,
    pub(crate) request: ExecutionResourceRequest,
    pub(crate) payload: crate::ArtifactSource,
    pub(crate) ty: NativeProgramType,
    pub(crate) value: NativeMatchValue,
    pub(crate) ordinal: u32,
}
impl NativeProgramEffect {
    pub const fn node(&self) -> NodeId {
        self.node
    }
    pub const fn requirement(&self) -> ApplicationRequirementId {
        self.requirement
    }
    pub fn request(&self) -> &ExecutionResourceRequest {
        &self.request
    }
    pub const fn payload(&self) -> crate::ArtifactSource {
        self.payload
    }
    pub fn ty(&self) -> &NativeProgramType {
        &self.ty
    }
    pub const fn value(&self) -> NativeMatchValue {
        self.value
    }
    pub const fn ordinal(&self) -> u32 {
        self.ordinal
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct NativeProgramConstant {
    pub(crate) value: NativeMatchValue,
    #[serde(serialize_with = "super::program_image::serialize_constant")]
    pub(crate) constant: Value,
}
impl NativeProgramConstant {
    pub const fn value(&self) -> NativeMatchValue {
        self.value
    }
    pub fn constant(&self) -> &Value {
        &self.constant
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct NativeProgramOutput {
    pub(crate) output: OutputId,
    pub(crate) slot: CellSlotId,
    pub(crate) name: String,
    pub(crate) interactive_binding: Option<crate::InteractiveSymbolBinding>,
    pub(crate) value: NativeMatchValue,
    pub(crate) ty: NativeProgramType,
}
impl NativeProgramOutput {
    pub const fn output(&self) -> OutputId {
        self.output
    }
    pub const fn slot(&self) -> CellSlotId {
        self.slot
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn interactive_binding(&self) -> Option<&crate::InteractiveSymbolBinding> {
        self.interactive_binding.as_ref()
    }
    pub const fn value(&self) -> NativeMatchValue {
        self.value
    }
    pub fn ty(&self) -> &NativeProgramType {
        &self.ty
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeProgramLibraryTemplate {
    // A selected leaf already owns this immutable metadata. Keep only that
    // owner in native-enabled builds instead of deep-cloning a second copy.
    // The featureless compiler representation has no selected leaf variant.
    #[cfg(not(any(feature = "pure_text", feature = "map_ops")))]
    origin: super::NativeMatchOrigin,
    #[cfg(not(any(feature = "pure_text", feature = "map_ops")))]
    revision: ProgramRevision,
    #[cfg(not(any(feature = "pure_text", feature = "map_ops")))]
    inputs: Box<[Schema]>,
    #[cfg(not(any(feature = "pure_text", feature = "map_ops")))]
    output: Schema,
    #[cfg(not(any(feature = "pure_text", feature = "map_ops")))]
    contract: mech_core::ResolvedOperationContract,
    #[cfg(not(any(feature = "pure_text", feature = "map_ops")))]
    selected_key: mech_core::ResidentOperationKey,
    #[cfg(any(feature = "pure_text", feature = "map_ops"))]
    selected: SelectedLibrary,
}
#[cfg(any(feature = "pure_text", feature = "map_ops"))]
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
enum SelectedLibrary {
    Root(super::NativeCollectionOperationTemplate),
    Lexical(super::NativeCollectionLexicalTemplate),
}
impl serde::Serialize for NativeProgramLibraryTemplate {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let fields = if cfg!(any(feature = "pure_text", feature = "map_ops")) {
            7
        } else {
            6
        };
        // Preserve the existing canonical wire fields AND their order. Removing
        // duplicate in-memory owners must not change a plan/Object identity.
        let mut record = serializer.serialize_struct("NativeProgramLibraryTemplate", fields)?;
        record.serialize_field("origin", &self.origin())?;
        record.serialize_field("revision", &self.program_revision())?;
        record.serialize_field("inputs", self.inputs())?;
        record.serialize_field("output", self.output())?;
        record.serialize_field("contract", self.contract())?;
        record.serialize_field("selected_key", self.selected_key())?;
        #[cfg(any(feature = "pure_text", feature = "map_ops"))]
        record.serialize_field("selected", &self.selected)?;
        record.end()
    }
}
impl NativeProgramLibraryTemplate {
    pub fn inputs(&self) -> &[Schema] {
        #[cfg(any(feature = "pure_text", feature = "map_ops"))]
        {
            match &self.selected {
                SelectedLibrary::Root(t) => t.inputs(),
                SelectedLibrary::Lexical(t) => t.inputs(),
            }
        }
        #[cfg(not(any(feature = "pure_text", feature = "map_ops")))]
        {
            &self.inputs
        }
    }
    pub fn output(&self) -> &Schema {
        #[cfg(any(feature = "pure_text", feature = "map_ops"))]
        {
            match &self.selected {
                SelectedLibrary::Root(t) => t.output(),
                SelectedLibrary::Lexical(t) => t.output(),
            }
        }
        #[cfg(not(any(feature = "pure_text", feature = "map_ops")))]
        {
            &self.output
        }
    }
    pub fn contract(&self) -> &mech_core::ResolvedOperationContract {
        #[cfg(any(feature = "pure_text", feature = "map_ops"))]
        {
            match &self.selected {
                SelectedLibrary::Root(t) => t.contract(),
                SelectedLibrary::Lexical(t) => t.contract(),
            }
        }
        #[cfg(not(any(feature = "pure_text", feature = "map_ops")))]
        {
            &self.contract
        }
    }
    pub fn selected_key(&self) -> &mech_core::ResidentOperationKey {
        #[cfg(any(feature = "pure_text", feature = "map_ops"))]
        {
            match &self.selected {
                SelectedLibrary::Root(t) => t.selected_key(),
                SelectedLibrary::Lexical(t) => t.selected_key(),
            }
        }
        #[cfg(not(any(feature = "pure_text", feature = "map_ops")))]
        {
            &self.selected_key
        }
    }
    pub const fn program_revision(&self) -> ProgramRevision {
        #[cfg(any(feature = "pure_text", feature = "map_ops"))]
        {
            match &self.selected {
                SelectedLibrary::Root(t) => t.program_revision(),
                SelectedLibrary::Lexical(t) => t.program_revision(),
            }
        }
        #[cfg(not(any(feature = "pure_text", feature = "map_ops")))]
        {
            self.revision
        }
    }
    pub const fn origin(&self) -> super::NativeMatchOrigin {
        #[cfg(any(feature = "pure_text", feature = "map_ops"))]
        {
            match &self.selected {
                SelectedLibrary::Root(t) => super::NativeMatchOrigin {
                    node: t.node(),
                    block: None,
                    local: None,
                },
                SelectedLibrary::Lexical(t) => super::NativeMatchOrigin {
                    node: t.owner(),
                    block: Some(t.block().0),
                    local: Some(t.local()),
                },
            }
        }
        #[cfg(not(any(feature = "pure_text", feature = "map_ops")))]
        {
            self.origin
        }
    }
    #[cfg(any(feature = "pure_text", feature = "map_ops"))]
    pub fn primitive(&self) -> super::NativeCollectionPrimitive {
        match &self.selected {
            SelectedLibrary::Root(t) => t.primitive(),
            SelectedLibrary::Lexical(t) => t.primitive(),
        }
    }
    #[cfg(any(feature = "pure_text", feature = "map_ops"))]
    pub fn implementation_memory(&self) -> mech_core::ImplementationMemoryClass {
        match &self.selected {
            SelectedLibrary::Root(t) => t.implementation_memory(),
            SelectedLibrary::Lexical(t) => t.implementation_memory(),
        }
    }
    #[cfg(any(feature = "pure_text", feature = "map_ops"))]
    pub fn root(&self) -> Option<&super::NativeCollectionOperationTemplate> {
        match &self.selected {
            SelectedLibrary::Root(t) => Some(t),
            SelectedLibrary::Lexical(_) => None,
        }
    }
    #[cfg(any(feature = "pure_text", feature = "map_ops"))]
    pub fn lexical(&self) -> Option<&super::NativeCollectionLexicalTemplate> {
        match &self.selected {
            SelectedLibrary::Root(_) => None,
            SelectedLibrary::Lexical(t) => Some(t),
        }
    }
    #[cfg(any(feature = "pure_text", feature = "map_ops"))]
    pub(crate) fn from_root(t: super::NativeCollectionOperationTemplate) -> Self {
        Self {
            selected: SelectedLibrary::Root(t),
        }
    }
    #[cfg(any(feature = "pure_text", feature = "map_ops"))]
    pub(crate) fn from_lexical(t: super::NativeCollectionLexicalTemplate) -> Self {
        Self {
            selected: SelectedLibrary::Lexical(t),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct NativeProgramArtifactPlan {
    pub(crate) revision: ProgramRevision,
    pub(crate) resource_requirements: Box<[ExecutionResourceRequest]>,
    pub(crate) observations: Box<[NativeProgramObservation]>,
    pub(crate) effects: Box<[NativeProgramEffect]>,
    pub(crate) inputs: Box<[NativeProgramInput]>,
    pub(crate) constants: Box<[NativeProgramConstant]>,
    pub(crate) values: Box<[NativeProgramType]>,
    pub(crate) value_scopes: Box<[NativeIterationValueScope]>,
    pub(crate) blocks: Box<[NativeIterationBlock]>,
    pub(crate) iterations: Box<[NativeControlLoop]>,
    pub(crate) collections: Box<[NativeProgramCollection]>,
    pub(crate) outputs: Box<[NativeProgramOutput]>,
    pub(crate) entry: u32,
    // Derived from the SAME resident classification/schedule, not host input.
    pub(crate) activation_nodes: Box<[NodeId]>,
    pub(crate) activation_boundary: Option<u32>,
}
impl NativeProgramArtifactPlan {
    pub const fn program_revision(&self) -> ProgramRevision {
        self.revision
    }
    pub fn inputs(&self) -> &[NativeProgramInput] {
        &self.inputs
    }
    /// ALL original requirements, including declarations without a live site.
    /// Native transport selection never prunes an authorization obligation.
    pub fn resource_requirements(&self) -> &[ExecutionResourceRequest] {
        &self.resource_requirements
    }
    pub fn observations(&self) -> &[NativeProgramObservation] {
        &self.observations
    }
    pub fn effects(&self) -> &[NativeProgramEffect] {
        &self.effects
    }
    /// Ordinary program inputs keep their real InputIds. Captured observation
    /// transport follows them without synthesizing application inputs.
    pub fn transport_input_count(&self) -> usize {
        self.inputs.len() + self.observations.len()
    }
    pub fn transport_input_value(&self, index: usize) -> NativeMatchValue {
        if let Some(input) = self.inputs.get(index) {
            input.value()
        } else {
            self.observations[index - self.inputs.len()].value()
        }
    }
    pub fn transport_input_type(&self, index: usize) -> &NativeProgramType {
        if let Some(input) = self.inputs.get(index) {
            input.ty()
        } else {
            self.observations[index - self.inputs.len()].ty()
        }
    }
    /// Ordinary outputs keep their real OutputIds. Deferred effect payloads
    /// follow them only in the host transport, not as fake application outputs.
    pub fn transport_output_count(&self) -> usize {
        self.outputs.len() + self.effects.len()
    }
    pub fn transport_output_value(&self, index: usize) -> NativeMatchValue {
        if let Some(output) = self.outputs.get(index) {
            output.value()
        } else {
            self.effects[index - self.outputs.len()].value()
        }
    }
    pub fn transport_output_type(&self, index: usize) -> &NativeProgramType {
        if let Some(output) = self.outputs.get(index) {
            output.ty()
        } else {
            self.effects[index - self.outputs.len()].ty()
        }
    }
    pub fn constants(&self) -> &[NativeProgramConstant] {
        &self.constants
    }
    pub fn values(&self) -> &[NativeProgramType] {
        &self.values
    }
    pub fn value_scopes(&self) -> &[NativeIterationValueScope] {
        &self.value_scopes
    }
    pub fn blocks(&self) -> &[NativeIterationBlock] {
        &self.blocks
    }
    pub fn iterations(&self) -> &[NativeControlLoop] {
        &self.iterations
    }
    pub fn collections(&self) -> &[NativeProgramCollection] {
        &self.collections
    }
    pub fn outputs(&self) -> &[NativeProgramOutput] {
        &self.outputs
    }
    pub const fn entry(&self) -> u32 {
        self.entry
    }
    /// Closed computation owners from the authoritative resident classifier.
    pub fn activation_nodes(&self) -> &[NodeId] {
        &self.activation_nodes
    }
    pub fn has_activation(&self) -> bool {
        !self.activation_nodes.is_empty()
    }
    /// The one root-scope checkpoint, before its first fresh-turn visit.
    /// Scalar compiler views deliberately have no phase boundary.
    pub const fn activation_boundary(&self) -> Option<u32> {
        self.activation_boundary
    }
    pub fn validate_current_visits(&self) -> Result<(), &'static str> {
        super::range_cursor::validate(self)?;
        super::phase_validation::validate(self)?;
        super::iteration_validation::validate(self)
    }
}
