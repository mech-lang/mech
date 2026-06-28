use std::collections::BTreeSet;

use mech_core::MResult;
use mech_host_cli::CliHostFactory;
use mech_runtime::{RuntimeBuilder, RuntimeHostFactory};

pub fn register_cli_host_factories(
    mut builder: RuntimeBuilder,
) -> MResult<(RuntimeBuilder, BTreeSet<String>)> {
    let cli_factory = CliHostFactory::new()?;
    let mut providers = BTreeSet::new();
    providers.insert(cli_factory.provider_name().to_string());
    builder = builder.host_factory(Box::new(cli_factory))?;

    #[cfg(feature = "host-robot-arm")]
    {
        let robot_factory = mech_host_robot_arm::RobotArmHostFactory::new()?;
        providers.insert(robot_factory.provider_name().to_string());
        builder = builder.host_factory(Box::new(robot_factory))?;
    }

    Ok((builder, providers))
}
