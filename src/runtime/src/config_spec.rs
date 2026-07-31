use mech_core::{MResult, MechErrorKind, Value};

use crate::{InMemoryDocsProvider, RuntimeCapabilityGrantRegistry, RuntimeCapabilityOperation, RuntimeResourceRegistry};

#[derive(Clone, Debug, Default)]
pub struct RuntimeConfigSpec {
  pub resources: Vec<RuntimeResourceConfigSpec>,
  pub capability_grants: Vec<RuntimeCapabilityGrantSpec>,
}

impl RuntimeConfigSpec {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn with_resource(
    mut self,
    resource: RuntimeResourceConfigSpec,
  ) -> Self {
    self.resources.push(resource);
    self
  }

  pub fn with_capability_grant(
    mut self,
    grant: RuntimeCapabilityGrantSpec,
  ) -> Self {
    self.capability_grants.push(grant);
    self
  }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeCapabilityGrantSpec {
  pub subject: String,
  pub resource: String,
  pub operations: Vec<RuntimeCapabilityOperation>,
  pub paths: Vec<String>,
}

impl RuntimeCapabilityGrantSpec {
  pub fn new(
    subject: impl Into<String>,
    resource: impl Into<String>,
  ) -> Self {
    Self {
      subject: subject.into(),
      resource: resource.into(),
      operations: Vec::new(),
      paths: Vec::new(),
    }
  }

  pub fn with_operation(
    mut self,
    operation: RuntimeCapabilityOperation,
  ) -> Self {
    self.operations.push(operation);
    self
  }

  pub fn with_operation_name(
    mut self,
    operation: impl Into<String>,
  ) -> MResult<Self> {
    self.operations.push(RuntimeCapabilityOperation::from_name(operation)?);
    Ok(self)
  }

  pub fn with_path(
    mut self,
    path: impl Into<String>,
  ) -> Self {
    self.paths.push(path.into());
    self
  }
}

#[derive(Clone, Debug)]
pub enum RuntimeResourceConfigSpec {
  InMemoryDocs(RuntimeInMemoryDocsResourceSpec),
}

#[derive(Clone, Debug)]
pub struct RuntimeInMemoryDocsResourceSpec {
  pub base_uri: String,
  pub entries: Vec<RuntimeDocsEntrySpec>,
}

impl RuntimeInMemoryDocsResourceSpec {
  pub fn new(base_uri: impl Into<String>) -> Self {
    Self {
      base_uri: base_uri.into(),
      entries: Vec::new(),
    }
  }

  pub fn with_entry(
    mut self,
    path: impl Into<String>,
    value: Value,
  ) -> Self {
    self.entries.push(RuntimeDocsEntrySpec {
      path: path.into(),
      value,
    });
    self
  }
}

#[derive(Clone, Debug)]
pub struct RuntimeDocsEntrySpec {
  pub path: String,
  pub value: Value,
}

#[derive(Debug, Clone)]
pub struct RuntimeConfigSpecInvalidResource {
  pub reason: String,
}

impl MechErrorKind for RuntimeConfigSpecInvalidResource {
  fn name(&self) -> &str {
    "RuntimeConfigSpecInvalidResource"
  }

  fn message(&self) -> String {
    format!("invalid runtime config resource: {}", self.reason)
  }
}

pub fn register_config_spec_resources(
  registry: &mut RuntimeResourceRegistry,
  spec: &RuntimeConfigSpec,
) -> MResult<()> {
  let mut docs_provider = InMemoryDocsProvider::new();
  let mut has_docs = false;

  for resource in &spec.resources {
    match resource {
      RuntimeResourceConfigSpec::InMemoryDocs(docs) => {
        has_docs = true;
        for entry in &docs.entries {
          docs_provider.insert(docs.base_uri.clone(), entry.path.clone(), entry.value.clone())?;
        }
      }
    }
  }

  if has_docs {
    registry.register_provider(Box::new(docs_provider))?;
  }

  Ok(())
}

pub fn register_config_spec_grants(
  registry: &mut RuntimeCapabilityGrantRegistry,
  spec: &RuntimeConfigSpec,
) -> MResult<()> {
  for grant in &spec.capability_grants {
    registry.add_spec(grant.clone())?;
  }
  Ok(())
}
