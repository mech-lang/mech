use crate::native_artifact::program::runtime::invocation::prefix::Retention;

#[test]
fn coordinates_reject_unknown_extents_regressions_and_sealed_range_mutation() {
    let open = Retention::checked([3, 0, 0], 3, None).unwrap();
    assert_eq!(
        open,
        Retention {
            first: 3,
            end: None
        }
    );
    for args in [
        [0, 0, 0],
        [5, 0, 0],
        [3, 1, 0],
        [3, 0, 2],
        [3, 2, 1],
        [3, 5, 1],
        [2, 0, 0],
        [2, 4, 1],
    ] {
        assert!(Retention::checked(args, 3, Some(open)).is_err(), "{args:?}");
    }
    let sealed = Retention::checked([3, 4, 1], 3, Some(open)).unwrap();
    assert_eq!(
        Retention::checked([3, 4, 1], 100, Some(sealed)).unwrap(),
        sealed
    );
    for args in [[3, 0, 0], [2, 4, 1], [3, 5, 1], [1, 1, 1]] {
        assert!(
            Retention::checked(args, 100, Some(sealed)).is_err(),
            "{args:?}"
        );
    }
    let empty = Retention::checked([1, 1, 1], 3, Some(open)).unwrap();
    assert_eq!(
        empty,
        Retention {
            first: 1,
            end: Some(1)
        }
    );
    assert!(Retention::checked([1, 2, 1], 100, Some(empty)).is_err());
    assert!(Retention::checked([1, 0, 0], u64::MAX, None).is_err());
}
