//! Prepared retained-document boundary for the browser controller cutover.

use std::sync::Arc;

use mech_core::{GenericError, MResult, MechError};
use mech_runtime::{CanonicalDocumentRenderer, CanonicalScopeResults, SourceDocument};
use mech_syntax::document::{ParseConfig, Revision, TextEdit};

/// Canonical browser source ownership kept separate from the legacy bootstrap
/// until the coordinated C route switch removes that bootstrap tree.
#[derive(Clone, Debug)]
pub struct CanonicalWasmDocument {
    document: SourceDocument,
}

impl CanonicalWasmDocument {
    /// Retain exact source and diagnostics. Strict admission happens at each
    /// consumer boundary so malformed source remains reportable.
    pub fn retain(
        canonical_uri: &str,
        revision: Revision,
        source: impl Into<Arc<str>>,
    ) -> MResult<Self> {
        SourceDocument::parse_resolved(canonical_uri, revision, source, ParseConfig::default())
            .map(|document| Self { document })
            .map_err(|error| {
                browser_source_error(format!("invalid retained browser source: {error:?}"))
            })
    }

    pub fn document(&self) -> &SourceDocument {
        &self.document
    }

    /// Return the exact accepted source instead of formatting and reparsing a
    /// detached legacy payload.
    pub fn initial_repl_source(&self) -> String {
        self.document.source().to_contiguous_string()
    }

    /// Prepare a transactional full-source replacement. The accepted owner is
    /// immutable; an invalid candidate returns an error and cannot consume or
    /// mutate its revision.
    pub fn replace_source(&self, source: impl Into<String>) -> MResult<Self> {
        if self.document.source().revision().0 == u64::MAX {
            return Err(browser_source_error("browser source revision is exhausted"));
        }
        let snapshot = self
            .document
            .source()
            .apply_edits(&[TextEdit::replace(
                self.document.source().full_range(),
                source.into(),
            )])
            .map_err(|error| browser_source_error(format!("invalid source edit: {error:?}")))?;
        let candidate = SourceDocument::parse(snapshot, ParseConfig::default());
        candidate
            .index()
            .map_err(|error| MechError::new(error, None))?;
        Ok(Self {
            document: candidate,
        })
    }

    /// Render only results belonging to this exact retained revision and its
    /// root/named/Mika owners.
    pub fn render_html(&self, results: &[CanonicalScopeResults]) -> MResult<String> {
        self.document
            .index()
            .map_err(|error| MechError::new(error, None))?;
        CanonicalDocumentRenderer
            .render_html(&self.document.document(), results)
            .map_err(|error| browser_source_error(error.to_string()))
    }

    /// Highlight a complete root-only console submission through canonical
    /// syntax. Prose, commands, fences, Mika-local and malformed input return
    /// `None` rather than falling back to the old formatter.
    pub fn repl_format_source(source: &str) -> Option<String> {
        if source.trim().is_empty() || source.trim_start().starts_with(':') {
            return None;
        }
        let document = Self::retain("browser:repl-entry", Revision(0), source).ok()?;
        document.document.index().ok()?;
        CanonicalDocumentRenderer
            .render_repl_source_html(&document.document.document())
            .ok()?
    }
}

