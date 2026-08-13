use std::sync::Arc;

#[cfg(feature = "resident-production-source")]
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

#[cfg(feature = "resident-production-source")]
use crate::{ModuleBuildOptions, ResidentExternalContractResolver, SourceRequest};
use crate::{
    ResidentExternalCoordinator, ResidentExternalLimits, ResidentRoutingPolicy,
    RuntimeCapabilityOperation, RuntimeResourceKey, RuntimeResourceReadRequest,
    RuntimeResourceRegistry, RuntimeResourceWriteIntent, RuntimeResourceWriteRequest,
    RuntimeValueSnapshot, runtime::MechRuntime,
};
#[cfg(feature = "resident-production-source")]
use mech_engine::{MechProgramConfig, MechProgramEnvironment, ProgramCompilationProduct};

use super::admission::{activation_failure, fallback_eligible};
use super::execution::initial_value;
use super::{
    ActiveProgramExecution, ResidentExternalExecution, ResidentPureExecution,
    ResidentRouteFailureClass, RuntimeProgramExecutionInfo, RuntimeProgramLoadOptions,
    RuntimeProgramLoadOutcome, RuntimeProgramRoute, invalid_active_program, route_failure,
};

impl MechRuntime {
    #[cfg(feature = "resident-production-source")]
    pub fn load_source_program(
        &mut self,
        source: &str,
        options: RuntimeProgramLoadOptions,
    ) -> MResult<RuntimeProgramLoadOutcome> {
        self.ensure_program_slot_available()?;
        super::ensure_supported_durability(options.durability)?;
        if options.routing == ResidentRoutingPolicy::LegacyOnly {
            let value = self.run_string(source)?;
            return self.install_legacy_outcome(value, options.routing);
        }

        let artifact = Arc::new(self.plan_source_product(source)?.into_parts().0);
        match self.install_resident_artifact(artifact, options) {
            Ok(outcome) => Ok(outcome),
            Err(error)
                if options.routing == ResidentRoutingPolicy::PreferResident
                    && fallback_eligible(&error) =>
            {
                let value = self.run_string(source)?;
                self.install_legacy_outcome(value, options.routing)
            }
            Err(error) => Err(error),
        }
    }

    #[cfg(feature = "resident-production-source")]
    pub fn load_root_program(
        &mut self,
        request: SourceRequest,
        module_options: ModuleBuildOptions<'_>,
        options: RuntimeProgramLoadOptions,
    ) -> MResult<RuntimeProgramLoadOutcome> {
        self.ensure_program_slot_available()?;
        if options.routing == ResidentRoutingPolicy::LegacyOnly {
            let value = self.resolve_and_run_root_module(request, module_options)?;
            return self.install_legacy_outcome(value, options.routing);
        }
        let resolved = self.resolve_source(request.clone())?.ok_or_else(|| {
            route_failure(
                ResidentRouteFailureClass::InvalidArtifact,
                format!("root source `{}` was not found", request.specifier),
            )
        })?;
        let artifact = match &resolved.source {
            MechSourceCode::String(source) => {
                Arc::new(self.plan_source_product(source)?.into_parts().0)
            }
            MechSourceCode::ByteCode(bytecode) => self.decode_artifact(bytecode)?,
            _ => {
                if options.routing == ResidentRoutingPolicy::PreferResident {
                    let value = self.resolve_and_run_root_module(request, module_options)?;
                    return self.install_legacy_outcome(value, options.routing);
                }
                return Err(route_failure(
                    ResidentRouteFailureClass::SemanticUnsupported,
                    "root source kind is not resident-routable",
                ));
            }
        };
        match self.install_resident_artifact(artifact, options) {
            Ok(outcome) => Ok(outcome),
            Err(error)
                if options.routing == ResidentRoutingPolicy::PreferResident
                    && fallback_eligible(&error) =>
            {
                let value = self.resolve_and_run_root_module(request, module_options)?;
                self.install_legacy_outcome(value, options.routing)
            }
            Err(error) => Err(error),
        }
    }

