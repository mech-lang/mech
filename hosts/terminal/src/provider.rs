use std::{
    env::VarError,
    io::Write,
    sync::{Arc, Mutex, MutexGuard},
};

use mech_core::{
    AccessMode, DeliveryMode, EffectContract, EffectDeliveryPolicy, ExternalInteraction,
    IdempotencyRequirement, InputPortLayout, InputPortPolicy, MResult, MechError, MechErrorKind,
    OperationContractDeclaration, Value, ValueData,
};
use mech_runtime::{
    ConfigValue, HostManifestConfig, PreparedRuntimeEffect, RuntimeAfterCommitEffect,
    RuntimeEffectCost, RuntimeEffectMetadata, RuntimeEffectSource, RuntimeHostFactory,
    RuntimeHostInputValue, RuntimeHostInstallation, RuntimeResourceProvider,
    RuntimeResourceReadNotPlannable, RuntimeResourceReadRequest, RuntimeResourceWriteIntent,
    RuntimeResourceWritePreflightRequest, RuntimeResourceWriteRequest, materialize_host_manifest,
};

static CLI_OUTPUT_EFFECT_CONTRACT: std::sync::LazyLock<OperationContractDeclaration> =
    std::sync::LazyLock::new(|| OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![InputPortPolicy {
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            }]
            .into_boxed_slice(),
        ),
        outputs: Box::new([]),
        interaction: ExternalInteraction::Effect(EffectContract {
            delivery: EffectDeliveryPolicy::AtMostOnce,
            idempotency: IdempotencyRequirement::NotRequired,
        }),
    });

pub trait CliBackend: std::fmt::Debug {
    fn env_var(&self, name: &str) -> MResult<Option<String>>;
    fn write_stdout(&mut self, text: &str) -> MResult<()>;
    fn write_stderr(&mut self, text: &str) -> MResult<()>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct StdCliBackend;

impl CliBackend for StdCliBackend {
    fn env_var(&self, name: &str) -> MResult<Option<String>> {
        match std::env::var(name) {
            Ok(value) => Ok(Some(value)),
            Err(VarError::NotPresent) => Ok(None),
            Err(VarError::NotUnicode(_)) => Err(cli_error(
                "cli://env".to_string(),
                format!("environment variable `{name}` exists but is not valid Unicode"),
            )),
        }
    }

    fn write_stdout(&mut self, text: &str) -> MResult<()> {
        let mut out = std::io::stdout().lock();
        out.write_all(text.as_bytes())
            .map_err(|err| cli_error("cli://stdout".to_string(), err.to_string()))?;
        out.flush()
            .map_err(|err| cli_error("cli://stdout".to_string(), err.to_string()))
    }

    fn write_stderr(&mut self, text: &str) -> MResult<()> {
        let mut err = std::io::stderr().lock();
        err.write_all(text.as_bytes())
            .map_err(|err| cli_error("cli://stderr".to_string(), err.to_string()))?;
        err.flush()
            .map_err(|err| cli_error("cli://stderr".to_string(), err.to_string()))
    }
}

#[derive(Debug)]
pub struct CliResourceProvider<B: CliBackend> {
    instance: String,
    backend: Arc<Mutex<B>>,
}

impl<B: CliBackend> CliResourceProvider<B> {
    pub fn new(backend: B) -> Self {
        Self::for_instance("cli", backend)
    }
    pub fn for_instance(instance: impl Into<String>, backend: B) -> Self {
        Self {
            instance: instance.into(),
            backend: Arc::new(Mutex::new(backend)),
        }
    }
    fn base(&self, context: &str) -> String {
        format!("cli://{}/{}", self.instance, context)
    }
    fn matches_base(&self, base_uri: &str, context: &str) -> bool {
        base_uri == self.base(context)
            || (self.instance == "cli" && base_uri == format!("cli://{}", context))
    }
    pub fn backend(&self) -> MutexGuard<'_, B> {
        self.backend.lock().expect("CLI backend lock poisoned")
    }
    pub fn backend_mut(&mut self) -> MutexGuard<'_, B> {
        self.backend.lock().expect("CLI backend lock poisoned")
    }
}

