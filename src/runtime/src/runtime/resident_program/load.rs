use std::sync::Arc;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use std::time::Instant;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use web_time::Instant;

#[cfg(feature = "resident-routing-source")]
use mech_core::MechSourceCode;
use mech_core::{
    ApplicationRequirement, ExecutionHostFunctionRequest, ExecutionResourceRequest, LegacyValue,
    MResult, MechError, MechExecutionServices, ParsedProgram, ReactiveInstanceId, Ref,
    ResourceIntent,
};
use mech_engine::{
    ProgramArtifact, decode_program_artifact_sections,
    resident::{
        ActivationFacts, ResidentActivationOptions, ResidentExternalAdmission,
        ResidentIntegrityMode, activate_with_options, preflight_activation,
    },
};

#[cfg(feature = "resident-routing-source")]
use crate::{ModuleBuildOptions, ResidentExternalContractResolver, SourceRequest};
use crate::{
    ResidentExternalCoordinator, ResidentExternalLimits, ResidentRoutingPolicy,
    RuntimeCapabilityOperation, RuntimeResourceKey, RuntimeResourceReadRequest,
    RuntimeResourceRegistry, RuntimeResourceWriteIntent, RuntimeResourceWriteRequest,
    RuntimeValueSnapshot, runtime::MechRuntime,
};
#[cfg(feature = "resident-routing-source")]
use mech_engine::{MechProgramConfig, MechProgramEnvironment, ProgramCompilationProduct};

use super::admission::activation_failure_for_artifact;
use super::execution::initial_value;
use super::{
    ActiveProgramExecution, ResidentExternalExecution, ResidentPureExecution,
    ResidentRouteFailureClass, RuntimeProgramExecutionInfo, RuntimeProgramLoadOptions,
    RuntimeProgramLoadOutcome, RuntimeProgramRoute, invalid_active_program, route_failure,
};

impl MechRuntime {
    #[cfg(feature = "resident-routing-source")]
    pub fn load_source_program(
        &mut self,
        source: &str,
        options: RuntimeProgramLoadOptions,
    ) -> MResult<RuntimeProgramLoadOutcome> {
        self.enforce_source_byte_limit(u64::try_from(source.len()).unwrap_or(u64::MAX))?;
        self.load_with_selection(
            options,
            |runtime| {
                Ok(Arc::new(
                    runtime.plan_source_product(source)?.into_parts().0,
                ))
            },
            |runtime| runtime.run_string(source),
        )
    }

