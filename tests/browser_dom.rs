use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use mech_core::{
  BrowserAuthority, BrowserCapabilityGrant, BrowserDomManifestEntry, BrowserDomPath,
  BrowserDomProperty, BrowserDomScope, BrowserOperation, BrowserResource, Value,
};
use mech_host_browser::{BrowserDomBackend, BrowserResourceProvider};
use mech_runtime::{MechRuntime, RuntimeCapabilityGrant, RuntimeCapabilityOperation, RuntimeConfig};

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
    self.state.borrow_mut().values.insert(path.to_string(), value.to_string());
    self
  }

  fn read_count(&self) -> usize {
    self.state.borrow().reads.len()
  }

  fn reads(&self) -> Vec<String> {
    self.state.borrow().reads.clone()
  }

  fn write_count(&self) -> usize {
    self.state.borrow().writes.len()
  }

  fn writes(&self) -> Vec<(String, String)> {
    self.state.borrow().writes.clone()
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
    Ok(state.values.get(requested_path.as_str()).cloned().unwrap_or_default())
  }

  fn write_dom_string(
    &mut self,
    _entry: &BrowserDomManifestEntry,
    requested_path: &BrowserDomPath,
    value: &str,
  ) -> mech_core::MResult<()> {
    let mut state = self.state.borrow_mut();
    state.writes.push((requested_path.as_str().to_string(), value.to_string()));
    state.values.insert(requested_path.as_str().to_string(), value.to_string());
    Ok(())
  }
}

fn runtime() -> MechRuntime {
  MechRuntime::new(RuntimeConfig::default()).unwrap()
}

fn register_browser_provider(
  runtime: &mut MechRuntime,
  authority: BrowserAuthority,
  host: FakeDomHost,
) {
  runtime
    .register_resource_provider(Box::new(BrowserResourceProvider::new(authority, host)))
    .unwrap();
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
  ));
}

fn authority(path: &str, selector: &str, allow: &[BrowserOperation]) -> BrowserAuthority {
  let mut authority = BrowserAuthority::default();
  bind_authority_path(&mut authority, path, selector, allow);
  authority
}

fn read_write_authority(path: &str, selector: &str) -> BrowserAuthority {
  authority(path, selector, &[BrowserOperation::Read, BrowserOperation::Write])
}

fn grant_runtime_context(runtime: &mut MechRuntime, operation: RuntimeCapabilityOperation, path: &str) {
  let subject = runtime.runtime_context().unwrap().subject;
  runtime
    .grant_capability(RuntimeCapabilityGrant {
      subject,
      resource: "browser://dom".to_string(),
      operations: vec![operation],
      paths: vec![path.to_string()],
    })
    .unwrap();
}

fn grant_runtime_context_read(runtime: &mut MechRuntime, path: &str) {
  grant_runtime_context(runtime, RuntimeCapabilityOperation::Read, path);
}

fn grant_runtime_context_write(runtime: &mut MechRuntime, path: &str) {
  grant_runtime_context(runtime, RuntimeCapabilityOperation::Write, path);
}

#[test]
fn runtime_binds_browser_resource_root() {
  let mut runtime = runtime();
  runtime.bind_resource_root("browser", "browser://dom/").unwrap();
  assert_eq!(
    runtime.resource_binding("browser").unwrap().root_path.as_str(),
    "",
  );
}

#[test]
fn runtime_resolves_child_path_under_browser_root() {
  let mut runtime = runtime();
  runtime.bind_resource_root("browser", "browser://dom/").unwrap();
  assert_eq!(
    runtime.resource_binding("browser").unwrap().root_path.as_str(),
    "",
  );
}

#[test]
fn runtime_resolves_child_path_under_narrow_browser_root() {
  let mut runtime = runtime();
  register_browser_provider(
    &mut runtime,
    read_write_authority("body/header/title", "#title"),
    FakeDomHost::default(),
  );
  runtime.bind_resource_root("head", "browser://dom/body/header/").unwrap();
  assert_eq!(
    runtime.resource_binding("head").unwrap().root_path.as_str(),
    "body/header",
  );
}

