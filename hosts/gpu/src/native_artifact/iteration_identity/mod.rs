//! Explicit versioned executable CFG identity; no Debug/serde encoding.
use crate::native_artifact::identity::transcript::{
    bytes, optional_index, origin, scalar_action, scalar_bindings, scalar_terminator, word,
};
use mech_engine::native_artifact::{
    NativeIterationAction, NativeIterationArtifactPlan, NativeIterationTerminator,
    NativeIterationValueScope,
};
use sha2::{Digest, Sha256};

pub const NATIVE_ITERATION_LINK_IDENTITY_BYTES: usize = 176;
pub const NATIVE_ITERATION_TYPED_IR_VERSION: u64 = 3;
pub const NATIVE_ITERATION_WORK_POLICY_VERSION: u64 = 1;
const MAGIC: &[u8; 16] = b"MECHNATIVEITER\0\0";

/// Read-only same-Object metadata; only the checked emitter constructs it.
#[derive(Clone, Debug)]
pub struct NativeIterationObjectIdentity {
    bytes: [u8; NATIVE_ITERATION_LINK_IDENTITY_BYTES],
    symbol: String,
}
impl NativeIterationObjectIdentity {
    #[cfg(feature = "canonical-native")]
    pub(in crate::native_artifact) fn from_descriptor(
        descriptor: &crate::native_artifact::iteration_abi::NativeIterationDescriptor,
    ) -> Result<Self, &'static str> {
        let mut bytes = [0; NATIVE_ITERATION_LINK_IDENTITY_BYTES];
        bytes[..16].copy_from_slice(MAGIC);
        for (at, value) in [
            (16, NATIVE_ITERATION_TYPED_IR_VERSION),
            (24, crate::native_artifact::iteration_abi::VERSION),
            (32, NATIVE_ITERATION_WORK_POLICY_VERSION),
            (112, descriptor.stack_bytes()),
            (120, descriptor.code_bytes()),
            (128, descriptor.inputs().len() as u64),
            (136, descriptor.value_count() as u64),
            (144, descriptor.visits().len() as u64),
            (152, descriptor.outputs().len() as u64),
            (160, descriptor.entry() as u64),
            (168, descriptor.iterations().len() as u64),
        ] {
            bytes[at..at + 8].copy_from_slice(&value.to_le_bytes());
        }
        bytes[40..72].copy_from_slice(descriptor.revision().as_bytes());
        bytes[72..104].copy_from_slice(descriptor.typed_ir_sha256());
        bytes[104..112].copy_from_slice(
            &crate::native_artifact::identity::host_target_tag()
                .ok_or("unsupported linked Iteration host target")?,
        );
        Ok(Self {
            bytes,
            symbol: format!(
                "mech_canonical_scalar_iteration_identity_v1_{}",
                crate::native_artifact::identity::hex(descriptor.typed_ir_sha256())
            ),
        })
    }
    pub fn bytes(&self) -> &[u8; NATIVE_ITERATION_LINK_IDENTITY_BYTES] {
        &self.bytes
    }
    pub fn symbol(&self) -> &str {
        &self.symbol
    }
}
fn get(bytes: &[u8; NATIVE_ITERATION_LINK_IDENTITY_BYTES], offset: usize) -> u64 {
    u64::from_le_bytes(
        bytes[offset..offset + 8]
            .try_into()
            .expect("fixed identity field"),
    )
}

/// Metadata binding only. Matching record bytes do not authenticate a foreign
/// entry; the unsafe build-owned constructor supplies that independent duty.
pub(in crate::native_artifact) fn descriptor_from_identity(
    plan: &NativeIterationArtifactPlan,
    bytes: &[u8; NATIVE_ITERATION_LINK_IDENTITY_BYTES],
) -> Result<crate::native_artifact::iteration_abi::NativeIterationDescriptor, &'static str> {
    if &bytes[..16] != MAGIC
        || get(bytes, 16) != NATIVE_ITERATION_TYPED_IR_VERSION
        || get(bytes, 24) != crate::native_artifact::iteration_abi::VERSION
        || get(bytes, 32) != NATIVE_ITERATION_WORK_POLICY_VERSION
    {
        return Err("unsupported linked Iteration identity/ABI/work-policy version");
    }
    let target = crate::native_artifact::identity::host_target_tag()
        .ok_or("unsupported linked Iteration host target")?;
    if bytes[104..112] != target {
        return Err("linked Iteration object target differs from this executable");
    }
    if bytes[40..72] != plan.program_revision().as_bytes()[..]
        || get(bytes, 128) != plan.inputs().len() as u64
        || get(bytes, 136) != plan.values().len() as u64
        || get(bytes, 144) != plan.blocks().len() as u64
        || get(bytes, 152) != plan.outputs().len() as u64
        || get(bytes, 160) != plan.entry() as u64
        || get(bytes, 168) != plan.iterations().len() as u64
    {
        return Err("linked Iteration object canonical revision/counts/entry differ from plan");
    }
    let stack = get(bytes, 112);
    let code = get(bytes, 120);
    if stack == 0 || code == 0 {
        return Err("linked Iteration object has no checked frame/code metadata");
    }
    let descriptor = crate::native_artifact::iteration_abi::NativeIterationDescriptor::from_plan(
        plan, stack, code,
    )?;
    if bytes[72..104] != descriptor.typed_ir_sha256()[..] {
        return Err("linked Iteration object typed CFG differs from selected canonical plan");
    }
    Ok(descriptor)
}

