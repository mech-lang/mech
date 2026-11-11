//! One checked immutable definition per compiler-emitted library signature.
//! Source headers remain separate; no invocation state or authority is shared.
use super::*;
use crate::native_artifact::{BuildOwnedImageRecord, NativeProgramLibraryTemplate};
use serde::Deserialize;

#[derive(Debug)]
pub(crate) struct NativeCollectionTemplateTable {
    leaves: Box<[SelectedCollectionLeaf]>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Root {
    revision: ProgramRevision,
    node: NodeId,
    target: CellSlotId,
    sources: Box<[image::Source]>,
    leaf: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Lexical {
    revision: ProgramRevision,
    owner: NodeId,
    block: u32,
    local: u32,
    sources: Box<[image::LexicalSource]>,
    leaf: u32,
}

#[derive(Deserialize)]
enum Template {
    Root(Root),
    Lexical(Lexical),
}

impl NativeCollectionTemplateTable {
    /// # Safety
    /// Confine every result until the enclosing loader has established the
    /// unchanged checked Object, schemas and source-plan association. The same
    /// canonical-schema, contract, actual factory and memory-witness validator
    /// used for standalone templates checks every table definition here.
    pub(crate) unsafe fn from_build_record(
        record: BuildOwnedImageRecord<'_>,
        catalog: &FunctionCatalog,
        schemas: &mech_core::SchemaTable,
    ) -> Result<Self, String> {
        let records: Box<[BuildOwnedImageRecord<'_>]> =
            record.decode().map_err(|error| error.to_string())?;
        let leaves = records
            .into_vec()
            .into_iter()
            .map(|record| {
                image::leaf(
                    crate::native_artifact::program_image_read::canonical_record(record)?,
                    catalog,
                    schemas,
                )
            })
            .collect::<Result<Box<[_]>, String>>()?;
        Ok(Self { leaves })
    }

    fn leaf(&self, index: u32, sources: usize) -> Result<SelectedCollectionLeaf, String> {
        let leaf = self
            .leaves
            .get(index as usize)
            .ok_or("template definition is outside its compiler table")?;
        if sources != leaf.metadata.inputs.len() {
            return Err("image source count differs".into());
        }
        // Arc owns an ordinary validated definition. Site headers and operand
        // identities are never interned; prepared caches remain independently
        // owned, with their original finite work and memory admission.
        Ok(leaf.clone())
    }

    /// # Safety
    /// The exact checked-build confinement and source-site duties of
    /// `from_build_record` apply. A table index is not a source-selection proof.
    pub(crate) unsafe fn template_record(
        &self,
        record: BuildOwnedImageRecord<'_>,
    ) -> Result<NativeProgramLibraryTemplate, String> {
        let wire: Template = crate::native_artifact::program_image_read::canonical_record(record)?;
        Ok(match wire {
            Template::Root(wire) => {
                let leaf = self.leaf(wire.leaf, wire.sources.len())?;
                NativeProgramLibraryTemplate::from_root(NativeCollectionOperationTemplate {
                    revision: wire.revision,
                    node: wire.node,
                    target: wire.target,
                    sources: wire
                        .sources
                        .into_vec()
                        .into_iter()
                        .map(image::Source::source)
                        .collect(),
                    leaf,
                })
            }
            Template::Lexical(wire) => {
                let leaf = self.leaf(wire.leaf, wire.sources.len())?;
                NativeProgramLibraryTemplate::from_lexical(NativeCollectionLexicalTemplate {
                    revision: wire.revision,
                    owner: wire.owner,
                    block: crate::ControlBlockId(wire.block),
                    local: wire.local,
                    sources: wire
                        .sources
                        .into_vec()
                        .into_iter()
                        .map(image::LexicalSource::source)
                        .collect(),
                    leaf,
                })
            }
        })
    }
}
