use core::ffi::c_void;
use mech_core::{NodeId, ProgramRevision};
use mech_engine::native_artifact::{NativeArtifactPlan, NativeScalarOp, NativeScalarType};

pub const NATIVE_ARTIFACT_ABI_VERSION: u64 = 1;
pub const NATIVE_ARTIFACT_ENTRY_SYMBOL: &str = "mech_canonical_scalar_entry_v1";
pub(in crate::native_artifact) const ABI_VERSION: u64 = NATIVE_ARTIFACT_ABI_VERSION;
pub(in crate::native_artifact) const NO_NODE: u64 = u64::MAX;
pub(in crate::native_artifact) const STATUS_OK: u64 = 0;
pub(in crate::native_artifact) const STATUS_ARITHMETIC: u64 = 1;
pub(in crate::native_artifact) const STATUS_ADMISSION: u64 = 2;
pub(in crate::native_artifact) const STATUS_ABI: u64 = 3;
pub(in crate::native_artifact) const STATUS_INTEGRITY: u64 = 9;

/// Checked primitive work in conservative logical units. One unit bounds one
/// explicit scalar/guard/load/store bookkeeping action, not machine cycles.
/// Each finite primitive template is checked during definition: explicit CLIF
/// instructions plus64 fixed allocation-free callback bookkeeping actions must
/// fit these256 units, including admission dispatch, guards and local store.
/// This cost policy is separate from the selected binder's semantic witness
/// and from the adapter's conservative source scratch metadata.
pub(in crate::native_artifact) const fn primitive_work(op: NativeScalarOp) -> u64 {
    match op {
        NativeScalarOp::U64Add
        | NativeScalarOp::U64BitAnd
        | NativeScalarOp::U64BitOr
        | NativeScalarOp::U64BitXor
        | NativeScalarOp::U64BitNot
        | NativeScalarOp::U64ShlWrap
        | NativeScalarOp::U64ShrWrap
        | NativeScalarOp::U64Clz
        | NativeScalarOp::U64Ctz
        | NativeScalarOp::U64WrappingAdd
        | NativeScalarOp::U64WrappingSub
        | NativeScalarOp::U64WrappingMul
        | NativeScalarOp::U64Sub
        | NativeScalarOp::U64Mul
        | NativeScalarOp::U64Div
        | NativeScalarOp::U64Rem
        | NativeScalarOp::U64Eq
        | NativeScalarOp::U64Ne
        | NativeScalarOp::U64Lt
        | NativeScalarOp::U64Le
        | NativeScalarOp::U64Gt
        | NativeScalarOp::U64Ge
        | NativeScalarOp::BoolAnd
        | NativeScalarOp::BoolOr
        | NativeScalarOp::BoolXor
        | NativeScalarOp::BoolNot
        | NativeScalarOp::BoolEq
        | NativeScalarOp::BoolNe
        | NativeScalarOp::I64Add
        | NativeScalarOp::I64Sub
        | NativeScalarOp::I64Mul
        | NativeScalarOp::I64Div
        | NativeScalarOp::I64Rem
        | NativeScalarOp::I64Neg
        | NativeScalarOp::I64Eq
        | NativeScalarOp::I64Ne
        | NativeScalarOp::I64Lt
        | NativeScalarOp::I64Le
        | NativeScalarOp::I64Gt
        | NativeScalarOp::I64Ge
        | NativeScalarOp::F64Add
        | NativeScalarOp::F64Sub
        | NativeScalarOp::F64Mul
        | NativeScalarOp::F64Div
        | NativeScalarOp::F64Neg
        | NativeScalarOp::F64Sqrt
        | NativeScalarOp::F64Eq
        | NativeScalarOp::F64Ne
        | NativeScalarOp::F64Lt
        | NativeScalarOp::F64Le
        | NativeScalarOp::F64Gt
        | NativeScalarOp::F64Ge
        | NativeScalarOp::U64ToI64
        | NativeScalarOp::I64ToU64
        | NativeScalarOp::U64ToF64
        | NativeScalarOp::I64ToF64
        | NativeScalarOp::F64ToU64
        | NativeScalarOp::F64ToI64
        | NativeScalarOp::U8ToU16
        | NativeScalarOp::U8ToU32
        | NativeScalarOp::U8ToU64
        | NativeScalarOp::U16ToU8
        | NativeScalarOp::U16ToU32
        | NativeScalarOp::U16ToU64
        | NativeScalarOp::U32ToU8
        | NativeScalarOp::U32ToU16
        | NativeScalarOp::U32ToU64
        | NativeScalarOp::U64ToU8
        | NativeScalarOp::U64ToU16
        | NativeScalarOp::U64ToU32
        => 256,
    }
}

