//! General resource/effect inference and observable runtime failure ordering.
#![cfg(feature = "sequence-cli")]

#[cfg(test)]
mod tests {
    use mech_engine::native_sequence::control::{analysis::Effect, compile};

    fn execute(
        source: &str,
        input: Option<&[u8]>,
        scheduled: bool,
    ) -> (Result<(), String>, Vec<u8>) {
        let program = mech_gpu::sequence::compile(source, 256).unwrap();
        let program = if scheduled {
            program.scheduled().unwrap()
        } else {
            program
        };
        let jit = mech_gpu::sequence::JitProgram::compile(&program, 256, false).unwrap();
        let mut context =
            mech_gpu::sequence::Context::new(&program.constants, input.map(<[u8]>::to_vec), false);
        let result = jit.run(&mut context);
        (result, context.output)
    }

    #[test]
    fn byte_encoding_preserves_prior_writes_and_failure_atomicity() {
        for (alphabet, mode, input, expected_error) in [
            ("", "exact", "A", Some("invalid byte alphabet")),
            ("A", "unknown", "A", Some("unknown byte encoding mode")),
            ("AA", "exact", "A", Some("duplicate alphabet symbol")),
            (
                "Aa",
                "ignore-ascii-case",
                "A",
                Some("duplicate alphabet symbol"),
            ),
            ("A", "exact", "B", Some("byte 66 is not in the alphabet")),
            ("A", "exact", "A", None),
        ] {
            let source = format!(
                "raw := io/read()\nio/write(\"before\")\nencoded := bytes/encode(raw, {alphabet:?}, {mode:?})\nio/write(\"after\")\n"
            );
            for scheduled in [false, true] {
                let (result, output) = execute(&source, Some(input.as_bytes()), scheduled);
                match expected_error {
                    Some(error) => {
                        assert_eq!(result, Err(error.into()), "{source}");
                        assert_eq!(output, b"before", "{source}");
                    }
                    None => {
                        assert_eq!(result, Ok(()), "{source}");
                        assert_eq!(output, b"beforeafter", "{source}");
                    }
                }
            }
        }
    }

    #[test]
    fn scheduled_runtime_failures_keep_source_order_across_operation_families() {
        for (source, expected_error, expected_output) in [
            (
                "io/write(\"before\")\nvalue := text/record(\"ABC\", \"\", \">\")\nio/write(\"after\")\n",
                "record prefixes cannot be empty",
                "before",
            ),
            (
                "encoded := bytes/encode(\"ABC\", \"ABC\", \"exact\")\nio/write(\"before\")\nwindows := sequence/packed-windows(encoded, 1, 1)\nio/write(\"after\")\n",
                "encoded symbol exceeds window bit width",
                "before",
            ),
            (
                "io/write(\"before\")\nvalue := number/format(1, 13)\ncount := regex/count-bytes(\"ABC\", \"[\")\nio/write(\"after\")\n",
                "decimal precision exceeds 12",
                "before",
            ),
            (
                "io/write(\"before\")\nencoded := bytes/encode(\"A\", \"\", \"exact\")\ncount := regex/count-bytes(\"ABC\", \"[\")\nio/write(\"after\")\n",
                "invalid byte alphabet",
                "before",
            ),
        ] {
            for scheduled in [false, true] {
                let (result, output) = execute(source, None, scheduled);
                assert_eq!(result, Err(expected_error.into()), "{source}");
                assert_eq!(output, expected_output.as_bytes(), "{source}");
            }
        }
    }

    #[test]
    fn typed_arbitrary_precision_resources_and_mutation_are_inferred() {
        let catalog = mech_stdlib::native_control_catalog();
        let p = compile("Scale(x<bigint>) = (y<bigint>) := y := bigint/multiply-u64-into(x, x, 3).\nx := bigint/new(7)\ny := Scale(x)\nio/print(\"{}\\n\", bigint/floor-u64(y, x))\n", catalog).unwrap();
        let s = &p.computation.functions["Scale"];
        assert!(s.coeffects.big_integers);
        assert!(s.effects.contains(&Effect::Mutation));
        assert!(!s.independent_worker());
        let worker = "Chunk(start<u64>, end<u64>, n<u64>) = (r<u64>) :=\n  x := bigint/new(n)\n  y := bigint/multiply-u64-into(x, x, 2)\n  r := bigint/floor-u64(y, x).\nr := parallel/reduce-ranges(0, 4, 1, Chunk, 7, \"sum\")\nio/print(\"{}\\n\", r)\n";
        assert!(
            compile(worker, catalog)
                .unwrap_err()
                .contains("unavailable effects/resources")
        );
        assert!(compile("x := bigint/new(1)\ny := memory/read(x, <u64>)\n", catalog).is_err());
        let value = compile("ToU64(x<bigint>) => <u64> := x\nx := bigint/new(1)\ny := x + 1\nio/print(\"{}\\n\", ToU64(y))\n", catalog).unwrap();
        assert!(value.computation.entry.big_integer_values);
    }
    #[test]
    fn regex_nodes_preserve_source_order_with_io() {
        let source = "raw := io/read()\na := regex/count-bytes(raw, \"foo+\")\nio/write(number/format(a, 0))\nb := regex/count-bytes(raw, \"bar[0-9]+\")\nio/write(number/format(b, 0))\nx := regex/replace-bytes(raw, \"<[^>]*>\", \"-\")\nio/write(x)\n";
        let p = mech_gpu::sequence::compile(source, 256)
            .unwrap()
            .scheduled()
            .unwrap();
        let counts = p
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.op == 53)
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        assert_eq!(counts.len(), 2);
        // Pattern compilation can fail at execution time. A later regex must
        // not move ahead of an earlier write, even with independent inputs.
        let observable = p
            .nodes
            .iter()
            .filter(|node| matches!(node.op, 8 | 53 | 54))
            .map(|node| node.op)
            .collect::<Vec<_>>();
        assert_eq!(observable, [53, 8, 53, 8, 54, 8]);
        let consumers = p
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.op == 24)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        assert_eq!(consumers.len(), 2);
        assert!(
            counts
                .iter()
                .zip(consumers)
                .all(|(&count, consumer)| count < consumer)
        );
        assert!(!p.object(256).unwrap().is_empty());
    }
}
