use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::edit::{TextEdit, TextRange, TextSize};
use super::flags::NodeFlags;
use super::ids::{DiagnosticId, ParserContextId, Revision, RuleId, SyntaxElementId};
use super::index::NodeIndex;
use super::source::TextSnapshot;
use super::syntax_kind::SyntaxKind;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct DiagnosticCode(pub String);

impl DiagnosticCode {
    pub fn syntax(name: &str) -> Self {
        Self(format!("syntax/{name}"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for DiagnosticCode {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "kebab-case"))]
pub enum DiagnosticPhase {
    Syntax,
    SyntaxValidation,
    Lowering,
    Kind,
    Dimension,
    Effect,
    Coeffect,
    Refinement,
    Liveness,
    Document,
    Runtime,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "kebab-case"))]
pub enum Severity {
    Error,
    Warning,
    Information,
    Hint,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[cfg_attr(feature = "serde", serde(tag = "kind", rename_all = "kebab-case"))]
pub enum DiagnosticAnchor {
    Element {
        element: SyntaxElementId,
        relative: TextRange,
    },
    Absolute {
        revision: Revision,
        range: TextRange,
    },
}

impl DiagnosticAnchor {
    pub fn resolve(&self, revision: Revision, nodes: &NodeIndex) -> Option<TextRange> {
        match self {
            Self::Element { element, relative } => {
                let base = nodes.range(*element)?;
                if relative.end.0 > base.len().0 {
                    return None;
                }
                Some(TextRange::new(
                    base.start + relative.start,
                    base.start + relative.end,
                ))
            }
            Self::Absolute {
                revision: anchor_revision,
                range,
            } => (*anchor_revision == revision).then_some(*range),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct DiagnosticLabel {
    pub anchor: DiagnosticAnchor,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[cfg_attr(
    feature = "serde",
    serde(tag = "kind", content = "value", rename_all = "kebab-case")
)]
pub enum ExpectedSyntax {
    Token(SyntaxKind),
    Production(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct FoundSyntax {
    pub kind: Option<SyntaxKind>,
    pub text: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "kebab-case"))]
pub enum FixApplicability {
    MachineApplicable,
    MaybeIncorrect,
    HasPlaceholders,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct DiagnosticFix {
    pub title: String,
    pub applicability: FixApplicability,
    pub edits: Vec<TextEdit>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[cfg_attr(feature = "serde", serde(tag = "kind", rename_all = "kebab-case"))]
pub enum RecoveryAction {
    Insert {
        syntax: ExpectedSyntax,
        at: TextSize,
    },
    Skip {
        range: TextRange,
    },
    Abandon {
        rule: RuleId,
        at: TextSize,
    },
    ResourceLimit {
        range: TextRange,
    },
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct DiagnosticTags(pub u16);

impl DiagnosticTags {
    pub const NONE: Self = Self(0);
    pub const UNNECESSARY: Self = Self(1 << 0);
    pub const DEPRECATED: Self = Self(1 << 1);
    pub const SUPPRESSED_CASCADE: Self = Self(1 << 2);

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct Diagnostic {
    pub id: DiagnosticId,
    pub code: DiagnosticCode,
    pub phase: DiagnosticPhase,
    pub severity: Severity,
    pub rule: Option<RuleId>,
    pub context: Option<ParserContextId>,
    pub primary: DiagnosticAnchor,
    pub labels: Vec<DiagnosticLabel>,
    pub expected: Vec<ExpectedSyntax>,
    pub found: Option<FoundSyntax>,
    pub fixes: Vec<DiagnosticFix>,
    pub related: Vec<DiagnosticId>,
    pub recovery: Option<RecoveryAction>,
    pub tags: DiagnosticTags,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedDiagnosticLabel {
    pub range: Option<TextRange>,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedDiagnosticFix {
    pub title: String,
    pub applicability: FixApplicability,
    pub edits: Vec<TextEdit>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedDiagnostic {
    pub code: DiagnosticCode,
    pub phase: DiagnosticPhase,
    pub severity: Severity,
    pub rule: Option<RuleId>,
    pub context: Option<ParserContextId>,
    pub primary: Option<TextRange>,
    pub labels: Vec<NormalizedDiagnosticLabel>,
    pub expected: Vec<ExpectedSyntax>,
    pub found: Option<FoundSyntax>,
    pub fixes: Vec<NormalizedDiagnosticFix>,
    pub related: Vec<usize>,
    pub recovery: Option<RecoveryAction>,
    pub tags: DiagnosticTags,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct DiagnosticStore {
    pub revision: Revision,
    diagnostics: Vec<Diagnostic>,
}

impl DiagnosticStore {
    pub fn new(revision: Revision) -> Self {
        Self {
            revision,
            diagnostics: Vec::new(),
        }
    }

    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics.iter()
    }

    pub fn as_slice(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn len(&self) -> usize {
        self.diagnostics.len()
    }

    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    pub fn retain_resolvable(mut self, revision: Revision, nodes: &NodeIndex) -> Self {
        self.diagnostics
            .retain(|diagnostic| diagnostic.primary.resolve(revision, nodes).is_some());
        self.revision = revision;
        self
    }

    #[cfg(feature = "serde")]
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

pub fn normalize_diagnostics(
    store: &DiagnosticStore,
    revision: Revision,
    nodes: &NodeIndex,
) -> Vec<NormalizedDiagnostic> {
    normalize_diagnostic_indices(
        store,
        revision,
        nodes,
        &(0..store.len()).collect::<Vec<_>>(),
    )
}

pub fn normalize_diagnostics_in_range(
    store: &DiagnosticStore,
    revision: Revision,
    nodes: &NodeIndex,
    range: TextRange,
) -> Vec<NormalizedDiagnostic> {
    let indices = store
        .iter()
        .enumerate()
        .filter_map(|(index, diagnostic)| {
            let primary = diagnostic.primary.resolve(revision, nodes)?;
            let belongs = if primary.is_empty() {
                range.contains_inclusive(primary.start)
            } else {
                range.contains_range(primary)
            };
            belongs.then_some(index)
        })
        .collect::<Vec<_>>();
    normalize_diagnostic_indices(store, revision, nodes, &indices)
}

fn normalize_diagnostic_indices(
    store: &DiagnosticStore,
    revision: Revision,
    nodes: &NodeIndex,
    indices: &[usize],
) -> Vec<NormalizedDiagnostic> {
    let related_indices = indices
        .iter()
        .enumerate()
        .map(|(normalized, original)| (store.diagnostics[*original].id, normalized))
        .collect::<BTreeMap<_, _>>();
    indices
        .iter()
        .map(|index| &store.diagnostics[*index])
        .map(|diagnostic| NormalizedDiagnostic {
            code: diagnostic.code.clone(),
            phase: diagnostic.phase,
            severity: diagnostic.severity,
            rule: diagnostic.rule,
            context: diagnostic.context,
            primary: diagnostic.primary.resolve(revision, nodes),
            labels: diagnostic
                .labels
                .iter()
                .map(|label| NormalizedDiagnosticLabel {
                    range: label.anchor.resolve(revision, nodes),
                    message: label.message.clone(),
                })
                .collect(),
            expected: diagnostic.expected.clone(),
            found: diagnostic.found.clone(),
            fixes: diagnostic
                .fixes
                .iter()
                .map(|fix| NormalizedDiagnosticFix {
                    title: fix.title.clone(),
                    applicability: fix.applicability,
                    edits: fix.edits.clone(),
                })
                .collect(),
            related: diagnostic
                .related
                .iter()
                .filter_map(|id| related_indices.get(id).copied())
                .collect(),
            recovery: diagnostic.recovery.clone(),
            tags: diagnostic.tags,
        })
        .collect()
}

pub fn render_plain(diagnostic: &Diagnostic, source: &TextSnapshot, nodes: &NodeIndex) -> String {
    let range = diagnostic
        .primary
        .resolve(source.revision(), nodes)
        .unwrap_or_else(|| TextRange::empty(TextSize::ZERO));
    let (line, column) = source.line_index().line_and_byte_column(range.start);
    let mut output = String::new();
    let _ = writeln!(
        output,
        "{:?}[{}] at {}:{}: {}",
        diagnostic.severity,
        diagnostic.code.as_str(),
        line + 1,
        column.0 + 1,
        diagnostic.message
    );
    for label in &diagnostic.labels {
        if let Some(label_range) = label.anchor.resolve(source.revision(), nodes) {
            let (label_line, label_column) =
                source.line_index().line_and_byte_column(label_range.start);
            let _ = writeln!(
                output,
                "  {}:{}: {}",
                label_line + 1,
                label_column.0 + 1,
                label.message
            );
        }
    }
    output
}

/// Console-style diagnostic presentation shared with passive editor clients.
/// Anchors are resolved against this source revision before excerpts are drawn.
pub fn render_pretty(diagnostic: &Diagnostic, source: &TextSnapshot, nodes: &NodeIndex) -> String {
    let mut output = format!(
        "{:?}[{}]: {}\n",
        diagnostic.severity,
        diagnostic.code.as_str(),
        diagnostic.message
    );
    if let Some(range) = diagnostic.primary.resolve(source.revision(), nodes) {
        output.push_str(&render_source_excerpt(source, range, &diagnostic.message));
    }
    for label in &diagnostic.labels {
        if let Some(range) = label.anchor.resolve(source.revision(), nodes) {
            output.push_str(&render_source_excerpt(source, range, &label.message));
        }
    }
    if !diagnostic.expected.is_empty() {
        let expected = diagnostic
            .expected
            .iter()
            .map(|expected| match expected {
                ExpectedSyntax::Production(name) => name.clone(),
                ExpectedSyntax::Token(kind) => match kind {
                    SyntaxKind::RightBracket => "`]`".into(),
                    SyntaxKind::RightParen => "`)`".into(),
                    SyntaxKind::RightBrace => "`}`".into(),
                    SyntaxKind::RightAngle => "`⟩`".into(),
                    SyntaxKind::Eof => "end of input".into(),
                    kind => format!("{kind:?}"),
                },
            })
            .collect::<Vec<_>>()
            .join(" or ");
        let _ = writeln!(output, "  = expected: {expected}");
    }
    for fix in &diagnostic.fixes {
        let _ = writeln!(output, "  = help: {}", fix.title);
        for edit in &fix.edits {
            if let Some(location) = source.source_location(edit.delete.start) {
                let _ = writeln!(
                    output,
                    "    at {}:{}: replace {} source bytes with {:?}",
                    location.row,
                    location.col,
                    edit.delete.len().0,
                    edit.insert
                );
            }
        }
    }
    output
}

/// Draw a source range, including one surrounding line and an insertion caret
/// for empty ranges. UTF-8 byte anchors are projected to source coordinates;
/// tab expansion and Unicode display width determine underline alignment.
pub fn render_source_excerpt(source: &TextSnapshot, range: TextRange, message: &str) -> String {
    if source.validate_range(range).is_err() {
        return String::new();
    }
    let index = source.line_index();
    let first = index.line_of(range.start);
    let last = index.line_of(if range.is_empty() {
        range.end
    } else {
        TextSize(range.end.0 - 1)
    });
    let start = first.saturating_sub(1);
    let end = (last + 1).min(index.line_count() - 1);
    let width = (end + 1).to_string().len();
    let location = source.source_location(range.start);
    let mut output = location
        .map(|at| format!(" --> document:{}:{}\n", at.row, at.col))
        .unwrap_or_default();
    let _ = writeln!(output, "{:width$} |", "");
    for line in start..=end {
        if last > first + 4 && line > first + 1 && line < last - 1 {
            if line == first + 2 {
                let _ = writeln!(output, "{:width$} | …", "");
            }
            continue;
        }
        let line_start = index.line_start(line).unwrap();
        let line_end = index
            .line_start(line + 1)
            .unwrap_or_else(|| source.byte_len());
        let text = source
            .text(TextRange::new(line_start, line_end))
            .unwrap_or_default();
        let text = text.trim_end_matches(['\r', '\n']);
        let _ = writeln!(output, "{:>width$} | {}", line + 1, display_source(text));
        if line >= first && line <= last {
            let from = range
                .start
                .0
                .saturating_sub(line_start.0)
                .min(text.len() as u32) as usize;
            let to = range
                .end
                .0
                .saturating_sub(line_start.0)
                .min(text.len() as u32) as usize;
            let prefix = display_source(&text[..from]);
            let through = display_source(&text[..to]);
            let indent = unicode_width::UnicodeWidthStr::width(prefix.as_str());
            let length = unicode_width::UnicodeWidthStr::width(through.as_str())
                .saturating_sub(indent)
                .max(1);
            let _ = writeln!(
                output,
                "{:width$} | {}{}{}",
                "",
                " ".repeat(indent),
                "^".repeat(length),
                if line == last {
                    format!(" {message}")
                } else {
                    String::new()
                }
            );
        }
    }
    output
}

fn display_source(text: &str) -> String {
    let mut output = String::new();
    for ch in text.chars() {
        if ch == '\t' {
            let count = 2 - unicode_width::UnicodeWidthStr::width(output.as_str()) % 2;
            output.push_str(&" ".repeat(count));
        } else if ch.is_control() {
            let escaped = format!("{ch:?}");
            output.push_str(&escaped);
        } else {
            output.push(ch);
        }
    }
    output
}

pub fn anchor_flags(anchor: &DiagnosticAnchor, nodes: &NodeIndex) -> Option<NodeFlags> {
    let DiagnosticAnchor::Element { element, .. } = anchor else {
        return None;
    };
    match element {
        SyntaxElementId::Node(id) => nodes.node(*id).map(|record| record.flags),
        SyntaxElementId::Token(id) => nodes
            .token(*id)
            .and_then(|record| nodes.node(record.parent))
            .map(|record| record.flags),
    }
}
