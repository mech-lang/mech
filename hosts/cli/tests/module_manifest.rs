#[test]
fn cli_module_manifest_matches_builtin_manifest() {
  let manifest = mech_host_cli::cli_module_manifest().unwrap();
  assert_eq!(manifest, mech_core::builtin_cli_module_manifest());
}
