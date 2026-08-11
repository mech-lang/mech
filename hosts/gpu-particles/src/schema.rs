use mech_core::{MResult, MechRecord, Value, hash_str};

use crate::gpu_particle_error;

#[derive(Clone, Debug, PartialEq)]
pub struct GpuParticleControl {
    pub particle_count: u32,
    pub gravity: f32,
    pub drag: f32,
    pub point_size: f32,
    pub time_scale: f32,
}

impl GpuParticleControl {
    pub fn from_value(value: &Value, max_particles: u32) -> MResult<Self> {
        if let Value::MutableReference(value) = value {
            return Self::from_value(&value.borrow(), max_particles);
        }
        let Value::Record(record) = value else {
            return Err(gpu_particle_error(
                "GpuParticleControl",
                "gpu particle control must be a record",
            ));
        };
        let record = record.borrow();
        let allowed = [
            "particle-count",
            "gravity",
            "drag",
            "point-size",
            "time-scale",
        ];
        for (_, name) in &record.field_names {
            if !allowed.contains(&name.as_str()) {
                return Err(gpu_particle_error(
                    "GpuParticleControl",
                    format!("unknown gpu particle control field `{name}`"),
                ));
            }
        }
        let raw_count = number(&record, "particle-count")?;
        if raw_count.fract() != 0.0 || raw_count < 1.0 || raw_count > f64::from(max_particles) {
            return Err(gpu_particle_error(
                "GpuParticleControl",
                format!("particle-count must be an integer between 1 and {max_particles}"),
            ));
        }
        let control = Self {
            particle_count: raw_count as u32,
            gravity: number(&record, "gravity")? as f32,
            drag: number(&record, "drag")? as f32,
            point_size: number(&record, "point-size")? as f32,
            time_scale: number(&record, "time-scale")? as f32,
        };
        if !(-8.0..=8.0).contains(&control.gravity) {
            return Err(gpu_particle_error(
                "GpuParticleControl",
                "gravity must be between -8 and 8",
            ));
        }
        if !(0.8..=1.0).contains(&control.drag) {
            return Err(gpu_particle_error(
                "GpuParticleControl",
                "drag must be between 0.8 and 1",
            ));
        }
        if !(0.5..=8.0).contains(&control.point_size) {
            return Err(gpu_particle_error(
                "GpuParticleControl",
                "point-size must be between 0.5 and 8",
            ));
        }
        if !(0.0..=4.0).contains(&control.time_scale) {
            return Err(gpu_particle_error(
                "GpuParticleControl",
                "time-scale must be between 0 and 4",
            ));
        }
        Ok(control)
    }
}

fn number(record: &MechRecord, field: &str) -> MResult<f64> {
    let value = record.get(&hash_str(field)).ok_or_else(|| {
        gpu_particle_error(
            "GpuParticleControl",
            format!("missing required gpu particle control field `{field}`"),
        )
    })?;
    let value = value.as_f64().map_err(|_| {
        gpu_particle_error(
            "GpuParticleControl",
            format!("gpu particle control field `{field}` must be numeric"),
        )
    })?;
    let value = *value.borrow();
    if !value.is_finite() {
        return Err(gpu_particle_error(
            "GpuParticleControl",
            format!("gpu particle control field `{field}` must be finite"),
        ));
    }
    Ok(value)
}
