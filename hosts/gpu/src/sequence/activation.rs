//! Explicit secure and trusted policies for the same native object and ports.
use super::NativeHostIo;
use mech_core::{ExecutionResourceRequest, ResourceDelivery, ResourceIntent, Value, ValueData};
use mech_engine::native_sequence::{NativeResourceIntent, NativeResourceRequirement};
use mech_runtime::{
    ConfigValue, HostInstanceConfig, MechRuntime, NativeResourceBindings, RunResourceGrantConfig,
    RuntimeBuilder, RuntimeCapabilityOperation, RuntimeContext, RuntimeHostInputValue,
    RuntimeResourceProvider, RuntimeResourceReadRequest, RuntimeResourceWriteCommand,
    RuntimeResourceWriteIntent, RuntimeResourceWritePreflightRequest,
};
use mech_terminal::{CliBackend, CliHostFactory, CliResourceProvider, StdCliBackend};
use serde::{Deserialize, Serialize};

/// Launch authority supplied by a host, separately from the compiled program.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeIoGrant {
    pub target: String,
    pub operations: Vec<String>,
    pub paths: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum NativeIoPolicy {
    Secure {
        grants: Vec<NativeIoGrant>,
    },
    /// Explicit trusted execution skips the capability kernel and runtime
    /// transactions, while preserving provider validation and delivery.
    Trusted,
}

enum Mode<B: CliBackend + Clone + 'static> {
    Secure {
        runtime: Box<MechRuntime>,
        bindings: NativeResourceBindings,
        context: RuntimeContext,
    },
    Trusted(CliResourceProvider<B>),
    Finished,
}

/// Activated text ports for a single native process. Both policies use the CLI
/// provider; secure calls also use the ordinary capability and transaction APIs.
pub struct CliHostIo<B: CliBackend + Clone + 'static = StdCliBackend> {
    requirements: Vec<NativeResourceRequirement>,
    mode: Mode<B>,
}

fn message(error: mech_core::MechError) -> String {
    error.display_message()
}

fn core_request(request: &NativeResourceRequirement) -> ExecutionResourceRequest {
    ExecutionResourceRequest {
        base_uri: request.base_uri.clone(),
        context_name: request.context_name.clone(),
        path: request.path.clone(),
        intent: match request.intent {
            NativeResourceIntent::Read => ResourceIntent::Read,
            NativeResourceIntent::Send => ResourceIntent::Send,
        },
        operation: match request.intent {
            NativeResourceIntent::Read => "read",
            NativeResourceIntent::Send => "write",
        }
        .into(),
        delivery: ResourceDelivery::Snapshot,
    }
}

fn read_request(request: &NativeResourceRequirement) -> RuntimeResourceReadRequest {
    RuntimeResourceReadRequest {
        base_uri: request.base_uri.clone(),
        context_name: request.context_name.clone(),
        path: request.path.clone(),
    }
}

fn string_bytes(value: Value) -> Result<Vec<u8>, String> {
    let ValueData::String(text) = value.data() else {
        return Err("native text resource returned a non-string value".into());
    };
    Ok(text.to_string().into_bytes())
}

impl<B: CliBackend + Clone + 'static> CliHostIo<B> {
    pub fn activate(
        requirements: &[NativeResourceRequirement],
        policy: NativeIoPolicy,
        backend: B,
    ) -> Result<Self, String> {
        // Validate the entire immutable manifest before constructing a port
        // that can perform an observation or deliver an effect.
        for request in requirements {
            request.validate()?;
        }
        let mode = match policy {
            NativeIoPolicy::Secure { grants } => {
                let mut builder = RuntimeBuilder::new()
                    .host_factory(Box::new(
                        CliHostFactory::with_backend(backend).map_err(message)?,
                    ))
                    .map_err(message)?
                    .host_instance(HostInstanceConfig {
                        name: "cli".into(),
                        provider: "cli".into(),
                        settings: ConfigValue::Map(Default::default()),
                    });
                for grant in grants {
                    builder = builder.run_resource_grant(RunResourceGrantConfig {
                        target: grant.target,
                        operations: grant.operations,
                        paths: grant.paths,
                    });
                }
                let mut runtime = builder.build().map_err(message)?;
                let requests = requirements.iter().map(core_request).collect::<Vec<_>>();
                let bindings = match runtime.bind_native_resources(&requests) {
                    Ok(bindings) => bindings,
                    Err(error) => {
                        let primary = message(error);
                        return match runtime.shutdown() {
                            Ok(()) => Err(primary),
                            Err(error) => Err(format!("{primary}; cleanup: {}", message(error))),
                        };
                    }
                };
                let context = (|| {
                    let mut context = runtime.runtime_context().map_err(message)?;
                    runtime.begin_transaction(&mut context).map_err(message)?;
                    Ok::<_, String>(context)
                })();
                let context = match context {
                    Ok(context) => context,
                    Err(primary) => {
                        return match runtime.shutdown() {
                            Ok(()) => Err(primary),
                            Err(error) => Err(format!("{primary}; cleanup: {}", message(error))),
                        };
                    }
                };
                Mode::Secure {
                    runtime: Box::new(runtime),
                    bindings,
                    context,
                }
            }
            NativeIoPolicy::Trusted => {
                let provider = CliResourceProvider::new(backend);
                for request in requirements {
                    match request.intent {
                        NativeResourceIntent::Read => {
                            // Planning validates a port without consulting argv.
                            provider.plan_read(read_request(request)).map_err(message)?;
                        }
                        NativeResourceIntent::Send => provider
                            .preflight_write(RuntimeResourceWritePreflightRequest {
                                base_uri: request.base_uri.clone(),
                                context_name: request.context_name.clone(),
                                path: request.path.clone(),
                                operation: RuntimeCapabilityOperation::Write,
                                intent: RuntimeResourceWriteIntent::Send,
                            })
                            .map_err(message)?,
                    }
                }
                Mode::Trusted(provider)
            }
        };
        Ok(Self {
            requirements: requirements.to_vec(),
            mode,
        })
    }

    fn index(&self, request: &NativeResourceRequirement) -> Result<usize, String> {
        self.requirements
            .iter()
            .position(|bound| bound == request)
            .ok_or_else(|| "native resource request is not activated".into())
    }
}

