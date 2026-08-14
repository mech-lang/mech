use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;

use mech_browser::{
    BrowserAuthority, BrowserCapabilityGrant, BrowserDomManifestEntry, BrowserDomPath,
    BrowserDomProperty, BrowserDomScope, BrowserOperation, BrowserResource,
};
use mech_browser::{BrowserDomBackend, BrowserResourceProvider};
use mech_core::LegacyValue;
use mech_runtime::{
    MechRuntime, ResourcePathCapability, RuntimeBuilder, RuntimeCapabilityOperation,
    RuntimeValueSnapshot,
};

#[derive(Debug, Default)]
struct FakeDomState {
    values: BTreeMap<String, String>,
    reads: Vec<String>,
    writes: Vec<(String, String)>,
}

#[derive(Clone, Debug, Default)]
struct FakeDomHost {
    state: Rc<RefCell<FakeDomState>>,
}

impl FakeDomHost {
    fn with_value(self, path: &str, value: &str) -> Self {
        self.state
            .borrow_mut()
            .values
            .insert(path.to_string(), value.to_string());
        self
    }

    fn read_count(&self) -> usize {
        self.state.borrow().reads.len()
    }

    fn reads(&self) -> Vec<String> {
        self.state.borrow().reads.clone()
    }
}

impl BrowserDomBackend for FakeDomHost {
    fn read_dom_string(
        &self,
        _entry: &BrowserDomManifestEntry,
        requested_path: &BrowserDomPath,
    ) -> mech_core::MResult<String> {
        let mut state = self.state.borrow_mut();
        state.reads.push(requested_path.as_str().to_string());
        Ok(state
            .values
            .get(requested_path.as_str())
            .cloned()
            .unwrap_or_default())
    }

    fn write_dom_string(
        &mut self,
        _entry: &BrowserDomManifestEntry,
        requested_path: &BrowserDomPath,
        value: &str,
    ) -> mech_core::MResult<()> {
        let mut state = self.state.borrow_mut();
        state
            .writes
            .push((requested_path.as_str().to_string(), value.to_string()));
        state
            .values
            .insert(requested_path.as_str().to_string(), value.to_string());
        Ok(())
    }
}

fn runtime_with_browser_binding(
    authority: BrowserAuthority,
    host: FakeDomHost,
    name: &str,
    uri: &str,
) -> MechRuntime {
    RuntimeBuilder::new()
        .resource_provider(Box::new(BrowserResourceProvider::new(authority, host)))
        .resource_binding(name, uri)
        .unwrap()
        .build()
        .unwrap()
}

fn runtime_with_binding(name: &str, uri: &str) -> MechRuntime {
    RuntimeBuilder::new()
        .resource_binding(name, uri)
        .unwrap()
        .build()
        .unwrap()
}

fn snapshot_string(value: RuntimeValueSnapshot) -> String {
    value.into_value().as_string().unwrap().borrow().clone()
}

fn bind_authority_path(
    authority: &mut BrowserAuthority,
    path: &str,
    selector: &str,
    allow: &[BrowserOperation],
) {
    let scope = BrowserDomScope::new(selector).unwrap();
    authority.grant(BrowserCapabilityGrant::new(
        BrowserResource::Dom(scope.clone()),
        allow.iter().copied(),
    ));
    authority.bind_dom_path(BrowserDomManifestEntry::new(
        BrowserDomPath::new(path).unwrap(),
        scope,
        BrowserDomProperty::Text,
        allow.iter().copied(),
    ));
}

fn authority(path: &str, selector: &str, allow: &[BrowserOperation]) -> BrowserAuthority {
    let mut authority = BrowserAuthority::default();
    bind_authority_path(&mut authority, path, selector, allow);
    authority
}

fn read_write_authority(path: &str, selector: &str) -> BrowserAuthority {
    authority(
        path,
        selector,
        &[BrowserOperation::Read, BrowserOperation::Write],
    )
}

