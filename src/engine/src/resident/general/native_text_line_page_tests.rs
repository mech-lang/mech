use super::*;
use crate::resident::budget::NativeLibraryWorkBudget;
use mech_core::{ManagedMemoryBudget, ManagedMemoryReservation};

fn setup(
    memory_limit: u64,
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
    limits.max_temporary_bytes = Some(16 << 20);
    limits.max_compute_work = Some(1 << 40);
    let (account, fixed) = Account::new(limits, &work, 64, 128).unwrap();
    (memory, work, account, fixed)
}

#[test]
fn mapped_capture_slices_keep_utf8_and_owners_after_pool_retirement() {
    let (memory, work, account, fixed) = setup(16 << 20);
    let before = memory.used_bytes();
    let mut pool = Pool::default();
    let ascii = "A\r\n\0".repeat(8192);
    let unicode = "猫é\n".repeat(4096);
    let a = pool.copy(&ascii, 8 << 20, &account).unwrap().unwrap();
    let b = pool.copy(&unicode, 8 << 20, &account).unwrap().unwrap();
    assert!(Rc::ptr_eq(&a.slab, &b.slab));
    assert_eq!(a.as_str(), ascii);
    assert_eq!(b.as_str(), unicode);
    let admitted = memory.used_bytes();
    assert!(admitted >= before + CAPACITY as u64);
    pool.clear();
    drop(a);
    assert_eq!(memory.used_bytes(), admitted);
    assert_eq!(b.as_str(), unicode);
    drop(b);
    assert_eq!(memory.used_bytes(), before);
    drop((pool, account, fixed, work));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn mapped_capture_recycles_only_after_every_published_slice_drops() {
    let (memory, work, account, fixed) = setup(16 << 20);
    let mut pool = Pool::default();
    let text = "x".repeat(MIN_FRAGMENT);
    let a = pool.copy(&text, 8 << 20, &account).unwrap().unwrap();
    let b = pool.copy(&text, 8 << 20, &account).unwrap().unwrap();
    assert_eq!(a.start, 0);
    assert_eq!(b.start, text.len());
    let pointer = a.slab.mapping.pointer;
    let charged = work.used();
    drop((a, b));
    let c = pool.copy(&text, 8 << 20, &account).unwrap().unwrap();
    assert_eq!(c.start, 0);
    assert_eq!(c.slab.mapping.pointer, pointer);
    assert_eq!(work.used(), charged);
    drop((c, pool, account, fixed, work));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn mapped_capture_releases_old_slabs_while_later_slices_remain() {
    let (memory, work, account, fixed) = setup(16 << 20);
    let before = memory.used_bytes();
    let mut pool = Pool::default();
    let a = pool
        .copy(&"x".repeat(CAPACITY), 8 << 20, &account)
        .unwrap()
        .unwrap();
    let b = pool
        .copy(&"y".repeat(MIN_FRAGMENT), 8 << 20, &account)
        .unwrap()
        .unwrap();
    assert!(!Rc::ptr_eq(&a.slab, &b.slab));
    let two = memory.used_bytes();
    drop(a);
    assert!(memory.used_bytes() <= two - CAPACITY as u64);
    assert!(memory.used_bytes() >= before + CAPACITY as u64);
    assert!(b.as_str().bytes().all(|byte| byte == b'y'));
    drop((b, pool, account, fixed, work));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn mapped_capture_small_or_unfunded_preference_keeps_heap_eligible() {
    let (memory, work, account, fixed) = setup(1 << 20);
    let before = (memory.used_bytes(), work.used(), account.used());
    let mut pool = Pool::default();
    let text = "x".repeat(MIN_FRAGMENT);
    assert!(
        pool.copy(&text, (CAPACITY - 1) as u64, &account)
            .unwrap()
            .is_none()
    );
    assert!(pool.copy("x", 8 << 20, &account).unwrap().is_none());
    assert!(pool.copy(&text, 8 << 20, &account).unwrap().is_none());
    assert_eq!(before, (memory.used_bytes(), work.used(), account.used()));
    let heap = super::super::Bytes::copy(&text, &account).unwrap();
    assert_eq!(heap.as_str(), text);
    drop((heap, pool, account, fixed, work));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn mapped_capture_rejects_a_foreign_account_before_reusing_bytes() {
    let (memory, work, account, fixed) = setup(16 << 20);
    let (_, other_work, other, other_fixed) = setup(16 << 20);
    let mut pool = Pool::default();
    let text = "x".repeat(MIN_FRAGMENT);
    let a = pool.copy(&text, 8 << 20, &account).unwrap().unwrap();
    let before = (work.used(), other_work.used());
    assert!(pool.copy(&text, 8 << 20, &other).is_err());
    assert_eq!(before, (work.used(), other_work.used()));
    assert_eq!(a.as_str(), text);
    drop((
        a,
        pool,
        account,
        fixed,
        work,
        other,
        other_fixed,
        other_work,
    ));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn mapped_capture_allocation_failure_is_funded_once_then_uses_heap() {
    let (memory, work, account, fixed) = setup(16 << 20);
    let before = memory.used_bytes();
    let mut pool = Pool::default();
    let text = "x".repeat(MIN_FRAGMENT);
    FAIL_NEXT.with(|flag| flag.set(true));
    assert!(pool.copy(&text, 8 << 20, &account).unwrap().is_none());
    assert_eq!(memory.used_bytes(), before);
    let failed_work = work.used();
    assert!(failed_work.0 > 0);
    assert!(pool.copy(&text, 8 << 20, &account).unwrap().is_none());
    assert_eq!(work.used(), failed_work);
    let heap = super::super::Bytes::copy(&text, &account).unwrap();
    drop((heap, pool, account, fixed, work));
    assert_eq!(memory.used_bytes(), 0);
}
