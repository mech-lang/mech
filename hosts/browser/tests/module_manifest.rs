use mech_runtime::ModuleManifestExportKind;

#[test]
fn browser_module_manifest_loads() {
  let module = mech_host_browser::browser_module_manifest().unwrap();
  assert_eq!(module.name, "browser");
  assert_eq!(module.exports.len(), 1);
  assert_eq!(module.exports[0].name, "dom");
  assert_eq!(module.exports[0].kind, ModuleManifestExportKind::Context);
  assert_eq!(module.exports[0].base_uri, "browser://dom");
  assert_eq!(module.exports[0].operations, vec!["read", "write"]);
}
