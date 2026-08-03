use std::collections::BTreeSet;
use std::io::BufRead;
use std::path::PathBuf;

use cargo_metadata::Message;
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
    pub build_finished: Option<bool>,
}

impl CargoBuildMessages {
    pub fn push_artifact(&mut self, artifact: CargoCompilerArtifact) {
        self.artifacts.push(artifact);
    }

    pub fn push_rendered_diagnostic(&mut self, diagnostic: impl Into<String>) {
        self.rendered_diagnostics.push(diagnostic.into());
    }

    pub fn diagnostics_summary(&self) -> String {
        self.rendered_diagnostics
            .iter()
            .map(|diagnostic| diagnostic.trim_end())
            .filter(|diagnostic| !diagnostic.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
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

/// Parse Cargo's newline-delimited JSON stream using the supported
/// `cargo_metadata` adapter.
pub fn parse_cargo_build_messages(reader: impl BufRead) -> MResult<CargoBuildMessages> {
    let mut messages = CargoBuildMessages::default();
    for message in Message::parse_stream(reader) {
        let message = message.map_err(|error| {
            native_build_error(
                NativeBuildErrorKind::NativeCargoFailed {
                    reason: format!("failed to read Cargo JSON messages: {error}"),
                },
                None,
            )
        })?;
        match message {
            Message::CompilerArtifact(artifact) => {
                messages.push_artifact(CargoCompilerArtifact {
                    target_name: artifact.target.name,
                    target_kinds: artifact
                        .target
                        .kind
                        .into_iter()
                        .map(|kind| kind.to_string())
                        .collect(),
                    executable: artifact
                        .executable
                        .map(|executable| executable.into_std_path_buf()),
                });
            }
            Message::CompilerMessage(message) => {
                messages.push_rendered_diagnostic(
                    message.message.rendered.unwrap_or(message.message.message),
                );
            }
            Message::BuildFinished(finished) => {
                messages.build_finished = Some(finished.success);
            }
            Message::TextLine(line) => messages.push_rendered_diagnostic(line),
            Message::BuildScriptExecuted(_) => {}
            _ => {}
        }
    }
    Ok(messages)
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
                artifact("native", &["bin"], Some("target/nonstandard/native")),
            ],
            rendered_diagnostics: Vec::new(),
            build_finished: Some(true),
        };
        assert_eq!(
            messages.select_binary("native").unwrap().executable,
            PathBuf::from("target/nonstandard/native")
        );
    }

    #[test]
    fn missing_and_ambiguous_artifacts_are_structured_errors() {
        let missing = CargoBuildMessages::default()
            .select_binary("native")
            .unwrap_err();
        assert_eq!(missing.kind_name(), "NativeCargoArtifactMissing");

        let messages = CargoBuildMessages {
            artifacts: vec![
                artifact("native", &["bin"], Some("target/one")),
                artifact("native", &["bin"], Some("target/two")),
            ],
            rendered_diagnostics: Vec::new(),
            build_finished: Some(true),
        };
        let ambiguous = messages.select_binary("native").unwrap_err();
        assert_eq!(ambiguous.kind_name(), "NativeCargoArtifactAmbiguous");
    }

    #[test]
    fn cargo_json_streams_are_parsed_without_deriving_artifact_paths() {
        let stream = concat!(
            "{\"reason\":\"compiler-artifact\",",
            "\"package_id\":\"path+file:///tmp/generated#0.0.0\",",
            "\"manifest_path\":\"/tmp/generated/Cargo.toml\",",
            "\"target\":{\"kind\":[\"bin\"],\"crate_types\":[\"bin\"],",
            "\"name\":\"native\",\"src_path\":\"/tmp/generated/src/main.rs\",",
            "\"edition\":\"2024\",\"doc\":true,\"doctest\":false,\"test\":true},",
            "\"profile\":{\"opt_level\":\"0\",\"debuginfo\":2,\"debug_assertions\":true,",
            "\"overflow_checks\":true,\"test\":false},",
            "\"features\":[],\"filenames\":[\"/tmp/target/native\"],",
            "\"executable\":\"/tmp/nonstandard/native\",\"fresh\":false}\n",
            "{\"reason\":\"build-finished\",\"success\":true}\n",
        );
        let messages = parse_cargo_build_messages(std::io::Cursor::new(stream)).unwrap();
        assert_eq!(messages.build_finished, Some(true));
        assert_eq!(
            messages.select_binary("native").unwrap().executable,
            PathBuf::from("/tmp/nonstandard/native")
        );
    }
}
