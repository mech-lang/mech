pub const CLI_HOST_MCFG: &str = include_str!("../host.mcfg");

pub fn cli_host_manifest() -> mech_core::MResult<mech_runtime::HostManifestConfig> {
  let doc = mech_runtime::parse_config_document(
    "hosts/cli/host.mcfg",
    CLI_HOST_MCFG,
    mech_runtime::ConfigProfileOptions::default(),
  )?;

  doc.host.ok_or_else(|| {
    mech_core::MechError::new(
      mech_runtime::InvalidConfigField::new("cli host manifest must contain top-level `host`"),
      None,
    ).with_compiler_loc()
  })
}

pub fn cli_module_manifest() -> mech_core::MResult<mech_core::ModuleManifestConfig> {
  Ok(mech_core::ModuleManifestConfig {
    name: "cli".to_string(),
    exports: cli_host_manifest()?.contexts.into_iter().map(|context| mech_core::ModuleManifestExportConfig {
      name: context.name,
      kind: mech_core::ModuleManifestExportKind::Context,
      base_uri: context.base_uri_template.replace("{instance}/", ""),
      operations: context.operations,
    }).collect(),
  })
}
