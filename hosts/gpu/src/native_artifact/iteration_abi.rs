//! Structured counted-loop ABI. Frozen scalar ABI1 and Match ABI2 are separate.
use mech_core::ProgramRevision;
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeIterationArtifactPlan as Plan, NativeIterationLoop,
    NativeIterationTerminator as Terminator, NativeMatchAction as Scalar, NativeMatchOrigin,
    NativeMatchTerminator as Term, NativeScalarType,
};
pub(super) const VERSION: u64 = 3;
#[cfg(feature = "canonical-native")]
pub(super) const ENTRY: &str = "mech_canonical_scalar_iteration_entry_v1";
pub(super) const STATUS_REJECT: u64 = 4;
#[cfg(feature = "canonical-native")]
pub(super) use super::abi::RawEntry as Entry;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Progress {
    Normal,
    Initialize(u32),
    Header(u32),
    Latch(u32),
    Exit(u32),
}
#[derive(Clone, Copy, Debug)]
pub struct NativeIterationVisit {
    pub(super) origin: NativeMatchOrigin,
    pub(super) successors: [Option<u32>; 2],
    pub(super) iteration: Option<u32>,
    pub(super) progress: Progress,
    pub(super) complete: bool,
    pub(super) arithmetic: bool,
    pub(super) reject: bool,
    pub(super) work: u64,
}
impl NativeIterationVisit {
    pub const fn origin(&self) -> NativeMatchOrigin {
        self.origin
    }
    pub const fn successors(&self) -> [Option<u32>; 2] {
        self.successors
    }
    pub const fn iteration(&self) -> Option<u32> {
        self.iteration
    }
    pub const fn is_completion(&self) -> bool {
        self.complete
    }
    pub const fn work(&self) -> u64 {
        self.work
    }
}
#[derive(Clone, Debug)]
pub struct NativeIterationDescriptor {
    pub(super) entry: u32,
    pub(super) revision: ProgramRevision,
    pub(super) inputs: Box<[NativeScalarType]>,
    pub(super) outputs: Box<[NativeScalarType]>,
    pub(super) value_count: usize,
    pub(super) constant_count: usize,
    pub(super) visits: Box<[NativeIterationVisit]>,
    pub(super) iterations: Box<[NativeIterationLoop]>,
    pub(super) stack_bytes: u64,
    pub(super) code_bytes: u64,
    pub(super) identity: [u8; 32],
}
impl NativeIterationDescriptor {
    pub(super) fn from_plan(
        plan: &Plan,
        stack_bytes: u64,
        code_bytes: u64,
    ) -> Result<Self, &'static str> {
        plan.validate_current_visits()?;
        let completion = (plan.outputs().len() as u64)
            .checked_mul(8)
            .and_then(|n| n.checked_add((plan.iterations().len() as u64).checked_mul(8)?))
            .and_then(|n| n.checked_add(256))
            .ok_or("iteration completion overflow")?;
        let visits = control_visits(plan.blocks(), plan.iterations(), completion)?;
        Ok(Self {
            entry: plan.entry(),
            revision: plan.program_revision(),
            inputs: plan
                .inputs()
                .iter()
                .map(|v| v.declaration().ty().clone())
                .collect(),
            outputs: plan.outputs().iter().map(|v| v.ty().clone()).collect(),
            value_count: plan.values().len(),
            constant_count: plan.constants().len(),
            visits: visits.into(),
            iterations: plan.iterations().into(),
            stack_bytes,
            code_bytes,
            identity: super::iteration_identity::typed_ir_sha256(plan),
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
    pub fn visits(&self) -> &[NativeIterationVisit] {
        &self.visits
    }
    pub fn iterations(&self) -> &[NativeIterationLoop] {
        &self.iterations
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
    pub(super) fn local_count(&self) -> Option<usize> {
        self.value_count.checked_add(self.iterations.len())
    }
}

pub(super) fn control_visits(
    blocks: &[mech_engine::native_artifact::NativeIterationBlock],
    iterations: &[NativeIterationLoop],
    completion: u64,
) -> Result<Vec<NativeIterationVisit>, &'static str> {
    let mut visits = Vec::with_capacity(blocks.len());
    for (id, block) in blocks.iter().enumerate() {
        let (successors, complete) = match block.terminator() {
            Terminator::Scalar(Term::Jump(next)) => ([Some(next), None], false),
            Terminator::Scalar(
                Term::Branch { yes, no, .. } | Term::LiteralBranch { yes, no, .. },
            ) => ([Some(yes), Some(no)], false),
            Terminator::Header { body, exit, .. } => ([Some(body), Some(exit)], false),
            Terminator::Scalar(Term::Complete) => ([None, None], true),
            Terminator::Scalar(Term::Reject) => ([None, None], false),
        };
        let progress = match (block.action(), block.terminator()) {
            (Action::Initialize { iteration }, _) => Progress::Initialize(*iteration),
            (Action::Latch { iteration }, _) => Progress::Latch(*iteration),
            (_, Terminator::Header { iteration, .. }) => Progress::Header(iteration),
            _ => iterations
                .iter()
                .position(|v| v.exit() as usize == id)
                .map(|v| Progress::Exit(v as u32))
                .unwrap_or(Progress::Normal),
        };
        let arithmetic = matches!(block.action(), Action::Scalar(Scalar::Primitive { .. }));
        let work = if complete {
            completion
        } else {
            match block.action() {
                Action::Scalar(Scalar::Primitive { op, .. }) => super::abi::primitive_work(*op)
                    .checked_add(128)
                    .ok_or("iteration primitive work overflow")?,
                _ => 256,
            }
        };
        visits.push(NativeIterationVisit {
            origin: block.origin(),
            successors,
            iteration: block.iteration(),
            progress,
            complete,
            arithmetic,
            reject: block.terminator() == Terminator::Scalar(Term::Reject),
            work,
        });
    }
    Ok(visits)
}
