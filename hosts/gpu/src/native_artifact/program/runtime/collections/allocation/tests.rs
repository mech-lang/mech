use crate::native_artifact::program::runtime::collections::allocation::*;

#[test]
fn owned_vec_rejection_drops_private_payloads_while_claim_is_live() {
    struct Probe<'a>(&'a std::cell::Cell<u32>, ManagedMemoryBudget);
    impl Drop for Probe<'_> {
        fn drop(&mut self) {
            assert_eq!(self.1.used_bytes(), 128);
            self.0.set(self.0.get() + 1);
        }
    }
    let memory = ManagedMemoryBudget::new(128);
    let reservation = memory.reserve_capacity(128).unwrap();
    let drops = std::cell::Cell::new(0);
    let mut values = Vec::with_capacity(3);
    values.push(Probe(&drops, memory.clone()));
    let result = exact_capacity(values, 2, "draft capacity");
    assert!(result.is_err());
    assert_eq!(drops.get(), 1);
    assert_eq!(memory.used_bytes(), 128);
    drop(reservation);
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn metadata_rejects_reported_spare_capacity_before_initialization() {
    let memory = ManagedMemoryBudget::new(4096);
    let reservation = memory.reserve_capacity(128).unwrap();
    let result = metadata_with::<u64>(2, "metadata capacity", |_| Ok(Vec::with_capacity(3)));
    assert!(result.is_err());
    assert_eq!(memory.used_bytes(), 128);
    drop(reservation);
    assert_eq!(memory.used_bytes(), 0);
    for count in [0, 1, 7] {
        let values = metadata::<u64>(count, "metadata capacity").unwrap();
        assert_eq!(values.len(), count);
        assert_eq!(values.capacity(), count);
        assert!(values.iter().all(Option::is_none));
    }
}

#[test]
fn text_rejects_reported_spare_capacity_without_replacing_live_backing() {
    let memory = ManagedMemoryBudget::new(256);
    let (mut old, old_claim) = text(64, &memory).unwrap();
    old.push_str("previous é");
    let result = text_with(128, &memory, |_| Ok(String::with_capacity(129)));
    assert!(result.is_err());
    assert_eq!(old, "previous é");
    assert_eq!(old.capacity(), 64);
    assert_eq!(memory.used_bytes(), 64);
    let (replacement, replacement_claim) = text(128, &memory).unwrap();
    assert_eq!(replacement.capacity(), 128);
    assert_eq!(memory.used_bytes(), 192);
    drop(replacement);
    drop(replacement_claim);
    assert_eq!(memory.used_bytes(), 64);
    drop(old);
    drop(old_claim);
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn text_exact_and_one_short_keep_the_existing_coexistence_claim() {
    let memory = ManagedMemoryBudget::new(192);
    let (old, old_claim) = text(64, &memory).unwrap();
    let (new, new_claim) = text(128, &memory).unwrap();
    assert_eq!(memory.used_bytes(), 192);
    drop(new);
    drop(new_claim);
    let ballast = memory.reserve_capacity(1).unwrap();
    let called = std::cell::Cell::new(false);
    let result = text_with(128, &memory, |_| {
        called.set(true);
        Ok(String::new())
    });
    assert!(result.is_err());
    assert!(!called.get(), "memory denial must precede allocation");
    assert_eq!(memory.used_bytes(), 65);
    drop(ballast);
    drop(old);
    drop(old_claim);
    assert_eq!(memory.used_bytes(), 0);
}
