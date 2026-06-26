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
    if interface.instance.trim().is_empty() { return Err(error("HostInterfaceInvalid", "host instance name must be non-empty")); }
    if self.interfaces.contains_key(&interface.instance) { return Err(error("HostInterfaceDuplicateInstance", format!("duplicate host instance `{}`", interface.instance))); }
    let mut contexts = BTreeSet::new();
    for context in &interface.contexts {
      if context.name.trim().is_empty() { return Err(error("HostInterfaceInvalid", "host context name must be non-empty")); }
      if !contexts.insert(context.name.clone()) { return Err(error("HostInterfaceDuplicateContext", format!("duplicate host context `{}`", context.name))); }
      if context.operations.is_empty() { return Err(error("HostInterfaceInvalid", format!("host context `{}` operations must be non-empty", context.name))); }
    }
    self.interfaces.insert(interface.instance.clone(), interface);
    Ok(())
  }
  pub fn resolve(&self, target: &str) -> MResult<&MaterializedHostContext> {
    let (instance, context) = parse_host_context_target(target)?;
    let Some(interface) = self.interfaces.get(instance) else { return Err(error("HostInterfaceUnknownInstance", format!("unknown host instance `{instance}`"))); };
    interface.contexts.iter().find(|item| item.name == context).ok_or_else(|| error("HostInterfaceUnknownContext", format!("unknown context `{context}` on host instance `{instance}`")))
  }
  pub fn interface(&self, instance: &str) -> Option<&MaterializedHostInterface> { self.interfaces.get(instance) }
}

pub fn parse_host_context_target(target: &str) -> MResult<(&str, &str)> {
  let mut parts = target.split('/');
  let Some(instance) = parts.next() else { return Err(error("HostInterfaceMalformedTarget", "host context target must be `<instance>/<context>`")); };
  let Some(context) = parts.next() else { return Err(error("HostInterfaceMalformedTarget", "host context target must be `<instance>/<context>`")); };
  if instance.is_empty() || context.is_empty() || parts.next().is_some() { return Err(error("HostInterfaceMalformedTarget", format!("host context target `{target}` must be `<instance>/<context>`"))); }
  Ok((instance, context))
}

#[derive(Debug, Clone)] struct HostInterfaceError { name: &'static str, message: String }
impl MechErrorKind for HostInterfaceError { fn name(&self) -> &str { self.name } fn message(&self) -> String { self.message.clone() } }
fn error(name: &'static str, message: impl Into<String>) -> MechError { MechError::new(HostInterfaceError { name, message: message.into() }, None) }
