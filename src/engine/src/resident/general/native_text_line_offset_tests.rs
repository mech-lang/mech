//! Exact offset backing and the producer's existing admission/error ordering.
use super::*;
use mech_core::{MemoryRuntimeError, TargetMemoryProfile};

const MEMORY: u64 = 1024 * 1024;

fn limits() -> MemoryBudgetLimits {
    let mut limits = TargetMemoryProfile::current_resident_cpu().unwrap().limits;
    limits.max_temporary_bytes = Some(MEMORY);
    limits.max_compute_work = Some(1 << 40);
    limits
}

fn setup_bytes() -> u64 {
    core::mem::size_of::<Account>() as u64 + 512
}

fn debit(bytes: u64) -> u64 {
    bytes * 8 + 128
}

fn account(
    memory: &ManagedMemoryBudget,
    limits: MemoryBudgetLimits,
) -> (
    NativeLibraryWorkBudget,
    Rc<Account>,
    ManagedMemoryReservation,
) {
    let work = NativeLibraryWorkBudget::new(1 << 42, 1 << 32, memory).unwrap();
    let (account, fixed) = Account::new(limits, &work, 0, 0).unwrap();
    assert_eq!(fixed.capacity_bytes(), setup_bytes());
    (work, account, fixed)
}

#[test]
fn exact_offsets_preserve_utf8_crlf_payload_and_release_only_capacity() {
    for (text, expected) in [
        ("", &[][..]),
        ("no newline 猫\r", &[][..]),
        ("\n", &[0][..]),
        ("\né\r\n猫\n", &[0, 4, 8][..]),
        ("\r\r\n\nlast", &[2, 3][..]),
    ] {
        let memory = ManagedMemoryBudget::new(MEMORY);
        let (work, account, fixed) = account(&memory, limits());
        let before_memory = memory.used_bytes();
        let before_used = account.used();
        let before_work = work.used();
        let before_spent = account.spent();
        let bytes = core::mem::size_of_val(expected) as u64;
        let payload = text.lines().map(str::len).sum();
        let offsets = Offsets::new(text, expected.len(), payload, &account).unwrap();
        assert_eq!(&*offsets.values, expected);
        assert_eq!(offsets.payload_bytes, payload);
        assert_eq!(memory.used_bytes(), before_memory + bytes);
        assert_eq!(account.used(), before_used + bytes);
        assert_eq!(account.peak(), before_used + bytes);
        assert_eq!(account.spent(), before_spent + debit(bytes));
        assert_eq!(work.used(), (before_work.0 + debit(bytes), before_work.1));
        drop(offsets);
        assert_eq!(memory.used_bytes(), before_memory);
        assert_eq!(account.used(), before_used);
        assert_eq!(account.spent(), before_spent + debit(bytes));
        assert_eq!(work.used(), (before_work.0 + debit(bytes), before_work.1));
        drop((account, fixed, work));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn mismatched_counts_fail_without_renewing_work_or_leaking_claims() {
    // Zero and one understate the genuine two offsets; three overstates them.
    // No underestimated count may grow the Vec beyond its prepaid capacity.
    for count in [0, 1, 3] {
        let memory = ManagedMemoryBudget::new(MEMORY);
        let (work, account, fixed) = account(&memory, limits());
        let before_memory = memory.used_bytes();
        let before_used = account.used();
        let before_work = work.used();
        let before_spent = account.spent();
        let bytes = (count * core::mem::size_of::<u32>()) as u64;
        assert!(matches!(
            Offsets::new("\n\n", count, 0, &account),
            Err(InvokeError::InvalidInput)
        ));
        assert_eq!(memory.used_bytes(), before_memory);
        assert_eq!(account.used(), before_used);
        assert_eq!(account.peak(), before_used + bytes);
        assert_eq!(account.spent(), before_spent + debit(bytes));
        assert_eq!(work.used(), (before_work.0 + debit(bytes), before_work.1));
        drop((account, fixed, work));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn overflowing_offset_capacity_fails_before_any_admission_or_debit() {
    let memory = ManagedMemoryBudget::new(MEMORY);
    let (work, account, fixed) = account(&memory, limits());
    let before = (
        memory.used_bytes(),
        account.used(),
        account.peak(),
        account.spent(),
        work.used(),
    );
    let count = usize::MAX / core::mem::size_of::<u32>() + 1;
    assert!(matches!(
        Offsets::new("", count, 0, &account),
        Err(InvokeError::InvalidInput)
    ));
    assert_eq!(
        (
            memory.used_bytes(),
            account.used(),
            account.peak(),
            account.spent(),
            work.used()
        ),
        before
    );
    drop((account, fixed, work));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn exact_and_one_short_local_capacity_preserve_admission_before_debit() {
    let bytes = 2 * core::mem::size_of::<u32>() as u64;
    for short in [false, true] {
        let memory = ManagedMemoryBudget::new(MEMORY);
        let mut limits = limits();
        let limit = setup_bytes() + bytes - u64::from(short);
        limits.max_temporary_bytes = Some(limit);
        let (work, account, fixed) = account(&memory, limits);
        let before_memory = memory.used_bytes();
        let before_work = work.used();
        let before_spent = account.spent();
        let result = Offsets::new("\n\n", 2, 0, &account);
        if short {
            assert!(matches!(
                result,
                Err(InvokeError::Kernel(ResidentKernelError::ResourceLimit {
                    phase: ResidentResourcePhase::Admission,
                    dimension: Dimension::TemporaryBytes,
                    required,
                    limit: actual_limit,
                })) if required == setup_bytes() + bytes && actual_limit == limit
            ));
            assert_eq!(work.used(), before_work);
            assert_eq!(account.spent(), before_spent);
            assert_eq!(account.peak(), setup_bytes());
        } else {
            let offsets = result.unwrap();
            assert_eq!(&*offsets.values, &[0, 1]);
            assert_eq!(account.used(), limit);
            assert_eq!(work.used(), (before_work.0 + debit(bytes), before_work.1));
            drop(offsets);
        }
        assert_eq!(memory.used_bytes(), before_memory);
        assert_eq!(account.used(), setup_bytes());
        drop((account, fixed, work));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn exact_and_one_short_local_work_preserve_compute_failure_before_allocation() {
    let bytes = 2 * core::mem::size_of::<u32>() as u64;
    let required = debit(setup_bytes()) + debit(bytes);
    for short in [false, true] {
        let memory = ManagedMemoryBudget::new(MEMORY);
        let mut limits = limits();
        let limit = required - u64::from(short);
        limits.max_compute_work = Some(limit);
        let (work, account, fixed) = account(&memory, limits);
        let before_memory = memory.used_bytes();
        let before_work = work.used();
        let before_spent = account.spent();
        let result = Offsets::new("\n\n", 2, 0, &account);
        if short {
            assert!(matches!(
                result,
                Err(InvokeError::Kernel(ResidentKernelError::ResourceLimit {
                    phase: ResidentResourcePhase::Admission,
                    dimension: Dimension::ComputeWork,
                    required: actual_required,
                    limit: actual_limit,
                })) if actual_required == required && actual_limit == limit
            ));
            assert_eq!(work.used(), before_work);
            assert_eq!(account.spent(), before_spent);
            assert_eq!(account.peak(), setup_bytes());
        } else {
            drop(result.unwrap());
            assert_eq!(account.spent(), required);
            assert_eq!(work.used(), (before_work.0 + debit(bytes), before_work.1));
        }
        assert_eq!(memory.used_bytes(), before_memory);
        assert_eq!(account.used(), setup_bytes());
        drop((account, fixed, work));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn global_capacity_failure_keeps_prepaid_work_but_no_new_live_claim() {
    let probe = ManagedMemoryBudget::new(MEMORY);
    let work = NativeLibraryWorkBudget::new(1 << 42, 1 << 32, &probe).unwrap();
    let work_metadata = probe.used_bytes();
    drop(work);
    assert_eq!(probe.used_bytes(), 0);
    let bytes = 2 * core::mem::size_of::<u32>() as u64;
    let required = work_metadata + setup_bytes() + bytes;
    for short in [false, true] {
        let limit = required - u64::from(short);
        let memory = ManagedMemoryBudget::new(limit);
        let (work, account, fixed) = account(&memory, limits());
        let before_memory = memory.used_bytes();
        let before_work = work.used();
        let before_spent = account.spent();
        let result = Offsets::new("\n\n", 2, 0, &account);
        if short {
            assert!(matches!(
                result,
                Err(InvokeError::Memory(MemoryRuntimeError::BudgetExceeded {
                    operation: None,
                    requested,
                    limit: actual_limit,
                })) if requested == required && actual_limit == limit
            ));
            assert_eq!(account.peak(), setup_bytes());
        } else {
            let offsets = result.unwrap();
            assert_eq!(memory.used_bytes(), limit);
            assert_eq!(account.peak(), setup_bytes() + bytes);
            drop(offsets);
        }
        assert_eq!(account.spent(), before_spent + debit(bytes));
        assert_eq!(work.used(), (before_work.0 + debit(bytes), before_work.1));
        assert_eq!(memory.used_bytes(), before_memory);
        assert_eq!(account.used(), setup_bytes());
        drop((account, fixed, work));
        assert_eq!(memory.used_bytes(), 0);
    }
}
