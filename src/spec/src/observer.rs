use std::fs;
use std::path::{Path, PathBuf};

use mech_core::{LegacyValue, MResult, Ref};
use mech_runtime::{
    MechRuntime, MessageRecord, ResidentDurabilityPolicy, ResidentRouteFailure,
    ResidentRouteFailureClass, RuntimeBuilder, RuntimeContext, RuntimeEventKind, RuntimeHostInput,
    RuntimeHostInputDriver, RuntimeHostInputSource, RuntimeHostInputValue, RuntimeIngress,
    RuntimeProgramRoute, RuntimeResourceProvider, RuntimeResourceReadRequest, TaskRecord,
};

use crate::case::{CaseManifest, ExecutorCase};
use crate::model::ExecutorObservations;
use crate::{Result, SpecError};

pub(crate) fn observe(
    case: &ExecutorCase,
    repo_root: &Path,
    cases: &[CaseManifest],
) -> Result<ExecutorObservations> {
    if !["RES-001", "TURN-004"]
        .iter()
        .all(|required| case.requirements.iter().any(|item| item == required))
    {
        return Err(SpecError::new(
            "resident transaction case must cover RES-001 and TURN-004",
        ));
    }
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

    let (repository_scanned_paths, repository_parser_import_paths) =
        observe_resident_parser_imports(repo_root)?;
    let benchmark_case = cases
        .iter()
        .find(|manifest| manifest.id == "benchmark-protocol-comparison")
        .ok_or_else(|| SpecError::new("benchmark protocol case is missing"))?;
    let benchmark_reference_protocol = benchmark_case.metadata("reference-protocol")?.to_string();
    let benchmark_candidate_protocol = benchmark_case.metadata("candidate-protocol")?.to_string();
    let activation_case = cases
        .iter()
        .find(|manifest| manifest.id == "activation-missing-grant")
        .ok_or_else(|| SpecError::new("missing-grant activation case is missing"))?;
    let activation_source = activation_case
        .resident_program
        .as_deref()
        .ok_or_else(|| SpecError::new("missing-grant activation case has no resident program"))?;
    let (activation_outcome, activation_failure_class, activation_instance_created) =
        observe_missing_grant_activation(activation_source)?;

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
        activation_outcome,
        activation_failure_class,
        activation_instance_created,
        backend_admission_result: "unsupported".to_string(),
        backend_admission_reason:
            "the prototype has no GPU observation provider and explicitly reports unsupported"
                .to_string(),
        repository_resident_parser_imports: !repository_parser_import_paths.is_empty(),
        repository_parser_import_paths,
        repository_scanned_paths,
        benchmark_reference_protocol,
        benchmark_candidate_protocol,
    })
}

fn observe_missing_grant_activation(source: &str) -> Result<(String, String, bool)> {
    let mut runtime = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .input_driver(SpecObservationInputDriver)
        .resource_provider(Box::new(SpecObservationProvider))
        .build()
        .map_err(|error| SpecError::new(format!("build activation observer: {error:?}")))?;
    let error = match runtime.load_source_program(source, ResidentDurabilityPolicy::Volatile) {
        Ok(outcome) => {
            return Err(SpecError::new(format!(
                "missing hard capability grant unexpectedly activated through {:?}",
                outcome.route
            )));
        }
        Err(error) => error,
    };
    let failure = error.kind_as::<ResidentRouteFailure>().ok_or_else(|| {
        SpecError::new(format!(
            "missing-grant activation returned an unclassified error: {error:?}"
        ))
    })?;
    let failure_class = match failure.class {
        ResidentRouteFailureClass::AuthorizationDenied => "authorization-denied",
        ResidentRouteFailureClass::ProviderUnavailable => "provider-unavailable",
        ResidentRouteFailureClass::ProviderContractMismatch => "provider-contract-mismatch",
        ResidentRouteFailureClass::ActivationFailure => "activation-failure",
        ResidentRouteFailureClass::SemanticUnsupported => "semantic-unsupported",
        ResidentRouteFailureClass::MultipleRootsUnsupported => "multiple-roots-unsupported",
        ResidentRouteFailureClass::InvalidArtifact => "invalid-artifact",
        ResidentRouteFailureClass::InvalidBytecode => "invalid-bytecode",
        ResidentRouteFailureClass::InternalFailure => "internal-failure",
    };
    Ok((
        "rejected".to_string(),
        failure_class.to_string(),
        runtime.program_route() != RuntimeProgramRoute::None,
    ))
}

#[derive(Debug)]
struct SpecObservationInputDriver;

impl RuntimeHostInputDriver for SpecObservationInputDriver {
    fn drives(&self, source: &RuntimeHostInputSource) -> bool {
        source.base_uri() == "test://clock/tick"
    }

    fn attach(&mut self, _ingress: RuntimeIngress) -> MResult<()> {
        Ok(())
    }

    fn start(&mut self) -> MResult<()> {
        Ok(())
    }

    fn stop(&mut self) -> MResult<()> {
        Ok(())
    }

    fn is_live(&self) -> bool {
        false
    }
}

#[derive(Debug)]
struct SpecObservationProvider;

impl RuntimeResourceProvider for SpecObservationProvider {
    fn scheme(&self) -> &str {
        "test"
    }

    fn base_uris(&self) -> Vec<String> {
        vec!["test://clock/tick".to_string()]
    }

    fn semantic_read_contract(&self) -> Option<&'static mech_core::OperationContractDeclaration> {
        Some(mech_runtime::resource_observation_contract())
    }

    fn plan_read(&self, _request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        Ok(LegacyValue::F64(Ref::new(1.0)))
    }

    fn read(&self, _request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        Ok(LegacyValue::F64(Ref::new(1.0)))
    }
}

fn observe_resident_parser_imports(repo_root: &Path) -> Result<(Vec<String>, Vec<String>)> {
    let program_root = repo_root.join("src/runtime/src/runtime/program");
    let mut files = Vec::new();
    collect_rust_files(&program_root, &mut files)?;
    files.retain(|path| {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        file_name != "compiler.rs"
            && !file_name.ends_with("tests.rs")
            && !path
                .components()
                .any(|component| component.as_os_str() == "tests")
    });
    files.sort();
    let mut scanned = Vec::new();
    let mut imports = Vec::new();
    for path in files {
        let relative = path
            .strip_prefix(repo_root)
            .unwrap_or(&path)
            .display()
            .to_string();
        let source = fs::read_to_string(&path).map_err(|error| {
            SpecError::new(format!(
                "repository provider could not read {}: {error}",
                path.display()
            ))
        })?;
        if source.contains("mech_syntax::") || source.contains("use mech_syntax") {
            imports.push(relative.clone());
        }
        scanned.push(relative);
    }
    if scanned.is_empty() {
        return Err(SpecError::new(format!(
            "repository provider found no resident execution modules under {}",
            program_root.display()
        )));
    }
    Ok((scanned, imports))
}

fn collect_rust_files(directory: &Path, output: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(directory).map_err(|error| {
        SpecError::new(format!(
            "repository provider could not inspect {}: {error}",
            directory.display()
        ))
    })? {
        let entry =
            entry.map_err(|error| SpecError::new(format!("inspect repository: {error}")))?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|error| {
            SpecError::new(format!(
                "inspect repository path {}: {error}",
                path.display()
            ))
        })?;
        if file_type.is_dir() {
            collect_rust_files(&path, output)?;
        } else if file_type.is_file() && path.extension().is_some_and(|value| value == "rs") {
            output.push(path);
        }
    }
    Ok(())
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
