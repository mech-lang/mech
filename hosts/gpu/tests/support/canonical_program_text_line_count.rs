use super::*;

#[test]
fn line_count_preserves_native_and_decoded_shapes_contents_and_publication() {
    let source = "+> text\ntext/lines(input<string>)\n";
    let mut cases = [
        "",
        "\n",
        "\n\n",
        "\r",
        "\r\n",
        "\r\r\n",
        "a\nb",
        "a\nb\n",
        "a\r\n\r\nb\r",
        "\0\n\0",
        "é\n🧑‍🚀\r\n終",
        "\u{0085}\u{2028}\u{2029}",
    ]
    .map(str::to_owned)
    .to_vec();
    for width in [1, 7, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129] {
        for ending in ["", "\n", "\r", "\r\n"] {
            cases.push(format!(
                "{}\n\n{}{}",
                "x".repeat(width),
                "é\0".repeat(width),
                ending
            ));
        }
    }
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(16 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for text in &cases {
            let args = inputs(&artifact, &[("input", Data::String(text.clone()))], &memory);
            let ordinary = reference(&artifact, &args);
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &ordinary);
            let [Some(result)] = output.as_slice() else {
                panic!("expected one canonical String matrix");
            };
            let mech_core::ValueData::Matrix(matrix) = result.data() else {
                panic!("expected canonical Matrix");
            };
            let SequenceView::String(lines) = matrix.elements() else {
                panic!("expected canonical String owners");
            };
            assert_eq!(
                lines.iter().map(AsRef::as_ref).collect::<Vec<&str>>(),
                text.lines().collect::<Vec<_>>(),
                "{text:?}"
            );
            // The first run may replace a differently shaped prior output.
            // Establish the identical published target for both the denied
            // attempt and recovery, including its comparison-work charges.
            let report = jit
                .evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &ordinary);
            let prior = output
                .iter()
                .map(|value| value.clone().unwrap())
                .collect::<Vec<_>>();
            let retained = memory.used_bytes();
            let mut denied = limits();
            denied.execution.max_work = report.work - 1;
            assert!(
                jit.evaluate_into(&args, &mut output, denied, &memory)
                    .is_err()
            );
            equal(&artifact, &output, &prior);
            assert_eq!(memory.used_bytes(), retained);
            let recovered = jit
                .evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            assert_eq!(recovered.work, report.work);
            equal(&artifact, &output, &ordinary);
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}
