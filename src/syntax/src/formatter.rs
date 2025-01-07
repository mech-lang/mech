use mech_core::*;
use mech_core::nodes::Kind;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Formatter{
  code: String,
  identifiers: HashMap<u64, String>,
  rows: usize,
  cols: usize,
  indent: usize,
  html: bool,
  nested: bool,
}

impl Formatter {

  pub fn new() -> Formatter {
    Formatter {
      code: String::new(),
      identifiers: HashMap::new(),
      rows: 0,
      cols: 0,
      indent: 0,
      html: false,
      nested: false,
    }
  }

  pub fn format(&mut self, tree: &Program) -> String {
    self.html = false;
    self.program(tree)
  }

  pub fn program(&mut self, node: &Program) -> String {
    let title = match &node.title {
      Some(title) => self.title(&title),
      None => "".to_string(),
    };
    let body = self.body(&node.body);
    format!("{}{}",title,body)
  }

  pub fn title(&mut self, node: &Title) -> String {
    format!("{}\n===============================================================================\n",node.to_string())
  }

  pub fn subtitle(&mut self, node: &Subtitle) -> String {
    format!("{}\n-------------------------------------------------------------------------------\n",node.to_string())
  }

  pub fn body(&mut self, node: &Body) -> String {
    let mut src = "".to_string();
    let section_count = node.sections.len();
    for (i, section) in node.sections.iter().enumerate() {
      let s = self.section(section);
      src = format!("{}{}", src, s);
    }
    src
  }

  pub fn section(&mut self, node: &Section) -> String {
    let mut src = match &node.subtitle {
      Some(title) => self.subtitle(title),
      None => "".to_string(),
    };
    for (i, el) in node.elements.iter().enumerate() {
      let el_str = self.section_element(el);
      src = format!("{}{}", src, el_str);
    }
    src
  }

  pub fn section_element(&mut self, node: &SectionElement) -> String {
    match node {
      SectionElement::Section(n) => todo!(),
      SectionElement::Comment(n) => todo!(),
      SectionElement::Paragraph(n) => n.to_string(),
      SectionElement::MechCode(n) => self.mech_code(n),
      SectionElement::UnorderedList(n) => todo!(),
      SectionElement::CodeBlock => todo!(),
      SectionElement::OrderedList => todo!(),
      SectionElement::BlockQuote => todo!(),
      SectionElement::ThematicBreak => todo!(),
      SectionElement::Image => todo!(),
    }
  }

  pub fn mech_code(&mut self, node: &MechCode) -> String {
    let c = match node {
      MechCode::Expression(expr) => self.expression(expr),
      MechCode::Statement(stmt) => self.statement(stmt),
      _ => todo!(),
      //MechCode::FsmSpecification(fsm_spec) => self.fsm_specification(fsm_spec, src),
      //MechCode::FsmImplementation(fsm_impl) => self.fsm_implementation(fsm_impl, src),
      //MechCode::FunctionDefine(func_def) => self.function_define(func_def, src),
    };
    format!("{}\n", c)
  }

  pub fn variable_define(&mut self, node: &VariableDefine) -> String {
    let mut var_def = if node.mutable {
      "~".to_string()
    } else {
      "".to_string()
    };
    let var = self.var(&node.var);
    let expression = self.expression(&node.expression);
    format!("{}{} := {}", var_def, var, expression)
  }

  pub fn statement(&mut self, node: &Statement) -> String {
    match node {
      Statement::VariableDefine(var_def) => self.variable_define(var_def),
      _ => todo!(),
      //Statement::VariableAssign(var_asgn) => self.variable_assign(var_asgn, src),
      //Statement::OpAssign(op_asgn) => self.op_assign(op_asgn, src),
      //Statement::EnumDefine(enum_def) => self.enum_define(enum_def, src),
      //Statement::FsmDeclare(fsm_decl) => self.fsm_declare(fsm_decl, src),
      //Statement::KindDefine(kind_def) => self.kind_define(kind_def, src),
    }
  }

  pub fn expression(&mut self, node: &Expression) -> String {
    match node {
      Expression::Var(var) => self.var(var),
      Expression::Formula(factor) => self.factor(factor),
      Expression::Literal(literal) => self.literal(literal),
      _ => todo!(),
      /*Expression::Range(range) => self.range(range, src),
      Expression::Slice(slice) => self.slice(slice, src),
      Expression::Structure(structure) => self.structure(structure, src),
      Expression::FunctionCall(function_call) => self.function_call(function_call, src),
      Expression::FsmPipe(fsm_pipe) => self.fsm_pipe(fsm_pipe, src),*/
    }
  }

  pub fn var(&mut self, node: &Var) -> String {
    let annotation = if let Some(kind) = &node.kind {
      self.kind_annotation(kind)
    } else {
      "".to_string()
    };
    format!("{}{}", node.name.to_string(), annotation)
  }

  pub fn kind_annotation(&mut self, node: &KindAnnotation) -> String {
    let kind = self.kind(&node.kind);
    format!("<{}>", kind)
  }

  /*
  #[derive(Clone, Debug, Serialize, Deserialize,Eq, PartialEq)]
pub enum Kind {
  Tuple(Vec<Kind>),
  Bracket((Vec<Kind>,Vec<Literal>)),
  Brace((Vec<Kind>,Vec<Literal>)),
  Map(Box<Kind>,Box<Kind>),
  Scalar(Identifier),
  Atom(Identifier),
  Function(Vec<Kind>,Vec<Kind>),
  Fsm(Vec<Kind>,Vec<Kind>),
  Empty,
}*/

  pub fn kind(&mut self, node: &Kind) -> String {
    match node {
      Kind::Scalar(ident) => ident.to_string(),
      Kind::Empty => "_".to_string(),
      _ => todo!(),
    }
  }

