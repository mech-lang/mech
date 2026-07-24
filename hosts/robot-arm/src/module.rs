pub const ROBOT_ARM_HOST_MCFG: &str = include_str!("../host.mcfg");

pub fn robot_arm_host_manifest() -> mech_core::MResult<mech_runtime::HostManifestConfig> {
    let doc = mech_runtime::parse_config_document(
        "hosts/robot-arm/host.mcfg",
        ROBOT_ARM_HOST_MCFG,
        mech_runtime::ConfigProfileOptions::default(),
    )?;
    doc.host.ok_or_else(|| {
        mech_core::MechError::new(
            mech_runtime::InvalidConfigField::new(
                "robot arm host manifest must contain top-level `host`",
            ),
            None,
        )
        .with_compiler_loc()
    })
}
