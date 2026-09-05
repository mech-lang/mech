use std::{
    collections::{BTreeMap, BTreeSet},
    num::NonZeroU32,
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, Ordering},
    },
};

use mech_compute::{
    BackendId, BackendRequest, ComputeBackendRegistry, ComputeCompletionOutcome,
    ComputeCompletionTarget, ComputeDispatchDisposition, ComputeDispatchReport,
    ComputeDispatchRequest, ComputeExecutionError, ComputeInitializerSet, ComputeInputUpdate,
    ComputeOutputSelection, ComputeOutputSnapshot, ComputePlatform, ComputePort, ComputeProgram,
    ComputeSession, ComputeValue, TensorLayout,
};
use mech_core::{
    ComputePlacement, MResult, MechError, MechErrorKind, OperationContractDeclaration, Ref, Value,
};
use mech_runtime::{
    ConfigValue, HostManifestConfig, PreparedRuntimeEffect, RuntimeAfterCommitEffect,
    RuntimeEffectCost, RuntimeEffectMetadata, RuntimeEffectSource, RuntimeHostFactory,
    RuntimeHostInput, RuntimeHostInputDriver, RuntimeHostInputSource, RuntimeHostInputUpdate,
    RuntimeHostInputValue, RuntimeHostInstallation, RuntimeIngress, RuntimeResourceProvider,
    RuntimeResourceReadRequest, RuntimeResourceWriteIntent, RuntimeResourceWritePreflightRequest,
    RuntimeResourceWriteRequest, materialize_host_manifest,
};

/// Installs one already-lowered compute program behind the ordinary resident
/// runtime host boundary. Backend selection and compilation are intentionally
/// contained here; compiler partitioning and artifact-to-program lowering
/// happen before this factory is constructed.
pub struct ComputeHostFactory {
    region: Box<str>,
    placement: ComputePlacement,
    program: Arc<ComputeProgram>,
    initializers: ComputeInitializerSet,
    registry: Arc<ComputeBackendRegistry>,
    platform: ComputePlatform,
    backend_override: Option<BackendRequest>,
    installed_instance: Mutex<Option<String>>,
    state_snapshot: ComputeHostStateSnapshotHandle,
    resume_state: Option<ComputeHostResumeState>,
    retained_outputs: BTreeSet<String>,
    manifest: HostManifestConfig,
}

impl std::fmt::Debug for ComputeHostFactory {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ComputeHostFactory")
            .field("region", &self.region)
            .field("placement", &self.placement)
            .field("platform", &self.platform)
            .field("backend_override", &self.backend_override)
            .field("registry", &self.registry)
            .finish_non_exhaustive()
    }
}

impl ComputeHostFactory {
    pub fn new(
        region: impl Into<Box<str>>,
        placement: ComputePlacement,
        program: ComputeProgram,
        initializers: ComputeInitializerSet,
        registry: Arc<ComputeBackendRegistry>,
        platform: ComputePlatform,
    ) -> MResult<Self> {
        let region = region.into();
        if region.trim().is_empty() {
            return Err(compute_host_error(
                "ComputeHostConfiguration",
                "the configured compute region name must be nonempty",
            ));
        }
        Ok(Self {
            region,
            placement,
            program: Arc::new(program),
            initializers,
            registry,
            platform,
            backend_override: None,
            installed_instance: Mutex::new(None),
            state_snapshot: ComputeHostStateSnapshotHandle::default(),
            resume_state: None,
            retained_outputs: BTreeSet::new(),
            manifest: compute_host_manifest(),
        })
    }

    pub fn with_backend_override(mut self, request: BackendRequest) -> Self {
        self.backend_override = Some(request);
        self
    }

    pub fn state_snapshot_handle(&self) -> ComputeHostStateSnapshotHandle {
        self.state_snapshot.clone()
    }

    pub fn with_resume_state(mut self, state: ComputeHostResumeState) -> Self {
        self.resume_state = Some(state);
        self
    }

    pub fn with_retained_outputs(mut self, outputs: BTreeSet<String>) -> MResult<Self> {
        for output in &outputs {
            if !self
                .program
                .interface()
                .outputs
                .iter()
                .any(|port| port.name.as_ref() == output)
            {
                return Err(compute_host_error(
                    "ComputeHostConfiguration",
                    format!("retained compute output `{output}` is not declared"),
                ));
            }
        }
        self.retained_outputs = outputs;
        Ok(self)
    }

    pub fn resolved_backend_id(&self, settings: &ConfigValue) -> MResult<mech_compute::BackendId> {
        let request = self.configured_request(settings)?;
        self.registry
            .resolve(&request, self.platform, self.placement, &self.program)
            .map(|backend| backend.descriptor().id.clone())
            .map_err(|error| {
                compute_host_error(
                    "ComputeBackendSelection",
                    format!(
                        "region `{}` has no compatible backend: {error}",
                        self.region
                    ),
                )
            })
    }

    fn configured_request(&self, settings: &ConfigValue) -> MResult<BackendRequest> {
        let configured = configured_compute_settings(settings)?;
        if configured.region != self.region.as_ref() {
            return Err(compute_host_error(
                "ComputeHostConfiguration",
                format!(
                    "configured region `{}` does not match compiled region `{}`",
                    configured.region, self.region
                ),
            ));
        }
        Ok(self.backend_override.clone().unwrap_or(configured.backend))
    }
}

impl RuntimeHostFactory for ComputeHostFactory {
    fn provider_name(&self) -> &str {
        "compute"
    }

    fn manifest(&self) -> &HostManifestConfig {
        &self.manifest
    }

    fn validate_settings(&self, _instance_name: &str, settings: &ConfigValue) -> MResult<()> {
        let request = self.configured_request(settings)?;
        self.registry
            .resolve(&request, self.platform, self.placement, &self.program)
            .map(|_| ())
            .map_err(|error| {
                compute_host_error(
                    "ComputeBackendSelection",
                    format!(
                        "region `{}` has no compatible backend: {error}",
                        self.region
                    ),
                )
            })
    }

    fn instantiate(
        &self,
        instance_name: &str,
        settings: &ConfigValue,
    ) -> MResult<RuntimeHostInstallation> {
        let mut installed = self.installed_instance.lock().map_err(|_| {
            compute_host_error(
                "ComputeHostConfiguration",
                "compute host installation lock is poisoned",
            )
        })?;
        if let Some(existing) = installed.as_ref() {
            return Err(compute_host_error(
                "MultipleComputeHostsUnsupported",
                format!(
                    "v0.4 supports one configured compute host; instance `{existing}` is already installed"
                ),
            ));
        }

        let request = self.configured_request(settings)?;
        let backend = self
            .registry
            .resolve(&request, self.platform, self.placement, &self.program)
            .map_err(|error| {
                compute_host_error(
                    "ComputeBackendSelection",
                    format!(
                        "region `{}` has no compatible backend: {error}",
                        self.region
                    ),
                )
            })?;
        let backend_id = backend.descriptor().id.clone();
        let executable = backend.compile(&self.program).map_err(|error| {
            compute_host_error(
                "ComputeBackendCompile",
                format!("region `{}` could not compile: {error}", self.region),
            )
        })?;
        let session = executable
            .create_session(&self.initializers)
            .map_err(|error| {
                compute_host_error(
                    "ComputeBackendInitialize",
                    format!("region `{}` could not initialize: {error}", self.region),
                )
            })?;

        let telemetry = Arc::new(Mutex::new(None));
        let live = Arc::new(AtomicBool::new(false));
        let base_uri = format!("compute://{instance_name}/kernel");
        let resume = self.resume_state.as_ref();
        let replay_on_start = resume.is_some();
        // Retained outputs come from the coordinator's explicit sample-read
        // capability contract. They stay current across compatible source
        // generations, while outputs absent from that contract remain entirely
        // backend-resident and incur no CPU readback.
        let mut sampled_outputs = initial_sampled_outputs(&self.program, &self.retained_outputs)?;
        if let Some(resume) = resume {
            sampled_outputs.extend(
                resume
                    .sampled_outputs
                    .iter()
                    .filter(|(name, _)| self.retained_outputs.contains(name.as_str()))
                    .map(|(name, value)| (name.clone(), value.clone())),
            );
        }
        let state = Arc::new(Mutex::new(ComputeHostState {
            backend: backend_id.to_string(),
            program: Arc::clone(&self.program),
            turns: Ref::new(resume.map_or(0.0, |state| state.turns)),
            dispatch_ms: Ref::new(resume.map_or(0.0, |state| state.dispatch_ms)),
            fault_count: Ref::new(resume.map_or(0.0, |state| state.fault_count)),
            last_fault: Ref::new(resume.map_or_else(String::new, |state| state.last_fault.clone())),
            sampled_outputs,
            // A declared sample read is both a retention contract and a
            // persistent telemetry subscription. Compatible coordinator
            // replacement does not necessarily repeat the non-planning read
            // that dynamically registers a sample, so seed the subscription
            // set from the source contract instead of depending on that
            // lifecycle side effect.
            sample_subscriptions: self.retained_outputs.clone(),
            phase: ComputeHostPhase::Ready {
                last_submitted_turn: resume
                    .and_then(|state| state.last_submitted_turn)
                    .map(ComputeTurnToken),
            },
            session,
        }));
        *self.state_snapshot.state.lock().map_err(|_| {
            compute_host_error(
                "ComputeBackendInitialize",
                "compute host snapshot handle lock is poisoned",
            )
        })? = Some(Arc::downgrade(&state));
        let completion_target = Arc::new(ComputeHostCompletionTarget {
            backend: backend_id.clone(),
            resource: base_uri.clone(),
            state: Arc::downgrade(&state),
            telemetry: Arc::clone(&telemetry),
        });
        state
            .lock()
            .map_err(|_| {
                compute_host_error(
                    "ComputeBackendInitialize",
                    "compute host state lock is poisoned",
                )
            })?
            .session
            .bind_completion_target(completion_target)
            .map_err(|error| {
                compute_host_error(
                    "ComputeBackendInitialize",
                    format!(
                        "region `{}` could not bind completion: {error}",
                        self.region
                    ),
                )
            })?;
        let installation = RuntimeHostInstallation {
            interface: materialize_host_manifest(instance_name, &self.manifest)?,
            resource_providers: vec![Box::new(ComputeResourceProvider {
                instance: instance_name.to_owned(),
                region: self.region.clone(),
                program: Arc::clone(&self.program),
                state: Arc::clone(&state),
                telemetry: Arc::clone(&telemetry),
            })],
            input_drivers: vec![Box::new(ComputeTelemetryDriver {
                base_uri,
                ingress: telemetry,
                live,
                state,
                replay_on_start,
                sample_outputs: self
                    .program
                    .interface()
                    .outputs
                    .iter()
                    .map(|port| port.name.to_string())
                    .collect(),
            })],
        };
        *installed = Some(instance_name.to_owned());
        Ok(installation)
    }
}

