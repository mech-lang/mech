//! Private assembly of the redundant outer template from its actual selection.
#![allow(dead_code)]
use crate::native_artifact::{
    NativeCollectionLexicalTemplate, NativeCollectionOperationTemplate,
    NativeProgramLibraryTemplate,
};
use mech_core::{FunctionCatalog, SchemaTable};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
enum Selected {
    Root(Value),
    Lexical(Value),
}

/// # Safety
/// Keep the returned template confined until the full image has unchanged
/// compiler/Object provenance. No source-site proof is inferred from JSON.
pub(crate) unsafe fn template(
    image: &Value,
    catalog: &FunctionCatalog,
    schemas: &SchemaTable,
) -> Result<NativeProgramLibraryTemplate, String> {
    let selected = Selected::deserialize(image.get("selected").ok_or("missing image selection")?)
        .map_err(|e| e.to_string())?;
    let template = match selected {
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
    if serde_json::to_value(&template).map_err(|e| e.to_string())? != *image {
        return Err("outer image template differs from selected source metadata".into());
    }
    Ok(template)
}
