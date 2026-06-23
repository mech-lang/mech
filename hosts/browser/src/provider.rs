use mech_core::{
  browser_capability_error, BrowserAuthority, BrowserDomManifestEntry, BrowserDomPath,
  BrowserOperation, BROWSER_DOM_PROVIDER_URI, MResult, MechError, MechErrorKind, Ref, Value,
};

use mech_runtime::{RuntimeResourceProvider, RuntimeResourceReadRequest, RuntimeResourceWriteIntent, RuntimeResourceWritePreflightRequest, RuntimeResourceWriteRequest};

pub trait BrowserDomBackend: std::fmt::Debug {
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

#[derive(Debug)]
pub struct BrowserResourceProvider<B> {
  authority: BrowserAuthority,
  backend: B,
}

impl<B> BrowserResourceProvider<B> {
  pub fn new(authority: BrowserAuthority, backend: B) -> Self {
    Self { authority, backend }
  }

  pub fn authority(&self) -> &BrowserAuthority {
    &self.authority
  }

  pub fn authority_mut(&mut self) -> &mut BrowserAuthority {
    &mut self.authority
  }

  pub fn backend(&self) -> &B {
    &self.backend
  }

  pub fn backend_mut(&mut self) -> &mut B {
    &mut self.backend
  }
}

impl<B: BrowserDomBackend> BrowserResourceProvider<B> {
  fn validate_dom_base(base_uri: &str) -> MResult<()> {
    if base_uri == BROWSER_DOM_PROVIDER_URI || base_uri == "browser://dom/" {
      return Ok(());
    }
    Err(browser_resource_provider_error(
      base_uri,
      "browser provider base is unsupported in this PR; only browser://dom is supported",
    ))
  }

  fn dom_path(path: String) -> MResult<BrowserDomPath> {
    BrowserDomPath::new(path).map_err(browser_capability_error)
  }
}

impl<B: BrowserDomBackend> RuntimeResourceProvider for BrowserResourceProvider<B> {
  fn scheme(&self) -> &str {
    "browser"
  }

  fn base_uris(&self) -> Vec<String> {
    vec![BROWSER_DOM_PROVIDER_URI.to_string()]
  }

  fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
    Self::validate_dom_base(&request.base_uri)?;
    let path = Self::dom_path(request.path)?;
    let Some(entry) = self.authority.dom_entry_for_path(&path) else {
      return Err(browser_resource_provider_error(
        path.as_str(),
        "no configured DOM manifest entry for path",
      ));
    };
    self
      .authority
      .allows_dom(entry.selector.selector.as_str(), BrowserOperation::Read)
      .map_err(browser_capability_error)?;
    Ok(Value::String(Ref::new(self.backend.read_dom_string(entry, &path)?)))
  }

  fn preflight_write(&self, request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
    if request.intent != RuntimeResourceWriteIntent::Assign {
      return Err(browser_resource_provider_error(&request.base_uri, "browser DOM resources do not support send intent; use assignment"));
    }
    Self::validate_dom_base(&request.base_uri)?;
    let path = Self::dom_path(request.path)?;
    let Some(entry) = self.authority.dom_entry_for_path(&path) else {
      return Err(browser_resource_provider_error(
        path.as_str(),
        "no configured DOM manifest entry for path",
      ));
    };
    self
      .authority
      .allows_dom(entry.selector.selector.as_str(), BrowserOperation::Write)
      .map_err(browser_capability_error)
  }

  fn write(&mut self, request: RuntimeResourceWriteRequest) -> MResult<()> {
    self.preflight_write(RuntimeResourceWritePreflightRequest {
      base_uri: request.base_uri.clone(),
      path: request.path.clone(),
      context_name: request.context_name.clone(),
      intent: request.intent,
    })?;
    let path = Self::dom_path(request.path)?;
    let entry = self
      .authority
      .dom_entry_for_path(&path)
      .cloned()
      .ok_or_else(|| {
        browser_resource_provider_error(
          path.as_str(),
          "no configured DOM manifest entry for path",
        )
      })?;
    let Value::String(value) = request.value else {
      return Err(browser_resource_provider_error(
        path.as_str(),
        "only string values can be written to browser DOM resources in this PR",
      ));
    };
    self.backend.write_dom_string(&entry, &path, value.borrow().as_str())
  }
}

#[derive(Debug, Clone)]
pub struct BrowserResourceProviderError {
  pub resource: String,
  pub reason: String,
}

impl MechErrorKind for BrowserResourceProviderError {
  fn name(&self) -> &str {
    "BrowserResourceProvider"
  }

  fn message(&self) -> String {
    format!("browser resource `{}` failed: {}", self.resource, self.reason)
  }
}

fn browser_resource_provider_error(
  resource: impl Into<String>,
  reason: impl Into<String>,
) -> MechError {
  MechError::new(
    BrowserResourceProviderError {
      resource: resource.into(),
      reason: reason.into(),
    },
    None,
  )
}