    #[cfg(feature = "resident-routing-source")]
    pub fn load_root_program(
        &mut self,
        request: SourceRequest,
        module_options: ModuleBuildOptions<'_>,
        options: RuntimeProgramLoadOptions,
    ) -> MResult<RuntimeProgramLoadOutcome> {
        let resident_request = request.clone();
        self.load_with_selection(
            options,
            |runtime| {
                let resolved = runtime
                    .resolve_source(resident_request.clone())?
                    .ok_or_else(|| {
                        route_failure(
                            ResidentRouteFailureClass::InvalidArtifact,
                            format!("root source `{}` was not found", resident_request.specifier),
                        )
                    })?;
                match &resolved.source {
                    MechSourceCode::String(source) => {
                        runtime.enforce_source_byte_limit(
                            u64::try_from(source.len()).unwrap_or(u64::MAX),
                        )?;
                        Ok(Arc::new(
                            runtime
                                .plan_root_source_product(resident_request.clone(), module_options)?
                                .into_parts()
                                .0,
                        ))
                    }
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
            },
            |runtime| runtime.resolve_and_run_root_module(request, module_options),
        )
    }

    #[cfg(feature = "resident-routing")]
    pub fn load_bytecode_program(
        &mut self,
        bytecode: &[u8],
        options: RuntimeProgramLoadOptions,
    ) -> MResult<RuntimeProgramLoadOutcome> {
        self.enforce_source_byte_limit(u64::try_from(bytecode.len()).unwrap_or(u64::MAX))?;
        self.load_with_selection(
            options,
            |runtime| runtime.decode_artifact(bytecode),
            |runtime| {
                let mut context = runtime.runtime_context()?;
                runtime.evaluate_bytecode_once_with_context(&mut context, bytecode)
            },
        )
    }

    fn load_with_selection<Resident, Legacy>(
        &mut self,
        options: RuntimeProgramLoadOptions,
        resident: Resident,
        legacy: Legacy,
    ) -> MResult<RuntimeProgramLoadOutcome>
    where
        Resident: FnOnce(&mut Self) -> MResult<Arc<ProgramArtifact>>,
        Legacy: FnOnce(&mut Self) -> MResult<RuntimeValueSnapshot>,
    {
        self.ensure_program_slot_available()?;
        super::ensure_supported_durability(options.durability)?;
        if options.routing == ResidentRoutingPolicy::LegacyOnly {
            let value = legacy(self)?;
            return self.install_legacy_outcome(value, options.routing);
        }

        let resident_outcome =
            resident(self).and_then(|artifact| self.install_resident_artifact(artifact, options));
        match resident_outcome {
            Ok(outcome) => Ok(outcome),
            Err(error)
                if options.routing == ResidentRoutingPolicy::PreferResident
                    && super::resident_fallback_eligible(&error) =>
            {
                let value = legacy(self)?;
                self.install_legacy_outcome(value, options.routing)
            }
            Err(error) => Err(error),
        }
    }

    #[cfg(feature = "resident-routing-source")]
    pub fn compile_root_program_bytecode(&mut self, request: SourceRequest) -> MResult<Vec<u8>> {
        let options =
            ModuleBuildOptions::new(env!("CARGO_PKG_VERSION"), "v0.3", "native", &[], &[]);
        Ok(self
            .plan_root_source_product(request, options)?
            .into_parts()
            .1)
    }

    #[cfg(feature = "resident-routing-source")]
    fn plan_root_source_product(
        &mut self,
        request: SourceRequest,
        module_options: ModuleBuildOptions<'_>,
    ) -> MResult<ProgramCompilationProduct> {
        let previous_mode = self.execution_mode;
        self.execution_mode = crate::runtime::RuntimeExecutionMode::Plan;
        let result = (|| {
            self.resolve_and_run_root_module(request, module_options)?;
            let resolver = ResidentExternalContractResolver::new(&self.resources);
            self.program
                .compile_program_product_with_external_contracts(&resolver)
                .map_err(|error| {
                    route_failure(
                        ResidentRouteFailureClass::InvalidArtifact,
                        format!("resident root ProgramArtifact finalization failed: {error:?}"),
                    )
                })
        })();
        self.execution_mode = previous_mode;

        // Module planning intentionally uses the real import/environment path,
        // but the resulting interpreter is only a compiler workspace. Resident
        // ownership starts from the artifact, so do not retain a second legacy
        // executable after the closure has been compiled.
        self.program = self.new_program(MechProgramConfig {
            name: self.config.name.clone(),
            environment: MechProgramEnvironment {
                trace_enabled: self.config.diagnostics.trace_enabled,
                debug_enabled: self.config.diagnostics.debug_enabled,
                profile_enabled: self.config.diagnostics.profile_enabled,
                rounds_per_step: self.config.limits.max_steps_per_turn_as_usize()?,
            },
        });
        self.live_input_bindings.clear();
        self.live_context_template = None;
        result
    }

    #[cfg(feature = "resident-routing-source")]
    fn plan_source_product(&mut self, source: &str) -> MResult<ProgramCompilationProduct> {
        let mut program = self.new_program(MechProgramConfig {
            name: self.config.name.clone(),
            environment: MechProgramEnvironment {
                trace_enabled: self.config.diagnostics.trace_enabled,
                debug_enabled: self.config.diagnostics.debug_enabled,
                profile_enabled: self.config.diagnostics.profile_enabled,
                rounds_per_step: self.config.limits.max_steps_per_turn_as_usize()?,
            },
        });
        let mut services = ResidentSourcePlanningServices {
            providers: &self.resources,
        };
        program
            .run_string_with_services(source, &mut services)
            .map_err(classify_source_planning)?;
        let resolver = ResidentExternalContractResolver::new(&self.resources);
        program
            .compile_program_product_with_external_contracts(&resolver)
            .map_err(|error| {
                route_failure(
                    ResidentRouteFailureClass::InvalidArtifact,
                    format!("resident ProgramArtifact finalization failed: {error:?}"),
                )
            })
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
        options: RuntimeProgramLoadOptions,
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
        // not semantic-admission failures. Check them first so an unrelated
        // legacy-opaque node cannot make a missing or incompatible provider
        // eligible for preferred legacy fallback.
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
            policy: options.routing,
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
                options.durability,
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

    fn install_legacy_outcome(
        &mut self,
        value: RuntimeValueSnapshot,
        policy: ResidentRoutingPolicy,
    ) -> MResult<RuntimeProgramLoadOutcome> {
        let info = RuntimeProgramExecutionInfo {
            route: RuntimeProgramRoute::Legacy,
            policy,
            legacy_turns: 1,
            ..RuntimeProgramExecutionInfo::default()
        };
        self.active_program = ActiveProgramExecution::Legacy;
        self.program_execution_info = info.clone();
        Ok(RuntimeProgramLoadOutcome {
            route: RuntimeProgramRoute::Legacy,
            initial_value: value,
            info,
        })
    }

    pub(super) fn ensure_program_slot_available(&self) -> MResult<()> {
        if !self.active_transactions.is_empty() {
            return Err(invalid_active_program(
                "a runtime transaction is active; finish it before loading a program",
            ));
        }
        if matches!(self.active_program, ActiveProgramExecution::None)
            && !self.program.interpreter().has_retained_execution_state()
            && self.live_input_bindings.is_empty()
            && self.live_context_template.is_none()
        {
            Ok(())
        } else {
            Err(invalid_active_program(
                "one runtime may own only one resident or retained legacy program; unload it first",
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

#[cfg(feature = "resident-routing-source")]
struct ResidentSourcePlanningServices<'a> {
    providers: &'a RuntimeResourceRegistry,
}

#[cfg(feature = "resident-routing-source")]
impl MechExecutionServices for ResidentSourcePlanningServices<'_> {
    fn invoke_host_function(
        &mut self,
        request: &ExecutionHostFunctionRequest,
        _arguments: &[LegacyValue],
    ) -> MResult<LegacyValue> {
        Err(super::unsupported_route(format!(
            "resident source requires host function `{}`",
            request.name,
        )))
    }

    fn plan_resource_read_output(
        &mut self,
        request: &ExecutionResourceRequest,
    ) -> MResult<LegacyValue> {
        self.plan_read(request)
    }

    fn read_resource(&mut self, request: &ExecutionResourceRequest) -> MResult<LegacyValue> {
        self.plan_read(request)
    }

    fn write_resource(
        &mut self,
        request: &ExecutionResourceRequest,
        value: &LegacyValue,
    ) -> MResult<()> {
        let key = RuntimeResourceKey::new(&request.base_uri, &request.path)?;
        let intent = match request.intent {
            ResourceIntent::Assign => RuntimeResourceWriteIntent::Assign,
            ResourceIntent::Send => RuntimeResourceWriteIntent::Send,
            ResourceIntent::Read => {
                return Err(route_failure(
                    ResidentRouteFailureClass::InvalidArtifact,
                    "resident source attempted a read through the write planning path",
                ));
            }
        };
        self.providers.plan_write(RuntimeResourceWriteRequest {
            base_uri: key.base_uri,
            path: key.path,
            context_name: request.context_name.clone(),
            operation: RuntimeCapabilityOperation::from_name(request.operation.clone())?,
            value: value.try_deep_snapshot()?,
            intent,
        })
    }

    fn bind_live_resource(
        &mut self,
        _interpreter_id: u64,
        _request: &ExecutionResourceRequest,
        _target: Ref<LegacyValue>,
    ) -> MResult<()> {
        Ok(())
    }
}

#[cfg(feature = "resident-routing-source")]
impl ResidentSourcePlanningServices<'_> {
    fn plan_read(&self, request: &ExecutionResourceRequest) -> MResult<LegacyValue> {
        let key = RuntimeResourceKey::new(&request.base_uri, &request.path)?;
        self.providers.plan_read(RuntimeResourceReadRequest {
            base_uri: key.base_uri,
            path: key.path,
            context_name: request.context_name.clone(),
        })
    }
}

#[cfg(feature = "resident-routing-source")]
fn classify_source_planning(error: MechError) -> MechError {
    if error.kind_name() == "ResidentRouteFailure" {
        return error;
    }
    let class = match error.kind_name().as_str() {
        "RuntimeResourceProviderNotFound" => ResidentRouteFailureClass::ProviderUnavailable,
        _ => ResidentRouteFailureClass::InvalidArtifact,
    };
    route_failure(class, format!("resident source planning failed: {error:?}"))
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
