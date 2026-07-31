use std::sync::{Arc, Mutex};

use mech_core::MResult;
use mech_runtime::{
    RuntimeHostInput, RuntimeHostInputSource, RuntimeHostInputUpdate, RuntimeHostInputValue,
};

pub const SCAN_PATHS: [&str; 5] = [
    "nearest-mm",
    "nearest-angle",
    "front-mm",
    "count",
    "scan-id",
];

pub fn lidar_input_base_uri(instance: &str) -> String {
    format!("lidar://{instance}/scan")
}

pub fn lidar_source_matches(instance: &str, source: &RuntimeHostInputSource) -> bool {
    source.base_uri() == lidar_input_base_uri(instance) && SCAN_PATHS.contains(&source.path())
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LidarSnapshot {
    pub nearest_mm: f64,
    pub nearest_angle: f64,
    pub front_mm: f64,
    pub count: f64,
    pub scan_id: f64,
}

impl LidarSnapshot {
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
