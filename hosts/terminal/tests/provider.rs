use std::collections::HashMap;

use mech_core::{LegacyValue, MResult, MechError, MechErrorKind, Ref};
use mech_runtime::{
    PreparedRuntimeEffect, RuntimeCapabilityOperation, RuntimeResourceProvider,
    RuntimeResourceReadRequest, RuntimeResourceWriteIntent, RuntimeResourceWriteRequest,
};
use mech_terminal::{CliBackend, CliResourceProvider};

#[derive(Debug, Default)]
struct FakeCliBackend {
    env: HashMap<String, String>,
    env_error: Option<String>,
    stdout: Vec<String>,
    stderr: Vec<String>,
}

impl CliBackend for FakeCliBackend {
    fn env_var(&self, name: &str) -> MResult<Option<String>> {
        if let Some(reason) = &self.env_error {
            return Err(MechError::new(
                FakeCliBackendError {
                    reason: reason.clone(),
                },
                None,
            ));
        }
        Ok(self.env.get(name).cloned())
    }
    fn write_stdout(&mut self, text: &str) -> MResult<()> {
        self.stdout.push(text.to_string());
        Ok(())
    }
    fn write_stderr(&mut self, text: &str) -> MResult<()> {
        self.stderr.push(text.to_string());
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct FakeCliBackendError {
    reason: String,
}

impl MechErrorKind for FakeCliBackendError {
    fn name(&self) -> &str {
        "FakeCliBackend"
    }

    fn message(&self) -> String {
        self.reason.clone()
    }
}

fn str_value(text: &str) -> LegacyValue {
    LegacyValue::String(Ref::new(text.to_string()))
}

fn execute_write(
    provider: &mut CliResourceProvider<FakeCliBackend>,
    request: RuntimeResourceWriteRequest,
) -> MResult<()> {
    match provider.prepare_write(request)? {
        PreparedRuntimeEffect::AfterCommit(mut effect) => effect.deliver(),
        effect => panic!("expected CLI after-commit effect, got {effect:?}"),
    }
}

#[test]
fn env_read_returns_fake_value_and_missing_errors() {
    let mut backend = FakeCliBackend::default();
    backend
        .env
        .insert("HOME".to_string(), "/tmp/home".to_string());
    let provider = CliResourceProvider::new(backend);
    let value = provider
        .read(RuntimeResourceReadRequest {
            base_uri: "cli://env".to_string(),
            path: "HOME".to_string(),
            context_name: "env".to_string(),
        })
        .unwrap();
    assert_eq!(value, str_value("/tmp/home"));
    assert!(
        provider
            .read(RuntimeResourceReadRequest {
                base_uri: "cli://env".to_string(),
                path: "MISSING".to_string(),
                context_name: "env".to_string()
            })
            .is_err()
    );
}

#[test]
fn env_plan_read_returns_an_empty_string_without_touching_the_backend() {
    let provider = CliResourceProvider::new(FakeCliBackend {
        env_error: Some("backend must not be called while planning".to_string()),
        ..FakeCliBackend::default()
    });

    let value = provider
        .plan_read(RuntimeResourceReadRequest {
            base_uri: "cli://env".to_string(),
            path: "HOME".to_string(),
            context_name: "env".to_string(),
        })
        .unwrap();

    assert_eq!(value, str_value(""));
}

#[test]
fn env_read_rejects_invalid_env_keys() {
    let provider = CliResourceProvider::new(FakeCliBackend::default());

    for path in ["", "1HOME", "HOME/PATH", "HOME-PATH"] {
        let result = provider.read(RuntimeResourceReadRequest {
            base_uri: "cli://env".to_string(),
            path: path.to_string(),
            context_name: "env".to_string(),
        });
        assert!(result.is_err(), "expected invalid env path to fail: {path}");
    }
}

#[test]
fn env_plan_read_validates_keys_and_other_cli_resources_are_not_plannable() {
    let provider = CliResourceProvider::new(FakeCliBackend::default());

    for path in ["", "1HOME", "HOME/PATH", "HOME-PATH"] {
        let result = provider.plan_read(RuntimeResourceReadRequest {
            base_uri: "cli://env".to_string(),
            path: path.to_string(),
            context_name: "env".to_string(),
        });
        assert!(result.is_err(), "expected invalid env path to fail: {path}");
    }

    let error = provider
        .plan_read(RuntimeResourceReadRequest {
            base_uri: "cli://stdout".to_string(),
            path: "line".to_string(),
            context_name: "stdout".to_string(),
        })
        .unwrap_err();
    assert_eq!(error.kind_name(), "RuntimeResourceReadNotPlannable");
}

#[test]
fn env_read_propagates_backend_errors() {
    let provider = CliResourceProvider::new(FakeCliBackend {
        env_error: Some("host env decode failed".to_string()),
        ..FakeCliBackend::default()
    });

    let result = provider.read(RuntimeResourceReadRequest {
        base_uri: "cli://env".to_string(),
        path: "HOME".to_string(),
        context_name: "env".to_string(),
    });

    assert!(result.is_err());
    let message = result.unwrap_err().display_message();
    assert!(message.contains("host env decode failed"), "{message}");
    assert!(!message.contains("is not set"), "{message}");
}

#[test]
fn env_write_and_send_error() {
    let mut provider = CliResourceProvider::new(FakeCliBackend::default());
    for intent in [
        RuntimeResourceWriteIntent::Assign,
        RuntimeResourceWriteIntent::Send,
    ] {
        assert!(
            provider
                .prepare_write(RuntimeResourceWriteRequest {
                    base_uri: "cli://env".to_string(),
                    path: "HOME".to_string(),
                    context_name: "env".to_string(),
                    operation: RuntimeCapabilityOperation::Write,
                    value: str_value("x"),
                    intent
                })
                .is_err()
        );
    }
}

#[test]
fn stdout_and_stderr_send_text_and_line() {
    let mut provider = CliResourceProvider::new(FakeCliBackend::default());
    execute_write(
        &mut provider,
        RuntimeResourceWriteRequest {
            base_uri: "cli://stdout".to_string(),
            path: "text".to_string(),
            context_name: "out".to_string(),
            operation: RuntimeCapabilityOperation::Write,
            value: str_value("abc"),
            intent: RuntimeResourceWriteIntent::Send,
        },
    )
    .unwrap();
    execute_write(
        &mut provider,
        RuntimeResourceWriteRequest {
            base_uri: "cli://stdout".to_string(),
            path: "line".to_string(),
            context_name: "out".to_string(),
            operation: RuntimeCapabilityOperation::Write,
            value: str_value("abc"),
            intent: RuntimeResourceWriteIntent::Send,
        },
    )
    .unwrap();
    execute_write(
        &mut provider,
        RuntimeResourceWriteRequest {
            base_uri: "cli://stderr".to_string(),
            path: "text".to_string(),
            context_name: "err".to_string(),
            operation: RuntimeCapabilityOperation::Write,
            value: str_value("warning"),
            intent: RuntimeResourceWriteIntent::Send,
        },
    )
    .unwrap();
    execute_write(
        &mut provider,
        RuntimeResourceWriteRequest {
            base_uri: "cli://stderr".to_string(),
            path: "line".to_string(),
            context_name: "err".to_string(),
            operation: RuntimeCapabilityOperation::Write,
            value: str_value("warning"),
            intent: RuntimeResourceWriteIntent::Send,
        },
    )
    .unwrap();
    assert_eq!(provider.backend().stdout, vec!["abc", "abc\n"]);
    assert_eq!(provider.backend().stderr, vec!["warning", "warning\n"]);
}

#[test]
fn stdout_prepare_write_buffers_until_delivery() {
    let mut provider = CliResourceProvider::new(FakeCliBackend::default());
    let effect = provider
        .prepare_write(RuntimeResourceWriteRequest {
            base_uri: "cli://stdout".to_string(),
            path: "line".to_string(),
            context_name: "out".to_string(),
            operation: RuntimeCapabilityOperation::Write,
            value: str_value("buffered"),
            intent: RuntimeResourceWriteIntent::Send,
        })
        .unwrap();

    assert!(provider.backend().stdout.is_empty());
    match effect {
        PreparedRuntimeEffect::AfterCommit(mut effect) => {
            effect.deliver().unwrap();
        }
        effect => panic!("expected CLI after-commit effect, got {effect:?}"),
    }
    assert_eq!(provider.backend().stdout, vec!["buffered\n"]);
}

#[test]
fn stdout_and_stderr_reject_assign_read_and_unknown_path() {
    let mut provider = CliResourceProvider::new(FakeCliBackend::default());
    assert!(
        provider
            .prepare_write(RuntimeResourceWriteRequest {
                base_uri: "cli://stdout".to_string(),
                path: "line".to_string(),
                context_name: "out".to_string(),
                operation: RuntimeCapabilityOperation::Write,
                value: str_value("abc"),
                intent: RuntimeResourceWriteIntent::Assign
            })
            .is_err()
    );
    assert!(
        provider
            .read(RuntimeResourceReadRequest {
                base_uri: "cli://stdout".to_string(),
                path: "line".to_string(),
                context_name: "out".to_string()
            })
            .is_err()
    );
    assert!(
        provider
            .prepare_write(RuntimeResourceWriteRequest {
                base_uri: "cli://stderr".to_string(),
                path: "foo".to_string(),
                context_name: "err".to_string(),
                operation: RuntimeCapabilityOperation::Write,
                value: str_value("abc"),
                intent: RuntimeResourceWriteIntent::Send
            })
            .is_err()
    );
    assert!(provider.backend().stdout.is_empty());
    assert!(provider.backend().stderr.is_empty());
}
