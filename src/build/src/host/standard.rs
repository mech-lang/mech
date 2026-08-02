use std::sync::Arc;

use mech_core::MResult;
use mech_runtime::{ConfigValue, RuntimeHostFactory};

use super::{NativeHostCatalog, NativeHostLinkage, NativeTargetFamily};

const CLI_TARGETS: &[NativeTargetFamily] = &[NativeTargetFamily::Unix, NativeTargetFamily::Windows];

fn validate_cli_settings(instance: &str, settings: &ConfigValue) -> MResult<()> {
    mech_host_cli::CliHostFactory::new()?.validate_settings(instance, settings)
}

/// Returns the trusted Phase 1 standard native-host catalog.
pub fn standard_native_host_catalog() -> MResult<Arc<NativeHostCatalog>> {
    let mut catalog = NativeHostCatalog::new();
    catalog.insert_provider(NativeHostLinkage {
        provider: "cli",
        package: "mech-host-cli",
        crate_name: "mech_host_cli",
        cargo_features: &["provider"],
        factory_path: "mech_host_cli::CliHostFactory::new",
        supported_targets: CLI_TARGETS,
        manifest: mech_host_cli::cli_host_manifest,
        validate_settings: validate_cli_settings,
    })?;
    Ok(Arc::new(catalog))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_one_standard_catalog_contains_only_cli() {
        let catalog = standard_native_host_catalog().unwrap();
        assert_eq!(catalog.provider_count(), 1);
        assert_eq!(catalog.function_count(), 0);

        let cli = catalog.provider("cli").unwrap();
        assert_eq!(cli.package, "mech-host-cli");
        assert_eq!(cli.crate_name, "mech_host_cli");
        assert_eq!(cli.cargo_features, ["provider"]);
        assert_eq!(cli.factory_path, "mech_host_cli::CliHostFactory::new");
        assert_eq!(cli.supported_targets, CLI_TARGETS);
        assert_eq!((cli.manifest)().unwrap().provider, "cli");
    }
}
