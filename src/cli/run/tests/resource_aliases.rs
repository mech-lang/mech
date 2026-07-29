use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use mech_core::{MResult, Value};
#[cfg(feature = "web_host")]
use mech_core::{
    BrowserAuthority, BrowserCapabilityGrant, BrowserDomManifestEntry, BrowserDomPath,
    BrowserDomProperty, BrowserDomScope, BrowserOperation, BrowserResource,
};
#[cfg(feature = "web_host")]
use mech_host_browser::{BrowserDomBackend, BrowserResourceProvider};
use mech_host_cli::{CliBackend, CliResourceProvider};
use mech_runtime::{
    ConfigValue, HostInstanceConfig, HostManifestConfig, MechRuntime, RunResourceGrantConfig,
    RuntimeBuilder, RuntimeEventKind, RuntimeHostFactory, RuntimeHostInstallation,
    materialize_host_manifest,
};

#[derive(Clone, Debug, Default)]
struct RecordingCliState {
    env: HashMap<String, String>,
    stdout: Vec<String>,
}

#[derive(Clone, Debug)]
struct RecordingCliBackend {
    state: Arc<Mutex<RecordingCliState>>,
}

impl CliBackend for RecordingCliBackend {
    fn env_var(&self, name: &str) -> MResult<Option<String>> {
        Ok(self.state.lock().unwrap().env.get(name).cloned())
    }

    fn write_stdout(&mut self, text: &str) -> MResult<()> {
        self.state.lock().unwrap().stdout.push(text.to_string());
        Ok(())
    }

    fn write_stderr(&mut self, _text: &str) -> MResult<()> {
        Ok(())
    }
}

#[derive(Debug)]
struct RecordingCliFactory {
    manifest: HostManifestConfig,
    state: Arc<Mutex<RecordingCliState>>,
}

impl RuntimeHostFactory for RecordingCliFactory {
    fn provider_name(&self) -> &str {
        "cli"
    }

    fn manifest(&self) -> &HostManifestConfig {
        &self.manifest
    }

    fn validate_settings(&self, _instance_name: &str, _settings: &ConfigValue) -> MResult<()> {
        Ok(())
    }

    fn instantiate(
        &self,
        instance_name: &str,
        _settings: &ConfigValue,
    ) -> MResult<RuntimeHostInstallation> {
        Ok(RuntimeHostInstallation {
            interface: materialize_host_manifest(instance_name, &self.manifest)?,
            input_drivers: Vec::new(),
            resource_providers: vec![Box::new(CliResourceProvider::for_instance(
                instance_name,
                RecordingCliBackend {
                    state: self.state.clone(),
                },
            ))],
        })
    }
}

fn runtime_with_cli_instance(
    instance: &str,
    state: Arc<Mutex<RecordingCliState>>,
    grant: RunResourceGrantConfig,
) -> MechRuntime {
    RuntimeBuilder::new()
        .host_factory(Box::new(RecordingCliFactory {
            manifest: mech_host_cli::cli_host_manifest().unwrap(),
            state,
        }))
        .unwrap()
        .host_instance(HostInstanceConfig {
            name: instance.to_string(),
            provider: "cli".to_string(),
            settings: ConfigValue::Map(Default::default()),
        })
        .run_resource_grant(grant)
        .build()
        .unwrap()
}

#[test]
fn run_resource_grant_authorizes_cli_stdout_legacy_alias() {
    let state = Arc::new(Mutex::new(RecordingCliState::default()));
    let mut runtime = runtime_with_cli_instance(
        "cli",
        state.clone(),
        RunResourceGrantConfig {
            target: "cli/stdout".to_string(),
            operations: vec!["write".to_string()],
            paths: vec!["line".to_string()],
        },
    );
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();

    runtime
        .run_string_with_context(
            &mut context,
            "@out := cli://stdout{:write(line)}\n@out/line <- \"legacy alias\"\n",
        )
        .unwrap();

    assert!(state.lock().unwrap().stdout.is_empty());
    assert!(
        !context
            .events()
            .iter()
            .any(|event| matches!(event.kind, RuntimeEventKind::CapabilityDenied { .. }))
    );

    runtime.commit_runtime_transaction(&mut context).unwrap();
    assert_eq!(
        state.lock().unwrap().stdout,
        vec!["legacy alias\n".to_string()],
    );

    let error = runtime
        .run_string("@out-text := cli://stdout{:write(text)}\n@out-text/text <- \"denied\"\n")
        .unwrap_err();
    let error_kind = error.kind_name();
    assert!(
        matches!(
            error_kind.as_str(),
            "RuntimeResourceCapabilityDenied" | "CapabilityDenied"
        ),
        "unexpected error: {error:?}",
    );
    assert_eq!(state.lock().unwrap().stdout.len(), 1);
}

