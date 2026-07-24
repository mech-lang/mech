use std::collections::BTreeSet;

use mech_core::MResult;
use mech_host_cli::CliHostFactory;
#[cfg(feature = "console_host_native")]
use mech_host_console::NativeConsoleHostFactory;
#[cfg(feature = "scene_host_native")]
use mech_host_scene::NativeSceneHostFactory;
#[cfg(feature = "time_host_native")]
use mech_host_time::NativeTimeHostFactory;
#[cfg(feature = "timer_host_native")]
use mech_host_timer::NativeTimerHostFactory;
use mech_runtime::{RuntimeBuilder, RuntimeHostFactory};

pub fn register_cli_host_factories(
    mut builder: RuntimeBuilder,
) -> MResult<(RuntimeBuilder, BTreeSet<String>)> {
    let cli_factory = CliHostFactory::new()?;
    let mut providers = BTreeSet::new();
    providers.insert(cli_factory.provider_name().to_string());
    builder = builder.host_factory(Box::new(cli_factory))?;

    #[cfg(feature = "time_host_native")]
    {
        let time_factory = NativeTimeHostFactory::new()?;
        providers.insert(time_factory.provider_name().to_string());
        builder = builder.host_factory(Box::new(time_factory))?;
    }

    #[cfg(feature = "timer_host_native")]
    {
        let timer_factory = NativeTimerHostFactory::new()?;
        providers.insert(timer_factory.provider_name().to_string());
        builder = builder.host_factory(Box::new(timer_factory))?;
    }

    #[cfg(feature = "console_host_native")]
    {
        let console_factory = NativeConsoleHostFactory::new()?;
        providers.insert(console_factory.provider_name().to_string());
        builder = builder.host_factory(Box::new(console_factory))?;
    }

    #[cfg(feature = "scene_host_native")]
    {
        let scene_factory = NativeSceneHostFactory::new()?;
        providers.insert(scene_factory.provider_name().to_string());
        builder = builder.host_factory(Box::new(scene_factory))?;
    }

    Ok((builder, providers))
}
