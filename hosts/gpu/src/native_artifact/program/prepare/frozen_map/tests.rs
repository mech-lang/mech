use crate::native_artifact::program::prepare::frozen_map::*;
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

const COUNT: &str = r#"
+> map
+> sequence
increment(h<{string:u64}>, key<string>) => (out<{string:u64}>) :=
  previous := (key ∈ h)? | true => h{key} | false => 0u64.
  out := map/insert(h, key, previous + 1u64).
#Accumulate(s<string>, width<u64>, n<u64>, initial<{string:u64}>) => <{string:u64}>
  | :Advance(index<u64>, counts<{string:u64}>)
  | :Finish(counts<{string:u64}>).
#Accumulate(s, width, n, initial) -> :Advance(0u64, initial)
  :Advance(index, counts)
    | width == 0u64 -> :Finish(counts)
    | index >= n -> :Finish(counts)
    | width > n - index -> :Finish(counts)
    | * -> :Advance(index + 1u64, increment(counts, s[(index + 1u64)..=(index + width)]))
  :Finish(counts) => counts.
empty<{string:u64}> := map/new()
"#;

fn artifact(source: &str) -> mech_engine::ProgramArtifact {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x46524f5a), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    mech_engine::CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap()
}
fn parts(
    artifact: &mech_engine::ProgramArtifact,
) -> (
    Plan,
    NativeMapOwnerWords,
    crate::native_artifact::program::prepare::tail::Layout,
) {
    let plan = mech_engine::native_artifact::lower_native_program_artifact(
        artifact,
        &mech_stdlib::source_catalog(),
    )
    .unwrap();
    let owners = plan.map_owner_words().unwrap();
    let borrowed = crate::native_artifact::program::prepare::borrowed::Layout::derive(
        &plan,
        &plan.scalar_aggregates(),
    )
    .unwrap();
    let visits = crate::native_artifact::iteration_abi::control_visits(
        plan.blocks(),
        plan.iterations().iter().map(Into::into),
        0,
    )
    .unwrap();
    let closed = crate::native_artifact::program::prepare::tail::Layout::derive(
        &plan,
        &borrowed,
        &visits,
        &plan.scalar_aggregates(),
        &owners,
    )
    .unwrap();
    (plan, owners, closed)
}
fn lookup(body: &str, expression: &str) -> String {
    format!(
        "{COUNT}\nlookup(s<string>, query<string>) => (result<u64>) :=\n  counts := #Accumulate(s, 1u64, 6u64, empty)\n{body}\n{expression}\n"
    )
}

#[test]
fn frozen_map_proof_follows_complete_aliases_and_arbitrary_query_branches() {
    let source = lookup(
        "  alias := counts\n  selected := (query ∈ alias)? | true => alias{query} | false => 0u64.\n  result := selected.",
        "lookup(text<string>, query<string>)",
    );
    let original = artifact(&source);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    let mut proofs = Vec::new();
    for artifact in [original, decoded] {
        let (plan, owners, closed) = parts(&artifact);
        let layout = Layout::derive(&plan, &owners, &closed).unwrap();
        assert_eq!(layout.regions.len(), 1);
        let region = &layout.regions[0];
        assert_eq!(region.parent, None);
        assert!(region.aliases.contains(&region.output));
        assert_eq!(layout.reads.iter().flatten().count(), 2);
        // The source alias is eliminated before this plan; both reads
        // reference the same completed result rather than a host copy.
        assert_eq!(region.aliases, vec![region.output]);
        for (site, selected) in layout.reads.iter().enumerate() {
            if selected.is_none() {
                continue;
            }
            let Action::LibraryCall { operands, .. } = plan.blocks()[site].action() else {
                panic!("ordinary read only");
            };
            assert!(region.aliases.contains(&operands[0].get()));
            assert_eq!(plan.blocks()[site].iteration(), region.parent);
        }
        let wire = serde_json::to_vec(&layout).unwrap();
        let restored: Layout = serde_json::from_slice(&wire).unwrap();
        assert_eq!(
            restored,
            Layout::derive(&plan, &owners, &closed).unwrap(),
            "wire data alone is not proof: rederive from the same checked plan"
        );
        proofs.push(layout);
    }
    assert_eq!(proofs[0], proofs[1]);
}

