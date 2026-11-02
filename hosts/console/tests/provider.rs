use mech_console::{ConsoleHostFactory, ConsoleResourceProvider, RecordingConsoleBackend};
use mech_core::{
    DimensionExpr, EffectContract, EffectDeliveryPolicy, ExternalInteraction,
    IdempotencyRequirement, IntegerWidth, SchemaBody, SchemaDraft,
};
use mech_runtime::{
    PreparedRuntimeEffect, ResourceSchemaPreflight, RuntimeCapabilityOperation, RuntimeEffectId,
    RuntimeHostFactory, RuntimeHostInputValue, RuntimeResourceProvider, RuntimeResourceWriteIntent,
    RuntimeResourceWritePreflightRequest, RuntimeResourceWriteRequest, TransactionId,
};

fn effect_id() -> RuntimeEffectId {
    RuntimeEffectId {
        transaction: TransactionId::new(1),
        sequence: 0,
    }
}

fn deliver(
    provider: &mut ConsoleResourceProvider<RecordingConsoleBackend>,
    request: RuntimeResourceWriteRequest,
) {
    match provider.prepare_write(request).unwrap() {
        PreparedRuntimeEffect::AfterCommit(mut effect) => effect.deliver().unwrap(),
        effect => panic!("expected console after-commit effect, got {effect:?}"),
    }
}

#[test]
fn provider_writes_line_to_backend() {
    let backend = RecordingConsoleBackend::new();
    let observed = backend.clone();
    let mut provider = ConsoleResourceProvider::new("console", backend);
    deliver(
        &mut provider,
        RuntimeResourceWriteRequest {
            base_uri: "console://console/output".to_string(),
            path: "line".to_string(),
            context_name: "out".to_string(),
            operation: RuntimeCapabilityOperation::Write,
            value: RuntimeHostInputValue::String("hello".to_string())
                .into_value()
                .unwrap(),
            intent: RuntimeResourceWriteIntent::Send,
            effect_id: effect_id(),
            idempotency_key: "console-test:0".to_owned(),
        },
    );
    assert_eq!(observed.lines(), vec!["hello".to_string()]);
}

#[test]
fn provider_rejects_unknown_path() {
    let backend = RecordingConsoleBackend::new();
    let provider = ConsoleResourceProvider::new("console", backend);
    let err = provider
        .prepare_write(RuntimeResourceWriteRequest {
            base_uri: "console://console/output".to_string(),
            path: "text".to_string(),
            context_name: "out".to_string(),
            operation: RuntimeCapabilityOperation::Write,
            value: RuntimeHostInputValue::String("hello".to_string())
                .into_value()
                .unwrap(),
            intent: RuntimeResourceWriteIntent::Send,
            effect_id: effect_id(),
            idempotency_key: "console-test:0".to_owned(),
        })
        .unwrap_err();
    assert!(format!("{err:?}").contains("line"));
}

#[test]
fn schema_preflight_preserves_any_value_output_without_delivery() {
    let backend = RecordingConsoleBackend::new();
    let observed = backend.clone();
    let provider = ConsoleResourceProvider::new("console", backend);
    let schemas = [
        SchemaBody::String,
        SchemaBody::UnsignedInteger(IntegerWidth::W64),
        SchemaBody::Matrix {
            element: Box::new(SchemaBody::String),
            dimensions: Box::new([DimensionExpr::Constant(1), DimensionExpr::Constant(2)]),
        },
    ]
    .map(|body| {
        SchemaDraft {
            body,
            dimension_parameters: Box::new([]),
        }
        .finalize()
        .unwrap()
    });
    let request = |base: &str, path: &str, intent| RuntimeResourceWritePreflightRequest {
        base_uri: base.into(),
        path: path.into(),
        context_name: "out".into(),
        operation: RuntimeCapabilityOperation::Write,
        intent,
    };
    for schema in &schemas {
        for base in ["console://console/output", "console://output"] {
            assert_eq!(
                provider
                    .preflight_write_schema(
                        request(base, "line", RuntimeResourceWriteIntent::Send),
                        schema
                    )
                    .unwrap(),
                ResourceSchemaPreflight::Validated,
            );
        }
        for (base, path, intent) in [
            (
                "console://other/output",
                "line",
                RuntimeResourceWriteIntent::Send,
            ),
            ("console://output", "text", RuntimeResourceWriteIntent::Send),
            (
                "console://output",
                "line",
                RuntimeResourceWriteIntent::Assign,
            ),
        ] {
            assert!(
                provider
                    .preflight_write_schema(request(base, path, intent), schema)
                    .is_err()
            );
        }
    }
    assert!(observed.lines().is_empty());
}

#[test]
fn factory_advertises_console_provider() {
    let factory = ConsoleHostFactory::with_backend(RecordingConsoleBackend::new()).unwrap();
    assert_eq!(factory.provider_name(), "console");
    let installation = factory
        .instantiate(
            "console",
            &mech_runtime::ConfigValue::Map(Default::default()),
        )
        .unwrap();
    assert_eq!(installation.interface.provider, "console");
    assert_eq!(installation.resource_providers.len(), 1);
}

#[test]
fn console_send_contract_is_at_most_once_without_idempotency() {
    let provider = ConsoleResourceProvider::new("console", RecordingConsoleBackend::new());
    let contract = provider
        .semantic_write_contract(RuntimeResourceWriteIntent::Send)
        .unwrap();

    assert!(matches!(
        &contract.interaction,
        ExternalInteraction::Effect(EffectContract {
            delivery: EffectDeliveryPolicy::AtMostOnce,
            idempotency: IdempotencyRequirement::NotRequired,
        })
    ));
    assert!(
        provider
            .semantic_write_contract(RuntimeResourceWriteIntent::Assign)
            .is_none()
    );
    assert!(!provider.supports_resident_idempotency(RuntimeResourceWriteIntent::Send));
}

#[test]
fn default_console_provider_declares_output_alias_equivalence() {
    let provider = ConsoleResourceProvider::new("console", RecordingConsoleBackend::new());

    assert_eq!(
        provider.base_uris(),
        vec![
            "console://console/output".to_string(),
            "console://output".to_string(),
        ],
    );
    assert_eq!(
        provider.equivalent_base_uri_groups(),
        vec![vec![
            "console://console/output".to_string(),
            "console://output".to_string(),
        ]],
    );
}

#[test]
fn non_default_console_provider_does_not_declare_legacy_alias() {
    let provider = ConsoleResourceProvider::new("terminal", RecordingConsoleBackend::new());

    assert_eq!(
        provider.base_uris(),
        vec!["console://terminal/output".to_string()],
    );
    assert!(provider.equivalent_base_uri_groups().is_empty());
}
