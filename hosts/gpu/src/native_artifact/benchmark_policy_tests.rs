use super::*;
use mech_runtime::{BasicCapability, BasicConstraints, BasicSubject, ResourcePathCapability};

fn request(subject: &str, operation: &str, resource: &str) -> CapabilityRequest {
    CapabilityRequest::from_keys(subject, operation, resource)
}

fn path_grant(id: u128, subject: &str, revocable: bool) -> CapabilityGrant {
    CapabilityGrant::new(Arc::new(
        ResourcePathCapability::exact(CapabilityId(id), subject, "test://io", ["read"], "input")
            .unwrap()
            .revocable(revocable),
    ))
}

fn finite_grant(id: u128, max_uses: u64) -> CapabilityGrant {
    CapabilityGrant::new(Arc::new(
        BasicCapability::from_keys(CapabilityId(id), "task", "test://io/input", ["read"])
            .with_constraints(BasicConstraints::default().with_max_uses(max_uses)),
    ))
}

fn kernel(policy: NativeBenchmarkPolicy, capacity: usize) -> NativeBenchmarkCapabilityKernel {
    NativeBenchmarkCapabilityKernel::new(policy, capacity, &ManagedMemoryBudget::new(65536))
        .unwrap()
}

#[test]
fn policy_requires_an_exact_explicit_choice() {
    for (name, policy) in [
        ("secured", NativeBenchmarkPolicy::Secured),
        ("trusted", NativeBenchmarkPolicy::Trusted),
    ] {
        assert_eq!(NativeBenchmarkPolicy::parse(name).unwrap(), policy);
        assert_eq!(policy.name(), name);
    }
    for invalid in ["", "default", "Trusted", "trusted ", " secured", "true"] {
        assert!(
            NativeBenchmarkPolicy::parse(invalid).is_err(),
            "{invalid:?}"
        );
    }
}

#[test]
fn trusted_really_bypasses_operation_and_resource_matching() {
    for policy in [
        NativeBenchmarkPolicy::Secured,
        NativeBenchmarkPolicy::Trusted,
    ] {
        let mut kernel = kernel(policy, 1);
        kernel.grant(path_grant(1, "task", true)).unwrap();
        assert_eq!(
            kernel
                .check(&request("task", "read", "test://io/input"))
                .unwrap(),
            CapabilityId(1)
        );
        for denied in [
            request("task", "write", "test://io/input"),
            request("task", "read", "test://io/other"),
            request("task", "write", "another://provider/output"),
        ] {
            let result = kernel.check(&denied);
            if policy == NativeBenchmarkPolicy::Trusted {
                assert_eq!(result.unwrap(), CapabilityId(1));
            } else {
                assert!(result.is_err());
            }
        }
    }
}

#[test]
fn trusted_keeps_subject_scope_exclusion_and_revocation_boundaries() {
    let mut kernel = kernel(NativeBenchmarkPolicy::Trusted, 1);
    kernel.grant(path_grant(1, "task", true)).unwrap();
    let denied = request("task", "write", "test://io/other");
    let allowed = RuntimeAuthorityScope::allow_list([CapabilityId(1)]);
    let empty = RuntimeAuthorityScope::allow_list([]);
    let excluded = HashSet::from([CapabilityId(1)]);
    let pending = HashMap::new();
    assert!(
        kernel
            .check(&request("stranger", "write", "test://io/other"))
            .is_err()
    );
    assert_eq!(
        kernel.check_scoped(&denied, &allowed).unwrap(),
        CapabilityId(1)
    );
    assert!(kernel.check_scoped(&denied, &empty).is_err());
    assert!(kernel.check_excluding(&denied, &excluded).is_err());
    assert!(kernel.preview_check_excluding(&denied, &excluded).is_err());
    assert!(
        kernel
            .preview_check_excluding_with_pending_uses(&denied, &excluded, &pending)
            .is_err()
    );
    assert!(
        kernel
            .preview_scoped_with_transaction(&denied, &empty, &HashSet::new(), &pending)
            .is_err()
    );
    assert!(
        kernel
            .preview_scoped_with_transaction(&denied, &allowed, &excluded, &pending)
            .is_err()
    );
    assert_eq!(
        kernel
            .preview_scoped_with_transaction(&denied, &allowed, &HashSet::new(), &pending)
            .unwrap(),
        CapabilityId(1)
    );
    kernel
        .revoke(CapabilityRevocation::new(CapabilityId(1)))
        .unwrap();
    assert!(kernel.is_revoked(CapabilityId(1)).unwrap());
    assert!(kernel.check(&denied).is_err());
    assert!(kernel.preview_check(&denied).is_err());
    assert!(
        kernel
            .preview_scoped_with_transaction(&denied, &allowed, &HashSet::new(), &pending)
            .is_err()
    );
}

#[test]
fn previews_do_not_consume_but_checks_use_the_real_ledger() {
    let mut kernel = kernel(NativeBenchmarkPolicy::Trusted, 1);
    kernel.grant(path_grant(1, "task", true)).unwrap();
    let denied = request("task", "write", "test://io/other");
    let empty = HashSet::new();
    let pending = HashMap::from([(CapabilityId(1), 7)]);
    assert_eq!(kernel.preview_check(&denied).unwrap(), CapabilityId(1));
    assert_eq!(
        kernel.preview_check_excluding(&denied, &empty).unwrap(),
        CapabilityId(1)
    );
    assert_eq!(
        kernel
            .preview_check_excluding_with_pending_uses(&denied, &empty, &pending)
            .unwrap(),
        CapabilityId(1)
    );
    // This succeeds only if the previews above left the committed count at zero.
    kernel
        .apply_usage_delta(CapabilityId(1), u64::MAX - 1)
        .unwrap();
    assert_eq!(kernel.check(&denied).unwrap(), CapabilityId(1));
    assert!(kernel.check(&denied).is_err());
}

