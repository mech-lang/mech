use mech_core::{
    LayoutGeneration, MResult, MechError, MechErrorKind, PlanGeneration, ProgramRevision,
};

use crate::{ResidentDurabilityPolicy, ResidentRoutingPolicy, RuntimeValueSnapshot};
use crate::{ResidentExternalHealth, runtime::MechRuntime};

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
            policy: ResidentRoutingPolicy::PreferResident,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeProgramLoadOptions {
    pub routing: ResidentRoutingPolicy,
    pub durability: ResidentDurabilityPolicy,
}

impl Default for RuntimeProgramLoadOptions {
    fn default() -> Self {
        Self {
            routing: ResidentRoutingPolicy::PreferResident,
            durability: ResidentDurabilityPolicy::Volatile,
        }
    }
}

#[derive(Debug)]
pub struct RuntimeProgramLoadOutcome {
    pub route: RuntimeProgramRoute,
    pub initial_value: RuntimeValueSnapshot,
    pub info: RuntimeProgramExecutionInfo,
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
    pub fn program_route(&self) -> RuntimeProgramRoute {
        self.program_execution_info.route
    }

    pub fn program_execution_info(&self) -> RuntimeProgramExecutionInfo {
        self.program_execution_info.clone()
    }

    pub fn record_legacy_program_route(
        &mut self,
        policy: crate::ResidentRoutingPolicy,
        turns: u64,
    ) -> MResult<()> {
        self.ensure_program_slot_available()?;
        self.active_program = ActiveProgramExecution::Legacy;
        self.program_execution_info = RuntimeProgramExecutionInfo {
            route: RuntimeProgramRoute::Legacy,
            policy,
            legacy_turns: turns,
            ..RuntimeProgramExecutionInfo::default()
        };
        Ok(())
    }

    pub(crate) fn program_route_for_debug(&self) -> RuntimeProgramRoute {
        self.program_route()
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
        if let ActiveProgramExecution::ResidentExternal(execution) = &self.active_program {
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
            if execution.coordinator.health() != &ResidentExternalHealth::Healthy
                || execution.coordinator.pending_outbox_count() != 0
                || execution.coordinator.has_active_candidate()
                || !drivers_stopped
            {
                return Err(MechError::new(ResidentProgramNotQuiescent, None));
            }
        }
        self.active_program = ActiveProgramExecution::None;
        self.program_execution_info = RuntimeProgramExecutionInfo::default();
        self.resident_production_probe = Default::default();
        Ok(())
    }
}
