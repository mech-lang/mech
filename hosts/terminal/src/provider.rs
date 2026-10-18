use std::{
    env::VarError,
    ffi::OsString,
    io::{Read, Write},
    sync::{Arc, Mutex, MutexGuard},
};

use mech_core::{
    AccessMode, DeliveryMode, DimensionExpr, EffectContract, EffectDeliveryPolicy,
    ExternalInteraction, IdempotencyRequirement, InputPortLayout, InputPortPolicy, IntegerWidth,
    MResult, MechError, MechErrorKind, OperationContractDeclaration, Schema, SchemaBody,
    SchemaDraft, SchemaTableBuilder, Value, ValueCell, ValueData,
    snapshot::{SequenceView, SnapshotValidationContext, ValueDataDraft, ValueDraft},
};
use mech_runtime::{
    ConfigValue, HostManifestConfig, PreparedRuntimeEffect, ResourceSchemaPreflight,
    RuntimeAfterCommitEffect, RuntimeEffectCost, RuntimeEffectMetadata, RuntimeEffectSource,
    RuntimeHostFactory, RuntimeHostInputValue, RuntimeHostInstallation, RuntimeResourceProvider,
    RuntimeResourceReadNotPlannable, RuntimeResourceReadRequest, RuntimeResourceWriteCommand,
    RuntimeResourceWriteIntent, RuntimeResourceWritePreflightRequest, RuntimeResourceWriteRequest,
    materialize_host_manifest,
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
    /// Return the one-based user argument, excluding the executable at index 0.
    /// Embedders can supply a user-argument snapshot instead of process argv.
    fn argument(&self, _index: usize) -> MResult<Option<OsString>> {
        Err(cli_error(
            "cli://args".to_string(),
            "command-line arguments are unavailable from this CLI backend",
        ))
    }
    /// Return the length of the same immutable payload-only snapshot used by
    /// `argument`. Backends without an explicit payload boundary leave this
    /// unavailable instead of counting executable or launcher arguments.
    fn argument_count(&self) -> MResult<u64> {
        Err(cli_error(
            "cli://args".to_string(),
            "payload argument count is unavailable from this CLI backend",
        ))
    }
    /// Read exact Unicode text through EOF. Providers retain the first result,
    /// so a consumptive stream is observed only once per provider instance.
    fn stdin_text(&self) -> MResult<String> {
        Err(cli_error(
            "cli://stdin".to_string(),
            "standard input is unavailable from this CLI backend",
        ))
    }
    /// Optional raw stream acquisition. `None` declares a legacy text-only
    /// backend and must not observe stdin. A supported backend returns `Some`
    /// for both success and failure; the provider caches that one acquisition.
    fn stdin_bytes(&self) -> Option<MResult<Vec<u8>>> {
        None
    }
    fn write_stdout(&mut self, text: &str) -> MResult<()>;
    fn write_stderr(&mut self, text: &str) -> MResult<()>;
    /// Deliver raw bytes without decoding or adding a suffix. Implementations
    /// must report write and flush failures through the returned result.
    fn write_stdout_bytes(&mut self, _bytes: &[u8]) -> MResult<()> {
        Err(cli_error(
            "cli://stdout".into(),
            "raw byte output is unavailable from this CLI backend",
        ))
    }
    fn write_stderr_bytes(&mut self, _bytes: &[u8]) -> MResult<()> {
        Err(cli_error(
            "cli://stderr".into(),
            "raw byte output is unavailable from this CLI backend",
        ))
    }
}

/// Supplies immutable application arguments while delegating all other CLI I/O.
/// The snapshot contains payload arguments only: element zero is `@args/1` and
/// no executable name or launcher controls are inserted or removed here.
#[derive(Clone, Debug)]
pub struct ArgumentSnapshotCliBackend<B> {
    backend: B,
    arguments: Arc<[OsString]>,
}

