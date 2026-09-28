//! Cross-artifact comparison resolves global identities in their owning tables.
use mech_core::{OperationContractId, ResolvedOperationContract, SchemaId};
use mech_engine::{ControlBlock, ControlValue, ExecutableNodeBody, ProgramArtifact};

pub(super) fn node_bodies_semantically_equal(
    source: &ProgramArtifact,
    left: &ExecutableNodeBody,
    target: &ProgramArtifact,
    right: &ExecutableNodeBody,
) -> bool {
    let comparison = Comparison { source, target };
    match (left, right) {
        (ExecutableNodeBody::Operation(left), ExecutableNodeBody::Operation(right)) => {
            left.operation == right.operation
                && comparison.contract(left.contract, right.contract)
                && left
                    .requirement
                    .and_then(|id| source.requirements().get(id))
                    == right
                        .requirement
                        .and_then(|id| target.requirements().get(id))
        }
        (ExecutableNodeBody::Match(left), ExecutableNodeBody::Match(right)) => {
            comparison.match_declaration(left, right)
        }
        (ExecutableNodeBody::Activation(left), ExecutableNodeBody::Activation(right)) => {
            comparison.match_declaration(left, right)
        }
        (ExecutableNodeBody::Comprehension(left), ExecutableNodeBody::Comprehension(right)) => {
            comparison.comprehension_declaration(left, right)
        }
        (ExecutableNodeBody::Fsm(left), ExecutableNodeBody::Fsm(right)) => left == right,
        _ => false,
    }
}

struct Comparison<'a> {
    source: &'a ProgramArtifact,
    target: &'a ProgramArtifact,
}

