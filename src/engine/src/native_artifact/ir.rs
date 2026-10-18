use mech_core::{
    CallMemoryPlan, CellSlotId, ConstantId, InputId, NodeId, OutputId, ResolvedValueDescriptor,
    SchemaId, SchemaKey,
};

/// Exact semantic kinds supported by the canonical scalar native adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeScalarKind {
    Bool,
    U64,
    I64,
    F64,
}

/// Checked operations certified by the actual selected maintained binder.
/// These are semantic identities, not source names or catalog hashes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeScalarOp {
    U64Add,
    U64Sub,
    U64Mul,
    U64Div,
    U64Rem,
    U64Eq,
    U64Ne,
    U64Lt,
    U64Le,
    U64Gt,
    U64Ge,
    BoolAnd,
    BoolOr,
    BoolXor,
    BoolNot,
    BoolEq,
    BoolNe,
    I64Add,
    I64Sub,
    I64Mul,
    I64Div,
    I64Rem,
    I64Neg,
    I64Eq,
    I64Ne,
    I64Lt,
    I64Le,
    I64Gt,
    I64Ge,
    F64Add,
    F64Sub,
    F64Mul,
    F64Div,
    F64Neg,
    F64Sqrt,
    F64Eq,
    F64Ne,
    F64Lt,
    F64Le,
    F64Gt,
    F64Ge,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeScalarValue {
    Bool(bool),
    U64(u64),
    I64(i64),
    F64(mech_core::snapshot::F64Bits),
}

/// An artifact value identity. A local is evaluated by its one ordered Let;
/// later users refer to that identity without duplicating its expression.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeValueRef {
    Constant(ConstantId),
    Input(InputId),
    Local(NodeId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeScalarType {
    pub(crate) kind: NativeScalarKind,
    pub(crate) schema: SchemaId,
    pub(crate) schema_key: SchemaKey,
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

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
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
