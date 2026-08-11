use core::sync::atomic::{AtomicU64, Ordering};

use mech_core::InstanceEpoch;

use super::{GateBArena, GateBControlFixture, GateBPlan, GateBWorkspace};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResidentExecutionError {
    EpochExhausted,
    IncompleteCandidate,
    LandmarkDistance,
    InnovationDeterminant,
    NonFiniteState,
    CovarianceDiagonal,
    CovarianceSymmetry,
}

#[inline]
pub(crate) fn publish_epoch(target: &AtomicU64, epoch: InstanceEpoch) {
    target.store(epoch.0, Ordering::Release);
}

#[derive(Debug)]
pub(crate) struct GateBInstance {
    pub(crate) plan: GateBPlan,
    pub(crate) state: GateBArena,
    pub(crate) workspace: GateBWorkspace,
    pub(crate) published_epoch: AtomicU64,
    pub(crate) next_epoch: Option<InstanceEpoch>,
    #[cfg(feature = "runtime_bench_probes")]
    pub(crate) publication_store_count: u64,
}

impl GateBInstance {
    pub(crate) fn new(instances: usize) -> Self {
        let plan = GateBPlan::from_control_fixture(GateBControlFixture::new(instances));
        let state = GateBArena::activate(instances);
        let workspace = GateBWorkspace::activate(&plan);
        Self {
            plan,
            state,
            workspace,
            published_epoch: AtomicU64::new(0),
            next_epoch: Some(InstanceEpoch(1)),
            #[cfg(feature = "runtime_bench_probes")]
            publication_store_count: 0,
        }
    }

    pub(crate) fn begin_candidate(
        &mut self,
        input: [f64; 4],
    ) -> Result<Candidate<'_>, ResidentExecutionError> {
        let working_epoch = self
            .next_epoch
            .ok_or(ResidentExecutionError::EpochExhausted)?;
        self.next_epoch = working_epoch.checked_next().ok();
        let base_epoch = InstanceEpoch(self.published_epoch.load(Ordering::Acquire));
        let (published_buffer, candidate_buffer) =
            self.state.begin_candidate(base_epoch, working_epoch);
        self.workspace.begin(input);
        Ok(Candidate {
            instance: self,
            base_epoch,
            working_epoch,
            published_buffer,
            candidate_buffer,
            finished: false,
        })
    }

    #[inline]
    pub(crate) fn published_epoch(&self) -> InstanceEpoch {
        InstanceEpoch(self.published_epoch.load(Ordering::Acquire))
    }
}

pub(crate) struct Candidate<'a> {
    pub(crate) instance: &'a mut GateBInstance,
    pub(crate) base_epoch: InstanceEpoch,
    pub(crate) working_epoch: InstanceEpoch,
    pub(crate) published_buffer: usize,
    pub(crate) candidate_buffer: usize,
    finished: bool,
}

impl Candidate<'_> {
    #[inline]
    pub(crate) fn publish(mut self) {
        publish_epoch(&self.instance.published_epoch, self.working_epoch);
        #[cfg(feature = "runtime_bench_probes")]
        {
            self.instance.publication_store_count += 1;
        }
        self.finished = true;
    }

    #[inline]
    pub(crate) fn abort(mut self) {
        self.instance
            .state
            .reject_candidate(self.candidate_buffer, self.working_epoch);
        self.finished = true;
    }
}

impl Drop for Candidate<'_> {
    fn drop(&mut self) {
        debug_assert!(self.finished, "candidate must publish or abort explicitly");
    }
}
