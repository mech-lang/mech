//! Private assembly of the redundant outer template from its actual selection.
use crate::native_artifact::BuildOwnedImageRecord;
use crate::native_artifact::{
    NativeCollectionLexicalTemplate, NativeCollectionOperationTemplate,
    NativeProgramLibraryTemplate,
};
use mech_core::{FunctionCatalog, NodeId, ProgramRevision, ResolvedOperationContract, SchemaTable};
use serde::Deserialize;
use serde_json::Value;

/// One immutable template-definition table, private to this plan restoration.
/// Legacy images retain their full independent template reconstruction path.
pub(super) struct Templates<'a> {
    catalog: &'a FunctionCatalog,
    schemas: &'a SchemaTable,
    shared: Option<
        crate::resident::general::native_collection_templates::NativeCollectionTemplateTable,
    >,
}
impl<'a> Templates<'a> {
    /// # Safety
    /// All unchanged-build confinement and source-site duties of
    /// `template_record` apply to the complete enclosing plan and table.
    pub(super) unsafe fn new(
        shared: Option<BuildOwnedImageRecord<'_>>,
        catalog: &'a FunctionCatalog,
        schemas: &'a SchemaTable,
    ) -> Result<Self, String> {
        let shared = shared.map(|record| unsafe {
            crate::resident::general::native_collection_templates::NativeCollectionTemplateTable::from_build_record(
                record, catalog, schemas,
            )
        }).transpose()?;
        Ok(Self {
            catalog,
            schemas,
            shared,
        })
    }
    /// # Safety
    /// Each record must be the unchanged source header from this exact plan's
    /// checked image. The table confers no visit or invocation authority.
    pub(super) unsafe fn read(
        &self,
        record: BuildOwnedImageRecord<'_>,
    ) -> Result<NativeProgramLibraryTemplate, String> {
        if let Some(table) = &self.shared {
            unsafe { table.template_record(record) }
        } else {
            unsafe { template_record(record, self.catalog, self.schemas) }
        }
    }
}

#[derive(Deserialize)]
struct Origin {
    node: NodeId,
    block: Option<u32>,
    local: Option<u32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename = "mech.native.image.Key")]
struct Key {
    module_path: Box<[String]>,
    operation_name: String,
}
#[derive(Deserialize)]
enum Selected<'a> {
    Root(#[serde(borrow)] BuildOwnedImageRecord<'a>),
    Lexical(#[serde(borrow)] BuildOwnedImageRecord<'a>),
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename = "mech.native.image.Template")]
struct Template<'a> {
    origin: Origin,
    revision: ProgramRevision,
    #[serde(borrow)]
    inputs: Box<[BuildOwnedImageRecord<'a>]>,
    #[serde(borrow)]
    output: BuildOwnedImageRecord<'a>,
    contract: ResolvedOperationContract,
    selected_key: Key,
    #[serde(borrow)]
    selected: Selected<'a>,
}

/// # Safety
/// Identical checked compiler/Object, confinement and source-site obligations
/// to `template`. The selected reader checks the actual maintained factory,
/// its memory class, canonical schema definitions and resolved contract.
pub(crate) unsafe fn template_record(
    image: BuildOwnedImageRecord<'_>,
    catalog: &FunctionCatalog,
    schemas: &SchemaTable,
) -> Result<NativeProgramLibraryTemplate, String> {
    let wire: Template<'_> = super::canonical_record(image)?;
    let template = match wire.selected {
        Selected::Root(image) => NativeProgramLibraryTemplate::from_root(unsafe {
            NativeCollectionOperationTemplate::from_build_record(image, catalog, schemas)?
        }),
        Selected::Lexical(image) => NativeProgramLibraryTemplate::from_lexical(unsafe {
            NativeCollectionLexicalTemplate::from_build_record(image, catalog, schemas)?
        }),
    };
    let origin = template.origin();
    let key = template.selected_key();
    if wire.origin.node != origin.node
        || wire.origin.block != origin.block
        || wire.origin.local != origin.local
        || wire.revision != template.program_revision()
        || wire.selected_key.module_path != key.module_path
        || wire.selected_key.operation_name != key.operation_name
        || wire.contract != *template.contract()
        || wire.inputs.len() != template.inputs().len()
    {
        return Err("outer image metadata differs from selected source metadata".into());
    }
    for (image, definition) in wire.inputs.into_vec().into_iter().zip(template.inputs()) {
        super::values::matching_schema_record(image, definition)?;
    }
    super::values::matching_schema_record(wire.output, template.output())?;
    Ok(template)
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
