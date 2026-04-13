// Statement and definition formatter routines.

use super::*;

impl Formatter {
  pub fn state_definition(&mut self, node: &StateDefinition) -> String {
    let name = node.name.to_string();
    let mut state_variables = "".to_string();
    match &node.state_variables {
      Some(vars) => {
        for (i, var) in vars.iter().enumerate() {
          let v = self.var(var);
          if i == 0 {
            state_variables = format!("{}", v);
          } else {
            state_variables = format!("{}, {}", state_variables, v);
          }
        }
      },
      None => {}
    }
    if self.html {
      format!("<div class=\"mech-state-definition\">
      <span class=\"mech-state-name\"><span class=\"mech-state-name-sigil\">:</span>{}</span>
      <span class=\"mech-left-paren\">(</span>
      <span class=\"mech-state-variables\">{}</span>
      <span class=\"mech-right-paren\">)</span>
      </div>",name,state_variables)
    } else {
      format!("{}({})", name, state_variables)
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
      format!("{}{} {} {}", mutable, var, ":=", expression)
    }
  }

  pub fn statement(&mut self, node: &Statement) -> String {
    let s = match node {
      Statement::VariableDefine(var_def) => self.variable_define(var_def),
      Statement::OpAssign(op_asgn) => self.op_assign(op_asgn),
      Statement::VariableAssign(var_asgn) => self.variable_assign(var_asgn),
      Statement::TupleDestructure(tpl_dstrct) => self.tuple_destructure(tpl_dstrct),
      Statement::KindDefine(kind_def) => self.kind_define(kind_def),
      Statement::EnumDefine(enum_def) => self.enum_define(enum_def),
      _ => todo!(),
      //Statement::FsmDeclare(fsm_decl) => self.fsm_declare(fsm_decl, src),
    };
    if self.html {
      format!("<span class=\"mech-statement\">{}</span>",s)
    } else {
      format!("{}", s)
    }
  }

  pub fn enum_define(&mut self, node: &EnumDefine) -> String {
    let name = node.name.to_string();
    let mut variants = "".to_string();
    for (i, variant) in node.variants.iter().enumerate() {
      if i == 0 {
        if self.html {
          variants = format!("<span class=\"mech-enum-variant\">{}</span>", self.enum_variant(variant));
        } else {
          variants = format!("{}", self.enum_variant(variant));
        }
      } else {
        if self.html {
          variants = format!("{}<span class=\"mech-enum-variant-sep\">|</span><span class=\"mech-enum-variant\">{}</span>", variants, self.enum_variant(variant));
        } else {
          variants = format!("{} | {}", variants, self.enum_variant(variant));
        }
      }
    }
    if self.html {
      format!("<span class=\"mech-enum-define\"><span class=\"mech-kind-annotation\">&lt;<span class=\"mech-enum-name\">{}</span>&gt;</span><span class=\"mech-enum-define-op\">:=</span><span class=\"mech-enum-variants\">{}</span></span>",name,variants)
    } else {
      format!("<{}> := {}", name, variants)
    }
  }

  pub fn enum_variant(&mut self, node: &EnumVariant) -> String {
    let name = node.name.to_string();
    let mut kind = "".to_string();
    match &node.value {
      Some(k) => {
        kind = self.kind_annotation(&k.kind);
      },
      None => {},
    }
    if self.html {
      format!("<span class=\"mech-enum-variant\"><span class=\"mech-enum-variant-name\">{}</span><span class=\"mech-enum-variant-kind\">{}</span></span>",name,kind)
    } else {
      format!("{}{}", name, kind)
    }
  }

  pub fn kind_define(&mut self, node: &KindDefine) -> String {
    let name = node.name.to_string();
    let kind = self.kind_annotation(&node.kind.kind);
    if self.html {
      format!("<span class=\"mech-kind-define\"><span class=\"mech-kind-annotation\">&lt;<span class=\"mech-kind\">{}</span>&gt;</span><span class=\"mech-kind-define-op\">:=</span><span class=\"mech-kind-annotation\">{}</span></span>",name,kind)
    } else {
      format!("<{}> := {}", name, kind)
    }
  }

  pub fn tuple_destructure(&mut self, node: &TupleDestructure) -> String {
    let mut vars = "".to_string();
    for (i, var) in node.vars.iter().enumerate() {
      let v = var.to_string();
      if i == 0 {
        if self.html {
          let id = format!("{}:{}",hash_str(&v),self.interpreter_id);
          vars = format!("<span id=\"{}\" class=\"mech-var-name mech-clickable\">{}</span>",id,v);
        } else {
          vars = format!("{}", v);
        }
      } else {
        if self.html {
          let id = format!("{}:{}",hash_str(&v),self.interpreter_id);
          vars = format!("{}, <span id=\"{}\" class=\"mech-var-name mech-clickable\">{}</span>", vars, id, v);
        } else {
          vars = format!("{}, {}", vars, v);
        }
      }
    }
    let expression = self.expression(&node.expression);
    if self.html {
      format!("<span class=\"mech-tuple-destructure\"><span class=\"mech-tuple-vars\">({})</span><span class=\"mech-assign-op\">:=</span><span class=\"mech-tuple-expression\">{}</span></span>",vars,expression)
    } else {
      format!("({}) := {}", vars, expression)
    }
  }

  pub fn variable_assign(&mut self, node: &VariableAssign) -> String {
    let target = self.slice_ref(&node.target);
    let expression = self.expression(&node.expression);
    if self.html {
      format!("<span class=\"mech-variable-assign\">
        <span class=\"mech-target\">{}</span>
        <span class=\"mech-assign-op\">=</span>
        <span class=\"mech-expression\">{}</span>
      </span>",target,expression)
    } else {
      format!("{} = {}", target, expression)
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
      OpAssignOp::Div => "/=".to_string(),
      OpAssignOp::Exp => "^=".to_string(),
      OpAssignOp::Mod => "%=".to_string(),
      OpAssignOp::Mul => "*=".to_string(),
      OpAssignOp::Sub => "-=".to_string(),
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
    let id = format!("{}:{}",hash_str(&name),self.interpreter_id);
    if self.html {
      format!("<span class=\"mech-slice-ref\"><span id=\"{}\" class=\"mech-var-name mech-clickable\">{}</span><span class=\"mech-subscript\">{}</span></span>",id,name,subscript)
    } else {
      format!("{}{}", name, subscript)
    }
  }

}
