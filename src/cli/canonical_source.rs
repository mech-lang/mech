//! Prepared canonical source boundary for CLI classification and formatting.
//!
//! S8A exposes this interface without changing the shipping command routes.
//! The coordinated cutover can adopt it without selecting between parsers.

use std::sync::Arc;

use mech_core::{MResult, MechError};
use mech_runtime::SourceDocument;
#[cfg(feature = "formatter")]
use mech_runtime::{CanonicalDocumentRenderer, CanonicalScopeResults};
use mech_syntax::document::{ParseConfig, Revision};

/// One losslessly retained CLI source revision.
#[derive(Clone, Debug)]
pub struct CanonicalCliSource {
    document: SourceDocument,
}

impl CanonicalCliSource {
    /// Retain source and diagnostics without claiming strict admission.
    pub fn retain(
        canonical_uri: &str,
        revision: Revision,
        source: impl Into<Arc<str>>,
    ) -> MResult<Self> {
        let document =
            SourceDocument::parse_resolved(canonical_uri, revision, source, ParseConfig::default())
                .map_err(|error| {
                    MechError::new(
                        crate::GenericError {
                            msg: format!("invalid retained CLI source: {error:?}"),
                        },
                        None,
                    )
                })?;
        Ok(Self { document })
    }

    pub fn document(&self) -> &SourceDocument {
        &self.document
    }

    /// Classify only a strictly admitted canonical document.
    #[cfg(feature = "run")]
    pub fn contains_executable_source(&self) -> bool {
        self.document.is_strictly_clean() && self.document.document().contains_executable_source()
    }

    /// Render through the qualified canonical document renderer. Invalid
    /// retained syntax cannot enter presentation as a partially valid source.
    #[cfg(feature = "formatter")]
    pub fn render_text(&self, results: &[CanonicalScopeResults]) -> MResult<String> {
        self.document
            .index()
            .map_err(|error| MechError::new(error, None))?;
        CanonicalDocumentRenderer
            .render_text(&self.document.document(), results)
            .map_err(|error| {
                MechError::new(
                    crate::GenericError {
                        msg: error.to_string(),
                    },
                    None,
                )
            })
    }

    #[cfg(feature = "formatter")]
    pub fn render_html(&self, results: &[CanonicalScopeResults]) -> MResult<String> {
        self.document
            .index()
            .map_err(|error| MechError::new(error, None))?;
        CanonicalDocumentRenderer
            .render_html(&self.document.document(), results)
            .map_err(|error| {
                MechError::new(
                    crate::GenericError {
                        msg: error.to_string(),
                    },
                    None,
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "formatter")]
    #[test]
    fn canonical_cli_format_adapter_preserves_raw_coordinates_and_rejects_malformed_input() {
        let valid = CanonicalCliSource::retain(
            "cli:format:test",
            Revision(7),
            "  first e\u{301}.\r\n\r\nsecond.\r\n",
        )
        .unwrap();
        assert_eq!(
            valid.document().source().to_contiguous_string(),
            "  first e\u{301}.\r\n\r\nsecond.\r\n"
        );
        assert_eq!(valid.document().source().revision(), Revision(7));
        assert_eq!(
            valid.render_text(&[]).unwrap(),
            "first e\u{301}.\r\n\r\nsecond.\r\n"
        );
        assert!(valid.render_html(&[]).unwrap().contains("first e\u{301}."));

        let invalid =
            CanonicalCliSource::retain("cli:format:test", Revision(8), "value := [\r\n").unwrap();
        assert_eq!(
            invalid.document().source().to_contiguous_string(),
            "value := [\r\n"
        );
        assert!(!invalid.document().snapshot().diagnostics.is_empty());
        assert!(invalid.render_text(&[]).is_err());
        assert!(invalid.render_html(&[]).is_err());
    }

    #[cfg(feature = "run")]
    #[test]
    fn canonical_cli_classifier_excludes_inert_local_and_malformed_source() {
        let classify = |source| {
            CanonicalCliSource::retain("cli:run:test", Revision(0), source)
                .unwrap()
                .contains_executable_source()
        };
        assert!(classify("answer := 42\n"));
        assert!(classify("The answer is {40 + 2}.\n"));
        assert!(!classify("Displayed {{40 + 2}}.\n"));
        assert!(!classify("```mech:disabled\nanswer := 42\n```\n"));
        assert!(!classify("~∘~⸢answer := 42\n⸥\n"));
        assert!(!classify("answer := [\n"));
    }
}
