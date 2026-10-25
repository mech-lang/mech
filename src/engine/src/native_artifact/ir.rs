use mech_core::{
    CallMemoryPlan, CellSlotId, ConstantId, InputId, NodeId, OutputId, ResolvedValueDescriptor,
    SchemaId, SchemaKey,
};

pub use crate::native_scalar::{NativeScalarKind, NativeScalarOp};

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
pub enum NativeScalarValue {
    Bool(bool),
    U64(u64),
    I64(i64),
    F64(mech_core::snapshot::F64Bits),
}

/// An artifact value identity. A local is evaluated by its one ordered Let;
/// later users refer to that identity without duplicating its expression.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
pub enum NativeValueRef {
    Constant(ConstantId),
    Input(InputId),
    Local(NodeId),
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct NativeScalarType {
    pub(crate) kind: NativeScalarKind,
    pub(crate) schema: SchemaId,
    pub(crate) schema_key: SchemaKey,
    #[serde(serialize_with = "super::program_image::serialize_descriptor")]
    pub(crate) descriptor: ResolvedValueDescriptor,
}
impl NativeScalarType {
    pub const fn kind(&self) -> NativeScalarKind {
        self.kind
    }
    pub fn descriptor(&self) -> &ResolvedValueDescriptor {
        &self.descriptor
    }
    pub fn schema(&self) -> SchemaId {
        self.schema
    }
    pub fn schema_key(&self) -> SchemaKey {
        self.schema_key
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct NativeInput {
    pub(crate) input: InputId,
    pub(crate) slot: CellSlotId,
    pub(crate) name: String,
    pub(crate) ty: NativeScalarType,
}
impl NativeInput {
    pub const fn input(&self) -> InputId {
        self.input
    }
    pub const fn slot(&self) -> CellSlotId {
        self.slot
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn ty(&self) -> &NativeScalarType {
        &self.ty
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct NativeConstant {
    pub(crate) constant: ConstantId,
    pub(crate) ty: NativeScalarType,
    pub(crate) value: NativeScalarValue,
}
impl NativeConstant {
    pub const fn constant(&self) -> ConstantId {
        self.constant
    }
    pub fn ty(&self) -> &NativeScalarType {
        &self.ty
    }
    pub const fn value(&self) -> NativeScalarValue {
        self.value
    }
}

#[derive(Clone, Debug)]
pub struct NativeLet {
    pub(crate) node: NodeId,
    pub(crate) target: CellSlotId,
    pub(crate) op: NativeScalarOp,
    pub(crate) operands: Box<[NativeValueRef]>,
    pub(crate) ty: NativeScalarType,
    pub(crate) memory_plan: CallMemoryPlan,
}
impl NativeLet {
    /// Origin for checked errors and deterministic source order.
    pub const fn node(&self) -> NodeId {
        self.node
    }
    pub const fn target(&self) -> CellSlotId {
        self.target
    }
    pub const fn op(&self) -> NativeScalarOp {
        self.op
    }
    pub fn operands(&self) -> &[NativeValueRef] {
        &self.operands
    }
    pub fn ty(&self) -> &NativeScalarType {
        &self.ty
    }
    /// NativeHost scalar storage with conservative selected-source scratch
    /// policy. Not a qualified native executor budget or executable code.
    pub fn memory_plan(&self) -> &CallMemoryPlan {
        &self.memory_plan
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct NativeOutput {
    pub(crate) output: OutputId,
    pub(crate) slot: CellSlotId,
    pub(crate) name: String,
    pub(crate) interactive_binding: Option<crate::InteractiveSymbolBinding>,
    pub(crate) value: NativeValueRef,
    pub(crate) ty: NativeScalarType,
}
impl NativeOutput {
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

    pub const fn value(&self) -> NativeValueRef {
        self.value
    }
    pub fn ty(&self) -> &NativeScalarType {
        &self.ty
    }
}