  pub fn factor(&mut self, node: &Factor) -> String {
    match node {
      Factor::Term(term) => self.term(term),
      Factor::Expression(expr) => self.expression(expr),
      Factor::Parenthetical(paren) => format!("({})", self.factor(&paren)),
      _ => todo!(),
      /*Factor::Negate(factor) => self.negate(factor, src),
      Factor::Not(factor) => self.not(factor, src),
      Factor::Transpose(factor) => self.transpose(factor, src),*/
    }
  }

  pub fn term(&mut self, node: &Term) -> String {
    let mut src = self.factor(&node.lhs);
    for (formula_operator, rhs) in &node.rhs {
      let op = self.formula_operator(formula_operator);
      let rhs = self.factor(rhs);
      src = format!("{}{}{}", src, op, rhs);
    }
    src
  }

  pub fn formula_operator(&mut self, node: &FormulaOperator) -> String {
    match node {
      FormulaOperator::AddSub(op) => self.add_sub_op(op),
      FormulaOperator::MulDiv(op) => self.mul_div_op(op),
      FormulaOperator::Exponent(op) => self.exponent_op(op),
      FormulaOperator::Vec(op) => self.vec_op(op),
      FormulaOperator::Comparison(op) => self.comparison_op(op),
      FormulaOperator::Logic(op) => self.logic_op(op),
    }
  }

  pub fn add_sub_op(&mut self, node: &AddSubOp) -> String {
    let op = match node {
      AddSubOp::Add => "+".to_string(),
      AddSubOp::Sub => "-".to_string(),
    };
    format!(" {} ", op)
  }

  pub fn mul_div_op(&mut self, node: &MulDivOp) -> String {
    let op = match node {
      MulDivOp::Mul => "*".to_string(),
      MulDivOp::Div => "/".to_string(),
    };
    format!(" {} ", op)
  }

  pub fn exponent_op(&mut self, node: &ExponentOp) -> String {
    let op = match node {
      ExponentOp::Exp => "^".to_string(),
    };
    format!(" {} ", op)
  }

  pub fn vec_op(&mut self, node: &VecOp) -> String {
    let op = match node {
      VecOp::MatMul => "**".to_string(),
      VecOp::Solve => "\\".to_string(),
      VecOp::Cross => "×".to_string(),
      VecOp::Dot => "·".to_string(),
    };
    format!(" {} ", op)
  }

  pub fn comparison_op(&mut self, node: &ComparisonOp) -> String {
    let op = match node {
      ComparisonOp::Equal => "==".to_string(),
      ComparisonOp::NotEqual => "≠".to_string(),
      ComparisonOp::GreaterThan => ">".to_string(),
      ComparisonOp::GreaterThanEqual => "≥".to_string(),
      ComparisonOp::LessThan => "<".to_string(),
      ComparisonOp::LessThanEqual => "≤".to_string(),
    };
    format!(" {} ", op)
  }

  pub fn logic_op(&mut self, node: &LogicOp) -> String {
    let op = match node {
      LogicOp::And => "&".to_string(),
      LogicOp::Or => "|".to_string(),
      LogicOp::Xor => "xor".to_string(),
      LogicOp::Not => "¬".to_string(),
    };
    format!(" {} ", op)
  }

  pub fn literal(&mut self, node: &Literal) -> String {
    match node {
      Literal::Empty(token) => "_".to_string(),
      Literal::Boolean(token) => token.to_string(),
      Literal::Number(number) => self.number(number),
      Literal::String(mech_string) => self.string(mech_string),
      Literal::Atom(atom) => self.atom(atom),
      Literal::TypedLiteral((boxed_literal, kind_annotation)) => {
        let literal = self.literal(boxed_literal);
        let annotation = self.kind_annotation(kind_annotation);
        format!("{}{}", literal, annotation)
      }
    }
  }

  pub fn atom(&mut self, node: &Atom) -> String {
    format!("`{}", node.name.to_string())
  }

  pub fn string(&mut self, node: &MechString) -> String {
    format!("\"{}\"", node.text.to_string())
  }

  pub fn number(&mut self, node: &Number) -> String {
    match node {
      Number::Real(real) => self.real_number(real),
      Number::Imaginary(complex) => self.complex_numer(complex),
    }
  }

  pub fn real_number(&mut self, node: &RealNumber) -> String {
    match node {
      RealNumber::Negated(real_number) => format!("-{}", self.real_number(real_number)),
      RealNumber::Integer(token) => token.to_string(),
      RealNumber::Float((whole, part)) => format!("{}.{}", whole.to_string(), part.to_string()),
      RealNumber::Decimal(token) => token.to_string(),
      RealNumber::Hexadecimal(token) => format!("0x{}", token.to_string()),
      RealNumber::Octal(token) => format!("0o{}", token.to_string()),
      RealNumber::Binary(token) => format!("0b{}", token.to_string()),
      RealNumber::Scientific(((whole, part), (sign, ewhole, epart))) => format!("{}.{}e{}{}.{}", whole.to_string(), part.to_string(), if *sign { "-" } else { "+" }, ewhole.to_string(), epart.to_string()),
      RealNumber::Rational((numerator, denominator)) => format!("{}/{}", numerator.to_string(), denominator.to_string()),
    }
  }

  pub fn complex_numer(&mut self, node: &ComplexNumber) -> String {
    let real = if let Some(real) = &node.real {
      let num = self.real_number(&real);
      format!("{}+", num)
    } else {
      "".to_string()
    };
    let im = self.imaginary_number(&node.imaginary);
    format!("{}{}", real, im)
  }

  pub fn imaginary_number(&mut self, node: &ImaginaryNumber) -> String {
    let real = self.real_number(&node.number);
    format!("{}i", real)
  }
  

}