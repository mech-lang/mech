//! Representation optimization must preserve the actual Unicode segmenter.
use super::{artifacts, equal, inputs, limits, reference};
use mech_core::{ManagedMemoryBudget, ValueData, ValueDataDraft as Data};
use mech_gpu::native_artifact::JitProgramArtifact;

fn oracle(text: &str) -> usize {
    grapheme::Graphemes::from_usvs(text).iter().count()
}

#[test]
fn ascii_length_matches_segmenter_for_every_byte_pair_and_scan_boundary() {
    let count = mech_engine::intrinsics::text_graphemes::count;
    assert_eq!(count(""), 0);
    for a in 0..=127u8 {
        for b in 0..=127u8 {
            let bytes = [a, b];
            let text = core::str::from_utf8(&bytes).unwrap();
            assert_eq!(count(text), oracle(text), "{bytes:?}");
        }
    }
    for width in [1, 2, 15, 16, 17, 31, 32, 33, 63, 64, 65, 1023, 1024] {
        for suffix in ["\r\n", "\r\r\n", "\n\r", "\0\t\x7f", "a\r\nb"] {
            let text = format!("{}{suffix}{}", "x".repeat(width), "y".repeat(width));
            assert_eq!(count(&text), oracle(&text), "width {width}, {suffix:?}");
        }
    }
}

#[test]
fn text_length_and_generator_count_agree_for_ascii_crlf_and_unicode() {
    let source = "+> sequence\ncount := sequence/length(input<string>)\nparts := [c | c <- input]\n(count, sequence/length(parts))\n";
    let (variants, last) = artifacts(source);
    for artifact in variants {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        for text in [
            "",
            "abc",
            "\r\n",
            "\r\r\n\n",
            "\0\t\x7f",
            "e\u{301}",
            "🧑‍🚀🇺🇸",
            "a\r\né\r\nz",
        ] {
            assert_eq!(
                mech_engine::intrinsics::text_graphemes::count(text),
                oracle(text)
            );
            let args = inputs(&artifact, &[("input", Data::String(text.into()))], &memory);
            let ordinary = reference(&artifact, &args);
            let mut output = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &ordinary);
            let ValueData::Tuple(pair) = output[last].as_ref().unwrap().data() else {
                panic!("both length observations must be published in the result tuple");
            };
            let lengths = pair
                .iter()
                .map(|value| match value {
                    ValueData::U64(n) => *n,
                    _ => panic!("expected a U64 length"),
                })
                .collect::<Vec<_>>();
            assert_eq!(lengths.len(), 2);
            assert!(
                lengths.iter().all(|n| *n == oracle(text) as u64),
                "{text:?}"
            );
        }
        assert_eq!(memory.used_bytes(), 0);
    }
}
