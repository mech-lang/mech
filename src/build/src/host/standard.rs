use std::sync::Arc;

use mech_core::{MResult, MechError};
use mech_runtime::{ConfigValue, RuntimeHostFactory};

use super::{
    NativeHostCatalog, NativeHostCatalogInvalid, NativeHostFunctionContext,
    NativeHostFunctionLinkage, NativeHostLinkage, NativeTargetFamily,
};

const STANDARD_TARGETS: &[NativeTargetFamily] =
    &[NativeTargetFamily::Unix, NativeTargetFamily::Windows];
const ACTOR_FEATURES: &[&str] = &["native-link", "runtime", "string"];

fn validate_cli_settings(instance: &str, settings: &ConfigValue) -> MResult<()> {
    mech_host_cli::CliHostFactory::new()?.validate_settings(instance, settings)
}

fn validate_console_settings(_instance: &str, settings: &ConfigValue) -> MResult<()> {
    mech_host_console::validate_console_settings(settings)
}

fn validate_time_settings(_instance: &str, settings: &ConfigValue) -> MResult<()> {
    mech_host_time::time_settings_from_config(settings).map(|_| ())
}

fn validate_timer_settings(_instance: &str, settings: &ConfigValue) -> MResult<()> {
    mech_host_timer::timer_settings_from_config(settings).map(|_| ())
}

fn validate_scene_settings(_instance: &str, settings: &ConfigValue) -> MResult<()> {
    mech_host_scene::scene_settings_from_config(settings).map(|_| ())
}

fn validate_robot_arm_settings(instance: &str, settings: &ConfigValue) -> MResult<()> {
    mech_host_robot_arm::RobotArmHostFactory::new()?.validate_settings(instance, settings)
}

fn cli_planning_factory() -> MResult<Box<dyn RuntimeHostFactory>> {
    Ok(Box::new(mech_host_cli::CliHostFactory::new()?))
}

fn console_planning_factory() -> MResult<Box<dyn RuntimeHostFactory>> {
    Ok(Box::new(
        mech_host_console::ConsoleHostFactory::with_backend(
            mech_host_console::RecordingConsoleBackend::new(),
        )?,
    ))
}

#[derive(Clone, Copy, Debug, Default)]
struct PlanningTimeBackend;

impl mech_host_time::TimeBackend for PlanningTimeBackend {
    fn snapshot(&self) -> MResult<mech_host_time::TimeSnapshot> {
        Ok(mech_host_time::TimeSnapshot::default())
    }
}

fn time_planning_factory() -> MResult<Box<dyn RuntimeHostFactory>> {
    Ok(Box::new(
        mech_host_time::NativeTimeHostFactory::with_backend(PlanningTimeBackend)?,
    ))
}

fn timer_planning_factory() -> MResult<Box<dyn RuntimeHostFactory>> {
    Ok(Box::new(
        mech_host_timer::NativeTimerHostFactory::with_backend(
            mech_host_timer::ManualMonotonicTimerBackend::new(),
        )?,
    ))
}

fn scene_planning_factory() -> MResult<Box<dyn RuntimeHostFactory>> {
    Ok(Box::new(mech_host_scene::NativeSceneHostFactory::new()?))
}

fn robot_arm_planning_factory() -> MResult<Box<dyn RuntimeHostFactory>> {
    Ok(Box::new(mech_host_robot_arm::RobotArmHostFactory::new()?))
}

fn standard_native_host_registrations() -> Vec<NativeHostLinkage> {
    vec![
        NativeHostLinkage {
            provider: "cli",
            package: "mech-host-cli",
            crate_name: "mech_host_cli",
            cargo_features: &["provider"],
            factory_path: "mech_host_cli::CliHostFactory::new",
            supported_targets: STANDARD_TARGETS,
            manifest: mech_host_cli::cli_host_manifest,
            validate_settings: validate_cli_settings,
            planning_factory: cli_planning_factory,
        },
        NativeHostLinkage {
            provider: "console",
            package: "mech-host-console",
            crate_name: "mech_host_console",
            cargo_features: &["native"],
            factory_path: "mech_host_console::NativeConsoleHostFactory::new",
            supported_targets: STANDARD_TARGETS,
            manifest: mech_host_console::console_host_manifest,
            validate_settings: validate_console_settings,
            planning_factory: console_planning_factory,
        },
        NativeHostLinkage {
            provider: "robot-arm",
            package: "mech-host-robot-arm",
            crate_name: "mech_host_robot_arm",
            cargo_features: &["provider"],
            factory_path: "mech_host_robot_arm::RobotArmHostFactory::new",
            supported_targets: STANDARD_TARGETS,
            manifest: mech_host_robot_arm::robot_arm_host_manifest,
            validate_settings: validate_robot_arm_settings,
            planning_factory: robot_arm_planning_factory,
        },
        NativeHostLinkage {
            provider: "scene",
            package: "mech-host-scene",
            crate_name: "mech_host_scene",
            cargo_features: &["native"],
            factory_path: "mech_host_scene::NativeSceneHostFactory::new",
            supported_targets: STANDARD_TARGETS,
            manifest: mech_host_scene::scene_host_manifest,
            validate_settings: validate_scene_settings,
            planning_factory: scene_planning_factory,
        },
        NativeHostLinkage {
            provider: "time",
            package: "mech-host-time",
            crate_name: "mech_host_time",
            cargo_features: &["native"],
            factory_path: "mech_host_time::NativeTimeHostFactory::new",
            supported_targets: STANDARD_TARGETS,
            manifest: mech_host_time::time_host_manifest,
            validate_settings: validate_time_settings,
            planning_factory: time_planning_factory,
        },
        NativeHostLinkage {
            provider: "timer",
            package: "mech-host-timer",
            crate_name: "mech_host_timer",
            cargo_features: &["native"],
            factory_path: "mech_host_timer::NativeTimerHostFactory::new",
            supported_targets: STANDARD_TARGETS,
            manifest: mech_host_timer::timer_host_manifest,
            validate_settings: validate_timer_settings,
            planning_factory: timer_planning_factory,
        },
    ]
}

