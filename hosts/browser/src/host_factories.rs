use std::collections::BTreeSet;

pub fn browser_available_host_providers() -> BTreeSet<String> {
  let mut providers = BTreeSet::new();
  providers.insert("browser".to_string());
  #[cfg(feature = "host-robot-arm")]
  {
    providers.insert("robot-arm".to_string());
  }
  providers
}
