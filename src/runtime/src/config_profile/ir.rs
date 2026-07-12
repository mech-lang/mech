#[derive(Clone, Debug, PartialEq)]
pub struct ConfigProgram {
    pub items: Vec<ConfigItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ConfigItem {
    Let(ConfigLet),
    Function(ConfigFunction),
    Expr(ConfigExpr),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConfigLet {
    pub name: String,
    pub expr: ConfigExpr,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConfigFunction {
    pub name: String,
    pub params: Vec<String>,
    pub body: ConfigExpr,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ConfigExpr {
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
