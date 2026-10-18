#![cfg(feature = "provider")]

use mech_core::{IntegerWidth, MResult, MechError, SchemaBody, Value, ValueData};
use mech_runtime::{
    ConfigValue, HostInstanceConfig, RunResourceGrantConfig, RuntimeBuilder,
    RuntimeCapabilityOperation, RuntimeHostInputValue, RuntimeResourceProvider,
    RuntimeResourceReadRequest, RuntimeResourceWriteCommand, RuntimeResourceWriteIntent,
};
use mech_terminal::{
    ArgumentSnapshotCliBackend, CliBackend, CliHostFactory, CliResourceProvider,
    CliResourceProviderError, StdCliBackend,
};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    sync::{Arc, Mutex},
};

#[derive(Debug, Default)]
struct Calls {
    arguments: usize,
    counts: usize,
}

#[derive(Clone, Debug, Default)]
struct Backend {
    arguments: Arc<[OsString]>,
    calls: Arc<Mutex<Calls>>,
    fail_count: bool,
}

fn failure(reason: &str) -> MechError {
    MechError::new(
        CliResourceProviderError {
            resource: "test://arguments".into(),
            reason: reason.into(),
        },
        None,
    )
}

impl Backend {
    fn with_arguments(arguments: Vec<OsString>) -> Self {
        Self {
            arguments: arguments.into(),
            ..Self::default()
        }
    }
}

impl CliBackend for Backend {
    fn env_var(&self, _: &str) -> MResult<Option<String>> {
        Ok(None)
    }

    fn argument(&self, index: usize) -> MResult<Option<OsString>> {
        self.calls.lock().unwrap().arguments += 1;
        Ok(index
            .checked_sub(1)
            .and_then(|index| self.arguments.get(index))
            .cloned())
    }

    fn argument_count(&self) -> MResult<u64> {
        self.calls.lock().unwrap().counts += 1;
        if self.fail_count {
            return Err(failure("injected payload-count failure"));
        }
        u64::try_from(self.arguments.len()).map_err(|_| failure("count overflow"))
    }

    fn write_stdout(&mut self, _: &str) -> MResult<()> {
        panic!("argument metadata cannot write stdout")
    }

    fn write_stderr(&mut self, _: &str) -> MResult<()> {
        panic!("argument metadata cannot write stderr")
    }
}

#[derive(Debug)]
struct LegacyBackend;

impl CliBackend for LegacyBackend {
    fn env_var(&self, _: &str) -> MResult<Option<String>> {
        Ok(None)
    }
    fn argument(&self, index: usize) -> MResult<Option<OsString>> {
        Ok((index == 1).then(|| OsString::from("legacy")))
    }
    fn write_stdout(&mut self, _: &str) -> MResult<()> {
        panic!("read-only fixture")
    }
    fn write_stderr(&mut self, _: &str) -> MResult<()> {
        panic!("read-only fixture")
    }
}

fn read(base: &str, path: &str) -> RuntimeResourceReadRequest {
    RuntimeResourceReadRequest {
        base_uri: base.into(),
        path: path.into(),
        context_name: "args".into(),
    }
}

fn assert_count(value: &Value, expected: u64) {
    assert!(matches!(value.data(), ValueData::U64(actual) if *actual == expected));
    let owner = value.schemas().unwrap();
    assert_eq!(
        owner.get(value.schema()).unwrap().body(),
        &SchemaBody::UnsignedInteger(IntegerWidth::W64)
    );
    assert!(value.shape().parameter_values().is_empty());
}

#[test]
fn count_is_exact_payload_snapshot_length_and_never_delegates_to_another_argv() {
    for arguments in [
        Vec::new(),
        vec![OsString::from("")],
        vec![
            OsString::from("héllo"),
            OsString::from("--once"),
            OsString::from("--runtime-info"),
            OsString::from(""),
        ],
    ] {
        let backend = Backend {
            fail_count: true,
            ..Backend::with_arguments(vec![OsString::from("unrelated-backend-argument")])
        };
        let calls = backend.calls.clone();
        let snapshot = ArgumentSnapshotCliBackend::new(backend, arguments.clone());
        let clone = snapshot.clone();
        assert_eq!(snapshot.argument_count().unwrap(), arguments.len() as u64);
        assert_eq!(clone.argument_count().unwrap(), arguments.len() as u64);
        for (index, argument) in arguments.iter().enumerate() {
            assert_eq!(clone.argument(index + 1).unwrap().as_ref(), Some(argument));
        }
        assert!(snapshot.argument(arguments.len() + 1).unwrap().is_none());
        assert_eq!(calls.lock().unwrap().counts, 0);
        assert_eq!(calls.lock().unwrap().arguments, 0);
        let provider = CliResourceProvider::new(snapshot);
        assert_count(
            &provider.read(read("cli://cli/args", "count")).unwrap(),
            arguments.len() as u64,
        );
    }
}

#[test]
fn count_planning_is_typed_but_observes_neither_real_count_nor_arguments() {
    let backend = Backend::with_arguments(vec![OsString::from("first"), OsString::from("second")]);
    let calls = backend.calls.clone();
    let provider = CliResourceProvider::new(backend);
    let planned = provider.plan_read(read("cli://cli/args", "count")).unwrap();
    assert_count(&planned, 0);
    provider.plan_read(read("cli://args", "1")).unwrap();
    assert_eq!(calls.lock().unwrap().counts, 0);
    assert_eq!(calls.lock().unwrap().arguments, 0);
    assert!(!provider.observation_requires_input_driver(&read("cli://args", "count")));
    assert_count(&provider.read(read("cli://args", "count")).unwrap(), 2);
    assert_eq!(calls.lock().unwrap().counts, 1);
    assert_eq!(calls.lock().unwrap().arguments, 0);
}

