//! Structured native collection loops, sharing the existing lexical CFG proof.
//! These are private-constructor compiler views of canonical comprehensions,
//! not another source artifact, interpreter, or executable callback authority.
use super::{
    NativeIterationLoop, NativeMatchOrigin, NativeMatchValue, NativeProgramLibraryTemplate,
};

/// A source strategy derived from actual selected canonical operations. A
/// cursor is NOT a Matrix Value: its semantic result slot is never initialized.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub enum NativeCollectionSource {
    Value(NativeMatchValue),
    U64Range(NativeU64RangeCursor),
}
impl NativeCollectionSource {
    /// Original canonical identity, including an explicitly elided result.
    pub const fn semantic_value(&self) -> NativeMatchValue {
        match self {
            Self::Value(value) => *value,
            Self::U64Range(cursor) => cursor.output,
        }
    }
    pub const fn materialized_value(&self) -> Option<NativeMatchValue> {
        match self {
            Self::Value(value) => Some(*value),
            Self::U64Range(_) => None,
        }
    }
}

/// Exact maintained-factory proof for an unstepped U64 range used by one
/// generator. Preparation remains at the original range's error boundary.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct NativeU64RangeCursor {
    pub(crate) prepare: u32,
    pub(crate) template: NativeProgramLibraryTemplate,
    pub(crate) operands: [NativeMatchValue; 2],
    pub(crate) output: NativeMatchValue,
    pub(crate) before_collection_begin: bool,
}
impl NativeU64RangeCursor {
    pub const fn prepare(&self) -> u32 {
        self.prepare
    }
    pub fn template(&self) -> &NativeProgramLibraryTemplate {
        &self.template
    }
    pub const fn operands(&self) -> &[NativeMatchValue; 2] {
        &self.operands
    }
    pub const fn semantic_output(&self) -> NativeMatchValue {
        self.output
    }
    pub const fn before_collection_begin(&self) -> bool {
        self.before_collection_begin
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub enum NativeControlLoop {
    Iteration(NativeIterationLoop),
    Collection(NativeCollectionLoop),
    Tail(NativeTailLoop),
}
impl NativeControlLoop {
    pub fn as_ref(&self) -> NativeControlLoopRef<'_> {
        self.into()
    }
    pub fn as_iteration(&self) -> Option<&NativeIterationLoop> {
        match self {
            Self::Iteration(value) => Some(value),
            Self::Collection(_) => None,
            Self::Tail(_) => None,
        }
    }
}

#[derive(Clone, Copy, Debug, serde::Serialize)]
pub enum NativeControlLoopRef<'a> {
    Iteration(&'a NativeIterationLoop),
    Collection(&'a NativeCollectionLoop),
    Tail(&'a NativeTailLoop),
}
impl<'a> From<&'a NativeIterationLoop> for NativeControlLoopRef<'a> {
    fn from(value: &'a NativeIterationLoop) -> Self {
        Self::Iteration(value)
    }
}
impl<'a> From<&'a NativeControlLoop> for NativeControlLoopRef<'a> {
    fn from(value: &'a NativeControlLoop) -> Self {
        match value {
            NativeControlLoop::Iteration(value) => Self::Iteration(value),
            NativeControlLoop::Collection(value) => Self::Collection(value),
            NativeControlLoop::Tail(value) => Self::Tail(value),
        }
    }
}
macro_rules! common_loop_accessors {
    ($($name:ident -> $result:ty),* $(,)?) => {$ (
        pub fn $name(self) -> $result {
            match self {
                Self::Iteration(value) => value.$name(),
                Self::Collection(value) => value.$name(),
                Self::Tail(value) => value.$name(),
            }
        }
    )*};
}
impl NativeControlLoopRef<'_> {
    common_loop_accessors! {
        origin -> NativeMatchOrigin,
        parent -> Option<u32>,
        initialize -> u32,
        header -> u32,
        body -> u32,
        latch -> u32,
        exit -> u32,
    }
}

/// One source generator. Its element is defined by an explicit generated
/// element-read visit, never carried forward from a previous body invocation.
#[derive(Clone, Debug, serde::Serialize)]
pub struct NativeCollectionLoop {
    pub(crate) origin: NativeMatchOrigin,
    pub(crate) parent: Option<u32>,
    pub(crate) collection: u32,
    pub(crate) source: NativeCollectionSource,
    pub(crate) element: Option<NativeMatchValue>,
    pub(crate) initialize: u32,
    pub(crate) header: u32,
    pub(crate) body: u32,
    pub(crate) latch: u32,
    pub(crate) exit: u32,
}
macro_rules! field_accessors {
    ($($name:ident -> $result:ty),* $(,)?) => {$ (
        pub const fn $name(&self) -> $result { self.$name }
    )*};
}
impl NativeCollectionLoop {
    field_accessors! {
        origin -> NativeMatchOrigin,
        parent -> Option<u32>,
        collection -> u32,
        element -> Option<NativeMatchValue>,
        initialize -> u32,
        header -> u32,
        body -> u32,
        latch -> u32,
        exit -> u32,
    }
    pub fn source(&self) -> &NativeCollectionSource {
        &self.source
    }
}

