pub const TIME_HOST_MCFG: &str = include_str!("../host.mcfg");

pub fn time_host_manifest() -> mech_core::MResult<mech_runtime::HostManifestConfig> {
  let doc = mech_runtime::parse_config_document(
    "hosts/time/host.mcfg",
    TIME_HOST_MCFG,
    mech_runtime::ConfigProfileOptions::default(),
  )?;
  doc.host.ok_or_else(|| {
    mech_core::MechError::new(
      mech_runtime::InvalidConfigField::new("time host manifest must contain top-level `host`"),
      None,
    ).with_compiler_loc()
  })
}
