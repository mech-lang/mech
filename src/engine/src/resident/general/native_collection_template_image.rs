//! Build-owned template reconstruction without source-artifact traversal.
#![allow(dead_code)] // Joined to the full private descriptor reader incrementally.
use super::*;
use crate::native_artifact::program_image_read::values::schema;
use serde::Deserialize;
use serde_json::Value as Json;

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
        || serde_json::to_value(memory).map_err(|e| e.to_string())? != wire.implementation_memory
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
        let wire = Root::deserialize(image).map_err(|e| e.to_string())?;
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
        let wire = Lexical::deserialize(image).map_err(|e| e.to_string())?;
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
