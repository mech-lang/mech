use mech_core::{BrowserDomManifestEntry, BrowserDomPath, MResult};

pub trait BrowserDomHost {
  fn read_dom_string(
    &self,
    entry: &BrowserDomManifestEntry,
    requested_path: &BrowserDomPath,
  ) -> MResult<String>;

  fn write_dom_string(
    &mut self,
    entry: &BrowserDomManifestEntry,
    requested_path: &BrowserDomPath,
    value: &str,
  ) -> MResult<()>;
}
