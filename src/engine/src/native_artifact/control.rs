//! Read-only, witness-derived scalar control flow. A plan is constructor-private.
use super::{NativeInput, NativeScalarOp, NativeScalarType, NativeScalarValue};
use mech_core::{CellSlotId, NodeId, OutputId};

/// Scalar word identity within one complete native invocation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
pub struct NativeMatchValue(pub(crate) u32);
impl NativeMatchValue {
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Source identity for errors and admissions, including lexical block locals.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
pub struct NativeMatchOrigin {
    pub(crate) node: NodeId,
    pub(crate) block: Option<u32>,
    pub(crate) local: Option<u32>,
}
impl NativeMatchOrigin {
    pub const fn node(self) -> NodeId {
        self.node
    }
    pub const fn block(self) -> Option<u32> {
        self.block
    }
    pub const fn local(self) -> Option<u32> {
        self.local
    }
}

/// One admitted scalar operation. Each operation occupies its own CFG block,
/// so an earlier arithmetic error precedes admission of a later operation.
#[derive(Clone, Debug, serde::Serialize)]
pub enum NativeMatchAction {
    Primitive {
        op: NativeScalarOp,
        operands: Box<[NativeMatchValue]>,
        output: NativeMatchValue,
    },
    Copy {
        source: NativeMatchValue,
        output: NativeMatchValue,
    },
    None,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
pub enum NativeMatchTerminator {
    Jump(u32),
    Branch {
        condition: NativeMatchValue,
        yes: u32,
        no: u32,
    },
    /// Scalar literal equality belongs to the canonical pattern. It needs no
    /// fabricated Bool schema/local or library-name semantic witness.
    LiteralBranch {
        left: NativeMatchValue,
        right: NativeMatchValue,
        yes: u32,
        no: u32,
    },
    Complete,
    /// An exhaustive validated Match cannot reach this block for valid inputs.
    Reject,
    /// The selected lexical invariant was false. Distinct from an unmatched
    /// pattern; generated code must preserve this checked failure boundary.
    IntegrityFailure,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct NativeMatchBlock {
    pub(crate) origin: NativeMatchOrigin,
    pub(crate) action: NativeMatchAction,
    pub(crate) terminator: NativeMatchTerminator,
}
impl NativeMatchBlock {
    pub const fn origin(&self) -> NativeMatchOrigin {
        self.origin
    }
    pub fn action(&self) -> &NativeMatchAction {
        &self.action
    }
    pub const fn terminator(&self) -> NativeMatchTerminator {
        self.terminator
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct NativeMatchInput {
    pub(crate) declaration: NativeInput,
    pub(crate) value: NativeMatchValue,
}
impl NativeMatchInput {
    pub fn declaration(&self) -> &NativeInput {
        &self.declaration
    }
    pub const fn value(&self) -> NativeMatchValue {
        self.value
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct NativeMatchConstant {
    pub(crate) value: NativeMatchValue,
    pub(crate) scalar: NativeScalarValue,
}
impl NativeMatchConstant {
    pub const fn value(&self) -> NativeMatchValue {
        self.value
    }
    pub const fn scalar(&self) -> NativeScalarValue {
        self.scalar
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct NativeMatchOutput {
    pub(crate) output: OutputId,
    pub(crate) slot: CellSlotId,
    pub(crate) name: String,
    pub(crate) interactive_binding: Option<crate::InteractiveSymbolBinding>,
    pub(crate) value: NativeMatchValue,
    pub(crate) ty: NativeScalarType,
}
impl NativeMatchOutput {
    pub const fn output(&self) -> OutputId {
        self.output
    }
    pub const fn slot(&self) -> CellSlotId {
        self.slot
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn interactive_binding(&self) -> Option<&crate::InteractiveSymbolBinding> {
        self.interactive_binding.as_ref()
    }
    pub const fn value(&self) -> NativeMatchValue {
        self.value
    }
    pub fn ty(&self) -> &NativeScalarType {
        &self.ty
    }
}
