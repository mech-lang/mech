use std::path::PathBuf;

use mech_core::{ResourceDelivery, ResourceIntent, RuntimeType};
use mech_runtime::{ConfigValue, HostInstanceConfig, RunResourceGrantConfig, RuntimeConfig};
use serde::{Deserialize, Serialize};

pub const NATIVE_BUILD_PLAN_SCHEMA: &str = "mech.native-build-plan.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeBuildProfile {
    Debug,
    Release,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeEmit {
    Native,
    Bytecode,
    CargoProject,
    Plan,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NativeBuildRequest {
    pub bytecode: Vec<u8>,
    pub runtime_config: Option<NativeRuntimeConfig>,
    pub target: Option<String>,
    pub profile: NativeBuildProfile,
    pub binary_name: String,
    pub output: PathBuf,
    pub emit: NativeEmit,
    pub keep_project: bool,
    pub offline: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NativeRuntimeConfig {
    pub runtime: RuntimeConfig,
    pub hosts: Vec<HostInstanceConfig>,
    pub run_grants: Vec<RunResourceGrantConfig>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeApplicationKind {
    Engine,
    Hosted,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PlannedRuntimeFunction {
    pub runtime_id: u64,
    pub runtime_name: String,
    pub package: String,
    pub crate_name: String,
    pub installer_path: String,
    pub cargo_features: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PlannedApplicationRequirement {
    HostFunction {
        name: String,
        package: String,
        crate_name: String,
        installer_path: String,
        cargo_features: Vec<String>,
    },
    Resource {
        base_uri: String,
        path: String,
        context_name: String,
        operation: String,
        intent: ResourceIntent,
        delivery: ResourceDelivery,
        host_instance: String,
        provider: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum PlannedPackageSource {
    Registry { version: String },
    Workspace { path: PathBuf },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PlannedPackage {
    pub package: String,
    pub crate_name: String,
    pub source: PlannedPackageSource,
    pub cargo_features: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlannedHostInstance {
    pub name: String,
    pub provider: String,
    pub package: String,
    pub crate_name: String,
    pub cargo_features: Vec<String>,
    pub factory_path: String,
    pub settings: ConfigValue,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum PlannedDependencySource {
    Registry { version: String },
    Workspace,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NativeBuildPlan {
    pub schema: String,
    pub bytecode_version: u16,
    pub mech_version: String,
    pub application_kind: NativeApplicationKind,
    pub runtime_config: RuntimeConfig,
    pub bytecode_sha256: String,
    pub plan_sha256: String,
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
