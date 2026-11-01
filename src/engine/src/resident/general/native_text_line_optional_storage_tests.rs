use super::*;

fn setup(
    memory_limit: u64,
    temporary: u64,
) -> (
    ManagedMemoryBudget,
    NativeLibraryWorkBudget,
    Rc<Account>,
    ManagedMemoryReservation,
) {
    let memory = ManagedMemoryBudget::new(memory_limit);
    let work = NativeLibraryWorkBudget::new(1 << 40, 1 << 40, &memory).unwrap();
    let mut limits = mech_core::TargetMemoryProfile::current_resident_cpu()
        .unwrap()
        .limits;
    limits.max_temporary_bytes = Some(temporary);
    limits.max_compute_work = Some(1 << 40);
    let (account, fixed) = Account::new(limits, &work, 64, 128).unwrap();
    (memory, work, account, fixed)
}

#[test]
fn preferred_capture_backing_holds_real_capacity_and_one_cumulative_debit() {
    let (memory, work, account, fixed) = setup(4 << 20, 4 << 20);
    let before_memory = memory.used_bytes();
    let before_work = work.used();
    let before_private = account.used();
    let claim = Account::reserve_preferred(&account, 2 << 20)
        .unwrap()
        .expect("funded preference");
    assert_eq!(memory.used_bytes(), before_memory + (2 << 20));
    assert_eq!(account.used(), before_private + (2 << 20));
    assert_eq!(
        work.used(),
        (before_work.0 + (2 << 20) * 8 + 128, before_work.1)
    );
    assert_eq!(account.spent(), work.used().0);
    drop(claim);
    assert_eq!(memory.used_bytes(), before_memory);
    assert_eq!(account.used(), before_private);
    assert_eq!(work.used().0, before_work.0 + (2 << 20) * 8 + 128);
    drop((account, fixed, work));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn preferred_capture_backing_losing_shared_headroom_keeps_small_path_eligible() {
    let (memory, work, account, fixed) = setup(4 << 20, 4 << 20);
    // Another domain consumes the shared account after storage was preferred.
    let competing = memory
        .reserve_capacity(memory.limit_bytes() - memory.used_bytes() - 65536)
        .unwrap();
    let before = (
        memory.used_bytes(),
        account.used(),
        account.spent(),
        work.used(),
    );
    assert!(
        Account::reserve_preferred(&account, 2 << 20)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        before,
        (
            memory.used_bytes(),
            account.used(),
            account.spent(),
            work.used()
        )
    );
    let bytes = Bytes::copy(&"x".repeat(65536), &account).unwrap();
    assert_eq!(memory.used_bytes(), memory.limit_bytes());
    drop((bytes, competing, account, fixed, work));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn preferred_capture_backing_declines_insufficient_temporary_or_work_without_spending() {
    for temporary in [1 << 20, 4 << 20] {
        let (memory, work, account, fixed) = setup(4 << 20, temporary);
        if temporary == 4 << 20 {
            work.restrict(work.used().0 + (2 << 20) * 8 + 127, 1 << 40)
                .unwrap();
        }
        let before = (
            memory.used_bytes(),
            account.used(),
            account.spent(),
            work.used(),
        );
        assert!(
            Account::reserve_preferred(&account, 2 << 20)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            before,
            (
                memory.used_bytes(),
                account.used(),
                account.spent(),
                work.used()
            )
        );
        let bytes = Bytes::copy(&"x".repeat(65536), &account).unwrap();
        drop((bytes, account, fixed, work));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn preferred_capture_backing_rejects_foreign_active_work_and_checks_overflow() {
    let (memory, work, account, fixed) = setup(4 << 20, 4 << 20);
    let foreign = NativeLibraryWorkBudget::new(1 << 40, 1 << 40, &memory).unwrap();
    let before = (
        memory.used_bytes(),
        account.used(),
        account.spent(),
        work.used(),
    );
    let result = foreign
        .with_scope(|| Account::reserve_preferred(&account, 4096))
        .unwrap();
    assert!(result.is_err());
    for bytes in [0, u64::MAX] {
        assert!(
            Account::reserve_preferred(&account, bytes)
                .unwrap()
                .is_none()
        );
    }
    assert_eq!(
        before,
        (
            memory.used_bytes(),
            account.used(),
            account.spent(),
            work.used()
        )
    );
    drop((foreign, account, fixed, work));
    assert_eq!(memory.used_bytes(), 0);
}
