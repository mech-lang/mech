use mech_core::{CanonicalNominalPath, GenericError, MResult, MechError, ProgramRevision};
use mech_engine::{
    CanonicalSourceFrontend, ProgramCompilationProduct, decode_program_artifact_bytecode_v1,
};

use crate::SourceDocument;

pub const CANONICAL_PROGRAM_BUNDLE_VERSION: u32 = 4;

/// Retained dependency text and the defining provenance that participated in
/// nominal compilation. Text-only callers may validate sources without
/// nominal declarations; provenance-bearing dependencies require this form.
pub struct CanonicalDependencySource<'a> {
    pub source: &'a str,
    pub nominal_origin: Option<&'a mech_core::CanonicalNominalPath>,
    pub nominal_package_id: Option<&'a str>,
}

/// Versioned browser handoff that keeps exact retained source and executable
/// artifact identity in one admission unit. It is deliberately incompatible
/// with the retired serialized syntax-tree payload.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalProgramBundle {
    pub version: u32,
    pub canonical_uri: String,
    pub document_id: u64,
    pub source_revision: u64,
    pub source_hash: u64,
    pub source: String,
    /// Present only when the root declares nominal types whose keys depend on it.
    pub root_nominal_origin: Option<CanonicalNominalPath>,
    pub root_nominal_package_id: Option<String>,
    /// Resolved transitive dependency URI -> retained source/provenance hash.
    pub source_dependencies: std::collections::BTreeMap<String, u64>,
    pub artifact_revision: [u8; 32],
    pub bytecode: Vec<u8>,
}

impl CanonicalProgramBundle {
    pub fn from_product(
        canonical_uri: impl Into<String>,
        document: &SourceDocument,
        product: &ProgramCompilationProduct,
    ) -> MResult<Self> {
        let source = document.source().to_contiguous_string();
        let has_nominal_declarations = !CanonicalSourceFrontend
            .declared_enum_names(&document.document())
            .map_err(|error| bundle_error(error.to_string()))?
            .is_empty();
        let root_nominal_origin = has_nominal_declarations
            .then(|| document.nominal_origin().cloned())
            .flatten();
        if has_nominal_declarations && root_nominal_origin.is_none() {
            return Err(bundle_error(
                "canonical bundle root enum has no defining origin",
            ));
        }
        let root_nominal_package_id = has_nominal_declarations
            .then(|| document.nominal_package_id().map(str::to_owned))
            .flatten();
        let artifact_revision = product.artifact().revision();
        let bundle = Self {
            version: CANONICAL_PROGRAM_BUNDLE_VERSION,
            canonical_uri: canonical_uri.into(),
            document_id: document.source().document().0,
            source_revision: document.source().revision().0,
            source_hash: super::compiler::canonical_dependency_identity_hash(
                &source,
                root_nominal_origin.as_ref(),
                root_nominal_package_id.as_deref(),
            ),
            source,
            root_nominal_origin,
            root_nominal_package_id,
            source_dependencies: product.source_dependencies().clone(),
            artifact_revision: artifact_revision.into_bytes(),
            bytecode: product.bytecode().to_vec(),
        };
        bundle.validate_root_with_provenance(
            None,
            document.nominal_origin(),
            document.nominal_package_id(),
        )?;
        Ok(bundle)
    }

    pub fn validate(&self, expected_source: Option<&str>) -> MResult<ProgramRevision> {
        self.validate_root_with_provenance(expected_source, None, None)
    }

    /// Validate the current root text and the provenance used to derive its
    /// nominal keys. Text-only validation remains valid for non-nominal roots.
    pub fn validate_root_with_provenance(
        &self,
        expected_source: Option<&str>,
        nominal_origin: Option<&CanonicalNominalPath>,
        nominal_package_id: Option<&str>,
    ) -> MResult<ProgramRevision> {
        if self.version != CANONICAL_PROGRAM_BUNDLE_VERSION {
            return Err(bundle_error(format!(
                "unsupported canonical program bundle version {}; retired AST and stale bundle payloads must be regenerated",
                self.version
            )));
        }
        if self.canonical_uri.is_empty()
            || self.document_id != mech_core::hash_str(&self.canonical_uri)
            || self.source_hash
                != super::compiler::canonical_dependency_identity_hash(
                    &self.source,
                    self.root_nominal_origin.as_ref(),
                    self.root_nominal_package_id.as_deref(),
                )
        {
            return Err(bundle_error(
                "canonical program bundle source identity is stale or invalid; regenerate the bundle",
            ));
        }
        if self.root_nominal_origin.is_some()
            && (self.root_nominal_origin.as_ref() != nominal_origin
                || self.root_nominal_package_id.as_deref() != nominal_package_id)
        {
            return Err(bundle_error(
                "canonical bundle root nominal provenance changed; regenerate the bundle",
            ));
        }
        if self.root_nominal_origin.is_none() && self.root_nominal_package_id.is_some() {
            return Err(bundle_error(
                "canonical bundle root nominal provenance is invalid",
            ));
        }
        if self
            .source_dependencies
            .keys()
            .any(|uri| uri.is_empty() || *uri == self.canonical_uri)
        {
            return Err(bundle_error(
                "canonical bundle has an invalid dependency identity; regenerate the bundle",
            ));
        }
        if expected_source.is_some_and(|expected| expected != self.source) {
            return Err(bundle_error(
                "canonical program bundle source and served source differ; regenerate the bundle",
            ));
        }
        let artifact = decode_program_artifact_bytecode_v1(&self.bytecode).map_err(|error| {
            bundle_error(format!(
                "canonical program bundle artifact is invalid: {error:?}"
            ))
        })?;
        if artifact.revision().as_bytes() != &self.artifact_revision {
            return Err(bundle_error(
                "canonical program bundle artifact identity is stale or invalid; regenerate the bundle",
            ));
        }
        Ok(artifact.revision())
    }

