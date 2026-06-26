use crate::{ConfigValue, RuntimeCapabilityOperation, RuntimeResourceProvider};
use mech_core::{MResult, MechError, MechErrorKind};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq)]
pub struct HostInstanceConfig {
    pub name: String,
    pub provider: String,
    pub settings: ConfigValue,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunResourceGrantConfig {
    pub target: String,
    pub operations: Vec<RuntimeCapabilityOperation>,
    pub paths: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostManifestConfig {
    pub provider: String,
    pub contexts: Vec<HostContextManifest>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostContextManifest {
    pub name: String,
    pub base_uri_template: String,
    pub operations: Vec<RuntimeCapabilityOperation>,
}
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
    pub operations: Vec<RuntimeCapabilityOperation>,
}
#[derive(Debug)]
pub struct RuntimeHostInstallation {
    pub interface: MaterializedHostInterface,
    pub resource_providers: Vec<Box<dyn RuntimeResourceProvider>>,
}

pub trait RuntimeHostFactory: std::fmt::Debug {
    fn provider_name(&self) -> &str;
    fn manifest(&self) -> &HostManifestConfig;
    fn validate_settings(&self, instance_name: &str, settings: &ConfigValue) -> MResult<()>;
    fn instantiate(
        &self,
        instance_name: &str,
        settings: &ConfigValue,
    ) -> MResult<RuntimeHostInstallation>;
}
#[derive(Debug, Default)]
pub struct RuntimeHostFactoryRegistry {
    factories: BTreeMap<String, Box<dyn RuntimeHostFactory>>,
}
impl RuntimeHostFactoryRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn register(&mut self, factory: Box<dyn RuntimeHostFactory>) -> MResult<()> {
        let provider = factory.provider_name().to_string();
        if provider.trim().is_empty() {
            return Err(err("provider name cannot be empty"));
        }
        if self.factories.contains_key(&provider) {
            return Err(err(format!("duplicate host provider `{provider}`")));
        }
        self.factories.insert(provider, factory);
        Ok(())
    }
    pub fn get(&self, provider: &str) -> MResult<&dyn RuntimeHostFactory> {
        self.factories
            .get(provider)
            .map(|f| f.as_ref())
            .ok_or_else(|| err(format!("missing provider implementation `{provider}`")))
    }
    pub fn instantiate(&self, config: &HostInstanceConfig) -> MResult<RuntimeHostInstallation> {
        let factory = self.get(&config.provider)?;
        factory.validate_settings(&config.name, &config.settings)?;
        factory.instantiate(&config.name, &config.settings)
    }
}
#[derive(Clone, Debug, Default)]
pub struct HostInterfaceCatalog {
    interfaces: BTreeMap<String, MaterializedHostInterface>,
}
impl HostInterfaceCatalog {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn register(&mut self, interface: MaterializedHostInterface) -> MResult<()> {
        if interface.instance.trim().is_empty() {
            return Err(err("host instance name cannot be empty"));
        }
        if self.interfaces.contains_key(&interface.instance) {
            return Err(err(format!(
                "duplicate host instance `{}`",
                interface.instance
            )));
        }
        let mut seen = BTreeSet::new();
        for c in &interface.contexts {
            if !seen.insert(c.name.clone()) {
                return Err(err(format!("duplicate host context `{}`", c.name)));
            }
        }
        self.interfaces
            .insert(interface.instance.clone(), interface);
        Ok(())
    }
    pub fn resolve(&self, target: &str) -> MResult<&MaterializedHostContext> {
        let (instance, context) = target.split_once('/').ok_or_else(|| {
            err(format!(
                "host target `{target}` must be <instance>/<context>"
            ))
        })?;
        if instance.is_empty() || context.is_empty() || context.contains('/') {
            return Err(err(format!(
                "host target `{target}` must be <instance>/<context>"
            )));
        }
        let interface = self
            .interfaces
            .get(instance)
            .ok_or_else(|| err(format!("unknown host instance `{instance}`")))?;
        interface
            .contexts
            .iter()
            .find(|c| c.name == context)
            .ok_or_else(|| {
                err(format!(
                    "unknown context `{context}` on host instance `{instance}`"
                ))
            })
    }
    pub fn instances(&self) -> impl Iterator<Item = &MaterializedHostInterface> {
        self.interfaces.values()
    }
}
#[derive(Clone, Debug)]
pub struct RuntimeHostInterfaceError {
    pub reason: String,
}
impl MechErrorKind for RuntimeHostInterfaceError {
    fn name(&self) -> &str {
        "RuntimeHostInterface"
    }
    fn message(&self) -> String {
        self.reason.clone()
    }
}
fn err(reason: impl Into<String>) -> MechError {
    MechError::new(
        RuntimeHostInterfaceError {
            reason: reason.into(),
        },
        None,
    )
}
pub fn materialize_host_manifest(
    instance: &str,
    manifest: &HostManifestConfig,
) -> MaterializedHostInterface {
    MaterializedHostInterface {
        instance: instance.to_string(),
        provider: manifest.provider.clone(),
        contexts: manifest
            .contexts
            .iter()
            .map(|c| MaterializedHostContext {
                name: c.name.clone(),
                base_uri: c.base_uri_template.replace("{instance}", instance),
                operations: c.operations.clone(),
            })
            .collect(),
    }
}
