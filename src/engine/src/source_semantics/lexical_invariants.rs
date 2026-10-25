//! Statement invariants belong to their current invocation/visit. Root
//! document constraints keep their separate retained-publication semantics.
use super::*;

impl SemanticBuilder {
    pub(super) fn lexical_assertion(
        &mut self,
        condition: PendingValue,
        name: String,
        syntax: &SyntaxNode,
    ) -> Result<(), SourceSemanticError> {
        if self.control_depth == 0 {
            return Err(SourceSemanticError {
                code: "source-semantics/unframed-lexical-invariant",
                message: "a lexical invariant requires its complete invocation frame".to_owned(),
                anchor: SourceSemanticAnchor::for_node(syntax),
            });
        }
        self.nodes.push(PendingNode {
            body: PendingNodeBody::LocalAssert,
            inferable_projection: false,
            inputs: vec![condition],
            schema: builtin_schema_draft(BuiltinSchema::Bool),
            exposes_output: true,
            state: None,
            semantic: SourceSemanticNode {
                operation: "assert".to_owned(),
                role: "local-invariant",
                detail: Some(name),
                anchor: SourceSemanticAnchor::for_node(syntax),
            },
        });
        Ok(())
    }
}
