use super::*;
use crate::{
    MaterializedHostContext, MaterializedHostInterface, PreparedRuntimeEffect,
    ResourcePathCapability, ResourcePathScope, RunResourceGrantConfig, RuntimeAfterCommitEffect,
    RuntimeBuilder, RuntimeEffectMetadata, RuntimeEffectSource, RuntimeHostInputValue,
    RuntimeResourceProvider, RuntimeResourceWritePreflightRequest, RuntimeResourceWriteRequest,
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Debug, Default)]
struct State {
    observations: AtomicUsize,
    preparations: AtomicUsize,
    deliveries: AtomicUsize,
    output: Mutex<String>,
}

#[derive(Clone, Copy, Debug)]
enum Mode {
    Normal,
    MissingReadContract,
    LiveRead,
    NonStringRead,
    UnsupportedSend,
}

#[derive(Debug)]
struct Provider {
    state: Arc<State>,
    mode: Mode,
}

fn text(value: &str) -> Value {
    RuntimeHostInputValue::String(value.into())
        .into_value()
        .unwrap()
}

impl RuntimeResourceProvider for Provider {
    fn scheme(&self) -> &str {
        "native-test"
    }
    fn base_uris(&self) -> Vec<String> {
        [
            "native-test://cli/args",
            "native-test://args",
            "native-test://cli/stdout",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    }
    fn equivalent_base_uri_groups(&self) -> Vec<Vec<String>> {
        vec![vec![
            "native-test://cli/args".into(),
            "native-test://args".into(),
        ]]
    }
    fn semantic_read_contract(&self) -> Option<&'static mech_core::OperationContractDeclaration> {
        (!matches!(self.mode, Mode::MissingReadContract)).then(crate::resource_observation_contract)
    }
    fn semantic_write_contract(
        &self,
        _: RuntimeResourceWriteIntent,
    ) -> Option<&'static mech_core::OperationContractDeclaration> {
        Some(if matches!(self.mode, Mode::UnsupportedSend) {
            crate::provider_defined_effect_contract()
        } else {
            crate::compute_effect_contract()
        })
    }
    fn observation_requires_input_driver(&self, _: &RuntimeResourceReadRequest) -> bool {
        matches!(self.mode, Mode::LiveRead)
    }
    fn plan_read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        if !request.base_uri.ends_with("args") || !matches!(request.path.as_str(), "1" | "2") {
            return invalid("invalid test argument path");
        }
        if matches!(self.mode, Mode::NonStringRead) {
            return mech_core::ValueCell::unit().snapshot();
        }
        Ok(text(""))
    }
    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        self.plan_read(request)?;
        self.state.observations.fetch_add(1, Ordering::SeqCst);
        Ok(text("5500"))
    }
    fn preflight_write(&self, request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
        if request.base_uri != "native-test://cli/stdout"
            || request.path != "line"
            || request.intent != RuntimeResourceWriteIntent::Send
        {
            return invalid("invalid test output path or intent");
        }
        Ok(())
    }
    fn plan_write(&self, request: RuntimeResourceWriteCommand) -> MResult<()> {
        if !matches!(request.value.data(), ValueData::String(_)) {
            return invalid("test output requires text");
        }
        self.preflight_write(RuntimeResourceWritePreflightRequest {
            base_uri: request.base_uri,
            path: request.path,
            context_name: request.context_name,
            operation: request.operation,
            intent: request.intent,
        })
    }
    fn prepare_write(
        &self,
        request: RuntimeResourceWriteRequest,
    ) -> MResult<PreparedRuntimeEffect> {
        let ValueData::String(value) = request.value.data() else {
            return invalid("test output requires text");
        };
        self.state.preparations.fetch_add(1, Ordering::SeqCst);
        Ok(PreparedRuntimeEffect::AfterCommit(Box::new(Output {
            state: self.state.clone(),
            value: value.to_string(),
        })))
    }
}

#[derive(Debug)]
struct Output {
    state: Arc<State>,
    value: String,
}
impl RuntimeAfterCommitEffect for Output {
    fn metadata(&self) -> RuntimeEffectMetadata {
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::ResourceProvider {
                scheme: "native-test".into(),
            },
            "write",
        )
    }
    fn deliver(&mut self) -> MResult<()> {
        self.state.deliveries.fetch_add(1, Ordering::SeqCst);
        self.state.output.lock().unwrap().push_str(&self.value);
        Ok(())
    }
}

