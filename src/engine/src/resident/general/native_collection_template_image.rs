//! Build-owned template reconstruction without source-artifact traversal.
use super::*;
use crate::native_artifact::program_image_read::values::schema;
use serde::Deserialize;
use serde_json::Value as Json;

// Only this descriptive build-image witness is wire data. The core memory
// planner and its process-local implementation contracts do not implement serde.
#[derive(serde::Serialize)]
enum MemoryClassImage {
    NoAdditionalScratch,
    CloneInput { input: u16 },
    CanonicalCloneInput { input: u16 },
    AbiContiguousBridge { input: u16, output: u16 },
    ExternalMarshalling,
    MatrixSolve,
    CanonicalFinalize,
    PackedStringMatrixOrCanonicalFinalize,
    CanonicalSortUnique,
}
impl From<ImplementationMemoryClass> for MemoryClassImage {
    fn from(memory: ImplementationMemoryClass) -> Self {
        use ImplementationMemoryClass as M;
        match memory {
            M::NoAdditionalScratch => Self::NoAdditionalScratch,
            M::CloneInput { input } => Self::CloneInput { input },
            M::CanonicalCloneInput { input } => Self::CanonicalCloneInput { input },
            M::AbiContiguousBridge { input, output } => Self::AbiContiguousBridge { input, output },
            M::ExternalMarshalling => Self::ExternalMarshalling,
            M::MatrixSolve => Self::MatrixSolve,
            M::CanonicalFinalize => Self::CanonicalFinalize,
            M::PackedStringMatrixOrCanonicalFinalize => Self::PackedStringMatrixOrCanonicalFinalize,
            M::CanonicalSortUnique => Self::CanonicalSortUnique,
        }
    }
}
pub(super) fn serialize_memory<S: serde::Serializer>(
    memory: &ImplementationMemoryClass,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serde::Serialize::serialize(&MemoryClassImage::from(*memory), serializer)
}

#[cfg(test)]
#[test]
fn memory_class_image_preserves_all_descriptive_tags_and_full_width_fields() {
    use ImplementationMemoryClass as M;
    for (memory, expected) in [
        (
            M::NoAdditionalScratch,
            serde_json::json!("NoAdditionalScratch"),
        ),
        (
            M::CloneInput { input: u16::MAX },
            serde_json::json!({"CloneInput":{"input":65535}}),
        ),
        (
            M::CanonicalCloneInput { input: u16::MAX },
            serde_json::json!({"CanonicalCloneInput":{"input":65535}}),
        ),
        (
            M::AbiContiguousBridge {
                input: u16::MAX,
                output: u16::MAX,
            },
            serde_json::json!({"AbiContiguousBridge":{"input":65535,"output":65535}}),
        ),
        (
            M::ExternalMarshalling,
            serde_json::json!("ExternalMarshalling"),
        ),
        (M::MatrixSolve, serde_json::json!("MatrixSolve")),
        (M::CanonicalFinalize, serde_json::json!("CanonicalFinalize")),
        (
            M::PackedStringMatrixOrCanonicalFinalize,
            serde_json::json!("PackedStringMatrixOrCanonicalFinalize"),
        ),
        (
            M::CanonicalSortUnique,
            serde_json::json!("CanonicalSortUnique"),
        ),
    ] {
        assert_eq!(
            serde_json::to_value(MemoryClassImage::from(memory)).unwrap(),
            expected
        );
    }
}

#[cfg(all(test, feature = "pure_text", feature = "semantic-compiler"))]
#[path = "native_collection_template_image_tests.rs"]
mod tests;