fn actor_host_function_linkages() -> [NativeHostFunctionLinkage; 5] {
    [
        NativeHostFunctionLinkage {
            name: "actor/message/kind",
            context: NativeHostFunctionContext::ActorTurn,
            package: "mech-runtime",
            crate_name: "mech_runtime",
            cargo_features: ACTOR_FEATURES,
            installer_path: "mech_runtime::__mech_native::install_actor_message_kind",
        },
        NativeHostFunctionLinkage {
            name: "actor/message/payload",
            context: NativeHostFunctionContext::ActorTurn,
            package: "mech-runtime",
            crate_name: "mech_runtime",
            cargo_features: ACTOR_FEATURES,
            installer_path: "mech_runtime::__mech_native::install_actor_message_payload",
        },
        NativeHostFunctionLinkage {
            name: "actor/state/get",
            context: NativeHostFunctionContext::ActorTurn,
            package: "mech-runtime",
            crate_name: "mech_runtime",
            cargo_features: ACTOR_FEATURES,
            installer_path: "mech_runtime::__mech_native::install_actor_state_get",
        },
        NativeHostFunctionLinkage {
            name: "actor/state/id",
            context: NativeHostFunctionContext::ActorTurn,
            package: "mech-runtime",
            crate_name: "mech_runtime",
            cargo_features: ACTOR_FEATURES,
            installer_path: "mech_runtime::__mech_native::install_actor_state_id",
        },
        NativeHostFunctionLinkage {
            name: "actor/state/put",
            context: NativeHostFunctionContext::ActorTurn,
            package: "mech-runtime",
            crate_name: "mech_runtime",
            cargo_features: ACTOR_FEATURES,
            installer_path: "mech_runtime::__mech_native::install_actor_state_put",
        },
    ]
}

/// Returns the complete trusted standard native-host and actor-function catalog.
pub fn standard_native_host_catalog() -> MResult<Arc<NativeHostCatalog>> {
    let mut catalog = NativeHostCatalog::new();
    for linkage in standard_native_host_registrations() {
        catalog.insert_provider(linkage)?;
    }
    for linkage in actor_host_function_linkages() {
        catalog.insert_function(linkage)?;
    }
    Ok(Arc::new(catalog))
}

/// Constructs the effect-free planning factory registered for one standard provider.
pub fn standard_planning_host_factory(provider: &str) -> MResult<Box<dyn RuntimeHostFactory>> {
    standard_native_host_registrations()
        .into_iter()
        .find(|linkage| linkage.provider == provider)
        .map(|linkage| (linkage.planning_factory)())
        .unwrap_or_else(|| {
            Err(MechError::new(
                NativeHostCatalogInvalid {
                    reason: format!("unknown standard planning provider `{provider}`"),
                },
                None,
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_catalog_has_exact_provider_and_actor_surfaces() {
        let catalog = standard_native_host_catalog().unwrap();
        assert_eq!(catalog.provider_count(), 6);
        assert_eq!(catalog.function_count(), 5);
        assert_eq!(
            catalog
                .providers()
                .map(|(name, _)| name)
                .collect::<Vec<_>>(),
            ["cli", "console", "robot-arm", "scene", "time", "timer"]
        );
        assert_eq!(
            catalog
                .functions()
                .map(|(name, _)| name)
                .collect::<Vec<_>>(),
            [
                "actor/message/kind",
                "actor/message/payload",
                "actor/state/get",
                "actor/state/id",
                "actor/state/put",
            ]
        );
    }

    #[test]
    fn catalog_and_planning_factory_provider_names_are_identical() {
        let catalog = standard_native_host_catalog().unwrap();
        let catalog_names = catalog
            .providers()
            .map(|(name, _)| name)
            .collect::<Vec<_>>();
        let planning_names = standard_native_host_registrations()
            .into_iter()
            .map(|linkage| linkage.provider)
            .collect::<Vec<_>>();
        assert_eq!(catalog_names, planning_names);
        for provider in catalog_names {
            assert_eq!(
                standard_planning_host_factory(provider)
                    .unwrap()
                    .provider_name(),
                provider
            );
        }
    }

    #[test]
    fn every_provider_supports_only_unix_and_windows() {
        let catalog = standard_native_host_catalog().unwrap();
        for (_, linkage) in catalog.providers() {
            assert_eq!(linkage.supported_targets, STANDARD_TARGETS);
        }
    }
}
