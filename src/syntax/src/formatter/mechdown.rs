// Mechdown and section-block formatter routines.
//
// This module isolates markdown/mechdown block rendering paths so
// `formatter.rs` can stay focused on formatter state and core AST formatting.

use super::*;

impl Formatter {
  pub fn fenced_mech_code(&mut self, block: &FencedMechCode) -> String {
    self.interpreter_id = block.config.namespace;
    let block_id = hash_str(&format!("{:?}",block));
    let namespace_str = &block.config.namespace_str;
    let mut src = String::new();
    for (code,cmmnt) in &block.code {
      let c = match code {
        MechCode::Comment(cmnt) => self.comment(cmnt),
        MechCode::Expression(expr) => self.expression(expr),
        MechCode::FsmSpecification(fsm_spec) => self.fsm_specification(fsm_spec),
        MechCode::FsmImplementation(fsm_impl) => self.fsm_implementation(fsm_impl),
        MechCode::FunctionDefine(func_def) => self.function_define(func_def),
        MechCode::Statement(stmt) => self.statement(stmt),
        x => format!("{{{:?}}}", x)
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
    let intrp_id = self.interpreter_id;
    self.interpreter_id = 0;
    let disabled_tag = match block.config.disabled {
      true => "disabled".to_string(),
      false => "".to_string(),
    };
    if self.html {
      let (out_node,_) = block.code.last().unwrap();
      let output_id = hash_str(&format!("{:?}", out_node));
      let style_attr = match &block.options {
        Some(option_map) if !option_map.elements.is_empty() => {
          let style_str = option_map
            .elements
            .iter()
            .map(|(k, v)| {
              let clean_value = v.to_string().trim_matches('"').to_string();
              format!("{}: {}", k.to_string(), clean_value)
            })
            .collect::<Vec<_>>()
            .join("; ");
          format!(" style=\"{}\"", style_str)
        }
        _ => "".to_string(),
      };
      if block.config.disabled {
        format!("<pre class=\"mech-code-block disabled\"{}>{}</pre>", style_attr, src)
      } else if block.config.hidden {
        // Print it, but give it a hidden class so it can be toggled visible via JS
        format!("<pre class=\"mech-code-block hidden\"{}>{}</pre>", style_attr, src)
      } else {
        let namespace_str = if namespace_str.is_empty() {
          "".to_string()
        } else {
          format!("<div class=\"mech-code-block-namespace\"><a href=\"#{}\">{}</a></div>", block_id, namespace_str)
        };
        format!("<div id=\"{}\" class=\"mech-fenced-mech-block\"{}>
          {}
          <div class=\"mech-code-block\">{}</div>
          <div class=\"mech-block-output\" id=\"{}:{}\"></div>
        </div>", block_id, style_attr, namespace_str, src, output_id, intrp_id)
      }
    } else {
      format!("```mech{}\n{}\n```", src, format!(":{}", disabled_tag))
    }
  }

  pub fn image(&mut self, node: &Image) -> String {
    self.figure_num += 1;

    let src = node.src.to_string();
    let caption_p = match &node.caption {
      Some(caption) => self.paragraph(caption),
      None => "".to_string(),
    };

    let figure_label = format!("Fig {}.{}", self.h2_num, self.figure_num);
    let image_id = hash_str(&src);
    let figure_id = hash_str(&figure_label);

    if self.html {
      let style_attr = match &node.style {
        Some(option_map) if !option_map.elements.is_empty() => {
          let style_str = option_map
            .elements
            .iter()
            .map(|(k, v)| {
              let clean_value = v.to_string().trim_matches('"').to_string();
              format!("{}: {}", k.to_string(), clean_value)
            })
            .collect::<Vec<_>>()
            .join("; ");
          format!(" style=\"{}\"", style_str)
        }
        _ => "".to_string(),
      };
      format!(
"<figure id=\"{}\" class=\"mech-figure\">
  <img id=\"{}\" class=\"mech-image\" src=\"{}\"{} />
  <figcaption class=\"mech-figure-caption\">
    <strong class=\"mech-figure-label\">{}</strong> {}
  </figcaption>
</figure>",figure_id, image_id, src, style_attr, figure_label, caption_p)
    } else {
      let style_str = match &node.style {
        Some(option_map) if !option_map.elements.is_empty() => {
          let inner = option_map
            .elements
            .iter()
            .map(|(k, v)| {
              let clean_value = v.to_string().trim_matches('"').to_string();
              format!("{}: \"{}\"", k.to_string(), clean_value)
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!("{{{}}}", inner)
      }
      _ => "".to_string(),
    };

    format!("![{}]({}){}", caption_p, src, style_str)
    }
  }

  pub fn figures(&mut self, node: &FigureTable) -> String {
    self.figure_num += 1;
    let figure_label = format!("Fig {}.{}", self.h2_num, self.figure_num);
    let figure_id = hash_str(&format!("{}-{:?}", figure_label, node.rows));

    let mut figure_ix = 0usize;
    let mut captions: Vec<String> = vec![];

    if self.html {
      let mut rows_html = String::new();
      for row in &node.rows {
        rows_html.push_str(&format!(
          "<div class=\"mech-figure-table-row\" style=\"grid-template-columns: repeat({}, minmax(0, 1fr));\">",
          row.len().max(1)
        ));
        for figure in row {
          let label = ((b'a' + (figure_ix as u8)) as char).to_string();
          let img_id = hash_str(&format!("{}-{}-{}", figure_label, figure_ix, figure.src.to_string()));
          rows_html.push_str(&format!(
            "<div class=\"mech-figure-table-cell\"><div class=\"mech-figure-panel\"><span class=\"mech-figure-subfigure-label\">{}</span><img id=\"{}\" class=\"mech-image mech-figure-grid-image\" src=\"{}\" /></div></div>",
            label,
            img_id,
            figure.src.to_string(),
          ));
          captions.push(format!(
            "<span class=\"mech-figure-caption-ref\">({})</span> <span class=\"mech-figure-caption-text\">{}</span>",
            label,
            figure.caption.to_string()
          ));
          figure_ix += 1;
        }
        rows_html.push_str("</div>");
      }
      let caption_block = captions.join(" ");
      format!(
        "<figure id=\"{}\" class=\"mech-figure-table\"><div class=\"mech-figure-grid\">{}</div><figcaption class=\"mech-figure-caption mech-figure-table-caption\"><strong class=\"mech-figure-label\">{}</strong> {}</figcaption></figure>",
        figure_id, rows_html, figure_label, caption_block
      )
    } else {
      let mut lines: Vec<String> = vec![];
      for row in &node.rows {
        let mut line = String::from("|");
        for figure in row {
          line.push(' ');
          line.push_str(&format!("![{}]({})", self.paragraph(&figure.caption), figure.src.to_string()));
          line.push_str(" |");
          let label = ((b'a' + (figure_ix as u8)) as char).to_string();
          captions.push(format!("({}) {}", label, self.paragraph(&figure.caption)));
          figure_ix += 1;
        }
        lines.push(line);
      }
      format!("{}\n{} {}\n", lines.join("\n"), figure_label, captions.join(" "))
    }
  }


  pub fn abstract_el(&mut self, node: &Vec<Paragraph>) -> String {
    let abstract_paragraph = node.iter().map(|p| self.paragraph(p)).collect::<String>();
    if self.html {
      format!("<div id=\"abstract\" class=\"mech-abstract\">{}</div>", abstract_paragraph)
    } else {
      format!("{}\n", abstract_paragraph)
    }
  }

  pub fn equation(&mut self, node: &Token) -> String {
    let id = hash_str(&format!("equation-{}",node.to_string()));
    if self.html {
      format!("<div id=\"{}\" equation=\"{}\" class=\"mech-equation\"></div>",id, node.to_string())
    } else {
      format!("$$ {}\n", node.to_string())
    }
  }

  pub fn diagram(&mut self, node: &Token) -> String {
    let id = hash_str(&format!("diagram-{}",node.to_string()));
    if self.html {
      format!("<div id=\"{}\" class=\"mech-diagram mermaid\">{}</div>",id, node.to_string())
    } else {
      format!("```{{diagram}}\n{}\n```", node.to_string())
    }
  }

  pub fn citation(&mut self, node: &Citation) -> String {
    let id = hash_str(&format!("{}",node.id.to_string()));
    self.citations.resize(self.citation_num, String::new());
    let citation_text = self.paragraph(&node.text);
    let citation_num = match self.citation_map.get(&id) {
      Some(&num) => num,
      None => {
        return format!("Citation {} not found in citation map.", node.id.to_string());
      }
    };
    let formatted_citation = if self.html {
      format!("<div id=\"{}\" class=\"mech-citation\">
      <div class=\"mech-citation-id\">[{}]:</div>
      {}
    </div>",id, citation_num, citation_text)
    } else {
      format!("[{}]: {}",node.id.to_string(), citation_text)
    };
    self.citations[citation_num - 1] = formatted_citation;
    String::new()
  }

  pub fn float(&mut self, node: &Box<SectionElement>, float_dir: &FloatDirection) -> String {
    let mut src = "".to_string();
    let id = hash_str(&format!("float-{:?}",*node));
    let (float_class,float_sigil) = match float_dir {
      FloatDirection::Left => ("mech-float left","<<"),
      FloatDirection::Right => ("mech-float right",">>"),
    };
    let el = self.section_element(node);
    if self.html {
      format!("<div id=\"{}\" class=\"{}\">{}</div>",id,float_class,el)
    } else {
      format!("{}{}\n",float_sigil, el)
    }
  }

  pub fn info_block(&mut self, node: &Vec<Paragraph>) -> String {
    let info_paragraph = node.iter().map(|p| self.paragraph(p)).collect::<String>();
    if self.html {
      format!("<div class=\"mech-info-block\">{}</div>",info_paragraph)
    } else {
      format!("(i)> {}\n",info_paragraph)
    }
  }

  pub fn question_block(&mut self, node: &Vec<Paragraph>) -> String {
    let question_paragraph = node.iter().map(|p| self.paragraph(p)).collect::<String>();
    if self.html {
      format!("<div class=\"mech-question-block\">{}</div>",question_paragraph)
    } else {
      format!("(?)> {}\n",question_paragraph)
    }
  }

  pub fn success_block(&mut self, node: &Vec<Paragraph>) -> String {
    let success_paragraph = node.iter().map(|p| self.paragraph(p)).collect::<String>();
    if self.html {
      format!("<div class=\"mech-success-block\">{}</div>",success_paragraph)
    } else {
      format!("(✓)>> {}\n",success_paragraph)
    }
  }

  pub fn warning_block(&mut self, node: &Vec<Paragraph>) -> String {
    let warning_paragraph = node.iter().map(|p| self.paragraph(p)).collect::<String>();
    if self.html {
      format!("<div class=\"mech-warning-block\">{}</div>",warning_paragraph)
    } else {
      format!("(!)>> {}\n",warning_paragraph)
    }
  }

  pub fn idea_block(&mut self, node: &Vec<Paragraph>) -> String {
    let idea_paragraph = node.iter().map(|p| self.paragraph(p)).collect::<String>();
    if self.html {
      format!("<div class=\"mech-idea-block\">{}</div>",idea_paragraph)
    } else {
      format!("(*)> {}\n",idea_paragraph)
    }
  }

  pub fn error_block(&mut self, node: &Vec<Paragraph>) -> String {
    let error_paragraph = node.iter().map(|p| self.paragraph(p)).collect::<String>();
    if self.html {
      format!("<div class=\"mech-error-block\">{}</div>",error_paragraph)
    } else {
      format!("(✗)>> {}\n",error_paragraph)
    }
  }

  pub fn mika(&mut self, node: &(Mika, Option<MikaSection>)) -> String {
    let (mika, section) = node;
    let mika_str = format!("<div class=\"mech-mika\">{}</div>", mika.to_string());
    if self.html {
      match section {
        Some(sec) => {
          let mut sec_str = "".to_string();
          for el in &sec.elements.elements {
            let section_element = self.section_element(el);
            sec_str.push_str(&section_element);
          }
          format!("<div class=\"mech-mika-section\">{} {}</div>", mika_str, sec_str)
        },
        None => mika_str,
      }
    } else {
      mika_str
    }
  }

  pub fn prompt(&mut self, node: &SectionElement) -> String {
    let prompt_str = self.section_element(node);
    if self.html {
      format!("<div class=\"mech-prompt\"><span class=\"mech-prompt-sigil\">>:</span>{}</div>", prompt_str)
    } else {
      format!(">: {}\n", prompt_str)
    }
  }
    
  pub fn section_element(&mut self, node: &SectionElement) -> String {
    match node {
      SectionElement::Abstract(n) => self.abstract_el(n),
      SectionElement::QuoteBlock(n) => self.quote_block(n),
      SectionElement::SuccessBlock(n) => self.success_block(n),
      SectionElement::IdeaBlock(n) => self.idea_block(n),
      SectionElement::InfoBlock(n) => self.info_block(n),
      SectionElement::WarningBlock(n) => self.warning_block(n),
      SectionElement::ErrorBlock(n) => self.error_block(n),
      SectionElement::QuestionBlock(n) => self.question_block(n),
      SectionElement::Citation(n) => self.citation(n),
      SectionElement::CodeBlock(n) => self.code_block(n),
      SectionElement::Comment(n) => self.comment(n),
      SectionElement::Diagram(n) => self.diagram(n),
      SectionElement::Equation(n) => self.equation(n),
      SectionElement::Prompt(n) => self.prompt(n),
      SectionElement::FencedMechCode(n) => self.fenced_mech_code(n),
      SectionElement::Float((n,f)) => self.float(n,f),
      SectionElement::Footnote(n) => self.footnote(n),
      SectionElement::Grammar(n) => self.grammar(n),
      SectionElement::FigureTable(n) => self.figures(n),
      SectionElement::Image(n) => self.image(n),
      SectionElement::List(n) => self.list(n),
      SectionElement::MechCode(n) => self.mech_code(n),
      SectionElement::Mika(n) => self.mika(n),
      SectionElement::Paragraph(n) => self.paragraph(n),
      SectionElement::Subtitle(n) => self.subtitle(n),
      SectionElement::Table(n) => self.mechdown_table(n),
      SectionElement::ThematicBreak => self.thematic_break(),
      SectionElement::Error(src, range) => self.section_error(src.clone(), range),
    }
  }

  pub fn section_error(&mut self, src: Token, range: &SourceRange) -> String {
    if self.html {
      let mut error_str = String::new();
      error_str.push_str(&format!("<div class=\"mech-section-error\">\n"));
      error_str.push_str(&format!("<strong>Error in section at range {:?}-{:?}:</strong>\n", range.start, range.end));
      error_str.push_str(&format!("<pre class=\"mech-error-source\">{}</pre>\n", src.to_string()));
      error_str.push_str("</div>\n");
      error_str  
    } else {
      let mut error_str = String::new();
      error_str.push_str(&format!("Error in section at range {:?}-{:?}:\n", range.start, range.end));
      error_str.push_str(&format!("{} ", src.to_string()));
      error_str.push_str("\n");
      error_str
    }
  }

  pub fn footnote(&mut self, node: &Footnote) -> String {
    let (id_name, paragraphs) = node;
    let note_paragraph = paragraphs.iter().map(|p| self.paragraph(p)).collect::<String>();
    let id: u64 = hash_str(&format!("footnote-{}",id_name.to_string()));
    if self.html {
      format!("<div class=\"mech-footnote\" id=\"{}\">
        <div class=\"mech-footnote-id\">{}:</div>
        {}
      </div>",id, id_name.to_string(), note_paragraph)  
    } else {
      format!("[^{}]: {}\n",id_name.to_string(), note_paragraph)
    }
  }

  pub fn quote_block(&mut self, node: &Vec<Paragraph>) -> String {
    let quote_paragraph = node.iter().map(|p| self.paragraph(p)).collect::<String>();
    if self.html {
      format!("<blockquote class=\"mech-block-quote\">{}</blockquote>",quote_paragraph)
    } else {
      format!("> {}\n",quote_paragraph)
    }
  }

  pub fn thematic_break(&mut self) -> String {
    if self.html {
      format!("<hr class=\"mech-thematic-break\"/>")
    } else {
      format!("***\n")
    }
  }

  pub fn mechdown_table(&mut self, node: &MarkdownTable) -> String {
    if self.html {
      self.mechdown_table_html(node)
    } else {
      self.mechdown_table_string(node)
    }
  }


  pub fn mechdown_table_string(&mut self, node: &MarkdownTable) -> String {
    // Helper to render a row of Paragraphs as `| ... | ... |`
    fn render_row(cells: &[Paragraph], f: &mut impl FnMut(&Paragraph) -> String) -> String {
        let mut row = String::from("|");
        for cell in cells {
            row.push_str(" ");
            row.push_str(&f(cell));
            row.push_str(" |");
        }
        row
    }

    // Render header
    let header_line = render_row(&node.header, &mut |p| self.paragraph(p));

    // Render alignment row
    let mut align_line = String::from("|");
    for align in &node.alignment {
        let spec = match align {
            ColumnAlignment::Left => ":---",
            ColumnAlignment::Center => ":---:",
            ColumnAlignment::Right => "---:",
        };
        align_line.push_str(&format!(" {} |", spec));
    }

    // Render body rows
    let mut body_lines = vec![];
    for row in &node.rows {
        body_lines.push(render_row(row, &mut |p| self.paragraph(p)));
    }

    // Join everything
    let mut markdown = String::new();
    markdown.push_str(&header_line);
    markdown.push('\n');
    markdown.push_str(&align_line);
    markdown.push('\n');
    for line in body_lines {
        markdown.push_str(&line);
        markdown.push('\n');
    }

    markdown
}


  pub fn mechdown_table_html(&mut self, node: &MarkdownTable) -> String {
    let mut html = String::new();
    html.push_str("<table class=\"mech-table\">");

    // Render the header
    if !node.header.is_empty() {
      html.push_str("<thead><tr class=\"mech-table-header\">");
      for (i, cell) in node.header.iter().enumerate() {
        let align = match node.alignment.get(i) {
          Some(ColumnAlignment::Left) => "left",
          Some(ColumnAlignment::Center) => "center",
          Some(ColumnAlignment::Right) => "right",
          None => "left", // Default alignment
        };
        let cell_html = self.paragraph(cell);
        html.push_str(&format!(
          "<th class=\"mech-table-header-cell {}\">{}</th>", 
          align, cell_html
        ));
      }
      html.push_str("</tr></thead>");
    }

    // Render the rows
    html.push_str("<tbody>");
    for (row_index, row) in node.rows.iter().enumerate() {
      let row_class = if row_index % 2 == 0 { "mech-table-row-even" } else { "mech-table-row-odd" };
      html.push_str(&format!("<tr class=\"mech-table-row {}\">", row_class));
      for (i, cell) in row.iter().enumerate() {
        let align = match node.alignment.get(i) {
          Some(ColumnAlignment::Left) => "left",
          Some(ColumnAlignment::Center) => "center",
          Some(ColumnAlignment::Right) => "right",
          None => "left", // Default alignment
        };
        let cell_html = self.paragraph(cell);
        html.push_str(&format!(
          "<td class=\"mech-table-cell {}\">{}</td>", 
          align, cell_html
        ));
      }
      html.push_str("</tr>");
    }
    html.push_str("</tbody>");
    html.push_str("</table>");
    html
  }

  pub fn grammar(&mut self, node: &Grammar) -> String {
    let mut src = "".to_string();
    for rule in node.rules.iter() {
      let id = self.grammar_identifier(&rule.name);
      let rule_str = format!("{} <span class=\"mech-grammar-define-op\">:=</span>{}", id, self.grammar_expression(&rule.expr));
      if self.html {
        src = format!("{}<div class=\"mech-grammar-rule\">{} ;</div>",src,rule_str);
      } else {
        src = format!("{}{};\n",src,rule_str); 
      }
    }
    if self.html {
      format!("<div class=\"mech-grammar\">{}</div>",src)
    } else {
      src
    }
  }

  fn grammar_identifier(&mut self, node: &GrammarIdentifier) -> String {
    let name = node.name.to_string();
    if self.html {
      format!("<span id=\"{}\" class=\"mech-grammar-identifier\">{}</span>",hash_str(&name), name)
    } else {
      name
    }
  }

  fn grammar_expression(&mut self, node: &GrammarExpression) -> String {
    let expr = match node {
      GrammarExpression::List(element,deliniator) => {
        let el = self.grammar_expression(element);
        let del = self.grammar_expression(deliniator);
        if self.html {
          format!("<span class=\"mech-grammar-list\">[<span class=\"mech-grammar-list-element\">{}</span>,<span class=\"mech-grammar-list-deliniator\">{}</span>]</span>",el,del)
        } else {
          format!("[{},{}]",el,del)
        }
      },
      GrammarExpression::Range(start,end) => {
        if self.html {
          format!("<span class=\"mech-grammar-range\"><span class=\"mech-grammar-terminal\">\"{}\"</span><span class=\"mech-grammar-range-op\">..</span><span class=\"mech-grammar-terminal\">\"{}\"</span></span>", start.to_string(), end.to_string())
        } else {
          format!("{}..{}", start.to_string(), end.to_string())
        }
      }
      GrammarExpression::Choice(choices) => {
        let mut src = "".to_string();
        let inline = choices.len() <= 3;
        for (i, choice) in choices.iter().enumerate() {
          let choice_str = self.grammar_expression(choice);
          if i == 0 {
            src = format!("{}", choice_str);
          } else {
            if self.html {
              src = if inline {
                format!("{} <span class=\"mech-grammar-choice-op\">|</span> {}", src, choice_str)
              } else {
                format!("{}<div class=\"mech-grammar-choice\"><span class=\"mech-grammar-choice-op\">|</span> {}</div>", src, choice_str)
              };
            } else {
              src = format!("{} | {}", src, choice_str);
            }
          }
        }
        src
      },
      GrammarExpression::Sequence(seq) => {
        let mut src = "".to_string();
        let inline = seq.len() <= 3;
        for (i, factor) in seq.iter().enumerate() {
          let factor_str = self.grammar_expression(factor);
          if i == 0 {
        src = format!("{}", factor_str);
          } else {
        if self.html {
          src = if inline {
            format!("{}, {}", src, factor_str)
          } else {
            format!("{}<div class=\"mech-grammar-sequence\"><span class=\"mech-grammar-sequence-op\">,</span> {}</div>", src, factor_str)
          };
        } else {
          src = format!("{}, {}", src, factor_str);
        }
          }
        }
        src
      },
      GrammarExpression::Repeat0(expr) => {
        let inner_expr = self.grammar_expression(expr);
        if self.html {
          format!("<span class=\"mech-grammar-repeat0-op\">*</span>{}", inner_expr)
        } else {
          format!("*{}", inner_expr)
        }
      },
      GrammarExpression::Repeat1(expr) => {
        let inner_expr = self.grammar_expression(expr);
        if self.html {
          format!("<span class=\"mech-grammar-repeat1-op\">+</span>{}", inner_expr)
        } else {
          format!("+{}", inner_expr)
        }
      },
      GrammarExpression::Optional(expr) => {
        let inner_expr = self.grammar_expression(expr);
        if self.html {
          format!("<span class=\"mech-grammar-optional-op\">?</span>{}", inner_expr)
        } else {
          format!("?{}", inner_expr)
        }
      },
      GrammarExpression::Peek(expr) => {
        let inner_expr = self.grammar_expression(expr);
        if self.html {
          format!("<span class=\"mech-grammar-peek-op\">&gt;</span>{}", inner_expr)
        } else {
          format!(">{}", inner_expr)
        }
      },
      GrammarExpression::Not(expr) => {
        let inner_expr = self.grammar_expression(expr);
        if self.html {
          format!("<span class=\"mech-grammar-not-op\">¬</span>{}", inner_expr)
        } else {
          format!("¬{}", inner_expr)
        }
      },
      GrammarExpression::Terminal(token) => {
        if self.html {
          format!("<span class=\"mech-grammar-terminal\">\"{}\"</span>", token.to_string())
        } else {
          format!("\"{}\"", token.to_string())
        }
      },
      GrammarExpression::Group(expr) => {
        let inner_expr = self.grammar_expression(expr);
        if self.html {
          format!("<span class=\"mech-grammar-group\">(<span class=\"mech-grammar-group-content\">{}</span>)</span>", inner_expr)
        } else {
          format!("({})", inner_expr)
        }
      },
      GrammarExpression::Definition(id) => {
        let name = id.name.to_string();
        if self.html {
          format!("<span class=\"mech-grammar-definition\"><a href=\"#{}\">{}</a></span>",hash_str(&name), name)
        } else {
          name
        }
      },
    };
    
    if self.html {
      format!("<span class=\"mech-grammar-expression\">{}</span>", expr)
    } else {
      expr
    }
  }

  pub fn code_block(&mut self, node: &Token) -> String {
    let code = node.to_string();
    if self.html {
      format!("<pre class=\"mech-code-block\">{}</pre>",code)
    } else {
      format!("{}\n",code)
    }
  }

  pub fn comment(&mut self, node: &Comment) -> String {
    let comment_text = self.paragraph(&node.paragraph);
    if self.html {
      format!("<span class=\"mech-comment\"><span class=\"mech-comment-sigil\">--</span>{}</span>", comment_text)
    } else {
      format!("{}\n",comment_text)
    }
  }

  pub fn list(&mut self, node: &MDList) -> String {
    match node {
      MDList::Ordered(ordered_list) => self.ordered_list(ordered_list),
      MDList::Unordered(unordered_list) => self.unordered_list(unordered_list),
      MDList::Check(check_list) => self.check_list(check_list),
    }
  }

  pub fn check_list(&mut self, node: &CheckList) -> String {
    let mut lis = "".to_string();
    for (i, ((checked, item), sublist)) in node.iter().enumerate() {
      let it = self.paragraph(item);
      if self.html {
        lis = format!("{}<li class=\"mech-check-list-item\"><input type=\"checkbox\" {}>{}</li>", lis, if *checked { "checked" } else { "" }, it);
      } else {
        lis = format!("{}* [{}] {}\n", lis, if *checked { "x" } else { " " }, it);
      }
      match sublist {
        Some(sublist) => {
          let sublist_str = self.list(sublist);
          lis = format!("{}{}", lis, sublist_str);
        },
        None => {},
      }
    }
    if self.html {
      format!("<ul class=\"mech-check-list\">{}</ul>", lis)
    } else {
      lis
    }
  }

  pub fn ordered_list(&mut self, node: &OrderedList) -> String {
    let mut lis = "".to_string();
    for (i, ((num,item),sublist)) in node.items.iter().enumerate() {
      let it = self.paragraph(item);
      if self.html {
        lis = format!("{}<li class=\"mech-ol-list-item\">{}</li>",lis,it);
      } else {
        lis = format!("{}{}. {}\n",lis,i+1,it);
      }
      match sublist {
        Some(sublist) => {
          let sublist_str = self.list(sublist);
          lis = format!("{}{}",lis,sublist_str);
        },
        None => {},
      }
    }
    if self.html {
      format!("<ol start=\"{}\" class=\"mech-ordered-list\">{}</ol>",node.start.to_string(),lis)
    } else {
      lis
    }
  }

  pub fn unordered_list(&mut self, node: &UnorderedList) -> String {
    let mut lis = "".to_string();
    for (i, ((bullet, item),sublist)) in node.iter().enumerate() {
      let it = self.paragraph(item);
      match (bullet, self.html) {
        (Some(bullet_tok),true) => lis = format!("{}<li data-bullet=\"{}\" class=\"mech-list-item-emoji\">{}</li>",lis,bullet_tok.to_string(),it),
        (None,true) => lis = format!("{}<li class=\"mech-ul-list-item\">{}</li>",lis,it),
        (_,false) => lis = format!("{}* {}\n",lis,it),
      }
      match sublist {
        Some(sublist) => {
          let sublist_str = self.list(sublist);
          lis = format!("{}{}",lis,sublist_str);
        },
        None => {},
      }
    }
    if self.html {
      format!("<ul class=\"mech-unordered-list\">{}</ul>",lis)
    } else {
      lis
    }
  }

}