impl Comparison<'_> {
    fn schema(&self, left: SchemaId, right: SchemaId) -> bool {
        self.source.schemas().get(left).is_some()
            && self.source.schemas().get(left) == self.target.schemas().get(right)
    }

    fn contract(&self, left: OperationContractId, right: OperationContractId) -> bool {
        let (
            Some(ResolvedOperationContract::Declared(left)),
            Some(ResolvedOperationContract::Declared(right)),
        ) = (
            self.source.contracts().get(left),
            self.target.contracts().get(right),
        )
        else {
            return false;
        };
        left.interaction == right.interaction
            && left.inputs.len() == right.inputs.len()
            && left.inputs.iter().zip(&right.inputs).all(|(left, right)| {
                self.schema(left.schema, right.schema)
                    && left.access == right.access
                    && left.delivery == right.delivery
            })
            && left.outputs.len() == right.outputs.len()
            && left
                .outputs
                .iter()
                .zip(&right.outputs)
                .all(|(left, right)| {
                    self.schema(left.schema, right.schema)
                        && left.access == right.access
                        && left.delivery == right.delivery
                        && left.construction == right.construction
                        && left.alias == right.alias
                        && left.change_detection == right.change_detection
                })
    }

    fn pattern(&self, left: &mech_engine::MatchPattern, right: &mech_engine::MatchPattern) -> bool {
        match (left, right) {
            (
                mech_engine::MatchPattern::Literal(left),
                mech_engine::MatchPattern::Literal(right),
            ) => self.value(
                ControlValue::Constant(*left),
                ControlValue::Constant(*right),
            ),
            (mech_engine::MatchPattern::Wildcard, mech_engine::MatchPattern::Wildcard)
            | (mech_engine::MatchPattern::Bind, mech_engine::MatchPattern::Bind) => true,
            (
                mech_engine::MatchPattern::Structural(left),
                mech_engine::MatchPattern::Structural(right),
            ) => self.collection_pattern(left, right, |comparison, left, right| {
                use mech_engine::MatchPatternValue;
                match (left, right) {
                    (MatchPatternValue::Literal(left), MatchPatternValue::Literal(right)) => {
                        comparison.value(
                            ControlValue::Constant(*left),
                            ControlValue::Constant(*right),
                        )
                    }
                    (MatchPatternValue::Binding(left), MatchPatternValue::Binding(right)) => {
                        left == right
                    }
                    (MatchPatternValue::Input(left), MatchPatternValue::Input(right)) => {
                        left == right
                    }
                    _ => false,
                }
            }),
            _ => false,
        }
    }

    fn value(&self, left: ControlValue, right: ControlValue) -> bool {
        match (left, right) {
            (ControlValue::Constant(left), ControlValue::Constant(right)) => {
                let left = self
                    .source
                    .constants()
                    .entry(left)
                    .map(|entry| entry.hash());
                left.is_some()
                    && left
                        == self
                            .target
                            .constants()
                            .entry(right)
                            .map(|entry| entry.hash())
            }
            // These identities are canonical within one declaration, not table offsets.
            (ControlValue::Parameter { .. }, ControlValue::Parameter { .. })
            | (ControlValue::Local { .. }, ControlValue::Local { .. }) => left == right,
            _ => false,
        }
    }

    fn collection_value(
        &self,
        left: mech_engine::ComprehensionValue,
        right: mech_engine::ComprehensionValue,
    ) -> bool {
        use mech_engine::ComprehensionValue;
        match (left, right) {
            (ComprehensionValue::Constant(left), ComprehensionValue::Constant(right)) => {
                self.value(ControlValue::Constant(left), ControlValue::Constant(right))
            }
            (ComprehensionValue::Input(_), ComprehensionValue::Input(_))
            | (ComprehensionValue::Local(_), ComprehensionValue::Local(_)) => left == right,
            _ => false,
        }
    }

    fn collection_pattern<V, F>(
        &self,
        left: &mech_engine::CollectionPattern<SchemaId, V>,
        right: &mech_engine::CollectionPattern<SchemaId, V>,
        values_equal: F,
    ) -> bool
    where
        F: Fn(&Self, &V, &V) -> bool + Copy,
    {
        use mech_engine::CollectionPattern;
        let fields = |left: &[CollectionPattern<SchemaId, V>],
                      right: &[CollectionPattern<SchemaId, V>]| {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| self.collection_pattern(left, right, values_equal))
        };
        match (left, right) {
            (CollectionPattern::Wildcard, CollectionPattern::Wildcard) => true,
            (
                CollectionPattern::Bind {
                    local: left,
                    schema: left_schema,
                },
                CollectionPattern::Bind {
                    local: right,
                    schema: right_schema,
                },
            ) => left == right && self.schema(*left_schema, *right_schema),
            (CollectionPattern::Equal(left), CollectionPattern::Equal(right)) => {
                values_equal(self, left, right)
            }
            (
                CollectionPattern::Enum {
                    ordinal: left_ordinal,
                    payload: left_payload,
                },
                CollectionPattern::Enum {
                    ordinal: right_ordinal,
                    payload: right_payload,
                },
            ) => {
                left_ordinal == right_ordinal
                    && match (left_payload, right_payload) {
                        (Some(left), Some(right)) => {
                            self.collection_pattern(left, right, values_equal)
                        }
                        (None, None) => true,
                        _ => false,
                    }
            }
            (CollectionPattern::Tuple(left), CollectionPattern::Tuple(right)) => {
                fields(left, right)
            }
            (
                CollectionPattern::Array {
                    prefix: left_prefix,
                    rest: left_rest,
                    suffix: left_suffix,
                },
                CollectionPattern::Array {
                    prefix: right_prefix,
                    rest: right_rest,
                    suffix: right_suffix,
                },
            ) => {
                fields(left_prefix, right_prefix)
                    && fields(left_suffix, right_suffix)
                    && match (left_rest, right_rest) {
                        (Some(left), Some(right)) => {
                            self.collection_pattern(left, right, values_equal)
                        }
                        (None, None) => true,
                        _ => false,
                    }
            }
            _ => false,
        }
    }

    fn match_declaration(
        &self,
        left: &mech_engine::MatchDeclaration,
        right: &mech_engine::MatchDeclaration,
    ) -> bool {
        left.scrutinee == right.scrutinee
            && left.partial == right.partial
            && left.captures.len() == right.captures.len()
            && left
                .captures
                .iter()
                .zip(&right.captures)
                .all(|(left, right)| {
                    left.input == right.input && self.schema(left.schema, right.schema)
                })
            && left.arms.len() == right.arms.len()
            && left.arms.iter().zip(&right.arms).all(|(left, right)| {
                self.pattern(&left.pattern, &right.pattern)
                    && match (&left.guard, &right.guard) {
                        (Some(left), Some(right)) => self.block(left, right),
                        (None, None) => true,
                        _ => false,
                    }
                    && self.block(&left.body, &right.body)
            })
    }

    fn comprehension_declaration(
        &self,
        left: &mech_engine::ComprehensionDeclaration,
        right: &mech_engine::ComprehensionDeclaration,
    ) -> bool {
        left.id == right.id
            && left.kind == right.kind
            && self.collection_value(left.yield_value, right.yield_value)
            && left.steps.len() == right.steps.len()
            && left.steps.iter().zip(&right.steps).all(|(left, right)| {
                use mech_engine::ComprehensionStep;
                match (left, right) {
                    (
                        ComprehensionStep::Generator {
                            source: left,
                            pattern: left_pattern,
                        },
                        ComprehensionStep::Generator {
                            source: right,
                            pattern: right_pattern,
                        },
                    ) => {
                        self.collection_value(*left, *right)
                            && self.collection_pattern(
                                left_pattern,
                                right_pattern,
                                |comparison, left, right| {
                                    comparison.collection_value(*left, *right)
                                },
                            )
                    }
                    (ComprehensionStep::Filter(left), ComprehensionStep::Filter(right)) => {
                        self.collection_value(*left, *right)
                    }
                    (ComprehensionStep::Operation(left), ComprehensionStep::Operation(right)) => {
                        left.local == right.local
                            && self.local_body(&left.body, &right.body)
                            && self.schema(left.schema, right.schema)
                            && left.inputs.len() == right.inputs.len()
                            && left
                                .inputs
                                .iter()
                                .zip(&right.inputs)
                                .all(|(left, right)| self.collection_value(*left, *right))
                    }
                    _ => false,
                }
            })
    }

    fn local_body(
        &self,
        left: &mech_engine::ControlOperationBody,
        right: &mech_engine::ControlOperationBody,
    ) -> bool {
        use mech_engine::ControlOperationBody;
        match (left, right) {
            (
                ControlOperationBody::Operation {
                    operation: left,
                    contract: left_contract,
                },
                ControlOperationBody::Operation {
                    operation: right,
                    contract: right_contract,
                },
            ) => left == right && self.contract(*left_contract, *right_contract),
            (ControlOperationBody::Match(left), ControlOperationBody::Match(right)) => {
                self.match_declaration(left, right)
            }
            (
                ControlOperationBody::Comprehension(left),
                ControlOperationBody::Comprehension(right),
            ) => self.comprehension_declaration(left, right),
            (ControlOperationBody::Recur(left), ControlOperationBody::Recur(right)) => {
                left == right
            }
            (ControlOperationBody::Suspend, ControlOperationBody::Suspend)
            | (ControlOperationBody::Publish, ControlOperationBody::Publish) => true,
            _ => false,
        }
    }

    fn block(&self, left: &ControlBlock, right: &ControlBlock) -> bool {
        left.id == right.id
            && left.parameters.len() == right.parameters.len()
            && left
                .parameters
                .iter()
                .zip(&right.parameters)
                .all(|(left, right)| {
                    left.source == right.source && self.schema(left.schema, right.schema)
                })
            && left.operations.len() == right.operations.len()
            && left
                .operations
                .iter()
                .zip(&right.operations)
                .all(|(left, right)| {
                    left.node == right.node
                        && self.local_body(&left.body, &right.body)
                        && self.schema(left.schema, right.schema)
                        && left.inputs.len() == right.inputs.len()
                        && left
                            .inputs
                            .iter()
                            .zip(&right.inputs)
                            .all(|(&left, &right)| self.value(left, right))
                })
            && self.value(left.yield_value, right.yield_value)
    }
}

