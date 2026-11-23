//! Rebuilt and retained buffer fields must agree at a fresh record boundary.
use super::artifacts;

#[test]
fn selected_record_axes_follow_fields_not_binding_order() {
    // Artifact construction also validates the decoded canonical representation.
    // Execution of the motivating full block selector is covered separately.
    let (_, _) = artifacts(include_str!("../fixtures/native/selected-record-axes.mec"));
}