#[derive(Clone, Debug)]
pub struct ComputeHostResumeState {
    turns: f64,
    dispatch_ms: f64,
    fault_count: f64,
    last_fault: String,
    sampled_outputs: BTreeMap<String, Value>,
    last_submitted_turn: Option<u128>,
}

#[derive(Clone, Default)]
pub struct ComputeHostStateSnapshotHandle {
    state: Arc<Mutex<Option<Weak<Mutex<ComputeHostState>>>>>,
}

impl std::fmt::Debug for ComputeHostStateSnapshotHandle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ComputeHostStateSnapshotHandle")
            .finish_non_exhaustive()
    }
}

impl ComputeHostStateSnapshotHandle {
    pub fn snapshot(&self) -> MResult<Option<ComputeHostResumeState>> {
        self.snapshot_retaining(None)
    }

    /// Captures host state for a compatible source replacement while copying
    /// only the samples named by the replacement's static retention contract.
    /// Runtime-only reads may populate other samples lazily, but those values
    /// must not silently become persistent migration state.
    pub fn snapshot_retained(
        &self,
        retained_outputs: &BTreeSet<String>,
    ) -> MResult<Option<ComputeHostResumeState>> {
        self.snapshot_retaining(Some(retained_outputs))
    }

    fn snapshot_retaining(
        &self,
        retained_outputs: Option<&BTreeSet<String>>,
    ) -> MResult<Option<ComputeHostResumeState>> {
        let state = self
            .state
            .lock()
            .map_err(|_| {
                compute_host_error(
                    "ComputeHostSnapshot",
                    "compute host snapshot handle lock is poisoned",
                )
            })?
            .as_ref()
            .and_then(Weak::upgrade);
        let Some(state) = state else {
            return Ok(None);
        };
        let state = state.lock().map_err(|_| {
            compute_host_error("ComputeHostSnapshot", "compute host state lock is poisoned")
        })?;
        let last_submitted_turn = match &state.phase {
            ComputeHostPhase::Ready {
                last_submitted_turn,
            } => last_submitted_turn.map(|turn| turn.0),
            ComputeHostPhase::InFlight { turn } => {
                return Err(compute_host_error(
                    "ComputeHostSnapshot",
                    format!("compute turn {} is still in flight", turn.0),
                ));
            }
            ComputeHostPhase::Failed { reason } => {
                return Err(compute_host_error(
                    "ComputeHostSnapshot",
                    format!("compute host is terminal: {reason}"),
                ));
            }
        };
        let sampled_outputs = state
            .sampled_outputs
            .iter()
            .filter(|(name, _)| {
                retained_outputs.is_none_or(|outputs| outputs.contains(name.as_str()))
            })
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect();
        Ok(Some(ComputeHostResumeState {
            turns: *state.turns.borrow(),
            dispatch_ms: *state.dispatch_ms.borrow(),
            fault_count: *state.fault_count.borrow(),
            last_fault: state.last_fault.borrow().clone(),
            sampled_outputs,
            last_submitted_turn,
        }))
    }
}

struct ComputeHostState {
    backend: String,
    program: Arc<ComputeProgram>,
    turns: Ref<f64>,
    dispatch_ms: Ref<f64>,
    fault_count: Ref<f64>,
    last_fault: Ref<String>,
    sampled_outputs: BTreeMap<String, Value>,
    sample_subscriptions: BTreeSet<String>,
    phase: ComputeHostPhase,
    session: Box<dyn ComputeSession>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ComputeHostPhase {
    Ready {
        last_submitted_turn: Option<ComputeTurnToken>,
    },
    InFlight {
        turn: ComputeTurnToken,
    },
    Failed {
        reason: Box<str>,
    },
}

impl ComputeHostState {
    fn require_ready(&self, operation: &'static str) -> MResult<Option<ComputeTurnToken>> {
        match &self.phase {
            ComputeHostPhase::Ready {
                last_submitted_turn,
            } => Ok(*last_submitted_turn),
            ComputeHostPhase::InFlight { turn } => Err(compute_host_error(
                operation,
                format!("compute turn {} is still in flight", turn.0),
            )),
            ComputeHostPhase::Failed { reason } => Err(compute_host_error(
                operation,
                format!("compute host is stopped after an incomplete transaction: {reason}"),
            )),
        }
    }

    fn require_in_flight(
        &mut self,
        operation: &'static str,
        turn: ComputeTurnToken,
    ) -> MResult<()> {
        match &self.phase {
            ComputeHostPhase::InFlight { turn: current } if *current == turn => Ok(()),
            ComputeHostPhase::Failed { reason } => Err(compute_host_error(
                operation,
                format!("compute host is stopped after an incomplete transaction: {reason}"),
            )),
            other => {
                let reason = format!(
                    "completion for compute turn {} does not match host phase {other:?}",
                    turn.0
                );
                self.fail(reason.clone());
                Err(compute_host_error(operation, reason))
            }
        }
    }

    fn ready_after(&mut self, turn: ComputeTurnToken) {
        self.phase = ComputeHostPhase::Ready {
            last_submitted_turn: Some(turn),
        };
    }

    fn fail(&mut self, reason: impl Into<String>) -> Box<str> {
        if let ComputeHostPhase::Failed { reason } = &self.phase {
            return reason.clone();
        }
        let reason = reason.into().into_boxed_str();
        self.phase = ComputeHostPhase::Failed {
            reason: reason.clone(),
        };
        reason
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ComputeTurnToken(u128);

struct ComputeHostCompletionTarget {
    backend: BackendId,
    resource: String,
    state: Weak<Mutex<ComputeHostState>>,
    telemetry: Arc<Mutex<Option<RuntimeIngress>>>,
}

impl ComputeCompletionTarget for ComputeHostCompletionTarget {
    fn complete(&self, outcome: ComputeCompletionOutcome) -> Result<(), ComputeExecutionError> {
        let state = self.state.upgrade().ok_or_else(|| {
            completion_error(&self.backend, "resident compute host has been retired")
        })?;
        let mut state = state
            .lock()
            .map_err(|_| completion_error(&self.backend, "compute host state lock is poisoned"))?;
        let (attempted_turn, report, sampled, transport_failure) = match outcome {
            ComputeCompletionOutcome::Completed {
                attempted_turn,
                report,
                snapshot,
            } => {
                let turn = ComputeTurnToken(attempted_turn);
                state
                    .require_in_flight("ComputeHostCompletion", turn)
                    .map_err(|error| completion_error(&self.backend, format!("{error:?}")))?;
                let sampled = if report.completed_turns > 0 {
                    match materialize_sampled_outputs(&state, &snapshot) {
                        Ok(sampled) => sampled,
                        Err(error) => {
                            state.fail(format!("sample publication failed: {error:?}"));
                            return Err(completion_error(&self.backend, format!("{error:?}")));
                        }
                    }
                } else {
                    BTreeMap::new()
                };
                (turn, report, sampled, None)
            }
            ComputeCompletionOutcome::IntegrityRejected {
                attempted_turn,
                report,
            } => {
                let turn = ComputeTurnToken(attempted_turn);
                state
                    .require_in_flight("ComputeHostCompletion", turn)
                    .map_err(|error| completion_error(&self.backend, format!("{error:?}")))?;
                (turn, report, BTreeMap::new(), None)
            }
            ComputeCompletionOutcome::TransportFailed {
                attempted_turn,
                reason,
            } => {
                let turn = ComputeTurnToken(attempted_turn);
                state
                    .require_in_flight("ComputeHostCompletion", turn)
                    .map_err(|error| completion_error(&self.backend, format!("{error:?}")))?;
                (
                    turn,
                    ComputeDispatchReport {
                        disposition: ComputeDispatchDisposition::Rejected,
                        completed_turns: 0,
                        fault_count: (*state.fault_count.borrow() as u64).saturating_add(1),
                        last_fault: Some(mech_compute::ComputeFaultEvidence {
                            attempted_turn,
                            constraint: "backend-transport".into(),
                            detail: reason.clone(),
                        }),
                        ..Default::default()
                    },
                    BTreeMap::new(),
                    Some(reason.to_string()),
                )
            }
        };
        if let Some(failure) = transport_failure.as_ref() {
            // The backend transport is the first terminal failure. Preserve
            // it even if best-effort fault telemetry subsequently encounters
            // another error.
            state.fail(failure.clone());
        }
        let mut candidate_outputs = state.sampled_outputs.clone();
        candidate_outputs.extend(sampled);
        let candidate_turns = *state.turns.borrow() + f64::from(report.completed_turns);
        let candidate_dispatch_ms = report.dispatch_milliseconds;
        let candidate_fault_count = report.fault_count as f64;
        let candidate_last_fault = report.last_fault.as_ref().map_or_else(
            || state.last_fault.borrow().clone(),
            |fault| {
                format!(
                    "turn {}: {}: {}",
                    fault.attempted_turn, fault.constraint, fault.detail
                )
            },
        );
        let updates = telemetry_updates_from_values(
            &self.resource,
            ComputeTelemetryValues {
                backend: &state.backend,
                turns: candidate_turns,
                dispatch_ms: candidate_dispatch_ms,
                fault_count: candidate_fault_count,
                last_fault: &candidate_last_fault,
            },
            &state.sample_subscriptions,
            &candidate_outputs,
        )
        .map_err(|error| {
            state.fail(
                transport_failure
                    .clone()
                    .unwrap_or_else(|| format!("telemetry encoding failed: {error:?}")),
            );
            completion_error(&self.backend, format!("{error:?}"))
        })?;
        let packet = RuntimeHostInput::new(updates).map_err(|error| {
            state.fail(
                transport_failure
                    .clone()
                    .unwrap_or_else(|| format!("telemetry packet creation failed: {error:?}")),
            );
            completion_error(&self.backend, format!("{error:?}"))
        })?;
        let telemetry = self.telemetry.lock().map_err(|_| {
            state.fail("telemetry ingress lock is poisoned");
            completion_error(&self.backend, "telemetry ingress lock is poisoned")
        })?;
        if let Some(ingress) = telemetry.as_ref() {
            if let Err(error) = ingress.submit_latest(packet) {
                state.fail(format!("telemetry publication failed: {error:?}"));
                return Err(completion_error(&self.backend, format!("{error:?}")));
            }
        }
        state.sampled_outputs = candidate_outputs;
        *state.turns.borrow_mut() = candidate_turns;
        *state.dispatch_ms.borrow_mut() = candidate_dispatch_ms;
        *state.fault_count.borrow_mut() = candidate_fault_count;
        *state.last_fault.borrow_mut() = candidate_last_fault;
        if let Some(failure) = transport_failure {
            state.fail(failure);
        } else {
            state.ready_after(attempted_turn);
        }
        Ok(())
    }
}

fn completion_error(backend: &BackendId, detail: impl Into<String>) -> ComputeExecutionError {
    ComputeExecutionError {
        backend: backend.clone(),
        operation: "complete asynchronous dispatch",
        detail: detail.into().into_boxed_str(),
        state_advanced: true,
    }
}

impl std::fmt::Debug for ComputeHostState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ComputeHostState")
            .field("backend", &self.backend)
            .field("program", &self.program)
            .field("turns", &self.turns)
            .field("dispatch_ms", &self.dispatch_ms)
            .field("fault_count", &self.fault_count)
            .field("last_fault", &self.last_fault)
            .field("sample_subscriptions", &self.sample_subscriptions)
            .field("phase", &self.phase)
            .field("session", &"<dyn ComputeSession>")
            .finish()
    }
}

#[derive(Debug)]
struct ComputeResourceProvider {
    instance: String,
    region: Box<str>,
    program: Arc<ComputeProgram>,
    state: Arc<Mutex<ComputeHostState>>,
    telemetry: Arc<Mutex<Option<RuntimeIngress>>>,
}

impl ComputeResourceProvider {
    fn base_uri(&self) -> String {
        format!("compute://{}/kernel", self.instance)
    }

