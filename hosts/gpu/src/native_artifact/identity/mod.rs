//! Versioned executable-IR identity. No Debug/serde representation is hashed.
use crate::native_artifact::abi::{ABI_VERSION, NativeAbiDescriptor};
use mech_engine::native_artifact::{NativeArtifactPlan, NativeScalarOp, NativeValueRef};
use sha2::{Digest, Sha256};

pub(in crate::native_artifact) mod transcript;
use transcript::{bytes, scalar_type as ty, scalar_value, word};

/// Fixed exported data size for the build-owned entry pairing record.
pub const NATIVE_LINK_IDENTITY_BYTES: usize = 152;
// Exact-width checked U8 addition and U32 subtraction join the scalar vocabulary.
pub const NATIVE_TYPED_IR_VERSION: u64 = 9;
pub const NATIVE_WORK_POLICY_VERSION: u64 = 1;
const MAGIC: &[u8; 16] = b"MECHNATIVEOBJv1\0";

/// Read-only identity of a real compiled object. Only the shared emitter can
/// construct one; its bytes are also exported from that exact object.
#[derive(Clone, Debug)]
pub struct NativeObjectIdentity {
    pub(in crate::native_artifact) bytes: [u8; NATIVE_LINK_IDENTITY_BYTES],
    symbol: String,
}
impl NativeObjectIdentity {
    #[cfg(feature = "canonical-native")]
    pub(in crate::native_artifact) fn from_descriptor(
        descriptor: &NativeAbiDescriptor,
    ) -> Option<Self> {
        let mut bytes = [0; NATIVE_LINK_IDENTITY_BYTES];
        bytes[..16].copy_from_slice(MAGIC);
        put(&mut bytes, 16, NATIVE_TYPED_IR_VERSION);
        put(&mut bytes, 24, ABI_VERSION);
        put(&mut bytes, 32, NATIVE_WORK_POLICY_VERSION);
        bytes[40..72].copy_from_slice(descriptor.revision().as_bytes());
        bytes[72..104].copy_from_slice(descriptor.typed_ir_sha256());
        bytes[104..112].copy_from_slice(&host_target_tag()?);
        put(&mut bytes, 112, descriptor.stack_bytes());
        put(&mut bytes, 120, descriptor.code_bytes());
        put(&mut bytes, 128, descriptor.inputs().len() as u64);
        put(&mut bytes, 136, descriptor.lets().len() as u64);
        put(&mut bytes, 144, descriptor.outputs().len() as u64);
        Some(Self {
            bytes,
            symbol: format!(
                "mech_canonical_scalar_identity_v1_{}",
                hex(descriptor.typed_ir_sha256())
            ),
        })
    }
    pub fn bytes(&self) -> &[u8; NATIVE_LINK_IDENTITY_BYTES] {
        &self.bytes
    }
    pub fn symbol(&self) -> &str {
        &self.symbol
    }
}
#[cfg(feature = "canonical-native")]
fn put(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}
fn get(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(
        bytes[offset..offset + 8]
            .try_into()
            .expect("fixed identity field"),
    )
}

/// The emitter is host-only. Architecture, OS/calling convention, pointer
/// width and endianness must agree at link and invocation. Unknown targets
/// fail closed instead of interpreting a host object as portable bytecode.
pub(in crate::native_artifact) fn host_target_tag() -> Option<[u8; 8]> {
    if !cfg!(all(target_pointer_width = "64", target_endian = "little")) {
        return None;
    }
    let arch = if cfg!(target_arch = "x86_64") {
        1
    } else if cfg!(target_arch = "aarch64") {
        2
    } else {
        return None;
    };
    let os = if cfg!(target_os = "macos") {
        1
    } else if cfg!(target_os = "linux") {
        2
    } else if cfg!(target_os = "windows") {
        3
    } else {
        return None;
    };
    Some([1, arch, os, 64, 1, 0, 0, 0])
}

