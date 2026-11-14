//! Compare every table entry and cache word with the unmodified pinned C walks.
use super::super::{inputs, limits};
use super::{KERNEL, artifacts};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::JitProgramArtifact;

#[derive(serde::Deserialize)]
struct Reference {
    cases: Vec<Case>,
}
#[derive(serde::Deserialize)]
struct Case {
    name: String,
    input: Vec<u8>,
    base: u64,
    position: u64,
    maximum: u64,
    nice: u64,
    depth: u64,
    skip: bool,
    hashes: [u64; 2],
    next_hashes: [u64; 2],
    before: Vec<(usize, u16)>,
    after: Vec<(usize, u16)>,
    matches: Vec<u32>,
}
const TABLE_SIZE: usize = 262144;
const CACHE_SIZE: usize = 512;
const CACHE_INITIAL: u32 = 0xA5A5A5A5;
const CACHE_POINTER: usize = 3;

fn table(entries: &[(usize, u16)]) -> Data {
    let mut words = vec![Data::U16(0); TABLE_SIZE];
    let mut previous = None;
    for &(index, value) in entries {
        assert!(index < TABLE_SIZE && value != 0);
        assert!(previous.is_none_or(|prior| prior < index));
        words[index] = Data::U16(value);
        previous = Some(index);
    }
    Data::Matrix(words.into())
}

#[test]
fn binary_tree_search_and_skip_match_all_pinned_c_table_and_cache_transitions() {
    let reference: Reference = serde_json::from_str(include_str!(
        "../fixtures/native/deflate-binary-tree-reference.json"
    ))
    .unwrap();
    assert_eq!(reference.cases.len(), 20);
    for skip in [false, true] {
        let call = if skip {
            "bt-skip-byte(tables<[u16]>, hash3<u64>, hash4<u64>, input<[u8]>, base<u64>, position<u64>, nice<u64>, depth<u64>)"
        } else {
            "bt-get-matches(tables<[u16]>, hash3<u64>, hash4<u64>, input<[u8]>, base<u64>, position<u64>, maximum<u64>, nice<u64>, depth<u64>, cache<[u32]>, pointer<u64>)"
        };
        for artifact in artifacts(&format!("{KERNEL}\n{call}\n")).0 {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut output = vec![None; artifact.outputs().len()];
            // Keep the capacity test's extent/execution/compute allowances,
            // with an explicit finite comparison allowance for current buffer
            // copying. This is correctness qualification; host defaults do not change.
            let mut policy = limits();
            policy.library.max_output_elements = Some(TABLE_SIZE as u64);
            policy.execution.max_work = 10_000_000_000;
            policy.library.max_compute_work = Some(10_000_000_000);
            policy.library.max_comparison_work = Some(10_000_000_000);
            for case in reference.cases.iter().filter(|case| case.skip == skip) {
                let before = table(&case.before);
                let cache = Data::Matrix(vec![Data::U32(CACHE_INITIAL); CACHE_SIZE].into());
                let args = inputs(
                    &artifact,
                    &[
                        ("tables", before.clone()),
                        ("hash3", Data::U64(case.hashes[0])),
                        ("hash4", Data::U64(case.hashes[1])),
                        (
                            "input",
                            Data::Matrix(case.input.iter().copied().map(Data::U8).collect()),
                        ),
                        ("base", Data::U64(case.base)),
                        ("position", Data::U64(case.position)),
                        ("maximum", Data::U64(case.maximum)),
                        ("nice", Data::U64(case.nice)),
                        ("depth", Data::U64(case.depth)),
                        ("cache", cache.clone()),
                        ("pointer", Data::U64(CACHE_POINTER as u64)),
                    ],
                    &memory,
                );
                jit.evaluate_into(&args, &mut output, policy, &memory)
                    .unwrap_or_else(|error| panic!("{}: {error:?}", case.name));
                let result = output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap();
                let Data::Record(fields) = result else {
                    panic!("{}: expected record", case.name)
                };
                let field = |name: &str| {
                    &fields
                        .iter()
                        .find(|field| field.name == name)
                        .unwrap()
                        .value
                };
                assert_eq!(
                    field("tables"),
                    &table(&case.after),
                    "{} complete table",
                    case.name
                );
                assert_eq!(
                    field("next-hash3"),
                    &Data::U64(case.next_hashes[0]),
                    "{} hash3",
                    case.name
                );
                assert_eq!(
                    field("next-hash4"),
                    &Data::U64(case.next_hashes[1]),
                    "{} hash4",
                    case.name
                );
                if !skip {
                    let mut expected = vec![Data::U32(CACHE_INITIAL); CACHE_SIZE];
                    for (index, &word) in case.matches.iter().enumerate() {
                        expected[CACHE_POINTER + index] = Data::U32(word);
                    }
                    assert_eq!(
                        field("cache"),
                        &Data::Matrix(expected.into()),
                        "{} complete cache",
                        case.name
                    );
                    assert_eq!(
                        field("cache-ptr"),
                        &Data::U64((CACHE_POINTER + case.matches.len()) as u64),
                        "{} pointer",
                        case.name
                    );
                }
                for (name, expected) in [("tables", &before), ("cache", &cache)] {
                    if let Some(index) = artifact.inputs().iter().position(|input| {
                        mech_engine::decode_source_input_name(&input.name).unwrap() == name
                    }) {
                        assert_eq!(
                            &args[index].canonical_data_draft().unwrap(),
                            expected,
                            "{} input {name}",
                            case.name
                        );
                    }
                }
                eprintln!(
                    "DEFLATE_BT case={} tables={TABLE_SIZE} matches={}",
                    case.name,
                    case.matches.len()
                );
            }
            drop(output);
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}