    fn declared_input(&self, path: &str) -> Option<&ComputePort> {
        let name = path.strip_prefix("input/")?;
        (!name.is_empty())
            .then(|| self.program.interface().input_named(name))
            .flatten()
    }

    fn declared_sample_output(&self, path: &str) -> Option<&ComputePort> {
        let name = path.strip_prefix("sample/")?;
        (!name.is_empty())
            .then(|| {
                self.program
                    .interface()
                    .outputs
                    .iter()
                    .find(|port| port.name.as_ref() == name)
            })
            .flatten()
    }

    fn telemetry_value(&self, path: &str, planning: bool) -> MResult<Value> {
        let mut state = self.state.lock().map_err(|_| {
            compute_host_error("ComputeHostRead", "compute host state lock is poisoned")
        })?;
        state.require_ready("ComputeHostRead")?;
        if let Some(port) = self.declared_sample_output(path) {
            if planning {
                return zero_sample_value(port);
            }
            state.sample_subscriptions.insert(port.name.to_string());
            if !state.sampled_outputs.contains_key(port.name.as_ref()) {
                let initial = initial_sampled_output(&state.program, port)?;
                state.sampled_outputs.insert(port.name.to_string(), initial);
            }
            let value = state
                .sampled_outputs
                .get(port.name.as_ref())
                .ok_or_else(|| {
                    compute_host_error(
                        "ComputeHostRead",
                        format!("sampled output `{}` was not initialized", port.name),
                    )
                })?
                .clone();
            return Ok(value);
        }
        if planning {
            return match path {
                "backend" | "last-fault" => {
                    RuntimeHostInputValue::String(String::new()).into_value()
                }
                "turns" | "dispatch-ms" | "fault-count" => {
                    RuntimeHostInputValue::F64(0.0).into_value()
                }
                other => Err(compute_host_error(
                    "ComputeHostRead",
                    format!("unknown compute telemetry path `{other}`"),
                )),
            };
        }
        match path {
            "backend" => RuntimeHostInputValue::String(state.backend.clone()).into_value(),
            "turns" => RuntimeHostInputValue::F64(*state.turns.borrow()).into_value(),
            "dispatch-ms" => RuntimeHostInputValue::F64(*state.dispatch_ms.borrow()).into_value(),
            "fault-count" => RuntimeHostInputValue::F64(*state.fault_count.borrow()).into_value(),
            "last-fault" => {
                RuntimeHostInputValue::String(state.last_fault.borrow().clone()).into_value()
            }
            other => Err(compute_host_error(
                "ComputeHostRead",
                format!("unknown compute telemetry path `{other}`"),
            )),
        }
    }
}

impl RuntimeResourceProvider for ComputeResourceProvider {
    fn scheme(&self) -> &str {
        "compute"
    }

    fn base_uris(&self) -> Vec<String> {
        vec![self.base_uri()]
    }

    fn semantic_read_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(mech_runtime::resource_observation_contract())
    }

    fn semantic_write_contract(
        &self,
        intent: RuntimeResourceWriteIntent,
    ) -> Option<&'static OperationContractDeclaration> {
        (intent == RuntimeResourceWriteIntent::Send)
            .then_some(mech_runtime::compute_effect_contract())
    }

    fn plan_read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        self.validate_base_uri(&request.base_uri, "ComputeHostRead")?;
        self.telemetry_value(&request.path, true)
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        self.validate_base_uri(&request.base_uri, "ComputeHostRead")?;
        self.telemetry_value(&request.path, false)
    }

    fn preflight_write(&self, request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
        self.validate_base_uri(&request.base_uri, "ComputeHostWrite")?;
        self.state
            .lock()
            .map_err(|_| {
                compute_host_error("ComputeHostWrite", "compute host state lock is poisoned")
            })?
            .require_ready("ComputeHostWrite")?;
        if request.intent != RuntimeResourceWriteIntent::Send {
            return Err(compute_host_error(
                "ComputeHostWrite",
                "compute dispatch is an effect; use <-",
            ));
        }
        if request.path != "turn" && self.declared_input(&request.path).is_none() {
            return Err(compute_host_error(
                "ComputeHostWrite",
                format!("unknown compute input path `{}`", request.path),
            ));
        }
        Ok(())
    }

    fn plan_write(&self, request: RuntimeResourceWriteRequest) -> MResult<()> {
        self.preflight_write(RuntimeResourceWritePreflightRequest {
            base_uri: request.base_uri,
            path: request.path.clone(),
            context_name: request.context_name,
            operation: request.operation,
            intent: request.intent,
        })?;
        if let Some(port) = self.declared_input(&request.path) {
            compute_input_update(&self.program, port, &request.value)?;
        } else {
            compute_turn_token(&request.value)?;
        }
        Ok(())
    }

    fn prepare_write(
        &self,
        request: RuntimeResourceWriteRequest,
    ) -> MResult<PreparedRuntimeEffect> {
        self.preflight_write(RuntimeResourceWritePreflightRequest {
            base_uri: request.base_uri.clone(),
            path: request.path.clone(),
            context_name: request.context_name,
            operation: request.operation,
            intent: request.intent,
        })?;
        if let Some(port) = self.declared_input(&request.path) {
            let update = compute_input_update(&self.program, port, &request.value)?;
            return Ok(PreparedRuntimeEffect::AfterCommit(Box::new(
                ComputeInputEffect {
                    resource: request.base_uri,
                    region: self.region.clone(),
                    update,
                    state: Arc::clone(&self.state),
                },
            )));
        }
        let turn = compute_turn_token(&request.value)?;
        Ok(PreparedRuntimeEffect::AfterCommit(Box::new(
            ComputeDispatchEffect {
                resource: request.base_uri,
                region: self.region.clone(),
                turn,
                state: Arc::clone(&self.state),
                telemetry: Arc::clone(&self.telemetry),
            },
        )))
    }
}

impl ComputeResourceProvider {
    fn validate_base_uri(&self, actual: &str, operation: &'static str) -> MResult<()> {
        if actual == self.base_uri() {
            Ok(())
        } else {
            Err(compute_host_error(
                operation,
                format!("unknown compute resource `{actual}`"),
            ))
        }
    }
}

#[derive(Debug)]
struct ComputeInputEffect {
    resource: String,
    region: Box<str>,
    update: ComputeInputUpdate,
    state: Arc<Mutex<ComputeHostState>>,
}

impl RuntimeAfterCommitEffect for ComputeInputEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::ResourceProvider {
                scheme: "compute".to_owned(),
            },
            format!("input:{}:{}", self.region, self.update.port.get()),
        )
        .with_resource(self.resource.clone())
        .with_cost(RuntimeEffectCost {
            bytes: compute_value_elements(&self.update.value)
                .saturating_mul(std::mem::size_of::<f32>() as u64),
            items: compute_value_elements(&self.update.value),
        })
    }

    fn deliver(&mut self) -> MResult<()> {
        let mut state = self.state.lock().map_err(|_| {
            compute_host_error("ComputeHostWrite", "compute host state lock is poisoned")
        })?;
        state.require_ready("ComputeHostWrite")?;
        if let Err(error) = state
            .session
            .update_inputs(std::slice::from_ref(&self.update))
        {
            if error.state_advanced {
                state.fail(error.to_string());
            }
            return Err(compute_host_error("ComputeHostWrite", error.to_string()));
        }
        Ok(())
    }
}

#[derive(Debug)]
struct ComputeDispatchEffect {
    resource: String,
    region: Box<str>,
    turn: ComputeTurnToken,
    state: Arc<Mutex<ComputeHostState>>,
    telemetry: Arc<Mutex<Option<RuntimeIngress>>>,
}

impl RuntimeAfterCommitEffect for ComputeDispatchEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::ResourceProvider {
                scheme: "compute".to_owned(),
            },
            format!("dispatch:{}", self.region),
        )
        .with_resource(self.resource.clone())
        .with_cost(RuntimeEffectCost { bytes: 0, items: 1 })
    }

    fn deliver(&mut self) -> MResult<()> {
        let turn = self.turn;
        let mut state = self.state.lock().map_err(|_| {
            compute_host_error("ComputeHostDispatch", "compute host state lock is poisoned")
        })?;
        let previous_turn = state.require_ready("ComputeHostDispatch")?;
        if previous_turn.is_some_and(|submitted| turn <= submitted) {
            return Ok(());
        }
        let outputs = state
            .program
            .interface()
            .outputs
            .iter()
            .filter(|port| state.sample_subscriptions.contains(port.name.as_ref()))
            .map(|port| port.id)
            .collect();
        let request = ComputeDispatchRequest {
            turns: NonZeroU32::MIN,
            outputs,
            logical_turn: turn.0,
        };
        state.phase = ComputeHostPhase::InFlight { turn };
        let report = match state.session.dispatch(&request) {
            Ok(report) => report,
            Err(error) => {
                if error.state_advanced {
                    state.fail(error.to_string());
                } else {
                    state.phase = ComputeHostPhase::Ready {
                        last_submitted_turn: previous_turn,
                    };
                }
                return Err(compute_host_error("ComputeHostDispatch", error.to_string()));
            }
        };
        // Asynchronous browser compute has accepted this exact logical turn
        // and will complete it through ComputeHostCompletionTarget. Do not
        // publish a speculative telemetry packet while the host is InFlight.
        if report.disposition == ComputeDispatchDisposition::Submitted {
            return Ok(());
        }
        let mut sampled = None;
        if report.completed_turns > 0 && !state.sample_subscriptions.is_empty() {
            let selected = state
                .program
                .interface()
                .outputs
                .iter()
                .filter(|port| state.sample_subscriptions.contains(port.name.as_ref()))
                .map(|port| port.id)
                .collect();
            let snapshot = match state
                .session
                .read_outputs(&ComputeOutputSelection::Samples {
                    ports: selected,
                    instance: 0,
                }) {
                Ok(snapshot) => snapshot,
                Err(error) => {
                    state.fail(error.to_string());
                    return Err(compute_host_error("ComputeHostRead", error.to_string()));
                }
            };
            sampled = Some(match materialize_sampled_outputs(&state, &snapshot) {
                Ok(sampled) => sampled,
                Err(error) => {
                    state.fail(format!("{error:?}"));
                    return Err(error);
                }
            });
        }
        let mut candidate_outputs = state.sampled_outputs.clone();
        if let Some(sampled) = sampled {
            candidate_outputs.extend(sampled);
        }
        let candidate_turns = *state.turns.borrow() + f64::from(report.completed_turns);
        let candidate_dispatch_ms = report.dispatch_milliseconds;
        let candidate_fault_count = report.fault_count as f64;
        let candidate_last_fault = report.last_fault.as_ref().map_or_else(
            || state.last_fault.borrow().clone(),
            |fault| {
                format!(
                    "turn {}: {}: {}",
                    fault.attempted_turn, fault.constraint, fault.detail
                )
            },
        );
        let updates = telemetry_updates_from_values(
            &self.resource,
            ComputeTelemetryValues {
                backend: &state.backend,
                turns: candidate_turns,
                dispatch_ms: candidate_dispatch_ms,
                fault_count: candidate_fault_count,
                last_fault: &candidate_last_fault,
            },
            &state.sample_subscriptions,
            &candidate_outputs,
        )
        .map_err(|error| {
            state.fail(format!("telemetry encoding failed: {error:?}"));
            error
        })?;
        let packet = RuntimeHostInput::new(updates).map_err(|error| {
            state.fail(format!("telemetry packet creation failed: {error:?}"));
            error
        })?;
        let telemetry = self.telemetry.lock().map_err(|_| {
            state.fail("compute telemetry ingress lock is poisoned");
            compute_host_error(
                "ComputeHostDispatch",
                "compute telemetry ingress lock is poisoned",
            )
        })?;
        if let Some(ingress) = telemetry.as_ref() {
            if let Err(error) = ingress.submit(packet) {
                state.fail(format!("telemetry publication failed: {error:?}"));
                return Err(error);
            }
        }
        state.sampled_outputs = candidate_outputs;
        *state.turns.borrow_mut() = candidate_turns;
        *state.dispatch_ms.borrow_mut() = candidate_dispatch_ms;
        *state.fault_count.borrow_mut() = candidate_fault_count;
        *state.last_fault.borrow_mut() = candidate_last_fault;
        state.ready_after(turn);
        Ok(())
    }
}

