use std::io::Write;

use mech_core::{MResult, MechError, MechErrorKind, Ref, Value};
use mech_runtime::{
    RuntimeResourceProvider, RuntimeResourceReadRequest, RuntimeResourceWriteIntent,
    RuntimeResourceWriteRequest,
};

pub trait CliBackend: std::fmt::Debug {
    fn env_var(&self, name: &str) -> MResult<Option<String>>;
    fn write_stdout(&mut self, text: &str) -> MResult<()>;
    fn write_stderr(&mut self, text: &str) -> MResult<()>;
}

#[derive(Debug, Default)]
pub struct StdCliBackend;

impl CliBackend for StdCliBackend {
    fn env_var(&self, name: &str) -> MResult<Option<String>> {
        Ok(std::env::var(name).ok())
    }

    fn write_stdout(&mut self, text: &str) -> MResult<()> {
        std::io::stdout().write_all(text.as_bytes()).map_err(|err| {
            MechError::new(
                CliResourceProviderError {
                    resource: "cli://stdout".to_string(),
                    reason: err.to_string(),
                },
                None,
            )
        })
    }

    fn write_stderr(&mut self, text: &str) -> MResult<()> {
        std::io::stderr().write_all(text.as_bytes()).map_err(|err| {
            MechError::new(
                CliResourceProviderError {
                    resource: "cli://stderr".to_string(),
                    reason: err.to_string(),
                },
                None,
            )
        })
    }
}

#[derive(Debug)]
pub struct CliResourceProvider<B: CliBackend> {
    backend: B,
}

impl<B: CliBackend> CliResourceProvider<B> {
    pub fn new(backend: B) -> Self {
        Self { backend }
    }
    pub fn backend(&self) -> &B {
        &self.backend
    }
    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }
}

impl<B: CliBackend> RuntimeResourceProvider for CliResourceProvider<B> {
    fn scheme(&self) -> &str {
        "cli"
    }

    fn base_uris(&self) -> Vec<String> {
        vec![
            "cli://env".to_string(),
            "cli://stdout".to_string(),
            "cli://stderr".to_string(),
        ]
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        match request.base_uri.as_str() {
            "cli://env" => {
                validate_env_key(&request.path)?;
                let value = self.backend.env_var(&request.path)?.ok_or_else(|| {
                    MechError::new(
                        CliResourceProviderError {
                            resource: request.base_uri.clone(),
                            reason: format!("environment variable `{}` is not set", request.path),
                        },
                        None,
                    )
                })?;
                Ok(Value::String(Ref::new(value)))
            }
            "cli://stdout" | "cli://stderr" => Err(cli_error(
                request.base_uri,
                "stdout/stderr are send-only and cannot be read; use <- to send",
            )),
            other => Err(cli_error(other.to_string(), "unsupported cli resource")),
        }
    }

    fn write(&mut self, request: RuntimeResourceWriteRequest) -> MResult<()> {
        match request.base_uri.as_str() {
            "cli://env" => Err(cli_error(
                request.base_uri,
                "cli env is read-only and does not support writes or sends",
            )),
            "cli://stdout" | "cli://stderr" => {
                if request.intent != RuntimeResourceWriteIntent::Send {
                    return Err(cli_error(
                        request.base_uri,
                        "stdout/stderr are send-only; use <-",
                    ));
                }
                let suffix = match request.path.as_str() {
                    "text" => "",
                    "line" => "\n",
                    _ => {
                        return Err(cli_error(
                            request.base_uri,
                            "stdout/stderr support only `text` and `line` paths",
                        ));
                    }
                };
                let text = value_to_text(&request.value) + suffix;
                if request.base_uri == "cli://stdout" {
                    self.backend.write_stdout(&text)
                } else {
                    self.backend.write_stderr(&text)
                }
            }
            other => Err(cli_error(other.to_string(), "unsupported cli resource")),
        }
    }
}

fn validate_env_key(key: &str) -> MResult<()> {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return Err(cli_error(
            "cli://env".to_string(),
            "env path must contain exactly one variable name",
        ));
    };
    if !(first.is_ascii_alphabetic() || first == '_')
        || !chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(cli_error(
            "cli://env".to_string(),
            "env path must match [A-Za-z_][A-Za-z0-9_]*",
        ));
    }
    Ok(())
}

fn value_to_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.borrow().clone(),
        other => format!("{}", other),
    }
}

fn cli_error(resource: String, reason: impl Into<String>) -> MechError {
    MechError::new(
        CliResourceProviderError {
            resource,
            reason: reason.into(),
        },
        None,
    )
}

#[derive(Debug, Clone)]
pub struct CliResourceProviderError {
    pub resource: String,
    pub reason: String,
}

impl MechErrorKind for CliResourceProviderError {
    fn name(&self) -> &str {
        "CliResourceProvider"
    }
    fn message(&self) -> String {
        format!("{}: {}", self.resource, self.reason)
    }
}
