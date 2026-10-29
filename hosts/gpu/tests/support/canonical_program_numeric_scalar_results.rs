//! Scalar result words compose with a numeric leaf's managed carrier.
use super::*;
use super::super::{equal, reference_result};

const PAIR: &str = r#"walk(xs<[u32]>, i<u64>, end<u64>, sum<u64>, count<u64>) => (out<(u64,u64)>) :=
  more := i < end
  word<u64> := (more? | true => xs[i + 1u64] | false => 0u32)
  out := (more? | true => walk(xs, i + 1u64, end, sum + word, count + 1u64) | false => (sum + 17u64, count + 5u64)).
(a,b) := walk(data<[u32]>, 0u64, steps<u64>, seed<u64>, 0u64)
(a + 0u64, b + 0u64)
"#;

fn record_source() -> String {
    PAIR.replace("out<(u64,u64)>", "out<{total<u64> count<u64>}>")
        .replace("(sum + 17u64, count + 5u64)", "{total: sum + 17u64, count: count + 5u64}")
        .replace("(a,b) := walk", "result := walk")
        .replace("(a + 0u64, b + 0u64)", "(result.total + 0u64, result.count + 0u64)")
}

fn selected(source: &str) {
    for artifact in artifacts(source).0 {
        let metadata = emit_program_object(&artifact, &mech_stdlib::source_catalog())
            .unwrap()
            .metadata();
        let regions = metadata["compiled_readonly_numeric_loops"].as_array().unwrap();
        assert!(
            !regions.is_empty(),
            "private scalar results must select the existing numeric leaf: {}",
            metadata["readonly_numeric_candidates"]
        );
        assert!(regions.iter().all(|r| r["successful_backedge_helpers"] == 0));
    }
}

#[test]
fn scalar_results_select_tuple_and_named_record_numeric_leaves() {
    // The one-scalar control already selects; only the return representation
    // differs from the pair. Both keep the same managed read-only carrier.
    let scalar = PAIR.replace("out<(u64,u64)>", "out<u64>")
        .replace("(sum + 17u64, count + 5u64)", "sum + 17u64")
        .replace("(a,b) :=", "a :=")
        .replace("(a + 0u64, b + 0u64)", "a + 0u64");
    selected(&scalar);
    selected(PAIR);
    selected(&record_source());
}

#[test]
fn scalar_results_preserve_fresh_visits_without_transport_slope() {
    for (name, source) in [("tuple", PAIR.to_owned()), ("record", record_source())] {
        for (form, artifact) in ["original", "decoded"].into_iter().zip(artifacts(&source).0) {
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let mut out = vec![None; artifact.outputs().len()];
            let mut observations = Vec::new();
            for steps in [0, 1, 8, 64, 256, 1] {
                let seed = 29 + steps;
                let args = inputs(&artifact, &[
                    ("data", Data::Matrix((1..=256).map(Data::U32).collect())),
                    ("steps", Data::U64(steps)), ("seed", Data::U64(seed))
                ], &memory);
                let aliases = args.clone();
                jit.evaluate_into(&args, &mut out, mode.execution(), &memory).unwrap();
                let (run, requests, bytes) = allocation_probe::measured_with_bytes(||
                    jit.evaluate_into(&args, &mut out, mode.execution(), &memory));
                let report = run.unwrap();
                assert_eq!(out.last().unwrap().as_ref().unwrap().canonical_data_draft().unwrap(),
                    Data::Tuple(vec![Data::U64(seed + steps * (steps + 1) / 2 + 17), Data::U64(steps + 5)].into()));
                if steps <= 8 {
                    equal(&artifact, &out, &reference_result(&artifact, &args).unwrap());
                }
                assert!(args.iter().zip(&aliases).all(|(a,b)| a.shares_frozen_storage(b)));
                observations.push((steps, requests, bytes, report.visits));
            }
            println!("{}", serde_json::json!({"witness":"scalar-numeric-results","case":name,"form":form,"observations":observations,"allocator":"System/probe; excluded from primary timing"}));
            assert!(observations.iter().all(|r| (r.1,r.2,r.3) == (observations[0].1,observations[0].2,observations[0].3)),
                "numeric scalar-result transport remains: {observations:?}");
            drop(out);
            assert_eq!((memory.used_bytes(),memory.retained_snapshot_nodes()),(0,0));
        }
    }
}
