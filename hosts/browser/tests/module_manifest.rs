#[test]
fn browser_module_manifest_loads() {
  let module = mech_host_browser::browser_module_manifest().unwrap();
  assert_eq!(module, mech_core::builtin_browser_module_manifest());
}
