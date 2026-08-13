//! Deterministic native application planning and generation for Mech.

mod analysis;
pub mod cargo;
pub mod dependency;
pub mod error;
pub mod host;
pub mod plan;
pub mod project;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::sync::Arc;

use mech_core::{FunctionCatalog, MResult, ParsedProgram};

pub use analysis::requirements::normalize_native_runtime_config;
pub use cargo::*;
pub use dependency::*;
pub use host::*;
pub use plan::*;
pub use project::*;

/// Exact version used by every component package in a generated registry-mode
/// native application. The root CLI crate may have a different release
/// cadence, so it is deliberately never consulted here.
pub const MECH_COMPONENT_VERSION: &str = env!("CARGO_PKG_VERSION");

const NATIVE_REGISTRY_RESOLUTION_SEED: &[u8] = include_bytes!("../native-resolution-seed.lock");

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
        if let Some(config) = request.runtime_config.as_ref() {
            config.runtime.validate_production_program_routing()?;
        }

        let program = ParsedProgram::from_bytes(&request.bytecode)?;
        plan::validate_target_index_constants(&program, request.target.as_deref())?;
        let mut native_resolver = analysis::NativeBytecodeContractResolver::new(
            &program.requirements,
            request.runtime_config.as_ref(),
            &self.environment.host_catalog,
            request.target.as_deref(),
        )?;
        program.validate_runtime_contracts_with(
            &self.environment.function_catalog,
            &mut native_resolver,
        )?;
        let runtime_functions =
            analysis::analyze_runtime_functions(&program, &self.environment.function_catalog)?;
        for function in &runtime_functions {
            plan::validate_installer_path(&function.installer_path)?;
        }
        let referenced_runtime_types = program.referenced_runtime_types()?;
        let runtime_types = analysis::analyze_runtime_types(&referenced_runtime_types)?;
        let requirements = native_resolver.finish()?;
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
        // `mech-engine` retains dynamic row-vector construction through its
        // matrix-assignment implementation. A program that returns only a
        // `RowVectorD` therefore still needs this engine-internal closure;
        // keep it engine-local so the core and runtime type selections remain
        // the exact bytecode-derived set.
        if engine_features.contains("row_vectord") {
            engine_features.insert("bool".to_owned());
            engine_features.insert("vectord".to_owned());
        }
        engine_features.insert("runtime".to_owned());
        let mut runtime_features = BTreeSet::new();
        if application_kind == NativeApplicationKind::Hosted {
            runtime_features.extend(runtime_types.cargo_features.iter().cloned());
            runtime_features.insert("runtime".to_owned());
            runtime_features.insert("string".to_owned());
            runtime_features.insert("resident-routing".to_owned());
            // Hosted execution owns the transaction boundary, so it must
            // enable the validation hook whenever the bytecode carries an
            // integrity-constraint marker. Engine-only applications enforce
            // the same contract directly in `MechProgram`.
            if runtime_functions
                .iter()
                .any(|function| function.runtime_name == "integrity/constraint")
            {
                runtime_features.insert("invariant_define".to_owned());
            }
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
        let dependency_lock_seed = self.dependency_lock_seed()?;
        let dependency_resolution_seed_sha256 = plan::sha256_hex(&dependency_lock_seed);
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
            actor_bootstrap: requirements.actor_bootstrap,
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
            dependency_resolution_seed_sha256,
            workspace_fingerprint,
        };
        refresh_plan_sha256(&mut plan)?;
        Ok(plan)
    }

    pub fn generate(
        &self,
        request: &NativeBuildRequest,
        plan: &NativeBuildPlan,
    ) -> MResult<GeneratedNativeProject> {
        let workspace_root = self.trusted_workspace_root()?;
        let cache_root = match workspace_root.as_ref() {
            Some(root) => root.clone(),
            None => project::normalized_absolute_project_root(&std::env::current_dir().map_err(
                |error| {
                    error::native_build_error(
                        error::NativeBuildErrorKind::NativeProjectInvalid {
                            reason: format!("failed to resolve the native project root: {error}"),
                        },
                        None,
                    )
                },
            )?)?,
        };
        let root = project::generated_project_root(&cache_root, &plan.plan_sha256)?;
        self.generate_internal(request, plan, root, workspace_root.as_deref())
    }

    pub fn generate_at(
        &self,
        request: &NativeBuildRequest,
        plan: &NativeBuildPlan,
        project_root: impl AsRef<Path>,
    ) -> MResult<GeneratedNativeProject> {
        let project_root = project::normalized_absolute_project_root(project_root.as_ref())?;
        let workspace_root = self.trusted_workspace_root()?;
        self.generate_internal(request, plan, project_root, workspace_root.as_deref())
    }

    fn generate_internal(
        &self,
        request: &NativeBuildRequest,
        plan: &NativeBuildPlan,
        root: impl Into<std::path::PathBuf>,
        workspace_root: Option<&Path>,
    ) -> MResult<GeneratedNativeProject> {
        let expected = self.plan(request)?;
        if &expected != plan {
            return Err(error::native_build_error(
                error::NativeBuildErrorKind::NativeProjectInvalid {
                    reason: "native build request and plan do not describe the same application"
                        .to_owned(),
                },
                None,
            ));
        }

        let project =
            project::render_generated_native_project(root, request, plan, workspace_root)?;
        project.materialize()?;
        let dependency_lock_seed = self.dependency_lock_seed()?;
        if plan::sha256_hex(&dependency_lock_seed) != plan.dependency_resolution_seed_sha256 {
            return Err(error::native_build_error(
                error::NativeBuildErrorKind::NativeProjectInvalid {
                    reason: "native dependency lock seed does not match the build plan".to_owned(),
                },
                None,
            ));
        }
        cargo::generate_project_lockfile(&project, &dependency_lock_seed, request.offline)?;
        Ok(project)
    }

    fn dependency_lock_seed(&self) -> MResult<Vec<u8>> {
        match &self.environment.dependency_source {
            NativeDependencySource::Registry { .. } => Ok(NATIVE_REGISTRY_RESOLUTION_SEED.to_vec()),
            NativeDependencySource::Workspace { root } => {
                let lockfile = root.join("Cargo.lock");
                fs::read(&lockfile).map_err(|error| {
                    error::native_build_error(
                        error::NativeBuildErrorKind::NativeWorkspaceInputInvalid {
                            path: lockfile,
                            reason: format!("workspace dependency lock cannot be read: {error}"),
                        },
                        None,
                    )
                })
            }
        }
    }

    fn trusted_workspace_root(&self) -> MResult<Option<std::path::PathBuf>> {
        match &self.environment.dependency_source {
            NativeDependencySource::Workspace { root } => {
                project::normalized_absolute_project_root(root).map(Some)
            }
            NativeDependencySource::Registry { .. } => Ok(None),
        }
    }

    pub fn build(
        &self,
        request: &NativeBuildRequest,
        plan: &NativeBuildPlan,
    ) -> MResult<NativeBuildArtifact> {
        let project = self.generate(request, plan)?;

        let workspace_root = match &self.environment.dependency_source {
            NativeDependencySource::Workspace { root } => root.clone(),
            NativeDependencySource::Registry { .. } => {
                std::env::current_dir().map_err(|error| {
                    error::native_build_error(
                        error::NativeBuildErrorKind::NativeCargoFailed {
                            reason: format!(
                                "failed to resolve the shared Cargo target root: {error}"
                            ),
                        },
                        None,
                    )
                })?
            }
        };
        cargo::build_native_project(
            &project,
            &plan.binary_name,
            plan.target.as_deref(),
            &project::shared_cargo_target_root(&workspace_root),
            plan.profile == NativeBuildProfile::Release,
            request.offline,
        )
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
            if version != MECH_COMPONENT_VERSION {
                return Err(error::native_build_error(
                    error::NativeBuildErrorKind::NativeComponentVersionMismatch {
                        package: "registry selection".to_owned(),
                        expected: MECH_COMPONENT_VERSION.to_owned(),
                        actual: version.clone(),
                    },
                    None,
                ));
            }
            let packages = drafts
                .into_iter()
                .map(|(package, draft)| PlannedPackage {
                    package,
                    crate_name: draft.crate_name,
                    source: PlannedPackageSource::Registry {
                        version: MECH_COMPONENT_VERSION.to_owned(),
                    },
                    cargo_features: draft.features.into_iter().collect(),
                })
                .collect();
            Ok((
                packages,
                PlannedDependencySource::Registry {
                    version: MECH_COMPONENT_VERSION.to_owned(),
                },
                None,
            ))
        }
        NativeDependencySource::Workspace { root } => {
            let selected = dependency::resolve_planned_packages(root, drafts.keys())?;
            validate_component_versions(root, &selected)?;
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

fn validate_component_versions(
    root: &Path,
    packages: &[dependency::WorkspacePackage],
) -> MResult<()> {
    for package in packages {
        let manifest = root.join(&package.relative_path).join("Cargo.toml");
        let source = fs::read_to_string(&manifest).map_err(|error| {
            error::native_build_error(
                error::NativeBuildErrorKind::NativeWorkspaceInputInvalid {
                    path: manifest.clone(),
                    reason: format!("failed to read component manifest: {error}"),
                },
                None,
            )
        })?;
        let document = source.parse::<toml_edit::DocumentMut>().map_err(|error| {
            error::native_build_error(
                error::NativeBuildErrorKind::NativeWorkspaceInputInvalid {
                    path: manifest.clone(),
                    reason: format!("failed to parse component manifest: {error}"),
                },
                None,
            )
        })?;
        let actual = document["package"]["version"].as_str().ok_or_else(|| {
            error::native_build_error(
                error::NativeBuildErrorKind::NativeWorkspaceInputInvalid {
                    path: manifest.clone(),
                    reason: "component package manifest lacks a string package.version".to_owned(),
                },
                None,
            )
        })?;
        if actual != MECH_COMPONENT_VERSION {
            return Err(error::native_build_error(
                error::NativeBuildErrorKind::NativeComponentVersionMismatch {
                    package: package.package.clone(),
                    expected: MECH_COMPONENT_VERSION.to_owned(),
                    actual: actual.to_owned(),
                },
                None,
            ));
        }
    }
    Ok(())
}
