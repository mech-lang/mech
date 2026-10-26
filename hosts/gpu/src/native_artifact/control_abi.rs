//! Versioned scalar CFG ABI, separate from the frozen straight-line protocol.
#[cfg(feature = "canonical-native")]
use crate::native_artifact::abi::NativeEntryFrame;
use mech_core::ProgramRevision;
use mech_engine::native_artifact::{
    NativeMatchAction, NativeMatchArtifactPlan, NativeMatchOrigin, NativeMatchTerminator,
    NativeScalarType,
};

pub(in crate::native_artifact) const VERSION: u64 = 2;
#[cfg(feature = "canonical-native")]
pub(in crate::native_artifact) const ENTRY: &str = "mech_canonical_scalar_match_entry_v1";
pub(in crate::native_artifact) const STATUS_REJECT: u64 = 4;
#[cfg(feature = "canonical-native")]
pub(in crate::native_artifact) type Entry = unsafe extern "C" fn(*mut NativeEntryFrame) -> u64;

#[derive(Clone, Debug)]
pub struct NativeMatchVisit {
    pub(in crate::native_artifact) origin: NativeMatchOrigin,
    pub(in crate::native_artifact) successors: [Option<u32>; 2],
    pub(in crate::native_artifact) complete: bool,
    pub(in crate::native_artifact) arithmetic: bool,
    pub(in crate::native_artifact) work: u64,
}
impl NativeMatchVisit {
    pub const fn origin(&self) -> NativeMatchOrigin {
        self.origin
    }
    pub const fn successors(&self) -> [Option<u32>; 2] {
        self.successors
    }
    pub const fn is_completion(&self) -> bool {
        self.complete
    }
    pub const fn work(&self) -> u64 {
        self.work
    }
}

/// Immutable same-plan call metadata. There is no public constructor or codec.
#[derive(Clone, Debug)]
pub struct NativeMatchDescriptor {
    pub(in crate::native_artifact) entry: u32,
    pub(in crate::native_artifact) revision: ProgramRevision,
    pub(in crate::native_artifact) inputs: Box<[NativeScalarType]>,
    pub(in crate::native_artifact) outputs: Box<[NativeScalarType]>,
    pub(in crate::native_artifact) value_count: usize,
    pub(in crate::native_artifact) constant_count: usize,
    pub(in crate::native_artifact) visits: Box<[NativeMatchVisit]>,
    pub(in crate::native_artifact) stack_bytes: u64,
    pub(in crate::native_artifact) code_bytes: u64,
    pub(in crate::native_artifact) identity: [u8; 32],
}
impl NativeMatchDescriptor {
    pub(in crate::native_artifact) fn from_plan(
        plan: &NativeMatchArtifactPlan,
        stack_bytes: u64,
        code_bytes: u64,
    ) -> Result<Self, &'static str> {
        let completion = (plan.outputs().len() as u64)
            .checked_mul(8)
            .and_then(|n| n.checked_add(128))
            .ok_or("Match completion work overflow")?;
        let visits = plan
            .blocks()
            .iter()
            .map(|block| {
                let (successors, complete) = match block.terminator() {
                    NativeMatchTerminator::Jump(next) => ([Some(next), None], false),
                    NativeMatchTerminator::Branch { yes, no, .. }
                    | NativeMatchTerminator::LiteralBranch { yes, no, .. } => {
                        ([Some(yes), Some(no)], false)
                    }
                    NativeMatchTerminator::Complete => ([None, None], true),
                    NativeMatchTerminator::Reject => ([None, None], false),
                };
                let work = if complete {
                    completion
                } else {
                    match block.action() {
                        NativeMatchAction::Primitive { op, .. } => {
                            crate::native_artifact::abi::primitive_work(*op)
                        }
                        _ => 128,
                    }
                };
                NativeMatchVisit {
                    origin: block.origin(),
                    successors,
                    complete,
                    arithmetic: matches!(block.action(), NativeMatchAction::Primitive { .. }),
                    work,
                }
            })
            .collect();
        Ok(Self {
            entry: plan.entry(),
            revision: plan.program_revision(),
            inputs: plan
                .inputs()
                .iter()
                .map(|input| input.declaration().ty().clone())
                .collect(),
            outputs: plan
                .outputs()
                .iter()
                .map(|output| output.ty().clone())
                .collect(),
            value_count: plan.values().len(),
            constant_count: plan.constants().len(),
            visits,
            stack_bytes,
            code_bytes,
            identity: crate::native_artifact::control_identity::typed_ir_sha256(plan),
        })
    }
    pub const fn revision(&self) -> ProgramRevision {
        self.revision
    }
    pub const fn entry(&self) -> u32 {
        self.entry
    }
    pub fn inputs(&self) -> &[NativeScalarType] {
        &self.inputs
    }
    pub fn outputs(&self) -> &[NativeScalarType] {
        &self.outputs
    }
    pub const fn value_count(&self) -> usize {
        self.value_count
    }
    pub fn visits(&self) -> &[NativeMatchVisit] {
        &self.visits
    }
    pub const fn stack_bytes(&self) -> u64 {
        self.stack_bytes
    }
    pub const fn code_bytes(&self) -> u64 {
        self.code_bytes
    }
    pub fn typed_ir_sha256(&self) -> &[u8; 32] {
        &self.identity
    }
}