pub(in crate::native_artifact) fn typed_ir_sha256(plan: &NativeIterationArtifactPlan) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"mech.canonical-scalar-iteration-cfg\0");
    word(&mut hash, NATIVE_ITERATION_TYPED_IR_VERSION); // Separate control IR version.
    word(&mut hash, crate::native_artifact::iteration_abi::VERSION);
    word(&mut hash, NATIVE_ITERATION_WORK_POLICY_VERSION);
    hash.update(plan.program_revision().as_bytes());
    word(&mut hash, plan.entry() as u64);
    scalar_bindings(&mut hash, plan.values(), plan.inputs(), plan.constants());
    word(&mut hash, plan.blocks().len() as u64);
    for block in plan.blocks() {
        origin(&mut hash, block.origin());
        optional_index(&mut hash, block.iteration());
        match block.action() {
            // Such a plan is rejected by both engine proof and scalar emitter.
            // Keep it disjoint from every valid frozen scalar action encoding.
            NativeIterationAction::LibraryCall { .. }
            | NativeIterationAction::PrepareU64Range { .. }
            | NativeIterationAction::PrepareStringSpanSelector { .. }
            | NativeIterationAction::ProjectStringSpan { .. }
            | NativeIterationAction::ProjectStringMapEntry { .. }
            | NativeIterationAction::Structural { .. }
            | NativeIterationAction::CollectionBegin { .. }
            | NativeIterationAction::CollectionElement { .. }
            | NativeIterationAction::CollectionPush { .. }
            | NativeIterationAction::CollectionFinish { .. }
            | NativeIterationAction::HistogramQueryStep { .. }
            | NativeIterationAction::HistogramFinish { .. }
            | NativeIterationAction::TextJoinFinish { .. } => hash.update([255]),
            NativeIterationAction::Scalar(action) => {
                hash.update([0]);
                scalar_action(&mut hash, action);
            }
            NativeIterationAction::Initialize { iteration } => {
                hash.update([1]);
                word(&mut hash, *iteration as u64);
            }
            NativeIterationAction::Latch { iteration } => {
                hash.update([2]);
                word(&mut hash, *iteration as u64);
            }
        }
        match block.terminator() {
            NativeIterationTerminator::TailHeader { .. } => hash.update([255]),
            NativeIterationTerminator::Scalar(terminator) => {
                hash.update([0]);
                scalar_terminator(&mut hash, terminator);
            }
            NativeIterationTerminator::Header {
                iteration,
                body,
                exit,
            } => {
                hash.update([1]);
                for id in [iteration, body, exit] {
                    word(&mut hash, id as u64);
                }
            }
        }
    }
    word(&mut hash, plan.value_scopes().len() as u64);
    for scope in plan.value_scopes() {
        match scope {
            NativeIterationValueScope::Immutable { iteration } => {
                hash.update([0]);
                optional_index(&mut hash, *iteration);
            }
            NativeIterationValueScope::Carrier { iteration } => {
                hash.update([1]);
                word(&mut hash, *iteration as u64);
            }
        }
    }
    word(&mut hash, plan.iterations().len() as u64);
    for iteration in plan.iterations() {
        word(&mut hash, iteration.origin().node().get() as u64);
        for id in [
            iteration.origin().block(),
            iteration.origin().local(),
            iteration.parent(),
        ] {
            optional_index(&mut hash, id);
        }
        word(&mut hash, iteration.count());
        for value in [
            iteration.seed(),
            iteration.carrier(),
            iteration.next(),
            iteration.result(),
        ] {
            word(&mut hash, value.get() as u64);
        }
        for id in [
            iteration.initialize(),
            iteration.header(),
            iteration.body(),
            iteration.latch(),
            iteration.exit(),
        ] {
            word(&mut hash, id as u64);
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
mod tests;
