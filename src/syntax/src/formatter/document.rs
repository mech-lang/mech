// Program/document-level formatter routines.
//
// Handles titles, sections, paragraph rendering, and inline paragraph elements.

use super::*;

impl Formatter {
  pub fn table_of_contents(&mut self, toc: &TableOfContents) -> String {
    self.toc = true;
    let sections = self.sections(&toc.sections);
    self.toc = false;
    let section_id = hash_str(&format!("section-{}", self.h2_num + 1));
    let formatted_works_cited = if self.html && self.citation_num > 0 && !self.citations.is_empty() {
      format!(
        "<section id=\"\" section=\"{}\" class=\"mech-program-section toc\">
  <h2 id=\"\" section=\"{}\" class=\"mech-program-subtitle toc active\">
    <a class=\"mech-program-subtitle-link toc\" href=\"#67320967384727436\">Works Cited</a>
  </h2>
</section>",
        section_id,
        self.h2_num + 1
      )
    } else {
      "".to_string()
    };
    format!("<div class=\"mech-toc\">{}{}</div>", sections, formatted_works_cited)
  }

  pub fn sections(&mut self, sections: &Vec<Section>) -> String {
    let mut src = "".to_string();
    let section_count = sections.len();
    for (i, section) in sections.iter().enumerate() {
      let s = self.section(section);
      src = format!("{}{}", src, s);
    }
    format!("<section class=\"mech-toc-sections\">{}</section>",src)
  }

  pub fn program(&mut self, node: &Program) -> String {
    let title = match &node.title {
      Some(title) => self.title(&title),
      None => "".to_string(),
    };
    let body = self.body(&node.body);
    let formatted_works_cited = self.works_cited();
    if self.html {
      format!("<div class=\"mech-content\"><div class=\"mech-program\">{}{}{}</div></div>",title,body,formatted_works_cited)
    } else {
      format!("{}{}{}",title,body,formatted_works_cited)
    }
  }

  pub fn title(&mut self, node: &Title) -> String {
    let title = node.text.to_string();

    if self.html {
      if let Some(byline) = &node.byline {
        let formatted_byline = self.paragraph(byline);
        format!(
          "<h1 class=\"mech-program-title\">{}</h1>\n<div class=\"mech-program-byline\">{}</div>",
          title,
          formatted_byline
        )
      } else {
        format!("<h1 class=\"mech-program-title\">{}</h1>", title)
      }
    } else {
      let mut out = format!(
        "{}\n===============================================================================\n",
        title
      );

      if let Some(byline) = &node.byline {
        let byline_str = self.paragraph(byline);
        out.push_str(&format!(
          "{}\n===============================================================================\n",
          byline_str
        ));
      }

      out
    }
  }

