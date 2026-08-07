use std::collections::BTreeMap;

use mech_core::{MResult, MechError, MechErrorKind};
use mech_runtime::{ConfigValue, HostManifestConfig, RuntimeHostFactory};
use serde::{Deserialize, Serialize};

/// Broad target families used by trusted native-host metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NativeTargetFamily {
    Unix,
    Windows,
    Wasm,
    Unsupported,
}

impl NativeTargetFamily {
    /// Classifies an explicit Rust target triple, or the current build target
    /// when no triple was requested.
    pub fn resolve(target: Option<&str>) -> Self {
        match target {
            Some(target) if target.starts_with("wasm") => Self::Wasm,
            Some(target) if target.contains("windows") => Self::Windows,
            Some(target) if explicit_unix_target(target) => Self::Unix,
            Some(_) => Self::Unsupported,
            None if cfg!(target_arch = "wasm32") => Self::Wasm,
            None if cfg!(target_family = "windows") => Self::Windows,
            None if cfg!(target_family = "unix") => Self::Unix,
            None => Self::Unsupported,
        }
    }
}

fn explicit_unix_target(target: &str) -> bool {
    target.split('-').any(|component| {
        matches!(
            component,
            "aix"
                | "android"
                | "darwin"
                | "dragonfly"
                | "freebsd"
                | "fuchsia"
                | "haiku"
                | "hurd"
                | "illumos"
                | "ios"
                | "linux"
                | "macos"
                | "netbsd"
                | "openbsd"
                | "redox"
                | "solaris"
                | "tvos"
                | "visionos"
                | "watchos"
        )
    })
}

#[derive(Clone)]
pub struct NativeHostLinkage {
    pub provider: &'static str,
    pub package: &'static str,
    pub crate_name: &'static str,
    pub cargo_features: &'static [&'static str],
    pub factory_path: &'static str,
    pub supported_targets: &'static [NativeTargetFamily],
    pub manifest: fn() -> MResult<HostManifestConfig>,
    pub validate_settings: fn(instance: &str, settings: &ConfigValue) -> MResult<()>,
    pub planning_factory: fn() -> MResult<Box<dyn RuntimeHostFactory>>,
}

impl std::fmt::Debug for NativeHostLinkage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NativeHostLinkage")
            .field("provider", &self.provider)
            .field("package", &self.package)
            .field("crate_name", &self.crate_name)
            .field("cargo_features", &self.cargo_features)
            .field("factory_path", &self.factory_path)
            .field("supported_targets", &self.supported_targets)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug)]
pub struct NativeHostFunctionLinkage {
    pub name: &'static str,
    pub context: NativeHostFunctionContext,
    pub package: &'static str,
    pub crate_name: &'static str,
    pub cargo_features: &'static [&'static str],
    pub installer_path: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeHostFunctionContext {
    Standalone,
    ActorTurn,
}

/// Trusted native linkage metadata keyed deterministically by provider and
/// host-function name.
#[derive(Clone, Debug, Default)]
pub struct NativeHostCatalog {
    providers: BTreeMap<String, NativeHostLinkage>,
    functions: BTreeMap<String, NativeHostFunctionLinkage>,
}

impl NativeHostCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_provider(&mut self, linkage: NativeHostLinkage) -> MResult<()> {
        validate_provider_linkage(&linkage)?;
        if self.providers.contains_key(linkage.provider) {
            return invalid(format!(
                "duplicate native host provider `{}`",
                linkage.provider
            ));
        }
        self.providers.insert(linkage.provider.to_string(), linkage);
        Ok(())
    }

    pub fn insert_function(&mut self, linkage: NativeHostFunctionLinkage) -> MResult<()> {
        validate_function_linkage(&linkage)?;
        if self.functions.contains_key(linkage.name) {
            return invalid(format!("duplicate native host function `{}`", linkage.name));
        }
        self.functions.insert(linkage.name.to_string(), linkage);
        Ok(())
    }

    pub fn provider(&self, provider: &str) -> Option<&NativeHostLinkage> {
        self.providers.get(provider)
    }

    pub fn function(&self, name: &str) -> Option<&NativeHostFunctionLinkage> {
        self.functions.get(name)
    }

    pub fn providers(
        &self,
    ) -> impl DoubleEndedIterator<Item = (&str, &NativeHostLinkage)> + ExactSizeIterator {
        self.providers
            .iter()
            .map(|(provider, linkage)| (provider.as_str(), linkage))
    }

    pub fn functions(
        &self,
    ) -> impl DoubleEndedIterator<Item = (&str, &NativeHostFunctionLinkage)> + ExactSizeIterator
    {
        self.functions
            .iter()
            .map(|(name, linkage)| (name.as_str(), linkage))
    }

    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }

    pub fn function_count(&self) -> usize {
        self.functions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.providers.is_empty() && self.functions.is_empty()
    }
}

