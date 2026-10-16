use std::{cell::Cell, collections::HashMap, ffi::OsString};

use mech_core::{
    DimensionExpr, EffectContract, EffectDeliveryPolicy, ExternalInteraction,
    IdempotencyRequirement, IntegerWidth, MResult, MechError, MechErrorKind, ObservationContract,
    ObservationReplayPolicy, SchemaBody, SchemaDraft, Value, ValueData,
};
use mech_runtime::{
    ConfigValue, PreparedRuntimeEffect, ResourceSchemaPreflight, RuntimeCapabilityOperation,
    RuntimeEffectId, RuntimeHostFactory, RuntimeResourceProvider, RuntimeResourceReadRequest,
    RuntimeResourceWriteCommand, RuntimeResourceWriteIntent, RuntimeResourceWritePreflightRequest,
    RuntimeResourceWriteRequest, TransactionId, value_empty, value_string,
};
use mech_terminal::{CliBackend, CliHostFactory, CliResourceProvider};

#[derive(Clone, Debug, Default)]
struct FakeCliBackend {
    env: HashMap<String, String>,
    env_error: Option<String>,
    arguments: Vec<OsString>,
    argument_error: Option<String>,
    argument_reads: Cell<usize>,
    stdin: String,
    stdin_error: Option<String>,
    stdin_reads: Cell<usize>,
    stdout: Vec<String>,
    stderr: Vec<String>,
}

impl CliBackend for FakeCliBackend {
    fn stdin_text(&self) -> MResult<String> {
        self.stdin_reads.set(self.stdin_reads.get() + 1);
        if let Some(reason) = &self.stdin_error {
            return Err(MechError::new(
                FakeCliBackendError {
                    reason: reason.clone(),
                },
                None,
            ));
        }
        Ok(self.stdin.clone())
    }

