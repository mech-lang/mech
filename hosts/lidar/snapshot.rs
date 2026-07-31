use std::sync::{Arc, Mutex};

use mech_core::MResult;
use mech_runtime::{
    RuntimeHostInput, RuntimeHostInputSource, RuntimeHostInputUpdate, RuntimeHostInputValue,
};

/// The five paths a `.mec` program can read from the lidar scan context.
/// Keep this in sync with `LidarSnapshot` field order below.
pub const SCAN_PATHS: [&str; 5] = [
    "nearest-mm",     // distance to the closest valid return this scan (mm)
    "nearest-angle",  // bearing of that closest return (degrees, 0..360)
    "front-mm",       // distance straight ahead (nearest point near 0°)
    "count",          // number of valid points in the scan
    "scan-id",        // monotonically increasing scan counter (proves liveness)
];

pub fn lidar_input_base_uri(instance: &str) -> String {
    format!("lidar://{instance}/scan")
}

pub fn lidar_source_matches(instance: &str, source: &RuntimeHostInputSource) -> bool {
    source.base_uri() == lidar_input_base_uri(instance) && SCAN_PATHS.contains(&source.path())
}

/// One reduced LiDAR scan. A full 360° sweep is reduced to a handful of
/// scalars that a reactive Mech program can consume directly.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LidarSnapshot {
    pub nearest_mm: f64,
    pub nearest_angle: f64,
    pub front_mm: f64,
    pub count: f64,
    pub scan_id: f64,
}

impl LidarSnapshot {
    /// Convert the snapshot into a runtime host-input packet: one update per
    /// path, all sharing the same base URI. Identical shape to the time host.
    pub fn into_host_input(self, instance: &str) -> MResult<RuntimeHostInput> {
        let base_uri = lidar_input_base_uri(instance);
        let values = [
            self.nearest_mm,
            self.nearest_angle,
            self.front_mm,
            self.count,
            self.scan_id,
        ];
        let mut updates = Vec::with_capacity(SCAN_PATHS.len());
        for (path, value) in SCAN_PATHS.iter().zip(values) {
            updates.push(RuntimeHostInputUpdate {
                source: RuntimeHostInputSource::new(base_uri.clone(), *path)?,
                value: RuntimeHostInputValue::F64(value),
            });
        }
        RuntimeHostInput::new(updates)
    }
}

pub type SharedLidarSnapshot = Arc<Mutex<LidarSnapshot>>;

pub fn new_shared_snapshot(snapshot: LidarSnapshot) -> SharedLidarSnapshot {
    Arc::new(Mutex::new(snapshot))
}