/// Binds compiled metadata to a newly derived opaque plan. This only checks
/// metadata, outside evaluation; the unsafe linked constructor additionally
/// requires the entry and this record to originate from the same object.
pub(in crate::native_artifact) fn descriptor_from_identity(
    plan: &NativeArtifactPlan,
    bytes: &[u8; NATIVE_LINK_IDENTITY_BYTES],
) -> Result<NativeAbiDescriptor, &'static str> {
    if &bytes[..16] != MAGIC
        || get(bytes, 16) != NATIVE_TYPED_IR_VERSION
        || get(bytes, 24) != ABI_VERSION
        || get(bytes, 32) != NATIVE_WORK_POLICY_VERSION
    {
        return Err("unsupported linked identity/ABI/work-policy version");
    }
    let target = host_target_tag().ok_or("unsupported linked host target")?;
    if bytes[104..112] != target {
        return Err("linked object target differs from this executable");
    }
    if bytes[40..72] != plan.program_revision().as_bytes()[..]
        || get(bytes, 128) != plan.inputs().len() as u64
        || get(bytes, 136) != plan.lets().len() as u64
        || get(bytes, 144) != plan.outputs().len() as u64
    {
        return Err("linked object canonical revision/counts differ from plan");
    }
    let stack = get(bytes, 112);
    let code = get(bytes, 120);
    if stack == 0 || code == 0 {
        return Err("linked object has no checked frame/code metadata");
    }
    let descriptor = NativeAbiDescriptor::from_compiled(plan, stack, code);
    if bytes[72..104] != descriptor.typed_ir_sha256()[..] {
        return Err("linked object typed IR differs from selected canonical plan");
    }
    Ok(descriptor)
}

