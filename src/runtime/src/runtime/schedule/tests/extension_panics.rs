use super::super::MechRuntime;
use crate::{
    InMemoryScheduler, RuntimeEventKind, RuntimeTurnOutcome, ScheduledWork, ScheduledWorkFailure,
    ScheduledWorkOutcome, Scheduler, TaskId,
};
use mech_core::MResult;

#[derive(Debug, Default)]
struct PanickingScheduler {
    inner: InMemoryScheduler,
}

impl Scheduler for PanickingScheduler {
    fn enqueue_work(&mut self, _work: ScheduledWork) -> MResult<()> {
        panic!("deliberate scheduler enqueue panic");
    }

    fn next_work(&mut self) -> MResult<Option<ScheduledWork>> {
        self.inner.next_work()
    }

    fn complete_work(&mut self, work: ScheduledWork, outcome: RuntimeTurnOutcome) -> MResult<()> {
        self.inner.complete_work(work, outcome)
    }

    fn fail_work(&mut self, work: ScheduledWork, message: String) -> MResult<()> {
        self.inner.fail_work(work, message)
    }

    fn begin_tick(&mut self) -> MResult<()> {
        self.inner.begin_tick()
    }

    fn complete_tick(&mut self, work_count: u64) -> MResult<()> {
        self.inner.complete_tick(work_count)
    }

    fn len(&self) -> usize {
        self.inner.len()
    }

    fn queued_work(&self) -> Vec<ScheduledWork> {
        self.inner.queued_work()
    }

    fn completed(&self) -> &[ScheduledWorkOutcome] {
        self.inner.completed()
    }

    fn failures(&self) -> &[ScheduledWorkFailure] {
        self.inner.failures()
    }

    fn pending_events(&self) -> &[RuntimeEventKind] {
        self.inner.pending_events()
    }

    fn drain_events(&mut self) -> Vec<RuntimeEventKind> {
        self.inner.drain_events()
    }
}

#[test]
fn scheduler_panic_is_converted_without_poisoning() {
    let mut runtime = MechRuntime::builder()
        .scheduler(PanickingScheduler::default())
        .build()
        .unwrap();

    let error = runtime.enqueue_task(TaskId(1)).unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeExtensionPanicked");
    assert!(format!("{error:?}").contains("deliberate scheduler enqueue panic"));
    assert!(!runtime.is_poisoned());
    runtime.list_events(None).unwrap();
}
