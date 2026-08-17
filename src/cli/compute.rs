use mech_compute::{BackendRequest, ComputePlatform};
use mech_core::{MResult, MechError, MechErrorKind};
use mech_engine::ProgramArtifact;
use mech_gpu::{
    ComputeHostFactory, lower_elementwise_compute_program, native_compute_backend_registry,
};
use mech_runtime::{FileSourceResolver, RuntimeHostFactory, SourceRequest};

use crate::cli::run::{cli_module_options, cli_runtime_builder};
use crate::cli::runtime_plan::RunExecutionPlan;

pub(crate) struct CompiledNativeComputeApplication {
    pub(crate) coordinator: ProgramArtifact,
    pub(crate) factory: Box<dyn RuntimeHostFactory>,
}

pub(crate) fn configured_compute_host(plan: &RunExecutionPlan) -> MResult<Option<&str>> {
    let mut instances = plan
        .configured_hosts
        .iter()
        .filter(|host| host.provider == "compute");
    let first = instances.next();
    if let Some(second) = instances.next() {
        return Err(compute_cli_error(
            "configure_compute_host",
            format!(
                "v0.4 supports one configured compute host; found `{}` and `{}`",
                first.expect("a second instance implies a first").name,
                second.name,
            ),
        ));
    }
    if let Some(host) = plan
        .configured_hosts
        .iter()
        .find(|host| host.provider == "gpu")
    {
        return Err(compute_cli_error(
            "configure_compute_host",
            format!(
                "host `{}` uses removed provider `gpu`; use provider `compute` and scheme `compute://`",
                host.name
            ),
        ));
    }
    Ok(first.map(|host| host.name.as_str()))
}

pub(crate) fn compile_inline_compute_application(
    plan: &RunExecutionPlan,
    source: &str,
    resolver: FileSourceResolver,
) -> MResult<CompiledNativeComputeApplication> {
    let tree = mech_syntax::parse(source.trim())?;
    compile_compute_application(plan, resolver, |compiler| {
        compiler.compile_mixed_tree(&tree)
    })
}

pub(crate) fn compile_root_compute_application(
    plan: &RunExecutionPlan,
    request: SourceRequest,
    resolver: FileSourceResolver,
) -> MResult<CompiledNativeComputeApplication> {
    compile_compute_application(plan, resolver, |compiler| {
        compiler.compile_mixed_root(request, cli_module_options())
    })
}

fn compile_compute_application(
    plan: &RunExecutionPlan,
    resolver: FileSourceResolver,
    compile: impl FnOnce(
        &mut mech_runtime::ProgramCompiler,
    ) -> MResult<mech_runtime::MixedProgramCompilation>,
) -> MResult<CompiledNativeComputeApplication> {
    configured_compute_host(plan)?.ok_or_else(|| {
        compute_cli_error(
            "compile_compute_application",
            "the application has no configured compute host",
        )
    })?;
    let ordinary_hosts = plan
        .configured_hosts
        .iter()
        .filter(|host| host.provider != "compute")
        .cloned()
        .collect::<Vec<_>>();
    let mut compiler = cli_runtime_builder(
        plan.runtime_config.clone(),
        &plan.cli_grants,
        &ordinary_hosts,
        &plan.configured_run_grants,
        Vec::new(),
    )?
    .source_resolver(resolver)
    .build_compiler()?;
    let mixed = compile(&mut compiler)?;

    let program = lower_elementwise_compute_program(&mixed.compute.artifact).map_err(|error| {
        compute_cli_error(
            "lower_compute_region",
            format!(
                "region `{}` is not supported by the native compute backends:\n{error}",
                mixed.compute.declaration.name
            ),
        )
    })?;
    let mut factory = ComputeHostFactory::new(
        mixed.compute.declaration.name.clone(),
        mixed.compute.declaration.placement,
        program,
        mixed.compute.initializers,
        native_compute_backend_registry(),
        ComputePlatform::Native,
    )?;
    if let Some(request) = plan.backend_override.as_deref() {
        factory =
            factory.with_backend_override(BackendRequest::parse(request).map_err(|error| {
                compute_cli_error(
                    "select_compute_backend",
                    format!("invalid backend override `{request}`: {error}"),
                )
            })?);
    }

    Ok(CompiledNativeComputeApplication {
        coordinator: mixed.coordinator.into_artifact(),
        factory: Box::new(factory),
    })
}

#[derive(Clone, Debug)]
struct NativeComputeCliError {
    operation: &'static str,
    reason: String,
}

impl MechErrorKind for NativeComputeCliError {
    fn name(&self) -> &str {
        "NativeComputeCliError"
    }

    fn message(&self) -> String {
        format!("{} failed: {}", self.operation, self.reason)
    }
}

fn compute_cli_error(operation: &'static str, reason: impl Into<String>) -> MechError {
    MechError::new(
        NativeComputeCliError {
            operation,
            reason: reason.into(),
        },
        None,
    )
    .with_compiler_loc()
}
