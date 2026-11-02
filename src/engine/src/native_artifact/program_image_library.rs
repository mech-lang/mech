//! Private assembly of the redundant outer template from its actual selection.
use crate::native_artifact::{
    NativeCollectionLexicalTemplate, NativeCollectionOperationTemplate,
    NativeProgramLibraryTemplate,
};
use mech_core::{FunctionCatalog, NodeId, SchemaTable};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
struct Origin {
    node: NodeId,
    block: Option<u32>,
    local: Option<u32>,
}
/// # Safety
/// Keep the returned template confined until the full image has unchanged
/// compiler/Object provenance. No source-site proof is inferred from JSON.
pub(crate) unsafe fn template(
    image: &Value,
    catalog: &FunctionCatalog,
    schemas: &SchemaTable,
) -> Result<NativeProgramLibraryTemplate, String> {
    const FIELDS: [&str; 7] = [
        "origin",
        "revision",
        "inputs",
        "output",
        "contract",
        "selected_key",
        "selected",
    ];
    let outer = image
        .as_object()
        .ok_or("template image must be an object")?;
    if outer.len() != FIELDS.len() || FIELDS.iter().any(|key| !outer.contains_key(*key)) {
        return Err("template image differs from its canonical wire structure".into());
    }
    let origin: Origin = super::canonical_json::decode(&outer["origin"])?;
    let selected = outer["selected"]
        .as_object()
        .ok_or("missing selected image tag")?;
    if selected.len() != 1 {
        return Err("selected image must have exactly one tag".into());
    }
    let (tag, selected_image) = selected.iter().next().unwrap();
    let leaf = selected_image
        .get("leaf")
        .ok_or("missing selected image leaf")?;
    // Borrow redundant wire fields for comparison. The selected reader still
    // finalizes schemas, validates the contract and checks the actual factory.
    // Deserializing another owned JSON tree or contract here adds no check.
    if ["inputs", "output", "contract", "selected_key"]
        .iter()
        .any(|key| leaf.get(*key) != Some(&outer[*key]))
        || selected_image.get("revision") != Some(&outer["revision"])
    {
        return Err("outer image metadata differs from selected source metadata".into());
    }
    let template = match tag.as_str() {
        "Root" => {
            // SAFETY: delegated unchanged build-image confinement obligation.
            NativeProgramLibraryTemplate::from_root(unsafe {
                NativeCollectionOperationTemplate::from_build_image(
                    selected_image,
                    catalog,
                    schemas,
                )?
            })
        }
        "Lexical" => {
            // SAFETY: delegated unchanged build-image confinement obligation.
            NativeProgramLibraryTemplate::from_lexical(unsafe {
                NativeCollectionLexicalTemplate::from_build_image(selected_image, catalog, schemas)?
            })
        }
        _ => return Err("unknown selected image tag".into()),
    };
    let expected = template.origin();
    if origin.node != expected.node
        || origin.block != expected.block
        || origin.local != expected.local
    {
        return Err("outer image template differs from selected source metadata".into());
    }
    Ok(template)
}
