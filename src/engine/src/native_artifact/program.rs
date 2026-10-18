//! Mixed canonical values in the existing native control lowering.
//! This is a constructor-private compiler view of ProgramArtifact, not another
//! serialized program, interpreter, scheduler or execution authority.
use super::{
    NativeArtifactLoweringError, NativeControlLoop, NativeIterationBlock,
    NativeIterationValueScope, NativeMatchValue, NativeProgramCollection, NativeScalarKind,
    NativeScalarType,
};
use mech_core::{
    CellSlotId, InputId, OutputId, ProgramRevision, Schema, SchemaId, SchemaKey, Value,
};

/// Exact symbolic type. Managed dimensions are deliberately not replaced by
/// the physical one-handle transport shape; a call must bind its current shape.
#[derive(Clone, Debug, Eq, PartialEq)]
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

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
pub struct NativeProgramConstant {
    pub(crate) value: NativeMatchValue,
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

#[derive(Clone, Debug)]
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
    origin: super::NativeMatchOrigin,
    revision: ProgramRevision,
    inputs: Box<[Schema]>,
    output: Schema,
    contract: mech_core::ResolvedOperationContract,
    selected_key: mech_core::ResidentOperationKey,
    #[cfg(any(feature = "pure_text", feature = "map_ops"))]
    selected: SelectedLibrary,
}
#[cfg(any(feature = "pure_text", feature = "map_ops"))]
#[derive(Clone, Debug, Eq, PartialEq)]
enum SelectedLibrary {
    Root(super::NativeCollectionOperationTemplate),
    Lexical(super::NativeCollectionLexicalTemplate),
}
impl NativeProgramLibraryTemplate {
    pub fn inputs(&self) -> &[Schema] {
        &self.inputs
    }
    pub fn output(&self) -> &Schema {
        &self.output
    }
    pub fn contract(&self) -> &mech_core::ResolvedOperationContract {
        &self.contract
    }
    pub fn selected_key(&self) -> &mech_core::ResidentOperationKey {
        &self.selected_key
    }
    pub const fn program_revision(&self) -> ProgramRevision {
        self.revision
    }
    pub const fn origin(&self) -> super::NativeMatchOrigin {
        self.origin
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
            origin: super::NativeMatchOrigin {
                node: t.node(),
                block: None,
                local: None,
            },
            revision: t.program_revision(),
            inputs: t.inputs().into(),
            output: t.output().clone(),
            contract: t.contract().clone(),
            selected_key: t.selected_key().clone(),
            selected: SelectedLibrary::Root(t),
        }
    }
    #[cfg(any(feature = "pure_text", feature = "map_ops"))]
    pub(crate) fn from_lexical(t: super::NativeCollectionLexicalTemplate) -> Self {
        Self {
            origin: super::NativeMatchOrigin {
                node: t.owner(),
                block: Some(t.block().0),
                local: Some(t.local()),
            },
            revision: t.program_revision(),
            inputs: t.inputs().into(),
            output: t.output().clone(),
            contract: t.contract().clone(),
            selected_key: t.selected_key().clone(),
            selected: SelectedLibrary::Lexical(t),
        }
    }
}

#[derive(Clone, Debug)]
pub struct NativeProgramArtifactPlan {
    pub(crate) revision: ProgramRevision,
    pub(crate) inputs: Box<[NativeProgramInput]>,
    pub(crate) constants: Box<[NativeProgramConstant]>,
    pub(crate) values: Box<[NativeProgramType]>,
    pub(crate) value_scopes: Box<[NativeIterationValueScope]>,
    pub(crate) blocks: Box<[NativeIterationBlock]>,
    pub(crate) iterations: Box<[NativeControlLoop]>,
    pub(crate) collections: Box<[NativeProgramCollection]>,
    pub(crate) outputs: Box<[NativeProgramOutput]>,
    pub(crate) entry: u32,
}
impl NativeProgramArtifactPlan {
    pub const fn program_revision(&self) -> ProgramRevision {
        self.revision
    }
    pub fn inputs(&self) -> &[NativeProgramInput] {
        &self.inputs
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
    pub fn validate_current_visits(&self) -> Result<(), &'static str> {
        super::iteration_validation::validate(self)
    }
}
