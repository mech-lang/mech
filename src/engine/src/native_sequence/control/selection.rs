//! Canonical selections and branch-local value updates for native FSMs.
//!
//! Indexed writes produce a new sequence value. They never mutate an input,
//! captured binding, or the value from which a branch-local binding was copied.

use super::{ControlKind, Expr, Lower, Op, native_kind, text};
use mech_syntax::document::{
    AstNode, SliceStemSyntax, SliceSyntax, SubscriptItemSyntax, SubscriptValueSyntax, SyntaxKind,
    SyntaxNode, VariableAssignSyntax, VariableDefineSyntax, VariableStemSyntax,
};
use std::collections::{BTreeMap, BTreeSet};

impl Lower<'_> {
    pub(super) fn selection(
        &mut self,
        slice: &SliceSyntax,
        vars: &BTreeMap<String, Expr>,
    ) -> Result<Expr, String> {
        let stem = slice.stem().ok_or("selection stem")?;
        let mut value = self.expr(stem.syntax(), vars)?;
        let items = slice.subscripts().ok_or("selection selectors")?.items();
        if items.is_empty() {
            return Err("native control selection requires a selector".into());
        }
        for item in items {
            match &item {
                SubscriptItemSyntax::Dot(dot) => {
                    let name = text(dot.identifier().ok_or("record field name")?.syntax())?;
                    let ControlKind::Record(fields) = &value.kind else {
                        return Err("dot field requires record".into());
                    };
                    let index = fields
                        .iter()
                        .position(|(field, _)| field == &name)
                        .ok_or("unknown record field")?;
                    let kind = fields[index].1.clone();
                    value = Expr {
                        kind,
                        op: Op::Field(Box::new(value), index),
                    };
                }
                SubscriptItemSyntax::Bracket(_) => {
                    let index = self.scalar_index(&item, vars)?;
                    // Keep the source index one-based. The shared sequence/get
                    // kernels own zero/out-of-range checks in every backend.
                    value = self.primitive("sequence/get".into(), vec![value, index])?;
                }
                _ => {
                    return Err(
                        "native control selections require record dot fields or one scalar bracket index"
                            .into(),
                    );
                }
            }
        }
        Ok(value)
    }

    fn scalar_index(
        &mut self,
        item: &SubscriptItemSyntax,
        vars: &BTreeMap<String, Expr>,
    ) -> Result<Expr, String> {
        let SubscriptItemSyntax::Bracket(bracket) = item else {
            return Err("native control indexing requires one scalar bracket index".into());
        };
        let values = bracket.values();
        let [SubscriptValueSyntax::Formula(formula)] = values.as_slice() else {
            return Err("native control indexing requires one scalar formula index".into());
        };
        let index = self.expr(formula.formula().ok_or("index formula")?.syntax(), vars)?;
        if index.kind != ControlKind::Integer {
            return Err("native control index must be u64".into());
        }
        Ok(index)
    }

    /// Lower a statement in one FSM branch's lexical environment.
    ///
    /// Callers create a fresh mutable-name set for each branch and discard it
    /// with that branch's environment. Only a local `~name := value` enters the
    /// set; input payloads and captures remain immutable.
    pub(super) fn branch_statement(
        &mut self,
        node: &SyntaxNode,
        vars: &mut BTreeMap<String, Expr>,
        mutable: &mut BTreeSet<String>,
    ) -> Result<(), String> {
        if let Some(definition) = VariableDefineSyntax::cast(node.clone())
            .or_else(|| node.children().find_map(VariableDefineSyntax::cast))
        {
            let is_mutable = definition.mutability_marker().is_some();
            // Ordinary canonical statements retain the existing immutable
            // definition path. Direct definition nodes are accepted as well,
            // since FSM block visitors can expose either typed representation.
            if !is_mutable && node.kind() != SyntaxKind::VariableDefine {
                return self.statement(node, vars);
            }
            let variable = definition.variable().ok_or("branch definition variable")?;
            let Some(VariableStemSyntax::Identifier(identifier)) = variable.stem() else {
                return Err("native FSM branch bindings require local identifiers".into());
            };
            let name = text(identifier.syntax())?;
            if vars.contains_key(&name) || mutable.contains(&name) {
                return Err("duplicate branch definition".into());
            }
            let mut value = self.expr(
                definition
                    .value()
                    .ok_or("branch definition value")?
                    .syntax(),
                vars,
            )?;
            if let Some(annotation) = variable.annotation() {
                let kind = native_kind(annotation.syntax(), self.aliases, &mut BTreeSet::new())?;
                value = self
                    .coerce(value, &kind)
                    .map_err(|error| format!("variable annotation mismatch: {error}"))?;
            }
            vars.insert(name.clone(), value);
            if is_mutable {
                mutable.insert(name);
            }
            return Ok(());
        }

        if let Some(assignment) = VariableAssignSyntax::cast(node.clone())
            .or_else(|| node.children().find_map(VariableAssignSyntax::cast))
        {
            let target = assignment.target().ok_or("branch assignment target")?;
            let Some(SliceStemSyntax::Identifier(identifier)) = target.stem() else {
                return Err("native FSM branch writes require local identifiers".into());
            };
            let name = text(identifier.syntax())?;
            if !mutable.contains(&name) {
                return Err(format!(
                    "native FSM branch assignment requires a branch-local mutable binding: {name}"
                ));
            }
            let previous = vars
                .get(&name)
                .cloned()
                .ok_or("mutable binding is outside this FSM branch")?;
            let selectors = target.subscripts().map(|s| s.items()).unwrap_or_default();
            let index = match selectors.as_slice() {
                [] => None,
                [item @ SubscriptItemSyntax::Bracket(_)] => Some(self.scalar_index(item, vars)?),
                _ => {
                    return Err(
                        "native FSM branch writes require a local value or one scalar bracket index"
                            .into(),
                    );
                }
            };
            let value = self.expr(
                assignment
                    .value()
                    .ok_or("branch assignment value")?
                    .syntax(),
                vars,
            )?;
            let replacement = if let Some(index) = index {
                // sequence/set returns a copy and preserves the established
                // one-based index and byte-value guards in native/resident code.
                self.primitive("sequence/set".into(), vec![previous.clone(), index, value])?
            } else {
                value
            };
            if replacement.kind != previous.kind {
                return Err("native FSM branch assignment must preserve the binding kind".into());
            }
            vars.insert(name, replacement);
            return Ok(());
        }

        self.statement(node, vars)
    }
}