fn grant_runtime_context(
    runtime: &mut MechRuntime,
    operation: RuntimeCapabilityOperation,
    path: &str,
) {
    let subject = runtime.runtime_context().unwrap().subject().to_string();
    let capability = ResourcePathCapability::exact(
        runtime.next_capability_id(),
        subject,
        "browser://dom",
        [operation.name()],
        path,
    )
    .unwrap();
    runtime.grant_capability(Arc::new(capability)).unwrap();
}

fn grant_runtime_context_read(runtime: &mut MechRuntime, path: &str) {
    grant_runtime_context(runtime, RuntimeCapabilityOperation::Read, path);
}

fn grant_runtime_context_write(runtime: &mut MechRuntime, path: &str) {
    grant_runtime_context(runtime, RuntimeCapabilityOperation::Write, path);
}

#[test]
fn runtime_binds_browser_resource_root() {
    let runtime = runtime_with_binding("browser", "browser://dom/");
    assert_eq!(
        runtime
            .resource_binding("browser")
            .unwrap()
            .root_path
            .as_str(),
        "",
    );
}

#[test]
fn runtime_resolves_child_path_under_browser_root() {
    let runtime = runtime_with_binding("browser", "browser://dom/");
    assert_eq!(
        runtime
            .resource_binding("browser")
            .unwrap()
            .root_path
            .as_str(),
        "",
    );
}

#[test]
fn runtime_resolves_child_path_under_narrow_browser_root() {
    let runtime = runtime_with_browser_binding(
        read_write_authority("body/header/title", "#title"),
        FakeDomHost::default(),
        "head",
        "browser://dom/body/header/",
    );
    assert_eq!(
        runtime.resource_binding("head").unwrap().root_path.as_str(),
        "body/header",
    );
}

#[test]
fn nested_browser_resource_binding_resolves_provider_from_builder_configuration() {
    let host = FakeDomHost::default().with_value("body/header/title", "Hello");
    let mut runtime = runtime_with_browser_binding(
        authority("body/header/title", "#title", &[BrowserOperation::Read]),
        host.clone(),
        "head",
        "browser://dom/body/header/",
    );
    grant_runtime_context_read(&mut runtime, "body/header/title");

    let value = runtime.read_bound_resource("head", "title").unwrap();

    assert_eq!(snapshot_string(value), "Hello");
    assert_eq!(host.reads(), vec!["body/header/title".to_string()]);
}

#[test]
fn nested_browser_resource_binding_resolves_provider_when_registered_before_binding() {
    let host = FakeDomHost::default().with_value("body/header/title", "Hello");
    let mut runtime = runtime_with_browser_binding(
        authority("body/header/title", "#title", &[BrowserOperation::Read]),
        host.clone(),
        "head",
        "browser://dom/body/header/",
    );
    grant_runtime_context_read(&mut runtime, "body/header/title");

    let value = runtime.read_bound_resource("head", "title").unwrap();

    assert_eq!(snapshot_string(value), "Hello");
    assert_eq!(host.reads(), vec!["body/header/title".to_string()]);
}

#[test]
fn runtime_reads_configured_browser_dom_path() {
    let mut runtime = runtime_with_browser_binding(
        authority("body/title", "#title", &[BrowserOperation::Read]),
        FakeDomHost::default().with_value("body/title", "Hello"),
        "browser",
        "browser://dom/",
    );
    grant_runtime_context_read(&mut runtime, "body/title");
    let value = runtime
        .read_bound_resource("browser", "body/title")
        .unwrap();
    assert_eq!(snapshot_string(value), "Hello");
}

#[test]
fn runtime_writes_configured_browser_dom_path() {
    let mut runtime = runtime_with_browser_binding(
        authority("body/title", "#title", &[BrowserOperation::Write]),
        FakeDomHost::default(),
        "browser",
        "browser://dom/",
    );
    grant_runtime_context_write(&mut runtime, "body/title");
    runtime
        .write_bound_resource(
            "browser",
            "body/title",
            &LegacyValue::from("Hello".to_string()),
        )
        .unwrap();
}