#[test]
fn nested_browser_resource_binding_resolves_provider_after_late_registration() {
  let mut runtime = runtime();
  let host = FakeDomHost::default().with_value("body/header/title", "Hello");
  runtime.bind_resource_root("head", "browser://dom/body/header/").unwrap();
  register_browser_provider(
    &mut runtime,
    authority("body/header/title", "#title", &[BrowserOperation::Read]),
    host.clone(),
  );
  grant_runtime_context_read(&mut runtime, "body/header/title");

  let value = runtime.read_bound_resource("head", "title").unwrap();

  assert_eq!(value.as_string().unwrap().borrow().as_str(), "Hello");
  assert_eq!(host.reads(), vec!["body/header/title".to_string()]);
}

#[test]
fn nested_browser_resource_binding_resolves_provider_when_registered_before_binding() {
  let mut runtime = runtime();
  let host = FakeDomHost::default().with_value("body/header/title", "Hello");
  register_browser_provider(
    &mut runtime,
    authority("body/header/title", "#title", &[BrowserOperation::Read]),
    host.clone(),
  );
  runtime.bind_resource_root("head", "browser://dom/body/header/").unwrap();
  grant_runtime_context_read(&mut runtime, "body/header/title");

  let value = runtime.read_bound_resource("head", "title").unwrap();

  assert_eq!(value.as_string().unwrap().borrow().as_str(), "Hello");
  assert_eq!(host.reads(), vec!["body/header/title".to_string()]);
}

#[test]
fn runtime_reads_configured_browser_dom_path() {
  let mut runtime = runtime();
  runtime.bind_resource_root("browser", "browser://dom/").unwrap();
  register_browser_provider(
    &mut runtime,
    authority("body/title", "#title", &[BrowserOperation::Read]),
    FakeDomHost::default().with_value("body/title", "Hello"),
  );
  let value = runtime.read_bound_resource("browser", "body/title").unwrap();
  assert_eq!(value.as_string().unwrap().borrow().as_str(), "Hello");
}

#[test]
fn runtime_writes_configured_browser_dom_path() {
  let mut runtime = runtime();
  runtime.bind_resource_root("browser", "browser://dom/").unwrap();
  register_browser_provider(
    &mut runtime,
    authority("body/title", "#title", &[BrowserOperation::Write]),
    FakeDomHost::default(),
  );
  runtime
    .write_bound_resource("browser", "body/title", &Value::from("Hello".to_string()))
    .unwrap();
}

#[test]
fn runtime_denies_browser_dom_read_without_read_grant() {
  let mut runtime = runtime();
  runtime.bind_resource_root("browser", "browser://dom/").unwrap();
  register_browser_provider(
    &mut runtime,
    authority("body/title", "#title", &[BrowserOperation::Write]),
    FakeDomHost::default(),
  );
  assert!(runtime.read_bound_resource("browser", "body/title").is_err());
}

#[test]
fn runtime_denies_browser_dom_write_without_write_grant() {
  let mut runtime = runtime();
  runtime.bind_resource_root("browser", "browser://dom/").unwrap();
  register_browser_provider(
    &mut runtime,
    authority("body/title", "#title", &[BrowserOperation::Read]),
    FakeDomHost::default(),
  );
  assert!(runtime
    .write_bound_resource("browser", "body/title", &Value::from("Hello".to_string()))
    .is_err());
}

#[test]
fn runtime_rejects_unknown_browser_dom_path() {
  let mut runtime = runtime();
  runtime.bind_resource_root("browser", "browser://dom/").unwrap();
  register_browser_provider(
    &mut runtime,
    authority("body/title", "#title", &[BrowserOperation::Read]),
    FakeDomHost::default(),
  );
  assert!(runtime.read_bound_resource("browser", "body/other").is_err());
}

#[test]
fn runtime_wildcard_dom_path_accepts_children() {
  let mut runtime = runtime();
  runtime.bind_resource_root("browser", "browser://dom/").unwrap();
  register_browser_provider(
    &mut runtime,
    authority("body/content/*", "#content", &[BrowserOperation::Read]),
    FakeDomHost::default().with_value("body/content/title", "Hello"),
  );
  assert!(runtime.read_bound_resource("browser", "body/content/title").is_ok());
}

