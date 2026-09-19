//! Prepared canonical source boundary for CLI classification and formatting.
//!
//! S8A exposes this interface without changing the shipping command routes.
//! The coordinated cutover can adopt it without selecting between parsers.

use std::sync::Arc;

use mech_core::{MResult, MechError};
#[cfg(feature = "formatter")]
use mech_runtime::CanonicalDocumentRenderer;
use mech_runtime::SourceDocument;
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
        self.document.index().is_ok() && self.document.document().contains_executable_source()
    }

    /// Format canonical syntax without compiling or evaluating executable code.
    /// Invalid retained syntax cannot enter presentation as a partially valid source.
    #[cfg(feature = "formatter")]
    pub fn render_text(&self) -> MResult<String> {
        self.document
            .index()
            .map_err(|error| MechError::new(error, None))?;
        CanonicalDocumentRenderer
            .format_text(&self.document.document())
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
    pub fn render_html(&self) -> MResult<String> {
        self.document
            .index()
            .map_err(|error| MechError::new(error, None))?;
        CanonicalDocumentRenderer
            .format_html(&self.document.document())
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
    fn canonical_cli_formats_executable_source_without_running_it() {
        let source =
            CanonicalCliSource::retain("cli:format:code", Revision(0), "answer := 42\nanswer\n")
                .unwrap();
        assert_eq!(source.render_text().unwrap(), "answer := 42\nanswer\n");
        let html = source.render_html().unwrap();
        assert!(html.contains("answer := 42"));
        assert!(!html.contains("class='mech-program-output'"));
    }

    #[cfg(feature = "formatter")]
    #[test]
    fn canonical_cli_source_format_preserves_fences_and_inline_code() {
        for text in [
            "```mech:worker\nanswer := 42\nanswer\n```\n",
            "```mech:hidden\nsecret := 7\n```\n",
            "answer := 42\nThe answer is {answer}; displayed {{answer}}.\n",
        ] {
            let source =
                CanonicalCliSource::retain("cli:format:scopes", Revision(0), text).unwrap();
            let formatted = source.render_text().unwrap();
            assert_eq!(formatted, text);
            let html = source.render_html().unwrap();
            assert!(!html.contains("class='mech-program-output'"));
            assert!(!html.contains("class='mech-output'"));
            if text.contains("{answer}") {
                assert!(html.contains("{answer}"));
            }
            if text.contains("mech:hidden") {
                assert!(html.contains("secret := 7"), "{html}");
                assert!(html.contains("class='mech-code-block hidden'"), "{html}");
            }
        }
    }

    #[cfg(all(feature = "formatter", feature = "mika"))]
    #[test]
    fn canonical_cli_source_format_preserves_mika_scope_boundaries() {
        let text = "╭◉╮⸢answer := 42\nanswer\n⸥\n";
        let source = CanonicalCliSource::retain("cli:format:mika", Revision(0), text).unwrap();
        assert_eq!(source.render_text().unwrap(), text);
        assert!(source.render_html().unwrap().contains("answer := 42"));
    }

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
            valid.render_text().unwrap(),
            "first e\u{301}.\r\n\r\nsecond.\r\n"
        );
        assert!(valid.render_html().unwrap().contains("first e\u{301}."));

        let invalid =
            CanonicalCliSource::retain("cli:format:test", Revision(8), "value := [\r\n").unwrap();
        assert_eq!(
            invalid.document().source().to_contiguous_string(),
            "value := [\r\n"
        );
        assert!(!invalid.document().snapshot().diagnostics.is_empty());
        assert!(invalid.render_text().is_err());
        assert!(invalid.render_html().is_err());
    }

    #[cfg(feature = "run")]
    #[test]
    fn canonical_cli_classifier_rejects_address_conflicts_in_every_owner() {
        let conflict = "@users := @main{:read(*)}\n\n```mech:users\nx := 1\n```\n";
        let mut sources = vec![format!("answer := 1\n{conflict}")];
        if cfg!(feature = "mika") {
            sources.extend([
                format!("answer := 1\n╭◉╮⸢{conflict}⸥\n"),
                format!("answer := 1\n╭◉╮⸢~∘~⸢{conflict}⸥\n⸥\n"),
            ]);
        }
        for source in sources {
            let retained =
                CanonicalCliSource::retain("cli:run:conflict", Revision(0), source.as_str())
                    .unwrap();
            assert!(retained.document().is_strictly_clean(), "{source:?}");
            assert!(retained.document().document().contains_executable_source());
            assert!(retained.document().index().is_err());
            assert!(!retained.contains_executable_source(), "{source:?}");
        }
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
