pub const CLI_MODULE_MCFG: &str = include_str!("../module.mcfg");

pub fn cli_module_manifest() -> mech_core::MResult<mech_core::ModuleManifestConfig> {
  let doc = mech_runtime::parse_config_document(
    "hosts/cli/module.mcfg",
    CLI_MODULE_MCFG,
    mech_runtime::ConfigProfileOptions::default(),
  )?;

  let manifest = doc.module.ok_or_else(|| {
    mech_core::MechError::new(
      mech_runtime::InvalidConfigField::new("cli module manifest must contain top-level `module`"),
      None,
    ).with_compiler_loc()
  })?;

  let expected = mech_core::builtin_cli_module_manifest();
  if manifest != expected {
    return Err(mech_core::MechError::new(
      mech_runtime::InvalidConfigField::new("cli module manifest differs from built-in cli manifest"),
      None,
    ).with_compiler_loc());
  }

  Ok(manifest)
}
