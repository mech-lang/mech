#![cfg(all(feature = "full_compiler", not(feature = "no_std")))]
use mech_core::ValueData;
mod support;
use support::evaluate_with_source_catalog as evaluate;

#[test]
fn source_map_insert_preserves_input_and_updates_one_entry() {
    let output = evaluate("+> map\na := {\"a\": 1u64}\nb := map/insert(a, \"b\", 2u64)\nc := map/insert(b, \"a\", 3u64)\n(a, b, c)\n").unwrap();
    let ValueData::Tuple(values) = output.data() else {
        panic!("expected tuple")
    };
    for (value, expected) in values.iter().zip([
        vec![("a", 1)],
        vec![("a", 1), ("b", 2)],
        vec![("a", 3), ("b", 2)],
    ]) {
        let ValueData::Map(map) = value else {
            panic!("expected map")
        };
        assert_eq!(map.entries().len(), expected.len());
        for (entry, (key, value)) in map.entries().iter().zip(expected) {
            assert!(
                matches!(entry.key().data(), ValueData::String(actual) if actual.as_ref() == key)
            );
            assert!(matches!(entry.value(), ValueData::U64(actual) if *actual == value));
        }
    }
}

#[test]
fn source_map_insert_supports_other_key_and_value_kinds() {
    let output = evaluate("+> map\na<{u64:string}> := map/new()\nb := map/insert(a, 7u64, \"α\")\nc := map/insert(b, 7u64, \"β\")\nmap/insert(c, 9u64, \"γ\")\n").unwrap();
    let ValueData::Map(map) = output.data() else {
        panic!("expected map")
    };
    assert_eq!(map.entries().len(), 2);
    for (entry, (key, value)) in map.entries().iter().zip([(7, "β"), (9, "γ")]) {
        assert!(matches!(entry.key().data(), ValueData::U64(actual) if *actual == key));
        assert!(matches!(entry.value(), ValueData::String(actual) if actual.as_ref() == value));
    }
}

const COUNT: &str = r#"
+> map
increment(h<{string:u64}>, key<string>) => (out<{string:u64}>) :=
  keys := {k | k <- h_keys}
  previous := (key ∈ keys)? | true => h{key} | false => 0u64.
  out := map/insert(h, key, previous + 1u64).
#Count(s<string>, width<u64>, n<u64>, initial<{string:u64}>) => <{string:u64}>
  | :Next(index<u64>, counts<{string:u64}>)
  | :Done(counts<{string:u64}>).
#Count(s, width, n, initial) -> :Next(0u64, initial)
  :Next(index, counts)
    | width == 0u64 -> :Done(counts)
    | index >= n -> :Done(counts)
    | width > n - index -> :Done(counts)
    | * -> :Next(index + 1u64, increment(counts, s[(index + 1u64)..=(index + width)]))
  :Done(counts) => counts.
empty<{string:u64}> := map/new()
"#;

#[test]
fn source_state_machine_counts_overlapping_windows_with_ordinary_map_entries() {
    let source = format!("{COUNT}\n#Count(\"GGTAGGT\", 3u64, 7u64, empty)\n");
    let output = evaluate(&source).unwrap_or_else(|error| panic!("{source}: {error:?}"));
    let ValueData::Map(map) = output.data() else {
        panic!("expected map")
    };
    assert_eq!(map.entries().len(), 4);
    for (entry, (key, count)) in
        map.entries()
            .iter()
            .zip([("AGG", 1), ("GGT", 2), ("GTA", 1), ("TAG", 1)])
    {
        assert!(matches!(entry.key().data(), ValueData::String(actual) if actual.as_ref() == key));
        assert!(matches!(entry.value(), ValueData::U64(actual) if *actual == count));
    }
}

#[test]
fn source_state_machine_window_guards_handle_empty_and_oversize_widths() {
    for (sequence, width, n) in [("", 1, 0), ("ACG", 0, 3), ("ACG", 4, 3)] {
        let source = format!("{COUNT}\n#Count(\"{sequence}\", {width}u64, {n}u64, empty)\n");
        let output = evaluate(&source).unwrap_or_else(|error| panic!("{source}: {error:?}"));
        assert!(matches!(output.data(), ValueData::Map(map) if map.entries().is_empty()));
    }
}

#[test]
fn source_map_insert_rejects_mismatched_entry_kinds_and_arity() {
    for source in [
        "+> map\nmap/insert({\"a\": 1u64}, true, 2u64)\n",
        "+> map\nmap/insert({\"a\": 1u64}, \"b\", \"wrong\")\n",
        "+> map\nmap/insert({\"a\": 1u64}, \"b\")\n",
    ] {
        assert!(
            evaluate(source).is_err(),
            "accepted invalid map entry: {source}"
        );
    }
}

#[test]
fn source_map_insert_keeps_matrix_values_and_the_input_map() {
    let output =
        evaluate("+> map\na := {\"a\": [1.0 2.0]}\nb := map/insert(a, \"b\", [3.0 4.0])\n(a, b)\n")
            .unwrap();
    let ValueData::Tuple(values) = output.data() else {
        panic!("expected tuple")
    };
    for (value, expected_len) in values.iter().zip([1, 2]) {
        let ValueData::Map(map) = value else {
            panic!("expected map")
        };
        assert_eq!(map.entries().len(), expected_len);
        for (entry, expected) in map.entries().iter().zip([[1.0, 2.0], [3.0, 4.0]]) {
            let ValueData::Matrix(matrix) = entry.value() else {
                panic!("expected matrix value")
            };
            let mech_core::snapshot::SequenceView::F64(elements) = matrix.elements() else {
                panic!("expected f64")
            };
            assert_eq!(
                elements
                    .iter()
                    .map(|value| value.to_f64())
                    .collect::<Vec<_>>(),
                expected
            );
        }
    }
}
