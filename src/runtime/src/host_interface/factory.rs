use super::{
    HostInstanceConfig, HostInterfaceCatalog, HostManifestConfig, MaterializedHostInterface,
    validate_host_interface, validate_host_manifest,
};
use crate::{ConfigValue, RuntimeResourceProvider};
use mech_core::{MResult, MechError, MechErrorKind};
use std::collections::HashMap;

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

#[derive(Debug)]
pub struct RuntimeHostInstallation {
    pub interface: MaterializedHostInterface,
    pub resource_providers: Vec<Box<dyn RuntimeResourceProvider>>,
}

#[derive(Debug, Default)]
pub struct RuntimeHostFactoryRegistry {
    factories: HashMap<String, Box<dyn RuntimeHostFactory>>,
}
impl RuntimeHostFactoryRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn register(&mut self, factory: Box<dyn RuntimeHostFactory>) -> MResult<()> {
        let provider = factory.provider_name().to_string();
        if provider.trim().is_empty() {
            return Err(error(
                "RuntimeHostProviderInvalid",
                "host provider name must be non-empty",
            ));
        }
        if factory.manifest().provider != provider {
            return Err(error(
                "RuntimeHostManifestProviderMismatch",
                format!(
                    "host factory `{provider}` manifest provider is `{}`",
                    factory.manifest().provider
                ),
            ));
        }
        validate_host_manifest(factory.manifest())?;
        if self.factories.contains_key(&provider) {
            return Err(error(
                "RuntimeHostProviderConflict",
                format!("host provider `{provider}` is already registered"),
            ));
        }
        self.factories.insert(provider, factory);
        Ok(())
    }
    pub fn contains_provider(&self, provider: &str) -> bool {
        self.factories.contains_key(provider)
    }

    pub fn provider_names(&self) -> impl Iterator<Item = &str> {
        self.factories.keys().map(String::as_str)
    }

    pub fn instantiate(&self, config: &HostInstanceConfig) -> MResult<RuntimeHostInstallation> {
        let Some(factory) = self.factories.get(&config.provider) else {
            return Err(error(
                "RuntimeHostProviderNotFound",
                format!("host provider `{}` is not registered", config.provider),
            ));
        };
        factory.validate_settings(&config.name, &config.settings)?;
        let installation = factory.instantiate(&config.name, &config.settings)?;
        if installation.interface.instance != config.name {
            return Err(error(
                "RuntimeHostInstallationMismatch",
                "host installation returned mismatched instance",
            ));
        }
        if installation.interface.provider != config.provider {
            return Err(error(
                "RuntimeHostInstallationMismatch",
                "host installation returned mismatched provider",
            ));
        }
        validate_host_interface(&installation.interface)?;
        let mut catalog = HostInterfaceCatalog::new();
        catalog.register(installation.interface.clone())?;
        Ok(installation)
    }
}

#[derive(Debug, Clone)]
struct RuntimeHostFactoryError {
    name: &'static str,
    message: String,
}
impl MechErrorKind for RuntimeHostFactoryError {
    fn name(&self) -> &str {
        self.name
    }
    fn message(&self) -> String {
        self.message.clone()
    }
}
fn error(name: &'static str, message: impl Into<String>) -> MechError {
    MechError::new(
        RuntimeHostFactoryError {
            name,
            message: message.into(),
        },
        None,
    )
}
