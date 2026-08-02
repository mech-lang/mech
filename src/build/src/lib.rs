//! Deterministic native application planning and generation for Mech.

mod analysis;
pub mod cargo;
pub mod dependency;
pub mod error;
pub mod host;
pub mod plan;
pub mod project;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use mech_core::{FunctionCatalog, MResult, ParsedProgram};

pub use cargo::*;
pub use dependency::*;
pub use host::*;
pub use plan::*;
pub use project::*;

#[derive(Clone)]
pub struct NativeBuildEnvironment {
    pub function_catalog: Arc<FunctionCatalog>,
    pub host_catalog: Arc<NativeHostCatalog>,
    pub dependency_source: NativeDependencySource,
}

pub struct NativeApplicationBuilder {
    environment: NativeBuildEnvironment,
}

impl NativeApplicationBuilder {
    pub fn new(environment: NativeBuildEnvironment) -> Self {
        Self { environment }
    }

    pub fn plan(&self, request: &NativeBuildRequest) -> MResult<NativeBuildPlan> {
        plan::validate_binary_name(&request.binary_name)?;
        if let Some(target) = request.target.as_deref() {
            plan::validate_target(target)?;
        }

        let program = ParsedProgram::from_bytes(&request.bytecode)?;
        let runtime_functions =
            analysis::analyze_runtime_functions(&program, &self.environment.function_catalog)?;
        for function in &runtime_functions {
            plan::validate_installer_path(&function.installer_path)?;
        }
        let runtime_types = analysis::analyze_runtime_types(&program.types)?;
        let requirements = analysis::analyze_application_requirements(
            &program.requirements,
            request.runtime_config.as_ref(),
            &self.environment.host_catalog,
            request.target.as_deref(),
        )?;
        let application_kind = if analysis::application_requires_hosting(&program.requirements)
            || request.runtime_config.is_some()
        {
            NativeApplicationKind::Hosted
        } else {
            NativeApplicationKind::Engine
        };

        let mut core_features = runtime_types
            .cargo_features
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        core_features.insert("program".to_owned());
        let mut engine_features = runtime_types
            .cargo_features
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        engine_features.insert("runtime".to_owned());
        let mut runtime_features = BTreeSet::new();
        if application_kind == NativeApplicationKind::Hosted {
            runtime_features.extend(runtime_types.cargo_features.iter().cloned());
            runtime_features.insert("runtime".to_owned());
            runtime_features.insert("string".to_owned());
        }

        let mut packages = BTreeMap::new();
        merge_package(
            &mut packages,
            "mech-core",
            "mech_core",
            core_features.iter().cloned(),
        )?;
        merge_package(
            &mut packages,
            "mech-engine",
            "mech_engine",
            engine_features.iter().cloned(),
        )?;

        for function in &runtime_functions {
            merge_package(
                &mut packages,
                &function.package,
                &function.crate_name,
                function.cargo_features.iter().cloned(),
            )?;
        }

        for requirement in &requirements.application_requirements {
            if let PlannedApplicationRequirement::HostFunction {
                package,
                crate_name,
                cargo_features,
                ..
            } = requirement
            {
                merge_package(
                    &mut packages,
                    package,
                    crate_name,
                    cargo_features.iter().cloned(),
                )?;
            }
        }

        for host in &requirements.hosts {
            merge_package(
                &mut packages,
                &host.package,
                &host.crate_name,
                host.cargo_features.iter().cloned(),
            )?;
        }

        if application_kind == NativeApplicationKind::Hosted {
            merge_package(
                &mut packages,
                "mech-runtime",
                "mech_runtime",
                runtime_features.iter().cloned(),
            )?;
        }

        core_features = packages
            .get("mech-core")
            .expect("base package was inserted")
            .features
            .clone();
        engine_features = packages
            .get("mech-engine")
            .expect("base package was inserted")
            .features
            .clone();
        if let Some(runtime) = packages.get("mech-runtime") {
            runtime_features = runtime.features.clone();
        }

        let (packages, dependency_source, workspace_fingerprint) =
            resolve_packages(&self.environment.dependency_source, packages)?;
        let mech_version = format!(
            "{}.{}.{}",
            program.header.mech_major, program.header.mech_minor, program.header.mech_patch
        );
        let mut plan = NativeBuildPlan {
            schema: NATIVE_BUILD_PLAN_SCHEMA.to_owned(),
            bytecode_version: program.header.version,
            mech_version,
            application_kind,
            runtime_config: requirements.runtime_config,
            bytecode_sha256: plan::sha256_hex(&request.bytecode),
            plan_sha256: String::new(),
            target: request.target.clone(),
            profile: request.profile,
            binary_name: request.binary_name.clone(),
            runtime_functions,
            runtime_types: runtime_types.runtime_types,
            application_requirements: requirements.application_requirements,
            packages,
            core_features: core_features.into_iter().collect(),
            engine_features: engine_features.into_iter().collect(),
            runtime_features: runtime_features.into_iter().collect(),
            hosts: requirements.hosts,
            run_grants: requirements.run_grants,
            live: requirements.live,
            dependency_source,
            workspace_fingerprint,
        };
        refresh_plan_sha256(&mut plan)?;
        Ok(plan)
    }

