//! Pointwise source functions keep their complete scalar CFG inside the loop.
use super::*;
use mech_engine::native_artifact::{NativeCollectionSource, NativeControlLoop};

const SOURCE: &str = r#"
shift(value<u16>, threshold<u64>, enabled<bool>) => (out<u16>) :=
  wide<u64> := value
  shifted := ((wide < threshold)? | true => 0u64 | false => wide - threshold)
  out<u16> := (enabled? | true => shifted | false => wide).
transform(values<[u16]>, threshold<u64>, enabled<bool>) => (out<[u16]>) :=
  out := [shift(value, threshold, enabled) | value <- values].
answer := transform(input<[u16]>, threshold<u64>, enabled<bool>)
answer
"#;

#[test]
fn pointwise_unsigned_branches_run_without_per_element_runtime_callbacks() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let metadata = jit.descriptor().metadata();
        let generator = metadata
            .plan
            .iterations()
            .iter()
            .find_map(|control| match control {
                NativeControlLoop::Collection(generator)
                    if matches!(generator.source(), NativeCollectionSource::Value(_)) =>
                {
                    Some(generator)
                }
                _ => None,
            })
            .unwrap();
        let push = metadata.plan.collections()[generator.collection() as usize].push();
        let memory = mode.memory_budget();
        let cells = [0u16, 1, 32767, 32768, 65534, 65535];
        for restored in [false, true] {
            let mut output = vec![None; artifact.outputs().len()];
            for (threshold, enabled) in [
                (32768, true),
                (1, true),
                (u64::MAX, true),
                (5, false),
                (0, true),
                (1, false),
            ] {
                let args = inputs(
                    &artifact,
                    &[
                        ("input", Data::Matrix(cells.map(Data::U16).into())),
                        ("threshold", Data::U64(threshold)),
                        ("enabled", Data::Bool(enabled)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let before = contents(&args);
                let (result, counts) = if restored {
                    observe::run_restored(&jit, &args, &mut output, mode.execution(), &memory)
                } else {
                    observe::run(&jit, &args, &mut output, mode.execution(), &memory)
                };
                result.unwrap();
                let expected = cells
                    .into_iter()
                    .map(|value| {
                        Data::U16(if enabled {
                            u64::from(value).saturating_sub(threshold) as u16
                        } else {
                            value
                        })
                    })
                    .collect();
                assert_eq!(
                    output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::Matrix(expected)
                );
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
                for site in [generator.body(), push] {
                    assert_eq!(
                        counts
                            .helpers
                            .get(&(u64::from(site), GlobalHelper::Collection.raw()))
                            .copied()
                            .unwrap_or(0),
                        0,
                        "the complete pointwise scalar computation must use the admitted source/sink views, not a callback per element: {counts:?}"
                    );
                }
            }
        }
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