fn canonical_turn_token(value: &RuntimeHostInputValue) -> MResult<ComputeTurnToken> {
    // `u128::MAX as f64` rounds to 2^128, the exclusive upper bound. Keeping
    // the comparison in floating point rejects that boundary and every larger
    // finite value before Rust's saturating float-to-integer cast can alias
    // distinct source tokens to `u128::MAX`.
    const U128_EXCLUSIVE_UPPER_BOUND: f64 = 340282366920938463463374607431768211456.0_f64;
    let invalid = || {
        compute_host_error(
            "ComputeHostDispatch",
            "compute turn tokens must be non-negative, finite integers representable as u128",
        )
    };
    let token = match value {
        RuntimeHostInputValue::U8(value) => u128::from(*value),
        RuntimeHostInputValue::U16(value) => u128::from(*value),
        RuntimeHostInputValue::U32(value) => u128::from(*value),
        RuntimeHostInputValue::U64(value) => u128::from(*value),
        RuntimeHostInputValue::U128(value) => *value,
        RuntimeHostInputValue::Index(value) => *value as u128,
        RuntimeHostInputValue::I8(value) if *value >= 0 => *value as u128,
        RuntimeHostInputValue::I16(value) if *value >= 0 => *value as u128,
        RuntimeHostInputValue::I32(value) if *value >= 0 => *value as u128,
        RuntimeHostInputValue::I64(value) if *value >= 0 => *value as u128,
        RuntimeHostInputValue::I128(value) if *value >= 0 => *value as u128,
        RuntimeHostInputValue::F32(value)
            if value.is_finite()
                && *value >= 0.0
                && value.fract() == 0.0
                && f64::from(*value) < U128_EXCLUSIVE_UPPER_BOUND =>
        {
            *value as u128
        }
        RuntimeHostInputValue::F64(value)
            if value.is_finite()
                && *value >= 0.0
                && value.fract() == 0.0
                && *value < U128_EXCLUSIVE_UPPER_BOUND =>
        {
            *value as u128
        }
        _ => return Err(invalid()),
    };
    Ok(ComputeTurnToken(token))
}

#[derive(Clone, Copy)]
struct ComputeTelemetryValues<'a> {
    backend: &'a str,
    turns: f64,
    dispatch_ms: f64,
    fault_count: f64,
    last_fault: &'a str,
}

fn telemetry_updates_from_values(
    resource: &str,
    values: ComputeTelemetryValues<'_>,
    sample_subscriptions: &BTreeSet<String>,
    sampled_outputs: &BTreeMap<String, Value>,
) -> MResult<Vec<RuntimeHostInputUpdate>> {
    let ComputeTelemetryValues {
        backend,
        turns,
        dispatch_ms,
        fault_count,
        last_fault,
    } = values;
    let mut updates = vec![
        telemetry_update(
            resource,
            "backend",
            RuntimeHostInputValue::String(backend.to_owned()),
        )?,
        telemetry_update(resource, "turns", RuntimeHostInputValue::F64(turns))?,
        telemetry_update(
            resource,
            "dispatch-ms",
            RuntimeHostInputValue::F64(dispatch_ms),
        )?,
        telemetry_update(
            resource,
            "fault-count",
            RuntimeHostInputValue::F64(fault_count),
        )?,
        telemetry_update(
            resource,
            "last-fault",
            RuntimeHostInputValue::String(last_fault.to_owned()),
        )?,
    ];
    for name in sample_subscriptions {
        let value = sampled_outputs.get(name).ok_or_else(|| {
            compute_host_error(
                "ComputeHostRead",
                format!("level output `{name}` has no retained sample"),
            )
        })?;
        updates.push(RuntimeHostInputUpdate {
            source: RuntimeHostInputSource::new(resource, format!("sample/{name}"))?,
            value: RuntimeHostInputValue::from_numeric_value(value)?,
        });
    }
    Ok(updates)
}

fn telemetry_update(
    resource: &str,
    path: &str,
    value: RuntimeHostInputValue,
) -> MResult<RuntimeHostInputUpdate> {
    Ok(RuntimeHostInputUpdate {
        source: RuntimeHostInputSource::new(resource, path)?,
        value,
    })
}

#[derive(Debug)]
struct ComputeTelemetryDriver {
    base_uri: String,
    ingress: Arc<Mutex<Option<RuntimeIngress>>>,
    live: Arc<AtomicBool>,
    state: Arc<Mutex<ComputeHostState>>,
    replay_on_start: bool,
    sample_outputs: BTreeSet<String>,
}

impl RuntimeHostInputDriver for ComputeTelemetryDriver {
    fn drives(&self, source: &RuntimeHostInputSource) -> bool {
        source.base_uri() == self.base_uri
            && (matches!(
                source.path(),
                "backend" | "turns" | "dispatch-ms" | "fault-count" | "last-fault"
            ) || source
                .path()
                .strip_prefix("sample/")
                .is_some_and(|name| self.sample_outputs.contains(name)))
    }

    fn attach(&mut self, ingress: RuntimeIngress) -> MResult<()> {
        let mut attached = self.ingress.lock().map_err(|_| {
            compute_host_error(
                "ComputeTelemetryDriver",
                "compute telemetry ingress lock is poisoned",
            )
        })?;
        if attached.is_some() {
            return Err(compute_host_error(
                "ComputeTelemetryDriver",
                "compute telemetry driver is already attached",
            ));
        }
        *attached = Some(ingress);
        Ok(())
    }

