// Value/structure formatter routines.

use super::*;

impl Formatter {
  pub fn structure(&mut self, node: &Structure) -> String {
    let s = match node {
      Structure::Matrix(matrix) => self.matrix(matrix),
      Structure::Record(record) => self.record(record),
      Structure::Empty => "_".to_string(),
      Structure::Table(table) => self.table(table),
      Structure::Tuple(tuple) => self.tuple(tuple),
      Structure::TupleStruct(tuple_struct) => self.tuple_struct(tuple_struct),
      Structure::Set(set) => self.set(set),
      Structure::Map(map) => self.map(map),
    };
    if self.html {
      format!("<span class=\"mech-structure\">{}</span>",s)
    } else {
      format!("{}", s)
    }
  }

  pub fn map(&mut self, node: &Map) -> String {
    let mut src = "".to_string();
    for (i, mapping) in node.elements.iter().enumerate() {
      let m = self.mapping(mapping);
      if i == 0 {
        src = format!("{}", m);
      } else {
        src = format!("{}, {}", src, m);
      }
    }
    if self.html {
      format!("<span class=\"mech-map\"><span class=\"mech-start-brace\">{{</span>{}<span class=\"mech-end-brace\">}}</span></span>",src)
    } else {
      format!("{{{}}}", src)
    }
  }

  pub fn mapping(&mut self, node: &Mapping) -> String {
    let key = self.expression(&node.key);
    let value = self.expression(&node.value);
    if self.html {
      format!("<span class=\"mech-mapping\"><span class=\"mech-key\">{}</span><span class=\"mech-colon-op\">:</span><span class=\"mech-value\">{}</span></span>",key,value)
    } else {
      format!("{}: {}", key, value)
    }
  }

  pub fn set(&mut self, node: &Set) -> String {
    let mut src = "".to_string();
    for (i, element) in node.elements.iter().enumerate() {
      let e = self.expression(element);
      if i == 0 {
        src = format!("{}", e);
      } else {
        src = format!("{}, {}", src, e);
      }
    }
    if self.html {
      format!("<span class=\"mech-set\"><span class=\"mech-start-brace\">{{</span>{}<span class=\"mech-end-brace\">}}</span></span>",src)
    } else {
      format!("{{{}}}", src)
    }
  }

  pub fn tuple_struct(&mut self, node: &TupleStruct) -> String {
    let name = node.name.to_string();
    let value = self.expression(&node.value);
    if self.html {
      format!("<span class=\"mech-tuple-struct\"><span class=\"mech-tuple-struct-name\">{}</span><span class=\"mech-tuple-struct-value\">{}</span></span>",name,value)
    } else {
      format!("{}{}", name, value)
    }
  }

  pub fn table(&mut self, node: &Table) -> String {
    let header = self.table_header(&node.header);
    let mut rows = "".to_string();
    for (i, row) in node.rows.iter().enumerate() {
      let r = self.table_row(row);
      if i == 0 {
        rows = format!("{}", r);
      } else {
        rows = format!("{}{}", rows, r);
      }
    }
    if self.html {
      format!("<table class=\"mech-table\">{}<tbody class=\"mech-table-body\">{}</tbody></table>",header,rows)
    } else {
      format!("{}{}", header, rows)
    }
  }

  pub fn table_header(&mut self, node: &TableHeader) -> String {
    let mut src = "".to_string();
    for (i, field) in node.0.iter().enumerate() {
      let f = self.field(field);
      if self.html {
        src = format!("{}<th class=\"mech-table-field\">{}</th>",src, f);
      } else {
        src = format!("{}{}",src, f);
      }
    }
    if self.html {
      format!("<thead class=\"mech-table-header\"><tr>{}</tr></thead>",src)
    } else {
      src
    }
  }

  pub fn table_row(&mut self, node: &TableRow) -> String {
    let mut src = "".to_string();
    for (i, column) in node.columns.iter().enumerate() {
      let c = self.table_column(column);
      if i == 0 {
        src = format!("{}", c);
      } else {
        src = format!("{} {}", src, c);
      }
    }
    if self.html {
      format!("<tr class=\"mech-table-row\">{}</tr>",src)
    } else {
      src
    }
  }

  pub fn table_column(&mut self, node: &TableColumn) -> String {
    let element = self.expression(&node.element);
    if self.html {
      format!("<td class=\"mech-table-column\">{}</td>",element)
    } else {
      element
    }
  }

  pub fn field(&mut self, node: &Field) -> String {
    let name = node.name.to_string();
    let kind = if let Some(kind) = &node.kind {
      self.kind_annotation(&kind.kind)
    } else {
      "".to_string()
    };
    if self.html {
      format!("<div class=\"mech-field\"><span class=\"mech-field-name\">{}</span><span class=\"mech-field-kind\">{}</span></div>",name,kind)
    } else {
      format!("{}: {}", name, kind)
    }
  }