#[test]
fn runtime_wildcard_dom_path_rejects_siblings() {
  let mut runtime = runtime();
  runtime.bind_resource_root("browser", "browser://dom/").unwrap();
  register_browser_provider(
    &mut runtime,
    authority("body/content/*", "#content", &[BrowserOperation::Read]),
    FakeDomHost::default(),
  );
  assert!(runtime.read_bound_resource("browser", "body/sidebar/title").is_err());
}

#[test]
fn program_browser_resource_write_uses_runtime_host() {
  let mut runtime = runtime();
  let host = FakeDomHost::default();
  register_browser_provider(
    &mut runtime,
    read_write_authority("body/header/title", "#title"),
    host.clone(),
  );
  grant_runtime_context_write(&mut runtime, "body/header/title");

  runtime
    .run_string("@browser := browser://dom/{:write(body/header/title)}\n@browser/body/header/title = \"Hello\"")
    .unwrap();

  assert_eq!(host.writes(), vec![("body/header/title".to_string(), "Hello".to_string())]);
}

#[test]
fn program_browser_resource_read_uses_runtime_host() {
  let mut runtime = runtime();
  let host = FakeDomHost::default().with_value("body/search/_value", "query");
  register_browser_provider(
    &mut runtime,
    read_write_authority("body/search/_value", "#search"),
    host.clone(),
  );
  grant_runtime_context_read(&mut runtime, "body/search/_value");

  let value = runtime
    .run_string("@browser := browser://dom/{:read(body/search/_value)}\nx := @browser/body/search/_value")
    .unwrap();

  assert_eq!(value.as_string().unwrap().borrow().as_str(), "query");
  assert_eq!(host.read_count(), 1);
}

#[test]
fn program_browser_resource_define_does_not_write() {
  let mut runtime = runtime();
  let host = FakeDomHost::default();
  register_browser_provider(
    &mut runtime,
    read_write_authority("title", "#title"),
    host.clone(),
  );
  grant_runtime_context_write(&mut runtime, "title");

  let result = runtime.run_string("@browser := browser://dom/{:write(title)}\n@browser/title := \"Hello\"");

  assert!(result.is_err());
  assert_eq!(host.write_count(), 0);
}

#[test]
fn runtime_browser_resource_binding_applies_before_following_write() {
  let mut runtime = runtime();
  let host = FakeDomHost::default();
  register_browser_provider(
    &mut runtime,
    read_write_authority("body/header/title", "#title"),
    host.clone(),
  );
  grant_runtime_context_write(&mut runtime, "body/header/title");

  runtime
    .run_string("@browser := browser://dom/{:write(body/header/title)}\n@browser/body/header/title = \"Hello\"")
    .unwrap();

  assert_eq!(host.write_count(), 1);
}

#[test]
fn program_browser_resource_write_accepts_string_variable() {
  let mut runtime = runtime();
  let host = FakeDomHost::default();
  register_browser_provider(
    &mut runtime,
    read_write_authority("body/header/title", "#title"),
    host.clone(),
  );
  grant_runtime_context_write(&mut runtime, "body/header/title");

  runtime
    .run_string(
      "@browser := browser://dom/{:write(body/header/title)}\nsome-string-var := \"Hello\"\n@browser/body/header/title = some-string-var",
    )
    .unwrap();

  assert_eq!(host.writes(), vec![("body/header/title".to_string(), "Hello".to_string())]);
}

