use super::{BrowserAuthority, BrowserCapabilityRequest, BrowserHostError};

#[derive(Clone, Debug, Default)]
pub struct BrowserHost {
    authority: BrowserAuthority,
}

impl BrowserHost {
    pub fn new(authority: BrowserAuthority) -> Self {
        Self { authority }
    }

    pub fn deny_by_default() -> Self {
        Self::default()
    }

    pub fn authority(&self) -> &BrowserAuthority {
        &self.authority
    }

    pub fn authority_mut(&mut self) -> &mut BrowserAuthority {
        &mut self.authority
    }

    pub fn check(&self, request: BrowserCapabilityRequest) -> Result<(), BrowserHostError> {
        self.authority
            .check(&request)
            .map_err(BrowserHostError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::{BrowserCapabilityGrant, BrowserOperation, BrowserResource};

    #[test]
    fn browser_host_defaults_to_empty_authority() {
        let host = BrowserHost::deny_by_default();
        assert!(host.authority().grants().is_empty());
        assert!(host
            .check(BrowserCapabilityRequest::Clipboard {
                operation: BrowserOperation::Read,
            })
            .is_err());
    }

    #[test]
    fn browser_host_delegates_checks_to_authority() {
        let authority = BrowserAuthority::new([BrowserCapabilityGrant::new(
            BrowserResource::Clipboard,
            [BrowserOperation::Write],
        )]);
        let host = BrowserHost::new(authority);
        assert_eq!(
            host.check(BrowserCapabilityRequest::Clipboard {
                operation: BrowserOperation::Write,
            }),
            Ok(())
        );
    }
}

pub struct WasmBrowserDomHost;

impl WasmBrowserDomHost {
  pub fn new() -> Self {
    Self
  }
}

impl Default for WasmBrowserDomHost {
  fn default() -> Self {
    Self::new()
  }
}

impl mech_runtime::BrowserDomHost for WasmBrowserDomHost {
  fn read_dom_string(
    &self,
    entry: &mech_core::BrowserDomManifestEntry,
    requested_path: &mech_core::BrowserDomPath,
  ) -> mech_core::MResult<String> {
    let (element, property) = resolve_dom_element(entry, requested_path)?;
    match property {
      mech_core::BrowserDomProperty::Text => Ok(element.text_content().unwrap_or_default()),
      mech_core::BrowserDomProperty::Value => {
        let Some(input) = wasm_bindgen::JsCast::dyn_ref::<web_sys::HtmlInputElement>(&element) else {
          return Err(browser_dom_error(requested_path.as_str(), "DOM _value requires an input element"));
        };
        Ok(input.value())
      }
      mech_core::BrowserDomProperty::InnerHtml => Ok(element.inner_html()),
      mech_core::BrowserDomProperty::Attribute(attribute) => {
        Ok(element.get_attribute(&attribute).unwrap_or_default())
      }
    }
  }

  fn write_dom_string(
    &mut self,
    entry: &mech_core::BrowserDomManifestEntry,
    requested_path: &mech_core::BrowserDomPath,
    value: &str,
  ) -> mech_core::MResult<()> {
    let (element, property) = resolve_dom_element(entry, requested_path)?;
    match property {
      mech_core::BrowserDomProperty::Text => element.set_text_content(Some(value)),
      mech_core::BrowserDomProperty::Value => {
        let Some(input) = wasm_bindgen::JsCast::dyn_ref::<web_sys::HtmlInputElement>(&element) else {
          return Err(browser_dom_error(requested_path.as_str(), "DOM _value requires an input element"));
        };
        input.set_value(value);
      }
      mech_core::BrowserDomProperty::InnerHtml => element.set_inner_html(value),
      mech_core::BrowserDomProperty::Attribute(attribute) => element
        .set_attribute(&attribute, value)
        .map_err(|_| browser_dom_error(requested_path.as_str(), "failed to set DOM attribute"))?,
    }
    Ok(())
  }
}

fn browser_dom_error(resource: impl Into<String>, reason: impl Into<String>) -> mech_core::MechError {
  mech_core::MechError::new(
    mech_runtime::BrowserRuntimeResourceError {
      resource: resource.into(),
      reason: reason.into(),
    },
    None,
  )
}

fn browser_document() -> mech_core::MResult<web_sys::Document> {
  web_sys::window()
    .and_then(|window| window.document())
    .ok_or_else(|| browser_dom_error("browser://dom", "window.document is unavailable"))
}

fn resolve_dom_element(
  entry: &mech_core::BrowserDomManifestEntry,
  requested_path: &mech_core::BrowserDomPath,
) -> mech_core::MResult<(web_sys::Element, mech_core::BrowserDomProperty)> {
  let document = browser_document()?;
  let root = document
    .query_selector(entry.selector.selector.as_str())
    .map_err(|_| browser_dom_error(requested_path.as_str(), "configured DOM selector is invalid"))?
    .ok_or_else(|| browser_dom_error(requested_path.as_str(), "configured DOM selector did not match an element"))?;

  if !entry.path.is_wildcard() {
    return Ok((root, entry.property.clone()));
  }

  let prefix = entry
    .path
    .as_str()
    .strip_suffix("/*")
    .expect("wildcard DOM path must end in /*");
  let relative = requested_path
    .as_str()
    .strip_prefix(prefix)
    .and_then(|suffix| suffix.strip_prefix('/'))
    .ok_or_else(|| browser_dom_error(requested_path.as_str(), "requested path is outside wildcard DOM root"))?;
  let requested_relative = mech_core::BrowserDomPath::new(relative.to_string())
    .map_err(mech_core::browser_capability_error)?;
  let property = requested_relative.dom_property();
  let node_path = requested_relative.without_property_suffix();
  let mut element = root;
  if !node_path.is_empty() && !node_path.starts_with('_') {
    for segment in node_path.split('/') {
      if segment.starts_with('_') {
        break;
      }
      let selector = format!(r#"[data-mech-node="{}"]"#, segment);
      element = element
        .query_selector(&selector)
        .map_err(|_| browser_dom_error(requested_path.as_str(), "failed to query data-mech-node descendant"))?
        .ok_or_else(|| browser_dom_error(requested_path.as_str(), format!("missing data-mech-node descendant `{segment}`")))?;
    }
  }
  Ok((element, property))
}
