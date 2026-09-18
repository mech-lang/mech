use mech_core::MechSourceCode;

#[cfg(feature = "source")]
use crate::SourceDocument;
use crate::{
    CapabilityRequest, ModuleId, ModuleScopeMetadata, ModuleVersionId, SourceAddressReference,
    SourceContextDeclaration, SourceExportDeclaration, SourceImportDeclaration, SourceKind,
};

#[derive(Clone, Debug)]
pub struct RuntimeModuleRecord {
    pub module_id: ModuleId,
    pub module_version: ModuleVersionId,
    pub name: String,
    pub canonical_uri: String,
    pub kind: SourceKind,
    pub source: MechSourceCode,
    #[cfg(feature = "source")]
    pub source_document: Option<SourceDocument>,
    pub compiler_version: String,
    pub language_edition: String,
    pub target: String,
    pub feature_flags: Vec<String>,
    pub exports: Vec<SourceExportDeclaration>,
    pub imports: Vec<SourceImportDeclaration>,
    pub contexts: Vec<SourceContextDeclaration>,
    pub address_references: Vec<SourceAddressReference>,
    pub scopes: Vec<ModuleScopeMetadata>,
    pub dependency_versions: Vec<ModuleVersionId>,
    pub capability_requirements: Vec<CapabilityRequest>,
    pub capability_requirement_keys: Vec<String>,
}

impl RuntimeModuleRecord {
    pub fn new(
        module_id: ModuleId,
        module_version: ModuleVersionId,
        name: impl Into<String>,
        canonical_uri: impl Into<String>,
        kind: SourceKind,
        source: MechSourceCode,
        #[cfg(feature = "source")] source_document: Option<SourceDocument>,
        compiler_version: impl Into<String>,
        language_edition: impl Into<String>,
        target: impl Into<String>,
        feature_flags: Vec<String>,
        exports: Vec<SourceExportDeclaration>,
        imports: Vec<SourceImportDeclaration>,
        contexts: Vec<SourceContextDeclaration>,
        address_references: Vec<SourceAddressReference>,
        scopes: Vec<ModuleScopeMetadata>,
        dependency_versions: Vec<ModuleVersionId>,
        capability_requirements: Vec<CapabilityRequest>,
        capability_requirement_keys: Vec<String>,
    ) -> Self {
        Self {
            module_id,
            module_version,
            name: name.into(),
            canonical_uri: canonical_uri.into(),
            kind,
            source,
            #[cfg(feature = "source")]
            source_document,
            compiler_version: compiler_version.into(),
            language_edition: language_edition.into(),
            target: target.into(),
            feature_flags,
            exports,
            imports,
            contexts,
            address_references,
            scopes,
            dependency_versions,
            capability_requirements,
            capability_requirement_keys,
        }
    }

    /// Read the retained canonical resolver authority without falling back to
    /// the temporary Program cache carried for the pre-cutover shipping path.
    #[cfg(feature = "source")]
    pub fn canonical_document_index(&self) -> mech_core::MResult<crate::CanonicalDocumentIndex> {
        self.source_document
            .as_ref()
            .ok_or_else(|| {
                mech_core::MechError::new(
                    crate::InvalidResolvedSourceError {
                        field: "source_document",
                        reason: "is required for canonical admission",
                    },
                    None,
                )
            })?
            .index()
            .map_err(|error| mech_core::MechError::new(error, None))
    }
}