#[test]
fn program_browser_resource_read_inside_expression() {
  let mut runtime = runtime();
  let host = FakeDomHost::default().with_value("body/search/_value", "query");
  register_browser_provider(
    &mut runtime,
    read_write_authority("body/search/_value", "#search"),
    host.clone(),
  );
  grant_runtime_context_read(&mut runtime, "body/search/_value");

  let value = runtime
    .run_string(r#"@browser := browser://dom/{:read(body/search/_value)}
message := "Search: " + @browser/body/search/_value"#)
    .unwrap();

  assert_eq!(value.as_string().unwrap().borrow().as_str(), "Search: query");
  assert_eq!(host.read_count(), 1);
}

#[test]
fn program_browser_resource_write_rhs_reads_browser_resource() {
  let mut authority = BrowserAuthority::default();
  bind_authority_path(&mut authority, "body/search/_value", "#search", &[BrowserOperation::Read]);
  bind_authority_path(&mut authority, "body/header/title", "#title", &[BrowserOperation::Write]);
  let mut runtime = runtime();
  let host = FakeDomHost::default().with_value("body/search/_value", "query");
  register_browser_provider(&mut runtime, authority, host.clone());
  grant_runtime_context_read(&mut runtime, "body/search/_value");
  grant_runtime_context_write(&mut runtime, "body/header/title");

  runtime
    .run_string(
      "@browser := browser://dom/{:read(body/search/_value), :write(body/header/title)}\n@browser/body/header/title = @browser/body/search/_value",
    )
    .unwrap();

  assert_eq!(host.read_count(), 1);
  assert_eq!(host.writes(), vec![("body/header/title".to_string(), "query".to_string())]);
}

#[test]
fn program_browser_resource_write_rhs_combines_string_and_browser_resource() {
  let mut authority = BrowserAuthority::default();
  bind_authority_path(&mut authority, "body/search/_value", "#search", &[BrowserOperation::Read]);
  bind_authority_path(&mut authority, "body/header/title", "#title", &[BrowserOperation::Write]);
  let mut runtime = runtime();
  let host = FakeDomHost::default().with_value("body/search/_value", "query");
  register_browser_provider(&mut runtime, authority, host.clone());
  grant_runtime_context_read(&mut runtime, "body/search/_value");
  grant_runtime_context_write(&mut runtime, "body/header/title");

  runtime
    .run_string(r#"@browser := browser://dom/{:read(body/search/_value), :write(body/header/title)}
@browser/body/header/title = "Search: " + @browser/body/search/_value"#)
    .unwrap();

  assert_eq!(host.read_count(), 1);
  assert_eq!(
    host.writes(),
    vec![("body/header/title".to_string(), "Search: query".to_string())],
  );
}

#[test]
fn program_browser_resource_read_inside_expression_denied_before_host_access() {
  let mut runtime = runtime();
  let host = FakeDomHost::default().with_value("body/search/_value", "query");
  register_browser_provider(
    &mut runtime,
    authority("body/search/_value", "#search", &[BrowserOperation::Write]),
    host.clone(),
  );
  grant_runtime_context_read(&mut runtime, "body/search/_value");

  let result = runtime.run_string(r#"@browser := browser://dom/{:read(body/search/_value)}
message := "Search: " + @browser/body/search/_value"#);

  assert!(result.is_err());
  assert_eq!(host.read_count(), 0);
}

#[test]
fn runtime_browser_dom_uses_generic_resource_provider_dispatch() {
  let mut runtime = runtime();
  runtime.bind_resource_root("browser", "browser://dom/").unwrap();
  let host = FakeDomHost::default().with_value("body/title", "Hello");
  register_browser_provider(
    &mut runtime,
    authority("body/title", "#title", &[BrowserOperation::Read]),
    host.clone(),
  );

  let value = runtime.read_bound_resource("browser", "body/title").unwrap();

  assert_eq!(value.as_string().unwrap().borrow().as_str(), "Hello");
  assert_eq!(host.read_count(), 1);
}

#[test]
fn prefix_context_browser_roundtrip_works_and_read_only_input_write_is_denied() {
  let mut authority = BrowserAuthority::default();
  bind_authority_path(&mut authority, "form/name/_value", "#name", &[BrowserOperation::Read]);
  bind_authority_path(&mut authority, "form/output/_value", "#output", &[BrowserOperation::Write]);
  let mut runtime = runtime();
  let host = FakeDomHost::default().with_value("form/name/_value", "Ada");
  register_browser_provider(&mut runtime, authority, host.clone());
  grant_runtime_context_read(&mut runtime, "form/name/_value");
  grant_runtime_context_write(&mut runtime, "form/output/_value");

  runtime
    .run_string(r#"@browser := browser://dom/{:read(form/name/_value), :write(form/output/_value)}
name := @browser/form/name/_value
@browser/form/output/_value = name"#)
    .unwrap();
  grant_runtime_context_write(&mut runtime, "form/name/_value");
  let denied = runtime.run_string(r#"@browser := browser://dom/{:write(form/name/_value)}
@browser/form/name/_value = "Grace""#);

  assert!(denied.is_err());
  assert_eq!(host.read_count(), 1);
  assert_eq!(host.writes(), vec![("form/output/_value".to_string(), "Ada".to_string())]);
}
