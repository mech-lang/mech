use mech_core::*;
use mech_core::nodes::AstNode;
use crate::compiler::Compiler;
use hashbrown::hash_map::{HashMap};

// # Formatter

// Formats a block as text syntax

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

  pub fn format(&mut self, block_ast: &AstNode) -> String {
    self.html = false;
    let code = self.write_node(block_ast);
    code
  }

  pub fn format_html(&mut self, block_ast: &AstNode) -> String {
    self.html = true;
    let header = r#"<style type="text/css">
.user-function {
  display: block;
  background-color: #17151E;
  color: #E3E1EA;
  padding: 10px;
  font-family: monospace;
  line-height: 20px;
  border-radius: 3px;
}
.block {
  display: block;
  background-color: #17151E;
  color: #E3E1EA;
  padding: 10px;
  font-family: monospace;
  line-height: 20px;
  border-radius: 3px;
}
.code-block {
  display: block;
  background-color: #17151E;
  color: #E3E1EA;
  padding: 10px;
  font-family: monospace;
  line-height: 20px;
  border-radius: 3px;
}
.statement {
  display: block;
}
.transformation {
  display: block;
}
.number-literal {
  color: #72CAAB;
}
.hashtag {
  color: #B27E9B
}
</style>"#;
    let code = self.write_node(block_ast);
    format!("{}{}",header,code)
  }

  pub fn write_node(&mut self, node: &AstNode) -> String {
    if self.html {
      self.indent += 1;
    }
    let mut code = String::new();
    let mut node_type = "";
    match node {
      AstNode::Empty => {
        node_type = "empty";
        code = "_".to_string();
      },
      AstNode::True => {
        node_type = "true";
        code = "true".to_string();
      },
      AstNode::False => {
        node_type = "false";
        code = "false".to_string();
      },
      AstNode::Function{name, children,..} => {
        match MechString::from_chars(name).to_string().as_ref()
        {
          "table/range" => {
            let lhs = self.write_node(&children[0]);
            let rhs = self.write_node(&children[1]);
            code = format!("{} : {}", lhs, rhs);
          },
          "math/add" => {
            let lhs = self.write_node(&children[0]);
            let rhs = self.write_node(&children[1]);
            code = format!("{} + {}", lhs, rhs);
          },
          "math/multiply" => {
            let lhs = self.write_node(&children[0]);
            let rhs = self.write_node(&children[1]);
            code = format!("{} * {}", lhs, rhs);
          },
          "math/divide" => {
            let lhs = self.write_node(&children[0]);
            let rhs = self.write_node(&children[1]);
            code = format!("{} / {}", lhs, rhs);
          },
          "math/subtract" => {
            let lhs = self.write_node(&children[0]);
            let rhs = self.write_node(&children[1]);
            if lhs == "<span class=\"constant\" id=\"constant\">0</span>" || lhs == "0" {
              code = format!("-{}", rhs);
            } else {
              code = format!("{} - {}", lhs, rhs);
            }
          }
          "logic/and" => {
            let lhs = self.write_node(&children[0]);
            let rhs = self.write_node(&children[1]);
            code = format!("{} & {}", lhs, rhs);
          },
          "logic/or" => {
            let lhs = self.write_node(&children[0]);
            let rhs = self.write_node(&children[1]);
            code = format!("{} | {}", lhs, rhs);
          },
          "compare/less-than" => {
            let lhs = self.write_node(&children[0]);
            let rhs = self.write_node(&children[1]);
            code = format!("{} < {}", lhs, rhs);
          },
          "compare/less-than-equal" => {
            let lhs = self.write_node(&children[0]);
            let rhs = self.write_node(&children[1]);
            code = format!("{} <= {}", lhs, rhs);
          },
          "compare/greater-than" => {
            let lhs = self.write_node(&children[0]);
            let rhs = self.write_node(&children[1]);
            code = format!("{} > {}", lhs, rhs);
          },
          "compare/greater-than-equal" => {
            let lhs = self.write_node(&children[0]);
            let rhs = self.write_node(&children[1]);
            code = format!("{} >= {}", lhs, rhs);
          },
          "compare/equal" => {
            let lhs = self.write_node(&children[0]);
            let rhs = self.write_node(&children[1]);
            code = format!("{} == {}", lhs, rhs);
          },
          "compare/not-equal" => {
            let lhs = self.write_node(&children[0]);
            let rhs = self.write_node(&children[1]);
            code = format!("{} != {}", lhs, rhs);
          },
          "matrix/multiply" => {
            let lhs = self.write_node(&children[0]);
            let rhs = self.write_node(&children[1]);
            code = format!("{} ** {}", lhs, rhs);
          },
          _ => {
            //node_type = "function";
            for (ix, child) in children.iter().enumerate() {
              let binding = self.write_node(&child);
              if ix == children.len() - 1 {
                code = format!("{}{}",code, binding);
              } else {
                if self.html {
                  code = format!("{}{}<span class=\"clear\">, </span>",code, binding);
                } else {
                  code = format!("{}{}, ",code, binding);
                }

              }
            }
            code = if self.html {
              format!("<span class=\"function-name\">{}</span>({})",MechString::from_chars(name).to_string(), code)
            } else {
              format!("{}({})", MechString::from_chars(name).to_string(), code)
            }
          }
        }
      },
      AstNode::Table{name, id: _,..} => {
        code = MechString::from_chars(name).to_string();
        if self.html {
          code = format!("<span class=\"hashtag\">#</span><span class=\"global-variable\">{}</span>", code)
        } else {
          code = format!("#{}", code)
        }
      },
      AstNode::Identifier{name, id: _,..} => {
        code = MechString::from_chars(name).to_string();
      },
      AstNode::TableDefine{children} => {
        let lhs = self.write_node(&children[0]);
        /*self.indent = if self.html {
          lhs.len() + 3 - 37 - 47
        } else {
          lhs.len() + 3
        };*/
        let rhs = self.write_node(&children[1]);
        let lhs = if self.html {
          format!("{}", lhs)
        } else {
          format!("{}", lhs)
        };
        code = if self.html {
          format!("{}{}<span class=\"equal\"> = </span>\n{}", self.tab(), lhs, rhs)
        } else {
          format!("{} = {}", lhs, rhs)
        };
      },
      AstNode::SetData{children} => {
        let lhs = self.write_node(&children[0]);
        let rhs = self.write_node(&children[1]);
        code = format!("{} := {}", lhs, rhs);
      },
      AstNode::SplitData{children} => {
        let lhs = self.write_node(&children[0]);
        /*self.indent = if self.html {
          lhs.len() + 4
        } else {
          lhs.len() + 2
        };*/
        let rhs = self.write_node(&children[1]);
        let lhs = if self.html {
          format!("<span class=\"local-variable\">{}</span>", lhs)
        } else {
          format!("{}", lhs)
        };
        code = format!("{} >- {}", lhs, rhs);
      },
      AstNode::AddRow{children} => {
        let lhs = self.write_node(&children[0]);
        let rhs = self.write_node(&children[1]);
        code = format!("{} += {}", lhs, rhs);
      },
      AstNode::VariableDefine{children} => {
        let lhs = self.write_node(&children[0]);
        /*self.indent = if self.html {
          lhs.len() + 4
        } else {
          lhs.len() + 2
        };*/
        let rhs = self.write_node(&children[1]);
        let lhs = if self.html {
          format!("<span class=\"local-variable\">{}</span>", lhs)
        } else {
          format!("{}", lhs)
        };
        code = format!("{} = {}", lhs, rhs);
      },
      AstNode::String{text,..} => {
        let string = MechString::from_chars(text).to_string();
        code = if self.html {
          format!("{}<span class=\"string\">{}</span>\n",self.tab(),string)
        } else {
          format!("{}", string)
        }
      },
      AstNode::SelectData{name, id, children,..} => {
        let name = MechString::from_chars(name).to_string();
        for child in children {
          let written_child = self.write_node(child);
          code = format!("{}{}",code, written_child);
        }
        let formatted_name = match id {
          TableId::Local(..) => {
            if self.html {
              format!("<span class=\"local-variable\">{}</span>", name)
            } else {
              format!("{}", name)
            }
          },
          TableId::Global(..) => {
            if self.html {
              format!("<span class=\"hashtag\">#</span><span class=\"global-variable\">{}</span>", name)
            } else {
              format!("#{}", name)
            }
          },
        };
        code = format!("{}{}",formatted_name, code);
      }
      AstNode::SubscriptIndex{children} => {
        for (ix, child) in children.iter().enumerate() {
          let written_child = self.write_node(child);
          if ix == children.len() - 1 {
            code = format!("{}{}",code, written_child);
          } else {
            if self.html {
              code = format!("{}{}<span class=\"clear\">, </span>",code, written_child);
            } else {
              code = format!("{}{}, ",code, written_child);
            }
          }
        }
        if self.html {
          code = format!("<span class=\"bracket\">{{</span>{}<span class=\"bracket\">}}</span>", code);
        } else {
          code = format!("{{{}}}", code);
        }
      }
      AstNode::DotIndex{children} => {
        let mut reversed = children.clone();
        reversed.reverse();
        for child in reversed {
          let written_child = self.write_node(&child);
          code = format!("{}{}", code, written_child);
        }
        code = format!(".{}", code);
      }
      AstNode::AnonymousTableDefine{children} => {
        let nested = self.nested;
        let rows = self.rows;
        let cols = self.cols;
        self.rows = 0;
        self.cols = 0;
        self.nested = true;
        for (ix, child) in children.iter().enumerate() {
          let mut newline = "";
          let written_child = self.write_node(&child);
          if ix != children.len() - 1 {
            newline = "\n";
          }
          code = format!("{}{}{}", code, written_child, newline);
        }
        self.nested = nested;
        if self.rows == 1 && self.cols == 1 && !self.nested {
          code = format!("{}", code);
        } else {
          if self.html {
            code = format!("<span class=\"bracket\">[</span>{}<span class=\"bracket\">]</span>", code);
          } else {
            code = format!("[{}]", code);
          }
        }
        self.rows = rows;
        self.cols = cols;
      }
      AstNode::SelectAll => {
        node_type = "function";
        code = ":".to_string();
      }
      AstNode::InlineTable{children} => {
        let nested = self.nested;
        self.nested = true;
        for (ix, child) in children.iter().enumerate() {
          let binding = self.write_node(&child);
          if ix == children.len() - 1 {
            code = format!("{}{}",code, binding);
          } else {
            if self.html {
              code = format!("{}{}<span class=\"clear\">, </span>",code, binding);
            } else {
              code = format!("{}{}, ",code, binding);
            }
          }
        }
        self.nested = nested;
        if self.html {
          code = format!("<span class=\"bracket\">[</span>{}<span class=\"bracket\">]</span>", code);
        } else {
          code = format!("[{}]", code);
        };
      }
      AstNode::Binding{children} => {
        let lhs = self.write_node(&children[0]);
        let rhs = self.write_node(&children[1]);
        if self.html {
          code = format!("<span class=\"parameter\">{}:</span> {}", lhs, rhs);
        } else {
          code = format!("{}: {}", lhs, rhs);
        };
      }
      AstNode::Whenever{children} => {
        let table = self.write_node(&children[0]);
        if self.html {
          code = format!("<span class=\"whenever\">~</span> {}", table);
        } else {
          code = format!("~ {}", table);
        };
      }
      AstNode::Wait{children} => {
        let table = self.write_node(&children[0]);
        if self.html {
          code = format!("<span class=\"wait\">|~</span> {}", table);
        } else {
          code = format!("|~ {}", table);
        };
      }
      AstNode::Until{children} => {
        let table = self.write_node(&children[0]);
        if self.html {
          code = format!("<span class=\"until\">~|</span> {}", table);
        } else {
          code = format!("~| {}", table);
        };
      }
      AstNode::TableHeader{children} => {
        self.rows += 1;
        node_type = "parameter";
        for child in children {
          let written_child = self.write_node(child);
          code = format!("{}{} ",code, written_child);
        }
        code = format!("|{}|",code);
      }
      AstNode::TableRow{children} => {
        self.rows += 1;
        self.cols = 0;
        for (ix, child) in children.iter().enumerate() {
          let mut space = "";
          let written_child = self.write_node(child);
          if ix != children.len() - 1 {
            space = " ";
          }
          code = format!("{}{}{}", code, written_child, space)
        }
        let indent = if self.rows != 1 {
          repeat_char(" ", self.indent)
        } else {
          "".to_string()
        };
        code = format!("{}{}", indent, code)
      }
      AstNode::TableColumn{children,..} => {
        self.cols += 1;
        for child in children {
          code = self.write_node(child);
        }
      }
      AstNode::ParagraphText{text, src_range} => {
        let paragraph = MechString::from_chars(text).to_string();
        if self.html {
          code = format!("{}{}<span class=\"paragraph-text\">{}</span>\n",code,self.tab(),paragraph);
        } else {
          code = format!("{}{}\n\n",code,paragraph);
        };
      }
      AstNode::NumberLiteral{kind, bytes, ..} => {
        let mut compiler = Compiler::new();
        let tfms = compiler.compile_node(&node).unwrap();
        match &tfms[1] {
          Transformation::NumberLiteral{kind,bytes} => {
            let mut num = NumberLiteral::new(*kind, bytes.to_vec());
            if self.html {
              code = format!("{}<span class=\"number-literal\">{}</span>",code,num.as_f32());
            } else {
              code = format!("{}{}",code,num.as_f32());
            };
          }
          _ => (),
        }
      }
      AstNode::Program{title,children} => {
        match title {
          Some(title) => {
            let title = MechString::from_chars(title).to_string();
            if self.html {
              code = format!("{}{}<h1 class=\"program-title\">{}</h1>\n",code,self.tab(),title);
            } else {
              let underline = repeat_char("=", title.len() + 2);
              code = format!("{}{}\n{}\n\n",code,title,underline);
            };
          }
          _ => (),
        }
        for child in children {
          code = format!("{}{}",code,self.write_node(child));
        }
      }
      AstNode::Section{title, children, level} => {
        if self.html {
          code = format!("{}{}<span class=\"{}\">\n",code, self.tab(), "section");
        }
        match title {
          Some(title) => {
            let title = MechString::from_chars(title).to_string();
            if self.html {
              let (tag,class) = match level {
                1 => ("h2","level-1-title"),
                2 => ("h3","level-2-title"),
                3 => ("h4","level-3-title"),
                _ => ("ERROR","ERROR")
              };
              code = format!("{}{}<{} class=\"{}\">{}</{}>\n",code,self.tab(),tag,class,title,tag);
            } else {
              let underline = repeat_char("-", title.len() + 2);
              code = format!("{}{}\n{}\n\n",code,title,underline);
            };
          }
          _ => (),
        }
        for child in children {
          code = format!("{}{}",code,self.write_node(child));
        }
        if self.html {
          code = format!("{}{}</span>\n",code,self.tab());
        }
      }
      AstNode::Paragraph{children,..} => {       
        if self.html {
          code = format!("{}{}<p class=\"paragraph\">\n",code, self.tab());
        }
        for child in children {
          code = format!("{}{}",code,self.write_node(child));
        }
        if self.html {
          code = format!("{}{}</p>\n",code,self.tab());
        }
      },
      AstNode::UnorderedList{children,..} => {       
        if self.html {
          code = format!("{}{}<ul class=\"unordrered-list\">\n",code, self.tab());
        }
        for child in children {
          code = format!("{}{}",code,self.write_node(child));
        }
        if self.html {
          code = format!("{}{}</ul>\n",code,self.tab());
        }
      },
      AstNode::ListItem{children,..} => {       
        if self.html {
          code = format!("{}{}<li class=\"list-item\">\n",code, self.tab());
        }
        for child in children {
          code = format!("{}{}",code,self.write_node(child));
        }
        if self.html {
          code = format!("{}{}</li>\n",code,self.tab());
        }
      },
      AstNode::CodeBlock{children,..} => {       
        if self.html {
          code = format!("{}{}<pre class=\"code-block\">\n",code, self.tab());
        }
        for child in children {
          code = format!("{}{}",code,self.write_node(child));
        }
        if self.html {
          code = format!("{}{}</pre>\n",code,self.tab());
        }
      },
      AstNode::UserFunction{children, ..} => {code = self.write_nodes(children,code,"user-function");},
      AstNode::FunctionArgs{children, ..} => {code = self.write_nodes(children,code,"function-args");},
      AstNode::FunctionOutput{children, ..} => {code = self.write_nodes(children,code,"function-output");},
      AstNode::FunctionInput{children, ..} => {code = self.write_nodes(children,code,"function-input");},
      AstNode::FunctionBody{children, ..} => {code = self.write_nodes(children,code,"function-body");},
      AstNode::FunctionBinding{children, ..} => {code = self.write_nodes(children,code,"function-binding");},

      AstNode::KindAnnotation{children, ..} => {code = self.write_nodes(children,code,"kind-annotation");},
      AstNode::InlineCode{children, ..} => {code = self.write_nodes(children,code,"inline-code");},
      AstNode::Swizzle{children, ..} => {code = self.write_nodes(children,code,"inline-code");},
      AstNode::FlattenData{children, ..} => {code = self.write_nodes(children,code,"inline-code");},
      AstNode::Comment{children, ..} => {code = self.write_nodes(children,code,"inline-code");},
      AstNode::UpdateData{children, ..} => {code = self.write_nodes(children,code,"inline-code");},


      AstNode::Transformation{children, ..} => {code = self.write_nodes(children,code,"transformation");},
      AstNode::Root{children,..} => {
        code = self.write_style(code);
        code = self.write_nodes(children,code,"root");
      },
      AstNode::Paragraph{children,..} => {code = self.write_nodes(children,code,"paragraph");},
      AstNode::Attribute{children,..} => {code = self.write_nodes(children,code,"attribute");},
      AstNode::MathExpression{children,..} => {code = self.write_nodes(children,code,"math-expression");},
      AstNode::Expression{children,..} => {code = self.write_nodes(children,code,"expression");},
      AstNode::Statement{children,..} => {code = self.write_nodes(children,code,"statement");},
      AstNode::Block{children, ..} => {code = self.write_nodes(children,code,"block");},
      AstNode::Null => (),
      x => println!("Unhandled Node {:?}", x),
    }
    if self.html && node_type != "" {
      code = format!("<span class=\"{}\">{}</span>", node_type, code);
    }
    if self.html {
      self.indent -= 1;
    }
    code
  }

  fn tab(&mut self) -> String {
    let mut result = "".to_string();
    if self.indent > 100 {
      self.indent = 0;
    }
    for _ in 0..self.indent {
      result = format!("{}{}", result, "  ");
    }
    result
  }

  fn write_style(&self, code: String) -> String {
    let mut code = code;
    let mut style = r#"    
<style type="text/css">
  .user-function {
    display: block;
    background-color: #17151E;
    color: #E3E1EA;
    padding: 10px;
    font-family: monospace;
    line-height: 20px;
    border-radius: 3px;
  }
  .block {
    display: block;
    background-color: #17151E;
    color: #E3E1EA;
    padding: 10px;
    font-family: monospace;
    line-height: 20px;
    border-radius: 3px;
  }
  .transformation {
    display: block;
  }
  .number-literal {
    color: #72CAAB;
  }
  .hashtag {
    color: #B27E9B
  }
</style>"#;
    format!("{}{}\n",code, style)
  }

  fn write_nodes(&mut self, children: &Vec<AstNode>, code: String, class: &str) -> String {
    let mut code = code;
    if self.html {
      code = format!("{}{}<span class=\"{}\">\n",code, self.tab(), class);
    }
    for child in children {
      code = format!("{}{}",code,self.write_node(child));
    }
    if self.html {
      code = format!("{}{}</span>\n",code,self.tab());
    }
    code
  }
  

}

fn repeat_char(to_print: &str, n: usize) -> String {
  let mut result = "".to_string();
  for _ in 0..n {
    result = format!("{}{}", result, to_print);
  }
  result
}
