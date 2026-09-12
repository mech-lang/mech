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
        (ExecutableNodeBody::BooleanMatch(left), ExecutableNodeBody::BooleanMatch(right)) => {
            left.scrutinee == right.scrutinee
                && left.captures.len() == right.captures.len()
                && left
                    .captures
                    .iter()
                    .zip(&right.captures)
                    .all(|(left, right)| {
                        left.input == right.input && comparison.schema(left.schema, right.schema)
                    })
                && left.arms.len() == right.arms.len()
                && left.arms.iter().zip(&right.arms).all(|(left, right)| {
                    left.pattern == right.pattern
                        && match (&left.guard, &right.guard) {
                            (Some(left), Some(right)) => comparison.block(left, right),
                            (None, None) => true,
                            _ => false,
                        }
                        && comparison.block(&left.body, &right.body)
                })
        }
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
                        && left.operation == right.operation
                        && self.schema(left.schema, right.schema)
                        && self.contract(left.contract, right.contract)
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
    use super::*;
    use mech_syntax::document::{
        AstNode, DocumentId, ExpressionSyntax, ParseConfig, Revision, SyntaxNode, TextSnapshot,
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
            .find(|node| matches!(node.body, ExecutableNodeBody::BooleanMatch(_)))
            .unwrap()
            .body
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
}