#[cfg(test)]
mod tests {
    use super::node_bodies_semantically_equal;
    use mech_engine::{CanonicalSourceFrontend, ExecutableNodeBody, ProgramArtifact};
    use mech_syntax::document::{
        AstNode, DocumentId, DocumentSyntax, ExpressionSyntax, ParseConfig, Revision, SyntaxNode,
        TextSnapshot, parse_canonical_document,
    };

    fn compile(source: &str) -> ProgramArtifact {
        use mech_syntax::document::parser::{
            canonical::parse_canonical_phase_2i_rule_for_test, rules,
        };
        fn expression(node: SyntaxNode) -> Option<ExpressionSyntax> {
            ExpressionSyntax::cast(node.clone()).or_else(|| node.children().find_map(expression))
        }
        let parsed = parse_canonical_phase_2i_rule_for_test(
            TextSnapshot::new(DocumentId(822), Revision(1), source).unwrap(),
            rules::EXPRESSION,
            ParseConfig::default(),
        )
        .unwrap();
        assert!(parsed.is_strictly_clean(), "{source}");
        assert_eq!(parsed.consumed.end.0 as usize, source.len());
        mech_engine::CanonicalSourceFrontend
            .compile_expression(&expression(parsed.syntax()).unwrap())
            .unwrap()
            .compile_artifact()
            .unwrap()
    }

