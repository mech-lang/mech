//! The ordinary suffix keeps the prepaid root's exact scope and authority.
use super::*;

#[test]
fn numeric_interval_cold_suffix_drops_dead_carrier_pins_before_replacement() {
    for artifact in artifacts(source()) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let memory = mode.memory_budget();
        let args = arguments(&artifact, &memory, 3, 1, u64::MAX);
        let aliases = args.clone();
        let before = contents(&args);
        for restored in [false, true] {
            let mut out = vec![None; artifact.outputs().len()];
            let (result, counts) = if restored {
                observe::run_restored(&jit, &args, &mut out, mode.execution(), &memory)
            } else {
                observe::run(&jit, &args, &mut out, mode.execution(), &memory)
            };
            result.unwrap();
            assert_eq!(counts.cold_private_arrays.len(), 1);
            assert_eq!(
                counts.replacement_backings.len(),
                1,
                "the cache increments once in the ordinary suffix: {counts:?}"
            );
            assert!(
                counts
                    .replacement_backings
                    .iter()
                    .all(|(_, from, to)| from == to),
                "a dead compiler carrier must not force a new cache payload: {counts:?}"
            );
            assert_eq!(contents(&args), before);
            assert_eq!(contents(&aliases), before);
            assert_eq!(
                out.last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Tuple(
                    vec![
                        Data::Matrix([3, 1, 0].map(Data::U16).into()),
                        Data::Matrix([3, 0].map(Data::U32).into()),
                        Data::U64(3)
                    ]
                    .into()
                )
            );
        }
        drop((args, aliases));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn numeric_interval_cold_suffix_preserves_a_live_prior_cache_alias() {
    let source = source()
        .replace(
            "walk(nodes<[u16]>",
            "verify-prior(old<[u32]>, next<[u32]>) => (out<bool>) :=\n  before<u64> := old[1u64]\n  after<u64> := next[1u64]\n  separate! := before + 1u64 == after\n  out := true.\nwalk(nodes<[u16]>",
        )
        .replace(
            "  changed := (active?",
            "  prior-cache := cache\n  changed := (active?",
        )
        .replace(
            "  out := (active?",
            "  history! := (slide? | true => verify-prior(prior-cache, counted) | false => true)\n  out := (active?",
        );
    for artifact in artifacts(&source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let metadata = jit.descriptor().metadata();
        assert!(metadata.closed_tails.regions.iter().any(|region| {
            region.iteration == 0
                && region
                    .numeric
                    .as_ref()
                    .is_some_and(|proof| !proof.prepaid_sites.is_empty() && proof.exits.len() == 1)
        }));
        let memory = mode.memory_budget();
        let args = arguments(&artifact, &memory, 3, 1, u64::MAX);
        let aliases = args.clone();
        let before = contents(&args);
        for restored in [false, true] {
            let mut out = vec![None; artifact.outputs().len()];
            let (result, counts) = if restored {
                observe::run_restored(&jit, &args, &mut out, mode.execution(), &memory)
            } else {
                observe::run(&jit, &args, &mut out, mode.execution(), &memory)
            };
            result.unwrap();
            assert_eq!(counts.cold_private_arrays.len(), 1);
            assert_eq!(counts.replacement_backings.len(), 1);
            assert!(
                counts
                    .replacement_backings
                    .iter()
                    .all(|(_, from, to)| from != to),
                "the observed prior cache still requires a distinct payload: {counts:?}"
            );
            assert_eq!(contents(&args), before);
            assert_eq!(contents(&aliases), before);
            assert_eq!(
                out.last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Tuple(
                    vec![
                        Data::Matrix([3, 1, 0].map(Data::U16).into()),
                        Data::Matrix([3, 0].map(Data::U32).into()),
                        Data::U64(3),
                    ]
                    .into()
                )
            );
        }
        drop((args, aliases));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

fn unchanged(out: &[Option<Value>], before: &[Option<Value>]) {
    for (value, prior) in out.iter().zip(before) {
        let (value, prior) = (value.as_ref().unwrap(), prior.as_ref().unwrap());
        assert!(value.shares_frozen_storage(prior));
        assert_eq!(
            value.canonical_data_draft().unwrap(),
            prior.canonical_data_draft().unwrap()
        );
    }
}

#[test]
fn numeric_interval_finite_cold_suffix_keeps_publication_account_and_retry() {
    for artifact in artifacts(source()) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let memory = mode.memory_budget();
        let args = arguments(&artifact, &memory, 3, 1, u64::MAX);
        let aliases = args.clone();
        let before = contents(&args);
        for restored in [false, true] {
            let mut out = vec![None; artifact.outputs().len()];
            let run = |out: &mut [Option<Value>], policy| {
                if restored {
                    observe::run_restored(&jit, &args, out, policy, &memory)
                } else {
                    observe::run(&jit, &args, out, policy, &memory)
                }
            };
            run(&mut out, mode.execution()).0.unwrap();
            let full = run(&mut out, mode.execution()).0.unwrap();
            let prior = out.clone();
            let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
            let mut finite = mode.execution();
            finite.execution.max_work = full.work - 1;
            let (failure, counts) = run(&mut out, finite);
            let error = format!("{:?}", failure.unwrap_err());
            assert!(error.contains("WorkLimit"), "{error}");
            assert_eq!(counts.cold_private_arrays.len(), 1);
            assert!(counts.phase_settlements.iter().any(|x| x.0 == 6));
            unchanged(&out, &prior);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                retained
            );
            assert_eq!(contents(&args), before);
            assert_eq!(contents(&aliases), before);
            assert_eq!(run(&mut out, mode.execution()).0.unwrap().work, full.work);
        }
        drop((args, aliases));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn numeric_interval_images_reconstruct_native_cuts_and_prepaid_membership() {
    for artifact in artifacts(source()) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let image: serde_json::Value =
            serde_json::from_slice(&jit.descriptor().encode_build_owned_image().unwrap()).unwrap();
        let mut outside = image
            .pointer("/descriptor/closed_tails/regions/0/numeric/prepaid_sites")
            .unwrap()
            .as_array()
            .unwrap()
            .clone();
        assert!(
            outside
                .windows(2)
                .all(|pair| pair[0].as_u64() < pair[1].as_u64())
        );
        let mut reordered = outside.clone();
        reordered.swap(0, 1);
        let mut duplicate = outside.clone();
        duplicate.insert(1, outside[0].clone());
        outside.insert(0, serde_json::json!(0));
        for (path, replacement) in [
            (
                "/descriptor/closed_tails/regions/0/numeric/prepaid_sites",
                serde_json::json!(reordered),
            ),
            (
                "/descriptor/closed_tails/regions/0/numeric/prepaid_sites",
                serde_json::json!(duplicate),
            ),
            (
                "/descriptor/closed_tails/regions/0/numeric/prepaid_sites",
                serde_json::json!(outside),
            ),
            (
                "/descriptor/closed_tails/regions/0/numeric/prepaid_sites",
                serde_json::json!([]),
            ),
            (
                "/descriptor/closed_tails/regions/0/numeric/exits/0/to",
                serde_json::json!(0),
            ),
            (
                "/descriptor/closed_tails/regions/0/numeric/exits/0/from",
                serde_json::json!(0),
            ),
            (
                "/descriptor/closed_tails/regions/0/numeric/exits/0/aliases",
                serde_json::json!([]),
            ),
            (
                "/descriptor/closed_tails/regions/0/numeric/exits/0/retire_carrier",
                serde_json::json!(false),
            ),
            (
                "/descriptor/closed_tails/regions/0/numeric/phase/loops/0/compute",
                serde_json::json!(0),
            ),
            (
                "/descriptor/closed_tails/regions/0/numeric/result",
                serde_json::json!("Unreachable"),
            ),
        ] {
            let mut bad = image.clone();
            let field = bad.pointer_mut(path).unwrap();
            assert_ne!(*field, replacement, "{path}");
            *field = replacement;
            assert!(
                super::super::super::tests::rejects_malformed_helper_image(
                    &serde_json::to_vec(&bad).unwrap()
                ),
                "{path}"
            );
        }
    }
}
