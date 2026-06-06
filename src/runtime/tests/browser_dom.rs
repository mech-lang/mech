use std::collections::BTreeMap;

use mech_core::Value;
use mech_runtime::*;

#[derive(Default)]
struct FakeDomHost {
  values: BTreeMap<String, String>,
  writes: Vec<(String, String)>,
}

impl FakeDomHost {
  fn with_value(mut self, path: &str, value: &str) -> Self {
    self.values.insert(path.to_string(), value.to_string());
    self
  }
}

impl BrowserDomHost for FakeDomHost {
  fn read_dom_string(
    &self,
    _entry: &BrowserDomManifestEntry,
    requested_path: &BrowserDomPath,
  ) -> mech_core::MResult<String> {
    Ok(self.values.get(requested_path.as_str()).cloned().unwrap_or_default())
  }

  fn write_dom_string(
    &mut self,
    _entry: &BrowserDomManifestEntry,
    requested_path: &BrowserDomPath,
    value: &str,
  ) -> mech_core::MResult<()> {
    self.writes.push((requested_path.as_str().to_string(), value.to_string()));
    self.values.insert(requested_path.as_str().to_string(), value.to_string());
    Ok(())
  }
}

fn runtime() -> MechRuntime {
  MechRuntime::new(RuntimeConfig::default()).unwrap()
}

fn authority(path: &str, selector: &str, allow: &[BrowserOperation]) -> BrowserAuthority {
  let mut authority = BrowserAuthority::default();
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
  authority
}

#[test]
fn runtime_binds_browser_resource_root() {
  let mut runtime = runtime();
  runtime.bind_resource_root("browser", "browser://dom/").unwrap();
  assert_eq!(runtime.resolve_resource_path("browser", "body/title").unwrap().as_str(), "body/title");
}

#[test]
fn runtime_resolves_child_path_under_browser_root() {
  let mut runtime = runtime();
  runtime.bind_resource_root("browser", "browser://dom/").unwrap();
  assert_eq!(runtime.resolve_resource_path("browser", "body/header/title").unwrap().as_str(), "body/header/title");
}

#[test]
fn runtime_resolves_child_path_under_narrow_root() {
  let mut runtime = runtime();
  runtime.bind_resource_root("head", "browser://dom/body/header/").unwrap();
  assert_eq!(runtime.resolve_resource_path("head", "title").unwrap().as_str(), "body/header/title");
}

#[test]
fn runtime_reads_configured_browser_dom_path() {
  let mut runtime = runtime();
  runtime.bind_resource_root("browser", "browser://dom/").unwrap();
  runtime.set_browser_authority(authority("body/title", "#title", &[BrowserOperation::Read]));
  runtime.set_browser_dom_host(Box::new(FakeDomHost::default().with_value("body/title", "Hello")));
  let value = runtime.read_browser_dom_resource("browser", "body/title").unwrap();
  assert_eq!(value.as_string().unwrap().borrow().as_str(), "Hello");
}

#[test]
fn runtime_writes_configured_browser_dom_path() {
  let mut runtime = runtime();
  runtime.bind_resource_root("browser", "browser://dom/").unwrap();
  runtime.set_browser_authority(authority("body/title", "#title", &[BrowserOperation::Write]));
  runtime.set_browser_dom_host(Box::new(FakeDomHost::default()));
  runtime.write_browser_dom_resource("browser", "body/title", &Value::from("Hello".to_string())).unwrap();
}

#[test]
fn runtime_denies_browser_dom_read_without_read_grant() {
  let mut runtime = runtime();
  runtime.bind_resource_root("browser", "browser://dom/").unwrap();
  runtime.set_browser_authority(authority("body/title", "#title", &[BrowserOperation::Write]));
  runtime.set_browser_dom_host(Box::new(FakeDomHost::default()));
  assert!(runtime.read_browser_dom_resource("browser", "body/title").is_err());
}

#[test]
fn runtime_denies_browser_dom_write_without_write_grant() {
  let mut runtime = runtime();
  runtime.bind_resource_root("browser", "browser://dom/").unwrap();
  runtime.set_browser_authority(authority("body/title", "#title", &[BrowserOperation::Read]));
  runtime.set_browser_dom_host(Box::new(FakeDomHost::default()));
  assert!(runtime.write_browser_dom_resource("browser", "body/title", &Value::from("Hello".to_string())).is_err());
}

#[test]
fn runtime_rejects_unknown_browser_dom_path() {
  let mut runtime = runtime();
  runtime.bind_resource_root("browser", "browser://dom/").unwrap();
  runtime.set_browser_authority(authority("body/title", "#title", &[BrowserOperation::Read]));
  runtime.set_browser_dom_host(Box::new(FakeDomHost::default()));
  assert!(runtime.read_browser_dom_resource("browser", "body/other").is_err());
}

#[test]
fn runtime_wildcard_dom_path_allows_child() {
  let mut runtime = runtime();
  runtime.bind_resource_root("browser", "browser://dom/").unwrap();
  runtime.set_browser_authority(authority("body/content/*", "#content", &[BrowserOperation::Read]));
  runtime.set_browser_dom_host(Box::new(FakeDomHost::default().with_value("body/content/title", "Hello")));
  assert!(runtime.read_browser_dom_resource("browser", "body/content/title").is_ok());
}

#[test]
fn runtime_wildcard_dom_path_rejects_sibling() {
  let mut runtime = runtime();
  runtime.bind_resource_root("browser", "browser://dom/").unwrap();
  runtime.set_browser_authority(authority("body/content/*", "#content", &[BrowserOperation::Read]));
  runtime.set_browser_dom_host(Box::new(FakeDomHost::default()));
  assert!(runtime.read_browser_dom_resource("browser", "body/sidebar/title").is_err());
}
