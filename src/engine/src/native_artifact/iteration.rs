//! Read-only lowered native control graph with structurally owned counted loops.
//!
//! This is a compiler view of the ordinary canonical `ProgramArtifact`, never
//! another language artifact. Constructors stay inside the engine's actual
//! selected-implementation lowering path.
use super::{NativeMatchAction, NativeMatchOrigin, NativeMatchTerminator, NativeMatchValue};

/// The lexical invocation that owns an immutable result word. Carrier words
/// have a separate role: only their own initializer and latch may replace them.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeIterationValueScope {
    Immutable { iteration: Option<u32> },
    Carrier { iteration: u32 },
}

#[derive(Clone, Debug)]
pub enum NativeIterationAction {
    Scalar(NativeMatchAction),
    /// A selected maintained leaf at its exact canonical root/lexical site.
    /// Managed program plans alone construct this action; frozen scalar plans
    /// reject it rather than treating handles as integer arithmetic operands.
    LibraryCall {
        template: super::NativeProgramLibraryTemplate,
        operands: Box<[NativeMatchValue]>,
        output: NativeMatchValue,
    },
    Initialize {
        iteration: u32,
    },
    Latch {
        iteration: u32,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeIterationTerminator {
    Scalar(NativeMatchTerminator),
    Header {
        iteration: u32,
        body: u32,
        exit: u32,
    },
}

impl From<NativeMatchTerminator> for NativeIterationTerminator {
    fn from(value: NativeMatchTerminator) -> Self {
        Self::Scalar(value)
    }
}

#[derive(Clone, Debug)]
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
#[derive(Clone, Debug)]
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