#[test]
fn count_requires_exact_path_and_semantic_context_without_getter_calls() {
    let backend = Backend::with_arguments(vec![OsString::from("first")]);
    let calls = backend.calls.clone();
    let provider = CliResourceProvider::new(backend);
    for path in ["Count", "count/1", "count/", " count", "length", "0", "01"] {
        assert!(provider.plan_read(read("cli://args", path)).is_err());
        assert!(provider.read(read("cli://args", path)).is_err());
    }
    let mut request = read("cli://args", "count");
    request.context_name = "out".into();
    assert!(provider.plan_read(request.clone()).is_err());
    assert!(provider.read(request).is_err());
    assert_eq!(calls.lock().unwrap().counts, 0);
    assert_eq!(calls.lock().unwrap().arguments, 0);
}

#[test]
fn legacy_and_unwrapped_standard_backends_report_count_unavailable() {
    assert!(StdCliBackend.argument_count().is_err());
    let provider = CliResourceProvider::new(LegacyBackend);
    let planned = provider.plan_read(read("cli://args", "count")).unwrap();
    assert_count(&planned, 0);
    let error = provider.read(read("cli://args", "count")).unwrap_err();
    assert!(error.display_message().contains("count is unavailable"));
    let text = provider.read(read("cli://args", "1")).unwrap();
    assert!(matches!(text.data(), ValueData::String(text) if text.as_ref() == "legacy"));
}

#[test]
fn actual_count_error_propagates_and_does_not_become_a_zero_count() {
    let backend = Backend {
        fail_count: true,
        ..Backend::with_arguments(vec![OsString::from("first")])
    };
    let provider = CliResourceProvider::new(backend);
    assert_count(&provider.plan_read(read("cli://args", "count")).unwrap(), 0);
    let error = provider.read(read("cli://args", "count")).unwrap_err();
    assert!(
        error
            .display_message()
            .contains("injected payload-count failure")
    );
    assert_eq!(provider.backend().calls.lock().unwrap().counts, 1);
    assert_eq!(provider.backend().calls.lock().unwrap().arguments, 0);
}

#[test]
fn count_is_read_only_under_assign_and_send() {
    let provider = CliResourceProvider::new(Backend::default());
    for intent in [
        RuntimeResourceWriteIntent::Assign,
        RuntimeResourceWriteIntent::Send,
    ] {
        assert!(
            provider
                .plan_write(RuntimeResourceWriteCommand {
                    base_uri: "cli://cli/args".into(),
                    path: "count".into(),
                    context_name: "args".into(),
                    operation: RuntimeCapabilityOperation::Write,
                    value: RuntimeHostInputValue::U64(1).into_value().unwrap(),
                    intent,
                })
                .is_err()
        );
    }
}

#[test]
fn exact_index_and_count_grants_stay_independent_and_wildcard_covers_both() {
    for instance in ["cli", "term"] {
        for paths in [vec!["1"], vec!["count"], vec!["*"]] {
            let backend = Backend::with_arguments(vec![OsString::from("first")]);
            let calls = backend.calls.clone();
            let mut runtime = RuntimeBuilder::new()
                .host_factory(Box::new(CliHostFactory::with_backend(backend).unwrap()))
                .unwrap()
                .host_instance(HostInstanceConfig {
                    name: instance.into(),
                    provider: "cli".into(),
                    settings: ConfigValue::Map(BTreeMap::new()),
                })
                .run_resource_grant(RunResourceGrantConfig {
                    target: format!("{instance}/args"),
                    operations: vec!["read".into()],
                    paths: paths.iter().map(|path| (*path).into()).collect(),
                })
                .build()
                .unwrap();
            let count = runtime.read_resource(read(&format!("cli://{instance}/args"), "count"));
            let allowed_count = paths.contains(&"count") || paths.contains(&"*");
            assert_eq!(count.is_ok(), allowed_count);
            if let Ok(count) = count {
                assert_count(count.value(), 1);
            }
            assert_eq!(calls.lock().unwrap().counts, usize::from(allowed_count));
            let text = runtime.read_resource(read(&format!("cli://{instance}/args"), "1"));
            let allowed_text = paths.contains(&"1") || paths.contains(&"*");
            assert_eq!(text.is_ok(), allowed_text);
            assert_eq!(calls.lock().unwrap().arguments, usize::from(allowed_text));
            runtime.shutdown().unwrap();
        }
    }
}

#[cfg(unix)]
#[test]
fn count_includes_non_unicode_payloads_without_decoding_or_poisoning_metadata() {
    use std::os::unix::ffi::OsStringExt;
    let raw = OsString::from_vec(vec![0xff, 0x80]);
    let provider = CliResourceProvider::new(ArgumentSnapshotCliBackend::new(
        LegacyBackend,
        vec![OsString::from(""), raw, OsString::from("--once")],
    ));
    assert_count(&provider.read(read("cli://args", "count")).unwrap(), 3);
    assert!(
        provider
            .read(read("cli://args", "2"))
            .unwrap_err()
            .display_message()
            .contains("Unicode")
    );
    assert_count(&provider.read(read("cli://args", "count")).unwrap(), 3);
    assert!(
        matches!(provider.read(read("cli://args", "1")).unwrap().data(), ValueData::String(text) if text.is_empty())
    );
}
