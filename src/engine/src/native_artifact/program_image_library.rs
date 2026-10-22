//! Private assembly of the redundant outer template from its actual selection.
#![allow(dead_code)]
use crate::native_artifact::{
    NativeCollectionLexicalTemplate, NativeCollectionOperationTemplate,
    NativeProgramLibraryTemplate,
};
use mech_core::{FunctionCatalog, NodeId, ProgramRevision, ResolvedOperationContract, SchemaTable};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
enum Selected {
    Root(Value),
    Lexical(Value),
}

#[derive(Deserialize)]
struct Origin {
    node: NodeId,
    block: Option<u32>,
    local: Option<u32>,
}
#[derive(Deserialize)]
struct Key {
    module_path: Box<[String]>,
    operation_name: String,
}
#[derive(Deserialize)]
struct Outer {
    origin: Origin,
    revision: ProgramRevision,
    inputs: Box<[Value]>,
    output: Value,
    contract: ResolvedOperationContract,
    selected_key: Key,
    selected: Selected,
}

/// # Safety
/// Keep the returned template confined until the full image has unchanged
/// compiler/Object provenance. No source-site proof is inferred from JSON.
pub(crate) unsafe fn template(
    image: &Value,
    catalog: &FunctionCatalog,
    schemas: &SchemaTable,
) -> Result<NativeProgramLibraryTemplate, String> {
    let wire: Outer = super::canonical_json::decode(image)?;
    // The selected reader validates these schemas exactly once below. Compare
    // their redundant outer images directly, preserving exact wire equality
    // without re-finalizing or re-serializing each schema a second time.
    let selected_image = match &wire.selected {
        Selected::Root(image) | Selected::Lexical(image) => image,
    };
    let leaf = selected_image
        .get("leaf")
        .ok_or("missing selected image leaf")?;
    if leaf
        .get("inputs")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        != Some(wire.inputs.as_ref())
        || leaf.get("output") != Some(&wire.output)
    {
        return Err("outer image schemas differ from selected source metadata".into());
    }
    let template = match wire.selected {
        Selected::Root(image) => {
            // SAFETY: delegated unchanged build-image confinement obligation.
            NativeProgramLibraryTemplate::from_root(unsafe {
                NativeCollectionOperationTemplate::from_build_image(&image, catalog, schemas)?
            })
        }
        Selected::Lexical(image) => {
            // SAFETY: delegated unchanged build-image confinement obligation.
            NativeProgramLibraryTemplate::from_lexical(unsafe {
                NativeCollectionLexicalTemplate::from_build_image(&image, catalog, schemas)?
            })
        }
    };
    let origin = template.origin();
    if wire.origin.node != origin.node
        || wire.origin.block != origin.block
        || wire.origin.local != origin.local
        || wire.revision != template.program_revision()
        || &wire.contract != template.contract()
        || wire.selected_key.module_path != template.selected_key().module_path
        || wire.selected_key.operation_name != template.selected_key().operation_name
    {
        return Err("outer image template differs from selected source metadata".into());
    }
    Ok(template)
}
