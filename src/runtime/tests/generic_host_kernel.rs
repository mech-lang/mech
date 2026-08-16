use std::sync::Arc;

use mech_core::{LegacyValue, MResult, MechError, MechErrorKind, Ref};
use mech_runtime::*;

#[derive(Debug, Clone)]
struct FakeHostError {
    message: String,
}

impl MechErrorKind for FakeHostError {
    fn name(&self) -> &str {
        "FakeHostError"
    }

    fn message(&self) -> String {
        self.message.clone()
    }
}

fn fake_error(message: impl Into<String>) -> MechError {
    MechError::new(
        FakeHostError {
            message: message.into(),
        },
        None,
    )
}

#[derive(Debug)]
struct FakeHostFactory {
    manifest: HostManifestConfig,
}

impl FakeHostFactory {
    fn new(provider: &str, context: &str, operations: &[&str]) -> Self {
        Self {
            manifest: HostManifestConfig {
                provider: provider.to_owned(),
                contexts: vec![HostContextManifest {
                    name: context.to_owned(),
                    base_uri_template: format!("{provider}://{{instance}}/{context}"),
                    operations: operations
                        .iter()
                        .map(|operation| (*operation).to_owned())
                        .collect(),
                }],
            },
        }
    }
}

impl RuntimeHostFactory for FakeHostFactory {
    fn provider_name(&self) -> &str {
        &self.manifest.provider
    }

    fn manifest(&self) -> &HostManifestConfig {
        &self.manifest
    }

    fn validate_settings(&self, _instance_name: &str, settings: &ConfigValue) -> MResult<()> {
        if matches!(settings, ConfigValue::Map(_)) {
            Ok(())
        } else {
            Err(fake_error("settings must be a map"))
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
            resource_providers: Vec::new(),
        })
    }
}

#[derive(Debug)]
struct AliasProvider {
    bases: Vec<String>,
}

impl AliasProvider {
    fn new(bases: &[&str]) -> Self {
        Self {
            bases: bases.iter().map(|base| (*base).to_owned()).collect(),
        }
    }
}

impl RuntimeResourceProvider for AliasProvider {
    fn scheme(&self) -> &str {
        "test"
    }

    fn base_uris(&self) -> Vec<String> {
        self.bases.clone()
    }

    fn read(&self, _request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        Ok(LegacyValue::String(Ref::new("ok".to_owned())))
    }
}

fn runtime_with_alias_provider() -> MechRuntime {
    RuntimeBuilder::new()
        .resource_provider(Box::new(AliasProvider::new(&[
            "test://default/context",
            "test://context",
        ])))
        .build()
        .unwrap()
}

fn grant_test_read(runtime: &mut MechRuntime, resource: &str) {
    let capability = ResourcePathCapability::exact(
        runtime.next_capability_id(),
        "subject",
        resource,
        ["read"],
        "item",
    )
    .unwrap();
    runtime.grant_capability(Arc::new(capability)).unwrap();
}

fn allows_test_read(runtime: &mut MechRuntime, resource: &str, path: &str) -> bool {
    runtime
        .check_capability(&CapabilityRequest::from_keys(
            "subject",
            "read",
            format!("{}/{}", resource.trim_end_matches('/'), path),
        ))
        .is_ok()
}

#[test]
fn duplicate_host_instance_registration_fails_generically() {
    let error = RuntimeBuilder::new()
        .host_factory(Box::new(FakeHostFactory::new(
            "browser",
            "dom",
            &["read", "write"],
        )))
        .unwrap()
        .host_factory(Box::new(FakeHostFactory::new(
            "fake-robot",
            "commands",
            &["write"],
        )))
        .unwrap()
        .host_instance(HostInstanceConfig {
            name: "shared".to_owned(),
            provider: "browser".to_owned(),
            settings: ConfigValue::Map(Default::default()),
        })
        .host_instance(HostInstanceConfig {
            name: "shared".to_owned(),
            provider: "fake-robot".to_owned(),
            settings: ConfigValue::Map(Default::default()),
        })
        .build()
        .expect_err("duplicate host instance registration should fail");
    let error = format!("{error:?}");
    assert!(error.contains("shared"), "got {error}");
    assert!(
        error.contains("duplicate") || error.contains("already"),
        "got {error}",
    );
}

#[test]
fn provider_advertised_alias_grant_does_not_change_resource_identity() {
    let mut runtime = runtime_with_alias_provider();
    grant_test_read(&mut runtime, "test://context");

    assert!(!allows_test_read(
        &mut runtime,
        "test://default/context",
        "item",
    ));
    assert!(allows_test_read(&mut runtime, "test://context", "item"));
}

#[test]
fn provider_advertised_materialized_grant_does_not_authorize_alias() {
    let mut runtime = runtime_with_alias_provider();
    grant_test_read(&mut runtime, "test://default/context");

    assert!(!allows_test_read(&mut runtime, "test://context", "item"));
}

#[test]
fn provider_advertised_alias_grant_does_not_authorize_unregistered_base() {
    let mut runtime = runtime_with_alias_provider();
    grant_test_read(&mut runtime, "test://context");

    assert!(!allows_test_read(
        &mut runtime,
        "test://other/context",
        "item",
    ));
}

#[test]
fn provider_advertised_alias_grants_do_not_use_string_heuristics() {
    let mut runtime = RuntimeBuilder::new()
        .resource_provider(Box::new(AliasProvider::new(&["test://context"])))
        .build()
        .unwrap();
    grant_test_read(&mut runtime, "test://context");

    assert!(!allows_test_read(
        &mut runtime,
        "test://default/context",
        "item",
    ));
}

#[test]
fn multiple_provider_bases_are_not_implicit_aliases() {
    let mut runtime = RuntimeBuilder::new()
        .resource_provider(Box::new(AliasProvider::new(&[
            "test://default/context",
            "test://context",
        ])))
        .build()
        .unwrap();
    grant_test_read(&mut runtime, "test://context");

    assert!(!allows_test_read(
        &mut runtime,
        "test://default/context",
        "item",
    ));
}

#[test]
fn in_memory_docs_bases_are_not_implicit_aliases() {
    let mut docs = InMemoryDocsProvider::new();
    docs.insert(
        "docs://manual",
        "title",
        LegacyValue::String(Ref::new("manual".to_owned())),
    )
    .unwrap();
    docs.insert(
        "docs://guide",
        "title",
        LegacyValue::String(Ref::new("guide".to_owned())),
    )
    .unwrap();

    let mut runtime = RuntimeBuilder::new()
        .resource_provider(Box::new(docs))
        .build()
        .unwrap();
    let capability = ResourcePathCapability::exact(
        runtime.next_capability_id(),
        "subject",
        "docs://manual",
        ["read"],
        "title",
    )
    .unwrap();
    runtime.grant_capability(Arc::new(capability)).unwrap();

    assert!(!allows_test_read(&mut runtime, "docs://guide", "title"));
}
