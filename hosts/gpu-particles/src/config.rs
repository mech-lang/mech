use mech_core::MResult;
use mech_runtime::ConfigValue;

use crate::gpu_particle_error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GpuParticleHostSettings {
    pub selector: String,
    pub max_particles: u32,
}

pub fn gpu_particle_settings_from_config(
    settings: &ConfigValue,
) -> MResult<GpuParticleHostSettings> {
    let ConfigValue::Map(map) = settings else {
        return Err(gpu_particle_error(
            "GpuParticleHostConfig",
            "gpu-particles host settings must be a map",
        ));
    };
    let mut selector = None;
    let mut max_particles = None;
    for (key, value) in map {
        match key.as_str() {
            "selector" => {
                let ConfigValue::String(raw) = value else {
                    return Err(gpu_particle_error(
                        "GpuParticleHostConfig",
                        "gpu-particles selector must be a string",
                    ));
                };
                if raw.trim().is_empty() {
                    return Err(gpu_particle_error(
                        "GpuParticleHostConfig",
                        "gpu-particles selector must be non-empty",
                    ));
                }
                selector = Some(raw.clone());
            }
            "max-particles" => {
                let ConfigValue::Integer(raw) = value else {
                    return Err(gpu_particle_error(
                        "GpuParticleHostConfig",
                        "gpu-particles max-particles must be an integer",
                    ));
                };
                if !(1..=16_000_000).contains(raw) {
                    return Err(gpu_particle_error(
                        "GpuParticleHostConfig",
                        "gpu-particles max-particles must be between 1 and 16000000",
                    ));
                }
                max_particles = Some(*raw as u32);
            }
            other => {
                return Err(gpu_particle_error(
                    "GpuParticleHostConfig",
                    format!("unknown gpu-particles host setting `{other}`"),
                ));
            }
        }
    }
    Ok(GpuParticleHostSettings {
        selector: selector.ok_or_else(|| {
            gpu_particle_error(
                "GpuParticleHostConfig",
                "gpu-particles selector is required",
            )
        })?,
        max_particles: max_particles.ok_or_else(|| {
            gpu_particle_error(
                "GpuParticleHostConfig",
                "gpu-particles max-particles is required",
            )
        })?,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    #[test]
    fn parses_valid_settings() {
        let settings = ConfigValue::Map(BTreeMap::from([
            (
                "selector".to_string(),
                ConfigValue::String("#particles".to_string()),
            ),
            ("max-particles".to_string(), ConfigValue::Integer(1_000_000)),
        ]));
        assert_eq!(
            gpu_particle_settings_from_config(&settings).unwrap(),
            GpuParticleHostSettings {
                selector: "#particles".to_string(),
                max_particles: 1_000_000,
            }
        );
    }

    #[test]
    fn rejects_unbounded_particle_counts() {
        let settings = ConfigValue::Map(BTreeMap::from([
            (
                "selector".to_string(),
                ConfigValue::String("#particles".to_string()),
            ),
            (
                "max-particles".to_string(),
                ConfigValue::Integer(16_000_001),
            ),
        ]));
        assert!(gpu_particle_settings_from_config(&settings).is_err());
    }
}
