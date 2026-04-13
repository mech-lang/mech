// Expression formatter routines.

use super::*;

impl Formatter {
  pub fn expression(&mut self, node: &Expression) -> String {
    let e = match node {
      Expression::Var(var) => self.var(var),
      Expression::Formula(factor) => self.factor(factor),
      Expression::Literal(literal) => self.literal(literal),
      Expression::Structure(structure) => self.structure(structure),
      Expression::Slice(slice) => self.slice(slice),
      Expression::FunctionCall(function_call) => self.function_call(function_call),
      Expression::Range(range) => self.range_expression(range),
      Expression::SetComprehension(set_comp) => self.set_comprehension(set_comp),
      Expression::MatrixComprehension(matrix_comp) => self.matrix_comprehension(matrix_comp),
      Expression::Match(match_expr) => self.match_expression(match_expr),
      Expression::FsmPipe(fsm_pipe) => self.fsm_pipe(fsm_pipe),
      x => todo!("Unhandled Expression: {:#?}", x),
    };
    if self.html {
      format!("<span class=\"mech-expression\">{}</span>",e)
    } else {
      format!("{}", e)
    }
  }

  pub fn pattern_array(&mut self, node: &PatternArray) -> String {
    let mut parts: Vec<String> = vec![];
    for p in &node.prefix {
      parts.push(self.pattern(p));
    }
    if let Some(spread) = &node.spread {
      parts.push("...".to_string());
      if let Some(binding) = &spread.binding {
        parts.push(self.pattern(binding));
      }
    }
    for p in &node.suffix {
      parts.push(self.pattern(p));
    }
    format!("[{}]", parts.join(" "))
  }

  pub fn match_expression(&mut self, node: &MatchExpression) -> String {
    let source = self.expression(&node.source);
    let mut lines = vec![format!("{}?", source)];
    for arm in &node.arms {
      let pattern = self.pattern(&arm.pattern);
      let guard = arm
        .guard
        .as_ref()
        .map(|expr| format!(", {}", self.expression(expr)))
        .unwrap_or_default();
      let expr = self.expression(&arm.expression);
      lines.push(format!("│ {}{} -> {}", pattern, guard, expr));
    }
    lines.join("\n")
  }

  pub fn fsm_instance(&mut self, node: &FsmInstance) -> String {
    let name = node.name.to_string();
    let mut args = "".to_string();
    match &node.args {
      Some(arguments) => {
        for (i, (ident, expr)) in arguments.iter().enumerate() {
          let e = self.expression(expr);
          let arg_str = match ident {
            Some(id) => format!("{}: {}", id.to_string(), e),
            None => e,
          };
          if i == 0 {
            args = format!("{}", arg_str);
          } else {
            args = format!("{}, {}", args, arg_str);
          }
        }
        if self.html {
          format!("<span class=\"mech-fsm-instance\"><span class=\"mech-fsm-name\">#{}</span><span class=\"mech-left-paren\">(</span><span class=\"mech-fsm-args\">{}</span><span class=\"mech-right-paren\">)</span></span>",name,args)
        } else {
          format!("#{}({})", name, args)
        }
      },
      None => {
        if self.html {
          format!("<span class=\"mech-fsm-instance\"><span class=\"mech-fsm-name\">#{}</span></span>",name)
        } else {
          format!("#{}", name)
        }
      },
    }
  }

  pub fn fsm_pipe(&mut self, node: &FsmPipe) -> String {
    let start = self.fsm_instance(&node.start);
    let mut transitions = "".to_string();
    for (i, transition) in node.transitions.iter().enumerate() {
      let t = self.transition(transition);
      if i == 0 {
        transitions = format!("{}", t);
      } else {
        transitions = format!("{}{}", transitions, t);
      }
    }
    if self.html {
      format!("<span class=\"mech-fsm-pipe\"><span class=\"mech-fsm-pipe-start\">{}</span><span class=\"mech-fsm-pipe-transitions\">{}</span></span>",start,transitions)
    } else {
      format!("{}{}", start, transitions)
    }
  }

  pub fn set_comprehension(&mut self, node: &SetComprehension) -> String {
    let expr = self.expression(&node.expression);

    let qualifiers = node
      .qualifiers
      .iter()
      .map(|q| self.comprehension_qualifier(q))
      .collect::<Vec<_>>()
      .join(", ");

    if self.html {
      format!(
        "<span class=\"mech-set-comprehension\">\
          <span class=\"mech-set-open\">{{</span>\
          <span class=\"mech-set-expression\">{}</span>\
          <span class=\"mech-set-bar\"> | </span>\
          <span class=\"mech-set-qualifiers\">{}</span>\
          <span class=\"mech-set-close\">}}</span>\
        </span>",
        expr, qualifiers
      )
    } else {
      format!("{{ {} | {} }}", expr, qualifiers)
    }
  }

