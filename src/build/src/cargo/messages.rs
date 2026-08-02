use std::collections::BTreeSet;
use std::path::PathBuf;

use mech_core::MResult;

use super::NativeBuildArtifact;
use crate::error::{NativeBuildErrorKind, native_build_error};

/// Relevant data extracted from one Cargo `CompilerArtifact` message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CargoCompilerArtifact {
    pub target_name: String,
    pub target_kinds: Vec<String>,
    pub executable: Option<PathBuf>,
}

impl CargoCompilerArtifact {
    pub fn is_binary(&self) -> bool {
        self.target_kinds.iter().any(|kind| kind == "bin")
    }
}

/// Deterministic collector used by the Cargo JSON adapter in the generation
/// phase. Artifact selection always uses Cargo's reported executable path.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CargoBuildMessages {
    pub artifacts: Vec<CargoCompilerArtifact>,
    pub rendered_diagnostics: Vec<String>,
}

impl CargoBuildMessages {
    pub fn push_artifact(&mut self, artifact: CargoCompilerArtifact) {
        self.artifacts.push(artifact);
    }

    pub fn push_rendered_diagnostic(&mut self, diagnostic: impl Into<String>) {
        self.rendered_diagnostics.push(diagnostic.into());
    }

    pub fn select_binary(&self, binary_name: &str) -> MResult<NativeBuildArtifact> {
        let paths = self
            .artifacts
            .iter()
            .filter(|artifact| artifact.target_name == binary_name && artifact.is_binary())
            .filter_map(|artifact| artifact.executable.clone())
            .collect::<BTreeSet<_>>();

        match paths.len() {
            0 => Err(native_build_error(
                NativeBuildErrorKind::NativeCargoArtifactMissing {
                    binary: binary_name.to_string(),
                },
                None,
            )),
            1 => Ok(NativeBuildArtifact::new(
                paths.into_iter().next().expect("one path was measured"),
            )),
            _ => Err(native_build_error(
                NativeBuildErrorKind::NativeCargoArtifactAmbiguous {
                    binary: binary_name.to_string(),
                    paths: paths.into_iter().collect(),
                },
                None,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn artifact(name: &str, kinds: &[&str], executable: Option<&str>) -> CargoCompilerArtifact {
        CargoCompilerArtifact {
            target_name: name.into(),
            target_kinds: kinds.iter().map(|kind| (*kind).into()).collect(),
            executable: executable.map(PathBuf::from),
        }
    }

    #[test]
    fn executable_is_selected_from_the_exact_cargo_artifact() {
        let messages = CargoBuildMessages {
            artifacts: vec![
                artifact("dependency", &["lib"], None),
                artifact("other", &["bin"], Some("target/other")),
                artifact("phase1", &["bin"], Some("target/nonstandard/phase1")),
            ],
            rendered_diagnostics: Vec::new(),
        };
        assert_eq!(
            messages.select_binary("phase1").unwrap().executable,
            PathBuf::from("target/nonstandard/phase1")
        );
    }

    #[test]
    fn missing_and_ambiguous_artifacts_are_structured_errors() {
        let missing = CargoBuildMessages::default()
            .select_binary("phase1")
            .unwrap_err();
        assert_eq!(missing.kind_name(), "NativeCargoArtifactMissing");

        let messages = CargoBuildMessages {
            artifacts: vec![
                artifact("phase1", &["bin"], Some("target/one")),
                artifact("phase1", &["bin"], Some("target/two")),
            ],
            rendered_diagnostics: Vec::new(),
        };
        let ambiguous = messages.select_binary("phase1").unwrap_err();
        assert_eq!(ambiguous.kind_name(), "NativeCargoArtifactAmbiguous");
    }
}
