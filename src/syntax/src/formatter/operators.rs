// Operator formatter routines.

use super::*;

impl Formatter {
  pub fn factor(&mut self, node: &Factor) -> String {
    let f = match node {
      Factor::Term(term) => self.term(term),
      Factor::Expression(expr) => self.expression(expr),
      Factor::Parenthetical(paren) => {
        if self.html {
          format!("<span class=\"mech-parenthetical\">({})</span>", self.factor(paren))
        } else {
          format!("({})", self.factor(&paren))
        }
      }
      Factor::Negate(factor) => {
        if self.html {
          format!("<span class=\"mech-negate-op\">-</span><span class=\"mech-negate\">{}</span>", self.factor(factor))
        } else {
          format!("-{}", self.factor(factor))
        }
      }
      Factor::Not(factor) => {
        if self.html {
          format!("<span class=\"mech-not-op\">¬</span><span class=\"mech-not\">{}</span>", self.factor(factor))
        } else {
          format!("¬{}", self.factor(factor))
        }
      }
      Factor::Transpose(factor) => {
        if self.html {
          format!("<span class=\"mech-transpose\">{}</span><span class=\"mech-transpose-op\">'</span>", self.factor(factor))
        } else {
          format!("{}'", self.factor(factor))
        }
      }
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
      FormulaOperator::Power(op) => self.power_op(op),
      FormulaOperator::Vec(op) => self.vec_op(op),
      FormulaOperator::Comparison(op) => self.comparison_op(op),
      FormulaOperator::Logic(op) => self.logic_op(op),
      FormulaOperator::Table(op) => self.table_op(op),
      FormulaOperator::Set(op) => self.set_op(op),
    };
    if self.html {
      format!("<span class=\"mech-formula-operator\">{}</span>",f)
    } else {
      format!(" {} ", f)
    }
  }

  pub fn table_op(&mut self, node: &TableOp) -> String {
    match node {
      TableOp::InnerJoin => "⋈".to_string(),
      TableOp::LeftOuterJoin => "⟕".to_string(),
      TableOp::RightOuterJoin => "⟖".to_string(),
      TableOp::FullOuterJoin => "⟗".to_string(),
      TableOp::LeftSemiJoin => "⋉".to_string(),
      TableOp::LeftAntiJoin => "▷".to_string(),
    }
  }

  pub fn set_op(&mut self, node: &SetOp) -> String {
    match node {
      SetOp::Union => "∪".to_string(),
      SetOp::Intersection => "∩".to_string(),
      SetOp::Difference => "∖".to_string(),
      SetOp::Complement => "∁".to_string(),
      SetOp::Subset => "⊂".to_string(),
      SetOp::Superset => "⊃".to_string(),
      SetOp::ProperSubset => "⊊".to_string(),
      SetOp::ProperSuperset => "⊋".to_string(),
      SetOp::ElementOf => "∈".to_string(),
      SetOp::NotElementOf => "∉".to_string(),
      SetOp::SymmetricDifference => "Δ".to_string(),
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
      MulDivOp::Div => "/".to_string(),
      MulDivOp::Mod => "%".to_string(),
      MulDivOp::Mul => "*".to_string(),
    }
  }

  pub fn power_op(&mut self, node: &PowerOp) -> String {
    match node {
      PowerOp::Pow => "^".to_string(),
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
      ComparisonOp::Equal => "⩵".to_string(),
      ComparisonOp::StrictEqual => "=:=".to_string(),
      ComparisonOp::StrictNotEqual => "=/=".to_string(),
      ComparisonOp::NotEqual => "≠".to_string(),
      ComparisonOp::GreaterThan => ">".to_string(),
      ComparisonOp::GreaterThanEqual => "≥".to_string(),
      ComparisonOp::LessThan => "<".to_string(),
      ComparisonOp::LessThanEqual => "≤".to_string(),
    }
  }

  pub fn logic_op(&mut self, node: &LogicOp) -> String {
    match node {
      LogicOp::And => "&&".to_string(),
      LogicOp::Or => "||".to_string(),
      LogicOp::Xor => "⊻".to_string(),
      LogicOp::Not => "¬".to_string(),
    }
  }

}
