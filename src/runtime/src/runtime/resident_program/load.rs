use std::sync::Arc;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use std::time::Instant;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use web_time::Instant;

#[cfg(feature = "resident-routing-source")]
use mech_core::MechSourceCode;
use mech_core::{
    ApplicationRequirement, MResult, MechError, ParsedProgram, ReactiveInstanceId, ResourceIntent,
};
use mech_engine::{
    ProgramArtifact, decode_program_artifact_sections,
    resident::{
        ActivationFacts, ResidentActivationOptions, ResidentExternalAdmission,
        ResidentIntegrityMode, activate_with_options, preflight_activation,
    },
};

#[cfg(feature = "resident-routing-source")]
use crate::{ModuleBuildOptions, SourceRequest};
use crate::{
    ResidentExternalCoordinator, ResidentExternalLimits, RuntimeResourceWriteIntent,
    runtime::MechRuntime,
};
#[cfg(feature = "resident-routing-source")]
use mech_engine::{MechProgramConfig, MechProgramEnvironment, ProgramCompilationProduct};

use super::admission::activation_failure_for_artifact;
use super::execution::initial_value;
use super::{
    ActiveProgramExecution, ResidentExternalExecution, ResidentPureExecution,
    ResidentRouteFailureClass, RuntimeProgramExecutionInfo, RuntimeProgramLoadOutcome,
    RuntimeProgramRoute, invalid_active_program, route_failure,
};

impl MechRuntime {
    #[cfg(feature = "resident-routing-source")]
    pub fn load_source_program(
        &mut self,
        source: &str,
        durability: crate::ResidentDurabilityPolicy,
    ) -> MResult<RuntimeProgramLoadOutcome> {
        self.enforce_source_byte_limit(u64::try_from(source.len()).unwrap_or(u64::MAX))?;
        self.load_production_with(durability, |runtime| {
            Ok(Arc::new(
                runtime.plan_source_product(source)?.into_parts().0,
            ))
        })
    }

    #[cfg(feature = "resident-routing-source")]
    pub fn load_root_program(
        &mut self,
        request: SourceRequest,
        module_options: ModuleBuildOptions<'_>,
        durability: crate::ResidentDurabilityPolicy,
    ) -> MResult<RuntimeProgramLoadOutcome> {
        self.load_production_with(durability, |runtime| {
            let resolved = runtime.resolve_source(request.clone())?.ok_or_else(|| {
                route_failure(
                    ResidentRouteFailureClass::InvalidArtifact,
                    format!("root source `{}` was not found", request.specifier),
                )
            })?;
            match &resolved.source {
                MechSourceCode::String(source) => {
                    runtime.enforce_source_byte_limit(
                        u64::try_from(source.len()).unwrap_or(u64::MAX),
                    )?;
                    Ok(Arc::new(
                        runtime
                            .plan_root_source_product(request.clone(), module_options)?
                            .into_parts()
                            .0,
                    ))
                }
                MechSourceCode::Tree(_) => Ok(Arc::new(
                    runtime
                        .plan_root_source_product(request.clone(), module_options)?
                        .into_parts()
                        .0,
                )),
                MechSourceCode::ByteCode(bytecode) => {
                    runtime.enforce_source_byte_limit(
                        u64::try_from(bytecode.len()).unwrap_or(u64::MAX),
                    )?;
                    runtime.decode_artifact(bytecode)
                }
                _ => Err(route_failure(
                    ResidentRouteFailureClass::SemanticUnsupported,
                    "root source kind is not resident-routable",
                )),
            }
        })
    }

    #[cfg(feature = "resident-routing")]
    pub fn load_bytecode_program(
        &mut self,
        bytecode: &[u8],
        durability: crate::ResidentDurabilityPolicy,
    ) -> MResult<RuntimeProgramLoadOutcome> {
        self.enforce_source_byte_limit(u64::try_from(bytecode.len()).unwrap_or(u64::MAX))?;
        self.load_production_with(durability, |runtime| runtime.decode_artifact(bytecode))
    }

    fn load_production_with<Resident>(
        &mut self,
        durability: crate::ResidentDurabilityPolicy,
        resident: Resident,
    ) -> MResult<RuntimeProgramLoadOutcome>
    where
        Resident: FnOnce(&mut Self) -> MResult<Arc<ProgramArtifact>>,
    {
        self.ensure_program_slot_available()?;
        super::ensure_supported_durability(durability)?;
        let result = resident(self)
            .and_then(|artifact| self.install_resident_artifact(artifact, durability));
        if result.is_err() {
            debug_assert!(matches!(self.active_program, ActiveProgramExecution::None));
            self.program_execution_info = RuntimeProgramExecutionInfo::default();
        }
        result
    }

    #[cfg(feature = "resident-routing-source")]
    fn plan_root_source_product(
        &mut self,
        request: SourceRequest,
        module_options: ModuleBuildOptions<'_>,
    ) -> MResult<ProgramCompilationProduct> {
        self.compiler_view()?.compile_root(request, module_options)
    }

