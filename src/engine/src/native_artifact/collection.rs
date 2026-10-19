//! Structured native collection loops, sharing the existing lexical CFG proof.
//! These are private-constructor compiler views of canonical comprehensions,
//! not another source artifact, interpreter, or executable callback authority.
use super::{NativeIterationLoop, NativeMatchOrigin, NativeMatchValue};

#[derive(Clone, Debug)]
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

#[derive(Clone, Copy, Debug)]
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
#[derive(Clone, Debug)]
pub struct NativeCollectionLoop {
    pub(crate) origin: NativeMatchOrigin,
    pub(crate) parent: Option<u32>,
    pub(crate) collection: u32,
    pub(crate) source: NativeMatchValue,
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
        source -> NativeMatchValue,
        element -> Option<NativeMatchValue>,
        initialize -> u32,
        header -> u32,
        body -> u32,
        latch -> u32,
        exit -> u32,
    }
}

/// A closed synchronous canonical Match with exact final Recur(0) sites.
/// The carried state and final result may have different canonical schemas.
/// There is one generated backedge and one return edge, both structurally
/// checked. Work admission, not a fabricated count, bounds execution.
#[derive(Clone, Debug)]
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

/// One staged result builder, separate from immutable language Values. All
/// generators run in emitted code; the builder only receives selected yields.
#[derive(Clone, Debug)]
pub struct NativeProgramCollection {
    pub(crate) origin: NativeMatchOrigin,
    pub(crate) parent: Option<u32>,
    pub(crate) parent_collection: Option<u32>,
    pub(crate) kind: crate::ComprehensionKind,
    pub(crate) output: NativeMatchValue,
    pub(crate) generators: Box<[u32]>,
    pub(crate) begin: u32,
    pub(crate) push: u32,
    pub(crate) finish: u32,
}
impl NativeProgramCollection {
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