/// Same-plan type and machine-layout metadata, constructed only by the shared
/// definition pass. There is no deserializer or public unchecked constructor.
#[derive(Clone, Debug)]
pub struct NativeAbiDescriptor {
    revision: ProgramRevision,
    inputs: Box<[NativeScalarType]>,
    outputs: Box<[NativeScalarType]>,
    lets: Box<[(NodeId, NativeScalarOp)]>,
    stack_bytes: u64,
    code_bytes: u64,
    typed_ir_sha256: [u8; 32],
}
impl NativeAbiDescriptor {
    pub(in crate::native_artifact) fn from_compiled(
        plan: &NativeArtifactPlan,
        stack_bytes: u64,
        code_bytes: u64,
    ) -> Self {
        Self {
            revision: plan.program_revision(),
            inputs: plan.inputs().iter().map(|v| v.ty().clone()).collect(),
            outputs: plan.outputs().iter().map(|v| v.ty().clone()).collect(),
            lets: plan.lets().iter().map(|v| (v.node(), v.op())).collect(),
            stack_bytes,
            code_bytes,
            typed_ir_sha256: crate::native_artifact::identity::typed_ir_sha256(plan),
        }
    }
    pub fn typed_ir_sha256(&self) -> &[u8; 32] {
        &self.typed_ir_sha256
    }
    pub const fn revision(&self) -> ProgramRevision {
        self.revision
    }
    pub fn inputs(&self) -> &[NativeScalarType] {
        &self.inputs
    }
    pub fn outputs(&self) -> &[NativeScalarType] {
        &self.outputs
    }
    pub fn lets(&self) -> &[(NodeId, NativeScalarOp)] {
        &self.lets
    }
    /// Actual generated entry frame, including supported ABI setup, spills and
    /// outgoing arguments. This is not the Rust caller/callback stack usage.
    pub const fn stack_bytes(&self) -> u64 {
        self.stack_bytes
    }
    pub const fn code_bytes(&self) -> u64 {
        self.code_bytes
    }
}

/// Private same-module synchronous ABI. The safe wrapper exclusively owns all
/// transport regions and the callback context for the entire invocation.
/// Bool transport uses U64 words restricted to 0/1; semantic Bool storage in
/// the artifact/call plan remains a distinct one-byte scalar representation.
/// Signed I64 values retain their two's-complement bits in one eight-byte lane.
/// F64 values retain every IEEE bit, including signed zero and NaN payloads.
#[repr(C)]
pub(in crate::native_artifact) struct NativeEntryFrame {
    pub version: u64,
    pub input_count: u64,
    pub local_count: u64,
    pub output_count: u64,
    pub inputs: *const u64,
    pub locals: *mut u64,
    pub outputs: *mut u64,
    pub context: *mut c_void,
    pub admit_next: unsafe extern "C" fn(*mut c_void, u64) -> u64,
    pub error_node: u64,
}
#[cfg(feature = "canonical-native")]
pub(in crate::native_artifact) type RawEntry = unsafe extern "C" fn(*mut NativeEntryFrame) -> u64;