    fn argument(&self, index: usize) -> MResult<Option<OsString>> {
        self.argument_reads.set(self.argument_reads.get() + 1);
        if let Some(reason) = &self.argument_error {
            return Err(MechError::new(
                FakeCliBackendError {
                    reason: reason.clone(),
                },
                None,
            ));
        }
        Ok(self.arguments.get(index - 1).cloned())
    }
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

fn str_value(text: &str) -> Value {
    value_string(text)
}

fn effect_id() -> RuntimeEffectId {
    RuntimeEffectId {
        transaction: TransactionId::new(1),
        sequence: 0,
    }
}

fn assert_string(value: &Value, expected: &str) {
    let ValueData::String(actual) = value.data() else {
        panic!("expected a canonical string")
    };
    assert_eq!(actual.as_ref(), expected);
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
    assert_string(&value, "/tmp/home");
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

fn args_read(base: &str, path: &str) -> RuntimeResourceReadRequest {
    RuntimeResourceReadRequest {
        base_uri: base.to_string(),
        path: path.to_string(),
        context_name: "args".to_string(),
    }
}

fn stdin_read(base: &str, path: &str) -> RuntimeResourceReadRequest {
    RuntimeResourceReadRequest {
        base_uri: base.to_string(),
        path: path.to_string(),
        context_name: "stdin".to_string(),
    }
}

#[test]
fn stdin_captures_exact_unicode_or_empty_text_once_per_provider() {
    for text in ["", "héllo e\u{301} 👋\n\nlast\r\n"] {
        let mut provider = CliResourceProvider::new(FakeCliBackend {
            stdin: text.to_string(),
            ..Default::default()
        });
        assert_string(
            &provider
                .read(stdin_read("cli://cli/stdin", "text"))
                .unwrap(),
            text,
        );
        provider.backend_mut().stdin = "a later consumptive read".to_string();
        for base in ["cli://stdin", "cli://cli/stdin"] {
            assert_string(&provider.read(stdin_read(base, "text")).unwrap(), text);
        }
        assert_eq!(provider.backend().stdin_reads.get(), 1);
    }
}

#[test]
fn stdin_planning_and_invalid_paths_never_observe_the_backend() {
    let provider = CliResourceProvider::new(FakeCliBackend {
        stdin_error: Some("planning must not consume stdin".to_string()),
        ..Default::default()
    });
    for base in ["cli://stdin", "cli://cli/stdin"] {
        assert_string(&provider.plan_read(stdin_read(base, "text")).unwrap(), "");
        assert!(!provider.observation_requires_input_driver(&stdin_read(base, "text")));
        for path in [
            "", "line", "Text", "text/1", "/text", " text", "text ", "text\n",
        ] {
            assert!(provider.read(stdin_read(base, path)).is_err(), "{path}");
            assert!(
                provider.plan_read(stdin_read(base, path)).is_err(),
                "{path}"
            );
        }
        let mut spoofed = stdin_read(base, "text");
        spoofed.context_name = "args".into();
        assert!(provider.read(spoofed.clone()).is_err());
        assert!(provider.plan_read(spoofed).is_err());
    }
    assert_eq!(provider.backend().stdin_reads.get(), 0);
}

#[test]
fn stdin_retains_the_first_backend_failure() {
    let mut provider = CliResourceProvider::new(FakeCliBackend {
        stdin_error: Some("stdin is not valid Unicode".to_string()),
        ..Default::default()
    });
    let first = provider
        .read(stdin_read("cli://stdin", "text"))
        .unwrap_err();
    provider.backend_mut().stdin_error = None;
    let second = provider
        .read(stdin_read("cli://cli/stdin", "text"))
        .unwrap_err();
    assert_eq!(first.display_message(), second.display_message());
    assert!(
        second
            .display_message()
            .contains("stdin is not valid Unicode")
    );
    assert_eq!(provider.backend().stdin_reads.get(), 1);
}

#[test]
fn stdin_manifest_aliases_remain_read_only_and_instance_scoped() {
    let factory = CliHostFactory::with_backend(FakeCliBackend {
        stdin: "injected snapshot".to_string(),
        ..Default::default()
    })
    .unwrap();
    let installation = factory
        .instantiate("term", &ConfigValue::Map(Default::default()))
        .unwrap();
    let provider = &installation.resource_providers[0];
    assert_string(
        &provider
            .read(stdin_read("cli://term/stdin", "text"))
            .unwrap(),
        "injected snapshot",
    );
    assert!(provider.read(stdin_read("cli://stdin", "text")).is_err());
    assert!(provider.equivalent_base_uri_groups().is_empty());
    let provider = CliResourceProvider::new(FakeCliBackend::default());
    assert!(provider.base_uris().contains(&"cli://stdin".to_string()));
    assert!(provider.equivalent_base_uri_groups().contains(&vec![
        "cli://cli/stdin".to_string(),
        "cli://stdin".to_string(),
    ]));
    for intent in [
        RuntimeResourceWriteIntent::Assign,
        RuntimeResourceWriteIntent::Send,
    ] {
        let error = provider
            .prepare_write(RuntimeResourceWriteRequest {
                base_uri: "cli://cli/stdin".into(),
                context_name: "stdin".into(),
                path: "text".into(),
                operation: RuntimeCapabilityOperation::Write,
                value: str_value("replace"),
                intent,
                effect_id: effect_id(),
                idempotency_key: "terminal-test:stdin".into(),
            })
            .unwrap_err();
        assert!(error.display_message().contains("read-only"));
    }
    assert_eq!(provider.backend().stdin_reads.get(), 0);
}

#[test]
fn args_reads_are_one_based_raw_unicode_and_preserve_empty_arguments() {
    let provider = CliResourceProvider::new(FakeCliBackend {
        arguments: ["5500", "héllo 👋", "", "--user-option"]
            .map(OsString::from)
            .to_vec(),
        ..Default::default()
    });
    for base in ["cli://args", "cli://cli/args"] {
        for (path, expected) in [
            ("1", "5500"),
            ("2", "héllo 👋"),
            ("3", ""),
            ("4", "--user-option"),
        ] {
            assert_string(&provider.read(args_read(base, path)).unwrap(), expected);
        }
        let error = provider.read(args_read(base, "5")).unwrap_err();
        assert!(
            error
                .display_message()
                .contains("argument 5 is not present")
        );
    }
}

#[test]
fn args_planning_and_invalid_paths_never_read_the_backend() {
    let provider = CliResourceProvider::new(FakeCliBackend {
        argument_error: Some("backend must not be called while planning".to_string()),
        ..Default::default()
    });
    for base in ["cli://args", "cli://cli/args"] {
        for path in ["1", "9", &usize::MAX.to_string()] {
            assert_string(&provider.plan_read(args_read(base, path)).unwrap(), "");
            assert!(!provider.observation_requires_input_driver(&args_read(base, path)));
        }
        for path in [
            "",
            "0",
            "00",
            "01",
            "-1",
            "+1",
            "1.0",
            " 1",
            "1 ",
            "1/2",
            "１",
            "184467440737095516160",
        ] {
            assert!(provider.read(args_read(base, path)).is_err(), "{path}");
            assert!(provider.plan_read(args_read(base, path)).is_err(), "{path}");
        }
    }
    assert_eq!(provider.backend().argument_reads.get(), 0);
}

#[test]
fn args_propagate_backend_errors_and_remain_optional_for_existing_backends() {
    let provider = CliResourceProvider::new(FakeCliBackend {
        argument_error: Some("argument snapshot unavailable".to_string()),
        ..Default::default()
    });
    let error = provider.read(args_read("cli://args", "1")).unwrap_err();
    assert!(
        error
            .display_message()
            .contains("argument snapshot unavailable")
    );

    #[derive(Debug)]
    struct LegacyBackend;
    impl CliBackend for LegacyBackend {
        fn env_var(&self, _: &str) -> MResult<Option<String>> {
            Ok(None)
        }
        fn write_stdout(&mut self, _: &str) -> MResult<()> {
            Ok(())
        }
        fn write_stderr(&mut self, _: &str) -> MResult<()> {
            Ok(())
        }
    }
    let provider = CliResourceProvider::new(LegacyBackend);
    assert!(
        provider
            .read(stdin_read("cli://stdin", "text"))
            .unwrap_err()
            .display_message()
            .contains("unavailable")
    );
    assert!(
        provider
            .read(args_read("cli://args", "1"))
            .unwrap_err()
            .display_message()
            .contains("unavailable")
    );
}

#[cfg(unix)]
#[test]
fn args_reject_non_unicode_without_lossy_conversion() {
    use std::os::unix::ffi::OsStringExt;
    let provider = CliResourceProvider::new(FakeCliBackend {
        arguments: vec![OsString::from_vec(vec![b'a', 0xff])],
        ..Default::default()
    });
    let error = provider.read(args_read("cli://args", "1")).unwrap_err();
    assert!(
        error
            .display_message()
            .contains("argument 1 is not valid Unicode")
    );
}

#[test]
fn args_manifest_aliases_and_factory_use_the_same_read_only_context() {
    let factory = CliHostFactory::with_backend(FakeCliBackend {
        arguments: vec![OsString::from("native-user-value")],
        ..Default::default()
    })
    .unwrap();
    let installation = factory
        .instantiate("term", &ConfigValue::Map(Default::default()))
        .unwrap();
    let provider = &installation.resource_providers[0];
    assert_string(
        &provider.read(args_read("cli://term/args", "1")).unwrap(),
        "native-user-value",
    );
    assert!(provider.read(args_read("cli://args", "1")).is_err());
    assert!(provider.equivalent_base_uri_groups().is_empty());
    let provider = CliResourceProvider::new(FakeCliBackend::default());
    assert!(provider.base_uris().contains(&"cli://args".to_string()));
    assert!(provider.equivalent_base_uri_groups().contains(&vec![
        "cli://cli/args".to_string(),
        "cli://args".to_string()
    ]));
    for intent in [
        RuntimeResourceWriteIntent::Assign,
        RuntimeResourceWriteIntent::Send,
    ] {
        let error = provider
            .prepare_write(RuntimeResourceWriteRequest {
                base_uri: "cli://args".to_string(),
                path: "1".to_string(),
                context_name: "args".to_string(),
                operation: RuntimeCapabilityOperation::Write,
                value: str_value("replace"),
                intent,
                effect_id: effect_id(),
                idempotency_key: "terminal-test:args".to_string(),
            })
            .unwrap_err();
        assert!(error.display_message().contains("read-only"));
    }
    assert_eq!(provider.backend().argument_reads.get(), 0);
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

    assert_string(&value, "");
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
fn terminal_declares_snapshot_observation_and_send_effect_semantics() {
    let provider = CliResourceProvider::new(FakeCliBackend::default());
    let env_request = RuntimeResourceReadRequest {
        base_uri: "cli://env".to_string(),
        path: "HOME".to_string(),
        context_name: "env".to_string(),
    };

    assert!(matches!(
        provider.semantic_read_contract().unwrap().interaction,
        ExternalInteraction::Observation(ObservationContract {
            replay: ObservationReplayPolicy::CaptureAsInputFact,
        }),
    ));
    assert!(!provider.observation_requires_input_driver(&env_request));
    assert!(matches!(
        provider
            .semantic_write_contract(RuntimeResourceWriteIntent::Send)
            .unwrap()
            .interaction,
        ExternalInteraction::Effect(EffectContract {
            delivery: EffectDeliveryPolicy::AtMostOnce,
            idempotency: IdempotencyRequirement::NotRequired,
        }),
    ));
    assert!(
        provider
            .semantic_write_contract(RuntimeResourceWriteIntent::Assign)
            .is_none()
    );
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
    let provider = CliResourceProvider::new(FakeCliBackend::default());
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
                    intent,
                    effect_id: effect_id(),
                    idempotency_key: "terminal-test:0".to_owned(),
                })
                .is_err()
        );
    }
}

