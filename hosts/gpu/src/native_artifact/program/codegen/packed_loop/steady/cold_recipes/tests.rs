use crate::native_artifact::program::codegen::packed_loop::steady::cold_recipes::*;

#[test]
fn affine_merges_preserve_only_equal_values_and_exact_frame_progress() {
    let mut left = State {
        words: BTreeMap::from([
            (1, ScalarRecipe::Index(0)),
            (2, ScalarRecipe::Index(1)),
            (3, ScalarRecipe::Constant(7)),
        ]),
        updates: Some(0),
        previous: Some(0),
    };
    let right = State {
        words: BTreeMap::from([(1, ScalarRecipe::Index(0)), (2, ScalarRecipe::Index(2))]),
        updates: Some(1),
        previous: Some(0),
    };
    left.intersect(&right);
    assert_eq!(left.words, BTreeMap::from([(1, ScalarRecipe::Index(0))]));
    assert_eq!((left.updates, left.previous), (None, Some(0)));
    left.rebase_completed();
    assert_eq!(left.words[&1], ScalarRecipe::Index(u64::MAX));
    assert_eq!(left.previous, Some(u64::MAX));
}