    fn control(artifact: &ProgramArtifact) -> &ExecutableNodeBody {
        &artifact
            .nodes()
            .iter()
            .find(|node| matches!(node.body, ExecutableNodeBody::Match(_)))
            .unwrap()
            .body
    }

    fn compile_document(source: &str) -> ProgramArtifact {
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(823), Revision(1), source).unwrap(),
            ParseConfig::default(),
        );
        let document = DocumentSyntax::cast(parsed.syntax()).unwrap();
        CanonicalSourceFrontend
            .compile_document(&document)
            .unwrap()
            .compile_artifact()
            .unwrap()
    }

    fn activation(artifact: &ProgramArtifact) -> &ExecutableNodeBody {
        &artifact
            .nodes()
            .iter()
            .find(|node| matches!(node.body, ExecutableNodeBody::Activation(_)))
            .unwrap()
            .body
    }

    #[test]
    fn activation_reuse_compares_computed_pattern_inputs() {
        let source = "event := event-source<[f64]:1,2>\nexpected := expected-source<f64>\n~selected := 0\n~> event\n  | [head, expected + 0] => { selected = head }\n  | * => { selected = -1 }\nselected\n";
        let original = compile_document(source);
        let shifted = compile_document(&format!("padding := 7u8\n{source}"));
        assert!(node_bodies_semantically_equal(
            &original,
            activation(&original),
            &shifted,
            activation(&shifted),
        ));
    }

    #[test]
    fn literal_pattern_reuse_resolves_the_owning_constant_arena() {
        let source = "signal<f64> ? | 0 => 10 | * => 20";
        let original = compile(source);
        let mut moved = false;
        for extra in 1..12 {
            let shifted = compile(&format!("({extra}u8, ({source}))"));
            moved |= control(&original) != control(&shifted);
            assert!(node_bodies_semantically_equal(
                &original,
                control(&original),
                &shifted,
                control(&shifted)
            ));
        }
        assert!(moved);
        let changed = compile("signal<f64> ? | 1 => 10 | * => 20");
        assert!(!node_bodies_semantically_equal(
            &original,
            control(&original),
            &changed,
            control(&changed)
        ));
    }

    #[test]
    fn structural_match_reuse_compares_the_complete_canonical_pattern() {
        let source = "signal<(f64,f64)> ? | (left, right) => left + right | * => 0";
        let original = compile(source);
        let shifted = compile(&format!("(7u8, ({source}))"));
        assert!(node_bodies_semantically_equal(
            &original,
            control(&original),
            &shifted,
            control(&shifted)
        ));

        let changed = compile("signal<(f64,f64)> ? | (same, same) => same + same | * => 0");
        assert!(!node_bodies_semantically_equal(
            &original,
            control(&original),
            &changed,
            control(&changed)
        ));

        let source = "[1 2 3] ? | [head | [2, 3]] => head | * => 0";
        let original = compile(source);
        let shifted = compile(&format!("(7u8, ({source}))"));
        assert!(node_bodies_semantically_equal(
            &original,
            control(&original),
            &shifted,
            control(&shifted)
        ));
        let changed = compile("[1 2 3] ? | [head | [2, 4]] => head | * => 0");
        assert!(!node_bodies_semantically_equal(
            &original,
            control(&original),
            &changed,
            control(&changed)
        ));
    }

    #[test]
    fn nested_control_reuse_resolves_inner_tables_and_detects_inner_changes() {
        let source = "signal<f64> ? | item => (item ? | 0 => item + 1 | * => item + 2)";
        let original = compile(source);
        let mut moved = false;
        for extra in 3..12 {
            let shifted = compile(&format!("({extra}u8, ({source}))"));
            moved |= control(&original) != control(&shifted);
            assert!(node_bodies_semantically_equal(
                &original,
                control(&original),
                &shifted,
                control(&shifted)
            ));
        }
        assert!(moved);
        for changed in [
            source.replace("| 0", "| 1"),
            source.replace("item + 2", "item * 2"),
        ] {
            let changed = compile(&changed);
            assert!(!node_bodies_semantically_equal(
                &original,
                control(&original),
                &changed,
                control(&changed)
            ));
        }
    }

    #[test]
    fn control_reuse_resolves_cross_artifact_tables_and_rejects_changed_semantics() {
        let source = "flag<bool> ? | x, !x => signal<f64> + 2 | * => signal<f64> + 1";
        let original = compile(source);
        let mut moved = false;
        for extra in 3..12 {
            let shifted = compile(&format!("({extra}u8, ({source}))"));
            moved |= control(&original) != control(&shifted);
            assert!(node_bodies_semantically_equal(
                &original,
                control(&original),
                &shifted,
                control(&shifted)
            ));
        }
        assert!(
            moved,
            "exercise different global IDs, not just identical arenas"
        );
        for changed in [
            "flag<bool> ? | x, x => signal<f64> + 2 | * => signal<f64> + 1",
            "flag<bool> ? | x, !x => signal<f64> + 3 | * => signal<f64> + 1",
            "flag<bool> ? | x, !x => signal<f64> * 2 | * => signal<f64> + 1",
        ] {
            let changed = compile(changed);
            assert!(!node_bodies_semantically_equal(
                &original,
                control(&original),
                &changed,
                control(&changed)
            ));
        }
    }
    #[test]
    fn collection_reuse_resolves_owning_tables_and_rejects_changed_control() {
        fn control(artifact: &ProgramArtifact) -> &ExecutableNodeBody {
            &artifact
                .nodes()
                .iter()
                .find(|node| matches!(node.body, ExecutableNodeBody::Comprehension(_)))
                .unwrap()
                .body
        }
        let source = "[x + 1 | x <- signal<[f64]:1,3>, x > 0]";
        let original = compile(source);
        let mut moved = false;
        for extra in 3..12 {
            let shifted = compile(&format!("({extra}u8, ({source}))"));
            moved |= control(&original) != control(&shifted);
            assert!(node_bodies_semantically_equal(
                &original,
                control(&original),
                &shifted,
                control(&shifted)
            ));
        }
        assert!(moved, "exercise artifact-local table identities");
        for source in [
            "[x + 2 | x <- signal<[f64]:1,3>, x > 0]",
            "[x + 1 | x <- signal<[f64]:1,3>, x > 1]",
            "{x + 1 | x <- signal<[f64]:1,3>, x > 0}",
            "[x + 1 | x <- signal<[f64]:1,3>, x < 0]",
        ] {
            let changed = compile(source);
            assert!(
                !node_bodies_semantically_equal(
                    &original,
                    control(&original),
                    &changed,
                    control(&changed)
                ),
                "{source}"
            );
        }
        let structured = compile("[x | (x, 1) <- pairs]");
        let changed = compile("[x | (x, 2) <- pairs]");
        assert!(!node_bodies_semantically_equal(
            &structured,
            control(&structured),
            &changed,
            control(&changed)
        ));
    }
}