#[test]
fn host_prepared_output_matches_runtime_validation_buffering_and_delivery() {
    for base in ["cli://stdout", "cli://stderr"] {
        for path in ["text", "line"] {
            let provider = CliResourceProvider::new(FakeCliBackend::default());
            let mut direct = provider
                .prepare_output(RuntimeResourceWriteCommand {
                    base_uri: base.to_string(),
                    path: path.to_string(),
                    context_name: "out".to_string(),
                    operation: RuntimeCapabilityOperation::Write,
                    value: str_value("λ"),
                    intent: RuntimeResourceWriteIntent::Send,
                })
                .unwrap();
            let PreparedRuntimeEffect::AfterCommit(mut runtime) = provider
                .prepare_write(RuntimeResourceWriteRequest {
                    base_uri: base.to_string(),
                    path: path.to_string(),
                    context_name: "out".to_string(),
                    operation: RuntimeCapabilityOperation::Write,
                    value: str_value("λ"),
                    intent: RuntimeResourceWriteIntent::Send,
                    effect_id: effect_id(),
                    idempotency_key: "terminal-test:parity".to_string(),
                })
                .unwrap()
            else {
                panic!("expected after-commit terminal effect");
            };
            assert_eq!(direct.metadata(), runtime.metadata());
            assert!(provider.backend().stdout.is_empty());
            assert!(provider.backend().stderr.is_empty());
            direct.deliver().unwrap();
            runtime.deliver().unwrap();
            let expected = if path == "line" { "λ\n" } else { "λ" };
            let backend = provider.backend();
            let actual = if base == "cli://stdout" {
                &backend.stdout
            } else {
                &backend.stderr
            };
            assert_eq!(actual, &[expected, expected]);
        }
    }
    let provider = CliResourceProvider::new(FakeCliBackend::default());
    for (base, path, intent, value) in [
        (
            "cli://args",
            "1",
            RuntimeResourceWriteIntent::Send,
            str_value("x"),
        ),
        (
            "cli://stdout",
            "line",
            RuntimeResourceWriteIntent::Assign,
            str_value("x"),
        ),
        (
            "cli://stdout",
            "unknown",
            RuntimeResourceWriteIntent::Send,
            str_value("x"),
        ),
        (
            "cli://stdout",
            "line",
            RuntimeResourceWriteIntent::Send,
            value_empty(),
        ),
    ] {
        let direct = provider
            .prepare_output(RuntimeResourceWriteCommand {
                base_uri: base.to_string(),
                path: path.to_string(),
                context_name: "out".to_string(),
                operation: RuntimeCapabilityOperation::Write,
                value: value.clone(),
                intent,
            })
            .unwrap_err();
        let runtime = provider
            .prepare_write(RuntimeResourceWriteRequest {
                base_uri: base.to_string(),
                path: path.to_string(),
                context_name: "out".to_string(),
                operation: RuntimeCapabilityOperation::Write,
                value,
                intent,
                effect_id: effect_id(),
                idempotency_key: "terminal-test:parity".to_string(),
            })
            .unwrap_err();
        assert_eq!(direct.display_message(), runtime.display_message());
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
            effect_id: effect_id(),
            idempotency_key: "terminal-test:0".to_owned(),
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
            effect_id: effect_id(),
            idempotency_key: "terminal-test:0".to_owned(),
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
            effect_id: effect_id(),
            idempotency_key: "terminal-test:0".to_owned(),
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
            effect_id: effect_id(),
            idempotency_key: "terminal-test:0".to_owned(),
        },
    )
    .unwrap();
    assert_eq!(provider.backend().stdout, vec!["abc", "abc\n"]);
    assert_eq!(provider.backend().stderr, vec!["warning", "warning\n"]);
}

