use mech_core::{MResult, Ref, Value};
use mech_runtime::{RuntimeResourceProvider, RuntimeResourceReadRequest};

use crate::{lidar_error, lidar_input_base_uri, LidarSnapshot, SharedLidarSnapshot};

#[derive(Debug)]
pub struct LidarResourceProvider {
    instance: String,
    snapshot: SharedLidarSnapshot,
}

impl LidarResourceProvider {
    pub fn new(instance: impl Into<String>, snapshot: SharedLidarSnapshot) -> Self {
        Self { instance: instance.into(), snapshot }
    }

    pub fn base_uri(&self) -> String {
        lidar_input_base_uri(&self.instance)
    }

    fn value_for(snapshot: LidarSnapshot, path: &str) -> MResult<Value> {
        let value = match path {
            "nearest-mm" => snapshot.nearest_mm,
            "nearest-angle" => snapshot.nearest_angle,
            "front-mm" => snapshot.front_mm,
            "count" => snapshot.count,
            "scan-id" => snapshot.scan_id,
            other => {
                return Err(lidar_error(
                    "LidarResourceProvider",
                    format!("unknown path `{other}`"),
                ))
            }
        };
        Ok(Value::F64(Ref::new(value)))
    }
}

impl RuntimeResourceProvider for LidarResourceProvider {
    fn scheme(&self) -> &str { "lidar" }
    fn base_uris(&self) -> Vec<String> { vec![self.base_uri()] }
    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        if request.base_uri != self.base_uri() {
            return Err(lidar_error(
                "LidarResourceProvider",
                format!("unknown resource `{}`", request.base_uri),
            ));
        }
        let snapshot = *self.snapshot.lock()
            .map_err(|_| lidar_error("LidarResourceProvider", "snapshot lock poisoned"))?;
        Self::value_for(snapshot, &request.path)
    }
}