fn runtime(mode: Mode) -> (MechRuntime, Arc<State>) {
    let state = Arc::new(State::default());
    let mut runtime = RuntimeBuilder::new()
        .resource_provider(Box::new(Provider {
            state: state.clone(),
            mode,
        }))
        .build()
        .unwrap();
    runtime
        .host_interfaces
        .register(MaterializedHostInterface {
            instance: "cli".into(),
            provider: "native-test".into(),
            contexts: vec![
                MaterializedHostContext {
                    name: "args".into(),
                    base_uri: "native-test://cli/args".into(),
                    operations: vec!["read".into()],
                },
                MaterializedHostContext {
                    name: "stdout".into(),
                    base_uri: "native-test://cli/stdout".into(),
                    operations: vec!["write".into()],
                },
            ],
        })
        .unwrap();
    (runtime, state)
}

fn read() -> ExecutionResourceRequest {
    ExecutionResourceRequest {
        base_uri: "native-test://cli/args".into(),
        path: "1".into(),
        context_name: "args".into(),
        operation: "read".into(),
        intent: ResourceIntent::Read,
        delivery: ResourceDelivery::Snapshot,
    }
}
fn send() -> ExecutionResourceRequest {
    ExecutionResourceRequest {
        base_uri: "native-test://cli/stdout".into(),
        path: "line".into(),
        context_name: "stdout".into(),
        operation: "write".into(),
        intent: ResourceIntent::Send,
        delivery: ResourceDelivery::Snapshot,
    }
}
fn grant(runtime: &mut MechRuntime, context: &str, operation: &str, path: &str) {
    runtime
        .install_run_resource_grant(&RunResourceGrantConfig {
            target: format!("cli/{context}"),
            operations: vec![operation.into()],
            paths: vec![path.into()],
        })
        .unwrap();
}
fn quiet(state: &State) {
    assert_eq!(state.observations.load(Ordering::SeqCst), 0);
    assert_eq!(state.preparations.load(Ordering::SeqCst), 0);
    assert_eq!(state.deliveries.load(Ordering::SeqCst), 0);
    assert!(state.output.lock().unwrap().is_empty());
}

#[test]
fn denied_requirements_fail_before_any_observation_or_output() {
    let (mut runtime, state) = runtime(Mode::Normal);
    assert!(runtime.bind_native_resources(&[read(), send()]).is_err());
    grant(&mut runtime, "args", "read", "1");
    assert!(runtime.bind_native_resources(&[read(), send()]).is_err());
    quiet(&state);
}

#[test]
fn declared_aliases_and_exact_paths_use_existing_authority() {
    let (mut runtime, state) = runtime(Mode::Normal);
    grant(&mut runtime, "args", "read", "1");
    grant(&mut runtime, "stdout", "write", "line");
    let mut alias = read();
    alias.base_uri = "native-test://args".into();
    let bindings = runtime
        .bind_native_resources(&[alias.clone(), send()])
        .unwrap();
    assert_eq!(bindings.len(), 2);
    assert!(!bindings.is_empty());
    quiet(&state);
    let value = runtime.read_native_resource(&bindings, 0).unwrap();
    assert!(matches!(value.data(), ValueData::String(value) if value.as_ref() == "5500"));
    runtime
        .write_native_resource(&bindings, 1, text("result"))
        .unwrap();
    assert_eq!(state.observations.load(Ordering::SeqCst), 1);
    assert_eq!(state.preparations.load(Ordering::SeqCst), 1);
    assert_eq!(state.deliveries.load(Ordering::SeqCst), 1);
    assert_eq!(*state.output.lock().unwrap(), "result");
    alias.path = "2".into();
    assert!(runtime.bind_native_resources(&[alias]).is_err());
}

#[test]
fn bindings_reject_wrong_runtime_index_intent_and_payload() {
    let (mut first, state) = runtime(Mode::Normal);
    grant(&mut first, "args", "read", "1");
    grant(&mut first, "stdout", "write", "line");
    let bindings = first.bind_native_resources(&[read(), send()]).unwrap();
    let (mut other, other_state) = runtime(Mode::Normal);
    assert!(other.read_native_resource(&bindings, 0).is_err());
    assert!(
        other
            .write_native_resource(&bindings, 1, text("wrong runtime"))
            .is_err()
    );
    assert!(first.read_native_resource(&bindings, 2).is_err());
    assert!(first.read_native_resource(&bindings, 1).is_err());
    assert!(
        first
            .write_native_resource(&bindings, 0, text("wrong intent"))
            .is_err()
    );
    assert!(
        first
            .write_native_resource(
                &bindings,
                1,
                mech_core::ValueCell::unit().snapshot().unwrap()
            )
            .is_err()
    );
    quiet(&state);
    quiet(&other_state);
}

