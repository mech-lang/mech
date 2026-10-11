#[cfg(test)]
mod tests {
    use mech_engine::native_sequence::control::{compile, analysis::Effect};
    #[test]
    fn typed_arbitrary_precision_resources_and_mutation_are_inferred() {
        let catalog = mech_stdlib::native_control_catalog();
        let p = compile("Scale(x<bigint>) = (y<bigint>) := y := bigint/multiply-u64-into(x, x, 3).\nx := bigint/new(7)\ny := Scale(x)\nio/print(\"{}\\n\", bigint/floor-u64(y, x))\n", catalog).unwrap();
        let s = &p.computation.functions["Scale"];
        assert!(s.coeffects.big_integers);
        assert!(s.effects.contains(&Effect::Mutation));
        assert!(!s.independent_worker());
        let worker = "Chunk(start<u64>, end<u64>, n<u64>) = (r<u64>) :=\n  x := bigint/new(n)\n  y := bigint/multiply-u64-into(x, x, 2)\n  r := bigint/floor-u64(y, x).\nr := parallel/reduce-ranges(0, 4, 1, Chunk, 7, \"sum\")\nio/print(\"{}\\n\", r)\n";
        assert!(compile(worker,catalog).unwrap_err().contains("unavailable effects/resources"));
        assert!(compile("x := bigint/new(1)\ny := memory/read(x, <u64>)\n",catalog).is_err());
        let value = compile("ToU64(x<bigint>) => <u64> := x\nx := bigint/new(1)\ny := x + 1\nio/print(\"{}\\n\", ToU64(y))\n", catalog).unwrap();
        assert!(value.computation.entry.big_integer_values);
    }
    #[test]
    fn independent_regex_nodes_are_scheduled_before_scalar_consumers() {
        let source = "raw := io/read()\na := regex/count-bytes(raw, \"foo+\")\nio/write(number/format(a, 0))\nb := regex/count-bytes(raw, \"bar[0-9]+\")\nio/write(number/format(b, 0))\nx := regex/replace-bytes(raw, \"<[^>]*>\", \"-\")\nio/write(x)\n";
        let p = mech_gpu::sequence::compile(source,256).unwrap().scheduled().unwrap();
        let counts = p.nodes.iter().enumerate().filter(|(_, n)| n.op==53).map(|(i,_)| i).collect::<Vec<_>>();
        let first_consumer = p.nodes.iter().position(|n| n.op==24).unwrap();
        assert_eq!(counts.len(),2);
        assert!(counts.iter().all(|&i| i<first_consumer));
        let replacement = p.nodes.iter().position(|n| n.op==54).unwrap();
        assert!(replacement<first_consumer);
        assert!(!p.object(256).unwrap().is_empty());
    }
}
