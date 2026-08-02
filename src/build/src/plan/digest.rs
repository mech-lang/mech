use mech_core::{MResult, RuntimeType};
use mech_runtime::{RunResourceGrantConfig, RuntimeConfig};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::error::{NativeBuildErrorKind, native_build_error};

use super::{
    NativeApplicationKind, NativeBuildPlan, NativeBuildProfile, PlannedApplicationRequirement,
    PlannedDependencySource, PlannedHostInstance, PlannedPackage, PlannedRuntimeFunction,
};

/// The complete, stable input to the v1 native-build plan digest.
///
/// Field order is intentionally identical to [`NativeBuildPlan`] after
/// removing `plan_sha256`. All collections have already been normalized by
/// the planner before this structure is serialized.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct NativeBuildPlanDigestInput {
    pub schema: String,
    pub bytecode_version: u16,
    pub mech_version: String,
    pub application_kind: NativeApplicationKind,
    pub runtime_config: RuntimeConfig,
    pub bytecode_sha256: String,
    pub target: Option<String>,
    pub profile: NativeBuildProfile,
    pub binary_name: String,
    pub runtime_functions: Vec<PlannedRuntimeFunction>,
    pub runtime_types: Vec<RuntimeType>,
    pub application_requirements: Vec<PlannedApplicationRequirement>,
    pub packages: Vec<PlannedPackage>,
    pub core_features: Vec<String>,
    pub engine_features: Vec<String>,
    pub runtime_features: Vec<String>,
    pub hosts: Vec<PlannedHostInstance>,
    pub run_grants: Vec<RunResourceGrantConfig>,
    pub live: bool,
    pub dependency_source: PlannedDependencySource,
    pub workspace_fingerprint: Option<String>,
}

impl From<&NativeBuildPlan> for NativeBuildPlanDigestInput {
    fn from(plan: &NativeBuildPlan) -> Self {
        Self {
            schema: plan.schema.clone(),
            bytecode_version: plan.bytecode_version,
            mech_version: plan.mech_version.clone(),
            application_kind: plan.application_kind,
            runtime_config: plan.runtime_config.clone(),
            bytecode_sha256: plan.bytecode_sha256.clone(),
            target: plan.target.clone(),
            profile: plan.profile,
            binary_name: plan.binary_name.clone(),
            runtime_functions: plan.runtime_functions.clone(),
            runtime_types: plan.runtime_types.clone(),
            application_requirements: plan.application_requirements.clone(),
            packages: plan.packages.clone(),
            core_features: plan.core_features.clone(),
            engine_features: plan.engine_features.clone(),
            runtime_features: plan.runtime_features.clone(),
            hosts: plan.hosts.clone(),
            run_grants: plan.run_grants.clone(),
            live: plan.live,
            dependency_source: plan.dependency_source.clone(),
            workspace_fingerprint: plan.workspace_fingerprint.clone(),
        }
    }
}

pub fn compute_plan_sha256(plan: &NativeBuildPlan) -> MResult<String> {
    let bytes = serde_json::to_vec(&NativeBuildPlanDigestInput::from(plan)).map_err(|error| {
        native_build_error(
            NativeBuildErrorKind::NativeProjectInvalid {
                reason: format!("failed to serialize native build plan digest input: {error}"),
            },
            None,
        )
    })?;
    Ok(sha256_hex(&bytes))
}

