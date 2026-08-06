pub fn cli_host_manifest() -> mech_core::MResult<mech_runtime::HostManifestConfig> {
    Ok(mech_runtime::HostManifestConfig {
        provider: "cli".to_string(),
        contexts: vec![
            mech_runtime::HostContextManifest {
                name: "env".to_string(),
                base_uri_template: "cli://{instance}/env".to_string(),
                operations: vec!["read".to_string()],
            },
            mech_runtime::HostContextManifest {
                name: "stdout".to_string(),
                base_uri_template: "cli://{instance}/stdout".to_string(),
                operations: vec!["write".to_string()],
            },
            mech_runtime::HostContextManifest {
                name: "stderr".to_string(),
                base_uri_template: "cli://{instance}/stderr".to_string(),
                operations: vec!["write".to_string()],
            },
        ],
    })
}

#[cfg(test)]
mod tests {
    const CLI_HOST_MCFG: &str = include_str!("../host.mcfg");

    #[test]
    fn direct_manifest_matches_the_documented_fixture() {
        let parsed = mech_runtime::parse_config_document(
            "hosts/terminal/host.mcfg",
            CLI_HOST_MCFG,
            mech_runtime::ConfigProfileOptions::default(),
        )
        .unwrap()
        .host
        .expect("CLI fixture must contain a top-level host manifest");

        assert_eq!(super::cli_host_manifest().unwrap(), parsed);
    }
}
