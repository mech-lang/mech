pub const BROWSER_HOST_MCFG: &str = include_str!("../host.mcfg");

pub fn browser_host_manifest() -> mech_core::MResult<mech_runtime::HostManifestConfig> {
  let doc = mech_runtime::parse_config_document(
    "hosts/browser/host.mcfg",
    BROWSER_HOST_MCFG,
    mech_runtime::ConfigProfileOptions::default(),
  )?;
  doc.host.ok_or_else(|| {
    mech_core::MechError::new(
      mech_runtime::InvalidConfigField::new("browser host manifest must contain top-level `host`"),
      None,
    ).with_compiler_loc()
  })
}

pub fn browser_module_manifest() -> mech_core::MResult<mech_core::ModuleManifestConfig> {
  Ok(mech_core::ModuleManifestConfig {
    name: "browser".to_string(),
    exports: browser_host_manifest()?.contexts.into_iter().map(|context| mech_core::ModuleManifestExportConfig {
      name: context.name,
      kind: mech_core::ModuleManifestExportKind::Context,
      base_uri: context.base_uri_template.replace("{instance}/", ""),
      operations: context.operations,
    }).collect(),
  })
}
