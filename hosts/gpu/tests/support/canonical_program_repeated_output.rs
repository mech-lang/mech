use super::{COMPONENTS, allocation_probe, complete_artifacts, inputs};
use mech_core::snapshot::SequenceView;
use mech_core::{ValueData, ValueDataDraft as Data};
use mech_gpu::native_artifact::{JitProgramArtifact, NativeProgramLimitMode};

pub(super) fn run() {
    let source = format!(
        "{}\ncompress-repeated(input<[u8]>, buffer<[u8]>, left<u64>) => (out<[u8]>) :=\n  out := ((left > 0u64)? | true => compress-repeated(input, deflate-compress-into(input, sequence/clear(buffer)), left - 1u64) | false => buffer).\ninitial := sequence/with-capacity(0u8, capacity<u64>)\ncompress-repeated(input<[u8]>, initial, rounds<u64>)\n",
        COMPONENTS
    );
    let forms = complete_artifacts(&source);
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/native/deflate-compress-reference.json"
    ))
    .unwrap();
    let case = fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["name"] == "eight-literals")
        .unwrap();
    let expected: Vec<u8> = serde_json::from_value(case["expected"].clone()).unwrap();
    let input: Vec<u8> = serde_json::from_value(case["input"].clone()).unwrap();
    let mode = NativeProgramLimitMode::NoLimits;
    for (form, artifact) in ["original", "decoded"].into_iter().zip(forms) {
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        for rounds in [0u64, 1, 2] {
            let mut measurements = Vec::new();
            for (sample, capacity) in [16u64, 16, 6633832].into_iter().enumerate() {
                let memory = mode.memory_budget();
                let args = inputs(
                    &artifact,
                    &[
                        (
                            "input",
                            Data::Matrix(input.iter().copied().map(Data::U8).collect()),
                        ),
                        ("capacity", Data::U64(capacity)),
                        ("rounds", Data::U64(rounds)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let mut output = vec![None; artifact.outputs().len()];
                println!(
                    "{}",
                    serde_json::json!({"phase":"repeated-source-start","artifact":form,"rounds":rounds,"capacity":capacity,"sample":sample,"mode":"NO_LIMITS"})
                );
                let started = std::time::Instant::now();
                let (status, allocations, requested) =
                    allocation_probe::measured_with_bytes(|| {
                        jit.evaluate_into(&args, &mut output, mode.execution(), &memory)
                    });
                let seconds = started.elapsed().as_secs_f64();
                let report = status.unwrap();
                let ValueData::Matrix(matrix) = output.last().unwrap().as_ref().unwrap().data()
                else {
                    panic!("matrix");
                };
                let SequenceView::U8(actual) = matrix.elements() else {
                    panic!("u8");
                };
                assert_eq!(
                    actual,
                    if rounds == 0 {
                        &[][..]
                    } else {
                        expected.as_slice()
                    }
                );
                assert_eq!(matrix.unsigned_buffer_capacity(), Some(capacity as usize));
                if rounds != 0 {
                    assert_eq!(
                        miniz_oxide::inflate::decompress_to_vec(actual).unwrap(),
                        input
                    );
                }
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(a, b)| a.shares_frozen_storage(b))
                );
                for ((port, value), alias) in artifact.inputs().iter().zip(&args).zip(&aliases) {
                    let name = mech_engine::decode_source_input_name(&port.name).unwrap();
                    let original = match name.as_str() {
                        "input" => Data::Matrix(input.iter().copied().map(Data::U8).collect()),
                        "capacity" => Data::U64(capacity),
                        "rounds" => Data::U64(rounds),
                        name => panic!("unexpected input {name}"),
                    };
                    assert_eq!(value.canonical_data_draft().unwrap(), original);
                    assert_eq!(alias.canonical_data_draft().unwrap(), original);
                }
                println!(
                    "{}",
                    serde_json::json!({"witness":"complete-compression-repeated-owned-output","artifact":form,"rounds":rounds,"capacity":capacity,"sample":sample,"seconds":seconds,"allocation_requests":allocations,"requested_bytes":requested,"visits":report.visits,"work":report.work,"exact_reference":true,"inputs_and_aliases_unchanged":true})
                );
                if sample > 0 {
                    measurements.push((allocations, requested));
                }
                drop((args, aliases, output));
                assert_eq!(memory.used_bytes(), 0);
                assert_eq!(memory.retained_snapshot_nodes(), 0);
            }
            let extra = measurements[1].1 - measurements[0].1;
            println!(
                "{}",
                serde_json::json!({"phase":"repeated-source-capacity-difference","artifact":form,"rounds":rounds,"extra_requested_bytes":extra,"one_initial_reservation":6633832-16,"allocation_requests_small":measurements[0].0,"allocation_requests_real":measurements[1].0})
            );
            assert_eq!(measurements[0].0, measurements[1].0);
            assert_eq!(
                extra,
                6633832 - 16,
                "the output payload may allocate once at initialization, with no payload allocations across clear/compress repetitions"
            );
        }
    }
}
