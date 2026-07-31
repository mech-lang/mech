use crate::config_profile::ConfigValue;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct HostInstanceConfig {
  pub name: String,
  pub provider: String,
  pub settings: ConfigValue,
}
