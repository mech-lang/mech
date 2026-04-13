// Literal and scalar formatter routines.

use super::*;

impl Formatter {
  pub fn boolean(&mut self, node: &Token) -> String {
    let b = node.to_string();
    if self.html {
      format!("<span class=\"mech-boolean\">{}</span>", b)
    } else {
      b
    }
  }

  pub fn empty(&mut self, node: &Token) -> String {
    let e = node.to_string();
    if self.html {
      format!("<span class=\"mech-empty\">{}</span>", e)
    } else {
      e
    }
  }

  pub fn literal(&mut self, node: &Literal) -> String {
    let l = match node {
      Literal::Empty(tkn) => self.empty(tkn),
      Literal::Boolean(tkn) => self.boolean(tkn),
      Literal::Number(num) => self.number(num),
      Literal::String(mech_string) => self.string(mech_string),
      Literal::Atom(atm) => self.atom(atm),
      Literal::Kind(knd) => self.kind_annotation(knd),
      Literal::TypedLiteral((boxed_literal, kind_annotation)) => {
        let literal = self.literal(boxed_literal);
        let annotation = self.kind_annotation(&kind_annotation.kind);
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
      format!("<span class=\"mech-atom\"><span class=\"mech-atom-name\">:{}</span></span>",node.name.to_string())
    } else {
      format!(":{}", node.name.to_string())
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
      Number::Complex(complex) => self.complex_numer(complex),
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
      RealNumber::Decimal(token) => format!("0d{}", token.to_string()),
      RealNumber::Hexadecimal(token) => format!("0x{}", token.to_string()),
      RealNumber::Octal(token) => format!("0o{}", token.to_string()),
      RealNumber::Binary(token) => format!("0b{}", token.to_string()),
      RealNumber::Scientific(((whole, part), (sign, ewhole, epart))) => format!("{}.{}e{}{}.{}", whole.to_string(), part.to_string(), if *sign { "-" } else { "+" }, ewhole.to_string(), epart.to_string()),
      RealNumber::Rational((numerator, denominator)) => format!("{}/{}", numerator.to_string(), denominator.to_string()),
      RealNumber::TypedInteger((token, kind_annotation)) => {
        let num = token.to_string();
        let annotation = &kind_annotation.kind.tokens().iter().map(|tkn| tkn.to_string()).collect::<Vec<String>>().join("");
        format!("{}{}", num, annotation)
      }
    }
  }

  pub fn complex_numer(&mut self, node: &C64Node) -> String {
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
    let mut result = String::new();
    let mut indent_level = 0;
    let mut in_special_tag = false;
    let mut special_tag = "";
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    
    let self_closing_tags = HashSet::from([
      "area", "base", "br", "col", "embed", "hr", "img", "input", 
      "link", "meta", "param", "source", "track", "wbr"
    ]);
    
    fn matches_tag(chars: &[char], pos: usize, tag: &str) -> bool {
      let tag_chars: Vec<char> = tag.chars().collect();
      pos + tag_chars.len() <= chars.len() && chars[pos..pos+tag_chars.len()] == tag_chars[..]
    }
    
    while i < chars.len() {
      // Handle <pre> and <code> tags
      if !in_special_tag && (matches_tag(&chars, i, "<pre") || matches_tag(&chars, i, "<code")) {
        in_special_tag = true;
        special_tag = if matches_tag(&chars, i, "<pre") { "pre" } else { "code" };
        
        result.push('\n');
        result.push_str(&"  ".repeat(indent_level));
        
        // Add the opening tag
        while i < chars.len() && chars[i] != '>' {
          result.push(chars[i]);
          i += 1;
        }
        result.push('>');
        i += 1;
        indent_level += 1;
        
        // Process content
        let start = i;
        while i < chars.len() && !matches_tag(&chars, i, &format!("</{}>", special_tag)) {
          i += 1;
        }
        
        // Add the content as is
        result.extend(chars[start..i].iter());
        
        // Add the closing tag
        if i < chars.len() {
          result.push_str(&format!("</{}>", special_tag));
          i += special_tag.len() + 3;
          in_special_tag = false;
          indent_level -= 1;
        }
      // Open tag
      } else if !in_special_tag && i < chars.len() && chars[i] == '<' && i+1 < chars.len() && chars[i+1] != '/' {
        let tag_start = i + 1;
        let mut j = tag_start;
        
        // Get tag name
        while j < chars.len() && chars[j].is_alphanumeric() {
          j += 1;
      }
        
        let tag_name: String = chars[tag_start..j].iter().collect();
        let is_self_closing = self_closing_tags.contains(tag_name.as_str());
        
        // Add newline and indentation
        result.push('\n');
        result.push_str(&"  ".repeat(indent_level));
        
        // Add the tag
        while i < chars.len() && chars[i] != '>' {
          result.push(chars[i]);
          i += 1;
        }
        result.push('>');
        i += 1;
        
        if !is_self_closing {
          indent_level += 1;
        }
      // Close tag
      } else if !in_special_tag && i < chars.len() && chars[i] == '<' && i+1 < chars.len() && chars[i+1] == '/' {
        indent_level = indent_level.saturating_sub(1);
        
        result.push('\n');
        result.push_str(&"  ".repeat(indent_level));
        
        while i < chars.len() && chars[i] != '>' {
            result.push(chars[i]);
            i += 1;
        }
        result.push('>');
        i += 1;
      // Regular text content
      } else if !in_special_tag {
        let start = i;
        while i < chars.len() && chars[i] != '<' {
          i += 1;
        }
        
        let content: String = chars[start..i].iter().collect();
        if !content.trim().is_empty() {
          result.push('\n');
          result.push_str(&"  ".repeat(indent_level));
          result.push_str(&content);
      }
      // Inside <pre> or <code>
      } else {
        result.push(chars[i]);
        i += 1;
      }
    }
    result.push('\n');
    result
  }
}