    #[cfg(feature = "resident-routing-source")]
    fn plan_source_product(&mut self, source: &str) -> MResult<ProgramCompilationProduct> {
        self.compiler_view()?.compile_source(source)
    }

    #[cfg(feature = "resident-routing-source")]
    fn compiler_view(&self) -> MResult<super::super::program::ProgramCompilerView<'_>> {
        let program_config = MechProgramConfig {
            name: self.config.name.clone(),
            environment: MechProgramEnvironment {
                trace_enabled: self.config.diagnostics.trace_enabled,
                debug_enabled: self.config.diagnostics.debug_enabled,
                profile_enabled: self.config.diagnostics.profile_enabled,
                rounds_per_step: self.config.limits.max_steps_per_turn_as_usize()?,
            },
        };
        Ok(super::super::program::ProgramCompilerView::new(
            Arc::clone(&self.function_catalog),
            self.source_resolver.as_ref(),
            &self.resources,
            &self.module_builder,
            &self.host_interfaces,
            &self.module_manifests,
            program_config,
        ))
    }

    #[cfg(feature = "resident-routing")]
    fn decode_artifact(&self, bytecode: &[u8]) -> MResult<Arc<ProgramArtifact>> {
        let parsed = ParsedProgram::from_bytes(bytecode).map_err(|error| {
            route_failure(
                ResidentRouteFailureClass::InvalidBytecode,
                format!("invalid bytecode v1: {error:?}"),
            )
        })?;
        let artifact = decode_program_artifact_sections(&parsed.artifact).map_err(|error| {
            route_failure(
                ResidentRouteFailureClass::InvalidBytecode,
                format!("invalid bytecode-v1 ProgramArtifact: {error:?}"),
            )
        })?;
        Ok(Arc::new(artifact))
    }

    #[cfg(feature = "resident-routing")]
    fn install_resident_artifact(
        &mut self,
        artifact: Arc<ProgramArtifact>,
        durability: crate::ResidentDurabilityPolicy,
    ) -> MResult<RuntimeProgramLoadOutcome> {
        let external = !artifact.requirements().is_empty();
        let activation_options = ResidentActivationOptions {
            integrity: ResidentIntegrityMode::Checked,
            external: if external {
                ResidentExternalAdmission::StructuralOnly
            } else {
                ResidentExternalAdmission::Deny
            },
        };

        // Provider availability and semantic contracts are authority failures,
        // not semantic-admission failures. Check them before activation so
        // callers receive the correct failure classification.
        if external {
            self.preflight_provider_contract_presence(&artifact)?;
        }

        let preflight = preflight_activation(
            &artifact,
            &self.function_catalog,
            &ActivationFacts::default(),
            activation_options,
        )
        .map_err(|error| activation_failure_for_artifact(&artifact, error))?;
        if !external && !preflight.plan.inputs.is_empty() {
            return Err(super::unsupported_route(
                "pure production resident programs cannot require turn inputs",
            ));
        }

        let authority = if external {
            let authority = self.build_resident_authority(&artifact)?;
            crate::bind_external_requirements(
                &preflight.plan,
                &artifact,
                &self.resources,
                &authority,
            )
            .map_err(classify_provider_preflight)?;
            Some(authority)
        } else {
            None
        };

        let instance_id = self.allocate_resident_instance()?;
        let mut instance = activate_with_options(
            instance_id,
            &artifact,
            &self.function_catalog,
            &ActivationFacts::default(),
            activation_options,
        )
        .map_err(|error| activation_failure_for_artifact(&artifact, error))?;

        let requirement_count = artifact.requirements().len();
        let observation_count = instance.plan.inputs.len();
        let effect_count = instance.plan.external_effect_count();
        let mut info = RuntimeProgramExecutionInfo {
            route: if external {
                RuntimeProgramRoute::ResidentExternal
            } else {
                RuntimeProgramRoute::ResidentPure
            },
            program_revision: Some(artifact.revision()),
            plan_generation: Some(instance.plan.plan_generation),
            layout_generation: Some(instance.plan.layout_generation),
            requirement_count,
            observation_count,
            effect_count,
            ..RuntimeProgramExecutionInfo::default()
        };

        let initial_snapshot;
        let active = if external {
            let authority = authority.expect("external authority was built");
            let mut coordinator = ResidentExternalCoordinator::new_live(
                instance,
                Arc::clone(&artifact),
                &self.resources,
                &authority,
                durability,
                ResidentExternalLimits::default(),
            )?;
            let trigger_sources = coordinator.trigger_sources()?;
            self.ensure_exact_resident_input_drivers(&trigger_sources)?;
            if trigger_sources.is_empty() {
                let max_turn_duration_ms = self.config.limits.max_turn_duration_ms;
                let turn_started = Instant::now();
                let admission = coordinator.admit_turn()?;
                let outcome = coordinator.execute_admitted_turn(admission, move || {
                    super::super::limits::enforce_turn_duration_limit(
                        max_turn_duration_ms,
                        turn_started,
                    )
                })?;
                if let Some(error) = super::resident_host_turn_error(&outcome) {
                    return Err(route_failure(
                        ResidentRouteFailureClass::ActivationFailure,
                        format!(
                            "effect-only resident activation did not complete cleanly: {error:?}"
                        ),
                    ));
                }
                info.resident_accepted_turns = 1;
            }
            initial_snapshot = initial_value(coordinator.instance())?;
            ActiveProgramExecution::ResidentExternal(ResidentExternalExecution {
                artifact,
                coordinator,
                trigger_sources,
                grants: authority,
            })
        } else {
            let max_turn_duration_ms = self.config.limits.max_turn_duration_ms;
            let turn_started = Instant::now();
            let prepared = instance.prepare_turn(&[]).map_err(|error| {
                route_failure(
                    ResidentRouteFailureClass::ActivationFailure,
                    format!("initial pure resident turn failed: {error:?}"),
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
                    format!("initial pure resident publication failed: {error:?}"),
                )
            })?;
            info.resident_accepted_turns = 1;
            initial_snapshot = initial_value(&instance)?;
            ActiveProgramExecution::ResidentPure(ResidentPureExecution { artifact, instance })
        };
        self.active_program = active;
        self.program_execution_info = info.clone();
        Ok(RuntimeProgramLoadOutcome {
            route: info.route,
            initial_value: initial_snapshot,
            info,
        })
    }

    fn ensure_exact_resident_input_drivers(
        &self,
        sources: &[crate::RuntimeHostInputSource],
    ) -> MResult<()> {
        for source in sources {
            let mut owners = 0usize;
            for driver in &self.input_drivers[..self.attached_input_driver_count] {
                if crate::runtime::extension::invoke_extension_value(
                    "host input driver",
                    "drives",
                    || driver.drives(source),
                )? {
                    owners += 1;
                }
            }
            match owners {
                1 => {}
                0 => {
                    return Err(route_failure(
                        ResidentRouteFailureClass::ProviderUnavailable,
                        format!(
                            "no active input driver owns resident trigger `{}/{}`",
                            source.base_uri(),
                            source.path(),
                        ),
                    ));
                }
                _ => {
                    return Err(route_failure(
                        ResidentRouteFailureClass::ProviderContractMismatch,
                        format!(
                            "resident trigger `{}/{}` has {owners} input drivers; exactly one is required",
                            source.base_uri(),
                            source.path(),
                        ),
                    ));
                }
            }
        }
        Ok(())
    }

    pub(super) fn ensure_program_slot_available(&self) -> MResult<()> {
        if !self.active_transactions.is_empty() {
            return Err(invalid_active_program(
                "a runtime transaction is active; finish it before loading a program",
            ));
        }
        if matches!(self.active_program, ActiveProgramExecution::None) {
            Ok(())
        } else {
            Err(invalid_active_program(
                "one runtime may own only one resident program; unload it first",
            ))
        }
    }

    fn allocate_resident_instance(&mut self) -> MResult<ReactiveInstanceId> {
        let index = self.next_resident_instance;
        self.next_resident_instance =
            self.next_resident_instance.checked_add(1).ok_or_else(|| {
                route_failure(
                    ResidentRouteFailureClass::ActivationFailure,
                    "resident instance identity exhausted",
                )
            })?;
        Ok(ReactiveInstanceId::new(index, 0))
    }

    fn preflight_provider_contract_presence(&self, artifact: &ProgramArtifact) -> MResult<()> {
        for (_, requirement) in artifact.requirements().iter() {
            let ApplicationRequirement::Resource(request) = requirement else {
                return Err(super::unsupported_route(
                    "resident host-function requirements are unsupported",
                ));
            };
            let binding = self
                .resources
                .resident_provider_binding(&request.base_uri)
                .map_err(classify_provider_preflight)?;
            let contract = match request.intent {
                ResourceIntent::Read => binding
                    .semantic_read_contract()
                    .map_err(classify_provider_preflight)?,
                ResourceIntent::Assign => binding
                    .semantic_write_contract(RuntimeResourceWriteIntent::Assign)
                    .map_err(classify_provider_preflight)?,
                ResourceIntent::Send => binding
                    .semantic_write_contract(RuntimeResourceWriteIntent::Send)
                    .map_err(classify_provider_preflight)?,
            };
            if contract.is_none() {
                return Err(route_failure(
                    ResidentRouteFailureClass::ProviderContractMismatch,
                    format!(
                        "provider for `{}` has no semantic contract for {:?}",
                        request.base_uri, request.intent,
                    ),
                ));
            }
        }
        Ok(())
    }
}

fn classify_provider_preflight(error: MechError) -> MechError {
    let kind = error.kind_name();
    let class = match kind.as_str() {
        "ResidentProviderContractMismatch" => ResidentRouteFailureClass::ProviderContractMismatch,
        "UnsupportedResidentExternalRequirement" => ResidentRouteFailureClass::SemanticUnsupported,
        "ResidentExternalBindingInvalid" => ResidentRouteFailureClass::InvalidArtifact,
        _ => ResidentRouteFailureClass::ProviderUnavailable,
    };
    route_failure(
        class,
        format!("resident provider preflight failed: {error:?}"),
    )
}
