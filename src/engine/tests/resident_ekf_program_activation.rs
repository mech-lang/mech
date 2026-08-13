#![cfg(all(feature = "resident-artifact", feature = "compiler"))]

use mech_core::{InstanceEpoch, MResult, ReactiveInstanceId};
use mech_engine::__resident::{
    ActivationFacts, FrozenEkfCompilationServices, ResidentStorageClass, ResidentValueBorrow,
    activate, compile_frozen_ekf_source, frozen_ekf_compiler_catalog,
};

const SOURCE: &str =
    include_str!("../../../tests/architecture/resident-activation/ekf-source-v1.mec");

fn f64_state(instance: &mech_engine::__resident::ReactiveInstance) -> Vec<Vec<f64>> {
    instance
        .plan
        .slots
        .iter()
        .filter(|slot| slot.storage == ResidentStorageClass::State)
        .map(
            |slot| match instance.state_borrow(slot.artifact_id).unwrap() {
                ResidentValueBorrow::F64 { values, .. } => values.to_vec(),
                _ => panic!("frozen EKF state is f64"),
            },
        )
        .collect()
}

#[test]
fn public_ekf_artifact_activates_into_generic_storage_without_a_turn() -> MResult<()> {
    let mut services = FrozenEkfCompilationServices::default();
    let compilation = compile_frozen_ekf_source(SOURCE, &mut services)?;
    let catalog = frozen_ekf_compiler_catalog()?;
    let instance = activate(
        ReactiveInstanceId::new(7, 3),
        &compilation.source_artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .expect("closed public EKF artifact activates through the generic path");

    assert_eq!(instance.id, ReactiveInstanceId::new(7, 3));
    assert_eq!(
        instance.plan.program_revision,
        compilation.source_artifact.revision()
    );
    assert_eq!(instance.plan.steps.len(), 20);
    assert!(instance.plan.activation_nodes.is_empty());
    assert_eq!(instance.plan.inputs.len(), 1);
    assert_eq!(instance.plan.outputs.len(), 1);
    assert_eq!(instance.state.candidate_bytes(), 96);
    assert_eq!(instance.state.dual_payload_bytes(), 192);
    assert_eq!(instance.published_epoch(), InstanceEpoch::ZERO);
    assert_eq!(instance.next_epoch(), Some(InstanceEpoch::new(1)));

    let state = f64_state(&instance);
    assert_eq!(state.len(), 2);
    assert!(state.contains(&vec![2.0, 1.0, 0.15]));
    assert!(state.contains(&vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.05]));
    Ok(())
}

#[test]
fn source_and_bytecode_ekf_activation_have_one_generic_layout() -> MResult<()> {
    let mut services = FrozenEkfCompilationServices::default();
    let compilation = compile_frozen_ekf_source(SOURCE, &mut services)?;
    let catalog = frozen_ekf_compiler_catalog()?;
    let id = ReactiveInstanceId::new(1, 0);
    let source = activate(
        id,
        &compilation.source_artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap();
    let repeated = activate(
        id,
        &compilation.source_artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap();
    let decoded = activate(
        id,
        &compilation.decoded_artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap();

    assert_eq!(source.plan.slots, repeated.plan.slots);
    assert_eq!(source.plan.slots, decoded.plan.slots);
    assert_eq!(source.plan.activation_nodes, decoded.plan.activation_nodes);
    assert_eq!(
        source.plan.topology.turn_root_mask,
        decoded.plan.topology.turn_root_mask
    );
    assert_eq!(f64_state(&source), f64_state(&decoded));
    Ok(())
}
