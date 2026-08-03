pub fn robot_arm_host_manifest() -> mech_core::MResult<mech_runtime::HostManifestConfig> {
    Ok(mech_runtime::HostManifestConfig {
        provider: "robot-arm".to_string(),
        contexts: vec![
            mech_runtime::HostContextManifest {
                name: "commands".to_string(),
                base_uri_template: "robot://{instance}/commands".to_string(),
                operations: vec!["move".to_string(), "grip".to_string(), "home".to_string()],
            },
            mech_runtime::HostContextManifest {
                name: "state".to_string(),
                base_uri_template: "robot://{instance}/state".to_string(),
                operations: vec!["read".to_string()],
            },
        ],
    })
}

#[cfg(test)]
mod tests {
    const HOST_MCFG: &str = include_str!("../host.mcfg");

    #[test]
    fn direct_manifest_matches_documented_fixture() {
        let parsed = mech_runtime::parse_config_document(
            "hosts/robot-arm/host.mcfg",
            HOST_MCFG,
            mech_runtime::ConfigProfileOptions::default(),
        )
        .unwrap()
        .host
        .unwrap();
        assert_eq!(super::robot_arm_host_manifest().unwrap(), parsed);
    }
}
