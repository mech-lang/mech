use mech_core::MResult;
use mech_runtime::ConfigValue;

use crate::lidar_error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LidarHostSettings {
    pub port: String,
    pub baud: u32,
    pub interval_ms: u64,
    pub points_per_scan: usize,
}

impl Default for LidarHostSettings {
    fn default() -> Self {
        Self {
            port: "/dev/ttyUSB0".to_string(),
            baud: 115_200,
            interval_ms: 100,
            points_per_scan: 360,
        }
    }
}

pub fn lidar_settings_from_config(settings: &ConfigValue) -> MResult<LidarHostSettings> {
    let ConfigValue::Map(map) = settings else {
        return Err(lidar_error("LidarHostConfig", "lidar host settings must be a map"));
    };
    let mut parsed = LidarHostSettings::default();
    for (key, value) in map {
        match key.as_str() {
            "port" => {
                let ConfigValue::String(s) = value else {
                    return Err(lidar_error("LidarHostConfig", "`port` must be a string"));
                };
                parsed.port = s.clone();
            }
            "baud" => {
                let ConfigValue::Integer(raw) = value else {
                    return Err(lidar_error("LidarHostConfig", "`baud` must be an integer"));
                };
                if *raw <= 0 {
                    return Err(lidar_error("LidarHostConfig", "`baud` must be positive"));
                }
                parsed.baud = *raw as u32;
            }
            "interval-ms" => {
                let ConfigValue::Integer(raw) = value else {
                    return Err(lidar_error("LidarHostConfig", "`interval-ms` must be an integer"));
                };
                if *raw <= 0 || *raw > 60_000 {
                    return Err(lidar_error("LidarHostConfig", "`interval-ms` must be 1..60000"));
                }
                parsed.interval_ms = *raw as u64;
            }
            "points-per-scan" => {
                let ConfigValue::Integer(raw) = value else {
                    return Err(lidar_error("LidarHostConfig", "`points-per-scan` must be an integer"));
                };
                if *raw <= 0 {
                    return Err(lidar_error("LidarHostConfig", "`points-per-scan` must be positive"));
                }
                parsed.points_per_scan = *raw as usize;
            }
            other => {
                return Err(lidar_error("LidarHostConfig", format!("unknown setting `{other}`")));
            }
        }
    }
    Ok(parsed)
}