pub(super) fn specialize_primitive(
    primitive: NativeCollectionPrimitive,
    inputs: &[Schema],
) -> NativeCollectionPrimitive {
    use NativeCollectionPrimitive as P;
    match primitive {
        P::CompositePack(_) => P::CompositePack(inputs.len()),
        P::StringScalarAccess => match inputs.first().map(Schema::body) {
            Some(SchemaBody::Matrix { .. }) => P::MatrixScalarAccess,
            Some(SchemaBody::Map { .. }) => P::MapKeyAccess,
            _ => primitive,
        },
        P::StringRangeAccess
            if inputs.len() == 2
                && matches!(
                    inputs.first().map(Schema::body),
                    Some(SchemaBody::Matrix { .. })
                ) =>
        {
            P::MatrixRangeAccess
        }
        P::StringRangeAccess if inputs.len() == 1 => P::StringAllAccess,
        _ => primitive,
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Key {
    module_path: Box<[String]>,
    operation_name: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Leaf {
    selected_key: Key,
    primitive: Json,
    inputs: Box<[Json]>,
    output: Json,
    contract: ResolvedOperationContract,
    implementation_memory: Json,
}
fn leaf(
    wire: Leaf,
    catalog: &FunctionCatalog,
    schemas: &mech_core::SchemaTable,
) -> Result<SelectedCollectionLeaf, String> {
    let key = ResidentOperationKey::new(
        wire.selected_key.module_path,
        wire.selected_key.operation_name,
    )
    .ok_or("invalid image factory key")?;
    let entry = catalog
        .resident_factory(&key.module_path, &key.operation_name)
        .ok_or("image factory is not linked")?;
    let primitive = crate::resident::native_collection_semantics::primitive(entry)
        .ok_or("image factory lacks maintained witness")?;
    let inputs = wire
        .inputs
        .iter()
        .map(schema)
        .collect::<Result<Box<[_]>, _>>()?;
    let output = schema(&wire.output)?;
    let primitive = specialize_primitive(primitive, &inputs);
    let memory = entry.implementation_memory();
    if serde_json::to_value(primitive).map_err(|e| e.to_string())? != wire.primitive
        || serde_json::to_value(MemoryClassImage::from(memory)).map_err(|e| e.to_string())?
            != wire.implementation_memory
    {
        return Err("image factory witness or memory class differs".into());
    }
    mech_core::validate_resolved_contract(&wire.contract)
        .map_err(|e| format!("image contract: {e:?}"))?;
    let contract = match &wire.contract {
        mech_core::ResolvedOperationContract::Declared(contract) => contract,
        _ => return Err("unsupported image operation contract".into()),
    };
    if contract.inputs.len() != inputs.len()
        || contract.outputs.len() != 1
        || contract
            .inputs
            .iter()
            .zip(inputs.iter())
            .any(|(p, s)| schemas.get(p.schema) != Some(s))
        || schemas.get(contract.outputs[0].schema) != Some(&output)
    {
        return Err("image contract schema IDs differ".into());
    }
    Ok(SelectedCollectionLeaf {
        selected_key: key,
        primitive,
        inputs,
        output,
        contract: wire.contract,
        implementation_memory: memory,
    })
}

#[derive(Deserialize)]
enum Source {
    Constant(u32),
    Slot(CellSlotId),
}
impl Source {
    fn source(self) -> ArtifactSource {
        match self {
            Self::Constant(id) => ArtifactSource::Constant(mech_core::ConstantId::new(id)),
            Self::Slot(id) => ArtifactSource::Slot(id),
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Root {
    revision: ProgramRevision,
    node: NodeId,
    target: CellSlotId,
    sources: Box<[Source]>,
    leaf: Leaf,
}
impl NativeCollectionOperationTemplate {
    /// # Safety
    /// Confine this metadata until the enclosing image is tied to the unchanged
    /// compiler-produced Object. Factory checks do not prove source-site identity.
    pub(crate) unsafe fn from_build_image(
        image: &Json,
        catalog: &FunctionCatalog,
        schemas: &mech_core::SchemaTable,
    ) -> Result<Self, String> {
        let wire: Root = crate::native_artifact::program_image_read::canonical_json::decode(image)?;
        let leaf = leaf(wire.leaf, catalog, schemas)?;
        if wire.sources.len() != leaf.inputs.len() {
            return Err("image source count differs".into());
        }
        Ok(Self {
            revision: wire.revision,
            node: wire.node,
            target: wire.target,
            sources: wire
                .sources
                .into_vec()
                .into_iter()
                .map(Source::source)
                .collect(),
            leaf,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
enum Control {
    Constant(u32),
    Parameter { block: u32, ordinal: u16 },
    Local { block: u32, node: u32 },
}
#[derive(Deserialize)]
enum Comprehension {
    Constant(u32),
    Input(u16),
    Local(u32),
}
#[derive(Deserialize)]
enum LexicalSource {
    Control(Control),
    Comprehension(Comprehension),
}
impl LexicalSource {
    fn source(self) -> NativeCollectionLexicalSource {
        use crate::{ComprehensionValue as V, ControlBlockId as B, ControlValue as C};
        match self {
            Self::Control(c) => NativeCollectionLexicalSource::Control(match c {
                Control::Constant(id) => C::Constant(mech_core::ConstantId::new(id)),
                Control::Parameter { block, ordinal } => C::Parameter {
                    block: B(block),
                    ordinal,
                },
                Control::Local { block, node } => C::Local {
                    block: B(block),
                    node,
                },
            }),
            Self::Comprehension(c) => NativeCollectionLexicalSource::Comprehension(match c {
                Comprehension::Constant(id) => V::Constant(mech_core::ConstantId::new(id)),
                Comprehension::Input(id) => V::Input(id),
                Comprehension::Local(id) => V::Local(id),
            }),
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Lexical {
    revision: ProgramRevision,
    owner: NodeId,
    block: u32,
    local: u32,
    sources: Box<[LexicalSource]>,
    leaf: Leaf,
}
impl NativeCollectionLexicalTemplate {
    /// # Safety
    /// The enclosing build-owned loader must establish unchanged Object/image
    /// provenance before this metadata can escape to an execution descriptor.
    pub(crate) unsafe fn from_build_image(
        image: &Json,
        catalog: &FunctionCatalog,
        schemas: &mech_core::SchemaTable,
    ) -> Result<Self, String> {
        let wire: Lexical =
            crate::native_artifact::program_image_read::canonical_json::decode(image)?;
        let leaf = leaf(wire.leaf, catalog, schemas)?;
        if wire.sources.len() != leaf.inputs.len() {
            return Err("image source count differs".into());
        }
        Ok(Self {
            revision: wire.revision,
            owner: wire.owner,
            block: crate::ControlBlockId(wire.block),
            local: wire.local,
            sources: wire
                .sources
                .into_vec()
                .into_iter()
                .map(LexicalSource::source)
                .collect(),
            leaf,
        })
    }
}