    /// Reject a root artifact when any source snapshot used during compilation
    /// differs from the dependency text supplied to its loader.
    pub fn validate_dependency_sources<'a>(
        &self,
        mut source: impl FnMut(&str) -> Option<&'a str>,
    ) -> MResult<()> {
        self.validate_dependency_sources_with_provenance(|uri| {
            source(uri).map(|source| CanonicalDependencySource {
                source,
                nominal_origin: None,
                nominal_package_id: None,
            })
        })
    }

    /// Validate dependency text together with its current defining origin and
    /// package owner. A moved package cannot reuse a bundle with old keys.
    pub fn validate_dependency_sources_with_provenance<'a>(
        &self,
        mut source: impl FnMut(&str) -> Option<CanonicalDependencySource<'a>>,
    ) -> MResult<()> {
        for (uri, expected_hash) in &self.source_dependencies {
            let retained = source(uri).ok_or_else(|| {
                bundle_error(format!(
                    "canonical bundle dependency {uri} is missing; regenerate the bundle"
                ))
            })?;
            if mech_core::hash_str(retained.source) != *expected_hash
                && super::compiler::canonical_dependency_identity_hash(
                    retained.source,
                    retained.nominal_origin,
                    retained.nominal_package_id,
                ) != *expected_hash
            {
                return Err(bundle_error(format!(
                    "canonical bundle dependency {uri} differs from the compiled source or nominal provenance; regenerate the bundle"
                )));
            }
        }
        Ok(())
    }

    #[cfg(feature = "serde")]
    pub fn encode(&self) -> MResult<String> {
        mech_core::nodes::compress_and_encode(self)
            .map_err(|error| bundle_error(format!("unable to encode canonical bundle: {error}")))
    }

    #[cfg(feature = "serde")]
    pub fn decode(encoded: &str, expected_source: Option<&str>) -> MResult<Self> {
        let bundle = Self::decode_payload(encoded)?;
        bundle.validate(expected_source)?;
        Ok(bundle)
    }

    #[cfg(feature = "serde")]
    pub fn decode_with_root_provenance(
        encoded: &str,
        expected_source: Option<&str>,
        nominal_origin: Option<&CanonicalNominalPath>,
        nominal_package_id: Option<&str>,
    ) -> MResult<Self> {
        let bundle = Self::decode_payload(encoded)?;
        bundle.validate_root_with_provenance(
            expected_source,
            nominal_origin,
            nominal_package_id,
        )?;
        Ok(bundle)
    }

    #[cfg(feature = "serde")]
    fn decode_payload(encoded: &str) -> MResult<Self> {
        mech_core::nodes::decode_and_decompress(encoded).map_err(|error| {
            bundle_error(format!(
                "invalid canonical program bundle; retired AST payloads must be regenerated: {error}"
            ))
        })
    }
}

fn bundle_error(message: impl Into<String>) -> MechError {
    MechError::new(
        GenericError {
            msg: message.into(),
        },
        None,
    )
    .with_compiler_loc()
}

#[cfg(all(test, feature = "serde", feature = "compiler_default"))]
mod tests {
    use super::{CanonicalDependencySource, CanonicalProgramBundle};
    use crate::SourceDocument;
    use mech_core::{CanonicalNominalPath, MResult};
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use mech_syntax::document::{ParseConfig, Revision};

    #[test]
    fn canonical_bundle_rejects_stale_source_and_retired_tree_payload() -> MResult<()> {
        let source = "answer := 42\n";
        let document = SourceDocument::parse_resolved(
            "bundle://answer.mec",
            Revision(7),
            Arc::<str>::from(source),
            ParseConfig::default(),
        )
        .unwrap();
        let mut compiler = crate::RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_catalog())
            .build_compiler()?;
        let product = compiler.compile_document(&document)?;
        let bundle =
            CanonicalProgramBundle::from_product("bundle://answer.mec", &document, &product)?;
        let encoded = bundle.encode()?;
        assert_eq!(
            CanonicalProgramBundle::decode(&encoded, Some(source))?,
            bundle
        );
        assert!(CanonicalProgramBundle::decode(&encoded, Some("answer := 0\n")).is_err());
        let mut old_envelope = bundle.clone();
        old_envelope.version = 1;
        assert!(old_envelope.validate(Some(source)).is_err());

