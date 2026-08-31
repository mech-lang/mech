use crate::{CanonicalStateJournal, Ref};

#[test]
fn exact_ref_capture_is_live_in_the_minimal_test_profile() {
    let target = Ref::new(7_u8);
    let mut journal = CanonicalStateJournal::new();

    journal.capture_exact_ref(&target).unwrap();
    *target.borrow_mut() = 9;
    journal.restore_before().unwrap();

    assert_eq!(*target.borrow(), 7);
}