#[cfg(any(
    feature = "canonical-native",
    feature = "canonical-native-program-runtime"
))]
pub(in crate::native_artifact) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Hashes every executable typed-IR field with explicit little-endian tags,
/// counts and byte lengths. Schema keys and canonical revision bind the
/// canonical type/metadata authority. The selected resident scratch plan and
/// interactive display metadata are not executable semantics; the revision
/// still binds their originating artifact. This encoding must be versioned
/// whenever its field set, operation tags, or checked ABI semantics change.
pub(in crate::native_artifact) fn typed_ir_sha256(plan: &NativeArtifactPlan) -> [u8; 32] {
    fn reference(hash: &mut Sha256, value: NativeValueRef) {
        let (tag, id) = match value {
            NativeValueRef::Constant(id) => (1, id.get()),
            NativeValueRef::Input(id) => (2, id.get()),
            NativeValueRef::Local(id) => (3, id.get()),
        };
        hash.update([tag]);
        word(hash, u64::from(id));
    }
    let mut hash = Sha256::new();
    hash.update(b"mech.canonical-executable-typed-ir\0");
    word(&mut hash, NATIVE_TYPED_IR_VERSION);
    word(&mut hash, ABI_VERSION);
    word(&mut hash, NATIVE_WORK_POLICY_VERSION);
    hash.update(plan.program_revision().as_bytes());
    word(&mut hash, plan.inputs().len() as u64);
    for input in plan.inputs() {
        word(&mut hash, u64::from(input.input().get()));
        word(&mut hash, u64::from(input.slot().get()));
        bytes(&mut hash, input.name().as_bytes());
        ty(&mut hash, input.ty());
    }
    word(&mut hash, plan.constants().len() as u64);
    for constant in plan.constants() {
        word(&mut hash, u64::from(constant.constant().get()));
        ty(&mut hash, constant.ty());
        scalar_value(&mut hash, constant.value());
    }
    word(&mut hash, plan.lets().len() as u64);
    for node in plan.lets() {
        word(&mut hash, u64::from(node.node().get()));
        word(&mut hash, u64::from(node.target().get()));
        hash.update([op_tag(node.op())]);
        ty(&mut hash, node.ty());
        word(&mut hash, node.operands().len() as u64);
        for operand in node.operands() {
            reference(&mut hash, *operand);
        }
    }
    word(&mut hash, plan.outputs().len() as u64);
    for output in plan.outputs() {
        word(&mut hash, u64::from(output.output().get()));
        word(&mut hash, u64::from(output.slot().get()));
        bytes(&mut hash, output.name().as_bytes());
        ty(&mut hash, output.ty());
        reference(&mut hash, output.value());
    }
    hash.finalize().into()
}
pub(in crate::native_artifact) fn op_tag(op: NativeScalarOp) -> u8 {
    match op {
        NativeScalarOp::U64BitAnd => 48,
        NativeScalarOp::U64BitOr => 49,
        NativeScalarOp::U64BitXor => 50,
        NativeScalarOp::U64BitNot => 51,
        NativeScalarOp::U64ShlWrap => 52,
        NativeScalarOp::U64ShrWrap => 53,
        NativeScalarOp::U64Clz => 54,
        NativeScalarOp::U64Ctz => 55,
        NativeScalarOp::U64WrappingAdd => 56,
        NativeScalarOp::U64WrappingSub => 57,
        NativeScalarOp::U64WrappingMul => 58,
        NativeScalarOp::U32Add => 89,
        NativeScalarOp::U32Sub => 90,
        NativeScalarOp::U8Add => 91,
        NativeScalarOp::U64Add => 1,
        NativeScalarOp::U64Sub => 2,
        NativeScalarOp::U64Mul => 3,
        NativeScalarOp::U64Div => 4,
        NativeScalarOp::U64Rem => 5,
        NativeScalarOp::U64Eq => 6,
        NativeScalarOp::U64Ne => 7,
        NativeScalarOp::U64Lt => 8,
        NativeScalarOp::U64Le => 9,
        NativeScalarOp::U64Gt => 10,
        NativeScalarOp::U64Ge => 11,
        NativeScalarOp::BoolAnd => 12,
        NativeScalarOp::BoolOr => 13,
        NativeScalarOp::BoolXor => 14,
        NativeScalarOp::BoolNot => 15,
        NativeScalarOp::BoolEq => 16,
        NativeScalarOp::BoolNe => 17,
        NativeScalarOp::I64Add => 18,
        NativeScalarOp::I64Sub => 19,
        NativeScalarOp::I64Mul => 20,
        NativeScalarOp::I64Div => 21,
        NativeScalarOp::I64Rem => 22,
        NativeScalarOp::I64Neg => 23,
        NativeScalarOp::I64Eq => 24,
        NativeScalarOp::I64Ne => 25,
        NativeScalarOp::I64Lt => 26,
        NativeScalarOp::I64Le => 27,
        NativeScalarOp::I64Gt => 28,
        NativeScalarOp::I64Ge => 29,
        NativeScalarOp::F64Add => 30,
        NativeScalarOp::F64Sub => 31,
        NativeScalarOp::F64Mul => 32,
        NativeScalarOp::F64Div => 33,
        NativeScalarOp::F64Neg => 34,
        NativeScalarOp::F64Sqrt => 35,
        NativeScalarOp::F64Eq => 36,
        NativeScalarOp::F64Ne => 37,
        NativeScalarOp::F64Lt => 38,
        NativeScalarOp::F64Le => 39,
        NativeScalarOp::F64Gt => 40,
        NativeScalarOp::F64Ge => 41,
        NativeScalarOp::U64ToI64 => 42,
        NativeScalarOp::I64ToU64 => 43,
        NativeScalarOp::U64ToF64 => 44,
        NativeScalarOp::I64ToF64 => 45,
        NativeScalarOp::F64ToU64 => 46,
        NativeScalarOp::F64ToI64 => 47,
        NativeScalarOp::U8ToU16 => 59,
        NativeScalarOp::U8ToU32 => 60,
        NativeScalarOp::U8ToU64 => 61,
        NativeScalarOp::U16ToU8 => 62,
        NativeScalarOp::U16ToU32 => 63,
        NativeScalarOp::U16ToU64 => 64,
        NativeScalarOp::U32ToU8 => 65,
        NativeScalarOp::U32ToU16 => 66,
        NativeScalarOp::U32ToU64 => 67,
        NativeScalarOp::U64ToU8 => 68,
        NativeScalarOp::U64ToU16 => 69,
        NativeScalarOp::U64ToU32 => 70,
        NativeScalarOp::U8Eq => 71,
        NativeScalarOp::U8Ne => 72,
        NativeScalarOp::U8Lt => 73,
        NativeScalarOp::U8Le => 74,
        NativeScalarOp::U8Gt => 75,
        NativeScalarOp::U8Ge => 76,
        NativeScalarOp::U16Eq => 77,
        NativeScalarOp::U16Ne => 78,
        NativeScalarOp::U16Lt => 79,
        NativeScalarOp::U16Le => 80,
        NativeScalarOp::U16Gt => 81,
        NativeScalarOp::U16Ge => 82,
        NativeScalarOp::U32Eq => 83,
        NativeScalarOp::U32Ne => 84,
        NativeScalarOp::U32Lt => 85,
        NativeScalarOp::U32Le => 86,
        NativeScalarOp::U32Gt => 87,
        NativeScalarOp::U32Ge => 88,
    }
}

#[cfg(all(test, feature = "canonical-native"))]
mod tests;