/// A closed synchronous canonical Match with exact final Recur(0) sites.
/// The carried state and final result may have different canonical schemas.
/// There is one generated backedge and one return edge, both structurally
/// checked. Work admission, not a fabricated count, bounds execution.
#[derive(Clone, Debug, serde::Serialize)]
pub struct NativeTailLoop {
    pub(crate) origin: NativeMatchOrigin,
    pub(crate) parent: Option<u32>,
    pub(crate) seed: NativeMatchValue,
    pub(crate) carrier: NativeMatchValue,
    pub(crate) next: NativeMatchValue,
    pub(crate) returned: NativeMatchValue,
    pub(crate) result: NativeMatchValue,
    pub(crate) initialize: u32,
    pub(crate) header: u32,
    pub(crate) body: u32,
    pub(crate) latch: u32,
    pub(crate) finish: u32,
    pub(crate) exit: u32,
}
impl NativeTailLoop {
    field_accessors! {
        origin -> NativeMatchOrigin,
        parent -> Option<u32>,
        seed -> NativeMatchValue,
        carrier -> NativeMatchValue,
        next -> NativeMatchValue,
        returned -> NativeMatchValue,
        result -> NativeMatchValue,
        initialize -> u32,
        header -> u32,
        body -> u32,
        latch -> u32,
        finish -> u32,
        exit -> u32,
    }
}

/// An explicit result strategy. A fused producer does NOT define its Matrix
/// semantic output; only the selected consumer later defines a String.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub enum NativeCollectionResult {
    Materialized,
    TextJoin(NativeTextJoinSink),
    GuardedHistogramQuery(NativeGuardedHistogramQuery),
    Histogram(NativeHistogramSink),
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct NativeHistogramSink {
    pub(crate) consumer: u32,
    pub(crate) template: NativeProgramLibraryTemplate,
    pub(crate) output: NativeMatchValue,
}
impl NativeHistogramSink {
    pub const fn consumer(&self) -> u32 {
        self.consumer
    }
    pub fn template(&self) -> &NativeProgramLibraryTemplate {
        &self.template
    }
    pub const fn output(&self) -> NativeMatchValue {
        self.output
    }
}