#[test]
fn frozen_map_proof_keeps_each_worker_visit_local() {
    let source = lookup(
        "  selected := (query ∈ counts)? | true => counts{query} | false => 0u64.\n  result := selected.",
        "[lookup(text<string>, q) | q <- queries<[string]>]",
    );
    let (plan, owners, closed) = parts(&artifact(&source));
    let layout = Layout::derive(&plan, &owners, &closed).unwrap();
    assert_eq!(layout.regions.len(), 1);
    let region = &layout.regions[0];
    assert!(region.parent.is_some());
    let parent = plan.iterations()[region.parent.unwrap() as usize].as_ref();
    let after = reachable(&plan, region.parent, region.exit, None);
    assert!(after[parent.latch() as usize]);
    assert!(
        !after[parent.header() as usize],
        "retirement prevents a prior visit reaching the next header"
    );
    assert!(!after[parent.body() as usize]);
    for value in &region.aliases {
        assert_eq!(
            plan.value_scopes()[*value as usize],
            Scope::Immutable {
                iteration: region.parent
            }
        );
    }
}

#[test]
fn frozen_map_proof_rejects_any_unsupported_use_before_selecting_storage() {
    let cases = [
        // A whole-map output preserves canonical publication and ordering.
        format!("{COUNT}\n#Accumulate(text<string>, 1u64, 6u64, empty)\n"),
        lookup(
            "  changed := map/insert(counts, query, 7u64)\n  result := changed{query}.",
            "lookup(text<string>, query<string>)",
        ),
        lookup(
            "  keys := map/keys(counts)\n  result := sequence/length(keys) + counts{query}.",
            "lookup(text<string>, query<string>)",
        ),
        lookup(
            "  chosen := (query == \"a\")? | true => counts | false => empty.\n  result := chosen{query}.",
            "lookup(text<string>, query<string>)",
        ),
        // A descendant comprehension captures this map: its lifetime and
        // current visit need a separate proof, even though every use reads.
        format!(
            "{COUNT}\nlookup(s<string>, queries<[string]>) => (result<u64>) :=\n  counts := #Accumulate(s, 1u64, 6u64, empty)\n  values := [counts{{q}} | q <- queries]\n  result := sequence/length(values).\nlookup(text<string>, queries<[string]>)\n"
        ),
    ];
    for source in cases {
        let (plan, owners, closed) = parts(&artifact(&source));
        assert!(
            closed.regions.iter().any(|region| region.packed.is_some()),
            "the negative fixture must reach the prior packed-owner proof"
        );
        assert!(
            Layout::derive(&plan, &owners, &closed)
                .unwrap()
                .regions
                .is_empty(),
            "{source}"
        );
    }
}

#[test]
fn frozen_map_proof_rejects_hidden_escape_and_ambiguous_definition_inventory() {
    let source = lookup(
        "  result := counts{query}.",
        "lookup(text<string>, query<string>)",
    );
    let (plan, owners, closed) = parts(&artifact(&source));
    let selected = Layout::derive(&plan, &owners, &closed).unwrap();
    assert_eq!(selected.regions.len(), 1);
    let region = &selected.regions[0];
    let mut inventory = Inventory::derive(&plan).unwrap();
    assert!(candidate(&plan, &owners, &closed, region.closed, &inventory).is_some());
    inventory.uses[region.output as usize].push(Use::Other);
    assert!(candidate(&plan, &owners, &closed, region.closed, &inventory).is_none());
    inventory.uses[region.output as usize].pop();
    inventory.definitions[region.output as usize].push(usize::MAX);
    assert!(candidate(&plan, &owners, &closed, region.closed, &inventory).is_none());
}