#[test]
fn run_resource_grant_authorizes_cli_env_legacy_alias() {
    let state = Arc::new(Mutex::new(RecordingCliState {
        env: HashMap::from([("HOME".to_string(), "/test/home".to_string())]),
        stdout: Vec::new(),
    }));
    let mut runtime = runtime_with_cli_instance(
        "cli",
        state,
        RunResourceGrantConfig {
            target: "cli/env".to_string(),
            operations: vec!["read".to_string()],
            paths: vec!["HOME".to_string()],
        },
    );

    let result = runtime
        .run_string("@env := cli://env{:read(HOME)}\nhome := @env/HOME\nhome\n")
        .unwrap();

    match result.into_value() {
        Value::String(value) => assert_eq!(value.borrow().as_str(), "/test/home"),
        value => panic!("expected string HOME value, got {value:?}"),
    }
}

#[cfg(feature = "web_host")]
#[derive(Clone, Debug, Default)]
struct RecordingBrowserBackend {
    writes: Arc<Mutex<Vec<String>>>,
}

#[cfg(feature = "web_host")]
impl BrowserDomBackend for RecordingBrowserBackend {
    fn read_dom_string(
        &self,
        _entry: &BrowserDomManifestEntry,
        _requested_path: &BrowserDomPath,
    ) -> MResult<String> {
        Ok(String::new())
    }

    fn write_dom_string(
        &mut self,
        _entry: &BrowserDomManifestEntry,
        _requested_path: &BrowserDomPath,
        value: &str,
    ) -> MResult<()> {
        self.writes.lock().unwrap().push(value.to_string());
        Ok(())
    }
}

#[cfg(feature = "web_host")]
#[derive(Debug)]
struct RecordingBrowserFactory {
    manifest: HostManifestConfig,
    authority: BrowserAuthority,
    backend: RecordingBrowserBackend,
}

#[cfg(feature = "web_host")]
impl RuntimeHostFactory for RecordingBrowserFactory {
    fn provider_name(&self) -> &str {
        "browser"
    }

    fn manifest(&self) -> &HostManifestConfig {
        &self.manifest
    }

    fn validate_settings(&self, _instance_name: &str, _settings: &ConfigValue) -> MResult<()> {
        Ok(())
    }

    fn instantiate(
        &self,
        instance_name: &str,
        _settings: &ConfigValue,
    ) -> MResult<RuntimeHostInstallation> {
        Ok(RuntimeHostInstallation {
            interface: materialize_host_manifest(instance_name, &self.manifest)?,
            input_drivers: Vec::new(),
            resource_providers: vec![Box::new(BrowserResourceProvider::for_instance(
                instance_name,
                self.authority.clone(),
                self.backend.clone(),
            ))],
        })
    }
}

#[cfg(feature = "web_host")]
fn browser_authority() -> BrowserAuthority {
    let selector = BrowserDomScope::new("#title").unwrap();
    let mut authority = BrowserAuthority::default();
    authority.bind_dom_path(BrowserDomManifestEntry::new(
        BrowserDomPath::new("body/header/title").unwrap(),
        selector.clone(),
        BrowserDomProperty::Text,
        [BrowserOperation::Write],
    ));
    authority.grant(BrowserCapabilityGrant::new(
        BrowserResource::Dom(selector),
        [BrowserOperation::Write],
    ));
    authority
}

#[test]
#[cfg(feature = "web_host")]
fn run_resource_grant_authorizes_browser_dom_legacy_alias() {
    let backend = RecordingBrowserBackend::default();
    let observed = backend.clone();
    let mut runtime = RuntimeBuilder::new()
        .host_factory(Box::new(RecordingBrowserFactory {
            manifest: mech_host_browser::browser_host_manifest().unwrap(),
            authority: browser_authority(),
            backend,
        }))
        .unwrap()
        .host_instance(HostInstanceConfig {
            name: "browser".to_string(),
            provider: "browser".to_string(),
            settings: ConfigValue::Map(Default::default()),
        })
        .run_resource_grant(RunResourceGrantConfig {
            target: "browser/dom".to_string(),
            operations: vec!["write".to_string()],
            paths: vec!["body/header/title".to_string()],
        })
        .build()
        .unwrap();

    runtime
        .run_string(
            "@dom := browser://dom{:write(body/header/title)}\n\
             @dom/body/header/title = \"legacy browser\"\n",
        )
        .unwrap();

    assert_eq!(
        observed.writes.lock().unwrap().as_slice(),
        ["legacy browser"],
    );
}

#[test]
fn run_resource_grant_does_not_cross_cli_instances() {
    let state = Arc::new(Mutex::new(RecordingCliState::default()));
    let mut runtime = runtime_with_cli_instance(
        "terminal",
        state.clone(),
        RunResourceGrantConfig {
            target: "terminal/stdout".to_string(),
            operations: vec!["write".to_string()],
            paths: vec!["line".to_string()],
        },
    );

    let error = runtime
        .run_string("@out := cli://stdout{:write(line)}\n@out/line <- \"must fail\"\n")
        .unwrap_err();
    let error_kind = error.kind_name();

    assert!(
        matches!(
            error_kind.as_str(),
            "RuntimeResourceCapabilityDenied" | "CapabilityDenied"
        ),
        "unexpected error: {error:?}",
    );
    assert!(state.lock().unwrap().stdout.is_empty());
}
