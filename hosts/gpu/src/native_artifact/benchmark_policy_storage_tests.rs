use super::*;

fn kernel_bytes(capacity: usize) -> u64 {
    (std::mem::size_of::<NativeBenchmarkCapabilityKernel>()
        + std::mem::size_of::<Arc<dyn Capability>>() * capacity) as u64
}

fn checkpoint_bytes(capacity: usize) -> u64 {
    (std::mem::size_of::<Checkpoint>() + std::mem::size_of::<Arc<dyn Capability>>() * capacity)
        as u64
}

#[test]
fn exact_backing_is_admitted_and_one_short_leaves_no_reservation() {
    for capacity in [0, 1, 3] {
        let required = kernel_bytes(capacity);
        let short = ManagedMemoryBudget::new(required - 1);
        assert!(
            NativeBenchmarkCapabilityKernel::new(NativeBenchmarkPolicy::Trusted, capacity, &short)
                .is_err()
        );
        assert_eq!(short.used_bytes(), 0);
        let exact = ManagedMemoryBudget::new(required);
        let kernel =
            NativeBenchmarkCapabilityKernel::new(NativeBenchmarkPolicy::Trusted, capacity, &exact)
                .unwrap();
        assert_eq!(kernel.carriers.entries.capacity(), capacity);
        assert_eq!(exact.used_bytes(), required);
        drop(kernel);
        assert_eq!(exact.used_bytes(), 0);
    }
}

#[test]
fn grant_capacity_failure_precedes_any_partial_kernel_mutation() {
    let mut kernel = kernel(NativeBenchmarkPolicy::Trusted, 1);
    kernel.grant(path_grant(1, "task", true)).unwrap();
    assert!(kernel.grant(path_grant(2, "another", true)).is_err());
    assert!(kernel.get(CapabilityId(2)).unwrap().is_none());
    assert!(
        kernel
            .list_for_subject(&BasicSubject::new("another"))
            .unwrap()
            .is_empty()
    );
    assert_eq!(kernel.carriers.entries.len(), 1);
    assert_eq!(
        kernel
            .check(&request("task", "write", "test://io/other"))
            .unwrap(),
        CapabilityId(1)
    );
    let mut zero = super::kernel(NativeBenchmarkPolicy::Trusted, 0);
    assert!(zero.grant(path_grant(1, "task", true)).is_err());
    assert!(
        zero.check(&request("task", "read", "test://io/input"))
            .is_err()
    );
}

#[test]
fn checkpoint_backing_coexists_and_restore_keeps_the_kernel_object_charge() {
    let required = kernel_bytes(2);
    let snapshot = checkpoint_bytes(2);
    let budget = ManagedMemoryBudget::new(required + snapshot);
    let mut kernel =
        NativeBenchmarkCapabilityKernel::new(NativeBenchmarkPolicy::Trusted, 2, &budget).unwrap();
    kernel.grant(path_grant(1, "task", true)).unwrap();
    let checkpoint = kernel.checkpoint().unwrap();
    assert_eq!(budget.used_bytes(), required + snapshot);
    assert!(kernel.checkpoint().is_err());
    assert_eq!(budget.used_bytes(), required + snapshot);
    kernel.rollback_grant(CapabilityId(1)).unwrap();
    kernel.restore(checkpoint).unwrap();
    assert_eq!(budget.used_bytes(), required);
    assert_eq!(
        kernel
            .check(&request("task", "write", "test://io/other"))
            .unwrap(),
        CapabilityId(1)
    );
    drop(kernel);
    assert_eq!(budget.used_bytes(), 0);
}

#[test]
fn checkpoint_one_short_releases_its_partial_claim_without_changing_live_state() {
    let required = kernel_bytes(2);
    let snapshot = checkpoint_bytes(2);
    let budget = ManagedMemoryBudget::new(required + snapshot - 1);
    let mut kernel =
        NativeBenchmarkCapabilityKernel::new(NativeBenchmarkPolicy::Trusted, 2, &budget).unwrap();
    kernel.grant(path_grant(1, "task", true)).unwrap();
    assert!(kernel.checkpoint().is_err());
    assert_eq!(budget.used_bytes(), required);
    assert_eq!(
        kernel
            .check(&request("task", "write", "test://io/other"))
            .unwrap(),
        CapabilityId(1)
    );
    drop(kernel);
    assert_eq!(budget.used_bytes(), 0);
}

#[test]
fn foreign_checkpoint_account_capacity_or_type_is_rejected_without_corruption() {
    let budget = ManagedMemoryBudget::new(65536);
    let mut kernel =
        NativeBenchmarkCapabilityKernel::new(NativeBenchmarkPolicy::Trusted, 1, &budget).unwrap();
    kernel.grant(path_grant(1, "task", true)).unwrap();
    let foreign = super::kernel(NativeBenchmarkPolicy::Trusted, 1);
    let foreign_budget = foreign.memory.clone();
    assert!(kernel.restore(foreign.checkpoint().unwrap()).is_err());
    assert_eq!(foreign_budget.used_bytes(), kernel_bytes(1));
    let other_capacity =
        NativeBenchmarkCapabilityKernel::new(NativeBenchmarkPolicy::Trusted, 2, &budget).unwrap();
    assert!(
        kernel
            .restore(other_capacity.checkpoint().unwrap())
            .is_err()
    );
    assert!(
        kernel
            .restore(BasicCapabilityKernel::new().checkpoint().unwrap())
            .is_err()
    );
    assert_eq!(budget.used_bytes(), kernel_bytes(1) + kernel_bytes(2));
    assert_eq!(
        kernel
            .check(&request("task", "write", "test://io/other"))
            .unwrap(),
        CapabilityId(1)
    );
    drop(other_capacity);
    drop(kernel);
    assert_eq!(budget.used_bytes(), 0);
    drop(foreign);
    assert_eq!(foreign_budget.used_bytes(), 0);
}
