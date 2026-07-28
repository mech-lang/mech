use super::super::{MechRuntime, ResourceBudgetExceededError, RuntimeConfig, TaskId, TaskRecord};

#[test]
fn max_tasks_is_enforced() {
    let mut config = RuntimeConfig::default();
    config.limits.max_tasks = Some(1);
    let mut runtime = MechRuntime::new(config).unwrap();

    runtime
        .put_task(TaskRecord::new(TaskId(1), "task:1"))
        .unwrap();

    let error = runtime
        .put_task(TaskRecord::new(TaskId(2), "task:2"))
        .unwrap_err();
    let budget = error.kind_as::<ResourceBudgetExceededError>().unwrap();
    assert_eq!(budget.resource, "tasks");
    assert_eq!(budget.used, 1);
    assert_eq!(budget.requested, 1);
    assert_eq!(budget.max, Some(1));

    let duplicate = runtime
        .put_task(TaskRecord::new(TaskId(1), "task:1"))
        .unwrap_err();
    assert_eq!(duplicate.kind_name(), "StoreRecordAlreadyExists");
}
