#![cfg(feature = "resident-ekf-artifact")]

use mech_core::{InstanceEpoch, MResult, ReactiveInstanceId};
use mech_engine::__gate_d::{
    ActivatedNodeKind, FrozenEkfCompilationServices, ResidentActivationError, activate,
    compile_frozen_ekf_source,
};

const SOURCE: &str =
    include_str!("../../../tests/architecture/resident-activation/ekf-source-v1.mec");

#[test]
fn public_artifact_activates_into_typed_resident_storage_without_a_turn() -> MResult<()> {
    let mut services = FrozenEkfCompilationServices::default();
    let compilation = compile_frozen_ekf_source(SOURCE, &mut services)?;
    let instance = activate(
        ReactiveInstanceId::new(7, 3),
        &compilation.source_artifact,
        &compilation.resource_request,
    )
    .expect("closed public artifact activates");

    assert_eq!(instance.id, ReactiveInstanceId::new(7, 3));
    assert_eq!(
        instance.plan.program_revision,
        compilation.source_closure.program_revision
    );
    assert_eq!(instance.plan.nodes.len(), 20);
    assert_eq!(
        instance
            .plan
            .nodes
            .iter()
            .filter(|node| matches!(node.kind, ActivatedNodeKind::Kernel(_)))
            .count(),
        15
    );
    assert_eq!(
        instance
            .plan
            .nodes
            .iter()
            .filter(|node| matches!(node.kind, ActivatedNodeKind::Predicate(_)))
            .count(),
        3
    );
    assert_eq!(
        instance
            .plan
            .nodes
            .iter()
            .filter(|node| matches!(node.kind, ActivatedNodeKind::StateCopy { .. }))
            .count(),
        2
    );
    assert_eq!(instance.plan.constraints.len(), 3);
    assert_eq!(instance.estimate(), &[2.0, 1.0, 0.15]);
    assert_eq!(
        instance.covariance(),
        &[1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.05]
    );
    assert_eq!(instance.state.candidate_bytes(), 96);
    assert_eq!(instance.published_epoch(), InstanceEpoch::ZERO);
    assert_eq!(instance.next_epoch(), Some(InstanceEpoch::new(1)));
    assert_eq!(instance.workspace.input(), &[0.0; 4]);
    assert_eq!(instance.workspace.predicate_values(), &[false; 3]);
    assert_eq!(instance.workspace.executed_node_count(), 0);
    Ok(())
}

#[test]
fn repeated_source_and_bytecode_activation_have_one_logical_projection() -> MResult<()> {
    let mut services = FrozenEkfCompilationServices::default();
    let compilation = compile_frozen_ekf_source(SOURCE, &mut services)?;
    let id = ReactiveInstanceId::new(1, 0);
    let left = activate(
        id,
        &compilation.source_artifact,
        &compilation.resource_request,
    )
    .expect("source artifact activates");
    let repeated = activate(
        id,
        &compilation.source_artifact,
        &compilation.resource_request,
    )
    .expect("source artifact activates repeatedly");
    let decoded = activate(
        id,
        &compilation.decoded_artifact,
        &compilation.resource_request,
    )
    .expect("decoded bytecode artifact activates");

    assert_eq!(
        left.logical_binding_projection(),
        repeated.logical_binding_projection()
    );
    assert_eq!(
        left.logical_binding_projection(),
        decoded.logical_binding_projection()
    );
    Ok(())
}

#[test]
fn activation_revalidates_the_observation_boundary() -> MResult<()> {
    let mut services = FrozenEkfCompilationServices::default();
    let compilation = compile_frozen_ekf_source(SOURCE, &mut services)?;
    let mut invalid_request = compilation.resource_request.clone();
    invalid_request.path = "wrong".into();
    let error = activate(
        ReactiveInstanceId::new(0, 0),
        &compilation.source_artifact,
        &invalid_request,
    )
    .expect_err("activation must not trust an earlier closure");
    assert!(matches!(error, ResidentActivationError::ArtifactClosure(_)));
    Ok(())
}
