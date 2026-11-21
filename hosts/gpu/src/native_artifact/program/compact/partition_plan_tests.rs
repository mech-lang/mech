use super::*;

struct Fixture {
    reachable: Vec<bool>,
    edges: Vec<Vec<usize>>,
    uses: Vec<Vec<u32>>,
    defs: Vec<Vec<u32>>,
    ends: Vec<End>,
    words: Vec<usize>,
    constants: BTreeSet<u32>,
    scopes: Vec<Vec<usize>>,
    inputs: Vec<u32>,
    outputs: Vec<u32>,
}
impl Fixture {
    fn new() -> Self {
        Self {
            reachable: vec![true; 6],
            edges: vec![vec![1], vec![2, 3], vec![4], vec![4], vec![5], vec![]],
            uses: vec![vec![], vec![1], vec![0], vec![], vec![2], vec![2]],
            defs: vec![vec![], vec![], vec![2], vec![], vec![], vec![]],
            ends: vec![
                End::Continue,
                End::Continue,
                End::Continue,
                End::Continue,
                End::Continue,
                End::Complete,
            ],
            words: vec![1, 1, 6],
            constants: BTreeSet::from([1]),
            scopes: vec![vec![1, 2, 3]],
            inputs: vec![0, 2],
            outputs: vec![2],
        }
    }
    fn graph(&self) -> Graph<'_> {
        Graph {
            reachable: &self.reachable,
            successors: &self.edges,
            uses: &self.uses,
            defs: &self.defs,
            ends: &self.ends,
            words: &self.words,
            invariant_constants: &self.constants,
            scopes: &self.scopes,
            inputs: &self.inputs,
            outputs: &self.outputs,
        }
    }
}
#[test]
fn conditional_writes_capture_the_old_live_result_and_keep_local_dense_ids() {
    let f = Fixture::new();
    let p = f.graph().partition(&[(1, 4)]).unwrap();
    let r = &p.regions[0];
    assert_eq!((r.entry, r.exit), (1, 4));
    assert_eq!(r.sites, [1, 2, 3]);
    assert_eq!(r.inputs, [0, 2]);
    assert_eq!(r.outputs, [2]);
    assert_eq!(r.constants, [1]);
    assert_eq!(r.local_values, [0, 1, 2]);
    assert_eq!(r.local_words, 8);
    assert_eq!(r.scopes, [0]);
    assert_eq!(p.root_sites, [0, 4, 5]);
    assert_eq!(p.root_values, [0, 2]);
    assert_eq!(p.root_words, 7);
    assert_eq!(
        p.owners,
        [Some(0), Some(1), Some(1), Some(1), Some(0), Some(0)]
    );
}
#[test]
fn a_failure_path_does_not_require_a_success_result_but_keeps_its_source_site() {
    let mut f = Fixture::new();
    f.edges[3].clear();
    f.ends[3] = End::Failure;
    let p = f.graph().partition(&[(1, 4)]).unwrap();
    let r = &p.regions[0];
    assert_eq!(r.failures, [3]);
    assert_eq!(r.inputs, [0]);
    assert_eq!(r.outputs, [2]);
}
#[test]
fn external_middle_entries_and_split_hidden_resources_are_refused() {
    let mut f = Fixture::new();
    f.edges[0].push(3);
    assert!(matches!(
        f.graph().partition(&[(1, 4)]),
        Err(Error::OutsideEntry(3, 0))
    ));
    let mut f = Fixture::new();
    f.scopes.push(vec![0, 2]);
    assert!(matches!(
        f.graph().partition(&[(1, 4)]),
        Err(Error::SplitScope(1))
    ));
}
#[test]
fn closed_cycles_and_sparse_canonical_values_still_have_small_local_domains() {
    let mut f = Fixture::new();
    f.edges[3] = vec![1, 4];
    f.words = vec![0; 8192];
    f.words[0] = 1;
    f.words[1] = 1;
    f.words[8191] = 6;
    f.defs[2] = vec![8191];
    f.uses[4] = vec![8191];
    f.uses[5] = vec![8191];
    f.inputs = vec![0, 8191];
    f.outputs = vec![8191];
    let p = f.graph().partition(&[(1, 4)]).unwrap();
    let r = &p.regions[0];
    assert_eq!(r.local_values, [0, 1, 8191]);
    assert_eq!(r.local_words, 8);
    assert_eq!(r.inputs, [0, 8191]);
}
#[test]
fn terminal_completion_overlap_invalid_ids_and_word_overflow_are_refused() {
    let f = Fixture::new();
    assert!(matches!(
        f.graph().partition(&[(6, 4)]),
        Err(Error::InvalidBoundary(6, 4))
    ));
    assert!(matches!(
        f.graph().partition(&[(1, 1)]),
        Err(Error::InvalidBoundary(1, 1))
    ));
    assert!(matches!(
        f.graph().partition(&[(1, 4), (2, 4)]),
        Err(Error::Overlap(2))
    ));
    let mut f = Fixture::new();
    f.ends[2] = End::Complete;
    f.edges[2].clear();
    assert!(matches!(
        f.graph().partition(&[(1, 4)]),
        Err(Error::CompletionInside(2))
    ));
    let mut f = Fixture::new();
    f.edges[1] = vec![3];
    f.edges[3].clear();
    f.ends[3] = End::Failure;
    assert!(matches!(
        f.graph().partition(&[(1, 4)]),
        Err(Error::NoNormalExit(1))
    ));
    let mut f = Fixture::new();
    f.words[0] = usize::MAX;
    assert!(matches!(
        f.graph().partition(&[(1, 4)]),
        Err(Error::WordCountOverflow)
    ));
}
#[test]
fn mutable_initial_values_cannot_be_presented_as_invariant_constants() {
    let mut f = Fixture::new();
    f.defs[0] = vec![1];
    assert!(matches!(
        f.graph().partition(&[(1, 4)]),
        Err(Error::Malformed)
    ));
    f.constants.clear();
    let p = f.graph().partition(&[(1, 4)]).unwrap();
    assert_eq!(p.regions[0].inputs, [0, 1, 2]);
    assert!(p.regions[0].constants.is_empty());
}
#[test]
fn separate_regions_own_each_original_operation_once() {
    let f = Fixture::new();
    let p = f.graph().partition(&[(1, 4), (4, 5)]).unwrap();
    assert_eq!(p.root_sites, [0, 5]);
    assert_eq!(p.regions[0].sites, [1, 2, 3]);
    assert_eq!(p.regions[1].sites, [4]);
    assert_eq!(
        p.owners,
        [Some(0), Some(1), Some(1), Some(1), Some(2), Some(0)]
    );
}