pub fn refresh_plan_sha256(plan: &mut NativeBuildPlan) -> MResult<()> {
    plan.plan_sha256 = compute_plan_sha256(plan)?;
    Ok(())
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use mech_core::BYTECODE_VERSION;
    use mech_runtime::ConfigValue;

    use super::*;

    fn empty_plan() -> NativeBuildPlan {
        NativeBuildPlan {
            schema: super::super::NATIVE_BUILD_PLAN_SCHEMA.to_owned(),
            bytecode_version: BYTECODE_VERSION,
            mech_version: "0.3.5".to_owned(),
            application_kind: NativeApplicationKind::Engine,
            runtime_config: RuntimeConfig::default(),
            bytecode_sha256: sha256_hex(b"bytecode"),
            plan_sha256: String::new(),
            target: None,
            profile: NativeBuildProfile::Debug,
            binary_name: "app".to_owned(),
            runtime_functions: Vec::new(),
            runtime_types: Vec::new(),
            application_requirements: Vec::new(),
            packages: Vec::new(),
            core_features: Vec::new(),
            engine_features: Vec::new(),
            runtime_features: Vec::new(),
            hosts: Vec::new(),
            run_grants: Vec::new(),
            live: false,
            dependency_source: PlannedDependencySource::Registry {
                version: "0.3.5".to_owned(),
            },
            workspace_fingerprint: None,
        }
    }

    #[test]
    fn digest_is_lowercase_hex_and_ignores_its_own_value() {
        let mut plan = empty_plan();
        let first = compute_plan_sha256(&plan).unwrap();
        assert_eq!(first.len(), 64);
        assert!(
            first
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        );

        plan.plan_sha256 = "not-an-input".to_owned();
        assert_eq!(compute_plan_sha256(&plan).unwrap(), first);
    }

    #[test]
    fn digest_changes_for_frozen_identity_fields() {
        let plan = empty_plan();
        assert_digest_changes(&plan, |changed| changed.bytecode_sha256.push('0'));
        assert_digest_changes(&plan, |changed| changed.mech_version = "0.3.6".to_owned());
        assert_digest_changes(&plan, |changed| {
            changed.runtime_config.limits.max_steps_per_turn = Some(123)
        });
        assert_digest_changes(&plan, |changed| {
            changed.target = Some("aarch64-unknown-linux-gnu".to_owned())
        });
        assert_digest_changes(&plan, |changed| {
            changed.profile = NativeBuildProfile::Release
        });
        assert_digest_changes(&plan, |changed| changed.binary_name = "other".to_owned());

        let mut with_function = plan.clone();
        with_function
            .runtime_functions
            .push(PlannedRuntimeFunction {
                runtime_id: 1,
                runtime_name: "Exact".to_owned(),
                package: "mech-exact".to_owned(),
                crate_name: "mech_exact".to_owned(),
                installer_path: "mech_exact::__mech_native::install_exact".to_owned(),
                cargo_features: vec!["runtime".to_owned()],
            });
        assert_digest_changes(&with_function, |changed| {
            changed.runtime_functions[0].runtime_id = 2
        });
        assert_digest_changes(&with_function, |changed| {
            changed.runtime_functions[0]
                .installer_path
                .push_str("_other")
        });
        assert_digest_changes(&with_function, |changed| {
            changed.runtime_functions[0].package.push_str("-other")
        });
        assert_digest_changes(&with_function, |changed| {
            changed.runtime_functions[0]
                .cargo_features
                .push("f64".to_owned())
        });

        let mut with_host = plan.clone();
        with_host.hosts.push(PlannedHostInstance {
            name: "cli".to_owned(),
            provider: "cli".to_owned(),
            package: "mech-host-cli".to_owned(),
            crate_name: "mech_host_cli".to_owned(),
            cargo_features: vec!["provider".to_owned()],
            factory_path: "mech_host_cli::CliHostFactory::new".to_owned(),
            settings: ConfigValue::Null,
        });
        assert_digest_changes(&with_host, |changed| {
            changed.hosts[0].settings = ConfigValue::Bool(true)
        });

        let mut with_grant = plan.clone();
        with_grant.run_grants.push(RunResourceGrantConfig {
            target: "cli/stdout".to_owned(),
            operations: vec!["write".to_owned()],
            paths: vec!["line".to_owned()],
        });
        assert_digest_changes(&with_grant, |changed| {
            changed.run_grants[0].paths[0] = "text".to_owned()
        });

        let mut workspace = plan;
        workspace.dependency_source = PlannedDependencySource::Workspace;
        workspace.workspace_fingerprint = Some("a".repeat(64));
        assert_digest_changes(&workspace, |changed| {
            changed.workspace_fingerprint = Some("b".repeat(64))
        });
    }

    fn assert_digest_changes(plan: &NativeBuildPlan, mutate: impl FnOnce(&mut NativeBuildPlan)) {
        let expected = compute_plan_sha256(plan).unwrap();
        let mut changed = plan.clone();
        mutate(&mut changed);
        assert_ne!(compute_plan_sha256(&changed).unwrap(), expected);
    }
}