    fn start(&mut self) -> MResult<()> {
        let ingress = self
            .ingress
            .lock()
            .map_err(|_| {
                compute_host_error(
                    "ComputeTelemetryDriver",
                    "compute telemetry ingress lock is poisoned",
                )
            })?
            .clone()
            .ok_or_else(|| {
                compute_host_error(
                    "ComputeTelemetryDriver",
                    "compute telemetry driver must be attached before start",
                )
            })?;
        if self.replay_on_start {
            let packet = {
                let state = self.state.lock().map_err(|_| {
                    compute_host_error(
                        "ComputeTelemetryDriver",
                        "compute host state lock is poisoned while starting telemetry",
                    )
                })?;
                state.require_ready("ComputeTelemetryDriver")?;
                RuntimeHostInput::new(telemetry_updates_from_values(
                    &self.base_uri,
                    ComputeTelemetryValues {
                        backend: &state.backend,
                        turns: *state.turns.borrow(),
                        dispatch_ms: *state.dispatch_ms.borrow(),
                        fault_count: *state.fault_count.borrow(),
                        last_fault: &state.last_fault.borrow(),
                    },
                    &state.sample_subscriptions,
                    &state.sampled_outputs,
                )?)?
            };
            // A resumed level-valued host may already be ahead of the last
            // snapshot accepted by the replacement runtime. Publish that state
            // as an explicit ingress packet instead of forcing the coordinator
            // to synthesize absent fields from a future provider read.
            ingress.submit_latest(packet)?;
        }
        self.live.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn stop(&mut self) -> MResult<()> {
        self.live.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn is_live(&self) -> bool {
        self.live.load(Ordering::SeqCst)
    }
}

pub fn compute_host_manifest() -> HostManifestConfig {
    HostManifestConfig {
        provider: "compute".to_owned(),
        contexts: vec![mech_runtime::HostContextManifest {
            name: "kernel".to_owned(),
            base_uri_template: "compute://{instance}/kernel".to_owned(),
            operations: vec!["read".to_owned(), "write".to_owned()],
        }],
    }
}

pub fn validate_compute_host_settings(settings: &ConfigValue) -> MResult<()> {
    configured_compute_settings(settings).map(|_| ())
}

/// Validates compute-host settings and returns the configured backend request.
/// Product integrations use this before lowering so source planning and host
/// installation observe the same configuration contract.
pub fn configured_compute_backend_request(settings: &ConfigValue) -> MResult<BackendRequest> {
    configured_compute_settings(settings).map(|settings| settings.backend)
}

struct ConfiguredComputeSettings {
    region: String,
    backend: BackendRequest,
}

fn configured_compute_settings(settings: &ConfigValue) -> MResult<ConfiguredComputeSettings> {
    let ConfigValue::Map(map) = settings else {
        return Err(compute_host_error(
            "ComputeHostConfiguration",
            "compute host settings must be a map",
        ));
    };
    for key in map.keys() {
        if !matches!(key.as_str(), "region" | "backend") {
            return Err(compute_host_error(
                "ComputeHostConfiguration",
                format!("unknown compute host setting `{key}`"),
            ));
        }
    }
    let Some(ConfigValue::String(region)) = map.get("region") else {
        return Err(compute_host_error(
            "ComputeHostConfiguration",
            "compute host setting `region` must be a string",
        ));
    };
    if region.trim().is_empty() {
        return Err(compute_host_error(
            "ComputeHostConfiguration",
            "compute host setting `region` must be nonempty",
        ));
    }
    let backend = match map.get("backend") {
        None => BackendRequest::Auto,
        Some(ConfigValue::String(value)) => BackendRequest::parse(value).map_err(|error| {
            compute_host_error(
                "ComputeHostConfiguration",
                format!("invalid compute backend `{value}`: {error}"),
            )
        })?,
        Some(_) => {
            return Err(compute_host_error(
                "ComputeHostConfiguration",
                "compute host setting `backend` must be a string",
            ));
        }
    };
    Ok(ConfiguredComputeSettings {
        region: region.clone(),
        backend,
    })
}

fn compute_input_update(
    program: &ComputeProgram,
    port: &ComputePort,
    value: &Value,
) -> MResult<ComputeInputUpdate> {
    let detached = RuntimeHostInputValue::from_numeric_value(value)?;
    let value = match detached {
        RuntimeHostInputValue::F32(value) => ComputeValue::ScalarF32(value),
        RuntimeHostInputValue::F64(value) => {
            ComputeValue::ScalarF32(narrow_f64_input(port.name.as_ref(), value)?)
        }
        RuntimeHostInputValue::F32Matrix {
            rows,
            columns,
            values,
        } => ComputeValue::TensorF32 {
            dimensions: vec![rows as u64, columns as u64].into_boxed_slice(),
            layout: TensorLayout::RowMajor,
            values: Arc::from(values),
        },
        RuntimeHostInputValue::F64Matrix {
            rows,
            columns,
            values,
        } => ComputeValue::TensorF32 {
            dimensions: vec![rows as u64, columns as u64].into_boxed_slice(),
            layout: TensorLayout::RowMajor,
            values: values
                .into_iter()
                .map(|value| narrow_f64_input(port.name.as_ref(), value))
                .collect::<MResult<Vec<_>>>()?
                .into(),
        },
        other => {
            return Err(compute_host_error(
                "ComputeHostWrite",
                format!(
                    "compute input `{}` requires fixed-shape f32 data, found `{:?}`",
                    port.name, other
                ),
            ));
        }
    };
    let update = program
        .normalize_input_update(ComputeInputUpdate {
            port: port.id,
            value,
        })
        .map_err(|error| {
            compute_host_error(
                "ComputeHostWrite",
                format!("compute input `{}` is invalid: {error}", port.name),
            )
        })?;
    Ok(update)
}

fn compute_turn_token(value: &Value) -> MResult<ComputeTurnToken> {
    let token = RuntimeHostInputValue::from_numeric_value(value)?;
    canonical_turn_token(&token)
}

fn narrow_f64_input(port: &str, value: f64) -> MResult<f32> {
    mech_compute::narrow_compute_f64(value).map_err(|value| {
        compute_host_error(
            "ComputeHostWrite",
            format!("compute input `{port}` contains f64 value {value} outside the f32 range"),
        )
    })
}

fn compute_value_elements(value: &ComputeValue) -> u64 {
    match value {
        ComputeValue::ScalarF32(_) => 1,
        ComputeValue::TensorF32 { values, .. } => u64::try_from(values.len()).unwrap_or(u64::MAX),
    }
}

fn initial_sampled_outputs(
    program: &ComputeProgram,
    retained_outputs: &BTreeSet<String>,
) -> MResult<BTreeMap<String, Value>> {
    program
        .interface()
        .outputs
        .iter()
        .filter(|port| retained_outputs.contains(port.name.as_ref()))
        .map(|port| {
            Ok((
                port.name.to_string(),
                initial_sampled_output(program, port)?,
            ))
        })
        .collect()
}

fn initial_sampled_output(program: &ComputeProgram, port: &ComputePort) -> MResult<Value> {
    let value = program
        .fixed_shape_storage()
        .and_then(|storage| storage.states.iter().find(|state| state.slot == port.slot))
        .map(|state| ComputeValue::TensorF32 {
            dimensions: port.dimensions.clone(),
            layout: TensorLayout::ColumnMajor,
            values: Arc::clone(&state.initializer),
        })
        .or_else(|| {
            program.elementwise_storage().and_then(|storage| {
                storage
                    .states
                    .iter()
                    .find(|state| state.slot == port.slot)
                    .map(|state| ComputeValue::TensorF32 {
                        dimensions: port.dimensions.clone(),
                        layout: TensorLayout::RowMajor,
                        values: Arc::clone(&state.initializer),
                    })
            })
        });
    match value {
        Some(ComputeValue::TensorF32 { values, .. })
            if port.dimensions.is_empty() && values.len() == 1 =>
        {
            sampled_output_value(port, ComputeValue::ScalarF32(values[0]))
        }
        Some(value) => sampled_output_value(port, value),
        None => zero_sample_value(port),
    }
}

fn zero_sample_value(port: &ComputePort) -> MResult<Value> {
    let elements = port.elements().map_err(|error| {
        compute_host_error(
            "ComputeHostConfiguration",
            format!(
                "compute output `{}` has an invalid shape: {error}",
                port.name
            ),
        )
    })?;
    sampled_output_value(
        port,
        if port.dimensions.is_empty() {
            ComputeValue::ScalarF32(0.0)
        } else {
            ComputeValue::TensorF32 {
                dimensions: port.dimensions.clone(),
                layout: TensorLayout::RowMajor,
                values: vec![0.0; elements].into(),
            }
        },
    )
}

fn materialize_sampled_outputs(
    state: &ComputeHostState,
    snapshot: &ComputeOutputSnapshot,
) -> MResult<BTreeMap<String, Value>> {
    let ports = state
        .program
        .interface()
        .outputs
        .iter()
        .filter(|port| state.sample_subscriptions.contains(port.name.as_ref()))
        .cloned()
        .collect::<Vec<_>>();
    let mut sampled = BTreeMap::new();
    for port in ports {
        let value = snapshot.values.get(&port.id).ok_or_else(|| {
            compute_host_error(
                "ComputeHostRead",
                format!("backend omitted sampled output `{}`", port.name),
            )
        })?;
        sampled.insert(
            port.name.to_string(),
            sampled_output_value(&port, value.clone())?,
        );
    }
    Ok(sampled)
}

fn sampled_output_value(port: &ComputePort, value: ComputeValue) -> MResult<Value> {
    let inner_elements = port.elements().map_err(|error| {
        compute_host_error(
            "ComputeHostRead",
            format!(
                "compute output `{}` has an invalid shape: {error}",
                port.name
            ),
        )
    })?;
    let row_major = match value {
        ComputeValue::ScalarF32(value) if port.dimensions.is_empty() => vec![value],
        ComputeValue::TensorF32 {
            dimensions,
            layout: TensorLayout::RowMajor,
            values,
        } => {
            let inner_shape = port.dimensions.as_ref();
            let expected_elements = if dimensions.as_ref() == inner_shape {
                Some(inner_elements)
            } else if dimensions.len() == inner_shape.len() + 1 && dimensions[1..] == *inner_shape {
                usize::try_from(dimensions[0])
                    .ok()
                    .and_then(|instances| instances.checked_mul(inner_elements))
            } else {
                None
            };
            if expected_elements != Some(values.len()) || values.len() < inner_elements {
                return Err(compute_host_error(
                    "ComputeHostRead",
                    format!(
                        "sampled output `{}` returned shape {:?} with {} elements; expected one or more {:?} values",
                        port.name,
                        dimensions,
                        values.len(),
                        inner_shape,
                    ),
                ));
            }
            values[..inner_elements].to_vec()
        }
        ComputeValue::TensorF32 {
            dimensions,
            layout: TensorLayout::ColumnMajor,
            values,
        } if dimensions.as_ref() == port.dimensions.as_ref() => {
            column_major_sample_to_row_major(&port.dimensions, values.as_ref())?
        }
        other => {
            return Err(compute_host_error(
                "ComputeHostRead",
                format!(
                    "sampled output `{}` does not match its inner {:?} contract: {other:?}",
                    port.name, port.dimensions,
                ),
            ));
        }
    };
    let row_major = row_major.into_iter().map(f64::from).collect::<Vec<_>>();
    let detached = match port.dimensions.as_ref() {
        [] => RuntimeHostInputValue::F64(row_major[0]),
        [columns] => RuntimeHostInputValue::F64Matrix {
            rows: 1,
            columns: *columns as usize,
            values: row_major,
        },
        [rows, columns] => RuntimeHostInputValue::F64Matrix {
            rows: *rows as usize,
            columns: *columns as usize,
            values: row_major,
        },
        dimensions => {
            return Err(compute_host_error(
                "ComputeHostRead",
                format!(
                    "sampled output `{}` has unsupported rank {}",
                    port.name,
                    dimensions.len(),
                ),
            ));
        }
    };
    detached.into_value()
}

fn column_major_sample_to_row_major(dimensions: &[u64], values: &[f32]) -> MResult<Vec<f32>> {
    let rows = dimensions.first().copied().unwrap_or(1) as usize;
    let columns = dimensions.get(1).copied().unwrap_or(1) as usize;
    if values.len() != rows.saturating_mul(columns) {
        return Err(compute_host_error(
            "ComputeHostRead",
            "sampled column-major output has the wrong element count",
        ));
    }
    let mut row_major = vec![0.0; values.len()];
    for row in 0..rows {
        for column in 0..columns {
            row_major[row * columns + column] = values[row + column * rows];
        }
    }
    Ok(row_major)
}

#[derive(Clone, Debug)]
struct ComputeHostError {
    name: &'static str,
    message: String,
}

impl MechErrorKind for ComputeHostError {
    fn name(&self) -> &str {
        self.name
    }

    fn message(&self) -> String {
        self.message.clone()
    }
}

fn compute_host_error(name: &'static str, message: impl Into<String>) -> MechError {
    MechError::new(
        ComputeHostError {
            name,
            message: message.into(),
        },
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use mech_compute::{
        BackendClass, CPU_SCALAR_BACKEND, ComputeBackendCapabilities, ComputeBackendDescriptor,
        ComputeBackendError, ComputeBackendFactory, ComputeBackendRejection, ComputeExecutable,
        ComputeFaultEvidence, ComputeKernel, ComputePhysicalPlan, ComputeRegionInterface,
        ElementwiseIr,
    };
    use mech_core::{CellSlotId, SchemaId};
    use mech_runtime::{
        HostInstanceConfig, RunResourceGrantConfig, RuntimeBuilder, RuntimeCapabilityOperation,
        RuntimeResourceReadRequest,
    };

    #[derive(Clone, Debug, PartialEq)]
    enum FakeCall {
        Update(ComputeInputUpdate),
        Dispatch(u32),
        Read(ComputeOutputSelection),
    }

    struct FakeBackend {
        descriptor: ComputeBackendDescriptor,
        calls: Arc<Mutex<Vec<FakeCall>>>,
        result: Result<ComputeDispatchReport, ComputeExecutionError>,
    }

    impl ComputeBackendFactory for FakeBackend {
        fn descriptor(&self) -> &ComputeBackendDescriptor {
            &self.descriptor
        }

        fn supports(&self, _program: &ComputeProgram) -> Result<(), ComputeBackendRejection> {
            Ok(())
        }

        fn compile(
            &self,
            _program: &ComputeProgram,
        ) -> Result<Box<dyn ComputeExecutable>, ComputeBackendError> {
            Ok(Box::new(FakeExecutable {
                calls: Arc::clone(&self.calls),
                result: self.result.clone(),
            }))
        }
    }

    struct FakeExecutable {
        calls: Arc<Mutex<Vec<FakeCall>>>,
        result: Result<ComputeDispatchReport, ComputeExecutionError>,
    }

    impl ComputeExecutable for FakeExecutable {
        fn create_session(
            &self,
            _initializers: &ComputeInitializerSet,
        ) -> Result<Box<dyn ComputeSession>, ComputeBackendError> {
            Ok(Box::new(FakeSession {
                calls: Arc::clone(&self.calls),
                result: self.result.clone(),
            }))
        }
    }

    struct FakeSession {
        calls: Arc<Mutex<Vec<FakeCall>>>,
        result: Result<ComputeDispatchReport, ComputeExecutionError>,
    }

    impl ComputeSession for FakeSession {
        fn update_inputs(
            &mut self,
            updates: &[ComputeInputUpdate],
        ) -> Result<(), mech_compute::ComputeExecutionError> {
            self.calls
                .lock()
                .unwrap()
                .extend(updates.iter().cloned().map(FakeCall::Update));
            Ok(())
        }

        fn dispatch(
            &mut self,
            request: &ComputeDispatchRequest,
        ) -> Result<ComputeDispatchReport, mech_compute::ComputeExecutionError> {
            self.calls
                .lock()
                .unwrap()
                .push(FakeCall::Dispatch(request.turns.get()));
            self.result.clone()
        }

        fn read_outputs(
            &mut self,
            selection: &mech_compute::ComputeOutputSelection,
        ) -> Result<mech_compute::ComputeOutputSnapshot, mech_compute::ComputeExecutionError>
        {
            self.calls
                .lock()
                .unwrap()
                .push(FakeCall::Read(selection.clone()));
            Ok(Default::default())
        }
    }

    fn program() -> ComputeProgram {
        ComputeProgram::new(
            ComputeRegionInterface {
                inputs: vec![ComputePort {
                    id: mech_compute::ComputePortId::new(0),
                    name: "matrix".into(),
                    slot: CellSlotId::new(0),
                    schema: SchemaId::new(0),
                    element: mech_compute::ComputeElementType::F32,
                    dimensions: vec![2, 3].into_boxed_slice(),
                }]
                .into_boxed_slice(),
                ..Default::default()
            },
            ComputePhysicalPlan::default(),
            ComputeKernel::Elementwise(ElementwiseIr::default()),
        )
    }

    fn program_with_sample_output() -> ComputeProgram {
        ComputeProgram::new(
            ComputeRegionInterface {
                outputs: vec![ComputePort {
                    id: mech_compute::ComputePortId::new(1),
                    name: "result".into(),
                    slot: CellSlotId::new(1),
                    schema: SchemaId::new(1),
                    element: mech_compute::ComputeElementType::F32,
                    dimensions: Vec::new().into_boxed_slice(),
                }]
                .into_boxed_slice(),
                ..Default::default()
            },
            ComputePhysicalPlan::default(),
            ComputeKernel::Elementwise(ElementwiseIr::default()),
        )
    }

    fn registry_with_observer(
        calls: Arc<Mutex<Vec<FakeCall>>>,
        report: ComputeDispatchReport,
    ) -> Arc<ComputeBackendRegistry> {
        registry_with_result(calls, Ok(report))
    }

    fn registry_with_result(
        calls: Arc<Mutex<Vec<FakeCall>>>,
        result: Result<ComputeDispatchReport, ComputeExecutionError>,
    ) -> Arc<ComputeBackendRegistry> {
        let mut registry = ComputeBackendRegistry::default();
        registry
            .register(Arc::new(FakeBackend {
                descriptor: ComputeBackendDescriptor {
                    id: mech_compute::BackendId::new("cpu-scalar").unwrap(),
                    class: BackendClass::Cpu,
                    priority: 1,
                    capabilities: ComputeBackendCapabilities {
                        elementwise: true,
                        native: true,
                        ..Default::default()
                    },
                },
                calls,
                result,
            }))
            .unwrap();
        Arc::new(registry)
    }

    fn registry() -> Arc<ComputeBackendRegistry> {
        registry_with_observer(
            Arc::new(Mutex::new(Vec::new())),
            ComputeDispatchReport {
                completed_turns: 1,
                ..Default::default()
            },
        )
    }

    fn settings() -> ConfigValue {
        ConfigValue::Map(BTreeMap::from([
            (
                "region".to_owned(),
                ConfigValue::String("particle-field".to_owned()),
            ),
            ("backend".to_owned(), ConfigValue::String("cpu".to_owned())),
        ]))
    }

    fn runtime_with_fake_backend(
        grant_access: bool,
        calls: Arc<Mutex<Vec<FakeCall>>>,
        report: ComputeDispatchReport,
    ) -> mech_runtime::MechRuntime {
        let factory = ComputeHostFactory::new(
            "particle-field",
            ComputePlacement::Compute,
            program(),
            ComputeInitializerSet::default(),
            registry_with_observer(calls, report),
            ComputePlatform::Native,
        )
        .unwrap();
        let builder = RuntimeBuilder::new()
            .host_factory(Box::new(factory))
            .unwrap()
            .host_instance(HostInstanceConfig {
                name: "particles".to_owned(),
                provider: "compute".to_owned(),
                settings: settings(),
            });
        let builder = if grant_access {
            builder.run_resource_grant(RunResourceGrantConfig {
                target: "particles/kernel".to_owned(),
                operations: vec!["read".to_owned(), "write".to_owned()],
                paths: vec!["*".to_owned()],
            })
        } else {
            builder
        };
        builder.build().unwrap()
    }

    fn runtime_with_advancing_failure(
        calls: Arc<Mutex<Vec<FakeCall>>>,
    ) -> mech_runtime::MechRuntime {
        let backend = mech_compute::BackendId::new("cpu-scalar").unwrap();
        let factory = ComputeHostFactory::new(
            "particle-field",
            ComputePlacement::Compute,
            program(),
            ComputeInitializerSet::default(),
            registry_with_result(
                calls,
                Err(ComputeExecutionError {
                    backend,
                    operation: "publish outputs",
                    detail: "accepted state could not be published".into(),
                    state_advanced: true,
                }),
            ),
            ComputePlatform::Native,
        )
        .unwrap();
        RuntimeBuilder::new()
            .host_factory(Box::new(factory))
            .unwrap()
            .host_instance(HostInstanceConfig {
                name: "particles".to_owned(),
                provider: "compute".to_owned(),
                settings: settings(),
            })
            .run_resource_grant(RunResourceGrantConfig {
                target: "particles/kernel".to_owned(),
                operations: vec!["read".to_owned(), "write".to_owned()],
                paths: vec!["*".to_owned()],
            })
            .build()
            .unwrap()
    }

    fn input_write(values: Vec<f32>) -> RuntimeResourceWriteRequest {
        RuntimeResourceWriteRequest {
            base_uri: "compute://particles/kernel".to_owned(),
            path: "input/matrix".to_owned(),
            context_name: "particles".to_owned(),
            operation: RuntimeCapabilityOperation::Write,
            value: RuntimeHostInputValue::F32Matrix {
                rows: 2,
                columns: 3,
                values,
            }
            .into_value()
            .unwrap(),
            intent: RuntimeResourceWriteIntent::Send,
        }
    }

    fn turn_write() -> RuntimeResourceWriteRequest {
        turn_write_with(1.0)
    }

    fn turn_write_with(value: f32) -> RuntimeResourceWriteRequest {
        turn_write_value(RuntimeHostInputValue::F32(value))
    }

    fn turn_write_value(value: RuntimeHostInputValue) -> RuntimeResourceWriteRequest {
        RuntimeResourceWriteRequest {
            base_uri: "compute://particles/kernel".to_owned(),
            path: "turn".to_owned(),
            context_name: "particles".to_owned(),
            operation: RuntimeCapabilityOperation::Write,
            value: value.into_value().unwrap(),
            intent: RuntimeResourceWriteIntent::Send,
        }
    }

    fn provider_state(
        phase: ComputeHostPhase,
        calls: Arc<Mutex<Vec<FakeCall>>>,
    ) -> Arc<Mutex<ComputeHostState>> {
        Arc::new(Mutex::new(ComputeHostState {
            backend: CPU_SCALAR_BACKEND.to_owned(),
            program: Arc::new(program()),
            turns: Ref::new(0.0),
            dispatch_ms: Ref::new(0.0),
            fault_count: Ref::new(0.0),
            last_fault: Ref::new(String::new()),
            sampled_outputs: BTreeMap::new(),
            sample_subscriptions: BTreeSet::new(),
            phase,
            session: Box::new(FakeSession {
                calls,
                result: Ok(ComputeDispatchReport {
                    completed_turns: 1,
                    ..Default::default()
                }),
            }),
        }))
    }

    fn provider_for_state(state: Arc<Mutex<ComputeHostState>>) -> ComputeResourceProvider {
        ComputeResourceProvider {
            instance: "particles".to_owned(),
            region: "particle-field".into(),
            program: Arc::new(program()),
            state,
            telemetry: Arc::new(Mutex::new(None)),
        }
    }

    #[test]
    fn manifest_uses_backend_neutral_provider_and_scheme() {
        let manifest = compute_host_manifest();
        assert_eq!(manifest.provider, "compute");
        assert_eq!(
            manifest.contexts[0].base_uri_template,
            "compute://{instance}/kernel"
        );
    }

    #[test]
    fn exact_matrix_shape_and_layout_are_checked_at_the_host_boundary() {
        let program = program();
        let port = &program.interface().inputs[0];
        let valid = RuntimeHostInputValue::F32Matrix {
            rows: 2,
            columns: 3,
            values: vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
        }
        .into_value()
        .unwrap();
        let update = compute_input_update(&program, port, &valid).unwrap();
        let ComputeValue::TensorF32 { layout, values, .. } = update.value else {
            panic!("matrix became a scalar")
        };
        assert_eq!(layout, TensorLayout::RowMajor);
        assert_eq!(values.as_ref(), [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);

        let wrong_shape = RuntimeHostInputValue::F32Matrix {
            rows: 3,
            columns: 2,
            values: vec![0.0; 6],
        }
        .into_value()
        .unwrap();
        assert!(compute_input_update(&program, port, &wrong_shape).is_err());
    }

    #[test]
    fn f64_resident_inputs_narrow_once_at_the_f32_compute_boundary() {
        let program = program();
        let port = &program.interface().inputs[0];
        let value = RuntimeHostInputValue::F64Matrix {
            rows: 2,
            columns: 3,
            values: vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
        }
        .into_value()
        .unwrap();
        let update = compute_input_update(&program, port, &value).unwrap();
        let ComputeValue::TensorF32 { layout, values, .. } = update.value else {
            panic!("f64 matrix became a scalar")
        };
        assert_eq!(layout, TensorLayout::RowMajor);
        assert_eq!(values.as_ref(), [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);

        let overflow = RuntimeHostInputValue::F64Matrix {
            rows: 2,
            columns: 3,
            values: vec![f64::MAX; 6],
        }
        .into_value()
        .unwrap();
        let error = compute_input_update(&program, port, &overflow).unwrap_err();
        assert!(error.display_message().contains("outside the f32 range"));
    }

    #[test]
    fn sampled_batch_returns_nonsquare_lane_zero_in_canonical_row_major_order() {
        let port = ComputePort {
            id: mech_compute::ComputePortId::new(1),
            name: "matrix".into(),
            slot: CellSlotId::new(1),
            schema: SchemaId::new(1),
            element: mech_compute::ComputeElementType::F32,
            dimensions: vec![2, 3].into_boxed_slice(),
        };
        let sampled = sampled_output_value(
            &port,
            ComputeValue::TensorF32 {
                dimensions: vec![2, 2, 3].into_boxed_slice(),
                layout: TensorLayout::RowMajor,
                values: Arc::from([
                    1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 10.0, 20.0, 30.0, 40.0, 50.0, 60.0,
                ]),
            },
        )
        .unwrap();
        assert_eq!(
            RuntimeHostInputValue::from_numeric_value(&sampled).unwrap(),
            RuntimeHostInputValue::F64Matrix {
                rows: 2,
                columns: 3,
                values: vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
            },
        );
    }

    #[test]
    fn factory_rejects_a_second_compute_host_instance() {
        let factory = ComputeHostFactory::new(
            "particle-field",
            ComputePlacement::Compute,
            program(),
            ComputeInitializerSet::default(),
            registry(),
            ComputePlatform::Native,
        )
        .unwrap();
        factory.instantiate("particles", &settings()).unwrap();
        let error = factory.instantiate("second", &settings()).unwrap_err();
        assert_eq!(error.kind_name(), "MultipleComputeHostsUnsupported");
    }

    #[test]
    fn compatible_factory_resumes_completed_host_turn_state() {
        let resume = ComputeHostResumeState {
            turns: 120.0,
            dispatch_ms: 3.25,
            fault_count: 2.0,
            last_fault: "retained diagnostic".to_owned(),
            sampled_outputs: BTreeMap::new(),
            last_submitted_turn: Some(120),
        };
        let factory = ComputeHostFactory::new(
            "particle-field",
            ComputePlacement::Compute,
            program(),
            ComputeInitializerSet::default(),
            registry(),
            ComputePlatform::Native,
        )
        .unwrap()
        .with_resume_state(resume);
        let snapshot = factory.state_snapshot_handle();
        let installation = factory.instantiate("particles", &settings()).unwrap();
        let resumed = snapshot.snapshot().unwrap().unwrap();
        assert_eq!(resumed.turns, 120.0);
        assert_eq!(resumed.dispatch_ms, 3.25);
        assert_eq!(resumed.fault_count, 2.0);
        assert_eq!(resumed.last_fault, "retained diagnostic");
        assert_eq!(resumed.last_submitted_turn, Some(120));
        drop(installation);
        assert!(snapshot.snapshot().unwrap().is_none());
    }

    #[test]
    fn compatible_factory_preserves_current_sample_for_retained_output_contract() {
        let current = RuntimeHostInputValue::F32(42.0).into_value().unwrap();
        let resume = ComputeHostResumeState {
            turns: 120.0,
            dispatch_ms: 3.25,
            fault_count: 2.0,
            last_fault: "retained diagnostic".to_owned(),
            sampled_outputs: BTreeMap::from([("result".to_owned(), current)]),
            last_submitted_turn: Some(120),
        };
        let factory = ComputeHostFactory::new(
            "particle-field",
            ComputePlacement::Compute,
            program_with_sample_output(),
            ComputeInitializerSet::default(),
            registry(),
            ComputePlatform::Native,
        )
        .unwrap()
        .with_retained_outputs(BTreeSet::from(["result".to_owned()]))
        .unwrap()
        .with_resume_state(resume);
        let snapshot = factory.state_snapshot_handle();
        let installation = factory.instantiate("particles", &settings()).unwrap();
        let observed = installation.resource_providers[0]
            .read(RuntimeResourceReadRequest {
                base_uri: "compute://particles/kernel".to_owned(),
                path: "sample/result".to_owned(),
                context_name: "candidate".to_owned(),
            })
            .unwrap();
        let resumed = snapshot.snapshot().unwrap().unwrap();

        assert_eq!(
            RuntimeHostInputValue::from_numeric_value(&observed).unwrap(),
            RuntimeHostInputValue::F32(42.0),
            "a newly demanded candidate output must read the current generation",
        );
        assert_eq!(
            RuntimeHostInputValue::from_numeric_value(
                resumed.sampled_outputs.get("result").unwrap()
            )
            .unwrap(),
            RuntimeHostInputValue::F32(42.0)
        );
    }

    #[test]
    fn report_only_factory_does_not_materialize_or_migrate_declared_outputs() {
        let stale = RuntimeHostInputValue::F32(42.0).into_value().unwrap();
        let resume = ComputeHostResumeState {
            turns: 120.0,
            dispatch_ms: 3.25,
            fault_count: 0.0,
            last_fault: String::new(),
            sampled_outputs: BTreeMap::from([("result".to_owned(), stale)]),
            last_submitted_turn: Some(120),
        };
        let factory = ComputeHostFactory::new(
            "particle-field",
            ComputePlacement::Compute,
            program_with_sample_output(),
            ComputeInitializerSet::default(),
            registry(),
            ComputePlatform::Native,
        )
        .unwrap()
        .with_resume_state(resume);
        let snapshot = factory.state_snapshot_handle();
        let _installation = factory.instantiate("particles", &settings()).unwrap();

        assert!(
            snapshot
                .snapshot()
                .unwrap()
                .unwrap()
                .sampled_outputs
                .is_empty(),
            "outputs absent from the retained contract must remain backend-resident",
        );
    }

    #[test]
    fn an_unplanned_actual_sample_read_materializes_only_that_output() {
        let factory = ComputeHostFactory::new(
            "particle-field",
            ComputePlacement::Compute,
            program_with_sample_output(),
            ComputeInitializerSet::default(),
            registry(),
            ComputePlatform::Native,
        )
        .unwrap();
        let snapshot = factory.state_snapshot_handle();
        let installation = factory.instantiate("particles", &settings()).unwrap();
        assert!(
            snapshot
                .snapshot()
                .unwrap()
                .unwrap()
                .sampled_outputs
                .is_empty(),
        );

        let observed = installation.resource_providers[0]
            .read(RuntimeResourceReadRequest {
                base_uri: "compute://particles/kernel".to_owned(),
                path: "sample/result".to_owned(),
                context_name: "candidate".to_owned(),
            })
            .unwrap();

        assert_eq!(
            RuntimeHostInputValue::from_numeric_value(&observed).unwrap(),
            RuntimeHostInputValue::F64(0.0),
        );
        let resumed = snapshot.snapshot().unwrap().unwrap();
        assert_eq!(
            resumed.sampled_outputs.keys().cloned().collect::<Vec<_>>(),
            ["result"],
        );
        assert!(
            snapshot
                .snapshot_retained(&BTreeSet::new())
                .unwrap()
                .unwrap()
                .sampled_outputs
                .is_empty(),
            "a runtime-only sample must not become replacement migration state",
        );
    }

    #[test]
    fn report_only_dispatch_keeps_declared_outputs_backend_resident() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let program = Arc::new(program_with_sample_output());
        let state = Arc::new(Mutex::new(ComputeHostState {
            backend: CPU_SCALAR_BACKEND.to_owned(),
            turns: Ref::new(0.0),
            dispatch_ms: Ref::new(0.0),
            fault_count: Ref::new(0.0),
            last_fault: Ref::new(String::new()),
            sampled_outputs: BTreeMap::new(),
            sample_subscriptions: BTreeSet::new(),
            phase: ComputeHostPhase::Ready {
                last_submitted_turn: None,
            },
            session: Box::new(FakeSession {
                calls: Arc::clone(&calls),
                result: Ok(ComputeDispatchReport {
                    completed_turns: 1,
                    ..Default::default()
                }),
            }),
            program,
        }));
        let mut effect = ComputeDispatchEffect {
            resource: "compute://particles/kernel".to_owned(),
            region: "particle-field".into(),
            turn: ComputeTurnToken(1),
            state,
            telemetry: Arc::new(Mutex::new(None)),
        };

        effect.deliver().unwrap();

        assert_eq!(calls.lock().unwrap().as_slice(), [FakeCall::Dispatch(1)]);
    }

    #[test]
    fn telemetry_start_replays_the_resumed_level_snapshot() {
        let state = provider_state(
            ComputeHostPhase::Ready {
                last_submitted_turn: Some(ComputeTurnToken(120)),
            },
            Arc::new(Mutex::new(Vec::new())),
        );
        *state.lock().unwrap().turns.borrow_mut() = 120.0;
        let runtime = RuntimeBuilder::new().build().unwrap();
        let mut driver = ComputeTelemetryDriver {
            base_uri: "compute://particles/kernel".to_owned(),
            ingress: Arc::new(Mutex::new(None)),
            live: Arc::new(AtomicBool::new(false)),
            state,
            replay_on_start: true,
            sample_outputs: BTreeSet::new(),
        };

        driver.attach(runtime.ingress()).unwrap();
        assert_eq!(runtime.pending_host_input_count().unwrap(), 0);
        driver.start().unwrap();
        assert_eq!(runtime.pending_host_input_count().unwrap(), 1);
        assert!(driver.is_live());
    }

    #[test]
    fn telemetry_start_does_not_inject_a_fresh_host_turn() {
        let state = provider_state(
            ComputeHostPhase::Ready {
                last_submitted_turn: None,
            },
            Arc::new(Mutex::new(Vec::new())),
        );
        let runtime = RuntimeBuilder::new().build().unwrap();
        let mut driver = ComputeTelemetryDriver {
            base_uri: "compute://particles/kernel".to_owned(),
            ingress: Arc::new(Mutex::new(None)),
            live: Arc::new(AtomicBool::new(false)),
            state,
            replay_on_start: false,
            sample_outputs: BTreeSet::new(),
        };

        driver.attach(runtime.ingress()).unwrap();
        driver.start().unwrap();
        assert_eq!(runtime.pending_host_input_count().unwrap(), 0);
        assert!(driver.is_live());
    }

    #[test]
    fn in_flight_host_state_cannot_be_snapshotted_for_replacement() {
        let state = provider_state(
            ComputeHostPhase::InFlight {
                turn: ComputeTurnToken(121),
            },
            Arc::new(Mutex::new(Vec::new())),
        );
        let snapshot = ComputeHostStateSnapshotHandle::default();
        *snapshot.state.lock().unwrap() = Some(Arc::downgrade(&state));
        let error = snapshot.snapshot().unwrap_err();
        assert_eq!(error.kind_name(), "ComputeHostSnapshot");
        assert!(error.display_message().contains("121 is still in flight"));
    }

    #[test]
    fn backend_override_is_resolved_by_registry_without_changing_source() {
        let factory = ComputeHostFactory::new(
            "particle-field",
            ComputePlacement::Compute,
            program(),
            ComputeInitializerSet::default(),
            registry(),
            ComputePlatform::Native,
        )
        .unwrap()
        .with_backend_override(BackendRequest::Exact(
            mech_compute::BackendId::new("wgpu").unwrap(),
        ));
        assert!(factory.validate_settings("particles", &settings()).is_err());
    }

    #[test]
    fn runtime_delivers_committed_inputs_before_exactly_one_dispatch_and_ingests_telemetry() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let report = ComputeDispatchReport {
            disposition: ComputeDispatchDisposition::Completed,
            completed_turns: 1,
            dispatch_milliseconds: 2.5,
            fault_count: 3,
            last_fault: Some(ComputeFaultEvidence {
                attempted_turn: 4,
                constraint: "finite".into(),
                detail: "candidate rejected".into(),
            }),
        };
        let mut runtime = runtime_with_fake_backend(true, Arc::clone(&calls), report);
        let mut context = runtime.runtime_context().unwrap();
        runtime.begin_transaction(&mut context).unwrap();
        runtime
            .write_resource_with_context(
                &mut context,
                input_write(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]),
            )
            .unwrap();
        runtime
            .write_resource_with_context(&mut context, turn_write())
            .unwrap();

        assert!(calls.lock().unwrap().is_empty());
        runtime
            .commit_runtime_transaction_detailed(&mut context)
            .unwrap();

        let observed = calls.lock().unwrap().clone();
        assert_eq!(observed.len(), 2);
        let FakeCall::Update(update) = &observed[0] else {
            panic!("input must be delivered before dispatch")
        };
        let ComputeValue::TensorF32 { layout, values, .. } = &update.value else {
            panic!("matrix input must remain a matrix")
        };
        assert_eq!(*layout, TensorLayout::RowMajor);
        assert_eq!(values.as_ref(), [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        assert_eq!(observed[1], FakeCall::Dispatch(1));
        assert_eq!(runtime.pending_host_input_count().unwrap(), 1);

        runtime.write_resource(turn_write()).unwrap();
        assert_eq!(
            calls.lock().unwrap().as_slice(),
            observed.as_slice(),
            "repeating a compute turn token must not create a feedback dispatch",
        );
        runtime.write_resource(turn_write_with(2.0)).unwrap();
        assert_eq!(calls.lock().unwrap().last(), Some(&FakeCall::Dispatch(1)));
        assert_eq!(calls.lock().unwrap().len(), 3);
    }

    #[test]
    fn turn_tokens_are_width_independent_and_never_move_backwards() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let mut runtime = runtime_with_fake_backend(
            true,
            Arc::clone(&calls),
            ComputeDispatchReport {
                completed_turns: 1,
                ..Default::default()
            },
        );

        runtime.write_resource(turn_write_with(2.0)).unwrap();
        runtime
            .write_resource(turn_write_value(RuntimeHostInputValue::F64(2.0)))
            .unwrap();
        runtime
            .write_resource(turn_write_value(RuntimeHostInputValue::F64(1.0)))
            .unwrap();
        assert_eq!(
            calls
                .lock()
                .unwrap()
                .iter()
                .filter(|call| matches!(call, FakeCall::Dispatch(_)))
                .count(),
            1,
        );

        runtime
            .write_resource(turn_write_value(RuntimeHostInputValue::F64(3.0)))
            .unwrap();
        assert_eq!(
            calls
                .lock()
                .unwrap()
                .iter()
                .filter(|call| matches!(call, FakeCall::Dispatch(_)))
                .count(),
            2,
        );
        assert!(canonical_turn_token(&RuntimeHostInputValue::F64(3.5)).is_err());
        assert!(canonical_turn_token(&RuntimeHostInputValue::F64(f64::MAX)).is_err());
        assert!(
            canonical_turn_token(&RuntimeHostInputValue::F64(u128::MAX as f64)).is_err(),
            "the rounded 2^128 boundary must not saturate to u128::MAX",
        );
        assert_eq!(
            canonical_turn_token(&RuntimeHostInputValue::F64(2.0_f64.powi(127))).unwrap(),
            ComputeTurnToken(1_u128 << 127),
        );
    }

    #[test]
    fn accepted_but_unpublished_dispatch_stops_retries() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let mut runtime = runtime_with_advancing_failure(Arc::clone(&calls));

        let mut context = runtime.runtime_context().unwrap();
        runtime.begin_transaction(&mut context).unwrap();
        runtime
            .write_resource_with_context(&mut context, turn_write())
            .unwrap();
        let first = runtime
            .commit_runtime_transaction_detailed(&mut context)
            .unwrap();
        assert!(
            first.delivery_failures[0]
                .message
                .contains("accepted state could not be published")
        );

        runtime.begin_transaction(&mut context).unwrap();
        let second = runtime
            .write_resource_with_context(&mut context, turn_write_with(2.0))
            .unwrap_err();
        assert!(second.kind_message().contains("incomplete transaction"));
        runtime
            .abort_runtime_transaction(&mut context, "expected failed-host rejection")
            .unwrap();
        assert_eq!(
            calls
                .lock()
                .unwrap()
                .iter()
                .filter(|call| matches!(call, FakeCall::Dispatch(_)))
                .count(),
            1,
        );
    }

    #[test]
    fn failed_and_inflight_phases_guard_every_resource_and_effect_boundary() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let state = provider_state(
            ComputeHostPhase::Ready {
                last_submitted_turn: None,
            },
            Arc::clone(&calls),
        );
        let provider = provider_for_state(Arc::clone(&state));
        let prepared = provider.prepare_write(input_write(vec![0.0; 6])).unwrap();

        state.lock().unwrap().phase = ComputeHostPhase::InFlight {
            turn: ComputeTurnToken(1),
        };
        let read = RuntimeResourceReadRequest {
            base_uri: "compute://particles/kernel".to_owned(),
            path: "turns".to_owned(),
            context_name: "particles".to_owned(),
        };
        assert!(provider.plan_read(read.clone()).is_err());
        assert!(provider.read(read.clone()).is_err());
        assert!(
            provider
                .preflight_write(RuntimeResourceWritePreflightRequest {
                    base_uri: "compute://particles/kernel".to_owned(),
                    path: "turn".to_owned(),
                    context_name: "particles".to_owned(),
                    operation: RuntimeCapabilityOperation::Write,
                    intent: RuntimeResourceWriteIntent::Send,
                })
                .is_err()
        );
        assert!(provider.plan_write(turn_write()).is_err());
        assert!(provider.prepare_write(turn_write()).is_err());
        let PreparedRuntimeEffect::AfterCommit(mut effect) = prepared else {
            panic!("compute input must remain an after-commit effect")
        };
        assert!(effect.deliver().is_err());
        assert!(calls.lock().unwrap().is_empty());

        state.lock().unwrap().fail("first terminal reason");
        assert!(provider.plan_read(read.clone()).is_err());
        assert!(provider.read(read).is_err());
        assert!(provider.plan_write(turn_write()).is_err());
        assert!(provider.prepare_write(turn_write()).is_err());
        assert!(calls.lock().unwrap().is_empty());
    }

    #[test]
    fn asynchronous_completion_requires_the_exact_inflight_turn_and_failure_is_absorbing() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let state = provider_state(
            ComputeHostPhase::InFlight {
                turn: ComputeTurnToken(7),
            },
            calls,
        );
        let target = ComputeHostCompletionTarget {
            backend: BackendId::new(CPU_SCALAR_BACKEND).unwrap(),
            resource: "compute://particles/kernel".to_owned(),
            state: Arc::downgrade(&state),
            telemetry: Arc::new(Mutex::new(None)),
        };
        let mismatch = target.complete(ComputeCompletionOutcome::Completed {
            attempted_turn: 6,
            report: ComputeDispatchReport {
                completed_turns: 1,
                ..Default::default()
            },
            snapshot: ComputeOutputSnapshot::default(),
        });
        assert!(mismatch.is_err());
        let first_reason = match &state.lock().unwrap().phase {
            ComputeHostPhase::Failed { reason } => reason.clone(),
            phase => panic!("mismatch did not stop the host: {phase:?}"),
        };
        let later = target.complete(ComputeCompletionOutcome::TransportFailed {
            attempted_turn: 7,
            reason: "later transport failure".into(),
        });
        assert!(later.is_err());
        assert!(matches!(
            &state.lock().unwrap().phase,
            ComputeHostPhase::Failed { reason } if *reason == first_reason
        ));
    }

