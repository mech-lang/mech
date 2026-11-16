//! A projected array may still have a dead canonical alias from its frame pack.
use super::{
    Data, JitProgramArtifact, ManagedMemoryBudget, NativeProgramLimitMode, allocation_probe,
    artifacts, inputs,
};

fn source(width: u64, old_after: bool, sibling_after: bool) -> String {
    let old = "  observed<u64> := read(words, 1u64)\n";
    let tag = "  tag := state.tag\n";
    format!(
        "read(words<[u{width}]>, at<u64>) => (out<u{width}>) :=\n  out := words[at].\nrun(count<u64>, incoming<u{width}>, permit<bool>) => (out<u64>) :=\n  words := [0u{width} | i <- 0u64..=count, i < count]\n  state := {{cache: words, tag: 3u64}}\n{}  view := state.cache\n{}  ~changed := view\n  changed[1u64] = incoming\n{}{}  allowed! := permit\n  written<u64> := read(changed, 1u64)\n  out := observed + written + tag.\nrun(count<u64>, incoming<u{width}>, permit<bool>)\n",
        if sibling_after { "" } else { tag },
        if old_after { "" } else { old },
        if old_after { old } else { "" },
        if sibling_after { tag } else { "" },
    )
}

fn word(width: u64, value: u64) -> Data {
    if width == 16 {
        Data::U16(value as u16)
    } else {
        Data::U32(value as u32)
    }
}

#[test]
fn dead_pack_aliases_allow_one_real_payload_after_projection_and_last_read() {
    let mut measurements = Vec::new();
    for (old_after, sibling_after) in [(false, false), (true, false), (false, true)] {
        for (width, real) in [(16, 262144), (32, 1500513)] {
            for (form, artifact) in ["original", "decoded"]
                .into_iter()
                .zip(artifacts(&source(width, old_after, sibling_after)).0)
            {
                let mode = NativeProgramLimitMode::NoLimits;
                let jit = JitProgramArtifact::compile_with_limits(
                    &artifact,
                    &mech_stdlib::source_catalog(),
                    mode.compilation(),
                )
                .unwrap();
                for incoming in [0, 17] {
                    for count in [288, real] {
                        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
                        let args = inputs(
                            &artifact,
                            &[
                                ("count", Data::U64(count)),
                                ("incoming", word(width, incoming)),
                                ("permit", Data::Bool(true)),
                            ],
                            &memory,
                        );
                        let mut out = vec![None; artifact.outputs().len()];
                        jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                            .unwrap();
                        out.fill(None);
                        let (run, requests, bytes) = allocation_probe::measured_with_bytes(|| {
                            jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                        });
                        let report = run.unwrap();
                        assert_eq!(
                            out.last()
                                .unwrap()
                                .as_ref()
                                .unwrap()
                                .canonical_data_draft()
                                .unwrap(),
                            Data::U64(incoming + 3)
                        );
                        println!(
                            "{}",
                            serde_json::json!({"witness":"dead-pack-alias","form":form,"width":width,"count":count,"incoming":incoming,"old_after":old_after,"sibling_after":sibling_after,"allocation_requests":requests,"requested_bytes":bytes,"visits":report.visits,"work":report.work})
                        );
                        measurements.push((
                            width,
                            real,
                            form,
                            incoming,
                            count,
                            bytes,
                            old_after,
                            sibling_after,
                        ));
                        drop((args, out));
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            (0, 0)
                        );
                    }
                }
                super::super::super::record_buffer_qualification::assert_build_owned_image_round_trip(
                &jit,
            );
            }
        }
    }
    for (width, real, form, incoming, count, bytes, old_after, sibling_after) in &measurements {
        if *count == 288 {
            continue;
        }
        let small = measurements
            .iter()
            .find(|(w, _, f, n, c, _, old, sibling)| {
                w == width
                    && f == form
                    && n == incoming
                    && old == old_after
                    && sibling == sibling_after
                    && *c == 288
            })
            .unwrap()
            .5;
        assert_eq!(
            bytes - small,
            (1 + usize::from(*incoming != 0 && (*old_after || *sibling_after)))
                * ((real - 288) * (width / 8)) as usize,
            "{form} u{width} incoming={incoming} old_after={old_after} sibling_after={sibling_after}: only the initial payload and a required immutable-alias copy may scale with length"
        );
    }
}

#[test]
fn pack_alias_reuse_keeps_live_reads_siblings_failure_atomicity_and_retry() {
    for (old_after, sibling_after) in [(false, false), (true, false), (false, true)] {
        for artifact in artifacts(&source(32, old_after, sibling_after)).0 {
            let mode = NativeProgramLimitMode::NoLimits;
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut out = vec![None; artifact.outputs().len()];
            for (incoming, permit) in [(7, true), (17, true), (23, false), (31, true)] {
                let args = inputs(
                    &artifact,
                    &[
                        ("count", Data::U64(288)),
                        ("incoming", Data::U32(incoming)),
                        ("permit", Data::Bool(permit)),
                    ],
                    &memory,
                );
                let prior = out.clone();
                let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let run = jit.evaluate_into(&args, &mut out, mode.execution(), &memory);
                if !permit {
                    let error = format!("{:?}", run.unwrap_err());
                    assert!(error.contains("Integrity"), "{error}");
                    assert!(out.iter().zip(&prior).all(|(a, b)| {
                        a.as_ref()
                            .unwrap()
                            .shares_frozen_storage(b.as_ref().unwrap())
                    }));
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        before
                    );
                } else {
                    run.unwrap();
                    assert_eq!(
                        out.last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        Data::U64(u64::from(incoming) + 3)
                    );
                }
            }
            drop(out);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
