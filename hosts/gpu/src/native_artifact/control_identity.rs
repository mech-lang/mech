//! Explicit versioned executable CFG identity; no Debug/serde encoding.
use mech_engine::native_artifact::{
    NativeMatchAction, NativeMatchArtifactPlan, NativeMatchTerminator, NativeScalarKind,
    NativeScalarValue,
};
use sha2::{Digest, Sha256};
fn word(hash: &mut Sha256, value: u64) {
    hash.update(value.to_le_bytes());
}
fn bytes(hash: &mut Sha256, value: &[u8]) {
    word(hash, value.len() as u64);
    hash.update(value);
}
pub const NATIVE_MATCH_LINK_IDENTITY_BYTES: usize = 168;
pub const NATIVE_MATCH_TYPED_IR_VERSION: u64 = 2;
pub const NATIVE_MATCH_WORK_POLICY_VERSION: u64 = 1;
const MAGIC: &[u8; 16] = b"MECHNATIVEMATCH\0";

/// Read-only same-Object metadata; only the checked emitter constructs it.
#[derive(Clone, Debug)]
pub struct NativeMatchObjectIdentity {
    bytes: [u8; NATIVE_MATCH_LINK_IDENTITY_BYTES],
    symbol: String,
}
impl NativeMatchObjectIdentity {
    #[cfg(feature = "canonical-native")]
    pub(super) fn from_descriptor(
        descriptor: &super::control_abi::NativeMatchDescriptor,
    ) -> Result<Self, &'static str> {
        let mut bytes = [0; NATIVE_MATCH_LINK_IDENTITY_BYTES];
        bytes[..16].copy_from_slice(MAGIC);
        for (at, value) in [
            (16, NATIVE_MATCH_TYPED_IR_VERSION),
            (24, super::control_abi::VERSION),
            (32, NATIVE_MATCH_WORK_POLICY_VERSION),
            (112, descriptor.stack_bytes()),
            (120, descriptor.code_bytes()),
            (128, descriptor.inputs().len() as u64),
            (136, descriptor.value_count() as u64),
            (144, descriptor.visits().len() as u64),
            (152, descriptor.outputs().len() as u64),
            (160, descriptor.entry() as u64),
        ] {
            bytes[at..at + 8].copy_from_slice(&value.to_le_bytes());
        }
        bytes[40..72].copy_from_slice(descriptor.revision().as_bytes());
        bytes[72..104].copy_from_slice(descriptor.typed_ir_sha256());
        bytes[104..112].copy_from_slice(
            &super::identity::host_target_tag().ok_or("unsupported linked Match host target")?,
        );
        Ok(Self {
            bytes,
            symbol: format!(
                "mech_canonical_scalar_match_identity_v1_{}",
                super::identity::hex(descriptor.typed_ir_sha256())
            ),
        })
    }
    pub fn bytes(&self) -> &[u8; NATIVE_MATCH_LINK_IDENTITY_BYTES] {
        &self.bytes
    }
    pub fn symbol(&self) -> &str {
        &self.symbol
    }
}
fn get(bytes: &[u8; NATIVE_MATCH_LINK_IDENTITY_BYTES], offset: usize) -> u64 {
    u64::from_le_bytes(
        bytes[offset..offset + 8]
            .try_into()
            .expect("fixed identity field"),
    )
}

