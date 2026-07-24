#[derive(Clone, Debug, PartialEq)]
pub(super) struct ConfigProgram {
    pub(super) items: Vec<ConfigItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum ConfigItem {
    Let(ConfigLet),
    Function(ConfigFunction),
    Expr(ConfigExpr),
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ConfigLet {
    pub(super) name: String,
    pub(super) expr: ConfigExpr,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ConfigFunction {
    pub(super) name: String,
    pub(super) params: Vec<String>,
    pub(super) body: ConfigExpr,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum ConfigExpr {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Atom(String),

    List(Vec<ConfigExpr>),
    Map(Vec<(String, ConfigExpr)>),

    Var(String),

    Call { name: String, args: Vec<ConfigExpr> },

    Add(Box<ConfigExpr>, Box<ConfigExpr>),
    Sub(Box<ConfigExpr>, Box<ConfigExpr>),
    Negate(Box<ConfigExpr>),
    Not(Box<ConfigExpr>),
}
