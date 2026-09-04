use std::path::PathBuf;

use mech_core::{MechError, MechErrorKind};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeProjectRelocationUnsupported {
    pub project_root: PathBuf,
    pub dependency: PathBuf,
    pub reason: String,
}

impl MechErrorKind for NativeProjectRelocationUnsupported {
    fn name(&self) -> &str {
        "NativeProjectRelocationUnsupported"
    }

    fn message(&self) -> String {
        format!(
            "generated project `{}` cannot address dependency `{}`: {}",
            self.project_root.display(),
            self.dependency.display(),
            self.reason
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeHostAddressabilityInvalid {
    pub instance: String,
    pub provider: String,
    pub reason: String,
}

impl MechErrorKind for NativeHostAddressabilityInvalid {
    fn name(&self) -> &str {
        "NativeHostAddressabilityInvalid"
    }

    fn message(&self) -> String {
        format!(
            "native host instance `{}` from provider `{}` has invalid resource addressability: {}",
            self.instance, self.provider, self.reason
        )
    }
}

/// Structured failures raised while planning or constructing a native Mech
/// application.
///
/// Keeping these failures in one error kind makes them downcastable through
/// [`MechError`] without reducing planning failures to opaque strings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeBuildErrorKind {
    NativeRuntimeFunctionUnknown {
        id: u64,
    },
    NativeRuntimeFunctionNameMismatch {
        id: u64,
        name: String,
    },
    NativeRuntimeFunctionLinkageMissing {
        id: u64,
        name: String,
    },
    NativeRuntimeFunctionLinkageInvalid {
        id: u64,
        name: String,
        reason: String,
    },
    NativeRuntimeFunctionBindingInvalid {
        reason: String,
    },
    NativeRuntimeTypeUnsupported {
        runtime_type: String,
    },
    NativeRuntimeConfigMissing {
        requirement: String,
    },
    NativeRuntimeConfigDuplicateHostInstance {
        instance: String,
    },
    NativeRuntimeConfigUnsupported {
        reason: String,
    },
    NativeHostInstanceUnknown {
        instance: String,
    },
    NativeHostProviderUnknown {
        provider: String,
    },
    NativeHostFunctionLinkageMissing {
        name: String,
    },
    NativeApplicationInstructionInvalid {
        instruction: u32,
        reason: String,
    },
    NativeActorBootstrapMissing,
    NativeActorBootstrapUnused,
    NativeActorBootstrapUnsupported,
    NativeActorLiveApplicationUnsupported,
    NativeResourceOwnerAmbiguous {
        target: String,
        instances: Vec<String>,
    },
    NativeTargetUnsupported {
        provider: String,
        target: Option<String>,
    },
    NativeHostSettingsInvalid {
        instance: String,
        reason: String,
    },
    NativeContextOperationInvalid {
        target: String,
        operation: String,
    },
    NativeResourceContextInvalid {
        target: String,
        expected: String,
        actual: String,
    },
    NativeResourcePathInvalid {
        target: String,
        path: String,
    },
    NativeRunGrantMissing {
        target: String,
        operation: String,
        path: String,
    },
    NativeIdentifierInvalid {
        kind: &'static str,
        value: String,
    },
    NativeBuildBinaryNameInvalid {
        value: String,
    },
    NativeBuildTargetInvalid {
        value: String,
    },
    NativeBuildTargetPointerWidthUnknown {
        target: String,
    },
    NativeBuildIndexConstantOutOfRange {
        target: String,
        pointer_width: u32,
        value: u64,
    },
    NativeBuildInstallerPathInvalid {
        value: String,
    },
    NativeDependencyInvalid {
        reason: String,
    },
    NativeComponentVersionMismatch {
        package: String,
        expected: String,
        actual: String,
    },
    NativeWorkspacePackageUnknown {
        package: String,
    },
    NativeWorkspacePackageDuplicate {
        package: String,
    },
    NativeWorkspaceInputInvalid {
        path: PathBuf,
        reason: String,
    },
    NativeProjectInvalid {
        reason: String,
    },
    NativeCargoFailed {
        reason: String,
    },
    NativeCargoArtifactMissing {
        binary: String,
    },
    NativeCargoArtifactAmbiguous {
        binary: String,
        paths: Vec<PathBuf>,
    },
}

impl MechErrorKind for NativeBuildErrorKind {
    fn name(&self) -> &str {
        match self {
            Self::NativeRuntimeFunctionUnknown { .. } => "NativeRuntimeFunctionUnknown",
            Self::NativeRuntimeFunctionNameMismatch { .. } => "NativeRuntimeFunctionNameMismatch",
            Self::NativeRuntimeFunctionLinkageMissing { .. } => {
                "NativeRuntimeFunctionLinkageMissing"
            }
            Self::NativeRuntimeFunctionLinkageInvalid { .. } => {
                "NativeRuntimeFunctionLinkageInvalid"
            }
            Self::NativeRuntimeFunctionBindingInvalid { .. } => {
                "NativeRuntimeFunctionBindingInvalid"
            }
            Self::NativeRuntimeTypeUnsupported { .. } => "NativeRuntimeTypeUnsupported",
            Self::NativeRuntimeConfigMissing { .. } => "NativeRuntimeConfigMissing",
            Self::NativeRuntimeConfigDuplicateHostInstance { .. } => {
                "NativeRuntimeConfigDuplicateHostInstance"
            }
            Self::NativeRuntimeConfigUnsupported { .. } => "NativeRuntimeConfigUnsupported",
            Self::NativeHostInstanceUnknown { .. } => "NativeHostInstanceUnknown",
            Self::NativeHostProviderUnknown { .. } => "NativeHostProviderUnknown",
            Self::NativeHostFunctionLinkageMissing { .. } => "NativeHostFunctionLinkageMissing",
            Self::NativeApplicationInstructionInvalid { .. } => {
                "NativeApplicationInstructionInvalid"
            }
            Self::NativeActorBootstrapMissing => "NativeActorBootstrapMissing",
            Self::NativeActorBootstrapUnused => "NativeActorBootstrapUnused",
            Self::NativeActorBootstrapUnsupported => "NativeActorBootstrapUnsupported",
            Self::NativeActorLiveApplicationUnsupported => "NativeActorLiveApplicationUnsupported",
            Self::NativeResourceOwnerAmbiguous { .. } => "NativeResourceOwnerAmbiguous",
            Self::NativeTargetUnsupported { .. } => "NativeTargetUnsupported",
            Self::NativeHostSettingsInvalid { .. } => "NativeHostSettingsInvalid",
            Self::NativeContextOperationInvalid { .. } => "NativeContextOperationInvalid",
            Self::NativeResourceContextInvalid { .. } => "NativeResourceContextInvalid",
            Self::NativeResourcePathInvalid { .. } => "NativeResourcePathInvalid",
            Self::NativeRunGrantMissing { .. } => "NativeRunGrantMissing",
            Self::NativeIdentifierInvalid { .. } => "NativeIdentifierInvalid",
            Self::NativeBuildBinaryNameInvalid { .. } => "NativeBuildBinaryNameInvalid",
            Self::NativeBuildTargetInvalid { .. } => "NativeBuildTargetInvalid",
            Self::NativeBuildTargetPointerWidthUnknown { .. } => {
                "NativeBuildTargetPointerWidthUnknown"
            }
            Self::NativeBuildIndexConstantOutOfRange { .. } => "NativeBuildIndexConstantOutOfRange",
            Self::NativeBuildInstallerPathInvalid { .. } => "NativeBuildInstallerPathInvalid",
            Self::NativeDependencyInvalid { .. } => "NativeDependencyInvalid",
            Self::NativeComponentVersionMismatch { .. } => "NativeComponentVersionMismatch",
            Self::NativeWorkspacePackageUnknown { .. } => "NativeWorkspacePackageUnknown",
            Self::NativeWorkspacePackageDuplicate { .. } => "NativeWorkspacePackageDuplicate",
            Self::NativeWorkspaceInputInvalid { .. } => "NativeWorkspaceInputInvalid",
            Self::NativeProjectInvalid { .. } => "NativeProjectInvalid",
            Self::NativeCargoFailed { .. } => "NativeCargoFailed",
            Self::NativeCargoArtifactMissing { .. } => "NativeCargoArtifactMissing",
            Self::NativeCargoArtifactAmbiguous { .. } => "NativeCargoArtifactAmbiguous",
        }
    }

    fn message(&self) -> String {
        match self {
            Self::NativeRuntimeFunctionUnknown { id } => {
                format!("runtime function ID {id} is not present in the function catalog")
            }
            Self::NativeRuntimeFunctionNameMismatch { id, name } => {
                format!("runtime function `{name}` does not hash to its declared ID {id}")
            }
            Self::NativeRuntimeFunctionLinkageMissing { id, name } => {
                format!("runtime function `{name}` ({id}) has no native linkage metadata")
            }
            Self::NativeRuntimeFunctionLinkageInvalid { id, name, reason } => {
                format!("runtime function `{name}` ({id}) has invalid native linkage: {reason}")
            }
            Self::NativeRuntimeFunctionBindingInvalid { reason } => {
                format!("native runtime semantic binding is invalid: {reason}")
            }
            Self::NativeRuntimeTypeUnsupported { runtime_type } => {
                format!("runtime type `{runtime_type}` is not supported by native applications")
            }
            Self::NativeRuntimeConfigMissing { requirement } => {
                format!("native runtime configuration is required for `{requirement}`")
            }
            Self::NativeRuntimeConfigDuplicateHostInstance { instance } => format!(
                "native runtime configuration contains duplicate host instance `{instance}`"
            ),
            Self::NativeRuntimeConfigUnsupported { reason } => {
                format!("native runtime configuration is unsupported: {reason}")
            }
            Self::NativeHostInstanceUnknown { instance } => {
                format!("native host instance `{instance}` is not configured")
            }
            Self::NativeHostProviderUnknown { provider } => {
                format!("native host provider `{provider}` is not trusted")
            }
            Self::NativeHostFunctionLinkageMissing { name } => {
                format!("host function `{name}` has no trusted native linkage")
            }
            Self::NativeApplicationInstructionInvalid {
                instruction,
                reason,
            } => format!(
                "native application bytecode instruction {instruction} is invalid: {reason}"
            ),
            Self::NativeActorBootstrapMissing => {
                "an actor-turn native application requires an explicit actor bootstrap".to_owned()
            }
            Self::NativeActorBootstrapUnused => {
                "an actor bootstrap was configured for an application with no actor-turn requirements"
                    .to_owned()
            }
            Self::NativeActorBootstrapUnsupported => {
                "actor bootstrap is not part of the production resident program contract"
                    .to_owned()
            }
            Self::NativeActorLiveApplicationUnsupported => {
                "actor-turn native applications cannot also contain live resource requirements"
                    .to_owned()
            }
            Self::NativeResourceOwnerAmbiguous { target, instances } => format!(
                "resource target `{target}` is owned by multiple host instances: {}",
                instances.join(", ")
            ),
            Self::NativeTargetUnsupported { provider, target } => format!(
                "native host provider `{provider}` does not support target `{}`",
                target.as_deref().unwrap_or("current")
            ),
            Self::NativeHostSettingsInvalid { instance, reason } => {
                format!("settings for native host instance `{instance}` are invalid: {reason}")
            }
            Self::NativeContextOperationInvalid { target, operation } => {
                format!("resource target `{target}` does not support operation `{operation}`")
            }
            Self::NativeResourceContextInvalid {
                target,
                expected,
                actual,
            } => format!(
                "resource target `{target}` requires context name `{expected}`, found `{actual}`"
            ),
            Self::NativeResourcePathInvalid { target, path } => {
                format!("resource path `{path}` is invalid for target `{target}`")
            }
            Self::NativeRunGrantMissing {
                target,
                operation,
                path,
            } => format!("no run grant permits `{operation}` on `{target}` at path `{path}`"),
            Self::NativeIdentifierInvalid { kind, value } => {
                format!("invalid native {kind} `{value}`")
            }
            Self::NativeBuildBinaryNameInvalid { value } => {
                format!("invalid native binary name `{value}`")
            }
            Self::NativeBuildTargetInvalid { value } => {
                format!("invalid native target triple `{value}`")
            }
            Self::NativeBuildTargetPointerWidthUnknown { target } => format!(
                "native target `{target}` has an unknown pointer width; Index constants cannot be validated deterministically"
            ),
            Self::NativeBuildIndexConstantOutOfRange {
                target,
                pointer_width,
                value,
            } => format!(
                "Index constant {value} exceeds the {pointer_width}-bit address space of native target `{target}`"
            ),
            Self::NativeBuildInstallerPathInvalid { value } => {
                format!("invalid native installer path `{value}`")
            }
            Self::NativeDependencyInvalid { reason } => {
                format!("invalid native dependency: {reason}")
            }
            Self::NativeComponentVersionMismatch {
                package,
                expected,
                actual,
            } => format!(
                "native component package `{package}` has version `{actual}`, expected `{expected}`"
            ),
            Self::NativeWorkspacePackageUnknown { package } => {
                format!("workspace package `{package}` is not registered")
            }
            Self::NativeWorkspacePackageDuplicate { package } => {
                format!("workspace package `{package}` is registered more than once")
            }
            Self::NativeWorkspaceInputInvalid { path, reason } => {
                format!("workspace input `{}` is invalid: {reason}", path.display())
            }
            Self::NativeProjectInvalid { reason } => {
                format!("generated native project is invalid: {reason}")
            }
            Self::NativeCargoFailed { reason } => {
                format!("Cargo invocation failed: {reason}")
            }
            Self::NativeCargoArtifactMissing { binary } => {
                format!("Cargo did not report an executable artifact for `{binary}`")
            }
            Self::NativeCargoArtifactAmbiguous { binary, paths } => format!(
                "Cargo reported multiple executable artifacts for `{binary}`: {}",
                paths
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

/// Wrap a structured build failure in the repository-wide error carrier.
pub fn native_build_error(kind: NativeBuildErrorKind, message: Option<String>) -> MechError {
    MechError::new(kind, message).with_compiler_loc()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_analysis_error_names_are_preserved() {
        let cases = [
            NativeBuildErrorKind::NativeRuntimeFunctionUnknown { id: 1 },
            NativeBuildErrorKind::NativeRuntimeFunctionNameMismatch {
                id: 1,
                name: "Wrong".into(),
            },
            NativeBuildErrorKind::NativeRuntimeFunctionLinkageMissing {
                id: 1,
                name: "Known".into(),
            },
            NativeBuildErrorKind::NativeRuntimeFunctionLinkageInvalid {
                id: 1,
                name: "Known".into(),
                reason: "bad path".into(),
            },
        ];

        assert_eq!(
            cases.map(|kind| native_build_error(kind, None).kind_name()),
            [
                "NativeRuntimeFunctionUnknown",
                "NativeRuntimeFunctionNameMismatch",
                "NativeRuntimeFunctionLinkageMissing",
                "NativeRuntimeFunctionLinkageInvalid",
            ]
        );
    }
}
