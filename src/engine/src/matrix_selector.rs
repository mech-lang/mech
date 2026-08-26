use crate::LegacyValue;

/// Short-lived source-compiler control for matrix specialization.
///
/// `All` represents the source-level `:` selector. Every evaluated selector
/// remains a value until the narrow legacy specialization boundary consumes it.
#[derive(Clone, Debug)]
pub(crate) enum MatrixSelector {
    All,
    #[expect(
        dead_code,
        reason = "wired to evaluated selectors in the next stack commit"
    )]
    Value(LegacyValue),
}
