use mech_core::*;
use mech_core::nodes::{Kind, Matrix};
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

  pub fn format_html(&mut self, tree: &Program) -> String {
    self.html = true;
    self.program(tree)
  }

  pub fn program(&mut self, node: &Program) -> String {
    let title = match &node.title {
      Some(title) => self.title(&title),
      None => "".to_string(),
    };
    let body = self.body(&node.body);
    if self.html {
      format!("<div class=\"mech-program\">{}{}</div>",title,body)
    } else {
      format!("{}{}",title,body)
    }
  }

  pub fn title(&mut self, node: &Title) -> String {
    if self.html {
      format!("<h1 class=\"mech-program-title\">{}</h1>",node.to_string())
    } else {
      format!("{}\n===============================================================================\n",node.to_string()) 
    }
  }

  pub fn subtitle(&mut self, node: &Subtitle) -> String {
    if self.html {
      format!("<h2 class=\"mech-program-subtitle\">{}</h2>",node.to_string())
    } else {
      format!("{}\n-------------------------------------------------------------------------------\n",node.to_string())
    }
  }

  pub fn body(&mut self, node: &Body) -> String {
    let mut src = "".to_string();
    let section_count = node.sections.len();
    for (i, section) in node.sections.iter().enumerate() {
      let s = self.section(section);
      src = format!("{}{}", src, s);
    }
    if self.html {
      format!("<div class=\"mech-program-body\">{}</div>",src)
    } else {
      src
    }
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
    if self.html {
      format!("<section class=\"mech-program-section\">{}</section>",src)
    } else {
      src
    }
  }

  pub fn paragraph(&mut self, node: &Paragraph) -> String {
    if self.html {
      format!("<p class=\"mech-paragraph\">{}</p>",node.to_string())
    } else {
      format!("{}\n",node.to_string())
    }
  }

  pub fn section_element(&mut self, node: &SectionElement) -> String {
    let element = match node {
      SectionElement::Section(n) => self.section(n),
      SectionElement::Comment(n) => todo!(),
      SectionElement::Paragraph(n) => self.paragraph(n),
      SectionElement::MechCode(n) => self.mech_code(n),
      SectionElement::UnorderedList(n) => self.unordered_list(n),
      SectionElement::CodeBlock => todo!(),
      SectionElement::OrderedList => todo!(),
      SectionElement::BlockQuote => todo!(),
      SectionElement::ThematicBreak => todo!(),
      SectionElement::Image => todo!(),
    };
    if self.html {
      format!("<div class=\"mech-section-element\">{}</div>",element)
    } else {
      element
    }
  }

  pub fn unordered_list(&mut self, node: &UnorderedList) -> String {
    let mut lis = "".to_string();
    for (i, item) in node.items.iter().enumerate() {
      let it = self.paragraph(item);
      if self.html {
        lis = format!("{}<li class=\"mech-list-item\">{}</li>",lis,it);
      } else {
        lis = format!("{}- {}\n",lis,it); 
      }
    }
    if self.html {
      format!("<ul class=\"mech-unordered-list\">{}</ul>",lis)
    } else {
      lis
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
    if self.html {
      format!("<div class=\"mech-code\">{}</div>",c)
    } else {
      format!("{}\n", c)
    }
  }

  pub fn variable_define(&mut self, node: &VariableDefine) -> String {
    let mut mutable = if node.mutable {
      "~".to_string()
    } else {
      "".to_string()
    };
    let var = self.var(&node.var);
    let expression = self.expression(&node.expression);
    if self.html {
      format!("<span class=\"mech-variable-define\"><span class=\"mech-variable-mutable\">{}</span>{}<span class=\"mech-variable-assign-op\">:=</span>{}</span>",mutable, var, expression)
    } else {
      format!("{}{} := {}", mutable, var, expression)
    }
  }

  pub fn statement(&mut self, node: &Statement) -> String {
    let s = match node {
      Statement::VariableDefine(var_def) => self.variable_define(var_def),
      Statement::OpAssign(op_asgn) => self.op_assign(op_asgn),
      _ => todo!(),
      //Statement::VariableAssign(var_asgn) => self.variable_assign(var_asgn, src),
      //Statement::EnumDefine(enum_def) => self.enum_define(enum_def, src),
      //Statement::FsmDeclare(fsm_decl) => self.fsm_declare(fsm_decl, src),
      //Statement::KindDefine(kind_def) => self.kind_define(kind_def, src),
    };
    if self.html {
      format!("<span class=\"mech-statement\">{}</span>",s)
    } else {
      format!("{}", s)
    }
  }

  pub fn op_assign(&mut self, node: &OpAssign) -> String {
    let target = self.slice_ref(&node.target);
    let op = self.op_assign_op(&node.op);
    let expression = self.expression(&node.expression);
    if self.html {
      format!("<span class=\"mech-op-assign\"><span class=\"mech-target\">{}</span><span class=\"mech-op\">{}</span><span class=\"mech-expression\">{}</span></span>",target,op,expression)
    } else {
      format!("{} {} {}", target, op, expression)
    }
  }

  pub fn op_assign_op(&mut self, node: &OpAssignOp) -> String {
    let op = match node {
      OpAssignOp::Add => "+=".to_string(),
      OpAssignOp::Sub => "-=".to_string(),
      OpAssignOp::Mul => "*=".to_string(),
      OpAssignOp::Div => "/=".to_string(),
      OpAssignOp::Exp => "^=".to_string(),
    };
    if self.html {
      format!("<span class=\"mech-op-assign-op\">{}</span>",op)
    } else {
      format!("{}", op)
    }
  }

  pub fn slice_ref(&mut self, node: &SliceRef) -> String {
    let name = node.name.to_string();
    let mut subscript = "".to_string();
    match &node.subscript {
      Some(subs) => {
        for sub in subs.iter() {
          let s = self.subscript(sub);
          subscript = format!("{}{}", subscript, s);
        }
      },
      None => {},
    }
    if self.html {
      format!("<span class=\"mech-slice-ref\"><span id=\"{}\" class=\"mech-var-name\">{}</span><span class=\"mech-subscript\">{}</span></span>",hash_str(&name),name,subscript)
    } else {
      format!("{}{}", name, subscript)
    }
  }

  pub fn expression(&mut self, node: &Expression) -> String {
    let e = match node {
      Expression::Var(var) => self.var(var),
      Expression::Formula(factor) => self.factor(factor),
      Expression::Literal(literal) => self.literal(literal),
      Expression::Structure(structure) => self.structure(structure),
      Expression::Slice(slice) => self.slice(slice),
      Expression::FunctionCall(function_call) => self.function_call(function_call),
      _ => todo!(),
      /*Expression::Range(range) => self.range(range, src),
      Expression::FsmPipe(fsm_pipe) => self.fsm_pipe(fsm_pipe, src),*/
    };
    if self.html {
      format!("<span class=\"mech-expression\">{}</span>",e)
    } else {
      format!("{}", e)
    }
  }

  pub fn function_call(&mut self, node: &FunctionCall) -> String {
    let name = node.name.to_string();
    let mut args = "".to_string();
    for (i, arg) in node.args.iter().enumerate() {
      let a = self.argument(arg);
      if i == 0 {
        args = format!("{}", a);
      } else {
        args = format!("{}, {}", args, a);
      }
    }
    if self.html {
      format!("<span class=\"mech-function-call\"><span id=\"{}\" class=\"mech-function-name\">{}</span><span class=\"mech-left-paren\">(</span><span class=\"mech-argument-list\">{}</span><span class=\"mech-right-paren\">)</span></span>",hash_str(&name),name,args)
    } else {
      format!("{}({})", name, args)
    }
  }

  pub fn argument(&mut self, node: &(Option<Identifier>, Expression)) -> String {
    let (name, expr) = node;
    let n = match name {
      Some(ident) => ident.to_string(),
      None => "".to_string(),
    };
    let e = self.expression(expr);
    if self.html {
      format!("<span class=\"mech-argument\"><span class=\"mech-argument-name\">{}</span><span class=\"mech-argument-expression\">{}</span></span>",n,e)
    } else {
      format!("{}{}", n, e)
    }
  }

  pub fn slice(&mut self, node: &Slice) -> String {
    let name = node.name.to_string();
    let mut subscript = "".to_string();
    for (i, sub) in node.subscript.iter().enumerate() {
      let s = self.subscript(sub);
      subscript = format!("{}{}", subscript, s);
    }
    if self.html {
      format!("<span class=\"mech-slice\"><span id=\"{}\" class=\"mech-var-name\">{}</span><span class=\"mech-subscript\">{}</span></span>",hash_str(&name),name,subscript)
    } else {
      format!("{}{}", name, subscript)
    }
  }

  pub fn subscript(&mut self, node: &Subscript) -> String {
    match node {
      Subscript::Bracket(subs) => self.bracket(subs),
      Subscript::Formula(factor) => self.factor(factor),
      Subscript::All => self.all(),
      Subscript::Dot(ident) => self.dot(ident),
      _ => todo!(),
      //Subscript::Swizzle(idents) => self.swizzle(idents),
      //Subscript::Range(range) => self.range(range),
      //Subscript::Brace(subs) => self.brace(subs),
      //Subscript::DotInt(real) => self.dot_int(real),*/
    }
  }

  pub fn dot(&mut self, node: &Identifier) -> String {
    if self.html {
      format!(".<span class=\"mech-dot\">{}</span>",node.to_string())
    } else {
      format!(".{}",node.to_string())
    }
  }

  pub fn all(&mut self) -> String {
    if self.html {
      format!("<span class=\"mech-all\">:</span>")
    } else {
      ":".to_string()
    }
  }

  pub fn bracket(&mut self, node: &Vec<Subscript>) -> String {
    let mut src = "".to_string();
    for (i, sub) in node.iter().enumerate() {
      let s = self.subscript(sub);
      if i == 0 {
        src = format!("{}", s);
      } else {
        src = format!("{},{}", src, s);
      }
    }
    if self.html {
      format!("<span class=\"mech-bracket\">[{}]</span>",src)
    } else {
      format!("[{}]",src)
    }
  }

  pub fn structure(&mut self, node: &Structure) -> String {
    let s = match node {
      Structure::Matrix(matrix) => self.matrix(matrix),
      Structure::Record(record) => self.record(record),
      _ => todo!(),
      //Structure::Empty => "".to_string(),
      //Structure::Table(table) => self.table(table),
      //Structure::Tuple(tuple) => self.tuple(tuple),
      //Structure::TupleStruct(tuple_struct) => self.tuple_struct(tuple_struct),
      //Structure::Set(set) => self.set(set),
      //Structure::Map(map) => self.map(map),
    };
    if self.html {
      format!("<span class=\"mech-structure\">{}</span>",s)
    } else {
      format!("{}", s)
    }
  }

  pub fn record(&mut self, node: &Record) -> String {
    let mut src = "".to_string();
    for (i, binding) in node.bindings.iter().enumerate() {
      let b = self.binding(binding);
      if i == 0 {
        src = format!("{}", b);
      } else {
        src = format!("{}, {}", src, b);
      }
    }
    if self.html {
      format!("<span class=\"mech-record\"><span class=\"mech-start-brace\">{{</span>{}<span class=\"mech-end-brace\">}}</span></span>",src)
    } else {
      format!("{{{}}}",src)
    }
  }

  pub fn binding(&mut self, node: &Binding) -> String {
    let name = node.name.to_string();
    let value = self.expression(&node.value);
    if self.html {
      format!("<span class=\"mech-binding\"><span class=\"mech-binding-name\">{}</span><span class=\"mech-binding-colon-op\">:</span><span class=\"mech-binding-value\">{}</span></span>",name,value)
    } else {
      format!("{}: {}", name, value)
    }
  }

  pub fn matrix(&mut self, node: &Matrix) -> String {
    let mut src = "".to_string();
    let column_count = node.rows[0].columns.len(); // Assume all rows have the same number of columns

    for col_index in 0..column_count {
        let mut column_elements = Vec::new();
        for row in &node.rows {
            column_elements.push(&row.columns[col_index]);
        }
        let c = self.matrix_column_elements(&column_elements);

        if col_index == 0 {
            src = format!("{}", c);
        } else {
            src = format!("{} {}", src, c);
        }
    }

    if self.html {
        format!("<span class=\"mech-matrix\"><span class=\"mech-bracket start\">[</span>{}<span class=\"mech-bracket end\">]</span></span>", src)
    } else {
        format!("[{}]", src)
    }
}

pub fn matrix_column_elements(&mut self, column_elements: &[&MatrixColumn]) -> String {
    let mut src = "".to_string();
    for (i, cell) in column_elements.iter().enumerate() {
        let c = self.matrix_column(cell);
        if i == 0 {
            src = format!("{}", c);
        } else {
            src = format!("{} {}", src, c);
        }
    }
    if self.html {
        format!("<div class=\"mech-matrix-column\">{}</div>", src)
    } else {
        src
    }
}


  pub fn matrix_row(&mut self, node: &MatrixRow) -> String {
    let mut src = "".to_string();
    for (i, cell) in node.columns.iter().enumerate() {
      let c = self.matrix_column(cell);
      if i == 0 {
        src = format!("{}", c);
      } else { 
        src = format!("{} {}", src, c);
      }
    }
    if self.html {
      format!("<div class=\"mech-matrix-row\">{}</div>",src)
    } else {
      src
    }
  }

  pub fn matrix_column(&mut self, node: &MatrixColumn) -> String {
    let element = self.expression(&node.element);
    if self.html {
      format!("<span class=\"mech-matrix-element\">{}</span>",element)
    } else {
      element
    }    
  }  

  pub fn var(&mut self, node: &Var) -> String {
    let annotation = if let Some(kind) = &node.kind {
      self.kind_annotation(kind)
    } else {
      "".to_string()
    };
    if self.html {
      format!("<span class=\"mech-var-name\" id=\"{}\">{}</span><span class=\"mech-kind-annotation\">{}</span>",hash_str(&node.name.to_string()), node.name.to_string(), annotation)
    } else {
      format!("{}{}", node.name.to_string(), annotation)
    }
  }

  pub fn kind_annotation(&mut self, node: &KindAnnotation) -> String {
    let kind = self.kind(&node.kind);
    format!("<{}>", kind)
  }

  pub fn kind(&mut self, node: &Kind) -> String {
    match node {
      Kind::Scalar(ident) => ident.to_string(),
      Kind::Empty => "_".to_string(),
      _ => todo!(),
    }
  }

  pub fn factor(&mut self, node: &Factor) -> String {
    let f = match node {
      Factor::Term(term) => self.term(term),
      Factor::Expression(expr) => self.expression(expr),
      Factor::Parenthetical(paren) => format!("({})", self.factor(&paren)),
      Factor::Negate(factor) => format!("-{}", self.factor(factor)),
      Factor::Not(factor) => format!("¬{}", self.factor(factor)),
      Factor::Transpose(factor) => format!("{}'", self.factor(factor)),
    };
    if self.html {
      format!("<span class=\"mech-factor\">{}</span>",f)
    } else {
      f
    }
  }

  pub fn term(&mut self, node: &Term) -> String {
    let mut src = self.factor(&node.lhs);
    for (formula_operator, rhs) in &node.rhs {
      let op = self.formula_operator(formula_operator);
      let rhs = self.factor(rhs);
      src = format!("{}{}{}", src, op, rhs);
    }
    if self.html {
      format!("<span class=\"mech-term\">{}</span>",src)
    } else {
      src
    }
  }

  pub fn formula_operator(&mut self, node: &FormulaOperator) -> String {
    let f = match node {
      FormulaOperator::AddSub(op) => self.add_sub_op(op),
      FormulaOperator::MulDiv(op) => self.mul_div_op(op),
      FormulaOperator::Exponent(op) => self.exponent_op(op),
      FormulaOperator::Vec(op) => self.vec_op(op),
      FormulaOperator::Comparison(op) => self.comparison_op(op),
      FormulaOperator::Logic(op) => self.logic_op(op),
    };
    if self.html {
      format!("<span class=\"mech-formula-operator\">{}</span>",f)
    } else {
      format!(" {} ", f)
    }
  }

  pub fn add_sub_op(&mut self, node: &AddSubOp) -> String {
    match node {
      AddSubOp::Add => "+".to_string(),
      AddSubOp::Sub => "-".to_string(),
    }
  }

  pub fn mul_div_op(&mut self, node: &MulDivOp) -> String {
    match node {
      MulDivOp::Mul => "*".to_string(),
      MulDivOp::Div => "/".to_string(),
    }
  }

  pub fn exponent_op(&mut self, node: &ExponentOp) -> String {
    match node {
      ExponentOp::Exp => "^".to_string(),
    }
  }

  pub fn vec_op(&mut self, node: &VecOp) -> String {
    match node {
      VecOp::MatMul => "**".to_string(),
      VecOp::Solve => "\\".to_string(),
      VecOp::Cross => "×".to_string(),
      VecOp::Dot => "·".to_string(),
    }
  }

  pub fn comparison_op(&mut self, node: &ComparisonOp) -> String {
    match node {
      ComparisonOp::Equal => "==".to_string(),
      ComparisonOp::NotEqual => "≠".to_string(),
      ComparisonOp::GreaterThan => ">".to_string(),
      ComparisonOp::GreaterThanEqual => "≥".to_string(),
      ComparisonOp::LessThan => "<".to_string(),
      ComparisonOp::LessThanEqual => "≤".to_string(),
    }
  }

  pub fn logic_op(&mut self, node: &LogicOp) -> String {
    match node {
      LogicOp::And => "&".to_string(),
      LogicOp::Or => "|".to_string(),
      LogicOp::Xor => "xor".to_string(),
      LogicOp::Not => "¬".to_string(),
    }
  }

  pub fn literal(&mut self, node: &Literal) -> String {
    let l = match node {
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
    };
    if self.html {
      format!("<span class=\"mech-literal\">{}</span>",l)
    } else {
      l
    }
  }

  pub fn atom(&mut self, node: &Atom) -> String {
    if self.html {
      format!("<span class=\"mech-atom\">{}</span>",node.name.to_string())
    } else {
      format!("`{}", node.name.to_string())
    }
  }

  pub fn string(&mut self, node: &MechString) -> String {
    if self.html {
      format!("<span class=\"mech-string\">\"{}\"</span>", node.text.to_string())
    } else {
      format!("\"{}\"", node.text.to_string())
    }
  }

  pub fn number(&mut self, node: &Number) -> String {
    let n = match node {
      Number::Real(real) => self.real_number(real),
      Number::Imaginary(complex) => self.complex_numer(complex),
    };
    if self.html {
      format!("<span class=\"mech-number\">{}</span>",n)
    } else {
      n
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

  pub fn humanize_html(input: String) -> String {
    let mut formatted = String::new();
    let mut indent_level: usize = 0;
    let mut i = 0;
    while i < input.len() {
      // Find the next tag
      if let Some(start) = input[i..].find('<') {
        let tag_start = i + start;
        if let Some(end) = input[tag_start..].find('>') {
          let tag_end = tag_start + end + 1;
          let tag = &input[tag_start..tag_end];
          // Add any content before the tag
          let content = &input[i..tag_start].trim();
          if !content.is_empty() {
            formatted.push('\n');
            formatted.push_str(&" ".repeat(indent_level));
            formatted.push_str(content);
          }
          // Check if this is a closing tag
          if tag.starts_with("</") {
            // Decrease indentation for closing tags
            indent_level = indent_level.saturating_sub(1);
            formatted.push('\n');
            formatted.push_str(&" ".repeat(indent_level));
            formatted.push_str(tag);
          } else if tag.ends_with("/>") {
            // Self-closing tag, no change in indentation
            formatted.push('\n');
            formatted.push_str(&" ".repeat(indent_level));
            formatted.push_str(tag);
          } else {
            // Opening tag
            formatted.push('\n');
            formatted.push_str(&" ".repeat(indent_level));
            formatted.push_str(tag);
            indent_level += 1;
          }
          // Move past the current tag
          i = tag_end;
          continue;
        }
      }
      // Handle remaining content (if no more tags)
      let content = &input[i..].trim();
      if !content.is_empty() {
        formatted.push('\n');
        formatted.push_str(&" ".repeat(indent_level));
        formatted.push_str(content);
      }
      break;
    }
    formatted
  }
 
}