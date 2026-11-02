pub fn timer_host_manifest() -> mech_core::MResult<mech_runtime::HostManifestConfig> {
    Ok(mech_runtime::HostManifestConfig {
        provider: "timer".to_string(),
        contexts: vec![mech_runtime::HostContextManifest {
            name: "tick".to_string(),
            base_uri_template: "timer://{instance}/tick".to_string(),
            operations: vec!["read".to_string()],
        }],
    })
}

#[cfg(test)]
mod tests {
    const HOST_MCFG: &str = include_str!("../host.mcfg");

    #[test]
    fn direct_manifest_matches_documented_fixture() {
        let parsed = mech_runtime::parse_config_document(
            "hosts/timer/host.mcfg",
            HOST_MCFG,
            mech_runtime::ConfigProfileOptions::default(),
        )
        .unwrap()
        .host
        .unwrap();
        assert_eq!(super::timer_host_manifest().unwrap(), parsed);
    }
}
