use std::collections::BTreeMap;

use mech_core::{MechRecord, Ref, Value};
use mech_host_gpu_particles::{GpuParticleResourceProvider, RecordingGpuParticleBackend};
use mech_runtime::{
    ConfigValue, PreparedRuntimeEffect, RuntimeCapabilityOperation, RuntimeResourceProvider,
    RuntimeResourceWriteIntent, RuntimeResourceWritePreflightRequest, RuntimeResourceWriteRequest,
};

fn number(value: f64) -> Value {
    Value::F64(Ref::new(value))
}

fn control(particle_count: f64) -> Value {
    Value::Record(Ref::new(MechRecord::new(vec![
        ("particle-count", number(particle_count)),
        ("gravity", number(0.34)),
        ("drag", number(0.997)),
        ("point-size", number(1.35)),
        ("time-scale", number(1.0)),
    ])))
}

fn request(value: Value) -> RuntimeResourceWriteRequest {
    RuntimeResourceWriteRequest {
        base_uri: "gpu-particles://particles/simulation".to_string(),
        path: "control".to_string(),
        context_name: "simulation".to_string(),
        operation: RuntimeCapabilityOperation::Write,
        intent: RuntimeResourceWriteIntent::Send,
        value,
    }
}

fn deliver(
    provider: &dyn RuntimeResourceProvider,
    request: RuntimeResourceWriteRequest,
) -> mech_core::MResult<()> {
    match provider.prepare_write(request)? {
        PreparedRuntimeEffect::AfterCommit(mut effect) => effect.deliver(),
        effect => panic!("expected after-commit effect, got {effect:?}"),
    }
}

#[test]
fn valid_control_is_delivered_after_commit() {
    let backend = RecordingGpuParticleBackend::default();
    let provider = GpuParticleResourceProvider::new("particles", 2_000_000, backend.clone());
    deliver(&provider, request(control(1_000_000.0))).unwrap();
    assert_eq!(backend.controls()[0].particle_count, 1_000_000);
}

#[test]
fn identical_control_is_not_delivered_twice() {
    let backend = RecordingGpuParticleBackend::default();
    let provider = GpuParticleResourceProvider::new("particles", 2_000_000, backend.clone());
    deliver(&provider, request(control(1_000_000.0))).unwrap();
    deliver(&provider, request(control(1_000_000.0))).unwrap();
    assert_eq!(backend.controls().len(), 1);
}

#[test]
fn out_of_range_count_fails_before_delivery() {
    let backend = RecordingGpuParticleBackend::default();
    let provider = GpuParticleResourceProvider::new("particles", 2_000_000, backend.clone());
    assert!(provider.plan_write(request(control(2_000_001.0))).is_err());
    assert!(backend.controls().is_empty());
}

#[test]
fn assignment_intent_is_rejected() {
    let provider = GpuParticleResourceProvider::new(
        "particles",
        2_000_000,
        RecordingGpuParticleBackend::default(),
    );
    assert!(
        provider
            .preflight_write(RuntimeResourceWritePreflightRequest {
                base_uri: "gpu-particles://particles/simulation".to_string(),
                path: "control".to_string(),
                context_name: "simulation".to_string(),
                operation: RuntimeCapabilityOperation::Write,
                intent: RuntimeResourceWriteIntent::Assign,
            })
            .is_err()
    );
}

#[test]
fn example_settings_shape_is_valid() {
    let settings = ConfigValue::Map(BTreeMap::from([
        (
            "selector".to_string(),
            ConfigValue::String("#particle-canvas".to_string()),
        ),
        ("max-particles".to_string(), ConfigValue::Integer(2_000_000)),
    ]));
    assert!(mech_host_gpu_particles::gpu_particle_settings_from_config(&settings).is_ok());
}
