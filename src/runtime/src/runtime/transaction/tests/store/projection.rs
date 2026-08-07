use super::new_runtime;
use crate::{
    RuntimeEventKind,
    turn_record::{
        TurnFailurePhase, TurnFailureRecord, TurnId, TurnRecordHeader, TurnRecordStatus,
        project_transaction_lifecycle_events,
    },
};

fn lifecycle(events: Vec<crate::RuntimeEvent>) -> Vec<RuntimeEventKind> {
    events
        .into_iter()
        .map(|event| event.kind)
        .filter(|kind| {
            matches!(
                kind,
                RuntimeEventKind::TransactionStarted { .. }
                    | RuntimeEventKind::TransactionCommitted { .. }
                    | RuntimeEventKind::TransactionAborted { .. }
            )
        })
        .collect()
}

#[test]
fn accepted_projection_matches_current_standalone_lifecycle_order() {
    let mut runtime = new_runtime();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime.commit_runtime_transaction(&mut context).unwrap();

    let header = TurnRecordHeader {
        turn_id: TurnId::new(1).unwrap(),
        transaction_id,
        input_range: None,
        status: TurnRecordStatus::Accepted,
        failure: None,
    };
    let projected = project_transaction_lifecycle_events(&header)
        .unwrap()
        .collect::<Vec<_>>();
    assert_eq!(projected, lifecycle(runtime.list_events(None).unwrap()));
    assert_eq!(
        projected
            .iter()
            .filter(|kind| matches!(kind, RuntimeEventKind::TransactionCommitted { .. }))
            .count(),
        1
    );
}

#[test]
fn rejected_projection_matches_current_standalone_lifecycle_order() {
    let mut runtime = new_runtime();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime
        .abort_runtime_transaction(&mut context, "rejected")
        .unwrap();

    let header = TurnRecordHeader {
        turn_id: TurnId::new(1).unwrap(),
        transaction_id,
        input_range: None,
        status: TurnRecordStatus::Rejected,
        failure: Some(TurnFailureRecord {
            phase: TurnFailurePhase::Execution,
            kind: "Rejected".to_string(),
            message: "rejected".to_string(),
        }),
    };
    let projected = project_transaction_lifecycle_events(&header)
        .unwrap()
        .collect::<Vec<_>>();
    assert_eq!(projected, lifecycle(runtime.list_events(None).unwrap()));
    assert_eq!(
        projected
            .iter()
            .filter(|kind| matches!(kind, RuntimeEventKind::TransactionAborted { .. }))
            .count(),
        1
    );
}
