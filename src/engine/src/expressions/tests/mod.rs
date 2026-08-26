#[cfg(all(
    any(feature = "matrix_comprehensions", feature = "set_comprehensions"),
    feature = "functions"
))]
mod comprehensions;

#[cfg(all(feature = "functions", feature = "f64"))]
mod registration;

#[cfg(all(
    feature = "functions",
    feature = "record",
    feature = "tuple",
    feature = "f64",
    feature = "program",
    feature = "semantic-compiler"
))]
mod structural_access;

#[cfg(all(
    feature = "access",
    feature = "bool",
    feature = "f64",
    feature = "logical_indexing",
    feature = "matrix",
    feature = "matrixd",
    feature = "subscript_formula",
    feature = "subscript_range",
    feature = "subscript_slice"
))]
mod matrix_selection;

#[cfg(all(
    feature = "functions",
    feature = "f64",
    feature = "u64",
    feature = "convert",
    feature = "kind_annotation",
    feature = "variable_define",
    feature = "variables"
))]
mod variables;
