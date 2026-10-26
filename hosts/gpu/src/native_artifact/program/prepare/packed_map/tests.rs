use crate::native_artifact::program::prepare::packed_map::*;
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

const COUNT: &str = r#"
+> map
increment(h<{string:u64}>, key<string>) => (out<{string:u64}>) :=
  previous := (key ∈ h)? | true => h{key} | false => 0u64.
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

fn each_plan(
    source: &str,
    mut check: impl FnMut(&Plan, &NativeMapOwnerWords, &[NativeIterationVisit]),
) {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x5041434b), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let catalog = mech_stdlib::source_catalog();
    let original = mech_engine::CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            catalog.clone(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    for artifact in [original, decoded] {
        let plan = mech_engine::native_artifact::lower_native_program_artifact(&artifact, &catalog)
            .unwrap();
        let owners = plan.map_owner_words().unwrap();
        let visits = crate::native_artifact::iteration_abi::control_visits(
            plan.blocks(),
            plan.iterations().iter().map(Into::into),
            0,
        )
        .unwrap();
        check(&plan, &owners, &visits);
    }
}

#[test]
fn packed_width_requires_same_base_and_invariant_offsets() {
    for (start, end, expected) in [
        ("(index + 1u64)", "(index + width)", true),
        ("(1u64 + index)", "(width + index)", true),
        ("index", "(index + width)", true),
        ("(index + 1u64)", "(index + index + 1u64)", false),
        ("(index + 1u64)", "(index + (width + index))", false),
        ("(index + 1u64)", "(width + width)", false),
    ] {
        let source = format!(
            "{}\n#Count(text<string>, width<u64>, n<u64>, empty)\n",
            COUNT.replace(
                "s[(index + 1u64)..=(index + width)]",
                &format!("s[{start}..={end}]")
            )
        );
        each_plan(&source, |plan, owners, visits| {
            assert_eq!(owners.regions().len(), 1);
            let proof = derive(plan, owners, 0, visits);
            assert_eq!(proof.is_some(), expected, "{start}..={end}");
            if let Some(proof) = proof {
                assert!(proof.inclusive());
                assert_eq!(proof.range_site(), owners.spans()[0].range_site());
                assert_eq!(proof.selection_site(), owners.spans()[0].selection_site());
                if start == "index" {
                    assert!(proof.start_offset().is_none());
                }
                assert!(proof.end_offset().is_some());
                let image = serde_json::to_vec(&proof).unwrap();
                let restored: Proof = serde_json::from_slice(&image).unwrap();
                assert_eq!(serde_json::to_vec(&restored).unwrap(), image);
            }
        });
    }
}

#[test]
fn packed_width_accepts_current_ancestor_element_and_excludes_child_scope() {
    let source = format!(
        r#"{COUNT}
+> number
+> text
+> sequence
count-text(width<u64>) => (output<string>) :=
  h := #Count("aaaaaa", width, 6u64, empty)
  output := number/format(sequence/length(map/values(h)), 0u64).
text/join([count-text(width) | width <- [1u64 2u64]], "")
"#
    );
    each_plan(&source, |plan, owners, visits| {
        assert_eq!(owners.regions().len(), 1);
        let owner = &owners.regions()[0];
        let parent = owner.tail().parent().expect("enclosing comprehension");
        assert!(derive(plan, owners, 0, visits).is_some());
        assert!(strict_ancestor(plan, Some(parent), owner.iteration()));
        assert!(!strict_ancestor(
            plan,
            Some(owner.iteration()),
            owner.iteration()
        ));
        assert!(!strict_ancestor(plan, Some(owner.iteration()), parent));
    });
}

#[test]
fn packed_offset_producer_must_dominate_entry_on_every_branch() {
    let source = format!("{COUNT}\n#Count(text<string>, width<u64>, n<u64>, empty)\n");
    each_plan(&source, |_, _, original| {
        let mut visits = vec![original[0]; 4];
        visits[0].successors = [Some(1), Some(2)];
        visits[1].successors = [Some(3), None];
        visits[2].successors = [Some(3), None];
        visits[3].successors = [None, None];
        assert!(!dominates(&visits, 0, 1, 3));
        assert!(!dominates(&visits, 0, 2, 3));
        assert!(dominates(&visits, 0, 0, 3));
        assert!(!dominates(&visits, 0, 3, 3));
        visits[0].successors = [Some(1), None];
        assert!(dominates(&visits, 0, 1, 3));
        assert!(!dominates(&visits, 0, 2, 3));
    });
}

#[test]
fn packed_domain_comes_from_source_map_values_and_join_separator() {
    for (mapping, separator, symbols) in [
        (
            r#"{"a": "A", "c": "C", "g": "G", "t": "T"}"#,
            "",
            Some("ACGT"),
        ),
        (
            r#"{"a": "A", "c": "C", "g": "G", "t": "T"}"#,
            ":",
            Some(":ACGT"),
        ),
        (r#"{"a": "ab", "b": "bc"}"#, "", Some("abc")),
        (r#"{"a": "é", "b": "b"}"#, "", None),
        (r#"{"a": replacement<string>, "b": "b"}"#, "", None),
    ] {
        let source = format!(
            r#"{COUNT}
+> text
mapping := {mapping}
copy := mapping
sequence := text/join([copy{{char}} | char <- input<string>], "{separator}")
#Count(sequence, 2u64, 4u64, empty)
"#
        );
        each_plan(&source, |plan, owners, visits| {
            let domain = derive(plan, owners, 0, visits).unwrap().domain();
            match symbols {
                Some(symbols) => {
                    let domain = domain.expect("proved finite output alphabet");
                    for byte in 0..128u8 {
                        assert_eq!(domain.contains(byte), symbols.as_bytes().contains(&byte));
                    }
                    if symbols == "ACGT" {
                        assert_eq!((domain.bits(), domain.shift()), (2, Some(1)));
                    }
                }
                None => assert!(domain.is_none()),
            }
        });
    }
}
