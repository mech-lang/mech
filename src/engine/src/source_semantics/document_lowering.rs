//! Document statements lower through the same maintained operations and state
//! slots as expression compilation. Each mutable binding retains one writer.

use mech_syntax::document::{
    CanonicalOpAssign, OpAssignSyntax, SliceRefSyntax, VariableAssignSyntax,
};

use super::*;

impl SemanticBuilder {
    /// A state reference in a statement reads the latest candidate produced by
    /// preceding statements. The writer's original self input denotes the
    /// committed value from the previous turn.
    pub(super) fn read_document_binding(
        &mut self,
        binding: PendingBinding,
        syntax: &SyntaxNode,
    ) -> PendingValue {
        let value = match binding {
            PendingBinding::Value(value) => value,
            PendingBinding::MutableState(state) => self.current_state_value(state),
        };
        if matches!(value, PendingValue::State(_)) {
            // A derived read preserves the value's source-order version even
            // when it is exported after the final state writer publishes.
            self.emit_with_schema_draft(
                "core/assign",
                vec![value],
                self.schema_draft_of(value),
                syntax,
                "state-read",
                None,
            )
        } else {
            value
        }
    }

    /// Artifact state reads before their writer refer to the previous turn.
    /// Derived candidates already encode statement order, so publication can
    /// occur after all candidates without introducing another state writer.
    pub(super) fn order_document_state_writers(&mut self) {
        let mut order = (0..self.nodes.len()).collect::<Vec<_>>();
        order.sort_by_key(|index| self.nodes[*index].state.is_some());
        let mut indices = vec![0; order.len()];
        for (new, old) in order.iter().enumerate() {
            indices[*old] = new as u32;
        }
        let remap = |value: &mut PendingValue| {
            if let PendingValue::Node(node) = value {
                *node = indices[*node as usize];
            }
        };
        let mut nodes = core::mem::take(&mut self.nodes)
            .into_iter()
            .map(Some)
            .collect::<Vec<_>>();
        self.nodes = order
            .into_iter()
            .map(|old| {
                let mut node = nodes[old].take().unwrap();
                for input in &mut node.inputs {
                    remap(input);
                }
                node
            })
            .collect();
        for state in &mut self.states {
            state.producer_node = indices[state.producer_node as usize];
            remap(&mut state.initializer);
        }
        for output in &mut self.outputs {
            remap(&mut output.source);
        }
        for binding in self.bindings.values_mut() {
            if let PendingBinding::Value(value) = binding {
                remap(value);
            }
        }
        for arm in &mut self.match_arms {
            arm.node = indices[arm.node as usize];
        }
        for qualifier in &mut self.comprehension_qualifiers {
            qualifier.node = indices[qualifier.node as usize];
        }
    }

    fn current_state_value(&self, state: u32) -> PendingValue {
        let writer = self.states[state as usize].producer_node as usize;
        self.nodes[writer].inputs[0]
    }

    pub(super) fn document_assignment(
        &mut self,
        syntax: &SyntaxNode,
    ) -> Result<(PendingValue, SyntaxNode), SourceSemanticError> {
        let (target, expression, operation) = match syntax.kind() {
            SyntaxKind::OpAssign => {
                let assignment = OpAssignSyntax::cast(syntax.clone()).unwrap();
                let operator =
                    self.required(assignment.operator(), syntax, "an assignment operator")?;
                let selected = self.required(
                    operator.selected(),
                    operator.syntax(),
                    "an assignment primitive",
                )?;
                let operation = match self.required(
                    selected.semantic(),
                    selected.syntax(),
                    "assignment semantics",
                )? {
                    CanonicalOpAssign::Add => "math/add",
                    CanonicalOpAssign::Sub => "math/sub",
                    CanonicalOpAssign::Mul => "math/mul",
                    CanonicalOpAssign::Div => "math/div",
                    CanonicalOpAssign::Exp => "math/pow",
                };
                (
                    self.required(assignment.target(), syntax, "an assignment target")?,
                    self.required(assignment.value(), syntax, "an assignment value")?,
                    Some(operation),
                )
            }
            SyntaxKind::VariableAssign => {
                let assignment = VariableAssignSyntax::cast(syntax.clone()).unwrap();
                (
                    self.required(assignment.target(), syntax, "an assignment target")?,
                    self.required(assignment.value(), syntax, "an assignment value")?,
                    None,
                )
            }
            _ => unreachable!("the document collector selects assignment statements"),
        };
        let state = self.assignment_state(&target)?;
        let expected = self.schema_draft_of(PendingValue::State(state));
        let mut value = self.expression(&expression)?.0;
        if let Some(operation) = operation {
            let current = self.current_state_value(state);
            let Some((inputs, schema)) =
                self.resolve_maintained_call(operation, vec![current, value], syntax)?
            else {
                return Err(internal(
                    SourceSemanticAnchor::for_node(syntax),
                    format!("assignment operation {operation} has no maintained type declaration"),
                ));
            };
            value = self.emit_with_schema_draft(
                operation,
                inputs,
                schema,
                syntax,
                "state-update",
                None,
            );
        }
        value = self.conform_schema_draft(
            value,
            &expected,
            expression.syntax(),
            "source-semantics/incompatible-assignment-kind",
            "assignment value does not satisfy the mutable binding's schema",
        )?;
        let writer = self.states[state as usize].producer_node as usize;
        self.nodes[writer].inputs[0] = value;
        Ok((value, syntax.clone()))
    }

    fn assignment_state(&self, target: &SliceRefSyntax) -> Result<u32, SourceSemanticError> {
        if let Some(subscripts) = target.subscripts() {
            return Err(SourceSemanticError {
                code: "source-semantics/unsupported-assignment-target",
                message: "indexed assignment requires a maintained update operation".to_owned(),
                anchor: SourceSemanticAnchor::for_node(subscripts.syntax()),
            });
        }
        let stem = self.required(target.stem(), target.syntax(), "an assignment target stem")?;
        let name = node_text(stem.syntax())?;
        match self.bindings.get(&name) {
            Some(PendingBinding::MutableState(state)) => Ok(*state),
            Some(_) => Err(SourceSemanticError {
                code: "source-semantics/immutable-assignment-target",
                message: format!("assignment target {name} is immutable"),
                anchor: SourceSemanticAnchor::for_node(stem.syntax()),
            }),
            None => Err(SourceSemanticError {
                code: "source-semantics/unknown-assignment-target",
                message: format!("assignment target {name} has no preceding mutable definition"),
                anchor: SourceSemanticAnchor::for_node(stem.syntax()),
            }),
        }
    }
}