    pub fn generate(
        &self,
        _request: &NativeBuildRequest,
        _plan: &NativeBuildPlan,
    ) -> MResult<GeneratedNativeProject> {
        Err(error::native_build_error(
            error::NativeBuildErrorKind::NativeProjectInvalid {
                reason: "native project generation is not available before the Phase 1 vertical-slice commit"
                    .to_owned(),
            },
            None,
        ))
    }

    pub fn build(
        &self,
        _request: &NativeBuildRequest,
        _plan: &NativeBuildPlan,
    ) -> MResult<NativeBuildArtifact> {
        Err(error::native_build_error(
            error::NativeBuildErrorKind::NativeCargoFailed {
                reason: "native Cargo execution is not available before the Phase 1 vertical-slice commit"
                    .to_owned(),
            },
            None,
        ))
    }
}

#[derive(Clone, Debug)]
struct PackageDraft {
    crate_name: String,
    features: BTreeSet<String>,
}

fn merge_package(
    packages: &mut BTreeMap<String, PackageDraft>,
    package: &str,
    crate_name: &str,
    features: impl IntoIterator<Item = String>,
) -> MResult<()> {
    use error::{NativeBuildErrorKind, native_build_error};

    let entry = packages
        .entry(package.to_owned())
        .or_insert_with(|| PackageDraft {
            crate_name: crate_name.to_owned(),
            features: BTreeSet::new(),
        });
    if entry.crate_name != crate_name {
        return Err(native_build_error(
            NativeBuildErrorKind::NativeDependencyInvalid {
                reason: format!(
                    "package `{package}` was linked with conflicting crate names `{}` and `{crate_name}`",
                    entry.crate_name
                ),
            },
            None,
        ));
    }
    entry.features.extend(features);
    Ok(())
}

fn resolve_packages(
    source: &NativeDependencySource,
    drafts: BTreeMap<String, PackageDraft>,
) -> MResult<(Vec<PlannedPackage>, PlannedDependencySource, Option<String>)> {
    match source {
        NativeDependencySource::Registry { version } => {
            dependency::validate_exact_registry_version(version)?;
            let packages = drafts
                .into_iter()
                .map(|(package, draft)| PlannedPackage {
                    package,
                    crate_name: draft.crate_name,
                    source: PlannedPackageSource::Registry {
                        version: version.clone(),
                    },
                    cargo_features: draft.features.into_iter().collect(),
                })
                .collect();
            Ok((
                packages,
                PlannedDependencySource::Registry {
                    version: version.clone(),
                },
                None,
            ))
        }
        NativeDependencySource::Workspace { root } => {
            let selected = dependency::resolve_planned_packages(root, drafts.keys())?;
            let fingerprint = dependency::fingerprint_workspace(root, &selected)?.into_string();
            let selected = selected
                .into_iter()
                .map(|package| (package.package.clone(), package))
                .collect::<BTreeMap<_, _>>();
            let packages = drafts
                .into_iter()
                .map(|(package, draft)| {
                    let selected = selected
                        .get(&package)
                        .expect("trusted registry resolved every requested package");
                    PlannedPackage {
                        package,
                        crate_name: draft.crate_name,
                        source: PlannedPackageSource::Workspace {
                            path: selected.relative_path.clone(),
                        },
                        cargo_features: draft.features.into_iter().collect(),
                    }
                })
                .collect();
            Ok((
                packages,
                PlannedDependencySource::Workspace,
                Some(fingerprint),
            ))
        }
    }
}
