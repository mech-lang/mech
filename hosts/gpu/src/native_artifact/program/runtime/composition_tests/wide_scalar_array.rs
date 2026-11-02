//! A branch-heavy ranked scalar product beside an array exceeds tiny fixed shims.
use super::*;
use crate::native_artifact::program::prepare::readonly_numeric::Operation;

const SOURCE: &str = r#"+> sequence
<rank> := <(u8,u8,u8,u8,u8,u64,u64,u64,u64,u64)>
<state> := <([u8],rank)>
put(xs<[u8]>, at<u64>) => (out<[u8]>) :=
  ~next := xs
  next[at] = 0u8
  out := next.
insert(best<rank>, value<u8>, at<u64>) => (out<rank>) :=
  out := ((value > best.1)?
    | true => (value, best.1, best.2, best.3, best.4, at, best.6, best.7, best.8, best.9)
    | false => ((value > best.2)?
      | true => (best.1, value, best.2, best.3, best.4, best.6, at, best.7, best.8, best.9)
      | false => ((value > best.3)?
        | true => (best.1, best.2, value, best.3, best.4, best.6, best.7, at, best.8, best.9)
        | false => ((value > best.4)?
          | true => (best.1, best.2, best.3, value, best.4, best.6, best.7, best.8, at, best.9)
          | false => (best.1, best.2, best.3, best.4, value, best.6, best.7, best.8, best.9, at))))).
lane(xs<[u8]>, at<u64>, best<rank>) => (out<state>) :=
  value := xs[at + 1u64]
  selected := ((value > best.5)? | true => insert(best, value, at) | false => best)
  zeroed := put(xs, at + 1u64)
  out := (zeroed, selected).
walk(xs<[u8]>, at<u64>, end<u64>, best<rank>) => (out<state>) :=
  more := at < end
  step := (more? | true => lane(xs, at, best) | false => (xs, best))
  (next, selected) := step
  out := (more? | true => walk(next, at + 1u64, end, selected) | false => (next, selected)).
empty<rank> := (0u8,0u8,0u8,0u8,0u8,0u64,0u64,0u64,0u64,0u64)
result := walk(bytes<[u8]>, 0u64, steps<u64>, empty)
(result.1, result.2.1, result.2.2, result.2.3, result.2.4, result.2.5, result.2.6, result.2.7, result.2.8, result.2.9, result.2.10, bytes)
"#;

#[test]
fn wide_scalar_state_funds_its_complete_native_boundary() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        assert!(
            jit.descriptor()
                .metadata()
                .closed_tails
                .regions
                .iter()
                .any(|region| {
                    region.numeric.as_ref().is_some_and(|proof| {
                        proof
                            .actions
                            .iter()
                            .any(|(_, action)| matches!(action, Operation::Write { .. }))
                    })
                })
        );
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut outputs = vec![None; artifact.outputs().len()];
            for steps in [0, 6, 2] {
                let cells = [7, 12, 12, 2, 9, 5];
                let args = inputs(
                    &artifact,
                    &[
                        ("bytes", Data::Matrix(cells.map(Data::U8).into())),
                        ("steps", Data::U64(steps)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let before = contents(&args);
                let result = if restored {
                    observe::run_restored(&jit, &args, &mut outputs, mode.execution(), &memory).0
                } else {
                    observe::run(&jit, &args, &mut outputs, mode.execution(), &memory).0
                };
                result.unwrap();
                let mut selected = cells[..steps as usize]
                    .iter()
                    .copied()
                    .enumerate()
                    .collect::<Vec<_>>();
                selected.sort_by_key(|&(at, value)| (std::cmp::Reverse(value), at));
                selected.resize(5, (0, 0));
                selected.truncate(5);
                let mut zeroed = cells;
                zeroed[..steps as usize].fill(0);
                let mut expected = vec![Data::Matrix(zeroed.map(Data::U8).into())];
                expected.extend(selected.iter().map(|&(_, value)| Data::U8(value)));
                expected.extend(selected.iter().map(|&(at, _)| Data::U64(at as u64)));
                expected.push(Data::Matrix(cells.map(Data::U8).into()));
                assert_eq!(
                    outputs
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::Tuple(expected.into())
                );
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
            }
            drop(outputs);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