    #[cfg(feature = "resident-production")]
    pub fn load_bytecode_program(
        &mut self,
        bytecode: &[u8],
        options: RuntimeProgramLoadOptions,
    ) -> MResult<RuntimeProgramLoadOutcome> {
        self.ensure_program_slot_available()?;
        super::ensure_supported_durability(options.durability)?;
        if options.routing == ResidentRoutingPolicy::LegacyOnly {
            let mut context = self.runtime_context()?;
            let value = self.evaluate_bytecode_once_with_context(&mut context, bytecode)?;
            return self.install_legacy_outcome(value, options.routing);
        }
        let artifact = self.decode_artifact(bytecode)?;
        match self.install_resident_artifact(artifact, options) {
            Ok(outcome) => Ok(outcome),
            Err(error)
                if options.routing == ResidentRoutingPolicy::PreferResident
                    && fallback_eligible(&error) =>
            {
                let mut context = self.runtime_context()?;
                let value = self.evaluate_bytecode_once_with_context(&mut context, bytecode)?;
                self.install_legacy_outcome(value, options.routing)
            }
            Err(error) => Err(error),
        }
    }

    #[cfg(feature = "resident-production-source")]
    pub fn compile_root_program_bytecode(&mut self, request: SourceRequest) -> MResult<Vec<u8>> {
        let resolved = self.resolve_source(request.clone())?.ok_or_else(|| {
            route_failure(
                ResidentRouteFailureClass::InvalidArtifact,
                format!("root source `{}` was not found", request.specifier),
            )
        })?;
        let MechSourceCode::String(source) = &resolved.source else {
            return Err(route_failure(
                ResidentRouteFailureClass::SemanticUnsupported,
                "root source kind cannot be compiled through resident source planning",
            ));
        };
        Ok(self.plan_source_product(source)?.into_parts().1)
    }

    #[cfg(feature = "resident-production-source")]
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

    #[cfg(feature = "resident-production")]
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

    #[cfg(feature = "resident-production")]
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

        let preflight = preflight_activation(
            &artifact,
            &self.function_catalog,
            &ActivationFacts::default(),
            activation_options,
        )
        .map_err(activation_failure)?;
        if !external && !preflight.plan.inputs.is_empty() {
            return Err(super::unsupported_route(
                "pure production resident programs cannot require turn inputs",
            ));
        }

        if external {
            self.preflight_provider_contract_presence(&artifact)?;
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
        .map_err(activation_failure)?;

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
            initial_snapshot = initial_value(&instance)?;
            let authority = authority.expect("external authority was built");
            let coordinator = ResidentExternalCoordinator::new_live(
                instance,
                Arc::clone(&artifact),
                &self.resources,
                &authority,
                options.durability,
                ResidentExternalLimits::default(),
            )?;
            let trigger_sources = coordinator.trigger_sources()?;
            ActiveProgramExecution::ResidentExternal(ResidentExternalExecution {
                artifact,
                coordinator,
                trigger_sources,
            })
        } else {
            instance.turn(&[]).map_err(|error| {
                route_failure(
                    ResidentRouteFailureClass::ActivationFailure,
                    format!("initial pure resident turn failed: {error:?}"),
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
        if matches!(self.active_program, ActiveProgramExecution::None) {
            Ok(())
        } else {
            Err(invalid_active_program(
                "one runtime may own only one active program; unload it first",
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
                ResourceIntent::Read => self
                    .resources
                    .resident_semantic_read_contract(binding)
                    .map_err(classify_provider_preflight)?,
                ResourceIntent::Assign => self
                    .resources
                    .resident_semantic_write_contract(binding, RuntimeResourceWriteIntent::Assign)
                    .map_err(classify_provider_preflight)?,
                ResourceIntent::Send => self
                    .resources
                    .resident_semantic_write_contract(binding, RuntimeResourceWriteIntent::Send)
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

#[cfg(feature = "resident-production-source")]
struct ResidentSourcePlanningServices<'a> {
    providers: &'a RuntimeResourceRegistry,
}

#[cfg(feature = "resident-production-source")]
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

#[cfg(feature = "resident-production-source")]
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

#[cfg(feature = "resident-production-source")]
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