#[test]
fn runtime_denies_browser_dom_read_without_read_grant() {
    let mut runtime = runtime_with_browser_binding(
        authority("body/title", "#title", &[BrowserOperation::Write]),
        FakeDomHost::default(),
        "browser",
        "browser://dom/",
    );
    assert!(
        runtime
            .read_bound_resource("browser", "body/title")
            .is_err()
    );
}

#[test]
fn runtime_denies_browser_dom_write_without_write_grant() {
    let mut runtime = runtime_with_browser_binding(
        authority("body/title", "#title", &[BrowserOperation::Read]),
        FakeDomHost::default(),
        "browser",
        "browser://dom/",
    );
    assert!(
        runtime
            .write_bound_resource(
                "browser",
                "body/title",
                &LegacyValue::from("Hello".to_string())
            )
            .is_err()
    );
}

#[test]
fn runtime_rejects_unknown_browser_dom_path() {
    let mut runtime = runtime_with_browser_binding(
        authority("body/title", "#title", &[BrowserOperation::Read]),
        FakeDomHost::default(),
        "browser",
        "browser://dom/",
    );
    assert!(
        runtime
            .read_bound_resource("browser", "body/other")
            .is_err()
    );
}

#[test]
fn runtime_wildcard_dom_path_accepts_children() {
    let mut runtime = runtime_with_browser_binding(
        authority("body/content/*", "#content", &[BrowserOperation::Read]),
        FakeDomHost::default().with_value("body/content/title", "Hello"),
        "browser",
        "browser://dom/",
    );
    grant_runtime_context_read(&mut runtime, "body/content/title");
    assert!(
        runtime
            .read_bound_resource("browser", "body/content/title")
            .is_ok()
    );
}

#[test]
fn runtime_wildcard_dom_path_rejects_siblings() {
    let mut runtime = runtime_with_browser_binding(
        authority("body/content/*", "#content", &[BrowserOperation::Read]),
        FakeDomHost::default(),
        "browser",
        "browser://dom/",
    );
    assert!(
        runtime
            .read_bound_resource("browser", "body/sidebar/title")
            .is_err()
    );
}

#[test]
fn runtime_browser_dom_uses_generic_resource_provider_dispatch() {
    let host = FakeDomHost::default().with_value("body/title", "Hello");
    let mut runtime = runtime_with_browser_binding(
        authority("body/title", "#title", &[BrowserOperation::Read]),
        host.clone(),
        "browser",
        "browser://dom/",
    );
    grant_runtime_context_read(&mut runtime, "body/title");

    let value = runtime
        .read_bound_resource("browser", "body/title")
        .unwrap();

    assert_eq!(snapshot_string(value), "Hello");
    assert_eq!(host.read_count(), 1);
}

#[test]
fn runtime_scopes_dom_operations_to_manifest_entry_path() {
    let mut authority = BrowserAuthority::default();
    bind_authority_path(
        &mut authority,
        "panel/text",
        "#panel",
        &[BrowserOperation::Read],
    );
    bind_authority_path(
        &mut authority,
        "panel/value",
        "#panel",
        &[BrowserOperation::Write],
    );
    let mut runtime = runtime_with_browser_binding(
        authority,
        FakeDomHost::default().with_value("panel/text", "readable"),
        "browser",
        "browser://dom/",
    );
    grant_runtime_context_read(&mut runtime, "panel/text");
    grant_runtime_context_write(&mut runtime, "panel/text");
    grant_runtime_context_write(&mut runtime, "panel/value");
    grant_runtime_context_read(&mut runtime, "panel/value");
    assert!(runtime.read_bound_resource("browser", "panel/text").is_ok());
    assert!(
        runtime
            .write_bound_resource(
                "browser",
                "panel/text",
                &LegacyValue::from("blocked".to_string())
            )
            .is_err()
    );
    assert!(
        runtime
            .write_bound_resource(
                "browser",
                "panel/value",
                &LegacyValue::from("writable".to_string())
            )
            .is_ok()
    );
    assert!(
        runtime
            .read_bound_resource("browser", "panel/value")
            .is_err()
    );
}
