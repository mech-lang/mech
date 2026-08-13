use std::sync::Arc;

use mech_console::{ConsoleHostFactory, ConsoleResourceProvider, RecordingConsoleBackend};
use mech_core::{
    EffectContract, EffectDeliveryPolicy, ExternalInteraction, FunctionCatalog,
    FunctionCatalogBuilder, IdempotencyRequirement, LegacyValue,
};
use mech_runtime::{
    ConfigValue, HostInstanceConfig, MechRuntime, PreparedRuntimeEffect, RunResourceGrantConfig,
    RuntimeBuilder, RuntimeCapabilityOperation, RuntimeEventKind, RuntimeHealth,
    RuntimeHostFactory, RuntimeResourceProvider, RuntimeResourceWriteIntent,
    RuntimeResourceWriteRequest,
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
            value: LegacyValue::from("hello".to_string()),
            intent: RuntimeResourceWriteIntent::Send,
        },
    );
    assert_eq!(observed.lines(), vec!["hello".to_string()]);
}

#[test]
fn console_send_declares_at_most_once_delivery() {
    let provider = ConsoleResourceProvider::new("console", RecordingConsoleBackend::new());
    let contract = provider
        .semantic_write_contract(RuntimeResourceWriteIntent::Send)
        .unwrap();

    assert!(matches!(
        contract.interaction,
        ExternalInteraction::Effect(EffectContract {
            delivery: EffectDeliveryPolicy::AtMostOnce,
            idempotency: IdempotencyRequirement::NotRequired,
        })
    ));
}

#[test]
fn provider_rejects_unknown_path() {
    let backend = RecordingConsoleBackend::new();
    let mut provider = ConsoleResourceProvider::new("console", backend);
    let err = provider
        .prepare_write(RuntimeResourceWriteRequest {
            base_uri: "console://console/output".to_string(),
            path: "text".to_string(),
            context_name: "out".to_string(),
            operation: RuntimeCapabilityOperation::Write,
            value: LegacyValue::from("hello".to_string()),
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

fn runtime_with_console_instance(
    instance: &str,
    backend: RecordingConsoleBackend,
    grant: RunResourceGrantConfig,
) -> MechRuntime {
    RuntimeBuilder::new()
        .function_catalog(intrinsic_source_catalog())
        .host_factory(Box::new(ConsoleHostFactory::with_backend(backend).unwrap()))
        .unwrap()
        .host_instance(HostInstanceConfig {
            name: instance.to_string(),
            provider: "console".to_string(),
            settings: ConfigValue::Map(Default::default()),
        })
        .run_resource_grant(grant)
        .build()
        .unwrap()
}

fn intrinsic_source_catalog() -> Arc<FunctionCatalog> {
    let mut builder = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_runtime(&mut builder).unwrap();
    mech_engine::install_intrinsic_source(&mut builder).unwrap();
    Arc::new(builder.build().unwrap())
}

#[test]
fn run_resource_grant_authorizes_console_output_legacy_alias() {
    let backend = RecordingConsoleBackend::new();
    let observed = backend.clone();
    let mut runtime = runtime_with_console_instance(
        "console",
        backend,
        RunResourceGrantConfig {
            target: "console/output".to_string(),
            operations: vec!["write".to_string()],
            paths: vec!["line".to_string()],
        },
    );
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();

    runtime
        .run_string_with_context(
            &mut context,
            "@out := console://output{:write(line)}\n\
       @out/line <- \"legacy console\"\n",
        )
        .unwrap();

    assert!(observed.lines().is_empty());
    assert!(
        !context
            .events()
            .iter()
            .any(|event| matches!(event.kind, RuntimeEventKind::CapabilityDenied { .. })),
    );

    runtime.commit_runtime_transaction(&mut context).unwrap();
    assert_eq!(observed.lines(), vec!["legacy console".to_string()]);

    let error = runtime
        .run_string(
            "@out := console://output{:write(text)}\n\
       @out/text <- \"denied\"\n",
        )
        .unwrap_err();
    assert!(
        matches!(
            error.kind_name().as_str(),
            "RuntimeResourceCapabilityDenied" | "CapabilityDenied"
        ),
        "unexpected error: {error:?}",
    );
    assert_eq!(observed.lines(), vec!["legacy console".to_string()]);
    assert_eq!(runtime.runtime_health(), RuntimeHealth::Healthy);
}

#[test]
fn run_resource_grant_does_not_cross_console_instances() {
    let backend = RecordingConsoleBackend::new();
    let observed = backend.clone();
    let mut runtime = runtime_with_console_instance(
        "terminal",
        backend,
        RunResourceGrantConfig {
            target: "terminal/output".to_string(),
            operations: vec!["write".to_string()],
            paths: vec!["line".to_string()],
        },
    );

    let error = runtime
        .run_string(
            "@out := console://output{:write(line)}\n\
       @out/line <- \"must fail\"\n",
        )
        .unwrap_err();

    assert!(
        format!("{error:?}").contains("console://output"),
        "unexpected error: {error:?}",
    );
    assert!(observed.lines().is_empty());
    assert_eq!(runtime.runtime_health(), RuntimeHealth::Healthy);
}
