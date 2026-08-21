//! Portable interactive-host events.
//!
//! These records describe what happened without prescribing where a frontend
//! renders it. Terminal, browser, desktop, and notebook hosts can therefore
//! consume the same ordered event stream.

#[cfg(feature = "no_std")]
use alloc::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    format,
    string::String,
    vec::Vec,
};
use core::fmt::{Display, Formatter};
#[cfg(not(feature = "no_std"))]
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub const MECH_EVENT_PROTOCOL_VERSION: u16 = 1;

/// Semantic actions produced by the standard REPL keyboard contract.
///
/// Hosts own their editor widgets and cursor placement. This contract keeps
/// submission behavior identical without putting DOM or terminal concerns in
/// the runtime: Enter submits, while Ctrl+Enter inserts a line break.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplInputAction {
    Submit,
    InsertLineBreak,
}

impl ReplInputAction {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Submit => "submit",
            Self::InsertLineBreak => "insert_line_break",
        }
    }
}

impl Display for ReplInputAction {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A renderer-neutral keyboard gesture from a REPL editor.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplInputGesture {
    pub key: String,
    pub control: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
}

impl ReplInputGesture {
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            control: false,
            alt: false,
            shift: false,
            meta: false,
        }
    }

    pub fn action(&self) -> Option<ReplInputAction> {
        if self.key != "Enter" {
            return None;
        }
        Some(if self.control {
            ReplInputAction::InsertLineBreak
        } else {
            ReplInputAction::Submit
        })
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MechEventEnvelope {
    pub protocol_version: u16,
    pub sequence: u64,
    pub event: MechEvent,
}

impl MechEventEnvelope {
    pub fn new(sequence: u64, event: MechEvent) -> Self {
        Self {
            protocol_version: MECH_EVENT_PROTOCOL_VERSION,
            sequence,
            event,
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(tag = "channel", content = "event", rename_all = "snake_case")
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MechEvent {
    Repl(ReplEvent),
    Output(OutputEvent),
    Diagnostic(DiagnosticEvent),
    Telemetry(TelemetryEvent),
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(tag = "kind", content = "payload", rename_all = "snake_case")
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplEvent {
    SourceEcho {
        source: String,
    },
    Response(ReplResponse),
    FocusDisplay {
        display_id: DisplayId,
        #[cfg_attr(feature = "serde", serde(default))]
        stream: OutputStream,
        content: OutputContent,
    },
    Clear(ReplClearTarget),
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplClearTarget {
    Interaction,
    Diagnostics,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplResponse {
    pub kind: ReplResponseKind,
    pub status: ReplResponseStatus,
    pub title: Option<String>,
    pub content: OutputContent,
}

impl ReplResponse {
    pub fn new(
        kind: ReplResponseKind,
        status: ReplResponseStatus,
        title: Option<String>,
        content: OutputContent,
    ) -> Self {
        Self {
            kind,
            status,
            title,
            content,
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplResponseKind {
    Command,
    Help,
    SymbolInspection,
    IntegrityConstraintInspection,
    ValueInspection,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplResponseStatus {
    Neutral,
    Info,
    Success,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DisplayId(pub String);

impl DisplayId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for DisplayId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DiagnosticId(pub String);

impl DiagnosticId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for DiagnosticId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(tag = "kind", content = "details", rename_all = "snake_case")
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OutputSource {
    Program {
        span: Option<SourceSpan>,
    },
    Host {
        name: String,
        span: Option<SourceSpan>,
    },
}

impl OutputSource {
    pub fn program() -> Self {
        Self::Program { span: None }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayOperation {
    Append,
    Create,
    Update,
    Replace,
    Clear,
    Remove,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum OutputStream {
    Stdout,
    Stderr,
}

impl Default for OutputStream {
    fn default() -> Self {
        Self::Stdout
    }
}

impl Display for OutputStream {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(match self {
            Self::Stdout => "stdout",
            Self::Stderr => "stderr",
        })
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputEvent {
    pub source: OutputSource,
    #[cfg_attr(feature = "serde", serde(default))]
    pub stream: OutputStream,
    pub display_id: Option<DisplayId>,
    pub operation: DisplayOperation,
    pub content: OutputContent,
}

impl OutputEvent {
    pub fn text(text: impl Into<String>) -> Self {
        Self::stream_text(OutputStream::Stdout, text)
    }

    pub fn stream_text(stream: OutputStream, text: impl Into<String>) -> Self {
        Self {
            source: OutputSource::program(),
            stream,
            display_id: None,
            operation: DisplayOperation::Append,
            content: OutputContent::Text(TextOutput::new(text)),
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(tag = "kind", content = "data", rename_all = "snake_case")
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OutputContent {
    Text(TextOutput),
    Value(ValueOutput),
    Table(TableOutput),
    Matrix(MatrixOutput),
    Plot(PlotOutput),
    Scene(SceneOutput),
    Image(ImageOutput),
    Custom(RichOutput),
    /// Ordered content retained after heterogeneous append operations.
    Fragments(Vec<OutputContent>),
}

impl OutputContent {
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::Text(_) => "text",
            Self::Value(_) => "value",
            Self::Table(_) => "table",
            Self::Matrix(_) => "matrix",
            Self::Plot(_) => "plot",
            Self::Scene(_) => "scene",
            Self::Image(_) => "image",
            Self::Custom(_) => "custom",
            Self::Fragments(_) => "fragments",
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextOutput {
    pub text: String,
}

impl TextOutput {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValueOutput {
    pub kind: String,
    /// Canonical single-line Mech syntax. This is the primary portable value
    /// payload consumed by rich and browser hosts.
    pub text: String,
    /// Backward-compatible canonical fallback for plain and constrained displays.
    #[cfg_attr(feature = "serde", serde(default))]
    pub inline_text: String,
}

impl ValueOutput {
    pub fn new(kind: impl Into<String>, text: impl Into<String>) -> Self {
        let text = text.into();
        Self {
            kind: kind.into(),
            inline_text: text.clone(),
            text,
        }
    }

    pub fn with_inline_text(mut self, inline_text: impl Into<String>) -> Self {
        self.inline_text = inline_text.into();
        self
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableOutput {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    /// Zero-based row indexes that hosts should present as secondary or disabled.
    ///
    /// This is a semantic presentation hint. Plain hosts may ignore it, while
    /// visual hosts can dim the corresponding rows without parsing cell text.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub muted_rows: Vec<usize>,
    /// Opaque, session-owned selection tokens aligned with `rows`.
    ///
    /// Visual hosts may use these to make an inspection row interactive
    /// without resolving its display name again. A token denotes the detached
    /// value captured for that exact row, so transcript history remains stable
    /// even after a resident name is rebound or an output slot is reused.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub row_selection_tokens: Vec<Option<String>>,
}

impl TableOutput {
    pub fn new(columns: Vec<String>, rows: Vec<Vec<String>>) -> Self {
        Self {
            columns,
            rows,
            muted_rows: Vec::new(),
            row_selection_tokens: Vec::new(),
        }
    }

    pub fn with_muted_rows(mut self, muted_rows: Vec<usize>) -> Self {
        self.muted_rows = muted_rows;
        self
    }

    pub fn with_row_selection_tokens(mut self, row_selection_tokens: Vec<Option<String>>) -> Self {
        self.row_selection_tokens = row_selection_tokens;
        self
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatrixOutput {
    pub element_kind: String,
    pub rows: usize,
    pub columns: usize,
    pub cells: Vec<String>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlotOutput {
    pub representations: RichOutput,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneOutput {
    pub representations: RichOutput,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageOutput {
    pub alt_text: Option<String>,
    pub representations: RichOutput,
}

impl ImageOutput {
    /// Text suitable for hosts that cannot render an image representation.
    pub fn text_fallback(&self) -> Option<&str> {
        self.representations
            .text_fallback()
            .or(self.alt_text.as_deref())
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RichOutput {
    pub representations: Vec<Representation>,
}

impl RichOutput {
    pub fn new(representations: Vec<Representation>) -> Self {
        Self { representations }
    }

    pub fn text_fallback(&self) -> Option<&str> {
        self.representations.iter().find_map(|representation| {
            if representation.media_type == "text/plain" {
                match &representation.data {
                    RepresentationData::Text(text) => Some(text.as_str()),
                    RepresentationData::Binary(_) => None,
                }
            } else {
                None
            }
        })
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Representation {
    pub media_type: String,
    pub data: RepresentationData,
}

impl Representation {
    pub fn text(media_type: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            media_type: media_type.into(),
            data: RepresentationData::Text(text.into()),
        }
    }

    pub fn binary(media_type: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self {
            media_type: media_type.into(),
            data: RepresentationData::Binary(bytes),
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(tag = "encoding", content = "value", rename_all = "snake_case")
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RepresentationData {
    Text(String),
    Binary(Vec<u8>),
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticEvent {
    pub id: DiagnosticId,
    #[cfg_attr(feature = "serde", serde(default))]
    pub owner: DiagnosticOwner,
    pub severity: Severity,
    pub phase: DiagnosticPhase,
    pub code: Option<String>,
    pub message: String,
    pub source: Option<SourceSpan>,
    pub notes: Vec<DiagnosticNote>,
    pub related: Vec<RelatedDiagnostic>,
}

/// The producer that owns a diagnostic and therefore its presentation route.
///
/// Program diagnostics belong in a host diagnostic surface. Interaction
/// diagnostics describe source or commands entered through a REPL and belong
/// alongside that interaction history.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DiagnosticOwner {
    #[default]
    Program,
    Interaction,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Info,
    Warning,
    Error,
    Fatal,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticPhase {
    Parse,
    Compile,
    Plan,
    Execute,
    Capability,
    Host,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceSpan {
    pub source: Option<String>,
    pub start: SourcePosition,
    pub end: SourcePosition,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourcePosition {
    pub line: usize,
    pub column: usize,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticNote {
    pub message: String,
    pub source: Option<SourceSpan>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelatedDiagnostic {
    pub id: DiagnosticId,
    pub message: String,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(tag = "kind", content = "payload", rename_all = "snake_case")
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TelemetryEvent {
    Profile { name: String, value: String },
    Trace { message: String },
    Timing { name: String, duration_ns: u64 },
    Debug { message: String },
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisplayCapability {
    pub name: String,
    pub support: DisplaySupport,
    pub fallback: Option<String>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplaySupport {
    Supported,
    Fallback,
    Unavailable,
}

impl Display for DisplaySupport {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(match self {
            Self::Supported => "yes",
            Self::Fallback => "fallback",
            Self::Unavailable => "unavailable",
        })
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputArtifactStatus {
    Active,
    Cleared,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputArtifact {
    pub id: DisplayId,
    pub event: OutputEvent,
    pub status: OutputArtifactStatus,
}

/// Ordered, renderer-neutral state for one interactive session.
///
/// Frontends drain envelopes at their own pace while the bus retains stable
/// display artifacts and diagnostics for later inspection.
#[derive(Debug, Default)]
pub struct MechEventBus {
    pending: VecDeque<MechEventEnvelope>,
    outputs: BTreeMap<DisplayId, OutputArtifact>,
    output_order: Vec<DisplayId>,
    reserved_outputs: BTreeSet<DisplayId>,
    anonymous_streams: BTreeMap<AnonymousStream, DisplayId>,
    diagnostics: BTreeMap<DiagnosticId, DiagnosticEvent>,
    diagnostic_order: Vec<DiagnosticId>,
    next_sequence: u64,
    next_output: u64,
}

impl MechEventBus {
    pub fn publish(&mut self, mut event: MechEvent) -> u64 {
        match &mut event {
            MechEvent::Output(output) => {
                self.assign_output_identity(output);
                self.record_output(output.clone());
            }
            MechEvent::Diagnostic(diagnostic) => {
                if !self.diagnostics.contains_key(&diagnostic.id) {
                    self.diagnostic_order.push(diagnostic.id.clone());
                }
                self.diagnostics
                    .insert(diagnostic.id.clone(), diagnostic.clone());
            }
            MechEvent::Repl(_) | MechEvent::Telemetry(_) => {}
        }

        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.saturating_add(1);
        self.coalesce_pending_update(&event);
        self.pending
            .push_back(MechEventEnvelope::new(sequence, event));
        sequence
    }

    pub fn publish_all(&mut self, events: impl IntoIterator<Item = MechEvent>) {
        for event in events {
            self.publish(event);
        }
    }

    pub fn drain(&mut self) -> Vec<MechEventEnvelope> {
        self.pending.drain(..).collect()
    }

    pub fn outputs(&self) -> Vec<OutputArtifact> {
        self.output_order
            .iter()
            .filter_map(|id| self.outputs.get(id).cloned())
            .collect()
    }

    pub fn output(&self, id: &str) -> Option<OutputArtifact> {
        self.outputs.get(&DisplayId::new(id)).cloned()
    }

    pub fn diagnostics(&self) -> Vec<DiagnosticEvent> {
        self.diagnostic_order
            .iter()
            .filter_map(|id| self.diagnostics.get(id).cloned())
            .collect()
    }

    pub fn clear_outputs(&mut self, source: OutputSource) {
        self.publish(MechEvent::Output(OutputEvent {
            source,
            stream: OutputStream::Stdout,
            display_id: None,
            operation: DisplayOperation::Clear,
            content: OutputContent::Text(TextOutput::new("")),
        }));
    }

    pub fn clear_diagnostics(&mut self) {
        self.diagnostics.clear();
        self.diagnostic_order.clear();
        self.publish(MechEvent::Repl(ReplEvent::Clear(
            ReplClearTarget::Diagnostics,
        )));
    }

    /// Retain only the newest undelivered update for a stable display. Create,
    /// replace, clear, and remove remain explicit lifecycle boundaries.
    fn coalesce_pending_update(&mut self, event: &MechEvent) {
        let MechEvent::Output(incoming) = event else {
            return;
        };
        if incoming.operation != DisplayOperation::Update {
            return;
        }
        let Some(display_id) = incoming.display_id.as_ref() else {
            return;
        };
        for (index, envelope) in self.pending.iter().enumerate().rev() {
            let MechEvent::Output(previous) = &envelope.event else {
                continue;
            };
            if previous.operation == DisplayOperation::Clear && previous.display_id.is_none() {
                return;
            }
            if previous.display_id.as_ref() == Some(display_id) {
                if previous.operation == DisplayOperation::Update {
                    self.pending.remove(index);
                }
                return;
            }
        }
    }

    fn record_output(&mut self, output: OutputEvent) {
        if output.display_id.is_none() && output.operation == DisplayOperation::Clear {
            self.outputs.clear();
            self.output_order.clear();
            self.anonymous_streams.clear();
            return;
        }

        let id = output
            .display_id
            .as_ref()
            .expect("non-global output events receive an identity before recording")
            .clone();
        self.reserved_outputs.insert(id.clone());

        match output.operation {
            DisplayOperation::Remove => {
                if self.outputs.remove(&id).is_some() {
                    self.output_order.retain(|existing| existing != &id);
                }
            }
            DisplayOperation::Clear => {
                self.record_output_creation(&id);
                self.outputs.insert(
                    id.clone(),
                    OutputArtifact {
                        id,
                        event: output,
                        status: OutputArtifactStatus::Cleared,
                    },
                );
            }
            DisplayOperation::Append => {
                self.record_output_creation(&id);
                let mut artifact = OutputArtifact {
                    id: id.clone(),
                    event: output,
                    status: OutputArtifactStatus::Active,
                };
                if let Some(previous) = self.outputs.get(&id) {
                    if previous.status == OutputArtifactStatus::Active {
                        artifact.event.content =
                            append_content(previous.event.content.clone(), artifact.event.content);
                    }
                }
                self.outputs.insert(id, artifact);
            }
            DisplayOperation::Create | DisplayOperation::Update | DisplayOperation::Replace => {
                self.record_output_creation(&id);
                self.outputs.insert(
                    id.clone(),
                    OutputArtifact {
                        id,
                        event: output,
                        status: OutputArtifactStatus::Active,
                    },
                );
            }
        }
    }

    fn record_output_creation(&mut self, id: &DisplayId) {
        if !self.outputs.contains_key(id) {
            self.output_order.push(id.clone());
        }
    }

    /// Normalize host-produced anonymous output before it enters either the
    /// retained journal or the portable event stream. Append fragments share
    /// one logical display per source and stdout/stderr stream; other
    /// anonymous display operations receive a fresh stable handle.
    fn assign_output_identity(&mut self, output: &mut OutputEvent) {
        if output.display_id.is_some() || output.operation == DisplayOperation::Clear {
            return;
        }

        let id = if output.operation == DisplayOperation::Append {
            let stream = AnonymousStream::from_output(output);
            if let Some(id) = self.anonymous_streams.get(&stream) {
                id.clone()
            } else {
                let id = self.allocate_output_id();
                self.anonymous_streams.insert(stream, id.clone());
                id
            }
        } else {
            self.allocate_output_id()
        };
        output.display_id = Some(id);
    }

    fn allocate_output_id(&mut self) -> DisplayId {
        loop {
            self.next_output = self.next_output.wrapping_add(1);
            let id = DisplayId::new(format!("output-{}", self.next_output));
            if self.reserved_outputs.insert(id.clone()) {
                return id;
            }
        }
    }
}

fn append_content(previous: OutputContent, next: OutputContent) -> OutputContent {
    let mut fragments = Vec::new();
    push_fragment(&mut fragments, previous);
    push_fragment(&mut fragments, next);
    if fragments.len() == 1 {
        fragments.pop().expect("one retained output fragment")
    } else {
        OutputContent::Fragments(fragments)
    }
}

fn push_fragment(fragments: &mut Vec<OutputContent>, content: OutputContent) {
    match content {
        OutputContent::Fragments(nested) => {
            for content in nested {
                push_fragment(fragments, content);
            }
        }
        OutputContent::Text(next) => {
            if let Some(OutputContent::Text(previous)) = fragments.last_mut() {
                previous.text.push_str(&next.text);
            } else {
                fragments.push(OutputContent::Text(next));
            }
        }
        content => fragments.push(content),
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct AnonymousStream {
    source: AnonymousOutputSource,
    stream: OutputStream,
}

impl AnonymousStream {
    fn from_output(output: &OutputEvent) -> Self {
        Self {
            source: match &output.source {
                OutputSource::Program { .. } => AnonymousOutputSource::Program,
                OutputSource::Host { name, .. } => AnonymousOutputSource::Host(name.clone()),
            },
            stream: output.stream,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum AnonymousOutputSource {
    Program,
    Host(String),
}

pub fn terminal_display_capabilities() -> Vec<DisplayCapability> {
    [
        ("display.text", DisplaySupport::Supported, None),
        ("display.value", DisplaySupport::Supported, None),
        ("display.table", DisplaySupport::Supported, None),
        ("display.matrix", DisplaySupport::Supported, None),
        (
            "display.plot.2d",
            DisplaySupport::Fallback,
            Some("text/plain"),
        ),
        (
            "display.scene.2d",
            DisplaySupport::Fallback,
            Some("text/plain"),
        ),
        ("display.scene.3d", DisplaySupport::Unavailable, None),
        (
            "display.image",
            DisplaySupport::Fallback,
            Some("text/plain"),
        ),
        (
            "display.animation",
            DisplaySupport::Fallback,
            Some("terminal"),
        ),
        ("display.interactive", DisplaySupport::Unavailable, None),
    ]
    .into_iter()
    .map(|(name, support, fallback)| DisplayCapability {
        name: name.into(),
        support,
        fallback: fallback.map(String::from),
    })
    .collect()
}

#[cfg(all(test, feature = "serde"))]
mod tests {
    use super::*;

    #[test]
    fn event_envelope_serializes_with_an_explicit_channel_and_version() {
        let envelope = MechEventEnvelope::new(
            7,
            MechEvent::Output(OutputEvent {
                source: OutputSource::program(),
                stream: OutputStream::Stdout,
                display_id: Some(DisplayId::new("nbody")),
                operation: DisplayOperation::Update,
                content: OutputContent::Scene(SceneOutput {
                    representations: RichOutput::new(vec![Representation::text(
                        "application/mech-scene+json",
                        r#"{"bodies":5}"#,
                    )]),
                }),
            }),
        );

        let json = serde_json::to_string(&envelope).unwrap();
        assert!(json.contains(r#""protocol_version":1"#));
        assert!(json.contains(r#""channel":"output""#));
        assert!(json.contains(r#""nbody""#));
        assert_eq!(
            serde_json::from_str::<MechEventEnvelope>(&json).unwrap(),
            envelope
        );
    }

    #[test]
    fn added_output_fields_have_compatible_plain_defaults() {
        let mut output = serde_json::to_value(OutputEvent::text("hello")).unwrap();
        output.as_object_mut().unwrap().remove("stream");
        let output: OutputEvent = serde_json::from_value(output).unwrap();
        assert_eq!(output.stream, OutputStream::Stdout);

        let mut focus = serde_json::to_value(MechEventEnvelope::new(
            8,
            MechEvent::Repl(ReplEvent::FocusDisplay {
                display_id: DisplayId::new("legacy-focus"),
                stream: OutputStream::Stderr,
                content: OutputContent::Text(TextOutput::new("legacy content")),
            }),
        ))
        .unwrap();
        focus
            .get_mut("event")
            .unwrap()
            .get_mut("event")
            .unwrap()
            .get_mut("payload")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove("stream");
        let focus: MechEventEnvelope = serde_json::from_value(focus).unwrap();
        assert!(matches!(
            focus.event,
            MechEvent::Repl(ReplEvent::FocusDisplay {
                display_id,
                stream: OutputStream::Stdout,
                content: OutputContent::Text(text),
            }) if display_id.as_str() == "legacy-focus" && text.text == "legacy content"
        ));

        let mut value = serde_json::to_value(ValueOutput::new("f64", "2")).unwrap();
        value.as_object_mut().unwrap().remove("inline_text");
        let value: ValueOutput = serde_json::from_value(value).unwrap();
        assert!(value.inline_text.is_empty());

        let mut table = serde_json::to_value(TableOutput::new(
            vec!["Command".to_string()],
            vec![vec![":help".to_string()]],
        ))
        .unwrap();
        table.as_object_mut().unwrap().remove("muted_rows");
        let table: TableOutput = serde_json::from_value(table).unwrap();
        assert!(table.muted_rows.is_empty());
    }

    #[test]
    fn table_row_presentation_hints_round_trip_without_changing_cells() {
        let table = TableOutput::new(
            vec!["Command".to_string(), "Description".to_string()],
            vec![
                vec![":help".to_string(), "show help".to_string()],
                vec![":load".to_string(), "load source".to_string()],
            ],
        )
        .with_muted_rows(vec![1]);

        let json = serde_json::to_string(&table).unwrap();
        assert!(json.contains(r#""muted_rows":[1]"#));
        assert_eq!(serde_json::from_str::<TableOutput>(&json).unwrap(), table);
    }

    #[test]
    fn repl_keyboard_contract_reserves_control_enter_for_multiline_input() {
        let enter = ReplInputGesture::new("Enter");
        let mut control_enter = ReplInputGesture::new("Enter");
        control_enter.control = true;
        let mut shifted_enter = ReplInputGesture::new("Enter");
        shifted_enter.shift = true;

        assert_eq!(enter.action(), Some(ReplInputAction::Submit));
        assert_eq!(
            control_enter.action(),
            Some(ReplInputAction::InsertLineBreak)
        );
        assert_eq!(shifted_enter.action(), Some(ReplInputAction::Submit));
        assert_eq!(ReplInputGesture::new("a").action(), None);
        assert_eq!(
            serde_json::to_string(&ReplInputAction::InsertLineBreak).unwrap(),
            r#""insert_line_break""#
        );
    }

    #[test]
    fn display_updates_replace_one_stable_artifact() {
        let mut bus = MechEventBus::default();
        for operation in [DisplayOperation::Create, DisplayOperation::Update] {
            bus.publish(MechEvent::Output(OutputEvent {
                source: OutputSource::program(),
                stream: OutputStream::Stdout,
                display_id: Some(DisplayId::new("nbody")),
                operation,
                content: OutputContent::Scene(SceneOutput {
                    representations: RichOutput::new(Vec::new()),
                }),
            }));
        }

        let outputs = bus.outputs();
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].id.as_str(), "nbody");
        assert_eq!(outputs[0].event.operation, DisplayOperation::Update);
    }

    #[test]
    fn pending_scene_updates_are_coalesced_without_erasing_lifecycle_boundaries() {
        let mut bus = MechEventBus::default();
        let scene = |operation, frame| {
            MechEvent::Output(OutputEvent {
                source: OutputSource::program(),
                stream: OutputStream::Stdout,
                display_id: Some(DisplayId::new("scene-orbit")),
                operation,
                content: OutputContent::Scene(SceneOutput {
                    representations: RichOutput::new(vec![Representation::text(
                        "application/vnd.mech.scene+json",
                        format!(r#"{{"frame":{frame}}}"#),
                    )]),
                }),
            })
        };
        bus.publish(scene(DisplayOperation::Create, 0));
        bus.publish(scene(DisplayOperation::Update, 1));
        bus.publish(scene(DisplayOperation::Update, 2));

        let pending = bus.drain();
        assert_eq!(pending.len(), 2);
        assert!(matches!(
            &pending[0].event,
            MechEvent::Output(event) if event.operation == DisplayOperation::Create
        ));
        assert_eq!(pending[1].event, scene(DisplayOperation::Update, 2));

        bus.publish(scene(DisplayOperation::Update, 3));
        bus.publish(scene(DisplayOperation::Remove, 3));
        bus.publish(scene(DisplayOperation::Update, 4));
        let pending = bus.drain();
        assert_eq!(pending.len(), 3);
        assert_eq!(pending[0].event, scene(DisplayOperation::Update, 3));
        assert_eq!(pending[1].event, scene(DisplayOperation::Remove, 3));
        assert_eq!(pending[2].event, scene(DisplayOperation::Update, 4));

        bus.publish(scene(DisplayOperation::Update, 5));
        bus.publish(MechEvent::Output(OutputEvent {
            source: OutputSource::program(),
            stream: OutputStream::Stdout,
            display_id: None,
            operation: DisplayOperation::Clear,
            content: OutputContent::Text(TextOutput::new("")),
        }));
        bus.publish(scene(DisplayOperation::Update, 6));
        let pending = bus.drain();
        assert_eq!(pending.len(), 3);
        assert_eq!(pending[0].event, scene(DisplayOperation::Update, 5));
        assert!(matches!(
            &pending[1].event,
            MechEvent::Output(event)
                if event.operation == DisplayOperation::Clear && event.display_id.is_none()
        ));
        assert_eq!(pending[2].event, scene(DisplayOperation::Update, 6));
    }

    #[test]
    fn heterogeneous_appends_retain_every_fragment_in_order() {
        let mut bus = MechEventBus::default();
        let id = Some(DisplayId::new("mixed"));
        let table = |value: &str| {
            OutputContent::Table(TableOutput::new(
                vec!["Value".to_string()],
                vec![vec![value.to_string()]],
            ))
        };
        for (operation, content) in [
            (DisplayOperation::Create, table("first")),
            (DisplayOperation::Append, table("second")),
            (
                DisplayOperation::Append,
                OutputContent::Text(TextOutput::new("third")),
            ),
            (
                DisplayOperation::Append,
                OutputContent::Text(TextOutput::new("-fourth")),
            ),
        ] {
            bus.publish(MechEvent::Output(OutputEvent {
                source: OutputSource::program(),
                stream: OutputStream::Stdout,
                display_id: id.clone(),
                operation,
                content,
            }));
        }

        let artifact = bus.output("mixed").expect("retained mixed display");
        let OutputContent::Fragments(fragments) = artifact.event.content else {
            panic!("heterogeneous append composition")
        };
        assert_eq!(fragments.len(), 3);
        assert_eq!(fragments[0], table("first"));
        assert_eq!(fragments[1], table("second"));
        assert_eq!(
            fragments[2],
            OutputContent::Text(TextOutput::new("third-fourth"))
        );
    }

    #[test]
    fn anonymous_stream_fragments_receive_stable_portable_display_ids() {
        let mut bus = MechEventBus::default();
        bus.publish(MechEvent::Output(OutputEvent::text("warn")));
        bus.publish(MechEvent::Output(OutputEvent::text("ing\n")));
        bus.publish(MechEvent::Output(OutputEvent::stream_text(
            OutputStream::Stderr,
            "problem\n",
        )));

        let events = bus.drain();
        let ids = events
            .iter()
            .map(|envelope| match &envelope.event {
                MechEvent::Output(output) => output.display_id.as_ref().unwrap().as_str(),
                event => panic!("expected output event, got {event:?}"),
            })
            .collect::<Vec<_>>();
        assert_eq!(ids, ["output-1", "output-1", "output-2"]);

        let outputs = bus.outputs();
        assert_eq!(outputs.len(), 2);
        assert_eq!(
            outputs[0].event.content,
            OutputContent::Text(TextOutput::new("warning\n"))
        );
        assert_eq!(outputs[1].event.stream, OutputStream::Stderr);
    }

    #[test]
    fn generated_display_ids_skip_named_and_retired_handles() {
        let mut bus = MechEventBus::default();
        bus.publish(MechEvent::Output(OutputEvent {
            source: OutputSource::program(),
            stream: OutputStream::Stdout,
            display_id: Some(DisplayId::new("output-1")),
            operation: DisplayOperation::Create,
            content: OutputContent::Text(TextOutput::new("named")),
        }));
        bus.publish(MechEvent::Output(OutputEvent::text("anonymous")));

        assert!(bus.output("output-1").is_some());
        assert_eq!(
            bus.output("output-2").unwrap().event.content,
            OutputContent::Text(TextOutput::new("anonymous"))
        );

        bus.publish(MechEvent::Output(OutputEvent {
            source: OutputSource::program(),
            stream: OutputStream::Stdout,
            display_id: Some(DisplayId::new("output-2")),
            operation: DisplayOperation::Remove,
            content: OutputContent::Text(TextOutput::new("")),
        }));
        bus.clear_outputs(OutputSource::program());
        bus.publish(MechEvent::Output(OutputEvent::text("after clear")));

        assert!(bus.output("output-1").is_none());
        assert!(bus.output("output-2").is_none());
        assert!(bus.output("output-3").is_some());
    }

    #[test]
    fn artifact_histories_preserve_creation_order_not_identifier_order() {
        fn named_output(id: &str, operation: DisplayOperation) -> MechEvent {
            MechEvent::Output(OutputEvent {
                source: OutputSource::program(),
                stream: OutputStream::Stdout,
                display_id: Some(DisplayId::new(id)),
                operation,
                content: OutputContent::Text(TextOutput::new(id)),
            })
        }

        fn diagnostic(id: &str, message: &str) -> MechEvent {
            MechEvent::Diagnostic(DiagnosticEvent {
                id: DiagnosticId::new(id),
                owner: DiagnosticOwner::Program,
                severity: Severity::Warning,
                phase: DiagnosticPhase::Execute,
                code: None,
                message: message.to_string(),
                source: None,
                notes: Vec::new(),
                related: Vec::new(),
            })
        }

        let mut bus = MechEventBus::default();
        bus.publish(named_output("z-first", DisplayOperation::Create));
        for ordinal in 1..=11 {
            bus.publish(MechEvent::Output(OutputEvent {
                source: OutputSource::program(),
                stream: OutputStream::Stdout,
                display_id: None,
                operation: DisplayOperation::Create,
                content: OutputContent::Text(TextOutput::new(format!("anonymous-{ordinal}"))),
            }));
        }
        bus.publish(named_output("a-last", DisplayOperation::Create));

        let expected = core::iter::once("z-first".to_string())
            .chain((1..=11).map(|ordinal| format!("output-{ordinal}")))
            .chain(core::iter::once("a-last".to_string()))
            .collect::<Vec<_>>();
        assert_eq!(
            bus.outputs()
                .into_iter()
                .map(|artifact| artifact.id.as_str().to_string())
                .collect::<Vec<_>>(),
            expected
        );

        bus.publish(named_output("output-2", DisplayOperation::Remove));
        bus.publish(named_output("output-2", DisplayOperation::Create));
        assert_eq!(bus.outputs().last().unwrap().id.as_str(), "output-2");

        for (id, message) in [
            ("diagnostic-2", "first"),
            ("diagnostic-10", "second"),
            ("diagnostic-1", "third"),
            ("diagnostic-10", "second revised"),
        ] {
            bus.publish(diagnostic(id, message));
        }
        let diagnostics = bus.diagnostics();
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.id.as_str())
                .collect::<Vec<_>>(),
            vec!["diagnostic-2", "diagnostic-10", "diagnostic-1"]
        );
        assert_eq!(diagnostics[1].message, "second revised");
    }
}
