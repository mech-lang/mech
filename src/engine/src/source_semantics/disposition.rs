/// The way one canonical Phase 2I production participates in source semantics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase2iSemanticDisposition {
    /// Produces an executable source value or operation.
    Executable,
    /// Contributes ordered structure to an enclosing executable form.
    Structural,
    /// Is resolved during semantic construction and emits no runtime node.
    CompileTime,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Phase2iSemanticRule {
    pub grammar_name: &'static str,
    pub disposition: Phase2iSemanticDisposition,
}

use Phase2iSemanticDisposition::{CompileTime, Executable, Structural};

/// Closed semantic policy for the generated 80-rule recursive component.
///
/// Keep this in the same lexical order as `phase-2i-syntax-schema.tsv` so the
/// certification gate can compare the two authorities without heuristics.
pub const PHASE_2I_SEMANTIC_RULES: [Phase2iSemanticRule; 80] = [
    rule("argument-list", Structural),
    rule("binding", Structural),
    rule("brace-subscript", Structural),
    rule("bracket-subscript", Structural),
    rule("call-arg", Structural),
    rule("call-arg-with-binding", Structural),
    rule("comprehension-qualifier", Structural),
    rule("expression", Executable),
    rule("factor", Executable),
    rule("fancy-table", Executable),
    rule("fancy-table-header", Structural),
    rule("field", CompileTime),
    rule("formula", Executable),
    rule("formula-subscript", Structural),
    rule("fsm-args", Structural),
    rule("fsm-async-transition", Structural),
    rule("fsm-instance", Structural),
    rule("fsm-output", Structural),
    rule("fsm-pipe", Executable),
    rule("fsm-state-transition", Structural),
    rule("fsm-value", Structural),
    rule("function-call", Executable),
    rule("generator", Structural),
    rule("header-field", CompileTime),
    rule("inline-table", Executable),
    rule("inline-table-header", Structural),
    rule("inline-table-row", Structural),
    rule("kind", CompileTime),
    rule("kind-annotation", CompileTime),
    rule("kind-kind", CompileTime),
    rule("kind-map", CompileTime),
    rule("kind-matrix", CompileTime),
    rule("kind-record", CompileTime),
    rule("kind-scalar", CompileTime),
    rule("kind-set", CompileTime),
    rule("kind-table", CompileTime),
    rule("kind-tuple", CompileTime),
    rule("kind-with-option", CompileTime),
    rule("l1", Executable),
    rule("l2", Executable),
    rule("l3", Executable),
    rule("l4", Executable),
    rule("l5", Executable),
    rule("l6", Executable),
    rule("l7", Executable),
    rule("literal", Executable),
    rule("map", Executable),
    rule("mapping", Structural),
    rule("match-arm", Structural),
    rule("matrix", Executable),
    rule("matrix-column", Structural),
    rule("matrix-comprehension", Executable),
    rule("matrix-row", Structural),
    rule("negate-factor", Executable),
    rule("not-factor", Executable),
    rule("parenthetical-term", Executable),
    rule("pattern", CompileTime),
    rule("pattern-array", CompileTime),
    rule("pattern-array-item", CompileTime),
    rule("pattern-array-token", CompileTime),
    rule("pattern-atom-struct", CompileTime),
    rule("pattern-tuple", CompileTime),
    rule("pattern-tuple-struct", CompileTime),
    rule("range-expression", Executable),
    rule("range-subscript", Structural),
    rule("record", Executable),
    rule("regular-table", Executable),
    rule("set", Executable),
    rule("set-comprehension", Executable),
    rule("slice", Executable),
    rule("structure", Executable),
    rule("subscript", Structural),
    rule("table", Executable),
    rule("table-header", Structural),
    rule("table-row", Structural),
    rule("table-row2", Structural),
    rule("tuple", Executable),
    rule("tuple-struct", Executable),
    rule("var", Executable),
    rule("variable-define", Structural),
];

const fn rule(
    grammar_name: &'static str,
    disposition: Phase2iSemanticDisposition,
) -> Phase2iSemanticRule {
    Phase2iSemanticRule {
        grammar_name,
        disposition,
    }
}

pub fn phase_2i_semantic_disposition(grammar_name: &str) -> Option<Phase2iSemanticDisposition> {
    PHASE_2I_SEMANTIC_RULES
        .iter()
        .find(|rule| rule.grammar_name == grammar_name)
        .map(|rule| rule.disposition)
}
