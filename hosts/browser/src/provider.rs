use crate::{
    BROWSER_DOM_PROVIDER_URI, BrowserAuthority, BrowserDomManifestEntry, BrowserDomPath,
    BrowserHostConfig, BrowserHostRuntimeConfig, BrowserOperation, browser_capability_error,
};
use mech_core::{
    AccessMode, DeliveryMode, EffectContract, EffectDeliveryPolicy, ExternalInteraction,
    IdempotencyRequirement, InputPortLayout, InputPortPolicy, MResult, MechError, MechErrorKind,
    OperationContractDeclaration, Value, ValueCell, ValueData,
};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};

use mech_runtime::{
    PreparedRuntimeEffect, RuntimeAfterCommitEffect, RuntimeEffectCost, RuntimeEffectMetadata,
    RuntimeEffectSource, RuntimeResourceProvider, RuntimeResourceReadRequest,
    RuntimeResourceWriteIntent, RuntimeResourceWritePreflightRequest, RuntimeResourceWriteRequest,
};

static BROWSER_DOM_EFFECT_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![InputPortPolicy {
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            }]
            .into_boxed_slice(),
        ),
        outputs: Box::new([]),
        interaction: ExternalInteraction::Effect(EffectContract {
            delivery: EffectDeliveryPolicy::AtMostOnce,
            idempotency: IdempotencyRequirement::NotRequired,
        }),
    });

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
    instance: String,
    authority: BrowserAuthority,
    backend: Arc<Mutex<B>>,
}

impl<B> BrowserResourceProvider<B> {
    pub fn new(authority: BrowserAuthority, backend: B) -> Self {
        Self::for_instance("browser", authority, backend)
    }

    pub fn for_instance(
        instance: impl Into<String>,
        authority: BrowserAuthority,
        backend: B,
    ) -> Self {
        Self {
            instance: instance.into(),
            authority,
            backend: Arc::new(Mutex::new(backend)),
        }
    }

    fn dom_base(&self) -> String {
        format!("browser://{}/dom", self.instance)
    }

    fn matches_dom_base(&self, base_uri: &str) -> bool {
        base_uri == self.dom_base()
            || (self.instance == "browser"
                && (base_uri == BROWSER_DOM_PROVIDER_URI || base_uri == "browser://dom/"))
    }

    pub fn authority(&self) -> &BrowserAuthority {
        &self.authority
    }

    pub fn authority_mut(&mut self) -> &mut BrowserAuthority {
        &mut self.authority
    }

    pub fn backend(&self) -> MutexGuard<'_, B> {
        self.backend
            .lock()
            .expect("browser DOM backend lock poisoned")
    }

    pub fn backend_mut(&mut self) -> MutexGuard<'_, B> {
        self.backend
            .lock()
            .expect("browser DOM backend lock poisoned")
    }
}

impl<B: BrowserDomBackend> BrowserResourceProvider<B> {
    fn dom_path(path: String) -> MResult<BrowserDomPath> {
        BrowserDomPath::new(path).map_err(browser_capability_error)
    }

    fn validate_read(&self, base_uri: &str, path: String) -> MResult<BrowserDomPath> {
        if !self.matches_dom_base(base_uri) {
            return Err(browser_resource_provider_error(
                base_uri,
                "unsupported browser DOM base URI",
            ));
        }
        let path = Self::dom_path(path)?;
        let Some(entry) = self.authority.dom_entry_for_path(&path) else {
            return Err(browser_resource_provider_error(
                path.as_str(),
                "no configured DOM manifest entry for path",
            ));
        };
        if !entry.allows_operation(BrowserOperation::Read) {
            return Err(browser_resource_provider_error(
                path.as_str(),
                "DOM manifest entry does not allow read",
            ));
        }
        self.authority
            .allows_dom(entry.selector.selector.as_str(), BrowserOperation::Read)
            .map_err(browser_capability_error)?;
        Ok(path)
    }
}

