use mech_core::MResult;
use mech_runtime::ConfigValue;

use crate::lidar_error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LidarHostSettings {
    /// Serial port of the RPLIDAR (e.g. "/dev/ttyUSB1").
    pub port: String,
    /// Serial baud rate. A2M8 = 115200; A3/S-series = 256000.
    pub baud: u32,
    /// How often to publish a reduced scan snapshot, in milliseconds.
    pub interval_ms: u64,
    /// How many scan points to read per snapshot (~one rotation for A2).
    pub points_per_scan: usize,
}

impl Default for LidarHostSettings {
    fn default() -> Self {
        Self {
            port: "/dev/ttyUSB1".to_string(),
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
                    return Err(lidar_error("LidarHostConfig", "lidar host `port` must be a string"));
                };
                parsed.port = s.clone();
            }
            "baud" => {
                let ConfigValue::Integer(raw) = value else {
                    return Err(lidar_error("LidarHostConfig", "lidar host `baud` must be an integer"));
                };
                if *raw <= 0 {
                    return Err(lidar_error("LidarHostConfig", "lidar host `baud` must be positive"));
                }
                parsed.baud = *raw as u32;
            }
            "interval-ms" => {
                let ConfigValue::Integer(raw) = value else {
                    return Err(lidar_error("LidarHostConfig", "lidar host `interval-ms` must be an integer"));
                };
                if *raw <= 0 {
                    return Err(lidar_error("LidarHostConfig", "lidar host `interval-ms` must be positive"));
                }
                if *raw > 60_000 {
                    return Err(lidar_error("LidarHostConfig", "lidar host `interval-ms` must be at most 60000"));
                }
                parsed.interval_ms = *raw as u64;
            }
            "points-per-scan" => {
                let ConfigValue::Integer(raw) = value else {
                    return Err(lidar_error("LidarHostConfig", "lidar host `points-per-scan` must be an integer"));
                };
                if *raw <= 0 {
                    return Err(lidar_error("LidarHostConfig", "lidar host `points-per-scan` must be positive"));
                }
                parsed.points_per_scan = *raw as usize;
            }
            other => {
                return Err(lidar_error(
                    "LidarHostConfig",
                    format!("unknown lidar host setting `{other}`"),
                ));
            }
        }
    }
    Ok(parsed)
}
