use mech_core::{CanonicalNominalPath, GenericError, MResult, MechError, ProgramRevision};
use mech_engine::{
    CanonicalSourceFrontend, ProgramArtifactCompilationProduct, ProgramCompilationProduct,
    decode_program_artifact_bytecode_v1, encode_program_artifact_bytecode_v1,
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
    /// Required for root enum/FSM declarations and retained across all revisions
    /// of an already-owned standalone document, including scalar and clear edits.
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
        Self::from_compiled_parts(
            canonical_uri,
            document,
            product.artifact().revision(),
            product.bytecode().to_vec(),
            product.source_dependencies().clone(),
        )
    }

    pub fn from_artifact_product(
        canonical_uri: impl Into<String>,
        document: &SourceDocument,
        product: &ProgramArtifactCompilationProduct,
        source_dependencies: std::collections::BTreeMap<String, u64>,
    ) -> MResult<Self> {
        Self::from_compiled_parts(
            canonical_uri,
            document,
            product.artifact().revision(),
            encode_program_artifact_bytecode_v1(product.artifact())
                .map_err(|error| bundle_error(format!("{error:?}")))?,
            source_dependencies,
        )
    }

    fn from_compiled_parts(
        canonical_uri: impl Into<String>,
        document: &SourceDocument,
        artifact_revision: ProgramRevision,
        bytecode: Vec<u8>,
        source_dependencies: std::collections::BTreeMap<String, u64>,
    ) -> MResult<Self> {
        let source = document.source().to_contiguous_string();
        let has_origin_dependent_declarations = CanonicalSourceFrontend
            .has_origin_dependent_declarations(&document.document())
            .map_err(|error| bundle_error(error.to_string()))?;
        validate_standalone_provenance(document.nominal_origin(), document.nominal_package_id())?;
        let retain_origin = has_origin_dependent_declarations
            || document.nominal_origin().is_some_and(is_standalone_origin);
        let root_nominal_origin = retain_origin
            .then(|| document.nominal_origin().cloned())
            .flatten();
        if has_origin_dependent_declarations && root_nominal_origin.is_none() {
            return Err(bundle_error(
                "canonical bundle root nominal declarations have no defining origin",
            ));
        }
        let root_nominal_package_id = retain_origin
            .then(|| document.nominal_package_id().map(str::to_owned))
            .flatten();
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
            source_dependencies,
            artifact_revision: artifact_revision.into_bytes(),
            bytecode,
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
        validate_standalone_provenance(
            self.root_nominal_origin.as_ref(),
            self.root_nominal_package_id.as_deref(),
        )?;
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
        let document = SourceDocument::parse_resolved(
            &self.canonical_uri,
            mech_syntax::document::Revision(self.source_revision),
            self.source.as_str(),
            mech_syntax::document::ParseConfig::default(),
        )
        .map_err(|error| bundle_error(format!("invalid canonical bundle source: {error:?}")))?;
        let has_origin_dependent_declarations = CanonicalSourceFrontend
            .has_origin_dependent_declarations(&document.document())
            .map_err(|error| bundle_error(error.to_string()))?;
        if has_origin_dependent_declarations && self.root_nominal_origin.is_none() {
            return Err(bundle_error(
                "canonical bundle root nominal declarations have no defining origin; regenerate the bundle",
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
        let expected = self
            .root_nominal_origin
            .as_ref()
            .map(|origin| {
                CanonicalSourceFrontend
                    .declared_nominal_keys(&document.document(), origin)
                    .map_err(|error| bundle_error(error.to_string()))
            })
            .transpose()?
            .unwrap_or_default();
        let retained = artifact
            .source_nominal_declarations()
            .iter()
            .filter(|declaration| declaration.document_id == self.document_id)
            .collect::<Vec<_>>();
        if retained.len() != expected.len() || expected.iter().any(|(path, key)| {
            !retained.iter().any(|declaration| declaration.relative_path.as_ref() == path.as_ref()
                && matches!(artifact.schemas().get(declaration.schema).map(|schema| schema.body()),
                    Some(mech_core::SchemaBody::Enum { key: compiled, .. }) if compiled == key))
        }) {
            return Err(bundle_error(
                "canonical bundle root nominal declarations differ from the compiled defining origin or lack declaration evidence; regenerate the bundle",
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
            if super::compiler::canonical_dependency_identity_hash(
                retained.source,
                retained.nominal_origin,
                retained.nominal_package_id,
            ) == *expected_hash
            {
                continue;
            }
            let text_matches = mech_core::hash_str(retained.source) == *expected_hash;
            let text_only = if text_matches {
                let document = SourceDocument::parse_resolved(
                    uri,
                    mech_syntax::document::Revision(0),
                    retained.source,
                    mech_syntax::document::ParseConfig::default(),
                )
                .map_err(|error| {
                    bundle_error(format!(
                        "invalid canonical bundle dependency {uri}: {error:?}"
                    ))
                })?;
                !CanonicalSourceFrontend
                    .has_origin_dependent_declarations(&document.document())
                    .map_err(|error| bundle_error(error.to_string()))?
            } else {
                false
            };
            if !text_only {
                return Err(bundle_error(format!(
                    "canonical bundle dependency {uri} differs from the compiled source or nominal provenance; regenerate the bundle"
                )));
            }
        }
        Ok(())
    }

    #[cfg(feature = "serde")]
    pub fn encode(&self) -> MResult<String> {
        mech_core::encoded_payload::compress_and_encode(self)
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

    /// Restore a standalone editable root's retained owner when no resolver
    /// provenance exists. Package roots still require explicit current provenance.
    #[cfg(feature = "serde")]
    pub fn decode_standalone(encoded: &str, expected_source: Option<&str>) -> MResult<Self> {
        let bundle = Self::decode_payload(encoded)?;
        let origin = bundle
            .root_nominal_origin
            .as_ref()
            .filter(|origin| is_standalone_origin(origin));
        bundle.validate_root_with_provenance(expected_source, origin, None)?;
        Ok(bundle)
    }

    #[cfg(feature = "serde")]
    fn decode_payload(encoded: &str) -> MResult<Self> {
        mech_core::encoded_payload::decode_and_decompress(encoded).map_err(|error| {
            bundle_error(format!(
                "invalid canonical program bundle; retired AST payloads must be regenerated: {error}"
            ))
        })
    }
}

fn is_standalone_origin(origin: &CanonicalNominalPath) -> bool {
    matches!(origin.segments(), [namespace, owner]
        if namespace == "mech:standalone"
            && uuid::Uuid::parse_str(owner).is_ok_and(|uuid|
                uuid.get_variant() == uuid::Variant::RFC4122
                    && uuid.get_version_num() == 7 && uuid.to_string() == *owner))
}

fn validate_standalone_provenance(
    origin: Option<&CanonicalNominalPath>,
    package_id: Option<&str>,
) -> MResult<()> {
    if origin.is_some_and(|origin| {
        origin
            .segments()
            .first()
            .is_some_and(|namespace| namespace == "mech:standalone")
            && (!is_standalone_origin(origin) || package_id.is_some())
    }) {
        return Err(bundle_error(
            "canonical bundle standalone provenance is invalid; regenerate the bundle",
        ));
    }
    Ok(())
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

    fn rehash_source(bundle: &mut CanonicalProgramBundle) {
        bundle.source_hash = super::super::compiler::canonical_dependency_identity_hash(
            &bundle.source,
            bundle.root_nominal_origin.as_ref(),
            bundle.root_nominal_package_id.as_deref(),
        );
    }

    fn document(source: &str) -> SourceDocument {
        SourceDocument::parse_resolved(
            "bundle:///document.mec",
            Revision(0),
            source,
            ParseConfig::default(),
        )
        .unwrap()
        .with_standalone_nominal_origin()
    }

    fn empty_product() -> mech_engine::ProgramArtifactCompilationProduct {
        let schemas = mech_core::SchemaTableBuilder::new()
            .finish()
            .unwrap()
            .into_parts()
            .0;
        let constants = mech_core::ConstantStoreBuilder::new(&schemas)
            .finish()
            .unwrap()
            .into_parts()
            .0;
        let artifact = mech_engine::compile_source_program_with_control_contracts(
            &mech_engine::SourceProgram::default(),
            &mut mech_engine::ArtifactBuildContext::new(&schemas, &constants),
            &[],
        )
        .unwrap();
        mech_engine::ProgramArtifactCompilationProduct::from_artifact(artifact)
    }

    fn owned_source(
        name: &str,
        source: &str,
        origin: &CanonicalNominalPath,
        package_id: &str,
    ) -> MResult<crate::ResolvedSource> {
        crate::ResolvedSource::new(
            name,
            format!("memory:{name}"),
            mech_core::MechSourceCode::String(source.to_owned()),
        )
        .with_kind(crate::SourceKind::Mech)
        .with_nominal_origin(origin.clone())
        .with_nominal_package_id(package_id)
        .retain_source_document(Revision(0), ParseConfig::default())?
        .admit_canonical_document()
    }

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
        let legacy =
            mech_core::encoded_payload::compress_and_encode(&retired_tree_payload).unwrap();
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
    fn canonical_bundle_fsm_dependency_freshness_includes_compiled_provenance() -> MResult<()> {
        let root_source = "+> ./dep.mec\nanswer := dep/value\nanswer\n";
        let origin =
            CanonicalNominalPath::new(["package-a".to_owned(), "drive".to_owned()]).unwrap();
        let changed_origin =
            CanonicalNominalPath::new(["package-a".to_owned(), "other-drive".to_owned()]).unwrap();
        for dependency_source in [
            "#Drive() => <f64>\n  | :Done.\n#Drive() -> :Done\n  :Done => 1.0.\nvalue := #Drive()\n<+ value\n",
            "#Drive() => <f64>\n  | :Done.\n#Drive() -> :Done\n  :Done => 1.0.\nvalue := 42\n<+ value\n",
            "```mech\n#Drive() => <f64>\n  | :Done.\n#Drive() -> :Done\n  :Done => 1.0.\nvalue := #Drive()\n```\n<+ value\n",
        ] {
            let mut resolver = crate::InMemorySourceResolver::new();
            resolver.insert_canonical_string("main.mec", root_source)?;
            resolver.insert_source(
                "dep.mec",
                owned_source("dep.mec", dependency_source, &origin, "package-a")?,
            )?;
            let root = SourceDocument::parse_resolved(
                "memory:main.mec",
                Revision(0),
                root_source,
                ParseConfig::default(),
            )
            .unwrap();
            let mut compiler = crate::RuntimeBuilder::new()
                .function_catalog(mech_stdlib::source_catalog())
                .source_resolver(resolver)
                .build_compiler()?;
            let products = [
                compiler.compile_canonical_root(crate::SourceRequest::new("main.mec"))?,
                compiler.compile_canonical_roots(
                    &[crate::SourceRequest::new("main.mec")],
                    crate::ModuleBuildOptions::new("test", "v0.4", "native", &[], &[]),
                )?,
            ];
            for product in products {
                assert_ne!(
                    product.source_dependencies()["memory:dep.mec"],
                    mech_core::hash_str(dependency_source),
                    "used and unused FSM declarations require provenance on both dependency routes"
                );
                let bundle =
                    CanonicalProgramBundle::from_product("memory:main.mec", &root, &product)?;
                let validate = |bundle: &CanonicalProgramBundle, origin, package_id| {
                    bundle.validate_dependency_sources_with_provenance(|uri| {
                        assert_eq!(uri, "memory:dep.mec");
                        Some(CanonicalDependencySource {
                            source: dependency_source,
                            nominal_origin: origin,
                            nominal_package_id: package_id,
                        })
                    })
                };
                validate(&bundle, Some(&origin), Some("package-a"))?;
                assert!(validate(&bundle, Some(&changed_origin), Some("package-a")).is_err());
                assert!(validate(&bundle, Some(&origin), Some("package-b")).is_err());
                assert!(validate(&bundle, None, None).is_err());
                let mut legacy = bundle.clone();
                legacy.source_dependencies.insert(
                    "memory:dep.mec".to_owned(),
                    mech_core::hash_str(dependency_source),
                );
                assert!(validate(&legacy, Some(&origin), Some("package-a")).is_err());
                assert!(validate(&legacy, Some(&changed_origin), Some("package-a")).is_err());
            }
        }
        Ok(())
    }

    #[test]
    fn canonical_bundle_resolver_rejects_fsm_owner_collisions() -> MResult<()> {
        let root_source =
            "+> ./left.mec\n+> ./right.mec\nanswer := left/value + right/value\nanswer\n";
        let dependency_source = "#Drive() => <f64>\n  | :Done.\n#Drive() -> :Done\n  :Done => 1.0.\nvalue := #Drive()\n<+ value\n";
        let origin =
            CanonicalNominalPath::new(["package".to_owned(), "shared-module".to_owned()]).unwrap();
        for right_source in [
            dependency_source.to_owned(),
            dependency_source.replace("Drive", "OtherDrive"),
        ] {
            let mut resolver = crate::InMemorySourceResolver::new();
            resolver.insert_canonical_string("main.mec", root_source)?;
            resolver.insert_source(
                "left.mec",
                owned_source("left.mec", dependency_source, &origin, "package")?,
            )?;
            resolver.insert_source(
                "right.mec",
                owned_source("right.mec", &right_source, &origin, "package")?,
            )?;
            let mut compiler = crate::RuntimeBuilder::new()
                .function_catalog(mech_stdlib::source_catalog())
                .source_resolver(resolver)
                .build_compiler()?;
            let product = compiler.compile_canonical_root(crate::SourceRequest::new("main.mec"));
            if right_source == dependency_source {
                let error = product.expect_err("distinct documents cannot define one FSM path");
                assert!(
                    error
                        .display_message()
                        .contains("ambiguous-nominal-declaration-v1"),
                    "{error:?}"
                );
            } else {
                assert_eq!(product?.source_dependencies().len(), 2);
            }
        }
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
        assert!(CanonicalProgramBundle::decode_standalone(&encoded, Some(source)).is_err());
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
    fn canonical_bundle_standalone_enum_and_fsm_origins_survive_reload_and_edit() -> MResult<()> {
        for source in [
            "<color> := :red | :blue\nmy-color<color> := :red\n",
            "#Drive() => <f64>\n  | :Done.\n#Drive() -> :Done\n  :Done => 1.0.\nresult := #Drive()\nresult\n",
        ] {
            let document = SourceDocument::parse_resolved(
                "bundle:///document.mec",
                Revision(0),
                source,
                ParseConfig::default(),
            )
            .unwrap()
            .with_standalone_nominal_origin();
            let mut compiler = crate::RuntimeBuilder::new()
                .function_catalog(mech_stdlib::source_catalog())
                .build_compiler()?;
            let product = compiler.compile_document(&document)?;
            let bundle = CanonicalProgramBundle::from_product(
                "bundle:///document.mec",
                &document,
                &product,
            )?;
            assert_eq!(
                bundle.root_nominal_origin.as_ref(),
                document.nominal_origin()
            );
            let encoded = bundle.encode()?;
            let restored = CanonicalProgramBundle::decode_standalone(&encoded, Some(source))?;
            let edited_source = source
                .replace("my-color<color> := :red", "my-color<color> := :blue")
                .replace("=> 1.0.", "=> 2.0.");
            let edited = SourceDocument::parse_resolved(
                "runtime:interactive",
                Revision(1),
                edited_source,
                ParseConfig::default(),
            )
            .unwrap()
            .with_nominal_provenance(restored.root_nominal_origin.unwrap(), None);
            let edited_product = compiler.compile_document(&edited)?;
            let keys = |artifact: &mech_engine::ProgramArtifact| {
                artifact
                    .schemas()
                    .entries()
                    .filter_map(|entry| {
                        matches!(entry.schema().body(), mech_core::SchemaBody::Enum { .. })
                            .then_some(entry.key())
                    })
                    .collect::<Vec<_>>()
            };
            let original_keys = keys(product.artifact());
            assert!(!original_keys.is_empty());
            assert_eq!(keys(edited_product.artifact()), original_keys);
            assert!(
                CanonicalProgramBundle::decode_with_root_provenance(
                    &encoded,
                    Some(source),
                    Some(&SourceDocument::new_standalone_origin()),
                    None,
                )
                .is_err()
            );

            let mut contradictory = bundle.clone();
            contradictory.root_nominal_package_id = Some("package-a".to_owned());
            contradictory.source_hash = super::super::compiler::canonical_dependency_identity_hash(
                source,
                contradictory.root_nominal_origin.as_ref(),
                contradictory.root_nominal_package_id.as_deref(),
            );
            assert!(
                CanonicalProgramBundle::decode_standalone(&contradictory.encode()?, Some(source),)
                    .is_err()
            );

            let mut invalid_owner = bundle.clone();
            invalid_owner.root_nominal_origin = Some(
                CanonicalNominalPath::new(["mech:standalone".to_owned(), "not-a-uuid".to_owned()])
                    .unwrap(),
            );
            invalid_owner.source_hash = super::super::compiler::canonical_dependency_identity_hash(
                source,
                invalid_owner.root_nominal_origin.as_ref(),
                None,
            );
            assert!(
                CanonicalProgramBundle::decode_standalone(&invalid_owner.encode()?, Some(source),)
                    .is_err()
            );
        }
        Ok(())
    }

    #[test]
    fn canonical_bundle_binds_used_and_unused_nominal_declarations_to_bytecode() -> MResult<()> {
        let mut compiler = crate::RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_catalog())
            .build_compiler()?;
        for source in [
            "<color> := :red | :blue\nmy-color<color> := :red\n",
            "<color> := :red | :blue\nanswer := 42\n",
            "#Drive() => <f64>\n  | :Done.\n#Drive() -> :Done\n  :Done => 1.0.\nresult := #Drive()\nresult\n",
            "#Drive() => <f64>\n  | :Done.\n#Drive() -> :Done\n  :Done => 1.0.\nanswer := 42\n",
        ] {
            let document = document(source);
            let product = compiler.compile_document(&document)?;
            assert!(!product.artifact().source_nominal_declarations().is_empty());
            let bundle = CanonicalProgramBundle::from_product(
                "bundle:///document.mec",
                &document,
                &product,
            )?;
            CanonicalProgramBundle::decode_standalone(&bundle.encode()?, Some(source))?;

            let changed_document = document
                .clone()
                .with_nominal_origin(SourceDocument::new_standalone_origin());
            assert!(
                CanonicalProgramBundle::from_product(
                    "bundle:///document.mec",
                    &changed_document,
                    &product,
                )
                .is_err(),
                "a product compiled under a different defining owner is stale"
            );
            let mut changed = bundle.clone();
            changed.root_nominal_origin = changed_document.nominal_origin().cloned();
            rehash_source(&mut changed);
            assert!(
                CanonicalProgramBundle::decode_standalone(&changed.encode()?, Some(source))
                    .is_err()
            );
            assert!(
                CanonicalProgramBundle::decode_with_root_provenance(
                    &changed.encode()?,
                    Some(source),
                    changed.root_nominal_origin.as_ref(),
                    None,
                )
                .is_err()
            );

            let mut removed = bundle.clone();
            removed.root_nominal_origin = None;
            rehash_source(&mut removed);
            assert!(
                CanonicalProgramBundle::decode_standalone(&removed.encode()?, Some(source))
                    .is_err()
            );
            assert!(CanonicalProgramBundle::decode(&removed.encode()?, Some(source)).is_err());

            for declarations in [
                Box::new([]) as Box<[mech_engine::SourceNominalDeclaration]>,
                product
                    .artifact()
                    .source_nominal_declarations()
                    .iter()
                    .map(|declaration| mech_engine::SourceNominalDeclaration {
                        document_id: mech_core::hash_str("bundle:///foreign.mec"),
                        ..declaration.clone()
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ] {
                let artifact = product
                    .artifact()
                    .clone()
                    .with_source_nominal_declarations(declarations)
                    .unwrap();
                let artifact_product =
                    mech_engine::ProgramArtifactCompilationProduct::from_artifact(artifact.clone());
                assert!(
                    CanonicalProgramBundle::from_artifact_product(
                        "bundle:///document.mec",
                        &document,
                        &artifact_product,
                        BTreeMap::new(),
                    )
                    .is_err()
                );
                let mut missing = bundle.clone();
                missing.bytecode =
                    mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
                missing.artifact_revision = artifact.revision().into_bytes();
                assert!(
                    CanonicalProgramBundle::decode_standalone(&missing.encode()?, Some(source))
                        .is_err()
                );
            }
        }
        Ok(())
    }

    #[test]
    fn canonical_bundle_rejects_ownerless_fsms_and_non_rfc_uuid_owners() -> MResult<()> {
        let source = "#Drive() => <f64>\n  | :Done.\n#Drive() -> :Done\n  :Done => 1.0.\nresult := #Drive()\nresult\n";
        let mut compiler = crate::RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_catalog())
            .build_compiler()?;
        let ownerless = SourceDocument::parse_resolved(
            "bundle:///document.mec",
            Revision(0),
            source,
            ParseConfig::default(),
        )
        .unwrap();
        let product = compiler.compile_document(&ownerless)?;
        assert!(
            CanonicalProgramBundle::from_product("bundle:///document.mec", &ownerless, &product)
                .is_err()
        );
        let artifact_product = compiler.compile_document_artifact(&ownerless)?;
        assert!(
            CanonicalProgramBundle::from_artifact_product(
                "bundle:///document.mec",
                &ownerless,
                &artifact_product,
                BTreeMap::new()
            )
            .is_err()
        );

        for source in [source, "answer := 42\n"] {
            let document = document(source);
            let product = compiler.compile_document(&document)?;
            let mut bundle = CanonicalProgramBundle::from_product(
                "bundle:///document.mec",
                &document,
                &product,
            )?;
            let invalid_owner = CanonicalNominalPath::new([
                "mech:standalone".to_owned(),
                "00000000-0000-7000-0000-000000000000".to_owned(),
            ])
            .unwrap();
            let uuid = uuid::Uuid::parse_str(&invalid_owner.segments()[1]).unwrap();
            assert_eq!(uuid.get_version_num(), 7);
            assert_ne!(uuid.get_variant(), uuid::Variant::RFC4122);
            let invalid_document = document.with_nominal_origin(invalid_owner.clone());
            assert!(
                CanonicalProgramBundle::from_product(
                    "bundle:///document.mec",
                    &invalid_document,
                    &product
                )
                .is_err()
            );
            bundle.root_nominal_origin = Some(invalid_owner.clone());
            rehash_source(&mut bundle);
            assert!(
                CanonicalProgramBundle::decode_standalone(&bundle.encode()?, Some(source)).is_err()
            );
            assert!(
                CanonicalProgramBundle::decode_with_root_provenance(
                    &bundle.encode()?,
                    Some(source),
                    Some(&invalid_owner),
                    None
                )
                .is_err()
            );
        }
        Ok(())
    }

    #[test]
    fn canonical_bundle_standalone_origin_survives_scalar_and_clear_revisions() -> MResult<()> {
        let enum_source = "<color> := :red | :blue\nmy-color<color> := :red\n";
        let initial = document(enum_source);
        let mut compiler = crate::RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_catalog())
            .build_compiler()?;
        let original = compiler.compile_document(&initial)?;
        let enum_schema = original
            .artifact()
            .schemas()
            .entries()
            .find(|entry| matches!(entry.schema().body(), mech_core::SchemaBody::Enum { .. }))
            .unwrap()
            .key();
        let mut owner = initial.nominal_origin().cloned();
        for source in ["answer := 42\n", "", "Cleared\n=======\n"] {
            let revision = SourceDocument::parse_resolved(
                "bundle:///document.mec",
                Revision(1),
                source,
                ParseConfig::default(),
            )
            .unwrap()
            .with_nominal_provenance(owner.clone().unwrap(), None);
            // Clearing a session has no executable source unit. Its transport
            // uses a valid empty artifact rather than compiling empty text.
            let product = if source == "answer := 42\n" {
                compiler.compile_document_artifact(&revision)?
            } else {
                empty_product()
            };
            assert!(product.artifact().source_nominal_declarations().is_empty());
            let bundle = CanonicalProgramBundle::from_artifact_product(
                "bundle:///document.mec",
                &revision,
                &product,
                BTreeMap::new(),
            )?;
            assert_eq!(bundle.root_nominal_origin, owner);
            let restored =
                CanonicalProgramBundle::decode_standalone(&bundle.encode()?, Some(source))?;
            owner = restored.root_nominal_origin;
            let restored_enum = SourceDocument::parse_resolved(
                "runtime:interactive",
                Revision(2),
                enum_source,
                ParseConfig::default(),
            )
            .unwrap()
            .with_nominal_provenance(owner.clone().unwrap(), None);
            let restored = compiler.compile_document(&restored_enum)?;
            assert!(
                restored
                    .artifact()
                    .schemas()
                    .find_by_key(enum_schema)
                    .is_some()
            );
        }
        let package_scalar = document("answer := 42\n").with_nominal_provenance(
            CanonicalNominalPath::new(["package".to_owned(), "module".to_owned()]).unwrap(),
            Some("package-id".to_owned()),
        );
        let product = compiler.compile_document(&package_scalar)?;
        let bundle = CanonicalProgramBundle::from_product(
            "bundle:///document.mec",
            &package_scalar,
            &product,
        )?;
        assert!(bundle.root_nominal_origin.is_none());
        CanonicalProgramBundle::decode(&bundle.encode()?, Some("answer := 42\n"))?;
        Ok(())
    }

    #[test]
    fn canonical_bundle_dependency_nominal_key_cannot_satisfy_root_declaration_evidence()
    -> MResult<()> {
        let source = "<color> := :red | :blue\nmy-color<color> := :red\n";
        let root = document(source);
        let dependency = SourceDocument::parse_resolved(
            "bundle:///dep.mec",
            Revision(0),
            "<color> := :red | :blue\nunrelated := 42\n",
            ParseConfig::default(),
        )
        .unwrap()
        .with_standalone_nominal_origin();
        let ordered = |document: &SourceDocument, identity, publish_result| {
            mech_engine::CanonicalOrderedDocument {
                document: document.document(),
                nominal_origin: document.nominal_origin().cloned(),
                nominal_package_id: None,
                identity,
                publish_result,
                input_schemas: BTreeMap::new(),
                resource_writes: BTreeMap::new(),
                imports: BTreeMap::new(),
                resolved_modules: std::collections::BTreeSet::new(),
            }
        };
        let program = mech_engine::CanonicalSourceFrontend
            .compile_ordered_documents_with_catalog(
                &[ordered(&dependency, 0, false), ordered(&root, 1, true)],
                mech_stdlib::source_catalog(),
            )
            .unwrap();
        let artifact = program.compile_artifact().unwrap();
        let dependency_key = mech_engine::CanonicalSourceFrontend
            .declared_nominal_keys(&dependency.document(), dependency.nominal_origin().unwrap())
            .unwrap()[0]
            .1;
        assert!(artifact.schemas().entries().any(|entry|
            matches!(entry.schema().body(), mech_core::SchemaBody::Enum { key, .. } if *key == dependency_key)));
        let product = mech_engine::ProgramArtifactCompilationProduct::from_artifact(artifact);
        let mut bundle = CanonicalProgramBundle::from_artifact_product(
            "bundle:///document.mec",
            &root,
            &product,
            BTreeMap::new(),
        )?;
        bundle.root_nominal_origin = dependency.nominal_origin().cloned();
        rehash_source(&mut bundle);
        assert!(
            CanonicalProgramBundle::decode_standalone(&bundle.encode()?, Some(source)).is_err()
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
        for origin in [
            CanonicalNominalPath::new(["package-a".to_owned(), "module".to_owned()]).unwrap(),
            CanonicalNominalPath::new(["package-b".to_owned(), "module".to_owned()]).unwrap(),
        ] {
            bundle.validate_dependency_sources_with_provenance(|_| {
                Some(CanonicalDependencySource {
                    source: dependency_source,
                    nominal_origin: Some(&origin),
                    nominal_package_id: Some("package-id"),
                })
            })?;
        }
        Ok(())
    }
}