impl<B: BrowserDomBackend + 'static> RuntimeResourceProvider for BrowserResourceProvider<B> {
    fn scheme(&self) -> &str {
        "browser"
    }

    fn base_uris(&self) -> Vec<String> {
        let mut bases = vec![self.dom_base()];
        if self.instance == "browser" {
            bases.push(BROWSER_DOM_PROVIDER_URI.to_string());
            bases.push("browser://dom/".to_string());
        }
        bases
    }

    fn semantic_read_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(mech_runtime::resource_observation_contract())
    }

    fn observation_requires_input_driver(&self, _request: &RuntimeResourceReadRequest) -> bool {
        false
    }

    fn semantic_write_contract(
        &self,
        intent: RuntimeResourceWriteIntent,
    ) -> Option<&'static OperationContractDeclaration> {
        (intent == RuntimeResourceWriteIntent::Assign).then_some(&BROWSER_DOM_EFFECT_CONTRACT)
    }

    fn equivalent_base_uri_groups(&self) -> Vec<Vec<String>> {
        if self.instance != "browser" {
            return Vec::new();
        }

        vec![vec![
            self.dom_base(),
            BROWSER_DOM_PROVIDER_URI.to_string(),
            "browser://dom/".to_string(),
        ]]
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        let path = self.validate_read(&request.base_uri, request.path)?;
        let entry = self
            .authority
            .dom_entry_for_path(&path)
            .expect("validated browser DOM manifest entry");
        ValueCell::from_exact(
            self.backend
                .lock()
                .map_err(|_| {
                    browser_resource_provider_error(
                        path.as_str(),
                        "browser DOM backend lock poisoned",
                    )
                })?
                .read_dom_string(entry, &path)?,
        )?
        .snapshot()
    }

    fn plan_read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        self.validate_read(&request.base_uri, request.path)?;
        ValueCell::from_exact(String::new())?.snapshot()
    }

    fn preflight_write(&self, request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
        if request.intent != RuntimeResourceWriteIntent::Assign {
            return Err(browser_resource_provider_error(
                &request.base_uri,
                "browser DOM resources do not support send intent; use assignment",
            ));
        }
        if !self.matches_dom_base(&request.base_uri) {
            return Err(browser_resource_provider_error(
                &request.base_uri,
                "unsupported browser DOM base URI",
            ));
        }
        let path = Self::dom_path(request.path)?;
        let Some(entry) = self.authority.dom_entry_for_path(&path) else {
            return Err(browser_resource_provider_error(
                path.as_str(),
                "no configured DOM manifest entry for path",
            ));
        };
        if !entry.allows_operation(BrowserOperation::Write) {
            return Err(browser_resource_provider_error(
                path.as_str(),
                "DOM manifest entry does not allow write",
            ));
        }
        self.authority
            .allows_dom(entry.selector.selector.as_str(), BrowserOperation::Write)
            .map_err(browser_capability_error)
    }

    fn prepare_write(
        &self,
        request: RuntimeResourceWriteRequest,
    ) -> MResult<PreparedRuntimeEffect> {
        self.preflight_write(RuntimeResourceWritePreflightRequest {
            base_uri: request.base_uri.clone(),
            path: request.path.clone(),
            context_name: request.context_name.clone(),
            operation: request.operation.clone(),
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
        let ValueData::String(value) = request.value.data() else {
            return Err(browser_resource_provider_error(
                "browser/dom",
                "browser DOM assignments require a scalar string payload",
            ));
        };
        let value = value.to_string();
        Ok(PreparedRuntimeEffect::AfterCommit(Box::new(
            BrowserDomWriteEffect {
                backend: self.backend.clone(),
                entry,
                path,
                value,
                resource: request.base_uri,
            },
        )))
    }

    fn plan_write(&self, request: RuntimeResourceWriteRequest) -> MResult<()> {
        self.preflight_write(RuntimeResourceWritePreflightRequest {
            base_uri: request.base_uri,
            path: request.path,
            context_name: request.context_name,
            operation: request.operation,
            intent: request.intent,
        })?;
        if !matches!(request.value.data(), ValueData::String(_)) {
            return Err(browser_resource_provider_error(
                "browser/dom",
                "browser DOM assignments require a scalar string payload",
            ));
        }
        Ok(())
    }
}

#[derive(Debug)]
struct BrowserDomWriteEffect<B: BrowserDomBackend> {
    backend: Arc<Mutex<B>>,
    entry: BrowserDomManifestEntry,
    path: BrowserDomPath,
    value: String,
    resource: String,
}

impl<B: BrowserDomBackend> RuntimeAfterCommitEffect for BrowserDomWriteEffect<B> {
    fn metadata(&self) -> RuntimeEffectMetadata {
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::ResourceProvider {
                scheme: "browser".to_string(),
            },
            "dom-write",
        )
        .with_resource(format!("{}/{}", self.resource, self.path.as_str()))
        .with_cost(RuntimeEffectCost {
            bytes: u64::try_from(self.value.len()).unwrap_or(u64::MAX),
            items: 1,
        })
    }

    fn deliver(&mut self) -> MResult<()> {
        self.backend
            .lock()
            .map_err(|_| {
                browser_resource_provider_error(
                    self.path.as_str(),
                    "browser DOM backend lock poisoned",
                )
            })?
            .write_dom_string(&self.entry, &self.path, &self.value)
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
        format!(
            "browser resource `{}` failed: {}",
            self.resource, self.reason
        )
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

#[derive(Debug)]
pub struct BrowserHostFactory<B: BrowserDomBackend + Clone + 'static> {
    manifest: mech_runtime::HostManifestConfig,
    backend: B,
}

impl<B: BrowserDomBackend + Clone + 'static> BrowserHostFactory<B> {
    pub fn new(backend: B) -> MResult<Self> {
        Ok(Self {
            manifest: crate::browser_host_manifest()?,
            backend,
        })
    }
}

impl<B: BrowserDomBackend + Clone + 'static> mech_runtime::RuntimeHostFactory
    for BrowserHostFactory<B>
{
    fn provider_name(&self) -> &str {
        "browser"
    }
    fn manifest(&self) -> &mech_runtime::HostManifestConfig {
        &self.manifest
    }
    fn validate_settings(
        &self,
        _instance_name: &str,
        settings: &mech_runtime::ConfigValue,
    ) -> MResult<()> {
        crate::browser_config_from_settings(settings).map(|_| ())
    }
    fn instantiate(
        &self,
        instance_name: &str,
        settings: &mech_runtime::ConfigValue,
    ) -> MResult<mech_runtime::RuntimeHostInstallation> {
        self.validate_settings(instance_name, settings)?;
        let browser = crate::browser_config_from_settings(settings)?;
        let authority = BrowserHostConfig {
            runtime: BrowserHostRuntimeConfig::from(&mech_runtime::RuntimeConfig::default()),
            browser,
        }
        .into_browser_authority()?;
        Ok(mech_runtime::RuntimeHostInstallation {
            interface: mech_runtime::materialize_host_manifest(instance_name, &self.manifest)?,
            input_drivers: Vec::new(),
            resource_providers: vec![Box::new(BrowserResourceProvider::for_instance(
                instance_name,
                authority,
                self.backend.clone(),
            ))],
        })
    }
}
