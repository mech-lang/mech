use std::collections::BTreeSet;

use mech_core::MResult;
use mech_host_cli::CliHostFactory;
#[cfg(feature = "time_host_native")]
use mech_host_time::NativeTimeHostFactory;
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

    Ok((builder, providers))
}