fn validate_provider_linkage(linkage: &NativeHostLinkage) -> MResult<()> {
    validate_identifier("provider", linkage.provider, true)?;
    validate_identifier("package", linkage.package, true)?;
    validate_identifier("crate name", linkage.crate_name, false)?;
    validate_rust_path("factory path", linkage.factory_path)?;
    validate_features(linkage.cargo_features)?;
    validate_targets(linkage.supported_targets)?;

    let manifest = (linkage.manifest)()?;
    mech_runtime::validate_host_manifest(&manifest)?;
    if manifest.provider != linkage.provider {
        return invalid(format!(
            "native host provider `{}` does not match manifest provider `{}`",
            linkage.provider, manifest.provider
        ));
    }
    let planning_factory = (linkage.planning_factory)()?;
    if planning_factory.provider_name() != linkage.provider {
        return invalid(format!(
            "native host provider `{}` does not match planning-factory provider `{}`",
            linkage.provider,
            planning_factory.provider_name()
        ));
    }
    if planning_factory.manifest() != &manifest {
        return invalid(format!(
            "native host provider `{}` planning-factory manifest differs from its trusted linkage manifest",
            linkage.provider
        ));
    }
    Ok(())
}

fn validate_function_linkage(linkage: &NativeHostFunctionLinkage) -> MResult<()> {
    if linkage.name.is_empty() {
        return invalid("native host function name must not be empty");
    }
    validate_identifier("package", linkage.package, true)?;
    validate_identifier("crate name", linkage.crate_name, false)?;
    validate_rust_path("installer path", linkage.installer_path)?;
    validate_features(linkage.cargo_features)
}

fn validate_identifier(label: &str, value: &str, allow_hyphen: bool) -> MResult<()> {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return invalid(format!("native host {label} must not be empty"));
    };
    if !(first.is_ascii_alphabetic() || first == '_')
        || !chars.all(|character| {
            character.is_ascii_alphanumeric()
                || character == '_'
                || (allow_hyphen && character == '-')
        })
    {
        return invalid(format!("invalid native host {label} `{value}`"));
    }
    Ok(())
}

fn validate_rust_path(label: &str, value: &str) -> MResult<()> {
    let mut segments = value.split("::");
    let Some(first) = segments.next() else {
        return invalid(format!("native host {label} must not be empty"));
    };
    let remaining: Vec<_> = segments.collect();
    if remaining.is_empty()
        || !std::iter::once(first)
            .chain(remaining)
            .all(validate_rust_identifier)
    {
        return invalid(format!("invalid native host {label} `{value}`"));
    }
    Ok(())
}

fn validate_rust_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some(first) if first.is_ascii_alphabetic() || first == '_')
        && chars.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn validate_features(features: &[&str]) -> MResult<()> {
    let mut previous = None;
    for feature in features {
        validate_identifier("Cargo feature", feature, true)?;
        if previous.is_some_and(|previous| previous >= *feature) {
            return invalid("native host Cargo features must be sorted and deduplicated");
        }
        previous = Some(*feature);
    }
    Ok(())
}

fn validate_targets(targets: &[NativeTargetFamily]) -> MResult<()> {
    if targets.is_empty() {
        return invalid("native host supported targets must not be empty");
    }
    if targets.windows(2).any(|pair| pair[0] >= pair[1]) {
        return invalid("native host supported targets must be sorted and deduplicated");
    }
    Ok(())
}

fn invalid<T>(reason: impl Into<String>) -> MResult<T> {
    Err(MechError::new(
        NativeHostCatalogInvalid {
            reason: reason.into(),
        },
        None,
    ))
}

#[derive(Clone, Debug)]
pub struct NativeHostCatalogInvalid {
    pub reason: String,
}

impl MechErrorKind for NativeHostCatalogInvalid {
    fn name(&self) -> &str {
        "NativeHostCatalogInvalid"
    }

