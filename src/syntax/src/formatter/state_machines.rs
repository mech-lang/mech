// State-machine and embedded Mech code formatter routines.
//
// Grouped similarly to parser/interpreter modules for easier navigation.

use super::*;

impl Formatter {
  pub fn mech_code(&mut self, node: &Vec<(MechCode,Option<Comment>)>) -> String {
    let mut src = String::new();
    for (code,cmmnt) in node {
      let c = match code {
        MechCode::Comment(cmnt) => self.comment(cmnt),
        MechCode::Expression(expr) => self.expression(expr),
        MechCode::FsmImplementation(fsm_impl) => self.fsm_implementation(fsm_impl),
        MechCode::FsmSpecification(fsm_spec) => self.fsm_specification(fsm_spec),
        MechCode::FunctionDefine(func_def) => self.function_define(func_def),
        MechCode::Statement(stmt) => self.statement(stmt),
        x => todo!("Unhandled MechCode: {:#?}", x),
      };
      let formatted_comment = match cmmnt {
        Some(cmmt) => self.comment(cmmt),
        None => String::new(),
      };
      if self.html {
        src.push_str(&format!("<span class=\"mech-code\">{}{}</span>", c, formatted_comment));
      } else {
        src.push_str(&format!("{}{}\n", c, formatted_comment));
      }
    }
    if self.html {
      format!("<span class=\"mech-code-block\">{}</span>",src)
    } else {
      src
    }
  }

