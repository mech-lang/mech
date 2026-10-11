use mech_gpu::sequence::compile;
#[test]
fn checked_collection_admission_rejects_unsupported_and_invalid_shapes() {
    for source in [
        "io/write(1)\n",
        "io/write(format/frequencies(map/histogram(sequence/packed-windows(bytes/encode(\"AB\", \"AB\", \"exact\"), 1, 1))))\n",
        "io/write(format/lookup(1, \"AB\"))\n",
        "f(x<f64>, x<f64>) = (y<f64>) := y := x.\nio/write(number/format(f(1.0, 2.0), 0))\n",
        "f(x<f64>) = (y<f64>) := y := f(x).\nio/write(number/format(f(1.0), 0))\n",
        "f(x<f64>) = (y<f64>) := y := x.\nio/write(number/format(f(1), 0))\n",
        "~x := 1\nio/write(\"mutable\")\n",
        "x := 1\nx = 2\nio/write(\"assignment\")\n",
        "x := unknown/function(1)\n",
        "x := bytes/encode(3, \"AB\", \"exact\")\n",
        "x := sequence/packed-windows(bytes/encode(\"AB\", \"AB\", \"exact\"), 1, 65)\n",
        "x := sequence/packed-windows(bytes/encode(\"AB\", \"AB\", \"exact\"), 2, 33)\n",
        "x := sequence/packed-windows(bytes/encode(\"AB\", \"AB\", \"exact\"), 0, 2)\n",
        "io/write(undefined)\n",
        "x := 1\nx := 2\nio/write(\"duplicate\")\n",
        "x := [1 2]\nio/write(\"unsupported matrix\")\n",
    ] {
        assert!(compile(source, 256).is_err(), "{source}");
    }
    assert!(compile("io/write(\"ok\")\n", 0).is_err());
    assert!(compile("io/write(\"ok\")\n", 8193).is_err());
}
#[test]
fn generic_source_and_full_width_key_emit_valid_objects() {
    let source = "b := bytes/encode(\"ABBA\", \"AB\", \"exact\")\nh := map/histogram(sequence/packed-windows(b, 1, 64))\nio/write(number/format(stats/sum(map/values(h)), 0))\n";
    let program = compile(source, 256).unwrap();
    assert!(!program.object(256).unwrap().is_empty());
    assert!(program.object(0).is_err());
    assert!(program.object(8193).is_err());
}

#[test]
fn compact_and_expanded_graphs_emit_identical_scheduled_objects() {
    let compact = compile(include_str!("../../knucleotide-compact.mec"), 256)
        .unwrap()
        .scheduled()
        .unwrap();
    let expanded = compile(include_str!("../../knucleotide.mec"), 256)
        .unwrap()
        .scheduled()
        .unwrap();
    assert_eq!(compact.constants, expanded.constants);
    for fuse in [false, true] {
        let compact = if fuse {
            compact.clone().fuse_record_encoding()
        } else {
            compact.clone()
        };
        let expanded = if fuse {
            expanded.clone().fuse_record_encoding()
        } else {
            expanded.clone()
        };
        if fuse {
            assert!(compact.clone().scheduled().is_err());
        }
        for pack8 in [false, true] {
            assert_eq!(
                compact.object_with_packing(256, pack8, pack8).unwrap(),
                expanded.object_with_packing(256, pack8, pack8).unwrap()
            );
        }
    }
}

#[test]
fn source_functions_lower_scalar_and_vector_arithmetic_natively() {
    let source = "reciprocal(x<f64>) = (y<f64>) := y := 1.0 / x.\nio/write(number/format(reciprocal(number/to-f64(sequence/length(sequence/fill(3, 1.0)))), 2))\nscale(x<f64>) = (y<f64>) := y := (x + 2.0) * 3.0 / 2.0 - 1.0.\nio/write(number/format(scale(4.0), 2))\nb := bytes/encode(\"ABBA\", \"AB\", \"exact\")\nh := map/histogram(sequence/packed-windows(b, 1, 1))\nio/write(text/join(number/format(number/to-f64(map/values(h)) * 2.0, 0), \",\"))\n";
    let program = compile(source, 256).unwrap();
    for op in 20..=23 {
        assert!(program.nodes.iter().any(|n| n.op == op));
    }
    assert!(!program.object(256).unwrap().is_empty());
}