impl<B> ArgumentSnapshotCliBackend<B> {
    pub fn new(backend: B, arguments: Vec<OsString>) -> Self {
        Self {
            backend,
            arguments: arguments.into(),
        }
    }
}

impl<B: CliBackend> CliBackend for ArgumentSnapshotCliBackend<B> {
    fn argument_count(&self) -> MResult<u64> {
        u64::try_from(self.arguments.len()).map_err(|_| {
            cli_error(
                "cli://args".to_string(),
                "payload argument count exceeds u64",
            )
        })
    }

    fn argument(&self, index: usize) -> MResult<Option<OsString>> {
        Ok(index
            .checked_sub(1)
            .and_then(|index| self.arguments.get(index))
            .cloned())
    }

    fn env_var(&self, name: &str) -> MResult<Option<String>> {
        self.backend.env_var(name)
    }

    fn stdin_text(&self) -> MResult<String> {
        self.backend.stdin_text()
    }

    fn stdin_bytes(&self) -> Option<MResult<Vec<u8>>> {
        self.backend.stdin_bytes()
    }

    fn write_stdout(&mut self, text: &str) -> MResult<()> {
        self.backend.write_stdout(text)
    }

    fn write_stderr(&mut self, text: &str) -> MResult<()> {
        self.backend.write_stderr(text)
    }

    fn write_stdout_bytes(&mut self, bytes: &[u8]) -> MResult<()> {
        self.backend.write_stdout_bytes(bytes)
    }

