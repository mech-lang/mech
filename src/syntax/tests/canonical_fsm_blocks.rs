//! Guarded FSM blocks must win their shared prefix with record-valued states.

use mech_syntax::document::{
    AstNode, DocumentId, FsmArmBodySyntax, FsmBodyTransitionSyntax, FsmImplementationSyntax,
    ParseConfig, Revision, SyntaxKind, SyntaxNode, TextSnapshot, VariableAssignSyntax,
    VariableDefineSyntax, parse_canonical_document,
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