  pub fn subtitle(&mut self, node: &Subtitle) -> String {
    let level = node.level;
    if level == 2 {
      self.h2_num  += 1;
      self.h3_num = 0;
      self.h4_num = 0;
      self.h5_num = 0;
      self.h6_num = 0;
      self.figure_num = 0;
    } else if level == 3 {
      self.h3_num += 1;
    } else if level == 4 {
      self.h4_num += 1;
    } else if level == 5 {
      self.h5_num += 1;
    } else if level == 6 {
      self.h6_num += 1;
    }
    
    let toc = if self.toc { "toc" } else { "" };
    let title_id = hash_str(&format!("{}.{}.{}.{}.{}{}",self.h2_num,self.h3_num,self.h4_num,self.h5_num,self.h6_num,toc));
    
    let link_str = format!("{}.{}.{}.{}.{}",self.h2_num,self.h3_num,self.h4_num,self.h5_num,self.h6_num);
    let link_id  = hash_str(&link_str);

    let section = if level == 2 { format!("section=\"{}.{}\"", self.h2_num, self.h3_num) } 
    else if level == 3 { format!("section=\"{}.{}\"", self.h2_num, self.h3_num) }
    else if level == 4 { format!("section=\"{}.{}.{}\"", self.h2_num, self.h3_num, self.h4_num) }
    else if level == 5 { format!("section=\"{}.{}.{}.{}\"", self.h2_num, self.h3_num, self.h4_num, self.h5_num) }
    else if level == 6 { format!("section=\"{}.{}.{}.{}.{}\"", self.h2_num, self.h3_num, self.h4_num, self.h5_num, self.h6_num) }
    else { "".to_string() };    

    if self.html {
      format!("<h{} id=\"{}\" {} class=\"mech-program-subtitle {}\"><a class=\"mech-program-subtitle-link {}\" href=\"#{}\">{}</a></h{}>", level, title_id, section, toc, toc, link_id, node.to_string(), level)
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
    for el in node.elements.iter() {
      let el_str = self.section_element(el);
      src = format!("{}{}", src, el_str);
    }
    let toc = if self.toc { "toc" } else { "" };
    let section_id = hash_str(&format!("section-{}",self.h2_num + 1));
    let id = hash_str(&format!("section-{}{}",self.h2_num + 1, toc));
     if self.html {
      format!("<section id=\"{}\" section=\"{}\" class=\"mech-program-section {}\">{}</section>",id,section_id,toc,src)
    } else {
      src
    }
  }

  pub fn paragraph(&mut self, node: &Paragraph) -> String {
    let mut src = "".to_string();
    for el in node.elements.iter() {
      let el_str = self.paragraph_element(el);
      src = format!("{}{}", src, el_str);
    }
    let result = if self.html {
      format!("<p class=\"mech-paragraph\">{}</p>",src)
    } else {
      format!("{}\n",src)
    };
    result
  }

  pub fn inline_paragraph(&mut self, node: &Paragraph) -> String {
    let mut src = "".to_string();
    for el in node.elements.iter() {
      let el_str = self.paragraph_element(el);
      src = format!("{}{}", src, el_str);
    }
    let result = if self.html {
      format!("<span class=\"mech-inline-paragraph\">{}</span>",src)
    } else {
      format!("{}",src)
    };
    result
  }

  fn footnote_reference(&mut self, node: &Token) -> String {
    let id_string = node.to_string();
    let id_hash = hash_str(&format!("footnote-{}",id_string));
    if self.html {
      format!("<a href=\"#{}\" class=\"mech-footnote-reference\">{}</a>",id_hash, id_string)
    } else {
      format!("[^{}]",id_string)
    }
  }

  fn inline_equation(&mut self, node: &Token) -> String {
    let id = hash_str(&format!("inline-equation-{}",node.to_string()));
    if self.html {
      format!("<span id=\"{}\" equation=\"{}\" class=\"mech-inline-equation\"></span>",id, node.to_string())
    } else {
      format!("$${}$$", node.to_string())
    }
  }

  fn highlight(&mut self, node: &Token) -> String {
    if self.html {
      format!("<mark class=\"mech-highlight\">{}</mark>", node.to_string())
    } else {
      format!("!!{}!!", node.to_string())
    }
  }

  fn reference(&mut self, node: &Token) -> String {
    self.citation_num += 1;
    let id = hash_str(&format!("reference-{}",node.to_string()));
    let ref_id = hash_str(&format!("{}",node.to_string()));
    self.citation_map.insert(ref_id, self.citation_num);
    if self.html {
      format!("<span id=\"{}\" class=\"mech-reference\">[<a href=\"#{}\" class=\"mech-reference-link\">{}</a>]</span>",id, ref_id, self.citation_num)
    } else {
      format!("[{}]",node.to_string())
    }
  }
  
  pub fn paragraph_element(&mut self, node: &ParagraphElement) -> String {
    match node {
      ParagraphElement::Error(t, s) => {
        if self.html {
          format!("<span class=\"mech-error\" title=\"Error at {:?}\">{}</span>", s, t.to_string())
        } else {
          format!("{{ERROR: {} at {:?}}}", t.to_string(), s)
        }
      },
      ParagraphElement::Highlight(n) => {
        if self.html {
          format!("<mark class=\"mech-highlight\">{}</mark>", n.to_string())
        } else {
          format!("!!{}!!", n.to_string())
        }
      },
      ParagraphElement::SectionReference(n) => {
        let section_id_str = n.to_string();
        let parts: Vec<&str> = section_id_str.split('.').collect();
        let mut nums = vec!["0"; 5]; // up to h6 level
        for (i, part) in parts.iter().enumerate() {
          nums[i] = part;
        }
        let id_str = format!("{}.{}.{}.{}.{}",nums[0], nums[1], nums[2], nums[3], nums[4]);
        let id = hash_str(&id_str);

        if self.html {
          format!(
            "<span class=\"mech-section-reference\">
              <a href=\"#{}\" class=\"mech-section-reference-link\">§{}</a>
            </span>",
            id, n.to_string()
          )
        } else {
          format!("§{}", n.to_string())
        }
      }
      ParagraphElement::Reference(n) => self.reference(n),
      ParagraphElement::InlineEquation(exq) => self.inline_equation(exq),
      ParagraphElement::Text(n) => {
        if self.html {
          format!("<span class=\"mech-text\">{}</span>", n.to_string())
        } else {
          n.to_string()
        }
      }
      ParagraphElement::FootnoteReference(n) => self.footnote_reference(n),
      ParagraphElement::Strong(n) => {
        let p = self.paragraph_element(n);
        if self.html {
          format!("<strong class=\"mech-strong\">{}</strong>", p)
        } else {
          format!("**{}**", p)
        }
      },
      ParagraphElement::Hyperlink((text, url)) => {
        let url_str = url.to_string();
        let text_str = self.inline_paragraph(text);
        if self.html {
          format!("<a href=\"{}\" class=\"mech-hyperlink\">{}</a>",url_str,text_str)
        } else {
          format!("[{}]({})",text_str,url_str)
        }
      },
      ParagraphElement::Emphasis(n) => {
        if self.html {
          format!("<em class=\"mech-em\">{}</em>", n.to_string())
        } else {
          format!("*{}*", n.to_string())
        }
      },
      ParagraphElement::Underline(n) => {
        if self.html {
          format!("<u class=\"mech-u\">{}</u>", n.to_string())
        } else {
          format!("_{}_", n.to_string())
        }
      },
      ParagraphElement::Strikethrough(n) => {
        if self.html {
          format!("<del class=\"mech-del\">{}</del>", n.to_string())
        } else {
          format!("~{}~", n.to_string())
        }
      },
      ParagraphElement::InlineCode(n) => {
        if self.html {
          format!("<code class=\"mech-inline-code\">{}</code>", n.to_string().trim())
        } else {
          format!("`{}`", n.to_string())
        }
      },
      ParagraphElement::InlineMechCode(code) => {
        let result = self.mech_code(&vec![(code.clone(),None)]);
        if self.html {
          format!("<span class=\"mech-inline-mech-code-formatted\">{}</span>", result)
        } else {
          format!("{{{}}}", result)
        }
      },
      ParagraphElement::EvalInlineMechCode(expr) => {
        let code_id = hash_str(&format!("{:?}", expr));
        let result = self.expression(expr);
        if self.html {
          format!("<code id=\"{}\" class=\"mech-inline-mech-code\">{}</code>", code_id, result)
        } else {
          format!("{{{}}}", result)
        }
      },
    }
  }

}