    fn message(&self) -> String {
        format!("invalid native host catalog: {}", self.reason)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestFactory {
        provider: &'static str,
        manifest: HostManifestConfig,
    }

    impl RuntimeHostFactory for TestFactory {
        fn provider_name(&self) -> &str {
            self.provider
        }

        fn manifest(&self) -> &HostManifestConfig {
            &self.manifest
        }

        fn validate_settings(&self, _instance: &str, _settings: &ConfigValue) -> MResult<()> {
            Ok(())
        }

        fn instantiate(
            &self,
            _instance: &str,
            _settings: &ConfigValue,
        ) -> MResult<mech_runtime::RuntimeHostInstallation> {
            unreachable!("catalog validation must not instantiate planning factories")
        }
    }

    fn test_manifest() -> MResult<HostManifestConfig> {
        Ok(HostManifestConfig {
            provider: "test".to_string(),
            contexts: vec![mech_runtime::HostContextManifest {
                name: "data".to_string(),
                base_uri_template: "test://{instance}/data".to_string(),
                operations: vec!["read".to_string()],
            }],
        })
    }

    fn validate_settings(_instance: &str, _settings: &ConfigValue) -> MResult<()> {
        Ok(())
    }

    fn planning_factory() -> MResult<Box<dyn RuntimeHostFactory>> {
        Ok(Box::new(TestFactory {
            provider: "test",
            manifest: test_manifest()?,
        }))
    }

    fn z_test_planning_factory() -> MResult<Box<dyn RuntimeHostFactory>> {
        Ok(Box::new(TestFactory {
            provider: "z-test",
            manifest: HostManifestConfig {
                provider: "z-test".to_owned(),
                ..test_manifest()?
            },
        }))
    }

    fn linkage() -> NativeHostLinkage {
        NativeHostLinkage {
            provider: "test",
            package: "mech-host-test",
            crate_name: "mech_host_test",
            cargo_features: &["provider"],
            factory_path: "mech_host_test::TestHostFactory::new",
            supported_targets: &[NativeTargetFamily::Unix, NativeTargetFamily::Windows],
            manifest: test_manifest,
            validate_settings,
            planning_factory,
        }
    }

    #[test]
    fn catalogs_iterate_in_provider_name_order() {
        let mut second = linkage();
        second.provider = "z-test";
        second.manifest = || {
            Ok(HostManifestConfig {
                provider: "z-test".to_string(),
                ..test_manifest()?
            })
        };
        second.planning_factory = z_test_planning_factory;

        let mut catalog = NativeHostCatalog::new();
        catalog.insert_provider(second).unwrap();
        catalog.insert_provider(linkage()).unwrap();

        assert_eq!(
            catalog
                .providers()
                .map(|(provider, _)| provider)
                .collect::<Vec<_>>(),
            ["test", "z-test"]
        );
    }

    #[test]
    fn duplicate_providers_and_unsorted_features_are_rejected() {
        let mut catalog = NativeHostCatalog::new();
        catalog.insert_provider(linkage()).unwrap();
        assert!(catalog.insert_provider(linkage()).is_err());

        let mut invalid = linkage();
        invalid.provider = "other";
        invalid.cargo_features = &["z", "a"];
        assert!(catalog.insert_provider(invalid).is_err());
    }

    #[test]
    fn planning_factory_identity_and_manifest_are_exact() {
        let mut wrong_provider = linkage();
        wrong_provider.planning_factory = z_test_planning_factory;
        let error = NativeHostCatalog::new()
            .insert_provider(wrong_provider)
            .unwrap_err();
        assert_eq!(error.kind_name(), "NativeHostCatalogInvalid");
        assert!(error.kind_message().contains("planning-factory provider"));

        fn wrong_manifest_factory() -> MResult<Box<dyn RuntimeHostFactory>> {
            Ok(Box::new(TestFactory {
                provider: "test",
                manifest: HostManifestConfig {
                    provider: "test".to_owned(),
                    contexts: vec![mech_runtime::HostContextManifest {
                        name: "other".to_owned(),
                        base_uri_template: "test://{instance}/other".to_owned(),
                        operations: vec!["read".to_owned()],
                    }],
                },
            }))
        }
        let mut wrong_manifest = linkage();
        wrong_manifest.planning_factory = wrong_manifest_factory;
        let error = NativeHostCatalog::new()
            .insert_provider(wrong_manifest)
            .unwrap_err();
        assert_eq!(error.kind_name(), "NativeHostCatalogInvalid");
        assert!(error.kind_message().contains("manifest differs"));
    }

    #[test]
    fn explicit_targets_are_classified_conservatively() {
        assert_eq!(
            NativeTargetFamily::resolve(Some("x86_64-unknown-linux-gnu")),
            NativeTargetFamily::Unix
        );
        assert_eq!(
            NativeTargetFamily::resolve(Some("aarch64-apple-darwin")),
            NativeTargetFamily::Unix
        );
        assert_eq!(
            NativeTargetFamily::resolve(Some("x86_64-pc-windows-msvc")),
            NativeTargetFamily::Windows
        );
        assert_eq!(
            NativeTargetFamily::resolve(Some("wasm32-wasip2")),
            NativeTargetFamily::Wasm
        );
        assert_eq!(
            NativeTargetFamily::resolve(Some("thumbv7em-none-eabihf")),
            NativeTargetFamily::Unsupported
        );
    }
}
