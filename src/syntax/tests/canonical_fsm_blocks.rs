//! Guarded FSM blocks must win their shared prefix with record-valued states.

use mech_syntax::document::{
    AstNode, DocumentId, DocumentStream, FsmArmBodySyntax, FsmBodyTransitionSyntax,
    FsmImplementationSyntax, ParseConfig, Revision, StreamProgress, SyntaxKind, SyntaxNode,
    TextSnapshot, VariableAssignSyntax, VariableDefineSyntax, parse_canonical_document,
};

fn collect<T: AstNode>(node: &SyntaxNode, values: &mut Vec<T>) {
    if let Some(value) = T::cast(node.clone()) {
        values.push(value);
    }
    for child in node.children() {
        collect(&child, values);
    }
}

fn machine(source: &str) -> FsmImplementationSyntax {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x46534d), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}\n{:?}",
        parsed.diagnostics
    );
    let mut implementations = Vec::new();
    collect(&parsed.syntax(), &mut implementations);
    assert_eq!(implementations.len(), 1);
    implementations.pop().unwrap()
}

#[test]
fn guarded_blocks_retain_definitions_mutability_and_indexed_assignments() {
    let implementation = machine(
        r#"
#Advance(p<[u8]:16,1>, c<[u8]:16,1>) => <([u8]:16,1,[u8]:16,1)>
  | :Carry(p<[u8]:16,1>, c<[u8]:16,1>, rank<u64>)
  | :Done(q<[u8]:16,1>, counts<[u8]:16,1>).
#Advance(p, c) -> :Carry(p, c, 1u64)
  :Carry(p, c, rank)
    | c[rank + 1u64] + 1u64 > rank -> {
        q := sequence/rotate-prefix(p, rank + 1u64, 1u64)
        ~counts := c
        counts[rank + 1u64] = 0u64
      } -> :Carry(q, counts, rank + 1u64)
    | * -> {
        q := sequence/rotate-prefix(p, rank + 1u64, 1u64)
        ~counts := c
        counts[rank + 1u64] = c[rank + 1u64] + 1u64
      } -> :Done(q, counts)
  :Done(q, counts) => (q, counts).
"#,
    );
    let arms = implementation.arms();
    let Some(FsmArmBodySyntax::Guard(guarded)) = arms[0].body() else {
        panic!("expected a guarded recurring state");
    };
    let guards = guarded.guards();
    assert_eq!(guards.len(), 2);
    for guard in guards {
        let transitions = guard.transitions();
        assert_eq!(transitions.len(), 2);
        let FsmBodyTransitionSyntax::Block(block) = &transitions[0] else {
            panic!("guarded braces must remain a block, not a recovered record value");
        };
        assert!(matches!(transitions[1], FsmBodyTransitionSyntax::State(_)));
        let statements = block
            .items()
            .into_iter()
            .map(|item| item.value().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(statements.len(), 3);
        assert!(statements.iter().all(|s| s.kind() == SyntaxKind::Statement));
        let ordinary = statements[0]
            .children()
            .find_map(VariableDefineSyntax::cast)
            .unwrap();
        assert!(ordinary.mutability_marker().is_none());
        let mutable = statements[1]
            .children()
            .find_map(VariableDefineSyntax::cast)
            .unwrap();
        assert!(mutable.mutability_marker().is_some());
        let assignment = statements[2]
            .children()
            .find_map(VariableAssignSyntax::cast)
            .unwrap();
        assert_eq!(
            assignment
                .target()
                .unwrap()
                .subscripts()
                .unwrap()
                .items()
                .len(),
            1
        );
    }
}

#[test]
fn guarded_empty_block_is_followed_by_its_state_transition() {
    let implementation = machine(
        r#"
#Empty(n<u64>) => <u64>
  | :Running(n<u64>)
  | :Done(value<u64>).
#Empty(n) -> :Running(n)
  :Running(n)
    | true -> {} -> :Done(n)
  :Done(value) => value.
"#,
    );
    let Some(FsmArmBodySyntax::Guard(guarded)) = implementation.arms()[0].body() else {
        panic!("expected guarded state");
    };
    let transitions = guarded.guards()[0].transitions();
    assert_eq!(transitions.len(), 2);
    let FsmBodyTransitionSyntax::Block(block) = &transitions[0] else {
        panic!("expected empty block");
    };
    assert!(block.items().is_empty());
    assert!(matches!(transitions[1], FsmBodyTransitionSyntax::State(_)));
}

fn comments(implementation: &FsmImplementationSyntax) -> Vec<String> {
    implementation
        .arms()
        .into_iter()
        .filter_map(|arm| match arm.body() {
            Some(FsmArmBodySyntax::Comment(comment)) => Some(comment.syntax().text().unwrap()),
            _ => None,
        })
        .collect()
}

#[test]
fn fsm_comments_keep_their_arm_instead_of_pattern_recovery() {
    for newline in ["\n", "\r\n"] {
        for separator in [" ", ""] {
            let source = format!(
                "#machine(n) -> :A(n)\n  --{separator}First comment\n  :A(n) -> :Done(n)\n  --{separator}Second comment\n  :Done(n) => n.\n"
            ).replace('\n', newline);
            let implementation = machine(&source);
            let comments = comments(&implementation);
            assert_eq!(comments.len(), 2, "{source}");
            assert!(comments[0].contains("First comment"));
            assert!(comments[1].contains("Second comment"));
            assert_eq!(implementation.arms().len(), 4);
        }
    }
}

#[test]
fn fsm_double_unary_patterns_still_select_transition_and_guard_arms() {
    for arm in ["--n -> :Done(n)", "--n\n    | true -> :Done(n)"] {
        for newline in ["\n", "\r\n"] {
            let source =
                format!("#machine(n) -> :A(n)\n  {arm}\n  :Done(n) => n.\n").replace('\n', newline);
            let implementation = machine(&source);
            assert!(comments(&implementation).is_empty(), "{source}");
            assert_eq!(implementation.arms().len(), 2);
            assert!(matches!(
                implementation.arms()[0].body(),
                Some(FsmArmBodySyntax::Transition(_) | FsmArmBodySyntax::Guard(_))
            ));
        }
    }
}

#[test]
fn maintained_bubble_sort_keeps_all_three_fsm_comment_arms() {
    let source = include_str!("../../../examples/working/bubble-sort.mec");
    let start = source.find("#bubble-sort(arr) →").unwrap();
    let end = start + source[start..].find("\r\n\r\n4. Interface").unwrap();
    let implementation = machine(&source[start..end]);
    let comments = comments(&implementation);
    assert_eq!(comments.len(), 3);
    assert!(comments[0].contains("Pass: compare adjacent elements"));
    assert!(comments[1].contains("After a pass, reverse the accumulator"));
    assert!(comments[2].contains("Return the sorted array"));
    assert_eq!(implementation.arms().len(), 11);
}

#[test]
fn fsm_comments_and_unary_patterns_survive_every_scalar_cut_and_tiny_yields() {
    fn drain(stream: &mut DocumentStream, mut progress: StreamProgress) {
        let mut polls = 0;
        while progress == StreamProgress::NeedsProcessing {
            progress = stream.advance(1).progress;
            polls += 1;
            assert!(polls < 2_000_000);
        }
        assert_eq!(progress, StreamProgress::NeedInput);
    }
    for newline in ["\n", "\r\n"] {
        for arm in [
            "-- First comment",
            "--First comment",
            "--n -> :A(n)",
            "-- n -> :A(n)",
        ] {
            let source = format!(
                "#machine(n) -> :A(n)\n  {arm}\n  :A(n) -> :Done(n)\n  -- Second comment\n  :Done(n) => n.\n"
            ).replace('\n', newline);
            let baseline = parse_canonical_document(
                TextSnapshot::new(DocumentId(0x46534d), Revision(1), source.as_str()).unwrap(),
                ParseConfig::default(),
            );
            assert!(
                baseline.is_strictly_clean(),
                "{source}\n{:?}",
                baseline.diagnostics
            );
            for cut in (0..=source.len()).filter(|cut| source.is_char_boundary(*cut)) {
                let mut stream = DocumentStream::new(DocumentId(0x46534d), ParseConfig::default());
                let update = stream.append(&source[..cut], 1).unwrap();
                drain(&mut stream, update.progress);
                let update = stream.append(&source[cut..], 1).unwrap();
                drain(&mut stream, update.progress);
                let mut progress = stream.finish(1).progress;
                while progress == StreamProgress::NeedsProcessing {
                    progress = stream.advance(1).progress;
                }
                assert_eq!(progress, StreamProgress::Finished, "cut={cut}: {source}");
                let parsed = stream.materialize().unwrap();
                assert!(
                    parsed.is_strictly_clean(),
                    "cut={cut}: {source}\n{:?}",
                    parsed.diagnostics
                );
                assert_eq!(
                    parsed.root.structural_hash, baseline.root.structural_hash,
                    "cut={cut}: {source}"
                );
                assert_eq!(parsed.syntax().text().unwrap(), source);
            }
        }
    }
}
