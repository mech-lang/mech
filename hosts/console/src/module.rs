pub fn console_host_manifest() -> mech_core::MResult<mech_runtime::HostManifestConfig> {
    Ok(mech_runtime::HostManifestConfig {
        provider: "console".to_string(),
        contexts: vec![mech_runtime::HostContextManifest {
            name: "output".to_string(),
            base_uri_template: "console://{instance}/output".to_string(),
            operations: vec!["write".to_string()],
        }],
    })
}

#[cfg(test)]
mod tests {
    const HOST_MCFG: &str = include_str!("../host.mcfg");

    #[test]
    fn direct_manifest_matches_documented_fixture() {
        let parsed = mech_runtime::parse_config_document(
            "hosts/console/host.mcfg",
            HOST_MCFG,
            mech_runtime::ConfigProfileOptions::default(),
        )
        .unwrap()
        .host
        .unwrap();
        assert_eq!(super::console_host_manifest().unwrap(), parsed);
    }
}
