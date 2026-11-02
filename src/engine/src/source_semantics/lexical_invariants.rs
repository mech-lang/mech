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

#[cfg(test)]
mod tests {
    use mech_syntax::document::{
        AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    };

    #[test]
    fn checked_ordinary_functions_preserve_turn_shaped_f64_arguments() {
        for checked in [false, true] {
            let assertion = if checked { "  allowed! := ok\n" } else { "" };
            let source = format!(
                "checked(xs<[f64]>, ok<bool>) => (out<u64>) :=\n{assertion}  out := 7u64.\nchecked(input<[f64]>, ok<bool>)\n"
            );
            let parsed = mech_syntax::document::parse_canonical_document(
                TextSnapshot::new(DocumentId(824), Revision(1), source.as_str()).unwrap(),
                ParseConfig::default(),
            );
            assert!(parsed.is_strictly_clean());
            let document = DocumentSyntax::cast(parsed.syntax()).unwrap();
            let artifact = crate::CanonicalSourceFrontend
                .compile_document(&document)
                .unwrap()
                .compile_artifact()
                .unwrap();
            assert!(artifact.constraints().is_empty());
            let bytes = crate::encode_program_artifact_bytecode_v1(&artifact).unwrap();
            crate::decode_program_artifact_bytecode_v1(&bytes).unwrap();
        }
    }
}