#[test]
fn stdout_prepare_write_buffers_until_delivery() {
    let provider = CliResourceProvider::new(FakeCliBackend::default());
    let effect = provider
        .prepare_write(RuntimeResourceWriteRequest {
            base_uri: "cli://stdout".to_string(),
            path: "line".to_string(),
            context_name: "out".to_string(),
            operation: RuntimeCapabilityOperation::Write,
            value: str_value("buffered"),
            intent: RuntimeResourceWriteIntent::Send,
            effect_id: effect_id(),
            idempotency_key: "terminal-test:0".to_owned(),
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
fn discarded_candidate_delivers_zero_terminal_output() {
    let provider = CliResourceProvider::new(FakeCliBackend::default());
    let effect = provider
        .prepare_write(RuntimeResourceWriteRequest {
            base_uri: "cli://stdout".to_string(),
            path: "line".to_string(),
            context_name: "out".to_string(),
            operation: RuntimeCapabilityOperation::Write,
            value: str_value("rejected"),
            intent: RuntimeResourceWriteIntent::Send,
            effect_id: effect_id(),
            idempotency_key: "terminal-test:0".to_owned(),
        })
        .unwrap();

    assert!(matches!(effect, PreparedRuntimeEffect::AfterCommit(_)));
    drop(effect);
    assert!(provider.backend().stdout.is_empty());
    assert!(provider.backend().stderr.is_empty());
}

#[test]
fn stdout_planning_accepts_only_scalar_strings_without_output() {
    let provider = CliResourceProvider::new(FakeCliBackend::default());
    let request = |value| RuntimeResourceWriteCommand {
        base_uri: "cli://stdout".to_string(),
        path: "line".to_string(),
        context_name: "out".to_string(),
        operation: RuntimeCapabilityOperation::Write,
        value,
        intent: RuntimeResourceWriteIntent::Send,
    };

    provider.plan_write(request(str_value("planned"))).unwrap();
    let error = provider.plan_write(request(value_empty())).unwrap_err();

    assert!(
        error
            .display_message()
            .contains("require a scalar string payload")
    );
    assert!(provider.backend().stdout.is_empty());
    assert!(provider.backend().stderr.is_empty());
}

#[test]
fn output_schema_preflight_accepts_only_scalar_strings_without_observation_or_delivery() {
    let provider = CliResourceProvider::new(FakeCliBackend::default());
    let schema = |body| {
        SchemaDraft {
            body,
            dimension_parameters: Box::new([]),
        }
        .finalize()
        .unwrap()
    };
    let string = schema(SchemaBody::String);
    let rejected = [
        schema(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
        schema(SchemaBody::Matrix {
            element: Box::new(SchemaBody::String),
            dimensions: Box::new([DimensionExpr::Constant(1), DimensionExpr::Constant(1)]),
        }),
        schema(SchemaBody::Matrix {
            element: Box::new(SchemaBody::String),
            dimensions: Box::new([DimensionExpr::Constant(0), DimensionExpr::Constant(3)]),
        }),
    ];
    for base in [
        "cli://stdout",
        "cli://stderr",
        "cli://cli/stdout",
        "cli://cli/stderr",
    ] {
        for path in ["text", "line"] {
            let request = || RuntimeResourceWritePreflightRequest {
                base_uri: base.into(),
                path: path.into(),
                context_name: "output-alias".into(),
                operation: RuntimeCapabilityOperation::Write,
                intent: RuntimeResourceWriteIntent::Send,
            };
            assert_eq!(
                provider.preflight_write_schema(request(), &string).unwrap(),
                ResourceSchemaPreflight::Validated
            );
            for payload in &rejected {
                let error = provider
                    .preflight_write_schema(request(), payload)
                    .unwrap_err();
                assert!(error.display_message().contains("scalar string payload"));
            }
        }
    }
    let backend = provider.backend();
    assert_eq!(backend.stdin_reads.get(), 0);
    assert_eq!(backend.argument_reads.get(), 0);
    assert!(backend.stdout.is_empty());
    assert!(backend.stderr.is_empty());
}

#[test]
fn output_schema_preflight_preserves_resource_direction_and_path_validation() {
    let provider = CliResourceProvider::new(FakeCliBackend::default());
    let string = SchemaDraft {
        body: SchemaBody::String,
        dimension_parameters: Box::new([]),
    }
    .finalize()
    .unwrap();
    for (base, path, intent) in [
        ("cli://env", "HOME", RuntimeResourceWriteIntent::Send),
        ("cli://args", "1", RuntimeResourceWriteIntent::Send),
        ("cli://stdin", "text", RuntimeResourceWriteIntent::Send),
        (
            "cli://other/stdout",
            "text",
            RuntimeResourceWriteIntent::Send,
        ),
        ("cli://stdout", "unknown", RuntimeResourceWriteIntent::Send),
        ("cli://stdout", "text", RuntimeResourceWriteIntent::Assign),
    ] {
        assert!(
            provider
                .preflight_write_schema(
                    RuntimeResourceWritePreflightRequest {
                        base_uri: base.into(),
                        path: path.into(),
                        context_name: "alias".into(),
                        operation: RuntimeCapabilityOperation::Write,
                        intent,
                    },
                    &string,
                )
                .is_err()
        );
    }
    let backend = provider.backend();
    assert_eq!(backend.stdin_reads.get(), 0);
    assert_eq!(backend.argument_reads.get(), 0);
    assert!(backend.stdout.is_empty());
    assert!(backend.stderr.is_empty());
}

#[test]
fn stdout_and_stderr_reject_assign_read_and_unknown_path() {
    let provider = CliResourceProvider::new(FakeCliBackend::default());
    assert!(
        provider
            .prepare_write(RuntimeResourceWriteRequest {
                base_uri: "cli://stdout".to_string(),
                path: "line".to_string(),
                context_name: "out".to_string(),
                operation: RuntimeCapabilityOperation::Write,
                value: str_value("abc"),
                intent: RuntimeResourceWriteIntent::Assign,
                effect_id: effect_id(),
                idempotency_key: "terminal-test:0".to_owned(),
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
                intent: RuntimeResourceWriteIntent::Send,
                effect_id: effect_id(),
                idempotency_key: "terminal-test:0".to_owned(),
            })
            .is_err()
    );
    assert!(provider.backend().stdout.is_empty());
    assert!(provider.backend().stderr.is_empty());
}
