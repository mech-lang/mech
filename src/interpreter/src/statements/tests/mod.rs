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

#[cfg(all(
    test,
    feature = "variable_define",
    feature = "f64",
    feature = "string",
    feature = "bool",
))]
mod variable_define;

#[cfg(all(
    test,
    feature = "functions",
    feature = "variables",
    feature = "variable_define",
    feature = "variable_assign",
    feature = "assign",
    feature = "f64",
    feature = "program",
    feature = "compiler",
))]
mod support;

#[cfg(all(
    test,
    feature = "functions",
    feature = "variables",
    feature = "variable_define",
    feature = "variable_assign",
    feature = "assign",
    feature = "f64",
    feature = "program",
    feature = "compiler",
))]
mod variable_assign;

#[cfg(all(
    test,
    feature = "functions",
    feature = "variables",
    feature = "variable_define",
    feature = "variable_assign",
    feature = "assign",
    feature = "f64",
    feature = "program",
    feature = "compiler",
))]
mod op_assign;