    #[test]
    fn integrity_rejection_returns_the_matching_host_to_ready() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let state = provider_state(
            ComputeHostPhase::InFlight {
                turn: ComputeTurnToken(9),
            },
            calls,
        );
        let target = ComputeHostCompletionTarget {
            backend: BackendId::new(CPU_SCALAR_BACKEND).unwrap(),
            resource: "compute://particles/kernel".to_owned(),
            state: Arc::downgrade(&state),
            telemetry: Arc::new(Mutex::new(None)),
        };
        target
            .complete(ComputeCompletionOutcome::IntegrityRejected {
                attempted_turn: 9,
                report: ComputeDispatchReport {
                    disposition: ComputeDispatchDisposition::Rejected,
                    completed_turns: 0,
                    fault_count: 1,
                    last_fault: Some(ComputeFaultEvidence {
                        attempted_turn: 9,
                        constraint: "finite".into(),
                        detail: "candidate rejected".into(),
                    }),
                    ..Default::default()
                },
            })
            .unwrap();
        assert_eq!(
            state.lock().unwrap().phase,
            ComputeHostPhase::Ready {
                last_submitted_turn: Some(ComputeTurnToken(9)),
            }
        );
        assert!(
            provider_for_state(state)
                .plan_write(turn_write_with(10.0))
                .is_ok()
        );
    }

    #[test]
    fn capability_denial_happens_before_the_compute_backend_is_touched() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let mut runtime =
            runtime_with_fake_backend(false, Arc::clone(&calls), ComputeDispatchReport::default());

        assert!(runtime.write_resource(turn_write()).is_err());
        assert!(calls.lock().unwrap().is_empty());
        assert_eq!(runtime.pending_host_input_count().unwrap(), 0);
    }

    #[test]
    fn rejected_runtime_candidate_produces_no_compute_dispatch() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let mut runtime =
            runtime_with_fake_backend(true, Arc::clone(&calls), ComputeDispatchReport::default());
        let mut context = runtime.runtime_context().unwrap();
        runtime.begin_transaction(&mut context).unwrap();
        runtime
            .write_resource_with_context(&mut context, turn_write())
            .unwrap();

        runtime
            .abort_runtime_transaction(&mut context, "candidate rejected")
            .unwrap();

        assert!(calls.lock().unwrap().is_empty());
        assert_eq!(runtime.pending_host_input_count().unwrap(), 0);
    }
}