/// Metadata binding only. Matching record bytes do not authenticate a foreign
/// entry; the unsafe build-owned constructor supplies that independent duty.
pub(super) fn descriptor_from_identity(
    plan: &NativeMatchArtifactPlan,
    bytes: &[u8; NATIVE_MATCH_LINK_IDENTITY_BYTES],
) -> Result<super::control_abi::NativeMatchDescriptor, &'static str> {
    if &bytes[..16] != MAGIC
        || get(bytes, 16) != NATIVE_MATCH_TYPED_IR_VERSION
        || get(bytes, 24) != super::control_abi::VERSION
        || get(bytes, 32) != NATIVE_MATCH_WORK_POLICY_VERSION
    {
        return Err("unsupported linked Match identity/ABI/work-policy version");
    }
    let target =
        super::identity::host_target_tag().ok_or("unsupported linked Match host target")?;
    if bytes[104..112] != target {
        return Err("linked Match object target differs from this executable");
    }
    if bytes[40..72] != plan.program_revision().as_bytes()[..]
        || get(bytes, 128) != plan.inputs().len() as u64
        || get(bytes, 136) != plan.values().len() as u64
        || get(bytes, 144) != plan.blocks().len() as u64
        || get(bytes, 152) != plan.outputs().len() as u64
        || get(bytes, 160) != plan.entry() as u64
    {
        return Err("linked Match object canonical revision/counts/entry differ from plan");
    }
    let stack = get(bytes, 112);
    let code = get(bytes, 120);
    if stack == 0 || code == 0 {
        return Err("linked Match object has no checked frame/code metadata");
    }
    let descriptor = super::control_abi::NativeMatchDescriptor::from_plan(plan, stack, code)?;
    if bytes[72..104] != descriptor.typed_ir_sha256()[..] {
        return Err("linked Match object typed CFG differs from selected canonical plan");
    }
    Ok(descriptor)
}
pub(super) fn typed_ir_sha256(plan: &NativeMatchArtifactPlan) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"mech.canonical-scalar-match-cfg\0");
    word(&mut hash, NATIVE_MATCH_TYPED_IR_VERSION);
    word(&mut hash, super::control_abi::VERSION);
    word(&mut hash, NATIVE_MATCH_WORK_POLICY_VERSION);
    hash.update(plan.program_revision().as_bytes());
    word(&mut hash, plan.entry() as u64);
    word(&mut hash, plan.values().len() as u64);
    for ty in plan.values() {
        hash.update([match ty.kind() {
            NativeScalarKind::Bool => 1,
            NativeScalarKind::U64 => 2,
            NativeScalarKind::I64 => 3,
            NativeScalarKind::F64 => 4,
        }]);
        word(&mut hash, ty.schema().get() as u64);
        hash.update(ty.schema_key().as_bytes());
    }
    word(&mut hash, plan.inputs().len() as u64);
    for input in plan.inputs() {
        word(&mut hash, input.declaration().input().get() as u64);
        word(&mut hash, input.declaration().slot().get() as u64);
        bytes(&mut hash, input.declaration().name().as_bytes());
        word(&mut hash, input.value().get() as u64);
    }
    word(&mut hash, plan.constants().len() as u64);
    for constant in plan.constants() {
        word(&mut hash, constant.value().get() as u64);
        match constant.scalar() {
            NativeScalarValue::Bool(value) => {
                hash.update([1]);
                word(&mut hash, value as u64);
            }
            NativeScalarValue::U64(value) => {
                hash.update([2]);
                word(&mut hash, value);
            }
            NativeScalarValue::I64(value) => {
                hash.update([3]);
                word(&mut hash, value as u64);
            }
            NativeScalarValue::F64(value) => {
                hash.update([4]);
                word(&mut hash, value.bits());
            }
        }
    }
    word(&mut hash, plan.blocks().len() as u64);
    for block in plan.blocks() {
        let at = block.origin();
        word(&mut hash, at.node().get() as u64);
        for id in [at.block(), at.local()] {
            match id {
                Some(id) => {
                    hash.update([1]);
                    word(&mut hash, id as u64);
                }
                None => hash.update([0]),
            }
        }
        match block.action() {
            NativeMatchAction::None => hash.update([0]),
            NativeMatchAction::Primitive {
                op,
                operands,
                output,
                ..
            } => {
                hash.update([1, super::identity::op_tag(*op)]);
                word(&mut hash, output.get() as u64);
                word(&mut hash, operands.len() as u64);
                for operand in operands {
                    word(&mut hash, operand.get() as u64);
                }
            }
            NativeMatchAction::Copy { source, output } => {
                hash.update([2]);
                word(&mut hash, source.get() as u64);
                word(&mut hash, output.get() as u64);
            }
        }
        match block.terminator() {
            NativeMatchTerminator::Jump(next) => {
                hash.update([1]);
                word(&mut hash, next as u64);
            }
            NativeMatchTerminator::Branch { condition, yes, no } => {
                hash.update([2]);
                word(&mut hash, condition.get() as u64);
                word(&mut hash, yes as u64);
                word(&mut hash, no as u64);
            }
            NativeMatchTerminator::LiteralBranch {
                left,
                right,
                yes,
                no,
            } => {
                hash.update([3]);
                for value in [left.get(), right.get(), yes, no] {
                    word(&mut hash, value as u64);
                }
            }
            NativeMatchTerminator::Complete => hash.update([4]),
            NativeMatchTerminator::Reject => hash.update([5]),
        }
    }
    word(&mut hash, plan.outputs().len() as u64);
    for output in plan.outputs() {
        word(&mut hash, output.output().get() as u64);
        word(&mut hash, output.slot().get() as u64);
        bytes(&mut hash, output.name().as_bytes());
        word(&mut hash, output.value().get() as u64);
    }
    hash.finalize().into()
}

