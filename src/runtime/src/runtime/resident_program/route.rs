use mech_core::{
    LayoutGeneration, MResult, MechError, MechErrorKind, PlanGeneration, ProgramRevision,
};

use crate::{ResidentDurabilityPolicy, ResidentRoutingPolicy, RuntimeValueSnapshot};
use crate::{ResidentExternalHealth, runtime::MechRuntime};
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use std::time::Instant;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use web_time::Instant;

use super::ActiveProgramExecution;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RuntimeProgramRoute {
    #[default]
    None,
    Legacy,
    ResidentPure,
    ResidentExternal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeProgramExecutionInfo {
    pub route: RuntimeProgramRoute,
    pub policy: ResidentRoutingPolicy,
    pub program_revision: Option<ProgramRevision>,
    pub plan_generation: Option<PlanGeneration>,
    pub layout_generation: Option<LayoutGeneration>,
    pub requirement_count: usize,
    pub observation_count: usize,
    pub effect_count: usize,
    pub resident_accepted_turns: u64,
    pub resident_rejected_turns: u64,
    pub coalesced_host_packets: u64,
    pub ignored_host_packets: u64,
    pub legacy_turns: u64,
}

impl Default for RuntimeProgramExecutionInfo {
    fn default() -> Self {
        Self {
            route: RuntimeProgramRoute::None,
            policy: ResidentRoutingPolicy::RequireResident,
            program_revision: None,
            plan_generation: None,
            layout_generation: None,
            requirement_count: 0,
            observation_count: 0,
            effect_count: 0,
            resident_accepted_turns: 0,
            resident_rejected_turns: 0,
            coalesced_host_packets: 0,
            ignored_host_packets: 0,
            legacy_turns: 0,
        }
    }
}

#[derive(Debug)]
pub struct RuntimeProgramLoadOutcome {
    pub route: RuntimeProgramRoute,
    pub initial_value: RuntimeValueSnapshot,
    pub info: RuntimeProgramExecutionInfo,
}

/// Retained resident evidence transferred out of the active coordinator for
/// checkpointing, replication, or archival. Draining releases the bounded
/// ledger capacity so retained production programs can continue executing.
#[derive(Debug, Default)]
pub struct RuntimeResidentEvidence {
    pub input_batches: Vec<(crate::LedgerSequence, crate::CapturedInputBatch)>,
    pub receipts: Vec<(crate::LedgerSequence, crate::ResidentTurnRecord)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResidentRouteFailureClass {
    SemanticUnsupported,
    MultipleRootsUnsupported,
    ReplUnsupported,
    ProviderUnavailable,
    ProviderContractMismatch,
    AuthorizationDenied,
    InvalidArtifact,
    InvalidBytecode,
    ActivationFailure,
    InternalFailure,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResidentRouteFailure {
    pub class: ResidentRouteFailureClass,
    pub reason: String,
}

impl MechErrorKind for ResidentRouteFailure {
    fn name(&self) -> &str {
        "ResidentRouteFailure"
    }

    fn message(&self) -> String {
        format!("{:?}: {}", self.class, self.reason)
    }
}

pub(crate) fn route_failure(
    class: ResidentRouteFailureClass,
    reason: impl Into<String>,
) -> mech_core::MechError {
    MechError::new(
        ResidentRouteFailure {
            class,
            reason: reason.into(),
        },
        None,
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResidentEnvironmentFrozen {
    pub operation: &'static str,
}

impl MechErrorKind for ResidentEnvironmentFrozen {
    fn name(&self) -> &str {
        "ResidentEnvironmentFrozen"
    }

    fn message(&self) -> String {
        format!(
            "runtime environment operation `{}` is frozen while a resident program is active",
            self.operation
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResidentProgramNotQuiescent;

impl MechErrorKind for ResidentProgramNotQuiescent {
    fn name(&self) -> &str {
        "ResidentProgramNotQuiescent"
    }

    fn message(&self) -> String {
        "the active resident program is not quiescent".to_owned()
    }
}

pub(crate) fn invalid_active_program(reason: impl Into<String>) -> mech_core::MechError {
    route_failure(ResidentRouteFailureClass::InternalFailure, reason)
}

pub(crate) fn unsupported_route(reason: impl Into<String>) -> mech_core::MechError {
    route_failure(ResidentRouteFailureClass::SemanticUnsupported, reason)
}

pub(crate) fn ensure_supported_durability(policy: ResidentDurabilityPolicy) -> MResult<()> {
    if matches!(
        policy,
        ResidentDurabilityPolicy::Volatile | ResidentDurabilityPolicy::Retained
    ) {
        Ok(())
    } else {
        Err(route_failure(
            ResidentRouteFailureClass::InternalFailure,
            format!("resident durability {policy:?} is unavailable in D4"),
        ))
    }
}

impl MechRuntime {
    /// Stages legacy ownership in the transaction that installed retained
    /// executable state. Planning transactions intentionally do not claim a
    /// production program route.
    pub(in crate::runtime) fn stage_legacy_program_owner(
        &mut self,
        context: &crate::RuntimeContext,
    ) -> MResult<()> {
        if self.execution_mode == crate::runtime::RuntimeExecutionMode::Plan {
            return Ok(());
        }
        let transaction_id = Self::context_transaction_id(context)?;
        self.active_execution_transaction_mut(transaction_id)?
            .claims_legacy_program_owner = true;
        Ok(())
    }

    /// Publishes a transaction's successful legacy ownership claim.
    pub(in crate::runtime) fn publish_legacy_program_owner(&mut self) {
        match self.active_program {
            ActiveProgramExecution::None => {
                self.active_program = ActiveProgramExecution::Legacy;
                self.program_execution_info = RuntimeProgramExecutionInfo {
                    route: RuntimeProgramRoute::Legacy,
                    policy: crate::ResidentRoutingPolicy::LegacyOnly,
                    legacy_turns: 1,
                    ..RuntimeProgramExecutionInfo::default()
                };
            }
            ActiveProgramExecution::Legacy => {
                self.program_execution_info.legacy_turns =
                    self.program_execution_info.legacy_turns.saturating_add(1);
            }
            ActiveProgramExecution::ResidentPure(_)
            | ActiveProgramExecution::ResidentExternal(_) => {
                debug_assert!(false, "resident ownership excludes legacy program commits");
            }
        }
    }

    pub fn program_route(&self) -> RuntimeProgramRoute {
        self.program_execution_info.route
    }

    pub fn program_execution_info(&self) -> RuntimeProgramExecutionInfo {
        self.program_execution_info.clone()
    }

    pub(crate) fn program_route_for_debug(&self) -> RuntimeProgramRoute {
        self.program_route()
    }

    pub fn drain_resident_evidence(&mut self) -> MResult<RuntimeResidentEvidence> {
        let ActiveProgramExecution::ResidentExternal(execution) = &mut self.active_program else {
            return Err(invalid_active_program(
                "retained evidence is available only for an active external resident program",
            ));
        };
        let mut evidence = RuntimeResidentEvidence::default();
        while let Some(batch) = execution.coordinator.release_next_input_batch() {
            evidence.input_batches.push(batch);
        }
        while let Some(receipt) = execution.coordinator.release_next_receipt() {
            evidence.receipts.push(receipt);
        }
        Ok(evidence)
    }

    pub fn retry_resident_outbox(&mut self) -> MResult<Box<[crate::RuntimeEffectFailure]>> {
        self.revalidate_active_resident_grants()?;
        let ActiveProgramExecution::ResidentExternal(execution) = &mut self.active_program else {
            return Err(invalid_active_program(
                "resident outbox retry requires an active external resident program",
            ));
        };
        execution.coordinator.retry_outbox()
    }

    /// Advances the active program through its owning execution route.
    pub fn step_active_program(&mut self) -> MResult<()> {
        let max_turn_duration_ms = self.config.limits.max_turn_duration_ms;
        match &mut self.active_program {
            ActiveProgramExecution::ResidentPure(execution) => {
                let turn_started = Instant::now();
                let prepared = execution.instance.prepare_turn(&[]).map_err(|error| {
                    route_failure(
                        ResidentRouteFailureClass::ActivationFailure,
                        format!("resident pure step failed: {error:?}"),
                    )
                })?;
                if let Err(error) = super::super::limits::enforce_turn_duration_limit(
                    max_turn_duration_ms,
                    turn_started,
                ) {
                    prepared.abort();
                    return Err(error);
                }
                prepared.publish().map_err(|error| {
                    route_failure(
                        ResidentRouteFailureClass::ActivationFailure,
                        format!("resident pure publication failed: {error:?}"),
                    )
                })?;
                self.program_execution_info.resident_accepted_turns = self
                    .program_execution_info
                    .resident_accepted_turns
                    .saturating_add(1);
                Ok(())
            }
            ActiveProgramExecution::ResidentExternal(_) => Err(invalid_active_program(
                "external resident programs advance only from admitted host input",
            )),
            ActiveProgramExecution::Legacy | ActiveProgramExecution::None => self.step(0),
        }
    }

    pub(crate) fn ensure_resident_environment_mutable(
        &self,
        operation: &'static str,
    ) -> MResult<()> {
        if matches!(
            self.active_program,
            ActiveProgramExecution::ResidentPure(_) | ActiveProgramExecution::ResidentExternal(_)
        ) {
            Err(MechError::new(
                ResidentEnvironmentFrozen { operation },
                None,
            ))
        } else {
            Ok(())
        }
    }

    pub fn unload_active_program(&mut self) -> MResult<()> {
        let retained_legacy = self.program.interpreter().has_retained_execution_state()
            || !self.live_input_bindings.is_empty()
            || self.live_context_template.is_some();
        let program_owned =
            !matches!(self.active_program, ActiveProgramExecution::None) || retained_legacy;
        let drivers_stopped = self.input_drivers[..self.attached_input_driver_count]
            .iter()
            .try_fold(true, |stopped, driver| {
                if !stopped {
                    return Ok(false);
                }
                crate::runtime::extension::invoke_extension_value(
                    "host input driver",
                    "is_live",
                    || driver.is_live(),
                )
                .map(|live| !live)
            })?;
        if let ActiveProgramExecution::ResidentExternal(execution) = &self.active_program {
            if execution.coordinator.health() != &ResidentExternalHealth::Healthy
                || execution.coordinator.pending_outbox_count() != 0
                || execution.coordinator.has_active_candidate()
                || execution.coordinator.input_facts().next().is_some()
                || execution.coordinator.receipts().next().is_some()
                || self.pending_host_input_count()? != 0
                || !drivers_stopped
            {
                return Err(MechError::new(ResidentProgramNotQuiescent, None));
            }
        } else if program_owned && (self.pending_host_input_count()? != 0 || !drivers_stopped) {
            return Err(MechError::new(ResidentProgramNotQuiescent, None));
        }
        if program_owned {
            let replacement = self.new_program(self.program.config.clone());
            self.program = replacement;
            self.live_input_bindings.clear();
            self.live_context_template = None;
        }
        self.active_program = ActiveProgramExecution::None;
        self.program_execution_info = RuntimeProgramExecutionInfo::default();
        self.resident_production_probe = Default::default();
        Ok(())
    }
}
