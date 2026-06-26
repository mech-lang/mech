use mech_core::{BrowserAuthority, BrowserCapabilityGrant, BrowserDomManifestEntry, BrowserDomPath, BrowserDomProperty, BrowserDomScope, BrowserOperation, BrowserResource, BROWSER_DOM_PROVIDER_URI, MResult};
use mech_host_browser::{BrowserDomBackend, BrowserResourceProvider};
use mech_runtime::{RuntimeResourceProvider, RuntimeResourceWriteIntent, RuntimeResourceWritePreflightRequest};

#[derive(Debug, Clone)]
struct TestDomBackend;

impl BrowserDomBackend for TestDomBackend {
  fn read_dom_string(&self, _entry: &BrowserDomManifestEntry, _requested_path: &BrowserDomPath) -> MResult<String> {
    Ok(String::new())
  }

  fn write_dom_string(&mut self, _entry: &BrowserDomManifestEntry, _requested_path: &BrowserDomPath, _value: &str) -> MResult<()> {
    Ok(())
  }
}

fn authority() -> BrowserAuthority {
  let selector = BrowserDomScope::new("#title").unwrap();
  let mut authority = BrowserAuthority::default();
  authority.bind_dom_path(BrowserDomManifestEntry::new(
    BrowserDomPath::new("body/header/title").unwrap(),
    selector.clone(),
    BrowserDomProperty::Text,
  ));
  authority.grant(BrowserCapabilityGrant::new(
    BrowserResource::Dom(selector),
    [BrowserOperation::Read, BrowserOperation::Write].into_iter().collect(),
  ));
  authority
}

#[test]
fn default_browser_provider_keeps_legacy_dom_base() {
  let provider = BrowserResourceProvider::new(authority(), TestDomBackend);
  let bases = provider.base_uris();
  assert!(bases.iter().any(|base| base == BROWSER_DOM_PROVIDER_URI));
  assert!(bases.iter().any(|base| base == "browser://dom/"));
  assert!(bases.iter().any(|base| base == "browser://browser/dom"));
}

#[test]
fn instance_browser_provider_advertises_instance_dom_base() {
  let provider = BrowserResourceProvider::for_instance("browser", authority(), TestDomBackend);
  assert!(provider.base_uris().iter().any(|base| base == "browser://browser/dom"));
}

#[test]
fn default_browser_provider_preflights_legacy_dom_base() {
  let provider = BrowserResourceProvider::new(authority(), TestDomBackend);
  provider.preflight_write(RuntimeResourceWritePreflightRequest {
    base_uri: BROWSER_DOM_PROVIDER_URI.to_string(),
    path: "body/header/title".to_string(),
    context_name: "ui".to_string(),
    intent: RuntimeResourceWriteIntent::Assign,
  }).unwrap();
}