#[test]
fn higher_order_arrays_require_closed_pure_typed_functions() {
    for source in [
        "f(i<f64>) = (x<f64>) := x := i.\nx := matrix/apply(sequence/fill(3, 1.0), 3, f, \"multiply\")\n",
        "f(i<f64>, j<f64>) = (x<u64>) := x := 1.\nx := matrix/apply(sequence/fill(3, 1.0), 3, f, \"multiply\")\n",
        "f(i<u64>, j<f64>) = (x<f64>) := x := j.\nx := matrix/apply(sequence/fill(3, 1.0), 3, f, \"multiply\")\n",
        "global := 1.0\nf(i<f64>, j<f64>) = (x<f64>) := x := i + global.\nx := matrix/apply(sequence/fill(3, 1.0), 3, f, \"multiply\")\n",
        "f(v<[f64]:_>) = (x<[f64]:_>) := a := io/read() x := v.\nx := sequence/iterate(sequence/fill(3, 1.0), 2, f)\n",
        "f(v<[f64]:_>) = (x<[f64]:_>) := x := sequence/iterate(v, 1, f).\nx := sequence/iterate(sequence/fill(3, 1.0), 2, f)\n",
        "f(x<f64>) = (y<f64>) := y := x.\nx := sequence/iterate(sequence/fill(3, 1.0), 2, f)\n",
        "x := matrix/apply(sequence/fill(3, 1.0), 3, missing, \"multiply\")\n",
    ] {
        assert!(compile(source, 256).is_err(), "{source}");
    }
    let source = "coefficient(i<f64>, j<f64>) = (x<f64>) := x := (i + 1.0) / (j + 2.0).\nstep(v<[f64]:_>) = (out<[f64]:_>) := out := matrix/apply(v, 7, coefficient, \"divide\").\nx := sequence/iterate(sequence/fill(3, 1.0), 2, step)\nio/write(text/join(number/format(x, 9), \",\"))\n";
    let graph = compile(source, 256).unwrap().scheduled().unwrap();
    for simd in [false, true] {
        assert!(
            !graph
                .object_with_array_simd(256, true, false, simd)
                .unwrap()
                .is_empty()
        );
    }
    let unsupported_mode = source.replace("\"divide\"", "\"unrecognized\"");
    assert!(
        compile(&unsupported_mode, 256)
            .unwrap()
            .object(256)
            .is_err()
    );
    let integer = "coefficient(i<u64>, j<u64>) = (x<f64>) := x := number/to-f64((i + 1) * (j + 2) / 3).\nx := matrix/apply(sequence/fill(3, 1.0), 7, coefficient, \"multiply\")\n";
    let graph = compile(integer, 256).unwrap();
    for simd in [false, true] {
        assert!(
            !graph
                .object_with_array_simd(256, true, false, simd)
                .unwrap()
                .is_empty()
        );
    }
}

#[test]
fn binary_divisors_use_exact_reciprocals_at_normal_and_subnormal_boundaries() {
    use mech_gpu::sequence::Constant;
    for exponent in [-1023, -1022, -100, -1, 0, 1, 100, 1023] {
        let divisor = 2.0f64.powi(exponent);
        let source = format!("io/write(number/format(1.0 / {divisor:e}, 9))\n");
        let graph = compile(&source, 256).unwrap();
        let arithmetic = graph.nodes.iter().find(|n| n.op == 20).unwrap();
        assert_eq!(
            graph.constants[arithmetic.args[1]],
            Constant::Float((1.0 / divisor).to_bits())
        );
        for bits in [
            0,
            1,
            2,
            0x000f_ffff_ffff_ffff,
            0x0010_0000_0000_0000,
            0x3ff0_0000_0000_0000,
            0x7fef_ffff_ffff_ffff,
            0x7ff0_0000_0000_0000,
        ] {
            for sign in [0, 1u64 << 63] {
                let value = f64::from_bits(bits | sign);
                assert_eq!(
                    (value / divisor).to_bits(),
                    (value * (1.0 / divisor)).to_bits()
                );
            }
        }
    }
    for divisor in [0.0, 3.0, 1e-300, f64::from_bits(1)] {
        let source = format!("io/write(number/format(1.0 / {divisor:e}, 9))\n");
        assert!(
            compile(&source, 256)
                .unwrap()
                .nodes
                .iter()
                .any(|n| n.op == 21)
        );
    }
}