  pub fn matrix_comprehension(&mut self, node: &MatrixComprehension) -> String {
    let expr = self.expression(&node.expression);
    let quals = node.qualifiers
      .iter()
      .map(|q| self.comprehension_qualifier(q))
      .collect::<Vec<_>>()
      .join(", ");

    if self.html {
      format!(
        "<span class=\"mech-matrix-comprehension\">\\
          <span class=\"mech-bracket start\">[</span>\\
          <span class=\"mech-comp-expr\">{}</span> \\
          <span class=\"mech-comp-bar\">|</span> \\
          <span class=\"mech-comp-quals\">{}</span>\\
          <span class=\"mech-bracket end\">]</span>\\
        </span>",
        expr, quals
      )
    } else {
      format!("[ {} | {} ]", expr, quals)
    }
  }

  pub fn comprehension_qualifier(&mut self, node: &ComprehensionQualifier) -> String {
    match node {
      ComprehensionQualifier::Generator((pattern, expr)) => {
        self.generator(pattern, expr)
      }
      ComprehensionQualifier::Let(var_def) => {
        self.variable_define(var_def)
      }
      ComprehensionQualifier::Filter(expr) => {
        self.expression(expr)
      }
    }
  }

  pub fn generator(&mut self, pattern: &Pattern, expr: &Expression) -> String {
    let p = self.pattern(pattern);
    let e = self.expression(expr);

    if self.html {
      format!(
        "<span class=\"mech-generator\">\
          <span class=\"mech-generator-pattern\">{}</span>\
          <span class=\"mech-generator-arrow\"> &lt;- </span>\
          <span class=\"mech-generator-expression\">{}</span>\
        </span>",
        p, e
      )
    } else {
      format!("{} <- {}", p, e)
    }
  }

  pub fn range_expression(&mut self, node: &RangeExpression) -> String {
    let start = self.factor(&node.start);
    let operator = match &node.operator {
      RangeOp::Inclusive => "..=".to_string(),
      RangeOp::Exclusive => "..".to_string(),
    };
    let terminal = self.factor(&node.terminal);
    let increment = match &node.increment {
      Some((op, factor)) => {
        let o = match op {
          RangeOp::Inclusive => "..=".to_string(),
          RangeOp::Exclusive => "..".to_string(),
        };
        let f = self.factor(factor);
        if self.html {
          format!("<span class=\"mech-range-increment\">{}{}</span>",o,f)
        } else {
          format!("{}{}", o, f)
        }
      },
      None => "".to_string(),
    };
    if self.html {
      format!("<span class=\"mech-range-expression\"><span class=\"mech-range-start\">{}</span><span class=\"mech-range-operator\">{}</span><span class=\"mech-range-terminal\">{}</span>{}</span>",start,operator,terminal,increment)
    } else {
      format!("{}{}{}{}", start, operator, terminal, increment)
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
    let id = format!("{}:{}",hash_str(&name),self.interpreter_id);
    if self.html {
      format!("<span class=\"mech-function-call\"><span id=\"{}\" class=\"mech-function-name mech-clickable\">{}</span><span class=\"mech-left-paren\">(</span><span class=\"mech-argument-list\">{}</span><span class=\"mech-right-paren\">)</span></span>",id,name,args)
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
    let id = format!("{}:{}",hash_str(&name),self.interpreter_id);
    if self.html {
      format!("<span class=\"mech-slice\"><span id=\"{}\" class=\"mech-var-name mech-clickable\">{}</span><span class=\"mech-subscript\">{}</span></span>",id,name,subscript)
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
      Subscript::Swizzle(idents) => self.swizzle(idents),
      Subscript::Range(range) => self.range_expression(range),
      Subscript::Brace(subs) => self.brace(subs),
      Subscript::DotInt(real) => self.dot_int(real),
    }
  }

  pub fn brace(&mut self, node: &Vec<Subscript>) -> String {
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
      format!("<span class=\"mech-brace\">{{{}}}</span>",src)
    } else {
      format!("{{{}}}",src)
    }
  }

  pub fn swizzle(&mut self, node: &Vec<Identifier>) -> String {
    let mut src = "".to_string();
    for (i, ident) in node.iter().enumerate() {
      let s = self.dot(ident);
      if i == 0 {
        src = format!("{}", s);
      } else {
        src = format!("{},{}", src, s);
      }
    }
    if self.html {
      format!("<span class=\"mech-swizzle\">{}</span>",src)
    } else {
      format!("{}",src)
    }
  }

  pub fn dot_int(&mut self, node: &RealNumber) -> String {
    let node_str = match node {
      RealNumber::Integer(tkn) => tkn.to_string(),
      _ => "".to_string(),
    };
    if self.html {
      format!(".<span class=\"mech-dot-int\">{}</span>",node_str)
    } else {
      format!(".{}",node_str)
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

}
