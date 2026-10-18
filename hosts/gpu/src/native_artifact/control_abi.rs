//! Versioned scalar CFG ABI, separate from the frozen straight-line protocol.
use super::{NativeCompileError, abi::NativeEntryFrame, compiler::backend};
use mech_engine::native_artifact::{
    NativeMatchAction, NativeMatchArtifactPlan, NativeMatchOrigin, NativeMatchTerminator,
    NativeScalarType,
};

pub(super) const VERSION: u64 = 2;
pub(super) const ENTRY: &str = "mech_canonical_scalar_match_entry_v1";
pub(super) const STATUS_REJECT: u64 = 4;
pub(super) type Entry = unsafe extern "C" fn(*mut NativeEntryFrame) -> u64;

#[derive(Clone, Debug)]
pub struct NativeMatchVisit {
    pub(super) origin: NativeMatchOrigin,
    pub(super) successors: [Option<u32>; 2],
    pub(super) complete: bool,
    pub(super) arithmetic: bool,
    pub(super) work: u64,
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
    pub(super) entry: u32,
    pub(super) inputs: Box<[NativeScalarType]>,
    pub(super) outputs: Box<[NativeScalarType]>,
    pub(super) value_count: usize,
    pub(super) constant_count: usize,
    pub(super) visits: Box<[NativeMatchVisit]>,
    pub(super) stack_bytes: u64,
    pub(super) code_bytes: u64,
    pub(super) identity: [u8; 32],
}
impl NativeMatchDescriptor {
    pub(super) fn from_compiled(
        plan: &NativeMatchArtifactPlan,
        stack_bytes: u64,
        code_bytes: u64,
    ) -> Result<Self, NativeCompileError> {
        let completion = (plan.outputs().len() as u64)
            .checked_mul(8)
            .and_then(|n| n.checked_add(128))
            .ok_or_else(|| backend("Match completion work overflow"))?;
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
                        NativeMatchAction::Primitive { op, .. } => super::abi::primitive_work(*op),
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
            identity: super::control_identity::typed_ir_sha256(plan),
        })
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
