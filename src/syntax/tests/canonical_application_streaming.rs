//! The ordinary application continuation owns all chunk/limit boundaries.
use mech_syntax::document::*;
#[path = "support/document_stream.rs"]
mod support;
const FIXTURES: &[&str] = &[
    "seed |> apply |> transpose\n",
    "seed |0> apply\n",
    "seed |19> apply\n",
    "seed |18446744073709551615> apply\n",
    "seed |18446744073709551616> apply\n",
    "seed |19>\n",
    "seed |>\n",
    "(flag ? | true => [x | x <- [1u64 2u64]] | false => [3u64]) |> apply\n",
    "[x + 1u64 | x <- [1u64 2u64]] |25> apply\n",
    "step(x<[f64]>) => <[f64]> := x |> apply |> transpose\n",
    "text := \"é👩‍👧é\"\ntext |> apply\n",
];
fn parse(text: &str, config: ParseConfig) -> SyntaxSnapshot {
    parse_canonical_document(
        TextSnapshot::new(DocumentId(0x41505053), Revision(1), text).unwrap(),
        config,
    )
}
#[test]
fn application_streaming_matches_batch_at_every_utf8_cut() {
    for &text in FIXTURES {
        let expected = parse(text, ParseConfig::default());
        validate_lossless(&expected.root, &expected.source).unwrap();
        for allowance in [1, 17] {
            for cut in text
                .char_indices()
                .map(|(at, _)| at)
                .chain(core::iter::once(text.len()))
            {
                let mut stream =
                    DocumentStream::new(DocumentId(0x41505053), ParseConfig::default());
                support::append(&mut stream, &text[..cut], allowance);
                support::append(&mut stream, &text[cut..], allowance);
                let actual = support::finish(&mut stream, allowance);
                if cut == 0 && allowance == 1 {
                    support::equivalent(&actual, text);
                } else {
                    support::equivalent_to(&actual, text, &expected);
                }
                assert_eq!(actual.is_strictly_clean(), expected.is_strictly_clean());
            }
        }
    }
}
#[test]
fn application_fuel_and_nesting_limits_stay_finite_and_lossless() {
    for &text in FIXTURES {
        let configs = (0..=32)
            .map(|fuel| ParseConfig {
                limits: ParseLimits {
                    fuel,
                    ..ParseLimits::default()
                },
            })
            .chain((0..=8).map(|max_nesting| ParseConfig {
                limits: ParseLimits {
                    max_nesting,
                    ..ParseLimits::default()
                },
            }));
        for config in configs {
            let batch = parse(text, config);
            validate_lossless(&batch.root, &batch.source).unwrap();
            assert_eq!(
                reconstruct_source(&batch.root, &batch.source).unwrap(),
                text
            );
            assert!(batch.stats.parser_steps <= config.limits.fuel);
            for allowance in [1, 17] {
                let limits = StreamLimits {
                    max_parser_work: config.limits.fuel,
                    ..StreamLimits::default()
                };
                let mut stream =
                    DocumentStream::with_limits(DocumentId(0x41505053), config, limits);
                let mut update = stream.append(text, allowance).unwrap();
                let mut polls = 0;
                loop {
                    assert!(polls < 100_000);
                    assert!(update.work.parser_work <= limits.max_parser_work);
                    match update.progress {
                        StreamProgress::NeedsProcessing => update = stream.advance(allowance),
                        StreamProgress::NeedInput => update = stream.finish(allowance),
                        StreamProgress::Finished | StreamProgress::Limited => break,
                        StreamProgress::Cancelled => {
                            panic!("uncancelled application parse cancelled")
                        }
                    }
                    polls += 1;
                }
                let actual = stream.materialize().unwrap();
                validate_lossless(&actual.root, &actual.source).unwrap();
                assert_eq!(
                    reconstruct_source(&actual.root, &actual.source).unwrap(),
                    text
                );
                assert!(actual.stats.parser_steps <= config.limits.fuel);
                assert!(actual.stats.events_emitted <= u64::from(config.limits.max_events));
            }
        }
    }
}