#[test]
fn revocation_is_checked_again_when_a_bound_port_is_used() {
    let (mut runtime, state) = runtime(Mode::Normal);
    let capability = ResourcePathCapability::new(
        runtime.next_capability_id(),
        runtime.runtime_context().unwrap().subject,
        "native-test://cli/args",
        ["read"],
        [ResourcePathScope::Exact("1".into())],
    )
    .unwrap();
    let id = runtime.grant_capability(Arc::new(capability)).unwrap();
    let bindings = runtime.bind_native_resources(&[read()]).unwrap();
    runtime.revoke_capability(id).unwrap();
    assert!(runtime.read_native_resource(&bindings, 0).is_err());
    quiet(&state);
}

#[test]
fn binding_previews_do_not_consume_finite_grant_uses() {
    let (mut runtime, state) = runtime(Mode::Normal);
    let capability = crate::BasicCapability::from_keys(
        runtime.next_capability_id(),
        runtime.runtime_context().unwrap().subject,
        "native-test://cli/args/1",
        ["read"],
    )
    .with_constraints(crate::BasicConstraints {
        max_uses: Some(1),
        ..Default::default()
    });
    runtime.grant_capability(Arc::new(capability)).unwrap();
    let bindings = runtime.bind_native_resources(&[read()]).unwrap();
    runtime.bind_native_resources(&[read()]).unwrap();
    quiet(&state);
    runtime.read_native_resource(&bindings, 0).unwrap();
    assert!(runtime.read_native_resource(&bindings, 0).is_err());
    assert_eq!(state.observations.load(Ordering::SeqCst), 1);
}

#[test]
fn explicit_transaction_delivers_native_sends_only_after_commit_in_order() {
    let (mut runtime, state) = runtime(Mode::Normal);
    grant(&mut runtime, "args", "read", "1");
    grant(&mut runtime, "stdout", "write", "line");
    let bindings = runtime.bind_native_resources(&[read(), send()]).unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction = runtime.begin_transaction(&mut context).unwrap();

    let value = runtime
        .read_native_resource_with_context(&bindings, 0, &mut context)
        .unwrap();
    assert!(matches!(value.data(), ValueData::String(value) if value.as_ref() == "5500"));
    for value in ["first\n", "second\n"] {
        runtime
            .write_native_resource_with_context(&bindings, 1, text(value), &mut context)
            .unwrap();
    }
    assert_eq!(context.transaction, Some(transaction));
    assert_eq!(state.observations.load(Ordering::SeqCst), 1);
    assert_eq!(state.preparations.load(Ordering::SeqCst), 2);
    assert_eq!(state.deliveries.load(Ordering::SeqCst), 0);
    assert!(state.output.lock().unwrap().is_empty());

    assert_eq!(
        runtime.commit_runtime_transaction(&mut context).unwrap(),
        transaction
    );
    assert_eq!(context.transaction, None);
    assert_eq!(state.deliveries.load(Ordering::SeqCst), 2);
    assert_eq!(*state.output.lock().unwrap(), "first\nsecond\n");
}

#[test]
fn later_failure_can_abort_all_prepared_native_sends() {
    let (mut runtime, state) = runtime(Mode::Normal);
    grant(&mut runtime, "stdout", "write", "line");
    let bindings = runtime.bind_native_resources(&[send()]).unwrap();
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();
    for value in ["first", "second"] {
        runtime
            .write_native_resource_with_context(&bindings, 0, text(value), &mut context)
            .unwrap();
    }
    assert!(
        runtime
            .write_native_resource_with_context(
                &bindings,
                0,
                mech_core::ValueCell::unit().snapshot().unwrap(),
                &mut context,
            )
            .is_err()
    );
    assert_eq!(state.preparations.load(Ordering::SeqCst), 2);
    assert_eq!(state.deliveries.load(Ordering::SeqCst), 0);
    runtime
        .abort_runtime_transaction(&mut context, "native execution failed")
        .unwrap();
    assert_eq!(context.transaction, None);
    assert_eq!(state.deliveries.load(Ordering::SeqCst), 0);
    assert!(state.output.lock().unwrap().is_empty());
}

#[test]
fn context_bound_calls_reject_another_runtimes_context_before_io() {
    let (mut runtime, state) = runtime(Mode::Normal);
    grant(&mut runtime, "args", "read", "1");
    grant(&mut runtime, "stdout", "write", "line");
    let bindings = runtime.bind_native_resources(&[read(), send()]).unwrap();
    let (other, other_state) = self::runtime(Mode::Normal);
    let mut context = other.runtime_context().unwrap();
    assert!(
        runtime
            .read_native_resource_with_context(&bindings, 0, &mut context)
            .is_err()
    );
    assert!(
        runtime
            .write_native_resource_with_context(&bindings, 1, text("denied"), &mut context)
            .is_err()
    );
    quiet(&state);
    quiet(&other_state);
}

