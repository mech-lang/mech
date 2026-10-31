//! Current argument fields through the real emitted entry, with an ordered
//! per-field fallback from that same entry as the differential reference.
use super::*;
use crate::native_artifact::program::protocol::abi::{Frame, Helper};
use std::cell::{Cell, RefCell};
use std::ffi::c_void;

const SOURCE: &str =
    include_str!("../../../../../tests/fixtures/native/frame-projection-batch.mec");
// Proposed private tag: the red test runs before its typed protocol exists.
const BATCH: u64 = u64::MAX - 29;
type Admit = unsafe extern "C" fn(*mut c_void, u64) -> u64;
#[derive(Default, Debug)]
struct Counts {
    admits: u64,
    helpers: u64,
    batches: u64,
}
thread_local! {
    static COUNTS: RefCell<Counts> = RefCell::new(Counts::default());
    static DECLINE: Cell<bool> = const { Cell::new(false) };
    static ADMIT: Cell<Option<Admit>> = const { Cell::new(None) };
    static HELPER: Cell<Option<Helper>> = const { Cell::new(None) };
}
unsafe extern "C" fn admit(context: *mut c_void, site: u64) -> u64 {
    COUNTS.with(|v| v.borrow_mut().admits += 1);
    // SAFETY: run retains the exact owning frame's synchronous callback/context.
    unsafe { ADMIT.with(Cell::get).unwrap()(context, site) }
}
unsafe extern "C" fn helper(
    context: *mut c_void,
    site: u64,
    operation: u64,
    a: u64,
    b: u64,
    c: u64,
    output: *mut u64,
) -> u64 {
    COUNTS.with(|v| {
        let mut counts = v.borrow_mut();
        counts.helpers += 1;
        counts.batches += u64::from(operation == BATCH);
    });
    if operation == BATCH && DECLINE.with(Cell::get) {
        // SAFETY: this is the owning checked entry's registered result lane.
        // Refuse before any action so its original per-field path must run.
        unsafe {
            *output = 0;
        }
        return 0;
    }
    // SAFETY: unchanged arguments/result lane from the retained emitted entry.
    unsafe { HELPER.with(Cell::get).unwrap()(context, site, operation, a, b, c, output) }
}
fn run(
    jit: &JitProgramArtifact,
    descriptor: &NativeProgramDescriptor,
    args: &[Value],
    published: &mut [Option<Value>],
    memory: &ManagedMemoryBudget,
    policy: NativeProgramExecutionLimits,
    decline: bool,
) -> (
    Result<NativeProgramExecutionReport, NativeProgramExecutionError>,
    Counts,
) {
    COUNTS.with(|v| *v.borrow_mut() = Counts::default());
    DECLINE.with(|v| v.set(decline));
    let entry = jit.test_checked_entry();
    let result = execute(
        descriptor,
        |frame: *mut Frame| {
            // SAFETY: execute retains this initialized frame through the call;
            // callbacks are restored before its immutable-frame checks.
            unsafe {
                let previous = ((*frame).base.admit_next, (*frame).helper);
                ADMIT.with(|v| v.set(Some(previous.0)));
                HELPER.with(|v| v.set(Some(previous.1)));
                (*frame).base.admit_next = admit;
                (*frame).helper = helper;
                let status = entry(frame);
                (*frame).base.admit_next = previous.0;
                (*frame).helper = previous.1;
                status
            }
        },
        args,
        published,
        policy,
        memory,
    );
    (
        result,
        COUNTS.with(|v| std::mem::take(&mut *v.borrow_mut())),
    )
}
fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    word: u8,
    left: u64,
    flag: bool,
    allowed: bool,
    divisor: u64,
    empty: bool,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            (
                "a",
                Data::Matrix(if empty { vec![] } else { vec![Data::U8(word)] }.into()),
            ),
            ("b", Data::Matrix([Data::U16(17)].into())),
            ("c", Data::Matrix([Data::U32(23)].into())),
            ("d", Data::Matrix([Data::U64(29)].into())),
            ("left", Data::U64(left)),
            ("number", Data::U64(2)),
            ("flag", Data::Bool(flag)),
            ("steps", Data::U64(2)),
            ("allowed", Data::Bool(allowed)),
            ("divisor", Data::U64(divisor)),
        ],
        memory,
    )
}

#[test]
fn managed_projections_keep_current_fields_ordered_errors_aliases_and_retry() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
            .unwrap();
        let image = jit.descriptor().encode_build_owned_image().unwrap();
        // SAFETY: this exact image and checked entry share the live JIT owner.
        let restored =
            unsafe { NativeProgramDescriptor::decode_build_owned_image(&image, &catalog) }.unwrap();
        assert_eq!(restored.encode_build_owned_image().unwrap(), image);
        for descriptor in [jit.descriptor(), &restored] {
            let mut reference_work = Vec::new();
            for decline in [false, true] {
                let memory = mode.memory_budget();
                let mut published = vec![None; artifact.outputs().len()];
                for (case, (word, left, flag, allowed, divisor, empty, error)) in [
                    (3, 2, true, true, 1, false, None),
                    (9, 3, false, true, 2, false, None),
                    (3, 2, true, false, 0, true, Some("Integrity")),
                    (3, 2, false, true, 1, true, Some("Library")),
                    (3, 2, true, true, 0, false, Some("Arithmetic")),
                    (11, 0, false, true, 3, false, None),
                ]
                .into_iter()
                .enumerate()
                {
                    let args = arguments(
                        &artifact, &memory, word, left, flag, allowed, divisor, empty,
                    );
                    let aliases = args.clone();
                    let before = contents(&args);
                    let prior = published.clone();
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let (result, counts) = run(
                        &jit,
                        descriptor,
                        &args,
                        &mut published,
                        &memory,
                        mode.execution(),
                        decline,
                    );
                    if let Some(error) = error {
                        assert!(format!("{:?}", result.unwrap_err()).contains(error));
                        assert!(published.iter().zip(&prior).all(|(a, b)| {
                            a.as_ref()
                                .unwrap()
                                .shares_frozen_storage(b.as_ref().unwrap())
                        }));
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            retained
                        );
                        if !decline {
                            reference_work.push(None);
                        }
                    } else {
                        let report = result.unwrap();
                        let sum = u64::from(word) + 17 * 10 + 23 * 100 + 29 * 1000;
                        let final_flag = flag ^ (left % 2 != 0);
                        let expected =
                            (2 + (left + 1) * (sum + 2) + u64::from(final_flag)) / divisor;
                        assert_eq!(
                            published[0]
                                .as_ref()
                                .unwrap()
                                .canonical_data_draft()
                                .unwrap(),
                            Data::U64(expected)
                        );
                        assert!(counts.batches > 0, "real bulk projection must execute: {counts:?}");
                        if decline {
                            assert_eq!(Some(report.work), reference_work[case]);
                        } else {
                            assert!(
                                counts.admits < report.visits,
                                "bulk transport must combine callbacks: {counts:?}"
                            );
                            reference_work.push(Some(report.work));
                        }
                    }
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                    assert!(
                        args.iter()
                            .zip(&aliases)
                            .all(|(a, b)| a.shares_frozen_storage(b))
                    );
                }
                drop(published);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}

#[test]
fn managed_projection_finite_boundaries_preserve_publication_and_recovery() {
    resources::finite_boundaries(SOURCE, |artifact, memory| {
        arguments(artifact, memory, 3, 2, true, true, 1, false)
    });
}
