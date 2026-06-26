use crate::config_profile::ConfigValue;

#[derive(Clone, Debug, PartialEq)]
pub struct HostInstanceConfig {
  pub name: String,
  pub provider: String,
  pub settings: ConfigValue,
}
