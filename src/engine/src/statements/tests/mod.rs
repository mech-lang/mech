#[cfg(all(
    test,
    feature = "program",
    feature = "functions",
    feature = "variables",
    feature = "variable_define",
    feature = "variable_assign",
    feature = "f64",
    feature = "math_add",
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
    feature = "math_add",
    feature = "assign"
))]
mod activation_scope;

#[cfg(all(
    test,
    feature = "functions",
    feature = "variables",
    feature = "variable_define",
    feature = "f64",
    feature = "program",
    feature = "semantic-compiler",
))]
mod context;

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
    feature = "semantic-compiler",
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
    feature = "semantic-compiler",
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
    feature = "semantic-compiler",
))]
mod op_assign;
