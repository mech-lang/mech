//! Read-only lowered native control graph with structurally owned counted loops.
//!
//! This is a compiler view of the ordinary canonical `ProgramArtifact`, never
//! another language artifact. Constructors stay inside the engine's actual
//! selected-implementation lowering path.
use super::{NativeMatchAction, NativeMatchOrigin, NativeMatchTerminator, NativeMatchValue};

/// The lexical invocation that owns an immutable result word. Carrier words
/// have a separate role: only their own initializer and latch may replace them.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
pub enum NativeIterationValueScope {
    Immutable { iteration: Option<u32> },
    Carrier { iteration: u32 },
}

#[derive(Clone, Debug, serde::Serialize)]
pub enum NativeIterationAction {
    Scalar(NativeMatchAction),
    /// A schema-checked structural observation. This never dispatches an arm
    /// or evaluates a control body; emitted code owns every branch.
    Structural {
        source: NativeMatchValue,
        path: Box<[NativeStructuralStep]>,
        observation: NativeStructuralObservation,
        output: NativeMatchValue,
    },
    /// A selected maintained leaf at its exact canonical root/lexical site.
    /// Managed program plans alone construct this action; frozen scalar plans
    /// reject it rather than treating handles as integer arithmetic operands.
    LibraryCall {
        template: super::NativeProgramLibraryTemplate,
        operands: Box<[NativeMatchValue]>,
        output: NativeMatchValue,
    },
    /// Initialize an admitted non-Value source at its ORIGINAL range call.
    /// The selected template and operands live on the exact generator.
    PrepareU64Range {
        iteration: u32,
    },
    /// Prepare a non-Value selector at its ORIGINAL selected range call.
    PrepareStringSpanSelector {
        collection: u32,
    },
    /// Pin the genuine source and prepare a checked non-Value byte span at
    /// the ORIGINAL selected String selection; the next proved Push consumes it.
    ProjectStringSpan {
        collection: u32,
    },
    /// Select a non-Value String entry using the current borrowed generator
    /// element at its ORIGINAL selected Map key access; the proved Push consumes it.
    ProjectStringMapEntry {
        collection: u32,
    },
    CollectionBegin {
        collection: u32,
    },
    CollectionElement {
        iteration: u32,
    },
    CollectionPush {
        collection: u32,
        value: NativeMatchValue,
    },
    CollectionFinish {
        collection: u32,
    },
    /// Finalize an admitted append sink at the ORIGINAL selected consumer.
    TextJoinFinish {
        collection: u32,
    },
    /// One proved original histogram/keys/set/guard/lookup/cleanup site.
    HistogramQueryStep {
        collection: u32,
        step: u32,
    },
    HistogramFinish {
        collection: u32,
    },
    Initialize {
        iteration: u32,
    },
    Latch {
        iteration: u32,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
pub enum NativeIterationTerminator {
    Scalar(NativeMatchTerminator),
    /// A work-metered synchronous tail loop has no invented iteration count.
    /// Its body either reaches the unique latch or the unique return site.
    TailHeader {
        iteration: u32,
        body: u32,
    },
    Header {
        iteration: u32,
        body: u32,
        exit: u32,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
pub enum NativeStructuralStep {
    EnumPayload { ordinal: u32 },
    TupleItem { ordinal: u32 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
pub enum NativeStructuralObservation {
    EnumOrdinal { ordinal: u32 },
    U64,
    StringEqual { peer: NativeMatchValue },
}

impl From<NativeMatchTerminator> for NativeIterationTerminator {
    fn from(value: NativeMatchTerminator) -> Self {
        Self::Scalar(value)
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct NativeIterationBlock {
    pub(crate) origin: NativeMatchOrigin,
    pub(crate) action: NativeIterationAction,
    pub(crate) terminator: NativeIterationTerminator,
    pub(crate) iteration: Option<u32>,
}
impl NativeIterationBlock {
    pub const fn origin(&self) -> NativeMatchOrigin {
        self.origin
    }
    pub fn action(&self) -> &NativeIterationAction {
        &self.action
    }
    pub const fn terminator(&self) -> NativeIterationTerminator {
        self.terminator
    }
    pub const fn iteration(&self) -> Option<u32> {
        self.iteration
    }
}

/// One literal-count carrier transition, linked to its exact lexical body.
/// Private scalar carrier and counter storage is independent of `count`.
#[derive(Clone, Debug, serde::Serialize)]
pub struct NativeIterationLoop {
    pub(crate) origin: NativeMatchOrigin,
    pub(crate) count: u64,
    pub(crate) parent: Option<u32>,
    pub(crate) seed: NativeMatchValue,
    pub(crate) carrier: NativeMatchValue,
    pub(crate) next: NativeMatchValue,
    pub(crate) result: NativeMatchValue,
    pub(crate) initialize: u32,
    pub(crate) header: u32,
    pub(crate) body: u32,
    pub(crate) latch: u32,
    pub(crate) exit: u32,
}
impl NativeIterationLoop {
    pub const fn origin(&self) -> NativeMatchOrigin {
        self.origin
    }
    pub const fn count(&self) -> u64 {
        self.count
    }
    pub const fn parent(&self) -> Option<u32> {
        self.parent
    }
    pub const fn seed(&self) -> NativeMatchValue {
        self.seed
    }
    pub const fn carrier(&self) -> NativeMatchValue {
        self.carrier
    }
    pub const fn next(&self) -> NativeMatchValue {
        self.next
    }
    pub const fn result(&self) -> NativeMatchValue {
        self.result
    }
    pub const fn initialize(&self) -> u32 {
        self.initialize
    }
    pub const fn header(&self) -> u32 {
        self.header
    }
    pub const fn body(&self) -> u32 {
        self.body
    }
    pub const fn latch(&self) -> u32 {
        self.latch
    }
    pub const fn exit(&self) -> u32 {
        self.exit
    }
}