fn browser_source_error(message: impl Into<String>) -> MechError {
    MechError::new(
        GenericError {
            msg: message.into(),
        },
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_engine::CanonicalSourceFrontend;
    use mech_runtime::{CanonicalRenderScope, RuntimeValueSnapshot};

    fn results(
        owner: mech_syntax::document::DocumentScopeId,
        scope: CanonicalRenderScope,
        program: &mech_engine::CanonicalSourceProgram,
    ) -> CanonicalScopeResults {
        let values = vec![RuntimeValueSnapshot::empty(); program.program().outputs.len()];
        CanonicalScopeResults::from_values(owner, scope, program, &values).unwrap()
    }

    #[test]
    fn canonical_browser_source_preserves_exact_text_and_transactional_revisions() {
        let first = CanonicalWasmDocument::retain(
            "browser:document.mec",
            Revision(4),
            "  value := 1\r\n-- e\u{301}\r\n",
        )
        .unwrap();
        assert_eq!(
            first.initial_repl_source(),
            "  value := 1\r\n-- e\u{301}\r\n"
        );
        let second = first.replace_source("value := 2\r\n").unwrap();
        assert_eq!(
            first.document().source().document(),
            second.document().source().document()
        );
        assert_eq!(first.document().source().revision(), Revision(4));
        assert_eq!(second.document().source().revision(), Revision(5));
        assert_eq!(
            first.initial_repl_source(),
            "  value := 1\r\n-- e\u{301}\r\n"
        );
        assert_eq!(second.initial_repl_source(), "value := 2\r\n");
        assert!(second.replace_source("value := [\r\n").is_err());
        assert_eq!(second.document().source().revision(), Revision(5));
        assert_eq!(second.initial_repl_source(), "value := 2\r\n");
    }

    #[test]
    fn canonical_browser_rejects_exhausted_revision_without_replacing_source() {
        let document = CanonicalWasmDocument::retain(
            "browser:document.mec",
            Revision(u64::MAX),
            "value := 1\n",
        )
        .unwrap();
        assert!(document.replace_source("value := 2\n").is_err());
        assert_eq!(document.document().source().revision(), Revision(u64::MAX));
        assert_eq!(document.initial_repl_source(), "value := 1\n");
    }

    #[test]
    fn canonical_browser_repl_formatting_has_no_legacy_fallback() {
        let formatted =
            CanonicalWasmDocument::repl_format_source("answer := 1 + 1; -- suppress this value")
                .expect("complete canonical source should highlight");
        assert!(formatted.contains("mech-code-block"), "{formatted}");
        assert!(formatted.contains("mech-variable-define"), "{formatted}");
        let terminal = formatted.find("mech-code-terminal").unwrap();
        let comment = formatted.find("suppress this value").unwrap();
        assert!(terminal < comment, "{formatted}");
        for rejected in [
            "answer := [",
            ":help",
            "plain prose.",
            "```mech\nanswer := 1\n```\n",
            "~∘~⸢answer := 1\n⸥\n",
        ] {
            assert!(
                CanonicalWasmDocument::repl_format_source(rejected).is_none(),
                "{rejected:?}"
            );
        }
    }

    #[cfg(feature = "mika")]
    #[test]
    fn canonical_browser_capture_keeps_hidden_disabled_named_and_mika_owners() {
        let source = "```mech:hidden\n1\n```\n\n```mech:disabled\n2\n```\n\n```mech:worker\n3\n```\n\n~∘~⸢```mech\n4\n```\n⸥\n";
        let document =
            CanonicalWasmDocument::retain("browser:document.mec", Revision(3), source).unwrap();
        let syntax = document.document().document();
        let child = &syntax.mika_scopes()[0].section;
        let frontend = CanonicalSourceFrontend;
        let root = frontend.compile_document(&syntax).unwrap();
        let named = frontend
            .compile_named_document_scope(&syntax, "worker")
            .unwrap();
        let child_root = frontend.compile_mika_section(child).unwrap();
        let rendered = document
            .render_html(&[
                results(syntax.scope_id(), CanonicalRenderScope::Root, &root),
                results(
                    syntax.scope_id(),
                    CanonicalRenderScope::Named("worker".to_owned()),
                    &named,
                ),
                results(child.scope_id(), CanonicalRenderScope::Root, &child_root),
            ])
            .unwrap();
        assert!(!rendered.contains(">1\n<"), "{rendered}");
        assert!(rendered.contains(">2\n<"), "{rendered}");
        assert!(rendered.contains(">3\n<"), "{rendered}");
        assert!(rendered.contains(">4\n<"), "{rendered}");

        let newer = document.replace_source("answer := 5\n").unwrap();
        assert!(
            newer
                .render_html(&[results(
                    syntax.scope_id(),
                    CanonicalRenderScope::Root,
                    &root,
                )])
                .is_err(),
            "stale capture results must not cross source revisions"
        );
    }
}