#[cfg(all(test, feature = "canonical-native"))]
mod tests {
    use super::*;
    use mech_engine::CanonicalSourceFrontend;
    use mech_syntax::document::{
        AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
        parse_canonical_document,
    };
    fn plan(source: &str) -> NativeMatchArtifactPlan {
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x4d4c4944), Revision(1), source).unwrap(),
            ParseConfig::default(),
        );
        assert!(parsed.is_strictly_clean());
        let artifact = CanonicalSourceFrontend
            .compile_document_with_catalog(
                &DocumentSyntax::cast(parsed.syntax()).unwrap(),
                mech_stdlib::source_catalog(),
            )
            .unwrap()
            .compile_artifact()
            .unwrap();
        mech_engine::native_artifact::lower_scalar_match_artifact(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap()
    }
    #[test]
    fn match_link_identity_binds_exact_cfg_revision_target_entry_and_counts() {
        let first = plan("(flag<bool>? | true => value<u64> + 1u64 | false => value<u64>)\n");
        let same = plan("(flag<bool>? | true => value<u64> + 1u64 | false => value<u64>)\n");
        let other = plan("(flag<bool>? | true => value<u64> + 2u64 | false => value<u64>)\n");
        assert_eq!(typed_ir_sha256(&first), typed_ir_sha256(&same));
        assert_ne!(typed_ir_sha256(&first), typed_ir_sha256(&other));
        let descriptor =
            super::super::control_abi::NativeMatchDescriptor::from_plan(&first, 32, 1024).unwrap();
        let identity = NativeMatchObjectIdentity::from_descriptor(&descriptor).unwrap();
        let restored = descriptor_from_identity(&same, identity.bytes()).unwrap();
        assert_eq!(restored.typed_ir_sha256(), descriptor.typed_ir_sha256());
        assert_eq!(restored.entry(), descriptor.entry());
        assert_eq!(restored.visits().len(), descriptor.visits().len());
        assert!(descriptor_from_identity(&other, identity.bytes()).is_err());
        for version in [0u64, 1] {
            let mut old = *identity.bytes();
            old[16..24].copy_from_slice(&version.to_le_bytes());
            assert!(descriptor_from_identity(&first, &old).is_err());
        }
        for offset in [0, 16, 24, 32, 40, 72, 104, 128, 136, 144, 152, 160] {
            let mut corrupt = *identity.bytes();
            corrupt[offset] ^= 1;
            assert!(
                descriptor_from_identity(&first, &corrupt).is_err(),
                "field {offset}"
            );
        }
        for offset in [112, 120] {
            let mut corrupt = *identity.bytes();
            corrupt[offset..offset + 8].fill(0);
            assert!(descriptor_from_identity(&first, &corrupt).is_err());
        }
    }
}
