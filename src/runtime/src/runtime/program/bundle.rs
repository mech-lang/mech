use mech_core::{GenericError, MResult, MechError, ProgramRevision};
use mech_engine::{ProgramCompilationProduct, decode_program_artifact_bytecode_v1};

use crate::SourceDocument;

pub const CANONICAL_PROGRAM_BUNDLE_VERSION: u32 = 1;

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
        let artifact_revision = product.artifact().revision();
        let bundle = Self {
            version: CANONICAL_PROGRAM_BUNDLE_VERSION,
            canonical_uri: canonical_uri.into(),
            document_id: document.source().document().0,
            source_revision: document.source().revision().0,
            source_hash: mech_core::hash_str(&source),
            source,
            artifact_revision: artifact_revision.into_bytes(),
            bytecode: product.bytecode().to_vec(),
        };
        bundle.validate(None)?;
        Ok(bundle)
    }

    pub fn validate(&self, expected_source: Option<&str>) -> MResult<ProgramRevision> {
        if self.version != CANONICAL_PROGRAM_BUNDLE_VERSION {
            return Err(bundle_error(format!(
                "unsupported canonical program bundle version {}; retired AST and stale bundle payloads must be regenerated",
                self.version
            )));
        }
        if self.canonical_uri.is_empty()
            || self.document_id != mech_core::hash_str(&self.canonical_uri)
            || self.source_hash != mech_core::hash_str(&self.source)
        {
            return Err(bundle_error(
                "canonical program bundle source identity is stale or invalid; regenerate the bundle",
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

    #[cfg(feature = "serde")]
    pub fn encode(&self) -> MResult<String> {
        mech_core::nodes::compress_and_encode(self)
            .map_err(|error| bundle_error(format!("unable to encode canonical bundle: {error}")))
    }

    #[cfg(feature = "serde")]
    pub fn decode(encoded: &str, expected_source: Option<&str>) -> MResult<Self> {
        let bundle: Self = mech_core::nodes::decode_and_decompress(encoded).map_err(|error| {
            bundle_error(format!(
                "invalid canonical program bundle; retired AST payloads must be regenerated: {error}"
            ))
        })?;
        bundle.validate(expected_source)?;
        Ok(bundle)
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
    use super::*;
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

        // A retired tree envelope is intentionally represented as an opaque
        // serialized payload here. The cutover test must not recreate the
        // parser dependency merely to prove that the canonical decoder rejects
        // a noncanonical shape.
        let legacy = mech_core::nodes::compress_and_encode(&("Program", source)).unwrap();
        let error = CanonicalProgramBundle::decode(&legacy, Some(source)).unwrap_err();
        assert!(error.display_message().contains("retired AST"));
        Ok(())
    }
}
