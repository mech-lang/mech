use mech_runtime::{
    MechRuntime, MessageRecord, ResidentDurabilityPolicy, RuntimeBuilder, RuntimeContext,
    RuntimeEventKind, RuntimeHostInput, RuntimeHostInputSource, RuntimeHostInputValue,
    RuntimeProgramRoute, TaskRecord,
};

use crate::case::ExecutorCase;
use crate::model::ExecutorObservations;
use crate::{Result, SpecError};

pub(crate) fn observe(case: &ExecutorCase) -> Result<ExecutorObservations> {
    let mut runtime = runtime()?;

    let loaded = runtime_call(
        "load case through the v0.4 resident executor",
        runtime.load_source_program(case.resident_program(), ResidentDurabilityPolicy::Volatile),
    )?;
    let resident_route = match loaded.route {
        RuntimeProgramRoute::ResidentPure => "resident-pure",
        RuntimeProgramRoute::ResidentExternal => "resident-external",
        RuntimeProgramRoute::None => "none",
    }
    .to_string();
    let resident_initial_state = resident_state(&runtime, "next-turn-state")?;
    runtime_call("advance resident program", runtime.step_active_program())?;
    let resident_next_turn_state = resident_state(&runtime, "next-turn-state")?;
    let resident_turns_advanced = runtime.program_execution_info().resident_accepted_turns == 2;
    runtime_call("unload resident program", runtime.unload_active_program())?;

    let actor = runtime_call(
        "create conformance mailbox",
        runtime.create_actor("spec:transaction-mailbox", None, None, Vec::new()),
    )?;
    runtime_call(
        "enqueue initial mailbox state",
        runtime.send_message(actor, "initial", case.initial_state.as_bytes().to_vec()),
    )?;
    runtime_call(
        "enqueue committed mailbox state",
        runtime.send_message(actor, "committed", case.committed_state.as_bytes().to_vec()),
    )?;

    let abort_before_state = mailbox_state(runtime_call(
        "read state before abort",
        runtime.peek_message(actor),
    )?)?;
    let mut abort_context = mailbox_context(&mut runtime)?;
    let abort_id = runtime_call(
        "begin explicit abort transaction",
        runtime.begin_transaction(&mut abort_context),
    )?;
    runtime_call(
        "stage state transition for abort",
        runtime.pop_message_with_context(&mut abort_context, actor),
    )?
    .ok_or_else(|| SpecError::new("abort transaction had no initial state"))?;
    let abort_visible_state = mailbox_state(runtime_call(
        "read state inside abort transaction",
        runtime.peek_message_with_context(&mut abort_context, actor),
    )?)?;
    runtime_call(
        "abort explicit transaction",
        runtime.abort_runtime_transaction(&mut abort_context, "conformance case requested abort"),
    )?;
    let abort_after_state = mailbox_state(runtime_call(
        "read state after abort",
        runtime.peek_message(actor),
    )?)?;

    let commit_before_state = mailbox_state(runtime_call(
        "read state before commit",
        runtime.peek_message(actor),
    )?)?;
    let mut commit_context = mailbox_context(&mut runtime)?;
    let commit_id = runtime_call(
        "begin explicit commit transaction",
        runtime.begin_transaction(&mut commit_context),
    )?;
    runtime_call(
        "stage state transition for commit",
        runtime.pop_message_with_context(&mut commit_context, actor),
    )?
    .ok_or_else(|| SpecError::new("commit transaction had no initial state"))?;
    let commit_visible_state = mailbox_state(runtime_call(
        "read state inside commit transaction",
        runtime.peek_message_with_context(&mut commit_context, actor),
    )?)?;
    let committed_id = runtime_call(
        "commit explicit transaction",
        runtime.commit_runtime_transaction(&mut commit_context),
    )?;
    if committed_id != commit_id {
        return Err(SpecError::new(format!(
            "v0.4 runtime committed transaction {committed_id}, expected {commit_id}",
        )));
    }
    let commit_after_state = mailbox_state(runtime_call(
        "read state after commit",
        runtime.peek_message(actor),
    )?)?;

    let mut next_turn_context = mailbox_context(&mut runtime)?;
    runtime_call(
        "begin next-turn observation",
        runtime.begin_transaction(&mut next_turn_context),
    )?;
    let next_turn_state = mailbox_state(runtime_call(
        "read committed state in next turn",
        runtime.peek_message_with_context(&mut next_turn_context, actor),
    )?)?;
    runtime_call(
        "close next-turn observation",
        runtime.abort_runtime_transaction(&mut next_turn_context, "read-only conformance probe"),
    )?;

    runtime_call("shut down runtime", runtime.shutdown())?;
    let shutdown_ingress_closed =
        runtime_call("read shutdown ingress state", runtime.ingress().is_closed())?;
    let shutdown_input_rejected = runtime
        .ingress()
        .submit(RuntimeHostInput::single(
            runtime_call(
                "construct shutdown probe source",
                RuntimeHostInputSource::new("spec://shutdown", "probe"),
            )?,
            RuntimeHostInputValue::String("probe".to_string()),
        ))
        .is_err();

    let events = runtime_call("read semantic runtime events", runtime.list_events(None))?;
    let commit_event_observed = events.iter().any(|event| {
        matches!(
            event.kind,
            RuntimeEventKind::TransactionCommitted { transaction_id }
                if transaction_id == commit_id
        )
    });
    let abort_event_observed = events.iter().any(|event| {
        matches!(
            event.kind,
            RuntimeEventKind::TransactionAborted { transaction_id, .. }
                if transaction_id == abort_id
        )
    });
    let shutdown_event_observed = events
        .iter()
        .any(|event| matches!(event.kind, RuntimeEventKind::RuntimeShutdown { .. }));
    let commit_record_observed = runtime_call(
        "read committed transaction record",
        runtime.get_transaction(commit_id),
    )?
    .is_some();
    let event_names = events
        .into_iter()
        .map(|event| event.name().to_string())
        .collect();

    Ok(ExecutorObservations {
        case_id: case.id.clone(),
        executor: "mech-runtime v0.4 resident executor".to_string(),
        resident_route,
        resident_initial_state,
        resident_next_turn_state,
        resident_turns_advanced,
        commit_before_state,
        commit_visible_state,
        commit_after_state,
        commit_outcome: "commit".to_string(),
        commit_record_observed,
        commit_event_observed,
        abort_before_state,
        abort_visible_state,
        abort_after_state,
        abort_outcome: "abort".to_string(),
        abort_event_observed,
        next_turn_state,
        shutdown_ingress_closed,
        shutdown_input_rejected,
        shutdown_event_observed,
        event_names,
    })
}

fn runtime() -> Result<MechRuntime> {
    runtime_call(
        "build v0.4 runtime",
        RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_catalog())
            .build(),
    )
}

fn mailbox_context(runtime: &mut MechRuntime) -> Result<RuntimeContext> {
    let task = TaskRecord::new(runtime.next_task_id(), "spec:transaction-mailbox");
    runtime_call(
        "create mailbox transaction context",
        runtime.context_for_task(&task),
    )
}

fn mailbox_state(message: Option<MessageRecord>) -> Result<String> {
    let message = message.ok_or_else(|| SpecError::new("mailbox state was unexpectedly empty"))?;
    String::from_utf8(message.payload)
        .map_err(|error| SpecError::new(format!("mailbox state was not UTF-8: {error}")))
}

fn resident_state(runtime: &MechRuntime, symbol: &str) -> Result<String> {
    runtime_call(
        &format!("observe resident symbol `{symbol}`"),
        runtime.root_symbol_value(symbol),
    )
    .map(|snapshot| snapshot.to_string())
}

fn runtime_call<T>(operation: &str, result: mech_core::MResult<T>) -> Result<T> {
    result.map_err(|error| {
        SpecError::new(format!("{operation} failed in the v0.4 runtime: {error:?}",))
    })
}
