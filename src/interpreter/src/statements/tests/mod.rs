#[cfg(all(test, feature = "program", feature = "functions", feature = "variables", feature = "variable_define", feature = "variable_assign", feature = "f64", feature = "math", feature = "assign"))]
mod scheduling;

#[cfg(all(
    test,
    feature = "program",
    feature = "functions",
    feature = "variables",
    feature = "variable_define",
    feature = "variable_assign",
    feature = "f64",
    feature = "math",
    feature = "assign"
))]
mod activation_scope;