#[test]
fn open_native_transaction_rechecks_local_and_kernel_revocations() {
    let (mut runtime, state) = runtime(Mode::Normal);
    let subject = runtime.runtime_context().unwrap().subject;
    let [read_capability, send_capability] = [read(), send()].map(|request| {
        let capability = ResourcePathCapability::new(
            runtime.next_capability_id(),
            &subject,
            request.base_uri,
            [request.operation],
            [ResourcePathScope::Exact(request.path)],
        )
        .unwrap();
        runtime.grant_capability(Arc::new(capability)).unwrap()
    });
    let bindings = runtime.bind_native_resources(&[read(), send()]).unwrap();
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();
    runtime
        .read_native_resource_with_context(&bindings, 0, &mut context)
        .unwrap();
    runtime
        .write_native_resource_with_context(&bindings, 1, text("pending"), &mut context)
        .unwrap();

    // Transaction-local revocation must affect the next call immediately.
    runtime
        .revoke_capability_with_context(&mut context, read_capability)
        .unwrap();
    assert!(
        runtime
            .read_native_resource_with_context(&bindings, 0, &mut context)
            .is_err()
    );
    // A host revocation outside this context also invalidates an already-used
    // bound port while its earlier effect is still pending in the transaction.
    runtime.revoke_capability(send_capability).unwrap();
    assert!(
        runtime
            .write_native_resource_with_context(&bindings, 1, text("denied"), &mut context)
            .is_err()
    );
    assert_eq!(state.observations.load(Ordering::SeqCst), 1);
    assert_eq!(state.preparations.load(Ordering::SeqCst), 1);
    assert_eq!(state.deliveries.load(Ordering::SeqCst), 0);
    runtime
        .abort_runtime_transaction(&mut context, "authority changed")
        .unwrap();
    assert!(state.output.lock().unwrap().is_empty());

    // Abort rolls back only the local revocation; the host's live revocation
    // remains effective after the native entry has been discarded.
    runtime.read_native_resource(&bindings, 0).unwrap();
    assert!(
        runtime
            .write_native_resource(&bindings, 1, text("still denied"))
            .is_err()
    );
    assert_eq!(state.observations.load(Ordering::SeqCst), 2);
    assert_eq!(state.deliveries.load(Ordering::SeqCst), 0);
}

#[test]
fn unsupported_contracts_driver_inputs_and_types_do_not_observe() {
    for mode in [
        Mode::MissingReadContract,
        Mode::LiveRead,
        Mode::NonStringRead,
    ] {
        let (mut runtime, state) = runtime(mode);
        grant(&mut runtime, "args", "read", "1");
        assert!(
            runtime.bind_native_resources(&[read()]).is_err(),
            "{mode:?}"
        );
        quiet(&state);
    }
    let (mut runtime, state) = runtime(Mode::UnsupportedSend);
    grant(&mut runtime, "stdout", "write", "line");
    assert!(runtime.bind_native_resources(&[send()]).is_err());
    quiet(&state);
}

#[test]
fn malformed_identity_context_path_and_unsupported_intent_are_rejected() {
    let (mut runtime, state) = runtime(Mode::Normal);
    grant(&mut runtime, "args", "read", "*");
    grant(&mut runtime, "stdout", "write", "*");
    let mut variants = Vec::new();
    let mut request = read();
    request.delivery = ResourceDelivery::Live;
    variants.push(request);
    let mut request = send();
    request.intent = ResourceIntent::Assign;
    variants.push(request);
    let mut request = read();
    request.context_name = "stdout".into();
    variants.push(request);
    let mut request = read();
    request.operation = "write".into();
    variants.push(request);
    let mut request = read();
    request.path = "../1".into();
    variants.push(request);
    let mut request = read();
    request.path = "3".into();
    variants.push(request);
    let mut request = read();
    request.base_uri.push('/');
    variants.push(request);
    let mut request = send();
    request.path = "invalid".into();
    variants.push(request);
    for request in variants {
        assert!(runtime.bind_native_resources(&[request]).is_err());
    }
    quiet(&state);
    runtime.resources = crate::RuntimeResourceRegistry::new();
    assert!(runtime.bind_native_resources(&[read()]).is_err());
    quiet(&state);
}
