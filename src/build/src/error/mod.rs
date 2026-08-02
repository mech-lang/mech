use std::path::PathBuf;

use mech_core::{MechError, MechErrorKind};

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
    NativeRuntimeTypeUnsupported {
        runtime_type: String,
    },
    NativeRuntimeConfigMissing {
        requirement: String,
    },
    NativeRuntimeConfigDuplicateHostInstance {
        instance: String,
    },
    NativeHostInstanceUnknown {
        instance: String,
    },
    NativeHostProviderUnknown {
        provider: String,
    },
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
    NativeBuildInstallerPathInvalid {
        value: String,
    },
    NativeDependencyInvalid {
        reason: String,
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
            Self::NativeRuntimeTypeUnsupported { .. } => "NativeRuntimeTypeUnsupported",
            Self::NativeRuntimeConfigMissing { .. } => "NativeRuntimeConfigMissing",
            Self::NativeRuntimeConfigDuplicateHostInstance { .. } => {
                "NativeRuntimeConfigDuplicateHostInstance"
            }
            Self::NativeHostInstanceUnknown { .. } => "NativeHostInstanceUnknown",
            Self::NativeHostProviderUnknown { .. } => "NativeHostProviderUnknown",
            Self::NativeResourceOwnerAmbiguous { .. } => "NativeResourceOwnerAmbiguous",
            Self::NativeTargetUnsupported { .. } => "NativeTargetUnsupported",
            Self::NativeHostSettingsInvalid { .. } => "NativeHostSettingsInvalid",
            Self::NativeContextOperationInvalid { .. } => "NativeContextOperationInvalid",
            Self::NativeResourcePathInvalid { .. } => "NativeResourcePathInvalid",
            Self::NativeRunGrantMissing { .. } => "NativeRunGrantMissing",
            Self::NativeIdentifierInvalid { .. } => "NativeIdentifierInvalid",
            Self::NativeBuildBinaryNameInvalid { .. } => "NativeBuildBinaryNameInvalid",
            Self::NativeBuildTargetInvalid { .. } => "NativeBuildTargetInvalid",
            Self::NativeBuildInstallerPathInvalid { .. } => "NativeBuildInstallerPathInvalid",
            Self::NativeDependencyInvalid { .. } => "NativeDependencyInvalid",
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
            Self::NativeRuntimeTypeUnsupported { runtime_type } => {
                format!("runtime type `{runtime_type}` is not supported by native Phase 1")
            }
            Self::NativeRuntimeConfigMissing { requirement } => {
                format!("native runtime configuration is required for `{requirement}`")
            }
            Self::NativeRuntimeConfigDuplicateHostInstance { instance } => format!(
                "native runtime configuration contains duplicate host instance `{instance}`"
            ),
            Self::NativeHostInstanceUnknown { instance } => {
                format!("native host instance `{instance}` is not configured")
            }
            Self::NativeHostProviderUnknown { provider } => {
                format!("native host provider `{provider}` is not trusted")
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
            Self::NativeBuildInstallerPathInvalid { value } => {
                format!("invalid native installer path `{value}`")
            }
            Self::NativeDependencyInvalid { reason } => {
                format!("invalid native dependency: {reason}")
            }
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
