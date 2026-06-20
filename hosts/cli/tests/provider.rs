use std::collections::HashMap;

use mech_core::{MResult, Ref, Value};
use mech_host_cli::{CliBackend, CliResourceProvider};
use mech_runtime::{
    RuntimeResourceProvider, RuntimeResourceReadRequest, RuntimeResourceWriteIntent,
    RuntimeResourceWriteRequest,
};

#[derive(Debug, Default)]
struct FakeCliBackend {
    env: HashMap<String, String>,
    stdout: Vec<String>,
    stderr: Vec<String>,
}

impl CliBackend for FakeCliBackend {
    fn env_var(&self, name: &str) -> MResult<Option<String>> {
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

fn str_value(text: &str) -> Value {
    Value::String(Ref::new(text.to_string()))
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
fn env_write_and_send_error() {
    let mut provider = CliResourceProvider::new(FakeCliBackend::default());
    for intent in [
        RuntimeResourceWriteIntent::Assign,
        RuntimeResourceWriteIntent::Send,
    ] {
        assert!(
            provider
                .write(RuntimeResourceWriteRequest {
                    base_uri: "cli://env".to_string(),
                    path: "HOME".to_string(),
                    context_name: "env".to_string(),
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
    provider
        .write(RuntimeResourceWriteRequest {
            base_uri: "cli://stdout".to_string(),
            path: "text".to_string(),
            context_name: "out".to_string(),
            value: str_value("abc"),
            intent: RuntimeResourceWriteIntent::Send,
        })
        .unwrap();
    provider
        .write(RuntimeResourceWriteRequest {
            base_uri: "cli://stdout".to_string(),
            path: "line".to_string(),
            context_name: "out".to_string(),
            value: str_value("abc"),
            intent: RuntimeResourceWriteIntent::Send,
        })
        .unwrap();
    provider
        .write(RuntimeResourceWriteRequest {
            base_uri: "cli://stderr".to_string(),
            path: "text".to_string(),
            context_name: "err".to_string(),
            value: str_value("warning"),
            intent: RuntimeResourceWriteIntent::Send,
        })
        .unwrap();
    provider
        .write(RuntimeResourceWriteRequest {
            base_uri: "cli://stderr".to_string(),
            path: "line".to_string(),
            context_name: "err".to_string(),
            value: str_value("warning"),
            intent: RuntimeResourceWriteIntent::Send,
        })
        .unwrap();
    assert_eq!(provider.backend().stdout, vec!["abc", "abc\n"]);
    assert_eq!(provider.backend().stderr, vec!["warning", "warning\n"]);
}

#[test]
fn stdout_and_stderr_reject_assign_read_and_unknown_path() {
    let mut provider = CliResourceProvider::new(FakeCliBackend::default());
    assert!(
        provider
            .write(RuntimeResourceWriteRequest {
                base_uri: "cli://stdout".to_string(),
                path: "line".to_string(),
                context_name: "out".to_string(),
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
            .write(RuntimeResourceWriteRequest {
                base_uri: "cli://stderr".to_string(),
                path: "foo".to_string(),
                context_name: "err".to_string(),
                value: str_value("abc"),
                intent: RuntimeResourceWriteIntent::Send
            })
            .is_err()
    );
    assert!(provider.backend().stdout.is_empty());
    assert!(provider.backend().stderr.is_empty());
}
