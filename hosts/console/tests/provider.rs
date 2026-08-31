use mech_console::{ConsoleHostFactory, ConsoleResourceProvider, RecordingConsoleBackend};
use mech_core::{
    EffectContract, EffectDeliveryPolicy, ExternalInteraction, IdempotencyRequirement,
};
use mech_runtime::{
    PreparedRuntimeEffect, RuntimeCapabilityOperation, RuntimeHostFactory, RuntimeHostInputValue,
    RuntimeResourceProvider, RuntimeResourceWriteIntent, RuntimeResourceWriteRequest,
};

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
        })
        .unwrap_err();
    assert!(format!("{err:?}").contains("line"));
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