impl<B: CliBackend + 'static> RuntimeResourceProvider for CliResourceProvider<B> {
    fn scheme(&self) -> &str {
        "cli"
    }

    fn base_uris(&self) -> Vec<String> {
        let mut bases = vec![self.base("env"), self.base("stdout"), self.base("stderr")];
        if self.instance == "cli" {
            bases.extend([
                "cli://env".to_string(),
                "cli://stdout".to_string(),
                "cli://stderr".to_string(),
            ]);
        }
        bases
    }

    fn semantic_read_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(mech_runtime::resource_observation_contract())
    }

    fn observation_requires_input_driver(&self, request: &RuntimeResourceReadRequest) -> bool {
        !self.matches_base(&request.base_uri, "env")
    }

    fn semantic_write_contract(
        &self,
        intent: RuntimeResourceWriteIntent,
    ) -> Option<&'static OperationContractDeclaration> {
        (intent == RuntimeResourceWriteIntent::Send).then_some(&CLI_OUTPUT_EFFECT_CONTRACT)
    }

    fn equivalent_base_uri_groups(&self) -> Vec<Vec<String>> {
        if self.instance != "cli" {
            return Vec::new();
        }

        vec![
            vec![self.base("env"), "cli://env".to_string()],
            vec![self.base("stdout"), "cli://stdout".to_string()],
            vec![self.base("stderr"), "cli://stderr".to_string()],
        ]
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        if self.matches_base(&request.base_uri, "env") {
            validate_env_key(&request.path)?;
            let value = self
                .backend
                .lock()
                .map_err(|_| cli_error(request.base_uri.clone(), "CLI backend lock poisoned"))?
                .env_var(&request.path)?
                .ok_or_else(|| {
                    MechError::new(
                        CliResourceProviderError {
                            resource: request.base_uri.clone(),
                            reason: format!("environment variable `{}` is not set", request.path),
                        },
                        None,
                    )
                })?;
            RuntimeHostInputValue::String(value).into_value()
        } else if self.matches_base(&request.base_uri, "stdout")
            || self.matches_base(&request.base_uri, "stderr")
        {
            Err(cli_error(
                request.base_uri,
                "stdout/stderr are send-only and cannot be read; use <- to send",
            ))
        } else {
            Err(cli_error(request.base_uri, "unsupported cli resource"))
        }
    }

    fn plan_read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        if self.matches_base(&request.base_uri, "env") {
            validate_env_key(&request.path)?;
            return RuntimeHostInputValue::String(String::new()).into_value();
        }
        Err(MechError::new(
            RuntimeResourceReadNotPlannable {
                scheme: self.scheme().to_string(),
                base_uri: request.base_uri,
                path: request.path,
            },
            None,
        ))
    }

    fn preflight_write(&self, request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
        if self.matches_base(&request.base_uri, "env") {
            Err(cli_error(
                request.base_uri,
                "cli env is read-only and does not support writes or sends",
            ))
        } else if self.matches_base(&request.base_uri, "stdout")
            || self.matches_base(&request.base_uri, "stderr")
        {
            if request.intent != RuntimeResourceWriteIntent::Send {
                return Err(cli_error(
                    request.base_uri,
                    "stdout/stderr are send-only; use <-",
                ));
            }
            match request.path.as_str() {
                "text" | "line" => Ok(()),
                _ => Err(cli_error(
                    request.base_uri,
                    "stdout/stderr support only `text` and `line` paths",
                )),
            }
        } else {
            Err(cli_error(request.base_uri, "unsupported cli resource"))
        }
    }

    fn prepare_write(
        &self,
        request: RuntimeResourceWriteRequest,
    ) -> MResult<PreparedRuntimeEffect> {
        self.preflight_write(RuntimeResourceWritePreflightRequest {
            base_uri: request.base_uri.clone(),
            path: request.path.clone(),
            context_name: request.context_name.clone(),
            operation: request.operation.clone(),
            intent: request.intent,
        })?;

        let suffix = match request.path.as_str() {
            "text" => "",
            "line" => "\n",
            _ => unreachable!("cli stdout/stderr path validated by preflight_write"),
        };
        let ValueData::String(value) = request.value.data() else {
            return Err(cli_error(
                request.base_uri,
                "stdout/stderr sends require a scalar string payload",
            ));
        };
        let text = value.to_string() + suffix;
        let stream = if self.matches_base(&request.base_uri, "stdout") {
            CliOutputStream::Stdout
        } else {
            CliOutputStream::Stderr
        };
        Ok(PreparedRuntimeEffect::AfterCommit(Box::new(
            CliOutputEffect {
                backend: self.backend.clone(),
                stream,
                text,
                resource: request.base_uri,
            },
        )))
    }

    fn plan_write(&self, request: RuntimeResourceWriteRequest) -> MResult<()> {
        self.preflight_write(RuntimeResourceWritePreflightRequest {
            base_uri: request.base_uri.clone(),
            path: request.path,
            context_name: request.context_name,
            operation: request.operation,
            intent: request.intent,
        })?;
        if !matches!(request.value.data(), ValueData::String(_)) {
            return Err(cli_error(
                request.base_uri,
                "stdout/stderr sends require a scalar string payload",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
enum CliOutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug)]
struct CliOutputEffect<B: CliBackend> {
    backend: Arc<Mutex<B>>,
    stream: CliOutputStream,
    text: String,
    resource: String,
}

impl<B: CliBackend> RuntimeAfterCommitEffect for CliOutputEffect<B> {
    fn metadata(&self) -> RuntimeEffectMetadata {
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::ResourceProvider {
                scheme: "cli".to_string(),
            },
            match self.stream {
                CliOutputStream::Stdout => "stdout",
                CliOutputStream::Stderr => "stderr",
            },
        )
        .with_resource(self.resource.clone())
        .with_cost(RuntimeEffectCost {
            bytes: u64::try_from(self.text.len()).unwrap_or(u64::MAX),
            items: 1,
        })
    }

    fn deliver(&mut self) -> MResult<()> {
        let mut backend = self
            .backend
            .lock()
            .map_err(|_| cli_error(self.resource.clone(), "CLI backend lock poisoned"))?;
        match self.stream {
            CliOutputStream::Stdout => backend.write_stdout(&self.text),
            CliOutputStream::Stderr => backend.write_stderr(&self.text),
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

#[derive(Debug)]
pub struct CliHostFactory<B = StdCliBackend> {
    manifest: HostManifestConfig,
    backend: B,
}

impl CliHostFactory<StdCliBackend> {
    pub fn new() -> MResult<Self> {
        Self::with_backend(StdCliBackend)
    }
}

impl<B> CliHostFactory<B> {
    pub fn with_backend(backend: B) -> MResult<Self> {
        Ok(Self {
            manifest: crate::cli_host_manifest()?,
            backend,
        })
    }
}

impl<B: CliBackend + Clone + 'static> RuntimeHostFactory for CliHostFactory<B> {
    fn provider_name(&self) -> &str {
        "cli"
    }
    fn manifest(&self) -> &HostManifestConfig {
        &self.manifest
    }
    fn validate_settings(&self, _instance_name: &str, settings: &ConfigValue) -> MResult<()> {
        match settings {
            ConfigValue::Map(map) if map.is_empty() => Ok(()),
            _ => Err(cli_error(
                "cli://settings".to_string(),
                "cli host settings must be an empty map",
            )),
        }
    }
    fn instantiate(
        &self,
        instance_name: &str,
        settings: &ConfigValue,
    ) -> MResult<RuntimeHostInstallation> {
        self.validate_settings(instance_name, settings)?;
        Ok(RuntimeHostInstallation {
            interface: materialize_host_manifest(instance_name, &self.manifest)?,
            input_drivers: Vec::new(),
            resource_providers: vec![Box::new(CliResourceProvider::for_instance(
                instance_name,
                self.backend.clone(),
            ))],
        })
    }
}
