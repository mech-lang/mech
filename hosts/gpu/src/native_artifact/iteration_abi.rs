//! Structured counted-loop ABI, separate from frozen scalar and Match entries.
use mech_core::ProgramRevision;
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeIterationArtifactPlan as Plan, NativeIterationLoop,
    NativeIterationTerminator as Terminator, NativeMatchAction as Scalar, NativeMatchOrigin,
    NativeMatchTerminator as Term, NativeScalarType,
};
pub(in crate::native_artifact) const VERSION: u64 = 4;
#[cfg(feature = "canonical-native")]
pub(in crate::native_artifact) const ENTRY: &str = "mech_canonical_scalar_iteration_entry_v1";
pub(in crate::native_artifact) const STATUS_REJECT: u64 = 4;
#[cfg(feature = "canonical-native")]
pub(in crate::native_artifact) use crate::native_artifact::abi::RawEntry as Entry;
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub(in crate::native_artifact) enum Progress {
    Normal,
    Initialize(u32),
    Header(u32),
    Latch(u32),
    Exit(u32),
}
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct NativeIterationVisit {
    pub(in crate::native_artifact) origin: NativeMatchOrigin,
    pub(in crate::native_artifact) successors: [Option<u32>; 2],
    pub(in crate::native_artifact) iteration: Option<u32>,
    pub(in crate::native_artifact) progress: Progress,
    pub(in crate::native_artifact) complete: bool,
    pub(in crate::native_artifact) arithmetic: bool,
    pub(in crate::native_artifact) reject: bool,
    pub(in crate::native_artifact) integrity: bool,
    pub(in crate::native_artifact) work: u64,
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
    pub(in crate::native_artifact) entry: u32,
    pub(in crate::native_artifact) revision: ProgramRevision,
    pub(in crate::native_artifact) inputs: Box<[NativeScalarType]>,
    pub(in crate::native_artifact) outputs: Box<[NativeScalarType]>,
    pub(in crate::native_artifact) value_count: usize,
    pub(in crate::native_artifact) constant_count: usize,
    pub(in crate::native_artifact) visits: Box<[NativeIterationVisit]>,
    pub(in crate::native_artifact) iterations: Box<[NativeIterationLoop]>,
    pub(in crate::native_artifact) stack_bytes: u64,
    pub(in crate::native_artifact) code_bytes: u64,
    pub(in crate::native_artifact) identity: [u8; 32],
}
impl NativeIterationDescriptor {
    pub(in crate::native_artifact) fn from_plan(
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
        let visits = control_visits(
            plan.blocks(),
            plan.iterations().iter().map(Into::into),
            completion,
        )?;
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
            identity: crate::native_artifact::iteration_identity::typed_ir_sha256(plan),
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
    pub(in crate::native_artifact) fn local_count(&self) -> Option<usize> {
        self.value_count.checked_add(self.iterations.len())
    }
}

pub(in crate::native_artifact) fn control_visits<'a>(
    blocks: &[mech_engine::native_artifact::NativeIterationBlock],
    iterations: impl Iterator<Item = mech_engine::native_artifact::NativeControlLoopRef<'a>> + Clone,
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
            Terminator::TailHeader { body, .. } => ([Some(body), None], false),
            Terminator::Scalar(Term::Complete) => ([None, None], true),
            Terminator::Scalar(Term::Reject | Term::IntegrityFailure) => ([None, None], false),
        };
        let progress = match (block.action(), block.terminator()) {
            (Action::Initialize { iteration }, _) => Progress::Initialize(*iteration),
            (Action::Latch { iteration }, _) => Progress::Latch(*iteration),
            (_, Terminator::Header { iteration, .. }) => Progress::Header(iteration),
            (_, Terminator::TailHeader { iteration, .. }) => Progress::Header(iteration),
            _ => iterations
                .clone()
                .position(|v| v.exit() as usize == id)
                .map(|v| Progress::Exit(v as u32))
                .unwrap_or(Progress::Normal),
        };
        let arithmetic = matches!(block.action(), Action::Scalar(Scalar::Primitive { .. }));
        let work = if complete {
            completion
        } else {
            match block.action() {
                Action::Scalar(Scalar::Primitive { op, .. }) => {
                    crate::native_artifact::abi::primitive_work(*op)
                        .checked_add(128)
                        .ok_or("iteration primitive work overflow")?
                }
                _ => 256,
            }
        };
        let work = work
            .checked_add(helper_operand_work(block.action())?)
            .ok_or("iteration helper work overflow")?;
        visits.push(NativeIterationVisit {
            origin: block.origin(),
            successors,
            iteration: block.iteration(),
            progress,
            complete,
            arithmetic,
            reject: block.terminator() == Terminator::Scalar(Term::Reject),
            integrity: block.terminator() == Terminator::Scalar(Term::IntegrityFailure),
            work,
        });
    }
    Ok(visits)
}

// Variable-arity leaf transport emits one load/constant and store per operand,
// with slack for address formation. Both emitted-code validation and runtime
// current-visit admission use this same arity-derived bound. Scalar ABI3 has
// no LibraryCall actions and therefore retains its existing work policy.
pub(in crate::native_artifact) fn helper_operand_work(
    action: &Action,
) -> Result<u64, &'static str> {
    match action {
        Action::PrepareU64Range { .. } | Action::PrepareStringSpanSelector { .. } => Ok(16),
        Action::LibraryCall { operands, .. } => (operands.len() as u64)
            .checked_mul(8)
            .ok_or("iteration operand work overflow"),
        _ => Ok(0),
    }
}
