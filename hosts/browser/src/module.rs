pub const BROWSER_MODULE_MCFG: &str = include_str!("../module.mcfg");

pub fn browser_module_manifest() -> mech_core::MResult<mech_runtime::ModuleManifestConfig> {
  let doc = mech_runtime::parse_config_document(
    "hosts/browser/module.mcfg",
    BROWSER_MODULE_MCFG,
    mech_runtime::ConfigProfileOptions::default(),
  )?;
  doc.module.ok_or_else(|| {
    mech_core::MechError::new(
      mech_runtime::InvalidConfigField::new("browser module manifest must contain top-level `module`"),
      None,
    ).with_compiler_loc()
  })
}
