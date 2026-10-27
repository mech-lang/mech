//! Explicit versioned executable CFG identity; no Debug/serde encoding.
use crate::native_artifact::identity::transcript::{
    bytes, origin, scalar_action, scalar_bindings, scalar_terminator, word,
};
use mech_engine::native_artifact::NativeMatchArtifactPlan;
use sha2::{Digest, Sha256};

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
    pub(in crate::native_artifact) fn from_descriptor(
        descriptor: &crate::native_artifact::control_abi::NativeMatchDescriptor,
    ) -> Result<Self, &'static str> {
        let mut bytes = [0; NATIVE_MATCH_LINK_IDENTITY_BYTES];
        bytes[..16].copy_from_slice(MAGIC);
        for (at, value) in [
            (16, NATIVE_MATCH_TYPED_IR_VERSION),
            (24, crate::native_artifact::control_abi::VERSION),
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
            &crate::native_artifact::identity::host_target_tag()
                .ok_or("unsupported linked Match host target")?,
        );
        Ok(Self {
            bytes,
            symbol: format!(
                "mech_canonical_scalar_match_identity_v1_{}",
                crate::native_artifact::identity::hex(descriptor.typed_ir_sha256())
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
pub(in crate::native_artifact) fn descriptor_from_identity(
    plan: &NativeMatchArtifactPlan,
    bytes: &[u8; NATIVE_MATCH_LINK_IDENTITY_BYTES],
) -> Result<crate::native_artifact::control_abi::NativeMatchDescriptor, &'static str> {
    if &bytes[..16] != MAGIC
        || get(bytes, 16) != NATIVE_MATCH_TYPED_IR_VERSION
        || get(bytes, 24) != crate::native_artifact::control_abi::VERSION
        || get(bytes, 32) != NATIVE_MATCH_WORK_POLICY_VERSION
    {
        return Err("unsupported linked Match identity/ABI/work-policy version");
    }
    let target = crate::native_artifact::identity::host_target_tag()
        .ok_or("unsupported linked Match host target")?;
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
    let descriptor =
        crate::native_artifact::control_abi::NativeMatchDescriptor::from_plan(plan, stack, code)?;
    if bytes[72..104] != descriptor.typed_ir_sha256()[..] {
        return Err("linked Match object typed CFG differs from selected canonical plan");
    }
    Ok(descriptor)
}
pub(in crate::native_artifact) fn typed_ir_sha256(plan: &NativeMatchArtifactPlan) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"mech.canonical-scalar-match-cfg\0");
    word(&mut hash, NATIVE_MATCH_TYPED_IR_VERSION);
    word(&mut hash, crate::native_artifact::control_abi::VERSION);
    word(&mut hash, NATIVE_MATCH_WORK_POLICY_VERSION);
    hash.update(plan.program_revision().as_bytes());
    word(&mut hash, plan.entry() as u64);
    scalar_bindings(&mut hash, plan.values(), plan.inputs(), plan.constants());
    word(&mut hash, plan.blocks().len() as u64);
    for block in plan.blocks() {
        origin(&mut hash, block.origin());
        scalar_action(&mut hash, block.action());
        scalar_terminator(&mut hash, block.terminator());
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
mod tests;