        let retired_tree_payload = vec!["retired", "syntax", "tree"];
        let legacy = mech_core::nodes::compress_and_encode(&retired_tree_payload).unwrap();
        let error = CanonicalProgramBundle::decode(&legacy, Some(source)).unwrap_err();
        assert!(error.display_message().contains("retired AST"));
        Ok(())
    }

    #[test]
    fn canonical_bundle_dependency_freshness_includes_nominal_provenance() -> MResult<()> {
        let root_source = "answer := 42\n";
        let dependency_source = "<event> := :idle\n";
        let origin =
            CanonicalNominalPath::new(vec!["package-a".to_owned(), "module".to_owned()]).unwrap();
        let changed_origin =
            CanonicalNominalPath::new(vec!["package-a".to_owned(), "other-module".to_owned()])
                .unwrap();
        let document = SourceDocument::parse_resolved(
            "bundle://answer.mec",
            Revision(7),
            Arc::<str>::from(root_source),
            ParseConfig::default(),
        )
        .unwrap();
        let mut compiler = crate::RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_catalog())
            .build_compiler()?;
        let product = compiler
            .compile_document(&document)?
            .with_source_dependencies(BTreeMap::from([(
                "bundle://dep.mec".to_owned(),
                super::super::compiler::canonical_dependency_identity_hash(
                    dependency_source,
                    Some(&origin),
                    Some("package-a"),
                ),
            )]));
        let bundle =
            CanonicalProgramBundle::from_product("bundle://answer.mec", &document, &product)?;
        let retained = |origin, package_id| {
            bundle.validate_dependency_sources_with_provenance(|_| {
                Some(CanonicalDependencySource {
                    source: dependency_source,
                    nominal_origin: origin,
                    nominal_package_id: package_id,
                })
            })
        };
        retained(Some(&origin), Some("package-a"))?;
        assert!(retained(Some(&changed_origin), Some("package-a")).is_err());
        assert!(retained(Some(&origin), Some("package-b")).is_err());
        assert!(
            bundle
                .validate_dependency_sources(|_| Some(dependency_source))
                .is_err()
        );
        Ok(())
    }

    #[test]
    fn canonical_bundle_root_requires_current_nominal_provenance() -> MResult<()> {
        let source = "<event> := :idle | :busy\nvalue<event> := :idle\nvalue\n";
        let origin =
            CanonicalNominalPath::new(vec!["package-a".to_owned(), "module".to_owned()]).unwrap();
        let changed_origin =
            CanonicalNominalPath::new(vec!["package-a".to_owned(), "moved".to_owned()]).unwrap();
        let document = SourceDocument::parse_resolved(
            "bundle://event.mec",
            Revision(7),
            Arc::<str>::from(source),
            ParseConfig::default(),
        )
        .unwrap()
        .with_nominal_origin(origin.clone())
        .with_nominal_package_id("package-a");
        let mut compiler = crate::RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_catalog())
            .build_compiler()?;
        let product = compiler.compile_document(&document)?;
        let bundle =
            CanonicalProgramBundle::from_product("bundle://event.mec", &document, &product)?;
        let encoded = bundle.encode()?;
        assert!(CanonicalProgramBundle::decode(&encoded, Some(source)).is_err());
        assert_eq!(
            CanonicalProgramBundle::decode_with_root_provenance(
                &encoded,
                Some(source),
                Some(&origin),
                Some("package-a"),
            )?,
            bundle,
        );
        assert!(
            bundle
                .validate_root_with_provenance(
                    Some(source),
                    Some(&changed_origin),
                    Some("package-a")
                )
                .is_err()
        );
        assert!(
            bundle
                .validate_root_with_provenance(Some(source), Some(&origin), Some("package-b"))
                .is_err()
        );
        Ok(())
    }

    #[test]
    fn canonical_bundle_text_only_dependency_remains_valid_without_nominal_declarations()
    -> MResult<()> {
        let source = "answer := 42\n";
        let dependency_source = "value := 1\n<+ value\n";
        let document = SourceDocument::parse_resolved(
            "bundle://answer.mec",
            Revision(7),
            Arc::<str>::from(source),
            ParseConfig::default(),
        )
        .unwrap();
        let mut compiler = crate::RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_catalog())
            .build_compiler()?;
        let product = compiler
            .compile_document(&document)?
            .with_source_dependencies(BTreeMap::from([(
                "bundle://dep.mec".to_owned(),
                mech_core::hash_str(dependency_source),
            )]));
        let bundle =
            CanonicalProgramBundle::from_product("bundle://answer.mec", &document, &product)?;
        bundle.validate_dependency_sources(|_| Some(dependency_source))?;
        Ok(())
    }
}