/// Stages are provenance/lifecycle, never alternate language Values.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
pub enum NativeHistogramQueryPhase {
    ProducerReady,
    HistogramReady,
    KeysReady,
    SetReady,
    Guarded,
    LookupRead,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
pub enum NativeHistogramQueryStepKind {
    Histogram,
    Keys,
    Set,
    Membership,
    Lookup,
    Cleanup,
    Alias { phase: NativeHistogramQueryPhase },
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct NativeHistogramQueryStep {
    pub(crate) site: u32,
    pub(crate) kind: NativeHistogramQueryStepKind,
    pub(crate) template: Option<NativeProgramLibraryTemplate>,
    pub(crate) operands: Box<[NativeMatchValue]>,
    pub(crate) output: Option<NativeMatchValue>,
}
impl NativeHistogramQueryStep {
    pub const fn site(&self) -> u32 {
        self.site
    }
    pub const fn kind(&self) -> NativeHistogramQueryStepKind {
        self.kind
    }
    pub fn template(&self) -> Option<&NativeProgramLibraryTemplate> {
        self.template.as_ref()
    }
    pub fn operands(&self) -> &[NativeMatchValue] {
        &self.operands
    }
    pub const fn output(&self) -> Option<NativeMatchValue> {
        self.output
    }
    pub const fn required_phase(&self) -> NativeHistogramQueryPhase {
        use NativeHistogramQueryPhase as P;
        use NativeHistogramQueryStepKind as K;
        match self.kind {
            K::Histogram => P::ProducerReady,
            K::Keys => P::HistogramReady,
            K::Set => P::KeysReady,
            K::Membership => P::SetReady,
            K::Lookup | K::Cleanup => P::Guarded,
            K::Alias { phase } => phase,
        }
    }
    /// Cleanup additionally permits LookupRead iff the guarded count is >0.
    pub const fn resulting_phase(&self) -> Option<NativeHistogramQueryPhase> {
        use NativeHistogramQueryPhase as P;
        use NativeHistogramQueryStepKind as K;
        match self.kind {
            K::Histogram => Some(P::HistogramReady),
            K::Keys => Some(P::KeysReady),
            K::Set => Some(P::SetReady),
            K::Membership => Some(P::Guarded),
            K::Lookup => Some(P::LookupRead),
            K::Cleanup => None,
            K::Alias { phase } => Some(phase),
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
pub enum NativeHistogramQuerySetShape {
    Dynamic,
    FreeExact,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct NativeGuardedHistogramQuery {
    pub(crate) query: NativeMatchValue,
    pub(crate) steps: Box<[NativeHistogramQueryStep]>,
    pub(crate) set_shape: NativeHistogramQuerySetShape,
    pub(crate) match_result: NativeMatchValue,
    pub(crate) zero: NativeMatchValue,
    pub(crate) bool_tests: Box<[u32]>,
    pub(crate) true_copy: u32,
    pub(crate) false_copy: u32,
}
impl NativeGuardedHistogramQuery {
    pub const fn query(&self) -> NativeMatchValue {
        self.query
    }
    pub fn steps(&self) -> &[NativeHistogramQueryStep] {
        &self.steps
    }
    pub const fn set_shape(&self) -> NativeHistogramQuerySetShape {
        self.set_shape
    }
    pub const fn match_result(&self) -> NativeMatchValue {
        self.match_result
    }
    pub const fn zero(&self) -> NativeMatchValue {
        self.zero
    }
    pub fn bool_tests(&self) -> &[u32] {
        &self.bool_tests
    }
    pub const fn true_copy(&self) -> u32 {
        self.true_copy
    }
    pub const fn false_copy(&self) -> u32 {
        self.false_copy
    }
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct NativeTextJoinSink {
    pub(crate) consumer: u32,
    pub(crate) template: NativeProgramLibraryTemplate,
    pub(crate) separator: NativeMatchValue,
    pub(crate) output: NativeMatchValue,
}
impl NativeTextJoinSink {
    pub const fn consumer(&self) -> u32 {
        self.consumer
    }
    pub fn template(&self) -> &NativeProgramLibraryTemplate {
        &self.template
    }
    pub const fn separator(&self) -> NativeMatchValue {
        self.separator
    }
    pub const fn output(&self) -> NativeMatchValue {
        self.output
    }
}

/// A selected range/String selection's unobserved intermediate storage strategy.
/// The selector and output are semantic identities, NEVER materialized Values.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct NativeStringSpanProjection {
    pub(crate) range_site: u32,
    pub(crate) selection_site: u32,
    pub(crate) range_template: NativeProgramLibraryTemplate,
    pub(crate) selection_template: NativeProgramLibraryTemplate,
    pub(crate) operands: [NativeMatchValue; 2],
    pub(crate) source: NativeMatchValue,
    pub(crate) selector: NativeMatchValue,
    pub(crate) output: NativeMatchValue,
}
impl NativeStringSpanProjection {
    field_accessors! {
        range_site -> u32,
        selection_site -> u32,
        source -> NativeMatchValue,
        selector -> NativeMatchValue,
        output -> NativeMatchValue,
    }
    pub fn range_template(&self) -> &NativeProgramLibraryTemplate {
        &self.range_template
    }
    pub fn selection_template(&self) -> &NativeProgramLibraryTemplate {
        &self.selection_template
    }
    pub const fn operands(&self) -> &[NativeMatchValue; 2] {
        &self.operands
    }
}

/// One staged result builder, separate from immutable language Values. All
/// generators run in emitted code; the builder only receives selected yields.
#[derive(Clone, Debug, serde::Serialize)]
pub struct NativeProgramCollection {
    pub(crate) origin: NativeMatchOrigin,
    pub(crate) parent: Option<u32>,
    pub(crate) parent_collection: Option<u32>,
    pub(crate) kind: crate::ComprehensionKind,
    pub(crate) output: NativeMatchValue,
    pub(crate) result: NativeCollectionResult,
    pub(crate) string_span_projection: Option<NativeStringSpanProjection>,
    pub(crate) string_element_map_projection: Option<super::NativeStringElementMapProjection>,
    pub(crate) generators: Box<[u32]>,
    pub(crate) begin: u32,
    pub(crate) push: u32,
    pub(crate) finish: u32,
}
impl NativeProgramCollection {
    pub fn string_element_map_projection(
        &self,
    ) -> Option<&super::NativeStringElementMapProjection> {
        self.string_element_map_projection.as_ref()
    }
    pub fn string_span_projection(&self) -> Option<&NativeStringSpanProjection> {
        self.string_span_projection.as_ref()
    }
    pub fn result(&self) -> &NativeCollectionResult {
        &self.result
    }
    field_accessors! {
        origin -> NativeMatchOrigin,
        parent -> Option<u32>,
        parent_collection -> Option<u32>,
        kind -> crate::ComprehensionKind,
        output -> NativeMatchValue,
        begin -> u32,
        push -> u32,
        finish -> u32,
    }
    pub fn generators(&self) -> &[u32] {
        &self.generators
    }
}
