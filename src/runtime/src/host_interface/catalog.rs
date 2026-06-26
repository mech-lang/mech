use std::collections::{BTreeSet, HashMap};
use mech_core::{MResult, MechError, MechErrorKind};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterializedHostInterface {
  pub instance: String,
  pub provider: String,
  pub contexts: Vec<MaterializedHostContext>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterializedHostContext {
  pub name: String,
  pub base_uri: String,
  pub operations: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct HostInterfaceCatalog {
  interfaces: HashMap<String, MaterializedHostInterface>,
}

impl HostInterfaceCatalog {
  pub fn new() -> Self { Self::default() }

  pub fn register(&mut self, interface: MaterializedHostInterface) -> MResult<()> {
    validate_host_interface(&interface)?;
    if self.interfaces.contains_key(&interface.instance) {
      return Err(error("HostInterfaceDuplicateInstance", format!("duplicate host instance `{}`", interface.instance)));
    }
    self.interfaces.insert(interface.instance.clone(), interface);
    Ok(())
  }

  pub fn has_instance(&self, instance: &str) -> bool {
    self.interfaces.contains_key(instance)
  }

  pub fn resolve_optional(&self, target: &str) -> MResult<Option<&MaterializedHostContext>> {
    let (instance, context) = parse_host_context_target(target)?;
    let Some(interface) = self.interfaces.get(instance) else {
      return Ok(None);
    };
    let Some(resolved) = interface.contexts.iter().find(|item| item.name == context) else {
      return Err(error("HostInterfaceUnknownContext", format!("unknown context `{context}` on host instance `{instance}`")));
    };
    Ok(Some(resolved))
  }

  pub fn resolve(&self, target: &str) -> MResult<&MaterializedHostContext> {
    match self.resolve_optional(target)? {
      Some(context) => Ok(context),
      None => {
        let (instance, _) = parse_host_context_target(target)?;
        Err(error("HostInterfaceUnknownInstance", format!("unknown host instance `{instance}`")))
      }
    }
  }

  pub fn interface(&self, instance: &str) -> Option<&MaterializedHostInterface> { self.interfaces.get(instance) }
}

pub fn validate_host_interface(interface: &MaterializedHostInterface) -> MResult<()> {
  if interface.instance.trim().is_empty() { return Err(error("HostInterfaceInvalid", "host instance name must be non-empty")); }
  if interface.provider.trim().is_empty() { return Err(error("HostInterfaceInvalid", "host provider name must be non-empty")); }
  let mut contexts = BTreeSet::new();
  let mut bases = BTreeSet::new();
  for context in &interface.contexts {
    if context.name.trim().is_empty() { return Err(error("HostInterfaceInvalid", "host context name must be non-empty")); }
    if context.base_uri.trim().is_empty() { return Err(error("HostInterfaceInvalid", format!("host context `{}` base URI must be non-empty", context.name))); }
    validate_uri_with_authority(&context.base_uri)?;
    if !contexts.insert(context.name.clone()) { return Err(error("HostInterfaceDuplicateContext", format!("duplicate host context `{}`", context.name))); }
    if !bases.insert(context.base_uri.clone()) { return Err(error("HostInterfaceDuplicateBaseUri", format!("duplicate host context base URI `{}`", context.base_uri))); }
    if context.operations.is_empty() { return Err(error("HostInterfaceInvalid", format!("host context `{}` operations must be non-empty", context.name))); }
    for operation in &context.operations {
      if operation != "read" && operation != "write" { return Err(error("HostInterfaceInvalid", format!("unknown host context operation `{operation}`"))); }
    }
  }
  Ok(())
}

pub fn parse_host_context_target(target: &str) -> MResult<(&str, &str)> {
  let mut parts = target.split('/');
  let Some(instance) = parts.next() else { return Err(error("HostInterfaceMalformedTarget", "host context target must be `<instance>/<context>`")); };
  let Some(context) = parts.next() else { return Err(error("HostInterfaceMalformedTarget", "host context target must be `<instance>/<context>`")); };
  if instance.is_empty() || context.is_empty() || parts.next().is_some() {
    return Err(error("HostInterfaceMalformedTarget", format!("host context target `{target}` must be `<instance>/<context>`")));
  }
  Ok((instance, context))
}

pub fn validate_uri_with_authority(uri: &str) -> MResult<()> {
  let Some((scheme, rest)) = uri.split_once("://") else {
    return Err(error("HostInterfaceInvalidUri", format!("resource URI `{uri}` must contain `://`")));
  };
  if scheme.is_empty() { return Err(error("HostInterfaceInvalidUri", format!("resource URI `{uri}` scheme cannot be empty"))); }
  let authority = rest.split('/').next().unwrap_or_default();
  if authority.is_empty() { return Err(error("HostInterfaceInvalidUri", format!("resource URI `{uri}` authority cannot be empty"))); }
  Ok(())
}

#[derive(Debug, Clone)] struct HostInterfaceError { name: &'static str, message: String }
impl MechErrorKind for HostInterfaceError { fn name(&self) -> &str { self.name } fn message(&self) -> String { self.message.clone() } }
fn error(name: &'static str, message: impl Into<String>) -> MechError { MechError::new(HostInterfaceError { name, message: message.into() }, None) }