  pub fn fsm_implementation(&mut self, node: &FsmImplementation) -> String {
    let name = node.name.to_string();
    let mut input = "".to_string();
    for (i, ident) in node.input.iter().enumerate() {
      let v = self.var(ident);
      if i == 0 {
        input = format!("{}", v);
      } else {
        input = format!("{}, {}", input, v);
      }
    }
    let start = self.pattern(&node.start);
    let mut arms = "".to_string();
    for (i, arm) in node.arms.iter().enumerate() {
      let a = self.fsm_arm(arm, i == node.arms.len() - 1);
      if i == 0 {
        arms = format!("{}", a);
      } else {
        arms = format!("{}{}", arms, a);
      }
    }
    if self.html {
      format!("<div class=\"mech-fsm-implementation\">
        <div class=\"mech-fsm-implementation-header\">
          <span class=\"mech-fsm-sigil\">#</span>
          <span class=\"mech-fsm-name\">{}</span>
          <span class=\"mech-left-paren\">(</span>
          <span class=\"mech-fsm-input\">{}</span>
          <span class=\"mech-right-paren\">)</span>
          <span class=\"mech-fsm-define-op\">→</span>
          <span class=\"mech-fsm-start\">{}</span>
        </div>
        <div class=\"mech-fsm-arms\">
          {}
        </div>
      </div>",name,input,start,arms)
    } else {
      format!("#{}({}) {} {}\n{}", name, input, "->" , start, arms)
    }
  }

  pub fn fsm_arm(&mut self, node: &FsmArm, last: bool) -> String {
    let arm = match node {
      FsmArm::Guard(pattern, guards) => {
        let p = self.pattern(pattern);
        let mut gs = "".to_string();
        for (i, guard) in guards.iter().enumerate() {
          let g = self.guard(guard);
          if i == 0 {
            if self.html {
              gs = format!("<div class=\"mech-fsm-guard-arm\">├ {}</div>", g);
            } else {
              gs = format!("    ├ {}\n", g);
            }
          } else if i == guards.len() - 1 {
            if self.html {
              gs = format!("{}<div class=\"mech-fsm-guard-arm\">└ {}</div>", gs, g);
            } else {
              gs = format!("{}    └ {}", gs, g); 
            }
          } else {  
            if self.html {
              gs = format!("{}<div class=\"mech-fsm-guard-arm\">├ {}</div>", gs, g);
            } else {
              gs = format!("{}    ├ {}\n", gs, g);
            }
          }
        }
        if self.html {
          format!("<div class=\"mech-fsm-arm-guard\">
            <span class=\"mech-fsm-start\">{}</span>
            <span class=\"mech-fsm-guards\">{}</span>
          </div>",p,gs)
        } else {
          format!("  {}\n{}", p, gs)
        }
      },
      FsmArm::Transition(pattern, transitions) => {
        let p = self.pattern(pattern);
        let mut ts = "".to_string();
        for (i, transition) in transitions.iter().enumerate() {
          let t = self.transition(transition);
          if i == 0 {
            ts = format!("{}", t);
          } else {
            ts = format!("{}{}", ts, t);
          }
        }
        if self.html {
          format!("<div class=\"mech-fsm-arm\">
            <span class=\"mech-fsm-arm-pattern\">{}</span>
            <span class=\"mech-fsm-arm-transitions\">{}</span>
          </div>",p,ts)
        } else {
          format!("  {}{}", p, ts)
        }
      },
    };
    if self.html {
      if last {
        format!("<div class=\"mech-fsm-arm-last\">{}.</div>",arm)
      } else {
        format!("<div class=\"mech-fsm-arm\">{}</div>",arm)
      }
    } else {
      if last { 
        format!("{}.", arm)
      } else {
        format!("{}\n", arm)
      }
    }
  }

  pub fn guard(&mut self, node: &Guard) -> String {
    let condition = self.pattern(&node.condition);
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
      format!("<div class=\"mech-guard\">
        <span class=\"mech-guard-condition\">{}</span>
        <span class=\"mech-guard-transitions\">{}</span>
      </div>",condition,transitions)
    } else {
      format!("{}{}", condition, transitions)
    }
  }
 

  pub fn pattern(&mut self, node: &Pattern) -> String {
    let p = match node {
      Pattern::Wildcard => {
        if self.html {
          format!("<span class=\"mech-pattern-wildcard\">*</span>")
        } else {
          format!("*")
        }
      },
      Pattern::Tuple(tpl) => self.pattern_tuple(tpl),
      Pattern::Array(arr) => self.pattern_array(arr),
      Pattern::Expression(expr) => self.expression(expr),
      Pattern::TupleStruct(tuple_struct) => self.pattern_tuple_struct(tuple_struct),
    };
    if self.html {
      format!("<span class=\"mech-pattern\">{}</span>",p)
    } else {
      p
    }
  }

  pub fn pattern_tuple_struct(&mut self, node: &PatternTupleStruct) -> String {
    let name = node.name.to_string();
    let mut patterns = "".to_string();
    for (i, pattern) in node.patterns.iter().enumerate() {
      let p = self.pattern(pattern);
      if i == 0 {
        patterns = format!("{}", p);
      } else {
        patterns = format!("{}, {}", patterns, p);
      }
    }
    if self.html {
      format!("<span class=\"mech-tuple-struct\">
        <span class=\"mech-tuple-struct-sigil\">:</span>
        <span class=\"mech-tuple-struct-name\">{}</span>
        <span class=\"mech-left-paren\">(</span>
        <span class=\"mech-tuple-struct-patterns\">{}</span>
        <span class=\"mech-right-paren\">)</span>
      </span>",name,patterns)
    } else {
      format!(":{}({})", name, patterns)
    }
  }

  pub fn pattern_tuple(&mut self, node: &PatternTuple) -> String {
    let mut patterns = "".to_string();
    for (i, pattern) in node.0.iter().enumerate() {
      let p = self.pattern(pattern);
      if i == 0 {
        patterns = format!("{}", p);
      } else {
        patterns = format!("{}, {}", patterns, p);
      }
    }
    if self.html {
      format!("<span class=\"mech-pattern-tuple\">
        <span class=\"mech-left-paren\">(</span>
        <span class=\"mech-patterns\">{}</span>
        <span class=\"mech-right-paren\">)</span>
      </span>",patterns)
    } else {
      format!("({})", patterns)
    }
  }

  pub fn transition(&mut self, node: &Transition) -> String {
    match node {
      Transition::Next(pattern) => {
        if self.html {
          format!("<span class=\"mech-transition-next\">→ {}</span>",self.pattern(pattern))
        } else {
          format!(" {} {}", "->", self.pattern(pattern))
        }
      }
      Transition::Output(pattern) => {
        if self.html {
          format!("<span class=\"mech-transition-output\">⇒ {}</span>",self.pattern(pattern))
        } else {
          format!(" {} {}", "=>", self.pattern(pattern))
        }
      }
      Transition::Async(pattern) => {
        if self.html {
          format!("<span class=\"mech-transition-async\">↝ {}</span>",self.pattern(pattern))
        } else {
          format!(" {} {}", "~>", self.pattern(pattern))

        }
      }
      Transition::Statement(stmt) => {
        if self.html {
          format!("<span class=\"mech-transition-statement\">→ {}</span>",self.statement(stmt))
        } else {
          format!(" {} {}", "->", self.statement(stmt))
        }
      }
      Transition::CodeBlock(code) => {
        let mut code_str = "".to_string();
        let formatted = self.mech_code(code);
        if self.html {
          code_str.push_str(&format!("<span class=\"mech-transition-code\">→ {}</span>", formatted));
        } else {
          code_str.push_str(&format!(" {} {}", "->", formatted));
        }
        code_str
      }
    }
  }

  pub fn fsm_specification(&mut self, node: &FsmSpecification) -> String {
    let name = node.name.to_string();
    let mut input = "".to_string();
    for (i, var) in node.input.iter().enumerate() {
      let v = self.var(var);
      if i == 0 {
        input = format!("{}", v);
      } else {
        input = format!("{}, {}", input, v);
      }
    }
    let output = match &node.output {
      Some(kind) => format!(" {} {}", "⇒", self.kind_annotation(&kind.kind)),
      None => "".to_string(),
    };
    let mut states = "".to_string();
    for (i, state) in node.states.iter().enumerate() {
      let v = self.state_definition(state);
      let state_arm = if node.states.len() == 1 {
        format!("{} {}", "└", v)
      } else if i == 0 {
        format!("{} {}", "├", v)
      } else if i == node.states.len() - 1 {
        format!("{} {}{}", "└", v, ".")
      } else {
        format!("{} {}", "├", v)
      };
      if self.html {
        states = format!("{}<span class=\"mech-fsm-state\">{}</span>",states,state_arm);
      } else {
        states = format!("{}    {}\n",states,state_arm);
      }
    }
    if self.html {
      format!("<div class=\"mech-fsm-specification\">
      <div class=\"mech-fsm-specification-header\">
        <span class=\"mech-fsm-sigil\">#</span>
        <span class=\"mech-fsm-name\">{}</span>
        <span class=\"mech-left-paren\">(</span>
        <span class=\"mech-fsm-input\">{}</span>
        <span class=\"mech-right-paren\">)</span>
        <span class=\"mech-fsm-output\">{}</span>
        <span class=\"mech-fsm-define-op\">:=</span>
      </div>
      <div class=\"mech-fsm-states\">{}</div>
      </div>",name,input,output,states)
    } else {
      format!("#{}({}){} {}\n{}", name, input, output, ":=", states)
    }
  }

}