#[test]
fn finite_use_capabilities_retain_normal_matching_and_transaction_limits() {
    for policy in [
        NativeBenchmarkPolicy::Secured,
        NativeBenchmarkPolicy::Trusted,
    ] {
        let mut kernel = kernel(policy, 1);
        kernel.grant(finite_grant(1, 2)).unwrap();
        let read = request("task", "read", "test://io/input");
        let write = request("task", "write", "test://io/input");
        let empty = HashSet::new();
        let scope = RuntimeAuthorityScope::allow_list([CapabilityId(1)]);
        assert!(kernel.check(&write).is_err());
        assert!(kernel.preview_check(&write).is_err());
        assert_eq!(kernel.preview_check(&read).unwrap(), CapabilityId(1));
        assert_eq!(kernel.preview_check(&read).unwrap(), CapabilityId(1));
        let one = HashMap::from([(CapabilityId(1), 1)]);
        let two = HashMap::from([(CapabilityId(1), 2)]);
        assert_eq!(
            kernel
                .preview_scoped_with_transaction(&read, &scope, &empty, &one)
                .unwrap(),
            CapabilityId(1)
        );
        assert!(
            kernel
                .preview_check_excluding_with_pending_uses(&read, &empty, &two)
                .is_err()
        );
        assert_eq!(kernel.check(&read).unwrap(), CapabilityId(1));
        assert!(
            kernel
                .preview_scoped_with_transaction(&read, &scope, &empty, &one)
                .is_err()
        );
        assert_eq!(kernel.check(&read).unwrap(), CapabilityId(1));
        assert!(kernel.preview_check(&read).is_err());
        assert!(kernel.check(&read).is_err());
        assert!(kernel.apply_usage_delta(CapabilityId(1), 1).is_err());
    }
}

#[test]
fn checkpoint_restores_grants_revocations_and_finite_usage_together() {
    for policy in [
        NativeBenchmarkPolicy::Secured,
        NativeBenchmarkPolicy::Trusted,
    ] {
        let mut kernel = kernel(policy, 2);
        kernel.grant(finite_grant(1, 1)).unwrap();
        let read = request("task", "read", "test://io/input");
        let checkpoint = kernel.checkpoint().unwrap();
        kernel.check(&read).unwrap();
        kernel
            .revoke(CapabilityRevocation::new(CapabilityId(1)))
            .unwrap();
        kernel.grant(path_grant(2, "another", true)).unwrap();
        kernel.restore(checkpoint).unwrap();
        assert!(!kernel.is_revoked(CapabilityId(1)).unwrap());
        assert!(kernel.get(CapabilityId(2)).unwrap().is_none());
        assert_eq!(kernel.carriers.entries.len(), 1);
        assert_eq!(kernel.check(&read).unwrap(), CapabilityId(1));
        assert!(kernel.check(&read).is_err());
    }
}

#[test]
fn rollback_is_idempotent_even_for_nonrevocable_grants_and_id_reuse() {
    let mut kernel = kernel(NativeBenchmarkPolicy::Trusted, 1);
    kernel.grant(path_grant(1, "task", false)).unwrap();
    assert!(
        kernel
            .revoke(CapabilityRevocation::new(CapabilityId(1)))
            .is_err()
    );
    assert!(kernel.grant(path_grant(1, "stranger", true)).is_err());
    assert_eq!(
        kernel.get(CapabilityId(1)).unwrap().unwrap().subject_key(),
        "task"
    );
    kernel.rollback_grant(CapabilityId(1)).unwrap();
    kernel.rollback_grant(CapabilityId(1)).unwrap();
    assert!(kernel.carriers.entries.is_empty());
    kernel.grant(path_grant(1, "stranger", true)).unwrap();
    assert!(
        kernel
            .check(&request("task", "write", "test://io/other"))
            .is_err()
    );
    assert_eq!(
        kernel
            .check(&request("stranger", "write", "test://io/other"))
            .unwrap(),
        CapabilityId(1)
    );
    assert_eq!(
        kernel
            .list_for_subject(&BasicSubject::new("stranger"))
            .unwrap(),
        [CapabilityId(1)]
    );
}

#[test]
fn stale_carrier_arc_cannot_authorize_a_removed_or_replaced_capability() {
    let mut kernel = kernel(NativeBenchmarkPolicy::Trusted, 1);
    kernel.grant(path_grant(1, "task", true)).unwrap();
    // Exercise the defensive Arc-identity boundary independently of the wrapper's
    // normal administrative methods, which retire these aliases together.
    kernel.inner.rollback_grant(CapabilityId(1)).unwrap();
    let denied = request("task", "write", "test://io/other");
    assert!(kernel.check(&denied).is_err());
    kernel.inner.grant(path_grant(1, "task", true)).unwrap();
    assert!(kernel.check(&denied).is_err());
    assert!(kernel.preview_check(&denied).is_err());
}

#[path = "benchmark_policy_storage_tests.rs"]
mod storage;
