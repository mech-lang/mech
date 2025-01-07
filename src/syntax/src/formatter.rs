use mech_core::*;
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
    self.program(tree,"".to_string())
  }

  pub fn program(&mut self, node: &Program, src: String) -> String {
    let src = self.title(&node.title, src);
    self.body(&node.body, src)
  }

  pub fn title(&mut self, node: &Option<Title>, src: String) -> String {
    if let Some(title) = node {
      format!("{}{}\n===============================================================================\n\n",src,title.to_string())
    } else {
      format!("{}",src)
    }
  }

  pub fn subtitle(&mut self, node: &Option<Subtitle>, src: String) -> String {
    if let Some(title) = node {
      format!("{}{}\n-------------------------------------------------------------------------------\n\n",src,title.to_string())
    } else {
      format!("{}",src)
    }
  }

  pub fn body(&mut self, node: &Body, src: String) -> String {
    let mut src = src.clone();
    for section in &node.sections {
      src = self.section(section, src);
    }
    src
  }

  pub fn section(&mut self, node: &Section, src: String) -> String {
    let mut src = self.subtitle(&node.subtitle, src);
    for el in &node.elements {
      src = self.section_element(el, src);
      src = format!("{}\n",src);
    }
    src
  }

  pub fn section_element(&mut self, node: &SectionElement, src: String) -> String {
    match node {
      SectionElement::Section(n) => todo!(),
      SectionElement::Comment(n) => todo!(),
      SectionElement::Paragraph(n) => todo!(),
      SectionElement::MechCode(n) => self.mech_code(n, src),
      SectionElement::UnorderedList(n) => todo!(),
      SectionElement::CodeBlock => todo!(),
      SectionElement::OrderedList => todo!(),
      SectionElement::BlockQuote => todo!(),
      SectionElement::ThematicBreak => todo!(),
      SectionElement::Image => todo!(),
    }
  }

  pub fn mech_code(&mut self, node: &MechCode, src: String) -> String {
    match node {
      MechCode::Expression(expr) => self.expression(expr,src),
      _ => todo!(),
    }
  }

  pub fn expression(&mut self, node: &Expression, src: String) -> String {
    match node {
      //Expression::Var(var) => self.var(var, src),
      Expression::Formula(factor) => self.factor(factor, src),
      Expression::Literal(literal) => self.literal(literal, src),
      _ => todo!(),
      /*Expression::Range(range) => self.range(range, src),
      Expression::Slice(slice) => self.slice(slice, src),
      Expression::Structure(structure) => self.structure(structure, src),
      Expression::FunctionCall(function_call) => self.function_call(function_call, src),
      Expression::FsmPipe(fsm_pipe) => self.fsm_pipe(fsm_pipe, src),*/
    }
  }

  pub fn factor(&mut self, node: &Factor, src: String) -> String {
    match node {
      Factor::Term(term) => self.term(term, src),
      Factor::Expression(expr) => self.expression(expr, src),
      _ => todo!(),
      /*Factor::Negate(factor) => self.negate(factor, src),
      Factor::Not(factor) => self.not(factor, src),
      Factor::Transpose(factor) => self.transpose(factor, src),*/
    }
  }

  pub fn term(&mut self, node: &Term, src: String) -> String {
    let mut src = self.factor(&node.lhs, src);
    for (formula_operator, rhs) in &node.rhs {
      src = self.formula_operator(formula_operator, src);
      src = self.factor(rhs, src);
    }
    src
  }

  pub fn formula_operator(&mut self, node: &FormulaOperator, src: String) -> String {
    match node {
      FormulaOperator::AddSub(op) => self.add_sub_op(op, src),
      _ => todo!(),
      //FormulaOperator::MulDiv(op) => self.mul_div_op(op, src),
      //FormulaOperator::Exponent(op) => self.exponent_op(op, src),
      //FormulaOperator::Vec(op) => self.vec_op(op, src),
      //FormulaOperator::Comparison(op) => self.comparison_op(op, src),
      //FormulaOperator::Logic(op) => self.logic_op(op, src),
    }
  }

  pub fn add_sub_op(&mut self, node: &AddSubOp, src: String) -> String {
    let op = match node {
      AddSubOp::Add => "+".to_string(),
      AddSubOp::Sub => "-".to_string(),
    };
    format!("{} {} ",src,op)
  }

  pub fn literal(&mut self, node: &Literal, src: String) -> String {
    match node {
      Literal::Empty(token) => format!("{}_",src),
      Literal::Boolean(token) => format!("{}{}", src, token.to_string()),
      Literal::Number(number) => self.number(number, src),
      Literal::String(mech_string) => self.string(mech_string, src),
      _ => todo!(),
      /*
      Literal::Atom(atom) => self.atom(atom, src),
      Literal::TypedLiteral((boxed_literal, kind_annotation)) => {
        let mut src = self.literal(boxed_literal, src);
        self.kind_annotation(kind_annotation, src)
      }*/
    }
  }

  pub fn string(&mut self, node: &MechString, src: String) -> String {
    format!("{}\"{}\"", src, node.text.to_string())
  }

  pub fn number(&mut self, node: &Number, src: String) -> String {
    match node {
      Number::Real(real) => self.real_number(real, src),
      Number::Imaginary(complex) => self.complex_numer(complex, src),
    }
  }

  pub fn real_number(&mut self, node: &RealNumber, src: String) -> String {
    match node {
      RealNumber::Negated(real_number) => format!("-{}", self.real_number(real_number, src)),
      RealNumber::Integer(token) => format!("{}{}", src, token.to_string()),
      RealNumber::Float((whole, part)) => format!("{}{}.{}", src, whole.to_string(), part.to_string()),
      RealNumber::Decimal(token) => format!("{}{}", src, token.to_string()),
      RealNumber::Hexadecimal(token) => format!("{}0x{}", src, token.to_string()),
      RealNumber::Octal(token) => format!("{}0o{}", src, token.to_string()),
      RealNumber::Binary(token) => format!("{}0b{}", src, token.to_string()),
      RealNumber::Scientific(((whole, part), (sign, ewhole, epart))) => format!("{}{}.{}e{}{}.{}", src, whole.to_string(), part.to_string(), if *sign { "-" } else { "+" }, ewhole.to_string(), epart.to_string()),
      RealNumber::Rational((numerator, denominator)) => format!("{}{}/{}", src, numerator.to_string(), denominator.to_string()),
    }
  }

  pub fn complex_numer(&mut self, node: &ComplexNumber, src: String) -> String {
    let src = if let Some(real) = &node.real {
      let src = self.real_number(&real, src);
      format!("{}i", src)
    } else {
      "".to_string()
    };
    self.imaginary_number(&node.imaginary, src)
  }

  pub fn imaginary_number(&mut self, node: &ImaginaryNumber, src: String) -> String {
    let formatted_real = self.real_number(&node.number, "".to_string());
    format!("{}{}i", src, formatted_real)
  }
  

}