impl CliHostIo {
    /// Policy is selected at launch, never while compiling the native object.
    /// Secure is the default. An absent grant file supplies no authority.
    pub fn from_environment(requirements: &[NativeResourceRequirement]) -> Result<Self, String> {
        let policy = match std::env::var("MECH_NATIVE_SECURITY") {
            Err(std::env::VarError::NotPresent) => NativeIoPolicy::Secure {
                grants: environment_grants()?,
            },
            Ok(mode) if mode == "secure" => NativeIoPolicy::Secure {
                grants: environment_grants()?,
            },
            Ok(mode) if mode == "trusted" => NativeIoPolicy::Trusted,
            Ok(_) | Err(_) => return Err("MECH_NATIVE_SECURITY must be secure or trusted".into()),
        };
        Self::activate(requirements, policy, StdCliBackend)
    }
}

fn environment_grants() -> Result<Vec<NativeIoGrant>, String> {
    let Some(path) = std::env::var_os("MECH_NATIVE_GRANTS") else {
        return Ok(Vec::new());
    };
    let bytes = std::fs::read(path)
        .map_err(|error| format!("cannot read native launch grants: {error}"))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("invalid native launch grants: {error}"))
}

impl<B: CliBackend + Clone + 'static> NativeHostIo for CliHostIo<B> {
    fn requirements(&self) -> &[NativeResourceRequirement] {
        &self.requirements
    }

    fn read(&mut self, request: &NativeResourceRequirement) -> Result<Vec<u8>, String> {
        let index = self.index(request)?;
        if request.intent != NativeResourceIntent::Read {
            return Err("native send port cannot be read".into());
        }
        let value = match &mut self.mode {
            Mode::Secure {
                runtime,
                bindings,
                context,
            } => runtime
                .read_native_resource_with_context(bindings, index, context)
                .map_err(message)?,
            Mode::Trusted(provider) => provider.read(read_request(request)).map_err(message)?,
            Mode::Finished => return Err("native host resources have been shut down".into()),
        };
        string_bytes(value)
    }

    fn send(&mut self, request: &NativeResourceRequirement, text: &[u8]) -> Result<(), String> {
        let index = self.index(request)?;
        if request.intent != NativeResourceIntent::Send {
            return Err("native read port cannot be sent to".into());
        }
        let text = std::str::from_utf8(text).map_err(|_| "native text send is not UTF-8")?;
        let value = RuntimeHostInputValue::String(text.to_owned())
            .into_value()
            .map_err(message)?;
        match &mut self.mode {
            Mode::Secure {
                runtime,
                bindings,
                context,
            } => runtime
                .write_native_resource_with_context(bindings, index, value, context)
                .map_err(message),
            Mode::Trusted(provider) => {
                let mut effect = provider
                    .prepare_output(RuntimeResourceWriteCommand {
                        base_uri: request.base_uri.clone(),
                        context_name: request.context_name.clone(),
                        path: request.path.clone(),
                        operation: RuntimeCapabilityOperation::Write,
                        intent: RuntimeResourceWriteIntent::Send,
                        value,
                    })
                    .map_err(message)?;
                effect.deliver().map_err(message)
            }
            Mode::Finished => Err("native host resources have been shut down".into()),
        }
    }

    fn finish(&mut self, success: bool) -> Result<(), String> {
        if let Mode::Secure {
            mut runtime,
            mut context,
            ..
        } = std::mem::replace(&mut self.mode, Mode::Finished)
        {
            let mut errors = Vec::new();
            if success {
                if let Err(error) = runtime.commit_runtime_transaction(&mut context) {
                    errors.push(message(error));
                }
            }
            // Failed entries never publish staged effects. A failed commit can
            // also retain a transaction that still needs cleanup.
            if context.transaction_id().is_some() {
                if let Err(error) = runtime
                    .abort_runtime_transaction(&mut context, "native execution did not commit")
                {
                    errors.push(message(error));
                }
            }
            if let Err(error) = runtime.shutdown() {
                errors.push(message(error));
            }
            if !errors.is_empty() {
                return Err(errors.join("; "));
            }
        }
        Ok(())
    }
}

impl<B: CliBackend + Clone + 'static> Drop for CliHostIo<B> {
    fn drop(&mut self) {
        let _ = self.finish(false);
    }
}