    fn write_stderr_bytes(&mut self, bytes: &[u8]) -> MResult<()> {
        self.backend.write_stderr_bytes(bytes)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct StdCliBackend;

impl CliBackend for StdCliBackend {
    fn stdin_bytes(&self) -> Option<MResult<Vec<u8>>> {
        Some((|| {
            let mut bytes = Vec::new();
            std::io::stdin()
                .lock()
                .read_to_end(&mut bytes)
                .map_err(|error| cli_error("cli://stdin".into(), error.to_string()))?;
            Ok(bytes)
        })())
    }

    fn stdin_text(&self) -> MResult<String> {
        let mut text = String::new();
        std::io::stdin()
            .lock()
            .read_to_string(&mut text)
            .map_err(|error| cli_error("cli://stdin".to_string(), error.to_string()))?;
        Ok(text)
    }

    fn argument(&self, index: usize) -> MResult<Option<OsString>> {
        Ok(std::env::args_os().nth(index))
    }

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

    fn write_stdout_bytes(&mut self, bytes: &[u8]) -> MResult<()> {
        let mut out = std::io::stdout().lock();
        out.write_all(bytes)
            .map_err(|err| cli_error("cli://stdout".into(), err.to_string()))?;
        out.flush()
            .map_err(|err| cli_error("cli://stdout".into(), err.to_string()))
    }

    fn write_stderr_bytes(&mut self, bytes: &[u8]) -> MResult<()> {
        let mut err = std::io::stderr().lock();
        err.write_all(bytes)
            .map_err(|error| cli_error("cli://stderr".into(), error.to_string()))?;
        err.flush()
            .map_err(|error| cli_error("cli://stderr".into(), error.to_string()))
    }
}

#[derive(Debug)]
enum CliStdinAcquisition {
    Raw(Vec<u8>),
    LegacyText,
}

#[derive(Debug, Default)]
struct CliStdinSnapshot {
    acquisition: Option<MResult<CliStdinAcquisition>>,
    text: Option<MResult<Value>>,
    bytes: Option<MResult<Value>>,
}

#[derive(Debug)]
pub struct CliResourceProvider<B: CliBackend> {
    instance: String,
    backend: Arc<Mutex<B>>,
    stdin_snapshot: Mutex<CliStdinSnapshot>,
}

impl<B: CliBackend> CliResourceProvider<B> {
    pub fn new(backend: B) -> Self {
        Self::for_instance("cli", backend)
    }
    pub fn for_instance(instance: impl Into<String>, backend: B) -> Self {
        Self {
            instance: instance.into(),
            backend: Arc::new(Mutex::new(backend)),
            stdin_snapshot: Mutex::new(CliStdinSnapshot::default()),
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

    /// Validate and buffer terminal output without delivering it.
    ///
    /// The Rust host owns authorization and delivery policy when using this
    /// helper directly. Mech resource writes use the runtime capability checks
    /// and deliver the same effect only after their transaction commits.
    pub fn prepare_output(
        &self,
        request: RuntimeResourceWriteCommand,
    ) -> MResult<Box<dyn RuntimeAfterCommitEffect>>
    where
        B: 'static,
    {
        self.preflight_write(RuntimeResourceWritePreflightRequest {
            base_uri: request.base_uri.clone(),
            path: request.path.clone(),
            context_name: request.context_name.clone(),
            operation: request.operation.clone(),
            intent: request.intent,
        })?;
        let payload = output_payload(&request.path, &request.value, &request.base_uri)?;
        let stream = if self.matches_base(&request.base_uri, "stdout") {
            CliOutputStream::Stdout
        } else {
            CliOutputStream::Stderr
        };
        Ok(Box::new(CliOutputEffect {
            backend: self.backend.clone(),
            stream,
            payload,
            resource: request.base_uri,
        }))
    }
}

impl<B: CliBackend + 'static> RuntimeResourceProvider for CliResourceProvider<B> {
    fn scheme(&self) -> &str {
        "cli"
    }

    fn base_uris(&self) -> Vec<String> {
        let mut bases = vec![
            self.base("env"),
            self.base("args"),
            self.base("stdin"),
            self.base("stdout"),
            self.base("stderr"),
        ];
        if self.instance == "cli" {
            bases.extend([
                "cli://env".to_string(),
                "cli://args".to_string(),
                "cli://stdin".to_string(),
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
            && !self.matches_base(&request.base_uri, "args")
            && !self.matches_base(&request.base_uri, "stdin")
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
            vec![self.base("args"), "cli://args".to_string()],
            vec![self.base("stdin"), "cli://stdin".to_string()],
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
        } else if self.matches_base(&request.base_uri, "args") {
            if request.path == "count" {
                validate_argument_count_read(&request)?;
                let count = self
                    .backend
                    .lock()
                    .map_err(|_| cli_error(request.base_uri.clone(), "CLI backend lock poisoned"))?
                    .argument_count()?;
                return argument_count_value(count);
            }
            let index = argument_index(&request.path)?;
            let argument = self
                .backend
                .lock()
                .map_err(|_| cli_error(request.base_uri.clone(), "CLI backend lock poisoned"))?
                .argument(index)?
                .ok_or_else(|| {
                    cli_error(
                        request.base_uri.clone(),
                        format!("argument {index} is not present"),
                    )
                })?;
            let value = argument.into_string().map_err(|_| {
                cli_error(
                    request.base_uri,
                    format!("argument {index} is not valid Unicode"),
                )
            })?;
            RuntimeHostInputValue::String(value).into_value()
        } else if self.matches_base(&request.base_uri, "stdin") {
            validate_stdin_read(&request)?;
            let mut guard = self.stdin_snapshot.lock().map_err(|_| {
                cli_error(request.base_uri.clone(), "CLI stdin snapshot lock poisoned")
            })?;
            let CliStdinSnapshot {
                acquisition,
                text,
                bytes,
            } = &mut *guard;
            let acquired = acquisition.get_or_insert_with(|| {
                let backend = self.backend.lock().map_err(|_| {
                    cli_error(request.base_uri.clone(), "CLI backend lock poisoned")
                })?;
                match backend.stdin_bytes() {
                    Some(result) => result.map(CliStdinAcquisition::Raw),
                    None => Ok(CliStdinAcquisition::LegacyText),
                }
            });
            match acquired {
                Err(error) => Err(error.clone()),
                Ok(CliStdinAcquisition::Raw(raw)) if request.path == "bytes" => bytes
                    .get_or_insert_with(|| byte_row_value(raw, &request.base_uri))
                    .clone(),
                Ok(CliStdinAcquisition::Raw(raw)) => text
                    .get_or_insert_with(|| {
                        let decoded = std::str::from_utf8(raw).map_err(|_| {
                            cli_error(
                                request.base_uri.clone(),
                                "standard input is not valid UTF-8",
                            )
                        })?;
                        RuntimeHostInputValue::String(decoded.to_owned()).into_value()
                    })
                    .clone(),
                Ok(CliStdinAcquisition::LegacyText) if request.path == "bytes" => bytes
                    .get_or_insert_with(|| {
                        Err(cli_error(
                            request.base_uri.clone(),
                            "raw byte input is unavailable from this CLI backend",
                        ))
                    })
                    .clone(),
                Ok(CliStdinAcquisition::LegacyText) => text
                    .get_or_insert_with(|| {
                        let decoded = self
                            .backend
                            .lock()
                            .map_err(|_| {
                                cli_error(request.base_uri.clone(), "CLI backend lock poisoned")
                            })?
                            .stdin_text()?;
                        RuntimeHostInputValue::String(decoded).into_value()
                    })
                    .clone(),
            }
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
        if self.matches_base(&request.base_uri, "args") {
            if request.path == "count" {
                validate_argument_count_read(&request)?;
                // This is only a typed planning value. It neither observes
                // the payload vector nor establishes its actual cardinality.
                return argument_count_value(0);
            }
            argument_index(&request.path)?;
            return RuntimeHostInputValue::String(String::new()).into_value();
        }
        if self.matches_base(&request.base_uri, "stdin") {
            validate_stdin_read(&request)?;
            return if request.path == "bytes" {
                byte_row_value(&[], &request.base_uri)
            } else {
                RuntimeHostInputValue::String(String::new()).into_value()
            };
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
        if self.matches_base(&request.base_uri, "env")
            || self.matches_base(&request.base_uri, "args")
            || self.matches_base(&request.base_uri, "stdin")
        {
            Err(cli_error(
                request.base_uri,
                "cli env/args/stdin are read-only and do not support writes or sends",
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
                "text" | "line" | "bytes" => Ok(()),
                _ => Err(cli_error(
                    request.base_uri,
                    "stdout/stderr support only `text`, `line` and `bytes` paths",
                )),
            }
        } else {
            Err(cli_error(request.base_uri, "unsupported cli resource"))
        }
    }

    fn preflight_write_schema(
        &self,
        request: RuntimeResourceWritePreflightRequest,
        payload_schema: &Schema,
    ) -> MResult<ResourceSchemaPreflight> {
        let resource = request.base_uri.clone();
        let path = request.path.clone();
        self.preflight_write(request)?;
        if path == "bytes" {
            return byte_row_schema(payload_schema, &resource);
        }
        if !matches!(payload_schema.body(), SchemaBody::String) {
            return Err(cli_error(
                resource,
                "stdout/stderr sends require a scalar string payload",
            ));
        }
        Ok(ResourceSchemaPreflight::Validated)
    }

    fn prepare_write(
        &self,
        request: RuntimeResourceWriteRequest,
    ) -> MResult<PreparedRuntimeEffect> {
        self.prepare_output(RuntimeResourceWriteCommand {
            base_uri: request.base_uri,
            path: request.path,
            context_name: request.context_name,
            operation: request.operation,
            value: request.value,
            intent: request.intent,
        })
        .map(PreparedRuntimeEffect::AfterCommit)
    }

    fn plan_write(&self, request: RuntimeResourceWriteCommand) -> MResult<()> {
        self.preflight_write(RuntimeResourceWritePreflightRequest {
            base_uri: request.base_uri.clone(),
            path: request.path.clone(),
            context_name: request.context_name,
            operation: request.operation,
            intent: request.intent,
        })?;
        validate_output_value(&request.path, &request.value, &request.base_uri)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
enum CliOutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug)]
enum CliOutputPayload {
    Text(String),
    Bytes(Vec<u8>),
}

impl CliOutputPayload {
    fn len(&self) -> usize {
        match self {
            Self::Text(text) => text.len(),
            Self::Bytes(bytes) => bytes.len(),
        }
    }
}

fn argument_count_value(count: u64) -> MResult<Value> {
    // This canonical scalar snapshot is independent of the legacy boxed-u64
    // feature. The standard distribution must not grow its feature closure
    // merely to expose fixed-size typed resource metadata.
    let mut builder = SchemaTableBuilder::new();
    let handle = builder.insert(
        SchemaDraft {
            dimension_parameters: Box::new([]),
            body: SchemaBody::UnsignedInteger(IntegerWidth::W64),
        }
        .finalize()?,
    )?;
    let built = builder.finish()?;
    let schema = built.resolve(handle)?;
    let (schemas, _) = built.into_parts();
    ValueDraft {
        schema,
        shape_values: Box::new([]),
        data: ValueDataDraft::U64(count),
    }
    .finalize(&SnapshotValidationContext::new(&schemas))
    .map_err(|error| {
        cli_error(
            "cli://args".into(),
            format!("invalid argument count snapshot: {error:?}"),
        )
    })
}

fn byte_row_value(bytes: &[u8], resource: &str) -> MResult<Value> {
    let columns = u64::try_from(bytes.len())
        .map_err(|_| cli_error(resource.into(), "byte input length exceeds u64"))?;
    ValueCell::dynamic_matrix(
        SchemaBody::UnsignedInteger(IntegerWidth::W8),
        Box::new([1, columns]),
        bytes.iter().copied().map(ValueDataDraft::U8).collect(),
    )?
    .snapshot()
}

fn byte_row_schema(schema: &Schema, resource: &str) -> MResult<ResourceSchemaPreflight> {
    let SchemaBody::Matrix {
        element,
        dimensions,
    } = schema.body()
    else {
        return Err(cli_error(
            resource.into(),
            "byte sends require a rank-two U8 row payload",
        ));
    };
    if element.as_ref() != &SchemaBody::UnsignedInteger(IntegerWidth::W8) || dimensions.len() != 2 {
        return Err(cli_error(
            resource.into(),
            "byte sends require a rank-two U8 row payload",
        ));
    }
    match &dimensions[0] {
        DimensionExpr::Constant(1) => Ok(ResourceSchemaPreflight::Validated),
        DimensionExpr::Constant(_) => Err(cli_error(
            resource.into(),
            "byte sends require exactly one row",
        )),
        _ => Ok(ResourceSchemaPreflight::DeferredToRuntime),
    }
}

fn byte_row<'a>(value: &'a Value, resource: &str) -> MResult<&'a [u8]> {
    let schemas = value
        .schemas()
        .ok_or_else(|| cli_error(resource.into(), "byte payload schema is absent"))?;
    let schema = schemas
        .get(value.schema())
        .ok_or_else(|| cli_error(resource.into(), "byte payload schema is absent"))?;
    byte_row_schema(schema, resource)?;
    let SchemaBody::Matrix { dimensions, .. } = schema.body() else {
        unreachable!("byte_row_schema validated a matrix")
    };
    let rows = value
        .shape()
        .resolve_dimension(&dimensions[0])
        .map_err(MechError::from)?;
    let columns = value
        .shape()
        .resolve_dimension(&dimensions[1])
        .map_err(MechError::from)?;
    if rows != 1 {
        return Err(cli_error(
            resource.into(),
            "byte sends require exactly one row",
        ));
    }
    let ValueData::Matrix(matrix) = value.data() else {
        return Err(cli_error(
            resource.into(),
            "byte sends require a matrix payload",
        ));
    };
    let SequenceView::U8(bytes) = matrix.elements() else {
        return Err(cli_error(resource.into(), "byte sends require U8 elements"));
    };
    if u64::try_from(bytes.len()).ok() != Some(columns) {
        return Err(cli_error(
            resource.into(),
            "byte payload shape and length disagree",
        ));
    }
    Ok(bytes)
}

fn validate_output_value(path: &str, value: &Value, resource: &str) -> MResult<()> {
    if path == "bytes" {
        byte_row(value, resource)?;
    } else if !matches!(value.data(), ValueData::String(_)) {
        return Err(cli_error(
            resource.into(),
            "stdout/stderr sends require a scalar string payload",
        ));
    }
    Ok(())
}

fn output_payload(path: &str, value: &Value, resource: &str) -> MResult<CliOutputPayload> {
    validate_output_value(path, value, resource)?;
    if path == "bytes" {
        return Ok(CliOutputPayload::Bytes(byte_row(value, resource)?.to_vec()));
    }
    let ValueData::String(text) = value.data() else {
        unreachable!("validate_output_value validated text")
    };
    let suffix = if path == "line" { "\n" } else { "" };
    Ok(CliOutputPayload::Text(text.to_string() + suffix))
}

#[derive(Debug)]
struct CliOutputEffect<B: CliBackend> {
    backend: Arc<Mutex<B>>,
    stream: CliOutputStream,
    payload: CliOutputPayload,
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
            bytes: u64::try_from(self.payload.len()).unwrap_or(u64::MAX),
            items: 1,
        })
    }

    fn deliver(&mut self) -> MResult<()> {
        let mut backend = self
            .backend
            .lock()
            .map_err(|_| cli_error(self.resource.clone(), "CLI backend lock poisoned"))?;
        match (self.stream, &self.payload) {
            (CliOutputStream::Stdout, CliOutputPayload::Text(text)) => backend.write_stdout(text),
            (CliOutputStream::Stderr, CliOutputPayload::Text(text)) => backend.write_stderr(text),
            (CliOutputStream::Stdout, CliOutputPayload::Bytes(bytes)) => {
                backend.write_stdout_bytes(bytes)
            }
            (CliOutputStream::Stderr, CliOutputPayload::Bytes(bytes)) => {
                backend.write_stderr_bytes(bytes)
            }
        }
    }
}

fn validate_argument_count_read(request: &RuntimeResourceReadRequest) -> MResult<()> {
    if request.context_name != "args" {
        return Err(cli_error(
            request.base_uri.clone(),
            "argument count reads require context `args` and exactly the `count` path",
        ));
    }
    Ok(())
}

fn argument_index(path: &str) -> MResult<usize> {
    if path.is_empty() || path.starts_with('0') || !path.bytes().all(|b| b.is_ascii_digit()) {
        return Err(cli_error(
            "cli://args".to_string(),
            "args path must be a positive canonical decimal argument index (1, 2, ...)",
        ));
    }
    path.parse().map_err(|_| {
        cli_error(
            "cli://args".to_string(),
            "argument index exceeds the platform index range",
        )
    })
}

fn validate_stdin_read(request: &RuntimeResourceReadRequest) -> MResult<()> {
    if request.context_name != "stdin" || !matches!(request.path.as_str(), "text" | "bytes") {
        return Err(cli_error(
            request.base_uri.clone(),
            "stdin reads require context `stdin` and exactly the `text` or `bytes` path",
        ));
    }
    Ok(())
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

    /// Use exact payload arguments supplied by the launcher, excluding its
    /// executable and controls. Unicode validation occurs at resource reads.
    pub fn with_arguments(
        arguments: Vec<OsString>,
    ) -> MResult<CliHostFactory<ArgumentSnapshotCliBackend<StdCliBackend>>> {
        CliHostFactory::with_backend(ArgumentSnapshotCliBackend::new(StdCliBackend, arguments))
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