  pub fn tuple(&mut self, node: &Tuple) -> String {
    let mut src = "".to_string();
    for (i, element) in node.elements.iter().enumerate() {
      let e = self.expression(element);
      if i == 0 {
        src = format!("{}", e);
      } else {
        src = format!("{},{}", src, e);
      }
    }
    if self.html {
      format!("<span class=\"mech-tuple\"><span class=\"mech-start-paren\">(</span>{}<span class=\"mech-end-paren\">)</span></span>",src)
    } else {
      format!("({})", src)
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
    let kind = if let Some(kind) = &node.kind {
      self.kind_annotation(&kind.kind)
    } else {
      "".to_string()
    };
    let value = self.expression(&node.value);
    if self.html {
      format!("<span class=\"mech-binding\"><span class=\"mech-binding-name\">{}</span><span class=\"mech-binding-kind\">{}</span><span class=\"mech-binding-colon-op\">:</span><span class=\"mech-binding-value\">{}</span></span>",name,kind,value)
    } else {
      format!("{}{}: {}", name, kind, value)
    }
  }

  pub fn matrix(&mut self, node: &Matrix) -> String {
    let mut src = "".to_string();
    if node.rows.len() == 0 {
      if self.html {
        return format!("<span class=\"mech-matrix empty\"><span class=\"mech-bracket start\">[</span><span class=\"mech-bracket end\">]</span></span>");
      } else {
        return format!("[]");
      }
    }
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
      self.kind_annotation(&kind.kind)
    } else {
      "".to_string()
    };
    let name = &node.name.to_string();
    let id = format!("{}:{}",hash_str(&name),self.interpreter_id);
    if self.html {
      format!("<span class=\"mech-var-name mech-clickable\" id=\"{}\">{}</span>{}", id, node.name.to_string(), annotation)
    } else {
      format!("{}{}", node.name.to_string(), annotation)
    }
  }

  pub fn kind_annotation(&mut self, node: &Kind) -> String {
    let kind = self.kind(node);
    if self.html {
      format!("<span class=\"mech-kind-annotation\">&lt;{}&gt;</span>",kind)
    } else {
      format!("<{}>", kind)
    }
  }

  pub fn kind(&mut self, node: &Kind) -> String {
    let annotation = match node {
      Kind::Kind(kind) => {
        let kind_kind = self.kind(kind);
        if self.html {
          format!("<span class=\"mech-kind-annotation\">&lt;{}&gt;</span>",kind_kind)
        } else {
          format!("<{}>", kind_kind)
        }
      },
      Kind::Option(kind) => {
        let k = self.kind(kind);
        if self.html {
          format!("{}<span class=\"mech-option-question\">?</span>", k)
        } else {
          format!("{}?", k)
        }
      },
      Kind::Set(kind,size) => {
        let k = self.kind(kind);
        let size_str = match size{
          Some(size) => {
            let size_ltrl = self.literal(size);
            format!(":{}", size_ltrl)
          }
          None => "".to_string(),
        };
        format!("{{{}}}{}", k, size_str)
      },
      Kind::Any => "*".to_string(),
      Kind::Scalar(ident) => ident.to_string(),
      Kind::Empty => "_".to_string(),
      Kind::Atom(ident) => format!(":{}",ident.to_string()),
      Kind::Tuple(kinds) => {
        let mut src = "".to_string();
        for (i, kind) in kinds.iter().enumerate() {
          let k = self.kind(kind);
          if i == 0 {
            src = format!("{}", k);
          } else {
            src = format!("{},{}", src, k);
          }
        }
        format!("({})", src)
      },
      Kind::Matrix((kind, literals)) => {
        let mut src = "".to_string();
        let k = self.kind(kind);
        src = format!("{}", k);
        let mut src2 = "".to_string();
        for (i, literal) in literals.iter().enumerate() {
          let l = self.literal(literal);
          if i == 0 {
            src2 = format!(":{}", l);
          } else {
            src2 = format!("{},{}", src2, l);
          }
        }
        format!("[{}]{}", src, src2)
      },
      Kind::Record(kinds) => {
        let mut src = "".to_string();
        for (i, (ident, kind)) in kinds.iter().enumerate() {
          let k = self.kind(kind);
          let ident_s = ident.to_string();
          if i == 0 {
            src = format!("{}&lt;{}&gt;", ident_s, k);
          } else {
            src = format!("{},{}&lt;{}&gt;", src, ident_s, k);
          }
        }
        format!("{{{}}}", src)
      },
      Kind::Table((kinds, literal)) => {
        let mut src = "".to_string();
        for (i, (ident,kind)) in kinds.iter().enumerate() {
          let k = self.kind(kind);
          let ident_s = ident.to_string();
          if i == 0 {
            src = format!("{}&lt;{}&gt;", ident_s, k);
          } else {
            src = format!("{},{}&lt;{}&gt;", src, ident_s, k);
          }
        }
        let mut src2 = "".to_string();
        let sz = match &**literal {
          Literal::Empty(_) => "".to_string(),
          _ => format!(":{}", self.literal(literal)),
        };
        format!("|{}|{}", src, sz)
      },
      Kind::Map(kind1, kind2) => {
        let k1 = self.kind(kind1);
        let k2 = self.kind(kind2);
        format!("{{{}:{}}}", k1, k2)
      },
    };
    if self.html {
      format!("<span class=\"mech-kind\">{}</span>",annotation)
    } else {
      annotation
    }
  }

}
