// Function formatter routines.

use super::*;

impl Formatter {
  pub fn function_define(&mut self, node: &FunctionDefine) -> String {
    let name = node.name.to_string();
    let input = node
      .input
      .iter()
      .map(|arg| self.function_argument(arg))
      .collect::<Vec<_>>()
      .join(", ");

    if !node.match_arms.is_empty() {
      let output_kind = node
        .output
        .first()
        .map(|arg| self.kind_annotation(&arg.kind.kind))
        .unwrap_or_else(|| "<_>".to_string());

      let arms = node
        .match_arms
        .iter()
        .enumerate()
        .map(|(ix, arm)| {
          let branch = if ix + 1 == node.match_arms.len() { "└" } else { "├" };
          let pattern = self.pattern(&arm.pattern);
          let expression = self.expression(&arm.expression);
          if self.html {
            format!("<div class=\"mech-function-match-arm\"><span class=\"mech-function-branch\">{}</span><span class=\"mech-function-pattern\">{}</span> <span class=\"mech-function-arrow\">-&gt;</span><span class=\"mech-function-expression\">{}</span></div>", branch, pattern, expression)
          } else {
            format!("  {} {} -> {}", branch, pattern, expression)
          }
        })
        .collect::<Vec<_>>()
        .join(if self.html { "" } else { "\n" });

      if self.html {
        format!("<div class=\"mech-function-define\"><div class=\"mech-function-signature\"><span class=\"mech-function-name\">{}</span><span class=\"mech-left-paren\">(</span><span class=\"mech-function-input\">{}</span><span class=\"mech-right-paren\">)</span> <span class=\"mech-function-arrow\">-&gt;</span> <span class=\"mech-function-output\">{}</span></div><div class=\"mech-function-match-arms\">{}<span class=\"mech-function-period\">.</span></div></div>", name, input, output_kind, arms)
      } else {
        format!("{}({}) -> {}\n{}.", name, input, output_kind, arms)
      }
    } else {
      let output = if node.output.len() == 1 {
        self.function_argument(&node.output[0])
      } else {
        format!(
          "({})",
          node.output
            .iter()
            .map(|arg| self.function_argument(arg))
            .collect::<Vec<_>>()
            .join(", ")
        )
      };

      let statements = node
        .statements
        .iter()
        .map(|stmt| self.statement(stmt))
        .collect::<Vec<_>>()
        .join(if self.html { "" } else { "\n" });

      if self.html {
        format!("<div class=\"mech-function-define\"><div class=\"mech-function-signature\"><span class=\"mech-function-name\">{}</span><span class=\"mech-left-paren\">(</span><span class=\"mech-function-input\">{}</span><span class=\"mech-right-paren\">)</span> <span class=\"mech-function-equals\">=</span> <span class=\"mech-function-output\">{}</span> <span class=\"mech-define-op\">:=</span></div><div class=\"mech-function-body\">{}.</div></div>", name, input, output, statements)
      } else {
        format!("{}({}) = {} :=\n{}.", name, input, output, statements)
      }
    }
  }

  pub fn function_argument(&mut self, node: &FunctionArgument) -> String {
    let name = node.name.to_string();
    let kind = self.kind_annotation(&node.kind.kind);
    if self.html {
      format!("<span class=\"mech-function-argument\"><span class=\"mech-function-argument-name\">{}</span><span class=\"mech-function-argument-kind\">{}</span></span>", name, kind)
    } else {
      format!("{}{}", name, kind)
    }
  }

}
